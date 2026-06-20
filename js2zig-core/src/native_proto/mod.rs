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

/// Zig type representation for type inference.
/// Only static types are supported; unknown types are compile errors.
#[derive(Debug, Clone, PartialEq)]
pub enum ZigType {
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
            // Otherwise, not compatible.
            _ => false,
        }
    }

    /// Get the Zig type string for code generation.
    pub fn to_zig_type(&self) -> String {
        match self {
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
        }
    }
}

/// JSDoc 解析结果，传递给 Codegen
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

mod codegen;
mod jsdoc;
#[cfg(test)]
mod tests;

/// Transpile a JS string to Zig source (native type system).
/// Returns error if type inference fails (strict static type system).
pub fn transpile_js(js_source: &str) -> Result<String, String> {
    // Pass 0: extract JSDoc annotations
    let (typedefs, type_annotations, return_types, param_types) = jsdoc::extract_all_jsdoc(js_source);
    let jsdoc_data = JSDocData { typedefs, type_annotations, return_types, param_types };

    let alloc = Allocator::default();
    let source_type = SourceType::default();
    let ret = Parser::new(&alloc, js_source, source_type).parse();
    if !ret.errors.is_empty() {
        return Err(format!("Parse errors: {:?}", ret.errors));
    }
    let mut cg = Codegen::new();
    cg.jsdoc_data = Some(jsdoc_data);
    cg.generate(&ret.program);
    if !cg.errors.is_empty() {
        return Err(cg.errors.join("\n"));
    }
    Ok(cg.output)
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
}
