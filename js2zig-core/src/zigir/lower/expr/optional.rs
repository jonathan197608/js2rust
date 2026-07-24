// zigir/lower/expr/optional.rs
// Optional chaining (?.) expression lowering.

use oxc_ast::ast::*;

use crate::types::ZigType;
use crate::zigir::ident::IrIdent;
use crate::zigir::kinds::{CallKind, ComputedKeyKind, FieldKind, IndexKind};

use super::Lowerer;

impl Lowerer {
    /// Lower an optional chain expression (`obj?.prop`, `obj?.method()`, `obj?.[key]`).
    pub(super) fn lower_optional_chain(
        &mut self,
        ce: &ChainExpression,
    ) -> crate::zigir::types::IrExpr {
        use crate::zigir::types::IrExpr;

        match &ce.expression {
            ChainElement::StaticMemberExpression(sme) => self.lower_optional_sme_chain(sme),
            ChainElement::ComputedMemberExpression(cme) => self.lower_optional_cme_chain(cme),
            ChainElement::CallExpression(call_ce) => self.lower_optional_call_chain(call_ce),
            _ => IrExpr::CompileError {
                span: crate::zigir::source_span::SourceSpan::default(),
                msg: "unsupported optional chain element".to_string(),
            },
        }
    }

    pub(super) fn lower_optional_sme_chain(
        &mut self,
        sme: &StaticMemberExpression,
    ) -> crate::zigir::types::IrExpr {
        use crate::zigir::types::IrExpr;

        let inner = self.lower_optional_chain_object(&sme.object);
        let field_name = sme.property.name.as_str();

        // When the object type is JsAny, use obj.get("prop") which returns
        // .undefined_value for null/undefined — exactly matching ?. semantics.
        // This avoids generating (if (obj) |oc| oc.prop else null) which
        // doesn't compile because JsAny is a union, not an optional.
        if self.expr_type_is_jsany(&sme.object) {
            return IrExpr::Call(crate::zigir::types::IrCallExpr {
                callee: Box::new(IrExpr::FieldAccess {
                    object: Box::new(inner),
                    field: "get".to_string(),
                    field_kind: FieldKind::StructField,
                }),
                args: vec![IrExpr::StringLiteral(field_name.to_string())],
                call_kind: CallKind::Method {
                    object_type: crate::zigir::kinds::MethodObjectKind::JsAny,
                },
            });
        }

        let capture_var = self.name_mangler.next_name("_oc");
        let needs_null_check = self.expr_might_be_null(&sme.object);
        let access_target = if needs_null_check {
            IrExpr::Ident(IrIdent::new(&capture_var))
        } else {
            inner.clone()
        };
        let body = IrExpr::FieldAccess {
            object: Box::new(access_target),
            field: field_name.to_string(),
            field_kind: FieldKind::StructField,
        };

        if !needs_null_check {
            return body;
        }

        IrExpr::OptionalChain {
            object: Box::new(inner),
            capture_var,
            body: Box::new(body),
            needs_null_check: true,
        }
    }

    pub(super) fn lower_optional_cme_chain(
        &mut self,
        cme: &ComputedMemberExpression,
    ) -> crate::zigir::types::IrExpr {
        use crate::zigir::types::IrExpr;

        let inner = self.lower_optional_chain_object(&cme.object);
        let index = self.lower_expr(&cme.expression);

        // When the object type is JsAny, use isNullish() + Conditional instead
        // of OptionalChain. OptionalChain emits (if (obj) |oc| ... else null)
        // which requires a Zig optional type — JsAny is a union, not an optional.
        if self.expr_type_is_jsany(&cme.object) {
            use crate::zigir::kinds::MethodObjectKind;
            use crate::zigir::types::{IrCallExpr, IrStmt, IrVarDecl};

            let temp_name = self.name_mangler.next_name("_oc");
            let block_label = format!("_oc_blk_{}", self.name_mangler.peek_count("_oc"));

            let temp_ident = |n: &str| IrExpr::Ident(IrIdent::new(n));

            return IrExpr::BlockExpr {
                label: block_label,
                body: vec![IrStmt::VarDecl(IrVarDecl::new_const(
                    &temp_name,
                    Some(ZigType::JsAny),
                    Some(inner),
                ))],
                result: Box::new(IrExpr::Conditional {
                    cond: Box::new(IrExpr::Call(IrCallExpr {
                        callee: Box::new(IrExpr::FieldAccess {
                            object: Box::new(temp_ident(&temp_name)),
                            field: "isNullish".to_string(),
                            field_kind: FieldKind::StructField,
                        }),
                        args: vec![],
                        call_kind: CallKind::Method {
                            object_type: MethodObjectKind::JsAny,
                        },
                    })),
                    then: Box::new(IrExpr::Undefined),
                    else_: Box::new(IrExpr::ComputedField {
                        object: Box::new(temp_ident(&temp_name)),
                        key: Box::new(index),
                        key_kind: ComputedKeyKind::JsAnyGetByKey,
                    }),
                }),
            };
        }

        let capture_var = self.name_mangler.next_name("_oc");
        let needs_null_check = self.expr_might_be_null(&cme.object);
        let access_target = if needs_null_check {
            IrExpr::Ident(IrIdent::new(&capture_var))
        } else {
            inner.clone()
        };
        let body = IrExpr::IndexAccess {
            object: Box::new(access_target),
            index: Box::new(index),
            index_kind: IndexKind::SliceIndex,
        };

        if !needs_null_check {
            return body;
        }

        IrExpr::OptionalChain {
            object: Box::new(inner),
            capture_var,
            body: Box::new(body),
            needs_null_check: true,
        }
    }

    pub(super) fn lower_optional_call_chain(
        &mut self,
        call_ce: &CallExpression,
    ) -> crate::zigir::types::IrExpr {
        use crate::zigir::types::IrExpr;

        let (receiver_name, method_name) = self.extract_optional_call_info(call_ce);
        let needs_null_check = receiver_name
            .as_ref()
            .map(|name| {
                self.get_var_type(name.as_str()).is_none_or(|ty| {
                    !matches!(
                        ty,
                        ZigType::Struct(_)
                            | ZigType::I64
                            | ZigType::F64
                            | ZigType::Bool
                            | ZigType::Str
                            | ZigType::ArrayList(_)
                            | ZigType::NamedStruct(_)
                    )
                })
            })
            .unwrap_or(true);

        let object = match &call_ce.callee {
            Expression::StaticMemberExpression(sme) => {
                self.lower_optional_chain_object(&sme.object)
            }
            Expression::ComputedMemberExpression(cme) => {
                self.lower_optional_chain_object(&cme.object)
            }
            _ => self.lower_expr(call_ce.callee.get_inner_expression()),
        };

        // When the receiver is JsAny, use isNullish() + Conditional instead of
        // OptionalChain. OptionalChain emits (if (obj) |oc| ... else null) which
        // requires a Zig optional type — JsAny is a union, not an optional, so
        // the unwrap syntax does not compile. Instead we bind the receiver to a
        // temp, check .isNullish(), and return undefined_value or the method call.
        let receiver_expr: Option<&Expression> = match &call_ce.callee {
            Expression::StaticMemberExpression(sme) => Some(&sme.object),
            Expression::ComputedMemberExpression(cme) => Some(&cme.object),
            _ => None,
        };
        if needs_null_check
            && let Some(method) = method_name.as_ref()
            && receiver_expr.is_some_and(|e| self.expr_type_is_jsany(e))
        {
            use crate::zigir::kinds::MethodObjectKind;
            use crate::zigir::types::{IrCallExpr, IrStmt, IrVarDecl};

            let temp_name = self.name_mangler.next_name("_oc");
            let block_label = format!("_oc_blk_{}", self.name_mangler.peek_count("_oc"));
            let args = self.lower_args(&call_ce.arguments);

            let temp_ident = |n: &str| IrExpr::Ident(IrIdent::new(n));

            return IrExpr::BlockExpr {
                label: block_label,
                body: vec![IrStmt::VarDecl(IrVarDecl::new_const(
                    &temp_name,
                    Some(ZigType::JsAny),
                    Some(object),
                ))],
                result: Box::new(IrExpr::Conditional {
                    cond: Box::new(IrExpr::Call(IrCallExpr {
                        callee: Box::new(IrExpr::FieldAccess {
                            object: Box::new(temp_ident(&temp_name)),
                            field: "isNullish".to_string(),
                            field_kind: FieldKind::StructField,
                        }),
                        args: vec![],
                        call_kind: CallKind::Method {
                            object_type: MethodObjectKind::JsAny,
                        },
                    })),
                    then: Box::new(IrExpr::Undefined),
                    else_: Box::new(IrExpr::Call(IrCallExpr {
                        callee: Box::new(IrExpr::FieldAccess {
                            object: Box::new(temp_ident(&temp_name)),
                            field: method.clone(),
                            field_kind: FieldKind::StructField,
                        }),
                        args,
                        call_kind: CallKind::Method {
                            object_type: MethodObjectKind::JsAny,
                        },
                    })),
                }),
            };
        }

        // JsAny CME call: obj?.["method"]() — method_name is None for CME.
        // Use isNullish() + Conditional + ComputedField(JsAnyGetByKey) to
        // avoid OptionalChain which requires a Zig optional type.
        if needs_null_check
            && method_name.is_none()
            && receiver_expr.is_some_and(|e| self.expr_type_is_jsany(e))
        {
            use crate::zigir::kinds::MethodObjectKind;
            use crate::zigir::types::{IrCallExpr, IrStmt, IrVarDecl};

            let temp_name = self.name_mangler.next_name("_oc");
            let block_label = format!("_oc_blk_{}", self.name_mangler.peek_count("_oc"));
            let args = self.lower_args(&call_ce.arguments);

            let temp_ident = |n: &str| IrExpr::Ident(IrIdent::new(n));

            // Extract the computed key from the CME callee
            let key_expr = match &call_ce.callee {
                Expression::ComputedMemberExpression(cme) => self.lower_expr(&cme.expression),
                _ => IrExpr::Undefined,
            };

            return IrExpr::BlockExpr {
                label: block_label,
                body: vec![IrStmt::VarDecl(IrVarDecl::new_const(
                    &temp_name,
                    Some(ZigType::JsAny),
                    Some(object),
                ))],
                result: Box::new(IrExpr::Conditional {
                    cond: Box::new(IrExpr::Call(IrCallExpr {
                        callee: Box::new(IrExpr::FieldAccess {
                            object: Box::new(temp_ident(&temp_name)),
                            field: "isNullish".to_string(),
                            field_kind: FieldKind::StructField,
                        }),
                        args: vec![],
                        call_kind: CallKind::Method {
                            object_type: MethodObjectKind::JsAny,
                        },
                    })),
                    then: Box::new(IrExpr::Undefined),
                    else_: Box::new(IrExpr::Call(IrCallExpr {
                        callee: Box::new(IrExpr::ComputedField {
                            object: Box::new(temp_ident(&temp_name)),
                            key: Box::new(key_expr),
                            key_kind: ComputedKeyKind::JsAnyGetByKey,
                        }),
                        args,
                        call_kind: CallKind::Method {
                            object_type: MethodObjectKind::JsAny,
                        },
                    })),
                }),
            };
        }

        let capture_var = self.name_mangler.next_name("_oc");
        let access_target = if needs_null_check {
            IrExpr::Ident(IrIdent::new(&capture_var))
        } else {
            object.clone()
        };

        let args = self.lower_args(&call_ce.arguments);

        let body = if let Some(name) = method_name {
            IrExpr::Call(crate::zigir::types::IrCallExpr {
                callee: Box::new(IrExpr::FieldAccess {
                    object: Box::new(access_target),
                    field: name,
                    field_kind: FieldKind::StructField,
                }),
                args,
                call_kind: CallKind::Method {
                    object_type: crate::zigir::kinds::MethodObjectKind::Unknown,
                },
            })
        } else if let Expression::ComputedMemberExpression(cme) = &call_ce.callee {
            // CME call: obj?.[key]() — build the call using access_target
            // (already bound to capture_var or object clone) instead of
            // re-lowering the entire call from AST via self.lower_call(call_ce),
            // which would double-evaluate the object expression.
            let key_expr = self.lower_expr(&cme.expression);
            let key_kind = match self.infer_expr_type(&cme.object) {
                Some(ZigType::NamedStruct(ref n)) if n == "Map" => ComputedKeyKind::MapGet,
                Some(ZigType::ArrayList(_)) => ComputedKeyKind::ArrayListItem,
                Some(ZigType::Str) => ComputedKeyKind::StringCharAt,
                Some(ZigType::Struct(_) | ZigType::NamedStruct(_)) => ComputedKeyKind::StructField,
                _ => ComputedKeyKind::JsAnyGetByKey,
            };
            IrExpr::Call(crate::zigir::types::IrCallExpr {
                callee: Box::new(IrExpr::ComputedField {
                    object: Box::new(access_target),
                    key: Box::new(key_expr),
                    key_kind,
                }),
                args,
                call_kind: CallKind::Method {
                    object_type: crate::zigir::kinds::MethodObjectKind::Unknown,
                },
            })
        } else {
            // Fallback for non-member call expressions (e.g., foo?.()).
            // Build the call directly using pre-lowered access_target and
            // args, avoiding self.lower_call(call_ce) which would re-lower
            // the entire AST node and double-evaluate arguments.
            IrExpr::Call(crate::zigir::types::IrCallExpr {
                callee: Box::new(access_target),
                args,
                call_kind: CallKind::Direct,
            })
        };

        if !needs_null_check {
            return body;
        }

        IrExpr::OptionalChain {
            object: Box::new(object),
            capture_var,
            body: Box::new(body),
            needs_null_check: true,
        }
    }

    pub(super) fn lower_optional_chain_object(
        &mut self,
        expr: &Expression,
    ) -> crate::zigir::types::IrExpr {
        match expr {
            Expression::StaticMemberExpression(sme) if sme.optional => {
                self.lower_optional_sme_chain(sme)
            }
            Expression::ComputedMemberExpression(cme) if cme.optional => {
                self.lower_optional_cme_chain(cme)
            }
            Expression::ChainExpression(ce) => self.lower_optional_chain(ce),
            _ => self.lower_expr(expr),
        }
    }

    /// Extract receiver expression and method name from an optional call expression.
    /// For `obj?.greet("World")`, returns (Some(obj_ident_or_expr), Some("greet"), ...).
    /// Returns owned data since we need to release the borrow before calling lower_expr.
    pub(super) fn extract_optional_call_info(
        &self,
        ce: &CallExpression,
    ) -> (Option<String>, Option<String>) {
        match &ce.callee {
            Expression::StaticMemberExpression(sme) => {
                // Extract receiver identifier name if it's a simple identifier
                let receiver_name = match &sme.object {
                    Expression::Identifier(id) => Some(id.name.to_string()),
                    _ => None,
                };
                (receiver_name, Some(sme.property.name.to_string()))
            }
            Expression::ComputedMemberExpression(cme) => {
                let receiver_name = match &cme.object {
                    Expression::Identifier(id) => Some(id.name.to_string()),
                    _ => None,
                };
                (receiver_name, None)
            }
            _ => (None, None),
        }
    }

    /// Check if an expression might be null at runtime.
    /// Returns false for known non-null types (structs, i64, f64, bool, str, ArrayList, etc.)
    pub(super) fn expr_might_be_null(&self, expr: &Expression) -> bool {
        match expr {
            Expression::NumericLiteral(_)
            | Expression::StringLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::ArrayExpression(_)
            | Expression::ObjectExpression(_) => false,

            Expression::NullLiteral(_) => true,

            Expression::Identifier(id) => {
                let ty = self.get_var_type(id.name.as_str());
                match ty {
                    Some(
                        ZigType::Struct(_)
                        | ZigType::I64
                        | ZigType::F64
                        | ZigType::Bool
                        | ZigType::Str
                        | ZigType::ArrayList(_)
                        | ZigType::NamedStruct(_),
                    ) => false,
                    // Unknown type: conservatively assume it might be null
                    // so that optional chaining null checks are not skipped.
                    None => true,
                    _ => true, // JsAny, Anytype, Void → might be null
                }
            }

            // ChainExpression result is always nullable (the else branch is null)
            Expression::ChainExpression(_) => true,

            // Call expressions might return null
            Expression::CallExpression(_) => true,

            // Member access: check recursively
            Expression::StaticMemberExpression(sme) => self.expr_might_be_null(&sme.object),
            Expression::ComputedMemberExpression(cme) => self.expr_might_be_null(&cme.object),

            // Conservative: assume everything else might be null
            _ => true,
        }
    }

    /// Check if an expression's inferred type is JsAny.
    /// Used to decide whether optional chaining should use obj.get() instead
    /// of (if (obj) |oc| oc.prop else null), since JsAny is a union not an optional.
    pub(super) fn expr_type_is_jsany(&self, expr: &Expression) -> bool {
        match expr {
            Expression::NullLiteral(_) => true,
            Expression::Identifier(id) => {
                matches!(self.get_var_type(id.name.as_str()), Some(ZigType::JsAny))
            }
            Expression::CallExpression(_) => {
                // Many runtime calls return JsAny; be conservative
                true
            }
            Expression::ParenthesizedExpression(pe) => {
                // Unwrap parentheses and check the inner expression
                self.expr_type_is_jsany(&pe.expression)
            }
            _ => {
                // Fallback: use type inference to check if the expression is JsAny.
                // This covers assignment, conditional, logical, member access, and
                // other expression types whose inferred type could be JsAny.
                self.infer_expr_type(expr) == Some(ZigType::JsAny)
            }
        }
    }

    /// Get the type of a variable, checking fn_local_types (per-function) first,
    /// then falling back to global var_types. This fixes the scoping issue where
    /// var_names from different functions collide in the flat var_types map.
    fn get_var_type(&self, name: &str) -> Option<ZigType> {
        // Per-function local types take priority
        if let Some(ty) = self
            .fn_ctx
            .as_ref()
            .and_then(|ctx| ctx.fn_local_types.get(name))
        {
            return Some(ty.clone());
        }
        // Fall back to global var_types
        self.type_info.var_types.get(name).cloned()
    }
}
