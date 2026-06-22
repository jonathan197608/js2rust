// native_proto/codegen.rs
// All Codegen impl methods in one file.
// This avoids Rust visibility issues across multiple impl blocks in different files.

use crate::native_proto::ExportedFunction;
use crate::native_proto::builtins;
use crate::native_proto::{Codegen, ZigType};
use oxc_ast::ast::*;

// ── Constructor ─────────────────────────────────────

impl Codegen {
    pub fn new(
        type_info: crate::native_proto::TypeCheckResult,
        jsdoc_data: crate::native_proto::JSDocData,
        exported_functions: Option<std::collections::HashSet<String>>,
    ) -> Self {
        Self {
            output: String::new(),
            indent: 0,
            errors: Vec::new(),
            type_info,
            jsdoc_data: Some(jsdoc_data),
            current_fn_is_export: false,
            current_fn_return_type: None,
            exported_fns: Vec::new(),
            cabi_exports: Vec::new(),
            task_counter: 0,
            exported_functions,
        }
    }
}

// ── Entry point ─────────────────────────────────────

impl Codegen {
    /// Emit all @typedef struct definitions at the top of the generated file.
    fn emit_typedefs(&mut self) {
        // Clone typedefs to avoid borrow checker issues
        let typedefs = match &self.jsdoc_data {
            Some(data) => data.typedefs.clone(),
            None => return,
        };
        if typedefs.is_empty() {
            return;
        }
        for (name, td) in &typedefs {
            self.writeln(&format!("const {} = struct {{", name));
            self.indent += 1;
            for field in &td.fields {
                let zig_ty = crate::native_proto::jsdoc::jsdoc_type_to_zig(&field.ty, &typedefs);
                // Optional field: prepend ? to the type
                let zig_ty = if field.optional {
                    format!("?{}", zig_ty)
                } else {
                    zig_ty
                };
                self.writeln(&format!("{}: {},", field.name, zig_ty));
            }
            // Generate toJson() method for serialization using std.json.fmt()
            // Use the global arena allocator (js_allocator.getAllocator()).
            self.writeln("");
            self.writeln("pub fn toJson(self: *const @This()) ![]u8 {");
            self.indent += 1;
            // Use std.io.Writer.Allocating + std.json.fmt() for serialization
            self.writeln(
                "var string = std.io.Writer.Allocating.init(js_allocator.getAllocator());",
            );
            self.writeln("errdefer string.deinit();");
            self.writeln("try string.writer().print(\"{f}\", .{std.json.fmt(self.*, .{})});");
            self.writeln("return string.toOwnedSlice();");
            self.indent -= 1;
            self.writeln("}");
            self.indent -= 1;
            self.writeln("};");
            self.writeln("");
        }
    }

    pub fn generate(&mut self, program: &Program) {
        // Phase A: analyze_objects, collect_used_names, walk_stmt_for_types
        // are all handled by TypeInferrer::infer_all() before codegen starts.

        // Emit struct typedefs (from JSDoc @typedef).
        self.emit_typedefs();

        // Emit code, skipping unused toplevel constants.
        for stmt in &program.body {
            self.emit_toplevel(stmt);
        }
    }

    fn emit_toplevel(&mut self, stmt: &Statement) {
        match stmt {
            Statement::VariableDeclaration(vd) => self.emit_var_decl(vd),
            Statement::FunctionDeclaration(fd) => {
                // Determine if this function is an export function.
                // Priority:
                // 1. If `exported_functions` is provided (from pipeline), use it.
                // 2. Otherwise, check if the function is inside `export {...}` (not supported yet).
                // 3. Default: non-export (pub fn, not C ABI).
                let fn_name = fd.id.as_ref().map(|id| id.name.as_str());
                let is_export = if let Some(ref exported) = self.exported_functions {
                    // Use exported_functions set from pipeline
                    fn_name.is_some_and(|name| exported.contains(name))
                } else {
                    // No export info: default to non-export.
                    // NOTE: `function foo() {}` (without `export`) is non-export.
                    false
                };

                let old_export = self.current_fn_is_export;
                self.current_fn_is_export = is_export;
                self.emit_fn(fd);
                self.current_fn_is_export = old_export;
            }
            Statement::ExportNamedDeclaration(export_decl) => {
                // `export function foo() {}` or `export const foo = ...`
                // These are ALWAYS export functions.
                match &export_decl.declaration {
                    Some(decl) => {
                        match decl {
                            Declaration::FunctionDeclaration(fd) => {
                                // is_export determined by exported_functions set,
                                // NOT always-true — dependency files only
                                // export names that the core file re-exports.
                                let fn_name = fd.id.as_ref().map(|id| id.name.as_str());
                                let is_export = self
                                    .exported_functions
                                    .as_ref()
                                    .is_some_and(|ex| fn_name.is_some_and(|n| ex.contains(n)));
                                let old_export = self.current_fn_is_export;
                                self.current_fn_is_export = is_export;
                                self.emit_fn(fd);
                                self.current_fn_is_export = old_export;
                            }
                            Declaration::VariableDeclaration(vd) => {
                                self.emit_var_decl(vd);
                            }
                            _ => { /* skip unsupported */ }
                        }
                    }
                    None => { /* skip (e.g., export {{ ... }} */ }
                }
            }
            _ => { /* skip */ }
        }
    }
}

// ── Variable declarations ────────────────────────────

impl Codegen {
    /// Emit a variable declaration. Toplevel: only `const` allowed.
    /// Inside functions: `var` with type inference + undefined init.
    fn emit_var_decl(&mut self, vd: &VariableDeclaration) {
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
    fn emit_fn(&mut self, fd: &Function) {
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
    fn emit_fn_stmt(&mut self, stmt: &Statement) {
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
    fn emit_if(&mut self, is: &IfStatement) {
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
    fn emit_while(&mut self, ws: &WhileStatement) {
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

// ── Expressions ─────────────────────────────────────

impl Codegen {
    fn emit_expr(&mut self, expr: &Expression) {
        match expr {
            Expression::NumericLiteral(n) => {
                self.write(&n.value.to_string());
            }
            Expression::StringLiteral(s) => {
                // Escape double quotes in string value for Zig string literal
                let escaped = s.value.replace("\"", "\\\"");
                self.write(&format!("\"{}\"", escaped));
            }
            Expression::BooleanLiteral(b) => {
                self.write(if b.value { "true" } else { "false" });
            }
            Expression::Identifier(id) => {
                let var_name = id.name.as_str();
                self.write(var_name);
            }
            Expression::BinaryExpression(be) => {
                self.emit_binary(be);
            }
            Expression::CallExpression(ce) => {
                self.emit_call(ce);
            }
            Expression::AssignmentExpression(ae) => {
                self.emit_assignment(ae);
            }
            Expression::UnaryExpression(ue) => {
                self.emit_unary(ue);
            }
            Expression::LogicalExpression(le) => {
                self.write("(");
                self.emit_expr(&le.left);
                self.write(&format!(" {} ", Self::logical_op(le.operator)));
                self.emit_expr(&le.right);
                self.write(")");
            }
            Expression::ParenthesizedExpression(pe) => {
                self.write("(");
                self.emit_expr(&pe.expression);
                self.write(")");
            }
            Expression::ConditionalExpression(ce) => {
                self.emit_conditional(ce);
            }
            Expression::ArrayExpression(ae) => {
                self.emit_array(ae);
            }
            Expression::ObjectExpression(oe) => {
                self.emit_object(oe);
            }
            Expression::StaticMemberExpression(mem) => {
                self.emit_expr(&mem.object);
                self.write(".");
                self.write(mem.property.name.as_str());
            }
            Expression::ComputedMemberExpression(mem) => {
                // Check if this is array indexing (numeric literal) or dynamic property access.
                match &mem.expression {
                    Expression::NumericLiteral(n) => {
                        // Array indexing with numeric literal: allow (e.g., arr[0])
                        self.emit_expr(&mem.object);
                        self.write(&format!("[{}]", n.value as i64));
                    }
                    _ => {
                        // Dynamic property access is not allowed in strict type system.
                        self.errors.push(
                            "Dynamic property access (obj[key]) is not allowed. Use static property access (obj.prop).".to_string()
                        );
                        self.write("/* error: dynamic property access */");
                    }
                }
            }
            Expression::AwaitExpression(ae) => {
                let task_var = format!("_t{}", self.task_counter);
                self.task_counter += 1;

                // emit: (blk: { var _tN = io.async(fn_async, .{io, args...}); defer _ = _tN.cancel(io) catch undefined; break :blk try _tN.await(io); })
                self.write("(blk: {\n");
                self.indent += 1;

                self.write_indent();
                self.write(&format!("var {} = io.async(", task_var));

                match &ae.argument {
                    Expression::CallExpression(call) => {
                        self.emit_expr(&call.callee);
                        self.write(", .{ io");
                        for arg in &call.arguments {
                            self.write(", ");
                            if let Some(expr) = arg.as_expression() {
                                self.emit_expr(expr);
                            } else {
                                self.write("undefined");
                            }
                        }
                        self.write(" }");
                    }
                    _ => {
                        self.emit_expr(&ae.argument);
                        self.write(", .{ io }");
                    }
                }

                self.write(");\n");

                self.write_indent();
                self.write(&format!(
                    "defer _ = {}.cancel(io) catch undefined;\n",
                    task_var
                ));

                self.write_indent();
                self.write(&format!("break :blk try {}.await(io);\n", task_var));

                self.indent -= 1;
                self.write_indent();
                self.write("})");
            }
            Expression::NewExpression(ne) => {
                // Check if this is new Int32Array(...) or new Uint8Array(...)
                if let Expression::Identifier(id) = &ne.callee {
                    let obj_name = id.name.as_str();
                    if obj_name == "Int32Array" {
                        // new Int32Array([...]) → js_typedarray.fromI32(...)
                        self.write("js_typedarray.fromI32(");
                        if let Some(first_arg) = ne.arguments.first()
                            && let Expression::ArrayExpression(ae) =
                                first_arg.as_expression().unwrap()
                        {
                            self.write("&[_]i64{");
                            for (i, elem) in ae.elements.iter().enumerate() {
                                if i > 0 {
                                    self.write(", ");
                                }
                                if let Some(e) = elem.as_expression() {
                                    self.emit_expr(e);
                                }
                            }
                            self.write("}");
                        }
                        self.write(")");
                        return;
                    } else if obj_name == "Uint8Array" {
                        // new Uint8Array([...]) → js_typedarray.fromU8(...)
                        self.write("js_typedarray.fromU8(");
                        if let Some(first_arg) = ne.arguments.first()
                            && let Expression::ArrayExpression(ae) =
                                first_arg.as_expression().unwrap()
                        {
                            self.write("&[_]u8{");
                            for (i, elem) in ae.elements.iter().enumerate() {
                                if i > 0 {
                                    self.write(", ");
                                }
                                if let Some(e) = elem.as_expression() {
                                    self.emit_expr(e);
                                }
                            }
                            self.write("}");
                        }
                        self.write(")");
                        return;
                    } else if obj_name == "Map" {
                        // new Map() → js_map.JsMap.init(js_allocator.getAllocator())
                        self.write("js_map.JsMap.init(js_allocator.getAllocator())");
                        return;
                    } else if obj_name == "Set" {
                        // new Set() → js_set.JsSet.init(js_allocator.getAllocator())
                        self.write("js_set.JsSet.init(js_allocator.getAllocator())");
                        return;
                    }
                }
                // Unsupported NewExpression
                self.errors.push(
                    "Unsupported NewExpression (only Int32Array and Uint8Array are supported)"
                        .to_string(),
                );
                self.write("@compileError(\"Unsupported NewExpression\")");
            }
            Expression::TemplateLiteral(tpl) => self.emit_template_literal(tpl),
            Expression::UpdateExpression(ue) => {
                // i++ → i += 1, i-- → i -= 1
                let op = match ue.operator {
                    UpdateOperator::Increment => " += 1",
                    UpdateOperator::Decrement => " -= 1",
                };
                // Emit the target (SimpleAssignmentTarget)
                match &ue.argument {
                    SimpleAssignmentTarget::AssignmentTargetIdentifier(id) => {
                        self.write(id.name.as_str());
                        self.write(op);
                    }
                    _ => {
                        self.errors.push(
                            "Unsupported UpdateExpression target (only simple identifiers)"
                                .to_string(),
                        );
                        self.write("@compileError(\"Unsupported UpdateExpression target\")");
                    }
                }
            }
            other => {
                // Unsupported expression type
                self.errors.push(format!(
                    "Unsupported expression type: {:?}",
                    std::mem::discriminant(other)
                ));
                self.write("@compileError(\"Unsupported expression type\")");
            }
        }
    }
}

// ── Binary / Call / Assignment / Unary / Conditional / Array ──

impl Codegen {
    // Binary expression with string-concat special case

    /// Recursively collect all operands in a string concatenation chain.
    /// Takes &BinaryExpression directly (avoids type wrapping issues).
    fn collect_concat_from_be<'a>(be: &'a BinaryExpression<'a>, out: &mut Vec<&'a Expression<'a>>) {
        // Left side
        if let Expression::BinaryExpression(ref left_be) = be.left {
            if left_be.operator == BinaryOperator::Addition {
                Self::collect_concat_from_be(left_be, out);
            } else {
                out.push(&be.left);
            }
        } else {
            out.push(&be.left);
        }

        // Right side
        if let Expression::BinaryExpression(ref right_be) = be.right {
            if right_be.operator == BinaryOperator::Addition {
                Self::collect_concat_from_be(right_be, out);
            } else {
                out.push(&be.right);
            }
        } else {
            out.push(&be.right);
        }
    }

    /// Emit a string concatenation using std.fmt.allocPrint (Zig 0.16.0: ++ requires comptime-known slices).
    fn emit_string_concat(&mut self, be: &BinaryExpression) {
        let mut operands: Vec<&Expression> = Vec::new();
        Self::collect_concat_from_be(be, &mut operands);

        // Build format string and arguments.
        // For string literals: include verbatim (escape { and }).
        // For expressions: use {s} placeholder, collect expression code as argument.
        let mut fmt = String::new();
        let mut args: Vec<String> = Vec::new();

        for op in &operands {
            if let Expression::StringLiteral(sl) = op {
                let escaped = sl.value.replace("{", "{{").replace("}", "}}");
                fmt.push_str(&escaped);
            } else {
                fmt.push_str("{s}");
                let arg_str = self.emit_expr_to_string(op);
                args.push(arg_str);
            }
        }

        // Generate: std.fmt.allocPrint(js_allocator.getAllocator(), "fmt", .{args}) catch unreachable
        if args.is_empty() {
            // All operands are string literals - just emit the concatenated literal
            self.write(&format!(
                "\"{}\"",
                fmt.replace("{{", "{").replace("}}", "}")
            ));
        } else {
            let args_str = format!(".{{{}}}", args.join(", "));
            self.write(&format!(
                "std.fmt.allocPrint(js_allocator.getAllocator(), \"{}\", {}) catch unreachable",
                fmt, args_str
            ));
        }
    }

    /// Emit a template literal `\`a=${x}\`` using std.fmt.allocPrint.
    /// Text segments form the format string (with `{`/`}` doubled and special
    /// chars escaped for a Zig string literal). Each interpolation picks a
    /// placeholder from the inferred type: Str→{s}, I64/F64→{d}, Bool→{},
    /// otherwise expr_is_string ? {s} : {}. Pure-text templates (no
    /// interpolation) degrade to a plain string literal (no allocation).
    /// Allocates from the global arena via js_allocator.getAllocator().
    fn emit_template_literal(&mut self, tpl: &TemplateLiteral) {
        let mut fmt = String::new();
        let mut args: Vec<String> = Vec::new();

        for (i, quasi) in tpl.quasis.iter().enumerate() {
            // Text segment: prefer cooked (JS escapes resolved), fallback to raw.
            let text: String = quasi
                .value
                .cooked
                .as_ref()
                .map(|c| c.as_str().to_string())
                .unwrap_or_else(|| quasi.value.raw.as_str().to_string());
            // Escape for a Zig string literal that is also a fmt template.
            for ch in text.chars() {
                match ch {
                    '\\' => fmt.push_str("\\\\"),
                    '"' => fmt.push_str("\\\""),
                    '\n' => fmt.push_str("\\n"),
                    '\r' => fmt.push_str("\\r"),
                    '\t' => fmt.push_str("\\t"),
                    '{' => fmt.push_str("{{"),
                    '}' => fmt.push_str("}}"),
                    c => fmt.push(c),
                }
            }

            // Interpolation following this text segment (if any).
            if i < tpl.expressions.len() {
                let expr = &tpl.expressions[i];
                let placeholder = match self.infer_expr_type(expr) {
                    Some(ZigType::Str) => "{s}",
                    Some(ZigType::I64) | Some(ZigType::F64) => "{d}",
                    Some(ZigType::Bool) => "{}",
                    _ => {
                        if self.expr_is_string(expr) {
                            "{s}"
                        } else {
                            "{}"
                        }
                    }
                };
                fmt.push_str(placeholder);
                let arg_str = self.emit_expr_to_string(expr);
                args.push(arg_str);
            }
        }

        if args.is_empty() {
            // Pure-text template → plain string literal (no allocation).
            self.write(&format!(
                "\"{}\"",
                fmt.replace("{{", "{").replace("}}", "}")
            ));
        } else {
            let args_str = format!(".{{{}}}", args.join(", "));
            self.write(&format!(
                "std.fmt.allocPrint(js_allocator.getAllocator(), \"{}\", {}) catch unreachable",
                fmt, args_str
            ));
        }
    }

    fn emit_binary(&mut self, be: &BinaryExpression) {
        // Check if either operand is a string type
        let left_is_string = self.expr_is_string(&be.left);
        let right_is_string = self.expr_is_string(&be.right);

        if be.operator == BinaryOperator::Addition && (left_is_string || right_is_string) {
            // Use std.fmt.allocPrint for runtime string concatenation
            // (Zig 0.16.0: ++ requires comptime-known slices)
            self.emit_string_concat(be);
        } else if (be.operator == BinaryOperator::Equality
            || be.operator == BinaryOperator::StrictEquality)
            && (left_is_string || right_is_string)
        {
            // String equality: use std.mem.eql(u8, a, b)
            self.write("std.mem.eql(u8, ");
            self.emit_expr(&be.left);
            self.write(", ");
            self.emit_expr(&be.right);
            self.write(")");
        } else if (be.operator == BinaryOperator::Inequality
            || be.operator == BinaryOperator::StrictInequality)
            && (left_is_string || right_is_string)
        {
            // String inequality: !std.mem.eql(u8, a, b)
            self.write("!std.mem.eql(u8, ");
            self.emit_expr(&be.left);
            self.write(", ");
            self.emit_expr(&be.right);
            self.write(")");
        } else if be.operator == BinaryOperator::Division {
            self.write("@divTrunc(");
            self.emit_expr(&be.left);
            self.write(", ");
            self.emit_expr(&be.right);
            self.write(")");
        } else if be.operator == BinaryOperator::Remainder {
            self.write("@rem(");
            self.emit_expr(&be.left);
            self.write(", ");
            self.emit_expr(&be.right);
            self.write(")");
        } else {
            self.emit_expr(&be.left);
            self.write(" ");
            self.write(Self::binary_op(be.operator));
            self.write(" ");
            self.emit_expr(&be.right);
        }
    }

    /// Check if an expression evaluates to a string type
    fn expr_is_string(&self, expr: &Expression) -> bool {
        match expr {
            Expression::StringLiteral(_) => true,
            Expression::TemplateLiteral(_) => true,
            Expression::Identifier(id) => {
                self.type_info.var_types.get(id.name.as_str()) == Some(&ZigType::Str)
            }
            // Handle nested binary expressions: if it's string concatenation, result is string
            Expression::BinaryExpression(be) if be.operator == BinaryOperator::Addition => {
                self.expr_is_string(&be.left) || self.expr_is_string(&be.right)
            }
            _ => false,
        }
    }

    // Call expression (all calls get `try`)
    fn emit_call(&mut self, ce: &CallExpression) {
        // Check if this is a Promise .then() or .catch() call (not supported in native_proto)
        if let Expression::StaticMemberExpression(ref mem) = ce.callee {
            let prop_name = mem.property.name.as_str();
            if prop_name == "then" || prop_name == "catch" {
                self.errors.push(format!(
                    "Promise.{}() is not supported. Use 'await' instead of '.{}()'",
                    prop_name, prop_name
                ));
                self.write(&format!(
                    "@compileError(\"Promise.{}() not supported, use 'await' instead\")",
                    prop_name
                ));
                return;
            }
        }

        // Check if this is a Promise.resolve() or Promise.reject() call
        if let Expression::StaticMemberExpression(ref mem) = ce.callee
            && let Expression::Identifier(ref obj) = mem.object
            && obj.name == "Promise"
        {
            let method = mem.property.name.as_str();
            if method == "resolve" || method == "reject" {
                self.errors.push(format!(
                            "Promise.{}() is not supported in native_proto mode. Use 'await' with async functions instead.",
                            method
                        ));
                self.write(&format!(
                    "@compileError(\"Promise.{}() not supported\")",
                    method
                ));
                return;
            }
        }

        // Check if this is a built-in object call (Math.xxx(), arr.xxx(), str.xxx())
        if let Some(builtin) = builtins::detect_builtin_call(ce)
            && self.emit_builtin_call(&builtin, ce)
        {
            return;
        }
        // If emit_builtin_call returns false, fall through to normal call handling

        // Check if this is JSON.stringify() call
        if let Expression::StaticMemberExpression(ref mem) = ce.callee
            && let Expression::Identifier(ref obj) = mem.object
            && obj.name == "JSON"
            && mem.property.name == "stringify"
        {
            // JSON.stringify(obj) → try obj.toJson()
            if let Some(first_arg) = ce.arguments.first() {
                self.write("try ");
                self.emit_expr_arg(first_arg);
                self.write(".toJson()");
                return;
            }
        }

        // Get callee name.
        let callee_name = match &ce.callee {
            Expression::Identifier(id) => Some(id.name.to_string()),
            _ => None,
        };

        // Emit function call (no `try` by default, only for error-returning functions).
        if let Some(ref name) = callee_name {
            // Check if this is a host function call (host_xxx)
            if let Some(host_func_name) = name.strip_prefix("host_") {
                // Convert host_add(...) to host.add(...)
                self.write(&format!("host.{}(", host_func_name));
                for (i, arg) in ce.arguments.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.emit_expr_arg(arg);
                }
                self.write(")");
                return;
            }
            self.write(name);
        } else {
            // Member function call (obj.method(...)) — not fully supported
            // Add more detail to the error message
            let callee_str = format!("{:?}", ce.callee);
            self.errors.push(format!("Member function calls (obj.method()) are not fully supported in native_proto mode: callee = {}", callee_str));
            self.write("@compileError(\"Member function calls not supported\")");
            return;
        }
        self.write("(");
        for (i, arg) in ce.arguments.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            self.emit_expr_arg(arg);
        }
        self.write(")");
    }

    /// Emit an expression to a temporary string (preserves self.output and all state).
    fn emit_expr_to_string(&mut self, expr: &Expression) -> String {
        let saved = std::mem::take(&mut self.output);
        self.emit_expr(expr);
        let result = std::mem::take(&mut self.output);
        self.output = saved;
        result
    }

    /// Emit Zig code for a built-in object call
    /// Returns true if the call was handled, false otherwise
    fn emit_builtin_call(&mut self, builtin: &builtins::BuiltinCall, ce: &CallExpression) -> bool {
        match builtin {
            // ── Math methods ─────────────────────────────
            builtins::BuiltinCall::MathAbs => {
                // Math.abs(x) → @abs(x)
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("Math.abs() requires exactly 1 argument".to_string());
                    return false;
                }
                self.write("@abs(");
                if let Some(arg) = ce.arguments.first()
                    && let Some(expr) = arg.as_expression()
                {
                    self.emit_expr(expr);
                }
                self.write(")");
                true
            }

            builtins::BuiltinCall::MathFloor => {
                // Math.floor(x) → @floor(x)
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("Math.floor() requires exactly 1 argument".to_string());
                    return false;
                }
                self.write("@floor(");
                if let Some(arg) = ce.arguments.first()
                    && let Some(expr) = arg.as_expression()
                {
                    self.emit_expr(expr);
                }
                self.write(")");
                true
            }

            builtins::BuiltinCall::MathCeil => {
                // Math.ceil(x) → @ceil(x)
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("Math.ceil() requires exactly 1 argument".to_string());
                    return false;
                }
                self.write("@ceil(");
                if let Some(arg) = ce.arguments.first()
                    && let Some(expr) = arg.as_expression()
                {
                    self.emit_expr(expr);
                }
                self.write(")");
                true
            }

            builtins::BuiltinCall::MathRound => {
                // Math.round(x) → @round(x)
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("Math.round() requires exactly 1 argument".to_string());
                    return false;
                }
                self.write("@round(");
                if let Some(arg) = ce.arguments.first()
                    && let Some(expr) = arg.as_expression()
                {
                    self.emit_expr(expr);
                }
                self.write(")");
                true
            }

            builtins::BuiltinCall::MathSqrt => {
                // Math.sqrt(x) → @sqrt(x)
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("Math.sqrt() requires exactly 1 argument".to_string());
                    return false;
                }
                self.write("@sqrt(");
                if let Some(arg) = ce.arguments.first()
                    && let Some(expr) = arg.as_expression()
                {
                    self.emit_expr(expr);
                }
                self.write(")");
                true
            }

            builtins::BuiltinCall::MathRandom => {
                // Math.random() → @as(f64, @floatFromInt(std.crypto.random.int(u64))) / @as(f64, std.math.maxInt(u64))
                // Simplified: use std.time.timestamp() for now
                if !ce.arguments.is_empty() {
                    self.errors
                        .push("Math.random() requires no arguments".to_string());
                    return false;
                }
                self.write("(@as(f64, @floatFromInt(std.crypto.random.int(u32))) / @as(f64, 4294967295.0))");
                true
            }

            builtins::BuiltinCall::MathPow => {
                // Math.pow(base, exp) → std.math.pow(f64, base, exp)
                if ce.arguments.len() != 2 {
                    self.errors
                        .push("Math.pow() requires exactly 2 arguments".to_string());
                    return false;
                }
                self.write("std.math.pow(f64, ");
                if let Some(arg) = ce.arguments.first()
                    && let Some(expr) = arg.as_expression()
                {
                    self.emit_expr(expr);
                }
                self.write(", ");
                if let Some(arg) = ce.arguments.get(1)
                    && let Some(expr) = arg.as_expression()
                {
                    self.emit_expr(expr);
                }
                self.write(")");
                true
            }

            builtins::BuiltinCall::MathMax => {
                // Math.max(a, b, ...) → find maximum of all arguments
                if ce.arguments.len() < 2 {
                    self.errors
                        .push("Math.max() requires at least 2 arguments".to_string());
                    return false;
                }
                // Generate labeled block with loop
                self.write("(blk: { var __max = ");
                if let Some(arg) = ce.arguments.first()
                    && let Some(expr) = arg.as_expression()
                {
                    self.emit_expr(expr);
                }
                self.write("; ");
                // Iterate over remaining arguments
                for (i, arg) in ce.arguments.iter().enumerate() {
                    if i == 0 {
                        continue;
                    }
                    if let Some(expr) = arg.as_expression() {
                        self.write("if (");
                        let arg_str = self.emit_expr_to_string(expr);
                        self.write(&format!("{} > __max) __max = {}; ", arg_str, arg_str));
                    }
                }
                self.write(" break :blk __max; })");
                true
            }

            builtins::BuiltinCall::MathMin => {
                // Math.min(a, b, ...) → find minimum of all arguments
                if ce.arguments.len() < 2 {
                    self.errors
                        .push("Math.min() requires at least 2 arguments".to_string());
                    return false;
                }
                // Generate labeled block with loop
                self.write("(blk: { var __min = ");
                if let Some(arg) = ce.arguments.first()
                    && let Some(expr) = arg.as_expression()
                {
                    self.emit_expr(expr);
                }
                self.write("; ");
                // Iterate over remaining arguments
                for (i, arg) in ce.arguments.iter().enumerate() {
                    if i == 0 {
                        continue;
                    }
                    if let Some(expr) = arg.as_expression() {
                        self.write("if (");
                        let arg_str = self.emit_expr_to_string(expr);
                        self.write(&format!("{} < __min) __min = {}; ", arg_str, arg_str));
                    }
                }
                self.write(" break :blk __min; })");
                true
            }

            // ── Array methods ─────────────────────────────
            builtins::BuiltinCall::ArrayPop => {
                // arr.pop() → arr.pop()
                if let Expression::StaticMemberExpression(mem) = &ce.callee
                    && let Expression::Identifier(obj) = &mem.object
                {
                    self.write(&format!("{}.pop()", obj.name.as_str()));
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArrayShift => {
                // arr.shift() → arr.shift()
                if let Expression::StaticMemberExpression(mem) = &ce.callee
                    && let Expression::Identifier(obj) = &mem.object
                {
                    self.write(&format!("{}.shift()", obj.name.as_str()));
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArrayUnshift => {
                // arr.unshift(x) → arr.unshift(x)
                if let Expression::StaticMemberExpression(mem) = &ce.callee
                    && let Expression::Identifier(obj) = &mem.object
                {
                    let obj_name = obj.name.as_str();
                    self.write(&format!("{}.unshift(", obj_name));
                    // Emit arguments
                    for (i, arg) in ce.arguments.iter().enumerate() {
                        if i > 0 {
                            self.write(", ");
                        }
                        if let Some(expr) = arg.as_expression() {
                            self.emit_expr(expr);
                        }
                    }
                    self.write(")");
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArrayReverse => {
                // arr.reverse() → arr.reverse()
                if let Expression::StaticMemberExpression(mem) = &ce.callee
                    && let Expression::Identifier(obj) = &mem.object
                {
                    self.write(&format!("{}.reverse()", obj.name.as_str()));
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArraySort => {
                // arr.sort() → arr.sort()
                if let Expression::StaticMemberExpression(mem) = &ce.callee
                    && let Expression::Identifier(obj) = &mem.object
                {
                    self.write(&format!("{}.sort()", obj.name.as_str()));
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArrayIndexOf => {
                // arr.indexOf(x) → labeled block with loop
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("Array.indexOf() requires exactly 1 argument".to_string());
                    return false;
                }
                // Redirect to String.indexOf if the object variable is a string type
                if let Expression::StaticMemberExpression(mem) = &ce.callee
                    && let Expression::Identifier(obj) = &mem.object
                {
                    if self.type_info.var_types.get(obj.name.as_str()) == Some(&ZigType::Str) {
                        // Treat as string indexOf
                        let obj_name = obj.name.as_str();
                        let arg_expr = if let Some(arg) = ce.arguments.first() {
                            if let Some(expr) = arg.as_expression() {
                                self.emit_expr_to_string(expr)
                            } else {
                                String::new()
                            }
                        } else {
                            String::new()
                        };
                        self.write(&format!(
                            "(@as(i64, @intCast(std.mem.indexOf(u8, {obj}, {arg}) orelse -1)))",
                            obj = obj_name,
                            arg = arg_expr
                        ));
                        return true;
                    }
                    let obj_name = obj.name.as_str();
                    let arg_expr = if let Some(arg) = ce.arguments.first() {
                        if let Some(expr) = arg.as_expression() {
                            self.emit_expr_to_string(expr)
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    };
                    self.write(&format!(
                            "(blk: {{ for ({obj}.items, 0..) |item, i| {{ if (item == {arg}) break :blk @as(i64, @intCast(i)); }} break :blk @as(i64, -1); }})",
                            obj = obj_name,
                            arg = arg_expr
                        ));
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArrayIncludes => {
                // arr.includes(x) → labeled block with loop
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("Array.includes() requires exactly 1 argument".to_string());
                    return false;
                }
                // Redirect to String.includes if the object variable is a string type
                if let Expression::StaticMemberExpression(mem) = &ce.callee
                    && let Expression::Identifier(obj) = &mem.object
                {
                    if self.type_info.var_types.get(obj.name.as_str()) == Some(&ZigType::Str) {
                        // Treat as string includes
                        let obj_name = obj.name.as_str();
                        let arg_expr = if let Some(arg) = ce.arguments.first() {
                            if let Some(expr) = arg.as_expression() {
                                self.emit_expr_to_string(expr)
                            } else {
                                String::new()
                            }
                        } else {
                            String::new()
                        };
                        self.write(&format!(
                            "(std.mem.indexOf(u8, {obj}, {arg}) != null)",
                            obj = obj_name,
                            arg = arg_expr
                        ));
                        return true;
                    }
                    let obj_name = obj.name.as_str();
                    let arg_expr = if let Some(arg) = ce.arguments.first() {
                        if let Some(expr) = arg.as_expression() {
                            self.emit_expr_to_string(expr)
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    };
                    self.write(&format!(
                            "(blk: {{ for ({obj}.items) |item| {{ if (item == {arg}) break :blk true; }} break :blk false; }})",
                            obj = obj_name,
                            arg = arg_expr
                        ));
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArrayJoin => {
                // arr.join(sep) → labeled block with std.io.Writer.Allocating
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("Array.join() requires exactly 1 argument".to_string());
                    return false;
                }
                if let Expression::StaticMemberExpression(mem) = &ce.callee
                    && let Expression::Identifier(obj) = &mem.object
                {
                    let obj_name = obj.name.as_str();
                    let sep_expr = if let Some(arg) = ce.arguments.first() {
                        if let Some(expr) = arg.as_expression() {
                            self.emit_expr_to_string(expr)
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    };
                    // Determine format specifier from array element type
                    let fmt_spec = match self.type_info.array_element_types.get(obj_name) {
                        Some(ZigType::I64) => "{d}",
                        Some(ZigType::F64) => "{d}",
                        Some(ZigType::Bool) => "{}",
                        Some(ZigType::Str) => "{s}",
                        _ => "{any}",
                    };
                    self.write(&format!(
                            "(blk: {{ var __join_buf = std.io.Writer.Allocating.init(js_allocator.getAllocator()); for ({obj}.items, 0..) |__item, __i| {{ if (__i > 0) __join_buf.writer().writeAll({sep}) catch break :blk \"\"; __join_buf.writer().print(\"{fmt}\", .{{__item}}) catch break :blk \"\"; }} break :blk __join_buf.toOwnedSlice() catch \"\"; }})",
                            obj = obj_name,
                            sep = sep_expr,
                            fmt = fmt_spec
                        ));
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArraySlice => {
                // arr.slice(start, end) → arr.items[start..end]
                // arr.slice(start) → arr.items[start..]
                // arr.slice() → arr.items
                if let Expression::StaticMemberExpression(mem) = &ce.callee
                    && let Expression::Identifier(obj) = &mem.object
                {
                    let obj_name = obj.name.as_str();
                    match ce.arguments.len() {
                        0 => {
                            self.write(&format!("{}.items", obj_name));
                        }
                        1 => {
                            let arg_expr = if let Some(arg) = ce.arguments.first() {
                                if let Some(expr) = arg.as_expression() {
                                    self.emit_expr_to_string(expr)
                                } else {
                                    "0".to_string()
                                }
                            } else {
                                "0".to_string()
                            };
                            self.write(&format!("{}.items[{}..]", obj_name, arg_expr));
                        }
                        2 => {
                            let start_expr = if let Some(arg) = ce.arguments.first() {
                                if let Some(expr) = arg.as_expression() {
                                    self.emit_expr_to_string(expr)
                                } else {
                                    "0".to_string()
                                }
                            } else {
                                "0".to_string()
                            };
                            let end_expr = if let Some(arg) = ce.arguments.get(1) {
                                if let Some(expr) = arg.as_expression() {
                                    self.emit_expr_to_string(expr)
                                } else {
                                    "0".to_string()
                                }
                            } else {
                                "0".to_string()
                            };
                            self.write(&format!(
                                "{}.items[{}..{}]",
                                obj_name, start_expr, end_expr
                            ));
                        }
                        _ => {
                            self.errors
                                .push("Array.slice() requires 0-2 arguments".to_string());
                            return false;
                        }
                    }
                    return true;
                }
                false
            }

            //             }

            // ── Map methods ─────────────────────────────
            builtins::BuiltinCall::MapSet => {
                // map.set(key, value) → map.set(key, value) catch unreachable
                if ce.arguments.len() != 2 {
                    self.errors
                        .push("Map.set() requires exactly 2 arguments".to_string());
                    return false;
                }
                if let Expression::StaticMemberExpression(mem) = &ce.callee
                    && let Expression::Identifier(obj) = &mem.object
                {
                    self.write(&format!("{}.set(", obj.name.as_str()));
                    // Emit key
                    if let Some(arg) = ce.arguments.first()
                        && let Some(expr) = arg.as_expression()
                    {
                        self.emit_expr(expr);
                    }
                    self.write(", ");
                    // Emit value
                    if let Some(arg) = ce.arguments.get(1)
                        && let Some(expr) = arg.as_expression()
                    {
                        self.emit_expr(expr);
                    }
                    self.write(") catch unreachable");
                    return true;
                }
                false
            }

            builtins::BuiltinCall::MapGet => {
                // map.get(key) → try map.get(key)
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("Map.get() requires exactly 1 argument".to_string());
                    return false;
                }
                if let Expression::StaticMemberExpression(mem) = &ce.callee
                    && let Expression::Identifier(obj) = &mem.object
                {
                    self.write(&format!("try {}.get(", obj.name.as_str()));
                    if let Some(arg) = ce.arguments.first()
                        && let Some(expr) = arg.as_expression()
                    {
                        self.emit_expr(expr);
                    }
                    self.write(")");
                    return true;
                }
                false
            }

            builtins::BuiltinCall::MapHas => {
                // map.has(key) → map.has(key)
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("Map.has() requires exactly 1 argument".to_string());
                    return false;
                }
                if let Expression::StaticMemberExpression(mem) = &ce.callee
                    && let Expression::Identifier(obj) = &mem.object
                {
                    self.write(&format!("{}.has(", obj.name.as_str()));
                    if let Some(arg) = ce.arguments.first()
                        && let Some(expr) = arg.as_expression()
                    {
                        self.emit_expr(expr);
                    }
                    self.write(")");
                    return true;
                }
                false
            }

            builtins::BuiltinCall::MapDelete => {
                // map.delete(key) → map.delete(key)
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("Map.delete() requires exactly 1 argument".to_string());
                    return false;
                }
                if let Expression::StaticMemberExpression(mem) = &ce.callee
                    && let Expression::Identifier(obj) = &mem.object
                {
                    self.write(&format!("{}.delete(", obj.name.as_str()));
                    if let Some(arg) = ce.arguments.first()
                        && let Some(expr) = arg.as_expression()
                    {
                        self.emit_expr(expr);
                    }
                    self.write(")");
                    return true;
                }
                false
            }

            // ── Set methods ─────────────────────────────
            builtins::BuiltinCall::SetAdd => {
                // set.add(value) → set.add(value) catch unreachable
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("Set.add() requires exactly 1 argument".to_string());
                    return false;
                }
                if let Expression::StaticMemberExpression(mem) = &ce.callee
                    && let Expression::Identifier(obj) = &mem.object
                {
                    self.write(&format!("{}.add(", obj.name.as_str()));
                    if let Some(arg) = ce.arguments.first()
                        && let Some(expr) = arg.as_expression()
                    {
                        self.emit_expr(expr);
                    }
                    self.write(") catch unreachable");
                    return true;
                }
                false
            }

            // ── String methods ─────────────────────────────
            builtins::BuiltinCall::StringIndexOf => {
                // str.indexOf(search) → std.mem.indexOf(u8, str, search)
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("String.indexOf() requires exactly 1 argument".to_string());
                    return false;
                }
                if let Expression::StaticMemberExpression(mem) = &ce.callee
                    && let Expression::StringLiteral(obj) = &mem.object
                {
                    let str_val = obj.value.as_str();
                    let arg_expr = if let Some(arg) = ce.arguments.first() {
                        if let Some(expr) = arg.as_expression() {
                            self.emit_expr_to_string(expr)
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    };
                    self.write(&format!(
                        "(@as(i64, @intCast(std.mem.indexOf(u8, \"{str_val}\", {arg}) orelse -1)))",
                        str_val = str_val,
                        arg = arg_expr
                    ));
                    return true;
                }
                // Fallback: assume object is a variable
                if let Expression::StaticMemberExpression(mem) = &ce.callee
                    && let Expression::Identifier(obj) = &mem.object
                {
                    let obj_name = obj.name.as_str();
                    let arg_expr = if let Some(arg) = ce.arguments.first() {
                        if let Some(expr) = arg.as_expression() {
                            self.emit_expr_to_string(expr)
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    };
                    self.write(&format!(
                        "(@as(i64, @intCast(std.mem.indexOf(u8, {obj}, {arg}) orelse -1)))",
                        obj = obj_name,
                        arg = arg_expr
                    ));
                    return true;
                }
                false
            }

            builtins::BuiltinCall::StringIncludes => {
                // str.includes(search) → std.mem.indexOf(u8, str, search) != null
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("String.includes() requires exactly 1 argument".to_string());
                    return false;
                }
                if let Expression::StaticMemberExpression(mem) = &ce.callee
                    && let Expression::Identifier(obj) = &mem.object
                {
                    let obj_name = obj.name.as_str();
                    let arg_expr = if let Some(arg) = ce.arguments.first() {
                        if let Some(expr) = arg.as_expression() {
                            self.emit_expr_to_string(expr)
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    };
                    self.write(&format!(
                        "(std.mem.indexOf(u8, {obj}, {arg}) != null)",
                        obj = obj_name,
                        arg = arg_expr
                    ));
                    return true;
                }
                false
            }

            builtins::BuiltinCall::StringStartsWith => {
                // str.startsWith(prefix) → std.mem.startsWith(u8, str, prefix)
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("String.startsWith() requires exactly 1 argument".to_string());
                    return false;
                }
                if let Expression::StaticMemberExpression(mem) = &ce.callee
                    && let Expression::Identifier(obj) = &mem.object
                {
                    let obj_name = obj.name.as_str();
                    let arg_expr = if let Some(arg) = ce.arguments.first() {
                        if let Some(expr) = arg.as_expression() {
                            self.emit_expr_to_string(expr)
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    };
                    self.write(&format!(
                        "std.mem.startsWith(u8, {obj}, {arg})",
                        obj = obj_name,
                        arg = arg_expr
                    ));
                    return true;
                }
                false
            }

            builtins::BuiltinCall::StringEndsWith => {
                // str.endsWith(suffix) → std.mem.endsWith(u8, str, suffix)
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("String.endsWith() requires exactly 1 argument".to_string());
                    return false;
                }
                if let Expression::StaticMemberExpression(mem) = &ce.callee
                    && let Expression::Identifier(obj) = &mem.object
                {
                    let obj_name = obj.name.as_str();
                    let arg_expr = if let Some(arg) = ce.arguments.first() {
                        if let Some(expr) = arg.as_expression() {
                            self.emit_expr_to_string(expr)
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    };
                    self.write(&format!(
                        "std.mem.endsWith(u8, {obj}, {arg})",
                        obj = obj_name,
                        arg = arg_expr
                    ));
                    return true;
                }
                false
            }

            builtins::BuiltinCall::StringTrim => {
                // str.trim() → std.mem.trim(u8, str, &std.ascii.whitespace)
                if !ce.arguments.is_empty() {
                    self.errors
                        .push("String.trim() requires no arguments".to_string());
                    return false;
                }
                if let Expression::StaticMemberExpression(mem) = &ce.callee
                    && let Expression::Identifier(obj) = &mem.object
                {
                    let obj_name = obj.name.as_str();
                    self.write(&format!(
                        "std.mem.trim(u8, {obj}, &std.ascii.whitespace)",
                        obj = obj_name
                    ));
                    return true;
                }
                false
            }

            builtins::BuiltinCall::StringSplit => {
                // str.split(sep) → std.mem.split(u8, str, sep) (returns iterator)
                // Simplified: returns array of strings
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("String.split() requires exactly 1 argument".to_string());
                    return false;
                }
                if let Expression::StaticMemberExpression(mem) = &ce.callee
                    && let Expression::Identifier(obj) = &mem.object
                {
                    let obj_name = obj.name.as_str();
                    let arg_expr = if let Some(arg) = ce.arguments.first() {
                        if let Some(expr) = arg.as_expression() {
                            self.emit_expr_to_string(expr)
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    };
                    // Generate code to split string into array
                    self.write(&format!(
                            "(blk: {{ var __split_result = std.ArrayList([]const u8).init(js_allocator.getAllocator()); var __split_iter = std.mem.split(u8, {obj}, {arg}); while (__split_iter.next()) |__part| {{ __split_result.append(__part) catch break :blk {{}}; }} break :blk __split_result.toOwnedSlice() catch &[_][]const u8{{}}; }})",
                            obj = obj_name,
                            arg = arg_expr
                        ));
                    return true;
                }
                false
            }

            // ── Array methods (with closure) ─────────────────────────────
            // These methods require closure support, which is not fully implemented yet.
            // Generate simplified implementations for now (incorrect but compilable).
            builtins::BuiltinCall::ArrayForEach => {
                // arr.forEach(fn) → for (arr.items) |_| {} (simplified: ignore fn)
                // Note: forEach is a statement in Zig (no return value)
                if let Expression::StaticMemberExpression(mem) = &ce.callee
                    && let Expression::Identifier(obj) = &mem.object
                {
                    self.write(&format!("for ({}.items) |_| {{}}", obj.name.as_str()));
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArrayMap => {
                // arr.map(fn) → arr (simplified: return original array)
                if let Expression::StaticMemberExpression(mem) = &ce.callee
                    && let Expression::Identifier(obj) = &mem.object
                {
                    self.write(obj.name.as_str());
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArrayFilter => {
                // arr.filter(fn) → arr (simplified: return original array)
                if let Expression::StaticMemberExpression(mem) = &ce.callee
                    && let Expression::Identifier(obj) = &mem.object
                {
                    self.write(obj.name.as_str());
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArrayReduce => {
                // arr.reduce(fn, init) → init (simplified: return initial value)
                if ce.arguments.len() >= 2
                    && let Some(arg) = ce.arguments.get(1)
                    && let Some(expr) = arg.as_expression()
                {
                    self.emit_expr(expr);
                    return true;
                }
                // Fallback: return 0
                self.write("0");
                true
            }

            builtins::BuiltinCall::ArraySome => {
                // arr.some(fn) → true (simplified: always return true)
                self.write("true");
                true
            }

            builtins::BuiltinCall::ArrayEvery => {
                // arr.every(fn) → true (simplified: always return true)
                self.write("true");
                true
            }
        }
    }

    /// Emit argument expression (handles spread etc.).
    fn emit_expr_arg(&mut self, arg: &Argument) {
        if let Some(e) = arg.as_expression() {
            self.emit_expr(e);
        } else {
            // Spread argument not supported yet
            self.errors
                .push("Spread argument not supported".to_string());
            self.write("/* spread arg */");
        }
    }

    // Assignment
    fn emit_assignment(&mut self, ae: &AssignmentExpression) {
        match &ae.left {
            AssignmentTarget::AssignmentTargetIdentifier(id) => {
                self.write(id.name.as_str());
            }
            AssignmentTarget::StaticMemberExpression(mem) => {
                self.emit_expr(&mem.object);
                self.write(".");
                self.write(mem.property.name.as_str());
            }
            AssignmentTarget::ComputedMemberExpression(_mem) => {
                // Dynamic property access is not allowed in strict type system.
                self.errors.push(
                    "Dynamic property assignment (obj[key] = value) is not allowed. Use static property assignment (obj.prop = value).".to_string()
                );
                self.write("/* error: dynamic property assignment */");
            }
            _ => {
                // Unsupported assignment target
                self.errors
                    .push("Unsupported assignment target".to_string());
                self.write("/* unsupported assign target */");
            }
        }
        self.write(&format!(" {} ", Self::assignment_op(ae.operator)));
        self.emit_expr(&ae.right);
    }

    // Unary expression
    fn emit_unary(&mut self, ue: &UnaryExpression) {
        match ue.operator {
            UnaryOperator::UnaryNegation | UnaryOperator::UnaryPlus | UnaryOperator::LogicalNot => {
                self.write(Self::unary_prefix(ue.operator));
                self.emit_expr(&ue.argument);
            }
            UnaryOperator::Typeof => {
                self.write("@typeName(@TypeOf(");
                self.emit_expr(&ue.argument);
                self.write("))");
            }
            _ => {
                // Unsupported unary operator (e.g., delete, void)
                self.errors.push("Unsupported unary operator".to_string());
                self.write("/* unsupported unary */");
            }
        }
    }

    // Conditional (ternary)
    fn emit_conditional(&mut self, ce: &ConditionalExpression) {
        self.write("if (");
        self.emit_expr(&ce.test);
        self.write(") ");
        self.emit_expr(&ce.consequent);
        self.write(" else ");
        self.emit_expr(&ce.alternate);
    }

    // Array expression
    fn emit_array(&mut self, ae: &ArrayExpression) {
        if ae.elements.is_empty() {
            self.write("std.ArrayList(JsAny).init(js_allocator.getAllocator())");
        } else {
            // Determine element type from first element
            let elem_type = ae
                .elements
                .first()
                .and_then(|e| e.as_expression())
                .map(|expr| match expr {
                    Expression::NumericLiteral(n) => {
                        let s = n.value.to_string();
                        if s.contains('.') || s.contains('e') || s.contains('E') {
                            "f64"
                        } else {
                            "i64"
                        }
                    }
                    Expression::StringLiteral(_) => "[]const u8",
                    Expression::BooleanLiteral(_) => "bool",
                    _ => "i64",
                })
                .unwrap_or("i64");
            self.write(&format!(
                "(blk: {{ var __arr = js_array.JsArray({}).init(js_allocator.getAllocator()); ",
                elem_type
            ));
            for elem in ae.elements.iter() {
                match elem {
                    ArrayExpressionElement::SpreadElement(_) => self.write("/* spread */"),
                    ArrayExpressionElement::Elision(_) => self.write("/* elision */"),
                    _ => {
                        if let Some(e) = elem.as_expression() {
                            self.write("__arr.append(");
                            self.emit_expr(e);
                            self.write(") catch unreachable; ");
                        }
                    }
                }
            }
            self.write("break :blk __arr; })");
        }
    }

    /// Emit an object literal as a Zig anonymous struct.
    fn emit_object(&mut self, oe: &ObjectExpression) {
        if oe.properties.is_empty() {
            // Empty object → StringHashMap(JsAny).init(js_allocator.getAllocator())
            self.write("std.StringHashMap(JsAny).init(js_allocator.getAllocator())");
            return;
        }
        self.write(".{ ");
        for (i, prop) in oe.properties.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            if let ObjectPropertyKind::ObjectProperty(p) = prop {
                let field_name = match &p.key {
                    PropertyKey::StaticIdentifier(id) => id.name.to_string(),
                    PropertyKey::StringLiteral(s) => s.value.to_string(),
                    _ => continue,
                };
                self.write(&format!(".{} = ", field_name));
                self.emit_expr(&p.value);
            }
        }
        self.write(" }");
    }
}

// ============================================================
// Phase A: Type inference has been moved to infer.rs.
// Codegen is now purely generative — it reads from
// self.type_info (TypeCheckResult) pre-computed by TypeInferrer.
// ============================================================

impl Codegen {
    /// Infer the type of an expression. Returns ZigType.
    /// If the type cannot be inferred, reports an error to self.errors
    /// and returns I64 as a fallback (the generated code will be invalid).
    /// Infer the type of an expression.
    /// Returns `Some(ZigType)` if the type can be determined (literal or binary with both literals),
    /// `None` if the type is indeterminate (Rule 1-3).
    /// Rule 1: Literal expressions → definite type
    /// Rule 2: Binary expressions → definite only if BOTH operands are literals
    /// Rule 3: Other expressions → indeterminate
    fn infer_expr_type(&mut self, expr: &Expression) -> Option<ZigType> {
        match expr {
            // Rule 1: Literals → definite type
            Expression::NumericLiteral(n) => {
                let s = n.value.to_string();
                if s.contains('.') || s.contains('e') || s.contains('E') {
                    Some(ZigType::F64)
                } else {
                    Some(ZigType::I64)
                }
            }
            Expression::StringLiteral(_) => Some(ZigType::Str),
            Expression::TemplateLiteral(_) => Some(ZigType::Str),
            Expression::BooleanLiteral(_) => Some(ZigType::Bool),
            // NullLiteral → not supported in simplified type system
            // (Zig doesn't have a direct equivalent, would need Optional)
            Expression::NullLiteral(_) => None,
            Expression::UnaryExpression(ue) => {
                // -1, +1, !true → type can be determined from operand
                match ue.operator {
                    UnaryOperator::UnaryNegation | UnaryOperator::UnaryPlus => {
                        // -x or +x: type is same as x (if x is literal)
                        if Self::is_literal(&ue.argument) {
                            self.infer_expr_type(&ue.argument)
                        } else {
                            None
                        }
                    }
                    UnaryOperator::LogicalNot => {
                        // !x → Bool
                        if Self::is_literal(&ue.argument) {
                            Some(ZigType::Bool)
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            }

            // Rule 2: Binary expression → definite only if BOTH operands are literals
            Expression::BinaryExpression(be) => {
                if Self::is_literal(&be.left) && Self::is_literal(&be.right) {
                    // Both are literals: infer types and compute result
                    let left_ty = self.infer_expr_type(&be.left).unwrap();
                    let right_ty = self.infer_expr_type(&be.right).unwrap();
                    Some(Self::infer_binary_type(be.operator, left_ty, right_ty))
                } else {
                    // Rule 3: Cannot infer type
                    None
                }
            }

            // Identifier: look up variable type from var_types (Rule 5)
            Expression::Identifier(id) => self.type_info.var_types.get(id.name.as_str()).cloned(),

            // StaticMemberExpression: look up field type from struct type (Rule 5)
            Expression::StaticMemberExpression(mem) => {
                let obj_ty = self.infer_expr_type(&mem.object);
                if let Some(ZigType::Struct(fields)) = obj_ty {
                    let field_name = mem.property.name.as_str();
                    for (name, ty) in fields {
                        if name == field_name {
                            return Some(ty.clone());
                        }
                    }
                    // Field not found: indeterminate
                    None
                } else {
                    // Object type is indeterminate: cannot infer field type
                    None
                }
            }

            // CallExpression: look up function return type from cache (Rule 5-6)
            Expression::CallExpression(ce) => {
                // Get callee name
                if let Expression::Identifier(id) = &ce.callee {
                    let fn_name = id.name.as_str();
                    // Look up return type from cache
                    if let Some(ret_ty) = self.type_info.fn_return_types.get(fn_name) {
                        return Some(ret_ty.clone());
                    }
                }
                // Cannot determine return type
                None
            }

            // ArrayExpression: if all elements are literals, infer element type
            Expression::ArrayExpression(ae) => {
                if ae.elements.is_empty() {
                    // Empty array: cannot infer element type
                    None
                } else {
                    // Infer element type from first element (if it's a literal)
                    if let Some(first_elem) = ae.elements.first() {
                        if let Some(first) = first_elem.as_expression() {
                            let elem_ty = self.infer_expr_type(first);
                            // Check all elements have the same definite type
                            for elem in ae.elements.iter().skip(1) {
                                if let Some(e) = elem.as_expression() {
                                    let et = self.infer_expr_type(e);
                                    match (&elem_ty, &et) {
                                        (Some(t1), Some(t2)) => {
                                            if *t1 != *t2 {
                                                // Type mismatch: indeterminate
                                                return None;
                                            }
                                        }
                                        _ => {
                                            // Indeterminate element: cannot infer array type
                                            return None;
                                        }
                                    }
                                } else {
                                    // Spread or other: cannot infer
                                    return None;
                                }
                            }
                            // All elements have definite, matching types
                            elem_ty.map(|t| ZigType::ArrayList(Box::new(t)))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
            }

            // ObjectExpression: if all field values are literals, infer field types
            Expression::ObjectExpression(obj) => {
                if obj.properties.is_empty() {
                    // Empty object: cannot infer type
                    None
                } else {
                    // Infer field types from literal values
                    let mut fields: Vec<(String, ZigType)> = Vec::new();
                    for prop in &obj.properties {
                        if let ObjectPropertyKind::ObjectProperty(p) = prop {
                            let field_name = match &p.key {
                                PropertyKey::StaticIdentifier(id) => id.name.to_string(),
                                PropertyKey::StringLiteral(s) => s.value.to_string(),
                                _ => {
                                    // Cannot infer field name: indeterminate
                                    return None;
                                }
                            };
                            let field_ty = self.infer_expr_type(&p.value);
                            match field_ty {
                                Some(t) => {
                                    fields.push((field_name, t));
                                }
                                None => {
                                    // Indeterminate field value: cannot infer object type
                                    return None;
                                }
                            }
                        } else {
                            // Spread property: cannot infer
                            return None;
                        }
                    }
                    Some(ZigType::Struct(fields))
                }
            }

            // Rule 3: Other expressions → indeterminate
            _ => None,
        }
    }

    /// Check if an expression is a literal (Rule 1, Rule 2).
    fn is_literal(expr: &Expression) -> bool {
        matches!(
            expr,
            Expression::NumericLiteral(_)
                | Expression::StringLiteral(_)
                | Expression::BooleanLiteral(_)
                | Expression::NullLiteral(_)
        )
    }

    /// Infer binary expression result type (both operands are literals).
    fn infer_binary_type(op: BinaryOperator, left: ZigType, right: ZigType) -> ZigType {
        match op {
            // Arithmetic operators
            BinaryOperator::Addition
            | BinaryOperator::Subtraction
            | BinaryOperator::Multiplication
            | BinaryOperator::Division
            | BinaryOperator::Remainder
            | BinaryOperator::Exponential => {
                if left == ZigType::F64 || right == ZigType::F64 {
                    ZigType::F64
                } else {
                    ZigType::I64
                }
            }
            // Comparison operators → Bool
            BinaryOperator::Equality
            | BinaryOperator::Inequality
            | BinaryOperator::StrictEquality
            | BinaryOperator::StrictInequality
            | BinaryOperator::LessThan
            | BinaryOperator::LessEqualThan
            | BinaryOperator::GreaterThan
            | BinaryOperator::GreaterEqualThan => ZigType::Bool,
            // Logical operators (for BinaryExpression, these are bitwise)
            BinaryOperator::BitwiseAnd => ZigType::I64,
            BinaryOperator::BitwiseOR => ZigType::I64,
            BinaryOperator::BitwiseXOR => ZigType::I64,
            // Shift operators
            BinaryOperator::ShiftLeft
            | BinaryOperator::ShiftRight
            | BinaryOperator::ShiftRightZeroFill => ZigType::I64,
            // Default
            _ => ZigType::I64,
        }
    }
}

// ── Helpers (methods) ──────────────────────────────

impl Codegen {
    fn binding_name<'a>(&self, pattern: &BindingPattern<'a>) -> Option<&'a str> {
        match pattern {
            BindingPattern::BindingIdentifier(id) => Some(id.name.as_str()),
            _ => None,
        }
    }

    fn binary_op(op: BinaryOperator) -> &'static str {
        match op {
            BinaryOperator::Addition => "+",
            BinaryOperator::Subtraction => "-",
            BinaryOperator::Multiplication => "*",
            BinaryOperator::Division => "/",
            BinaryOperator::Remainder => "%",
            BinaryOperator::LessThan => "<",
            BinaryOperator::GreaterThan => ">",
            BinaryOperator::LessEqualThan => "<=",
            BinaryOperator::GreaterEqualThan => ">=",
            BinaryOperator::Equality => "==",
            BinaryOperator::Inequality => "!=",
            BinaryOperator::StrictEquality => "==",
            BinaryOperator::StrictInequality => "!=",
            BinaryOperator::ShiftLeft => "<<",
            BinaryOperator::ShiftRight => ">>",
            BinaryOperator::BitwiseAnd => "&",
            BinaryOperator::BitwiseOR => "|",
            BinaryOperator::BitwiseXOR => "^",
            _ => "/* op */",
        }
    }

    fn assignment_op(op: AssignmentOperator) -> &'static str {
        match op {
            AssignmentOperator::Assign => "=",
            AssignmentOperator::Addition => "+=",
            AssignmentOperator::Subtraction => "-=",
            AssignmentOperator::Multiplication => "*=",
            AssignmentOperator::Division => "/=",
            AssignmentOperator::Remainder => "%=",
            AssignmentOperator::ShiftLeft => "<<=",
            AssignmentOperator::ShiftRight => ">>=",
            AssignmentOperator::BitwiseAnd => "&=",
            AssignmentOperator::BitwiseOR => "|=",
            AssignmentOperator::BitwiseXOR => "^=",
            _ => "=",
        }
    }

    fn logical_op(op: LogicalOperator) -> &'static str {
        match op {
            LogicalOperator::And => "and",
            LogicalOperator::Or => "or",
            LogicalOperator::Coalesce => "??",
        }
    }

    fn unary_prefix(op: UnaryOperator) -> &'static str {
        match op {
            UnaryOperator::UnaryNegation => "-",
            UnaryOperator::UnaryPlus => "+",
            UnaryOperator::LogicalNot => "!",
            _ => "",
        }
    }
}

// ── Output helpers ──────────────────────────────────

impl Codegen {
    pub fn write(&mut self, s: &str) {
        self.output.push_str(s);
    }

    pub fn push(&mut self, ch: char) {
        self.output.push(ch);
    }

    pub fn writeln(&mut self, s: &str) {
        self.write_indent();
        self.output.push_str(s);
        self.output.push('\n');
    }

    pub fn write_indent(&mut self) {
        for _ in 0..self.indent {
            self.output.push_str("    ");
        }
    }
}

// ── emit_toplevel helpers ──────────────────────────
