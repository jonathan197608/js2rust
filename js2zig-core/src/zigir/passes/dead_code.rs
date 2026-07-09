// zigir/passes/dead_code.rs
// DeadCodeElimPass — removes unreachable code and unused top-level declarations.
//
// Two elimination strategies:
//   1. Unreachable code after Return/Break/Continue/Throw in a block
//   2. Unused top-level variable declarations (const with no references)

use crate::zigir::passes::{IrPass, PassResult};
use crate::zigir::types::{IrBlock, IrDecl, IrExpr, IrModule, IrStmt};

/// Dead code elimination pass.
///
/// Removes:
/// - Statements after Return/Break/Continue/Throw in blocks
/// - Unused top-level const variable declarations
pub struct DeadCodeElimPass;

impl DeadCodeElimPass {
    pub fn new() -> Self {
        Self
    }

    /// Remove unreachable statements after a terminator in a block.
    /// Returns true if any statements were removed.
    fn eliminate_unreachable_in_block(block: &mut IrBlock) -> bool {
        let mut changed = false;

        // Find the first terminator in the block
        let terminator_idx = block.stmts.iter().position(is_terminator);

        if let Some(idx) = terminator_idx {
            // Remove everything after the terminator
            let remaining = block.stmts.len() - idx - 1;
            if remaining > 0 {
                block.stmts.truncate(idx + 1);
                changed = true;
            }
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
            collect_decl_refs(decl, &mut referenced);
        }
        for cs in &module.closure_structs {
            collect_block_refs(&cs.body, &mut referenced);
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
    match expr {
        IrExpr::IntLiteral(_)
        | IrExpr::FloatLiteral(_)
        | IrExpr::StringLiteral(_)
        | IrExpr::BoolLiteral(_)
        | IrExpr::BigIntLiteral(_)
        | IrExpr::Null
        | IrExpr::Undefined
        | IrExpr::Ident(_)
        | IrExpr::This => false,

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
        IrExpr::CompileError { .. } => true,
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
        IrStmt::CompileError { .. } | IrStmt::Comment(_) => false,
        IrStmt::DestructureDecl(data) => expr_has_side_effects(&data.init),
        IrStmt::NestedFnDecl { .. } => true,
    }
}

fn eliminate_unreachable_in_decl(decl: &mut IrDecl) -> bool {
    match decl {
        IrDecl::Fn(f) => DeadCodeElimPass::eliminate_unreachable_in_block(&mut f.body),
        IrDecl::Var(v) => {
            if let Some(e) = &mut v.init {
                eliminate_unreachable_in_expr(e)
            } else {
                false
            }
        }
        IrDecl::Class(c) => {
            let mut changed = false;
            if let Some(ctor) = &mut c.constructor
                && DeadCodeElimPass::eliminate_unreachable_in_block(&mut ctor.body)
            {
                changed = true;
            }
            for m in &mut c.methods {
                if DeadCodeElimPass::eliminate_unreachable_in_block(&mut m.body) {
                    changed = true;
                }
            }
            for (_name, init, _ty) in &mut c.static_inits {
                if eliminate_unreachable_in_expr(init) {
                    changed = true;
                }
            }
            for block in &mut c.static_blocks {
                if DeadCodeElimPass::eliminate_unreachable_in_block(block) {
                    changed = true;
                }
            }
            changed
        }
        IrDecl::CompileError { .. } => false,
    }
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
                let terminator_idx = case.body.iter().position(is_terminator);
                if let Some(idx) = terminator_idx {
                    let remaining = case.body.len() - idx - 1;
                    if remaining > 0 {
                        case.body.truncate(idx + 1);
                        changed = true;
                    }
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
    match expr {
        IrExpr::Closure(c) => DeadCodeElimPass::eliminate_unreachable_in_block(&mut c.body),
        IrExpr::ArrowFn(af) => DeadCodeElimPass::eliminate_unreachable_in_block(&mut af.body),
        IrExpr::FnExpr(fe) => DeadCodeElimPass::eliminate_unreachable_in_block(&mut fe.body),
        IrExpr::BlockExpr { body, .. } => {
            let mut changed = false;
            // Find terminator in block body
            let terminator_idx = body.iter().position(is_terminator);
            if let Some(idx) = terminator_idx {
                let remaining = body.len() - idx - 1;
                if remaining > 0 {
                    body.truncate(idx + 1);
                    changed = true;
                }
            }
            for s in body {
                if eliminate_unreachable_in_stmt(s) {
                    changed = true;
                }
            }
            changed
        }
        // For other expression types, recurse into sub-expressions
        IrExpr::Binary { left, right, .. } => {
            eliminate_unreachable_in_expr(left) || eliminate_unreachable_in_expr(right)
        }
        IrExpr::Unary { operand, .. } => eliminate_unreachable_in_expr(operand),
        IrExpr::Logical { left, right, .. } => {
            eliminate_unreachable_in_expr(left) || eliminate_unreachable_in_expr(right)
        }
        IrExpr::Conditional { cond, then, else_ } => {
            eliminate_unreachable_in_expr(cond)
                || eliminate_unreachable_in_expr(then)
                || eliminate_unreachable_in_expr(else_)
        }
        IrExpr::Call(call) => {
            let mut changed = eliminate_unreachable_in_expr(&mut call.callee);
            for arg in &mut call.args {
                if eliminate_unreachable_in_expr(arg) {
                    changed = true;
                }
            }
            changed
        }
        IrExpr::BuiltinCall(bc) => {
            let mut changed = false;
            for arg in &mut bc.args {
                if eliminate_unreachable_in_expr(arg) {
                    changed = true;
                }
            }
            changed
        }
        IrExpr::HostCall(hc) => {
            let mut changed = false;
            for arg in &mut hc.args {
                if eliminate_unreachable_in_expr(arg) {
                    changed = true;
                }
            }
            changed
        }
        IrExpr::ArrayLiteral(arr) => {
            let mut changed = false;
            for e in &mut arr.elements {
                if eliminate_unreachable_in_expr(e) {
                    changed = true;
                }
            }
            changed
        }
        IrExpr::ObjectLiteral(obj) => {
            use crate::zigir::types::IrObjectItem;
            let mut changed = false;
            for item in &mut obj.items {
                match item {
                    IrObjectItem::Field(f) => {
                        if eliminate_unreachable_in_expr(&mut f.value) {
                            changed = true;
                        }
                    }
                    IrObjectItem::Spread(e) => {
                        if eliminate_unreachable_in_expr(e) {
                            changed = true;
                        }
                    }
                }
            }
            changed
        }
        IrExpr::TemplateLiteral { exprs, .. } => {
            let mut changed = false;
            for e in exprs {
                if eliminate_unreachable_in_expr(e) {
                    changed = true;
                }
            }
            changed
        }
        IrExpr::AllocPrint { args, .. } => {
            let mut changed = false;
            for a in args {
                if eliminate_unreachable_in_expr(a) {
                    changed = true;
                }
            }
            changed
        }
        IrExpr::FieldAccess { object, .. } => eliminate_unreachable_in_expr(object),
        IrExpr::IndexAccess { object, index, .. } => {
            eliminate_unreachable_in_expr(object) || eliminate_unreachable_in_expr(index)
        }
        IrExpr::ComputedField { object, key, .. } => {
            eliminate_unreachable_in_expr(object) || eliminate_unreachable_in_expr(key)
        }
        IrExpr::Assign { value, .. } => eliminate_unreachable_in_expr(value),
        IrExpr::Spread(e) | IrExpr::Typeof(e) | IrExpr::Void(e) | IrExpr::Paren(e) => {
            eliminate_unreachable_in_expr(e)
        }
        IrExpr::Sequence(exprs) => {
            let mut changed = false;
            for e in exprs {
                if eliminate_unreachable_in_expr(e) {
                    changed = true;
                }
            }
            changed
        }
        IrExpr::Await(a) => {
            let mut changed = eliminate_unreachable_in_expr(&mut a.callee);
            for arg in &mut a.args {
                if eliminate_unreachable_in_expr(arg) {
                    changed = true;
                }
            }
            changed
        }
        IrExpr::New(n) => {
            let mut changed = false;
            for arg in &mut n.args {
                if eliminate_unreachable_in_expr(arg) {
                    changed = true;
                }
            }
            changed
        }
        IrExpr::Update { .. } => false,
        IrExpr::ArrayCallbackInline(inline_data) => {
            let mut changed = false;
            for stmt in &mut inline_data.body {
                if eliminate_unreachable_in_stmt(stmt) {
                    changed = true;
                }
            }
            if let Some(obj_expr) = &mut inline_data.obj_expr
                && eliminate_unreachable_in_expr(obj_expr)
            {
                changed = true;
            }
            changed
        }
        IrExpr::ArrayMethodInline(inline_data) => {
            let mut changed = false;
            if let Some(obj_expr) = &mut inline_data.obj_expr
                && eliminate_unreachable_in_expr(obj_expr)
            {
                changed = true;
            }
            for arg in &mut inline_data.args {
                if eliminate_unreachable_in_expr(arg) {
                    changed = true;
                }
            }
            changed
        }
        IrExpr::OptionalChain { object, body, .. } => {
            eliminate_unreachable_in_expr(object) | eliminate_unreachable_in_expr(body)
        }
        IrExpr::PowExpr { base, exp, .. } => {
            eliminate_unreachable_in_expr(base) | eliminate_unreachable_in_expr(exp)
        }
        // Leaf expressions
        IrExpr::IntLiteral(_)
        | IrExpr::FloatLiteral(_)
        | IrExpr::StringLiteral(_)
        | IrExpr::BoolLiteral(_)
        | IrExpr::BigIntLiteral(_)
        | IrExpr::Null
        | IrExpr::Undefined
        | IrExpr::Ident(_)
        | IrExpr::This
        | IrExpr::CompileError { .. } => false,
    }
}

/// Collect all identifier references from a declaration.
fn collect_decl_refs(decl: &IrDecl, refs: &mut std::collections::HashSet<String>) {
    match decl {
        IrDecl::Fn(f) => collect_block_refs(&f.body, refs),
        IrDecl::Var(v) => {
            if let Some(e) = &v.init {
                collect_expr_refs(e, refs);
            }
        }
        IrDecl::Class(c) => {
            if let Some(ctor) = &c.constructor {
                collect_block_refs(&ctor.body, refs);
            }
            for m in &c.methods {
                collect_block_refs(&m.body, refs);
            }
            for (_name, init, _ty) in &c.static_inits {
                collect_expr_refs(init, refs);
            }
            for block in &c.static_blocks {
                collect_block_refs(block, refs);
            }
        }
        IrDecl::CompileError { .. } => {}
    }
}

fn collect_block_refs(block: &IrBlock, refs: &mut std::collections::HashSet<String>) {
    for stmt in &block.stmts {
        collect_stmt_refs(stmt, refs);
    }
}

fn collect_stmt_refs(stmt: &IrStmt, refs: &mut std::collections::HashSet<String>) {
    match stmt {
        IrStmt::VarDecl(v) => {
            if let Some(e) = &v.init {
                collect_expr_refs(e, refs);
            }
        }
        IrStmt::Assign { target, value, .. } => {
            collect_target_refs(target, refs);
            collect_expr_refs(value, refs);
        }
        IrStmt::If { cond, then, else_ } => {
            collect_expr_refs(cond, refs);
            collect_block_refs(then, refs);
            if let Some(e) = else_ {
                collect_block_refs(e, refs);
            }
        }
        IrStmt::While { cond, body, .. } => {
            collect_expr_refs(cond, refs);
            collect_block_refs(body, refs);
        }
        IrStmt::DoWhile { body, cond, .. } => {
            collect_block_refs(body, refs);
            collect_expr_refs(cond, refs);
        }
        IrStmt::For {
            init,
            cond,
            update,
            body,
            ..
        } => {
            if let Some(i) = init {
                collect_stmt_refs(i, refs);
            }
            if let Some(c) = cond {
                collect_expr_refs(c, refs);
            }
            if let Some(u) = update {
                collect_stmt_refs(u, refs);
            }
            collect_block_refs(body, refs);
        }
        IrStmt::ForIn { iterable, body, .. } => {
            collect_expr_refs(iterable, refs);
            collect_block_refs(body, refs);
        }
        IrStmt::ForOf { iterable, body, .. } => {
            collect_expr_refs(iterable, refs);
            collect_block_refs(body, refs);
        }
        IrStmt::Switch { expr, cases } => {
            collect_expr_refs(expr, refs);
            for case in cases {
                for s in &case.body {
                    collect_stmt_refs(s, refs);
                }
            }
        }
        IrStmt::Try {
            try_block,
            catch_block,
            finally,
            ..
        } => {
            collect_block_refs(try_block, refs);
            collect_block_refs(catch_block, refs);
            if let Some(f) = finally {
                collect_block_refs(f, refs);
            }
        }
        IrStmt::Throw { value, .. } => collect_expr_refs(value, refs),
        IrStmt::Return { value } => {
            if let Some(v) = value {
                collect_expr_refs(v, refs);
            }
        }
        IrStmt::Expr(e) => collect_expr_refs(e, refs),
        IrStmt::Block(b) => collect_block_refs(b, refs),
        IrStmt::Break { .. }
        | IrStmt::Continue { .. }
        | IrStmt::CompileError { .. }
        | IrStmt::Comment(_) => {}
        IrStmt::DestructureDecl(data) => {
            collect_expr_refs(&data.init, refs);
            for binding in &data.bindings {
                if let Some(d) = &binding.default {
                    collect_expr_refs(d, refs);
                }
            }
        }
        IrStmt::NestedFnDecl {
            struct_def,
            instance,
        } => {
            collect_block_refs(&struct_def.body, refs);
            if let Some(closure) = instance {
                for cap in &closure.captured {
                    refs.insert(cap.name.js_name.clone());
                }
            }
        }
    }
}

fn collect_expr_refs(expr: &IrExpr, refs: &mut std::collections::HashSet<String>) {
    match expr {
        IrExpr::Ident(id) => {
            refs.insert(id.zig_name.clone());
        }
        IrExpr::Binary { left, right, .. } => {
            collect_expr_refs(left, refs);
            collect_expr_refs(right, refs);
        }
        IrExpr::Unary { operand, .. } => collect_expr_refs(operand, refs),
        IrExpr::Logical { left, right, .. } => {
            collect_expr_refs(left, refs);
            collect_expr_refs(right, refs);
        }
        IrExpr::Call(call) => {
            collect_expr_refs(&call.callee, refs);
            for arg in &call.args {
                collect_expr_refs(arg, refs);
            }
        }
        IrExpr::BuiltinCall(bc) => {
            for arg in &bc.args {
                collect_expr_refs(arg, refs);
            }
        }
        IrExpr::HostCall(hc) => {
            for arg in &hc.args {
                collect_expr_refs(arg, refs);
            }
        }
        IrExpr::FieldAccess { object, .. } => collect_expr_refs(object, refs),
        IrExpr::IndexAccess { object, index, .. } => {
            collect_expr_refs(object, refs);
            collect_expr_refs(index, refs);
        }
        IrExpr::ComputedField { object, key, .. } => {
            collect_expr_refs(object, refs);
            collect_expr_refs(key, refs);
        }
        IrExpr::Conditional { cond, then, else_ } => {
            collect_expr_refs(cond, refs);
            collect_expr_refs(then, refs);
            collect_expr_refs(else_, refs);
        }
        IrExpr::TemplateLiteral { exprs, .. } => {
            for e in exprs {
                collect_expr_refs(e, refs);
            }
        }
        IrExpr::ArrayLiteral(arr) => {
            for e in &arr.elements {
                collect_expr_refs(e, refs);
            }
        }
        IrExpr::ObjectLiteral(obj) => {
            use crate::zigir::types::IrObjectItem;
            for item in &obj.items {
                match item {
                    IrObjectItem::Field(f) => {
                        collect_expr_refs(&f.value, refs);
                    }
                    IrObjectItem::Spread(e) => {
                        collect_expr_refs(e, refs);
                    }
                }
            }
        }
        IrExpr::Assign { target, value, .. } => {
            collect_target_refs(target, refs);
            collect_expr_refs(value, refs);
        }
        IrExpr::Update { target, .. } => collect_target_refs(target, refs),
        IrExpr::Closure(c) => collect_block_refs(&c.body, refs),
        IrExpr::ArrowFn(af) => collect_block_refs(&af.body, refs),
        IrExpr::FnExpr(fe) => collect_block_refs(&fe.body, refs),
        IrExpr::Await(a) => {
            collect_expr_refs(&a.callee, refs);
            for arg in &a.args {
                collect_expr_refs(arg, refs);
            }
        }
        IrExpr::New(n) => {
            for arg in &n.args {
                collect_expr_refs(arg, refs);
            }
        }
        IrExpr::BlockExpr { body, result, .. } => {
            for s in body {
                collect_stmt_refs(s, refs);
            }
            collect_expr_refs(result, refs);
        }
        IrExpr::AllocPrint { args, .. } => {
            for a in args {
                collect_expr_refs(a, refs);
            }
        }
        IrExpr::Spread(e) | IrExpr::Typeof(e) | IrExpr::Void(e) | IrExpr::Paren(e) => {
            collect_expr_refs(e, refs);
        }
        IrExpr::Sequence(exprs) => {
            for e in exprs {
                collect_expr_refs(e, refs);
            }
        }
        IrExpr::ArrayCallbackInline(inline_data) => {
            for stmt in &inline_data.body {
                collect_stmt_refs(stmt, refs);
            }
            if let Some(obj_expr) = &inline_data.obj_expr {
                collect_expr_refs(obj_expr, refs);
            }
        }
        IrExpr::ArrayMethodInline(inline_data) => {
            for arg in &inline_data.args {
                collect_expr_refs(arg, refs);
            }
        }
        IrExpr::OptionalChain { object, body, .. } => {
            collect_expr_refs(object, refs);
            collect_expr_refs(body, refs);
        }
        IrExpr::PowExpr { base, exp, .. } => {
            collect_expr_refs(base, refs);
            collect_expr_refs(exp, refs);
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

fn collect_target_refs(
    target: &crate::zigir::types::IrAssignTarget,
    refs: &mut std::collections::HashSet<String>,
) {
    match target {
        crate::zigir::types::IrAssignTarget::Ident(id) => {
            refs.insert(id.zig_name.clone());
        }
        crate::zigir::types::IrAssignTarget::Member { object, .. } => {
            collect_expr_refs(object, refs);
        }
        crate::zigir::types::IrAssignTarget::Index { object, index, .. } => {
            collect_expr_refs(object, refs);
            collect_expr_refs(index, refs);
        }
        crate::zigir::types::IrAssignTarget::Destructure(bindings) => {
            for b in bindings {
                if let Some(d) = &b.default {
                    collect_expr_refs(d, refs);
                }
            }
        }
        crate::zigir::types::IrAssignTarget::CompileError { .. } => {}
    }
}

// ═══════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ZigType;
    use crate::zigir::ident::IrIdent;
    use crate::zigir::ops::BinOp;
    use crate::zigir::types::{IrBlock, IrDecl, IrFnDecl, IrParam, IrStmt, IrVarDecl};

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
        module.declarations.push(IrDecl::Var(IrVarDecl {
            name: IrIdent::new("unused"),
            is_const: true,
            zig_type: Some(ZigType::I64),
            init: Some(IrExpr::IntLiteral(42)),
            is_json_parse: false,
            needs_var_suppression: false,
            needs_const_suppression: false,
            needs_deinit: false,
        }));
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
        module.declarations.push(IrDecl::Var(IrVarDecl {
            name: IrIdent::new("x"),
            is_const: true,
            zig_type: Some(ZigType::I64),
            init: Some(IrExpr::IntLiteral(42)),
            is_json_parse: false,
            needs_var_suppression: false,
            needs_const_suppression: false,
            needs_deinit: false,
        }));
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
        module.declarations.push(IrDecl::Var(IrVarDecl {
            name: IrIdent::new("result"),
            is_const: true,
            zig_type: Some(ZigType::I64),
            init: Some(IrExpr::Call(crate::zigir::types::IrCallExpr {
                callee: Box::new(IrExpr::Ident(IrIdent::new("compute"))),
                args: vec![],
                call_kind: crate::zigir::kinds::CallKind::Direct,
            })),
            is_json_parse: false,
            needs_var_suppression: false,
            needs_const_suppression: false,
            needs_deinit: false,
        }));

        let mut pass = DeadCodeElimPass::new();
        let result = pass.run(&mut module);
        assert!(!result.changed); // call has side effects
    }

    #[test]
    fn test_no_change_on_clean_code() {
        let mut module = IrModule::new("test".to_string());
        module.declarations.push(IrDecl::Fn(IrFnDecl {
            name: IrIdent::new("add"),
            params: vec![
                IrParam {
                    name: IrIdent::new("a"),
                    zig_type: ZigType::I64,
                    is_unused: false,
                    is_rest: false,
                },
                IrParam {
                    name: IrIdent::new("b"),
                    zig_type: ZigType::I64,
                    is_unused: false,
                    is_rest: false,
                },
            ],
            return_type: ZigType::I64,
            body: IrBlock::new(vec![IrStmt::Return {
                value: Some(IrExpr::Binary {
                    op: BinOp::Add,
                    left: Box::new(IrExpr::Ident(IrIdent::new("a"))),
                    right: Box::new(IrExpr::Ident(IrIdent::new("b"))),
                    left_type: None,
                    right_type: None,
                }),
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
    }
}
