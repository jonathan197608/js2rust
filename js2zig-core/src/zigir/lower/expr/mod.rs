// zigir/lower/expr/mod.rs
// Expression lowering: main dispatch + identifier handling.

use oxc_ast::ast::*;

use crate::zigir::ident::IrIdent;
use crate::zigir::kinds::FieldKind;
use crate::zigir::ops::LogicalOp;
use crate::zigir::source_span::SourceSpan;

use super::Lowerer;
use super::cabi::expr_type_name;

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
                // JS regexp literal `/pattern/flags` → new RegExp(pattern)
                // Produce an IrNewExpr equivalent to `new RegExp("pattern")`
                let pattern = rl.regex.pattern.text.as_str();
                let escaped = pattern.replace('\\', "\\\\").replace('"', "\\\"");
                IrExpr::New(crate::zigir::types::IrNewExpr {
                    constructor: crate::zigir::kinds::NewConstructor::RegExp,
                    args: vec![IrExpr::StringLiteral(escaped)],
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
                if self.current_class.is_some() {
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
                IrExpr::Logical {
                    op,
                    left: Box::new(self.lower_expr(&le.left)),
                    right: Box::new(self.lower_expr(&le.right)),
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
            Expression::StaticMemberExpression(mem) => self.lower_static_member(mem),
            Expression::ComputedMemberExpression(mem) => self.lower_computed_member(mem),

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
                let span = self.span_to_source_span(mp.span);
                self.add_error(
                    span,
                    "MetaProperty (import.meta/new.target) is not supported",
                );
                IrExpr::CompileError {
                    span: SourceSpan::default(),
                    msg: "MetaProperty not supported".to_string(),
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

            // ── Class expression (anonymous class as value) ──
            Expression::ClassExpression(ce) => {
                let name = ce
                    .id
                    .as_ref()
                    .map(|id| id.name.as_str())
                    .unwrap_or("<anonymous>");
                IrExpr::CompileError {
                    span: SourceSpan::default(),
                    msg: format!("class expression '{}' is not supported", name),
                }
            }

            // ── Fallback ──────────────────────────────────────────────────────
            _ => IrExpr::CompileError {
                span: SourceSpan::default(),
                msg: format!("unsupported expression type: {}", expr_type_name(expr)),
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

        // arguments object: not supported
        if var_name == "arguments" {
            let span = self.span_to_source_span(id.span);
            self.add_error(
                span,
                "arguments object is not supported. Use rest parameters (...args) instead.",
            );
            return IrExpr::CompileError {
                span: SourceSpan::default(),
                msg: "arguments not supported".to_string(),
            };
        }

        // JS global constants
        if var_name == "NaN" {
            return IrExpr::BuiltinCall(crate::zigir::types::IrBuiltinCall {
                module: crate::zigir::builtins::BuiltinModule::JsMath,
                method: "nan_f64".to_string(),
                obj_name: None,
                obj_expr: None,
                args: vec![],
                return_type: crate::types::ZigType::F64,
                ta_type_suffix: None,
                regex_info: None,
            });
        }
        if var_name == "Infinity" {
            return IrExpr::BuiltinCall(crate::zigir::types::IrBuiltinCall {
                module: crate::zigir::builtins::BuiltinModule::JsMath,
                method: "inf_f64".to_string(),
                obj_name: None,
                obj_expr: None,
                args: vec![],
                return_type: crate::types::ZigType::F64,
                ta_type_suffix: None,
                regex_info: None,
            });
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
