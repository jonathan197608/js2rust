// Class declaration codegen: struct definition, constructor, methods, field init.

use super::Codegen;
use crate::types::ZigType;
use oxc_ast::ast::*;

/// Extract the string name from a PropertyKey (IdentifierName or StringLiteral).
pub(crate) fn property_key_name(key: &PropertyKey) -> Option<String> {
    match key {
        PropertyKey::StaticIdentifier(id) => Some(id.name.to_string()),
        PropertyKey::StringLiteral(sl) => Some(sl.value.to_string()),
        PropertyKey::PrivateIdentifier(id) => Some(id.name.to_string()),
        _ => None,
    }
}

/// Check if a MethodDefinition is `constructor()`.
fn is_constructor_method(md: &MethodDefinition) -> bool {
    if let Some(name) = property_key_name(&md.key) {
        name == "constructor"
    } else {
        false
    }
}

// ── Class declaration helpers ────────────────────────

/// Convert a simple expression to a Zig default value string.
/// Used for class field default values (e.g., `#secret = 42` → "42").
fn expr_to_default_str(expr: &Expression) -> String {
    match expr {
        Expression::NumericLiteral(n) => format!("{}", n.value),
        Expression::StringLiteral(s) => format!("\"{}\"", s.value),
        Expression::BooleanLiteral(b) => format!("{}", b.value),
        Expression::NullLiteral(_) => "null".to_string(),
        _ => "0".to_string(), // fallback: use zero
    }
}

// ── Class declaration codegen ────────────────────────

impl Codegen {
    /// Emit a class declaration as a Zig struct with methods.
    pub(crate) fn emit_class(&mut self, cd: &Class) {
        let class_name = cd
            .id
            .as_ref()
            .map(|id| id.name.as_str())
            .unwrap_or("AnonymousClass");
        let class_name_s = class_name.to_string();
        let safe_class_name = self.zig_safe_name(class_name);

        // Collect fields and methods from the class body
        let mut field_names: Vec<String> = Vec::new();
        let mut field_types: Vec<ZigType> = Vec::new();
        let mut field_defaults: Vec<Option<String>> = Vec::new();
        let mut static_field_names: Vec<String> = Vec::new();
        let mut has_constructor = false;

        // First pass: collect field names from property definitions
        for elem in &cd.body.body {
            match elem {
                ClassElement::PropertyDefinition(pd) => {
                    let is_static = pd.r#static;
                    let is_computed = pd.computed;
                    if is_computed {
                        continue; // skip computed properties
                    }
                    if let Some(name) = property_key_name(&pd.key) {
                        if is_static {
                            if !static_field_names.contains(&name) {
                                static_field_names.push(name);
                            }
                        } else if !field_names.contains(&name) {
                            // Look up field type from TypeInferrer first (before name is moved)
                            let field_ty = self
                                .type_info
                                .class_field_types
                                .get(&class_name_s)
                                .and_then(|fields| fields.get(&name))
                                .cloned()
                                .unwrap_or(ZigType::I64);
                            // Extract default value from initializer (e.g., `secret = 42` → "42")
                            let default_val = pd.value.as_ref().map(expr_to_default_str);
                            field_names.push(name);
                            field_types.push(field_ty);
                            field_defaults.push(default_val);
                        }
                    }
                }
                ClassElement::MethodDefinition(md) if is_constructor_method(md) => {
                    has_constructor = true;
                }
                _ => {}
            }
        }

        // Second pass: scan constructor body for `this.x = expr` implicit fields
        {
            let mut constructor_stmts: Option<&[Statement]> = None;
            for elem in &cd.body.body {
                if let ClassElement::MethodDefinition(md) = elem
                    && is_constructor_method(md)
                    && let Some(body) = &md.value.body
                {
                    constructor_stmts = Some(&body.statements);
                    break;
                }
            }
            if let Some(body_stmts) = constructor_stmts {
                self.collect_implicit_class_fields(
                    body_stmts,
                    &class_name_s,
                    &mut field_names,
                    &mut field_types,
                    &mut field_defaults,
                );
            }
        }

        self.class_names.insert(class_name_s.clone());

        // ── Generate struct definition ──
        self.writeln(&format!("const {} = struct {{", safe_class_name));

        // Emit struct fields
        self.indent += 1;
        for (i, fname) in field_names.iter().enumerate() {
            let ftype = &field_types[i];
            self.writeln(&format!("{}: {},", fname, ftype.to_zig_type()));
        }
        self.writeln("");

        // Save current state
        let saved_class = self.current_class.take();

        // Emit methods (constructor → init, regular methods → pub fn methodName)
        for elem in &cd.body.body {
            match elem {
                ClassElement::MethodDefinition(md) => {
                    self.emit_class_method(class_name, &field_names, md);
                }
                ClassElement::PropertyDefinition(pd)
                    // Static property initializers
                    if pd.r#static && !pd.computed => {
                        self.emit_static_field_init(pd);
                    }
                // 🔘 static {} blocks: not supported — generate @compileError
                ClassElement::StaticBlock(sb) => {
                    self.compile_error_stmt(
                        sb.span,
                        "static {} blocks are not supported. Use static field initializers instead.",
                    );
                }
                _ => {}
            }
        }

        // If no explicit constructor, generate a default init()
        if !has_constructor {
            self.writeln("");
            self.writeln(&format!("pub fn init() {} {{", safe_class_name));
            self.indent += 1;
            if field_names.is_empty() {
                self.writeln("return .{};");
            } else {
                let indent = "    ".repeat(self.indent);
                self.write(&format!("{}return .{{ ", indent));
                for (i, fname) in field_names.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    let default_val = field_defaults
                        .get(i)
                        .and_then(|d| d.as_deref())
                        .unwrap_or("0");
                    self.write(&format!(".{} = {}", fname, default_val));
                }
                self.write(" };\n");
            }
            self.indent -= 1;
            self.writeln("}");
        }

        // Restore state
        self.current_class = saved_class;

        self.indent -= 1;
        self.writeln("};");
        self.writeln("");
    }

    /// Recursively scan constructor body for `this.x = expr` assignments
    /// and add discovered fields to field_names/field_types (if not already present).
    fn collect_implicit_class_fields(
        &self,
        stmts: &[Statement],
        class_name: &str,
        field_names: &mut Vec<String>,
        field_types: &mut Vec<ZigType>,
        field_defaults: &mut Vec<Option<String>>,
    ) {
        for stmt in stmts {
            match stmt {
                Statement::ExpressionStatement(es) => {
                    if let Expression::AssignmentExpression(ae) = &es.expression
                        && let AssignmentTarget::StaticMemberExpression(sme) = &ae.left
                        && matches!(&sme.object, Expression::ThisExpression(_))
                    {
                        let fname = sme.property.name.to_string();
                        if !field_names.contains(&fname) {
                            let ftype = self
                                .type_info
                                .class_field_types
                                .get(class_name)
                                .and_then(|fields| fields.get(&fname))
                                .cloned()
                                .unwrap_or(ZigType::I64);
                            field_names.push(fname);
                            field_types.push(ftype);
                            field_defaults.push(None); // set by constructor, no static default
                        }
                    }
                }
                Statement::IfStatement(is) => {
                    self.collect_implicit_class_fields(
                        std::slice::from_ref(&is.consequent),
                        class_name,
                        field_names,
                        field_types,
                        field_defaults,
                    );
                    if let Some(alt) = &is.alternate {
                        self.collect_implicit_class_fields(
                            std::slice::from_ref(alt),
                            class_name,
                            field_names,
                            field_types,
                            field_defaults,
                        );
                    }
                }
                Statement::BlockStatement(bs) => {
                    self.collect_implicit_class_fields(
                        &bs.body,
                        class_name,
                        field_names,
                        field_types,
                        field_defaults,
                    );
                }
                _ => {}
            }
        }
    }

    /// Emit a class method (or constructor) as part of a struct.
    fn emit_class_method(
        &mut self,
        class_name: &str,
        field_names: &[String],
        md: &MethodDefinition,
    ) {
        let method_name = property_key_name(&md.key).unwrap_or_else(|| "anonymous".to_string());

        // Set current_class so `this.x` → `self.x` rewriting activates
        self.current_class = Some(class_name.to_string());

        if is_constructor_method(md) {
            // constructor → pub fn init(...)
            // The function body needs `return .{ .field1 = val1, ... };` if no explicit return
            self.emit_class_constructor(class_name, field_names, &md.value);
        } else {
            // Regular method → pub fn methodName(self: @This(), ...)
            self.emit_class_regular_method(class_name, &method_name, &md.value);
        }

        self.writeln("");
        self.current_class = None;
    }

    /// Emit the class constructor as `pub fn init(...) ClassName { ... }`.
    fn emit_class_constructor(
        &mut self,
        class_name: &str,
        field_names: &[String],
        func: &oxc_allocator::Box<'_, Function>,
    ) {
        // Check if constructor has a body
        let body_stmts = match &func.body {
            Some(body) => &body.statements,
            None => &[] as &[_],
        };

        // Check if body has explicit return
        let has_return = body_stmts
            .iter()
            .any(|s| matches!(s, Statement::ReturnStatement(_)));

        let saved_fn = self.current_fn.take();
        self.current_fn = Some("init".to_string());

        // Generate function signature
        // pub fn init(...) ClassName {
        self.writeln("");
        self.write("pub fn init(");

        // Parameters
        let mut param_idx = 0;
        for param in &func.params.items {
            if let Some(pname) = crate::infer::binding_name(&param.pattern) {
                if param_idx > 0 {
                    self.write(", ");
                }
                // Read param type from type_info if available
                let ptype = self
                    .type_info
                    .fn_param_types
                    .get("init")
                    .and_then(|params| {
                        params
                            .iter()
                            .find(|(n, _)| n == pname)
                            .map(|(_, t)| t.clone())
                    })
                    .unwrap_or(ZigType::Anytype);
                self.write(&format!("{}: {}", pname, ptype.to_zig_type()));
                param_idx += 1;
            }
        }

        let safe_class_name = self.zig_safe_name(class_name);
        self.writeln(&format!(") {} {{", safe_class_name));

        // Emit body
        self.indent += 1;

        // If no explicit return, generate the struct return at the end
        // (fields are initialized in the body via `this.x = val` or `return .{...}`)
        if !has_return {
            // First emit the body statements (which may set local variables)
            for stmt in body_stmts {
                self.emit_stmt_with_this_rewrite(stmt, field_names);
            }

            // Then generate the struct return using field values
            // We assume variables with same names as fields exist
            self.write("return .{ ");
            for (i, fname) in field_names.iter().enumerate() {
                if i > 0 {
                    self.write(", ");
                }
                self.write(&format!(".{} = {}", fname, fname));
            }
            self.writeln(" };");
        } else {
            // Has explicit return — emit body as-is with this→self rewriting
            for stmt in body_stmts {
                self.emit_stmt_with_this_rewrite(stmt, field_names);
            }
        }

        self.indent -= 1;
        self.writeln("}");

        self.current_fn = saved_fn;
    }

    /// Emit a regular class method as `pub fn methodName(self: @This(), ...) RetTy { ... }`.
    fn emit_class_regular_method(
        &mut self,
        class_name: &str,
        method_name: &str,
        func: &oxc_allocator::Box<'_, Function>,
    ) {
        let saved_fn = self.current_fn.take();
        self.current_fn = Some(method_name.to_string());

        // Build fully-qualified key for TypeInferrer lookups
        let fq_method = format!("{}.{}", class_name, method_name);

        // Resolve return type:
        // 1. Check JSDoc @returns annotation
        // 2. Check TypeInferrer return types (fully-qualified key)
        // 3. Quick body scan for return statements
        // 4. Default to void
        let ret_ty = self
            .jsdoc_data
            .as_ref()
            .and_then(|d| d.return_types.get(method_name))
            .cloned()
            .or_else(|| {
                self.type_info
                    .fn_return_types
                    .get(&fq_method)
                    .map(|t| t.to_zig_type())
            })
            .or_else(|| {
                // Quick body scan: find return statement and infer type
                func.body.as_ref().and_then(|body| {
                    body.statements.iter().find_map(|s| {
                        if let Statement::ReturnStatement(rs) = s
                            && let Some(ret_expr) = &rs.argument
                        {
                            scan_ret_expr_type(ret_expr)
                        } else {
                            None
                        }
                    })
                })
            })
            .unwrap_or_else(|| "void".to_string());

        // Generate signature
        self.write(&format!("pub fn {}(self: @This()", method_name));

        // Parameters (skip self)
        // Look up with fully-qualified key first, fall back to plain method name
        let param_list = self
            .type_info
            .fn_param_types
            .get(&fq_method)
            .or_else(|| self.type_info.fn_param_types.get(method_name))
            .cloned();
        if let Some(params) = param_list {
            for (pname, ptype) in &params {
                self.write(", ");
                self.write(&format!("{}: {}", pname, ptype.to_zig_type()));
            }
        } else {
            // Fallback: generate from AST
            for param in &func.params.items {
                if let Some(pname) = crate::infer::binding_name(&param.pattern) {
                    self.write(&format!(", {}: anytype", pname));
                }
            }
        }

        // Return type
        self.writeln(&format!(") {} {{", ret_ty));

        // Emit body with this→self rewriting
        self.indent += 1;

        if let Some(body) = &func.body {
            for stmt in &body.statements {
                self.emit_fn_stmt(stmt);
            }
        }

        self.indent -= 1;
        self.writeln("}");

        self.current_fn = saved_fn;
    }

    /// Emit a statement with `this.x` → `self.x` rewriting.
    /// This is used in constructor bodies where JS code assigns `this.field = value`.
    fn emit_stmt_with_this_rewrite(&mut self, stmt: &Statement, field_names: &[String]) {
        match stmt {
            Statement::ExpressionStatement(es) => {
                // Check if this is `this.field = value`
                if let Expression::AssignmentExpression(ae) = &es.expression
                    && let AssignmentTarget::StaticMemberExpression(sme) = &ae.left
                    && matches!(&sme.object, Expression::ThisExpression(_))
                {
                    // this.field = value → const field = value;
                    let fname = sme.property.name.to_string();
                    if field_names.contains(&fname) {
                        self.write_indent();
                        self.write("const ");
                        self.write(&fname);
                        self.write(" = ");
                        self.in_return_expr = true;
                        self.emit_expr(&ae.right);
                        self.in_return_expr = false;
                        self.writeln(";");
                        return;
                    }
                }
                // Fallback: emit as normal
                self.write_indent();
                self.in_expr_stmt = true;
                self.emit_expr(&es.expression);
                self.in_expr_stmt = false;
                self.writeln(";");
            }
            Statement::VariableDeclaration(vd) => {
                self.emit_var_decl(vd);
            }
            Statement::IfStatement(is) => {
                self.write_indent();
                self.write("if (");
                self.emit_condition(&is.test);
                self.writeln(") {");
                self.indent += 1;
                self.emit_stmt_with_this_rewrite(&is.consequent, field_names);
                self.indent -= 1;
                if let Some(alt) = &is.alternate {
                    self.writeln("} else {");
                    self.indent += 1;
                    self.emit_stmt_with_this_rewrite(alt, field_names);
                    self.indent -= 1;
                }
                self.writeln("}");
            }
            Statement::ReturnStatement(rs) => {
                self.write_indent();
                self.write("return ");
                if let Some(arg) = &rs.argument {
                    self.emit_expr(arg);
                }
                self.writeln(";");
            }
            Statement::BlockStatement(bs) => {
                self.writeln("{");
                self.indent += 1;
                for s in &bs.body {
                    self.emit_stmt_with_this_rewrite(s, field_names);
                }
                self.indent -= 1;
                self.writeln("}");
            }
            _ => {
                // For other statements, emit normally
                self.emit_fn_stmt(stmt);
            }
        }
    }

    /// Emit a static field initializer.
    fn emit_static_field_init(&mut self, pd: &PropertyDefinition) {
        if let Some(name) = property_key_name(&pd.key)
            && let Some(value) = &pd.value
        {
            self.writeln(&format!("pub const {} = ", name));
            self.indent += 1;
            self.emit_expr(value);
            self.writeln(";");
            self.indent -= 1;
        }
    }
}

/// Quick expression type scanner for return statements in class methods.
/// Returns a Zig type string ("i64", "f64", "[]const u8", "bool") or None.
fn scan_ret_expr_type(expr: &Expression) -> Option<String> {
    match expr {
        Expression::NumericLiteral(n) => {
            if n.value.fract() == 0.0 {
                Some("i64".to_string())
            } else {
                Some("f64".to_string())
            }
        }
        Expression::StringLiteral(_) => Some("[]const u8".to_string()),
        Expression::BooleanLiteral(_) => Some("bool".to_string()),
        Expression::BinaryExpression(be) => {
            let left = scan_ret_expr_type(&be.left);
            let right = scan_ret_expr_type(&be.right);
            match (left, right) {
                (Some(ref l), Some(ref r)) if l == "[]const u8" || r == "[]const u8" => {
                    Some("[]const u8".to_string())
                }
                (Some(ref l), _) => Some(l.clone()),
                (_, Some(ref r)) => Some(r.clone()),
                _ => Some("i64".to_string()),
            }
        }
        Expression::CallExpression(_) => None,
        Expression::StaticMemberExpression(sme) => {
            scan_ret_expr_type(&sme.object).or(Some("i64".to_string()))
        }
        Expression::Identifier(_) => None,
        Expression::ParenthesizedExpression(pe) => scan_ret_expr_type(&pe.expression),
        _ => Some("i64".to_string()),
    }
}
