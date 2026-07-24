// zigir/lower/expr/container.rs
// Array, object, template literal lowering.

use oxc_ast::ast::*;

use super::Lowerer;
use crate::zigir::lower::helpers;
use crate::zigir::source_span::SourceSpan;

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
                    // JS spec: array holes are `undefined`, NOT `null`.
                    // `[1, , 3]` has holes at index 1 (sparse array behavior).
                    elements.push(crate::zigir::types::IrExpr::Undefined);
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
                            // { get x() { return; } } → .x = undefined
                            // Only single-return getters are inlined; complex ones get @compileError
                            if let Expression::FunctionExpression(func) = &op.value
                                && let Some(body) = &func.body
                                && body.statements.len() == 1
                                && let Statement::ReturnStatement(ret) = &body.statements[0]
                            {
                                let value = match &ret.argument {
                                    Some(return_expr) => self.lower_expr(return_expr),
                                    None => crate::zigir::types::IrExpr::Undefined,
                                };
                                items.push(IrObjectItem::Field(
                                    crate::zigir::types::IrObjectField {
                                        key,
                                        value,
                                        is_computed,
                                    },
                                ));
                            } else {
                                // Complex getter (multiple statements) — @compileError
                                let span = self.span_to_source_span(op.span);
                                self.add_error(span, "getter with complex body is not supported (only single-return getters are inlined)");
                                items.push(IrObjectItem::Field(
                                    crate::zigir::types::IrObjectField {
                                        key,
                                        value: crate::zigir::types::IrExpr::CompileError {
                                            span: SourceSpan::default(),
                                            msg: "complex getter not supported".to_string(),
                                        },
                                        is_computed,
                                    },
                                ));
                            }
                        }
                        PropertyKind::Set => {
                            // Setter: @compileError — Zig structs don't support setters
                            let span = self.span_to_source_span(op.span);
                            self.add_error(
                                span,
                                "setter property is not supported (Zig structs have no setters)",
                            );
                            items.push(IrObjectItem::Field(crate::zigir::types::IrObjectField {
                                key,
                                value: crate::zigir::types::IrExpr::CompileError {
                                    span: SourceSpan::default(),
                                    msg: "setter not supported".to_string(),
                                },
                                is_computed,
                            }));
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

    /// Lower a template literal.
    pub(super) fn lower_template_literal(
        &mut self,
        tl: &TemplateLiteral,
    ) -> crate::zigir::types::IrExpr {
        // Use the COOKED value (post-escape-interpretation), not `raw`.
        // `raw` is the literal source text (e.g. `\\n` as two bytes 0x5C 0x6E);
        // `cooked` is the interpreted string (a real 0x0A newline byte). When
        // the emitter wraps a quasi in a Zig string literal via
        // `escape_zig_format_string`, the cooked value's real control bytes
        // get re-escaped to Zig escapes (e.g. `\n`) which Zig then correctly
        // interprets at runtime. Using `raw` instead produced double-escaped
        // output ("hello\\nworld" → runtime "hello\nworld" with literal
        // backslash-n). Note: `cooked` is `None` for invalid escape sequences
        // (e.g. `\u` not followed by valid hex); fall back to `raw` only then
        // (R6-3).
        let parts: Vec<String> = tl
            .quasis
            .iter()
            .map(|q| {
                q.value
                    .cooked
                    .as_ref()
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| q.value.raw.to_string())
            })
            .collect();
        let exprs: Vec<crate::zigir::types::IrExpr> =
            tl.expressions.iter().map(|e| self.lower_expr(e)).collect();

        // Determine the Zig format specifier for each interpolation expression.
        // This must match the Emitter's logic:
        //   Str→{s}, I64/F64→{d}, Bool→{}, other→expr_is_string?{s}:{}
        let format_specs: Vec<String> = tl
            .expressions
            .iter()
            .map(|expr| {
                if self.expr_is_string(expr) {
                    "{s}".to_string()
                } else {
                    match self.infer_expr_type(expr) {
                        Some(ty) => helpers::format_specifier_for_type(&ty).to_string(),
                        None => "{any}".to_string(),
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
