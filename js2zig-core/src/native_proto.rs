// js2zig-core/src/native_proto.rs
//
// Native-type system transpilation module.
// Pipeline: AST → Lowerer → ZigIR → PassPipeline → Emitter → Zig source.

// Re-export types from the dedicated types module.
pub use crate::types::{
    ClosureManager, Diagnostic, DiagnosticKind, ExportedFunction, JSDocData, NameGen,
    NativeCabiExport, TranspileResult, ZigType,
};

use oxc_ast::ast::Program;

use crate::zigir::types::IrDecl;

pub use crate::infer::TypeCheckResult;

/// Transpile JS source text to Zig (native type system).
///
/// **New API** — accepts a pre-parsed `&Program` plus the original source text
/// (needed for JSDoc extraction).  The caller should obtain the `Program` from
/// `analyze_single_group` so that the AST is only built once.
///
/// Returns full `TranspileResult` with generated code AND metadata
/// (exported functions, diagnostics, etc.).
///
/// `exported_functions`: Optional set of exported function names.
/// If provided, only functions in this set generate `pub fn` (export semantics).
/// If None, treat all toplevel functions as exports (backward compatibility).
pub fn transpile_js(
    program: &Program<'_>,
    js_source: &str,
    exported_functions: Option<std::collections::HashSet<String>>,
    host_fns: Option<&crate::host::HostFnRegistry>,
) -> Result<TranspileResult, String> {
    transpile_js_inner(program, js_source, exported_functions, host_fns)
}

/// Internal helper: transpile JS AST to Zig, returning TranspileResult.
///
/// Production pipeline (ZigIR):
///   1. TypeInferrer::infer_all() — walk AST, collect type info
///   2. Lowerer::lower() — convert AST to ZigIR (IrModule)
///   3. PassPipeline — optimize IrModule (dead code, constant fold, validate)
///   4. Emitter::emit_module() — emit Zig source from IrModule
fn transpile_js_inner(
    program: &Program<'_>,
    js_source: &str,
    exported_functions: Option<std::collections::HashSet<String>>,
    host_fns: Option<&crate::host::HostFnRegistry>,
) -> Result<TranspileResult, String> {
    // JSDoc extraction (still needs raw source text)
    let (typedefs, type_annotations, return_types, param_types) =
        crate::jsdoc::extract_all_jsdoc(js_source);
    let jsdoc_data = JSDocData {
        typedefs,
        type_annotations,
        return_types,
        param_types,
    };

    // ── Step 1: Type inference ──
    let mut inferrer = crate::infer::TypeInferrer::new();
    inferrer.set_jsdoc_data(jsdoc_data.clone());
    if let Some(hf) = host_fns {
        inferrer.set_host_fn_types(hf);
    }
    let type_info = inferrer.infer_all(program, exported_functions.clone());

    let infer_errors = type_info.errors.clone();
    let var_types = type_info.var_types.clone();

    // Extract async host function names for io.async() emission.
    let async_host_fns: std::collections::HashSet<String> = if let Some(hf) = host_fns {
        hf.async_fn_names().into_iter().collect()
    } else {
        std::collections::HashSet::new()
    };

    // ── Step 2: Lower AST → ZigIR ──
    use crate::zigir::lower::Lowerer;
    let mut lowerer = Lowerer::new(
        type_info,
        jsdoc_data,
        exported_functions.clone(),
        async_host_fns.clone(),
        js_source.to_string(),
    );
    let mut ir_module = lowerer.lower(program);

    // ── Step 3: Optimization passes ──
    let mut pipeline = crate::zigir::passes::PassPipeline::default_pipeline();
    let _pipeline_result = pipeline.run(&mut ir_module);

    // ── Step 4: Emit Zig source ──
    use crate::zigir::emit::Emitter;
    let zig_code = Emitter::emit_module(&ir_module);

    // ── Extract TranspileResult fields from IrModule ──

    // Errors / warnings from Lowerer diagnostics
    let mut errors: Vec<String> = infer_errors;
    let mut warnings: Vec<String> = Vec::new();
    for diag in &ir_module.diagnostics {
        match diag.level {
            crate::zigir::source_span::DiagnosticLevel::Error => {
                errors.push(diag.message.clone());
            }
            crate::zigir::source_span::DiagnosticLevel::Warning => {
                warnings.push(diag.message.clone());
            }
        }
    }

    // Exported functions from IrDecl::Fn where is_export
    let exports: Vec<ExportedFunction> = ir_module
        .declarations
        .iter()
        .filter_map(|decl| {
            if let IrDecl::Fn(f) = decl
                && f.is_export
            {
                Some(ExportedFunction {
                    name: f.name.zig_name.clone(),
                    params: f.params.iter().map(|p| p.zig_type.clone()).collect(),
                    return_type: f.return_type.clone(),
                    can_throw: f.can_throw,
                })
            } else {
                None
            }
        })
        .collect();

    // C ABI exports from IrModule
    let cabi_exports: Vec<NativeCabiExport> = ir_module
        .cabi_exports
        .iter()
        .map(|ce| NativeCabiExport {
            name: ce.name.clone(),
            params: ce
                .params
                .iter()
                .enumerate()
                .map(|(i, p)| (format!("arg{}", i), p.zig_type.clone()))
                .collect(),
            ret_type: ce.return_type.clone(),
            is_async: ce.is_async,
            can_throw: ce.can_throw,
            ret_struct_name: ce.ret_struct_name.clone(),
            ret_struct_fields: None, // populated from host_fns in pipeline.rs
        })
        .collect();

    Ok(TranspileResult {
        zig_code,
        errors,
        warnings,
        exports,
        var_types,
        cabi_exports,
    })
}
