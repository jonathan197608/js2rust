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
            // Arithmetic with simple catch: _a.op(&_b, alloc) catch @panic("BigInt op OOM")
            BinOp::Add | BinOp::Sub | BinOp::Mul => {
                let (method, label) = match op {
                    BinOp::Add => ("add", "add"),
                    BinOp::Sub => ("sub", "sub"),
                    BinOp::Mul => ("mul", "mul"),
                    _ => unreachable!(),
                };
                self.write("(");
                self.emit_expr(left);
                self.write(&format!(".{}(&", method));
                self.emit_expr(right);
                self.write(&format!(
                    ", js_allocator.allocator()) catch @panic(\"BigInt {} OOM\"))",
                    label
                ));
            }
            // Bitwise with simple catch: _a.op(&_b, alloc) catch @panic("BigInt op OOM")
            BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor => {
                let (method, label) = match op {
                    BinOp::BitAnd => ("bitwiseAnd", "and"),
                    BinOp::BitOr => ("bitwiseOr", "or"),
                    BinOp::BitXor => ("bitwiseXor", "xor"),
                    _ => unreachable!(),
                };
                self.write("(");
                self.emit_expr(left);
                self.write(&format!(".{}(&", method));
                self.emit_expr(right);
                self.write(&format!(
                    ", js_allocator.allocator()) catch @panic(\"BigInt {} OOM\"))",
                    label
                ));
            }
            BinOp::Div => {
                self.write("(");
                self.emit_expr(left);
                self.write(".div(&");
                self.emit_expr(right);
                self.write(", js_allocator.allocator()) catch |err| switch (err) { error.DivisionByZero => return error.JsThrow, else => @panic(\"BigInt div OOM\") })");
            }
            BinOp::Mod => {
                self.write("(");
                self.emit_expr(left);
                self.write(".rem(&");
                self.emit_expr(right);
                self.write(", js_allocator.allocator()) catch |err| switch (err) { error.DivisionByZero => return error.JsThrow, else => @panic(\"BigInt rem OOM\") })");
            }
            BinOp::Pow => {
                self.write("(");
                self.emit_expr(left);
                self.write(".pow(");
                self.emit_expr(right);
                self.write(".toU64() catch @panic(\"BigInt toU64 failed\"), js_allocator.allocator()) catch @panic(\"BigInt pow OOM\"))");
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
            BinOp::Eq | BinOp::StrictEq | BinOp::Ne | BinOp::StrictNe => {
                let negate = matches!(op, BinOp::Ne | BinOp::StrictNe);
                if negate {
                    self.write("(!");
                }
                self.write("std.mem.eql(u8, ");
                self.emit_expr(left);
                self.write(", ");
                self.emit_expr(right);
                self.write(")");
                if negate {
                    self.write(")");
                }
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
            // Equality: Eq/StrictEq and Ne/StrictNe share the same emit logic;
            // only the method name (.eq vs .strictEq) and negation differ.
            BinOp::Eq | BinOp::StrictEq | BinOp::Ne | BinOp::StrictNe => {
                let method = match op {
                    BinOp::StrictEq | BinOp::StrictNe => "strictEq",
                    _ => "eq",
                };
                let negate = matches!(op, BinOp::Ne | BinOp::StrictNe);
                if negate {
                    self.write("!(");
                }
                emit_left_as_jsany(self);
                self.write(&format!(".{}(", method));
                emit_right_as_jsany(self);
                self.write(")");
                if negate {
                    self.write(")");
                }
            }
            // Ordering: use JsAny.from().asI64() for numeric comparison
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                let zig_op = bin_op_to_zig(op);
                // Both sides need to go through JsAny for consistent comparison.
                // For already-JsAny sides, call .asI64() directly.
                // For non-JsAny sides, wrap with JsAny.from() then .asI64().
                if left_is_jsany {
                    self.write("(");
                    self.emit_expr(left);
                    self.write(".asI64())");
                } else {
                    self.write("(JsAny.from(");
                    self.emit_expr(left);
                    self.write(").asI64())");
                }
                self.write(&format!(" {} ", zig_op));
                if right_is_jsany {
                    self.write("(");
                    self.emit_expr(right);
                    self.write(".asI64())");
                } else {
                    self.write("(JsAny.from(");
                    self.emit_expr(right);
                    self.write(").asI64())");
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

    /// Emit a comparison where one operand is BigInt and the other is a numeric type
    /// (I64, F64, Bool) or StringLiteral. Convert the non-BigInt operand to JsBigInt
    /// via fromI64 (for numeric) or JsBigInt.init (for string), then use .eq() / .order().
    pub(super) fn emit_bigint_cross_comparison(
        &mut self,
        op: crate::zigir::ops::BinOp,
        left: &crate::zigir::types::IrExpr,
        right: &crate::zigir::types::IrExpr,
        left_is_bigint: bool,
        right_is_bigint: bool,
    ) {
        use crate::zigir::ops::BinOp;

        // Emit a non-BigInt operand as JsBigInt.
        // For StringLiteral: use JsBigInt.init(allocator, "value")
        // For other types: use JsBigInt.fromI64(allocator, value)
        let emit_as_bigint = |s: &mut Self, expr: &crate::zigir::types::IrExpr| {
            if let crate::zigir::types::IrExpr::StringLiteral(val) = expr {
                s.write("(js_bigint.JsBigInt.init(js_allocator.allocator(), \"");
                s.write(&crate::zigir::emit::helpers::escape_zig_string(val));
                s.write("\") catch @panic(\"OOM: BigInt init\"))");
            } else {
                s.write("(js_bigint.JsBigInt.fromI64(js_allocator.allocator(), ");
                s.emit_expr(expr);
                s.write(") catch @panic(\"BigInt fromI64 OOM\"))");
            }
        };

        match op {
            BinOp::Eq | BinOp::StrictEq | BinOp::Ne | BinOp::StrictNe => {
                let negate = matches!(op, BinOp::Ne | BinOp::StrictNe);
                if negate {
                    self.write("!");
                }
                if left_is_bigint {
                    self.emit_expr(left);
                } else {
                    emit_as_bigint(self, left);
                }
                self.write(".eq(&");
                if right_is_bigint {
                    self.emit_expr(right);
                } else {
                    emit_as_bigint(self, right);
                }
                self.write(")");
            }
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                // Use .order() which returns std.math.Order (.lt, .eq, .gt)
                let cmp_expr =
                    |s: &mut Self, is_bigint: bool, expr: &crate::zigir::types::IrExpr| {
                        if is_bigint {
                            s.emit_expr(expr);
                        } else {
                            emit_as_bigint(s, expr);
                        }
                    };
                // lhs.order(&rhs) compare to expected Order value
                self.write("(");
                cmp_expr(self, left_is_bigint, left);
                self.write(".order(&");
                cmp_expr(self, right_is_bigint, right);
                self.write(") ");
                match op {
                    BinOp::Lt => self.write("== .lt"),
                    BinOp::Le => self.write("!= .gt"),
                    BinOp::Gt => self.write("== .gt"),
                    BinOp::Ge => self.write("!= .lt"),
                    _ => unreachable!(),
                }
                self.write(")");
            }
            _ => unreachable!(),
        }
    }
}
