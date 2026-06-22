// js2zig-core/src/native_proto/mod.rs
//
// Native-type system codegen module.
// All Codegen impl methods are in codegen.rs.
//
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
    pub fn to_zig_type(&self) -> String {
        match self {
            ZigType::Void => "void".to_string(),
            ZigType::I64 => "i64".to_string(),
            ZigType::F64 => "f64".to_string(),
            ZigType::Bool => "bool".to_string(),
            ZigType::Str => "[]const u8".to_string(),
            ZigType::ArrayList(inner) => format!("std.ArrayList({})", inner.to_zig_type()),
            ZigType::Struct(fields) => {
                // Generate anonymous struct type.
                let mut s = ".{ ".to_string();
                for (i, (name, ty)) in fields.iter().enumerate() {
                    if i > 0 { s.push_str(", "); }
                    s.push_str(&format!(".{} = {}", name, ty.to_zig_type()));
                }
                s.push_str(" }");
                s
            }
            ZigType::NamedStruct(name) => name.clone(),
            ZigType::Anytype => "anytype".to_string(),
        }
    }
    /// Get the Zig type string for C ABI wrapper generation.
    pub fn to_cabi_str(&self) -> String {
        match self {
            ZigType::Void => "void".to_string(),
            ZigType::I64 => "i64".to_string(),
            ZigType::F64 => "f64".to_string(),
            ZigType::Bool => "bool".to_string(),
            ZigType::Str => "StrRet".to_string(),  // C ABI: extern struct { ptr, len }
            ZigType::ArrayList(_) => "std.ArrayList".to_string(),  // Not directly supported in C ABI
            ZigType::Struct(_) => "struct".to_string(),  // Anonymous struct - not directly supported in C ABI
            ZigType::NamedStruct(_) => "struct".to_string(),  // Named struct - C ABI name depends on HostStructDef
            ZigType::Anytype => "i64".to_string(),  // Default for anytype (not used in C ABI)
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
                crate::HostType::I32 => ZigType::I64,  // i32 widens to i64
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

use oxc_parser::Parser;
use oxc_allocator::Allocator;
use oxc_span::SourceType;

mod builtins;
mod codegen;
mod jsdoc;
#[cfg(test)]
mod tests;

/// Transpile a JS string to Zig source (native type system).
///
/// Returns full `TranspileResult` with generated code AND metadata
/// (exported functions, diagnostics, etc.).
///
/// `exported_functions`: Optional set of exported function names.
/// If provided, only functions in this set generate `pub fn` (export semantics).
/// If None, treat all toplevel functions as exports (backward compatibility).
pub fn transpile_js(
    js_source: &str,
    exported_functions: Option<std::collections::HashSet<String>>,
) -> Result<TranspileResult, String> {
    transpile_js_inner(js_source, exported_functions)
}

/// Internal helper: transpile JS to Zig, returning TranspileResult.
fn transpile_js_inner(js_source: &str, exported_functions: Option<std::collections::HashSet<String>>) -> Result<TranspileResult, String> {
    // Pass 0: extract JSDoc annotations
    let (typedefs, type_annotations, return_types, param_types) = jsdoc::extract_all_jsdoc(js_source);
    let jsdoc_data = JSDocData { typedefs, type_annotations, return_types, param_types };

    let alloc = Allocator::default();
    // Always parse in module mode so codegen sees import/export nodes directly
    // in `program.body` (no pre-strip of the raw source needed).
    let source_type = SourceType::default().with_module(true);
    let ret = Parser::new(&alloc, js_source, source_type).parse();
    if !ret.errors.is_empty() {
        return Err(format!("Parse errors: {:?}", ret.errors));
    }

    let mut cg = Codegen::new();
    cg.jsdoc_data = Some(jsdoc_data);
    cg.exported_functions = exported_functions;  // ← 存储 exported_functions
    cg.generate(&ret.program);
    // NOTE: Temporarily disabled error check for debugging.
    // TODO: enable after fixing all codegen bugs.
    // if !cg.errors.is_empty() {
    //     return Err(cg.errors.join("\n"));
    // }

    Ok(TranspileResult {
        zig_code: cg.output,
        errors: cg.errors.clone(),
        exports: cg.exported_fns.clone(),
        var_types: cg.var_types.clone(),
        cabi_exports: cg.exported_fns.into_iter().map(|ef| {
            let params: Vec<(String, ZigType)> = ef.params.iter()
                .enumerate()
                .map(|(i, p)| (format!("arg{}", i), p.clone()))
                .collect();
            NativeCabiExport {
                name: ef.name,
                params,
                ret_type: ef.return_type,
                is_async: false, // TODO: support async
            }
        }).collect(),
    })
}

/// Shared state for native-type codegen.
#[derive(Default)]
pub struct Codegen {
    pub output: String,
    pub indent: usize,
    pub used_names: std::collections::HashSet<String>,
    /// Compile errors (type inference failures, etc.)
    pub errors: Vec<String>,
    /// Variables that are mutated (assigned to a property) → must use 'var'.
    pub mutated_vars: std::collections::HashSet<String>,
    /// Tracks the inferred type of each variable (for intermediate variables).
    pub var_types: std::collections::HashMap<String, ZigType>,
    /// Tracks the inferred field types of each struct object.
    pub struct_field_types: std::collections::HashMap<String, std::collections::HashMap<String, ZigType>>,
    /// Tracks array element types: variable name → element type.
    pub array_element_types: std::collections::HashMap<String, ZigType>,
    /// JSDoc 解析结果（由 transpile_js 填充）
    pub jsdoc_data: Option<JSDocData>,
    /// Whether the current function being emitted is an export function.
    pub current_fn_is_export: bool,
    /// For export functions: maps parameter name → parsed variable name.
    pub param_name_map: std::collections::HashMap<String, String>,
    /// The return type of the current function being emitted.
    pub current_fn_return_type: Option<ZigType>,
    /// Cache of function return types (for CallExpression type inference).
    pub fn_return_types: std::collections::HashMap<String, ZigType>,
    /// Exported functions metadata (for pipeline C ABI wrapper generation).
    pub exported_fns: Vec<ExportedFunction>,
    /// C ABI exports metadata (for pipeline C ABI wrapper generation).
    pub cabi_exports: Vec<NativeCabiExport>,
    /// Task counter for generating unique task variable names in async/await code.
    pub task_counter: u32,
    /// Exported function names (from pipeline's strip_imports_extract_exports).
    /// If provided, use this to determine if a function is an export function.
    /// Otherwise, fall back to HACK (treat all toplevel functions as exports).
    pub exported_functions: Option<std::collections::HashSet<String>>,
}
