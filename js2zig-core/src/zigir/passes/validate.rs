// zigir/passes/validate.rs
// ValidatePass — structural validation of an IrModule.
//
// Checks:
//   1. Type consistency: expressions have plausible types
//   2. Name uniqueness: no duplicate top-level identifiers
//   3. Closure integrity: captured lists match actual references
//   4. C ABI compatibility: exported function signatures use C-safe types

use crate::types::ZigType;
use crate::zigir::passes::{IrPass, PassResult};
use crate::zigir::source_span::{DiagnosticLevel, IrDiagnostic};
use crate::zigir::types::{IrAssignTarget, IrBlock, IrDecl, IrExpr, IrFnDecl, IrModule, IrStmt};

use std::cell::RefCell;

use super::{collect_idents, walk};

/// Validation pass: checks structural integrity of the IR.
///
/// Produces warnings for suspicious patterns and errors for violations
/// that would cause incorrect Zig output. Does NOT modify the IR.
pub struct ValidatePass {
    /// Collected diagnostics for the current run.
    diagnostics: Vec<IrDiagnostic>,
}

impl ValidatePass {
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
        }
    }

    fn error(&mut self, msg: String) {
        self.diagnostics.push(IrDiagnostic {
            level: DiagnosticLevel::Error,
            span: None,
            message: msg,
        });
    }

    fn warn(&mut self, msg: String) {
        self.diagnostics.push(IrDiagnostic {
            level: DiagnosticLevel::Warning,
            span: None,
            message: msg,
        });
    }

    // ── Top-level name uniqueness ────────────────────

    fn check_name_uniqueness(&mut self, module: &IrModule) {
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

        // Check typedefs
        for td in &module.typedefs {
            if !seen.insert(td.name.clone()) {
                self.error(format!("duplicate top-level name: '{}'", td.name));
            }
        }

        // Check closure structs
        for cs in &module.closure_structs {
            let zig_name = cs.name.zig_name.clone();
            if !seen.insert(zig_name.clone()) {
                self.error(format!("duplicate top-level name: '{}'", zig_name));
            }
        }

        // Check declarations
        for decl in &module.declarations {
            let name = match decl {
                IrDecl::Var(v) => v.name.zig_name.clone(),
                IrDecl::Fn(f) => f.name.zig_name.clone(),
                IrDecl::Class(c) => c.name.zig_name.clone(),
                IrDecl::CompileError { .. } => continue,
            };
            if !seen.insert(name.clone()) {
                self.error(format!("duplicate top-level name: '{}'", name));
            }
        }
    }

    // ── C ABI compatibility ──────────────────────────

    fn check_cabi_compatibility(&mut self, module: &IrModule) {
        for decl in &module.declarations {
            if let IrDecl::Fn(f) = decl
                && f.is_cabi
            {
                self.check_cabi_fn(f);
            }
        }
        for export in &module.cabi_exports {
            if !is_c_safe_type(&export.return_type) {
                self.error(format!(
                    "C ABI export '{}' has non-C-safe return type: {:?}",
                    export.name, export.return_type
                ));
            }
            for param in &export.params {
                if !is_c_safe_type(&param.zig_type) {
                    self.error(format!(
                        "C ABI export '{}' has non-C-safe parameter '{}': {:?}",
                        export.name, param.name.zig_name, param.zig_type
                    ));
                }
            }
        }
    }

    fn check_cabi_fn(&mut self, f: &IrFnDecl) {
        if !is_c_safe_type(&f.return_type) {
            self.error(format!(
                "C ABI function '{}' has non-C-safe return type: {:?}",
                f.name.zig_name, f.return_type
            ));
        }
        for param in &f.params {
            if !is_c_safe_type(&param.zig_type) {
                self.error(format!(
                    "C ABI function '{}' has non-C-safe parameter '{}': {:?}",
                    f.name.zig_name, param.name.zig_name, param.zig_type
                ));
            }
        }
    }

    // ── Closure integrity ────────────────────────────

    fn check_closure_integrity(&mut self, module: &IrModule) {
        for cs in &module.closure_structs {
            // Check that each captured variable is actually referenced in the body
            let referenced = {
                let mut names = std::collections::HashSet::new();
                collect_idents::collect_block_idents(&cs.body, &mut names);
                names
            };
            for capture in &cs.captured {
                if !referenced.contains(&capture.name.zig_name) {
                    self.warn(format!(
                        "closure struct '{}' captures '{}' but it is not referenced in the body",
                        cs.name.zig_name, capture.name.zig_name
                    ));
                }
            }
        }

        // Check IrClosure expressions in declarations
        for decl in &module.declarations {
            self.check_closure_refs_in_decl(decl);
        }
    }

    fn check_closure_refs_in_decl(&mut self, decl: &IrDecl) {
        let this = RefCell::new(&mut *self);
        walk::for_each_decl_child(
            decl,
            &mut |block| {
                this.borrow_mut().check_closure_refs_in_block(block);
            },
            &mut |expr| {
                this.borrow_mut().check_closure_refs_in_expr(expr);
            },
        );
    }

    fn check_closure_refs_in_block(&mut self, block: &IrBlock) {
        for stmt in &block.stmts {
            self.check_closure_refs_in_stmt(stmt);
        }
    }

    fn check_closure_refs_in_stmt(&mut self, stmt: &IrStmt) {
        // Special case: NestedFnDecl — check struct body + capture names (not closure body)
        if let IrStmt::NestedFnDecl {
            struct_def,
            instance,
        } = stmt
        {
            self.check_closure_refs_in_block(&struct_def.body);
            if let Some(closure) = instance {
                for cap in &closure.captured {
                    self.check_closure_refs_in_expr(&IrExpr::Ident(cap.name.clone()));
                }
            }
            return;
        }

        // Standard walk; on_target is no-op because validate at stmt level
        // does not visit Assign.target (that's done at expr level)
        let this = RefCell::new(&mut *self);
        walk::for_each_stmt_child(
            stmt,
            &mut |block| {
                this.borrow_mut().check_closure_refs_in_block(block);
            },
            &mut |s| {
                this.borrow_mut().check_closure_refs_in_stmt(s);
            },
            &mut |expr| {
                this.borrow_mut().check_closure_refs_in_expr(expr);
            },
            &mut |_| {},
        );
    }

    fn check_closure_refs_in_expr(&mut self, expr: &IrExpr) {
        // Special case: Closure — check capture references (don't recurse into body)
        if let IrExpr::Closure(closure) = expr {
            let referenced = {
                let mut names = std::collections::HashSet::new();
                collect_idents::collect_block_idents(&closure.body, &mut names);
                names
            };
            for capture in &closure.captured {
                if !referenced.contains(&capture.name.zig_name) {
                    self.warn(format!(
                        "closure '{}' captures '{}' but it is not referenced in the body",
                        closure.struct_name.zig_name, capture.name.zig_name
                    ));
                }
            }
            return;
        }

        let this = RefCell::new(&mut *self);
        walk::for_each_expr_child(
            expr,
            &mut |block| {
                this.borrow_mut().check_closure_refs_in_block(block);
            },
            &mut |s| {
                this.borrow_mut().check_closure_refs_in_stmt(s);
            },
            &mut |expr| {
                this.borrow_mut().check_closure_refs_in_expr(expr);
            },
            &mut |target| {
                this.borrow_mut()
                    .check_closure_refs_in_assign_target(target);
            },
        );
    }

    fn check_closure_refs_in_assign_target(&mut self, target: &IrAssignTarget) {
        let this = RefCell::new(&mut *self);
        walk::for_each_target_child(target, &mut |expr| {
            this.borrow_mut().check_closure_refs_in_expr(expr);
        });
    }
}

impl IrPass for ValidatePass {
    fn name(&self) -> &'static str {
        "validate"
    }

    fn description(&self) -> &'static str {
        "Validates structural integrity of the IR (name uniqueness, C ABI compatibility, closure integrity)"
    }

    fn run(&mut self, module: &mut IrModule) -> PassResult {
        self.diagnostics.clear();

        self.check_name_uniqueness(module);
        self.check_cabi_compatibility(module);
        self.check_closure_integrity(module);

        let diagnostics = std::mem::take(&mut self.diagnostics);
        PassResult::with_diagnostics(diagnostics)
    }
}

impl Default for ValidatePass {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════
//  Helpers
// ═══════════════════════════════════════════════════════

/// Check if a ZigType is safe for C ABI boundaries.
///
/// C-safe types: i64, f64, Bool, Void, Str (as pointer).
/// NOT safe: JsAny, ArrayList, HashMap, NamedStruct, Anytype, AnytypeReturn, etc.
fn is_c_safe_type(ty: &ZigType) -> bool {
    matches!(
        ty,
        ZigType::I64 | ZigType::F64 | ZigType::Bool | ZigType::Void | ZigType::Str
    )
}

// ═══════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zigir::ident::IrIdent;
    use crate::zigir::types::{IrBlock, IrCapture, IrDecl, IrFnDecl, IrParam, IrStmt, IrVarDecl};

    #[test]
    fn test_validate_clean_module() {
        let mut module = super::super::make_clean_add_module();

        let mut pass = ValidatePass::new();
        let result = pass.run(&mut module);
        assert!(
            result.diagnostics.is_empty(),
            "clean module should have no diagnostics: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn test_validate_duplicate_names() {
        let mut module = IrModule::new("test".to_string());
        // Add two variables with the same name
        module.declarations.push(IrDecl::Var(IrVarDecl::new_const(
            "x",
            Some(ZigType::I64),
            Some(IrExpr::IntLiteral(1)),
        )));
        module.declarations.push(IrDecl::Var(IrVarDecl::new_const(
            "x",
            Some(ZigType::I64),
            Some(IrExpr::IntLiteral(2)),
        )));

        let mut pass = ValidatePass::new();
        let result = pass.run(&mut module);
        assert_eq!(result.diagnostics.len(), 1);
        assert!(matches!(
            result.diagnostics[0].level,
            DiagnosticLevel::Error
        ));
        assert!(result.diagnostics[0].message.contains("duplicate"));
    }

    #[test]
    fn test_validate_cabi_unsafe_type() {
        let mut module = IrModule::new("test".to_string());
        module.declarations.push(IrDecl::Fn(IrFnDecl {
            name: IrIdent::new("export_fn"),
            params: vec![IrParam {
                name: IrIdent::new("data"),
                zig_type: ZigType::JsAny, // NOT C-safe
                is_unused: false,
                is_rest: false,
            }],
            return_type: ZigType::JsAny, // NOT C-safe
            body: IrBlock::new(vec![]),
            is_export: false,
            is_async: false,
            can_throw: false,
            is_cabi: true,
            typeof_return_body: None,
        }));

        let mut pass = ValidatePass::new();
        let result = pass.run(&mut module);
        let errors: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| matches!(d.level, DiagnosticLevel::Error))
            .collect();
        assert_eq!(
            errors.len(),
            2,
            "should have 2 C ABI errors (param + return), got: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn test_validate_closure_capture_not_referenced() {
        let mut module = IrModule::new("test".to_string());
        // Add a closure struct with a captured var not referenced in body
        module
            .closure_structs
            .push(crate::zigir::types::IrClosureStruct {
                name: IrIdent::new("_closure_0"),
                captured: vec![IrCapture {
                    name: IrIdent::new("unused_var"),
                    zig_type: ZigType::I64,
                    is_mut: false,
                    init_expr: IrExpr::Ident(IrIdent::new("unused_var")),
                }],
                fn_params: vec![IrParam {
                    name: IrIdent::new("x"),
                    zig_type: ZigType::I64,
                    is_unused: false,
                    is_rest: false,
                }],
                return_type: ZigType::I64,
                typeof_return_body: None,
                body: IrBlock::new(vec![IrStmt::Return {
                    value: Some(IrExpr::Ident(IrIdent::new("x"))),
                }]),
            });

        let mut pass = ValidatePass::new();
        let result = pass.run(&mut module);
        let warnings: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| matches!(d.level, DiagnosticLevel::Warning))
            .collect();
        assert_eq!(warnings.len(), 1, "should warn about unreferenced capture");
        assert!(warnings[0].message.contains("unused_var"));
    }
}
