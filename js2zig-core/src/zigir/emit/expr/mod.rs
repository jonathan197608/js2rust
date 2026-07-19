// zigir/emit/expr/mod.rs
// Expression-level Zig emission from IrExpr nodes.

pub mod binary;
pub mod call_member;
pub mod container;
pub mod template_new;

use crate::zigir::emit::Emitter;
use crate::zigir::emit::helpers as emit_helpers;
use crate::zigir::emit::helpers::{EmitterHelpers, escape_zig_string, update_op_to_zig};

// ═══════════════════════════════════════════════════════
//  Expression dispatch
// ═══════════════════════════════════════════════════════

impl Emitter {
    pub(crate) fn emit_expr(&mut self, expr: &crate::zigir::types::IrExpr) {
        match expr {
            crate::zigir::types::IrExpr::IntLiteral(n) => {
                self.write(&n.to_string());
            }

            crate::zigir::types::IrExpr::FloatLiteral(n) => {
                if *n == -0.0 && n.is_sign_negative() {
                    self.write("-0.0");
                } else {
                    self.write(&n.to_string());
                }
            }

            crate::zigir::types::IrExpr::StringLiteral(s) => {
                self.write(&format!("\"{}\"", escape_zig_string(s)));
            }

            crate::zigir::types::IrExpr::BoolLiteral(b) => {
                self.write(if *b { "true" } else { "false" });
            }

            crate::zigir::types::IrExpr::BigIntLiteral(s) => {
                // Wrap in parentheses: `catch` has lower precedence than `+`/`-`,
                // so `BigInt.init(...) catch @panic(...) + x` would parse incorrectly.
                self.write(&format!(
                    "(js_bigint.JsBigInt.init(js_allocator.allocator(), \"{}\") catch @panic(\"OOM: BigInt init\"))",
                    escape_zig_string(s)
                ));
            }

            crate::zigir::types::IrExpr::Null => {
                self.write("JsAny.fromNull()");
            }

            crate::zigir::types::IrExpr::Undefined => {
                self.write("JsAny.fromUndefined()");
            }

            crate::zigir::types::IrExpr::Ident(ident) => {
                self.write(&ident.zig_name);
            }

            crate::zigir::types::IrExpr::This => {
                self.write("self");
            }

            // ── Arithmetic / comparison ─────────────
            crate::zigir::types::IrExpr::Binary {
                op,
                left,
                right,
                left_type,
                right_type,
            } => {
                use crate::types::ZigType;
                use crate::zigir::ops::BinOp;

                // Determine if operands are BigInt, JsAny, String, or other
                let lt = left_type.as_ref();
                let rt = right_type.as_ref();
                let left_is_bigint = lt == Some(&ZigType::BigInt);
                let right_is_bigint = rt == Some(&ZigType::BigInt);
                // Check both type inference AND the expression node itself.
                // String literals like "5" often have `left_type = None` since they
                // lack a JSDoc annotation, but `IrExpr::StringLiteral` is unambiguous.
                let left_is_str = lt == Some(&ZigType::Str)
                    || matches!(&**left, crate::zigir::types::IrExpr::StringLiteral(_));
                let right_is_str = rt == Some(&ZigType::Str)
                    || matches!(&**right, crate::zigir::types::IrExpr::StringLiteral(_));
                let left_is_jsany = lt == Some(&ZigType::JsAny);
                let right_is_jsany = rt == Some(&ZigType::JsAny);
                let left_is_float = lt == Some(&ZigType::F64);
                let right_is_float = rt == Some(&ZigType::F64);
                let left_is_symbol = lt == Some(&ZigType::JsSymbol);
                let right_is_symbol = rt == Some(&ZigType::JsSymbol);
                // Pre-compute cross-type flags (needed for the if/else-if chain below)
                // Anytype should not count as a distinct type — Zig resolves it at comptime.
                let left_is_anytype = lt == Some(&ZigType::Anytype);
                let right_is_anytype = rt == Some(&ZigType::Anytype);
                let cross_type_known = lt.is_some()
                    && rt.is_some()
                    && lt != rt
                    && !left_is_bigint
                    && !right_is_bigint
                    && !left_is_anytype
                    && !right_is_anytype;
                let str_vs_numeric = (left_is_str && !right_is_str && !right_is_bigint)
                    || (right_is_str && !left_is_str && !left_is_bigint);

                // ── BigInt arithmetic/comparison ──
                if left_is_bigint && right_is_bigint {
                    self.emit_bigint_binary(*op, left, right);
                }
                // ── Symbol equality/comparison ──
                // JsSymbol is a struct with slice fields; Zig doesn't support == on it.
                // Use .eql() for equality, and .id for ordering (shouldn't occur in practice).
                else if left_is_symbol
                    && right_is_symbol
                    && matches!(
                        op,
                        BinOp::Eq | BinOp::Ne | BinOp::StrictEq | BinOp::StrictNe
                    )
                {
                    let negate = matches!(*op, BinOp::Ne | BinOp::StrictNe);
                    if negate {
                        self.write("!");
                    }
                    self.emit_expr(left);
                    self.write(".eql(");
                    self.emit_expr(right);
                    self.write(")");
                }
                // ── String + BigInt concatenation ──
                // JS spec: "hello" + 5n → "hello5", 5n + "hello" → "5hello"
                // Only the Add operator allows String+BigInt; all other mixed ops throw TypeError.
                else if *op == BinOp::Add
                    && ((left_is_str && right_is_bigint) || (left_is_bigint && right_is_str))
                {
                    self.emit_bigint_string_concat(left, right, left_is_str);
                }
                // ── BigInt × Number non-comparison ops: JS throws TypeError ──
                // e.g. 1n + 2, 3n & x, 2n ** 3 where x: number
                // JS spec: "Cannot mix BigInt and other types, consider explicit conversions"
                else if (left_is_bigint || right_is_bigint)
                    && matches!(
                        op,
                        BinOp::Add
                            | BinOp::Sub
                            | BinOp::Mul
                            | BinOp::Div
                            | BinOp::Mod
                            | BinOp::Pow
                            | BinOp::BitAnd
                            | BinOp::BitOr
                            | BinOp::BitXor
                            | BinOp::Shl
                            | BinOp::Shr
                    )
                {
                    if let Some(label) = &self.inside_try_block {
                        self.write(&format!(
                            "({{ break :{} @as(anyerror!void, error.JsThrow); }})",
                            label
                        ));
                    } else {
                        self.write("({ return error.JsThrow; })");
                    }
                }
                // ── String equality/comparison ──
                else if left_is_str && right_is_str {
                    self.emit_string_comparison(*op, left, right);
                }
                // ── BigInt cross-type comparison (e.g. 0n === 0, bigint == 5) ──
                // Convert the non-BigInt operand to JsBigInt and use .eq() / .order()
                else if (left_is_bigint || right_is_bigint)
                    && matches!(
                        op,
                        BinOp::Eq
                            | BinOp::Ne
                            | BinOp::StrictEq
                            | BinOp::StrictNe
                            | BinOp::Lt
                            | BinOp::Le
                            | BinOp::Gt
                            | BinOp::Ge
                    )
                {
                    self.emit_bigint_cross_comparison(
                        *op,
                        left,
                        right,
                        left_is_bigint,
                        right_is_bigint,
                    );
                }
                // ── Cross-type comparison (e.g. String vs Number, Bool vs I64) ──
                // When both types are known but different, Zig's == / != / < etc.
                // are type-mismatch errors. Route through JsAny comparison instead.
                // EXCEPTION: exclude BigInt — JsAny.from() does not support BigInt,
                // and BigInt cross-type comparisons need a different strategy.
                //
                // Also handle String vs numeric (even when numeric side type is unknown,
                // e.g. "5" > 3 where 3 is a comptime_int literal).
                else if (cross_type_known || str_vs_numeric)
                    && matches!(
                        op,
                        BinOp::Eq
                            | BinOp::Ne
                            | BinOp::StrictEq
                            | BinOp::StrictNe
                            | BinOp::Lt
                            | BinOp::Le
                            | BinOp::Gt
                            | BinOp::Ge
                    )
                {
                    // Neither side is JsAny here (that branch is above),
                    // but emit_jsany_comparison will wrap both in JsAny.from().
                    self.emit_jsany_comparison(*op, left, right, false, false);
                }
                // ── JsAny comparison ──
                else if (left_is_jsany || right_is_jsany)
                    && matches!(
                        op,
                        BinOp::Eq
                            | BinOp::Ne
                            | BinOp::StrictEq
                            | BinOp::StrictNe
                            | BinOp::Lt
                            | BinOp::Le
                            | BinOp::Gt
                            | BinOp::Ge
                    )
                {
                    self.emit_jsany_comparison(*op, left, right, left_is_jsany, right_is_jsany);
                }
                // ── JsAny arithmetic: convert JsAny side to i64 via .asI64()
                // (or to f64 via .asF64() when the other side is F64) ──
                // This handles cases like `sum + mapValue` where mapValue is JsAny
                // (e.g., from Map/Set for-of iteration or forEach callbacks).
                else if (left_is_jsany || right_is_jsany)
                    && matches!(
                        op,
                        BinOp::Add
                            | BinOp::Sub
                            | BinOp::Mul
                            | BinOp::BitAnd
                            | BinOp::BitOr
                            | BinOp::BitXor
                            | BinOp::Shl
                            | BinOp::Shr
                    )
                {
                    self.emit_jsany_arithmetic(
                        *op,
                        left,
                        right,
                        left_is_jsany,
                        right_is_jsany,
                        lt,
                        rt,
                    );
                }
                // ── Division / Remainder ──
                // Note: Integer `%` is handled by RemExpr node, not Binary(Mod).
                // Binary(Mod) is only reached for Float and BigInt operands.
                else if *op == BinOp::Div || *op == BinOp::Mod {
                    if left_is_float || right_is_float {
                        if *op == BinOp::Mod {
                            self.write("@rem(");
                            self.emit_expr(left);
                            self.write(", ");
                            self.emit_expr(right);
                            self.write(")");
                        } else {
                            self.write("(");
                            self.emit_expr(left);
                            self.write(" / ");
                            self.emit_expr(right);
                            self.write(")");
                        }
                    } else if *op == BinOp::Div {
                        // JS `/` always returns float (5/2 === 2.5).
                        // Convert integer operands to f64 before division.
                        self.write("(@as(f64, @floatFromInt(");
                        self.emit_expr(left);
                        self.write(")) / @as(f64, @floatFromInt(");
                        self.emit_expr(right);
                        self.write(")))");
                    } else {
                        // BigInt %: emit via bigint binary method
                        self.emit_bigint_binary(BinOp::Mod, left, right);
                    }
                }
                // ── Unsigned right shift ──
                // BigInt × any: JS throws TypeError at runtime
                else if *op == BinOp::UrShr && (left_is_bigint || right_is_bigint) {
                    if let Some(label) = &self.inside_try_block {
                        self.write(&format!(
                            "({{ break :{} @as(anyerror!void, error.JsThrow); }})",
                            label
                        ));
                    } else {
                        self.write("({ return error.JsThrow; })");
                    }
                }
                // ── Unsigned right shift (non-BigInt) ──
                else if *op == BinOp::UrShr {
                    self.write("@as(i64, @intCast(@as(u32, @bitCast(@as(i32, @truncate(");
                    self.emit_expr(left);
                    self.write(")))) >> @intCast(");
                    self.emit_expr(right);
                    self.write(" & 31)))");
                }
                // ── `in` operator: key in obj → obj.has(JsAny.from(key)) for Map/Set, obj.contains(key) otherwise ──
                else if *op == BinOp::In {
                    // Right side is the object, left side is the key (operands swapped for .has()/.contains())
                    let is_map_or_set = matches!(
                        right_type,
                        Some(ZigType::NamedStruct(n)) if n == "Map" || n == "Set"
                    );
                    self.emit_expr(right);
                    if is_map_or_set {
                        self.write(".has(JsAny.from(");
                        self.emit_expr(left);
                        self.write("))");
                    } else {
                        self.write(".contains(");
                        self.emit_expr(left);
                        self.write(")");
                    }
                }
                // ── Default: direct operator ──
                else {
                    self.emit_default_binop(*op, left, right);
                }
            }

            crate::zigir::types::IrExpr::PowExpr {
                base,
                exp,
                base_type,
                exp_type,
                result_type,
            } => {
                // JS `**` always returns f64. Use std.math.pow(f64, ...) with
                // temporary f64 variables in a labeled block.
                let pow_id = self.peek_label_id();
                let blk = self.next_label();
                // If result_type is i64, wrap the entire block in @as(i64, @intFromFloat(...))
                if let Some(crate::types::ZigType::I64) = result_type {
                    self.write("@as(i64, @intFromFloat(");
                }
                self.write(&format!("({blk}: {{ "));
                self.write(&format!("const _base_f64_{pow_id}: f64 = "));
                self.emit_float_conversion(base, base_type);
                self.write(&format!("; const _exp_f64_{pow_id}: f64 = "));
                self.emit_float_conversion(exp, exp_type);
                self.write(&format!(
                    "; break :{blk} std.math.pow(f64, _base_f64_{pow_id}, _exp_f64_{pow_id}); }})",
                ));
                if let Some(crate::types::ZigType::I64) = result_type {
                    self.write("))");
                }
            }

            crate::zigir::types::IrExpr::RemExpr {
                left,
                right,
                left_type,
                right_type,
                result_type,
            } => {
                // JS `%` for integer operands: always uses jsRem which returns f64
                // (preserves signed zero -0). When result_type is i64, wrap in
                // @as(i64, @intFromFloat(...)) for assignment to i64 variable.
                //
                // For JsAny operands, route through `@rem(f64, f64)` with
                // `.asF64()` coercion — this preserves the float payload
                // (e.g. `JsAny.from(5.7) % 2` must give 1.7, not 5 % 2 = 1).
                // Mirrors the DivExpr JsAny path. @rem on f64 preserves IEEE
                // 754 signed-zero (-0) semantics just like jsRem does for i64.
                //
                // BigInt and F64 are routed to `Binary(Mod)` by the lowerer
                // and never reach this node. comptime_int operands emit
                // directly as comptime_int and are accepted by jsRem's i64
                // parameter via comptime coercion.
                use crate::types::ZigType;
                let left_is_jsany = *left_type == ZigType::JsAny;
                let right_is_jsany = *right_type == ZigType::JsAny;

                if let Some(ZigType::I64) = result_type {
                    self.write("@as(i64, @intFromFloat(");
                }
                if left_is_jsany || right_is_jsany {
                    // At least one operand is JsAny: use @rem(f64, f64) to
                    // preserve the float payload. emit_float_conversion maps
                    // F64 → direct, I64/BigInt → @floatFromInt, JsAny →
                    // .asF64(), comptime_int → @as(f64, ...).
                    self.write("@rem(");
                    self.emit_float_conversion(left, left_type);
                    self.write(", ");
                    self.emit_float_conversion(right, right_type);
                    self.write(")");
                } else {
                    // Pure integer operands (I64, comptime_int, BigInt-only
                    // here because lowerer routes BigInt to Binary(Mod) — so
                    // we actually only see I64/comptime_int at this point).
                    // Use jsRem(i64, i64)→f64 for signed-zero preservation.
                    self.write("js_runtime.jsRem(");
                    self.emit_expr(left);
                    self.write(", ");
                    self.emit_expr(right);
                    self.write(")");
                }
                if let Some(ZigType::I64) = result_type {
                    self.write("))");
                }
            }

            crate::zigir::types::IrExpr::DivExpr {
                left,
                right,
                left_type,
                right_type,
                result_type,
            } => {
                // JS `/` always returns float. For integer operands, convert to f64 first.
                // For JsAny operands, .asF64() preserves the float payload.
                // When result_type is i64, wrap in @as(i64, @intFromFloat(...)).
                use crate::types::ZigType;
                let left_is_float = *left_type == ZigType::F64;
                let right_is_float = *right_type == ZigType::F64;
                let left_is_jsany = *left_type == ZigType::JsAny;
                let right_is_jsany = *right_type == ZigType::JsAny;

                if let Some(ZigType::I64) = result_type {
                    self.write("@as(i64, @intFromFloat(");
                }
                if left_is_float || right_is_float || left_is_jsany || right_is_jsany {
                    // At least one operand is F64 (already float) or JsAny (.asF64()):
                    // route both through emit_float_conversion which handles each
                    // case appropriately (no @floatFromInt needed on already-numeric).
                    self.write("(");
                    self.emit_float_conversion(left, left_type);
                    self.write(" / ");
                    self.emit_float_conversion(right, right_type);
                    self.write(")");
                } else {
                    // Both operands are integer-ish (I64, BigInt, comptime_int,
                    // Anytype-int-coercible). Wrap both in @as(f64, @floatFromInt(...)).
                    // This matches the pre-Round-4 behavior and works for anytype params.
                    self.write("(@as(f64, @floatFromInt(");
                    self.emit_expr(left);
                    self.write(")) / @as(f64, @floatFromInt(");
                    self.emit_expr(right);
                    self.write(")))");
                }
                if let Some(ZigType::I64) = result_type {
                    self.write("))");
                }
            }

            crate::zigir::types::IrExpr::Unary {
                op,
                operand,
                operand_type,
            } => {
                match op {
                    crate::zigir::ops::UnaOp::Neg => {
                        self.write("-");
                        self.emit_expr(operand);
                    }
                    crate::zigir::ops::UnaOp::Not => {
                        self.write("!");
                        // Coerce operand to bool first — JS `!x` works on any type,
                        // but Zig `!` requires a bool operand.
                        self.emit_expr_as_bool(operand);
                    }
                    crate::zigir::ops::UnaOp::BitNot => {
                        // JS `~x` operates on 32-bit integer. Convert operand to i32 first.
                        // For f64 operands: @intFromFloat → i64, then @intCast → i32.
                        // For integer/comptime operands: @intCast works directly.
                        if let Some(crate::types::ZigType::F64) = operand_type {
                            self.write("~@as(i32, @intCast(@as(i64, @intFromFloat(");
                            self.emit_expr(operand);
                            self.write("))))");
                        } else {
                            self.write("~@as(i32, @intCast(");
                            self.emit_expr(operand);
                            self.write("))");
                        }
                    }
                    crate::zigir::ops::UnaOp::Void => {
                        // void expr → evaluate and discard
                        self.emit_expr(operand);
                    }
                    // TypeOf and Delete are resolved by the lowerer into
                    // StringLiteral / BuiltinCall / CompileError before
                    // reaching the emit layer.  These arms are unreachable
                    // but must exist for exhaustive match coverage.
                    _ => unreachable!("typeof/delete should be resolved at lower time"),
                }
            }

            crate::zigir::types::IrExpr::Logical {
                op,
                left,
                right,
                left_type,
                right_type,
            } => {
                // Value-returning logical operators (JS semantics):
                //   a && b → returns a if falsy, else b
                //   a || b → returns a if truthy, else b
                //   a ?? b → returns a if not null/undefined, else b
                //
                // Emitted as a Zig labeled-block if-expression so the result
                // preserves the operand type instead of flattening to bool.
                let id = self.peek_label_id();
                let blk = self.next_label();
                let lt = left_type.as_ref();
                let rt = right_type.as_ref();
                let same_type = lt.is_some() && rt.is_some() && lt == rt;

                // When same_type, add an explicit type annotation to the temp var
                // so that comptime_int / comptime_float values are materialized
                // as proper runtime types (e.g., i64, f64). Without this, Zig
                // rejects comptime-only values in runtime control flow branches.
                self.write(&format!("({blk}: {{ const _lv_{id}"));
                if same_type {
                    self.write(&format!(": {}", lt.unwrap().to_zig_type()));
                }
                self.write(" = ");
                self.emit_expr(left);
                self.write("; ");

                match op {
                    crate::zigir::ops::LogicalOp::And => {
                        // a && b: if truthy(a) → b, else → a
                        self.write("if (js_runtime.isTruthy(_lv_");
                        self.write(&format!("{id}))"));
                        self.write(&format!(" break :{blk} "));
                        if same_type {
                            self.emit_expr(right);
                        } else {
                            self.write("JsAny.from(");
                            self.emit_expr(right);
                            self.write(")");
                        }
                        self.write(&format!(" else break :{blk} "));
                        if same_type {
                            self.write(&format!("_lv_{id}"));
                        } else {
                            self.write(&format!("JsAny.from(_lv_{id})"));
                        }
                    }
                    crate::zigir::ops::LogicalOp::Or => {
                        // a || b: if truthy(a) → a, else → b
                        self.write("if (js_runtime.isTruthy(_lv_");
                        self.write(&format!("{id}))"));
                        self.write(&format!(" break :{blk} "));
                        if same_type {
                            self.write(&format!("_lv_{id}"));
                        } else {
                            self.write(&format!("JsAny.from(_lv_{id})"));
                        }
                        self.write(&format!(" else break :{blk} "));
                        if same_type {
                            self.emit_expr(right);
                        } else {
                            self.write("JsAny.from(");
                            self.emit_expr(right);
                            self.write(")");
                        }
                    }
                    crate::zigir::ops::LogicalOp::Nullish => {
                        // a ?? b: if a is not null/undefined → a, else → b
                        self.write(&format!("if (!_lv_{id}.isNullish())"));
                        self.write(&format!(" break :{blk} "));
                        if same_type {
                            self.write(&format!("_lv_{id}"));
                        } else {
                            self.write(&format!("JsAny.from(_lv_{id})"));
                        }
                        self.write(&format!(" else break :{blk} "));
                        if same_type {
                            self.emit_expr(right);
                        } else {
                            self.write("JsAny.from(");
                            self.emit_expr(right);
                            self.write(")");
                        }
                    }
                }
                self.write("; })");
            }

            crate::zigir::types::IrExpr::Update {
                op,
                target,
                is_expr_stmt,
                prefix,
            } => {
                if *is_expr_stmt {
                    // Statement context: `i += 1` (no parens, no _ = prefix)
                    self.emit_assign_target_inner(target);
                    self.write(&format!(" {}", update_op_to_zig(*op)));
                } else if *prefix {
                    // Prefix `++x` in expression context: returns NEW value.
                    // (_blk: { x += 1; break :_blk x; })
                    let blk = self.next_label();
                    self.write(&format!("({}: {{ ", blk));
                    self.emit_assign_target_inner(target);
                    self.write(&format!(" {}; ", update_op_to_zig(*op)));
                    self.write("break :");
                    self.write(&blk);
                    self.write(" ");
                    self.emit_assign_target_inner(target);
                    self.write("; })");
                } else {
                    // Postfix `x++` in expression context: returns OLD value.
                    // (_blk: { const _old = x; x += 1; break :_blk _old; })
                    let blk = self.next_label();
                    self.write(&format!("({}: {{ const _old = ", blk));
                    self.emit_assign_target_inner(target);
                    self.write("; ");
                    self.emit_assign_target_inner(target);
                    self.write(&format!(" {}; ", update_op_to_zig(*op)));
                    self.write(&format!("break :{} _old; }})", blk));
                }
            }

            crate::zigir::types::IrExpr::Assign { op, target, value } => {
                self.emit_compound_assign(target, *op, value);
            }

            // ── Calls ───────────────────────────────
            crate::zigir::types::IrExpr::Call(call) => {
                self.emit_call_expr(call);
            }

            crate::zigir::types::IrExpr::BuiltinCall(bc) => {
                self.emit_builtin_call(bc);
            }

            crate::zigir::types::IrExpr::HostCall(hc) => {
                self.write(&format!("host.{}", hc.name));
                self.emit_args(&hc.args);
            }

            // ── Member access ───────────────────────
            crate::zigir::types::IrExpr::FieldAccess {
                object,
                field,
                field_kind,
            } => {
                self.emit_field_access(object, field, field_kind);
            }

            crate::zigir::types::IrExpr::IndexAccess {
                object,
                index,
                index_kind,
            } => {
                self.emit_index_access(object, index, index_kind);
            }

            crate::zigir::types::IrExpr::ComputedField {
                object,
                key,
                key_kind,
            } => {
                self.emit_computed_field(object, key, key_kind);
            }

            // ── Object / Array ──────────────────────
            crate::zigir::types::IrExpr::ArrayLiteral(arr) => {
                self.emit_array_literal(arr);
            }

            crate::zigir::types::IrExpr::ObjectLiteral(obj) => {
                self.emit_object_literal(obj);
            }

            // ── Function expressions ────────────────
            // NOTE: Dead code path — Lowerer always converts ArrowFn to IrExpr::Ident
            // + pending_arrow_structs before reaching the emitter.
            crate::zigir::types::IrExpr::ArrowFn(_arrow) => {
                // Should never be reached; emit a placeholder to avoid panic.
                self.write("/* ArrowFn — should be lowered to closure struct */");
            }

            crate::zigir::types::IrExpr::Closure(closure) => {
                // Closure instance: `StructName { .captured = val, ... }`
                self.write(&closure.struct_name.zig_name);
                self.write("{ ");
                for (i, cap) in closure.captured.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    if cap.is_mut {
                        self.write(&format!(".{} = &{}", cap.name.zig_name, cap.name.zig_name));
                    } else {
                        self.write(&format!(".{} = {}", cap.name.zig_name, cap.name.zig_name));
                    }
                }
                self.write(" }");
            }

            crate::zigir::types::IrExpr::FnExpr(fn_expr) => {
                // Function expression: emit as struct with call method
                let name = fn_expr
                    .name
                    .as_ref()
                    .map(|n| n.zig_name.clone())
                    .unwrap_or_else(|| "_fn_expr".to_string());
                self.write(&name);
            }

            // ── Conditional / template ───────────────
            crate::zigir::types::IrExpr::Conditional { cond, then, else_ } => {
                self.write("if (");
                self.emit_expr_as_bool(cond);
                self.write(") ");
                self.emit_expr(then);
                self.write(" else ");
                self.emit_expr(else_);
            }

            crate::zigir::types::IrExpr::TemplateLiteral {
                parts,
                exprs,
                format_specs,
            } => {
                self.emit_template_literal(parts, exprs, format_specs);
            }

            // ── Async ───────────────────────────────
            crate::zigir::types::IrExpr::Await(await_expr) => {
                // Emit: (blk: { var _tN = io.async(callee, .{ io, args... }); defer _ = _tN.cancel(io) catch undefined; break :blk try _tN.await(io); })
                self.write(&format!("({}: {{\n", await_expr.block_label));
                self.indent_push();
                self.write_indent();
                self.write(&format!("var {} = io.async(", await_expr.task_var.zig_name));

                if await_expr.is_host_async {
                    // For host async, callee is an Ident holding the host fn name
                    if let crate::zigir::types::IrExpr::Ident(id) = &*await_expr.callee {
                        self.write(&format!("host.{}_async", id.zig_name));
                    } else {
                        // Fallback
                        self.emit_expr(&await_expr.callee);
                    }
                } else {
                    self.emit_expr(&await_expr.callee);
                }

                self.write(", .{ io");
                for arg in &await_expr.args {
                    self.write(", ");
                    self.emit_expr(arg);
                }
                self.write(" });\n");

                self.write_indent();
                self.write(&format!(
                    "defer _ = {}.cancel(io) catch undefined;\n",
                    await_expr.task_var.zig_name
                ));

                self.write_indent();
                self.write(&format!(
                    "break :{} try {}.await(io);\n",
                    await_expr.block_label, await_expr.task_var.zig_name
                ));

                self.indent_pop();
                self.write_indent();
                self.write("})");
            }

            // ── Construction ────────────────────────
            crate::zigir::types::IrExpr::New(new_expr) => {
                self.emit_new_expr(new_expr);
            }

            // ── String formatting ──────────────────────
            crate::zigir::types::IrExpr::AllocPrint { fmt, args } => {
                if args.is_empty() {
                    // Pure-text → plain string literal (no allocation)
                    let unescaped = fmt.replace("{{", "{").replace("}}", "}");
                    self.write(&format!("\"{}\"", unescaped));
                } else {
                    // Emit args by capturing each as a string
                    self.emit_alloc_print(fmt, args);
                }
            }

            // ── Block expression ────────────────────
            crate::zigir::types::IrExpr::BlockExpr {
                label,
                body,
                result,
            } => {
                self.write(&format!("({}: {{", label));
                for stmt in body {
                    self.emit_stmt(stmt);
                }
                self.write("break :");
                self.write(label);
                self.write(" ");
                self.emit_expr(result);
                // Close: `; })` — `;` ends the break STMT (Zig requires `;`
                // after `break :label value`), `}` closes the labeled block,
                // `)` closes the wrapping paren.
                // Pre-fix typo wrote `}})` (two `}`, no `;`) which produced
                // invalid Zig: `expected ';' after statement` on the `}`.
                self.write("; })");
            }

            // ── Special ─────────────────────────────
            crate::zigir::types::IrExpr::Spread(inner) => {
                self.emit_expr(inner);
            }

            crate::zigir::types::IrExpr::Typeof(inner) => {
                // typeof on identifiers with known types is resolved at lower time
                // to StringLiteral. This branch handles any remaining typeof
                // expressions (shouldn't occur with current lowerer, but as fallback).
                self.write("js_runtime.jsTypeof(");
                self.emit_expr(inner);
                self.write(")");
            }

            crate::zigir::types::IrExpr::Void(inner) => {
                // void expr: evaluate expr for side effects, return undefined.
                // Output: blk_N: { _ = expr; break :blk_N JsAny.fromUndefined(); }
                let label = self.next_label();
                self.write(&format!("{}: {{ _ = ", label));
                self.emit_expr(inner);
                self.write(&format!("; break :{} JsAny.fromUndefined(); }}", label));
            }

            crate::zigir::types::IrExpr::Paren(inner) => {
                self.write("(");
                self.emit_expr(inner);
                self.write(")");
            }

            crate::zigir::types::IrExpr::Sequence(exprs) => {
                // JS comma operator: evaluate all expressions, return the last.
                // Zig has no comma operator, so use a labeled block.
                // `a, b, c` → `blk: { _ = a; _ = b; break :blk c; }`
                if exprs.len() == 1 {
                    self.emit_expr(&exprs[0]);
                } else {
                    let blk = self.next_label();
                    self.write(&format!("({blk}: {{ "));
                    for e in &exprs[..exprs.len() - 1] {
                        self.write("_ = ");
                        self.emit_expr(e);
                        self.write("; ");
                    }
                    self.write(&format!("break :{blk} "));
                    self.emit_expr(&exprs[exprs.len() - 1]);
                    self.write("; })");
                }
            }

            crate::zigir::types::IrExpr::ArrayCallbackInline(inline_data) => {
                self.emit_array_callback_inline(inline_data);
            }

            crate::zigir::types::IrExpr::ArrayMethodInline(inline_data) => {
                self.emit_array_method_inline(inline_data);
            }

            crate::zigir::types::IrExpr::OptionalChain {
                object,
                capture_var,
                body,
                needs_null_check,
            } => {
                if *needs_null_check {
                    self.write("(if (");
                    self.emit_expr(object);
                    self.write(") |");
                    self.write(capture_var);
                    self.write("| ");
                    self.emit_expr(body);
                    self.write(" else null)");
                } else {
                    self.emit_expr(body);
                }
            }

            crate::zigir::types::IrExpr::CompileError { span, msg } => {
                let loc = format!("{}:{}", span.js_line, span.js_col);
                self.write(&emit_helpers::compile_error(&format!(
                    "{} (at {})",
                    msg, loc
                )));
            }
        }
    }

    /// Convert an expression to a string without writing to the output buffer.
    /// Used for contexts where we need the expression text as a value.
    pub(crate) fn expr_to_string(&mut self, expr: &crate::zigir::types::IrExpr) -> String {
        let saved_output = std::mem::take(&mut self.output);
        self.emit_expr(expr);
        let result = std::mem::take(&mut self.output);
        self.output = saved_output;
        result
    }

    // ── Internal helpers ───────────────────────────

    pub(super) fn emit_assign_target_inner(
        &mut self,
        target: &crate::zigir::types::IrAssignTarget,
    ) {
        match target {
            crate::zigir::types::IrAssignTarget::Ident(ident) => {
                self.write(&ident.zig_name);
            }
            crate::zigir::types::IrAssignTarget::Member {
                object,
                field,
                is_pointer,
                field_kind,
            } => {
                use crate::zigir::kinds::FieldKind;
                match field_kind {
                    FieldKind::StaticField { class_name } => {
                        self.emit_static_field(class_name, field);
                    }
                    _ => {
                        self.emit_expr(object);
                        if *is_pointer {
                            self.write(&format!(".{}.*", field));
                        } else {
                            self.write(&format!(".{}", field));
                        }
                    }
                }
            }
            crate::zigir::types::IrAssignTarget::Index {
                object,
                index,
                index_kind,
            } => {
                use crate::zigir::kinds::IndexKind;
                match index_kind {
                    IndexKind::ArrayListItem => {
                        self.emit_arraylist_item(object, index);
                    }
                    IndexKind::SliceIndex => {
                        self.emit_slice_index(object, index);
                    }
                }
            }
            crate::zigir::types::IrAssignTarget::Destructure(bindings) => {
                for (i, binding) in bindings.iter().enumerate() {
                    if i > 0 {
                        self.write("; ");
                    }
                    self.write(&binding.pattern.zig_name);
                    if let Some(default) = &binding.default {
                        self.write(" orelse ");
                        self.emit_expr(default);
                    }
                }
            }
            crate::zigir::types::IrAssignTarget::CompileError { msg } => {
                self.write(&format!("@compileError(\"{}\")", msg));
            }
        }
    }

    /// Emit a compound assignment (LogicAnd/LogicOr/Nullish or simple op).
    /// Shared by IrExpr::Assign (expression context) and emit_assign_inline (statement context).
    /// Note: Mod (%) is expanded to RemExpr and Div (/) to DivExpr in the lowerer.
    pub(super) fn emit_compound_assign(
        &mut self,
        target: &crate::zigir::types::IrAssignTarget,
        op: crate::zigir::ops::AssignOp,
        value: &crate::zigir::types::IrExpr,
    ) {
        use crate::zigir::ops::AssignOp;
        if op == AssignOp::LogicAnd {
            // a &&= b → a = if (js_runtime.isTruthy(a)) b else a
            // Use isTruthy (works for anytype: i64, f64, bool, JsAny, string, ...)
            // instead of .toBool() which only exists on JsAny.
            self.emit_assign_target_inner(target);
            self.write(" = if (js_runtime.isTruthy(");
            self.emit_assign_target_inner(target);
            self.write(")) ");
            self.emit_expr(value);
            self.write(" else ");
            self.emit_assign_target_inner(target);
        } else if op == AssignOp::LogicOr {
            // a ||= b → a = if (!js_runtime.isTruthy(a)) b else a
            self.emit_assign_target_inner(target);
            self.write(" = if (!js_runtime.isTruthy(");
            self.emit_assign_target_inner(target);
            self.write(")) ");
            self.emit_expr(value);
            self.write(" else ");
            self.emit_assign_target_inner(target);
        } else if op == AssignOp::Nullish {
            // a ??= b → a = if (a.isNullish()) b else a
            self.emit_assign_target_inner(target);
            self.write(" = if (");
            self.emit_assign_target_inner(target);
            self.write(".isNullish()) ");
            self.emit_expr(value);
            self.write(" else ");
            self.emit_assign_target_inner(target);
        } else {
            self.emit_assign_target_inner(target);
            self.write(&format!(" {} ", op.to_zig_str()));
            self.emit_expr(value);
        }
    }

    /// Emit a `std.fmt.allocPrint(allocator, fmt, .{args})` call.
    /// Shared by IrExpr::AllocPrint and emit_template_literal.
    pub(super) fn emit_alloc_print(&mut self, fmt: &str, args: &[crate::zigir::types::IrExpr]) {
        let arg_strs: Vec<String> = args.iter().map(|arg| self.expr_to_string(arg)).collect();
        let args_str = format!(".{{{}}}", arg_strs.join(", "));
        self.write(&format!(
            "std.fmt.allocPrint(js_allocator.allocator(), \"{}\", {}) catch @panic(\"OOM: template literal allocPrint\")",
            fmt, args_str
        ));
    }

    pub(super) fn emit_args(&mut self, args: &[crate::zigir::types::IrExpr]) {
        self.write("(");
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            // foo(...args) → foo(args.items)  [ArrayList spread]
            // foo(...restParam) → foo(restParam)  [rest param: already []const JsAny]
            // foo(...[1,2,3]) → foo(&[_]JsAny{ JsAny.from(1), ... })  [literal spread]
            if let crate::zigir::types::IrExpr::Spread(inner) = arg {
                match inner.as_ref() {
                    // Rest param spread: pass slice directly (no .items)
                    crate::zigir::types::IrExpr::Ident(ident)
                        if self.rest_param_names.contains(&ident.zig_name) =>
                    {
                        self.emit_expr(inner);
                    }
                    // Array literal spread: emit as &[_]JsAny{ ... }
                    // Each element must be wrapped in JsAny.from() since the
                    // array literal type is explicitly JsAny.
                    crate::zigir::types::IrExpr::ArrayLiteral(arr) => {
                        self.write("&[_]JsAny{ ");
                        for (j, elem) in arr.elements.iter().enumerate() {
                            if j > 0 {
                                self.write(", ");
                            }
                            if arr.spread_indices.contains(&j) {
                                // Nested spread inside array literal spread:
                                // not supported in &[_]JsAny{} syntax.
                                self.write(
                                    "@compileError(\"nested spread in call args not supported\")",
                                );
                            } else {
                                self.write("JsAny.from(");
                                self.emit_expr(elem);
                                self.write(")");
                            }
                        }
                        self.write(" }");
                    }
                    // Default: ArrayList spread → .items
                    _ => {
                        self.emit_expr(inner);
                        self.write(".items");
                    }
                }
            } else {
                self.emit_expr(arg);
            }
        }
        self.write(")");
    }

    /// Check if an IrExpr is definitely of type `bool`.
    pub(super) fn ir_expr_is_bool(expr: &crate::zigir::types::IrExpr) -> bool {
        use crate::zigir::ops::BinOp;
        use crate::zigir::types::IrExpr;

        match expr {
            IrExpr::BoolLiteral(_) => true,
            // Logical expressions are value-returning in JS, not bool.
            // They use if-expressions at emit time, producing the operand type or JsAny.
            IrExpr::Logical { .. } => false,
            IrExpr::Unary {
                op: crate::zigir::ops::UnaOp::Not,
                ..
            } => true,
            IrExpr::Binary { op, .. } => matches!(
                op,
                BinOp::Eq
                    | BinOp::Ne
                    | BinOp::StrictEq
                    | BinOp::StrictNe
                    | BinOp::Lt
                    | BinOp::Le
                    | BinOp::Gt
                    | BinOp::Ge
                    | BinOp::In
                    | BinOp::InstanceOf
            ),
            _ => false,
        }
    }

    /// Emit an expression with truthiness coercion for Zig `bool` context.
    ///
    /// When a non-bool expression appears in a position that Zig requires `bool`
    /// (e.g. `if` condition, `while` condition), we coerce via `js_runtime.isTruthy()`
    /// which handles all JS types correctly (bool, i64, f64, string, comptime_int, etc.).
    pub(crate) fn emit_expr_as_bool(&mut self, expr: &crate::zigir::types::IrExpr) {
        if Self::ir_expr_is_bool(expr) {
            self.emit_expr(expr);
        } else {
            self.write("js_runtime.isTruthy(");
            self.emit_expr(expr);
            self.write(")");
        }
    }
}

/// Map TypedArrayKind to (module, init_fn) for construction.
/// Uses fromI64AsI8/fromI64AsU8/.../fromF64 series instead of direct .init().
pub(super) fn typed_array_init(
    kind: &crate::zigir::kinds::TypedArrayKind,
) -> (&'static str, &'static str) {
    use crate::zigir::kinds::TypedArrayKind;
    match kind {
        TypedArrayKind::Int8Array => ("js_runtime.js_typedarray", "fromI64AsI8"),
        TypedArrayKind::Uint8Array => ("js_runtime.js_typedarray", "fromI64AsU8"),
        TypedArrayKind::Uint8ClampedArray => ("js_runtime.js_typedarray", "fromI64AsU8"),
        TypedArrayKind::Int16Array => ("js_runtime.js_typedarray", "fromI64AsI16"),
        TypedArrayKind::Uint16Array => ("js_runtime.js_typedarray", "fromI64AsU16"),
        TypedArrayKind::Int32Array => ("js_runtime.js_typedarray", "fromI64AsI32"),
        TypedArrayKind::Uint32Array => ("js_runtime.js_typedarray", "fromI64AsU32"),
        TypedArrayKind::Float32Array => ("js_runtime.js_typedarray", "fromF64AsF32"),
        TypedArrayKind::Float64Array => ("js_runtime.js_typedarray", "fromF64"),
        TypedArrayKind::BigInt64Array => ("js_runtime.js_typedarray", "fromI64AsI64"),
        TypedArrayKind::BigUint64Array => ("js_runtime.js_typedarray", "fromI64AsU64"),
    }
}
