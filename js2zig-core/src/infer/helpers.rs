// native_proto/infer/helpers.rs
// Public utility functions used by Codegen and TypeInferrer.

use oxc_ast::ast::*;
use std::collections::HashSet;

/// Extract variable name from a binding pattern.
pub fn binding_name<'a>(pattern: &BindingPattern<'a>) -> Option<&'a str> {
    match pattern {
        BindingPattern::BindingIdentifier(id) => Some(id.name.as_str()),
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
        // Literals and constants never depend on anytype params
        Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::RegExpLiteral(_)
        | Expression::TemplateLiteral(_)
        | Expression::FunctionExpression(_)
        | Expression::ArrowFunctionExpression(_) => false,
        // Conservative: assume no dependency for unhandled expression types
        _ => false,
    }
}

/// Find the first return expression in a function body (depth-first).
/// Used by Codegen to generate `@TypeOf(first_return_expr)` for AnytypeReturn.
pub fn find_first_return_expr<'a>(fd: &'a Function<'a>) -> Option<&'a Expression<'a>> {
    let body = fd.body.as_ref()?;
    for stmt in &body.statements {
        if let Some(expr) = find_first_return_in_stmt(stmt) {
            return Some(expr);
        }
    }
    None
}

fn find_first_return_in_stmt<'a>(stmt: &'a Statement<'a>) -> Option<&'a Expression<'a>> {
    match stmt {
        Statement::ReturnStatement(rs) => rs.argument.as_ref(),
        Statement::BlockStatement(bs) => {
            for s in &bs.body {
                if let Some(e) = find_first_return_in_stmt(s) {
                    return Some(e);
                }
            }
            None
        }
        Statement::IfStatement(is) => find_first_return_in_stmt(&is.consequent).or_else(|| {
            is.alternate
                .as_ref()
                .and_then(|a| find_first_return_in_stmt(a))
        }),
        _ => None,
    }
}
