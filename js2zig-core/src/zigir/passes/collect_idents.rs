// zigir/passes/collect_idents.rs
// Shared identifier-collection helpers used by dead_code and validate passes.
//
// Walks the IR tree and collects all referenced identifier names into
// a HashSet<String>. Uses walk.rs for structural traversal.

use std::cell::RefCell;
use std::collections::HashSet;

use super::walk;
use crate::zigir::kinds::NewConstructor;
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
                    names.insert(cap.name.zig_name.clone());
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
    match expr {
        IrExpr::Ident(id) => {
            names.insert(id.zig_name.clone());
        }
        // IrExpr::New stores the class name as a plain String in
        // NewConstructor::Class(name), not as a child IrExpr::Ident.
        // walk.rs only visits .args, so we must extract it here.
        IrExpr::New(n) => {
            if let NewConstructor::Class(name) = &n.constructor {
                names.insert(name.clone());
            }
        }
        _ => {}
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ZigType;
    use crate::zigir::ident::IrIdent;
    use crate::zigir::types::{IrBlock, IrCapture, IrClosure, IrClosureStruct, IrExpr, IrStmt};

    /// Bug #3: `collect_stmt_idents` for `NestedFnDecl` inserted the
    /// **js_name** of each captured variable instead of the **zig_name**.
    /// When a JS identifier collides with a Zig reserved keyword
    /// (e.g. `comptime` → zig_name `_comptime`), the mismatch caused
    /// `dead_code` to check `zig_name` against a set containing `js_name`
    /// and potentially remove a referenced top-level const.
    #[test]
    fn test_nested_fn_decl_capture_uses_zig_name() {
        let cap_ident = IrIdent {
            js_name: "comptime".to_string(),
            zig_name: "_comptime".to_string(),
        };
        let capture = IrCapture {
            name: cap_ident.clone(),
            zig_type: ZigType::I64,
            is_mut: false,
            init_expr: None,
        };

        // Bodies intentionally avoid referencing the captured ident so
        // the ONLY path that inserts the capture name is the captured-vec
        // loop in `collect_stmt_idents`.
        let struct_def = IrClosureStruct {
            name: IrIdent::new("inner"),
            captured: vec![capture.clone()],
            fn_params: vec![],
            return_type: ZigType::I64,
            typeof_return_body: None,
            body: IrBlock::new(vec![IrStmt::Return {
                value: Some(IrExpr::IntLiteral(0)),
            }]),
        };
        let instance = IrClosure {
            struct_name: IrIdent::new("inner"),
            captured: vec![capture],
            fn_params: vec![],
            return_type: ZigType::I64,
            body: IrBlock::new(vec![]),
            instance_name: IrIdent::new("inner_inst"),
        };
        let nested = IrStmt::NestedFnDecl {
            struct_def,
            instance: Some(instance),
        };

        let mut names = HashSet::new();
        collect_stmt_idents(&nested, &mut names);

        assert!(
            names.contains("_comptime"),
            "captured name should use zig_name '_comptime', got: {:?}",
            names
        );
        assert!(
            !names.contains("comptime"),
            "captured name should NOT use js_name 'comptime', got: {:?}",
            names
        );
    }
}
