// zigir/passes/collect_idents.rs
// Shared identifier-collection helpers used by dead_code and validate passes.
//
// Walks the IR tree and collects all referenced identifier names into
// a HashSet<String>. This module is the single source of truth for
// collection logic — both DeadCodeElimPass and ValidatePass delegate
// here so that the traversal stays in sync.

use std::collections::HashSet;

use crate::zigir::types::{IrAssignTarget, IrBlock, IrDecl, IrExpr, IrStmt};

/// Collect all identifier names referenced in a declaration.
pub fn collect_decl_idents(decl: &IrDecl, names: &mut HashSet<String>) {
    match decl {
        IrDecl::Fn(f) => collect_block_idents(&f.body, names),
        IrDecl::Var(v) => {
            if let Some(e) = &v.init {
                collect_expr_idents(e, names);
            }
        }
        IrDecl::Class(c) => {
            if let Some(ctor) = &c.constructor {
                collect_block_idents(&ctor.body, names);
            }
            for m in &c.methods {
                collect_block_idents(&m.body, names);
            }
            for (_name, init, _ty) in &c.static_inits {
                collect_expr_idents(init, names);
            }
            for block in &c.static_blocks {
                collect_block_idents(block, names);
            }
        }
        IrDecl::CompileError { .. } => {}
    }
}

/// Collect all identifier names referenced in a block.
pub fn collect_block_idents(block: &IrBlock, names: &mut HashSet<String>) {
    collect_stmts_idents(&block.stmts, names);
}

/// Collect all identifier names referenced in a slice of statements.
pub fn collect_stmts_idents(stmts: &[IrStmt], names: &mut HashSet<String>) {
    for stmt in stmts {
        collect_stmt_idents(stmt, names);
    }
}

/// Collect all identifier names referenced in a statement.
pub fn collect_stmt_idents(stmt: &IrStmt, names: &mut HashSet<String>) {
    match stmt {
        IrStmt::VarDecl(v) => {
            if let Some(e) = &v.init {
                collect_expr_idents(e, names);
            }
        }
        IrStmt::Assign { target, value, .. } => {
            collect_target_idents(target, names);
            collect_expr_idents(value, names);
        }
        IrStmt::If { cond, then, else_ } => {
            collect_expr_idents(cond, names);
            collect_block_idents(then, names);
            if let Some(e) = else_ {
                collect_block_idents(e, names);
            }
        }
        IrStmt::While { cond, body, .. } => {
            collect_expr_idents(cond, names);
            collect_block_idents(body, names);
        }
        IrStmt::DoWhile { body, cond, .. } => {
            collect_block_idents(body, names);
            collect_expr_idents(cond, names);
        }
        IrStmt::For {
            init,
            cond,
            update,
            body,
            ..
        } => {
            if let Some(i) = init {
                collect_stmt_idents(i, names);
            }
            if let Some(c) = cond {
                collect_expr_idents(c, names);
            }
            if let Some(u) = update {
                collect_stmt_idents(u, names);
            }
            collect_block_idents(body, names);
        }
        IrStmt::ForIn { iterable, body, .. } => {
            collect_expr_idents(iterable, names);
            collect_block_idents(body, names);
        }
        IrStmt::ForOf { iterable, body, .. } => {
            collect_expr_idents(iterable, names);
            collect_block_idents(body, names);
        }
        IrStmt::Switch { expr, cases } => {
            collect_expr_idents(expr, names);
            for case in cases {
                collect_stmts_idents(&case.body, names);
            }
        }
        IrStmt::Try {
            try_block,
            catch_block,
            finally,
            ..
        } => {
            collect_block_idents(try_block, names);
            collect_block_idents(catch_block, names);
            if let Some(f) = finally {
                collect_block_idents(f, names);
            }
        }
        IrStmt::Throw { value, .. } => collect_expr_idents(value, names),
        IrStmt::Return { value } => {
            if let Some(v) = value {
                collect_expr_idents(v, names);
            }
        }
        IrStmt::Expr(e) => collect_expr_idents(e, names),
        IrStmt::Block(b) => collect_block_idents(b, names),
        IrStmt::Break { .. }
        | IrStmt::Continue { .. }
        | IrStmt::CompileError { .. }
        | IrStmt::Comment(_) => {}
        IrStmt::DestructureDecl(data) => {
            collect_expr_idents(&data.init, names);
            for binding in &data.bindings {
                if let Some(d) = &binding.default {
                    collect_expr_idents(d, names);
                }
            }
        }
        IrStmt::NestedFnDecl {
            struct_def,
            instance,
        } => {
            collect_block_idents(&struct_def.body, names);
            if let Some(closure) = instance {
                for cap in &closure.captured {
                    names.insert(cap.name.js_name.clone());
                }
            }
        }
    }
}

/// Collect all identifier names referenced in an expression.
pub fn collect_expr_idents(expr: &IrExpr, names: &mut HashSet<String>) {
    match expr {
        IrExpr::Ident(id) => {
            names.insert(id.zig_name.clone());
        }
        IrExpr::Binary { left, right, .. } => {
            collect_expr_idents(left, names);
            collect_expr_idents(right, names);
        }
        IrExpr::Unary { operand, .. } => collect_expr_idents(operand, names),
        IrExpr::Logical { left, right, .. } => {
            collect_expr_idents(left, names);
            collect_expr_idents(right, names);
        }
        IrExpr::Call(call) => {
            collect_expr_idents(&call.callee, names);
            for arg in &call.args {
                collect_expr_idents(arg, names);
            }
        }
        IrExpr::BuiltinCall(bc) => {
            for arg in &bc.args {
                collect_expr_idents(arg, names);
            }
        }
        IrExpr::HostCall(hc) => {
            for arg in &hc.args {
                collect_expr_idents(arg, names);
            }
        }
        IrExpr::FieldAccess { object, .. } => collect_expr_idents(object, names),
        IrExpr::IndexAccess { object, index, .. } => {
            collect_expr_idents(object, names);
            collect_expr_idents(index, names);
        }
        IrExpr::ComputedField { object, key, .. } => {
            collect_expr_idents(object, names);
            collect_expr_idents(key, names);
        }
        IrExpr::Conditional { cond, then, else_ } => {
            collect_expr_idents(cond, names);
            collect_expr_idents(then, names);
            collect_expr_idents(else_, names);
        }
        IrExpr::TemplateLiteral { exprs, .. } => {
            for e in exprs {
                collect_expr_idents(e, names);
            }
        }
        IrExpr::ArrayLiteral(arr) => {
            for e in &arr.elements {
                collect_expr_idents(e, names);
            }
        }
        IrExpr::ObjectLiteral(obj) => {
            use crate::zigir::types::IrObjectItem;
            for item in &obj.items {
                match item {
                    IrObjectItem::Field(f) => {
                        collect_expr_idents(&f.value, names);
                    }
                    IrObjectItem::Spread(e) => {
                        collect_expr_idents(e, names);
                    }
                }
            }
        }
        IrExpr::Assign { target, value, .. } => {
            collect_target_idents(target, names);
            collect_expr_idents(value, names);
        }
        IrExpr::Update { target, .. } => collect_target_idents(target, names),
        IrExpr::Closure(c) => collect_block_idents(&c.body, names),
        IrExpr::ArrowFn(af) => collect_block_idents(&af.body, names),
        IrExpr::FnExpr(fe) => collect_block_idents(&fe.body, names),
        IrExpr::Await(a) => {
            collect_expr_idents(&a.callee, names);
            for arg in &a.args {
                collect_expr_idents(arg, names);
            }
        }
        IrExpr::New(n) => {
            for arg in &n.args {
                collect_expr_idents(arg, names);
            }
        }
        IrExpr::BlockExpr { body, result, .. } => {
            collect_stmts_idents(body, names);
            collect_expr_idents(result, names);
        }
        IrExpr::AllocPrint { args, .. } => {
            for a in args {
                collect_expr_idents(a, names);
            }
        }
        IrExpr::Spread(e) | IrExpr::Typeof(e) | IrExpr::Void(e) | IrExpr::Paren(e) => {
            collect_expr_idents(e, names);
        }
        IrExpr::Sequence(exprs) => {
            for e in exprs {
                collect_expr_idents(e, names);
            }
        }
        IrExpr::ArrayCallbackInline(inline_data) => {
            if let Some(obj_expr) = &inline_data.obj_expr {
                collect_expr_idents(obj_expr, names);
            }
            for stmt in &inline_data.body {
                collect_stmt_idents(stmt, names);
            }
        }
        IrExpr::ArrayMethodInline(inline_data) => {
            if let Some(obj_expr) = &inline_data.obj_expr {
                collect_expr_idents(obj_expr, names);
            }
            for arg in &inline_data.args {
                collect_expr_idents(arg, names);
            }
        }
        IrExpr::OptionalChain { object, body, .. } => {
            collect_expr_idents(object, names);
            collect_expr_idents(body, names);
        }
        IrExpr::PowExpr { base, exp, .. } => {
            collect_expr_idents(base, names);
            collect_expr_idents(exp, names);
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

/// Collect all identifier names referenced in an assignment target.
pub fn collect_target_idents(target: &IrAssignTarget, names: &mut HashSet<String>) {
    match target {
        IrAssignTarget::Ident(id) => {
            names.insert(id.zig_name.clone());
        }
        IrAssignTarget::Member { object, .. } => {
            collect_expr_idents(object, names);
        }
        IrAssignTarget::Index { object, index, .. } => {
            collect_expr_idents(object, names);
            collect_expr_idents(index, names);
        }
        IrAssignTarget::Destructure(bindings) => {
            for b in bindings {
                if let Some(d) = &b.default {
                    collect_expr_idents(d, names);
                }
            }
        }
        IrAssignTarget::CompileError { .. } => {}
    }
}
