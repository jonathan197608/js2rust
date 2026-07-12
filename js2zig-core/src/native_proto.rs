// js2zig-core/src/native_proto.rs
//
// Native-type system transpilation module.
// Pipeline: AST → Lowerer → ZigIR → PassPipeline → Emitter → Zig source.

// Re-export types from the dedicated types module.
pub use crate::types::{
    ClosureManager, JSDocData, NameGen, NativeCabiExport, TranspileResult, ZigType,
};

use oxc_ast::ast::Program;

use crate::zigir::types::{IrDecl, IrExpr, IrStmt};

pub use crate::infer::TypeCheckResult;

/// Transpile JS source text to Zig (native type system).
///
/// Accepts a pre-parsed `&Program` plus the original source text
/// (needed for JSDoc extraction).  The caller should obtain the `Program` from
/// `analyze_single_group` so that the AST is only built once.
///
/// Returns full `TranspileResult` with generated code AND metadata
/// (diagnostics, cabi_exports, etc.).
///
/// `exported_functions`: Optional set of exported function names.
/// If provided, only functions in this set generate `pub fn` (export semantics).
/// If None, treat all toplevel functions as exports (backward compatibility).
///
/// Production pipeline (ZigIR):
///   1. TypeInferrer::infer_all() — walk AST, collect type info
///   2. Lowerer::lower() — convert AST to ZigIR (IrModule)
///   3. PassPipeline — optimize IrModule (dead code, constant fold, validate)
///   4. Emitter::emit_module() — emit Zig source from IrModule
pub fn transpile_js(
    program: &Program<'_>,
    js_source: &str,
    exported_functions: Option<std::collections::HashSet<String>>,
    host_fns: Option<&crate::host::HostFnRegistry>,
    module_name: &str,
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
        exported_functions,
        async_host_fns,
        js_source.to_string(),
        module_name.to_string(),
    );
    let mut ir_module = lowerer.lower(program);

    // ── Step 3: Optimization passes ──
    let mut pipeline = crate::zigir::passes::PassPipeline::default_pipeline();
    let _pipeline_result = pipeline.run(&mut ir_module);

    // ── Step 3.5: Collect @compileError messages from IR ──
    let compile_errors = collect_compile_errors(&ir_module);

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
        compile_errors,
        var_types,
        cabi_exports,
    })
}

/// Collect all `@compileError` messages from the IR tree.
///
/// Walks the entire `IrModule` — declarations, closure structs, typedefs —
/// and extracts messages from `IrDecl::CompileError`, `IrStmt::CompileError`,
/// and `IrExpr::CompileError` nodes.
///
/// These correspond to JS features that the transpiler does not support.
/// Zig's lazy analysis may never trigger the generated `@compileError`,
/// so we surface them at transpile time as non-blocking warnings.
fn collect_compile_errors(module: &crate::zigir::types::IrModule) -> Vec<String> {
    use crate::zigir::passes::walk;
    use std::cell::RefCell;

    let results = RefCell::new(Vec::new());

    fn push_msg(msg: &str, results: &RefCell<Vec<String>>) {
        // Split merged messages (joined with "\n\nAlso: ") into individual entries
        for part in msg.split("\n\nAlso: ") {
            results.borrow_mut().push(part.to_string());
        }
    }

    fn collect_from_block(block: &crate::zigir::types::IrBlock, results: &RefCell<Vec<String>>) {
        for stmt in &block.stmts {
            collect_from_stmt(stmt, results);
        }
    }

    fn collect_from_stmt(stmt: &IrStmt, results: &RefCell<Vec<String>>) {
        match stmt {
            IrStmt::CompileError { msg, .. } => push_msg(msg, results),
            _ => walk::for_each_stmt_child(
                stmt,
                &mut |block| collect_from_block(block, results),
                &mut |s| collect_from_stmt(s, results),
                &mut |expr| collect_from_expr(expr, results),
                &mut |_| {},
            ),
        }
    }

    fn collect_from_expr(expr: &IrExpr, results: &RefCell<Vec<String>>) {
        match expr {
            IrExpr::CompileError { msg, .. } => push_msg(msg, results),
            _ => walk::for_each_expr_child(
                expr,
                &mut |block| collect_from_block(block, results),
                &mut |s| collect_from_stmt(s, results),
                &mut |expr| collect_from_expr(expr, results),
                &mut |_| {},
            ),
        }
    }

    // Top-level declarations
    for decl in &module.declarations {
        match decl {
            IrDecl::CompileError { msg, .. } => push_msg(msg, &results),
            _ => walk::for_each_decl_child(
                decl,
                &mut |block| collect_from_block(block, &results),
                &mut |expr| collect_from_expr(expr, &results),
            ),
        }
    }

    // Closure structs (their body may contain CompileError)
    for cs in &module.closure_structs {
        collect_from_block(&cs.body, &results);
    }

    results.into_inner()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_compile_errors_from_ir() {
        use crate::types::ZigType;
        use crate::zigir::ident::IrIdent;
        use crate::zigir::source_span::SourceSpan;
        use crate::zigir::types::{IrBlock, IrDecl, IrFnDecl, IrModule, IrStmt};

        let mut module = IrModule::new("test".to_string());

        // Add a CompileError declaration
        module.declarations.push(IrDecl::CompileError {
            span: SourceSpan::new(1, 1),
            msg: "unsupported import".to_string(),
        });

        // Add a function with a CompileError statement
        module.declarations.push(IrDecl::Fn(IrFnDecl {
            name: IrIdent::new("testFn"),
            params: vec![],
            return_type: ZigType::Void,
            body: IrBlock::new(vec![IrStmt::CompileError {
                span: SourceSpan::new(5, 10),
                msg: "nested class not supported".to_string(),
            }]),
            is_export: false,
            is_async: false,
            can_throw: false,
            is_cabi: false,
            typeof_return_body: None,
        }));

        let results = collect_compile_errors(&module);
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|m| m.contains("unsupported import")));
        assert!(
            results
                .iter()
                .any(|m| m.contains("nested class not supported"))
        );
    }

    #[test]
    fn test_collect_compile_errors_splits_merged() {
        use crate::types::ZigType;
        use crate::zigir::ident::IrIdent;
        use crate::zigir::source_span::SourceSpan;
        use crate::zigir::types::{IrBlock, IrDecl, IrFnDecl, IrModule, IrStmt};

        let mut module = IrModule::new("test".to_string());

        // Simulate a merged CompileError (from Scheme B)
        module.declarations.push(IrDecl::Fn(IrFnDecl {
            name: IrIdent::new("testFn"),
            params: vec![],
            return_type: ZigType::Void,
            body: IrBlock::new(vec![IrStmt::CompileError {
                span: SourceSpan::new(1, 1),
                msg: "error 1\n\nAlso: error 2\n\nAlso: error 3".to_string(),
            }]),
            is_export: false,
            is_async: false,
            can_throw: false,
            is_cabi: false,
            typeof_return_body: None,
        }));

        let results = collect_compile_errors(&module);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0], "error 1");
        assert_eq!(results[1], "error 2");
        assert_eq!(results[2], "error 3");
    }
}
