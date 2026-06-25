// js2zig-core/src/native_proto/mod.rs
//
// Native-type system codegen module.
// All Codegen impl methods are in codegen.rs.
//
use std::collections::{HashMap, HashSet};

// Strict static type system:
// - All types must be known at compile time.
// - No dynamic types (JsAny, Map, etc.).
// - ComputedMemberExpression (obj[key]) is a compile error.
// - Array elements must all have the same type.
// - push() must push the same type as the array element type.

/// C ABI export metadata for a single function.
/// Used by pipeline.rs to generate C ABI wrappers.
#[derive(Debug, Clone)]
pub struct NativeCabiExport {
    pub name: String,
    /// (param_name, param_type)
    pub params: Vec<(String, ZigType)>,
    pub ret_type: ZigType,
    /// Whether this is an async export (impl takes io: Io as first param).
    pub is_async: bool,
    /// Whether this function can throw (contains throw/try-catch).
    /// When true, C ABI wrappers generate error propagation (StrRet sign-bit or _err out-param).
    pub can_throw: bool,
    /// For async functions returning a struct: the struct name (e.g., "FetchUserResult").
    /// Set when `ret_type` is `ZigType::NamedStruct(name)`.
    pub ret_struct_name: Option<String>,
    /// For async functions returning a struct: the struct fields as (name, zig_type) pairs.
    /// Used by js2rust-bridge-macro to generate the #[repr(C)] struct.
    pub ret_struct_fields: Option<Vec<(String, String)>>,
}

/// Diagnostic severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticKind {
    Error,
    Warning,
}

/// A single diagnostic message (compile error/warning).
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub kind: DiagnosticKind,
    /// (line, col) in JS source — None for non-location errors.
    pub span: Option<(usize, usize)>,
    pub message: String,
}

/// Result of transpiling a JS file to Zig.
/// Contains the generated Zig source AND metadata needed by the pipeline
/// (exported functions, diagnostics, etc.).
#[derive(Debug)]
pub struct TranspileResult {
    /// Generated Zig source code.
    pub zig_code: String,
    /// Compile errors (type inference failures, etc.).
    pub errors: Vec<String>,
    /// Non-fatal warnings (try-catch limitations, etc.) — do NOT block file generation.
    pub warnings: Vec<String>,
    /// Exported functions: (name, param_types, return_type).
    pub exports: Vec<ExportedFunction>,
    /// Inferred variable types (for cross-file type propagation, future use).
    pub var_types: std::collections::HashMap<String, ZigType>,
    /// C ABI exports metadata (for bridge macro to generate Rust FFI bindings).
    /// Uses `codegen::CabiExport` for compatibility with the pipeline.
    pub cabi_exports: Vec<NativeCabiExport>,
}

/// An exported function from a JS file.
#[derive(Debug, Clone)]
pub struct ExportedFunction {
    pub name: String,
    pub params: Vec<ZigType>,
    pub return_type: ZigType,
    /// Whether this function contains throw/try-catch statements.
    pub can_throw: bool,
}

/// Zig type representation for type inference.
/// Only static types are supported; unknown types are compile errors.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ZigType {
    /// No return value
    Void,
    /// i64
    I64,
    /// f64
    F64,
    /// bool
    Bool,
    /// []const u8
    Str,
    /// std.ArrayList(T) — T must be a static type
    ArrayList(Box<ZigType>),
    /// Anonymous struct: .{ .field1 = T1, .field2 = T2 }
    Struct(Vec<(String, ZigType)>),
    /// Named struct (defined in HostStructDef, e.g. "UserInfo")
    NamedStruct(String),
    /// anytype (for non-export function parameters)
    Anytype,
    /// Dynamic JSON value (JsAny in generated code).
    /// Used for JSON.parse() return type and dynamic property access.
    /// Allows runtime type coercion via asI64(), asF64(), asBool(), asString().
    JsAny,
}

impl ZigType {
    /// Check if this type is compatible with another type for assignment.
    /// Returns true if assignment is allowed.
    pub fn is_compatible_with(&self, other: &ZigType) -> bool {
        match (self, other) {
            // Same type is always compatible.
            (a, b) if a == b => true,
            // I64 can be widened to F64.
            (ZigType::I64, ZigType::F64) => true,
            // F64 cannot be narrowed to I64 (would lose precision).
            (ZigType::F64, ZigType::I64) => false,
            // Void is only compatible with Void.
            (ZigType::Void, ZigType::Void) => true,
            // Otherwise, not compatible.
            _ => false,
        }
    }

    /// Get the Zig type string for code generation.
    /// NOTE: This method does NOT add "host." prefix for NamedStruct.
    /// If the type refers to a host-defined struct, the caller must add "host."
    /// prefix manually (e.g., in codegen when generating non-host module code).
    pub fn to_zig_type(&self) -> String {
        match self {
            ZigType::Void => "void".to_string(),
            ZigType::I64 => "i64".to_string(),
            ZigType::F64 => "f64".to_string(),
            ZigType::Bool => "bool".to_string(),
            ZigType::Str => "[]const u8".to_string(),
            ZigType::ArrayList(inner) => {
                format!("std.ArrayList({})", inner.to_zig_type())
            }
            ZigType::Struct(fields) => {
                // Generate anonymous struct type.
                let mut s = ".{ ".to_string();
                for (i, (name, ty)) in fields.iter().enumerate() {
                    if i > 0 {
                        s.push_str(", ");
                    }
                    s.push_str(&format!(".{} = {}", name, ty.to_zig_type()));
                }
                s.push_str(" }");
                s
            }
            ZigType::NamedStruct(name) => {
                // Do NOT add "host." prefix here.
                // The caller is responsible for adding it if needed.
                name.clone()
            }
            ZigType::Anytype => "anytype".to_string(),
            ZigType::JsAny => "JsAny".to_string(),
        }
    }
    /// Get the Zig type string for C ABI wrapper generation.
    pub fn to_cabi_str(&self) -> String {
        match self {
            ZigType::Void => "void".to_string(),
            ZigType::I64 => "i64".to_string(),
            ZigType::F64 => "f64".to_string(),
            ZigType::Bool => "bool".to_string(),
            ZigType::Str => "StrRet".to_string(), // C ABI: extern struct { ptr, len }
            ZigType::ArrayList(_) => "std.ArrayList".to_string(), // Not directly supported in C ABI
            ZigType::Struct(_) => "struct".to_string(), // Anonymous struct - not directly supported in C ABI
            ZigType::NamedStruct(_) => "struct".to_string(), // Named struct - C ABI name depends on HostStructDef
            ZigType::Anytype => "i64".to_string(), // Default for anytype (not used in C ABI)
            ZigType::JsAny => "JsAny".to_string(), // JsAny is not directly supported in C ABI
        }
    }

    /// Check if this type is a string type (returns StrRet in C ABI).
    pub fn is_string(&self) -> bool {
        matches!(self, ZigType::Str)
    }
    /// Check if this type is void (no return value).
    pub fn is_void(&self) -> bool {
        matches!(self, ZigType::Void)
    }
}

/// Convert HostType to ZigType.
impl From<crate::HostType> for ZigType {
    fn from(t: crate::HostType) -> Self {
        match t {
            crate::HostType::Void => ZigType::Void,
            crate::HostType::Bool => ZigType::Bool,
            crate::HostType::I32 => ZigType::I64, // i32 widens to i64
            crate::HostType::I64 => ZigType::I64,
            crate::HostType::F64 => ZigType::F64,
            crate::HostType::Str => ZigType::Str,
        }
    }
}

/// JSDoc 解析结果，传递给 Codegen
#[derive(Debug, Clone)]
pub struct JSDocData {
    /// @typedef 定义：类型名 → TypedefDef
    pub typedefs: std::collections::HashMap<String, jsdoc::TypedefDef>,
    /// @type 注解：变量名 → 类型名
    pub type_annotations: std::collections::HashMap<String, String>,
    /// @returns 注解：函数名 → 类型名
    pub return_types: std::collections::HashMap<String, String>,
    /// @param 注解：函数名 → [(参数名, 类型名)]
    pub param_types: std::collections::HashMap<String, Vec<(String, String)>>,
}

use oxc_ast::ast::Program;

mod builtins;
mod codegen;
mod infer;
mod jsdoc;
#[cfg(test)]
mod tests;

pub use infer::TypeCheckResult;

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
/// Two-pass flow (Phase A):
///   1. TypeInferrer::infer_all() — walk AST once, collect all type info
///   2. Codegen::generate() — read pre-computed type info, emit Zig code
fn transpile_js_inner(
    program: &Program<'_>,
    js_source: &str,
    exported_functions: Option<std::collections::HashSet<String>>,
    host_fns: Option<&crate::host::HostFnRegistry>,
) -> Result<TranspileResult, String> {
    // JSDoc extraction (still needs raw source text)
    let (typedefs, type_annotations, return_types, param_types) =
        jsdoc::extract_all_jsdoc(js_source);
    let jsdoc_data = JSDocData {
        typedefs,
        type_annotations,
        return_types,
        param_types,
    };

    // ── Pass 1: Type inference ──
    let mut inferrer = infer::TypeInferrer::new();
    inferrer.set_jsdoc_data(jsdoc_data.clone());
    if let Some(hf) = host_fns {
        inferrer.set_host_fn_types(hf);
    }
    let type_info = inferrer.infer_all(program, exported_functions.clone());

    // Extract TypeInferrer errors before type_info is moved to Codegen.
    let infer_errors = type_info.errors.clone();

    // ── Pass 2: Code generation ──
    // Extract async host function names for io.async() codegen.
    let async_host_fns: std::collections::HashSet<String> = if let Some(hf) = host_fns {
        hf.async_fn_names().into_iter().collect()
    } else {
        std::collections::HashSet::new()
    };
    let mut cg = Codegen::new(
        type_info,
        jsdoc_data,
        exported_functions,
        async_host_fns,
        js_source.to_string(),
    );
    cg.generate(program);

    // Merge TypeInferrer errors with Codegen errors.
    let mut combined_errors = infer_errors;
    combined_errors.append(&mut cg.errors.clone());
    let warnings = cg.warnings.clone();

    Ok(TranspileResult {
        zig_code: cg.output,
        errors: combined_errors,
        warnings,
        exports: cg.exported_fns.clone(),
        var_types: cg.type_info.var_types.clone(),
        cabi_exports: cg
            .exported_fns
            .into_iter()
            .map(|ef| {
                let params: Vec<(String, ZigType)> = ef
                    .params
                    .iter()
                    .enumerate()
                    .map(|(i, p)| (format!("arg{}", i), p.clone()))
                    .collect();
                let is_async = cg
                    .type_info
                    .is_async
                    .get(&ef.name)
                    .copied()
                    .unwrap_or(false);
                // Extract struct name if return type is NamedStruct
                let ret_struct_name =
                    if let crate::native_proto::ZigType::NamedStruct(ref s) = ef.return_type {
                        Some(s.clone())
                    } else {
                        None
                    };
                NativeCabiExport {
                    name: ef.name,
                    params,
                    ret_type: ef.return_type,
                    is_async,
                    can_throw: ef.can_throw,
                    ret_struct_name,
                    ret_struct_fields: None, // populated from host_fns in pipeline.rs
                }
            })
            .collect(),
    })
}

/// Metadata about a class declaration, used to generate struct definitions
/// and route `new ClassName()` expressions to `ClassName.init()`.
#[derive(Debug, Clone)]
pub struct ClassInfo {
    /// Class name
    pub name: String,
    /// Field names (from property definitions or constructor assignments)
    pub fields: Vec<String>,
    /// Field types (defaults to i64 if not JSDoc-annotated)
    pub field_types: Vec<ZigType>,
    /// Static field names
    pub static_fields: Vec<String>,
    /// Whether the class has a constructor
    pub has_constructor: bool,
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
    /// C ABI exports metadata (for pipeline C ABI wrapper generation).
    pub cabi_exports: Vec<NativeCabiExport>,
    /// Task counter for generating unique task variable names in async/await code.
    pub task_counter: u32,
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
    /// Counter for generating unique try-block labels (for nested try-catch).
    pub try_label_counter: u32,
    /// Counter for generating unique arrow function names.
    pub arrow_counter: u32,
    /// Pending arrow function declarations to be emitted at the top level.
    pub pending_arrow_fns: Vec<String>,
    /// When inside a try block, the label name for `break :label`.
    /// throw statements inside the try block emit `break :label error.JsThrow`
    /// instead of `return error.JsThrow`.
    pub inside_try_block: Option<String>,
    /// Current function name being generated (for function-scoped mutated_vars).
    pub current_fn: Option<String>,
    /// Captured variables for the current arrow function: (name, type, is_mutable)
    pub current_captured: Vec<(String, ZigType, bool)>,
    /// Map: closure variable name → list of (captured_name, type, is_mutable)
    /// Used to generate struct instance at assignment and .call() at call site.
    pub closure_vars: HashMap<String, Vec<(String, ZigType, bool)>>,
    /// Set of variable names that are closure instances (assigned from arrow functions with captures).
    /// Used to rewrite `fn(args)` to `fn.call(args)` in emit_call.
    pub closure_instances: HashSet<String>,
    /// Closure struct definitions to be prepended to output (module-level).
    pub closure_defs: Vec<String>,
    /// Counter for generating unique temp variable names in optional chaining (?.)
    pub oc_counter: u32,
    /// Counter for generating unique temp variable names in destructuring patterns.
    pub destructure_counter: u32,
    /// Variables initialized with TypedArray constructors (Int32Array, Uint8Array, Float64Array).
    /// Maps variable name → element Zig type suffix (e.g. "I32", "U8", "F64").
    /// Used to route method calls and property accesses correctly.
    pub typedarray_vars: std::collections::HashMap<String, String>,
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
    /// Class definitions collected during codegen: class_name → ClassInfo.
    pub class_defs: std::collections::HashMap<String, ClassInfo>,
    /// When inside a class method body, this holds the class name.
    /// Used to rewrite `this.x` → `self.x`.
    pub current_class: Option<String>,
    /// Set of class names known at the module level.
    /// Used to route `new ClassName()` → `ClassName.init()` in emit_expr.
    pub class_names: std::collections::HashSet<String>,
    /// Original JS source text, used to convert byte offsets → line:col for diagnostics.
    pub source: String,
}
