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
}

// ── emit_toplevel helpers ──────────────────────────
