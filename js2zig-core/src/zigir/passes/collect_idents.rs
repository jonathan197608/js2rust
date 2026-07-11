// zigir/passes/collect_idents.rs
// Shared identifier-collection helpers used by dead_code and validate passes.
//
// Walks the IR tree and collects all referenced identifier names into
// a HashSet<String>. Uses walk.rs for structural traversal.

use std::cell::RefCell;
use std::collections::HashSet;

use super::walk;
use crate::zigir::types::{IrAssignTarget, IrBlock, IrDecl, IrExpr, IrStmt};

/// Collect all identifier names referenced in a declaration.
pub fn collect_decl_idents(decl: &IrDecl, names: &mut HashSet<String>) {
    let names = RefCell::new(names);
    walk::for_each_decl_child(
        decl,
        &mut |block| collect_block_idents(block, *names.borrow_mut()),
        &mut |expr| collect_expr_idents(expr, *names.borrow_mut()),
    );
}

/// Collect all identifier names referenced in a block.
pub fn collect_block_idents(block: &IrBlock, names: &mut HashSet<String>) {
    for stmt in &block.stmts {
        collect_stmt_idents(stmt, names);
    }
}

/// Collect all identifier names referenced in a slice of statements.
#[allow(dead_code)]
pub fn collect_stmts_idents(stmts: &[IrStmt], names: &mut HashSet<String>) {
    for stmt in stmts {
        collect_stmt_idents(stmt, names);
    }
}

/// Collect all identifier names referenced in a statement.
pub fn collect_stmt_idents(stmt: &IrStmt, names: &mut HashSet<String>) {
    match stmt {
        // NestedFnDecl needs custom handling for captured names
        IrStmt::NestedFnDecl {
            struct_def,
            instance,
        } => {
            collect_block_idents(&struct_def.body, names);
            if let Some(closure) = instance {
                collect_block_idents(&closure.body, names);
                for cap in &closure.captured {
                    names.insert(cap.name.js_name.clone());
                }
            }
        }
        _ => {
            let names = RefCell::new(names);
            walk::for_each_stmt_child(
                stmt,
                &mut |block| collect_block_idents(block, *names.borrow_mut()),
                &mut |s| collect_stmt_idents(s, *names.borrow_mut()),
                &mut |expr| collect_expr_idents(expr, *names.borrow_mut()),
                &mut |target| collect_target_idents(target, *names.borrow_mut()),
            );
        }
    }
}

/// Collect all identifier names referenced in an expression.
pub fn collect_expr_idents(expr: &IrExpr, names: &mut HashSet<String>) {
    if let IrExpr::Ident(id) = expr {
        names.insert(id.zig_name.clone());
    }
    let names = RefCell::new(names);
    walk::for_each_expr_child(
        expr,
        &mut |block| collect_block_idents(block, *names.borrow_mut()),
        &mut |s| collect_stmt_idents(s, *names.borrow_mut()),
        &mut |e| collect_expr_idents(e, *names.borrow_mut()),
        &mut |target| collect_target_idents(target, *names.borrow_mut()),
    );
}

/// Collect all identifier names referenced in an assignment target.
pub fn collect_target_idents(target: &IrAssignTarget, names: &mut HashSet<String>) {
    if let IrAssignTarget::Ident(id) = target {
        names.insert(id.zig_name.clone());
    }
    let names = RefCell::new(names);
    walk::for_each_target_child(target, &mut |expr| {
        collect_expr_idents(expr, *names.borrow_mut())
    });
}
