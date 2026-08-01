// zigir/lower/expr/container.rs
// Array, object, template literal lowering.

use oxc_ast::ast::*;

use super::Lowerer;
use crate::types::ZigType;
use crate::zigir::builtins::BuiltinModule;
use crate::zigir::lower::helpers;
use crate::zigir::source_span::SourceSpan;
use crate::zigir::types::{IrBuiltinCall, IrExpr};

impl Lowerer {
    /// Lower an array expression.
    pub(super) fn lower_array_expr(&mut self, ae: &ArrayExpression) -> crate::zigir::types::IrExpr {
        let mut elements = Vec::new();
        let mut spread_indices = Vec::new();

        for (i, elem) in ae.elements.iter().enumerate() {
            match elem {
                ArrayExpressionElement::SpreadElement(se) => {
                    spread_indices.push(i);
                    // Detect if the spread source is a Set or Map identifier.
                    // Set/Map don't have `.items` (they use a HashMap internally),
                    // so we convert the spread to a keys()/entries() BuiltinCall
                    // which returns ArrayList(JsAny) — compatible with the emit
                    // layer's `.items` iteration.
                    let spread_inner = if let Expression::Identifier(id) = &se.argument {
                        let var_name = id.name.as_str();
                        match self.infer_ident_type(var_name) {
                            Some(ZigType::NamedStruct(n)) if n == "Set" => {
                                let zig_name = self.make_ident(var_name).zig_name;
                                IrExpr::BuiltinCall(IrBuiltinCall::simple(
                                    BuiltinModule::JsCollections,
                                    "keys",
                                    Some(zig_name),
                                    None,
                                    vec![],
                                    ZigType::ArrayList(Box::new(ZigType::JsAny)),
                                ))
                            }
                            Some(ZigType::NamedStruct(n)) if n == "Map" => {
                                let zig_name = self.make_ident(var_name).zig_name;
                                IrExpr::BuiltinCall(IrBuiltinCall::simple(
                                    BuiltinModule::JsCollections,
                                    "entries",
                                    Some(zig_name),
                                    None,
                                    vec![],
                                    ZigType::ArrayList(Box::new(ZigType::JsAny)),
                                ))
                            }
                            _ => self.lower_expr(&se.argument),
                        }
                    } else {
                        self.lower_expr(&se.argument)
                    };
                    elements.push(IrExpr::Spread(Box::new(spread_inner)));
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
        // Determine the Zig format specifier for each interpolation expression.
        // This must match the Emitter's logic:
        //   Str→{s}, I64/F64→{d}, Bool→{}, other→expr_is_string?{s}:{}
        // R39-LEX-3: BigInt interpolation must be wrapped in toString() to
        // avoid the trailing "n" that JsBigInt.format() appends.
        let mut exprs: Vec<crate::zigir::types::IrExpr> = Vec::with_capacity(tl.expressions.len());
        let mut format_specs: Vec<String> = Vec::with_capacity(tl.expressions.len());
        for expr in tl.expressions.iter() {
            let lowered = self.lower_expr(expr);
            if self.expr_is_string(expr) {
                format_specs.push("{s}".to_string());
                exprs.push(lowered);
            } else {
                match self.infer_expr_type(expr) {
                    Some(ZigType::BigInt) => {
                        // Wrap BigInt in toString() to avoid trailing "n"
                        exprs.push(crate::zigir::types::IrExpr::BuiltinCall(
                            crate::zigir::types::IrBuiltinCall::simple(
                                BuiltinModule::JsBigInt,
                                "toString",
                                None,
                                Some(Box::new(lowered)),
                                vec![],
                                ZigType::Str,
                            ),
                        ));
                        format_specs.push("{s}".to_string());
                    }
                    Some(ty) => {
                        format_specs.push(helpers::format_specifier_for_type(&ty).to_string());
                        exprs.push(lowered);
                    }
                    None => {
                        format_specs.push("{any}".to_string());
                        exprs.push(lowered);
                    }
                }
            }
        }

        crate::zigir::types::IrExpr::TemplateLiteral {
            parts,
            exprs,
            format_specs,
        }
    }
}
