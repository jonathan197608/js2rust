// native_proto/infer/helpers.rs
// Public utility functions used by Codegen.

use oxc_ast::ast::*;

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
