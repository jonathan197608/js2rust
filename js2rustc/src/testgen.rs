use oxc_ast::ast::*;
use std::collections::HashSet;

/// A single test case extracted from a test_* variable
#[derive(Debug, Clone)]
pub struct TestCase {
    /// The test_ variable name (e.g. "test_add")
    pub var_name: String,
    /// The expression text reconstructed from AST (e.g. "add(3, 5)")
    pub expr_text: String,
}

/// Extract test_* variable definitions from the AST.
/// Reconstructs expression text from AST nodes (no source/span needed).
pub fn extract_test_cases(program: &Program) -> Vec<TestCase> {
    let mut cases = Vec::new();

    for stmt in &program.body {
        let Statement::VariableDeclaration(decl) = stmt else {
            continue;
        };
        for declarator in &decl.declarations {
            let name = match &declarator.id {
                BindingPattern::BindingIdentifier(id) => id.name.as_str().to_string(),
                _ => continue,
            };
            if !name.starts_with("test_") {
                continue;
            }
            let Some(ref init) = declarator.init else {
                continue;
            };
            let expr_text = expr_to_string(init);
            cases.push(TestCase {
                var_name: name,
                expr_text,
            });
        }
    }

    cases
}

/// Generate Zig smoke-test code from test cases.
/// Each test simply calls the expression and discards the result,
/// verifying that the translated Zig code compiles and runs without crashing.
///
/// `closure_fns`: names of functions that return closure structs (needs .call() syntax)
pub fn generate_test_code(
    test_cases: &[TestCase],
    closure_fns: &HashSet<&str>,
) -> String {
    if test_cases.is_empty() {
        return String::new();
    }

    let mut out = String::new();
    out.push_str("// Auto-generated smoke tests from test_* variables\n\n");

    for tc in test_cases {
        let test_name = tc.var_name.strip_prefix("test_").unwrap_or(&tc.var_name);

        // Transform closure calls: makeAdder(10)(5) → makeAdder(10).call(5)
        let call_expr = rewrite_closure_calls(&tc.expr_text, closure_fns);

        out.push_str(&format!(
            "test \"{}\" {{\n",
            sanitize_name(test_name)
        ));
        out.push_str("    var arena = std.heap.ArenaAllocator.init(std.testing.allocator);\n");
        out.push_str("    defer arena.deinit();\n");
        out.push_str("    const allocator = arena.allocator();\n");
        out.push_str("    init_js2rust(allocator);\n");
        out.push_str("    defer deinit_js2rust();\n");
        out.push_str(&format!("    _ = {};\n", call_expr));
        out.push_str("}\n\n");
    }

    out
}

/// Sanitize a name for use in Zig test strings
fn sanitize_name(name: &str) -> String {
    let mut s = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            s.push(ch);
        } else {
            s.push('_');
        }
    }
    s
}

/// Reconstruct a JS expression as a string from its AST node.
/// Handles the expression types that appear in test_ variable initializers
/// (function calls, numeric/string/boolean literals, identifiers).
fn expr_to_string(expr: &Expression) -> String {
    match expr {
        Expression::ArrowFunctionExpression(arrow) => {
            // () => expr → extract the expression from the body statement
            if let Statement::ExpressionStatement(es) = &arrow.body.statements[0] {
                expr_to_string(&es.expression)
            } else {
                "<expr>".to_string()
            }
        }
        Expression::CallExpression(call) => {
            let callee_str = expr_to_string(&call.callee);
            let args_str: Vec<String> = call
                .arguments
                .iter()
                .map(|arg| match arg {
                    Argument::SpreadElement(spread) => {
                        format!("...{}", expr_to_string(&spread.argument))
                    }
                    _ => arg
                        .as_expression()
                        .map(|e| expr_to_string(e))
                        .unwrap_or_else(|| "_".to_string()),
                })
                .collect();
            format!("{}({})", callee_str, args_str.join(", "))
        }
        Expression::Identifier(id) => id.name.to_string(),
        Expression::NumericLiteral(lit) => {
            if let Some(raw) = &lit.raw {
                raw.to_string()
            } else if lit.value.fract() == 0.0 {
                format!("{}", lit.value as i64)
            } else {
                format!("{}", lit.value)
            }
        }
        Expression::StringLiteral(lit) => {
            format!("\"{}\"", lit.value)
        }
        Expression::BooleanLiteral(lit) => lit.value.to_string(),
        Expression::NullLiteral(_) => "null".to_string(),
        Expression::ArrayExpression(arr) => {
            let elements: Vec<String> = arr
                .elements
                .iter()
                .filter_map(|elem| {
                    elem.as_expression().map(|e| expr_to_string(e))
                })
                .collect();
            format!("&[_]i64{{ {} }}", elements.join(", "))
        }
        Expression::UnaryExpression(unary) => {
            let arg = expr_to_string(&unary.argument);
            match unary.operator {
                UnaryOperator::UnaryNegation => format!("-{}", arg),
                UnaryOperator::LogicalNot => format!("!{}", arg),
                UnaryOperator::BitwiseNot => format!("~{}", arg),
                _ => arg,
            }
        }
        Expression::BinaryExpression(bin) => {
            let left = expr_to_string(&bin.left);
            let right = expr_to_string(&bin.right);
            let op = match bin.operator {
                BinaryOperator::Addition => "+",
                BinaryOperator::Subtraction => "-",
                BinaryOperator::Multiplication => "*",
                BinaryOperator::Division => "/",
                BinaryOperator::Remainder => "%",
                BinaryOperator::Equality => "==",
                BinaryOperator::Inequality => "!=",
                BinaryOperator::StrictEquality => "===",
                BinaryOperator::StrictInequality => "!==",
                BinaryOperator::LessThan => "<",
                BinaryOperator::LessEqualThan => "<=",
                BinaryOperator::GreaterThan => ">",
                BinaryOperator::GreaterEqualThan => ">=",
                BinaryOperator::ShiftLeft => "<<",
                BinaryOperator::ShiftRight => ">>",
                BinaryOperator::ShiftRightZeroFill => ">>>",
                BinaryOperator::BitwiseAnd => "&",
                BinaryOperator::BitwiseOR => "|",
                BinaryOperator::BitwiseXOR => "^",
                _ => "??",
            };
            format!("({} {} {})", left, op, right)
        }
        Expression::ConditionalExpression(cond) => {
            format!("({} ? {} : {})",
                expr_to_string(&cond.test),
                expr_to_string(&cond.consequent),
                expr_to_string(&cond.alternate))
        }
        Expression::LogicalExpression(logic) => {
            let left = expr_to_string(&logic.left);
            let right = expr_to_string(&logic.right);
            let op = match logic.operator {
                LogicalOperator::And => "&&",
                LogicalOperator::Or => "||",
                LogicalOperator::Coalesce => "??",
            };
            format!("({} {} {})", left, op, right)
        }
        Expression::ObjectExpression(obj) => {
            let props: Vec<String> = obj.properties.iter().filter_map(|prop| {
                match prop {
                    ObjectPropertyKind::ObjectProperty(p) => {
                        let key = match &p.key {
                            PropertyKey::StaticIdentifier(id) => id.name.to_string(),
                            _ => "?".to_string(),
                        };
                        let val = expr_to_string(&p.value);
                        Some(format!(".{} = {}", key, val))
                    }
                    _ => None,
                }
            }).collect();
            format!(".{{ {} }}", props.join(", "))
        }
        Expression::ParenthesizedExpression(parens) => {
            format!("({})", expr_to_string(&parens.expression))
        }
        Expression::TemplateLiteral(tl) => {
            if tl.expressions.is_empty() && tl.quasis.len() == 1 {
                if let Some(cooked) = &tl.quasis[0].value.cooked {
                    return format!("\"{}\"", cooked);
                }
            }
            // For template literals with expressions, just reconstruct the args
            let mut parts = Vec::new();
            for (i, quasi) in tl.quasis.iter().enumerate() {
                if let Some(cooked) = &quasi.value.cooked {
                    if !cooked.is_empty() {
                        parts.push(format!("\"{}\"", cooked));
                    }
                }
                if i < tl.expressions.len() {
                    parts.push(expr_to_string(&tl.expressions[i]));
                }
            }
            parts.join(", ")
        }
        Expression::AssignmentExpression(assign) => {
            format!("({} = {})", expr_to_string_simple(&assign.left), expr_to_string(&assign.right))
        }
        _ => "<expr>".to_string(),
    }
}

/// Simplified expression to string for assignment targets
fn expr_to_string_simple(target: &AssignmentTarget) -> String {
    match target {
        AssignmentTarget::AssignmentTargetIdentifier(id) => id.name.to_string(),
        _ => "_".to_string(),
    }
}

/// Rewrite closure function calls: `makeAdder(10)(5)` → `makeAdder(10).call(5)`.
/// Only applies when the base function name is in `closure_fns`.
fn rewrite_closure_calls(expr: &str, closure_fns: &HashSet<&str>) -> String {
    // Check if the expression starts with a known closure function name followed by '('
    let first_paren = match expr.find('(') {
        Some(pos) => pos,
        None => return expr.to_string(),
    };
    let fn_name = &expr[..first_paren];
    if !closure_fns.contains(fn_name) {
        return expr.to_string();
    }

    // Find the matching closing paren for the first call
    if let Some(close_pos) = find_matching_paren(expr, first_paren) {
        let after_first_call = &expr[close_pos + 1..];
        if after_first_call.starts_with('(') {
            // Replace subsequent calls with .call(...)
            let prefix = &expr[..=close_pos];
            let rest = after_first_call.replacen('(', ".call(", 1);
            return format!("{}{}", prefix, rest);
        }
    }

    expr.to_string()
}

/// Find the position of the matching closing paren for an opening paren at `open_pos`
fn find_matching_paren(s: &str, open_pos: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    if open_pos >= bytes.len() || bytes[open_pos] != b'(' {
        return None;
    }
    let mut depth = 0;
    for (i, &ch) in bytes.iter().enumerate().skip(open_pos) {
        match ch {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_name() {
        assert_eq!(sanitize_name("add_main"), "add_main");
        assert_eq!(sanitize_name("add-main"), "add_main");
        assert_eq!(sanitize_name("数学运算"), "____");
    }

    #[test]
    fn test_find_matching_paren() {
        assert_eq!(find_matching_paren("add(3, 5)", 3), Some(8));
        assert_eq!(find_matching_paren("add(3, max(1, 2))", 3), Some(16));
        assert_eq!(find_matching_paren("no_paren", 0), None);
    }

    #[test]
    fn test_rewrite_closure_calls() {
        let mut fns = HashSet::new();
        fns.insert("makeAdder");
        assert_eq!(
            rewrite_closure_calls("makeAdder(10)(5)", &fns),
            "makeAdder(10).call(5)"
        );
        // Not a closure function — no rewrite
        let empty: HashSet<&str> = HashSet::new();
        assert_eq!(
            rewrite_closure_calls("add(1, 2)", &empty),
            "add(1, 2)"
        );
    }
}
