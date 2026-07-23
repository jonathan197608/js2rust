// infer/ast_walk.rs
// Shared structural traversal helpers for the oxc AST tree.
//
// Each `for_each_*_child` function visits the *direct* children of a node
// and invokes the appropriate callback. These are single-level traversals;
// the caller decides whether and how to recurse.
//
// Eliminates duplicated AST walker logic across:
// - closure.rs: detect_mutated_in_stmt/expr, collect_idents_from_stmt/expr
// - passes.rs: collect_idents_from_stmt/expr
// - fn_types.rs: detect_string_param_usage/in_expr

use oxc_ast::ast::*;

/// Visit direct children of a `Statement`.
///
/// Callbacks:
/// - `on_stmt`: child statements (if body, while body, for body, etc.)
/// - `on_expr`: child expressions (test conditions, init values, etc.)
/// - `on_var_decl`: VariableDeclaration nodes (to separately handle init expressions)
///
/// The walker does NOT recurse — it only visits direct children.
/// For full traversal, call `for_each_stmt_child` recursively from within callbacks.
pub fn for_each_stmt_child(
    stmt: &Statement,
    on_stmt: &mut impl FnMut(&Statement),
    on_expr: &mut impl FnMut(&Expression),
    on_var_decl: &mut impl FnMut(&VariableDeclaration),
) {
    match stmt {
        Statement::ExpressionStatement(es) => {
            on_expr(&es.expression);
        }
        Statement::ReturnStatement(rs) => {
            if let Some(arg) = &rs.argument {
                on_expr(arg);
            }
        }
        Statement::VariableDeclaration(vd) => {
            on_var_decl(vd);
        }
        Statement::IfStatement(is) => {
            on_expr(&is.test);
            on_stmt(&is.consequent);
            if let Some(alt) = &is.alternate {
                on_stmt(alt);
            }
        }
        Statement::WhileStatement(ws) => {
            on_expr(&ws.test);
            on_stmt(&ws.body);
        }
        Statement::DoWhileStatement(dws) => {
            on_stmt(&dws.body);
            on_expr(&dws.test);
        }
        Statement::ForStatement(fs) => {
            if let Some(init) = &fs.init {
                match init {
                    ForStatementInit::VariableDeclaration(vd) => on_var_decl(vd),
                    other => {
                        if let Some(expr) = other.as_expression() {
                            on_expr(expr);
                        }
                    }
                }
            }
            if let Some(test) = &fs.test {
                on_expr(test);
            }
            if let Some(update) = &fs.update {
                on_expr(update);
            }
            on_stmt(&fs.body);
        }
        Statement::ForOfStatement(fos) => {
            if let ForStatementLeft::VariableDeclaration(vd) = &fos.left {
                on_var_decl(vd);
            }
            on_expr(&fos.right);
            on_stmt(&fos.body);
        }
        Statement::ForInStatement(fis) => {
            on_expr(&fis.right);
            on_stmt(&fis.body);
        }
        Statement::SwitchStatement(ss) => {
            on_expr(&ss.discriminant);
            for case in &ss.cases {
                if let Some(test) = &case.test {
                    on_expr(test);
                }
                for s in &case.consequent {
                    on_stmt(s);
                }
            }
        }
        Statement::TryStatement(ts) => {
            for s in &ts.block.body {
                on_stmt(s);
            }
            if let Some(handler) = &ts.handler {
                for s in &handler.body.body {
                    on_stmt(s);
                }
            }
            if let Some(finalizer) = &ts.finalizer {
                for s in &finalizer.body {
                    on_stmt(s);
                }
            }
        }
        Statement::BlockStatement(bs) => {
            for s in &bs.body {
                on_stmt(s);
            }
        }
        Statement::LabeledStatement(ls) => {
            on_stmt(&ls.body);
        }
        Statement::ThrowStatement(ts) => {
            on_expr(&ts.argument);
        }
        // FunctionDeclaration, ClassDeclaration, Export*, etc.
        // are not recursed by any current consumer; add as needed.
        _ => {}
    }
}

/// Visit direct children of an `Expression`.
///
/// Callbacks:
/// - `on_expr`: child expressions (operands, arguments, object/key, etc.)
/// - `on_ident`: identifier leaf nodes (for collectors that need names)
/// - `on_target`: assignment target side of AssignmentExpression
/// - `on_fn_scope`: function/arrow expression scope boundary (params + body stmts)
///
/// The walker does NOT recurse — it only visits direct children.
/// For full traversal, call `for_each_expr_child` recursively from within callbacks.
#[allow(clippy::too_many_arguments)]
pub fn for_each_expr_child(
    expr: &Expression,
    on_expr: &mut impl FnMut(&Expression),
    on_ident: &mut impl FnMut(&str),
    on_target: &mut impl FnMut(&AssignmentTarget),
    on_simple_target: &mut impl FnMut(&SimpleAssignmentTarget),
    on_fn_scope: &mut impl FnMut(&[FormalParameter], &oxc_allocator::Vec<'_, Statement>),
) {
    match expr {
        Expression::Identifier(id) => {
            on_ident(id.name.as_str());
        }
        Expression::BinaryExpression(be) => {
            on_expr(&be.left);
            on_expr(&be.right);
        }
        Expression::LogicalExpression(le) => {
            on_expr(&le.left);
            on_expr(&le.right);
        }
        Expression::UnaryExpression(ue) => {
            on_expr(&ue.argument);
        }
        Expression::UpdateExpression(ue) => {
            on_simple_target(&ue.argument);
        }
        Expression::AssignmentExpression(ae) => {
            on_target(&ae.left);
            on_expr(&ae.right);
        }
        Expression::CallExpression(ce) => {
            on_expr(&ce.callee);
            for arg in &ce.arguments {
                match arg {
                    Argument::SpreadElement(se) => on_expr(&se.argument),
                    _ => {
                        if let Some(e) = arg.as_expression() {
                            on_expr(e);
                        }
                    }
                }
            }
        }
        Expression::NewExpression(ne) => {
            on_expr(&ne.callee);
            for arg in &ne.arguments {
                match arg {
                    Argument::SpreadElement(se) => on_expr(&se.argument),
                    _ => {
                        if let Some(e) = arg.as_expression() {
                            on_expr(e);
                        }
                    }
                }
            }
        }
        Expression::ConditionalExpression(ce) => {
            on_expr(&ce.test);
            on_expr(&ce.consequent);
            on_expr(&ce.alternate);
        }
        Expression::StaticMemberExpression(sme) => {
            on_expr(&sme.object);
        }
        Expression::ComputedMemberExpression(cme) => {
            on_expr(&cme.object);
            on_expr(&cme.expression);
        }
        Expression::ParenthesizedExpression(pe) => {
            on_expr(&pe.expression);
        }
        Expression::SequenceExpression(se) => {
            for e in &se.expressions {
                on_expr(e);
            }
        }
        Expression::TemplateLiteral(tl) => {
            for e in &tl.expressions {
                on_expr(e);
            }
        }
        Expression::AwaitExpression(ae) => {
            on_expr(&ae.argument);
        }
        Expression::FunctionExpression(fe) => {
            if let Some(body) = &fe.body {
                on_fn_scope(&fe.params.items, &body.statements);
            }
        }
        Expression::ArrowFunctionExpression(af) => {
            on_fn_scope(&af.params.items, &af.body.statements);
        }
        Expression::ArrayExpression(ae) => {
            for elem in &ae.elements {
                match elem {
                    ArrayExpressionElement::SpreadElement(se) => on_expr(&se.argument),
                    _ => {
                        if let Some(e) = elem.as_expression() {
                            on_expr(e);
                        }
                    }
                }
            }
        }
        Expression::ObjectExpression(oe) => {
            for prop in &oe.properties {
                match prop {
                    ObjectPropertyKind::ObjectProperty(op) => {
                        if op.computed
                            && let Some(expr) = op.key.as_expression()
                        {
                            on_expr(expr);
                        }
                        on_expr(&op.value);
                    }
                    ObjectPropertyKind::SpreadProperty(sp) => {
                        on_expr(&sp.argument);
                    }
                }
            }
        }
        Expression::ChainExpression(ce) => match &ce.expression {
            ChainElement::CallExpression(call_ce) => {
                on_expr(&call_ce.callee);
                for arg in &call_ce.arguments {
                    match arg {
                        Argument::SpreadElement(se) => on_expr(&se.argument),
                        _ => {
                            if let Some(e) = arg.as_expression() {
                                on_expr(e);
                            }
                        }
                    }
                }
            }
            ChainElement::StaticMemberExpression(sme) => {
                on_expr(&sme.object);
            }
            ChainElement::ComputedMemberExpression(cme) => {
                on_expr(&cme.object);
                on_expr(&cme.expression);
            }
            _ => {}
        },
        Expression::TaggedTemplateExpression(tte) => {
            on_expr(&tte.tag);
            for e in &tte.quasi.expressions {
                on_expr(e);
            }
        }
        // Private field access: traverse the receiver object
        Expression::PrivateFieldExpression(pfe) => {
            on_expr(&pfe.object);
        }
        // Literals, ThisExpression, etc. — leaf nodes with no children
        _ => {}
    }
}

/// Convenience: extract init expressions from a VariableDeclaration.
/// Calls `on_expr` for each declarator's init expression.
pub fn for_each_var_decl_init(vd: &VariableDeclaration, on_expr: &mut impl FnMut(&Expression)) {
    for decl in &vd.declarations {
        if let Some(init) = &decl.init {
            on_expr(init);
        }
    }
}
