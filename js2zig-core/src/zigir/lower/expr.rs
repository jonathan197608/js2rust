// zigir/lower/expr.rs
// Expression lowering: all AST expression types ¡ú IrExpr.

use std::collections::HashSet;

use oxc_ast::ast::*;

use crate::types::ZigType;
use crate::zigir::builtins::BuiltinModule;
use crate::zigir::ident::IrIdent;
use crate::zigir::kinds::{CallKind, ComputedKeyKind, FieldKind, IndexKind};
use crate::zigir::ops::{AssignOp, BinOp, LogicalOp, UnaOp, UpdateOp};
use crate::zigir::source_span::SourceSpan;
use crate::zigir::types::IrBlock;

use super::Lowerer;
use super::cabi::{builtin_call_to_ir, expr_type_name};

impl Lowerer {
    pub(super) fn lower_expr(&mut self, expr: &Expression) -> crate::zigir::types::IrExpr {
        use crate::zigir::types::IrExpr;

        match expr {
            // ©¤©¤ Literals ©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤
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
                // JS regexp literal `/pattern/flags` ¡ú new RegExp(pattern)
                // Produce an IrNewExpr equivalent to `new RegExp("pattern")`
                let pattern = rl.regex.pattern.text.as_str();
                let escaped = pattern.replace('\\', "\\\\").replace('"', "\\\"");
                crate::zigir::types::IrExpr::New(crate::zigir::types::IrNewExpr {
                    constructor: crate::zigir::kinds::NewConstructor::RegExp,
                    args: vec![crate::zigir::types::IrExpr::StringLiteral(escaped)],
                    result_type: crate::types::ZigType::JsAny,
                })
            }
            Expression::BigIntLiteral(bi) => {
                // BigInt literal: store the decimal value string (without trailing 'n')
                let s = bi.value.as_str().to_string();
                crate::zigir::types::IrExpr::BigIntLiteral(s)
            }

            // ©¤©¤ Identifier ©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤
            Expression::Identifier(id) => self.lower_ident_expr(id),

            // ©¤©¤ This ©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤
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

            // ©¤©¤ Binary expression ©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤
            Expression::BinaryExpression(be) => self.lower_binary(be),

            // ©¤©¤ Logical expression ©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤
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

            // ©¤©¤ Unary expression ©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤
            Expression::UnaryExpression(ue) => self.lower_unary(ue),

            // ©¤©¤ Update expression ©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤
            Expression::UpdateExpression(ue) => self.lower_update(ue),

            // ©¤©¤ Assignment expression ©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤
            Expression::AssignmentExpression(ae) => self.lower_assignment(ae),

            // ©¤©¤ Parenthesized ©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤
            Expression::ParenthesizedExpression(pe) => {
                IrExpr::Paren(Box::new(self.lower_expr(&pe.expression)))
            }

            // ©¤©¤ Conditional ©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤
            Expression::ConditionalExpression(ce) => IrExpr::Conditional {
                cond: Box::new(self.lower_expr(&ce.test)),
                then: Box::new(self.lower_expr(&ce.consequent)),
                else_: Box::new(self.lower_expr(&ce.alternate)),
            },

            // ©¤©¤ Sequence expression ©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤
            Expression::SequenceExpression(se) => {
                let exprs: Vec<IrExpr> =
                    se.expressions.iter().map(|e| self.lower_expr(e)).collect();
                IrExpr::Sequence(exprs)
            }

            // ©¤©¤ Calls ©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤
            Expression::CallExpression(ce) => self.lower_call(ce),
            Expression::NewExpression(ne) => self.lower_new(ne),

            // ©¤©¤ Member access ©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤
            Expression::StaticMemberExpression(mem) => self.lower_static_member(mem),
            Expression::ComputedMemberExpression(mem) => self.lower_computed_member(mem),

            // ©¤©¤ Array / Object literals ©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤
            Expression::ArrayExpression(ae) => self.lower_array_expr(ae),
            Expression::ObjectExpression(oe) => self.lower_object_expr(oe),

            // ©¤©¤ Function expressions ©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤
            Expression::ArrowFunctionExpression(af) => self.lower_arrow_fn(af),
            Expression::FunctionExpression(fe) => self.lower_fn_expr(fe),

            // ©¤©¤ Template literal ©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤
            Expression::TemplateLiteral(tl) => self.lower_template_literal(tl),

            // ©¤©¤ Tagged template ©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤
            Expression::TaggedTemplateExpression(tte) => {
                let span = self.span_to_source_span(tte.span);
                self.add_error(span, "Tagged template literals are not supported");
                IrExpr::CompileError {
                    span: SourceSpan::default(),
                    msg: "Tagged template literals not supported".to_string(),
                }
            }

            // ©¤©¤ Await ©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤
            Expression::AwaitExpression(ae) => self.lower_await(ae),

            // ©¤©¤ typeof / void / delete ©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤
            // (handled via UnaryExpression, but also here as fallback)

            // ©¤©¤ Yield ©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤
            Expression::YieldExpression(ye) => {
                let span = self.span_to_source_span(ye.span);
                self.add_error(span, "Yield expressions are not supported");
                IrExpr::CompileError {
                    span: SourceSpan::default(),
                    msg: "yield not supported".to_string(),
                }
            }

            // ©¤©¤ MetaProperty (import.meta, new.target) ©¤©¤
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

            // ©¤©¤ Super ©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤
            Expression::Super(sup) => {
                let span = self.span_to_source_span(sup.span);
                self.add_error(span, "super is not supported");
                IrExpr::CompileError {
                    span: SourceSpan::default(),
                    msg: "super not supported".to_string(),
                }
            }

            // ©¤©¤ Import ©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤
            Expression::ImportExpression(ie) => {
                let span = self.span_to_source_span(ie.span);
                self.add_error(span, "dynamic import() is not supported");
                IrExpr::CompileError {
                    span: SourceSpan::default(),
                    msg: "dynamic import() not supported".to_string(),
                }
            }

            // ©¤©¤ PrivateFieldAccess ©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤
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

            // ©¤©¤ Optional chaining (?.) ©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤
            Expression::ChainExpression(ce) => self.lower_optional_chain(ce),

            // ©¤©¤ Class expression (anonymous class as value) ©¤©¤
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

            // ©¤©¤ Fallback ©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤
            _ => IrExpr::CompileError {
                span: SourceSpan::default(),
                msg: format!("unsupported expression type: {}", expr_type_name(expr)),
            },
        }
    }

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
                span: SourceSpan::default(),
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
                    _ => true, // Void, JsAny, Anytype, unknown ¡ú might be null
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
            return IrExpr::FieldAccess {
                object: Box::new(IrExpr::Ident(IrIdent::new("std"))),
                field: "nan".to_string(),
                field_kind: FieldKind::Namespace,
            };
        }
        if var_name == "Infinity" {
            return IrExpr::FieldAccess {
                object: Box::new(IrExpr::Ident(IrIdent::new("std"))),
                field: "inf".to_string(),
                field_kind: FieldKind::Namespace,
            };
        }
        // undefined ¡ú JsAny.fromUndefined()
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
            if *is_mut {
                // Reference capture: dereference the pointer
                return IrExpr::FieldAccess {
                    object: Box::new(self_access),
                    field: "*".to_string(),
                    field_kind: FieldKind::PointerDeref,
                };
            } else {
                return self_access;
            }
        }

        IrExpr::Ident(self.make_ident(var_name))
    }

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

        // ©¤©¤ Unsupported operators ¡ú compile error ©¤©¤
        match be.operator {
            BinaryOperator::Instanceof => {
                return IrExpr::CompileError {
                    span: self.span_to_source_span(be.span),
                    msg: "instanceof operator is not supported in Zig".to_string(),
                };
            }
            BinaryOperator::In => {
                // `key in obj` ¡ú obj.contains(key)
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
                // JS `**` always returns f64. Emit a PowExpr with type info
                // so the Emitter can generate the correct f64 coercion.
                let left_type = self.infer_expr_type(&be.left).unwrap_or(ZigType::F64);
                let right_type = self.infer_expr_type(&be.right).unwrap_or(ZigType::F64);
                return IrExpr::PowExpr {
                    base: Box::new(self.lower_expr(&be.left)),
                    exp: Box::new(self.lower_expr(&be.right)),
                    base_type: left_type,
                    exp_type: right_type,
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
            UnaryOperator::UnaryNegation => IrExpr::Unary {
                op: UnaOp::Neg,
                operand: Box::new(self.lower_expr(&ue.argument)),
            },
            UnaryOperator::UnaryPlus => {
                // Unary plus is a no-op in terms of IR; just lower the argument
                self.lower_expr(&ue.argument)
            }
            UnaryOperator::LogicalNot => IrExpr::Unary {
                op: UnaOp::Not,
                operand: Box::new(self.lower_expr(&ue.argument)),
            },
            UnaryOperator::BitwiseNot => IrExpr::Unary {
                op: UnaOp::BitNot,
                operand: Box::new(self.lower_expr(&ue.argument)),
            },
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
                            module: crate::zigir::builtins::BuiltinModule::JsRuntime,
                            method: "jsTypeof".to_string(),
                            obj_name: None,
                            obj_expr: None,
                            args: vec![self.lower_expr(&ue.argument)],
                            return_type: crate::types::ZigType::Str,
                            regex_info: None,
                            ta_type_suffix: None,
                        })
                    }
                } else {
                    IrExpr::BuiltinCall(crate::zigir::types::IrBuiltinCall {
                        module: crate::zigir::builtins::BuiltinModule::JsRuntime,
                        method: "jsTypeof".to_string(),
                        obj_name: None,
                        obj_expr: None,
                        args: vec![self.lower_expr(&ue.argument)],
                        return_type: crate::types::ZigType::Str,
                        regex_info: None,
                        ta_type_suffix: None,
                    })
                }
            }
            UnaryOperator::Void => IrExpr::Void(Box::new(self.lower_expr(&ue.argument))),
            UnaryOperator::Delete => {
                // delete obj.prop ¡ú IrBuiltinCall { JsRuntime, "deleteKey", obj, [prop] }
                // delete obj[expr] ¡ú IrBuiltinCall { JsRuntime, "deleteByKey", obj, [expr] }
                match &ue.argument {
                    Expression::StaticMemberExpression(mem) => {
                        let obj_name = match &mem.object {
                            Expression::Identifier(id) => Some(id.name.as_str().to_string()),
                            _ => None,
                        };
                        IrExpr::BuiltinCall(crate::zigir::types::IrBuiltinCall {
                            module: crate::zigir::builtins::BuiltinModule::JsRuntime,
                            method: "deleteKey".to_string(),
                            obj_name,
                            obj_expr: None,
                            args: vec![IrExpr::StringLiteral(
                                mem.property.name.as_str().to_string(),
                            )],
                            return_type: crate::types::ZigType::Bool,
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
                            module: crate::zigir::builtins::BuiltinModule::JsRuntime,
                            method: "deleteByKey".to_string(),
                            obj_name,
                            obj_expr: None,
                            args: vec![self.lower_expr(&mem.expression)],
                            return_type: crate::types::ZigType::Bool,
                            regex_info: None,
                            ta_type_suffix: None,
                        })
                    }
                    _ => {
                        // Unsupported delete target ¡ª emit compile error
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

        // ©¤©¤ Special-case compound assignments that need expansion ©¤©¤
        match ae.operator {
            // **= ¡ú a = std.math.pow(a, b) via PowExpr
            AssignmentOperator::Exponential => {
                let target = self.lower_assign_target(&ae.left);
                let value = Box::new(self.lower_expr(&ae.right));
                // Read target as expression for the PowExpr base
                let base_ident = match &target {
                    crate::zigir::types::IrAssignTarget::Ident(name) => IrExpr::Ident(name.clone()),
                    _ => IrExpr::Ident(IrIdent::new("__target")),
                };
                return IrExpr::Assign {
                    op: AssignOp::Assign,
                    target: Box::new(target),
                    value: Box::new(IrExpr::PowExpr {
                        base: Box::new(base_ident),
                        exp: value,
                        base_type: crate::types::ZigType::F64,
                        exp_type: crate::types::ZigType::F64,
                    }),
                };
            }
            // &&= / ||= / ??= ¡ú use AssignOp, Emitter will expand
            _ => {}
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
                                // e.g. { name: alias } ¡ª extract binding from value
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

    /// Check for unsupported global object calls (Atomics, Reflect, etc.)
    /// that should produce compile errors instead of silent code generation.
    pub(super) fn check_unsupported_call(&self, ce: &CallExpression) -> Option<String> {
        // Match patterns: Atomics.load(), Reflect.apply(), Map.groupBy(), etc.
        match &ce.callee {
            Expression::StaticMemberExpression(mem) => {
                if let Expression::Identifier(id) = &mem.object {
                    let obj_name = id.name.as_str();
                    let method_name = mem.property.name.as_str();
                    match obj_name {
                        "Atomics" => Some(format!(
                            "Atomics.{}() is not supported (shared memory atomics are not available in Zig)",
                            method_name
                        )),
                        "Reflect" => Some(format!(
                            "Reflect.{}() is not supported (meta-programming API is not available)",
                            method_name
                        )),
                        "Object" => match method_name {
                            "getOwnPropertySymbols" => Some(
                                "Object.getOwnPropertySymbols() is not supported (Symbol keys are not available in js2zig)".to_string(),
                            ),
                            "getOwnPropertyNames" => Some(
                                "Object.getOwnPropertyNames() is not yet implemented in js2zig".to_string(),
                            ),
                            _ => None,
                        },
                        "Map" if method_name == "groupBy" => Some(
                            "Map.groupBy() is not supported (requires iterable grouping)".to_string(),
                        ),
                        _ => None,
                    }
                } else {
                    None
                }
            }
            Expression::Identifier(id) => match id.name.as_str() {
                name @ "Atomics" | name @ "Reflect" => {
                    Some(format!("{} is not supported in js2zig", name))
                }
                _ => None,
            },
            _ => None,
        }
    }

    /// Lower a call expression.
    ///
    /// Routing priority (mirrors Codegen's `emit_call`):
    /// 1. Builtin detection ¡ú `IrBuiltinCall`
    /// 2. Closure / nested function call ¡ú `IrCall { call_kind: Closure }`
    /// 3. Host function call ¡ú `IrHostCall`
    /// 4. Direct user function ¡ú `IrCall { call_kind: Direct }`
    /// 5. Method call ¡ú `IrCall { call_kind: Method { .. } }`
    /// 6. IIFE / expression callee ¡ú `IrCall { call_kind: Closure }`
    pub(super) fn lower_call(&mut self, ce: &CallExpression) -> crate::zigir::types::IrExpr {
        use crate::zigir::types::IrExpr;

        // ©¤©¤ Step 0: Check for unsupported global objects ©¤©¤
        if let Some(err_msg) = self.check_unsupported_call(ce) {
            return IrExpr::CompileError {
                span: self.span_to_source_span(ce.span),
                msg: err_msg,
            };
        }

        let args: Vec<IrExpr> = ce
            .arguments
            .iter()
            .map(|arg| {
                match arg {
                    Argument::SpreadElement(se) => {
                        IrExpr::Spread(Box::new(self.lower_expr(&se.argument)))
                    }
                    // Argument inherits all Expression variants
                    _ => {
                        // All Expression variants are directly accessible
                        let expr = arg.as_expression().unwrap();
                        self.lower_expr(expr)
                    }
                }
            })
            .collect();

        // ©¤©¤ Step 1: Builtin detection ©¤©¤
        if let Some(builtin) = crate::native_builtins::detect_builtin_call(ce) {
            // ©¤©¤ Step 1a: Array callback inlining ©¤©¤
            if let Some(inlined) = self.try_inline_array_callback(ce, &builtin) {
                return inlined;
            }

            // ©¤©¤ Step 1b: Array non-callback method inlining ©¤©¤
            if let Some(inlined) = self.try_inline_array_method(ce, &builtin, &args) {
                return inlined;
            }

            // ©¤©¤ Step 1c: eval() ¡ú compile error ©¤©¤
            if matches!(builtin, crate::native_builtins::BuiltinCall::Eval) {
                return IrExpr::CompileError {
                    span: self.span_to_source_span(ce.span),
                    msg: "eval() is not supported (security risk, cannot dynamically execute at compile time)".to_string(),
                };
            }

            let (module, method, return_type) = builtin_call_to_ir(&builtin);
            let obj_name = Self::extract_callee_object_name_static(&ce.callee);

            // ©¤©¤ Fix string-variable methods misidentified as array ©¤©¤
            // detect_builtin_call only checks if the callee object is a StringLiteral,
            // not if it's a variable of type string. Fix up the module/method here.
            let (module, method, return_type) = if let Some(name) = &obj_name {
                if let Some(var_type) = self.type_info.var_types.get(name.as_str()) {
                    if matches!(var_type, ZigType::Str)
                        && module == crate::zigir::builtins::BuiltinModule::JsArray
                    {
                        match method.as_str() {
                            "at" => (
                                crate::zigir::builtins::BuiltinModule::JsString,
                                "at".into(),
                                ZigType::Str,
                            ),
                            "indexOf" => (
                                crate::zigir::builtins::BuiltinModule::JsString,
                                "indexOf".into(),
                                ZigType::I64,
                            ),
                            "includes" => (
                                crate::zigir::builtins::BuiltinModule::JsString,
                                "includes".into(),
                                ZigType::Bool,
                            ),
                            "lastIndexOf" => (
                                crate::zigir::builtins::BuiltinModule::JsString,
                                "lastIndexOf".into(),
                                ZigType::I64,
                            ),
                            "slice" => (
                                crate::zigir::builtins::BuiltinModule::JsString,
                                "slice".into(),
                                ZigType::Str,
                            ),
                            _ => (module, method, return_type),
                        }
                    } else if let ZigType::NamedStruct(n) = var_type {
                        if Self::is_typedarray_type(n) {
                            let ta_mod = BuiltinModule::JsTypedArray;
                            match method.as_str() {
                                "set" => (ta_mod, "set".into(), ZigType::Void),
                                "get" => (ta_mod, "get".into(), ZigType::I64),
                                "fill" => (ta_mod, "fill".into(), ZigType::Void),
                                "slice" => {
                                    (ta_mod, "slice".into(), ZigType::NamedStruct(n.clone()))
                                }
                                "copyWithin" => (ta_mod, "copyWithin".into(), ZigType::Void),
                                _ => (module, method, return_type),
                            }
                        } else {
                            (module, method, return_type)
                        }
                    } else {
                        (module, method, return_type)
                    }
                } else {
                    (module, method, return_type)
                }
            } else {
                (module, method, return_type)
            };
            let obj_name = Self::extract_callee_object_name_static(&ce.callee);

            // ©¤©¤ Extract regex metadata for match/matchAll/search ©¤©¤
            let regex_info = Self::extract_regex_info(ce, &builtin);

            // ©¤©¤ Derive TypedArray type suffix for JsTypedArray calls ©¤©¤
            let ta_type_suffix = if module == BuiltinModule::JsTypedArray {
                obj_name.as_ref().and_then(|name| {
                    self.type_info.var_types.get(name.as_str()).and_then(|zt| {
                        if let ZigType::NamedStruct(n) = zt {
                            Self::typedarray_type_suffix(n).map(|s| s.to_string())
                        } else {
                            None
                        }
                    })
                })
            } else {
                None
            };

            // ©¤©¤ Handle complex receiver expressions (method chaining) ©¤©¤
            // When the receiver is a CallExpression (e.g., encodeURIComponent(str).replace(...)),
            // extract_callee_object_name_static returns None. We lower the receiver expression
            // and store it in obj_expr so the Emitter can inline it.
            let obj_expr = if obj_name.is_none() {
                if let Expression::StaticMemberExpression(sme) = &ce.callee {
                    match &sme.object {
                        Expression::CallExpression(_) => {
                            Some(Box::new(self.lower_expr(&sme.object)))
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            } else {
                None
            };

            return IrExpr::BuiltinCall(crate::zigir::types::IrBuiltinCall {
                module,
                method,
                obj_name,
                obj_expr,
                args,
                return_type,
                regex_info,
                ta_type_suffix,
            });
        }

        // ©¤©¤ Step 1.5: RegExp variable method interception ©¤©¤
        // `r.test(s)` or `r.exec(s)` where `r` is a known RegExp variable.
        // detect_builtin_call only identifies RegExpTest/RegExpExec for RegExpLiteral receivers.
        // For variable receivers, we intercept here using regexp_vars tracking.
        if let Expression::StaticMemberExpression(sme) = &ce.callee {
            if let Expression::Identifier(id) = &sme.object {
                let var_name = id.name.as_str();
                if let Some(ctx) = &self.fn_ctx {
                    if ctx.regexp_vars.contains(var_name) {
                        let method = sme.property.name.as_str();
                        match method {
                            "test" => {
                                return IrExpr::BuiltinCall(crate::zigir::types::IrBuiltinCall {
                                    module: BuiltinModule::JsRegExp,
                                    method: "test".into(),
                                    obj_name: Some(var_name.to_string()),
                                    obj_expr: None,
                                    args,
                                    return_type: ZigType::Bool,
                                    regex_info: Some(crate::zigir::types::IrRegexInfo {
                                        pattern: None,
                                        has_global: false,
                                        is_var_ref: true,
                                        var_name: Some(var_name.to_string()),
                                    }),
                                    ta_type_suffix: None,
                                });
                            }
                            "exec" => {
                                return IrExpr::BuiltinCall(crate::zigir::types::IrBuiltinCall {
                                    module: BuiltinModule::JsRegExp,
                                    method: "exec".into(),
                                    obj_name: Some(var_name.to_string()),
                                    obj_expr: None,
                                    args,
                                    return_type: ZigType::JsAny,
                                    regex_info: Some(crate::zigir::types::IrRegexInfo {
                                        pattern: None,
                                        has_global: false,
                                        is_var_ref: true,
                                        var_name: Some(var_name.to_string()),
                                    }),
                                    ta_type_suffix: None,
                                });
                            }
                            _ => {} // other methods fall through
                        }
                    }
                }
            }
        }

        // ©¤©¤ Step 2: Identify callee pattern ©¤©¤
        match &ce.callee {
            // Identifier callee: direct function call, host call, or closure
            Expression::Identifier(id) => {
                let name = id.name.as_str();

                // Host function call: starts with "host_"
                if let Some(host_name) = name.strip_prefix("host_") {
                    let is_async = self.async_host_fns.contains(name);
                    let return_type = self.infer_host_return_type(host_name);
                    return IrExpr::HostCall(crate::zigir::types::IrHostCall {
                        name: host_name.to_string(),
                        args,
                        return_type,
                        is_async,
                    });
                }

                // Closure / nested function call
                if self
                    .closure_mgr
                    .current_captured
                    .iter()
                    .any(|(n, _, _)| n.as_str() == name)
                    || self.is_closure_instance(name)
                {
                    return IrExpr::Call(crate::zigir::types::IrCallExpr {
                        callee: Box::new(IrExpr::Ident(IrIdent::new(name))),
                        args,
                        call_kind: CallKind::Closure,
                    });
                }

                // Nested function call: rewrite to name.call(args)
                if let Some(ctx) = &self.fn_ctx {
                    if ctx.is_nested_fn(name) {
                        let callee_ident = self.make_ident(name);
                        return IrExpr::Call(crate::zigir::types::IrCallExpr {
                            callee: Box::new(IrExpr::Ident(callee_ident)),
                            args,
                            call_kind: CallKind::Closure,
                        });
                    }
                }

                // Direct user function call
                IrExpr::Call(crate::zigir::types::IrCallExpr {
                    callee: Box::new(IrExpr::Ident(IrIdent::new(name))),
                    args,
                    call_kind: CallKind::Direct,
                })
            }

            // Static member expression callee: obj.method()
            Expression::StaticMemberExpression(mem) => {
                let method_name = mem.property.name.as_str();
                let obj_expr = self.lower_expr(&mem.object);

                // Determine method object type for CallKind::Method
                let object_type = self.infer_method_object_kind(&mem.object);

                IrExpr::Call(crate::zigir::types::IrCallExpr {
                    callee: Box::new(IrExpr::FieldAccess {
                        object: Box::new(obj_expr),
                        field: method_name.to_string(),
                        field_kind: FieldKind::StructField,
                    }),
                    args,
                    call_kind: CallKind::Method { object_type },
                })
            }

            // Function expression / arrow function callee (IIFE)
            Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_) => {
                // IIFE: emit the function then call it
                let callee = self.lower_expr(&ce.callee);
                IrExpr::Call(crate::zigir::types::IrCallExpr {
                    callee: Box::new(callee),
                    args,
                    call_kind: CallKind::Closure,
                })
            }

            // Parenthesized expression containing function
            Expression::ParenthesizedExpression(_) => {
                let callee = self.lower_expr(&ce.callee);
                IrExpr::Call(crate::zigir::types::IrCallExpr {
                    callee: Box::new(callee),
                    args,
                    call_kind: CallKind::Closure,
                })
            }

            // Any other callee type (computed member, etc.)
            _ => {
                let callee = self.lower_expr(&ce.callee);
                IrExpr::Call(crate::zigir::types::IrCallExpr {
                    callee: Box::new(callee),
                    args,
                    call_kind: CallKind::Direct,
                })
            }
        }
    }

    /// Determine the MethodObjectKind for a method call's receiver object.
    pub(super) fn infer_method_object_kind(
        &self,
        obj: &Expression,
    ) -> crate::zigir::kinds::MethodObjectKind {
        use crate::zigir::kinds::MethodObjectKind;

        match obj {
            Expression::Identifier(id) => {
                if let Some(zig_type) = self.type_info.var_types.get(id.name.as_str()) {
                    match zig_type {
                        ZigType::ArrayList(_) => MethodObjectKind::ArrayList,
                        ZigType::Str => MethodObjectKind::String,
                        ZigType::NamedStruct(name) => match name.as_str() {
                            "Map" => MethodObjectKind::Map,
                            "Set" => MethodObjectKind::Set,
                            "Date" | "JsDate" => MethodObjectKind::Date,
                            other => {
                                if self.class_names.contains(other) {
                                    MethodObjectKind::Class(other.to_string())
                                } else {
                                    MethodObjectKind::Unknown
                                }
                            }
                        },
                        ZigType::JsAny | ZigType::Anytype => MethodObjectKind::JsAny,
                        _ => MethodObjectKind::Unknown,
                    }
                } else {
                    MethodObjectKind::Unknown
                }
            }
            _ => MethodObjectKind::Unknown,
        }
    }

    /// Check if a name refers to a closure instance.
    pub(super) fn is_closure_instance(&self, name: &str) -> bool {
        self.closure_mgr.closure_instances.contains(name)
    }

    /// Infer the return type of a host function.
    pub(super) fn infer_host_return_type(&self, _host_name: &str) -> ZigType {
        // TODO: look up host function return type from type_info
        ZigType::JsAny
    }

    /// Lower a static member expression (`obj.field`).
    ///
    /// Determines the FieldKind based on:
    /// - Math constants ¡ú `MathConstant`
    /// - Number constants ¡ú `NumberConstant`
    /// - Symbol well-known ¡ú `SymbolWellKnown`
    /// - TypedArray properties ¡ú `TypedArrayProp`
    /// - Map/Set `.size` ¡ú `MapSetSize`
    /// - ArrayList `.length` ¡ú `ArrayListLen`
    /// - Other `.length` ¡ú `StringLen`
    /// - Default ¡ú `StructField`
    pub(super) fn lower_static_member(
        &mut self,
        mem: &StaticMemberExpression,
    ) -> crate::zigir::types::IrExpr {
        use crate::zigir::types::IrExpr;

        let field_name = mem.property.name.as_str();

        // ©¤©¤ Math constants: Math.PI, Math.E, etc. ©¤©¤
        if let Expression::Identifier(id) = &mem.object {
            if id.name.as_str() == "Math" {
                return IrExpr::FieldAccess {
                    object: Box::new(self.lower_expr(&mem.object)),
                    field: field_name.to_string(),
                    field_kind: FieldKind::MathConstant(field_name.to_string()),
                };
            }
            // ©¤©¤ Number constants: Number.MAX_VALUE, Number.NaN, etc. ©¤©¤
            if id.name.as_str() == "Number" {
                return IrExpr::FieldAccess {
                    object: Box::new(self.lower_expr(&mem.object)),
                    field: field_name.to_string(),
                    field_kind: FieldKind::NumberConstant(field_name.to_string()),
                };
            }
            // ©¤©¤ Symbol well-known: Symbol.iterator, etc. ©¤©¤
            if id.name.as_str() == "Symbol" {
                return IrExpr::FieldAccess {
                    object: Box::new(self.lower_expr(&mem.object)),
                    field: field_name.to_string(),
                    field_kind: FieldKind::SymbolWellKnown(field_name.to_string()),
                };
            }
            // ©¤©¤ TypedArray properties ©¤©¤
            if let Some(zig_type) = self.type_info.var_types.get(id.name.as_str()) {
                if let ZigType::NamedStruct(name) = zig_type {
                    // ©¤©¤ TypedArray properties (buffer, byteLength, byteOffset) ©¤©¤
                    if Self::is_typedarray_type(name)
                        && matches!(field_name, "buffer" | "byteLength" | "byteOffset")
                    {
                        let type_suffix = Self::typedarray_type_suffix(name).map(|s| s.to_string());
                        return IrExpr::FieldAccess {
                            object: Box::new(self.lower_expr(&mem.object)),
                            field: field_name.to_string(),
                            field_kind: FieldKind::TypedArrayProp {
                                prop: field_name.to_string(),
                                type_suffix,
                            },
                        };
                    }
                    // ©¤©¤ Map/Set .size ©¤©¤
                    if matches!(name.as_str(), "Map" | "Set") && field_name == "size" {
                        return IrExpr::FieldAccess {
                            object: Box::new(self.lower_expr(&mem.object)),
                            field: field_name.to_string(),
                            field_kind: FieldKind::MapSetSize,
                        };
                    }
                }
                // ©¤©¤ ArrayList .length ¡ú .items.len ©¤©¤
                if matches!(zig_type, ZigType::ArrayList(_)) && field_name == "length" {
                    return IrExpr::FieldAccess {
                        object: Box::new(self.lower_expr(&mem.object)),
                        field: field_name.to_string(),
                        field_kind: FieldKind::ArrayListLen,
                    };
                }
            }
            // ©¤©¤ .length on other types (string, slice, etc.) ©¤©¤
            if field_name == "length" {
                return IrExpr::FieldAccess {
                    object: Box::new(self.lower_expr(&mem.object)),
                    field: field_name.to_string(),
                    field_kind: FieldKind::StringLen,
                };
            }
        }

        // ©¤©¤ Default: struct field access ©¤©¤
        IrExpr::FieldAccess {
            object: Box::new(self.lower_expr(&mem.object)),
            field: field_name.to_string(),
            field_kind: FieldKind::StructField,
        }
    }

    /// Lower a computed member expression (`obj[key]`).
    ///
    /// Three sub-cases:
    /// - NumericLiteral key ¡ú IndexAccess (ArrayListItem or SliceIndex)
    /// - StringLiteral key ¡ú ComputedField (StructField, MapGet, JsAnyGetByKey)
    /// - Dynamic expression key ¡ú ComputedField (varies by object type)
    pub(super) fn lower_computed_member(
        &mut self,
        mem: &ComputedMemberExpression,
    ) -> crate::zigir::types::IrExpr {
        use crate::zigir::types::IrExpr;

        let object = Box::new(self.lower_expr(&mem.object));

        // Determine the ZigType of the object (for routing)
        let obj_type = self.infer_expr_type(&mem.object);

        // ©¤©¤ Case 1: NumericLiteral key ¡ú IndexAccess ©¤©¤
        if let Expression::NumericLiteral(nl) = &mem.expression {
            let is_arraylist = obj_type
                .as_ref()
                .map(|t| matches!(t, ZigType::ArrayList(_)))
                .unwrap_or(false);
            return IrExpr::IndexAccess {
                object,
                index: Box::new(IrExpr::IntLiteral(nl.value as i64)),
                index_kind: if is_arraylist {
                    IndexKind::ArrayListItem
                } else {
                    IndexKind::SliceIndex
                },
            };
        }

        // ©¤©¤ Case 2: StringLiteral key ¡ú ComputedField ©¤©¤
        if let Expression::StringLiteral(sl) = &mem.expression {
            let key_kind = match &obj_type {
                Some(ZigType::Struct(_)) => ComputedKeyKind::StructField,
                Some(ZigType::NamedStruct(name)) if name == "Map" => ComputedKeyKind::MapGet,
                Some(ZigType::NamedStruct(_)) => ComputedKeyKind::StructField,
                Some(ZigType::Anytype) | Some(ZigType::JsAny) => ComputedKeyKind::JsAnyGetByKey,
                _ => ComputedKeyKind::JsAnyGetByKey,
            };
            return IrExpr::ComputedField {
                object,
                key: Box::new(IrExpr::StringLiteral(sl.value.to_string())),
                key_kind,
            };
        }

        // ©¤©¤ Case 3: Dynamic expression key ¡ú ComputedField ©¤©¤
        let key = Box::new(self.lower_expr(&mem.expression));
        let key_kind = match &obj_type {
            Some(ZigType::Anytype) | Some(ZigType::JsAny) => ComputedKeyKind::JsAnyGetByKey,
            Some(ZigType::NamedStruct(name)) if name == "Map" => ComputedKeyKind::MapGet,
            Some(ZigType::ArrayList(_)) => ComputedKeyKind::ArrayListItem,
            Some(ZigType::Struct(_)) | Some(ZigType::NamedStruct(_)) => {
                ComputedKeyKind::StructField
            }
            None => ComputedKeyKind::JsAnyGetByKey,
            _ => ComputedKeyKind::CompileError(format!(
                "computed access on unsupported type: {:?}",
                obj_type
            )),
        };
        IrExpr::ComputedField {
            object,
            key,
            key_kind,
        }
    }

    /// Infer the ZigType of an expression based on type_info and expression structure.
    /// Enhanced version that covers literal types, member access, calls, and more.
    pub(super) fn infer_expr_type(&self, expr: &Expression) -> Option<ZigType> {
        match expr {
            Expression::Identifier(id) => {
                // Special globals
                match id.name.as_str() {
                    "Infinity" | "NaN" => return Some(ZigType::F64),
                    "undefined" => return Some(ZigType::JsAny),
                    _ => {}
                }
                // Try exact match, then qualified, then suffix-based
                if let Some(ty) = self.type_info.var_types.get(id.name.as_str()) {
                    return Some(ty.clone());
                }
                if let Some(ctx) = self.fn_ctx.as_ref() {
                    let qualified = format!("{}::{}", ctx.name, id.name);
                    if let Some(ty) = self.type_info.var_types.get(&qualified) {
                        return Some(ty.clone());
                    }
                }
                let suffix = format!("::{}", id.name);
                for (k, v) in &self.type_info.var_types {
                    if k.ends_with(&suffix) {
                        return Some(v.clone());
                    }
                }
                None
            }
            Expression::NumericLiteral(nl) => {
                // Distinguish I64 vs F64 based on presence of decimal point / exponent
                let s = nl.value.to_string();
                if s.contains('.') || s.contains('e') || s.contains('E') {
                    Some(ZigType::F64)
                } else {
                    Some(ZigType::I64)
                }
            }
            Expression::StringLiteral(_) => Some(ZigType::Str),
            Expression::TemplateLiteral(_) => Some(ZigType::Str),
            Expression::BooleanLiteral(_) => Some(ZigType::Bool),
            Expression::BigIntLiteral(_) => Some(ZigType::BigInt),
            Expression::NullLiteral(_) => Some(ZigType::JsAny),
            Expression::UnaryExpression(ue) => match ue.operator {
                UnaryOperator::LogicalNot => Some(ZigType::Bool),
                UnaryOperator::Void => Some(ZigType::JsAny),
                UnaryOperator::Typeof => Some(ZigType::Str),
                UnaryOperator::UnaryNegation | UnaryOperator::UnaryPlus => {
                    self.infer_expr_type(&ue.argument)
                }
                _ => None,
            },
            Expression::BinaryExpression(be) => {
                let left_ty = self.infer_expr_type(&be.left);
                let right_ty = self.infer_expr_type(&be.right);
                Self::infer_binary_result_type(&be.operator, left_ty, right_ty)
            }
            Expression::ConditionalExpression(ce) => {
                let then_ty = self.infer_expr_type(&ce.consequent);
                let else_ty = self.infer_expr_type(&ce.alternate);
                match (then_ty, else_ty) {
                    (Some(a), Some(b)) if a == b => Some(a),
                    (Some(ZigType::F64), _) | (_, Some(ZigType::F64)) => Some(ZigType::F64),
                    _ => None,
                }
            }
            Expression::ParenthesizedExpression(pe) => self.infer_expr_type(&pe.expression),
            Expression::StaticMemberExpression(mem) => {
                // Known constants
                if let Expression::Identifier(id) = &mem.object {
                    match id.name.as_str() {
                        "Math" => {
                            return match mem.property.name.as_str() {
                                "PI" | "E" | "LN2" | "LN10" | "LOG2E" | "LOG10E" | "SQRT1_2"
                                | "SQRT2" => Some(ZigType::F64),
                                _ => None,
                            };
                        }
                        "Number" => {
                            return match mem.property.name.as_str() {
                                "MAX_SAFE_INTEGER" | "MIN_SAFE_INTEGER" | "MAX_VALUE"
                                | "MIN_VALUE" => Some(ZigType::F64),
                                "POSITIVE_INFINITY" | "NEGATIVE_INFINITY" | "NaN" => {
                                    Some(ZigType::F64)
                                }
                                "EPSILON" => Some(ZigType::F64),
                                _ => None,
                            };
                        }
                        _ => {}
                    }
                }
                // Try struct field inference
                let obj_ty = self.infer_expr_type(&mem.object);
                match obj_ty {
                    Some(ZigType::NamedStruct(name)) => {
                        if let Some(fields) = self.type_info.class_field_types.get(&name)
                            && let Some(ty) = fields.get(mem.property.name.as_str())
                        {
                            return Some(ty.clone());
                        }
                        None
                    }
                    _ => None,
                }
            }
            Expression::CallExpression(ce) => {
                // Try to infer from known method calls
                if let Expression::StaticMemberExpression(mem) = &ce.callee {
                    if let Expression::Identifier(id) = &mem.object {
                        match id.name.as_str() {
                            "parseInt" | "Number" => return Some(ZigType::I64),
                            "parseFloat" => return Some(ZigType::F64),
                            _ => {}
                        }
                    }
                    // Method return type from object type
                    let obj_ty = self.infer_expr_type(&mem.object);
                    if let Some(ZigType::NamedStruct(name)) = &obj_ty {
                        match name.as_str() {
                            "Map" => match mem.property.name.as_str() {
                                "get" => return Some(ZigType::JsAny),
                                "has" | "delete" => return Some(ZigType::Bool),
                                _ => {}
                            },
                            "Set" => match mem.property.name.as_str() {
                                "has" | "delete" => return Some(ZigType::Bool),
                                _ => {}
                            },
                            _ => {}
                        }
                    }
                    // String method returns
                    if obj_ty == Some(ZigType::Str) {
                        match mem.property.name.as_str() {
                            "charAt" | "substring" | "slice" | "toLowerCase" | "toUpperCase"
                            | "trim" | "repeat" | "replace" | "replaceAll" | "padStart"
                            | "padEnd" => return Some(ZigType::Str),
                            "indexOf" | "lastIndexOf" | "charCodeAt" | "codePointAt" => {
                                return Some(ZigType::I64);
                            }
                            "includes" | "startsWith" | "endsWith" => return Some(ZigType::Bool),
                            _ => {}
                        }
                    }
                }
                // Try function return type lookup
                if let Expression::Identifier(id) = &ce.callee
                    && let Some(ty) = self.type_info.fn_return_types.get(id.name.as_str())
                {
                    return Some(ty.clone());
                }
                None
            }
            // Could add more patterns here from Codegen's infer_expr_type
            _ => None,
        }
    }

    /// Infer the result type of a binary operation from operand types.
    pub(super) fn infer_binary_result_type(
        op: &BinaryOperator,
        left_ty: Option<ZigType>,
        right_ty: Option<ZigType>,
    ) -> Option<ZigType> {
        match op {
            // Comparison operators always produce bool
            BinaryOperator::Equality
            | BinaryOperator::Inequality
            | BinaryOperator::StrictEquality
            | BinaryOperator::StrictInequality
            | BinaryOperator::LessThan
            | BinaryOperator::GreaterThan
            | BinaryOperator::LessEqualThan
            | BinaryOperator::GreaterEqualThan
            | BinaryOperator::In => Some(ZigType::Bool),

            // Addition: string if either operand is string, otherwise numeric
            BinaryOperator::Addition => match (left_ty.as_ref(), right_ty.as_ref()) {
                (Some(ZigType::Str), _) | (_, Some(ZigType::Str)) => Some(ZigType::Str),
                (Some(ZigType::F64), _) | (_, Some(ZigType::F64)) => Some(ZigType::F64),
                (Some(ZigType::I64), Some(ZigType::I64)) => Some(ZigType::I64),
                _ => None,
            },

            // Arithmetic: F64 if either F64, else I64
            BinaryOperator::Subtraction
            | BinaryOperator::Multiplication
            | BinaryOperator::Division
            | BinaryOperator::Remainder => match (left_ty.as_ref(), right_ty.as_ref()) {
                (Some(ZigType::F64), _) | (_, Some(ZigType::F64)) => Some(ZigType::F64),
                (Some(ZigType::I64), Some(ZigType::I64)) => Some(ZigType::I64),
                _ => None,
            },

            // Exponential always produces f64
            BinaryOperator::Exponential => Some(ZigType::F64),

            // Bitwise: always I64
            BinaryOperator::BitwiseAnd
            | BinaryOperator::BitwiseOR
            | BinaryOperator::BitwiseXOR
            | BinaryOperator::ShiftLeft
            | BinaryOperator::ShiftRight
            | BinaryOperator::ShiftRightZeroFill => Some(ZigType::I64),

            _ => None,
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
                            Some(ZigType::I64) | Some(ZigType::F64) => "{d}",
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
                            // { get x() { return expr; } } ¡ú .x = expr
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
    /// Used by getter property lowering: `{ get x() { return expr; } }` ¡ú `.x = expr`.
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

    /// Lower an arrow function expression.
    ///
    /// If the arrow captures variables from the enclosing scope, we produce
    /// an `IrClosure` (struct + instance).  Otherwise we produce a plain
    /// `IrArrowFn` (struct + static call ¡ª Zig 0.16 doesn't allow nested
    /// fn declarations with return statements).
    pub(super) fn lower_arrow_fn(
        &mut self,
        af: &ArrowFunctionExpression,
    ) -> crate::zigir::types::IrExpr {
        use crate::zigir::types::{IrArrowFn, IrCapture, IrClosure, IrExpr};

        let captured = self.collect_arrow_captures(af);
        let is_concise = af.body.statements.len() == 1
            && matches!(af.body.statements[0], Statement::ExpressionStatement(_));
        let return_type = self.infer_arrow_return_type(af, &captured);
        let params = self.lower_arrow_params(af);

        // Enter a temporary fn context so that lower_stmt / lower_expr can
        // see captured-variable state.
        let arrow_fn_label = format!("_arrow_{}", self.name_mangler.next_name("arrow"));
        let saved_fn = self.enter_fn(&arrow_fn_label, false, Some(return_type.clone()));

        // Set closure_mgr.current_captured so that lower_ident_expr can
        // rewrite captured identifiers to self.xxx.
        let saved_captured = self.closure_mgr.take_captured();
        self.closure_mgr.current_captured = captured
            .iter()
            .map(|(n, t, m)| (n.clone(), t.clone(), *m))
            .collect();

        // Lower the body
        let body = if is_concise {
            if let Statement::ExpressionStatement(es) = &af.body.statements[0] {
                let expr_ir = self.lower_expr(&es.expression);
                IrBlock::new(vec![crate::zigir::types::IrStmt::Return {
                    value: Some(expr_ir),
                }])
            } else {
                self.lower_block(&af.body.statements)
            }
        } else {
            self.lower_block(&af.body.statements)
        };

        // Restore closure state
        self.closure_mgr.restore_captured(saved_captured);
        self.exit_fn(saved_fn);

        if !captured.is_empty() {
            // Has captures ¡ú IrClosure
            let idx = self.name_mangler.peek_count("closure");
            let struct_name = IrIdent::new(&format!("Closure_{}", idx));
            let instance_name = IrIdent::new(&format!("_cl_{}", idx));
            self.name_mangler.next_name("closure"); // advance counter

            let ir_captures: Vec<IrCapture> = captured
                .into_iter()
                .map(|(name, zig_type, is_mut)| IrCapture {
                    name: self.make_ident(&name),
                    zig_type,
                    is_mut,
                })
                .collect();

            // Register this as a closure instance
            self.closure_mgr
                .closure_instances
                .insert(instance_name.zig_name.clone());

            // Register the closure struct definition so the Emitter can emit it
            // at module scope.
            self.pending_arrow_structs
                .push(crate::zigir::types::IrClosureStruct {
                    name: struct_name.clone(),
                    captured: ir_captures.clone(),
                    fn_params: params.clone(),
                    return_type: return_type.clone(),
                    typeof_return_body: None,
                    body: body.clone(),
                });

            IrExpr::Closure(IrClosure {
                struct_name,
                captured: ir_captures,
                fn_params: params,
                return_type,
                body,
                instance_name,
            })
        } else {
            // No captures ¡ú IrArrowFn
            IrExpr::ArrowFn(IrArrowFn {
                params,
                return_type,
                body,
                is_concise,
            })
        }
    }

    /// Lower a function expression.
    ///
    /// Like arrow functions, if the function captures variables we produce
    /// an `IrClosure`; otherwise a plain `IrFnExpr`.
    pub(super) fn lower_fn_expr(&mut self, fe: &Function) -> crate::zigir::types::IrExpr {
        use crate::zigir::types::{IrCapture, IrClosure, IrExpr, IrFnExpr};

        let name = fe
            .id
            .as_ref()
            .map(|id| id.name.to_string())
            .unwrap_or_else(|| {
                let idx = self.name_mangler.peek_count("_fn_expr");
                self.name_mangler.next_name("_fn_expr"); // advance counter
                format!("_fn_expr_{}", idx)
            });

        let captured = self.detect_fn_body_captures(fe);
        let return_type = self
            .type_info
            .fn_return_types
            .get(&name)
            .cloned()
            .unwrap_or_else(|| self.infer_fn_expr_return_type(fe, &captured));

        // Enter function context
        let _has_throw = fe
            .body
            .as_ref()
            .is_some_and(|b| Self::has_throw_in_stmts(&b.statements));
        let saved_fn = self.enter_fn(&name, false, Some(return_type.clone()));

        // Set captured variables for identifier rewriting
        let saved_captured = self.closure_mgr.take_captured();
        self.closure_mgr.current_captured = captured
            .iter()
            .map(|(n, t, m)| (n.clone(), t.clone(), *m))
            .collect();

        // Lower params
        let params = self.lower_fn_params(fe, &name);

        // Lower body
        let body = fe
            .body
            .as_ref()
            .map(|b| self.lower_block(&b.statements))
            .unwrap_or_else(|| IrBlock::new(vec![]));

        // Restore
        self.closure_mgr.restore_captured(saved_captured);
        self.exit_fn(saved_fn);

        if !captured.is_empty() {
            // Has captures ¡ú IrClosure
            let struct_name = self.make_ident(&name);
            let instance_name = IrIdent::new(&format!("_{}_inst", name));

            let ir_captures: Vec<IrCapture> = captured
                .into_iter()
                .map(|(n, zig_type, is_mut)| IrCapture {
                    name: self.make_ident(&n),
                    zig_type,
                    is_mut,
                })
                .collect();

            self.closure_mgr
                .closure_instances
                .insert(instance_name.zig_name.clone());

            // Register the closure struct definition so the Emitter can emit it
            // at module scope.
            self.pending_arrow_structs
                .push(crate::zigir::types::IrClosureStruct {
                    name: struct_name.clone(),
                    captured: ir_captures.clone(),
                    fn_params: params.clone(),
                    return_type: return_type.clone(),
                    typeof_return_body: None,
                    body: body.clone(),
                });

            IrExpr::Closure(IrClosure {
                struct_name,
                captured: ir_captures,
                fn_params: params,
                return_type,
                body,
                instance_name,
            })
        } else {
            // No captures ¡ú IrFnExpr
            // Still register a closure struct so the Emitter emits the
            // `const _fn_expr_N = struct { pub fn call() ... }` definition
            // at module scope (the FnExpr reference only emits the name).
            let struct_name = self.make_ident(&name);
            self.pending_arrow_structs
                .push(crate::zigir::types::IrClosureStruct {
                    name: struct_name.clone(),
                    captured: vec![],
                    fn_params: params.clone(),
                    return_type: return_type.clone(),
                    typeof_return_body: None,
                    body: body.clone(),
                });

            IrExpr::FnExpr(IrFnExpr {
                name: Some(self.make_ident(&name)),
                params,
                return_type,
                body,
            })
        }
    }

    /// Collect all identifier names (js_name) referenced in an IR block.
    /// Used to determine which function parameters are unused.
    pub(super) fn collect_ir_idents_in_block(block: &IrBlock) -> std::collections::HashSet<String> {
        let mut idents = std::collections::HashSet::new();
        for stmt in &block.stmts {
            Self::collect_ir_idents_in_stmt(stmt, &mut idents);
        }
        idents
    }

    pub(super) fn collect_ir_idents_in_stmt(
        stmt: &crate::zigir::types::IrStmt,
        idents: &mut std::collections::HashSet<String>,
    ) {
        use crate::zigir::types::IrStmt;
        match stmt {
            IrStmt::VarDecl(vd) => {
                if let Some(init) = &vd.init {
                    Self::collect_ir_idents_in_expr(init, idents);
                }
            }
            IrStmt::Assign { target, value, .. } => {
                Self::collect_ir_idents_in_assign_target(target, idents);
                Self::collect_ir_idents_in_expr(value, idents);
            }
            IrStmt::If { cond, then, else_ } => {
                Self::collect_ir_idents_in_expr(cond, idents);
                for s in &then.stmts {
                    Self::collect_ir_idents_in_stmt(s, idents);
                }
                if let Some(e) = else_ {
                    for s in &e.stmts {
                        Self::collect_ir_idents_in_stmt(s, idents);
                    }
                }
            }
            IrStmt::While { cond, body, .. } | IrStmt::DoWhile { cond, body, .. } => {
                Self::collect_ir_idents_in_expr(cond, idents);
                for s in &body.stmts {
                    Self::collect_ir_idents_in_stmt(s, idents);
                }
            }
            IrStmt::For {
                init,
                cond,
                update,
                body,
                ..
            } => {
                if let Some(s) = init {
                    Self::collect_ir_idents_in_stmt(s, idents);
                }
                if let Some(e) = cond {
                    Self::collect_ir_idents_in_expr(e, idents);
                }
                if let Some(s) = update {
                    Self::collect_ir_idents_in_stmt(s, idents);
                }
                for s in &body.stmts {
                    Self::collect_ir_idents_in_stmt(s, idents);
                }
            }
            IrStmt::ForIn { iterable, body, .. } | IrStmt::ForOf { iterable, body, .. } => {
                Self::collect_ir_idents_in_expr(iterable, idents);
                for s in &body.stmts {
                    Self::collect_ir_idents_in_stmt(s, idents);
                }
            }
            IrStmt::Switch { expr, cases } => {
                Self::collect_ir_idents_in_expr(expr, idents);
                for c in cases {
                    if let Some(e) = &c.test {
                        Self::collect_ir_idents_in_expr(e, idents);
                    }
                    for s in &c.body {
                        Self::collect_ir_idents_in_stmt(s, idents);
                    }
                }
            }
            IrStmt::Try {
                try_block,
                catch_block,
                finally,
                ..
            } => {
                for s in &try_block.stmts {
                    Self::collect_ir_idents_in_stmt(s, idents);
                }
                for s in &catch_block.stmts {
                    Self::collect_ir_idents_in_stmt(s, idents);
                }
                if let Some(f) = finally {
                    for s in &f.stmts {
                        Self::collect_ir_idents_in_stmt(s, idents);
                    }
                }
            }
            IrStmt::Throw { value } => {
                Self::collect_ir_idents_in_expr(value, idents);
            }
            IrStmt::Return { value } => {
                if let Some(e) = value {
                    Self::collect_ir_idents_in_expr(e, idents);
                }
            }
            IrStmt::Break { .. } | IrStmt::Continue { .. } => {}
            IrStmt::Expr(e) => {
                Self::collect_ir_idents_in_expr(e, idents);
            }
            IrStmt::Block(b) => {
                for s in &b.stmts {
                    Self::collect_ir_idents_in_stmt(s, idents);
                }
            }
            IrStmt::CompileError { .. } | IrStmt::Comment(_) => {}
            IrStmt::DestructureDecl(data) => {
                Self::collect_ir_idents_in_expr(&data.init, idents);
                for binding in &data.bindings {
                    if let Some(d) = &binding.default {
                        Self::collect_ir_idents_in_expr(d, idents);
                    }
                }
            }
            IrStmt::NestedFnDecl {
                struct_def,
                instance,
            } => {
                for s in &struct_def.body.stmts {
                    Self::collect_ir_idents_in_stmt(s, idents);
                }
                if let Some(closure) = instance {
                    for cap in &closure.captured {
                        idents.insert(cap.name.js_name.clone());
                    }
                }
            }
        }
    }

    pub(super) fn collect_ir_idents_in_assign_target(
        target: &crate::zigir::types::IrAssignTarget,
        idents: &mut std::collections::HashSet<String>,
    ) {
        use crate::zigir::types::IrAssignTarget;
        match target {
            IrAssignTarget::Ident(name) => {
                idents.insert(name.js_name.clone());
            }
            IrAssignTarget::Member { object, .. } => {
                Self::collect_ir_idents_in_expr(object, idents);
            }
            IrAssignTarget::Index { object, index } => {
                Self::collect_ir_idents_in_expr(object, idents);
                Self::collect_ir_idents_in_expr(index, idents);
            }
            IrAssignTarget::Destructure(bindings) => {
                for b in bindings {
                    if let Some(d) = &b.default {
                        Self::collect_ir_idents_in_expr(d, idents);
                    }
                }
            }
        }
    }

    /// Collect identifier names from an AST expression (used for tracking
    /// references that are optimized away at compile time, e.g. typeof).
    pub(super) fn collect_ast_expr_idents(
        expr: &oxc_ast::ast::Expression,
        idents: &mut HashSet<String>,
    ) {
        use oxc_ast::ast::Expression;
        match expr {
            Expression::Identifier(id) => {
                idents.insert(id.name.to_string());
            }
            Expression::BinaryExpression(be) => {
                Self::collect_ast_expr_idents(&be.left, idents);
                Self::collect_ast_expr_idents(&be.right, idents);
            }
            Expression::UnaryExpression(ue) => {
                Self::collect_ast_expr_idents(&ue.argument, idents);
            }
            Expression::CallExpression(ce) => {
                Self::collect_ast_expr_idents(&ce.callee, idents);
            }
            Expression::StaticMemberExpression(me) => {
                Self::collect_ast_expr_idents(&me.object, idents);
            }
            Expression::ComputedMemberExpression(me) => {
                Self::collect_ast_expr_idents(&me.object, idents);
            }
            Expression::ParenthesizedExpression(pe) => {
                Self::collect_ast_expr_idents(&pe.expression, idents);
            }
            _ => {}
        }
    }

    pub(super) fn collect_ir_idents_in_expr(
        expr: &crate::zigir::types::IrExpr,
        idents: &mut std::collections::HashSet<String>,
    ) {
        use crate::zigir::types::IrExpr;
        match expr {
            IrExpr::Ident(name) => {
                idents.insert(name.js_name.clone());
            }
            IrExpr::Binary { left, right, .. } | IrExpr::Logical { left, right, .. } => {
                Self::collect_ir_idents_in_expr(left, idents);
                Self::collect_ir_idents_in_expr(right, idents);
            }
            IrExpr::Unary { operand, .. }
            | IrExpr::Typeof(operand)
            | IrExpr::Void(operand)
            | IrExpr::Paren(operand)
            | IrExpr::Spread(operand) => {
                Self::collect_ir_idents_in_expr(operand, idents);
            }
            IrExpr::Update { target, .. } => {
                Self::collect_ir_idents_in_assign_target(target, idents);
            }
            IrExpr::Assign { target, value, .. } => {
                Self::collect_ir_idents_in_assign_target(target, idents);
                Self::collect_ir_idents_in_expr(value, idents);
            }
            IrExpr::Call(call) => {
                Self::collect_ir_idents_in_expr(&call.callee, idents);
                for a in &call.args {
                    Self::collect_ir_idents_in_expr(a, idents);
                }
            }
            IrExpr::BuiltinCall(bc) => {
                if let Some(ref obj) = bc.obj_name {
                    idents.insert(obj.clone());
                }
                for a in &bc.args {
                    Self::collect_ir_idents_in_expr(a, idents);
                }
            }
            IrExpr::HostCall(hc) => {
                for a in &hc.args {
                    Self::collect_ir_idents_in_expr(a, idents);
                }
            }
            IrExpr::FieldAccess { object, .. }
            | IrExpr::IndexAccess { object, .. }
            | IrExpr::ComputedField { object, .. } => {
                Self::collect_ir_idents_in_expr(object, idents);
                if let IrExpr::IndexAccess { index, .. } = expr {
                    Self::collect_ir_idents_in_expr(index, idents);
                }
                if let IrExpr::ComputedField { key, .. } = expr {
                    Self::collect_ir_idents_in_expr(key, idents);
                }
            }
            IrExpr::Conditional { cond, then, else_ } => {
                Self::collect_ir_idents_in_expr(cond, idents);
                Self::collect_ir_idents_in_expr(then, idents);
                Self::collect_ir_idents_in_expr(else_, idents);
            }
            IrExpr::Closure(c) => {
                for cap in &c.captured {
                    idents.insert(cap.name.js_name.clone());
                }
                for s in &c.body.stmts {
                    Self::collect_ir_idents_in_stmt(s, idents);
                }
            }
            IrExpr::ArrowFn(a) => {
                for s in &a.body.stmts {
                    Self::collect_ir_idents_in_stmt(s, idents);
                }
            }
            IrExpr::FnExpr(f) => {
                for s in &f.body.stmts {
                    Self::collect_ir_idents_in_stmt(s, idents);
                }
            }
            IrExpr::ArrayLiteral(al) => {
                for e in &al.elements {
                    Self::collect_ir_idents_in_expr(e, idents);
                }
            }
            IrExpr::ObjectLiteral(ol) => {
                use crate::zigir::types::IrObjectItem;
                for item in &ol.items {
                    match item {
                        IrObjectItem::Field(f) => {
                            Self::collect_ir_idents_in_expr(&f.value, idents);
                        }
                        IrObjectItem::Spread(e) => {
                            Self::collect_ir_idents_in_expr(e, idents);
                        }
                    }
                }
            }
            IrExpr::New(ne) => {
                for a in &ne.args {
                    Self::collect_ir_idents_in_expr(a, idents);
                }
            }
            IrExpr::TemplateLiteral { exprs, .. } => {
                for e in exprs {
                    Self::collect_ir_idents_in_expr(e, idents);
                }
            }
            IrExpr::AllocPrint { args, .. } => {
                for a in args {
                    Self::collect_ir_idents_in_expr(a, idents);
                }
            }
            IrExpr::BlockExpr { body, result, .. } => {
                for s in body {
                    Self::collect_ir_idents_in_stmt(s, idents);
                }
                Self::collect_ir_idents_in_expr(result, idents);
            }
            IrExpr::Sequence(exprs) => {
                for e in exprs {
                    Self::collect_ir_idents_in_expr(e, idents);
                }
            }
            IrExpr::Await(ae) => {
                Self::collect_ir_idents_in_expr(&ae.callee, idents);
                for a in &ae.args {
                    Self::collect_ir_idents_in_expr(a, idents);
                }
            }
            IrExpr::ArrayCallbackInline(inline_data) => {
                for s in &inline_data.body {
                    Self::collect_ir_idents_in_stmt(s, idents);
                }
                if let Some(ref init) = inline_data.reduce_init {
                    Self::collect_ir_idents_in_expr(init, idents);
                }
            }
            IrExpr::ArrayMethodInline(inline_data) => {
                for arg in &inline_data.args {
                    Self::collect_ir_idents_in_expr(arg, idents);
                }
            }
            IrExpr::OptionalChain { object, body, .. } => {
                Self::collect_ir_idents_in_expr(object, idents);
                Self::collect_ir_idents_in_expr(body, idents);
            }
            IrExpr::PowExpr { base, exp, .. } => {
                Self::collect_ir_idents_in_expr(base, idents);
                Self::collect_ir_idents_in_expr(exp, idents);
            }
            IrExpr::IntLiteral(_)
            | IrExpr::FloatLiteral(_)
            | IrExpr::StringLiteral(_)
            | IrExpr::BoolLiteral(_)
            | IrExpr::BigIntLiteral(_)
            | IrExpr::Null
            | IrExpr::Undefined
            | IrExpr::This
            | IrExpr::CompileError { .. } => {}
        }
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
        // This must match Codegen's logic:
        //   Str¡ú{s}, I64/F64¡ú{d}, Bool¡ú{}, other¡úexpr_is_string?{s}:{}
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

    /// Lower an await expression.
    pub(super) fn lower_await(&mut self, ae: &AwaitExpression) -> crate::zigir::types::IrExpr {
        let task_var = IrIdent::new(&self.name_mangler.next_name("_t"));
        let block_label = format!("blk_{}", self.name_mangler.peek_count("_t"));

        // Check if this is an async host function call
        if let Expression::CallExpression(call) = &ae.argument {
            if let Expression::Identifier(id) = &call.callee {
                let name = id.name.as_str();
                if self.async_host_fns.contains(name) {
                    // Host async function: emit as host.{name}_async
                    let args: Vec<_> = call
                        .arguments
                        .iter()
                        .filter_map(|a| a.as_expression().map(|e| self.lower_expr(e)))
                        .collect();
                    return crate::zigir::types::IrExpr::Await(crate::zigir::types::IrAwaitExpr {
                        task_var,
                        callee: Box::new(crate::zigir::types::IrExpr::Ident(IrIdent::new(name))),
                        args,
                        is_host_async: true,
                        block_label,
                    });
                }
            }
            // Regular async call (non-host)
            let callee = self.lower_expr(&call.callee);
            let args: Vec<_> = call
                .arguments
                .iter()
                .filter_map(|a| a.as_expression().map(|e| self.lower_expr(e)))
                .collect();
            return crate::zigir::types::IrExpr::Await(crate::zigir::types::IrAwaitExpr {
                task_var,
                callee: Box::new(callee),
                args,
                is_host_async: false,
                block_label,
            });
        }

        // Non-call await (unusual but valid JS)
        let argument = self.lower_expr(&ae.argument);
        crate::zigir::types::IrExpr::Await(crate::zigir::types::IrAwaitExpr {
            task_var,
            callee: Box::new(argument),
            args: vec![],
            is_host_async: false,
            block_label,
        })
    }

    /// Lower a new expression.
    pub(super) fn lower_new(&mut self, ne: &NewExpression) -> crate::zigir::types::IrExpr {
        use crate::zigir::kinds::NewConstructor;

        // Determine constructor kind from callee
        let constructor = match &ne.callee {
            Expression::Identifier(id) => match id.name.as_str() {
                "Map" => NewConstructor::Map,
                "Set" => NewConstructor::Set,
                "Date" => {
                    // Determine DateConstructorKind from arguments
                    let kind = if ne.arguments.is_empty() {
                        crate::zigir::kinds::DateConstructorKind::Now
                    } else if ne.arguments.len() >= 2 {
                        crate::zigir::kinds::DateConstructorKind::FromComponents
                    } else if let Some(first_arg) = ne.arguments.first()
                        && let Some(expr) = first_arg.as_expression()
                    {
                        // Detect if argument is a string literal
                        let is_string = matches!(expr, Expression::StringLiteral(_));
                        if is_string {
                            crate::zigir::kinds::DateConstructorKind::FromString
                        } else {
                            crate::zigir::kinds::DateConstructorKind::FromMillis
                        }
                    } else {
                        crate::zigir::kinds::DateConstructorKind::Now
                    };
                    NewConstructor::Date(kind)
                }
                "RegExp" => NewConstructor::RegExp,
                "Int8Array" | "Uint8Array" | "Uint8ClampedArray" | "Int16Array" | "Uint16Array"
                | "Int32Array" | "Uint32Array" | "Float32Array" | "Float64Array" => {
                    let kind = match id.name.as_str() {
                        "Int8Array" => crate::zigir::kinds::TypedArrayKind::Int8Array,
                        "Uint8Array" => crate::zigir::kinds::TypedArrayKind::Uint8Array,
                        "Uint8ClampedArray" => {
                            crate::zigir::kinds::TypedArrayKind::Uint8ClampedArray
                        }
                        "Int16Array" => crate::zigir::kinds::TypedArrayKind::Int16Array,
                        "Uint16Array" => crate::zigir::kinds::TypedArrayKind::Uint16Array,
                        "Int32Array" => crate::zigir::kinds::TypedArrayKind::Int32Array,
                        "Uint32Array" => crate::zigir::kinds::TypedArrayKind::Uint32Array,
                        "Float32Array" => crate::zigir::kinds::TypedArrayKind::Float32Array,
                        "Float64Array" => crate::zigir::kinds::TypedArrayKind::Float64Array,
                        _ => crate::zigir::kinds::TypedArrayKind::Float64Array,
                    };
                    NewConstructor::TypedArray(kind)
                }
                "Error" => NewConstructor::Error("Error".to_string()),
                "TypeError" => NewConstructor::Error("TypeError".to_string()),
                "RangeError" => NewConstructor::Error("RangeError".to_string()),
                // Wrapper constructors ¡ª emit argument value directly (no wrapper object in Zig)
                "String" => {
                    if let Some(first_arg) = ne.arguments.first()
                        && let Some(expr) = first_arg.as_expression()
                    {
                        return self.lower_expr(expr);
                    } else {
                        return crate::zigir::types::IrExpr::StringLiteral("".to_string());
                    }
                }
                "Number" => {
                    if let Some(first_arg) = ne.arguments.first()
                        && let Some(expr) = first_arg.as_expression()
                    {
                        return self.lower_expr(expr);
                    } else {
                        return crate::zigir::types::IrExpr::IntLiteral(0);
                    }
                }
                "Boolean" => {
                    if let Some(first_arg) = ne.arguments.first()
                        && let Some(expr) = first_arg.as_expression()
                    {
                        return self.lower_expr(expr);
                    } else {
                        return crate::zigir::types::IrExpr::BoolLiteral(false);
                    }
                }
                name if self.class_names.contains(name) => NewConstructor::Class(name.to_string()),
                // Known-unsupported constructors ¡ú structured Unsupported (Emitter generates proper @compileError)
                "ArrayBuffer" | "SharedArrayBuffer" | "Function" | "Promise" | "WeakMap"
                | "WeakSet" | "DataView" => {
                    NewConstructor::Unsupported(id.name.as_str().to_string())
                }
                other => {
                    let span = oxc_span::GetSpan::span(ne);
                    return crate::zigir::types::IrExpr::CompileError {
                        span: self.span_to_source_span(span),
                        msg: format!("Unsupported NewExpression: new {}()", other),
                    };
                }
            },
            _ => {
                let span = oxc_span::GetSpan::span(ne);
                return crate::zigir::types::IrExpr::CompileError {
                    span: self.span_to_source_span(span),
                    msg: "Unsupported NewExpression".to_string(),
                };
            }
        };

        let args: Vec<crate::zigir::types::IrExpr> = ne
            .arguments
            .iter()
            .map(|arg| match arg {
                Argument::SpreadElement(se) => {
                    crate::zigir::types::IrExpr::Spread(Box::new(self.lower_expr(&se.argument)))
                }
                _ => {
                    let expr = arg.as_expression().unwrap();
                    self.lower_expr(expr)
                }
            })
            .collect();

        let result_type = match &constructor {
            NewConstructor::Map => ZigType::NamedStruct("Map".to_string()),
            NewConstructor::Set => ZigType::NamedStruct("Set".to_string()),
            NewConstructor::Date(_) => ZigType::NamedStruct("JsDate".to_string()),
            NewConstructor::RegExp => ZigType::JsAny,
            NewConstructor::TypedArray(_) => ZigType::NamedStruct("TypedArray".to_string()),
            NewConstructor::Class(name) => ZigType::NamedStruct(name.clone()),
            NewConstructor::Error(_) => ZigType::JsAny,
            _ => ZigType::JsAny,
        };

        crate::zigir::types::IrExpr::New(crate::zigir::types::IrNewExpr {
            constructor,
            args,
            result_type,
        })
    }
}
