// zigir/lower/expr/operators.rs
// Binary, unary, update, assignment expression lowering + string concatenation.

use std::collections::HashSet;

use oxc_ast::ast::*;

use crate::types::ZigType;
use crate::zigir::builtins::BuiltinModule;
use crate::zigir::ident::IrIdent;
use crate::zigir::kinds::{FieldKind, IndexKind};
use crate::zigir::ops::{AssignOp, BinOp, LogicalOp, UnaOp, UpdateOp};

use super::Lowerer;
use crate::zigir::lower::helpers;

impl Lowerer {
    /// Lower a binary expression.
    pub(super) fn lower_binary(&mut self, be: &BinaryExpression) -> crate::zigir::types::IrExpr {
        use crate::zigir::types::IrExpr;

        // String concatenation detection: if `+` has any string operand,
        // flatten the chain and produce IrExpr::AllocPrint instead of
        // a binary tree of IrExpr::Binary nodes.
        if be.operator == BinaryOperator::Addition {
            let left_is_str = self.expr_is_string(&be.left);
            let right_is_str = self.expr_is_string(&be.right);
            if left_is_str || right_is_str {
                return self.lower_string_concat(be);
            }
        }

        // ── Unsupported operators → compile error (or special handling) ──
        match be.operator {
            BinaryOperator::Instanceof => {
                return self.lower_instanceof(be);
            }
            BinaryOperator::In => {
                // `key in obj` → obj.has(key) for Map/Set, obj.contains(key) otherwise
                let right_expr = self.lower_expr(&be.right);
                let left_expr = self.lower_expr(&be.left);
                let right_type = self.infer_expr_type(&be.right);
                return IrExpr::Binary {
                    op: BinOp::In,
                    left: Box::new(left_expr),
                    right: Box::new(right_expr),
                    left_type: Some(ZigType::Str),
                    right_type,
                };
            }
            _ => {}
        }

        let op = match be.operator {
            BinaryOperator::Addition => BinOp::Add,
            BinaryOperator::Subtraction => BinOp::Sub,
            BinaryOperator::Multiplication => BinOp::Mul,
            BinaryOperator::Division => {
                let left_type = self.infer_expr_type(&be.left).unwrap_or(ZigType::I64);
                let right_type = self.infer_expr_type(&be.right).unwrap_or(ZigType::I64);
                // BigInt / BigInt or mixed BigInt: use BinOp::Div so emit_bigint_binary
                // (for pure BigInt) or error.JsThrow (for mixed) is generated at emit time.
                if left_type == ZigType::BigInt || right_type == ZigType::BigInt {
                    if let Some(ctx) = self.fn_ctx.as_mut() {
                        ctx.has_bigint_div = true;
                        // Mixed BigInt division throws TypeError at runtime
                        if left_type != right_type {
                            ctx.has_catchable_error = true;
                        }
                    }
                    return IrExpr::Binary {
                        op: BinOp::Div,
                        left: Box::new(self.lower_expr(&be.left)),
                        right: Box::new(self.lower_expr(&be.right)),
                        left_type: Some(left_type),
                        right_type: Some(right_type),
                    };
                }
                // Non-BigInt: JS `/` always returns f64. Emit a DivExpr with type info
                // so the Emitter can generate the correct f64 coercion and i64 wrapping
                // (when result_type is later set to Some(I64) by coerce_i64_result_type).
                return IrExpr::DivExpr {
                    left: Box::new(self.lower_expr(&be.left)),
                    right: Box::new(self.lower_expr(&be.right)),
                    left_type,
                    right_type,
                    result_type: None, // standalone `/` keeps f64 result
                };
            }
            BinaryOperator::Remainder => {
                let left_type = self.infer_expr_type(&be.left).unwrap_or(ZigType::I64);
                let right_type = self.infer_expr_type(&be.right).unwrap_or(ZigType::I64);
                // BigInt % BigInt: use BinOp::Mod so emit_bigint_binary generates .rem() call.
                if left_type == ZigType::BigInt || right_type == ZigType::BigInt {
                    // BigInt modulo can throw RangeError — mark function as can_throw
                    if let Some(ctx) = self.fn_ctx.as_mut() {
                        ctx.has_bigint_div = true;
                    }
                    return IrExpr::Binary {
                        op: BinOp::Mod,
                        left: Box::new(self.lower_expr(&be.left)),
                        right: Box::new(self.lower_expr(&be.right)),
                        left_type: Some(left_type),
                        right_type: Some(right_type),
                    };
                }
                // Float % any: use BinOp::Mod so emit layer generates @rem(f64, f64).
                if left_type == ZigType::F64 || right_type == ZigType::F64 {
                    return IrExpr::Binary {
                        op: BinOp::Mod,
                        left: Box::new(self.lower_expr(&be.left)),
                        right: Box::new(self.lower_expr(&be.right)),
                        left_type: Some(left_type),
                        right_type: Some(right_type),
                    };
                }
                // Integer %: use RemExpr (jsRem returns f64, preserves signed zero).
                // result_type is set later by lower_assignment when assigning to i64.
                return IrExpr::RemExpr {
                    left: Box::new(self.lower_expr(&be.left)),
                    right: Box::new(self.lower_expr(&be.right)),
                    left_type,
                    right_type,
                    result_type: None, // standalone `%` keeps f64 result
                };
            }
            BinaryOperator::Exponential => {
                let left_type = self.infer_expr_type(&be.left).unwrap_or(ZigType::F64);
                let right_type = self.infer_expr_type(&be.right).unwrap_or(ZigType::F64);
                // BigInt ** BigInt: use BinOp::Pow so emit_bigint_binary generates .pow() call.
                if left_type == ZigType::BigInt || right_type == ZigType::BigInt {
                    // BigInt ** can throw at runtime:
                    // - Mixed BigInt (e.g., 2n ** 2): TypeError
                    // - BigInt ** BigInt with negative exponent: RangeError (toU64 fails)
                    if let Some(ctx) = self.fn_ctx.as_mut() {
                        ctx.has_catchable_error = true;
                    }
                    return IrExpr::Binary {
                        op: BinOp::Pow,
                        left: Box::new(self.lower_expr(&be.left)),
                        right: Box::new(self.lower_expr(&be.right)),
                        left_type: Some(left_type),
                        right_type: Some(right_type),
                    };
                }
                // Non-BigInt: JS `**` always returns f64. Emit a PowExpr with type info
                // so the Emitter can generate the correct f64 coercion.
                return IrExpr::PowExpr {
                    base: Box::new(self.lower_expr(&be.left)),
                    exp: Box::new(self.lower_expr(&be.right)),
                    base_type: left_type,
                    exp_type: right_type,
                    result_type: None, // standalone `**` always returns f64
                };
            }
            BinaryOperator::LessThan => BinOp::Lt,
            BinaryOperator::GreaterThan => BinOp::Gt,
            BinaryOperator::LessEqualThan => BinOp::Le,
            BinaryOperator::GreaterEqualThan => BinOp::Ge,
            BinaryOperator::Equality => BinOp::Eq,
            BinaryOperator::Inequality => BinOp::Ne,
            BinaryOperator::StrictEquality => BinOp::StrictEq,
            BinaryOperator::StrictInequality => BinOp::StrictNe,
            BinaryOperator::BitwiseAnd => BinOp::BitAnd,
            BinaryOperator::BitwiseOR => BinOp::BitOr,
            BinaryOperator::BitwiseXOR => BinOp::BitXor,
            BinaryOperator::ShiftLeft => BinOp::Shl,
            BinaryOperator::ShiftRight => BinOp::Shr,
            BinaryOperator::ShiftRightZeroFill => BinOp::UrShr,
            // Instanceof/In are handled above (CompileError). Kept for
            // exhaustiveness — use safe fallback instead of unreachable! (P0-9 fix).
            BinaryOperator::Instanceof | BinaryOperator::In => BinOp::Add,
        };

        let left_type = self.infer_expr_type(&be.left);
        let right_type = self.infer_expr_type(&be.right);

        // BigInt division/modulo can throw RangeError — mark function as can_throw
        if matches!(op, BinOp::Div | BinOp::Mod)
            && (left_type.as_ref() == Some(&ZigType::BigInt)
                || right_type.as_ref() == Some(&ZigType::BigInt))
            && let Some(ctx) = self.fn_ctx.as_mut()
        {
            ctx.has_bigint_div = true;
        }

        // BigInt mixed-type ops, BigInt >>>, BigInt ** BigInt, and BigInt << >> BigInt
        // throw or can throw at runtime. These are emitted as `return error.JsThrow`
        // (not @panic) so JS try/catch can catch them.
        // Mark the function as can_throw so the signature includes `!`.
        {
            let left_is_bigint = left_type.as_ref() == Some(&ZigType::BigInt);
            let right_is_bigint = right_type.as_ref() == Some(&ZigType::BigInt);
            let is_mixed_bigint = left_is_bigint != right_is_bigint; // XOR: exactly one is BigInt
            let is_bigint_urshr = op == BinOp::UrShr && (left_is_bigint || right_is_bigint);
            // BigInt << >> BigInt: toI64() can fail for very large shift amounts → error.JsThrow
            let is_bigint_shift =
                matches!(op, BinOp::Shl | BinOp::Shr) && left_is_bigint && right_is_bigint;
            if ((is_mixed_bigint
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
                ))
                || is_bigint_urshr
                || is_bigint_shift)
                && let Some(ctx) = self.fn_ctx.as_mut()
            {
                ctx.has_catchable_error = true;
            }
        }

        IrExpr::Binary {
            op,
            left: Box::new(self.lower_expr(&be.left)),
            right: Box::new(self.lower_expr(&be.right)),
            left_type,
            right_type,
        }
    }

    /// Lower a unary expression.
    pub(super) fn lower_unary(&mut self, ue: &UnaryExpression) -> crate::zigir::types::IrExpr {
        use crate::zigir::types::IrExpr;

        match ue.operator {
            UnaryOperator::UnaryNegation => {
                // -0.0 must be lowered directly as FloatLiteral(-0.0) to
                // preserve IEEE 754 signed zero. Otherwise the operand 0.0 is
                // lowered as IntLiteral(0) (0.0 fits in i64), and constant
                // folding collapses Neg(IntLiteral(0)) -> IntLiteral(0),
                // losing the sign bit.
                if let Expression::NumericLiteral(nl) = &ue.argument
                    && nl.value == 0.0
                {
                    return IrExpr::FloatLiteral(-0.0);
                }
                if self.infer_expr_type(&ue.argument) == Some(ZigType::BigInt) {
                    let operand = self.lower_expr(&ue.argument);
                    return self.bigint_unary_builtin("bigIntNeg", operand);
                }
                IrExpr::Unary {
                    op: UnaOp::Neg,
                    operand: Box::new(self.lower_expr(&ue.argument)),
                    operand_type: self.infer_expr_type(&ue.argument),
                }
            }
            UnaryOperator::UnaryPlus => {
                // JS unary `+` performs ToNumber conversion:
                //   +true → 1, +false → 0, +null → 0, +undefined → NaN,
                //   +"5" → 5, +5 → 5 (no-op for numbers).
                // Previously this was a no-op for all types, leaving bool and
                // string operands unconverted (R8-E1).
                let arg = &ue.argument;
                let ty = self.infer_expr_type(arg);
                match ty {
                    Some(ZigType::Bool) => {
                        // Constant-fold boolean literals.
                        if let Expression::BooleanLiteral(bl) = arg {
                            return IrExpr::IntLiteral(if bl.value { 1 } else { 0 });
                        }
                        // For a bool variable, emit `(if (expr) 1 else 0)`
                        // to coerce to i64 without needing a new IR node.
                        IrExpr::Conditional {
                            cond: Box::new(self.lower_expr(arg)),
                            then: Box::new(IrExpr::IntLiteral(1)),
                            else_: Box::new(IrExpr::IntLiteral(0)),
                        }
                    }
                    Some(ZigType::Str) => {
                        // String → f64 via js_number.constructor (matches
                        // Number(str) semantics: parseFloat with NaN fallback).
                        IrExpr::BuiltinCall(crate::zigir::types::IrBuiltinCall::simple(
                            BuiltinModule::JsNumber,
                            "constructor",
                            None,
                            None,
                            vec![self.lower_expr(arg)],
                            ZigType::F64,
                        ))
                    }
                    // BigInt: JS spec says +BigInt throws TypeError.
                    Some(ZigType::BigInt) => {
                        let span = self.span_to_source_span(ue.span);
                        self.add_error(
                            span.clone(),
                            "unary + on BigInt is a TypeError in JavaScript",
                        );
                        IrExpr::CompileError {
                            span,
                            msg: "unary + on BigInt is not supported (TypeError in JS)".to_string(),
                        }
                    }
                    // Already numeric (i64/f64): no-op.
                    _ => self.lower_expr(arg),
                }
            }
            UnaryOperator::LogicalNot => IrExpr::Unary {
                op: UnaOp::Not,
                operand: Box::new(self.lower_expr(&ue.argument)),
                operand_type: self.infer_expr_type(&ue.argument),
            },
            UnaryOperator::BitwiseNot => {
                if self.infer_expr_type(&ue.argument) == Some(ZigType::BigInt) {
                    let operand = self.lower_expr(&ue.argument);
                    return self.bigint_unary_builtin("bigIntBitwiseNot", operand);
                }
                IrExpr::Unary {
                    op: UnaOp::BitNot,
                    operand: Box::new(self.lower_expr(&ue.argument)),
                    operand_type: self.infer_expr_type(&ue.argument),
                }
            }
            UnaryOperator::Typeof => {
                if let Some(ty) = self.infer_expr_type(&ue.argument)
                    && let Some(js_typeof) = ty.to_js_typeof()
                {
                    let mut idents = HashSet::new();
                    Self::collect_ast_expr_idents(&ue.argument, &mut idents);
                    if let Some(ctx) = self.fn_ctx.as_mut() {
                        ctx.compile_time_referenced_idents.extend(idents);
                    }
                    return IrExpr::StringLiteral(js_typeof.to_string());
                }
                IrExpr::BuiltinCall(crate::zigir::types::IrBuiltinCall::simple(
                    BuiltinModule::JsRuntime,
                    "jsTypeof",
                    None,
                    None,
                    vec![self.lower_expr(&ue.argument)],
                    ZigType::Str,
                ))
            }
            UnaryOperator::Void => IrExpr::Void(Box::new(self.lower_expr(&ue.argument))),
            UnaryOperator::Delete => {
                // delete obj.prop → IrBuiltinCall { JsRuntime, "deleteKey", obj, [prop] }
                // delete obj[expr] → IrBuiltinCall { JsRuntime, "deleteByKey", obj, [expr] }
                //
                // Bug fix (Agent1 P1-6): When the receiver is a non-Identifier
                // expression (e.g. `delete getObj().prop`, `delete arr[i].prop`,
                // `delete obj.a.b`), the previous code set both `obj_name` and
                // `obj_expr` to `None`, silently dropping the receiver —
                // including its side effects. The emit layer would then produce
                // invalid Zig like `_ = .deleteKey("prop")`. Now we lower the
                // receiver expression and pass it via `obj_expr` so the Emitter
                // renders it inline (`Self::emit_expr_inline`).
                match &ue.argument {
                    Expression::StaticMemberExpression(mem) => {
                        let (obj_name, obj_expr) = match &mem.object {
                            Expression::Identifier(id) => {
                                (Some(id.name.as_str().to_string()), None)
                            }
                            other => {
                                // Lower the receiver so its side effects happen
                                // exactly once at the delete call site.
                                (None, Some(Box::new(self.lower_expr(other))))
                            }
                        };
                        IrExpr::BuiltinCall(crate::zigir::types::IrBuiltinCall::simple(
                            BuiltinModule::JsRuntime,
                            "deleteKey",
                            obj_name,
                            obj_expr,
                            vec![IrExpr::StringLiteral(
                                mem.property.name.as_str().to_string(),
                            )],
                            ZigType::Bool,
                        ))
                    }
                    Expression::ComputedMemberExpression(mem) => {
                        let (obj_name, obj_expr) = match &mem.object {
                            Expression::Identifier(id) => {
                                (Some(id.name.as_str().to_string()), None)
                            }
                            other => (None, Some(Box::new(self.lower_expr(other)))),
                        };
                        IrExpr::BuiltinCall(crate::zigir::types::IrBuiltinCall::simple(
                            BuiltinModule::JsRuntime,
                            "deleteByKey",
                            obj_name,
                            obj_expr,
                            vec![self.lower_expr(&mem.expression)],
                            ZigType::Bool,
                        ))
                    }
                    _ => {
                        self.compile_error_expr(ue.span, "delete operator requires property access")
                    }
                }
            }
        }
    }

    /// Lower an update expression (++/--).
    pub(super) fn lower_update(&mut self, ue: &UpdateExpression) -> crate::zigir::types::IrExpr {
        use crate::zigir::types::IrExpr;

        // Check if the target is BigInt — BigInt has no Zig +=, -=,
        // so we must expand `x++` into `x = x + BigInt(1)` and
        // `x--` into `x = x - BigInt(1)` using method calls.
        let target_type = self.infer_simple_assign_target_type(&ue.argument);
        if target_type == Some(ZigType::BigInt) {
            let target = self.lower_simple_assign_target(&ue.argument);
            let Some(read_expr) = target.to_read_expr() else {
                return IrExpr::Update {
                    op: if ue.operator == UpdateOperator::Increment {
                        UpdateOp::Increment
                    } else {
                        UpdateOp::Decrement
                    },
                    target: Box::new(target),
                    is_expr_stmt: self.in_expr_stmt,
                    prefix: ue.prefix,
                };
            };
            let bin_op = if ue.operator == UpdateOperator::Increment {
                BinOp::Add
            } else {
                BinOp::Sub
            };

            // For prefix `++x`: an `Assign` expression evaluates to the
            // assigned (new) value, which matches JS semantics for prefix
            // (the expression returns the value AFTER incrementing).
            if ue.prefix {
                return IrExpr::Assign {
                    op: AssignOp::Assign,
                    target: Box::new(target),
                    value: Box::new(IrExpr::Binary {
                        op: bin_op,
                        left: Box::new(read_expr),
                        right: Box::new(IrExpr::BigIntLiteral("1".to_string())),
                        left_type: Some(ZigType::BigInt),
                        right_type: Some(ZigType::BigInt),
                    }),
                };
            }

            // For postfix `x++`: JS spec returns the OLD value of `x`.
            // An `Assign` expression alone would return the NEW value, so
            // we capture the pre-increment value in a temp variable, then
            // wrap the assign + temp in a BlockExpr so the expression
            // result is the temp (old value):
            //   (blk: {
            //     const __bi_post_N = <read_expr>;
            //     <target> = __bi_post_N + BigInt(1);
            //     break :blk __bi_post_N;
            //   })
            use crate::zigir::types::{IrStmt, IrVarDecl};
            let temp_name = self.name_mangler.next_name("__bi_post");
            let blk_label = self.name_mangler.next_name("_bi_post_blk");
            let temp_ident = IrExpr::Ident(IrIdent::new(&temp_name));
            let var_decl = IrStmt::VarDecl(IrVarDecl {
                name: IrIdent::new(&temp_name),
                is_const: true,
                zig_type: None,
                init: Some(read_expr),
                is_json_parse: false,
                needs_var_suppression: false,
                needs_deinit: false,
            });
            let assign_stmt = IrStmt::Expr(IrExpr::Assign {
                op: AssignOp::Assign,
                target: Box::new(target),
                value: Box::new(IrExpr::Binary {
                    op: bin_op,
                    left: Box::new(temp_ident.clone()),
                    right: Box::new(IrExpr::BigIntLiteral("1".to_string())),
                    left_type: Some(ZigType::BigInt),
                    right_type: Some(ZigType::BigInt),
                }),
            });
            return IrExpr::BlockExpr {
                label: blk_label,
                body: vec![var_decl, assign_stmt],
                result: Box::new(temp_ident),
            };
        }

        // Non-BigInt: emit standard ++/--
        let op = if ue.operator == UpdateOperator::Increment {
            UpdateOp::Increment
        } else {
            UpdateOp::Decrement
        };
        let target = Box::new(self.lower_simple_assign_target(&ue.argument));
        IrExpr::Update {
            op,
            target,
            is_expr_stmt: self.in_expr_stmt,
            prefix: ue.prefix,
        }
    }

    /// Lower an assignment expression.
    pub(super) fn lower_assignment(
        &mut self,
        ae: &AssignmentExpression,
    ) -> crate::zigir::types::IrExpr {
        use crate::zigir::types::IrExpr;

        // ── Special-case compound assignments that need expansion ──
        // **= → a = a ** b
        if ae.operator == AssignmentOperator::Exponential {
            let target = self.lower_assign_target(&ae.left);
            let value = Box::new(self.lower_expr(&ae.right));
            let target_type = self.infer_assign_target_type(&ae.left);
            let (bind, target, base_expr) = self.maybe_bind_member_object_for_compound(target);
            let break_value = base_expr.clone();
            let assign = if target_type == Some(ZigType::BigInt) {
                IrExpr::Assign {
                    op: AssignOp::Assign,
                    target: Box::new(target),
                    value: Box::new(IrExpr::Binary {
                        op: BinOp::Pow,
                        left: Box::new(base_expr),
                        right: value,
                        left_type: Some(ZigType::BigInt),
                        right_type: Some(ZigType::BigInt),
                    }),
                }
            } else {
                // Non-BigInt: a = std.math.pow(a, b) via PowExpr
                let base_type = target_type.unwrap_or(ZigType::F64);
                let exp_type = self.infer_expr_type(&ae.right).unwrap_or(ZigType::F64);
                // When assigning pow result to an i64 variable, set result_type so
                // the emit layer wraps in @as(i64, @intFromFloat(...))
                let result_type = if base_type == ZigType::I64 {
                    Some(ZigType::I64)
                } else {
                    None
                };
                IrExpr::Assign {
                    op: AssignOp::Assign,
                    target: Box::new(target),
                    value: Box::new(IrExpr::PowExpr {
                        base: Box::new(base_expr),
                        exp: value,
                        base_type,
                        exp_type,
                        result_type,
                    }),
                }
            };
            return self.wrap_compound_assign(bind, assign, break_value);
        }
        // >>>= → a = a >>> b (unsigned right shift cannot use >>= which is signed)
        // Expand early, same pattern as **=, so the emitter uses the UrShr path.
        if ae.operator == AssignmentOperator::ShiftRightZeroFill {
            let target = self.lower_assign_target(&ae.left);
            let value = Box::new(self.lower_expr(&ae.right));
            let target_type = self.infer_assign_target_type(&ae.left);
            let base_type = target_type.unwrap_or(ZigType::I64);
            let right_type = self.infer_expr_type(&ae.right).unwrap_or(ZigType::I64);
            let (bind, target, base_expr) = self.maybe_bind_member_object_for_compound(target);
            let break_value = base_expr.clone();
            let assign = IrExpr::Assign {
                op: AssignOp::Assign,
                target: Box::new(target),
                value: Box::new(IrExpr::Binary {
                    op: BinOp::UrShr,
                    left: Box::new(base_expr),
                    right: value,
                    left_type: Some(base_type),
                    right_type: Some(right_type),
                }),
            };
            return self.wrap_compound_assign(bind, assign, break_value);
        }
        // %= → a = a % b  (non-BigInt: use RemExpr for jsRem with type conversion)
        if ae.operator == AssignmentOperator::Remainder {
            let target = self.lower_assign_target(&ae.left);
            let target_type = self.infer_assign_target_type(&ae.left);
            let value = Box::new(self.lower_expr(&ae.right));
            // BigInt %= falls through to the BigInt compound expansion below
            if target_type != Some(ZigType::BigInt) {
                let (bind, target, base_expr) = self.maybe_bind_member_object_for_compound(target);
                let break_value = base_expr.clone();
                // When assigning % result to an i64 variable, set result_type so
                // the emit layer wraps in @as(i64, @intFromFloat(jsRem(...)))
                let result_type = if target_type == Some(ZigType::I64) {
                    Some(ZigType::I64)
                } else {
                    None
                };
                let left_type = target_type.unwrap_or(ZigType::I64);
                let right_type = self.infer_expr_type(&ae.right).unwrap_or(ZigType::I64);
                let assign = IrExpr::Assign {
                    op: AssignOp::Assign,
                    target: Box::new(target),
                    value: Box::new(IrExpr::RemExpr {
                        left: Box::new(base_expr),
                        right: value,
                        left_type,
                        right_type,
                        result_type,
                    }),
                };
                return self.wrap_compound_assign(bind, assign, break_value);
            }
            // BigInt %=: fall through to BigInt compound expansion below
        }
        // /= → a = a / b  (JS `/` always returns float; for i64 target, truncate back)
        if ae.operator == AssignmentOperator::Division {
            let target = self.lower_assign_target(&ae.left);
            let target_type = self.infer_assign_target_type(&ae.left);
            // BigInt /= falls through to the BigInt compound expansion below
            if target_type != Some(ZigType::BigInt) {
                let value = Box::new(self.lower_expr(&ae.right));
                let (bind, target, base_expr) = self.maybe_bind_member_object_for_compound(target);
                let break_value = base_expr.clone();
                let left_type = target_type.unwrap_or(ZigType::I64);
                let right_type = self.infer_expr_type(&ae.right).unwrap_or(ZigType::I64);
                // When assigning / result to an i64 variable, set result_type so
                // the emit layer wraps in @as(i64, @intFromFloat(...))
                let result_type = if left_type == ZigType::I64 {
                    Some(ZigType::I64)
                } else {
                    None
                };
                let assign = IrExpr::Assign {
                    op: AssignOp::Assign,
                    target: Box::new(target),
                    value: Box::new(IrExpr::DivExpr {
                        left: Box::new(base_expr),
                        right: value,
                        left_type,
                        right_type,
                        result_type,
                    }),
                };
                return self.wrap_compound_assign(bind, assign, break_value);
            }
            // BigInt /=: fall through to BigInt compound expansion below
        }
        // &&= / ||= / ??= → expand into Assign + Logical
        // This reuses the Logical emitter's type-aware JsAny.from() wrapping,
        // avoiding type mismatches between comptime_int and JsAny branches.
        // For &&/|| the emitter uses js_runtime.isTruthy() (anytype), so it works
        // for i64 as well. For ??= on non-JsAny types, the value can't be
        // null/undefined, so it's handled as a no-op (evaluate RHS, keep target).
        if matches!(
            ae.operator,
            AssignmentOperator::LogicalAnd
                | AssignmentOperator::LogicalOr
                | AssignmentOperator::LogicalNullish
        ) {
            let target_type = self
                .infer_assign_target_type(&ae.left)
                .unwrap_or(ZigType::JsAny);

            // ??= on non-JsAny types is a no-op: the value can't be null/undefined
            // in our type system (i64, bool, string, etc.). Short-circuit: just
            // return the target value without evaluating RHS (consistent with ??).
            if ae.operator == AssignmentOperator::LogicalNullish && target_type != ZigType::JsAny {
                let target = self.lower_assign_target(&ae.left);
                if let Some(read) = target.to_read_expr() {
                    return read;
                }
                // Fall through for unsupported targets
            } else {
                let target = self.lower_assign_target(&ae.left);
                // Only expand for Member/Ident targets (to_read_expr returns Some).
                // Index/Destructure fall through to the default path below.
                if matches!(
                    target,
                    crate::zigir::types::IrAssignTarget::Member { .. }
                        | crate::zigir::types::IrAssignTarget::Ident(_)
                ) {
                    let value = self.lower_expr(&ae.right);
                    let value_type = self.infer_expr_type(&ae.right).unwrap_or(ZigType::JsAny);
                    let logical_op = match ae.operator {
                        AssignmentOperator::LogicalAnd => LogicalOp::And,
                        AssignmentOperator::LogicalOr => LogicalOp::Or,
                        AssignmentOperator::LogicalNullish => LogicalOp::Nullish,
                        _ => LogicalOp::And, // safe fallback (P0-10 fix)
                    };
                    let (bind, target, read) = self.maybe_bind_member_object_for_compound(target);
                    let break_value = read.clone();
                    let assign = IrExpr::Assign {
                        op: AssignOp::Assign,
                        target: Box::new(target),
                        value: Box::new(IrExpr::Logical {
                            op: logical_op,
                            left: Box::new(read),
                            right: Box::new(value),
                            left_type: Some(target_type),
                            right_type: Some(value_type),
                        }),
                    };
                    return self.wrap_compound_assign(bind, assign, break_value);
                }
                // For unsupported targets (index, destructure), fall through to default path
            }
        }

        // ── BigInt compound assignment expansion ──
        // BigInt has no Zig +=, -=, etc. Expand `bigVar += expr` into
        // `bigVar = bigVar + expr` (using IrExpr::Binary with BigInt type info
        // so the Emitter generates .add() / .sub() / etc. method calls).
        let is_compound = ae.operator != AssignmentOperator::Assign
            && ae.operator != AssignmentOperator::LogicalAnd
            && ae.operator != AssignmentOperator::LogicalOr
            && ae.operator != AssignmentOperator::LogicalNullish;

        if is_compound {
            // Infer target type from the left-hand side
            let target_type = self.infer_assign_target_type(&ae.left);
            if target_type == Some(ZigType::BigInt) {
                let target = self.lower_assign_target(&ae.left);
                // Only expand for Member/Ident/Index targets (to_read_expr returns Some).
                if matches!(
                    target,
                    crate::zigir::types::IrAssignTarget::Member { .. }
                        | crate::zigir::types::IrAssignTarget::Ident(_)
                        | crate::zigir::types::IrAssignTarget::Index { .. }
                ) {
                    let value = Box::new(self.lower_expr(&ae.right));
                    let (bind, target, read) = self.maybe_bind_member_object_for_compound(target);
                    let break_value = read.clone();
                    let bin_op = match ae.operator {
                        AssignmentOperator::Addition => BinOp::Add,
                        AssignmentOperator::Subtraction => BinOp::Sub,
                        AssignmentOperator::Multiplication => BinOp::Mul,
                        AssignmentOperator::Division => BinOp::Div,
                        AssignmentOperator::Remainder => BinOp::Mod,
                        AssignmentOperator::ShiftLeft => BinOp::Shl,
                        AssignmentOperator::ShiftRight => BinOp::Shr,
                        AssignmentOperator::ShiftRightZeroFill => BinOp::UrShr,
                        AssignmentOperator::BitwiseAnd => BinOp::BitAnd,
                        AssignmentOperator::BitwiseOR => BinOp::BitOr,
                        AssignmentOperator::BitwiseXOR => BinOp::BitXor,
                        _ => BinOp::Add, // fallback, shouldn't reach here
                    };
                    let assign = IrExpr::Assign {
                        op: AssignOp::Assign,
                        target: Box::new(target),
                        value: Box::new(IrExpr::Binary {
                            op: bin_op,
                            left: Box::new(read),
                            right: value,
                            left_type: Some(ZigType::BigInt),
                            right_type: Some(ZigType::BigInt),
                        }),
                    };
                    return self.wrap_compound_assign(bind, assign, break_value);
                }
                // For unsupported BigInt targets (index, destructure), fall through
                // to default path (may produce invalid Zig but handles common cases)
            }
        }

        let op = match ae.operator {
            AssignmentOperator::Assign => AssignOp::Assign,
            AssignmentOperator::Addition => AssignOp::Add,
            AssignmentOperator::Subtraction => AssignOp::Sub,
            AssignmentOperator::Multiplication => AssignOp::Mul,
            AssignmentOperator::Division => AssignOp::Div,
            AssignmentOperator::Remainder => AssignOp::Mod,
            AssignmentOperator::ShiftLeft => AssignOp::Shl,
            AssignmentOperator::ShiftRight => AssignOp::Shr,
            AssignmentOperator::BitwiseAnd => AssignOp::BitAnd,
            AssignmentOperator::BitwiseOR => AssignOp::BitOr,
            AssignmentOperator::BitwiseXOR => AssignOp::BitXor,
            AssignmentOperator::LogicalAnd => AssignOp::LogicAnd,
            AssignmentOperator::LogicalOr => AssignOp::LogicOr,
            AssignmentOperator::LogicalNullish => AssignOp::Nullish,
            // Exponential handled above
            _ => AssignOp::Assign,
        };
        let target = Box::new(self.lower_assign_target(&ae.left));
        let value = Box::new(self.lower_expr(&ae.right));

        // ── Detect `target = expr % expr2` where target is i64 ──
        // If the value is a RemExpr (integer %) and the target is i64,
        // set result_type so the emitter wraps in @intFromFloat.
        if op == AssignOp::Assign {
            let target_type = self.infer_assign_target_type(&ae.left);
            if target_type == Some(ZigType::I64)
                && let IrExpr::RemExpr {
                    result_type: None,
                    left,
                    right,
                    left_type,
                    right_type,
                } = value.as_ref()
            {
                return IrExpr::Assign {
                    op,
                    target,
                    value: Box::new(IrExpr::RemExpr {
                        left: left.clone(),
                        right: right.clone(),
                        left_type: left_type.clone(),
                        right_type: right_type.clone(),
                        result_type: Some(ZigType::I64),
                    }),
                };
            }
            // Same for PowExpr: `x = base ** exp` when target is i64
            if target_type == Some(ZigType::I64)
                && let IrExpr::PowExpr {
                    result_type: None,
                    base,
                    exp,
                    base_type,
                    exp_type,
                } = value.as_ref()
            {
                return IrExpr::Assign {
                    op,
                    target,
                    value: Box::new(IrExpr::PowExpr {
                        base: base.clone(),
                        exp: exp.clone(),
                        base_type: base_type.clone(),
                        exp_type: exp_type.clone(),
                        result_type: Some(ZigType::I64),
                    }),
                };
            }
            // Same for DivExpr: `x = a / b` when target is i64
            if target_type == Some(ZigType::I64)
                && let IrExpr::DivExpr {
                    result_type: None,
                    left,
                    right,
                    left_type,
                    right_type,
                } = value.as_ref()
            {
                return IrExpr::Assign {
                    op,
                    target,
                    value: Box::new(IrExpr::DivExpr {
                        left: left.clone(),
                        right: right.clone(),
                        left_type: left_type.clone(),
                        right_type: right_type.clone(),
                        result_type: Some(ZigType::I64),
                    }),
                };
            }
        }

        IrExpr::Assign { op, target, value }
    }

    /// For compound-assignment expansions (`**=`, `>>>=`, `%=`, `/=`, `&&=`,
    /// `||=`, `??=`, BigInt compound), check if the Member target's object
    /// expression might have side effects (e.g. `getObj().prop += 1`).
    ///
    /// If so, bind it to a temp variable `__co_N` so the object is only
    /// evaluated once. Returns `(optional (var_decl, block_label),
    /// possibly-rewritten target, read_expr)`. Callers should pass the
    /// returned bind info and final Assign to `wrap_compound_assign`.
    ///
    /// For simple lvalue paths (`a`, `a.b`, `a[i]`) — no side effects — or
    /// non-Member targets, returns `(None, original_target, to_read_expr)`.
    fn maybe_bind_member_object_for_compound(
        &mut self,
        target: crate::zigir::types::IrAssignTarget,
    ) -> (
        Option<(crate::zigir::types::IrStmt, String)>,
        crate::zigir::types::IrAssignTarget,
        crate::zigir::types::IrExpr,
    ) {
        use crate::zigir::types::{IrAssignTarget, IrExpr, IrStmt, IrVarDecl};

        let IrAssignTarget::Member {
            object,
            field,
            is_pointer,
            field_kind,
        } = &target
        else {
            let read = target
                .to_read_expr()
                .unwrap_or_else(|| IrExpr::Ident(IrIdent::new("__target")));
            return (None, target, read);
        };

        // Simple lvalue paths (Ident, FieldAccess chain, IndexAccess of simple)
        // have no side effects — safe to evaluate twice without binding.
        if self.ir_object_is_simple_lvalue(object) {
            let read = target
                .to_read_expr()
                .unwrap_or_else(|| IrExpr::Ident(IrIdent::new("__target")));
            return (None, target, read);
        }

        // Bind the object to a temp variable to ensure single evaluation.
        let temp_name = self.name_mangler.next_name("__co");
        let blk_label = format!("_co_blk{}", self.name_mangler.peek_count("__co"));
        let temp_ident = IrExpr::Ident(IrIdent::new(&temp_name));

        let var_decl = IrStmt::VarDecl(IrVarDecl {
            name: IrIdent::new(&temp_name),
            is_const: true,
            zig_type: None,
            init: Some((**object).clone()),
            is_json_parse: false,
            needs_var_suppression: false,
            needs_deinit: false,
        });

        let new_target = IrAssignTarget::Member {
            object: Box::new(temp_ident.clone()),
            field: field.clone(),
            is_pointer: *is_pointer,
            field_kind: field_kind.clone(),
        };
        let read = IrExpr::FieldAccess {
            object: Box::new(temp_ident),
            field: field.clone(),
            field_kind: field_kind.clone(),
        };

        (Some((var_decl, blk_label)), new_target, read)
    }

    /// Check if an IrExpr is a "simple lvalue path": an Ident, This, or a chain of
    /// FieldAccess/IndexAccess over simple lvalue paths. Such expressions have
    /// no side effects and can be safely evaluated twice without a temp binding.
    fn ir_object_is_simple_lvalue(&self, expr: &crate::zigir::types::IrExpr) -> bool {
        use crate::zigir::types::IrExpr;
        match expr {
            IrExpr::Ident(_) | IrExpr::TypedIdent { .. } | IrExpr::This => true,
            IrExpr::FieldAccess { object, .. } => self.ir_object_is_simple_lvalue(object),
            IrExpr::IndexAccess { object, index, .. } => {
                self.ir_object_is_simple_lvalue(object) && self.ir_object_is_simple_lvalue(index)
            }
            _ => false,
        }
    }

    /// Wrap a compound-assignment expansion in a BlockExpr if the target's
    /// object was bound to a temp variable. The BlockExpr runs the temp
    /// binding, executes the Assign as a statement, then breaks with the
    /// `read_expr` (which re-reads the target field, yielding the NEW value).
    /// This matches JS semantics where `a op= b` returns the assigned value,
    /// and ensures single evaluation of side-effecting object expressions.
    fn wrap_compound_assign(
        &self,
        bind: Option<(crate::zigir::types::IrStmt, String)>,
        assign: crate::zigir::types::IrExpr,
        read_expr: crate::zigir::types::IrExpr,
    ) -> crate::zigir::types::IrExpr {
        use crate::zigir::types::{IrExpr, IrStmt};
        if let Some((var_decl, label)) = bind {
            IrExpr::BlockExpr {
                label,
                body: vec![var_decl, IrStmt::Expr(assign)],
                result: Box::new(read_expr),
            }
        } else {
            assign
        }
    }

    /// Lower an identifier assignment target, handling captured closure variables.
    fn lower_ident_assign_target(&mut self, var_name: &str) -> crate::zigir::types::IrAssignTarget {
        // Check if this identifier is a captured closure variable.
        if let Some((_, _, is_mut)) = self
            .closure_mgr
            .current_captured
            .iter()
            .find(|(n, _, _)| n == var_name)
        {
            let field_name = self.make_ident(var_name).zig_name;
            return crate::zigir::types::IrAssignTarget::Member {
                object: Box::new(crate::zigir::types::IrExpr::Ident(IrIdent::new("self"))),
                field: field_name,
                is_pointer: *is_mut,
                field_kind: if *is_mut {
                    FieldKind::PointerDeref
                } else {
                    FieldKind::StructField
                },
            };
        }

        crate::zigir::types::IrAssignTarget::Ident(IrIdent::new(var_name))
    }

    /// Lower a static member assignment target (obj.prop).
    fn lower_static_member_assign_target(
        &mut self,
        object: &Expression,
        property_name: &str,
    ) -> crate::zigir::types::IrAssignTarget {
        crate::zigir::types::IrAssignTarget::Member {
            object: Box::new(self.lower_expr(object)),
            field: property_name.to_string(),
            is_pointer: false,
            field_kind: self.infer_member_field_kind(object, property_name),
        }
    }

    /// Lower a computed member assignment target (obj[expr]).
    fn lower_computed_member_assign_target(
        &mut self,
        object: &Expression,
        expression: &Expression,
    ) -> crate::zigir::types::IrAssignTarget {
        let obj_type = self.infer_expr_type(object);
        let is_arraylist = obj_type
            .as_ref()
            .map(|t| matches!(t, ZigType::ArrayList(_)))
            .unwrap_or(false);
        crate::zigir::types::IrAssignTarget::Index {
            object: Box::new(self.lower_expr(object)),
            index: Box::new(self.lower_expr(expression)),
            index_kind: if is_arraylist {
                IndexKind::ArrayListItem
            } else {
                IndexKind::SliceIndex
            },
        }
    }

    /// Lower a simple assignment target (from UpdateExpression).
    /// SimpleAssignmentTarget can be an identifier or member expression.
    pub(super) fn lower_simple_assign_target(
        &mut self,
        target: &SimpleAssignmentTarget,
    ) -> crate::zigir::types::IrAssignTarget {
        match target {
            SimpleAssignmentTarget::AssignmentTargetIdentifier(id) => {
                self.lower_ident_assign_target(id.name.as_str())
            }
            SimpleAssignmentTarget::StaticMemberExpression(mem) => {
                self.lower_static_member_assign_target(&mem.object, mem.property.name.as_str())
            }
            SimpleAssignmentTarget::ComputedMemberExpression(mem) => {
                self.lower_computed_member_assign_target(&mem.object, &mem.expression)
            }
            SimpleAssignmentTarget::PrivateFieldExpression(pfe) => {
                // Private field assignment (this.#field += ...) — treat as struct field access
                self.lower_static_member_assign_target(&pfe.object, pfe.field.name.as_str())
            }
            _ => crate::zigir::types::IrAssignTarget::CompileError {
                msg: "unsupported assignment target".to_string(),
            },
        }
    }

    /// Extract (pattern, default) from an AssignmentTargetMaybeDefault.
    pub(super) fn lower_maybe_default(
        &mut self,
        target: &AssignmentTargetMaybeDefault,
    ) -> (IrIdent, Option<crate::zigir::types::IrExpr>) {
        match target {
            AssignmentTargetMaybeDefault::AssignmentTargetWithDefault(atwd) => {
                let pattern = self.extract_target_ident(&atwd.binding);
                let default = Some(self.lower_expr(&atwd.init));
                (pattern, default)
            }
            AssignmentTargetMaybeDefault::AssignmentTargetIdentifier(id) => {
                (IrIdent::new(id.name.as_str()), None)
            }
            _ => (IrIdent::new("_"), None),
        }
    }

    /// Extract an identifier name from an AssignmentTarget (best-effort).
    pub(super) fn extract_target_ident(&self, target: &AssignmentTarget) -> IrIdent {
        match target {
            AssignmentTarget::AssignmentTargetIdentifier(id) => IrIdent::new(id.name.as_str()),
            _ => IrIdent::new("_"),
        }
    }

    /// Lower an assignment target (lhs).
    pub(super) fn lower_assign_target(
        &mut self,
        target: &AssignmentTarget,
    ) -> crate::zigir::types::IrAssignTarget {
        match target {
            AssignmentTarget::AssignmentTargetIdentifier(id) => {
                self.lower_ident_assign_target(id.name.as_str())
            }
            AssignmentTarget::StaticMemberExpression(mem) => {
                self.lower_static_member_assign_target(&mem.object, mem.property.name.as_str())
            }
            AssignmentTarget::ComputedMemberExpression(mem) => {
                self.lower_computed_member_assign_target(&mem.object, &mem.expression)
            }
            AssignmentTarget::PrivateFieldExpression(pfe) => {
                // Private field assignment (this.#field = ...) — treat as struct field access
                self.lower_static_member_assign_target(&pfe.object, pfe.field.name.as_str())
            }
            AssignmentTarget::ObjectAssignmentTarget(ot) => {
                let bindings: Vec<crate::zigir::types::IrDestructureBinding> = ot
                    .properties
                    .iter()
                    .map(|prop| {
                        match prop {
                            AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(ap) => {
                                let pattern = IrIdent::new(ap.binding.name.as_str());
                                let default = ap.init.as_ref().map(|e| self.lower_expr(e));
                                crate::zigir::types::IrDestructureBinding { pattern, default }
                            }
                            AssignmentTargetProperty::AssignmentTargetPropertyProperty(ap) => {
                                // e.g. { name: alias } — extract binding from value
                                let (pattern, default) = self.lower_maybe_default(&ap.binding);
                                crate::zigir::types::IrDestructureBinding { pattern, default }
                            }
                        }
                    })
                    .collect();
                crate::zigir::types::IrAssignTarget::Destructure(bindings)
            }
            AssignmentTarget::ArrayAssignmentTarget(at) => {
                let bindings: Vec<crate::zigir::types::IrDestructureBinding> = at
                    .elements
                    .iter()
                    .filter_map(|elem| {
                        elem.as_ref().map(|target| {
                            let (pattern, default) = self.lower_maybe_default(target);
                            crate::zigir::types::IrDestructureBinding { pattern, default }
                        })
                    })
                    .collect();
                crate::zigir::types::IrAssignTarget::Destructure(bindings)
            }
            _ => crate::zigir::types::IrAssignTarget::CompileError {
                msg: "unsupported assignment target".to_string(),
            },
        }
    }

    /// Check if an expression is a string type (for string concatenation detection).
    ///
    /// NOTE: A parallel implementation exists on `TypeInferrer` (infer/expr.rs).
    /// The two are NOT merged because they use different type-lookup mechanisms:
    /// - `TypeInferrer` checks `self.var_types` (built during analysis pass)
    /// - `Lowerer` calls `self.infer_expr_type` (reads from TypeCheckResult snapshot)
    ///
    /// The match structure (literals, binary `+`, conditional, paren) is aligned
    /// between the two (P2-2).
    pub(super) fn expr_is_string(&self, expr: &Expression) -> bool {
        match expr {
            Expression::StringLiteral(_) => true,
            Expression::TemplateLiteral(_) => true,
            Expression::Identifier(_id) => {
                // Use infer_expr_type which handles qualified name lookup
                self.infer_expr_type(expr) == Some(ZigType::Str)
            }
            Expression::BinaryExpression(be) if be.operator == BinaryOperator::Addition => {
                self.expr_is_string(&be.left) || self.expr_is_string(&be.right)
            }
            // ConditionalExpression: result is string only if both branches are strings
            Expression::ConditionalExpression(ce) => {
                self.expr_is_string(&ce.consequent) && self.expr_is_string(&ce.alternate)
            }
            Expression::ParenthesizedExpression(pe) => self.expr_is_string(&pe.expression),
            _ => self.infer_expr_type(expr) == Some(ZigType::Str),
        }
    }

    /// Lower a string concatenation chain into IrExpr::AllocPrint.
    /// Flattens nested `a + b + c` into a single format string + args list.
    pub(super) fn lower_string_concat(
        &mut self,
        be: &BinaryExpression,
    ) -> crate::zigir::types::IrExpr {
        use crate::zigir::types::IrExpr;

        let mut operands: Vec<&Expression> = Vec::new();
        self.collect_concat_from_be(be, &mut operands);

        let mut fmt = String::new();
        let mut args: Vec<IrExpr> = Vec::new();

        for op in &operands {
            match op {
                Expression::StringLiteral(sl) => {
                    fmt.push_str(&crate::zigir::emit::helpers::escape_zig_format_string(
                        &sl.value,
                    ));
                }
                _ => {
                    // Lower the operand ONCE so we can wrap BigInt operands in
                    // a `.toString(allocator)` call (R6-4). Pre-fix the BigInt
                    // operand was lowered directly and formatted via `{any}`,
                    // which invoked `JsBigInt.format()` — and that method
                    // appends a trailing `"n"` (for Node.js console.log
                    // parity), producing `"12n"` instead of `"12"` for
                    // `"1" + 2n`.
                    //
                    // Unwrap parentheses before lowering.
                    let lowered = match op {
                        Expression::ParenthesizedExpression(pe) => self.lower_expr(&pe.expression),
                        _ => self.lower_expr(op),
                    };

                    let operand_type = if self.expr_is_string(op) {
                        Some(ZigType::Str)
                    } else {
                        self.infer_expr_type(op)
                    };

                    // Each arm pushes `lowered` to `args` exactly once. Moving
                    // `lowered` only inside the BigInt arm BuiltinCall used to
                    // break the borrow checker — the runtime
                    // `if !matches!(operand_type, …)` guard on a post-match
                    // `args.push(lowered)` couldn't narrow the move.
                    let placeholder = match operand_type {
                        Some(ZigType::BigInt) => {
                            // Wrap the lowered BigInt expression in
                            // `(.toString(allocator) catch @panic(...))` so the
                            // resulting slice formats as the decimal value
                            // (no trailing "n") via the `{s}` specifier.
                            args.push(IrExpr::BuiltinCall(
                                crate::zigir::types::IrBuiltinCall::simple(
                                    BuiltinModule::JsBigInt,
                                    "toString",
                                    None,
                                    Some(Box::new(lowered)),
                                    vec![],
                                    ZigType::Str,
                                ),
                            ));
                            "{s}"
                        }
                        Some(ZigType::Str) => {
                            args.push(lowered);
                            "{s}"
                        }
                        Some(ref ty) => {
                            args.push(lowered);
                            helpers::format_specifier_for_type(ty)
                        }
                        None => {
                            args.push(lowered);
                            "{any}"
                        }
                    };
                    fmt.push_str(placeholder);
                }
            }
        }

        IrExpr::AllocPrint { fmt, args }
    }

    /// Recursively collect all operands in a string concatenation chain.
    /// Only recurses into nested `+` when the sub-tree itself involves a
    /// string operand; this preserves JS left-to-right evaluation for
    /// purely numeric sub-expressions (e.g. `1 + 2 + "3"` must produce
    /// `"33"`, not `"123"`).
    pub(super) fn collect_concat_from_be<'a>(
        &self,
        be: &'a BinaryExpression<'a>,
        out: &mut Vec<&'a Expression<'a>>,
    ) {
        // Left side
        if let Expression::BinaryExpression(ref left_be) = be.left {
            if left_be.operator == BinaryOperator::Addition && self.expr_is_string(&be.left) {
                self.collect_concat_from_be(left_be, out);
            } else {
                out.push(&be.left);
            }
        } else {
            out.push(&be.left);
        }

        // Right side
        if let Expression::BinaryExpression(ref right_be) = be.right {
            if right_be.operator == BinaryOperator::Addition && self.expr_is_string(&be.right) {
                self.collect_concat_from_be(right_be, out);
            } else {
                out.push(&be.right);
            }
        } else {
            out.push(&be.right);
        }
    }

    /// Lower `x instanceof Type` with prototype chain semantics.
    ///
    /// Three strategies:
    /// 1. **Error types** → `e.name == "TypeName"` (existing efficient approach)
    /// 2. **Compile-time type-aware** → known left type resolves at transpile time
    /// 3. **Runtime** → emit `js_runtime.instanceOf(value, "TypeName")` for dynamic types
    fn lower_instanceof(&mut self, be: &BinaryExpression) -> crate::zigir::types::IrExpr {
        use crate::zigir::builtins::BuiltinModule;
        use crate::zigir::kinds::FieldKind;
        use crate::zigir::types::{IrBuiltinCall, IrExpr};

        // Get the type name from the right operand
        let type_name = if let Expression::Identifier(ident) = &be.right {
            ident.name.to_string()
        } else {
            return self.compile_error_expr(
                be.span,
                "instanceof: right operand must be an identifier (constructor name)",
            );
        };

        // ── Strategy 1: Error types → .name comparison ──
        let error_types = [
            "Error",
            "URIError",
            "TypeError",
            "RangeError",
            "SyntaxError",
            "ReferenceError",
            "EvalError",
            "AggregateError",
            "SuppressedError",
        ];
        if error_types.contains(&type_name.as_str()) {
            let left_expr = self.lower_expr(&be.left);
            return IrExpr::Binary {
                op: BinOp::Eq,
                left: Box::new(IrExpr::FieldAccess {
                    object: Box::new(left_expr),
                    field: "name".to_string(),
                    field_kind: FieldKind::StructField,
                }),
                right: Box::new(IrExpr::StringLiteral(type_name)),
                left_type: Some(ZigType::Str),
                right_type: Some(ZigType::Str),
            };
        }

        // ── Strategy 2: Compile-time type-aware instanceof ──
        if let Some(left_ty) = self.infer_expr_type(&be.left)
            && let Some(result) = self.resolve_instanceof_compile_time(&left_ty, &type_name)
        {
            return result;
        }

        // ── Strategy 3: Runtime instanceof check ──
        let left_expr = self.lower_expr(&be.left);
        IrExpr::BuiltinCall(IrBuiltinCall::simple(
            BuiltinModule::JsRuntime,
            "instanceOf",
            None,
            Some(Box::new(left_expr)),
            vec![IrExpr::StringLiteral(type_name)],
            ZigType::Bool,
        ))
    }

    /// Resolve `instanceof` at compile time when the left operand's type is known.
    ///
    /// Returns `Some(IrExpr)` if resolved, `None` if we need runtime dispatch.
    fn resolve_instanceof_compile_time(
        &self,
        left_ty: &ZigType,
        type_name: &str,
    ) -> Option<crate::zigir::types::IrExpr> {
        use crate::zigir::types::IrExpr;

        match left_ty {
            // ArrayList matches Array
            ZigType::ArrayList(_) => {
                if type_name == "Array" || type_name == "Object" {
                    return Some(IrExpr::BoolLiteral(true));
                }
                Some(IrExpr::BoolLiteral(false))
            }
            // Map matches Map and Object
            ZigType::NamedStruct(name) if name == "Map" => {
                if type_name == "Map" || type_name == "Object" {
                    return Some(IrExpr::BoolLiteral(true));
                }
                Some(IrExpr::BoolLiteral(false))
            }
            // Set matches Set and Object
            ZigType::NamedStruct(name) if name == "Set" => {
                if type_name == "Set" || type_name == "Object" {
                    return Some(IrExpr::BoolLiteral(true));
                }
                Some(IrExpr::BoolLiteral(false))
            }
            // Custom class: check direct match and prototype chain
            ZigType::NamedStruct(class_name) => {
                if class_name == type_name {
                    return Some(IrExpr::BoolLiteral(true));
                }
                if type_name == "Object" {
                    // All class instances are instanceof Object
                    return Some(IrExpr::BoolLiteral(true));
                }
                // Walk prototype chain via class_extends_map
                let mut current = class_name.as_str();
                loop {
                    if current == type_name {
                        return Some(IrExpr::BoolLiteral(true));
                    }
                    if let Some(parent) = self.class_extends_map.get(current) {
                        current = parent.as_str();
                    } else {
                        break;
                    }
                }
                Some(IrExpr::BoolLiteral(false))
            }
            // String primitives: NOT instanceof String in JS (only String objects are)
            ZigType::Str => Some(IrExpr::BoolLiteral(false)),
            // Numeric/Boolean primitives are never `instanceof` their wrapper types
            // in JS (primitives lack [[Prototype]]).  Both branches always return false
            // regardless of the right-hand type_name.
            ZigType::I64 | ZigType::F64 | ZigType::Bool => Some(IrExpr::BoolLiteral(false)),
            // For JsAny / Anytype, we can't resolve at compile time
            ZigType::JsAny | ZigType::Anytype => None,
            // Other types: conservatively say we can't resolve
            _ => None,
        }
    }

    fn bigint_unary_builtin(
        &mut self,
        method: &str,
        operand: crate::zigir::types::IrExpr,
    ) -> crate::zigir::types::IrExpr {
        crate::zigir::types::IrExpr::BuiltinCall(crate::zigir::types::IrBuiltinCall::simple(
            BuiltinModule::JsBigInt,
            method,
            None,
            Some(Box::new(operand)),
            vec![],
            ZigType::BigInt,
        ))
    }
}
