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

    /// Emit a default binary operator: `left OP right` (no parens).
    /// Shared by bigint/string/jsany fallback and the main Binary dispatch in mod.rs.
    pub(super) fn emit_default_binop(
        &mut self,
        op: crate::zigir::ops::BinOp,
        left: &crate::zigir::types::IrExpr,
        right: &crate::zigir::types::IrExpr,
    ) {
        self.emit_expr(left);
        self.write(&format!(
            " {} ",
            crate::zigir::emit::helpers::bin_op_to_zig(op)
        ));
        self.emit_expr(right);
    }

    /// Emit an ordering comparison: `(order_expr == .lt)` / `!= .gt` etc.
    /// `order_expr_fn` emits the expression that yields `std.math.Order`.
    pub(super) fn emit_order_cmp<F>(&mut self, op: crate::zigir::ops::BinOp, order_expr_fn: F)
    where
        F: FnOnce(&mut Self),
    {
        self.write("(");
        order_expr_fn(self);
        match op {
            crate::zigir::ops::BinOp::Lt => self.write(" == .lt)"),
            crate::zigir::ops::BinOp::Le => self.write(" != .gt)"),
            crate::zigir::ops::BinOp::Gt => self.write(" == .gt)"),
            crate::zigir::ops::BinOp::Ge => self.write(" != .lt)"),
            _ => unreachable!("emit_order_cmp: expected Lt/Le/Gt/Ge, got {:?}", op),
        }
    }

    /// BigInt arithmetic uses method calls like `_a.add(&_b, alloc)`.
    pub(super) fn emit_bigint_binary(
        &mut self,
        op: crate::zigir::ops::BinOp,
        left: &crate::zigir::types::IrExpr,
        right: &crate::zigir::types::IrExpr,
    ) {
        use crate::zigir::ops::BinOp;

        match op {
            // Method calls: _a.op(&_b, alloc) catch ...
            // Division/remainder may throw DivisionByZero; all others just panic on OOM.
            BinOp::Add
            | BinOp::Sub
            | BinOp::Mul
            | BinOp::BitAnd
            | BinOp::BitOr
            | BinOp::BitXor
            | BinOp::Div
            | BinOp::Mod => {
                let (method, label) = match op {
                    BinOp::Add => ("add", "add"),
                    BinOp::Sub => ("sub", "sub"),
                    BinOp::Mul => ("mul", "mul"),
                    BinOp::BitAnd => ("bitwiseAnd", "and"),
                    BinOp::BitOr => ("bitwiseOr", "or"),
                    BinOp::BitXor => ("bitwiseXor", "xor"),
                    BinOp::Div => ("div", "div"),
                    BinOp::Mod => ("rem", "rem"),
                    _ => unreachable!("emit_bigint_binary: unsupported BigInt op {:?}", op),
                };
                self.write("(");
                self.emit_expr(left);
                self.write(&format!(".{}(&", method));
                self.emit_expr(right);
                self.write(", js_allocator.allocator()) catch ");
                if matches!(op, BinOp::Div | BinOp::Mod) {
                    self.write(&format!(
                        "|err| switch (err) {{ error.DivisionByZero => return error.JsThrow, else => @panic(\"BigInt {} OOM\") }})",
                        label
                    ));
                } else {
                    self.write(&format!("@panic(\"BigInt {} OOM\"))", label));
                }
            }
            BinOp::Pow => {
                self.write("(");
                self.emit_expr(left);
                self.write(".pow(");
                self.emit_expr(right);
                self.write(".toU64() catch @panic(\"BigInt toU64 failed\"), js_allocator.allocator()) catch @panic(\"BigInt pow OOM\"))");
            }
            BinOp::Shl | BinOp::Shr => {
                let (method, label) = match op {
                    BinOp::Shl => ("shiftLeft", "shl"),
                    BinOp::Shr => ("shiftRight", "shr"),
                    _ => unreachable!("emit_bigint_binary: unsupported shift op {:?}", op),
                };
                self.write("(");
                self.emit_expr(left);
                self.write(&format!(".{}(@as(usize, @intCast(", method));
                self.emit_expr(right);
                self.write(&format!(
                    ".toU64() catch @panic(\"BigInt toU64 failed\"))), js_allocator.allocator()) catch @panic(\"BigInt {} OOM\"))",
                    label
                ));
            }
            // Equality
            BinOp::Eq | BinOp::StrictEq | BinOp::Ne | BinOp::StrictNe => {
                let negate = matches!(op, BinOp::Ne | BinOp::StrictNe);
                if negate {
                    self.write("!");
                }
                self.emit_expr(left);
                self.write(".eq(&");
                self.emit_expr(right);
                self.write(")");
            }
            // Ordering
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                let left_clone = left.clone();
                let right_clone = right.clone();
                self.emit_order_cmp(op, |emitter| {
                    emitter.emit_expr(&left_clone);
                    emitter.write(".order(&");
                    emitter.emit_expr(&right_clone);
                    emitter.write(")");
                });
            }
            // >>> is not supported for BigInt (JS throws TypeError at runtime)
            BinOp::UrShr => {
                self.write(
                    "@panic(\"TypeError: BigInt does not support unsigned right shift (>>>)\")",
                );
            }
            _ => {
                self.emit_default_binop(op, left, right);
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
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                let left_clone = left.clone();
                let right_clone = right.clone();
                self.emit_order_cmp(op, |emitter| {
                    emitter.write("std.mem.order(u8, ");
                    emitter.emit_expr(&left_clone);
                    emitter.write(", ");
                    emitter.emit_expr(&right_clone);
                    emitter.write(")");
                });
            }
            _ => {
                self.emit_default_binop(op, left, right);
            }
        }
    }

    /// Emit a JsAny comparison operation.
    /// JsAny equality uses .eq()/.strictEq(), ordering uses .asI64().
    /// Emit an expression, optionally wrapping it in `JsAny.from()` if it
    /// is not already a JsAny-typed value.
    fn emit_expr_as_jsany(&mut self, expr: &crate::zigir::types::IrExpr, is_jsany: bool) {
        if is_jsany {
            self.emit_expr(expr);
        } else {
            self.write("JsAny.from(");
            self.emit_expr(expr);
            self.write(")");
        }
    }

    /// Emit an expression as `.asI64()`, wrapping with `JsAny.from()` first
    /// if it is not already a JsAny-typed value.
    fn emit_expr_as_i64(&mut self, expr: &crate::zigir::types::IrExpr, is_jsany: bool) {
        self.write("(");
        if is_jsany {
            self.emit_expr(expr);
            self.write(".asI64())");
        } else {
            self.write("JsAny.from(");
            self.emit_expr(expr);
            self.write(").asI64())");
        }
    }

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
                self.emit_expr_as_jsany(left, left_is_jsany);
                self.write(&format!(".{}(", method));
                self.emit_expr_as_jsany(right, right_is_jsany);
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
                self.emit_expr_as_i64(left, left_is_jsany);
                self.write(&format!(" {} ", zig_op));
                self.emit_expr_as_i64(right, right_is_jsany);
            }
            _ => {
                self.emit_default_binop(op, left, right);
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
                let left_clone = left.clone();
                let right_clone = right.clone();
                self.emit_order_cmp(op, |emitter| {
                    let cmp_expr =
                        |s: &mut Self, is_bigint: bool, expr: &crate::zigir::types::IrExpr| {
                            if is_bigint {
                                s.emit_expr(expr);
                            } else {
                                emit_as_bigint(s, expr);
                            }
                        };
                    cmp_expr(emitter, left_is_bigint, &left_clone);
                    emitter.write(".order(&");
                    cmp_expr(emitter, right_is_bigint, &right_clone);
                    emitter.write(")");
                });
            }
            _ => unreachable!("emit_binary: unsupported BinOp {:?}", op),
        }
    }

    /// Emit a String + BigInt concatenation.
    /// JS spec: `"hello" + 5n` → `"hello5"`, `5n + "hello"` → `"5hello"`.
    /// Converts the BigInt operand to its decimal string representation,
    /// then concatenates with the String operand using `std.fmt.allocPrint`.
    pub(super) fn emit_bigint_string_concat(
        &mut self,
        left: &crate::zigir::types::IrExpr,
        right: &crate::zigir::types::IrExpr,
        left_is_str: bool,
    ) {
        // Determine which side is string and which is BigInt
        let (str_expr, bigint_expr) = if left_is_str {
            (left, right)
        } else {
            (right, left)
        };
        let str_on_left = left_is_str;

        self.write("(blk: { var __buf = std.ArrayList(u8).init(js_allocator.allocator()); errdefer __buf.deinit(); ");
        // Write the first part
        if str_on_left {
            self.write("__buf.writer().print(\"{s}\", .{");
            self.emit_expr(str_expr);
            self.write("}) catch @panic(\"OOM: string concat\"); ");
        } else {
            self.write("__buf.writer().print(\"{f}\", .{");
            self.emit_expr(bigint_expr);
            self.write("}) catch @panic(\"OOM: bigint string concat\"); ");
        }
        // Write the second part
        if str_on_left {
            self.write("__buf.writer().print(\"{f}\", .{");
            self.emit_expr(bigint_expr);
            self.write("}) catch @panic(\"OOM: bigint string concat\"); ");
        } else {
            self.write("__buf.writer().print(\"{s}\", .{");
            self.emit_expr(str_expr);
            self.write("}) catch @panic(\"OOM: string concat\"); ");
        }
        self.write("break :blk __buf.toOwnedSlice() catch @panic(\"OOM: string concat\"); })");
    }
}
