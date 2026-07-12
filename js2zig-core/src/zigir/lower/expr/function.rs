// zigir/lower/expr/function.rs
// Arrow function and function expression lowering.

use oxc_ast::ast::*;

use crate::types::ZigType;
use crate::zigir::ident::IrIdent;
use crate::zigir::types::IrBlock;

use super::Lowerer;

impl Lowerer {
    // ─── Shared pipeline ──────────────────────────────────

    /// Enter the function context for closure lowering: save fn state, set captured vars.
    /// Returns the saved state that must be passed to `exit_closure_context`.
    fn enter_closure_context(
        &mut self,
        name: &str,
        return_type: ZigType,
        captured: &[(String, ZigType, bool)],
    ) -> (
        Option<crate::zigir::lower::helpers::FnContext>,
        Vec<(String, ZigType, bool)>,
    ) {
        let saved_fn = self.enter_fn(name, false, Some(return_type));
        let saved_captured = self.closure_mgr.take_captured();
        self.closure_mgr.current_captured = captured
            .iter()
            .map(|(n, t, m)| (n.clone(), t.clone(), *m))
            .collect();
        (saved_fn, saved_captured)
    }

    /// Exit the function context for closure lowering: clear deinit, restore captured, exit fn.
    fn exit_closure_context(
        &mut self,
        body: &mut IrBlock,
        saved_fn: Option<crate::zigir::lower::helpers::FnContext>,
        saved_captured: Vec<(String, ZigType, bool)>,
    ) {
        Self::clear_deinit_for_returned_vars(body);
        self.closure_mgr.restore_captured(saved_captured);
        self.exit_fn(saved_fn);
    }

    /// Build an `IrClosure` (struct + instance) from closure lowering results.
    ///
    /// Also registers the closure struct definition in `pending_arrow_structs`.
    ///
    /// Two scenarios produce `@compileError`:
    /// 1. Any captured variable is itself a field on the enclosing closure.
    /// 2. A nested `IrExpr::Closure` in the body initializes one of its
    ///    captured fields with a name that overlaps with THIS closure's
    ///    own captured fields — the emitter would use a bare name instead
    ///    of `self.field`, so we reject it explicitly.
    fn build_closure_expr(
        &mut self,
        captured: Vec<(String, ZigType, bool)>,
        params: Vec<crate::zigir::types::IrParam>,
        return_type: ZigType,
        body: IrBlock,
        struct_name: IrIdent,
        instance_name: IrIdent,
    ) -> crate::zigir::types::IrExpr {
        use crate::zigir::types::{IrClosure, IrExpr};

        let ir_captures = self.make_ir_captures(captured.clone().into_iter().collect());

        self.closure_mgr
            .closure_instances
            .insert(instance_name.zig_name.clone());

        let mut body = body;

        // Check 1: nested closure capture (variable captured from enclosing
        // closure's fields).
        let nested = self.detect_nested_captures(&captured);

        // Check 2: body contains a Closure whose init values overlap with
        // our own captured field names.  The emitter writes bare names for
        // init values, but inside our call() method those need `self.`
        // prefix — unsupported, so emit @compileError.
        let own_capture_names: Vec<String> = ir_captures
            .iter()
            .map(|c| c.name.zig_name.clone())
            .collect();
        let overlap = Self::find_nested_closure_capture_overlap(&body, &own_capture_names);

        if !nested.is_empty() || !overlap.is_empty() {
            let mut parts = Vec::new();
            if !nested.is_empty() {
                parts.push(format!(
                    "variable(s) {} captured from enclosing closure",
                    nested.join(", ")
                ));
            }
            if !overlap.is_empty() {
                parts.push(format!(
                    "nested closure init uses own captured field(s) {}",
                    overlap.join(", ")
                ));
            }
            let msg = format!(
                "nested closure capture is not supported: {}",
                parts.join("; ")
            );
            body.stmts.insert(
                0,
                crate::zigir::types::IrStmt::CompileError {
                    span: crate::zigir::source_span::SourceSpan::default(),
                    msg,
                },
            );
        }

        self.pending_arrow_structs
            .push(crate::zigir::types::IrClosureStruct {
                name: struct_name.clone(),
                captured: ir_captures.clone(),
                fn_params: params.clone(),
                return_type: return_type.clone(),
                typeof_return_body: None,
                body: body.clone(),
            });

        IrExpr::Closure(IrClosure {
            struct_name,
            captured: ir_captures,
            fn_params: params,
            return_type,
            body,
            instance_name,
        })
    }

    /// Recursively scan an IR block for `IrExpr::Closure` nodes whose
    /// captured field names overlap with `own_capture_names`.  Returns
    /// the names that appear in both sets.
    fn find_nested_closure_capture_overlap(
        block: &IrBlock,
        own_capture_names: &[String],
    ) -> Vec<String> {
        let mut overlap = std::collections::HashSet::new();
        for stmt in &block.stmts {
            Self::scan_stmt_for_closure_overlap(stmt, own_capture_names, &mut overlap);
        }
        let mut result: Vec<String> = overlap.into_iter().collect();
        result.sort();
        result
    }

    fn scan_stmt_for_closure_overlap(
        stmt: &crate::zigir::types::IrStmt,
        own: &[String],
        overlap: &mut std::collections::HashSet<String>,
    ) {
        use crate::zigir::types::IrStmt;
        match stmt {
            IrStmt::Expr(expr) | IrStmt::Return { value: Some(expr) } => {
                Self::scan_expr_for_closure_overlap(expr, own, overlap);
            }
            IrStmt::VarDecl(vd) => {
                if let Some(init) = &vd.init {
                    Self::scan_expr_for_closure_overlap(init, own, overlap);
                }
            }
            IrStmt::Assign { value, .. } => {
                Self::scan_expr_for_closure_overlap(value, own, overlap);
            }
            IrStmt::If { then, else_, .. } => {
                for s in &then.stmts {
                    Self::scan_stmt_for_closure_overlap(s, own, overlap);
                }
                if let Some(eb) = else_ {
                    for s in &eb.stmts {
                        Self::scan_stmt_for_closure_overlap(s, own, overlap);
                    }
                }
            }
            IrStmt::Block(inner) => {
                for s in &inner.stmts {
                    Self::scan_stmt_for_closure_overlap(s, own, overlap);
                }
            }
            IrStmt::While { body, .. } | IrStmt::DoWhile { body, .. } => {
                for s in &body.stmts {
                    Self::scan_stmt_for_closure_overlap(s, own, overlap);
                }
            }
            IrStmt::For { body, .. } | IrStmt::ForOf { body, .. } => {
                for s in &body.stmts {
                    Self::scan_stmt_for_closure_overlap(s, own, overlap);
                }
            }
            IrStmt::Switch { cases, .. } => {
                for case in cases {
                    for s in &case.body {
                        Self::scan_stmt_for_closure_overlap(s, own, overlap);
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
                    Self::scan_stmt_for_closure_overlap(s, own, overlap);
                }
                for s in &catch_block.stmts {
                    Self::scan_stmt_for_closure_overlap(s, own, overlap);
                }
                if let Some(fb) = finally {
                    for s in &fb.stmts {
                        Self::scan_stmt_for_closure_overlap(s, own, overlap);
                    }
                }
            }
            IrStmt::NestedFnDecl { .. } => {
                // Nested fn decl contains its own closure — scan the instance
                if let IrStmt::NestedFnDecl {
                    instance: Some(cl), ..
                } = stmt
                {
                    for cap in &cl.captured {
                        if own.contains(&cap.name.zig_name) {
                            overlap.insert(cap.name.zig_name.clone());
                        }
                    }
                }
            }
            // Return None, Throw, Break, Continue, DestructureDecl, CompileError, Comment — no expr
            _ => {}
        }
    }

    fn scan_expr_for_closure_overlap(
        expr: &crate::zigir::types::IrExpr,
        own: &[String],
        overlap: &mut std::collections::HashSet<String>,
    ) {
        use crate::zigir::types::IrExpr;
        match expr {
            IrExpr::Closure(c) => {
                for cap in &c.captured {
                    if own.contains(&cap.name.zig_name) {
                        overlap.insert(cap.name.zig_name.clone());
                    }
                }
                // Also scan the closure body for deeper nesting
                for stmt in &c.body.stmts {
                    Self::scan_stmt_for_closure_overlap(stmt, own, overlap);
                }
            }
            IrExpr::Binary { left, right, .. } => {
                Self::scan_expr_for_closure_overlap(left, own, overlap);
                Self::scan_expr_for_closure_overlap(right, own, overlap);
            }
            IrExpr::Call(call) => {
                Self::scan_expr_for_closure_overlap(&call.callee, own, overlap);
                for arg in &call.args {
                    Self::scan_expr_for_closure_overlap(arg, own, overlap);
                }
            }
            IrExpr::FieldAccess { object, .. } | IrExpr::IndexAccess { object, .. } => {
                Self::scan_expr_for_closure_overlap(object, own, overlap);
            }
            IrExpr::ArrowFn(af) => {
                for stmt in &af.body.stmts {
                    Self::scan_stmt_for_closure_overlap(stmt, own, overlap);
                }
            }
            IrExpr::FnExpr(fe) => {
                for stmt in &fe.body.stmts {
                    Self::scan_stmt_for_closure_overlap(stmt, own, overlap);
                }
            }
            IrExpr::ArrayLiteral(arr) => {
                for el in &arr.elements {
                    Self::scan_expr_for_closure_overlap(el, own, overlap);
                }
            }
            IrExpr::Conditional { cond, then, else_ } => {
                Self::scan_expr_for_closure_overlap(cond, own, overlap);
                Self::scan_expr_for_closure_overlap(then, own, overlap);
                Self::scan_expr_for_closure_overlap(else_, own, overlap);
            }
            IrExpr::Paren(inner)
            | IrExpr::Spread(inner)
            | IrExpr::Typeof(inner)
            | IrExpr::Void(inner) => {
                Self::scan_expr_for_closure_overlap(inner, own, overlap);
            }
            IrExpr::Unary { operand, .. } => {
                Self::scan_expr_for_closure_overlap(operand, own, overlap);
            }
            // Update target is IrAssignTarget — no nested Closure possible
            IrExpr::Logical { left, right, .. } => {
                Self::scan_expr_for_closure_overlap(left, own, overlap);
                Self::scan_expr_for_closure_overlap(right, own, overlap);
            }
            IrExpr::ObjectLiteral(obj) => {
                for item in &obj.items {
                    match item {
                        crate::zigir::types::IrObjectItem::Field(f) => {
                            Self::scan_expr_for_closure_overlap(&f.value, own, overlap);
                        }
                        crate::zigir::types::IrObjectItem::Spread(expr) => {
                            Self::scan_expr_for_closure_overlap(expr, own, overlap);
                        }
                    }
                }
            }
            IrExpr::TemplateLiteral { exprs, .. } => {
                for e in exprs {
                    Self::scan_expr_for_closure_overlap(e, own, overlap);
                }
            }
            IrExpr::BlockExpr { body, result, .. } => {
                for stmt in body {
                    Self::scan_stmt_for_closure_overlap(stmt, own, overlap);
                }
                Self::scan_expr_for_closure_overlap(result, own, overlap);
            }
            // Literals, Ident, This, Null, Undefined, etc. — safe to skip
            _ => {}
        }
    }

    /// Lower an arrow function expression.
    ///
    /// If the arrow captures variables from the enclosing scope, we produce
    /// an `IrClosure` (struct + instance).  Otherwise we produce a plain
    /// `IrArrowFn` (struct + static call — Zig 0.16 doesn't allow nested
    /// fn declarations with return statements).
    pub(super) fn lower_arrow_fn(
        &mut self,
        af: &ArrowFunctionExpression,
    ) -> crate::zigir::types::IrExpr {
        use crate::zigir::types::{IrArrowFn, IrExpr};

        let captured = self.collect_arrow_captures(af);
        let is_concise = af.body.statements.len() == 1
            && matches!(af.body.statements[0], Statement::ExpressionStatement(_));
        let return_type = self.infer_arrow_return_type(af, &captured);
        let params = self.lower_arrow_params(af);
        let arrow_fn_label = format!("_arrow_{}", self.name_mangler.next_name("arrow"));

        // Enter fn context with captured vars set up
        let (saved_fn, saved_captured) =
            self.enter_closure_context(&arrow_fn_label, return_type.clone(), &captured);

        // Lower body
        let mut body = if is_concise {
            if let Statement::ExpressionStatement(es) = &af.body.statements[0] {
                let expr_ir = self.lower_expr(&es.expression);
                IrBlock::new(vec![crate::zigir::types::IrStmt::Return {
                    value: Some(expr_ir),
                }])
            } else {
                self.lower_block(&af.body.statements)
            }
        } else {
            self.lower_block(&af.body.statements)
        };

        self.exit_closure_context(&mut body, saved_fn, saved_captured);

        if !captured.is_empty() {
            let idx = self.name_mangler.peek_count("closure");
            let struct_name = IrIdent::new(&format!("Closure_{}", idx));
            let instance_name = IrIdent::new(&format!("_cl_{}", idx));
            self.name_mangler.next_name("closure");

            self.build_closure_expr(
                captured,
                params,
                return_type,
                body,
                struct_name,
                instance_name,
            )
        } else {
            IrExpr::ArrowFn(IrArrowFn {
                params,
                return_type,
                body,
                is_concise,
            })
        }
    }

    /// Lower a function expression.
    ///
    /// Like arrow functions, if the function captures variables we produce
    /// an `IrClosure`; otherwise a plain `IrFnExpr`.
    pub(super) fn lower_fn_expr(&mut self, fe: &Function) -> crate::zigir::types::IrExpr {
        use crate::zigir::types::{IrExpr, IrFnExpr};

        let name = fe
            .id
            .as_ref()
            .map(|id| id.name.to_string())
            .unwrap_or_else(|| {
                let idx = self.name_mangler.peek_count("_fn_expr");
                self.name_mangler.next_name("_fn_expr");
                format!("_fn_expr_{}", idx)
            });

        let captured = self.detect_fn_body_captures(fe);
        let return_type = self
            .type_info
            .fn_return_types
            .get(&name)
            .cloned()
            .unwrap_or_else(|| self.infer_fn_expr_return_type(fe, &captured));

        let params = self.lower_fn_params(fe, &name);

        // Enter fn context with captured vars set up
        let (saved_fn, saved_captured) =
            self.enter_closure_context(&name, return_type.clone(), &captured);

        // Lower body
        let mut body = fe
            .body
            .as_ref()
            .map(|b| self.lower_block(&b.statements))
            .unwrap_or_else(|| IrBlock::new(vec![]));

        self.exit_closure_context(&mut body, saved_fn, saved_captured);

        if !captured.is_empty() {
            let struct_name = self.make_ident(&name);
            let instance_name = IrIdent::new(&format!("_{}_inst", name));
            self.build_closure_expr(
                captured,
                params,
                return_type,
                body,
                struct_name,
                instance_name,
            )
        } else {
            // No captures → IrFnExpr, but still register a closure struct so
            // the Emitter emits the `const _fn_expr_N = struct { pub fn call() ... }`
            // definition at module scope.
            let struct_name = self.make_ident(&name);
            self.pending_arrow_structs
                .push(crate::zigir::types::IrClosureStruct {
                    name: struct_name.clone(),
                    captured: vec![],
                    fn_params: params.clone(),
                    return_type: return_type.clone(),
                    typeof_return_body: None,
                    body: body.clone(),
                });

            IrExpr::FnExpr(IrFnExpr {
                name: Some(self.make_ident(&name)),
                params,
                return_type,
                body,
            })
        }
    }
}
