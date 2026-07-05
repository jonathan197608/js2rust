// zigir/lower/expr/optional.rs
// Optional chaining (?.) expression lowering.

use oxc_ast::ast::*;

use crate::types::ZigType;
use crate::zigir::ident::IrIdent;
use crate::zigir::kinds::{CallKind, FieldKind, IndexKind};

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

        let capture_var = self.name_mangler.next_name("_oc");
        let needs_null_check = self.expr_might_be_null(&sme.object);
        let access_target = if needs_null_check {
            IrExpr::Ident(IrIdent::new(&capture_var))
        } else {
            inner.clone()
        };
        let body = IrExpr::FieldAccess {
            object: Box::new(access_target),
            field: sme.property.name.to_string(),
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
                self.type_info
                    .var_types
                    .get(name.as_str())
                    .is_none_or(|ty| {
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

        let capture_var = self.name_mangler.next_name("_oc");
        let access_target = if needs_null_check {
            IrExpr::Ident(IrIdent::new(&capture_var))
        } else {
            object.clone()
        };

        let args: Vec<IrExpr> = call_ce
            .arguments
            .iter()
            .map(|arg| {
                let expr = arg.as_expression().unwrap();
                self.lower_expr(expr)
            })
            .collect();

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
        } else {
            self.lower_call(call_ce)
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
                let ty = self.type_info.var_types.get(id.name.as_str());
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
                    _ => true, // Void, JsAny, Anytype, unknown → might be null
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
}
