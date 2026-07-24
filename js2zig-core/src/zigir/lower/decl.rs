// zigir/lower/decl.rs
// Declaration lowering: variables, functions, parameters, nested functions.

use oxc_ast::ast::*;

use crate::types::ZigType;
use crate::zigir::ident::IrIdent;
use crate::zigir::source_span::SourceSpan;
use crate::zigir::types::{IrBlock, IrDecl, IrParam, IrVarDecl};

use super::Lowerer;
use super::cabi::{init_may_have_side_effects, property_key_name};
use super::helpers::FnContext;

/// Extract the binding name and whether it has a default value from a
/// `BindingPattern`. Returns `None` for patterns we don't handle
/// (e.g. nested destructuring).
fn binding_name_and_default<'a>(pattern: &'a BindingPattern<'a>) -> Option<(&'a str, bool)> {
    match pattern {
        BindingPattern::BindingIdentifier(id) => Some((id.name.as_str(), false)),
        BindingPattern::AssignmentPattern(ap) => {
            let name = crate::infer::binding_name(&ap.left)?;
            Some((name, true))
        }
        _ => None,
    }
}

/// Intermediate result of entering a function context and lowering
/// its parameters and body. Callers can inspect/modify the params
/// and body while still inside the fn context, then call
/// [`Lowerer::exit_fn_body`] to finalize.
struct FnBodyScope {
    saved: Option<FnContext>,
    params: Vec<IrParam>,
    body: IrBlock,
}

impl Lowerer {
    // ── Shared query helpers ─────────────────────────────

    /// Current function name, or `"__toplevel__"` if outside any function.
    /// Used as a key prefix for `type_info.mutated_vars` / `reassigned_vars` lookups.
    fn fn_prefix(&self) -> &str {
        self.fn_ctx
            .as_ref()
            .map(|ctx| ctx.name.as_str())
            .unwrap_or("__toplevel__")
    }

    fn qualified_var_key(&self, var_name: &str) -> String {
        format!("{}::{}", self.fn_prefix(), var_name)
    }

    fn lookup_fn_return_type(&self, name: &str) -> ZigType {
        self.type_info
            .fn_return_types
            .get(name)
            .cloned()
            .unwrap_or(ZigType::Void)
    }

    fn is_var_mutated(&self, var_name: &str) -> bool {
        self.type_info
            .mutated_vars
            .contains(&self.qualified_var_key(var_name))
    }

    fn is_var_reassigned(&self, var_name: &str) -> bool {
        self.type_info
            .reassigned_vars
            .contains(&self.qualified_var_key(var_name))
    }

    /// Check if a toplevel binding should be skipped.
    /// At module level: unused const or non-const (var/let) are skipped.
    fn should_skip_toplevel_binding(&self, bind_name: &str, is_const: bool) -> bool {
        if self.fn_ctx.is_none() && is_const && !self.type_info.used_names.contains(bind_name) {
            return true;
        }
        if self.fn_ctx.is_none() && !is_const {
            return true;
        }
        false
    }

    /// Infer the type of a destructuring init expression (only supports identifiers).
    fn init_expr_type(&self, init_expr: &Expression) -> Option<ZigType> {
        if let Expression::Identifier(id) = init_expr {
            self.type_info.var_types.get(id.name.as_str()).cloned()
        } else {
            None
        }
    }

    /// Compute the source string for destructuring access patterns:
    /// uses the temp variable if needed, otherwise the init identifier name,
    /// or generates a fresh `_js_dest` name.
    fn compute_destructure_source(
        &mut self,
        needs_temp: bool,
        temp_name: &str,
        init_expr: &Expression,
    ) -> String {
        if needs_temp {
            temp_name.to_string()
        } else if let Expression::Identifier(id) = init_expr {
            id.name.to_string()
        } else {
            self.name_mangler.next_name("_js_dest")
        }
    }

    /// When the return type is `AnytypeReturn`, capture the first return
    /// expression from the body so the Emitter can emit `@TypeOf(expr)`.
    fn resolve_typeof_return_body(
        return_type: &ZigType,
        body: &IrBlock,
    ) -> Option<Box<crate::zigir::types::IrExpr>> {
        if matches!(return_type, ZigType::AnytypeReturn) {
            Self::find_first_return_expr_in_block(body).map(|e| Box::new(e.clone()))
        } else {
            None
        }
    }

    // ── Function body lowering pipeline ──────────────────

    /// Phase 1: enter function context and lower params + body.
    /// The caller still holds the active fn context and can do
    /// additional work (e.g., `__arguments` injection, unused param
    /// marking, reading fn_ctx flags).
    fn enter_fn_body(
        &mut self,
        fd: &Function,
        fn_name: &str,
        is_export: bool,
        return_type: &ZigType,
    ) -> FnBodyScope {
        let saved = self.enter_fn(fn_name, is_export, Some(return_type.clone()));
        self.name_mangler.push_shadow_scope();

        let mut params = self.lower_fn_params(fd, fn_name);

        // ── Synthetic rest param for `arguments` support ──
        // Non-export functions using `arguments` (but without explicit rest param)
        // get a synthetic `...__arguments` rest param so `arguments` captures ALL
        // runtime arguments, not just declared params.
        let needs_synthetic_rest = !is_export
            && fd.params.rest.is_none()
            && self
                .type_info
                .functions_needing_synthetic_rest
                .contains(fn_name);

        if needs_synthetic_rest {
            params.push(IrParam {
                name: self.make_ident("__arguments"),
                zig_type: ZigType::Anytype, // Rendered as []const JsAny by Emitter
                is_unused: false,
                is_rest: true,
            });
        }

        // Set rest_param_name in fn_ctx so lower_ident_expr can rewrite
        // `arguments` → rest param name (synthetic `__arguments` or explicit `args`).
        let rest_param_name = if let Some(rname) = fd
            .params
            .rest
            .as_ref()
            .and_then(|r| crate::infer::binding_name(&r.rest.argument))
        {
            Some(rname.to_string())
        } else if needs_synthetic_rest {
            Some("__arguments".to_string())
        } else {
            None
        };
        if let Some(ctx) = self.fn_ctx.as_mut() {
            ctx.rest_param_name = rest_param_name;
        }

        // Register parameter names in fn_scope_vars (for shadow detection)
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

        FnBodyScope {
            saved,
            params,
            body,
        }
    }

    /// Phase 2: finalize function body lowering — run ownership transfer,
    /// exit function context, and compute typeof_return_body.
    fn exit_fn_body(
        &mut self,
        scope: FnBodyScope,
        return_type: &ZigType,
    ) -> (
        Vec<IrParam>,
        IrBlock,
        Option<Box<crate::zigir::types::IrExpr>>,
    ) {
        let FnBodyScope {
            saved,
            params,
            mut body,
        } = scope;

        Self::clear_deinit_for_returned_vars(&mut body);

        self.name_mangler.pop_shadow_scope();
        let _fn_ctx = self.exit_fn(saved);

        let typeof_return_body = Self::resolve_typeof_return_body(return_type, &body);

        (params, body, typeof_return_body)
    }

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
        let is_actually_mutated = self.is_var_mutated(js_name);
        // Check if the variable is *directly* reassigned (x = ..., not x.y = ...).
        // This distinguishes "variable reassignment" from "property mutation".
        let is_directly_reassigned = self.is_var_reassigned(js_name);
        let is_const = !is_actually_mutated;

        // Track JS-const variables that get directly reassigned — these need a
        // runtime TypeError guard because JS throws on const reassignment but
        // Zig uses `var`.  Only direct reassignment (x = ...) triggers this,
        // not property mutation (x.y = ...) which is legal on JS const objects.
        if _vd_is_const
            && is_directly_reassigned
            && let Some(ctx) = self.fn_ctx.as_mut()
        {
            ctx.js_const_reassigned.insert(js_name.to_string());
        }

        // When a JS-const variable is directly reassigned (x = newValue), the
        // reassignment is converted to a throw (TypeError). Since the Zig `var`
        // is never actually assigned after initialization, Zig 0.16 reports
        // "local variable is never mutated".  We keep `var` (in case the value
        // is used later, e.g., console.log after catch) and use needs_var_suppression
        // to add `_ = &x;` to silence the warning.
        let is_js_const_reassigned = _vd_is_const && is_directly_reassigned;

        // Skip unused toplevel constants
        let has_type_annotation = self.jsdoc_data.type_annotations.contains_key(js_name);
        // Don't skip if the init expression would produce a CompileError when
        // lowered — otherwise the user-facing `@compileError` (warning that an
        // unsupported feature was used in source JS) gets silently dropped
        // alongside the unused const, hiding the diagnostic. Tagged templates,
        // `yield`, and `import.meta`/`new.target` always lower to CompileError.
        let init_might_compile_error = decl.init.as_ref().is_some_and(|e| {
            matches!(
                e,
                Expression::TaggedTemplateExpression(_)
                    | Expression::YieldExpression(_)
                    | Expression::MetaProperty(_)
            )
        });
        if self.fn_ctx.is_none()
            && is_const
            && !self.type_info.used_names.contains(js_name)
            && !has_type_annotation
            && !init_might_compile_error
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

        // Force 'var' for Map/Set/ArrayList/BigInt types (mutated via methods or needs deinit).
        // R8-E5/C1: Also force 'var' for class instances (NamedStruct whose name
        // is a registered class). Methods that mutate self take `*@This()`,
        // which requires a mutable receiver. `_ = &x;` (via needs_var_suppression,
        // already covering NamedStruct) silences Zig's "var never mutated" for
        // instances that only call read-only methods.
        let is_const = if let Some(inferred_ty) = self.type_info.var_types.get(js_name) {
            match inferred_ty {
                ZigType::ArrayList(_) => false,
                ZigType::NamedStruct(n) if n == "Map" || n == "Set" || n == "RegExp" => false,
                ZigType::NamedStruct(n) if self.class_names.contains(n) => false,
                ZigType::BigInt => false,
                _ => is_const,
            }
        } else {
            is_const
        };

        // Type from inference
        let zig_type = self.type_info.var_types.get(js_name).cloned();

        // Record local variable type in fn_ctx for per-function scoping.
        // This gives priority to the current function's variable types over
        // global var_types (which can have stale entries from other functions).
        // Use the init expression to infer type when var_types is unreliable.
        let local_type: Option<ZigType> = if let Some(init_expr) = &decl.init {
            self.infer_expr_type(init_expr).or(zig_type.clone())
        } else {
            zig_type.clone()
        };
        if let Some(ty) = local_type {
            self.fn_ctx
                .as_mut()
                .map(|ctx| ctx.fn_local_types.insert(js_name.to_string(), ty));
        }

        // JSON.parse special case
        let is_json_parse = self.type_info.has_json_parse_types.contains(js_name);

        // std.json.parse (is_json_parse var decl) can fail at runtime — mark can_throw
        if is_json_parse && let Some(ctx) = self.fn_ctx.as_mut() {
            ctx.has_catchable_error = true;
        }

        // Needs var suppression (ArrayList/Map/Set method calls need `_= &var;`)
        // Also for JS-const variables whose reassignment is replaced by a throw:
        // the var is never mutated at the Zig level, so Zig 0.16 reports
        // "local variable is never mutated". `_ = &x;` silences this.
        let needs_var_suppression = (!is_const
            && matches!(
                zig_type,
                Some(ZigType::ArrayList(_)) | Some(ZigType::NamedStruct(_))
            ))
            || is_js_const_reassigned;

        // Lower initializer expression
        let init = match decl.init.as_ref() {
            Some(expr) => {
                // Class expression: register the variable name in class_names
                // so that `new VarName()` routes correctly.
                if let Expression::ClassExpression(_) = expr {
                    self.class_names.insert(js_name.to_string());
                    // Set class_expr_var_name so lower_class_decl can look up
                    // field types stored under the variable name by the type inferrer.
                    self.class_expr_var_name = Some(js_name.to_string());
                }
                // Special case: arrow function / closure initializer.
                // Instead of returning IrExpr::ArrowFn as init, we:
                // 1. Register the struct definition in module.closure_structs
                // 2. Return IrExpr::Ident pointing to the struct name
                // This produces the output pattern:
                //   const _arrow_fn_0 = struct { ... };
                //   const double = _arrow_fn_0;
                let ir = self.lower_expr(expr);
                self.class_expr_var_name = None;
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

        // needs_deinit: true for Map/Set/BigInt/ArrayList types and will be checked for class
        // instances by the Emitter using class_needs_deinit. Set to false for
        // types that don't own resources.
        let needs_deinit = matches!(
            zig_type,
            Some(ZigType::NamedStruct(ref n)) if n == "Map" || n == "Set"
        ) || matches!(zig_type, Some(ZigType::BigInt))
            || matches!(zig_type, Some(ZigType::ArrayList(_)));

        IrDecl::Var(IrVarDecl {
            name: ident,
            is_const,
            zig_type,
            init,
            is_json_parse,
            needs_var_suppression,
            needs_deinit,
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
                let init_type = self.init_expr_type(init_expr);

                let struct_field_names: Option<Vec<String>> = match &init_type {
                    Some(ZigType::Struct(fields)) if !fields.is_empty() => {
                        Some(fields.iter().map(|(n, _)| n.clone()).collect())
                    }
                    _ => None,
                };
                let is_struct = struct_field_names.is_some();

                // Decide if we need a temp variable
                let needs_temp = init_may_have_side_effects(init_expr)
                    || op.properties.len() > 1
                    || !matches!(init_expr, Expression::Identifier(_));
                let temp_name = self.name_mangler.next_name("_js_dest");

                // Source for access patterns: temp var name or inline the init expr
                let source = self.compute_destructure_source(needs_temp, &temp_name, init_expr);

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

                    let (bind_name, has_default) = match binding_name_and_default(&prop.value) {
                        Some(pair) => pair,
                        None => continue,
                    };

                    let is_const = !self.is_var_mutated(bind_name);

                    if self.should_skip_toplevel_binding(bind_name, is_const) {
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
                                temp_name.clone()
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
                    temp_name: if needs_temp {
                        Some(temp_name.clone())
                    } else {
                        None
                    },
                    init: init_ir,
                    kind: IrDestructureKind::Object {
                        is_struct,
                        struct_field_names,
                    },
                    bindings,
                })
            }

            BindingPattern::ArrayPattern(ap) => {
                // ── Array destructuring ──
                let init_type = self.init_expr_type(init_expr);
                let is_arraylist = matches!(init_type, Some(ZigType::ArrayList(_)));

                // Decide if we need a temp variable
                let element_count = ap.elements.iter().filter(|e| e.is_some()).count();
                let needs_temp = init_may_have_side_effects(init_expr)
                    || element_count > 1
                    || !matches!(init_expr, Expression::Identifier(_));
                let temp_name = self.name_mangler.next_name("_js_dest");

                let source = self.compute_destructure_source(needs_temp, &temp_name, init_expr);

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

                    let (bind_name, has_default) = match binding_name_and_default(pattern) {
                        Some(pair) => pair,
                        None => continue,
                    };

                    let is_const = !self.is_var_mutated(bind_name);

                    if self.should_skip_toplevel_binding(bind_name, is_const) {
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
                                temp_name.clone()
                            } else {
                                source.clone()
                            },
                            index: rb.index,
                        },
                        default: default_ir,
                    });
                }

                crate::zigir::types::IrStmt::DestructureDecl(IrDestructureDecl {
                    temp_name: if needs_temp {
                        Some(temp_name.clone())
                    } else {
                        None
                    },
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
        let return_type = self.lookup_fn_return_type(name);

        // Enter function context and lower params + body
        let mut scope = self.enter_fn_body(fd, name, is_export, &return_type);

        // If the body references `arguments` (lowered as `__arguments`):
        // - Non-export with synthetic rest: `__arguments` is a rest param (injected
        //   in enter_fn_body), no VarDecl needed.
        // - Non-export with explicit rest (`...args`): body uses `args`, not
        //   `__arguments`, so no VarDecl needed.
        // - Export function (or any function without rest): inject
        //   `const __arguments = &[_]JsAny{ JsAny.from(param0), ... }`
        //   with declared params.
        {
            let used_idents_pre = Self::collect_ir_idents_in_block(&scope.body);
            let has_rest_param = scope.params.iter().any(|p| p.is_rest);
            if used_idents_pre.contains("__arguments") && !has_rest_param {
                let args_exprs: Vec<crate::zigir::types::IrExpr> = scope
                    .params
                    .iter()
                    .filter(|p| p.name.zig_name != "io" && !p.is_rest)
                    .map(|p| {
                        crate::zigir::types::IrExpr::Call(crate::zigir::types::IrCallExpr {
                            callee: Box::new(crate::zigir::types::IrExpr::FieldAccess {
                                object: Box::new(crate::zigir::types::IrExpr::Ident(IrIdent::new(
                                    "JsAny",
                                ))),
                                field: "from".to_string(),
                                field_kind: crate::zigir::kinds::FieldKind::StructField,
                            }),
                            args: vec![crate::zigir::types::IrExpr::Ident(p.name.clone())],
                            call_kind: crate::zigir::kinds::CallKind::Direct,
                        })
                    })
                    .collect();
                let arguments_init = crate::zigir::types::IrStmt::VarDecl(IrVarDecl {
                    name: IrIdent::new("__arguments"),
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
                    needs_deinit: false,
                });
                scope.body.stmts.insert(0, arguments_init);
            }
        }

        // Mark unused parameters: collect all identifier references in the body,
        // then check which params don't appear. Also include identifiers from
        // compile-time-resolved expressions (e.g., typeof x → "number") that
        // were optimized away but still semantically reference the parameter.
        let mut used_idents = Self::collect_ir_idents_in_block(&scope.body);
        if let Some(ctx) = self.fn_ctx.as_ref() {
            used_idents.extend(ctx.compile_time_referenced_idents.iter().cloned());
        }
        // For async functions, `io` is always used by await/async emission
        if is_async {
            used_idents.insert("io".to_string());
        }
        for param in &mut scope.params {
            if !used_idents.contains(&param.name.js_name) {
                param.is_unused = true;
            }
        }

        // Read has_bigint_div, has_catchable_error, and has_js_const_reassign BEFORE
        // exiting the function context, since exit_fn() restores the outer context.
        let has_bigint_div = self.fn_ctx.as_ref().is_some_and(|ctx| ctx.has_bigint_div);
        let has_catchable_error = self
            .fn_ctx
            .as_ref()
            .is_some_and(|ctx| ctx.has_catchable_error);
        let fn_ever_catchable = self
            .fn_ctx
            .as_ref()
            .is_some_and(|ctx| ctx.fn_ever_catchable);
        let has_js_const_reassign = self
            .fn_ctx
            .as_ref()
            .is_some_and(|ctx| !ctx.js_const_reassigned.is_empty());

        // Exit function context and finalize body
        let (params, body, typeof_return_body) = self.exit_fn_body(scope, &return_type);

        // Determine C ABI: export functions use `export fn` calling convention
        let is_cabi = is_export;

        Some(crate::zigir::types::IrFnDecl {
            name: self.make_ident(name),
            params,
            return_type,
            typeof_return_body,
            body,
            is_export,
            is_async,
            can_throw: has_throw
                || has_bigint_div
                || has_catchable_error
                || fn_ever_catchable
                || has_js_const_reassign,
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
        use crate::zigir::types::{IrClosure, IrClosureStruct, IrStmt};

        let fn_name = fd
            .id
            .as_ref()
            .map(|id| id.name.as_str())
            .unwrap_or("_anon_fn");

        // Detect captures from enclosing scope (before entering fn context)
        let captures = self.detect_fn_body_captures(fd);

        // Return type from inference
        let return_type = self.lookup_fn_return_type(fn_name);

        // Enter function context and lower params + body, then exit
        let scope = self.enter_fn_body(fd, fn_name, false, &return_type);
        let (params, body, typeof_return_body) = self.exit_fn_body(scope, &return_type);

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

            let ir_captures = self.make_ir_captures(captures.into_iter().collect());

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

        // Async functions get `io: AsyncIo` as the first parameter
        if is_async {
            params.push(IrParam {
                name: self.make_ident("io"),
                zig_type: ZigType::AsyncIo,
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
        use super::helpers::stmt_has_throw;
        body.statements.iter().any(|s| stmt_has_throw(s))
    }
}
