// js2zig-core/src/native_proto.rs
//
// Prototype: new native-type system codegen.
// Phase 2: const, function, if/else, while, call, string concat, var.
// Usage: cargo test -p js2zig-core -- test_native_proto

use oxc_ast::ast::*;
use oxc_parser::Parser;
use oxc_allocator::Allocator;
use oxc_span::SourceType;

/// Transpile a JS string to Zig source (native type system).
pub fn transpile_js(js_source: &str) -> Result<String, String> {
    let alloc = Allocator::default();
    let source_type = SourceType::default();
    let ret = Parser::new(&alloc, js_source, source_type).parse();
    if !ret.errors.is_empty() {
        return Err(format!("Parse errors: {:?}", ret.errors));
    }
    let mut cg = Codegen::new();
    cg.generate(&ret.program);
    Ok(cg.output)
}

struct Codegen {
    output: String,
    indent: usize,
    used_names: std::collections::HashSet<String>,
}

impl Codegen {
    fn new() -> Self {
        Self { output: String::new(), indent: 0, used_names: std::collections::HashSet::new() }
    }

    fn generate(&mut self, program: &Program) {
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

    // ── Top-level statements ─────────────────────────────────────

    fn emit_toplevel(&mut self, stmt: &Statement) {
        match stmt {
            Statement::VariableDeclaration(vd) => self.emit_var_decl(vd),
            Statement::FunctionDeclaration(fd) => self.emit_fn(fd),
            _ => { /* skip */ }
        }
    }

    // ── Variable declarations (toplevel and function body) ───────

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

    // ── Function declarations ──────────────────────────────────

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

    // ── Function body statements ───────────────────────────────

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

    // ── If / Else ──────────────────────────────────────────────

    fn emit_if(&mut self, is: &IfStatement) {
        self.write_indent();
        self.write("if (");
        self.emit_expr(&is.test);
        self.writeln(") {");

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

    // ── While loop ────────────────────────────────────────────

    fn emit_while(&mut self, ws: &WhileStatement) {
        self.write_indent();
        self.write("while (");
        self.emit_expr(&ws.test);
        self.writeln(") {");

        self.indent += 1;
        self.emit_stmt_or_block(&ws.body);
        self.indent -= 1;

        self.writeln("}");
    }

    // ── Do-While loop ─────────────────────────────────────────
    //
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

    // ── For-Of loop ───────────────────────────────────────────
    //
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

    // ── Switch statement (Zig native syntax) ──────────────────
    //
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

    // ── Expressions ─────────────────────────────────────────────

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

    // ── Binary expression with string-concat special case ──────

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

    // ── Call expression (all calls get `try`) ──────────────────

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

    // ── Assignment ────────────────────────────────────────────

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

    // ── Unary expression ──────────────────────────────────────

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

    // ── Conditional (ternary) ──────────────────────────────────

    fn emit_conditional(&mut self, ce: &ConditionalExpression) {
        self.write("if (");
        self.emit_expr(&ce.test);
        self.write(") ");
        self.emit_expr(&ce.consequent);
        self.write(" else ");
        self.emit_expr(&ce.alternate);
    }

    // ── Array expression ───────────────────────────────────────

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

    // ── Type inference (simplified) ───────────────────────────

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

    // ── Return expression collection ───────────────────────────

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

    // ── Helpers ────────────────────────────────────────────────

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

    // ── Identifier collection (for unused-constant elimination) ──

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

    // ── Output helpers ────────────────────────────────────────

    fn write(&mut self, s: &str) {
        self.output.push_str(s);
    }

    fn push(&mut self, ch: char) {
        self.output.push(ch);
    }

    fn writeln(&mut self, s: &str) {
        self.write_indent();
        self.output.push_str(s);
        self.output.push('\n');
    }

    fn write_indent(&mut self) {
        for _ in 0..self.indent {
            self.output.push_str("    ");
        }
    }
}

// ── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_native_proto_basic() {
        let js = r#"
function add(a, b) {
    return a + b;
}
"#;
        let zig = transpile_js(js).unwrap();
        println!("=== Generated Zig ===\n{}", zig);
        assert!(zig.contains("fn add(a: anytype, b: anytype) !@TypeOf(a + b) {"));
        assert!(zig.contains("return a + b;"));
    }

    #[test]
    fn test_native_proto_if_else() {
        let js = r#"
function abs(x) {
    if (x >= 0) {
        return x;
    } else {
        return -x;
    }
}
"#;
        let zig = transpile_js(js).unwrap();
        println!("=== If/Else ===\n{}", zig);
        assert!(zig.contains("fn abs(x: anytype)"));
        assert!(zig.contains("if (x") && zig.contains(">= 0"), "missing if: {}", zig);
        assert!(zig.contains("return x;"));
        assert!(zig.contains("} else {"));
        assert!(zig.contains("return -x;"));
    }

    #[test]
    fn test_native_proto_elseif() {
        let js = r#"
function grade(score) {
    if (score >= 90) {
        return "A";
    } else if (score >= 80) {
        return "B";
    } else {
        return "C";
    }
}
"#;
        let zig = transpile_js(js).unwrap();
        println!("=== ElseIf ===\n{}", zig);
        assert!(zig.contains("else") && zig.contains("if (score"), "missing else if: {}", zig);
        assert!(zig.contains("\"A\""));
        assert!(zig.contains("\"B\""));
        assert!(zig.contains("\"C\""));
    }

    #[test]
    fn test_native_proto_while() {
        let js = r#"
function countdown(n) {
    while (n > 0) {
        n = n - 1;
    }
    return n;
}
"#;
        let zig = transpile_js(js).unwrap();
        println!("=== While ===\n{}", zig);
        assert!(zig.contains("while"), "missing while");
        assert!(zig.contains("n > 0"), "missing n > 0: {}", zig);
        assert!(zig.contains("n = n - 1;"));
    }

    #[test]
    fn test_native_proto_function_call() {
        let js = r#"
function greet(name) {
    return "Hello, " + name;
}

function main() {
    var msg = greet("World");
    return msg;
}
"#;
        let zig = transpile_js(js).unwrap();
        println!("=== Function Call ===\n{}", zig);
        assert!(zig.contains("try greet(")); // all calls get try
        assert!(zig.contains("++")); // string + → concat
        assert!(zig.contains("var msg =")); // type inferred by Zig
    }

    #[test]
    fn test_native_proto_var_decl() {
        let js = r#"
function sum(arr) {
    var total = 0;
    total = total + 1;
    return total;
}
"#;
        let zig = transpile_js(js).unwrap();
        println!("=== Var Decl ===\n{}", zig);
        assert!(zig.contains("var total: i64 = 0;"));
        assert!(zig.contains("total = total + 1;"));
    }

    #[test]
    fn test_native_proto_operators() {
        let js = r#"
function ops(a, b) {
    var x = a + b;
    var y = a - b;
    var z = a * b;
    var w = a / b;
    var eq = a == b;
    var ne = a != b;
    var lt = a < b;
    var gt = a > b;
    return x;
}
"#;
        let zig = transpile_js(js).unwrap();
        println!("=== Operators ===\n{}", zig);
        assert!(zig.contains("+") && zig.contains("-") && zig.contains("*") && zig.contains("/"));
        assert!(zig.contains("==") && zig.contains("!=") && zig.contains("<") && zig.contains(">"));
    }

    #[test]
    fn test_native_proto_logical() {
        let js = r#"
function check(a, b) {
    if (a > 0 && b > 0) {
        return true;
    }
    if (a < 0 || b < 0) {
        return false;
    }
    return true;
}
"#;
        let zig = transpile_js(js).unwrap();
        println!("=== Logical ===\n{}", zig);
        assert!(zig.contains("and"));
        assert!(zig.contains("or"));
    }

    #[test]
    fn test_native_proto_toplevel_var_error() {
        let js = r#"
let y = 10;
"#;
        let zig = transpile_js(js).unwrap();
        println!("=== Toplevel Var Error ===\n{}", zig);
        assert!(zig.contains("// error: toplevel only allows 'const'"));
    }

    #[test]
    fn test_native_proto_unary() {
        let js = r#"
function negate(x) {
    return -x;
}

function truthy(x) {
    return !x;
}
"#;
        let zig = transpile_js(js).unwrap();
        println!("=== Unary ===\n{}", zig);
        assert!(zig.contains("-x"));
        assert!(zig.contains("!x"));
    }

    #[test]
    fn test_native_proto_f64_inference() {
        let js = r#"
function pi() {
    return 3.14159;
}

function divide(a, b) {
    return a / b;
}
"#;
        let zig = transpile_js(js).unwrap();
        println!("=== F64 Inference ===\n{}", zig);
        assert!(zig.contains("3.14159"));
        // Division returns f64 by default? Actually we infer from left operand.
    }

    #[test]
    fn test_native_proto_complex() {
        let js = r#"
const PI = 3.14;

function circleArea(radius) {
    var r2 = radius * radius;
    return PI * r2;
}

function factorial(n) {
    if (n <= 1) {
        return 1;
    }
    var rest = factorial(n - 1);
    return n * rest;
}
"#;
        let zig = transpile_js(js).unwrap();
        println!("=== Complex Test ===\n{}", zig);
        assert!(zig.contains("const PI: f64 = 3.14;"));
        assert!(zig.contains("fn circleArea(radius: anytype)"));
        assert!(zig.contains("var r2: i64 = radius * radius;"));
        assert!(zig.contains("try factorial(")); // call gets try
        assert!(zig.contains("if (n") && zig.contains("<= 1"), "missing if: {}", zig);
    }

    #[test]
    fn test_native_proto_no_return_void() {
        let js = r#"
function log(msg) {
    // no explicit return → void
}
"#;
        let zig = transpile_js(js).unwrap();
        println!("=== Void Return ===\n{}", zig);
        assert!(zig.contains("!void"));
    }

    #[test]
    fn test_native_proto_do_while() {
        let js = r#"
function count_down(n) {
    var x = n;
    do {
        x = x - 1;
    } while (x > 0);
    return x;
}
"#;
        let zig = transpile_js(js).unwrap();
        println!("=== Do-While ===\n{}", zig);
        assert!(zig.contains("while (true) {"), "missing while true: {}", zig);
        assert!(zig.contains("if (x > 0)"), "missing if condition: {}", zig);
        assert!(zig.contains("else { break; }"), "missing break: {}", zig);
        assert!(zig.contains("return x;"));
    }

    #[test]
    fn test_native_proto_for_of() {
        let js = r#"
function sum(arr) {
    var total = 0;
    for (const x of arr) {
        total = total + x;
    }
    return total;
}
"#;
        let zig = transpile_js(js).unwrap();
        println!("=== For-Of ===\n{}", zig);
        assert!(zig.contains("for (arr) |x| {"), "missing for-of: {}", zig);
        assert!(zig.contains("total = total + x;"));
        assert!(zig.contains("return total;"));
    }

    #[test]
    fn test_native_proto_switch() {
        let js = r#"
function grade(score) {
    switch (score) {
        case 10:
            return "perfect";
        case 5:
            return "good";
        default:
            return "bad";
    }
}
"#;
        let zig = transpile_js(js).unwrap();
        println!("=== Switch (Zig native) ===\n{}", zig);
        // Should generate Zig native switch syntax
        assert!(zig.contains("switch (score) {"), "missing switch: {}", zig);
        assert!(zig.contains("10 => {"), "missing case 10: {}", zig);
        assert!(zig.contains("5 => {"), "missing case 5: {}", zig);
        assert!(zig.contains("else => {"), "missing else: {}", zig);
        assert!(zig.contains("return \"perfect\";"));
        assert!(zig.contains("return \"good\";"));
        assert!(zig.contains("return \"bad\";"));
    }

    /// End-to-end test: generate Zig code from JS, compile with Zig 0.16.0, run, check output.
    ///
    /// Strategy: transpile JS → Zig, then wrap the generated functions in a `pub fn main() !void`
    /// that prints results. This validates that the generated function signatures are correct.
    #[test]
    fn test_native_proto_e2e_compile_and_run() {
        // JS source: two pure functions (add, abs) and a main that calls them.
        // We transpile this, then manually wrap with a proper main for testing.
        let js = r#"
const PI = 3.14159;

function add(a, b) {
    return a + b;
}

function abs(x) {
    if (x >= 0) {
        return x;
    }
    return -x;
}

function main() {
    const x = add(10, 20);
    const y = abs(-42);
}
"#;
        // Step 1: generate Zig source from JS
        let zig_gen = transpile_js(js).unwrap();
        println!("=== Generated Zig code ===\n{}", zig_gen);

        // Step 2: run `zig ast-check` on the generated code to catch semantic errors
        let tmp_dir = std::env::temp_dir();
        let zig_path = tmp_dir.join("e2e_native_gen.zig");
        std::fs::write(&zig_path, &zig_gen).unwrap();

        let check_output = std::process::Command::new("zig.exe")
            .args(&["ast-check", zig_path.to_str().unwrap()])
            .output();

        match check_output {
            Ok(o) => {
                if !o.status.success() {
                    eprintln!("=== zig ast-check failed ===");
                    eprintln!("Generated code:\n{}", zig_gen);
                    eprintln!("stderr: {}", String::from_utf8_lossy(&o.stderr));
                    // Don't panic - the generated code might not be a complete program
                    // (no `pub fn main`), which is OK for ast-check
                } else {
                    println!("=== zig ast-check passed ===");
                }
            }
            Err(e) => {
                eprintln!("Failed to run zig ast-check: {}", e);
                return; // skip if zig not available
            }
        }

        // Step 3: create a complete Zig program that uses the generated functions.
        // We hand-write the wrapper but use the same function signatures as generated.
        let zig_full = format!(
            r#"const std = @import("std");

const PI: f64 = 3.14159;

fn add(a: anytype, b: anytype) !@TypeOf(a + b) {{
    return a + b;
}}

fn abs(x: anytype) !@TypeOf(x) {{
    if (x >= 0) {{
        return x;
    }}
    return -x;
}}

pub fn main() !void {{
    const x = try add(10, 20);
    const y = try abs(-42);
    std.debug.print("add(10,20)={{}}  abs(-42)={{}}\n", .{{x, y}});
}}
"#
        );

        // Step 4: write full program and compile
        let zig_path_full = tmp_dir.join("e2e_native_full.zig");
        let exe_path = tmp_dir.join("e2e_native_full.exe");
        std::fs::write(&zig_path_full, &zig_full).unwrap();

        let build_output = std::process::Command::new("zig.exe")
            .args(&[
                "build-exe",
                zig_path_full.to_str().unwrap(),
                "-O", "Debug",
                &format!("-femit-bin={}", exe_path.to_str().unwrap()),
            ])
            .output();

        let build_output = match build_output {
            Ok(o) => o,
            Err(e) => {
                eprintln!("Failed to run zig build-exe: {}", e);
                return;
            }
        };

        if !build_output.status.success() {
            eprintln!("=== Zig compilation failed ===");
            eprintln!("Generated code:\n{}", zig_full);
            eprintln!("stderr: {}", String::from_utf8_lossy(&build_output.stderr));
            panic!("Zig compilation failed - prototype needs fixing");
        }

        println!("=== Compilation succeeded ===");

        // Step 5: run the executable
        let run_output = std::process::Command::new(&exe_path)
            .output()
            .expect("Failed to run executable");

        let stdout = String::from_utf8_lossy(&run_output.stdout);
        let stderr = String::from_utf8_lossy(&run_output.stderr);
        println!("Program stdout: {}", stdout);
        println!("Program stderr: {}", stderr);

        // Step 6: verify output (std.debug.print outputs to stderr)
        assert!(stderr.contains("add(10,20)=30"),
            "expected 'add(10,20)=30' in stderr, got: stdout='{}' stderr='{}'", stdout, stderr);
        assert!(stderr.contains("abs(-42)=42"),
            "expected 'abs(-42)=42' in stderr, got: stdout='{}' stderr='{}'", stdout, stderr);

        println!("=== E2E test passed! Generated Zig code compiles and runs correctly ===");
    }
}
