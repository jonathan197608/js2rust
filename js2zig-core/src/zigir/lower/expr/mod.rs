// zigir/lower/expr/mod.rs
// Expression lowering: main dispatch + identifier handling.

use oxc_ast::ast::*;

use crate::zigir::ident::IrIdent;
use crate::zigir::kinds::FieldKind;
use crate::zigir::ops::LogicalOp;
use crate::zigir::source_span::SourceSpan;

use super::Lowerer;

pub mod call;
pub mod container;
pub mod function;
pub mod idents;
pub mod member;
pub mod operators;
pub mod optional;

impl Lowerer {
    pub(crate) fn lower_expr(&mut self, expr: &Expression) -> crate::zigir::types::IrExpr {
        use crate::zigir::types::IrExpr;

        match expr {
            // ── Literals ──────────────────────────────────────────────────────
            Expression::NumericLiteral(n) => {
                // Zig considers `-0` ambiguous; emit `-0.0` explicitly for negative zero
                if n.value == -0.0 && n.value.is_sign_negative() {
                    IrExpr::FloatLiteral(-0.0)
                } else if n.value.fract() == 0.0 && n.value.abs() < i64::MAX as f64 {
                    IrExpr::IntLiteral(n.value as i64)
                } else {
                    IrExpr::FloatLiteral(n.value)
                }
            }
            Expression::StringLiteral(s) => IrExpr::StringLiteral(s.value.to_string()),
            Expression::BooleanLiteral(b) => IrExpr::BoolLiteral(b.value),
            Expression::NullLiteral(_) => IrExpr::Null,
            Expression::RegExpLiteral(rl) => {
                // JS regexp literal `/pattern/flags` → new RegExp(pattern, flags)
                let pattern = rl.regex.pattern.text.as_str();
                let escaped = pattern.replace('\\', "\\\\").replace('"', "\\\"");
                let flags = rl.regex.flags.to_string();
                IrExpr::New(crate::zigir::types::IrNewExpr {
                    constructor: crate::zigir::kinds::NewConstructor::RegExp,
                    args: vec![IrExpr::StringLiteral(escaped), IrExpr::StringLiteral(flags)],
                    result_type: crate::types::ZigType::JsAny,
                })
            }
            Expression::BigIntLiteral(bi) => {
                // BigInt literal: store the decimal value string (without trailing 'n')
                let s = bi.value.as_str().to_string();
                IrExpr::BigIntLiteral(s)
            }

            // ── Identifier ────────────────────────────────────────────────────
            Expression::Identifier(id) => self.lower_ident_expr(id),

            // ── This ──────────────────────────────────────────────────────────
            Expression::ThisExpression(te) => {
                if self.in_static_block {
                    // In a static block, `this` refers to the class constructor itself.
                    // Replace with the class name identifier so that `this.field`
                    // is correctly routed to `__ClassName_field` via the existing
                    // static field detection logic.
                    if let Some(ref class_name) = self.current_class {
                        IrExpr::Ident(IrIdent::new(class_name))
                    } else {
                        let span = self.span_to_source_span(te.span);
                        self.add_error(span, "`this` used in static block without class context");
                        IrExpr::CompileError {
                            span: SourceSpan::default(),
                            msg: "`this` used in static block without class context".to_string(),
                        }
                    }
                } else if self.current_class.is_some() {
                    IrExpr::This
                } else {
                    let span = self.span_to_source_span(te.span);
                    self.add_error(span, "`this` used outside of a class method");
                    IrExpr::CompileError {
                        span: SourceSpan::default(),
                        msg: "`this` used outside of a class method".to_string(),
                    }
                }
            }

            // ── Binary expression ─────────────────────────────────────────────
            Expression::BinaryExpression(be) => self.lower_binary(be),

            // ── Logical expression ────────────────────────────────────────────
            Expression::LogicalExpression(le) => {
                let op = match le.operator {
                    LogicalOperator::And => LogicalOp::And,
                    LogicalOperator::Or => LogicalOp::Or,
                    LogicalOperator::Coalesce => LogicalOp::Nullish,
                };
                let left_type = self
                    .infer_expr_type(&le.left)
                    .unwrap_or(crate::types::ZigType::JsAny);
                let right_type = self
                    .infer_expr_type(&le.right)
                    .unwrap_or(crate::types::ZigType::JsAny);
                IrExpr::Logical {
                    op,
                    left: Box::new(self.lower_expr(&le.left)),
                    right: Box::new(self.lower_expr(&le.right)),
                    left_type: Some(left_type),
                    right_type: Some(right_type),
                }
            }

            // ── Unary expression ──────────────────────────────────────────────
            Expression::UnaryExpression(ue) => self.lower_unary(ue),

            // ── Update expression ─────────────────────────────────────────────
            Expression::UpdateExpression(ue) => self.lower_update(ue),

            // ── Assignment expression ─────────────────────────────────────────
            Expression::AssignmentExpression(ae) => self.lower_assignment(ae),

            // ── Parenthesized ─────────────────────────────────────────────────
            Expression::ParenthesizedExpression(pe) => {
                IrExpr::Paren(Box::new(self.lower_expr(&pe.expression)))
            }

            // ── Conditional ──────────────────────────────────────────────────
            Expression::ConditionalExpression(ce) => IrExpr::Conditional {
                cond: Box::new(self.lower_expr(&ce.test)),
                then: Box::new(self.lower_expr(&ce.consequent)),
                else_: Box::new(self.lower_expr(&ce.alternate)),
            },

            // ── Sequence expression ───────────────────────────────────────────
            Expression::SequenceExpression(se) => {
                let exprs: Vec<IrExpr> =
                    se.expressions.iter().map(|e| self.lower_expr(e)).collect();
                IrExpr::Sequence(exprs)
            }

            // ── Calls ─────────────────────────────────────────────────────────
            Expression::CallExpression(ce) => self.lower_call(ce),
            Expression::NewExpression(ne) => self.lower_new(ne),

            // ── Member access ─────────────────────────────────────────────────
            Expression::StaticMemberExpression(mem) => {
                // Handle optional member access (?.) via the optional chain path
                if mem.optional {
                    return self.lower_optional_sme_chain(mem);
                }
                self.lower_static_member(mem)
            }
            Expression::ComputedMemberExpression(mem) => {
                if mem.optional {
                    return self.lower_optional_cme_chain(mem);
                }
                self.lower_computed_member(mem)
            }

            // ── Array / Object literals ──────────────────────────────────────
            Expression::ArrayExpression(ae) => self.lower_array_expr(ae),
            Expression::ObjectExpression(oe) => self.lower_object_expr(oe),

            // ── Function expressions ─────────────────────────────────────────
            Expression::ArrowFunctionExpression(af) => self.lower_arrow_fn(af),
            Expression::FunctionExpression(fe) => self.lower_fn_expr(fe),

            // ── Template literal ──────────────────────────────────────────────
            Expression::TemplateLiteral(tl) => self.lower_template_literal(tl),

            // ── Tagged template ──────────────────────────────────────────────
            Expression::TaggedTemplateExpression(tte) => {
                let span = self.span_to_source_span(tte.span);
                self.add_error(span, "Tagged template literals are not supported");
                IrExpr::CompileError {
                    span: SourceSpan::default(),
                    msg: "Tagged template literals not supported".to_string(),
                }
            }

            // ── Await ─────────────────────────────────────────────────────────
            Expression::AwaitExpression(ae) => self.lower_await(ae),

            // ── typeof / void / delete ────────────────────────────────────────
            // (handled via UnaryExpression, but also here as fallback)

            // ── Yield ─────────────────────────────────────────────────────────
            Expression::YieldExpression(ye) => {
                let span = self.span_to_source_span(ye.span);
                self.add_error(span, "Yield expressions are not supported");
                IrExpr::CompileError {
                    span: SourceSpan::default(),
                    msg: "yield not supported".to_string(),
                }
            }

            // ── MetaProperty (import.meta, new.target) ──────
            Expression::MetaProperty(mp) => {
                let meta_name = mp.meta.name.as_str();
                let prop_name = mp.property.name.as_str();
                if meta_name == "import" && prop_name == "meta" {
                    // import.meta → struct literal with .url field
                    let url = if self.source_name.is_empty() {
                        "import.meta.url".to_string()
                    } else {
                        self.source_name.clone()
                    };
                    IrExpr::ObjectLiteral(crate::zigir::types::IrObjectLiteral {
                        items: vec![crate::zigir::types::IrObjectItem::Field(
                            crate::zigir::types::IrObjectField {
                                key: "url".to_string(),
                                value: IrExpr::StringLiteral(url),
                                is_computed: false,
                            },
                        )],
                    })
                } else {
                    let span = self.span_to_source_span(mp.span);
                    self.add_error(
                        span,
                        format!("MetaProperty {}.{} is not supported", meta_name, prop_name),
                    );
                    IrExpr::CompileError {
                        span: SourceSpan::default(),
                        msg: format!("MetaProperty {}.{} not supported", meta_name, prop_name),
                    }
                }
            }

            // ── Super ────────────────────────────────────────────────────────
            Expression::Super(sup) => {
                let span = self.span_to_source_span(sup.span);
                self.add_error(span, "super is not supported");
                IrExpr::CompileError {
                    span: SourceSpan::default(),
                    msg: "super not supported".to_string(),
                }
            }

            // ── Import ────────────────────────────────────────────────────────
            Expression::ImportExpression(ie) => {
                let span = self.span_to_source_span(ie.span);
                self.add_error(span, "dynamic import() is not supported");
                IrExpr::CompileError {
                    span: SourceSpan::default(),
                    msg: "dynamic import() not supported".to_string(),
                }
            }

            // ── PrivateFieldAccess ────────────────────────────────────────────
            Expression::PrivateFieldExpression(pfe) => {
                // Private fields are lowered like normal member access
                // with a field_kind marker
                let object = Box::new(self.lower_expr(&pfe.object));
                let field = pfe.field.name.to_string();
                IrExpr::FieldAccess {
                    object,
                    field,
                    field_kind: FieldKind::Private,
                }
            }

            // ── Optional chaining (?.) ───────────────────────────────────────
            Expression::ChainExpression(ce) => self.lower_optional_chain(ce),

            // ── Class expression (anonymous or named class as value) ──
            // e.g. `const X = class {}` or `const X = class Y {}`
            // Lower same as ClassDeclaration, register as pending decl,
            // return an Ident referencing the class struct name.
            Expression::ClassExpression(ce) => {
                if let Some(ir_class) = self.lower_class_decl(ce) {
                    let class_name = ir_class.name.js_name.clone();
                    self.class_names.insert(class_name.clone());
                    // Register extends relationship for instanceof chain traversal
                    if let Some(ref parent) = ir_class.extends {
                        self.class_extends_map
                            .insert(class_name.clone(), parent.clone());
                    }
                    self.pending_expr_fns
                        .push(crate::zigir::types::IrDecl::Class(ir_class));
                    IrExpr::Ident(self.make_ident(&class_name))
                } else {
                    IrExpr::CompileError {
                        span: SourceSpan::default(),
                        msg: "class expression could not be lowered".to_string(),
                    }
                }
            }

            // ── Fallback ──────────────────────────────────────────────────────
            _ => IrExpr::CompileError {
                span: SourceSpan::default(),
                msg: format!("unsupported expression type: {expr:?}"),
            },
        }
    }

    /// Lower an identifier expression with special handling for
    /// built-in globals (NaN, Infinity, undefined, arguments)
    /// and captured closure variables.
    pub(super) fn lower_ident_expr(
        &mut self,
        id: &IdentifierReference,
    ) -> crate::zigir::types::IrExpr {
        use crate::zigir::types::IrExpr;

        let var_name = id.name.as_str();

        // arguments object: rewrite to rest param name
        // - Non-export with synthetic rest: `__arguments` (the rest param)
        // - Function with explicit rest: the rest param name (e.g., `args`)
        // - Export function (no rest): falls back to `__arguments` VarDecl
        if var_name == "arguments" {
            if let Some(ctx) = self.fn_ctx.as_ref()
                && let Some(rest_name) = &ctx.rest_param_name
            {
                return IrExpr::Ident(IrIdent::new(rest_name));
            }
            return IrExpr::Ident(IrIdent::new("__arguments"));
        }

        // JS global constants
        if var_name == "NaN" {
            return IrExpr::BuiltinCall(crate::zigir::types::IrBuiltinCall::simple(
                crate::zigir::builtins::BuiltinModule::JsMath,
                "nan_f64",
                None,
                None,
                vec![],
                crate::types::ZigType::F64,
            ));
        }
        if var_name == "Infinity" {
            return IrExpr::BuiltinCall(crate::zigir::types::IrBuiltinCall::simple(
                crate::zigir::builtins::BuiltinModule::JsMath,
                "inf_f64",
                None,
                None,
                vec![],
                crate::types::ZigType::F64,
            ));
        }
        // undefined → JsAny.fromUndefined()
        // (Stored as Ident with special name; Emitter will handle)
        if var_name == "undefined" {
            return IrExpr::Undefined;
        }

        // Check if this identifier is a captured closure variable.
        // If so, rewrite to self.var_name (value capture) or self.var_name.* (ref capture).
        if let Some((_, _, is_mut)) = self
            .closure_mgr
            .current_captured
            .iter()
            .find(|(n, _, _)| n == var_name)
        {
            let field_name = self.make_ident(var_name);
            let self_access = IrExpr::FieldAccess {
                object: Box::new(IrExpr::Ident(IrIdent::new("self"))),
                field: field_name.zig_name.clone(),
                field_kind: FieldKind::StructField,
            };
            return if *is_mut {
                // Reference capture: dereference the pointer
                IrExpr::FieldAccess {
                    object: Box::new(self_access),
                    field: "*".to_string(),
                    field_kind: FieldKind::PointerDeref,
                }
            } else {
                self_access
            };
        }

        IrExpr::Ident(self.make_ident(var_name))
    }
}
