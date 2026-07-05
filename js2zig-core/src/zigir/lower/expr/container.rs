// zigir/lower/expr/container.rs
// Array, object, template literal lowering.

use oxc_ast::ast::*;

use crate::types::ZigType;

use super::Lowerer;

impl Lowerer {
    /// Lower an array expression.
    pub(super) fn lower_array_expr(&mut self, ae: &ArrayExpression) -> crate::zigir::types::IrExpr {
        let mut elements = Vec::new();
        let mut spread_indices = Vec::new();

        for (i, elem) in ae.elements.iter().enumerate() {
            match elem {
                ArrayExpressionElement::SpreadElement(se) => {
                    spread_indices.push(i);
                    elements.push(crate::zigir::types::IrExpr::Spread(Box::new(
                        self.lower_expr(&se.argument),
                    )));
                }
                ArrayExpressionElement::Elision(_) => {
                    elements.push(crate::zigir::types::IrExpr::Null);
                }
                _ => {
                    if let Some(expr) = elem.as_expression() {
                        elements.push(self.lower_expr(expr));
                    }
                }
            }
        }

        crate::zigir::types::IrExpr::ArrayLiteral(crate::zigir::types::IrArrayLiteral {
            elements,
            spread_indices,
        })
    }

    /// Lower an object expression.
    pub(super) fn lower_object_expr(
        &mut self,
        oe: &ObjectExpression,
    ) -> crate::zigir::types::IrExpr {
        use crate::zigir::types::IrObjectItem;

        let mut items = Vec::new();

        for prop in oe.properties.iter() {
            match prop {
                ObjectPropertyKind::ObjectProperty(op) => {
                    let (key, is_computed) = match &op.key {
                        PropertyKey::StaticIdentifier(id) => (id.name.to_string(), false),
                        PropertyKey::StringLiteral(sl) => (sl.value.to_string(), false),
                        PropertyKey::NumericLiteral(nl) => (nl.value.to_string(), false),
                        _ => ("__computed__".to_string(), true),
                    };

                    match op.kind {
                        PropertyKind::Init => {
                            let value = self.lower_expr(&op.value);
                            items.push(IrObjectItem::Field(crate::zigir::types::IrObjectField {
                                key,
                                value,
                                is_computed,
                            }));
                        }
                        PropertyKind::Get => {
                            // Getter: extract return expression from function body
                            // { get x() { return expr; } } → .x = expr
                            if let Expression::FunctionExpression(func) = &op.value
                                && let Some(body) = &func.body
                                && let Some(return_expr) = Self::extract_return_expr_from_body(body)
                            {
                                let value = self.lower_expr(return_expr);
                                items.push(IrObjectItem::Field(
                                    crate::zigir::types::IrObjectField {
                                        key,
                                        value,
                                        is_computed,
                                    },
                                ));
                            }
                            // If getter body is more complex, skip it (can't inline)
                        }
                        PropertyKind::Set => {
                            // Setter: skip entirely, doesn't contribute a field
                        }
                    }
                }
                ObjectPropertyKind::SpreadProperty(sp) => {
                    items.push(IrObjectItem::Spread(self.lower_expr(&sp.argument)));
                }
            }
        }

        crate::zigir::types::IrExpr::ObjectLiteral(crate::zigir::types::IrObjectLiteral { items })
    }

    /// Extract the return expression from a function body with a single return statement.
    /// Used by getter property lowering: `{ get x() { return expr; } }` → `.x = expr`.
    pub(super) fn extract_return_expr_from_body<'a>(
        body: &'a FunctionBody<'a>,
    ) -> Option<&'a Expression<'a>> {
        if body.statements.len() == 1
            && let Statement::ReturnStatement(ret) = &body.statements[0]
        {
            return ret.argument.as_ref();
        }
        None
    }

    /// Lower a template literal.
    pub(super) fn lower_template_literal(
        &mut self,
        tl: &TemplateLiteral,
    ) -> crate::zigir::types::IrExpr {
        let parts: Vec<String> = tl.quasis.iter().map(|q| q.value.raw.to_string()).collect();
        let exprs: Vec<crate::zigir::types::IrExpr> =
            tl.expressions.iter().map(|e| self.lower_expr(e)).collect();

        // Determine the Zig format specifier for each interpolation expression.
        // This must match the Emitter's logic:
        //   Str→{s}, I64/F64→{d}, Bool→{}, other→expr_is_string?{s}:{}
        let format_specs: Vec<String> = tl
            .expressions
            .iter()
            .map(|expr| match self.infer_expr_type(expr) {
                Some(ZigType::Str) => "{s}".to_string(),
                Some(ZigType::I64) | Some(ZigType::F64) => "{d}".to_string(),
                Some(ZigType::Bool) => "{}".to_string(),
                _ => {
                    if self.expr_is_string(expr) {
                        "{s}".to_string()
                    } else {
                        "{}".to_string()
                    }
                }
            })
            .collect();

        crate::zigir::types::IrExpr::TemplateLiteral {
            parts,
            exprs,
            format_specs,
        }
    }
}
