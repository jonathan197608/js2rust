// zigir/lower/expr/idents.rs
// IR identifier collection utilities (used for unused parameter detection).

use std::collections::HashSet;

use super::Lowerer;

impl Lowerer {
    /// Collect all identifier names (js_name) referenced in an IR block.
    /// Used to determine which function parameters are unused.
    pub(crate) fn collect_ir_idents_in_block(
        block: &crate::zigir::types::IrBlock,
    ) -> HashSet<String> {
        let mut idents = HashSet::new();
        for stmt in &block.stmts {
            Self::collect_ir_idents_in_stmt(stmt, &mut idents);
        }
        idents
    }

    pub(super) fn collect_ir_idents_in_stmt(
        stmt: &crate::zigir::types::IrStmt,
        idents: &mut HashSet<String>,
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
        idents: &mut HashSet<String>,
    ) {
        use crate::zigir::types::IrAssignTarget;
        match target {
            IrAssignTarget::Ident(name) => {
                idents.insert(name.js_name.clone());
            }
            IrAssignTarget::Member { object, .. } => {
                Self::collect_ir_idents_in_expr(object, idents);
            }
            IrAssignTarget::Index { object, index, .. } => {
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
    pub(crate) fn collect_ast_expr_idents(
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
        idents: &mut HashSet<String>,
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
}
