// js2zig-core/src/native_proto.rs
//
// Native-type system transpilation module.
// Production pipeline: AST → Lowerer → ZigIR → PassPipeline → Emitter → Zig source.
// Legacy Codegen is available via ZIGIR_DUAL_TRACK=1 for parity comparison.

// Re-export types from the dedicated types module.
pub use crate::types::{
    ClosureManager, Diagnostic, DiagnosticKind, ExportedFunction, JSDocData, NameGen,
    NativeCabiExport, TranspileResult, ZigType,
};

use oxc_ast::ast::Program;

use crate::zigir::types::IrDecl;

pub use crate::infer::TypeCheckResult;

// ── ZigIR dual-track flag ─────────────────────────────
// When true, the old Codegen is also invoked after the ZigIR pipeline
// and the two outputs are compared for parity checking.
// The ZigIR (Lowerer+Emitter) output is always used as the result.
//
// Enable with: set ZIGIR_DUAL_TRACK=1
const ZIGIR_DUAL_TRACK_DEFAULT: bool = false;

fn zigir_dual_track_enabled() -> bool {
    if ZIGIR_DUAL_TRACK_DEFAULT {
        return true;
    }
    // Check environment variable at runtime
    std::env::var("ZIGIR_DUAL_TRACK")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

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

    // Extract async host function names for io.async() codegen.
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

    // ── ZigIR dual-track: optionally run Codegen and compare ──
    if zigir_dual_track_enabled() {
        // Re-run type inference for Codegen (Lowerer consumed the original type_info)
        let (td, ta, rt, pt) = crate::jsdoc::extract_all_jsdoc(js_source);
        let jd = JSDocData {
            typedefs: td,
            type_annotations: ta,
            return_types: rt,
            param_types: pt,
        };
        let mut inf2 = crate::infer::TypeInferrer::new();
        inf2.set_jsdoc_data(jd.clone());
        if let Some(hf) = host_fns {
            inf2.set_host_fn_types(hf);
        }
        let ti2 = inf2.infer_all(program, exported_functions.clone());

        let mut cg = Codegen::new(
            ti2,
            jd,
            exported_functions,
            async_host_fns,
            js_source.to_string(),
        );
        cg.generate(program);
        run_zigir_dual_track_compare(&ir_module, &cg.output);
    }

    Ok(TranspileResult {
        zig_code,
        errors,
        warnings,
        exports,
        var_types,
        cabi_exports,
    })
}

/// Run the Codegen output and compare with the ZigIR Emitter output.
///
/// When dual-track is enabled, this function emits Zig source from the IrModule
/// via the Emitter and compares it with the old Codegen output, logging summary
/// statistics + diff. This is purely for parity checking — it does NOT affect
/// the transpilation result (Emitter output is always used).
fn run_zigir_dual_track_compare(ir_module: &crate::zigir::types::IrModule, codegen_output: &str) {
    use crate::zigir::emit::Emitter;

    // Log summary for debugging
    let n_decls = ir_module.declarations.len();
    let n_imports = ir_module.imports.len();
    let n_typedefs = ir_module.typedefs.len();
    let n_closures = ir_module.closure_structs.len();
    let n_diagnostics = ir_module.diagnostics.len();
    let n_cabi = ir_module.cabi_exports.len();

    eprintln!(
        "[ZigIR dual-track] module='{}' imports={} typedefs={} closures={} decls={} diagnostics={} cabi={}",
        ir_module.name, n_imports, n_typedefs, n_closures, n_decls, n_diagnostics, n_cabi
    );

    // Log any diagnostics from the Lowerer
    for diag in &ir_module.diagnostics {
        eprintln!("[ZigIR dual-track]   {}", diag);
    }

    // Run the Emitter and compare with the old Codegen output
    let emitter_output = Emitter::emit_module(ir_module);
    let emitter_lines = emitter_output.lines().count();
    let codegen_lines = codegen_output.lines().count();

    if emitter_output == codegen_output {
        eprintln!(
            "[ZigIR dual-track] Emitter output MATCHES Codegen ({} lines)",
            emitter_lines
        );
    } else {
        eprintln!(
            "[ZigIR dual-track] Emitter output DIFFERS from Codegen (emitter={} lines, codegen={} lines)",
            emitter_lines, codegen_lines
        );
        // Log first few differing lines for debugging
        let max_diff = 10;
        let mut diff_count = 0;
        for (i, (e_line, c_line)) in emitter_output
            .lines()
            .zip(codegen_output.lines())
            .enumerate()
        {
            if e_line != c_line {
                if diff_count < max_diff {
                    eprintln!(
                        "[ZigIR dual-track]   line {}: emitter='{}' codegen='{}'",
                        i + 1,
                        if e_line.len() > 80 {
                            &e_line[..80]
                        } else {
                            e_line
                        },
                        if c_line.len() > 80 {
                            &c_line[..80]
                        } else {
                            c_line
                        }
                    );
                }
                diff_count += 1;
            }
        }
        if emitter_lines != codegen_lines {
            eprintln!(
                "[ZigIR dual-track]   line count differs: emitter={} codegen={}",
                emitter_lines, codegen_lines
            );
        }
        eprintln!("[ZigIR dual-track]   total differing lines: {}", diff_count);
    }
}

/// Shared state for native-type codegen.
///
/// Phase A: Codegen is now purely generative — all type inference runs in
/// `TypeInferrer::infer_all()` before codegen.  `type_info` holds the
/// pre-computed type snapshot.
pub struct Codegen {
    pub output: String,
    pub indent: usize,
    /// Compile errors collected during codegen.
    pub errors: Vec<String>,
    /// Non-fatal warnings (try-catch limitations, etc.) — do NOT block file generation.
    pub warnings: Vec<String>,
    /// Pre-computed type information (read-only during codegen).
    pub type_info: TypeCheckResult,
    /// JSDoc data for typedef generation.
    pub jsdoc_data: Option<JSDocData>,
    /// Whether the current function being emitted is an export function.
    pub current_fn_is_export: bool,
    /// The return type of the current function (derived from type_info).
    pub current_fn_return_type: Option<ZigType>,
    /// Exported functions metadata (for pipeline C ABI wrapper generation).
    pub exported_fns: Vec<ExportedFunction>,
    /// Task counter for generating unique task variable names in async/await code.
    /// (Moved into NameGen — keep this doc for context.)
    pub names: crate::types::NameGen,
    /// Exported function names (from pipeline).
    pub exported_functions: Option<std::collections::HashSet<String>>,
    /// Whether a return/throw statement was seen in the current function body.
    pub seen_return: bool,
    /// Whether the current function contains `throw` or `try-catch` statements.
    /// Determined by pre-scan before signature generation. When true, the function
    /// return type is `!T` (error union) instead of plain `T`.
    pub fn_has_throw: bool,
    /// Whether we are currently emitting the return value expression.
    /// When true, array methods that normally discard with `_ = ` should skip the prefix.
    pub in_return_expr: bool,
    /// Whether we are currently emitting the top-level expression of an ExpressionStatement.
    /// When true, builtins that return non-void values should discard with `_ = `.
    pub in_expr_stmt: bool,
    /// Whether the current call expression generated a `catch |_| { ... }` block.
    /// Used to suppress the `_ = ` discard prefix in emit_fn_stmt when a catch
    /// block is already present (Zig 0.16 rejects `_ = <err union> catch |_| { }`).
    pub call_generated_catch: bool,
    /// When inside a try block, the label name for `break :label`.
    /// throw statements inside the try block emit `break :label error.JsThrow`
    /// instead of `return error.JsThrow`.
    pub inside_try_block: Option<String>,
    /// Current function name being generated (for function-scoped mutated_vars).
    pub current_fn: Option<String>,
    /// Closure state: captures, instances, struct definitions.
    pub closures: crate::types::ClosureManager,
    /// Function definitions deferred from expression context (Arrow/FunctionExpression in emit_expr).
    /// These need to be emitted before the current statement at the current indent level.
    pub pending_expr_fns: Vec<String>,
    /// Variables initialized with TypedArray constructors (Int32Array, Uint8Array, Float64Array).
    /// Maps variable name → element Zig type suffix (e.g. "I32", "U8", "F64").
    /// Used to route method calls and property accesses correctly.
    pub typedarray_vars: std::collections::HashMap<String, String>,
    /// Variables initialized with `new RegExp(expr)` — dynamic RegExp objects.
    /// Used to route .test()/.exec() calls on RegExp variables, and
    /// str.match(regexpVar) / str.search(regexpVar) calls.
    pub regexp_vars: std::collections::HashSet<String>,
    /// Async host function names (for io.async() codegen).
    /// When await calls an async host function, use `{name}_async` wrapper.
    pub async_host_fns: std::collections::HashSet<String>,
    /// Names of nested function declarations (inside another function body).
    /// Used to rewrite `nestedFn(args)` to `nestedFn.call(args)` in emit_call.
    pub nested_fn_names: std::collections::HashSet<String>,
    /// When generating a nested function declaration's body via emit_fn(),
    /// this holds the outer JS function name so emit_fn can override the
    /// generated function signature to use `pub fn call(...)` instead of
    /// `pub fn <js_name>(...)`.
    pub current_nested_fn_name: Option<String>,
    /// When inside a class method body, this holds the class name.
    /// Used to rewrite `this.x` → `self.x`.
    pub current_class: Option<String>,
    /// Set of class names known at the module level.
    /// Used to route `new ClassName()` → `ClassName.init()` in emit_expr.
    pub class_names: std::collections::HashSet<String>,
    /// Original JS source text, used to convert byte offsets → line:col for diagnostics.
    pub source: String,
    /// Set of variable names declared in the current function scope.
    /// Used to detect shadowing in nested blocks (Zig 0.16.0 forbids it).
    pub fn_scope_vars: std::collections::HashSet<String>,
    /// Stack of shadowing rename maps: one HashMap per block scope depth.
    /// When a shadowed variable is declared, its original name → renamed name
    /// mapping is stored in the topmost HashMap. `zig_safe_name()` checks this
    /// stack to rewrite references to the renamed variable.
    pub shadow_renames: Vec<std::collections::HashMap<String, String>>,
}
