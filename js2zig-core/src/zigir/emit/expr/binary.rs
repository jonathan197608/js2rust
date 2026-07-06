// zigir/emit/expr/binary.rs
// Binary operation helpers: float conversion, BigInt, String, and JsAny comparisons.

use crate::zigir::emit::Emitter;
use crate::zigir::emit::helpers::EmitterHelpers;

// ═══════════════════════════════════════════════════════
//  Float conversion helpers for PowExpr
// ═══════════════════════════════════════════════════════

impl Emitter {
    /// Emit a float conversion for a `PowExpr` operand.
    /// - F64: emit directly
    /// - I64/BigInt: wrap in `@as(f64, @floatFromInt(...))`
    /// - Other: wrap in `@as(f64, ...)` (comptime_int, bool, etc.)
    pub(super) fn emit_float_conversion(
        &mut self,
        expr: &crate::zigir::types::IrExpr,
        ty: &crate::types::ZigType,
    ) {
        match ty {
            crate::types::ZigType::F64 => {
                self.emit_expr(expr);
            }
            crate::types::ZigType::I64 | crate::types::ZigType::BigInt => {
                self.write("@as(f64, @floatFromInt(");
                self.emit_expr(expr);
                self.write("))");
            }
            _ => {
                // comptime_int, bool, or unknown — @as(f64, expr) works for comptime_int
                self.write("@as(f64, ");
                self.emit_expr(expr);
                self.write(")");
            }
        }
    }

    /// Emit a BigInt binary operation.
    /// BigInt arithmetic uses method calls like `_a.add(&_b, alloc)`.
    pub(super) fn emit_bigint_binary(
        &mut self,
        op: crate::zigir::ops::BinOp,
        left: &crate::zigir::types::IrExpr,
        right: &crate::zigir::types::IrExpr,
    ) {
        use crate::zigir::ops::BinOp;

        match op {
            // Arithmetic: _a.op(&_b, alloc) catch @panic("BigInt op OOM")
            BinOp::Add => {
                self.write("(");
                self.emit_expr(left);
                self.write(".add(&");
                self.emit_expr(right);
                self.write(", js_allocator.allocator()) catch @panic(\"BigInt add OOM\"))");
            }
            BinOp::Sub => {
                self.write("(");
                self.emit_expr(left);
                self.write(".sub(&");
                self.emit_expr(right);
                self.write(", js_allocator.allocator()) catch @panic(\"BigInt sub OOM\"))");
            }
            BinOp::Mul => {
                self.write("(");
                self.emit_expr(left);
                self.write(".mul(&");
                self.emit_expr(right);
                self.write(", js_allocator.allocator()) catch @panic(\"BigInt mul OOM\"))");
            }
            BinOp::Div => {
                self.write("(");
                self.emit_expr(left);
                self.write(".div(&");
                self.emit_expr(right);
                self.write(", js_allocator.allocator()) catch @panic(\"BigInt div OOM\"))");
            }
            BinOp::Mod => {
                self.write("(");
                self.emit_expr(left);
                self.write(".rem(&");
                self.emit_expr(right);
                self.write(", js_allocator.allocator()) catch @panic(\"BigInt rem OOM\"))");
            }
            BinOp::Pow => {
                self.write("(");
                self.emit_expr(left);
                self.write(".pow(");
                self.emit_expr(right);
                self.write(".toU64() catch @panic(\"BigInt toU64 failed\"), js_allocator.allocator()) catch @panic(\"BigInt pow OOM\"))");
            }
            BinOp::BitAnd => {
                self.write("(");
                self.emit_expr(left);
                self.write(".bitwiseAnd(&");
                self.emit_expr(right);
                self.write(", js_allocator.allocator()) catch @panic(\"BigInt and OOM\"))");
            }
            BinOp::BitOr => {
                self.write("(");
                self.emit_expr(left);
                self.write(".bitwiseOr(&");
                self.emit_expr(right);
                self.write(", js_allocator.allocator()) catch @panic(\"BigInt or OOM\"))");
            }
            BinOp::BitXor => {
                self.write("(");
                self.emit_expr(left);
                self.write(".bitwiseXor(&");
                self.emit_expr(right);
                self.write(", js_allocator.allocator()) catch @panic(\"BigInt xor OOM\"))");
            }
            BinOp::Shl => {
                self.write("(");
                self.emit_expr(left);
                self.write(".shiftLeft(@as(usize, @intCast(");
                self.emit_expr(right);
                self.write(".toU64() catch @panic(\"BigInt toU64 failed\"))), js_allocator.allocator()) catch @panic(\"BigInt shl OOM\"))");
            }
            BinOp::Shr => {
                self.write("(");
                self.emit_expr(left);
                self.write(".shiftRight(@as(usize, @intCast(");
                self.emit_expr(right);
                self.write(".toU64() catch @panic(\"BigInt toU64 failed\"))), js_allocator.allocator()) catch @panic(\"BigInt shr OOM\"))");
            }
            // Equality
            BinOp::Eq | BinOp::StrictEq => {
                self.emit_expr(left);
                self.write(".eq(&");
                self.emit_expr(right);
                self.write(")");
            }
            BinOp::Ne | BinOp::StrictNe => {
                self.write("!");
                self.emit_expr(left);
                self.write(".eq(&");
                self.emit_expr(right);
                self.write(")");
            }
            // Ordering
            BinOp::Lt => {
                self.write("(");
                self.emit_expr(left);
                self.write(".order(&");
                self.emit_expr(right);
                self.write(") == .lt)");
            }
            BinOp::Le => {
                self.write("(");
                self.emit_expr(left);
                self.write(".order(&");
                self.emit_expr(right);
                self.write(") != .gt)");
            }
            BinOp::Gt => {
                self.write("(");
                self.emit_expr(left);
                self.write(".order(&");
                self.emit_expr(right);
                self.write(") == .gt)");
            }
            BinOp::Ge => {
                self.write("(");
                self.emit_expr(left);
                self.write(".order(&");
                self.emit_expr(right);
                self.write(") != .lt)");
            }
            // >>> is not supported for BigInt (JS throws TypeError)
            BinOp::UrShr => {
                self.write("@compileError(\"BigInt does not support unsigned right shift\")");
            }
            _ => {
                // Fallback: try direct operator
                self.emit_expr(left);
                self.write(&format!(
                    " {} ",
                    crate::zigir::emit::helpers::bin_op_to_zig(op)
                ));
                self.emit_expr(right);
            }
        }
    }

    /// Emit a String comparison operation.
    pub(super) fn emit_string_comparison(
        &mut self,
        op: crate::zigir::ops::BinOp,
        left: &crate::zigir::types::IrExpr,
        right: &crate::zigir::types::IrExpr,
    ) {
        use crate::zigir::emit::helpers::bin_op_to_zig;
        use crate::zigir::ops::BinOp;
        match op {
            BinOp::Eq | BinOp::StrictEq => {
                self.write("std.mem.eql(u8, ");
                self.emit_expr(left);
                self.write(", ");
                self.emit_expr(right);
                self.write(")");
            }
            BinOp::Ne | BinOp::StrictNe => {
                self.write("(!std.mem.eql(u8, ");
                self.emit_expr(left);
                self.write(", ");
                self.emit_expr(right);
                self.write("))");
            }
            BinOp::Lt => {
                self.write("(std.mem.order(u8, ");
                self.emit_expr(left);
                self.write(", ");
                self.emit_expr(right);
                self.write(") == .lt)");
            }
            BinOp::Le => {
                self.write("(std.mem.order(u8, ");
                self.emit_expr(left);
                self.write(", ");
                self.emit_expr(right);
                self.write(") != .gt)");
            }
            BinOp::Gt => {
                self.write("(std.mem.order(u8, ");
                self.emit_expr(left);
                self.write(", ");
                self.emit_expr(right);
                self.write(") == .gt)");
            }
            BinOp::Ge => {
                self.write("(std.mem.order(u8, ");
                self.emit_expr(left);
                self.write(", ");
                self.emit_expr(right);
                self.write(") != .lt)");
            }
            _ => {
                // Other string operators (not expected)
                self.emit_expr(left);
                self.write(&format!(" {} ", bin_op_to_zig(op)));
                self.emit_expr(right);
            }
        }
    }

    /// Emit a JsAny comparison operation.
    /// JsAny equality uses .eq()/.strictEq(), ordering uses .asI64().
    pub(super) fn emit_jsany_comparison(
        &mut self,
        op: crate::zigir::ops::BinOp,
        left: &crate::zigir::types::IrExpr,
        right: &crate::zigir::types::IrExpr,
        left_is_jsany: bool,
        right_is_jsany: bool,
    ) {
        use crate::zigir::emit::helpers::bin_op_to_zig;
        use crate::zigir::ops::BinOp;

        // Wrap non-JsAny operand with JsAny.from() if needed
        let emit_left_as_jsany = |emitter: &mut Emitter| {
            if left_is_jsany {
                emitter.emit_expr(left);
            } else {
                emitter.write("JsAny.from(");
                emitter.emit_expr(left);
                emitter.write(")");
            }
        };
        let emit_right_as_jsany = |emitter: &mut Emitter| {
            if right_is_jsany {
                emitter.emit_expr(right);
            } else {
                emitter.write("JsAny.from(");
                emitter.emit_expr(right);
                emitter.write(")");
            }
        };

        match op {
            BinOp::Eq => {
                emit_left_as_jsany(self);
                self.write(".eq(");
                emit_right_as_jsany(self);
                self.write(")");
            }
            BinOp::StrictEq => {
                emit_left_as_jsany(self);
                self.write(".strictEq(");
                emit_right_as_jsany(self);
                self.write(")");
            }
            BinOp::Ne => {
                self.write("!(");
                emit_left_as_jsany(self);
                self.write(".eq(");
                emit_right_as_jsany(self);
                self.write("))");
            }
            BinOp::StrictNe => {
                self.write("!(");
                emit_left_as_jsany(self);
                self.write(".strictEq(");
                emit_right_as_jsany(self);
                self.write("))");
            }
            // Ordering: convert JsAny to i64 for numeric comparison
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                let zig_op = bin_op_to_zig(op);
                // Use .asI64() on JsAny sides for numeric comparison
                if left_is_jsany {
                    self.write("(");
                    self.emit_expr(left);
                    self.write(".asI64())");
                } else {
                    self.emit_expr(left);
                }
                self.write(&format!(" {} ", zig_op));
                if right_is_jsany {
                    self.write("(");
                    self.emit_expr(right);
                    self.write(".asI64())");
                } else {
                    self.emit_expr(right);
                }
            }
            _ => {
                // Fallback
                self.emit_expr(left);
                self.write(&format!(" {} ", bin_op_to_zig(op)));
                self.emit_expr(right);
            }
        }
    }
}
