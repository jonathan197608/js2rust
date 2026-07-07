// zigir/lower/decl.rs
// Declaration lowering: variables, functions, parameters, nested functions.

use oxc_ast::ast::*;

use crate::types::ZigType;
use crate::zigir::ident::IrIdent;
use crate::zigir::source_span::SourceSpan;
use crate::zigir::types::{IrBlock, IrDecl, IrParam, IrVarDecl};

use super::Lowerer;
use super::cabi::{init_may_have_side_effects, property_key_name};

impl Lowerer {
    /// Lower a variable declaration.
    ///
    /// Translates JS `const`/`var`/`let` into `IrDecl::Var`. The Lowerer
    /// resolves semantic information (const vs var, type annotation, JSON.parse
    /// special case) and defers all formatting to the Emitter.
    ///
    /// Shadow renaming is NOT done here ¡ª it's a scope-level concern handled
    /// by the NameMangler during identifier resolution.
    pub(super) fn lower_var_decl(
        &mut self,
        decl: &VariableDeclarator,
        _vd_is_const: bool,
    ) -> IrDecl {
        let js_name = match crate::infer::binding_name(&decl.id) {
            Some(n) => n,
            None => {
                return IrDecl::CompileError {
                    span: SourceSpan::default(),
                    msg: "unsupported binding pattern in variable declaration".to_string(),
                };
            }
        };

        // ©¤©¤ Shadow renaming: detect duplicate variable names ©¤©¤
        // If the name already exists in fn_scope_vars, rename it to avoid
        // Zig's "redeclaration" error. This handles cases like:
        //   let x = 10; { let x = 20; }  ¡ú  const x = 10; { const x_shadow_1 = 20; }
        // Also handles: function f(data) { let data = 100; }  ¡ú  fn f(data: i64) { const data_shadow_1 = 100; }
        let ident = if let Some(ctx) = self.fn_ctx.as_ref() {
            if ctx.fn_scope_vars.contains(js_name) {
                // Name is already declared in this function ¡ª generate a shadow name
                let shadow_zig_name = format!(
                    "{}_shadow_{}",
                    js_name,
                    self.name_mangler.next_name("shadow")
                );
                self.name_mangler.record_shadow(js_name, shadow_zig_name);
                // Also register the shadow name in fn_scope_vars to prevent further collisions
                if let Some(ctx) = self.fn_ctx.as_mut() {
                    ctx.add_scope_var(js_name);
                }
                IrIdent::with_zig_name(js_name, self.name_mangler.resolve_name(js_name))
            } else {
                // First occurrence ¡ª register and use normal name resolution
                if let Some(ctx) = self.fn_ctx.as_mut() {
                    ctx.add_scope_var(js_name);
                }
                self.make_ident(js_name)
            }
        } else {
            self.make_ident(js_name)
        };

        // Determine const vs var based on mutation analysis (not JS keyword).
        // Zig 'const' for never-mutated, 'var' for actually reassigned.
        let fn_prefix = self
            .fn_ctx
            .as_ref()
            .map(|ctx| ctx.name.as_str())
            .unwrap_or("__toplevel__");
        let is_const = !self
            .type_info
            .mutated_vars
            .contains(&format!("{}::{}", fn_prefix, js_name));

        // Skip unused toplevel constants
        let has_type_annotation = self.jsdoc_data.type_annotations.contains_key(js_name);
        if self.fn_ctx.is_none()
            && is_const
            && !self.type_info.used_names.contains(js_name)
            && !has_type_annotation
        {
            return IrDecl::CompileError {
                span: SourceSpan::default(),
                msg: format!("skipped unused toplevel const: {}", js_name),
            };
        }

        // Toplevel var/let ¡ú error (only const allowed at module level)
        if self.fn_ctx.is_none() && !is_const {
            return IrDecl::CompileError {
                span: SourceSpan::default(),
                msg: format!("toplevel only allows 'const', not '{}'", js_name),
            };
        }

        // Force 'var' for Map/Set/ArrayList types (mutated via methods)
        let is_const = if let Some(inferred_ty) = self.type_info.var_types.get(js_name) {
            match inferred_ty {
                ZigType::ArrayList(_) => false,
                ZigType::NamedStruct(n) if n == "Map" || n == "Set" => false,
                _ => is_const,
            }
        } else {
            is_const
        };

        // Type from inference
        let zig_type = self.type_info.var_types.get(js_name).cloned();

        // JSON.parse special case
        let is_json_parse = self.type_info.has_json_parse_types.contains(js_name);

        // Needs var suppression (ArrayList/Map/Set method calls need `_= &var;`)
        let needs_var_suppression = !is_const
            && matches!(
                zig_type,
                Some(ZigType::ArrayList(_)) | Some(ZigType::NamedStruct(_))
            );

        // Lower initializer expression
        let init = match decl.init.as_ref() {
            Some(expr) => {
                // Special case: arrow function / closure initializer.
                // Instead of returning IrExpr::ArrowFn as init, we:
                // 1. Register the struct definition in module.closure_structs
                // 2. Return IrExpr::Ident pointing to the struct name
                // This produces the output pattern:
                //   const _arrow_fn_0 = struct { ... };
                //   const double = _arrow_fn_0;
                let ir = self.lower_expr(expr);
                match ir {
                    crate::zigir::types::IrExpr::ArrowFn(ref arrow) => {
                        // Generate a unique struct name like _arrow_fn_0, _arrow_fn_1, ...
                        // Use name_mangler counter to get sequential numbering.
                        let idx = self.name_mangler.peek_count("arrow_fn");
                        self.name_mangler.next_name("arrow_fn"); // advance counter
                        let struct_name = format!("_arrow_fn_{}", idx);
                        let struct_ident = IrIdent::new(&struct_name);
                        // Register as closure struct (with empty captures)
                        self.pending_arrow_structs
                            .push(crate::zigir::types::IrClosureStruct {
                                name: struct_ident.clone(),
                                captured: vec![],
                                fn_params: arrow.params.clone(),
                                return_type: arrow.return_type.clone(),
                                typeof_return_body: None,
                                body: arrow.body.clone(),
                            });
                        Some(crate::zigir::types::IrExpr::Ident(struct_ident))
                    }
                    crate::zigir::types::IrExpr::Closure(ref closure) => {
                        // Struct already registered by lower_fn_expr / lower_arrow_fn.
                        // Closure instance: StructName { .captured = val, ... }
                        Some(crate::zigir::types::IrExpr::Closure(closure.clone()))
                    }
                    _ => Some(ir),
                }
            }
            None => None,
        };

        // Register TypedArray variables (for suffix lookup in lower_call)
        if let Some(ZigType::NamedStruct(n)) = &zig_type
            && let Some(suffix) = Self::typedarray_type_suffix(n)
            && let Some(ctx) = self.fn_ctx.as_mut()
        {
            ctx.add_typedarray_var(&ident.zig_name, suffix);
        }

        // Register RegExp variables (for .test()/.exec() method dispatch in lower_call)
        if let Some(expr) = decl.init.as_ref()
            && Self::is_regexp_expr(expr)
            && let Some(ctx) = self.fn_ctx.as_mut()
        {
            ctx.add_regexp_var(&ident.zig_name);
        }

        IrDecl::Var(IrVarDecl {
            name: ident,
            is_const,
            zig_type,
            init,
            is_json_parse,
            needs_var_suppression,
        })
    }

    /// Lower a destructuring variable declaration.
    ///
    /// Handles `const {a, b} = expr` and `const [a, b] = expr` by constructing
    /// an `IrStmt::DestructureDecl` that the Emitter expands into temp variable
    /// + individual const/var declarations.
    pub(super) fn lower_destructure_decl(
        &mut self,
        decl: &VariableDeclarator,
    ) -> crate::zigir::types::IrStmt {
        use crate::zigir::types::{
            IrDestructureAccess, IrDestructureBindingDecl, IrDestructureDecl, IrDestructureKind,
        };

        let Some(init_expr) = &decl.init else {
            return crate::zigir::types::IrStmt::CompileError {
                span: SourceSpan::default(),
                msg: "destructuring requires an initializer".to_string(),
            };
        };

        let init_ir = self.lower_expr(init_expr);

        match &decl.id {
            BindingPattern::ObjectPattern(op) => {
                // ©¤©¤ Object destructuring ©¤©¤
                let init_type = if let Expression::Identifier(id) = init_expr {
                    self.type_info.var_types.get(id.name.as_str()).cloned()
                } else {
                    None
                };

                let struct_field_names: Option<Vec<String>> = match &init_type {
                    Some(ZigType::Struct(fields)) if !fields.is_empty() => {
                        Some(fields.iter().map(|(n, _)| n.clone()).collect())
                    }
                    _ => None,
                };
                let is_struct = struct_field_names.is_some();

                // Decide if we need a temp variable
                let needs_temp = init_may_have_side_effects(init_expr) || op.properties.len() > 1;
                let temp_name = if needs_temp {
                    Some(self.name_mangler.next_name("_js_dest"))
                } else {
                    None
                };

                // Source for access patterns: temp var name or inline the init expr
                let source = if needs_temp {
                    temp_name.clone().unwrap()
                } else if let Expression::Identifier(id) = init_expr {
                    id.name.to_string()
                } else {
                    self.name_mangler.next_name("_js_dest")
                };

                let fn_prefix = self
                    .fn_ctx
                    .as_ref()
                    .map(|ctx| ctx.name.as_str())
                    .unwrap_or("__toplevel__");

                // Phase 1: Collect binding metadata (no lower_expr calls yet)
                struct RawObjBinding {
                    key_name: String,
                    bind_name: String,
                    has_default: bool,
                    default_index: usize, // index into op.properties if has_default
                    is_struct_field: bool,
                    is_const: bool,
                }
                let mut raw_bindings: Vec<RawObjBinding> = Vec::new();
                let mut default_counter: usize = 0;
                for prop in &op.properties {
                    let key_name = match property_key_name(&prop.key) {
                        Some(k) => k,
                        None => continue,
                    };

                    let (bind_name, has_default) = match &prop.value {
                        BindingPattern::BindingIdentifier(id) => (id.name.as_str(), false),
                        BindingPattern::AssignmentPattern(ap) => {
                            let Some(name) = crate::infer::binding_name(&ap.left) else {
                                continue;
                            };
                            (name, true)
                        }
                        _ => continue,
                    };

                    let is_const = !self
                        .type_info
                        .mutated_vars
                        .contains(&format!("{}::{}", fn_prefix, bind_name));

                    // Skip unused toplevel constants
                    if self.fn_ctx.is_none()
                        && is_const
                        && !self.type_info.used_names.contains(bind_name)
                    {
                        default_counter += if has_default { 1 } else { 0 };
                        continue;
                    }
                    if self.fn_ctx.is_none() && !is_const {
                        default_counter += if has_default { 1 } else { 0 };
                        continue;
                    }

                    let is_struct_field = struct_field_names
                        .as_ref()
                        .is_some_and(|names| names.contains(&key_name));

                    let di = default_counter;
                    default_counter += if has_default { 1 } else { 0 };

                    raw_bindings.push(RawObjBinding {
                        key_name,
                        bind_name: bind_name.to_string(),
                        has_default,
                        default_index: di,
                        is_struct_field,
                        is_const,
                    });
                }

                // Collect default expression references from AST
                let default_exprs: Vec<&Expression> = op
                    .properties
                    .iter()
                    .filter_map(|prop| match &prop.value {
                        BindingPattern::AssignmentPattern(ap) => Some(&ap.right),
                        _ => None,
                    })
                    .collect();

                // Phase 2: Build IrDestructureBindingDecl (can call self.lower_expr now)
                let mut bindings = Vec::new();
                for rb in raw_bindings {
                    let default_ir = if rb.has_default {
                        default_exprs
                            .get(rb.default_index)
                            .map(|e| self.lower_expr(e))
                    } else {
                        None
                    };
                    bindings.push(IrDestructureBindingDecl {
                        name: self.make_ident(&rb.bind_name),
                        is_const: rb.is_const,
                        access: IrDestructureAccess::ObjectField {
                            source: if needs_temp {
                                temp_name.clone().unwrap()
                            } else {
                                source.clone()
                            },
                            key: rb.key_name,
                            is_struct_field: rb.is_struct_field,
                        },
                        default: default_ir,
                    });
                }

                crate::zigir::types::IrStmt::DestructureDecl(IrDestructureDecl {
                    temp_name: if needs_temp { temp_name } else { None },
                    init: init_ir,
                    kind: IrDestructureKind::Object {
                        is_struct,
                        struct_field_names,
                    },
                    bindings,
                })
            }

            BindingPattern::ArrayPattern(ap) => {
                // ©¤©¤ Array destructuring ©¤©¤
                let init_type = if let Expression::Identifier(id) = init_expr {
                    self.type_info.var_types.get(id.name.as_str()).cloned()
                } else {
                    None
                };
                let is_arraylist = matches!(init_type, Some(ZigType::ArrayList(_)));

                // Decide if we need a temp variable
                let element_count = ap.elements.iter().filter(|e| e.is_some()).count();
                let needs_temp = init_may_have_side_effects(init_expr) || element_count > 1;
                let temp_name = if needs_temp {
                    Some(self.name_mangler.next_name("_js_dest"))
                } else {
                    None
                };

                let source = if needs_temp {
                    temp_name.clone().unwrap()
                } else if let Expression::Identifier(id) = init_expr {
                    id.name.to_string()
                } else {
                    self.name_mangler.next_name("_js_dest")
                };

                let fn_prefix = self
                    .fn_ctx
                    .as_ref()
                    .map(|ctx| ctx.name.as_str())
                    .unwrap_or("__toplevel__");

                // Phase 1: Collect binding metadata
                struct RawArrBinding {
                    bind_name: String,
                    index: usize,
                    has_default: bool,
                    default_index: usize,
                    is_const: bool,
                }
                let mut raw_bindings: Vec<RawArrBinding> = Vec::new();
                let mut default_counter: usize = 0;
                for (i, elem) in ap.elements.iter().enumerate() {
                    let Some(pattern) = elem else {
                        continue; // skip holes
                    };

                    let (bind_name, has_default) = match pattern {
                        BindingPattern::BindingIdentifier(id) => (id.name.as_str(), false),
                        BindingPattern::AssignmentPattern(ap_inner) => {
                            let Some(name) = crate::infer::binding_name(&ap_inner.left) else {
                                continue;
                            };
                            (name, true)
                        }
                        _ => continue,
                    };

                    let is_const = !self
                        .type_info
                        .mutated_vars
                        .contains(&format!("{}::{}", fn_prefix, bind_name));

                    if self.fn_ctx.is_none()
                        && is_const
                        && !self.type_info.used_names.contains(bind_name)
                    {
                        default_counter += if has_default { 1 } else { 0 };
                        continue;
                    }
                    if self.fn_ctx.is_none() && !is_const {
                        default_counter += if has_default { 1 } else { 0 };
                        continue;
                    }

                    let di = default_counter;
                    default_counter += if has_default { 1 } else { 0 };

                    raw_bindings.push(RawArrBinding {
                        bind_name: bind_name.to_string(),
                        index: i,
                        has_default,
                        default_index: di,
                        is_const,
                    });
                }

                // Collect default expression references from AST
                let default_exprs: Vec<&Expression> = ap
                    .elements
                    .iter()
                    .flatten()
                    .filter_map(|pattern| match pattern {
                        BindingPattern::AssignmentPattern(ap_inner) => Some(&ap_inner.right),
                        _ => None,
                    })
                    .collect();

                // Phase 2: Build IrDestructureBindingDecl
                let mut bindings = Vec::new();
                for rb in raw_bindings {
                    let default_ir = if rb.has_default {
                        default_exprs
                            .get(rb.default_index)
                            .map(|e| self.lower_expr(e))
                    } else {
                        None
                    };
                    bindings.push(IrDestructureBindingDecl {
                        name: self.make_ident(&rb.bind_name),
                        is_const: rb.is_const,
                        access: IrDestructureAccess::ArrayIndex {
                            source: if needs_temp {
                                temp_name.clone().unwrap()
                            } else {
                                source.clone()
                            },
                            index: rb.index,
                        },
                        default: default_ir,
                    });
                }

                crate::zigir::types::IrStmt::DestructureDecl(IrDestructureDecl {
                    temp_name: if needs_temp { temp_name } else { None },
                    init: init_ir,
                    kind: IrDestructureKind::Array { is_arraylist },
                    bindings,
                })
            }

            _ => crate::zigir::types::IrStmt::CompileError {
                span: SourceSpan::default(),
                msg: "unsupported binding pattern in variable declaration".to_string(),
            },
        }
    }

    /// Lower a function declaration.
    ///
    /// Translates JS `function foo(a, b) { ... }` into `IrDecl::Fn`.
    /// Handles:
    /// - export / C ABI determination
    /// - async detection (from type_info.is_async)
    /// - throw/catch detection (pre-scan body)
    /// - parameter type resolution
    /// - return type resolution (including AnytypeReturn ¡ú @TypeOf)
    /// - shadow renaming for parameters
    pub(super) fn lower_fn_decl(
        &mut self,
        fd: &Function,
        is_export: bool,
    ) -> Option<crate::zigir::types::IrFnDecl> {
        let name = fd.id.as_ref().map(|id| id.name.as_str())?;

        // Pre-scan: check if function contains throw or try-catch
        let has_throw = fd.body.as_ref().is_some_and(|b| Self::has_throw_in_body(b));

        // Check async from type_info
        let is_async = self.type_info.is_async.get(name).copied().unwrap_or(false);

        // Return type from inference
        let return_type = self
            .type_info
            .fn_return_types
            .get(name)
            .cloned()
            .unwrap_or(ZigType::Void);

        // Enter function context
        let saved = self.enter_fn(name, is_export, Some(return_type.clone()));

        // Push shadow scope for function body (param renames live here)
        self.name_mangler.push_shadow_scope();

        // Lower parameters
        let mut params = self.lower_fn_params(fd, name);

        // Register parameter names in fn_scope_vars (for shadow detection)
        if let Some(ctx) = self.fn_ctx.as_mut() {
            for param in &params {
                ctx.add_scope_var(&param.name.js_name);
            }
        }

        // Lower function body
        let mut body = fd
            .body
            .as_ref()
            .map(|b| self.lower_block(&b.statements))
            .unwrap_or_else(|| IrBlock::new(vec![]));

        // If the body references `arguments` (lowered as `__arguments`), inject
        // `const __arguments = &[_]JsAny{ JsAny.from(param0), JsAny.from(param1), ... }`
        // at the start of the function body.
        let used_idents_pre = Self::collect_ir_idents_in_block(&body);
        if used_idents_pre.contains("__arguments") {
            let args_exprs: Vec<crate::zigir::types::IrExpr> = params
                .iter()
                .filter(|p| p.name.zig_name != "io" && !p.is_rest)
                .map(|p| {
                    crate::zigir::types::IrExpr::Call(crate::zigir::types::IrCallExpr {
                        callee: Box::new(crate::zigir::types::IrExpr::FieldAccess {
                            object: Box::new(crate::zigir::types::IrExpr::Ident(
                                crate::zigir::ident::IrIdent::new("JsAny"),
                            )),
                            field: "from".to_string(),
                            field_kind: crate::zigir::kinds::FieldKind::StructField,
                        }),
                        args: vec![crate::zigir::types::IrExpr::Ident(p.name.clone())],
                        call_kind: crate::zigir::kinds::CallKind::Direct,
                    })
                })
                .collect();
            let arguments_init =
                crate::zigir::types::IrStmt::VarDecl(crate::zigir::types::IrVarDecl {
                    name: crate::zigir::ident::IrIdent::new("__arguments"),
                    is_const: true,
                    zig_type: None,
                    init: Some(crate::zigir::types::IrExpr::ArrayLiteral(
                        crate::zigir::types::IrArrayLiteral {
                            elements: args_exprs,
                            spread_indices: vec![],
                        },
                    )),
                    is_json_parse: false,
                    needs_var_suppression: false,
                });
            body.stmts.insert(0, arguments_init);
        }

        // Mark unused parameters: collect all identifier references in the body,
        // then check which params don't appear. Also include identifiers from
        // compile-time-resolved expressions (e.g., typeof x → "number") that
        // were optimized away but still semantically reference the parameter.
        let mut used_idents = Self::collect_ir_idents_in_block(&body);
        if let Some(ctx) = self.fn_ctx.as_ref() {
            used_idents.extend(ctx.compile_time_referenced_idents.iter().cloned());
        }
        // For async functions, `io` is always used by await/async emission
        if is_async {
            used_idents.insert("io".to_string());
        }
        for param in &mut params {
            if !used_idents.contains(&param.name.js_name) {
                param.is_unused = true;
            }
        }

        // Read has_bigint_div BEFORE exiting the function context, since
        // exit_fn() takes the current fn_ctx and restores the outer one.
        let has_bigint_div = self.fn_ctx.as_ref().is_some_and(|ctx| ctx.has_bigint_div);

        // Exit function context and shadow scope
        self.name_mangler.pop_shadow_scope();
        let _fn_ctx = self.exit_fn(saved);

        // Determine C ABI: export functions use `export fn` calling convention
        let is_cabi = is_export;

        // For AnytypeReturn: capture the first return expression so the Emitter
        // can emit `@TypeOf(expr)` instead of the literal `anytype` keyword.
        let typeof_return_body = if matches!(return_type, ZigType::AnytypeReturn) {
            Self::find_first_return_expr_in_block(&body).map(|e| Box::new(e.clone()))
        } else {
            None
        };

        Some(crate::zigir::types::IrFnDecl {
            name: self.make_ident(name),
            params,
            return_type,
            typeof_return_body,
            body,
            is_export,
            is_async,
            can_throw: has_throw || has_bigint_div,
            is_cabi,
        })
    }

    /// Lower a nested function declaration into struct+call() pattern.
    ///
    /// Zig doesn't allow nested function declarations with return statements,
    /// so we emit `const name = struct { pub fn call(...) ... };` and rewrite
    /// call sites to `name.call(args)`.
    ///
    /// For functions that capture variables from the enclosing scope:
    /// `const _name_type = struct { x: i64, pub fn call(self: *@This(), ...) ... };`
    /// `const name = _name_type{ .x = x };`
    pub(super) fn lower_nested_fn_decl(&mut self, fd: &Function) -> crate::zigir::types::IrStmt {
        use crate::zigir::types::{IrCapture, IrClosure, IrClosureStruct, IrStmt};

        let fn_name = fd
            .id
            .as_ref()
            .map(|id| id.name.as_str())
            .unwrap_or("_anon_fn");

        // Detect captures from enclosing scope
        let captures = self.detect_fn_body_captures(fd);

        // Enter a sub-function context to lower the body
        let return_type = self
            .type_info
            .fn_return_types
            .get(fn_name)
            .cloned()
            .unwrap_or(ZigType::Void);

        let saved = self.enter_fn(fn_name, false, Some(return_type.clone()));
        self.name_mangler.push_shadow_scope();

        let params = self.lower_fn_params(fd, fn_name);

        // Register param names in fn_scope_vars
        if let Some(ctx) = self.fn_ctx.as_mut() {
            for param in &params {
                ctx.add_scope_var(&param.name.js_name);
            }
        }

        let body = fd
            .body
            .as_ref()
            .map(|b| self.lower_block(&b.statements))
            .unwrap_or_else(|| IrBlock::new(vec![]));

        self.name_mangler.pop_shadow_scope();
        let _fn_ctx = self.exit_fn(saved);

        // For AnytypeReturn: capture the first return expression so the Emitter
        // can emit `@TypeOf(expr)` instead of `anytype`.
        let typeof_return_body = if matches!(return_type, ZigType::AnytypeReturn) {
            Self::find_first_return_expr_in_block(&body).map(|e| Box::new(e.clone()))
        } else {
            None
        };

        // Register as a nested function name (for call-site rewriting)
        if let Some(ctx) = self.fn_ctx.as_mut() {
            ctx.add_nested_fn(fn_name);
        }

        let fn_ident = self.make_ident(fn_name);

        if !captures.is_empty() {
            // ©¤©¤ Has captures: named type struct + instance ©¤©¤
            // Generates:
            //   const _inner_type = struct { x: i64, pub fn call(self: *@This(), y: anytype) ... { ... } };
            //   const inner = _inner_type{ .x = x };
            let type_name = format!("_{}_type", fn_ident.zig_name);
            let type_ident = IrIdent::with_zig_name(fn_name, type_name.clone());
            let instance_ident = fn_ident;

            let ir_captures: Vec<IrCapture> = captures
                .into_iter()
                .map(|(name, zig_type, is_mut)| IrCapture {
                    name: self.make_ident(&name),
                    zig_type,
                    is_mut,
                })
                .collect();

            // Create the struct definition (emitted inline in function body)
            let closure_struct = IrClosureStruct {
                name: type_ident.clone(),
                captured: ir_captures.clone(),
                fn_params: params,
                return_type: return_type.clone(),
                typeof_return_body: typeof_return_body.clone(),
                body,
            };

            // Create the instance: TypeName { .x = x, ... }
            let instance = IrClosure {
                struct_name: type_ident,
                captured: ir_captures,
                fn_params: vec![], // already in struct def
                return_type: return_type.clone(),
                body: IrBlock::new(vec![]), // body already in struct def
                instance_name: instance_ident,
            };

            IrStmt::NestedFnDecl {
                struct_def: closure_struct,
                instance: Some(instance),
            }
        } else {
            // ©¤©¤ No captures: inline struct with static call method ©¤©¤
            // Generates:
            //   const inner = struct { pub fn call(y: anytype) ... { ... } };
            let closure_struct = IrClosureStruct {
                name: fn_ident,
                captured: vec![],
                fn_params: params,
                return_type,
                typeof_return_body,
                body,
            };

            IrStmt::NestedFnDecl {
                struct_def: closure_struct,
                instance: None,
            }
        }
    }

    /// Lower function parameters into IrParam list.
    ///
    /// Reads parameter types from type_info when available, falls back to
    /// anytype for untyped parameters.
    pub(super) fn lower_fn_params(&mut self, fd: &Function, fn_name: &str) -> Vec<IrParam> {
        let mut params = Vec::new();
        let is_async = self
            .type_info
            .is_async
            .get(fn_name)
            .copied()
            .unwrap_or(false);

        // Async functions get `io: anytype` as the first parameter
        if is_async {
            params.push(IrParam {
                name: self.make_ident("io"),
                zig_type: ZigType::Anytype,
                is_unused: false,
                is_rest: false,
            });
        }

        // Try to get param types from type_info
        let param_types = self.type_info.fn_param_types.get(fn_name).cloned();

        if let Some(ptypes) = param_types {
            for (pname, ptype) in &ptypes {
                // Skip 'io' param for async functions (it's already injected above)
                if is_async && pname == "io" {
                    continue;
                }
                params.push(IrParam {
                    name: self.make_ident(pname),
                    zig_type: ptype.clone(),
                    is_unused: false, // set later in lower_fn_decl
                    is_rest: false,
                });
            }
        } else {
            // Fallback: generate params from AST with anytype
            for param in &fd.params.items {
                if let Some(pname) = crate::infer::binding_name(&param.pattern) {
                    params.push(IrParam {
                        name: self.make_ident(pname),
                        zig_type: ZigType::Anytype,
                        is_unused: false, // set later in lower_fn_decl
                        is_rest: false,
                    });
                }
            }
        }

        // Handle rest parameter (...args) ¡ú []const JsAny
        if let Some(rname) = fd
            .params
            .rest
            .as_ref()
            .and_then(|r| crate::infer::binding_name(&r.rest.argument))
        {
            params.push(IrParam {
                name: self.make_ident(rname),
                zig_type: ZigType::Anytype, // Will be rendered as []const JsAny by Emitter
                is_unused: false,           // set later in lower_fn_decl
                is_rest: true,
            });
        }

        params
    }

    /// Pre-scan: check if a function body contains `throw` or `try-catch`.
    ///
    /// This is needed to determine whether the return type should be an
    /// error union (`!T` vs `T`).
    pub(super) fn has_throw_in_body(body: &FunctionBody) -> bool {
        body.statements.iter().any(|s| Self::stmt_has_throw(s))
    }

    pub(super) fn stmt_has_throw(stmt: &Statement) -> bool {
        match stmt {
            Statement::ThrowStatement(_) => true,
            Statement::BlockStatement(bs) => bs.body.iter().any(|s| Self::stmt_has_throw(s)),
            Statement::IfStatement(is) => {
                Self::stmt_has_throw(&is.consequent)
                    || is
                        .alternate
                        .as_ref()
                        .is_some_and(|a| Self::stmt_has_throw(a))
            }
            Statement::WhileStatement(ws) => Self::stmt_has_throw(&ws.body),
            Statement::DoWhileStatement(dws) => Self::stmt_has_throw(&dws.body),
            Statement::ForStatement(fs) => Self::stmt_has_throw(&fs.body),
            Statement::ForOfStatement(fos) => Self::stmt_has_throw(&fos.body),
            Statement::ForInStatement(fis) => Self::stmt_has_throw(&fis.body),
            Statement::TryStatement(_) => true, // try-catch implies potential throw
            Statement::SwitchStatement(ss) => ss
                .cases
                .iter()
                .any(|c| c.consequent.iter().any(|s| Self::stmt_has_throw(s))),
            _ => false,
        }
    }
}
