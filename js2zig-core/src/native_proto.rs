// js2zig-core/src/native_proto.rs
//
// Prototype: new native-type system codegen.
// Only handles minimum viable target.
// Usage: cargo test -p js2zig-core -- test_native_proto

use oxc_ast::ast::*;
use oxc_parser::Parser;
use oxc_allocator::Allocator;
use oxc_span::SourceType;

/// Transpile a JS string to Zig source (native type system).
pub fn transpile_js(js_source: &str) -> Result<String, String> {
    let alloc = Allocator::default();
    let source_type = SourceType::default(); // auto-detect module vs script
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
}

impl Codegen {
    fn new() -> Self {
        Self { output: String::new(), indent: 0 }
    }

    fn generate(&mut self, program: &Program) {
        self.writeln("const std = @import(\"std\");");
        self.writeln("");
        for stmt in &program.body {
            self.emit_toplevel(stmt);
        }
    }

    // ── Top-level statements ─────────────────────────────────────

    fn emit_toplevel(&mut self, stmt: &Statement) {
        match stmt {
            Statement::VariableDeclaration(vd) => {
                self.emit_const(vd);
            }
            Statement::FunctionDeclaration(fd) => {
                self.emit_fn(fd);
            }
            _ => { /* skip unsupported */ }
        }
    }

    // ── Const declarations ──────────────────────────────────

    fn emit_const(&mut self, vd: &VariableDeclaration) {
        for decl in &vd.declarations {
            if let Some(name) = self.binding_name(&decl.id) {
                if let Some(init) = &decl.init {
                    let ty = self.infer_type(init);
                    self.write(&format!("const {}: ", name));
                    self.write(&ty);
                    self.write(" = ");
                    self.emit_expr(init);
                    self.writeln(";");
                }
            }
        }
    }

    // ── Function declarations ──────────────────────────────────

    fn emit_fn(&mut self, fd: &Function) {
        let name = fd.id.as_ref()
            .map(|id| id.name.as_str())
            .unwrap_or("anonymous");

        // Collect return expressions for @TypeOf.
        let return_exprs = Self::collect_return_exprs(fd);
        let ret_ty = if return_exprs.is_empty() {
            "void".to_string()
        } else {
            format!("@TypeOf({})", return_exprs.join(", "))
        };

        // Function signature.
        self.write(&format!("fn {}(", name));
        for (i, param) in fd.params.items.iter().enumerate() {
            if i > 0 { self.write(", "); }
            if let Some(pname) = self.binding_name(&param.pattern) {
                self.write(&format!("{}: anytype", pname));
            }
        }
        self.writeln(&format!(") !{} {{", ret_ty));

        // Function body.
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
                for decl in &vd.declarations {
                    if let Some(name) = self.binding_name(&decl.id) {
                        if let Some(init) = &decl.init {
                            let ty = self.infer_type(init);
                            self.write_indent();
                            self.write(&format!("var {}: ", name));
                            self.write(&ty);
                            self.write(" = ");
                            self.emit_expr(init);
                            self.writeln(";");
                        }
                    }
                }
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
                self.writeln(";");
            }
            _ => { /* skip unsupported */ }
        }
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
                self.emit_expr(&be.left);
                self.write(&format!(" {} ", Self::binary_op(be.operator)));
                self.emit_expr(&be.right);
            }
            _ => {
                self.write("/* TODO */");
            }
        }
    }

    // ── Type inference (simplified) ───────────────────────────

    fn infer_type(&self, expr: &Expression) -> String {
        match expr {
            Expression::NumericLiteral(n) => {
                let s = n.value.to_string();
                if s.contains('.') { "f64".to_string() } else { "i64".to_string() }
            }
            Expression::StringLiteral(_) => "[]const u8".to_string(),
            Expression::BooleanLiteral(_) => "bool".to_string(),
            Expression::Identifier(_) => "i64".to_string(), // placeholder
            Expression::BinaryExpression(be) => {
                self.infer_type(&be.left)
            }
            _ => "JsAny".to_string(),
        }
    }

    // ── Return expression collection ───────────────────────────

    fn collect_return_exprs(fd: &Function) -> Vec<String> {
        let mut exprs = Vec::new();
        if let Some(body) = &fd.body {
            for stmt in &body.statements {
                Self::collect_returns(stmt, &mut exprs);
            }
        }
        exprs
    }

    fn collect_returns(stmt: &Statement, exprs: &mut Vec<String>) {
        match stmt {
            Statement::ReturnStatement(rs) => {
                if let Some(arg) = &rs.argument {
                    exprs.push(Self::expr_to_string(arg));
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

    fn expr_to_string(expr: &Expression) -> String {
        match expr {
            Expression::NumericLiteral(n) => n.value.to_string(),
            Expression::StringLiteral(s) => format!("\"{}\"", s.value),
            Expression::BooleanLiteral(b) => b.value.to_string(),
            Expression::Identifier(id) => id.name.to_string(),
            Expression::BinaryExpression(be) => {
                let left = Self::expr_to_string(&be.left);
                let right = Self::expr_to_string(&be.right);
                let op = Self::binary_op(be.operator);
                format!("{} {} {}", left, op, right)
            }
            _ => "undefined".to_string(),
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
            _ => "+", // placeholder
        }
    }

    // ── Output helpers ────────────────────────────────────────

    fn write(&mut self, s: &str) {
        self.output.push_str(s);
    }

    fn writeln(&mut self, s: &str) {
        self.write_indent();
        self.output.push_str(s);
        self.output.push('\n');
    }

    fn indent_str(&self) -> String {
        "    ".repeat(self.indent)
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
    fn test_native_proto() {
        let js = r#"
const x = 42;

function add(a, b) {
    return a + b;
}
"#;
        let zig = transpile_js(js).unwrap();
        println!("=== Generated Zig ===\n{}", zig);
        // Basic assertions.
        assert!(zig.contains("const x: i64 = 42;"), "const x not found");
        assert!(zig.contains("fn add(a: anytype, b: anytype) !@TypeOf(a + b) {"), "fn add not found");
        assert!(zig.contains("return a + b;"), "return not found");
    }
}
