// zigir/lower/expr/idents.rs
// IR identifier collection utilities (used for unused parameter detection).

use std::collections::HashSet;

use super::Lowerer;

impl Lowerer {
    /// Collect zig_names of variables that appear as direct return values
    /// (e.g., `return myMap;`). These variables transfer ownership to the
    /// caller and should not have `defer deinit()` auto-cleanup.
    pub(crate) fn collect_returned_var_zig_names_in_block(
        block: &crate::zigir::types::IrBlock,
    ) -> HashSet<String> {
        let mut names = HashSet::new();
        for stmt in &block.stmts {
            Self::collect_returned_var_zig_names_in_stmt(stmt, &mut names);
        }
        names
    }

    /// Clear `needs_deinit` on VarDecls whose names appear in the returned-vars set.
    /// Called after `collect_returned_var_zig_names_in_block` to implement ownership transfer.
    /// Recurses into nested blocks (if/for/while/try/switch) so that VarDecls
    /// declared inside inner blocks are also cleared when returned.
    pub(crate) fn clear_deinit_for_returned_vars(body: &mut crate::zigir::types::IrBlock) {
        let returned_vars = Self::collect_returned_var_zig_names_in_block(body);
        if !returned_vars.is_empty() {
            for stmt in &mut body.stmts {
                Self::clear_returned_deinit_in_stmt(stmt, &returned_vars);
            }
        }
    }

    /// Recursive helper: clear needs_deinit on returned VarDecls,
    /// recursing into nested blocks to match the collection logic in
    /// `collect_returned_var_zig_names_in_stmt`.
    fn clear_returned_deinit_in_stmt(
        stmt: &mut crate::zigir::types::IrStmt,
        returned_vars: &HashSet<String>,
    ) {
        use crate::zigir::types::IrStmt;
        match stmt {
            IrStmt::VarDecl(vd) if vd.needs_deinit && returned_vars.contains(&vd.name.zig_name) => {
                vd.needs_deinit = false;
            }
            IrStmt::If { then, else_, .. } => {
                for s in &mut then.stmts {
                    Self::clear_returned_deinit_in_stmt(s, returned_vars);
                }
                if let Some(e) = else_ {
                    for s in &mut e.stmts {
                        Self::clear_returned_deinit_in_stmt(s, returned_vars);
                    }
                }
            }
            IrStmt::Block(b) => {
                for s in &mut b.stmts {
                    Self::clear_returned_deinit_in_stmt(s, returned_vars);
                }
            }
            IrStmt::Try {
                try_block,
                catch_block,
                finally,
                ..
            } => {
                for s in &mut try_block.stmts {
                    Self::clear_returned_deinit_in_stmt(s, returned_vars);
                }
                for s in &mut catch_block.stmts {
                    Self::clear_returned_deinit_in_stmt(s, returned_vars);
                }
                if let Some(f) = finally {
                    for s in &mut f.stmts {
                        Self::clear_returned_deinit_in_stmt(s, returned_vars);
                    }
                }
            }
            IrStmt::While { body, .. } | IrStmt::DoWhile { body, .. } => {
                for s in &mut body.stmts {
                    Self::clear_returned_deinit_in_stmt(s, returned_vars);
                }
            }
            IrStmt::For { body, .. } => {
                for s in &mut body.stmts {
                    Self::clear_returned_deinit_in_stmt(s, returned_vars);
                }
            }
            IrStmt::ForIn { body, .. } | IrStmt::ForOf { body, .. } => {
                for s in &mut body.stmts {
                    Self::clear_returned_deinit_in_stmt(s, returned_vars);
                }
            }
            IrStmt::Switch { cases, .. } => {
                for c in cases {
                    for s in &mut c.body {
                        Self::clear_returned_deinit_in_stmt(s, returned_vars);
                    }
                }
            }
            // DestructureDecl: no VarDecl children; binding needs_deinit
            // is not tracked in the IR (emitter handles defer generation).
            IrStmt::DestructureDecl(_) => {}
            _ => {}
        }
    }

    fn collect_returned_var_zig_names_in_stmt(
        stmt: &crate::zigir::types::IrStmt,
        names: &mut HashSet<String>,
    ) {
        use crate::zigir::types::IrStmt;
        match stmt {
            IrStmt::Return { value: Some(expr) } => {
                Self::collect_returned_idents_in_expr(expr, names);
            }
            IrStmt::If { then, else_, .. } => {
                for s in &then.stmts {
                    Self::collect_returned_var_zig_names_in_stmt(s, names);
                }
                if let Some(e) = else_ {
                    for s in &e.stmts {
                        Self::collect_returned_var_zig_names_in_stmt(s, names);
                    }
                }
            }
            IrStmt::Block(b) => {
                for s in &b.stmts {
                    Self::collect_returned_var_zig_names_in_stmt(s, names);
                }
            }
            IrStmt::Try {
                try_block,
                catch_block,
                finally,
                ..
            } => {
                for s in &try_block.stmts {
                    Self::collect_returned_var_zig_names_in_stmt(s, names);
                }
                for s in &catch_block.stmts {
                    Self::collect_returned_var_zig_names_in_stmt(s, names);
                }
                if let Some(f) = finally {
                    for s in &f.stmts {
                        Self::collect_returned_var_zig_names_in_stmt(s, names);
                    }
                }
            }
            IrStmt::While { body, .. } | IrStmt::DoWhile { body, .. } => {
                for s in &body.stmts {
                    Self::collect_returned_var_zig_names_in_stmt(s, names);
                }
            }
            IrStmt::For { body, .. } => {
                for s in &body.stmts {
                    Self::collect_returned_var_zig_names_in_stmt(s, names);
                }
            }
            IrStmt::ForIn { body, .. } | IrStmt::ForOf { body, .. } => {
                for s in &body.stmts {
                    Self::collect_returned_var_zig_names_in_stmt(s, names);
                }
            }
            IrStmt::Switch { cases, .. } => {
                for c in cases {
                    for s in &c.body {
                        Self::collect_returned_var_zig_names_in_stmt(s, names);
                    }
                }
            }
            // VarDecl, Assign, Expr, Throw, Break, Continue, Comment, etc.
            // do not contain return statements in their structure.
            _ => {}
        }
    }

    /// Recursively collect identifiers from expressions where ownership is
    /// transferred to the caller (e.g., `return { map: m }` or `return [arr]`).
    /// Direct identifiers, object/array literals, parenthesized expressions,
    /// conditional branches, and comma sequences can embed owned variables
    /// whose `needs_deinit` must be cleared to prevent double-free.
    fn collect_returned_idents_in_expr(
        expr: &crate::zigir::types::IrExpr,
        names: &mut HashSet<String>,
    ) {
        use crate::zigir::types::{IrExpr, IrObjectItem};
        match expr {
            IrExpr::Ident(ident) | IrExpr::TypedIdent { ident, .. } => {
                names.insert(ident.zig_name.clone());
            }
            IrExpr::ObjectLiteral(ol) => {
                for item in &ol.items {
                    match item {
                        IrObjectItem::Field(f) => {
                            Self::collect_returned_idents_in_expr(&f.value, names);
                        }
                        IrObjectItem::Spread(e) => {
                            Self::collect_returned_idents_in_expr(e, names);
                        }
                    }
                }
            }
            IrExpr::ArrayLiteral(al) => {
                for e in &al.elements {
                    Self::collect_returned_idents_in_expr(e, names);
                }
            }
            IrExpr::Paren(inner) => {
                Self::collect_returned_idents_in_expr(inner, names);
            }
            IrExpr::Conditional { then, else_, .. } => {
                Self::collect_returned_idents_in_expr(then, names);
                Self::collect_returned_idents_in_expr(else_, names);
            }
            IrExpr::Sequence(exprs) => {
                for e in exprs {
                    Self::collect_returned_idents_in_expr(e, names);
                }
            }
            IrExpr::Logical { left, right, .. } => {
                // ||, &&, ?? — both operands may be the returned value
                // (short-circuit returns LHS for falsy/false/nullish, else RHS).
                Self::collect_returned_idents_in_expr(left, names);
                Self::collect_returned_idents_in_expr(right, names);
            }
            // Closure: non-mut captures transfer ownership (the closure struct
            // stores a copy of the value). Mut captures use pointers, so
            // ownership stays with the original variable.
            IrExpr::Closure(closure) => {
                for cap in &closure.captured {
                    if !cap.is_mut {
                        names.insert(cap.name.zig_name.clone());
                    }
                }
            }
            // BlockExpr (block expression): `result` is the block's value and
            // carries ownership (e.g. `return { blk: { const x = ...; break
            // :blk x; } };`). Body statements are side effects — only `result`
            // transfers ownership. Matches the BigInt postfix expansion used by
            // operators.rs and the optional-chain wrapping (optional.rs).
            // Without this arm, returned BlockExpr temps drop their deinit,
            // risking double-free when a variable is moved into `result`.
            // (R24-LOW-1)
            IrExpr::BlockExpr { result, .. } => {
                Self::collect_returned_idents_in_expr(result, names);
            }
            // Assignment: the RHS value's ownership transfers to the target,
            // so identifiers embedded in the value must have needs_deinit
            // cleared to prevent double-free. (LOW-20)
            IrExpr::Assign { value, .. } => {
                Self::collect_returned_idents_in_expr(value, names);
            }
            // New: constructor args may embed identifiers whose ownership
            // transfers to the newly constructed object (e.g., host struct
            // constructors that take ownership of a Map argument).
            IrExpr::New(ne) => {
                for arg in &ne.args {
                    Self::collect_returned_idents_in_expr(arg, names);
                }
            }
            // Binary, Call, FieldAccess, etc. — the variable's value is
            // consumed to compute a new value; ownership of the variable
            // itself is NOT transferred, so needs_deinit should remain true.
            _ => {}
        }
    }

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
            IrStmt::Throw { value, .. } => {
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
            IrAssignTarget::CompileError { .. } => {}
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
                for arg in &ce.arguments {
                    if let Some(e) = arg.as_expression() {
                        Self::collect_ast_expr_idents(e, idents);
                    }
                }
            }
            Expression::StaticMemberExpression(me) => {
                Self::collect_ast_expr_idents(&me.object, idents);
            }
            Expression::ComputedMemberExpression(me) => {
                Self::collect_ast_expr_idents(&me.object, idents);
                Self::collect_ast_expr_idents(&me.expression, idents);
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
            IrExpr::Ident(name) | IrExpr::TypedIdent { ident: name, .. } => {
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
                if let Some(ref obj_expr) = bc.obj_expr {
                    Self::collect_ir_idents_in_expr(obj_expr, idents);
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
                // R31-PASS-1: Must collect obj_name and recurse into obj_expr,
                // otherwise the array variable appears unused and may be
                // erroneously removed by dead-code elimination.
                idents.insert(inline_data.obj_name.clone());
                if let Some(ref obj_expr) = inline_data.obj_expr {
                    Self::collect_ir_idents_in_expr(obj_expr, idents);
                }
                for s in &inline_data.body {
                    Self::collect_ir_idents_in_stmt(s, idents);
                }
                if let Some(ref init) = inline_data.reduce_init {
                    Self::collect_ir_idents_in_expr(init, idents);
                }
            }
            IrExpr::ArrayMethodInline(inline_data) => {
                // R31-PASS-1: Same fix as ArrayCallbackInline above.
                idents.insert(inline_data.obj_name.clone());
                if let Some(ref obj_expr) = inline_data.obj_expr {
                    Self::collect_ir_idents_in_expr(obj_expr, idents);
                }
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
            IrExpr::RemExpr { left, right, .. } => {
                Self::collect_ir_idents_in_expr(left, idents);
                Self::collect_ir_idents_in_expr(right, idents);
            }
            IrExpr::DivExpr { left, right, .. } => {
                Self::collect_ir_idents_in_expr(left, idents);
                Self::collect_ir_idents_in_expr(right, idents);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ZigType;
    use crate::zigir::ident::IrIdent;
    use crate::zigir::types::{IrBlock, IrExpr, IrStmt, IrVarDecl};

    #[test]
    fn test_collect_returned_var_direct() {
        // return myMap; → myMap should be collected
        let block = IrBlock::new(vec![
            IrStmt::VarDecl(IrVarDecl {
                name: IrIdent::new("myMap"),
                is_const: true,
                zig_type: Some(ZigType::NamedStruct("Map".to_string())),
                init: None,
                is_json_parse: false,
                needs_var_suppression: false,
                needs_deinit: true,
            }),
            IrStmt::Return {
                value: Some(IrExpr::Ident(IrIdent::new("myMap"))),
            },
        ]);
        let names = Lowerer::collect_returned_var_zig_names_in_block(&block);
        assert!(names.contains("myMap"));
        assert_eq!(names.len(), 1);
    }

    #[test]
    fn test_collect_returned_var_in_if() {
        // if (cond) { return s; } → s should be collected
        let block = IrBlock::new(vec![IrStmt::If {
            cond: IrExpr::BoolLiteral(true),
            then: IrBlock::new(vec![IrStmt::Return {
                value: Some(IrExpr::Ident(IrIdent::new("s"))),
            }]),
            else_: None,
        }]);
        let names = Lowerer::collect_returned_var_zig_names_in_block(&block);
        assert!(names.contains("s"));
    }

    #[test]
    fn test_collect_returned_var_not_ident() {
        // return 42; → nothing collected
        let block = IrBlock::new(vec![IrStmt::Return {
            value: Some(IrExpr::IntLiteral(42)),
        }]);
        let names = Lowerer::collect_returned_var_zig_names_in_block(&block);
        assert!(names.is_empty());
    }

    #[test]
    fn test_collect_returned_var_no_return() {
        // No return → empty
        let block = IrBlock::new(vec![IrStmt::VarDecl(IrVarDecl {
            name: IrIdent::new("m"),
            is_const: true,
            zig_type: Some(ZigType::NamedStruct("Map".to_string())),
            init: None,
            is_json_parse: false,
            needs_var_suppression: false,
            needs_deinit: true,
        })]);
        let names = Lowerer::collect_returned_var_zig_names_in_block(&block);
        assert!(names.is_empty());
    }

    #[test]
    fn test_collect_returned_var_nested_object() {
        // return { map: m }; → m should be collected (P0-1 fix)
        use crate::zigir::types::{IrObjectField, IrObjectItem, IrObjectLiteral};
        let block = IrBlock::new(vec![
            IrStmt::VarDecl(IrVarDecl {
                name: IrIdent::new("m"),
                is_const: true,
                zig_type: Some(ZigType::NamedStruct("Map".to_string())),
                init: None,
                is_json_parse: false,
                needs_var_suppression: false,
                needs_deinit: true,
            }),
            IrStmt::Return {
                value: Some(IrExpr::ObjectLiteral(IrObjectLiteral {
                    items: vec![IrObjectItem::Field(IrObjectField {
                        key: "map".to_string(),
                        value: IrExpr::Ident(IrIdent::new("m")),
                        is_computed: false,
                    })],
                })),
            },
        ]);
        let names = Lowerer::collect_returned_var_zig_names_in_block(&block);
        assert!(
            names.contains("m"),
            "nested object field ident should be collected"
        );
    }

    #[test]
    fn test_collect_returned_var_nested_array() {
        // return [m]; → m should be collected (P0-1 fix)
        let block = IrBlock::new(vec![IrStmt::Return {
            value: Some(IrExpr::ArrayLiteral(crate::zigir::types::IrArrayLiteral {
                elements: vec![IrExpr::Ident(IrIdent::new("m"))],
                spread_indices: vec![],
            })),
        }]);
        let names = Lowerer::collect_returned_var_zig_names_in_block(&block);
        assert!(
            names.contains("m"),
            "nested array element ident should be collected"
        );
    }

    #[test]
    fn test_collect_returned_var_paren() {
        // return (m); → m should be collected
        let block = IrBlock::new(vec![IrStmt::Return {
            value: Some(IrExpr::Paren(Box::new(IrExpr::Ident(IrIdent::new("m"))))),
        }]);
        let names = Lowerer::collect_returned_var_zig_names_in_block(&block);
        assert!(names.contains("m"));
    }

    #[test]
    fn test_collect_returned_var_conditional() {
        // return cond ? a : b; → both a and b should be collected
        let block = IrBlock::new(vec![IrStmt::Return {
            value: Some(IrExpr::Conditional {
                cond: Box::new(IrExpr::BoolLiteral(true)),
                then: Box::new(IrExpr::Ident(IrIdent::new("a"))),
                else_: Box::new(IrExpr::Ident(IrIdent::new("b"))),
            }),
        }]);
        let names = Lowerer::collect_returned_var_zig_names_in_block(&block);
        assert!(names.contains("a"));
        assert!(names.contains("b"));
    }

    #[test]
    fn test_collect_returned_var_not_method_call() {
        // return m + 1; → m should NOT be collected (ownership not transferred)
        use crate::zigir::ops::BinOp;
        let block = IrBlock::new(vec![IrStmt::Return {
            value: Some(IrExpr::Binary {
                op: BinOp::Add,
                left: Box::new(IrExpr::Ident(IrIdent::new("m"))),
                right: Box::new(IrExpr::IntLiteral(1)),
                left_type: None,
                right_type: None,
            }),
        }]);
        let names = Lowerer::collect_returned_var_zig_names_in_block(&block);
        assert!(
            !names.contains("m"),
            "binary expression should not transfer ownership"
        );
    }

    // LOW-1: clear_deinit_for_returned_vars must recurse into nested blocks
    // (if/try/for/switch) to clear needs_deinit on VarDecls whose names
    // appear in the returned-vars set.  Before the fix, only top-level
    // VarDecls were processed, so a VarDecl inside an if-block that was
    // returned from that block would retain needs_deinit=true (double-free).
    #[test]
    fn test_clear_deinit_nested_if() {
        // VarDecl inside if-block, return inside same if-block.
        let mut block = IrBlock::new(vec![IrStmt::If {
            cond: IrExpr::BoolLiteral(true),
            then: IrBlock::new(vec![
                IrStmt::VarDecl(IrVarDecl {
                    name: IrIdent::new("inner_map"),
                    is_const: true,
                    zig_type: Some(ZigType::NamedStruct("Map".to_string())),
                    init: None,
                    is_json_parse: false,
                    needs_var_suppression: false,
                    needs_deinit: true,
                }),
                IrStmt::Return {
                    value: Some(IrExpr::Ident(IrIdent::new("inner_map"))),
                },
            ]),
            else_: None,
        }]);
        Lowerer::clear_deinit_for_returned_vars(&mut block);
        // Verify that needs_deinit was cleared on the VarDecl inside the
        // if-block.  Before LOW-1, this would still be true.
        let cleared = match &block.stmts[0] {
            IrStmt::If { then, .. } => match &then.stmts[0] {
                IrStmt::VarDecl(vd) => !vd.needs_deinit,
                _ => false,
            },
            _ => false,
        };
        assert!(
            cleared,
            "needs_deinit should be cleared for returned var inside if-block"
        );
    }

    // LOW-2: collect_returned_idents_in_expr must handle LogicalExpression
    // (||, &&, ??) so that variables in either operand are collected as
    // returned (ownership may transfer).
    #[test]
    fn test_collect_returned_var_logical() {
        // return a || b; → both a and b should be collected
        use crate::zigir::ops::LogicalOp;
        let block = IrBlock::new(vec![IrStmt::Return {
            value: Some(IrExpr::Logical {
                op: LogicalOp::Or,
                left: Box::new(IrExpr::Ident(IrIdent::new("a"))),
                right: Box::new(IrExpr::Ident(IrIdent::new("b"))),
                left_type: None,
                right_type: None,
            }),
        }]);
        let names = Lowerer::collect_returned_var_zig_names_in_block(&block);
        assert!(
            names.contains("a"),
            "logical || left operand should be collected as returned"
        );
        assert!(
            names.contains("b"),
            "logical || right operand should be collected as returned"
        );
    }
}
