// native_proto/infer/helpers.rs
// Public utility functions used by TypeInferrer and ZigIR Lowerer.

use oxc_ast::ast::*;
use std::collections::HashSet;

/// Extract variable name from a binding pattern.
pub fn binding_name<'a>(pattern: &BindingPattern<'a>) -> Option<&'a str> {
    match pattern {
        BindingPattern::BindingIdentifier(id) => Some(id.name.as_str()),
        BindingPattern::AssignmentPattern(ap) => binding_name(&ap.left),
        _ => None,
    }
}

/// Extract the identifier name from an Expression if it is an Identifier.
pub fn extract_expr_identifier_name(expr: &Expression) -> Option<String> {
    match expr {
        Expression::Identifier(id) => Some(id.name.to_string()),
        _ => None,
    }
}

/// Recursively check whether an expression depends on any anytype parameter.
/// Used to decide whether a function's return type should be `@TypeOf` (AnytypeReturn)
/// rather than defaulting to I64.
pub fn expr_depends_on_anytype(expr: &Expression, anytype_params: &HashSet<String>) -> bool {
    match expr {
        Expression::Identifier(id) => anytype_params.contains(id.name.as_str()),
        Expression::BinaryExpression(be) => {
            expr_depends_on_anytype(&be.left, anytype_params)
                || expr_depends_on_anytype(&be.right, anytype_params)
        }
        Expression::UnaryExpression(ue) => expr_depends_on_anytype(&ue.argument, anytype_params),
        Expression::ParenthesizedExpression(pe) => {
            expr_depends_on_anytype(&pe.expression, anytype_params)
        }
        Expression::CallExpression(ce) => {
            expr_depends_on_anytype(&ce.callee, anytype_params)
                || ce.arguments.iter().any(|a| {
                    a.as_expression()
                        .is_some_and(|e| expr_depends_on_anytype(e, anytype_params))
                })
        }
        Expression::ConditionalExpression(ce) => {
            expr_depends_on_anytype(&ce.consequent, anytype_params)
                || expr_depends_on_anytype(&ce.alternate, anytype_params)
        }
        Expression::LogicalExpression(le) => {
            expr_depends_on_anytype(&le.left, anytype_params)
                || expr_depends_on_anytype(&le.right, anytype_params)
        }
        Expression::ArrayExpression(ae) => ae.elements.iter().any(|e| {
            e.as_expression()
                .is_some_and(|e| expr_depends_on_anytype(e, anytype_params))
        }),
        Expression::ObjectExpression(oe) => oe.properties.iter().any(|p| match p {
            ObjectPropertyKind::ObjectProperty(op) => {
                expr_depends_on_anytype(&op.value, anytype_params)
            }
            ObjectPropertyKind::SpreadProperty(sp) => {
                expr_depends_on_anytype(&sp.argument, anytype_params)
            }
        }),
        Expression::ComputedMemberExpression(cme) => {
            expr_depends_on_anytype(&cme.object, anytype_params)
                || expr_depends_on_anytype(&cme.expression, anytype_params)
        }
        Expression::StaticMemberExpression(sme) => {
            expr_depends_on_anytype(&sme.object, anytype_params)
        }
        Expression::AssignmentExpression(ae) => expr_depends_on_anytype(&ae.right, anytype_params),
        Expression::SequenceExpression(se) => se
            .expressions
            .last()
            .is_some_and(|e| expr_depends_on_anytype(e, anytype_params)),
        // Constructor calls: arguments may depend on anytype params.
        Expression::NewExpression(ne) => {
            expr_depends_on_anytype(&ne.callee, anytype_params)
                || ne.arguments.iter().any(|a| {
                    a.as_expression()
                        .is_some_and(|e| expr_depends_on_anytype(e, anytype_params))
                })
        }
        // Tagged templates: tag and interpolated expressions may depend on anytype.
        Expression::TaggedTemplateExpression(tte) => {
            expr_depends_on_anytype(&tte.tag, anytype_params)
                || tte
                    .quasi
                    .expressions
                    .iter()
                    .any(|e| expr_depends_on_anytype(e, anytype_params))
        }
        // Optional/chain expression wraps a ChainElement (call or member access).
        Expression::ChainExpression(ce) => match &ce.expression {
            ChainElement::CallExpression(cce) => {
                expr_depends_on_anytype(&cce.callee, anytype_params)
                    || cce.arguments.iter().any(|a| {
                        a.as_expression()
                            .is_some_and(|e| expr_depends_on_anytype(e, anytype_params))
                    })
            }
            ChainElement::StaticMemberExpression(sme) => {
                expr_depends_on_anytype(&sme.object, anytype_params)
            }
            ChainElement::ComputedMemberExpression(cme) => {
                expr_depends_on_anytype(&cme.object, anytype_params)
                    || expr_depends_on_anytype(&cme.expression, anytype_params)
            }
            _ => false,
        },
        // ++/-- operand is a SimpleAssignmentTarget (identifier or member).
        Expression::UpdateExpression(ue) => match &ue.argument {
            SimpleAssignmentTarget::AssignmentTargetIdentifier(id) => {
                anytype_params.contains(id.name.as_str())
            }
            SimpleAssignmentTarget::StaticMemberExpression(sme) => {
                expr_depends_on_anytype(&sme.object, anytype_params)
            }
            SimpleAssignmentTarget::ComputedMemberExpression(cme) => {
                expr_depends_on_anytype(&cme.object, anytype_params)
                    || expr_depends_on_anytype(&cme.expression, anytype_params)
            }
            SimpleAssignmentTarget::PrivateFieldExpression(pfe) => {
                expr_depends_on_anytype(&pfe.object, anytype_params)
            }
            _ => false,
        },
        // Literals and constants never depend on anytype params
        Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::RegExpLiteral(_)
        | Expression::FunctionExpression(_)
        | Expression::ArrowFunctionExpression(_) => false,
        // Template literal: check interpolated expressions
        Expression::TemplateLiteral(tl) => tl
            .expressions
            .iter()
            .any(|e| expr_depends_on_anytype(e, anytype_params)),
        // Await expression: check the awaited value
        Expression::AwaitExpression(ae) => expr_depends_on_anytype(&ae.argument, anytype_params),
        // Private field access: check the receiver
        Expression::PrivateFieldExpression(pfe) => {
            expr_depends_on_anytype(&pfe.object, anytype_params)
        }
        // Conservative: assume no dependency for unhandled expression types
        _ => false,
    }
}
