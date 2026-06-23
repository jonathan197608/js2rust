// native_proto/codegen/stmt.rs
// Statement-level code generation: toplevel, var_decl, fn, if, while, for, switch.

use super::Codegen;
use crate::native_proto::builtins;
use crate::native_proto::{ExportedFunction, ZigType};
use oxc_ast::ast::*;
use std::collections::HashSet;

// ── Variable declarations ────────────────────────────

/// Check if an identifier name is referenced in a list of statements.
/// Uses a simplified AST walk — covers common patterns in catch bodies
/// (console.log(e), return e.message, etc.).
fn stmt_list_references_name(stmts: &[Statement], name: &str) -> bool {
    stmts.iter().any(|s| stmt_references_name(s, name))
}

fn stmt_references_name(stmt: &Statement, name: &str) -> bool {
    match stmt {
        Statement::ExpressionStatement(es) => expr_references_name(&es.expression, name),
        Statement::ReturnStatement(rs) => {
            rs.argument.as_ref().is_some_and(|a| expr_references_name(a, name))
        }
        Statement::VariableDeclaration(vd) => vd.declarations.iter().any(|d| {
            d.init.as_ref().is_some_and(|init| expr_references_name(init, name))
        }),
        Statement::BlockStatement(bs) => stmt_list_references_name(&bs.body, name),
        _ => false,
    }
}

fn expr_references_name(expr: &Expression, name: &str) -> bool {
    match expr {
        Expression::Identifier(id) => id.name.as_str() == name,
        Expression::BinaryExpression(be) => {
            expr_references_name(&be.left, name) || expr_references_name(&be.right, name)
        }
        Expression::CallExpression(ce) => {
            // Callee is Expression, so check it directly
            expr_references_name(&ce.callee, name)
        }
        Expression::StaticMemberExpression(sme) => expr_references_name(&sme.object, name),
        _ => false,
    }
}

impl Codegen {
    /// Emit a variable declaration. Toplevel: only `const` allowed.
    /// Inside functions: `var` with type inference + undefined init.
    pub(crate) fn emit_var_decl(&mut self, vd: &VariableDeclaration) {
        for decl in &vd.declarations {
            if let Some(name) = crate::native_proto::infer::binding_name(&decl.id) {
                // Use Zig 'const' when the variable is never mutated (regardless of JS const/var/let).
                // Only use Zig 'var' when the variable is actually reassigned.
                let fn_prefix = self
                    .current_fn
                    .as_deref()
                    .unwrap_or("__toplevel__");
                let is_const = !self
                    .type_info
                    .mutated_vars
                    .contains(&format!("{}::{}", fn_prefix, name));

                // Skip unused toplevel constants to avoid Zig unused warnings.
                let has_type_annotation = self
                    .jsdoc_data
                    .as_ref()
                    .is_some_and(|d| d.type_annotations.contains_key(name));
                if self.indent == 0
                    && is_const
                    && !self.type_info.used_names.contains(name)
                    && !has_type_annotation
                {
                    continue;
                }
                // Rule: toplevel var/let → error. Only allow const.
                if self.indent == 0 && !is_const {
                    self.write_indent();
                    self.write(&format!(
                        "// error: toplevel only allows 'const', not '{}'",
                        name
                    ));
                    self.writeln("");
                    continue;
                }

                match &decl.init {
                    Some(Expression::ArrowFunctionExpression(arrow)) => {
                        // Generate arrow function or closure
                        let fn_name = self.emit_arrow_function(arrow);
                        // Check if this is a closure (struct name in closure_vars)
                        if self.closure_vars.contains_key(&fn_name) {
                            // Closure: generate instantiation code
                            // Clone the captured vars to avoid borrow conflict
                            let captured = self.closure_vars.get(&fn_name).cloned();
                            if let Some(captured) = captured {
                                self.write_indent();
                                self.write(&format!("const {} = {} {{ ", name, fn_name));
                                // Generate field initializers
                        for (i, (cap_name, _, is_mut)) in captured.iter().enumerate() {
                            if i > 0 { self.write(", "); }
                            if *is_mut {
                                self.write(&format!(".{} = &{}", cap_name, cap_name));
                            } else {
                                self.write(&format!(".{} = {}", cap_name, cap_name));
                            }
                        }
                                self.write(" };\n");
                            }
                            // Mark this variable as a closure instance
                            self.closure_instances.insert(name.to_string());
                        } else {
                            // Plain arrow function: assign function to variable
                            self.write_indent();
                            self.write(&format!("const {} = {};
", name, fn_name));
                        }
                    }
                    Some(init) => {
                        self.write_indent();
                        // Force 'var' for Map/Set/ArrayList types (mutated via methods).
                        let is_const = if let Some(inferred_ty) = self.type_info.var_types.get(name)
                        {
                            match inferred_ty {
                                ZigType::ArrayList(_) => false,
                                ZigType::NamedStruct(n) if n == "Map" || n == "Set" => false,
                                _ => is_const,
                            }
                        } else {
                            is_const
                        };
                        let kw = if is_const { "const" } else { "var" };

                        let is_json_parse = self.type_info.has_json_parse_types.contains(name);

                        if is_json_parse {
                            // JSDoc-annotated JSON.parse: type is NamedStruct
                            let type_name = self
                                .type_info
                                .var_types
                                .get(name)
                                .and_then(|t| match t {
                                    ZigType::NamedStruct(n) => Some(n.as_str()),
                                    _ => None,
                                })
                                .unwrap_or("i64");
                            self.write(&format!(
                                "{} {}: {} = std.json.parse({}, ",
                                kw, name, type_name, type_name
                            ));
                            if let Expression::CallExpression(ce) = init
                                && let Some(first_arg) = ce.arguments.first()
                            {
                                self.emit_expr_arg(first_arg);
                            }
                            self.write(") catch @panic(\"OOM: JSON.parse alloc\");\n");
                        } else if let Some(inferred_ty) = self.type_info.var_types.get(name) {
                            let inferred_ty = inferred_ty.clone();
                            // Definite type from pre-computed type info.
                            // Skip type annotation for NamedStruct and ArrayList — Zig infers from init.
                            let skip_type_annotation = matches!(
                                inferred_ty,
                                ZigType::NamedStruct(_) | ZigType::ArrayList(_)
                            );
                            if is_const || skip_type_annotation {
                                self.write(&format!("{} {} = ", kw, name));
                            } else {
                                self.write(&format!(
                                    "{} {}: {} = ",
                                    kw,
                                    name,
                                    inferred_ty.to_zig_type()
                                ));
                            }
                            self.emit_expr(init);
                            self.write(";\n");
                            if let Some(ta_type) = typedarray_init_type(init) {
                                self.typedarray_vars
                                    .insert(name.to_string(), ta_type.to_string());
                            }
                            // Zig 0.16.0: 'var' for ArrayList/Map/Set needs &var suppression
                            // (method calls like reverse/sort go through .items, not &arr)
                            if !is_const {
                                match inferred_ty {
                                    ZigType::ArrayList(_) => {
                                        self.write_indent();
                                        self.write(&format!("_ = &{}; // var usage\n", name));
                                    }
                                    ZigType::NamedStruct(n) if n == "Map" || n == "Set" => {
                                        self.write_indent();
                                        self.write(&format!("_ = &{}; // var usage\n", name));
                                    }
                                    _ => {}
                                }
                            }
                        } else {
                            // Indeterminate type (Rule 8 error already in type_info.errors)
                            self.write(&format!("{} {} = ", kw, name));
                            self.emit_expr(init);
                            self.write(";\n");
                            if let Some(ta_type) = typedarray_init_type(init) {
                                self.typedarray_vars
                                    .insert(name.to_string(), ta_type.to_string());
                            }
                        }
                    }
                    None => {
                        // No initializer → error.
                        self.write_indent();
                        self.write(&format!(
                            "// error: variable '{}' must be initialized",
                            name
                        ));
                        self.writeln("");
                    }
                }
            }
        }
    }
}

// ── Function declarations ──────────────────────────────

impl Codegen {
    pub(crate) fn has_throw_in_body(body: &FunctionBody) -> bool {
        fn stmt_has_throw(stmt: &Statement) -> bool {
            match stmt {
                Statement::ThrowStatement(_) | Statement::TryStatement(_) => true,
                Statement::LabeledStatement(s) => stmt_or_expr_has_throw(&s.body),
                Statement::IfStatement(s) => {
                    stmt_or_expr_has_throw(&s.consequent)
                        || s.alternate.as_ref().is_some_and(|a| stmt_or_expr_has_throw(a))
                }
                Statement::WhileStatement(s) => stmt_or_expr_has_throw(&s.body),
                Statement::DoWhileStatement(s) => stmt_or_expr_has_throw(&s.body),
                Statement::ForStatement(s) => stmt_or_expr_has_throw(&s.body),
                Statement::ForOfStatement(s) => stmt_or_expr_has_throw(&s.body),
                Statement::ForInStatement(s) => stmt_or_expr_has_throw(&s.body),
                Statement::BlockStatement(s) => s.body.iter().any(stmt_has_throw),
                Statement::SwitchStatement(s) => {
                    s.cases.iter().any(|c| c.consequent.iter().any(stmt_has_throw))
                }
                _ => false,
            }
        }

        fn stmt_or_expr_has_throw(stmt: &Statement) -> bool {
            match stmt {
                Statement::BlockStatement(s) => s.body.iter().any(stmt_has_throw),
                other => stmt_has_throw(other),
            }
        }

        body.statements.iter().any(stmt_has_throw)
    }

    /// Check if a single statement contains a throw (does NOT count try-catch blocks).
    /// Used for pre-scanning try blocks to decide whether to generate catch handler.
    pub(crate) fn stmt_has_throw_any(stmt: &Statement) -> bool {
        match stmt {
            Statement::ThrowStatement(_) => true,
            Statement::LabeledStatement(s) => {
                crate::native_proto::codegen::stmt::Codegen::stmt_has_throw_any_alt(&s.body)
            }
            Statement::IfStatement(s) => {
                crate::native_proto::codegen::stmt::Codegen::stmt_has_throw_any_alt(&s.consequent)
                    || s.alternate.as_ref().is_some_and(|a| {
                        crate::native_proto::codegen::stmt::Codegen::stmt_has_throw_any_alt(a)
                    })
            }
            Statement::WhileStatement(s) => {
                crate::native_proto::codegen::stmt::Codegen::stmt_has_throw_any_alt(&s.body)
            }
            Statement::DoWhileStatement(s) => {
                crate::native_proto::codegen::stmt::Codegen::stmt_has_throw_any_alt(&s.body)
            }
            Statement::ForStatement(s) => {
                crate::native_proto::codegen::stmt::Codegen::stmt_has_throw_any_alt(&s.body)
            }
            Statement::ForOfStatement(s) => {
                crate::native_proto::codegen::stmt::Codegen::stmt_has_throw_any_alt(&s.body)
            }
            Statement::ForInStatement(s) => {
                crate::native_proto::codegen::stmt::Codegen::stmt_has_throw_any_alt(&s.body)
            }
            Statement::BlockStatement(s) => s.body.iter().any(|s1| Self::stmt_has_throw_any(s1)),
            Statement::SwitchStatement(s) => s
                .cases
                .iter()
                .any(|c| c.consequent.iter().any(|s1| Self::stmt_has_throw_any(s1))),
            _ => false,
        }
    }

    fn stmt_has_throw_any_alt(stmt: &Statement) -> bool {
        match stmt {
            Statement::BlockStatement(s) => s.body.iter().any(|s1| Self::stmt_has_throw_any(s1)),
            other => Self::stmt_has_throw_any(other),
        }
    }

    /// Enter a try block. Sets `inside_try_block` so that `throw` statements
    /// inside the block generate `break :label error.JsThrow` instead of
    /// `return error.JsThrow`.
    pub(crate) fn start_try_block(&mut self, label: &str) {
        self.inside_try_block = Some(label.to_string());
    }

    /// Exit a try block. Clears `inside_try_block`.
    pub(crate) fn end_try_block(&mut self) {
        self.inside_try_block = None;
    }

    pub(crate) fn emit_fn(&mut self, fd: &Function) {
        let name = fd
            .id
            .as_ref()
            .map(|id| id.name.as_str())
            .unwrap_or("anonymous");
        let saved_current_fn = std::mem::take(&mut self.current_fn);
        self.current_fn = Some(name.to_string());

        // Check if function contains await (from pre-computed type_info)
        let is_async = self.type_info.is_async.get(name).copied().unwrap_or(false);

        // Pre-scan: check if function contains throw or try-catch.
        // This must happen BEFORE generating the return signature (need !T for throw).
        let has_throw = fd.body.as_ref().is_some_and(|b| Codegen::has_throw_in_body(b));
        self.fn_has_throw = has_throw;

        // Read pre-computed return type from TypeInferResult.
        let ret_ty = self.type_info.fn_return_types.get(name).cloned();
        self.current_fn_return_type = ret_ty.clone();

        // Build set of identifiers used in THIS function body (per-function).
        let mut fn_used_names = HashSet::new();
        if let Some(body) = &fd.body {
            for s in &body.statements {
                Self::collect_stmt_idents(s, &mut fn_used_names);
            }
        }

        // Generate function signature.
        if is_async {
            self.write(&format!("pub fn {}(io: anytype", name));
        } else {
            self.write(&format!("pub fn {}(", name));
        }

        // Generate parameter list (read param types from type_info).
        let param_list = self.type_info.fn_param_types.get(name).cloned();
        if let Some(params) = param_list {
            let mut param_idx = 0;
            for (pname, ptype) in &params {
                if is_async && pname == "io" {
                    continue;
                }
                if param_idx > 0 || is_async {
                    self.write(", ");
                }
                // Zig 0.16.0: unused params are compile errors. Prefix with _ if unused
                // in THIS function body (per-function tracking).
                let zig_pname = if fn_used_names.contains(pname) {
                    pname.as_str()
                } else {
                    self.write("_");
                    pname.as_str()
                };
                // Export function params: always typed; non-export: use anytype
                if self.current_fn_is_export {
                    self.write(&format!("{}: {}", zig_pname, ptype.to_zig_type()));
                } else {
                    // Non-export params are inferred as anytype in the type info
                    self.write(&format!("{}: {}", zig_pname, ptype.to_zig_type()));
                }
                param_idx += 1;
            }
            // Handle rest parameter (...args) from type_info or AST
            if let Some(rest_name) = fd.params.rest.as_ref().map(|r| {
                crate::native_proto::infer::binding_name(&r.rest.argument)
            }) {
                if let Some(rname) = rest_name {
                    if param_idx > 0 || is_async {
                        self.write(", ");
                    }
                    let zig_pname = if fn_used_names.contains(rname) {
                        rname
                    } else {
                        self.write("_");
                        rname
                    };
                    // Rest parameter: accepts []const JsAny
                    self.write(&format!("{}: []const JsAny", zig_pname));
                }
            }
        } else {
            // Fallback: generate params from AST with anytype
            let mut param_idx = 0;
            for param in &fd.params.items {
                if let Some(pname) = crate::native_proto::infer::binding_name(&param.pattern) {
                    if is_async && pname == "io" {
                        continue;
                    }
                    if param_idx > 0 || is_async {
                        self.write(", ");
                    }
                    // Zig 0.16.0: unused params are compile errors.
                    let zig_pname = if fn_used_names.contains(pname) {
                        pname
                    } else {
                        self.write("_");
                        pname
                    };
                    self.write(&format!("{}: anytype", zig_pname));
                    param_idx += 1;
                }
            }
            // Handle rest parameter (...args) in fallback mode
            if let Some(rest_name) = fd.params.rest.as_ref().map(|r| {
                crate::native_proto::infer::binding_name(&r.rest.argument)
            }) {
                if let Some(rname) = rest_name {
                    if param_idx > 0 || is_async {
                        self.write(", ");
                    }
                    let zig_pname = if fn_used_names.contains(rname) {
                        rname
                    } else {
                        self.write("_");
                        rname
                    };
                    self.write(&format!("{}: []const JsAny", zig_pname));
                }
            }
        }

        // Return type — async + throw functions return error unions
        let ret_zig_type = match &self.current_fn_return_type {
            Some(ZigType::I64) => "i64".to_string(),
            Some(ZigType::F64) => "f64".to_string(),
            Some(ZigType::Bool) => "bool".to_string(),
            Some(ZigType::Str) => "[]const u8".to_string(),
            Some(ZigType::Void) => "void".to_string(),
            None => "void".to_string(),
            Some(other) => other.to_zig_type(),
        };
        let ret_zig_type = if (is_async || self.fn_has_throw) && ret_zig_type != "void" {
            format!("!{}", ret_zig_type)
        } else if self.fn_has_throw && ret_zig_type == "void" {
            "!void".to_string()
        } else {
            ret_zig_type
        };
        self.writeln(&format!(") {} {{", ret_zig_type));

        self.indent += 1;
        self.seen_return = false;

        // Suppress "unused parameter" errors for params not used in body
        // (fetch params again to determine which are unused)
        let unused_params: Vec<String> =
            if let Some(param_list) = self.type_info.fn_param_types.get(name) {
                param_list
                    .iter()
                    .filter(|(pn, _)| !fn_used_names.contains(pn.as_str()))
                    .map(|(pn, _)| pn.clone())
                    .collect()
            } else {
                fd.params
                    .items
                    .iter()
                    .filter_map(|p| crate::native_proto::infer::binding_name(&p.pattern))
                    .filter(|pn| !fn_used_names.contains(*pn))
                    .map(|pn| pn.to_string())
                    .collect()
            };
        for pname in &unused_params {
            self.write_indent();
            self.write(&format!("_ = _{};\n", pname));
        }

        if let Some(body) = &fd.body {
            for stmt in &body.statements {
                self.emit_fn_stmt(stmt);
            }
        }
        // If function has non-void return type but no explicit return,
        // add a default return 0 to avoid Zig compile error.
        if !self.seen_return && ret_zig_type != "void" {
            self.write_indent();
            self.write("return 0;\n");
        }
        self.indent -= 1;
        self.writeln("}");

        // If this is an export function, add to exported_fns for C ABI wrapper generation.
        if self.current_fn_is_export {
            let func_name = name.to_string();
            let return_type = self.current_fn_return_type.clone().unwrap_or(ZigType::I64);

            // Get parameter types from type_info.
            let params: Vec<ZigType> = self
                .type_info
                .fn_param_types
                .get(name)
                .map(|p| {
                    p.iter()
                        .filter(|(n, _)| !is_async || n != "io")
                        .map(|(_, t)| t.clone())
                        .collect()
                })
                .unwrap_or_default();

            self.exported_fns.push(ExportedFunction {
                name: func_name,
                params,
                return_type,
                can_throw: self.fn_has_throw,
            });
        }
        self.current_fn = saved_current_fn;
    }
}

// ── Function body statements ─────────────────────────

impl Codegen {
    pub(crate) fn emit_fn_stmt(&mut self, stmt: &Statement) {
        match stmt {
            Statement::VariableDeclaration(vd) => {
                self.emit_var_decl(vd);
            }
            Statement::ReturnStatement(rs) => {
                self.write_indent();
                if let Some(arg) = &rs.argument {
                    self.write("return ");
                    self.in_return_expr = true;
                    self.emit_expr(arg);
                    self.in_return_expr = false;
                    self.write(";\n");
                } else {
                    self.write("return;\n");
                }
                self.seen_return = true;
            }
            Statement::ExpressionStatement(es) => {
                // Special handling for forEach/some/every: they generate 'for' loops (statements), not expressions.
                // If we add ';' after a 'for' loop, Zig will report a syntax error.
                let mut need_semi = true;
                if let Expression::CallExpression(ce) = &es.expression
                    && let Some(builtin) = builtins::detect_builtin_call(ce)
                {
                    match builtin {
                        builtins::BuiltinCall::ArrayForEach
                        | builtins::BuiltinCall::ArraySome
                        | builtins::BuiltinCall::ArrayEvery => {
                            // These generate 'for' loops (statements), no ';' needed
                            need_semi = false;
                        }
                        _ => {}
                    }
                }

                self.write_indent();
                self.in_expr_stmt = true;
                self.emit_expr(&es.expression);
                self.in_expr_stmt = false;
                if need_semi {
                    self.write(";\n");
                } else {
                    self.write("\n");
                }
            }
            Statement::IfStatement(is) => {
                self.emit_if(is);
            }
            Statement::WhileStatement(ws) => {
                self.emit_while(ws);
            }
            Statement::DoWhileStatement(dws) => {
                self.emit_do_while(dws);
            }
            Statement::ForStatement(fs) => {
                self.emit_for(fs);
            }
            Statement::ForOfStatement(fos) => {
                self.emit_for_of(fos);
            }
            Statement::ForInStatement(fis) => {
                self.emit_for_in(fis);
            }
            Statement::SwitchStatement(ss) => {
                self.emit_switch(ss);
            }
            Statement::BreakStatement(bs) => {
                self.write_indent();
                if let Some(ref label) = bs.label {
                    self.write(&format!("break :{};\n", label.name));
                } else {
                    self.write("break;\n");
                }
            }
            Statement::ContinueStatement(cs) => {
                self.write_indent();
                if let Some(ref label) = cs.label {
                    self.write(&format!("continue :{};\n", label.name));
                } else {
                    self.write("continue;\n");
                }
            }
            Statement::LabeledStatement(ls) => {
                // labeled statement → Zig labeled block or loop
                let label_name = ls.label.name.as_str();
                match &ls.body {
                    // For loops: label attaches directly to the loop syntax
                    Statement::WhileStatement(_) => {
                        self.write_indent();
                        self.write(&format!("{}: ", label_name));
                        self.emit_while_labeled(match &ls.body {
                            Statement::WhileStatement(ws) => ws,
                            _ => unreachable!(),
                        });
                    }
                    Statement::ForStatement(_) => {
                        self.write_indent();
                        self.write(&format!("{}: ", label_name));
                        self.emit_for_labeled(match &ls.body {
                            Statement::ForStatement(fs) => fs,
                            _ => unreachable!(),
                        });
                    }
                    Statement::ForOfStatement(_) => {
                        self.write_indent();
                        self.write(&format!("{}: ", label_name));
                        self.emit_for_of_labeled(match &ls.body {
                            Statement::ForOfStatement(fos) => fos,
                            _ => unreachable!(),
                        });
                    }
                    Statement::ForInStatement(_) => {
                        self.write_indent();
                        self.write(&format!("{}: ", label_name));
                        self.emit_for_in_labeled(match &ls.body {
                            Statement::ForInStatement(fis) => fis,
                            _ => unreachable!(),
                        });
                    }
                    Statement::DoWhileStatement(_) => {
                        self.write_indent();
                        self.write(&format!("{}: ", label_name));
                        self.emit_do_while_labeled(match &ls.body {
                            Statement::DoWhileStatement(dws) => dws,
                            _ => unreachable!(),
                        });
                    }
                    _ => {
                        // Generic labeled block (for if/switch/block etc)
                        self.write_indent();
                        self.writeln(&format!("{}: {{", label_name));
                        self.indent += 1;
                        self.emit_fn_stmt(&ls.body);
                        self.indent -= 1;
                        self.write_indent();
                        self.writeln("}");
                    }
                }
            }
            Statement::BlockStatement(bs) => {
                self.emit_block(bs);
            }
            Statement::ThrowStatement(ts) => {
                // JS throw expr → Zig: evaluate expr for side effects, then propagate error.
                // If inside a try block, use `break :label error.JsThrow` so the catch handler
                // catches it. Otherwise, `return error.JsThrow`.
                self.write_indent();
                self.write("_ = ");
                self.in_return_expr = true;
                self.emit_expr(&ts.argument);
                self.in_return_expr = false;
                self.write(";\n");

                if let Some(ref label) = self.inside_try_block.clone() {
                    // Inside try-catch: break the try block with error
                    self.write_indent();
                    self.writeln(&format!("break :{} error.JsThrow;", label));
                } else {
                    // Bare throw: propagate to function return
                    self.write_indent();
                    self.writeln("return error.JsThrow;");
                }
                self.seen_return = true;
            }
            Statement::TryStatement(ts) => {
                // JS try { ... } catch(e) { ... } finally { ... }
                //
                // Two code paths:
                //   A) Try body has throw → labeled block + catch handler (real semantics)
                //   B) Try body has no throw → emit body inline, skip catch (unreachable)

                // Pre-scan: does the try body contain throw statements?
                let has_throw = ts.block.body.iter().any(|s| Self::stmt_has_throw_any(s));

                if !has_throw && ts.handler.is_none() {
                    // Case B1 (finally only, no throw, no catch):
                    // emit body, then emit finally inline after (not defer).
                    for stmt in &ts.block.body {
                        self.emit_fn_stmt(stmt);
                    }
                    if let Some(ref finalizer) = ts.finalizer {
                        for stmt in &finalizer.body {
                            self.emit_fn_stmt(stmt);
                        }
                    }
                    return; // caller continues
                }

                if !has_throw {
                    // Case B2 (with catch handler, no throw):
                    // emit body, emit finally inline after, handler unreachable.
                    for stmt in &ts.block.body {
                        self.emit_fn_stmt(stmt);
                    }
                    if let Some(ref finalizer) = ts.finalizer {
                        for stmt in &finalizer.body {
                            self.emit_fn_stmt(stmt);
                        }
                    }
                    return; // caller continues
                }

                // Case A: try body has throw → full labeled block + catch handler

                let label_id = self.try_label_counter;
                self.try_label_counter += 1;
                let blk_label = format!("_js_try_blk_{}", label_id);
                let result_var = format!("_js_try_{}", label_id);

                // ── Try body wrapped in labeled block ──
                self.start_try_block(&blk_label);
                self.write_indent();
                self.writeln(&format!(
                    "const {}: anyerror!void = {blk}: {{",
                    result_var,
                    blk = blk_label,
                ));
                self.indent += 1;

                // Emit try body — throw statements will use break :blk_label
                // Track whether body exited early (return/throw) to skip normal completion break.
                let seen_before = self.seen_return;
                self.seen_return = false;
                for stmt in &ts.block.body {
                    self.emit_fn_stmt(stmt);
                }
                let body_exited = self.seen_return;
                self.seen_return = seen_before;

                // Normal completion: break with void (only if body didn't exit early)
                if !body_exited {
                    self.write_indent();
                    self.writeln(&format!("break :{blk} {{}};", blk = blk_label));
                }

                self.indent -= 1;
                self.write_indent();
                self.writeln("};");

                // ── Catch handler ──
                if let Some(ref handler) = ts.handler {
                    self.write_indent();
                    self.write(&format!(
                        "_ = {} catch |err| {{\n",
                        result_var
                    ));
                    self.indent += 1;

                    // Bind catch parameter: JS `catch(e)` → map `e` to `err` in Zig.
                    // If `e` is referenced in the catch body, emit a named const.
                    // Otherwise, emit a discard to suppress Zig's unused warning.
                    if let Some(ref param) = handler.param
                        && let BindingPattern::BindingIdentifier(ref id) = param.pattern
                    {
                        let name = id.name.as_str();
                        let is_referenced = stmt_list_references_name(&handler.body.body, name);
                        self.write_indent();
                        if is_referenced {
                            // `@errorName(err)` gives the error tag name (e.g. "JsThrow")
                            self.writeln(&format!("const {} = @errorName(err);", name));
                        } else {
                            self.writeln("_ = @errorName(err);");
                        }
                    }

                    for stmt in &handler.body.body {
                        self.emit_fn_stmt(stmt);
                    }

                    self.indent -= 1;
                    self.write_indent();
                    self.writeln("};");
                } else {
                    // No handler: discard the result (error still propagates naturally
                    // for throw-only cases, and for cleanup-only the void is consumed)
                    self.write_indent();
                    self.writeln(&format!("_ = {};", result_var));
                }

                self.end_try_block();

                // ── Emit finally body inline (after try-catch, before subsequent code) ──
                if let Some(ref finalizer) = ts.finalizer {
                    for stmt in &finalizer.body {
                        self.emit_fn_stmt(stmt);
                    }
                }
            }
            _ => {
                // Unsupported statement type: generate @compileError
                self.write_indent();
                self.write("@compileError(\"Unsupported statement type\")");
            }
        }
    }
}

// ── If / Else ──────────────────────────────────────

impl Codegen {
    pub(crate) fn emit_if(&mut self, is: &IfStatement) {
        self.write_indent();
        self.write("if (");
        self.emit_expr(&is.test);
        self.write(") {\n");

        self.indent += 1;
        self.emit_stmt_or_block(&is.consequent);
        self.indent -= 1;

        if let Some(alt) = &is.alternate {
            let inner: &Statement = alt;
            match inner {
                Statement::IfStatement(else_if) => {
                    self.write_indent();
                    self.write("} else ");
                    self.emit_if(else_if);
                    return;
                }
                other => {
                    self.writeln("} else {");
                    self.indent += 1;
                    self.emit_stmt_or_block(other);
                    self.indent -= 1;
                }
            }
        }
        self.writeln("}");
    }

    fn emit_stmt_or_block(&mut self, stmt: &Statement) {
        match stmt {
            Statement::BlockStatement(bs) => {
                for s in &bs.body {
                    self.emit_fn_stmt(s);
                }
            }
            _ => self.emit_fn_stmt(stmt),
        }
    }

    fn emit_block(&mut self, bs: &BlockStatement) {
        self.writeln("{");
        self.indent += 1;
        for stmt in &bs.body {
            self.emit_fn_stmt(stmt);
        }
        self.indent -= 1;
        self.writeln("}");
    }
}

// ── While / Do-While / For-Of / Switch ───────────

impl Codegen {
    pub(crate) fn emit_while(&mut self, ws: &WhileStatement) {
        self.write_indent();
        self.write("while (");
        self.emit_expr(&ws.test);
        self.write(") {\n");
        self.indent += 1;
        self.emit_stmt_or_block(&ws.body);
        self.indent -= 1;
        self.writeln("}");
    }

    fn emit_while_labeled(&mut self, ws: &WhileStatement) {
        // Label already written by LabeledStatement handler, no indent needed
        self.write("while (");
        self.emit_expr(&ws.test);
        self.write(") {\n");
        self.indent += 1;
        self.emit_stmt_or_block(&ws.body);
        self.indent -= 1;
        self.writeln("}");
    }

    // JS:  do { ... } while (cond);
    // Zig: while (true) { ...; if (cond) {} else { break; } }
    fn emit_do_while(&mut self, dws: &DoWhileStatement) {
        self.write_indent();
        self.writeln("while (true) {");

        self.indent += 1;
        self.emit_stmt_or_block(&dws.body);
        self.write_indent();
        self.write("if (");
        self.emit_expr(&dws.test);
        self.write(") {} else { break; }\n");

        self.indent -= 1;

        self.writeln("}");
    }

    fn emit_do_while_labeled(&mut self, dws: &DoWhileStatement) {
        self.writeln("while (true) {");
        self.indent += 1;
        self.emit_stmt_or_block(&dws.body);
        self.write_indent();
        self.write("if (");
        self.emit_expr(&dws.test);
        self.write(") {} else { break; }\n");
        self.indent -= 1;
        self.writeln("}");
    }

    // JS:  for (init; test; update) { ... }
    // Zig: { init; while (test) : (update) { ... } }
    fn emit_for(&mut self, fs: &ForStatement) {
        self.write_indent();
        self.emit_for_body(fs);
    }

    fn emit_for_labeled(&mut self, fs: &ForStatement) {
        self.emit_for_body(fs);
    }

    fn emit_for_body(&mut self, fs: &ForStatement) {
        self.write("{\n");
        self.indent += 1;

        // init
        if let Some(init) = &fs.init {
            if let ForStatementInit::VariableDeclaration(vd) = init {
                for decl in &vd.declarations {
                    if let Some(name) = crate::native_proto::infer::binding_name(&decl.id) {
                        self.write_indent();
                        self.write(&format!("var {}: i64 = 0;\n", name));
                    }
                }
            } else {
                // Expression init: emit as statement
                if let Some(expr) = init.as_expression() {
                    self.write_indent();
                    self.emit_expr(expr);
                    self.write(";\n");
                }
            }
        }

        // test — generate a while loop
        self.write_indent();
        self.write("while (");
        if let Some(test) = &fs.test {
            self.emit_expr(test);
        } else {
            self.write("true");
        }
        self.write(")");

        // update
        if let Some(update) = &fs.update {
            self.write(" : (");
            self.emit_expr(update);
            self.write(")");
        }

        self.write(" {\n");
        self.indent += 1;
        self.emit_stmt_or_block(&fs.body);
        self.indent -= 1;

        self.write_indent();
        self.write("}\n");

        // Close the block
        self.indent -= 1;
        self.write_indent();
        self.write("}\n");
    }

    // JS:  for (const x of iterable) { ... }
    // Zig: for (iterable) |x| { ... }
    fn emit_for_of(&mut self, fos: &ForOfStatement) {
        let var_name = match &fos.left {
            ForStatementLeft::VariableDeclaration(vd) => vd
                .declarations
                .first()
                .and_then(|decl| self.binding_name(&decl.id))
                .unwrap_or("item")
                .to_string(),
            _ => "item".to_string(),
        };

        // Check if the iterable is an ArrayList variable
        let iterable_is_arraylist = match &fos.right {
            Expression::Identifier(id) => self
                .type_info
                .var_types
                .get(id.name.as_str())
                .map(|t| matches!(t, ZigType::ArrayList(_)))
                .unwrap_or(false),
            _ => false,
        };

        self.write_indent();
        self.write("for (");
        self.emit_expr(&fos.right);
        if iterable_is_arraylist {
            self.write(".items");
        }
        self.write(&format!(") |{}| {{\n", var_name));

        self.indent += 1;
        self.emit_stmt_or_block(&fos.body);
        self.indent -= 1;

        self.writeln("}");
    }

    fn emit_for_of_labeled(&mut self, fos: &ForOfStatement) {
        let var_name = match &fos.left {
            ForStatementLeft::VariableDeclaration(vd) => vd
                .declarations
                .first()
                .and_then(|decl| self.binding_name(&decl.id))
                .unwrap_or("item")
                .to_string(),
            _ => "item".to_string(),
        };
        let iterable_is_arraylist = match &fos.right {
            Expression::Identifier(id) => self
                .type_info
                .var_types
                .get(id.name.as_str())
                .map(|t| matches!(t, ZigType::ArrayList(_)))
                .unwrap_or(false),
            _ => false,
        };
        self.write("for (");
        self.emit_expr(&fos.right);
        if iterable_is_arraylist { self.write(".items"); }
        self.write(&format!(") |{}| {{\n", var_name));
        self.indent += 1;
        self.emit_stmt_or_block(&fos.body);
        self.indent -= 1;
        self.writeln("}");
    }

    /// JS: for (var key in obj) { ... }
    /// Zig:
    ///   - HashMap: var it = obj.iterator(); while (it.next()) |kv| { const key = kv.key_ptr.*; ... }
    ///   - Static struct: unroll loop — one block per field with const key = "fieldName"
    fn emit_for_in(&mut self, fis: &ForInStatement) {
        let var_name = match &fis.left {
            ForStatementLeft::VariableDeclaration(vd) => vd
                .declarations
                .first()
                .and_then(|decl| self.binding_name(&decl.id))
                .unwrap_or("key")
                .to_string(),
            ForStatementLeft::AssignmentTargetIdentifier(id) => {
                // for (key in obj) — key is an existing variable
                id.name.to_string()
            }
            _ => "key".to_string(),
        };

        // Get the object name from the right side (must be a simple identifier)
        let obj_name = match &fis.right {
            Expression::Identifier(id) => id.name.to_string(),
            _ => {
                // Non-identifier expressions not supported for for-in
                self.write_indent();
                self.writeln(
                    "@compileError(\"for-in only supported with identifier objects\");",
                );
                return;
            }
        };

        // Check if the object is a dynamic object (HashMap-like).
        let obj_type = self.type_info.var_types.get(&obj_name);

        // Case 1: HashMap → iterator-based for-in
        let is_dynamic = obj_type
            .map(|t| matches!(t, ZigType::Anytype))
            .unwrap_or(false);

        if is_dynamic {
            self.write_indent();
            self.write(&format!(
                "{{ var __it = {obj}.iterator(); while (__it.next()) |__kv| {{ const {var} = __kv.key_ptr.*;\n",
                obj = obj_name,
                var = var_name
            ));
            self.indent += 1;
            self.emit_stmt_or_block(&fis.body);
            self.indent -= 1;
            self.write_indent();
            self.writeln(&format!("}}}} // for-in {}", obj_name));
            return;
        }

        // Case 2: Static struct with known fields → unroll loop
        if let Some(ZigType::Struct(fields)) = obj_type
            && !fields.is_empty() {
                let fields: Vec<_> = fields.iter().map(|(n, t)| (n.clone(), t.clone())).collect();
                for (field_name, _) in &fields {
                    self.write_indent();
                    self.writeln("{");
                    self.indent += 1;
                    self.write_indent();
                    self.writeln(&format!("const {} = \"{}\";", var_name, field_name));
                    self.emit_stmt_or_block(&fis.body);
                    self.indent -= 1;
                    self.write_indent();
                    self.writeln("}");
                }
                return;
            }

        // Case 3: Unknown type → compile error
        self.write_indent();
        self.write(&format!(
            "@compileError(\"for-in: '{}' is not a dynamic object\");\n",
            obj_name
        ));
    }

    fn emit_for_in_labeled(&mut self, fis: &ForInStatement) {
        let var_name = match &fis.left {
            ForStatementLeft::VariableDeclaration(vd) => vd
                .declarations
                .first()
                .and_then(|decl| self.binding_name(&decl.id))
                .unwrap_or("key")
                .to_string(),
            ForStatementLeft::AssignmentTargetIdentifier(id) => id.name.to_string(),
            _ => "key".to_string(),
        };
        let obj_name = match &fis.right {
            Expression::Identifier(id) => id.name.to_string(),
            _ => {
                self.write("@compileError(\"for-in only supported with identifier objects\");\n");
                return;
            }
        };
        let obj_type = self.type_info.var_types.get(&obj_name);
        let is_dynamic = obj_type
            .map(|t| matches!(t, ZigType::Anytype))
            .unwrap_or(false);
        if is_dynamic {
            self.write(&format!(
                "{{ var __it = {obj}.iterator(); while (__it.next()) |__kv| {{ const {var} = __kv.key_ptr.*;\n",
                obj = obj_name, var = var_name
            ));
            self.indent += 1;
            self.emit_stmt_or_block(&fis.body);
            self.indent -= 1;
            self.write_indent();
            self.writeln(&format!("}}}} // for-in {}", obj_name));
            return;
        }
        if let Some(ZigType::Struct(fields)) = obj_type
            && !fields.is_empty() {
                let fields: Vec<_> = fields.iter().map(|(n, t)| (n.clone(), t.clone())).collect();
                for (field_name, _) in &fields {
                    self.write_indent();
                    self.writeln("{");
                    self.indent += 1;
                    self.write_indent();
                    self.writeln(&format!("const {} = \"{}\";", var_name, field_name));
                    self.emit_stmt_or_block(&fis.body);
                    self.indent -= 1;
                    self.write_indent();
                    self.writeln("}");
                }
                return;
            }
        self.write(&format!(
            "@compileError(\"for-in: '{}' is not a dynamic object\");\n",
            obj_name
        ));
    }

    // JS:  switch (expr) { case v: ...; break; default: ... }
    // Zig: switch (expr) { v => { ... }, else => { ... }, }
    fn emit_switch(&mut self, ss: &SwitchStatement) {
        self.write_indent();

        self.write("switch (");
        self.emit_expr(&ss.discriminant);
        self.write(") {\n");

        self.indent += 1;
        let mut has_default = false;

        for case in ss.cases.iter() {
            self.write_indent();
            if let Some(test) = &case.test {
                self.emit_expr(test);
            } else {
                has_default = true;
                self.write("else");
            }
            self.write(" => {\n");

            self.indent += 1;
            for stmt in &case.consequent {
                // Skip break statements (not needed in Zig switch)
                if let Statement::BreakStatement(_) = stmt {
                    continue;
                }
                self.emit_fn_stmt(stmt);
            }
            self.indent -= 1;

            self.write_indent();
            self.write("},\n");
        }

        // Zig switch must be exhaustive; add empty else if no default
        if !has_default {
            self.write_indent();
            self.writeln("else => {},");
        }

        self.indent -= 1;

        self.writeln("}");
    }

    /// Recursively collect all identifier names from a statement and its children.
    fn collect_stmt_idents(stmt: &Statement, names: &mut HashSet<String>) {
        match stmt {
            Statement::ExpressionStatement(es) => {
                Self::collect_expr_idents(&es.expression, names);
            }
            Statement::ReturnStatement(rs) => {
                if let Some(arg) = &rs.argument {
                    Self::collect_expr_idents(arg, names);
                }
            }
            Statement::IfStatement(is) => {
                Self::collect_expr_idents(&is.test, names);
                Self::collect_stmt_idents(&is.consequent, names);
                if let Some(alt) = &is.alternate {
                    Self::collect_stmt_idents(alt, names);
                }
            }
            Statement::WhileStatement(ws) => {
                Self::collect_expr_idents(&ws.test, names);
                Self::collect_stmt_idents(&ws.body, names);
            }
            Statement::DoWhileStatement(dws) => {
                Self::collect_stmt_idents(&dws.body, names);
                Self::collect_expr_idents(&dws.test, names);
            }
            Statement::ForStatement(fs) => {
                if let Some(test) = &fs.test {
                    Self::collect_expr_idents(test, names);
                }
                if let Some(update) = &fs.update {
                    Self::collect_expr_idents(update, names);
                }
                Self::collect_stmt_idents(&fs.body, names);
            }
            Statement::ForOfStatement(fos) => {
                Self::collect_expr_idents(&fos.right, names);
                Self::collect_stmt_idents(&fos.body, names);
            }
            Statement::VariableDeclaration(vd) => {
                for decl in &vd.declarations {
                    if let Some(init) = &decl.init {
                        Self::collect_expr_idents(init, names);
                    }
                }
            }
            Statement::BlockStatement(bs) => {
                for s in &bs.body {
                    Self::collect_stmt_idents(s, names);
                }
            }
            Statement::TryStatement(ts) => {
                for s in &ts.block.body {
                    Self::collect_stmt_idents(s, names);
                }
                if let Some(handler) = &ts.handler {
                    for s in &handler.body.body {
                        Self::collect_stmt_idents(s, names);
                    }
                }
            }
            Statement::SwitchStatement(ss) => {
                Self::collect_expr_idents(&ss.discriminant, names);
                for case in &ss.cases {
                    if let Some(test) = &case.test {
                        Self::collect_expr_idents(test, names);
                    }
                    for s in &case.consequent {
                        Self::collect_stmt_idents(s, names);
                    }
                }
            }
            _ => {}
        }
    }

    /// Recursively collect all identifier names from an expression.
    fn collect_expr_idents(expr: &Expression, names: &mut HashSet<String>) {
        match expr {
            Expression::Identifier(id) => {
                names.insert(id.name.to_string());
            }
            Expression::BinaryExpression(be) => {
                Self::collect_expr_idents(&be.left, names);
                Self::collect_expr_idents(&be.right, names);
            }
            Expression::CallExpression(ce) => {
                Self::collect_expr_idents(&ce.callee, names);
                for arg in &ce.arguments {
                    if let Some(e) = arg.as_expression() {
                        Self::collect_expr_idents(e, names);
                    }
                }
            }
            Expression::AssignmentExpression(ae) => {
                if let AssignmentTarget::AssignmentTargetIdentifier(id) = &ae.left {
                    names.insert(id.name.to_string());
                }
                Self::collect_expr_idents(&ae.right, names);
            }
            Expression::UnaryExpression(ue) => {
                Self::collect_expr_idents(&ue.argument, names);
            }
            Expression::AwaitExpression(ae) => {
                Self::collect_expr_idents(&ae.argument, names);
            }
            Expression::UpdateExpression(ue) => {
                if let SimpleAssignmentTarget::AssignmentTargetIdentifier(id) = &ue.argument {
                    names.insert(id.name.to_string());
                }
            }
            Expression::LogicalExpression(le) => {
                Self::collect_expr_idents(&le.left, names);
                Self::collect_expr_idents(&le.right, names);
            }
            Expression::ConditionalExpression(ce) => {
                Self::collect_expr_idents(&ce.test, names);
                Self::collect_expr_idents(&ce.consequent, names);
                Self::collect_expr_idents(&ce.alternate, names);
            }
            Expression::ArrayExpression(ae) => {
                for elem in &ae.elements {
                    if let Some(e) = elem.as_expression() {
                        Self::collect_expr_idents(e, names);
                    }
                }
            }
            Expression::StaticMemberExpression(mem) => {
                Self::collect_expr_idents(&mem.object, names);
            }
            Expression::ComputedMemberExpression(mem) => {
                Self::collect_expr_idents(&mem.object, names);
                Self::collect_expr_idents(&mem.expression, names);
            }
            Expression::NewExpression(ne) => {
                Self::collect_expr_idents(&ne.callee, names);
                for arg in &ne.arguments {
                    if let Some(e) = arg.as_expression() {
                        Self::collect_expr_idents(e, names);
                    }
                }
            }
            Expression::ParenthesizedExpression(pe) => {
                Self::collect_expr_idents(&pe.expression, names);
            }
            Expression::TemplateLiteral(tl) => {
                for e in &tl.expressions {
                    Self::collect_expr_idents(e, names);
                }
            }
            _ => {}
        }
    }
}

// ── Arrow function support ─────────────────────────────

impl Codegen {
    /// Emit an arrow function as a Zig function.
    /// Generates the function definition and returns the function name.
    /// Detect which variables are mutated (assigned to or updated) in a list of statements.
    /// Returns a set of variable names that are mutated.
    fn detect_mutated_vars_in_stmts(stmts: &[Statement]) -> std::collections::HashSet<String> {
        let mut mutated = std::collections::HashSet::new();
        for stmt in stmts {
            Self::detect_mutated_in_stmt(stmt, &mut mutated);
        }
        mutated
    }

    fn detect_mutated_in_stmt(stmt: &Statement, mutated: &mut std::collections::HashSet<String>) {
        match stmt {
            Statement::ExpressionStatement(es) => {
                Self::detect_mutated_in_expr(&es.expression, mutated);
            }
            Statement::ReturnStatement(rs) => {
                if let Some(expr) = &rs.argument {
                    Self::detect_mutated_in_expr(expr, mutated);
                }
            }
            Statement::BlockStatement(bs) => {
                for s in &bs.body {
                    Self::detect_mutated_in_stmt(s, mutated);
                }
            }
            Statement::IfStatement(is) => {
                Self::detect_mutated_in_expr(&is.test, mutated);
                Self::detect_mutated_in_stmt(&is.consequent, mutated);
                if let Some(alt) = &is.alternate {
                    Self::detect_mutated_in_stmt(alt, mutated);
                }
            }
            Statement::WhileStatement(ws) => {
                Self::detect_mutated_in_expr(&ws.test, mutated);
                Self::detect_mutated_in_stmt(&ws.body, mutated);
            }
            Statement::ForStatement(fs) => {
                if let Some(test) = &fs.test {
                    Self::detect_mutated_in_expr(test, mutated);
                }
                if let Some(update) = &fs.update {
                    Self::detect_mutated_in_expr(update, mutated);
                }
                Self::detect_mutated_in_stmt(&fs.body, mutated);
            }
            Statement::ForOfStatement(fos) => {
                Self::detect_mutated_in_expr(&fos.right, mutated);
                Self::detect_mutated_in_stmt(&fos.body, mutated);
            }
            Statement::SwitchStatement(ss) => {
                Self::detect_mutated_in_expr(&ss.discriminant, mutated);
                for case in &ss.cases {
                    for s in &case.consequent {
                        Self::detect_mutated_in_stmt(s, mutated);
                    }
                }
            }
            Statement::TryStatement(ts) => {
                for s in &ts.block.body {
                    Self::detect_mutated_in_stmt(s, mutated);
                }
                if let Some(handler) = &ts.handler {
                    for s in &handler.body.body {
                        Self::detect_mutated_in_stmt(s, mutated);
                    }
                }
                if let Some(finalizer) = &ts.finalizer {
                    for s in &finalizer.body {
                        Self::detect_mutated_in_stmt(s, mutated);
                    }
                }
            }
            _ => {}
        }
    }

    fn detect_mutated_in_expr(expr: &Expression, mutated: &mut std::collections::HashSet<String>) {
        match expr {
            Expression::AssignmentExpression(ae) => {
                // The assignment target is mutated
                if let AssignmentTarget::AssignmentTargetIdentifier(id) = &ae.left {
                    mutated.insert(id.name.to_string());
                }
                // Also check the right side (might contain mutations)
                Self::detect_mutated_in_expr(&ae.right, mutated);
            }
            Expression::UpdateExpression(ue) => {
                // x++ or ++x
                if let SimpleAssignmentTarget::AssignmentTargetIdentifier(id) = &ue.argument {
                    mutated.insert(id.name.to_string());
                }
            }
            Expression::BinaryExpression(be) => {
                Self::detect_mutated_in_expr(&be.left, mutated);
                Self::detect_mutated_in_expr(&be.right, mutated);
            }
            Expression::CallExpression(ce) => {
                Self::detect_mutated_in_expr(&ce.callee, mutated);
                for arg in &ce.arguments {
                    if let Some(e) = arg.as_expression() {
                        Self::detect_mutated_in_expr(e, mutated);
                    }
                }
            }
            Expression::LogicalExpression(le) => {
                Self::detect_mutated_in_expr(&le.left, mutated);
                Self::detect_mutated_in_expr(&le.right, mutated);
            }
            Expression::ConditionalExpression(ce) => {
                Self::detect_mutated_in_expr(&ce.test, mutated);
                Self::detect_mutated_in_expr(&ce.consequent, mutated);
                Self::detect_mutated_in_expr(&ce.alternate, mutated);
            }
            Expression::UnaryExpression(ue) => {
                Self::detect_mutated_in_expr(&ue.argument, mutated);
            }
            Expression::AwaitExpression(ae) => {
                Self::detect_mutated_in_expr(&ae.argument, mutated);
            }
            _ => {}
        }
    }

    /// Collect captured variables from an arrow function body.
    /// A variable is "captured" if it's referenced in the body but is not a parameter.
    /// Correctly sets `is_mut` by detecting mutations in the arrow body.
    fn collect_captured_vars(&self, arrow: &ArrowFunctionExpression) -> Vec<(String, ZigType, bool)> {
        let mut captured = Vec::new();
        let mut seen = std::collections::HashSet::new();

        // Collect parameter names (as String for comparison)
        let param_names: std::collections::HashSet<String> = arrow
            .params
            .items
            .iter()
            .filter_map(|p| crate::native_proto::infer::binding_name(&p.pattern))
            .map(|s| s.to_string())
            .collect();

        // Walk the body statements to find Identifier references
        for stmt in &arrow.body.statements {
            Self::collect_idents_from_stmt(stmt, &mut captured, &mut seen, &param_names, &self.type_info);
        }

        // Detect which captured variables are mutated in the arrow body
        let mutated = Self::detect_mutated_vars_in_stmts(&arrow.body.statements);
        // Update is_mut for each captured variable
        for (name, _ztype, is_mut) in &mut captured {
            *is_mut = mutated.contains(name);
        }

        captured
    }

    /// Helper: collect identifiers from a statement
    fn collect_idents_from_stmt(
        stmt: &Statement,
        captured: &mut Vec<(String, ZigType, bool)>,
        seen: &mut std::collections::HashSet<String>,
        param_names: &std::collections::HashSet<String>,
        type_info: &crate::native_proto::TypeCheckResult,
    ) {
        match stmt {
            Statement::ExpressionStatement(es) => {
                Self::collect_idents_from_expr(&es.expression, captured, seen, param_names, type_info);
            }
            Statement::ReturnStatement(ret) => {
                if let Some(expr) = &ret.argument {
                    Self::collect_idents_from_expr(expr, captured, seen, param_names, type_info);
                }
            }
            _ => {}
        }
    }

    /// Helper: collect identifiers from an expression
    fn collect_idents_from_expr(
        expr: &Expression,
        captured: &mut Vec<(String, ZigType, bool)>,
        seen: &mut std::collections::HashSet<String>,
        param_names: &std::collections::HashSet<String>,
        type_info: &crate::native_proto::TypeCheckResult,
    ) {
        use oxc_ast::ast::Expression;
        match expr {
            Expression::Identifier(id) => {
                let name = id.name.as_str();
                // If not a parameter and not already seen, it's a captured variable
                if !param_names.contains(name) && !seen.contains(name) {
                    seen.insert(name.to_string());
                    let ztype = type_info.var_types.get(name).cloned().unwrap_or(ZigType::I64);
                    // TODO: properly detect if captured var is mutated in arrow body
                    let is_mut = false;
                    captured.push((name.to_string(), ztype, is_mut));
                }
            }
            Expression::BinaryExpression(be) => {
                Self::collect_idents_from_expr(&be.left, captured, seen, param_names, type_info);
                Self::collect_idents_from_expr(&be.right, captured, seen, param_names, type_info);
            }
            Expression::CallExpression(ce) => {
                for arg in &ce.arguments {
                    if let Some(expr) = arg.as_expression() {
                        Self::collect_idents_from_expr(expr, captured, seen, param_names, type_info);
                    }
                }
                Self::collect_idents_from_expr(&ce.callee, captured, seen, param_names, type_info);
            }
            _ => {}
        }
    }

    /// Get the return type string for an arrow function.
    fn arrow_return_type_str(&self, arrow: &ArrowFunctionExpression) -> &'static str {
        let inferred = self.infer_arrow_return_type(arrow);
        match inferred {
            Some(ZigType::I64) => "i64",
            Some(ZigType::F64) => "f64",
            Some(ZigType::Bool) => "bool",
            Some(ZigType::Str) => "[]const u8",
            Some(ZigType::Void) => "void",
            Some(_) => "i64", // NamedStruct, ArrayList, etc. — use i64 fallback
            None => {
                // When type is indeterminate:
                // - Single-expression arrow: always returns a value → i64
                // - Block-body without return → void
                if arrow.body.statements.len() == 1
                    && matches!(
                        arrow.body.statements[0],
                        Statement::ExpressionStatement(_)
                    )
                {
                    "i64"
                } else {
                    // Check if any statement in the block is a return
                    let has_return = arrow
                        .body
                        .statements
                        .iter()
                        .any(|s| matches!(s, Statement::ReturnStatement(_)));
                    if has_return {
                        "i64"
                    } else {
                        "void"
                    }
                }
            }
        }
    }

    /// Infer the return type of an arrow function by examining its body.
    fn infer_arrow_return_type(&self, arrow: &ArrowFunctionExpression) -> Option<ZigType> {
        // Single-expression arrow: type is the expression's type
        if arrow.body.statements.len() == 1
            && let Statement::ExpressionStatement(es) = &arrow.body.statements[0]
        {
            return self.infer_arrow_expr_type(&es.expression);
        }
        // Block body: scan return statements
        for stmt in &arrow.body.statements {
            if let Statement::ReturnStatement(rs) = stmt {
                if let Some(ref arg) = rs.argument {
                    return self.infer_arrow_expr_type(arg);
                }
                return None; // bare `return;` means void
            }
        }
        None // no return → void
    }

    /// Best-effort type inference for arrow body expressions.
    fn infer_arrow_expr_type(&self, expr: &Expression) -> Option<ZigType> {
        match expr {
            Expression::NumericLiteral(nl) => {
                if let Some(raw) = &nl.raw {
                    let s = raw.as_str();
                    if s.contains('.') || s.contains('e') || s.contains('E') {
                        Some(ZigType::F64)
                    } else {
                        Some(ZigType::I64)
                    }
                } else {
                    Some(ZigType::I64)
                }
            }
            Expression::StringLiteral(_) => Some(ZigType::Str),
            Expression::BooleanLiteral(_) => Some(ZigType::Bool),
            Expression::Identifier(id) => {
                self.type_info.var_types.get(id.name.as_str()).cloned()
            }
            Expression::BinaryExpression(be) => {
                // Heuristic: try left operand first (covers patterns like `x * 2`, `x > 0`)
                self.infer_arrow_expr_type(&be.left).or_else(|| self.infer_arrow_expr_type(&be.right))
            }
            Expression::UnaryExpression(ue) => {
                self.infer_arrow_expr_type(&ue.argument)
            }
            Expression::CallExpression(ce) => {
                // Look up callee in fn_return_types
                if let Expression::Identifier(id) = &ce.callee {
                    self.type_info.fn_return_types.get(id.name.as_str()).cloned()
                } else {
                    None
                }
            }
            Expression::StaticMemberExpression(sme) => {
                // Handle patterns like obj.prop, arr.length etc.
                let field = sme.property.name.as_str();
                match field {
                    "length" | "len" => Some(ZigType::I64),
                    _ => None,
                }
            }
            Expression::ConditionalExpression(ce) => {
                // For ternary, prefer consequent type (they should match)
                self.infer_arrow_expr_type(&ce.consequent)
                    .or_else(|| self.infer_arrow_expr_type(&ce.alternate))
            }
            _ => None,
        }
    }
    /// Generates the struct definition (with fields and call method) and stores it in self.closure_defs.
    /// Returns the struct name.
    fn emit_closure_struct(&mut self, arrow: &ArrowFunctionExpression, captured: Vec<(String, ZigType, bool)>) -> String {
        let struct_name = format!("Closure_{}", self.arrow_counter);
        self.arrow_counter += 1;

        // Store closure info for assignment site (so emit_var_decl can generate instantiation)
        self.closure_vars.insert(struct_name.clone(), captured.clone());

        // ── Temporarily redirect output to build the struct definition ──
        let old_output = std::mem::take(&mut self.output);
        let old_indent = self.indent;
        self.output = String::new();
        self.indent = 0;

        // ── Struct definition ──
        self.writeln(&format!("const {} = struct {{", struct_name));
        self.indent = 1;

        // Fields for captured variables
        // Value capture (is_mut=false):  T
        // Reference capture (is_mut=true): *T  (the closure holds a pointer)
        for (name, ztype, is_mut) in &captured {
            let tstr = match ztype {
                ZigType::I64 => "i64".to_string(),
                ZigType::F64 => "f64".to_string(),
                ZigType::Bool => "bool".to_string(),
                ZigType::Str => "[]const u8".to_string(),
                ZigType::Void => "void".to_string(),
                ZigType::NamedStruct(s) => s.clone(),
                ZigType::ArrayList(_) => "std.ArrayList(JsAny)".to_string(),
                _ => "i64".to_string(),
            };
            if *is_mut {
                // Reference capture: store a pointer so the closure can mutate the outer variable
                self.writeln(&format!("{}: *{},", name, tstr));
            } else {
                // Value capture: store a copy
                self.writeln(&format!("{}: {},", name, tstr));
            }
        }

        // ── call method (single-line signature) ──
        let mut sig = String::from("fn call(self: *@This()");
        for param in &arrow.params.items {
            sig.push_str(", ");
            if let Some(pname) = crate::native_proto::infer::binding_name(&param.pattern) {
                sig.push_str(&format!("{}: anytype", pname));
            }
        }
        // Infer return type
        sig.push_str(&format!(") {} {{", self.arrow_return_type_str(arrow)));
        self.writeln(&sig);
        self.indent += 1;

        // ── Generate method body ──
        // Set current_captured so emit_expr rewrites identifiers to self.xxx
        let saved_captured = std::mem::take(&mut self.current_captured);
        self.current_captured = captured.clone();

        // Check if arrow function has expression body (single ExpressionStatement without return)
        // In JS: `(y) => x + y` → oxc parses as ExpressionStatement(x + y)
        // In Zig: need to add `return` prefix
        if arrow.body.statements.len() == 1 {
            if let Statement::ExpressionStatement(es) = &arrow.body.statements[0] {
                self.write_indent();
                self.write("return ");
                self.emit_expr(&es.expression);
                self.write(";\n");
            } else {
                // Block body with statements
                for stmt in &arrow.body.statements {
                    self.emit_fn_stmt(stmt);
                }
            }
        } else {
            // Multiple statements or empty: generate as-is
            for stmt in &arrow.body.statements {
                self.emit_fn_stmt(stmt);
            }
        }

        // Restore
        self.current_captured = saved_captured;

        self.indent = 1;
        self.writeln("}");

        self.indent = 0;
        self.writeln("};");

        // ── Get the generated struct definition and restore output ──
        let struct_def = std::mem::take(&mut self.output);
        self.output = old_output;
        self.indent = old_indent;

        // Store in closure_defs (will be prepended to output later)
        self.closure_defs.push(struct_def);

        struct_name
    }

    pub(crate) fn emit_arrow_function(&mut self, arrow: &ArrowFunctionExpression) -> String {
        // Detect captured variables (closure check)
        let captured = self.collect_captured_vars(arrow);
        if !captured.is_empty() {
            // Generate closure struct (captures outer variables)
            let struct_name = self.emit_closure_struct(arrow, captured);
            return struct_name;
        }
        // No captured vars: generate plain nested function (current behavior)
        let fn_name = format!("_arrow_fn_{}", self.arrow_counter);
        self.arrow_counter += 1;
        
        // Generate function signature (in a single string to avoid whitespace issues)
        let mut sig = format!("fn {}(", fn_name);
        
        // Generate params
        for (param_idx, param) in arrow.params.items.iter().enumerate() {
            if param_idx > 0 {
                sig.push_str(", ");
            }
            if let Some(pname) = crate::native_proto::infer::binding_name(&param.pattern) {
                sig.push_str(&format!("{}: anytype", pname));
            }
        }
        
        // Infer return type from arrow body.
        sig.push_str(&format!(") {} {{", self.arrow_return_type_str(arrow)));
        self.write_indent();
        self.writeln(&sig);
        
        // Generate function body
        self.indent += 1;
        
        // Handle body: for single-expression arrows, the body is a FunctionBody
        // with a single ExpressionStatement.
        // We need to generate "return expr;" for the expression.
        if arrow.body.statements.len() == 1 {
            if let Statement::ExpressionStatement(es) = &arrow.body.statements[0] {
                // Single-expression arrow: generate "return expr;"
                self.write_indent();
                self.write("return ");
                self.emit_expr(&es.expression);
                self.write(";
");
            } else {
                // Block body with a single statement (not expression)
                for stmt in &arrow.body.statements {
                    self.emit_fn_stmt(stmt);
                }
            }
        } else {
            // Block body with multiple statements
            for stmt in &arrow.body.statements {
                self.emit_fn_stmt(stmt);
            }
        }
        
        self.indent -= 1;
        self.writeln("}");
        
        fn_name
    }
}

/// Check if an expression is a TypedArray constructor call
/// (new Int32Array(...), new Uint8Array(...), new Float64Array(...)).
/// Returns the Zig type suffix (e.g., "I32", "U8", "F64") if it is.
pub(crate) fn typedarray_init_type(expr: &Expression) -> Option<&'static str> {
    match expr {
        Expression::NewExpression(ne) => {
            if let Expression::Identifier(id) = &ne.callee {
                match id.name.as_str() {
                    "Int32Array" => Some("I32"),
                    "Uint8Array" => Some("U8"),
                    "Float64Array" => Some("F64"),
                    _ => None,
                }
            } else {
                None
            }
        }
        _ => None,
    }
}
