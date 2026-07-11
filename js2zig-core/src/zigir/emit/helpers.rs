// zigir/emit/helpers.rs
// Emitter output helpers: write, writeln, write_indent, etc.
// Also trait definition for shared helper methods.

use crate::types::ZigType;
use crate::zigir::ident::IrIdent;
use crate::zigir::ops::AssignOp;
use crate::zigir::ops::BinOp;
use crate::zigir::ops::LogicalOp;
use crate::zigir::ops::UnaOp;
use crate::zigir::ops::UpdateOp;

// ═══════════════════════════════════════════════════════
//  Emitter output helpers
// ═══════════════════════════════════════════════════════

pub trait EmitterHelpers {
    fn output(&self) -> &str;
    fn output_mut(&mut self) -> &mut String;
    fn indent(&self) -> usize;
    fn indent_mut(&mut self) -> &mut usize;

    /// Append raw string to output buffer.
    fn write(&mut self, s: &str) {
        self.output_mut().push_str(s);
    }

    /// Append single char to output buffer.
    fn push(&mut self, ch: char) {
        self.output_mut().push(ch);
    }

    /// Write an indented line: indent + content + newline.
    fn writeln(&mut self, s: &str) {
        self.write_indent();
        self.output_mut().push_str(s);
        self.output_mut().push('\n');
    }

    /// Emit current indent level (4 spaces per level).
    fn write_indent(&mut self) {
        for _ in 0..self.indent() {
            self.output_mut().push_str("    ");
        }
    }

    /// Increase indent level.
    fn indent_push(&mut self) {
        *self.indent_mut() += 1;
    }

    /// Decrease indent level.
    fn indent_pop(&mut self) {
        assert!(self.indent() > 0, "indent underflow");
        *self.indent_mut() -= 1;
    }
}

// ═══════════════════════════════════════════════════════
//  Operator → Zig string conversion
// ═══════════════════════════════════════════════════════

/// Binary operator → Zig operator string.
pub fn bin_op_to_zig(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%",
        BinOp::Pow => "@pow", // Zig uses std.math.pow / @pow
        BinOp::Lt => "<",
        BinOp::Gt => ">",
        BinOp::Le => "<=",
        BinOp::Ge => ">=",
        BinOp::Eq => "==",
        BinOp::Ne => "!=",
        BinOp::StrictEq => "==",
        BinOp::StrictNe => "!=",
        BinOp::BitAnd => "&",
        BinOp::BitOr => "|",
        BinOp::BitXor => "^",
        BinOp::Shl => "<<",
        BinOp::Shr => ">>",
        BinOp::UrShr => ">>", // Zig logical right shift needs @as + @truncate
        BinOp::In => "==",    // JS `in` → Zig field existence check (handled at call site)
        BinOp::InstanceOf => "==", // Simplified
    }
}

/// Unary operator → Zig operator string.
pub fn una_op_to_zig(op: UnaOp) -> &'static str {
    match op {
        UnaOp::Neg => "-",
        UnaOp::Not => "!",
        UnaOp::BitNot => "~",
        UnaOp::TypeOf => "typeof", // handled specially at emit time
        UnaOp::Void => "void",     // handled specially at emit time
        UnaOp::Delete => "delete", // handled specially at emit time
    }
}

/// Logical operator → Zig operator string.
pub fn logical_op_to_zig(op: LogicalOp) -> &'static str {
    match op {
        LogicalOp::And => "and",
        LogicalOp::Or => "or",
        LogicalOp::Nullish => "orelse",
    }
}

/// Update operator → Zig code fragment.
pub fn update_op_to_zig(op: UpdateOp) -> &'static str {
    match op {
        UpdateOp::Increment => "+= 1",
        UpdateOp::Decrement => "-= 1",
    }
}

// AssignOp already has to_zig_str() in ops.rs — re-export for convenience.
pub fn assign_op_to_zig(op: AssignOp) -> &'static str {
    op.to_zig_str()
}

// ═══════════════════════════════════════════════════════
//  ZigType → Zig type string helpers
// ═══════════════════════════════════════════════════════

/// Convert a ZigType to a Zig type annotation string for declarations.
/// This wraps ZigType::to_zig_type() but handles Option<ZigType> (None = inferred).
pub fn zig_type_annotation(ty: Option<&ZigType>) -> Option<String> {
    ty.map(|t| t.to_zig_type())
}

/// Format a function return type, considering async/throw modifiers.
///
/// For async functions returning a `NamedStruct`, the `host.` prefix is added
/// because the struct type is defined in the host module (e.g., `host.FetchUserResult`).
pub fn format_return_type(ret_type: &ZigType, is_async: bool, can_throw: bool) -> String {
    let base = match ret_type {
        ZigType::NamedStruct(name) if is_async => format!("host.{}", name),
        _ => ret_type.to_zig_type(),
    };
    if (is_async || can_throw) && base != "void" {
        format!("!{}", base)
    } else if can_throw && base == "void" {
        "!void".to_string()
    } else {
        base
    }
}

/// Format a parameter: `name: Type`.
pub fn format_param(name: &IrIdent, ty: &ZigType) -> String {
    format!("{}: {}", name.zig_name, ty.to_zig_type())
}

/// Format a parameter with rest awareness.
/// If `is_rest` is true, the type is rendered as `[]const JsAny` regardless of `ty`.
pub fn format_param_with_rest(name: &IrIdent, ty: &ZigType, is_rest: bool) -> String {
    let type_str = if is_rest {
        "[]const JsAny".to_string()
    } else {
        ty.to_zig_type()
    };
    format!("{}: {}", name.zig_name, type_str)
}

/// Escape a string for use in a Zig string literal.
pub(crate) fn escape_zig_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 16);
    for byte in s.bytes() {
        match byte {
            b'\\' => result.push_str("\\\\"),
            b'"' => result.push_str("\\\""),
            b'\n' => result.push_str("\\n"),
            b'\r' => result.push_str("\\r"),
            b'\t' => result.push_str("\\t"),
            c @ 0x00..=0x1F | c @ 0x7F..=0xFF => {
                result.push_str(&format!("\\x{:02X}", c));
            }
            _ => result.push(byte as char),
        }
    }
    result
}

/// Escape a string for use in a Zig **format** string literal.
/// Same as `escape_zig_string` but also doubles `{` and `}` for Zig's
/// `std.fmt` format-string escaping (`{` → `{{`, `}` → `}}`).
pub(crate) fn escape_zig_format_string(s: &str) -> String {
    let escaped = escape_zig_string(s);
    let mut result = String::with_capacity(escaped.len() + 16);
    for ch in escaped.chars() {
        match ch {
            '{' => result.push_str("{{"),
            '}' => result.push_str("}}"),
            _ => result.push(ch),
        }
    }
    result
}

/// Format a `@compileError("...")` with proper string escaping.
pub(crate) fn compile_error(msg: &str) -> String {
    format!("@compileError(\"{}\")", escape_zig_string(msg))
}

// ═══════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_return_type() {
        assert_eq!(format_return_type(&ZigType::I64, false, false), "i64");
        assert_eq!(format_return_type(&ZigType::I64, true, false), "!i64");
        assert_eq!(format_return_type(&ZigType::I64, false, true), "!i64");
        assert_eq!(format_return_type(&ZigType::Void, false, true), "!void");
        assert_eq!(format_return_type(&ZigType::Void, false, false), "void");
        assert_eq!(format_return_type(&ZigType::F64, true, true), "!f64");
        // Async NamedStruct should get host. prefix
        assert_eq!(
            format_return_type(
                &ZigType::NamedStruct("FetchUserResult".to_string()),
                true,
                false
            ),
            "!host.FetchUserResult"
        );
        // Non-async NamedStruct should NOT get host. prefix
        assert_eq!(
            format_return_type(&ZigType::NamedStruct("MyStruct".to_string()), false, false),
            "MyStruct"
        );
    }

    #[test]
    fn test_escape_zig_string() {
        assert_eq!(escape_zig_string("hello"), "hello");
        assert_eq!(escape_zig_string("say \"hi\""), "say \\\"hi\\\"");
        assert_eq!(escape_zig_string("line1\nline2"), "line1\\nline2");
        assert_eq!(escape_zig_string("tab\there"), "tab\\there");
        assert_eq!(escape_zig_string("back\\slash"), "back\\\\slash");
    }

    #[test]
    fn test_escape_zig_format_string() {
        // Basic: no braces
        assert_eq!(escape_zig_format_string("hello"), "hello");
        // Braces are doubled
        assert_eq!(escape_zig_format_string("{s}"), "{{s}}");
        // Control chars + braces
        assert_eq!(escape_zig_format_string("a\n{b}"), "a\\n{{b}}");
        // Quote + brace
        assert_eq!(escape_zig_format_string("\"{x}\""), "\\\"{{x}}\\\"");
    }

    #[test]
    fn test_bin_op_to_zig() {
        assert_eq!(bin_op_to_zig(BinOp::Add), "+");
        assert_eq!(bin_op_to_zig(BinOp::Le), "<=");
        assert_eq!(bin_op_to_zig(BinOp::StrictEq), "==");
    }

    #[test]
    fn test_logical_op_to_zig() {
        assert_eq!(logical_op_to_zig(LogicalOp::And), "and");
        assert_eq!(logical_op_to_zig(LogicalOp::Or), "or");
        assert_eq!(logical_op_to_zig(LogicalOp::Nullish), "orelse");
    }

    #[test]
    fn test_update_op_to_zig() {
        assert_eq!(update_op_to_zig(UpdateOp::Increment), "+= 1");
        assert_eq!(update_op_to_zig(UpdateOp::Decrement), "-= 1");
    }

    #[test]
    fn test_assign_op_to_zig() {
        assert_eq!(assign_op_to_zig(AssignOp::Assign), "=");
        assert_eq!(assign_op_to_zig(AssignOp::Add), "+=");
        assert_eq!(assign_op_to_zig(AssignOp::LogicOr), "or=");
    }

    #[test]
    fn test_format_param() {
        let ident = IrIdent::new("foo");
        let param = format_param(&ident, &ZigType::I64);
        assert_eq!(param, "foo: i64");
    }
}
