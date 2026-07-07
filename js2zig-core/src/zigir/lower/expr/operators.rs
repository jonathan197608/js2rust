// zigir/lower/expr/operators.rs
// Binary, unary, update, assignment expression lowering + string concatenation.

use std::collections::HashSet;

use oxc_ast::ast::*;

use crate::types::ZigType;
use crate::zigir::builtins::BuiltinModule;
use crate::zigir::ident::IrIdent;
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
                // Special case: `e instanceof SomeError` → compare e.name
                // with the error constructor name at runtime.
                if let Expression::Identifier(ident) = &be.right {
                    let name = ident.name.as_str();
                    if matches!(
                        name,
                        "Error"
                            | "URIError"
                            | "TypeError"
                            | "RangeError"
                            | "SyntaxError"
                            | "ReferenceError"
                            | "EvalError"
                    ) {
                        let left_expr = self.lower_expr(&be.left);
                        return IrExpr::Binary {
                            op: BinOp::Eq,
                            left: Box::new(IrExpr::FieldAccess {
                                object: Box::new(left_expr),
                                field: "name".to_string(),
                                field_kind: crate::zigir::kinds::FieldKind::StructField,
                            }),
                            right: Box::new(IrExpr::StringLiteral(name.to_string())),
                            left_type: Some(ZigType::Str),
                            right_type: Some(ZigType::Str),
                        };
                    }
                }
                return IrExpr::CompileError {
                    span: self.span_to_source_span(be.span),
                    msg: "instanceof operator is not supported in Zig".to_string(),
                };
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
        let op = if ue.operator == UpdateOperator::Increment {
            UpdateOp::Increment
        } else {
            UpdateOp::Decrement
        };
        let target = Box::new(self.lower_simple_assign_target(&ue.argument));
        crate::zigir::types::IrExpr::Update {
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
            // Read target as expression for the base
            let base_ident = match &target {
                crate::zigir::types::IrAssignTarget::Ident(name) => IrExpr::Ident(name.clone()),
                _ => IrExpr::Ident(IrIdent::new("__target")),
            };
            // BigInt **= : use IrExpr::Binary with BinOp::Pow so emit_bigint_binary
            // generates .pow() method call.
            if target_type == Some(ZigType::BigInt) {
                let name_clone = match &target {
                    crate::zigir::types::IrAssignTarget::Ident(name) => name.clone(),
                    _ => IrIdent::new("__target"),
                };
                return IrExpr::Assign {
                    op: AssignOp::Assign,
                    target: Box::new(target),
                    value: Box::new(IrExpr::Binary {
                        op: BinOp::Pow,
                        left: Box::new(IrExpr::Ident(name_clone)),
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
                    base: Box::new(base_ident),
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
                // Only Ident targets are supported for now; other targets
                // (member, index) fall through to the default path.
                if let crate::zigir::types::IrAssignTarget::Ident(name) = &target {
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
                    let name_clone = name.clone();
                    return IrExpr::Assign {
                        op: AssignOp::Assign,
                        target: Box::new(target),
                        value: Box::new(IrExpr::Binary {
                            op: bin_op,
                            left: Box::new(IrExpr::Ident(name_clone)),
                            right: value,
                            left_type: Some(ZigType::BigInt),
                            right_type: Some(ZigType::BigInt),
                        }),
                    };
                }
                // For non-Ident BigInt targets, fall through to default path
                // (may produce invalid Zig but handles the common case first)
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

    /// Lower a simple assignment target (from UpdateExpression).
    /// SimpleAssignmentTarget can be an identifier or member expression.
    pub(super) fn lower_simple_assign_target(
        &mut self,
        target: &SimpleAssignmentTarget,
    ) -> crate::zigir::types::IrAssignTarget {
        match target {
            SimpleAssignmentTarget::AssignmentTargetIdentifier(id) => {
                let var_name = id.name.as_str();

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
                    };
                }

                crate::zigir::types::IrAssignTarget::Ident(IrIdent::new(var_name))
            }
            SimpleAssignmentTarget::StaticMemberExpression(mem) => {
                crate::zigir::types::IrAssignTarget::Member {
                    object: Box::new(self.lower_expr(&mem.object)),
                    field: mem.property.name.to_string(),
                    is_pointer: false,
                }
            }
            SimpleAssignmentTarget::ComputedMemberExpression(mem) => {
                crate::zigir::types::IrAssignTarget::Index {
                    object: Box::new(self.lower_expr(&mem.object)),
                    index: Box::new(self.lower_expr(&mem.expression)),
                }
            }
            _ => crate::zigir::types::IrAssignTarget::Ident(IrIdent::new("__unsupported_target")),
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
                let var_name = id.name.as_str();

                // Check if this identifier is a captured closure variable.
                // If so, rewrite the assignment target to self.xxx (value) or self.xxx.* (ref).
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
                    };
                }

                crate::zigir::types::IrAssignTarget::Ident(IrIdent::new(var_name))
            }
            AssignmentTarget::StaticMemberExpression(mem) => {
                crate::zigir::types::IrAssignTarget::Member {
                    object: Box::new(self.lower_expr(&mem.object)),
                    field: mem.property.name.to_string(),
                    is_pointer: false,
                }
            }
            AssignmentTarget::ComputedMemberExpression(mem) => {
                crate::zigir::types::IrAssignTarget::Index {
                    object: Box::new(self.lower_expr(&mem.object)),
                    index: Box::new(self.lower_expr(&mem.expression)),
                }
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
            _ => crate::zigir::types::IrAssignTarget::Ident(IrIdent::new("__unsupported_target")),
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
}
