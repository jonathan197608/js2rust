// native_proto/codegen.rs
// All Codegen impl methods in one file.
// This avoids Rust visibility issues across multiple impl blocks in different files.

use oxc_ast::ast::*;
use crate::native_proto::{Codegen, ZigType};

// ── Constructor ─────────────────────────────────────

impl Codegen {
    pub fn new() -> Self {
        Self::default()
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
                let zig_ty = crate::native_proto::jsdoc::jsdoc_type_to_zig(&field.ty);
                self.writeln(&format!("{}: {},", field.name, zig_ty));
            }
            // Generate toJson() method for serialization using std.json.fmt()
            self.writeln("");
            self.writeln("pub fn toJson(self: *const @This(), allocator: std.mem.Allocator) ![]u8 {");
            self.indent += 1;
            // Use std.io.Writer.Allocating + std.json.fmt() for serialization
            self.writeln("var string = std.io.Writer.Allocating.init(allocator);");
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
        // Pass 0: analyze objects (detect maps and mutations).
        self.analyze_objects(program);

        // Pass 1: collect identifiers referenced in function bodies.
        self.used_names.clear();
        for stmt in &program.body {
            match stmt {
                Statement::FunctionDeclaration(fd) => {
                    Self::collect_idents_from_function(fd, &mut self.used_names);
                }
                Statement::ExportNamedDeclaration(export_decl) => {
                    // Also collect identifiers from export functions.
                    if let Some(decl) = &export_decl.declaration {
                        if let oxc_ast::ast::Declaration::FunctionDeclaration(fd) = decl {
                            Self::collect_idents_from_function(fd, &mut self.used_names);
                        }
                    }
                }
                _ => {}
            }
        }
        
        // Debug: print used_names
        println!("=== used_names: {:?}", self.used_names);
        println!("=== jsdoc_data.type_annotations: {:?}", self.jsdoc_data.as_ref().map(|d| &d.type_annotations));

        // Pass 2: emit struct typedefs (from JSDoc @typedef).
        self.emit_typedefs();

        // Pass 3: emit code, skipping unused toplevel constants.
        self.writeln("const std = @import(\"std\");");
        self.writeln("");
        for stmt in &program.body {
            self.emit_toplevel(stmt);
        }
    }

    fn emit_toplevel(&mut self, stmt: &Statement) {
        match stmt {
            Statement::VariableDeclaration(vd) => self.emit_var_decl(vd),
            Statement::FunctionDeclaration(fd) => {
                // Check if this is an export function (not wrapped in ExportNamedDeclaration)
                // For now, assume non-export (anytype params)
                let old_export = self.current_fn_is_export;
                self.current_fn_is_export = false;
                self.emit_fn(fd);
                self.current_fn_is_export = old_export;
            }
            Statement::ExportNamedDeclaration(export_decl) => {
                // Handle export function/var declarations
                match &export_decl.declaration {
                    Some(decl) => {
                        match decl {
                            oxc_ast::ast::Declaration::FunctionDeclaration(fd) => {
                                // This is an export function
                                let old_export = self.current_fn_is_export;
                                self.current_fn_is_export = true;
                                self.emit_fn(fd);
                                self.current_fn_is_export = old_export;
                            }
                            oxc_ast::ast::Declaration::VariableDeclaration(vd) => {
                                // This is an export variable
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
            if let Some(name) = self.binding_name(&decl.id) {
                let is_const = matches!(vd.kind, VariableDeclarationKind::Const);

                // Override: if the variable is mutated (assigned to a property), use 'var'.
                let is_const = is_const && !self.mutated_vars.contains(name);

                // Skip unused toplevel constants to avoid Zig unused warnings.
                // But don't skip variables with @type annotation (JSON.parse()).
                let has_type_annotation = self.jsdoc_data.as_ref()
                    .map_or(false, |d| d.type_annotations.contains_key(name));
                if self.indent == 0 && is_const && !self.used_names.contains(name) && !has_type_annotation {
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
                        // Check if this is a JSON.parse() call with @type annotation.
                        let json_parse_type = self.get_json_parse_type(name, init);

                        self.write_indent();
                        let kw = if is_const { "const" } else { "var" };

                        if let Some(type_name) = json_parse_type {
                            // JSON.parse() with @type annotation: generate std.json.parse(Type, ...)
                            self.write(&format!("{} {}: {} = std.json.parse({}, ", kw, name, type_name, type_name));
                            // Emit the argument to JSON.parse()
                            if let Expression::CallExpression(ce) = init
                                && let Some(first_arg) = ce.arguments.first() {
                                self.emit_expr_arg(first_arg);
                            }
                            self.write(") catch unreachable;\n");

                            // Store the type for later use.
                            self.var_types.insert(name.to_string(), ZigType::Struct(Vec::new()));
                        } else {
                            // Normal variable declaration with type inference.
                            let ty = self.infer_expr_type(init);

                            // Check if type inference failed.
                            if !self.errors.is_empty() {
                                // Type inference failed: default to []const u8 (string)
                                self.errors.clear(); // Clear errors, don't report
                                let default_ty = ZigType::Str;
                                self.var_types.insert(name.to_string(), default_ty.clone());
                                self.write(&format!("{} {}: []const u8 = ", kw, name));
                                self.emit_expr(init);
                                self.write(";\n");
                                continue;
                            }

                            // Store the inferred type for later use (e.g., member access).
                            self.var_types.insert(name.to_string(), ty.clone());

                            // Skip type annotation for Struct (Zig can infer it).
                            let skip_annotation = matches!(ty, ZigType::Struct(_));
                            if skip_annotation {
                                // Inferable type: let Zig infer.
                                self.write(&format!("{} {} = ", kw, name));
                            } else {
                                self.write(&format!("{} {}: {} = ", kw, name, ty.to_zig_type()));
                            }
                            self.emit_expr(init);
                            self.write(";\n");

                            // Track array element type for ArrayList push type checking.
                            if let ZigType::ArrayList(elem_ty) = &ty {
                                self.array_element_types.insert(name.to_string(), (**elem_ty).clone());
                            }
                        }
                    }
                    None => {
                        // No initializer → error in new type system.
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
        let name = fd.id.as_ref()
            .map(|id| id.name.as_str())
            .unwrap_or("anonymous");

        // Pass 1: insert parameter types into var_types.
        for param in &fd.params.items {
            if let Some(pname) = self.binding_name(&param.pattern) {
                // Default parameter type is i64.
                self.var_types.insert(pname.to_string(), ZigType::I64);
            }
        }

        // Pass 2: walk function body to collect ALL local variable types.
        if let Some(body) = &fd.body {
            // Create a temporary codegen to collect types without generating code.
            let mut type_collector = Codegen::new();
            type_collector.var_types = self.var_types.clone();
            type_collector.array_element_types = self.array_element_types.clone();

            // Walk the function body to collect variable types.
            for stmt in &body.statements {
                type_collector.walk_stmt_for_types(stmt);
            }

            // Now type_collector.var_types contains all local variable types.
            // Merge them into self.var_types.
            for (k, v) in type_collector.var_types {
                self.var_types.insert(k, v);
            }
            for (k, v) in type_collector.array_element_types {
                self.array_element_types.insert(k, v);
            }
        }

        // Pass 3: infer return type from return expressions.
        // For export functions: require @returns annotation.
        let return_exprs = Self::collect_return_exprs(fd);
        let ret_ty = if self.current_fn_is_export {
            // Export function: check for @returns annotation.
            if let Some(ref jsdoc_data) = self.jsdoc_data {
                if let Some(ret_type_name) = jsdoc_data.return_types.get(name) {
                    // Use the annotated type.
                    let zig_ty = crate::native_proto::jsdoc::jsdoc_type_to_zig(ret_type_name);
                    // Set current_fn_return_type from the annotated type.
                    self.current_fn_return_type = Some(match zig_ty.as_str() {
                        "i64" => ZigType::I64,
                        "f64" => ZigType::F64,
                        "bool" => ZigType::Bool,
                        "[]const u8" => ZigType::Str,
                        _ => ZigType::I64, // default
                    });
                    zig_ty
                } else {
                    // No @returns annotation: report error.
                    self.errors.push(format!(
                        "Export function '{}' must have @returns annotation",
                        name
                    ));
                    "[]const u8".to_string() // default for export functions
                }
            } else {
                // No JSDoc data: report error.
                self.errors.push(format!(
                    "Export function '{}' must have @returns annotation (no JSDoc data)",
                    name
                ));
                "[]const u8".to_string() // default for export functions
            }
        } else if return_exprs.is_empty() {
            "void".to_string()
        } else {
            // Use the first return expression to infer type.
            let mut ty = ZigType::I64; // default
            for expr in &return_exprs {
                let expr_ty = self.infer_expr_type(expr);
                if !self.errors.is_empty() {
                    // Type inference failed.
                    break;
                }
                if ty == ZigType::I64 {
                    ty = expr_ty;
                } else if ty != expr_ty {
                    self.errors.push(format!(
                        "Return type mismatch: expected {:?}, found {:?}",
                        ty, expr_ty
                    ));
                    break;
                }
            }
            if !self.errors.is_empty() {
                // Type inference failed: default to []const u8 (string) for non-export functions
                self.errors.clear(); // Clear errors, don't report
                if self.current_fn_is_export {
                    // Export function: still need @returns annotation
                    self.current_fn_return_type = Some(ZigType::Str);
                    "[]const u8".to_string()
                } else {
                    // Non-export function: default to string
                    self.current_fn_return_type = Some(ZigType::Str);
                    "[]const u8".to_string()
                }
            } else {
                let rt = ty.clone();
                self.current_fn_return_type = Some(rt);
                ty.to_zig_type()
            }
        };

        // Clear return type for void functions.
        if ret_ty == "void" {
            self.current_fn_return_type = None;
        }

        // Pass 4: generate function code.
        // For export functions: parameters are []const u8, return is []const u8 or void.
        // For non-export functions: use anytype for parameters, inferred return type.
        self.write(&format!("fn {}(", name));
        if self.current_fn_is_export {
            // Export function: first parameter is allocator, then []const u8 params.
            self.write("allocator: std.mem.Allocator");
            if !fd.params.items.is_empty() {
                self.write(", ");
            }
            // Also generate parameter parsing code.
            self.param_name_map.clear();
            let mut param_parsing_code = String::new();

            // Get @param annotations for this function
            let fn_param_type_map: std::collections::HashMap<String, String> = self.jsdoc_data.as_ref()
                .and_then(|data| data.param_types.get(name))
                .map(|params| params.iter().cloned().collect())
                .unwrap_or_default();

            for (i, param) in fd.params.items.iter().enumerate() {
                if i > 0 { self.write(", "); }
                if let Some(pname) = self.binding_name(&param.pattern) {
                    // Check @param annotation for this parameter
                    let param_type = fn_param_type_map.get(pname)
                        .cloned()
                        .unwrap_or("number".to_string()); // Default to number
                    
                    let zig_type = crate::native_proto::jsdoc::jsdoc_type_to_zig(&param_type);
                    
                    // Check if this is a custom type (from @typedef)
                    let is_custom_type = self.jsdoc_data.as_ref()
                        .and_then(|data| data.typedefs.get(&zig_type))
                        .is_some();
                    
                    if is_custom_type {
                        // Custom type: declare as User, no parsing needed
                        self.write(&format!("{}: {}", pname, zig_type));
                        self.param_name_map.insert(pname.to_string(), pname.to_string());
                    } else {
                        // Primitive type: declare as []const u8
                        self.write(&format!("{}: []const u8", pname));
                        
                        match zig_type.as_str() {
                            "i64" => {
                                // number → i64: parseInt
                                let parsed_name = format!("{}_int", pname);
                                self.param_name_map.insert(pname.to_string(), parsed_name.clone());
                                param_parsing_code.push_str(&format!("    const {} = try std.fmt.parseInt(i64, {}, 10);\n", parsed_name, pname));
                            }
                            "bool" => {
                                // boolean → bool: parse "true"/"false"
                                let parsed_name = format!("{}_bool", pname);
                                self.param_name_map.insert(pname.to_string(), parsed_name.clone());
                                param_parsing_code.push_str(&format!("    const {} = std.mem.eql(u8, {}, \"true\");\n", parsed_name, pname));
                            }
                            "[]const u8" => {
                                // string → []const u8: no parsing needed
                                self.param_name_map.insert(pname.to_string(), pname.to_string());
                            }
                            _ => {
                                // Default: assume i64
                                let parsed_name = format!("{}_int", pname);
                                self.param_name_map.insert(pname.to_string(), parsed_name.clone());
                                param_parsing_code.push_str(&format!("    const {} = try std.fmt.parseInt(i64, {}, 10);\n", parsed_name, pname));
                            }
                        }
                    }
                }
            }
            // Export function return type: ![]u8 (for allocPrint) or void.
            let ret_ty_str = if ret_ty == "void" {
                "void".to_string()
            } else {
                "![]u8".to_string()
            };
            self.writeln(&format!(") {} {{", ret_ty_str));
            // Generate parameter parsing code.
            if !param_parsing_code.is_empty() {
                self.write(&param_parsing_code);
            }
        } else {
            // Non-export function: use anytype for parameters.
            for (i, param) in fd.params.items.iter().enumerate() {
                if i > 0 { self.write(", "); }
                if let Some(pname) = self.binding_name(&param.pattern) {
                    self.write(&format!("{}: anytype", pname));
                }
            }
            // Use the inferred return type from Pass 3.
            let ret_ty_str = if ret_ty == "void" {
                "void".to_string()
            } else {
                ret_ty.clone()
            };
            self.writeln(&format!(") {} {{", ret_ty_str));
        }

        self.indent += 1;
        if let Some(body) = &fd.body {
            for stmt in &body.statements {
                self.emit_fn_stmt(stmt);
            }
        }
        self.indent -= 1;
        self.writeln("}");
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
                if self.current_fn_is_export {
                    // Export function: convert return value to []u8 (allocPrint returns []u8)
                    if let Some(arg) = &rs.argument {
                        match &self.current_fn_return_type {
                            Some(ZigType::I64) => {
                                // i64 → []u8: use std.fmt.allocPrint
                                self.write("return std.fmt.allocPrint(allocator, \"{}\", .{");
                                self.emit_expr(arg);
                                self.write("}) catch unreachable;\n");
                            }
                            Some(ZigType::Bool) => {
                                // bool → []const u8: return "true" or "false"
                                self.write("return if (");
                                self.emit_expr(arg);
                                self.write(") \"true\" else \"false\";\n");
                            }
                            Some(ZigType::Str) => {
                                // []const u8 → []u8: need to dupe
                                self.write("return allocator.dupe(u8, ");
                                self.emit_expr(arg);
                                self.write(") catch unreachable;\n");
                            }
                            Some(ZigType::F64) => {
                                // f64 → []u8: use std.fmt.allocPrint
                                self.write("return std.fmt.allocPrint(allocator, \"{}\", .{");
                                self.emit_expr(arg);
                                self.write("}) catch unreachable;\n");
                            }
                            _ => {
                                // Unknown type: just return as-is
                                self.write("return ");
                                self.emit_expr(arg);
                                self.write(";\n");
                            }
                        }
                    } else {
                        self.write("return;\n");
                    }
                } else {
                    self.write("return ");
                    if let Some(arg) = &rs.argument {
                        self.emit_expr(arg);
                    }
                    self.write(";\n");
                }
            }
            Statement::ExpressionStatement(es) => {
                self.write_indent();
                self.emit_expr(&es.expression);
                self.write(";\n");
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
            Statement::ForOfStatement(fos) => {
                self.emit_for_of(fos);
            }
            Statement::SwitchStatement(ss) => {
                self.emit_switch(ss);
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

    // JS:  for (const x of iterable) { ... }
    // Zig: for (iterable) |x| { ... }
    fn emit_for_of(&mut self, fos: &ForOfStatement) {
        let var_name = match &fos.left {
            ForStatementLeft::VariableDeclaration(vd) => {
                vd.declarations.first()
                    .and_then(|decl| self.binding_name(&decl.id))
                    .unwrap_or("item")
                    .to_string()
            }
            _ => "item".to_string(),
        };

        self.write_indent();
        self.write("for (");
        self.emit_expr(&fos.right);
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
                // Check if this is a parameter that was parsed (export function).
                let var_name = id.name.as_str();
                if let Some(parsed_name) = self.param_name_map.get(var_name).cloned() {
                    self.write(&parsed_name);
                } else {
                    self.write(var_name);
                }
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
            Expression::ComputedMemberExpression(_mem) => {
                // Dynamic property access is not allowed in strict type system.
                self.errors.push(
                    "Dynamic property access (obj[key]) is not allowed. Use static property access (obj.prop).".to_string()
                );
                self.write("/* error: dynamic property access */");
            }
            _ => {
                self.write("/* TODO expr */");
            }
        }
    }
}

// ── Binary / Call / Assignment / Unary / Conditional / Array ──

impl Codegen {
    // Binary expression with string-concat special case
    fn emit_binary(&mut self, be: &BinaryExpression) {
        // Check if either operand is a string type
        let left_is_string = self.expr_is_string(&be.left);
        let right_is_string = self.expr_is_string(&be.right);

        if be.operator == BinaryOperator::Addition && (left_is_string || right_is_string) {
            self.emit_expr(&be.left);
            self.write(" ++ ");
            self.emit_expr(&be.right);
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
            Expression::Identifier(id) => {
                self.var_types.get(id.name.as_str()) == Some(&ZigType::Str)
            }
            // Handle nested binary expressions: if it's string concatenation, result is string
            Expression::BinaryExpression(be)
                if be.operator == BinaryOperator::Addition => {
                    self.expr_is_string(&be.left) || self.expr_is_string(&be.right)
                }
            _ => false,
        }
    }

    // Call expression (all calls get `try`)
    fn emit_call(&mut self, ce: &CallExpression) {
        // Check if this is JSON.stringify() call
        if let Expression::StaticMemberExpression(ref mem) = ce.callee
            && let Expression::Identifier(ref obj) = mem.object
            && obj.name == "JSON" && mem.property.name == "stringify" {
            // JSON.stringify(obj) → try obj.toJson(allocator)
            if let Some(first_arg) = ce.arguments.first() {
                self.write("try ");
                self.emit_expr_arg(first_arg);
                self.write(".toJson(allocator)");
                return;
            }
        }

        // Get callee name.
        let callee_name = match &ce.callee {
            Expression::Identifier(id) => Some(id.name.to_string()),
            _ => None,
        };

        // All function calls use `try`.
        self.write("try ");
        if let Some(ref name) = callee_name {
            self.write(name);
        } else {
            self.emit_expr(&ce.callee);
        }
        self.write("(");
        for (i, arg) in ce.arguments.iter().enumerate() {
            if i > 0 { self.write(", "); }
            self.emit_expr_arg(arg);
        }
        self.write(")");
    }

    /// Emit argument expression (handles spread etc.).
    fn emit_expr_arg(&mut self, arg: &Argument) {
        if let Some(e) = arg.as_expression() {
            self.emit_expr(e);
        } else {
            self.write("/* TODO arg */");
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
            _ => self.write("/* TODO assign target */"),
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
                self.write("/* TODO unary */");
                self.emit_expr(&ue.argument);
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
            self.write("std.ArrayList(JsAny).init(allocator)");
        } else {
            self.write(".{");
            for (i, elem) in ae.elements.iter().enumerate() {
                if i > 0 { self.write(", "); }
                match elem {
                    ArrayExpressionElement::SpreadElement(_) => self.write("/* spread */"),
                    ArrayExpressionElement::Elision(_) => self.write("undefined"),
                    _ => {
                        // Inherited from Expression — use as_expression().
                        if let Some(e) = elem.as_expression() {
                            self.emit_expr(e);
                        }
                    },
                }
            }
            self.push('}');
        }
    }

    /// Emit an object literal as a Zig anonymous struct.
    fn emit_object(&mut self, oe: &ObjectExpression) {
        if oe.properties.is_empty() {
            // Empty object → StringHashMap(JsAny).init(allocator)
            self.write("std.StringHashMap(JsAny).init(allocator)");
            return;
        }
        self.write(".{ ");
        for (i, prop) in oe.properties.iter().enumerate() {
            if i > 0 { self.write(", "); }
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

// ── Type inference (ZigType) ───────────────────────

impl Codegen {
    /// Infer the type of an expression. Returns ZigType.
    /// If the type cannot be inferred, reports an error to self.errors
    /// and returns I64 as a fallback (the generated code will be invalid).
    fn infer_expr_type(&mut self, expr: &Expression) -> ZigType {
        match expr {
            Expression::NumericLiteral(n) => {
                let s = n.value.to_string();
                if s.contains('.') || s.contains('e') || s.contains('E') {
                    ZigType::F64
                } else {
                    ZigType::I64
                }
            }
            Expression::StringLiteral(_) => ZigType::Str,
            Expression::BooleanLiteral(_) => ZigType::Bool,
            Expression::Identifier(id) => {
                // Look up the variable's type from var_types.
                if let Some(ty) = self.var_types.get(id.name.as_str()) {
                    ty.clone()
                } else {
                    // Cannot infer type: this is ok in permissive mode,
                    // but should be an error in strict mode.
                    // For now, default to I64 to allow code generation.
                    ZigType::I64 // default fallback
                }
            }
            Expression::BinaryExpression(be) => {
                let left_ty = self.infer_expr_type(&be.left);
                let right_ty = self.infer_expr_type(&be.right);
                self.infer_binary_type(be.operator, left_ty, right_ty)
            }
            Expression::LogicalExpression(_) => ZigType::Bool,
            Expression::ArrayExpression(ae) => {
                if ae.elements.is_empty() {
                    // Error: cannot infer element type for empty array.
                    self.errors.push(
                        "Cannot infer element type for empty array. Use ArrayList with explicit type.".to_string()
                    );
                    // Return ArrayList(I64) as fallback.
                    ZigType::ArrayList(Box::new(ZigType::I64))
                } else {
                    // Infer from first element, then check all elements have the same type.
                    if let Some(first_elem) = ae.elements.first() {
                        if let Some(first) = first_elem.as_expression() {
                            let elem_ty = self.infer_expr_type(first);
                            // Check all elements have the same type.
                            for elem in ae.elements.iter().skip(1) {
                                if let Some(e) = elem.as_expression() {
                                    let ty = self.infer_expr_type(e);
                                    if ty != elem_ty {
                                        self.errors.push(format!(
                                            "Array elements must have the same type. Expected {:?}, found {:?}",
                                            elem_ty, ty
                                        ));
                                    }
                                }
                            }
                            ZigType::ArrayList(Box::new(elem_ty))
                        } else {
                            self.errors.push("Cannot infer array element type (spread/not supported)".to_string());
                            ZigType::ArrayList(Box::new(ZigType::I64))
                        }
                    } else {
                        ZigType::ArrayList(Box::new(ZigType::I64))
                    }
                }
            }
            Expression::ObjectExpression(obj) => {
                if obj.properties.is_empty() {
                    // Error: empty object literal is not allowed in strict type system.
                    self.errors.push(
                        "Empty object literal is not allowed. Use a typed struct.".to_string()
                    );
                    // Return a dummy struct as fallback.
                    ZigType::Struct(Vec::new())
                } else {
                    // Generate struct type from properties.
                    let mut fields = Vec::new();
                    for prop in &obj.properties {
                        if let ObjectPropertyKind::ObjectProperty(p) = prop {
                            let field_name = match &p.key {
                                PropertyKey::StaticIdentifier(id) => id.name.to_string(),
                                PropertyKey::StringLiteral(s) => s.value.to_string(),
                                _ => {
                                    self.errors.push("Unsupported property key type".to_string());
                                    continue;
                                }
                            };
                            let field_ty = self.infer_expr_type(&p.value);
                            fields.push((field_name, field_ty));
                        }
                    }
                    ZigType::Struct(fields)
                }
            }
            Expression::CallExpression(_ce) => {
                // For function calls, return i64 as default (simplified).
                // In a real implementation, we would look up the function's return type.
                ZigType::I64 // default fallback
            }
            Expression::StaticMemberExpression(mem) => {
                // For obj.prop, infer the type of prop based on the object's type.
                let obj_ty = self.infer_expr_type(&mem.object);
                match obj_ty {
                    ZigType::Struct(fields) => {
                        // Look up the field type.
                        let field_name = mem.property.name.as_str();
                        for (name, ty) in fields {
                            if name == field_name {
                                return ty.clone();
                            }
                        }
                        // Field not found: report error.
                        self.errors.push(format!(
                            "Field '{}' not found in struct",
                            field_name
                        ));
                        ZigType::I64 // fallback
                    }
                    _ => {
                        // Not a struct: cannot infer field type.
                        // For anytype parameters, just return a placeholder type.
                        // Don't report an error - let Zig handle type checking at compile time.
                        ZigType::I64 // placeholder, actual type checked by Zig
                    }
                }
            }
            Expression::UnaryExpression(ue) => {
                // For unary expressions (-x, !x, etc.), the type is the same as the operand's type.
                let operand_ty = self.infer_expr_type(&ue.argument);
                match ue.operator {
                    UnaryOperator::UnaryNegation => {
                        // -x: type is same as x
                        operand_ty
                    }
                    UnaryOperator::UnaryPlus => {
                        // +x: type is same as x
                        operand_ty
                    }
                    UnaryOperator::LogicalNot => {
                        // !x: type is bool
                        ZigType::Bool
                    }
                    _ => {
                        // Other unary operators: return operand type
                        operand_ty
                    }
                }
            }
            _ => {
                // Unsupported expression: report error.
                self.errors.push(format!(
                    "Unsupported expression for type inference: {:?}",
                    std::any::type_name::<Expression>()
                ));
                ZigType::I64 // fallback
            }
        }
    }

    /// Infer the result type of a binary expression.
    fn infer_binary_type(&mut self, op: BinaryOperator, left: ZigType, right: ZigType) -> ZigType {
        match op {
            // Arithmetic operators.
            BinaryOperator::Addition | BinaryOperator::Subtraction |
            BinaryOperator::Multiplication | BinaryOperator::Division => {
                // String concatenation.
                if left == ZigType::Str || right == ZigType::Str {
                    return ZigType::Str;
                }
                // If either operand is f64, result is f64.
                if left == ZigType::F64 || right == ZigType::F64 {
                    ZigType::F64
                } else {
                    ZigType::I64
                }
            }
            // Comparison operators → bool.
            BinaryOperator::Equality | BinaryOperator::Inequality |
            BinaryOperator::LessThan | BinaryOperator::LessEqualThan |
            BinaryOperator::GreaterThan | BinaryOperator::GreaterEqualThan => {
                ZigType::Bool
            }
            // Default: error.
            _ => {
                self.errors.push(format!(
                    "Unsupported binary operator: {:?}",
                    op
                ));
                ZigType::I64 // fallback
            }
        }
    }
}

// ── Return expression collection ─────────────────────

impl Codegen {
    fn collect_return_exprs<'a>(fd: &'a Function<'a>) -> Vec<&'a Expression<'a>> {
        let mut exprs = Vec::new();
        if let Some(body) = &fd.body {
            for stmt in &body.statements {
                Self::collect_returns(stmt, &mut exprs);
            }
        }
        exprs
    }

    fn collect_returns<'a>(stmt: &'a Statement<'a>, exprs: &mut Vec<&'a Expression<'a>>) {
        match stmt {
            Statement::ReturnStatement(rs) => {
                if let Some(ref arg) = rs.argument {
                    exprs.push(arg);
                }
            }
            Statement::IfStatement(is) => {
                Self::collect_returns(&is.consequent, exprs);
                if let Some(alt) = &is.alternate {
                    Self::collect_returns(alt, exprs);
                }
            }
            Statement::BlockStatement(bs) => {
                for stmt in &bs.body {
                    Self::collect_returns(stmt, exprs);
                }
            }
            Statement::WhileStatement(ws) => {
                Self::collect_returns(&ws.body, exprs);
            }
            _ => {}
        }
    }
}

// ── Identifier collection (for unused-constant elimination) ──

impl Codegen {
    /// Walk a function and collect all identifier names referenced in its body.
    /// This is used to determine which toplevel constants are actually used.
    fn collect_idents_from_function<'a>(fd: &'a Function<'a>, names: &mut std::collections::HashSet<String>) {
        if let Some(body) = &fd.body {
            for stmt in &body.statements {
                Self::collect_idents_from_stmt(stmt, names);
            }
        }
    }

    fn collect_idents_from_stmt<'a>(stmt: &'a Statement<'a>, names: &mut std::collections::HashSet<String>) {
        match stmt {
            Statement::ExpressionStatement(es) => {
                Self::collect_idents_from_expr(&es.expression, names);
            }
            Statement::ReturnStatement(rs) => {
                if let Some(arg) = &rs.argument {
                    Self::collect_idents_from_expr(arg, names);
                }
            }
            Statement::IfStatement(is) => {
                Self::collect_idents_from_expr(&is.test, names);
                Self::collect_idents_from_stmt(&is.consequent, names);
                if let Some(alt) = &is.alternate {
                    Self::collect_idents_from_stmt(alt, names);
                }
            }
            Statement::WhileStatement(ws) => {
                Self::collect_idents_from_expr(&ws.test, names);
                Self::collect_idents_from_stmt(&ws.body, names);
            }
            Statement::BlockStatement(bs) => {
                for s in &bs.body {
                    Self::collect_idents_from_stmt(s, names);
                }
            }
            Statement::VariableDeclaration(vd) => {
                for decl in &vd.declarations {
                    if let Some(init) = &decl.init {
                        Self::collect_idents_from_expr(init, names);
                    }
                }
            }
            _ => {}
        }
    }

    fn collect_idents_from_expr<'a>(expr: &'a Expression<'a>, names: &mut std::collections::HashSet<String>) {
        match expr {
            Expression::Identifier(id) => {
                names.insert(id.name.to_string());
            }
            Expression::BinaryExpression(be) => {
                Self::collect_idents_from_expr(&be.left, names);
                Self::collect_idents_from_expr(&be.right, names);
            }
            Expression::CallExpression(ce) => {
                Self::collect_idents_from_expr(&ce.callee, names);
                for arg in &ce.arguments {
                    if let Some(e) = arg.as_expression() {
                        Self::collect_idents_from_expr(e, names);
                    }
                }
            }
            Expression::AssignmentExpression(ae) => {
                // For `x = expr`, collect idents from both sides.
                // The left side (target) may be an identifier.
                if let AssignmentTarget::AssignmentTargetIdentifier(id) = &ae.left {
                    names.insert(id.name.to_string());
                }
                Self::collect_idents_from_expr(&ae.right, names);
            }
            Expression::UnaryExpression(ue) => {
                Self::collect_idents_from_expr(&ue.argument, names);
            }
            Expression::LogicalExpression(le) => {
                Self::collect_idents_from_expr(&le.left, names);
                Self::collect_idents_from_expr(&le.right, names);
            }
            Expression::ParenthesizedExpression(pe) => {
                Self::collect_idents_from_expr(&pe.expression, names);
            }
            Expression::ConditionalExpression(ce) => {
                Self::collect_idents_from_expr(&ce.test, names);
                Self::collect_idents_from_expr(&ce.consequent, names);
                Self::collect_idents_from_expr(&ce.alternate, names);
            }
            Expression::ArrayExpression(ae) => {
                for elem in &ae.elements {
                    if let Some(e) = elem.as_expression() {
                        Self::collect_idents_from_expr(e, names);
                    }
                }
            }
            _ => {}
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

    /// Check if an initializer is a JSON.parse() call and return the @type annotation if present.
    /// Returns Some(type_name) if this is JSON.parse() with @type annotation, None otherwise.
    fn get_json_parse_type(&self, var_name: &str, init: &Expression) -> Option<String> {
        // Check if init is a CallExpression
        let ce = if let Expression::CallExpression(ce) = init {
            ce
        } else {
            return None;
        };

        // Check if callee is JSON.parse
        let is_json_parse = if let Expression::StaticMemberExpression(mem) = &ce.callee {
            // Check if object is Identifier "JSON" and property is "parse"
            if let Expression::Identifier(obj_id) = &mem.object {
                obj_id.name.as_str() == "JSON" && mem.property.name.as_str() == "parse"
            } else {
                false
            }
        } else {
            false
        };

        if !is_json_parse {
            return None;
        }

        // Look up @type annotation for this variable
        if let Some(ref jsdoc_data) = self.jsdoc_data
            && let Some(type_name) = jsdoc_data.type_annotations.get(var_name) {
            return Some(type_name.clone());
        }

        None
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

// ── Object analysis (Pass 0) ─────────────────────

impl Codegen {
    /// Analyze the program to detect object kinds (struct vs map) and mutations.
    pub fn analyze_objects(&mut self, program: &Program) {
        for stmt in &program.body {
            self.walk_stmt_for_analysis(stmt);
        }
    }

    /// Walk a statement for analysis.
    fn walk_stmt_for_analysis(&mut self, stmt: &Statement) {
        match stmt {
            Statement::VariableDeclaration(vd) => {
                for decl in &vd.declarations {
                    if let Some(init) = &decl.init {
                        self.walk_expr_for_analysis(init);
                    }
                }
            }
            Statement::FunctionDeclaration(fd) => {
                if let Some(body) = &fd.body {
                    for stmt in &body.statements {
                        self.walk_stmt_for_analysis(stmt);
                    }
                }
            }
            Statement::ExpressionStatement(es) => {
                self.walk_expr_for_analysis(&es.expression);
            }
            Statement::IfStatement(is) => {
                self.walk_expr_for_analysis(&is.test);
                self.walk_stmt_for_analysis(&is.consequent);
                if let Some(alt) = &is.alternate {
                    self.walk_stmt_for_analysis(alt);
                }
            }
            Statement::WhileStatement(ws) => {
                self.walk_expr_for_analysis(&ws.test);
                self.walk_stmt_for_analysis(&ws.body);
            }
            Statement::BlockStatement(bs) => {
                for stmt in &bs.body {
                    self.walk_stmt_for_analysis(stmt);
                }
            }
            _ => {}
        }
    }

    /// Walk an expression for analysis (detect ComputedMemberExpression and assignments).
    fn walk_expr_for_analysis(&mut self, expr: &Expression) {
        match expr {
            Expression::ComputedMemberExpression(mem) => {
                // Dynamic property access is not allowed in strict type system.
                self.errors.push(
                    "Dynamic property access (obj[key]) is not allowed. Use static property access (obj.prop).".to_string()
                );
                // Still walk into sub-expressions to find more errors.
                self.walk_expr_for_analysis(&mem.object);
                self.walk_expr_for_analysis(&mem.expression);
            }
            Expression::StaticMemberExpression(mem) => {
                self.walk_expr_for_analysis(&mem.object);
            }
            Expression::AssignmentExpression(ae) => {
                // Check assignment target for mutation.
                self.check_assignment_target(&ae.left);
                self.walk_expr_for_analysis(&ae.right);
            }
            Expression::BinaryExpression(be) => {
                self.walk_expr_for_analysis(&be.left);
                self.walk_expr_for_analysis(&be.right);
            }
            Expression::CallExpression(ce) => {
                self.walk_expr_for_analysis(&ce.callee);
                for arg in &ce.arguments {
                    if let Some(e) = arg.as_expression() {
                        self.walk_expr_for_analysis(e);
                    }
                }
            }
            Expression::ParenthesizedExpression(pe) => {
                self.walk_expr_for_analysis(&pe.expression);
            }
            Expression::ConditionalExpression(ce) => {
                self.walk_expr_for_analysis(&ce.test);
                self.walk_expr_for_analysis(&ce.consequent);
                self.walk_expr_for_analysis(&ce.alternate);
            }
            Expression::UnaryExpression(ue) => {
                self.walk_expr_for_analysis(&ue.argument);
            }
            Expression::LogicalExpression(le) => {
                self.walk_expr_for_analysis(&le.left);
                self.walk_expr_for_analysis(&le.right);
            }
            Expression::ArrayExpression(ae) => {
                for elem in &ae.elements {
                    if let Some(e) = elem.as_expression() {
                        self.walk_expr_for_analysis(e);
                    }
                }
            }
            Expression::ObjectExpression(oe) => {
                for prop in &oe.properties {
                    if let ObjectPropertyKind::ObjectProperty(p) = prop {
                        self.walk_expr_for_analysis(&p.value);
                    }
                }
            }
            _ => {}
        }
    }

    /// Check if an assignment target is a member expression, mark as mutated.
    fn check_assignment_target(&mut self, target: &AssignmentTarget) {
        match target {
            AssignmentTarget::StaticMemberExpression(mem) => {
                if let Expression::Identifier(id) = &mem.object {
                    self.mutated_vars.insert(id.name.to_string());
                }
            }
            AssignmentTarget::ComputedMemberExpression(mem) => {
                // Dynamic property assignment is not allowed, but we still mark as mutated for error reporting.
                if let Expression::Identifier(id) = &mem.object {
                    self.mutated_vars.insert(id.name.to_string());
                }
            }
            _ => {}
        }
    }
}

// ── Type collection (Pass 2) ─────────────────────

impl Codegen {
    /// Walk a statement to collect variable types (without generating code).
    /// This is used to populate `var_types` before code generation.
    fn walk_stmt_for_types(&mut self, stmt: &Statement) {
        match stmt {
            Statement::VariableDeclaration(vd) => {
                for decl in &vd.declarations {
                    if let Some(name) = self.binding_name(&decl.id) {
                        if let Some(init) = &decl.init {
                            let ty = self.infer_expr_type(init);
                            self.var_types.insert(name.to_string(), ty.clone());

                            // Track array element type for ArrayList push type checking.
                            if let ZigType::ArrayList(elem_ty) = &ty {
                                self.array_element_types.insert(name.to_string(), (**elem_ty).clone());
                            }
                        } else {
                            // No initializer → error in strict type system.
                            self.errors.push(format!(
                                "Variable '{}' must be initialized (strict type system)",
                                name
                            ));
                        }
                    }
                }
            }
            Statement::IfStatement(is) => {
                self.walk_expr_for_analysis(&is.test); // Check for errors
                self.walk_stmt_for_types(&is.consequent);
                if let Some(alt) = &is.alternate {
                    self.walk_stmt_for_types(alt);
                }
            }
            Statement::WhileStatement(ws) => {
                self.walk_expr_for_analysis(&ws.test); // Check for errors
                self.walk_stmt_for_types(&ws.body);
            }
            Statement::BlockStatement(bs) => {
                for stmt in &bs.body {
                    self.walk_stmt_for_types(stmt);
                }
            }
            Statement::FunctionDeclaration(fd) => {
                // Nested function: collect its parameter types.
                for param in &fd.params.items {
                    if let Some(pname) = self.binding_name(&param.pattern) {
                        self.var_types.insert(pname.to_string(), ZigType::I64);
                    }
                }
                // Walk the function body.
                if let Some(body) = &fd.body {
                    for stmt in &body.statements {
                        self.walk_stmt_for_types(stmt);
                    }
                }
            }
            _ => {}
        }
    }
}
