// zigir/lower/expr/operators.rs
// Binary, unary, update, assignment expression lowering + string concatenation.

use std::collections::HashSet;

use oxc_ast::ast::*;

use crate::types::ZigType;
use crate::zigir::builtins::BuiltinModule;
use crate::zigir::ident::IrIdent;
use crate::zigir::kinds::{FieldKind, IndexKind};
use crate::zigir::ops::{AssignOp, BinOp, UnaOp, UpdateOp};

use super::Lowerer;

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
                // `key in obj` → obj.contains(key)
                let right_expr = self.lower_expr(&be.right);
                let left_expr = self.lower_expr(&be.left);
                return IrExpr::Binary {
                    op: BinOp::In,
                    left: Box::new(left_expr),
                    right: Box::new(right_expr),
                    left_type: Some(ZigType::Str),
                    right_type: None,
                };
            }
            _ => {}
        }

        let op = match be.operator {
            BinaryOperator::Addition => BinOp::Add,
            BinaryOperator::Subtraction => BinOp::Sub,
            BinaryOperator::Multiplication => BinOp::Mul,
            BinaryOperator::Division => BinOp::Div,
            BinaryOperator::Remainder => BinOp::Mod,
            BinaryOperator::Exponential => {
                let left_type = self.infer_expr_type(&be.left).unwrap_or(ZigType::F64);
                let right_type = self.infer_expr_type(&be.right).unwrap_or(ZigType::F64);
                // BigInt ** BigInt: use BinOp::Pow so emit_bigint_binary generates .pow() call.
                if left_type == ZigType::BigInt || right_type == ZigType::BigInt {
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
            // Instanceof is handled above (CompileError). In is also handled above (BinOp::In).
            // These arms are unreachable but kept for exhaustiveness.
            BinaryOperator::Instanceof | BinaryOperator::In => unreachable!(),
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
                // BigInt cannot use Zig's `-` operator — expand to .neg() method call
                if self.infer_expr_type(&ue.argument) == Some(ZigType::BigInt) {
                    return IrExpr::BuiltinCall(crate::zigir::types::IrBuiltinCall {
                        module: BuiltinModule::JsBigInt,
                        method: "bigIntNeg".to_string(),
                        obj_name: None,
                        obj_expr: Some(Box::new(self.lower_expr(&ue.argument))),
                        args: vec![],
                        return_type: ZigType::BigInt,
                        ta_type_suffix: None,
                        regex_info: None,
                    });
                }
                IrExpr::Unary {
                    op: UnaOp::Neg,
                    operand: Box::new(self.lower_expr(&ue.argument)),
                }
            }
            UnaryOperator::UnaryPlus => {
                // Unary plus is a no-op in terms of IR; just lower the argument
                self.lower_expr(&ue.argument)
            }
            UnaryOperator::LogicalNot => IrExpr::Unary {
                op: UnaOp::Not,
                operand: Box::new(self.lower_expr(&ue.argument)),
            },
            UnaryOperator::BitwiseNot => {
                // BigInt cannot use Zig's `~` operator — expand to .bitwiseNot() method call
                if self.infer_expr_type(&ue.argument) == Some(ZigType::BigInt) {
                    return IrExpr::BuiltinCall(crate::zigir::types::IrBuiltinCall {
                        module: BuiltinModule::JsBigInt,
                        method: "bigIntBitwiseNot".to_string(),
                        obj_name: None,
                        obj_expr: Some(Box::new(self.lower_expr(&ue.argument))),
                        args: vec![],
                        return_type: ZigType::BigInt,
                        ta_type_suffix: None,
                        regex_info: None,
                    });
                }
                IrExpr::Unary {
                    op: UnaOp::BitNot,
                    operand: Box::new(self.lower_expr(&ue.argument)),
                }
            }
            UnaryOperator::Typeof => {
                // Use inferred Zig type to emit the JS typeof string at compile time.
                // For dynamic types (JsAny/Anytype), call the runtime jsTypeof() helper.
                if let Some(ty) = self.infer_expr_type(&ue.argument) {
                    if let Some(js_typeof) = ty.to_js_typeof() {
                        // Compile-time resolution: the argument is not included in the IR.
                        // Track its identifiers so unused-param detection doesn't
                        // falsely mark them as unused.
                        let mut idents = HashSet::new();
                        Self::collect_ast_expr_idents(&ue.argument, &mut idents);
                        if let Some(ctx) = self.fn_ctx.as_mut() {
                            ctx.compile_time_referenced_idents.extend(idents);
                        }
                        IrExpr::StringLiteral(js_typeof.to_string())
                    } else {
                        IrExpr::BuiltinCall(crate::zigir::types::IrBuiltinCall {
                            module: BuiltinModule::JsRuntime,
                            method: "jsTypeof".to_string(),
                            obj_name: None,
                            obj_expr: None,
                            args: vec![self.lower_expr(&ue.argument)],
                            return_type: ZigType::Str,
                            regex_info: None,
                            ta_type_suffix: None,
                        })
                    }
                } else {
                    IrExpr::BuiltinCall(crate::zigir::types::IrBuiltinCall {
                        module: BuiltinModule::JsRuntime,
                        method: "jsTypeof".to_string(),
                        obj_name: None,
                        obj_expr: None,
                        args: vec![self.lower_expr(&ue.argument)],
                        return_type: ZigType::Str,
                        regex_info: None,
                        ta_type_suffix: None,
                    })
                }
            }
            UnaryOperator::Void => IrExpr::Void(Box::new(self.lower_expr(&ue.argument))),
            UnaryOperator::Delete => {
                // delete obj.prop → IrBuiltinCall { JsRuntime, "deleteKey", obj, [prop] }
                // delete obj[expr] → IrBuiltinCall { JsRuntime, "deleteByKey", obj, [expr] }
                match &ue.argument {
                    Expression::StaticMemberExpression(mem) => {
                        let obj_name = match &mem.object {
                            Expression::Identifier(id) => Some(id.name.as_str().to_string()),
                            _ => None,
                        };
                        IrExpr::BuiltinCall(crate::zigir::types::IrBuiltinCall {
                            module: BuiltinModule::JsRuntime,
                            method: "deleteKey".to_string(),
                            obj_name,
                            obj_expr: None,
                            args: vec![IrExpr::StringLiteral(
                                mem.property.name.as_str().to_string(),
                            )],
                            return_type: ZigType::Bool,
                            regex_info: None,
                            ta_type_suffix: None,
                        })
                    }
                    Expression::ComputedMemberExpression(mem) => {
                        let obj_name = if let Expression::Identifier(id) = &mem.object {
                            Some(id.name.as_str().to_string())
                        } else {
                            None
                        };
                        IrExpr::BuiltinCall(crate::zigir::types::IrBuiltinCall {
                            module: BuiltinModule::JsRuntime,
                            method: "deleteByKey".to_string(),
                            obj_name,
                            obj_expr: None,
                            args: vec![self.lower_expr(&mem.expression)],
                            return_type: ZigType::Bool,
                            regex_info: None,
                            ta_type_suffix: None,
                        })
                    }
                    _ => {
                        // Unsupported delete target — emit compile error
                        IrExpr::CompileError {
                            span: self.span_to_source_span(ue.span),
                            msg: "delete operator requires property access".to_string(),
                        }
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
            // Build read-side expression
            let read_expr = match &target {
                crate::zigir::types::IrAssignTarget::Ident(name) => IrExpr::Ident(name.clone()),
                crate::zigir::types::IrAssignTarget::Member {
                    object,
                    field,
                    field_kind,
                    ..
                } => IrExpr::FieldAccess {
                    object: object.clone(),
                    field: field.clone(),
                    field_kind: field_kind.clone(),
                },
                _ => {
                    // Fallback for unsupported target types (index, destructure)
                    return IrExpr::Update {
                        op: if ue.operator == UpdateOperator::Increment {
                            UpdateOp::Increment
                        } else {
                            UpdateOp::Decrement
                        },
                        target: Box::new(target),
                        is_expr_stmt: self.in_expr_stmt,
                    };
                }
            };
            let bin_op = if ue.operator == UpdateOperator::Increment {
                BinOp::Add
            } else {
                BinOp::Sub
            };
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
            // Build read-side expression from the assignment target
            let base_expr = match &target {
                crate::zigir::types::IrAssignTarget::Ident(name) => IrExpr::Ident(name.clone()),
                crate::zigir::types::IrAssignTarget::Member {
                    object,
                    field,
                    field_kind,
                    ..
                } => IrExpr::FieldAccess {
                    object: object.clone(),
                    field: field.clone(),
                    field_kind: field_kind.clone(),
                },
                _ => IrExpr::Ident(IrIdent::new("__target")),
            };
            // BigInt **= : use IrExpr::Binary with BinOp::Pow so emit_bigint_binary
            // generates .pow() method call.
            if target_type == Some(ZigType::BigInt) {
                return IrExpr::Assign {
                    op: AssignOp::Assign,
                    target: Box::new(target),
                    value: Box::new(IrExpr::Binary {
                        op: BinOp::Pow,
                        left: Box::new(base_expr),
                        right: value,
                        left_type: Some(ZigType::BigInt),
                        right_type: Some(ZigType::BigInt),
                    }),
                };
            }
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
            return IrExpr::Assign {
                op: AssignOp::Assign,
                target: Box::new(target),
                value: Box::new(IrExpr::PowExpr {
                    base: Box::new(base_expr),
                    exp: value,
                    base_type,
                    exp_type,
                    result_type,
                }),
            };
        }
        // &&= / ||= / ??= → use AssignOp, Emitter will expand

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
                let value = Box::new(self.lower_expr(&ae.right));
                // Build the read-side expression for the target.
                let read_expr = match &target {
                    crate::zigir::types::IrAssignTarget::Ident(name) => {
                        Some(IrExpr::Ident(name.clone()))
                    }
                    crate::zigir::types::IrAssignTarget::Member {
                        object,
                        field,
                        field_kind,
                        ..
                    } => Some(IrExpr::FieldAccess {
                        object: object.clone(),
                        field: field.clone(),
                        field_kind: field_kind.clone(),
                    }),
                    _ => None,
                };
                if let Some(read) = read_expr {
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
                    return IrExpr::Assign {
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
            AssignmentOperator::ShiftRightZeroFill => AssignOp::Shr,
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
        IrExpr::Assign { op, target, value }
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
        Self::collect_concat_from_be(be, &mut operands);

        let mut fmt = String::new();
        let mut args: Vec<IrExpr> = Vec::new();

        for op in &operands {
            match op {
                Expression::StringLiteral(sl) => {
                    // Escape for Zig format string literal
                    for ch in sl.value.chars() {
                        match ch {
                            '\\' => fmt.push_str("\\\\"),
                            '"' => fmt.push_str("\\\""),
                            '\n' => fmt.push_str("\\n"),
                            '\r' => fmt.push_str("\\r"),
                            '\t' => fmt.push_str("\\t"),
                            '{' => fmt.push_str("{{"),
                            '}' => fmt.push_str("}}"),
                            c => fmt.push(c),
                        }
                    }
                }
                _ => {
                    // Pick placeholder based on inferred type
                    let placeholder = if self.expr_is_string(op) {
                        "{s}"
                    } else {
                        match self.infer_expr_type(op) {
                            Some(ZigType::Str) => "{s}",
                            Some(ZigType::I64) => "{d}",
                            Some(ZigType::F64) => "{d:.15}",
                            Some(ZigType::Bool) => "{}",
                            _ => "{}",
                        }
                    };
                    fmt.push_str(placeholder);
                    // Unwrap parentheses before lowering
                    let lowered = match op {
                        Expression::ParenthesizedExpression(pe) => self.lower_expr(&pe.expression),
                        _ => self.lower_expr(op),
                    };
                    args.push(lowered);
                }
            }
        }

        IrExpr::AllocPrint { fmt, args }
    }

    /// Recursively collect all operands in a string concatenation chain.
    /// Only recurses into BinaryExpression(+); other nodes become leaves.
    pub(super) fn collect_concat_from_be<'a>(
        be: &'a BinaryExpression<'a>,
        out: &mut Vec<&'a Expression<'a>>,
    ) {
        // Left side
        if let Expression::BinaryExpression(ref left_be) = be.left {
            if left_be.operator == BinaryOperator::Addition {
                Self::collect_concat_from_be(left_be, out);
            } else {
                out.push(&be.left);
            }
        } else {
            out.push(&be.left);
        }

        // Right side
        if let Expression::BinaryExpression(ref right_be) = be.right {
            if right_be.operator == BinaryOperator::Addition {
                Self::collect_concat_from_be(right_be, out);
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
            return IrExpr::CompileError {
                span: self.span_to_source_span(be.span),
                msg: "instanceof: right operand must be an identifier (constructor name)"
                    .to_string(),
            };
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
        IrExpr::BuiltinCall(IrBuiltinCall {
            module: BuiltinModule::JsRuntime,
            method: "instanceOf".to_string(),
            obj_name: None,
            obj_expr: Some(Box::new(left_expr)),
            args: vec![IrExpr::StringLiteral(type_name)],
            return_type: ZigType::Bool,
            regex_info: None,
            ta_type_suffix: None,
        })
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
            // Numeric/Boolean primitives: NOT instanceof their wrapper types
            ZigType::I64 | ZigType::F64 => {
                if type_name == "Number" {
                    return Some(IrExpr::BoolLiteral(false)); // primitives aren't objects
                }
                Some(IrExpr::BoolLiteral(false))
            }
            ZigType::Bool => {
                if type_name == "Boolean" {
                    return Some(IrExpr::BoolLiteral(false)); // primitives aren't objects
                }
                Some(IrExpr::BoolLiteral(false))
            }
            // For JsAny / Anytype, we can't resolve at compile time
            ZigType::JsAny | ZigType::Anytype => None,
            // Other types: conservatively say we can't resolve
            _ => None,
        }
    }
}
