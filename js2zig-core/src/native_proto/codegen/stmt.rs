// native_proto/codegen/stmt.rs
// Statement-level code generation: toplevel, var_decl, fn, if, while, for, switch.

use super::Codegen;
use crate::native_proto::builtins;
use crate::native_proto::{ExportedFunction, ZigType};
use oxc_ast::ast::*;

// ── Variable declarations ────────────────────────────

impl Codegen {
    /// Emit a variable declaration. Toplevel: only `const` allowed.
    /// Inside functions: `var` with type inference + undefined init.
    pub(crate) fn emit_var_decl(&mut self, vd: &VariableDeclaration) {
        for decl in &vd.declarations {
            if let Some(name) = crate::native_proto::infer::binding_name(&decl.id) {
                let is_const = matches!(vd.kind, VariableDeclarationKind::Const);

                // Override: if the variable is mutated, use 'var'.
                // If the variable is never mutated, use 'const' regardless of JS kind.
                let is_const = is_const && !self.type_info.mutated_vars.contains(name);

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
                    Some(init) => {
                        self.write_indent();
                        // Force 'var' for Map/Set types (they are mutated via methods)
                        let is_const = if let Some(inferred_ty) = self.type_info.var_types.get(name)
                        {
                            match inferred_ty {
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
                            self.write(") catch unreachable;\n");
                        } else if let Some(inferred_ty) = self.type_info.var_types.get(name) {
                            // Definite type from pre-computed type info.
                            // Skip type annotation for NamedStruct (Map/Set) — Zig infers from init.
                            let skip_type_annotation =
                                matches!(inferred_ty, ZigType::NamedStruct(_));
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
                        } else {
                            // Indeterminate type (Rule 8 error already in type_info.errors)
                            self.write(&format!("{} {} = ", kw, name));
                            self.emit_expr(init);
                            self.write(";\n");
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
    pub(crate) fn emit_fn(&mut self, fd: &Function) {
        let name = fd
            .id
            .as_ref()
            .map(|id| id.name.as_str())
            .unwrap_or("anonymous");

        // Check if function contains await (from pre-computed type_info)
        let is_async = self.type_info.is_async.get(name).copied().unwrap_or(false);

        // Read pre-computed return type from TypeInferResult.
        let ret_ty = self.type_info.fn_return_types.get(name).cloned();
        self.current_fn_return_type = ret_ty.clone();

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
                // Export function params: always typed; non-export: use anytype
                if self.current_fn_is_export {
                    self.write(&format!("{}: {}", pname, ptype.to_zig_type()));
                } else {
                    // Non-export params are inferred as anytype in the type info
                    self.write(&format!("{}: {}", pname, ptype.to_zig_type()));
                }
                param_idx += 1;
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
                    self.write(&format!("{}: anytype", pname));
                    param_idx += 1;
                }
            }
        }

        // Return type
        let ret_zig_type = match &self.current_fn_return_type {
            Some(ZigType::I64) => "i64".to_string(),
            Some(ZigType::F64) => "f64".to_string(),
            Some(ZigType::Bool) => "bool".to_string(),
            Some(ZigType::Str) => "[]const u8".to_string(),
            Some(ZigType::Void) => "void".to_string(),
            None => "void".to_string(),
            Some(other) => other.to_zig_type(),
        };
        self.writeln(&format!(") {} {{", ret_zig_type));

        self.indent += 1;
        if let Some(body) = &fd.body {
            for stmt in &body.statements {
                self.emit_fn_stmt(stmt);
            }
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
            });
        }
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
                    self.emit_expr(arg);
                    self.write(";\n");
                } else {
                    self.write("return;\n");
                }
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
                self.emit_expr(&es.expression);
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
            Statement::SwitchStatement(ss) => {
                self.emit_switch(ss);
            }
            Statement::BreakStatement(_) => {
                self.write_indent();
                self.write("break;\n");
            }
            Statement::ContinueStatement(_) => {
                self.write_indent();
                self.write("continue;\n");
            }
            Statement::BlockStatement(bs) => {
                self.emit_block(bs);
            }
            _ => { /* skip unsupported */ }
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

    // JS:  for (init; test; update) { ... }
    // Zig: { init; while (test) : (update) { ... } }
    fn emit_for(&mut self, fs: &ForStatement) {
        self.write_indent();
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
}
