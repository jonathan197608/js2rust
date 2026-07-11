// zigir/passes/mod.rs
// Optimization and validation passes for the ZigIR pipeline.
//
// Each pass implements the `IrPass` trait and can be composed into a
// `PassPipeline` that runs them in sequence on an `IrModule`.

mod collect_idents;
mod constant_fold;
mod dead_code;
mod validate;
pub(crate) mod walk;

pub use constant_fold::ConstantFoldPass;
pub use dead_code::DeadCodeElimPass;
pub use validate::ValidatePass;

use crate::zigir::source_span::IrDiagnostic;
use crate::zigir::types::IrModule;

// ═══════════════════════════════════════════════════════
//  IrPass trait
// ═══════════════════════════════════════════════════════

/// Result of running a single pass on an IrModule.
#[derive(Debug, Clone)]
pub struct PassResult {
    /// Whether the pass modified the IR.
    pub changed: bool,
    /// Diagnostics produced by this pass (warnings/errors).
    pub diagnostics: Vec<IrDiagnostic>,
}

impl PassResult {
    /// Create an unchanged result with no diagnostics.
    pub fn unchanged() -> Self {
        Self {
            changed: false,
            diagnostics: Vec::new(),
        }
    }

    /// Create a changed result with no diagnostics.
    pub fn changed() -> Self {
        Self {
            changed: true,
            diagnostics: Vec::new(),
        }
    }

    /// Create a result with diagnostics. `changed` is true if any diagnostic
    /// is an error (errors typically indicate the IR was invalid and may
    /// have been repaired), false for pure warnings.
    pub fn with_diagnostics(diagnostics: Vec<IrDiagnostic>) -> Self {
        let changed = diagnostics
            .iter()
            .any(|d| matches!(d.level, crate::zigir::source_span::DiagnosticLevel::Error));
        Self {
            changed,
            diagnostics,
        }
    }

    /// Merge another result into this one.
    pub fn merge(&mut self, other: PassResult) {
        self.changed = self.changed || other.changed;
        self.diagnostics.extend(other.diagnostics);
    }
}

/// A transformation or validation pass over the IR.
///
/// Passes should be idempotent where possible: running the same pass
/// twice on the same module should produce the same result, and the
/// second run should return `changed: false`.
pub trait IrPass {
    /// Human-readable name of the pass (for logging/diagnostics).
    fn name(&self) -> &'static str;

    /// Short description of what the pass does.
    fn description(&self) -> &'static str;

    /// Run the pass on the module, potentially mutating it in place.
    ///
    /// Returns a `PassResult` indicating whether the IR was modified
    /// and any diagnostics produced.
    fn run(&mut self, module: &mut IrModule) -> PassResult;
}

// ═══════════════════════════════════════════════════════
//  PassPipeline
// ═══════════════════════════════════════════════════════

/// A sequence of IR passes that are run in order on a module.
///
/// The pipeline stops early if a pass produces error-level diagnostics
/// (unless `continue_on_error` is set).
pub struct PassPipeline {
    passes: Vec<Box<dyn IrPass>>,
    /// Whether to continue running passes after one produces errors.
    pub continue_on_error: bool,
}

/// Summary of a pipeline run.
#[derive(Debug, Clone)]
pub struct PipelineResult {
    /// Total number of passes that were run (may be less than total
    /// if stopped early due to errors).
    pub passes_run: usize,
    /// Whether any pass modified the IR.
    pub changed: bool,
    /// All diagnostics from all passes, in order.
    pub diagnostics: Vec<IrDiagnostic>,
    /// Names of passes that reported changes, in order.
    pub changed_passes: Vec<&'static str>,
}

impl PassPipeline {
    /// Create an empty pipeline.
    pub fn new() -> Self {
        Self {
            passes: Vec::new(),
            continue_on_error: false,
        }
    }

    /// Create the default optimization pipeline.
    ///
    /// Order: Validate → DeadCodeElim → ConstantFold → Validate
    /// (validate again after transforms to check invariants hold).
    pub fn default_pipeline() -> Self {
        let mut pipeline = Self::new();
        pipeline.add_pass(ValidatePass::new());
        pipeline.add_pass(DeadCodeElimPass::new());
        pipeline.add_pass(ConstantFoldPass::new());
        pipeline.add_pass(ValidatePass::new());
        pipeline
    }

    /// Add a pass to the end of the pipeline.
    pub fn add_pass(&mut self, pass: impl IrPass + 'static) {
        self.passes.push(Box::new(pass));
    }

    /// Run all passes in sequence on the module.
    pub fn run(&mut self, module: &mut IrModule) -> PipelineResult {
        let mut result = PipelineResult {
            passes_run: 0,
            changed: false,
            diagnostics: Vec::new(),
            changed_passes: Vec::new(),
        };

        for pass in &mut self.passes {
            let pass_result = pass.run(module);
            result.passes_run += 1;

            let had_errors = pass_result
                .diagnostics
                .iter()
                .any(|d| matches!(d.level, crate::zigir::source_span::DiagnosticLevel::Error));

            result.diagnostics.extend(pass_result.diagnostics);

            if pass_result.changed {
                result.changed = true;
                result.changed_passes.push(pass.name());
            }

            if had_errors && !self.continue_on_error {
                break;
            }
        }

        result
    }
}

impl Default for PassPipeline {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════
//  Shared test helpers (available to all pass submodules)
// ═══════════════════════════════════════════════════════

#[cfg(test)]
pub(crate) fn make_clean_add_module() -> IrModule {
    use crate::types::ZigType;
    use crate::zigir::ident::IrIdent;
    use crate::zigir::ops::BinOp;
    use crate::zigir::types::*;

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
    module
}

// ═══════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ZigType;
    use crate::zigir::types::{IrDecl, IrVarDecl};

    /// A no-op pass for testing the pipeline infrastructure.
    struct NoOpPass;

    impl IrPass for NoOpPass {
        fn name(&self) -> &'static str {
            "no-op"
        }
        fn description(&self) -> &'static str {
            "does nothing"
        }
        fn run(&mut self, _module: &mut IrModule) -> PassResult {
            PassResult::unchanged()
        }
    }

    /// A pass that adds a diagnostic for testing.
    struct WarnPass;

    impl IrPass for WarnPass {
        fn name(&self) -> &'static str {
            "warn"
        }
        fn description(&self) -> &'static str {
            "emits a warning"
        }
        fn run(&mut self, _module: &mut IrModule) -> PassResult {
            PassResult::with_diagnostics(vec![IrDiagnostic::warning("test warning".to_string())])
        }
    }

    /// A pass that adds a variable declaration for testing change detection.
    struct AddVarPass;

    impl IrPass for AddVarPass {
        fn name(&self) -> &'static str {
            "add-var"
        }
        fn description(&self) -> &'static str {
            "adds a variable"
        }
        fn run(&mut self, module: &mut IrModule) -> PassResult {
            module.declarations.push(IrDecl::Var(IrVarDecl::new_const(
                "__test_added",
                Some(ZigType::I64),
                Some(crate::zigir::types::IrExpr::IntLiteral(0)),
            )));
            PassResult::changed()
        }
    }

    #[test]
    fn test_empty_pipeline() {
        let mut pipeline = PassPipeline::new();
        let mut module = IrModule::new("test".to_string());
        let result = pipeline.run(&mut module);
        assert_eq!(result.passes_run, 0);
        assert!(!result.changed);
    }

    #[test]
    fn test_noop_pass() {
        let mut pipeline = PassPipeline::new();
        pipeline.add_pass(NoOpPass);
        let mut module = IrModule::new("test".to_string());
        let result = pipeline.run(&mut module);
        assert_eq!(result.passes_run, 1);
        assert!(!result.changed);
        assert!(result.changed_passes.is_empty());
    }

    #[test]
    fn test_warn_pass() {
        let mut pipeline = PassPipeline::new();
        pipeline.add_pass(WarnPass);
        let mut module = IrModule::new("test".to_string());
        let result = pipeline.run(&mut module);
        assert_eq!(result.passes_run, 1);
        assert!(!result.changed); // warnings don't count as "changed"
        assert_eq!(result.diagnostics.len(), 1);
    }

    #[test]
    fn test_change_detection() {
        let mut pipeline = PassPipeline::new();
        pipeline.add_pass(AddVarPass);
        let mut module = IrModule::new("test".to_string());
        let result = pipeline.run(&mut module);
        assert_eq!(result.passes_run, 1);
        assert!(result.changed);
        assert_eq!(result.changed_passes, vec!["add-var"]);
        assert_eq!(module.declarations.len(), 1);
    }

    #[test]
    fn test_multi_pass_pipeline() {
        let mut pipeline = PassPipeline::new();
        pipeline.add_pass(NoOpPass);
        pipeline.add_pass(WarnPass);
        pipeline.add_pass(AddVarPass);
        let mut module = IrModule::new("test".to_string());
        let result = pipeline.run(&mut module);
        assert_eq!(result.passes_run, 3);
        assert!(result.changed);
        assert_eq!(result.changed_passes, vec!["add-var"]);
        assert_eq!(result.diagnostics.len(), 1);
    }

    #[test]
    fn test_default_pipeline_runs() {
        let mut pipeline = PassPipeline::default_pipeline();
        let mut module = IrModule::new("test".to_string());
        let result = pipeline.run(&mut module);
        // 4 passes: Validate → DeadCodeElim → ConstantFold → Validate
        assert_eq!(result.passes_run, 4);
    }

    #[test]
    fn test_pass_result_merge() {
        let mut r1 = PassResult::changed();
        let r2 = PassResult::with_diagnostics(vec![IrDiagnostic::warning("w".to_string())]);
        r1.merge(r2);
        assert!(r1.changed);
        assert_eq!(r1.diagnostics.len(), 1);
    }
}
