// native_proto/codegen/helpers.rs
// Type inference helpers, operator mappings, and output helpers.

use super::Codegen;
use oxc_ast::ast::*;

// ── Helpers (methods) ──────────────────────────────

impl Codegen {
    pub(crate) fn binding_name<'a>(&self, pattern: &BindingPattern<'a>) -> Option<&'a str> {
        match pattern {
            BindingPattern::BindingIdentifier(id) => Some(id.name.as_str()),
            _ => None,
        }
    }

    pub(crate) fn binary_op(op: BinaryOperator) -> &'static str {
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

    pub(crate) fn assignment_op(op: AssignmentOperator) -> &'static str {
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

    pub(crate) fn logical_op(op: LogicalOperator) -> &'static str {
        match op {
            LogicalOperator::And => "and",
            LogicalOperator::Or => "or",
            LogicalOperator::Coalesce => "??",
        }
    }

    pub(crate) fn unary_prefix(op: UnaryOperator) -> &'static str {
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

// ── Argument extraction helpers ───────────────────

impl Codegen {
    /// Extract the first call argument as a string (via emit_expr_to_string).
    /// Returns empty string if no argument or argument is not an expression.
    pub(crate) fn first_arg_string(&mut self, args: &[Argument]) -> String {
        if let Some(arg) = args.first()
            && let Some(expr) = arg.as_expression()
        {
            self.emit_expr_to_string(expr)
        } else {
            String::new()
        }
    }

    /// Extract the first call argument as a string, with a custom fallback.
    pub(crate) fn first_arg_string_or(&mut self, args: &[Argument], default: &str) -> String {
        if let Some(arg) = args.first()
            && let Some(expr) = arg.as_expression()
        {
            self.emit_expr_to_string(expr)
        } else {
            default.to_string()
        }
    }

    /// Emit the first call argument directly. Silently does nothing if none.
    pub(crate) fn emit_first_arg(&mut self, args: &[Argument]) {
        if let Some(arg) = args.first()
            && let Some(expr) = arg.as_expression()
        {
            self.emit_expr(expr);
        }
    }

    /// Emit all call arguments separated by ", ".
    pub(crate) fn emit_comma_separated_args(&mut self, args: &[Argument]) {
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            self.emit_expr_arg(arg);
        }
    }

    /// Emit array expression elements separated by ", ".
    /// Skips elements that are not expressions (spread, elision).
    pub(crate) fn emit_comma_separated_array_elements(
        &mut self,
        elements: &[ArrayExpressionElement],
    ) {
        for (i, elem) in elements.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            if let Some(e) = elem.as_expression() {
                self.emit_expr(e);
            }
        }
    }
}

// ── Template / format string helpers ───────────────

impl Codegen {
    /// If the callee is `obj.method()`, return `Some("obj")`.
    /// Returns `None` if it's not a static member expression on an identifier.
    pub(crate) fn callee_object_name<'a>(&self, callee: &'a Expression) -> Option<&'a str> {
        if let Expression::StaticMemberExpression(mem) = callee
            && let Expression::Identifier(obj) = &mem.object
        {
            Some(obj.name.as_str())
        } else {
            None
        }
    }

    /// Emit a format string expression: either a plain string literal (no args)
    /// or an allocPrint call (when interpolating args).
    ///
    /// `fmt` is a Zig format string (with `{{` / `}}` escaped).
    /// `args` are the already-emitted argument strings.
    pub(crate) fn emit_format_string(&mut self, fmt: &str, args: &[String]) {
        if args.is_empty() {
            // Pure-text → plain string literal (no allocation).
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
}

// ── emit_toplevel helpers ──────────────────────────
