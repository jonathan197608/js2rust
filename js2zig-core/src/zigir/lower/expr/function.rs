// zigir/lower/expr/function.rs
// Arrow function and function expression lowering.

use oxc_ast::ast::*;

use crate::zigir::ident::IrIdent;
use crate::zigir::types::IrBlock;

use super::Lowerer;

impl Lowerer {
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
        use crate::zigir::types::{IrArrowFn, IrCapture, IrClosure, IrExpr};

        let captured = self.collect_arrow_captures(af);
        let is_concise = af.body.statements.len() == 1
            && matches!(af.body.statements[0], Statement::ExpressionStatement(_));
        let return_type = self.infer_arrow_return_type(af, &captured);
        let params = self.lower_arrow_params(af);

        // Enter a temporary fn context so that lower_stmt / lower_expr can
        // see captured-variable state.
        let arrow_fn_label = format!("_arrow_{}", self.name_mangler.next_name("arrow"));
        let saved_fn = self.enter_fn(&arrow_fn_label, false, Some(return_type.clone()));

        // Set closure_mgr.current_captured so that lower_ident_expr can
        // rewrite captured identifiers to self.xxx.
        let saved_captured = self.closure_mgr.take_captured();
        self.closure_mgr.current_captured = captured
            .iter()
            .map(|(n, t, m)| (n.clone(), t.clone(), *m))
            .collect();

        // Lower the body
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

        // Ownership transfer: clear needs_deinit for returned Map/Set variables
        Self::clear_deinit_for_returned_vars(&mut body);
        self.closure_mgr.restore_captured(saved_captured);
        self.exit_fn(saved_fn);

        if !captured.is_empty() {
            // Has captures → IrClosure
            let idx = self.name_mangler.peek_count("closure");
            let struct_name = IrIdent::new(&format!("Closure_{}", idx));
            let instance_name = IrIdent::new(&format!("_cl_{}", idx));
            self.name_mangler.next_name("closure"); // advance counter

            let ir_captures: Vec<IrCapture> = captured
                .into_iter()
                .map(|(name, zig_type, is_mut)| IrCapture {
                    name: self.make_ident(&name),
                    zig_type,
                    is_mut,
                })
                .collect();

            // Register this as a closure instance
            self.closure_mgr
                .closure_instances
                .insert(instance_name.zig_name.clone());

            // Register the closure struct definition so the Emitter can emit it
            // at module scope.
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
        } else {
            // No captures → IrArrowFn
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
        use crate::zigir::types::{IrCapture, IrClosure, IrExpr, IrFnExpr};

        let name = fe
            .id
            .as_ref()
            .map(|id| id.name.to_string())
            .unwrap_or_else(|| {
                let idx = self.name_mangler.peek_count("_fn_expr");
                self.name_mangler.next_name("_fn_expr"); // advance counter
                format!("_fn_expr_{}", idx)
            });

        let captured = self.detect_fn_body_captures(fe);
        let return_type = self
            .type_info
            .fn_return_types
            .get(&name)
            .cloned()
            .unwrap_or_else(|| self.infer_fn_expr_return_type(fe, &captured));

        // Enter function context
        let _has_throw = fe
            .body
            .as_ref()
            .is_some_and(|b| Self::has_throw_in_stmts(&b.statements));
        let saved_fn = self.enter_fn(&name, false, Some(return_type.clone()));

        // Set captured variables for identifier rewriting
        let saved_captured = self.closure_mgr.take_captured();
        self.closure_mgr.current_captured = captured
            .iter()
            .map(|(n, t, m)| (n.clone(), t.clone(), *m))
            .collect();

        // Lower params
        let params = self.lower_fn_params(fe, &name);

        // Lower body
        let mut body = fe
            .body
            .as_ref()
            .map(|b| self.lower_block(&b.statements))
            .unwrap_or_else(|| IrBlock::new(vec![]));

        // Ownership transfer: clear needs_deinit for returned Map/Set variables
        Self::clear_deinit_for_returned_vars(&mut body);
        self.closure_mgr.restore_captured(saved_captured);
        self.exit_fn(saved_fn);

        if !captured.is_empty() {
            // Has captures → IrClosure
            let struct_name = self.make_ident(&name);
            let instance_name = IrIdent::new(&format!("_{}_inst", name));

            let ir_captures: Vec<IrCapture> = captured
                .into_iter()
                .map(|(n, zig_type, is_mut)| IrCapture {
                    name: self.make_ident(&n),
                    zig_type,
                    is_mut,
                })
                .collect();

            self.closure_mgr
                .closure_instances
                .insert(instance_name.zig_name.clone());

            // Register the closure struct definition so the Emitter can emit it
            // at module scope.
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
        } else {
            // No captures → IrFnExpr
            // Still register a closure struct so the Emitter emits the
            // `const _fn_expr_N = struct { pub fn call() ... }` definition
            // at module scope (the FnExpr reference only emits the name).
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
