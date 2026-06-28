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

    /// Extract a property key name from a PropertyKey for destructuring.
    /// Returns None for computed keys (not yet supported).
    pub(crate) fn property_key_name(&self, key: &PropertyKey) -> Option<String> {
        match key {
            PropertyKey::StaticIdentifier(id) => Some(id.name.to_string()),
            PropertyKey::PrivateIdentifier(id) => Some(id.name.to_string()),
            _ => None,
        }
    }

    /// Resolve a destructuring binding pattern, returning (name, optional_default_expr).
    /// Handles both plain bindings and `BindingPattern::AssignmentPattern` (with default).
    pub(crate) fn destructure_binding<'a>(
        &mut self,
        pattern: &BindingPattern<'a>,
    ) -> Option<(&'a str, Option<String>)> {
        match pattern {
            BindingPattern::BindingIdentifier(id) => Some((id.name.as_str(), None)),
            BindingPattern::AssignmentPattern(ap) => {
                let name = self.binding_name(&ap.left)?;
                let default_str = self.emit_expr_to_string(&ap.right);
                Some((name, Some(default_str)))
            }
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
            AssignmentOperator::Exponential => "**=",
            AssignmentOperator::ShiftLeft => "<<=",
            AssignmentOperator::ShiftRight => ">>=",
            AssignmentOperator::ShiftRightZeroFill => ">>>=",
            AssignmentOperator::BitwiseAnd => "&=",
            AssignmentOperator::BitwiseOR => "|=",
            AssignmentOperator::BitwiseXOR => "^=",
            AssignmentOperator::LogicalAnd => "&&=",
            AssignmentOperator::LogicalOr => "||=",
            AssignmentOperator::LogicalNullish => "??=",
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

    /// Like `callee_object_name` but also handles string literals:
    /// `"hello".trimStart()` → `Some("\"hello\"")`.
    /// For identifiers, returns the name. For string literals, returns the escaped literal.
    /// Returns `None` for other object types.
    pub(crate) fn callee_object_repr(&self, callee: &Expression) -> Option<String> {
        if let Expression::StaticMemberExpression(mem) = callee {
            if let Expression::Identifier(obj) = &mem.object {
                return Some(obj.name.to_string());
            }
            if let Expression::StringLiteral(lit) = &mem.object {
                let escaped = lit
                    .value
                    .as_str()
                    .replace('\\', "\\\\")
                    .replace('\"', "\\\"")
                    .replace('\n', "\\n")
                    .replace('\r', "\\r")
                    .replace('\t', "\\t");
                return Some(format!("\"{}\"", escaped));
            }
        }
        None
    }

    /// Try to emit a numeric literal receiver for Number instance methods
    /// (toFixed, toExponential, toPrecision).
    /// Handles `(77.1234).method()` where the AST object is
    /// `ParenthesizedExpression(NumericLiteral)`.
    /// `args_required`: if true, the method requires at least 1 argument.
    /// Returns true if successfully emitted.
    pub(crate) fn emit_numeric_receiver(
        &mut self,
        callee: &Expression,
        method_name: &str,
        arguments: &[Argument],
        args_required: bool,
    ) -> bool {
        let num_expr = if let Expression::StaticMemberExpression(mem) = callee {
            match &mem.object {
                Expression::NumericLiteral(_) => Some(&mem.object),
                Expression::ParenthesizedExpression(pe)
                    if matches!(&pe.expression, Expression::NumericLiteral(_)) =>
                {
                    Some(&pe.expression)
                }
                _ => None,
            }
        } else {
            None
        };
        if let Some(expr) = num_expr {
            if args_required && arguments.is_empty() {
                self.errors
                    .push(format!("{method_name}() requires at least 1 argument"));
                return false;
            }
            self.write(&format!(
                "js_number.{method_name}(js_allocator.getAllocator(), ",
            ));
            self.emit_expr(expr);
            if arguments.is_empty() {
                self.write(", null");
            } else {
                self.write(", ");
                self.emit_first_arg(arguments);
            }
            self.write(")");
            return true;
        }
        false
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
                "std.fmt.allocPrint(js_allocator.getAllocator(), \"{}\", {}) catch @panic(\"OOM: template literal allocPrint\")",
                fmt, args_str
            ));
        }
    }
}

// ── emit_toplevel helpers ──────────────────────────
