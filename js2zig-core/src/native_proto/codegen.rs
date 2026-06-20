// native_proto/codegen.rs
// All Codegen impl methods in one file.
// This avoids Rust visibility issues across multiple impl blocks in different files.

use oxc_ast::ast::*;
use crate::native_proto::Codegen;

// ── Constructor ─────────────────────────────────────

impl Codegen {
    pub fn new() -> Self {
        Self { output: String::new(), indent: 0, used_names: std::collections::HashSet::new() }
    }
}

// ── Entry point ─────────────────────────────────────

impl Codegen {
    pub fn generate(&mut self, program: &Program) {
        // Pass 1: collect identifiers referenced in function bodies.
        self.used_names.clear();
        for stmt in &program.body {
            if let Statement::FunctionDeclaration(fd) = stmt {
                Self::collect_idents_from_function(fd, &mut self.used_names);
            }
        }

        // Pass 2: emit code, skipping unused toplevel constants.
        self.writeln("const std = @import(\"std\");");
        self.writeln("");
        for stmt in &program.body {
            self.emit_toplevel(stmt);
        }
    }

    fn emit_toplevel(&mut self, stmt: &Statement) {
        match stmt {
            Statement::VariableDeclaration(vd) => self.emit_var_decl(vd),
            Statement::FunctionDeclaration(fd) => self.emit_fn(fd),
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

                // Skip unused toplevel constants to avoid Zig unused warnings.
                if self.indent == 0 && is_const && !self.used_names.contains(name) {
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
                        let ty = self.infer_type(init);
                        self.write_indent();
                        let kw = if is_const { "const" } else { "var" };
                        if ty.is_empty() {
                            // Unknown type: let Zig infer.
                            self.write(&format!("{} {} = ", kw, name));
                        } else {
                            self.write(&format!("{} {}: {} = ", kw, name, ty));
                        }
                        self.emit_expr(init);
                        self.write(";\n");
                    }
                    None => {
                        // No initializer → undefined (error in new type system).
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

        let return_exprs = Self::collect_return_exprs(fd);
        let ret_ty = if return_exprs.is_empty() {
            "void".to_string()
        } else {
            // Emit each return expression to get its Zig representation,
            // then use @TypeOf(expr1, expr2, ...) for the return type.
            let mut expr_strs = Vec::new();
            for expr in &return_exprs {
                let mut tmp = Codegen::new();
                tmp.emit_expr(expr);
                let s = tmp.output.trim().to_string();
                if !s.is_empty() {
                    expr_strs.push(s);
                }
            }
            if expr_strs.is_empty() {
                // Cannot infer return type from expressions; fall back to JsAny.
                "JsAny".to_string()
            } else {
                format!("@TypeOf({})", expr_strs.join(", "))
            }
        };

        self.write(&format!("fn {}(", name));
        for (i, param) in fd.params.items.iter().enumerate() {
            if i > 0 { self.write(", "); }
            if let Some(pname) = self.binding_name(&param.pattern) {
                self.write(&format!("{}: anytype", pname));
            }
        }
        self.writeln(&format!(") !{} {{", ret_ty));

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
                self.write("return ");
                if let Some(arg) = &rs.argument {
                    self.emit_expr(arg);
                }
                self.write(";\n");
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
                self.write(&format!("\"{}\"", s.value));
            }
            Expression::BooleanLiteral(b) => {
                self.write(if b.value { "true" } else { "false" });
            }
            Expression::Identifier(id) => {
                self.write(id.name.as_str());
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
        let left_is_string = matches!(be.left, Expression::StringLiteral(_));
        let right_is_string = matches!(be.right, Expression::StringLiteral(_));

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

    // Call expression (all calls get `try`)
    fn emit_call(&mut self, ce: &CallExpression) {
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
}

// ── Type inference (simplified) ───────────────────────

impl Codegen {
    fn infer_type(&self, expr: &Expression) -> String {
        match expr {
            Expression::NumericLiteral(n) => {
                let s = n.value.to_string();
                if s.contains('.') || s.contains('e') || s.contains('E') {
                    "f64".to_string()
                } else {
                    "i64".to_string()
                }
            }
            Expression::StringLiteral(_) => "[]const u8".to_string(),
            Expression::BooleanLiteral(_) => "bool".to_string(),
            Expression::Identifier(_) => "i64".to_string(), // placeholder
            Expression::BinaryExpression(be) => {
                // For string concat (+), result is []const u8.
                if be.operator == BinaryOperator::Addition {
                    let left_str = matches!(be.left, Expression::StringLiteral(_));
                    let right_str = matches!(be.right, Expression::StringLiteral(_));
                    if left_str || right_str {
                        return "[]const u8".to_string();
                    }
                }
                self.infer_type(&be.left)
            }
            Expression::CallExpression(_) => "".to_string(), // let Zig infer from context
            Expression::ArrayExpression(ae) => {
                if ae.elements.is_empty() {
                    "std.ArrayList(u8)".to_string() // default element type
                } else {
                    // Infer from first element.
                    if let Some(first_elem) = ae.elements.first()
                        && let Some(first) = first_elem.as_expression() {
                        format!("[{}]const {}", ae.elements.len(), self.infer_type(first))
                    } else {
                        "[0]u8".to_string()
                    }
                }
            }
            _ => "".to_string(), // unknown: let Zig infer
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
                match &ae.left {
                    AssignmentTarget::AssignmentTargetIdentifier(id) => {
                        names.insert(id.name.to_string());
                    }
                    _ => {}
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
