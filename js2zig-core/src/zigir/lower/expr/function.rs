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
    fn build_closure_expr(
        &mut self,
        captured: Vec<(String, ZigType, bool)>,
        params: Vec<crate::zigir::types::IrParam>,
        return_type: ZigType,
        body: IrBlock,
        struct_name: IrIdent,
        instance_name: IrIdent,
    ) -> crate::zigir::types::IrExpr {
        use crate::zigir::types::{IrCapture, IrClosure, IrExpr};

        let ir_captures: Vec<IrCapture> = captured
            .into_iter()
            .map(|(name, zig_type, is_mut)| IrCapture {
                name: self.make_ident(&name),
                zig_type,
                is_mut,
            })
            .collect();

        self.closure_mgr
            .closure_instances
            .insert(instance_name.zig_name.clone());

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

        let _has_throw = fe
            .body
            .as_ref()
            .is_some_and(|b| Self::has_throw_in_stmts(&b.statements));
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
