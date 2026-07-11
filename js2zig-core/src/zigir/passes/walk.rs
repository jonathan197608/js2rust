// zigir/passes/walk.rs
// Shared structural traversal helpers for the IR tree.
//
// Each `for_each_*_child` function visits the *direct* children of a node
// and invokes the appropriate callback. These are single-level traversals;
// the caller decides whether and how to recurse.
//
// Callback categories:
//   on_block  鈥?IrBlock children (If.then/else_, While.body, etc.)
//   on_stmt   鈥?IrStmt children (For.init/update, DestructureDecl body, etc.)
//   on_expr   鈥?IrExpr children (conditions, init values, call args, etc.)
//   on_target 鈥?IrAssignTarget children (Assign.target)

use crate::zigir::types::{
    IrAssignTarget, IrBlock, IrDecl, IrDestructureBindingDecl, IrExpr, IrStmt,
};

// 鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺?//  Read-only traversals
// 鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺?
/// Visit direct children of an `IrDecl`.
pub fn for_each_decl_child(
    decl: &IrDecl,
    on_block: &mut impl FnMut(&IrBlock),
    on_expr: &mut impl FnMut(&IrExpr),
) {
    match decl {
        IrDecl::Fn(f) => on_block(&f.body),
        IrDecl::Var(v) => {
            if let Some(e) = &v.init {
                on_expr(e);
            }
        }
        IrDecl::Class(c) => {
            if let Some(ctor) = &c.constructor {
                on_block(&ctor.body);
            }
            for m in &c.methods {
                on_block(&m.body);
            }
            for (_name, init, _ty) in &c.static_inits {
                on_expr(init);
            }
            for block in &c.static_blocks {
                on_block(block);
            }
        }
        IrDecl::CompileError { .. } => {}
    }
}

/// Visit direct children of an `IrStmt`.
pub fn for_each_stmt_child(
    stmt: &IrStmt,
    on_block: &mut impl FnMut(&IrBlock),
    on_stmt: &mut impl FnMut(&IrStmt),
    on_expr: &mut impl FnMut(&IrExpr),
    on_target: &mut impl FnMut(&IrAssignTarget),
) {
    match stmt {
        IrStmt::VarDecl(v) => {
            if let Some(e) = &v.init {
                on_expr(e);
            }
        }
        IrStmt::Assign { target, value, .. } => {
            on_target(target);
            on_expr(value);
        }
        IrStmt::If { cond, then, else_ } => {
            on_expr(cond);
            on_block(then);
            if let Some(e) = else_ {
                on_block(e);
            }
        }
        IrStmt::While { cond, body, .. } => {
            on_expr(cond);
            on_block(body);
        }
        IrStmt::DoWhile { body, cond, .. } => {
            on_block(body);
            on_expr(cond);
        }
        IrStmt::For {
            init,
            cond,
            update,
            body,
            ..
        } => {
            if let Some(i) = init {
                on_stmt(i);
            }
            if let Some(c) = cond {
                on_expr(c);
            }
            if let Some(u) = update {
                on_stmt(u);
            }
            on_block(body);
        }
        IrStmt::ForIn { iterable, body, .. } => {
            on_expr(iterable);
            on_block(body);
        }
        IrStmt::ForOf { iterable, body, .. } => {
            on_expr(iterable);
            on_block(body);
        }
        IrStmt::Switch { expr, cases } => {
            on_expr(expr);
            for case in cases {
                for s in &case.body {
                    on_stmt(s);
                }
            }
        }
        IrStmt::Try {
            try_block,
            catch_block,
            finally,
            ..
        } => {
            on_block(try_block);
            on_block(catch_block);
            if let Some(f) = finally {
                on_block(f);
            }
        }
        IrStmt::Throw { value, .. } => on_expr(value),
        IrStmt::Return { value } => {
            if let Some(v) = value {
                on_expr(v);
            }
        }
        IrStmt::Expr(e) => on_expr(e),
        IrStmt::Block(b) => on_block(b),
        IrStmt::Break { .. }
        | IrStmt::Continue { .. }
        | IrStmt::CompileError { .. }
        | IrStmt::Comment(_) => {}
        IrStmt::DestructureDecl(data) => {
            on_expr(&data.init);
            for binding in &data.bindings {
                visit_destructure_binding_decl(binding, on_expr);
            }
        }
        IrStmt::NestedFnDecl {
            struct_def,
            instance,
        } => {
            on_block(&struct_def.body);
            if let Some(closure) = instance {
                on_block(&closure.body);
            }
        }
    }
}

/// Leaf `IrExpr` variants that carry no children. Used by both read-only and
/// mutable traversal match arms to avoid repeating the exhaustive list.
macro_rules! leaf_expr_variants {
    () => {
        IrExpr::IntLiteral(_)
            | IrExpr::FloatLiteral(_)
            | IrExpr::StringLiteral(_)
            | IrExpr::BoolLiteral(_)
            | IrExpr::BigIntLiteral(_)
            | IrExpr::Null
            | IrExpr::Undefined
            | IrExpr::Ident(_)
            | IrExpr::This
            | IrExpr::CompileError { .. }
    };
}

/// Visit direct children of an `IrExpr`.
pub fn for_each_expr_child(
    expr: &IrExpr,
    on_block: &mut impl FnMut(&IrBlock),
    on_stmt: &mut impl FnMut(&IrStmt),
    on_expr: &mut impl FnMut(&IrExpr),
    on_target: &mut impl FnMut(&IrAssignTarget),
) {
    match expr {
        IrExpr::Binary { left, right, .. } => {
            on_expr(left);
            on_expr(right);
        }
        IrExpr::Unary { operand, .. } => on_expr(operand),
        IrExpr::Logical { left, right, .. } => {
            on_expr(left);
            on_expr(right);
        }
        IrExpr::Call(call) => {
            on_expr(&call.callee);
            for arg in &call.args {
                on_expr(arg);
            }
        }
        IrExpr::BuiltinCall(bc) => {
            if let Some(obj) = &bc.obj_expr {
                on_expr(obj);
            }
            for arg in &bc.args {
                on_expr(arg);
            }
        }
        IrExpr::HostCall(hc) => {
            for arg in &hc.args {
                on_expr(arg);
            }
        }
        IrExpr::FieldAccess { object, .. } => on_expr(object),
        IrExpr::IndexAccess { object, index, .. } => {
            on_expr(object);
            on_expr(index);
        }
        IrExpr::ComputedField { object, key, .. } => {
            on_expr(object);
            on_expr(key);
        }
        IrExpr::Conditional { cond, then, else_ } => {
            on_expr(cond);
            on_expr(then);
            on_expr(else_);
        }
        IrExpr::TemplateLiteral { exprs, .. } => {
            for e in exprs {
                on_expr(e);
            }
        }
        IrExpr::ArrayLiteral(arr) => {
            for e in &arr.elements {
                on_expr(e);
            }
        }
        IrExpr::ObjectLiteral(obj) => {
            use crate::zigir::types::IrObjectItem;
            for item in &obj.items {
                match item {
                    IrObjectItem::Field(f) => on_expr(&f.value),
                    IrObjectItem::Spread(e) => on_expr(e),
                }
            }
        }
        IrExpr::Assign { target, value, .. } => {
            on_target(target);
            on_expr(value);
        }
        IrExpr::Update { target, .. } => on_target(target),
        IrExpr::Closure(c) => on_block(&c.body),
        IrExpr::ArrowFn(af) => on_block(&af.body),
        IrExpr::FnExpr(fe) => on_block(&fe.body),
        IrExpr::Await(a) => {
            on_expr(&a.callee);
            for arg in &a.args {
                on_expr(arg);
            }
        }
        IrExpr::New(n) => {
            for arg in &n.args {
                on_expr(arg);
            }
        }
        IrExpr::BlockExpr { body, result, .. } => {
            for s in body {
                on_stmt(s);
            }
            on_expr(result);
        }
        IrExpr::AllocPrint { args, .. } => {
            for a in args {
                on_expr(a);
            }
        }
        IrExpr::Spread(e) | IrExpr::Typeof(e) | IrExpr::Void(e) | IrExpr::Paren(e) => {
            on_expr(e);
        }
        IrExpr::Sequence(exprs) => {
            for e in exprs {
                on_expr(e);
            }
        }
        IrExpr::ArrayCallbackInline(inline_data) => {
            if let Some(obj) = &inline_data.obj_expr {
                on_expr(obj);
            }
            for s in &inline_data.body {
                on_stmt(s);
            }
            if let Some(init) = &inline_data.reduce_init {
                on_expr(init);
            }
        }
        IrExpr::ArrayMethodInline(inline_data) => {
            if let Some(obj) = &inline_data.obj_expr {
                on_expr(obj);
            }
            for arg in &inline_data.args {
                on_expr(arg);
            }
        }
        IrExpr::OptionalChain { object, body, .. } => {
            on_expr(object);
            on_expr(body);
        }
        IrExpr::PowExpr { base, exp, .. } => {
            on_expr(base);
            on_expr(exp);
        }
        // Leaf nodes and CompileError have no children
        leaf_expr_variants!() => {}
    }
}

/// Visit direct children of an `IrAssignTarget`.
pub fn for_each_target_child(target: &IrAssignTarget, on_expr: &mut impl FnMut(&IrExpr)) {
    match target {
        IrAssignTarget::Member { object, .. } => on_expr(object),
        IrAssignTarget::Index { object, index, .. } => {
            on_expr(object);
            on_expr(index);
        }
        IrAssignTarget::Destructure(bindings) => {
            for b in bindings {
                if let Some(d) = &b.default {
                    on_expr(d);
                }
            }
        }
        IrAssignTarget::Ident(_) | IrAssignTarget::CompileError { .. } => {}
    }
}

// 鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺?//  Mutable traversals
// 鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺?
/// Visit direct children of a mutable `IrDecl`.
pub fn for_each_decl_child_mut(
    decl: &mut IrDecl,
    on_block: &mut impl FnMut(&mut IrBlock),
    on_expr: &mut impl FnMut(&mut IrExpr),
) {
    match decl {
        IrDecl::Fn(f) => on_block(&mut f.body),
        IrDecl::Var(v) => {
            if let Some(e) = &mut v.init {
                on_expr(e);
            }
        }
        IrDecl::Class(c) => {
            if let Some(ctor) = &mut c.constructor {
                on_block(&mut ctor.body);
            }
            for m in &mut c.methods {
                on_block(&mut m.body);
            }
            for (_name, init, _ty) in &mut c.static_inits {
                on_expr(init);
            }
            for block in &mut c.static_blocks {
                on_block(block);
            }
        }
        IrDecl::CompileError { .. } => {}
    }
}

/// Visit direct children of a mutable `IrStmt`.
pub fn for_each_stmt_child_mut(
    stmt: &mut IrStmt,
    on_block: &mut impl FnMut(&mut IrBlock),
    on_stmt: &mut impl FnMut(&mut IrStmt),
    on_expr: &mut impl FnMut(&mut IrExpr),
    on_target: &mut impl FnMut(&mut IrAssignTarget),
) {
    match stmt {
        IrStmt::VarDecl(v) => {
            if let Some(e) = &mut v.init {
                on_expr(e);
            }
        }
        IrStmt::Assign { target, value, .. } => {
            on_target(target);
            on_expr(value);
        }
        IrStmt::If { cond, then, else_ } => {
            on_expr(cond);
            on_block(then);
            if let Some(e) = else_ {
                on_block(e);
            }
        }
        IrStmt::While { cond, body, .. } => {
            on_expr(cond);
            on_block(body);
        }
        IrStmt::DoWhile { body, cond, .. } => {
            on_block(body);
            on_expr(cond);
        }
        IrStmt::For {
            init,
            cond,
            update,
            body,
            ..
        } => {
            if let Some(i) = init {
                on_stmt(i);
            }
            if let Some(c) = cond {
                on_expr(c);
            }
            if let Some(u) = update {
                on_stmt(u);
            }
            on_block(body);
        }
        IrStmt::ForIn { iterable, body, .. } => {
            on_expr(iterable);
            on_block(body);
        }
        IrStmt::ForOf { iterable, body, .. } => {
            on_expr(iterable);
            on_block(body);
        }
        IrStmt::Switch { expr, cases } => {
            on_expr(expr);
            for case in cases {
                for s in &mut case.body {
                    on_stmt(s);
                }
            }
        }
        IrStmt::Try {
            try_block,
            catch_block,
            finally,
            ..
        } => {
            on_block(try_block);
            on_block(catch_block);
            if let Some(f) = finally {
                on_block(f);
            }
        }
        IrStmt::Throw { value, .. } => on_expr(value),
        IrStmt::Return { value } => {
            if let Some(v) = value {
                on_expr(v);
            }
        }
        IrStmt::Expr(e) => on_expr(e),
        IrStmt::Block(b) => on_block(b),
        IrStmt::Break { .. }
        | IrStmt::Continue { .. }
        | IrStmt::CompileError { .. }
        | IrStmt::Comment(_) => {}
        IrStmt::DestructureDecl(data) => {
            on_expr(&mut data.init);
            for binding in &mut data.bindings {
                visit_destructure_binding_decl_mut(binding, on_expr);
            }
        }
        IrStmt::NestedFnDecl {
            struct_def,
            instance,
        } => {
            on_block(&mut struct_def.body);
            if let Some(closure) = instance {
                on_block(&mut closure.body);
            }
        }
    }
}

/// Visit direct children of a mutable `IrExpr`.
pub fn for_each_expr_child_mut(
    expr: &mut IrExpr,
    on_block: &mut impl FnMut(&mut IrBlock),
    on_stmt: &mut impl FnMut(&mut IrStmt),
    on_expr: &mut impl FnMut(&mut IrExpr),
    on_target: &mut impl FnMut(&mut IrAssignTarget),
) {
    match expr {
        IrExpr::Binary { left, right, .. } => {
            on_expr(left);
            on_expr(right);
        }
        IrExpr::Unary { operand, .. } => on_expr(operand),
        IrExpr::Logical { left, right, .. } => {
            on_expr(left);
            on_expr(right);
        }
        IrExpr::Call(call) => {
            on_expr(&mut call.callee);
            for arg in &mut call.args {
                on_expr(arg);
            }
        }
        IrExpr::BuiltinCall(bc) => {
            if let Some(obj) = &mut bc.obj_expr {
                on_expr(obj);
            }
            for arg in &mut bc.args {
                on_expr(arg);
            }
        }
        IrExpr::HostCall(hc) => {
            for arg in &mut hc.args {
                on_expr(arg);
            }
        }
        IrExpr::FieldAccess { object, .. } => on_expr(object),
        IrExpr::IndexAccess { object, index, .. } => {
            on_expr(object);
            on_expr(index);
        }
        IrExpr::ComputedField { object, key, .. } => {
            on_expr(object);
            on_expr(key);
        }
        IrExpr::Conditional { cond, then, else_ } => {
            on_expr(cond);
            on_expr(then);
            on_expr(else_);
        }
        IrExpr::TemplateLiteral { exprs, .. } => {
            for e in exprs {
                on_expr(e);
            }
        }
        IrExpr::ArrayLiteral(arr) => {
            for e in &mut arr.elements {
                on_expr(e);
            }
        }
        IrExpr::ObjectLiteral(obj) => {
            use crate::zigir::types::IrObjectItem;
            for item in &mut obj.items {
                match item {
                    IrObjectItem::Field(f) => on_expr(&mut f.value),
                    IrObjectItem::Spread(e) => on_expr(e),
                }
            }
        }
        IrExpr::Assign { target, value, .. } => {
            on_target(target);
            on_expr(value);
        }
        IrExpr::Update { target, .. } => on_target(target),
        IrExpr::Closure(c) => on_block(&mut c.body),
        IrExpr::ArrowFn(af) => on_block(&mut af.body),
        IrExpr::FnExpr(fe) => on_block(&mut fe.body),
        IrExpr::Await(a) => {
            on_expr(&mut a.callee);
            for arg in &mut a.args {
                on_expr(arg);
            }
        }
        IrExpr::New(n) => {
            for arg in &mut n.args {
                on_expr(arg);
            }
        }
        IrExpr::BlockExpr { body, result, .. } => {
            for s in body {
                on_stmt(s);
            }
            on_expr(result);
        }
        IrExpr::AllocPrint { args, .. } => {
            for a in args {
                on_expr(a);
            }
        }
        IrExpr::Spread(e) | IrExpr::Typeof(e) | IrExpr::Void(e) | IrExpr::Paren(e) => {
            on_expr(e);
        }
        IrExpr::Sequence(exprs) => {
            for e in exprs {
                on_expr(e);
            }
        }
        IrExpr::ArrayCallbackInline(inline_data) => {
            if let Some(obj) = &mut inline_data.obj_expr {
                on_expr(obj);
            }
            for s in &mut inline_data.body {
                on_stmt(s);
            }
            if let Some(init) = &mut inline_data.reduce_init {
                on_expr(init);
            }
        }
        IrExpr::ArrayMethodInline(inline_data) => {
            if let Some(obj) = &mut inline_data.obj_expr {
                on_expr(obj);
            }
            for arg in &mut inline_data.args {
                on_expr(arg);
            }
        }
        IrExpr::OptionalChain { object, body, .. } => {
            on_expr(object);
            on_expr(body);
        }
        IrExpr::PowExpr { base, exp, .. } => {
            on_expr(base);
            on_expr(exp);
        }
        leaf_expr_variants!() => {}
    }
}

/// Visit direct children of a mutable `IrAssignTarget`.
pub fn for_each_target_child_mut(
    target: &mut IrAssignTarget,
    on_expr: &mut impl FnMut(&mut IrExpr),
) {
    match target {
        IrAssignTarget::Member { object, .. } => on_expr(object),
        IrAssignTarget::Index { object, index, .. } => {
            on_expr(object);
            on_expr(index);
        }
        IrAssignTarget::Destructure(bindings) => {
            for b in bindings {
                if let Some(d) = &mut b.default {
                    on_expr(d);
                }
            }
        }
        IrAssignTarget::Ident(_) | IrAssignTarget::CompileError { .. } => {}
    }
}

// 鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺?//  Private helpers
// 鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺?
fn visit_destructure_binding_decl(
    binding: &IrDestructureBindingDecl,
    on_expr: &mut impl FnMut(&IrExpr),
) {
    if let Some(d) = &binding.default {
        on_expr(d);
    }
}

fn visit_destructure_binding_decl_mut(
    binding: &mut IrDestructureBindingDecl,
    on_expr: &mut impl FnMut(&mut IrExpr),
) {
    if let Some(d) = &mut binding.default {
        on_expr(d);
    }
}
