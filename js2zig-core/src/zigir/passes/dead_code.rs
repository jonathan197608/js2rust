// zigir/passes/dead_code.rs
// DeadCodeElimPass — removes unreachable code and unused top-level declarations.
//
// Three elimination strategies:
//   1. Unreachable code after Return/Break/Continue/Throw in a block
//   2. After @compileError: keep subsequent @compileError statements, remove the rest
//   3. VarDecl with CompileError init → convert to IrStmt::CompileError
//   4. Unused top-level variable declarations (const with no references)

use crate::zigir::passes::{IrPass, PassResult};
use crate::zigir::types::{IrAssignTarget, IrBlock, IrDecl, IrExpr, IrModule, IrStmt};

use std::cell::RefCell;

use super::{collect_idents, walk};

/// Dead code elimination pass.
///
/// Removes:
/// - Statements after Return/Break/Continue/Throw in blocks
/// - After @compileError: keeps subsequent @compileError, removes other statements
/// - VarDecl with CompileError init → IrStmt::CompileError
/// - Unused top-level const variable declarations
pub struct DeadCodeElimPass;

/// Remove statements after the first terminator in a list.
/// Returns true if any statements were removed.
fn truncate_after_terminator(stmts: &mut Vec<IrStmt>) -> bool {
    let terminator_idx = stmts.iter().position(is_terminator);
    if let Some(idx) = terminator_idx {
        let remaining = stmts.len() - idx - 1;
        if remaining > 0 {
            stmts.truncate(idx + 1);
            return true;
        }
    }
    false
}

/// After the first `IrStmt::CompileError`, merge any subsequent `IrStmt::CompileError`
/// messages into the first one and remove all other statements. This eliminates
/// "unreachable code" noise in the generated Zig output — Zig treats ALL code
/// after `@compileError` as unreachable, even another `@compileError`.
///
/// Returns true if any statements were removed or merged.
fn truncate_after_compile_error(stmts: &mut Vec<IrStmt>) -> bool {
    let first_ce_idx = stmts.iter().position(is_compile_error_stmt);
    if let Some(idx) = first_ce_idx {
        let original_len = stmts.len();
        if idx + 1 >= original_len {
            return false; // nothing after the first compile error
        }
        // Collect additional messages from subsequent CompileError statements
        let mut extra_msgs: Vec<String> = Vec::new();
        for stmt in stmts.iter().skip(idx + 1) {
            if let IrStmt::CompileError { msg, .. } = stmt {
                extra_msgs.push(msg.clone());
            }
        }
        // Merge extra messages into the first CompileError
        if !extra_msgs.is_empty()
            && let IrStmt::CompileError { msg, .. } = &mut stmts[idx]
        {
            msg.push_str("\n\nAlso: ");
            msg.push_str(&extra_msgs.join("\nAlso: "));
        }
        // Truncate: keep only up to and including the first CompileError
        stmts.truncate(idx + 1);
        true
    } else {
        false
    }
}

/// Check if a statement is `IrStmt::CompileError`.
fn is_compile_error_stmt(stmt: &IrStmt) -> bool {
    matches!(stmt, IrStmt::CompileError { .. })
}

/// Check if an expression is `IrExpr::CompileError`.
fn is_compile_error_expr(expr: &IrExpr) -> bool {
    matches!(expr, IrExpr::CompileError { .. })
}

/// Convert `IrStmt::VarDecl` with a `CompileError` init into `IrStmt::CompileError`.
/// This handles patterns like `const r = @compileError("Unsupported NewExpression")`
/// which would otherwise emit `const r = @compileError(...)` — valid Zig but produces
/// "unreachable code" noise for subsequent statements referencing the variable.
///
/// Returns true if any conversions were made.
fn convert_vardecl_compile_error(stmts: &mut [IrStmt]) -> bool {
    use crate::zigir::source_span::SourceSpan;

    // Phase 1: identify indices of VarDecls with CompileError init
    let indices: Vec<usize> = stmts
        .iter()
        .enumerate()
        .filter_map(|(i, stmt)| {
            if let IrStmt::VarDecl(vd) = stmt
                && vd.init.as_ref().is_some_and(is_compile_error_expr)
            {
                return Some(i);
            }
            None
        })
        .collect();

    if indices.is_empty() {
        return false;
    }

    // Phase 2: convert those VarDecls to IrStmt::CompileError
    for i in indices {
        let old_stmt = std::mem::replace(
            &mut stmts[i],
            IrStmt::CompileError {
                span: SourceSpan::default(),
                msg: String::new(),
            },
        );
        if let IrStmt::VarDecl(vd) = old_stmt
            && let Some(IrExpr::CompileError { span, msg }) = vd.init
        {
            stmts[i] = IrStmt::CompileError { span, msg };
        }
    }
    true
}

impl DeadCodeElimPass {
    pub fn new() -> Self {
        Self
    }

    /// Remove unreachable statements after a terminator in a block.
    /// Returns true if any statements were removed.
    fn eliminate_unreachable_in_block(block: &mut IrBlock) -> bool {
        let mut changed = false;

        // Phase 1: Convert VarDecl with CompileError init → IrStmt::CompileError
        if convert_vardecl_compile_error(&mut block.stmts) {
            changed = true;
        }

        // Phase 2: After a CompileError, keep subsequent CompileError stmts only
        if truncate_after_compile_error(&mut block.stmts) {
            changed = true;
        }

        // Phase 3: Find and truncate after the first traditional terminator
        if truncate_after_terminator(&mut block.stmts) {
            changed = true;
        }

        // Recurse into sub-blocks
        for stmt in &mut block.stmts {
            if eliminate_unreachable_in_stmt(stmt) {
                changed = true;
            }
        }

        changed
    }

    /// Remove unused top-level const declarations.
    /// A const is "unused" if it has no side effects and is never referenced.
    /// Returns true if any declarations were removed.
    fn eliminate_unused_decls(module: &mut IrModule) -> bool {
        // Collect all identifier references across the entire module
        let mut referenced = std::collections::HashSet::new();

        for decl in &module.declarations {
            collect_idents::collect_decl_idents(decl, &mut referenced);
        }
        for cs in &module.closure_structs {
            collect_idents::collect_block_idents(&cs.body, &mut referenced);
        }

        // Remove unused const declarations
        let before = module.declarations.len();
        module.declarations.retain(|decl| {
            if let IrDecl::Var(v) = decl {
                // Only remove const vars that are never referenced and have
                // no side effects in their initializer
                if v.is_const && !referenced.contains(&v.name.zig_name) && !v.is_json_parse {
                    // Check if the init expression has side effects
                    if let Some(init) = &v.init {
                        if !expr_has_side_effects(init) {
                            return false; // remove it
                        }
                    } else {
                        return false; // no init, no side effects, remove
                    }
                }
            }
            true
        });

        module.declarations.len() != before
    }
}

impl IrPass for DeadCodeElimPass {
    fn name(&self) -> &'static str {
        "dead-code-elim"
    }

    fn description(&self) -> &'static str {
        "Removes unreachable code after terminators and unused top-level const declarations"
    }

    fn run(&mut self, module: &mut IrModule) -> PassResult {
        let mut changed = false;

        // Pass 1: Remove unreachable code in all blocks
        for decl in &mut module.declarations {
            if eliminate_unreachable_in_decl(decl) {
                changed = true;
            }
        }
        for cs in &mut module.closure_structs {
            if Self::eliminate_unreachable_in_block(&mut cs.body) {
                changed = true;
            }
        }

        // Pass 2: Remove unused top-level declarations
        if Self::eliminate_unused_decls(module) {
            changed = true;
        }

        if changed {
            PassResult::changed()
        } else {
            PassResult::unchanged()
        }
    }
}

impl Default for DeadCodeElimPass {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════
//  Helpers
// ═══════════════════════════════════════════════════════

/// Whether a statement is a control flow terminator (after which nothing runs).
fn is_terminator(stmt: &IrStmt) -> bool {
    matches!(
        stmt,
        IrStmt::Return { .. }
            | IrStmt::Break { .. }
            | IrStmt::Continue { .. }
            | IrStmt::Throw { .. }
    )
}

/// Whether an expression may have side effects (function calls, assignments, etc.).
fn expr_has_side_effects(expr: &IrExpr) -> bool {
    if expr.is_leaf() {
        return false;
    }
    match expr {
        IrExpr::Call(_) | IrExpr::BuiltinCall(_) | IrExpr::HostCall(_) => true,
        IrExpr::Assign { .. } | IrExpr::Update { .. } => true,
        IrExpr::New(_) => true,
        IrExpr::Await(_) => true,

        IrExpr::Binary { left, right, .. } => {
            expr_has_side_effects(left) || expr_has_side_effects(right)
        }
        IrExpr::Unary { operand, .. } => expr_has_side_effects(operand),
        IrExpr::Logical { left, right, .. } => {
            expr_has_side_effects(left) || expr_has_side_effects(right)
        }
        IrExpr::Conditional { cond, then, else_ } => {
            expr_has_side_effects(cond)
                || expr_has_side_effects(then)
                || expr_has_side_effects(else_)
        }
        IrExpr::FieldAccess { object, .. } => expr_has_side_effects(object),
        IrExpr::IndexAccess { object, index, .. } => {
            expr_has_side_effects(object) || expr_has_side_effects(index)
        }
        IrExpr::ComputedField { object, key, .. } => {
            expr_has_side_effects(object) || expr_has_side_effects(key)
        }
        IrExpr::ArrayLiteral(arr) => arr.elements.iter().any(expr_has_side_effects),
        IrExpr::ObjectLiteral(obj) => {
            use crate::zigir::types::IrObjectItem;
            obj.items.iter().any(|item| match item {
                IrObjectItem::Field(f) => expr_has_side_effects(&f.value),
                IrObjectItem::Spread(e) => expr_has_side_effects(e),
            })
        }
        IrExpr::TemplateLiteral { exprs, .. } => exprs.iter().any(expr_has_side_effects),
        IrExpr::Closure(_) | IrExpr::ArrowFn(_) | IrExpr::FnExpr(_) => false,
        IrExpr::BlockExpr { body, result, .. } => {
            body.iter().any(stmt_has_side_effects) || expr_has_side_effects(result)
        }
        IrExpr::AllocPrint { args, .. } => args.iter().any(expr_has_side_effects),
        IrExpr::Spread(e) | IrExpr::Typeof(e) | IrExpr::Void(e) | IrExpr::Paren(e) => {
            expr_has_side_effects(e)
        }
        IrExpr::Sequence(exprs) => exprs.iter().any(expr_has_side_effects),
        IrExpr::ArrayCallbackInline(inline_data) => {
            inline_data.body.iter().any(stmt_has_side_effects)
                || inline_data
                    .obj_expr
                    .as_ref()
                    .is_some_and(|e| expr_has_side_effects(e))
        }
        IrExpr::ArrayMethodInline(inline_data) => {
            // All array method inlines have side effects (loops, allocs, mutations)
            let _ = inline_data;
            true
        }
        IrExpr::OptionalChain {
            object,
            body,
            needs_null_check,
            ..
        } => expr_has_side_effects(object) || (*needs_null_check || expr_has_side_effects(body)),
        IrExpr::PowExpr { base, exp, .. } => {
            expr_has_side_effects(base) || expr_has_side_effects(exp)
        }
        IrExpr::RemExpr { left, right, .. } => {
            expr_has_side_effects(left) || expr_has_side_effects(right)
        }
        IrExpr::DivExpr { left, right, .. } => {
            expr_has_side_effects(left) || expr_has_side_effects(right)
        }
        IrExpr::CompileError { .. } => true,
        // Leaf expressions handled by is_leaf() early return above
        _ => false,
    }
}

/// Whether a statement has side effects.
fn stmt_has_side_effects(stmt: &IrStmt) -> bool {
    match stmt {
        IrStmt::VarDecl(v) => v.init.as_ref().is_some_and(expr_has_side_effects),
        IrStmt::Assign { .. } | IrStmt::Throw { .. } | IrStmt::Return { .. } => true,
        IrStmt::Expr(e) => expr_has_side_effects(e),
        IrStmt::If { cond, then, else_ } => {
            expr_has_side_effects(cond)
                || then.stmts.iter().any(stmt_has_side_effects)
                || else_
                    .as_ref()
                    .is_some_and(|e| e.stmts.iter().any(stmt_has_side_effects))
        }
        IrStmt::While { cond, body, .. } => {
            expr_has_side_effects(cond) || body.stmts.iter().any(stmt_has_side_effects)
        }
        IrStmt::DoWhile { body, cond, .. } => {
            body.stmts.iter().any(stmt_has_side_effects) || expr_has_side_effects(cond)
        }
        IrStmt::For {
            init,
            cond,
            update,
            body,
            ..
        } => {
            init.as_ref().is_some_and(|s| stmt_has_side_effects(s))
                || cond.as_ref().is_some_and(expr_has_side_effects)
                || update.as_ref().is_some_and(|s| stmt_has_side_effects(s))
                || body.stmts.iter().any(stmt_has_side_effects)
        }
        IrStmt::Switch { expr, cases } => {
            expr_has_side_effects(expr)
                || cases
                    .iter()
                    .any(|c| c.body.iter().any(stmt_has_side_effects))
        }
        IrStmt::Try {
            try_block,
            catch_block,
            finally,
            ..
        } => {
            try_block.stmts.iter().any(stmt_has_side_effects)
                || catch_block.stmts.iter().any(stmt_has_side_effects)
                || finally
                    .as_ref()
                    .is_some_and(|f| f.stmts.iter().any(stmt_has_side_effects))
        }
        IrStmt::Break { .. } | IrStmt::Continue { .. } => true,
        IrStmt::Block(b) => b.stmts.iter().any(stmt_has_side_effects),
        IrStmt::ForIn { iterable, body, .. } => {
            expr_has_side_effects(iterable) || body.stmts.iter().any(stmt_has_side_effects)
        }
        IrStmt::ForOf { iterable, body, .. } => {
            expr_has_side_effects(iterable) || body.stmts.iter().any(stmt_has_side_effects)
        }
        // CompileError must be preserved: it carries a compile-time
        // diagnostic that needs to surface to the user. Treating it as
        // side-effect-free would let dead-code elimination drop it (and any
        // preceding statements that the error is meant to flag).
        IrStmt::CompileError { .. } => true,
        IrStmt::Comment(_) => false,
        IrStmt::DestructureDecl(data) => expr_has_side_effects(&data.init),
        IrStmt::NestedFnDecl { .. } => true,
    }
}

fn eliminate_unreachable_in_decl(decl: &mut IrDecl) -> bool {
    let changed = RefCell::new(false);
    walk::for_each_decl_child_mut(
        decl,
        &mut |block| {
            *changed.borrow_mut() |= DeadCodeElimPass::eliminate_unreachable_in_block(block);
        },
        &mut |expr| {
            *changed.borrow_mut() |= eliminate_unreachable_in_expr(expr);
        },
    );
    changed.into_inner()
}

fn eliminate_unreachable_in_target(target: &mut IrAssignTarget) -> bool {
    let mut changed = false;
    walk::for_each_target_child_mut(target, &mut |expr| {
        changed |= eliminate_unreachable_in_expr(expr);
    });
    changed
}

fn eliminate_unreachable_in_stmt(stmt: &mut IrStmt) -> bool {
    match stmt {
        IrStmt::VarDecl(v) => {
            if let Some(e) = &mut v.init {
                eliminate_unreachable_in_expr(e)
            } else {
                false
            }
        }
        IrStmt::If { then, else_, .. } => {
            let mut changed = false;
            if DeadCodeElimPass::eliminate_unreachable_in_block(then) {
                changed = true;
            }
            if let Some(e) = else_
                && DeadCodeElimPass::eliminate_unreachable_in_block(e)
            {
                changed = true;
            }
            changed
        }
        IrStmt::While { body, .. } => DeadCodeElimPass::eliminate_unreachable_in_block(body),
        IrStmt::DoWhile { body, .. } => DeadCodeElimPass::eliminate_unreachable_in_block(body),
        IrStmt::For {
            init, update, body, ..
        } => {
            let mut changed = false;
            if let Some(i) = init
                && eliminate_unreachable_in_stmt(i)
            {
                changed = true;
            }
            if let Some(u) = update
                && eliminate_unreachable_in_stmt(u)
            {
                changed = true;
            }
            if DeadCodeElimPass::eliminate_unreachable_in_block(body) {
                changed = true;
            }
            changed
        }
        IrStmt::ForIn { body, .. } => DeadCodeElimPass::eliminate_unreachable_in_block(body),
        IrStmt::ForOf { body, .. } => DeadCodeElimPass::eliminate_unreachable_in_block(body),
        IrStmt::Switch { cases, .. } => {
            let mut changed = false;
            for case in cases {
                // Cases use Vec<IrStmt>, not IrBlock
                if truncate_after_terminator(&mut case.body) {
                    changed = true;
                }
                for s in &mut case.body {
                    if eliminate_unreachable_in_stmt(s) {
                        changed = true;
                    }
                }
            }
            changed
        }
        IrStmt::Try {
            try_block,
            catch_block,
            finally,
            ..
        } => {
            let mut changed = false;
            if DeadCodeElimPass::eliminate_unreachable_in_block(try_block) {
                changed = true;
            }
            if DeadCodeElimPass::eliminate_unreachable_in_block(catch_block) {
                changed = true;
            }
            if let Some(f) = finally
                && DeadCodeElimPass::eliminate_unreachable_in_block(f)
            {
                changed = true;
            }
            changed
        }
        IrStmt::Block(b) => DeadCodeElimPass::eliminate_unreachable_in_block(b),
        IrStmt::Assign { value, .. } => eliminate_unreachable_in_expr(value),
        IrStmt::Throw { value, .. } => eliminate_unreachable_in_expr(value),
        IrStmt::Return { value } => {
            if let Some(v) = value {
                eliminate_unreachable_in_expr(v)
            } else {
                false
            }
        }
        IrStmt::Expr(e) => eliminate_unreachable_in_expr(e),
        IrStmt::Break { .. }
        | IrStmt::Continue { .. }
        | IrStmt::CompileError { .. }
        | IrStmt::Comment(_) => false,
        IrStmt::DestructureDecl(_) => false,
        IrStmt::NestedFnDecl { .. } => false,
    }
}

fn eliminate_unreachable_in_expr(expr: &mut IrExpr) -> bool {
    // Special case: BlockExpr needs truncate_before recursion + result visit
    if let IrExpr::BlockExpr { body, result, .. } = expr {
        let mut changed = false;
        if truncate_after_terminator(body) {
            changed = true;
        }
        for s in &mut *body {
            if eliminate_unreachable_in_stmt(s) {
                changed = true;
            }
        }
        if eliminate_unreachable_in_expr(result) {
            changed = true;
        }
        return changed;
    }

    // Standard walk for all other variants.
    // Note: walk.rs visits BuiltinCall.obj_expr, ArrayCallbackInline.reduce_init,
    // and Assign/Update targets — these were previously missed; visiting them
    // is a correctness improvement (eliminates unreachable code in those sub-trees).
    let changed = RefCell::new(false);
    walk::for_each_expr_child_mut(
        expr,
        &mut |block| {
            *changed.borrow_mut() |= DeadCodeElimPass::eliminate_unreachable_in_block(block);
        },
        &mut |stmt| {
            *changed.borrow_mut() |= eliminate_unreachable_in_stmt(stmt);
        },
        &mut |expr| {
            *changed.borrow_mut() |= eliminate_unreachable_in_expr(expr);
        },
        &mut |target| {
            *changed.borrow_mut() |= eliminate_unreachable_in_target(target);
        },
    );
    changed.into_inner()
}

// ═══════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ZigType;
    use crate::zigir::ident::IrIdent;
    use crate::zigir::types::{IrBlock, IrDecl, IrFnDecl, IrStmt, IrVarDecl};

    #[test]
    fn test_remove_unreachable_after_return() {
        let mut module = IrModule::new("test".to_string());
        module.declarations.push(IrDecl::Fn(IrFnDecl {
            name: IrIdent::new("foo"),
            params: vec![],
            return_type: ZigType::Void,
            body: IrBlock::new(vec![
                IrStmt::Return { value: None },
                IrStmt::Expr(IrExpr::IntLiteral(42)), // unreachable
                IrStmt::Expr(IrExpr::IntLiteral(99)), // unreachable
            ]),
            is_export: false,
            is_async: false,
            can_throw: false,
            is_cabi: false,
            typeof_return_body: None,
        }));

        let mut pass = DeadCodeElimPass::new();
        let result = pass.run(&mut module);
        assert!(result.changed);

        if let IrDecl::Fn(f) = &module.declarations[0] {
            assert_eq!(f.body.stmts.len(), 1); // only the return remains
        } else {
            panic!("expected Fn decl");
        }
    }

    #[test]
    fn test_remove_unused_const() {
        let mut module = IrModule::new("test".to_string());
        module.declarations.push(IrDecl::Var(IrVarDecl::new_const(
            "unused",
            Some(ZigType::I64),
            Some(IrExpr::IntLiteral(42)),
        )));
        module.declarations.push(IrDecl::Fn(IrFnDecl {
            name: IrIdent::new("main"),
            params: vec![],
            return_type: ZigType::Void,
            body: IrBlock::new(vec![IrStmt::Return { value: None }]),
            is_export: true,
            is_async: false,
            can_throw: false,
            is_cabi: false,
            typeof_return_body: None,
        }));

        let mut pass = DeadCodeElimPass::new();
        let result = pass.run(&mut module);
        assert!(result.changed);
        assert_eq!(module.declarations.len(), 1);
        assert!(matches!(&module.declarations[0], IrDecl::Fn(_)));
    }

    #[test]
    fn test_keep_used_const() {
        let mut module = IrModule::new("test".to_string());
        module.declarations.push(IrDecl::Var(IrVarDecl::new_const(
            "x",
            Some(ZigType::I64),
            Some(IrExpr::IntLiteral(42)),
        )));
        module.declarations.push(IrDecl::Fn(IrFnDecl {
            name: IrIdent::new("main"),
            params: vec![],
            return_type: ZigType::I64,
            body: IrBlock::new(vec![IrStmt::Return {
                value: Some(IrExpr::Ident(IrIdent::new("x"))),
            }]),
            is_export: true,
            is_async: false,
            can_throw: false,
            is_cabi: false,
            typeof_return_body: None,
        }));

        let mut pass = DeadCodeElimPass::new();
        let result = pass.run(&mut module);
        assert!(!result.changed);
        assert_eq!(module.declarations.len(), 2);
    }

    #[test]
    fn test_keep_const_with_side_effects() {
        let mut module = IrModule::new("test".to_string());
        module.declarations.push(IrDecl::Var(IrVarDecl::new_const(
            "result",
            Some(ZigType::I64),
            Some(IrExpr::Call(crate::zigir::types::IrCallExpr {
                callee: Box::new(IrExpr::Ident(IrIdent::new("compute"))),
                args: vec![],
                call_kind: crate::zigir::kinds::CallKind::Direct,
            })),
        )));

        let mut pass = DeadCodeElimPass::new();
        let result = pass.run(&mut module);
        assert!(!result.changed); // call has side effects
    }

    #[test]
    fn test_no_change_on_clean_code() {
        let mut module = super::super::make_clean_add_module();

        let mut pass = DeadCodeElimPass::new();
        let result = pass.run(&mut module);
        assert!(!result.changed);
    }
}
