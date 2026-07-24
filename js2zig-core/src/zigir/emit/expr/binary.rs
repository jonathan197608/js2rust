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
    /// - JsAny: call `.asF64()` (preserves the JsAny value's float payload)
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
            crate::types::ZigType::JsAny => {
                self.emit_expr(expr);
                self.write(".asF64()");
            }
            _ => {
                // comptime_int, bool, or unknown — @as(f64, expr) works for comptime_int
                self.write("@as(f64, ");
                self.emit_expr(expr);
                self.write(")");
            }
        }
    }

    /// Emit a default binary operator: `(left OP right)`.
    /// Shared by bigint/string/jsany fallback and the main Binary dispatch in mod.rs.
    ///
    /// The outer parentheses are MANDATORY: JS and Zig disagree on operator
    /// precedence for some operator pairs (most notably, JS `+` > `<<` but
    /// Zig `<<` > `+`). Without wrapping each binary expression in parens,
    /// flat emission like `a + b << c` would be re-parsed by Zig with
    /// different grouping than the JS AST intended.
    pub(super) fn emit_default_binop(
        &mut self,
        op: crate::zigir::ops::BinOp,
        left: &crate::zigir::types::IrExpr,
        right: &crate::zigir::types::IrExpr,
    ) {
        self.write("(");
        self.emit_expr(left);
        self.write(&format!(
            " {} ",
            crate::zigir::emit::helpers::bin_op_to_zig(op)
        ));
        self.emit_expr(right);
        self.write(")");
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
                    if let Some(try_label) = &self.inside_try_block {
                        self.write(&format!(
                            "|err| switch (err) {{ error.DivisionByZero => break :{} @as(anyerror!void, error.JsThrow), else => @panic(\"BigInt {} OOM\") }})",
                            try_label, label
                        ));
                    } else {
                        self.write(&format!(
                            "|err| switch (err) {{ error.DivisionByZero => return error.JsThrow, else => @panic(\"BigInt {} OOM\") }})",
                            label
                        ));
                    }
                } else {
                    self.write(&format!("@panic(\"BigInt {} OOM\"))", label));
                }
            }
            BinOp::Pow => {
                self.write("(");
                self.emit_expr(left);
                self.write(".pow(");
                self.emit_expr(right);
                // `BigInt.pow` returns `error.RangeError` when `exp > maxInt(u32)`
                // (R8 P0-4); converting to `error.JsThrow` lets the surrounding
                // try-catch machinery surface it as a JS TypeError instead of
                // a Zig runtime panic.
                if let Some(try_label) = &self.inside_try_block {
                    self.write(&format!(
                        ".toU64() catch break :{} @as(anyerror!void, error.JsThrow), js_allocator.allocator()) catch |err| switch (err) {{ error.RangeError => break :{} @as(anyerror!void, error.JsThrow), else => @panic(\"BigInt pow OOM\") }})",
                        try_label, try_label
                    ));
                } else {
                    self.write(".toU64() catch return error.JsThrow, js_allocator.allocator()) catch |err| switch (err) { error.RangeError => return error.JsThrow, else => @panic(\"BigInt pow OOM\") })");
                }
            }
            BinOp::Shl | BinOp::Shr => {
                let (method, label) = match op {
                    BinOp::Shl => ("shiftLeft", "shl"),
                    BinOp::Shr => ("shiftRight", "shr"),
                    _ => unreachable!("emit_bigint_binary: unsupported shift op {:?}", op),
                };
                self.write("(");
                self.emit_expr(left);
                self.write(&format!(".{}(", method));
                self.emit_expr(right);
                if let Some(try_label) = &self.inside_try_block {
                    self.write(&format!(
                        ".toI64() catch break :{} @as(anyerror!void, error.JsThrow), js_allocator.allocator()) catch @panic(\"BigInt {} OOM\"))",
                        try_label, label
                    ));
                } else {
                    self.write(&format!(
                        ".toI64() catch return error.JsThrow, js_allocator.allocator()) catch @panic(\"BigInt {} OOM\"))",
                        label
                    ));
                }
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
                if let Some(label) = &self.inside_try_block {
                    self.write(&format!(
                        "({{ break :{} @as(anyerror!void, error.JsThrow); }})",
                        label
                    ));
                } else {
                    self.write("({ return error.JsThrow; })");
                }
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
    /// JsAny equality uses .eq()/.strictEq(), ordering uses .asF64() (preserves
    /// float precision since JS numbers are doubles).
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

    /// Emit a JsAny arithmetic operation.
    ///
    /// Coerces JsAny side(s) via `.asI64()` by default (to match the inferred
    /// integer result type for `i64 op JsAny` cases). However, when the OTHER
    /// side has type F64, the JS result must be a float — use `.asF64()` in
    /// that case to avoid Zig `i64 + f64` type errors AND to preserve float
    /// precision (otherwise `5.0 + jsany` would compute `5 + jsany.asI64()`,
    /// losing the float register and possibly truncating the JsAny value).
    ///
    /// The signature carries two extra type args (`left_type` / `right_type`)
    /// so the emitter can decide `.asF64()` vs `.asI64()` based on the OTHER
    /// side's type. Clippy's default 7-arg threshold is exceeded; suppressing
    /// the lint here keeps the call-site (Binary dispatch in mod.rs) readable.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn emit_jsany_arithmetic(
        &mut self,
        op: crate::zigir::ops::BinOp,
        left: &crate::zigir::types::IrExpr,
        right: &crate::zigir::types::IrExpr,
        left_is_jsany: bool,
        right_is_jsany: bool,
        left_type: Option<&crate::types::ZigType>,
        right_type: Option<&crate::types::ZigType>,
    ) {
        use crate::types::ZigType;
        use crate::zigir::emit::helpers::bin_op_to_zig;

        // Coercion decision: if the OTHER side is F64, use .asF64(); otherwise .asI64().
        // The "other" side governs because Zig binary ops require both operands to
        // share a type. If one side is f64, the other must be f64 too.
        let left_coerce_f64 = left_is_jsany && right_type == Some(&ZigType::F64);
        let right_coerce_f64 = right_is_jsany && left_type == Some(&ZigType::F64);

        // Outer parens protect against JS/Zig operator precedence mismatches
        // (e.g. nested `JsAny + JsAny << JsAny` would be re-grouped by Zig
        // without the wrapping). They also ensure `.asI64()`/`.asF64()` appended
        // after emit_expr binds to the entire sub-expression, not just the
        // trailing token.
        self.write("(");
        // Left side
        if left_is_jsany {
            self.emit_expr(left);
            if left_coerce_f64 {
                self.write(".asF64()");
            } else {
                self.write(".asI64()");
            }
        } else {
            self.emit_expr(left);
        }
        self.write(&format!(" {} ", bin_op_to_zig(op)));
        // Right side
        if right_is_jsany {
            self.emit_expr(right);
            if right_coerce_f64 {
                self.write(".asF64()");
            } else {
                self.write(".asI64()");
            }
        } else {
            self.emit_expr(right);
        }
        self.write(")");
    }

    pub(super) fn emit_jsany_comparison(
        &mut self,
        op: crate::zigir::ops::BinOp,
        left: &crate::zigir::types::IrExpr,
        right: &crate::zigir::types::IrExpr,
        left_is_jsany: bool,
        right_is_jsany: bool,
    ) {
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
            // Ordering: delegate to JsAny runtime methods (lt/le/gt/ge).
            // R8-P1-16: Previously inlined .asF64() which always compared
            // numerically, making string<string always return false.
            // The runtime methods now check isString() for lexicographic
            // comparison before falling back to numeric asF64().
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                let method = match op {
                    BinOp::Lt => "lt",
                    BinOp::Le => "le",
                    BinOp::Gt => "gt",
                    BinOp::Ge => "ge",
                    _ => unreachable!(),
                };
                self.emit_expr_as_jsany(left, left_is_jsany);
                self.write(&format!(".{}(", method));
                self.emit_expr_as_jsany(right, right_is_jsany);
                self.write(")");
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

        // Use a unique label rather than the hardcoded `blk:` so nested
        // BigInt-string concatenations (e.g. `("a" + 1n) + 2n` if both `+`
        // ops resolve to string concat) do not produce `redefinition of
        // label 'blk'` in Zig.
        let label = self.next_label();
        self.write(&format!(
            "({}: {{ var __buf = std.ArrayList(u8).init(js_allocator.allocator()); errdefer __buf.deinit(); ",
            label
        ));
        // Write the first part
        if str_on_left {
            self.write("__buf.writer().print(\"{s}\", .{");
            self.emit_expr(str_expr);
            self.write("}) catch @panic(\"OOM: string concat\"); ");
        } else {
            self.write("__buf.writer().print(\"{}\", .{");
            self.emit_expr(bigint_expr);
            self.write("}) catch @panic(\"OOM: bigint string concat\"); ");
        }
        // Write the second part
        if str_on_left {
            self.write("__buf.writer().print(\"{}\", .{");
            self.emit_expr(bigint_expr);
            self.write("}) catch @panic(\"OOM: bigint string concat\"); ");
        } else {
            self.write("__buf.writer().print(\"{s}\", .{");
            self.emit_expr(str_expr);
            self.write("}) catch @panic(\"OOM: string concat\"); ");
        }
        self.write(&format!(
            "break :{} __buf.toOwnedSlice() catch @panic(\"OOM: string concat\"); }})",
            label
        ));
    }
}
