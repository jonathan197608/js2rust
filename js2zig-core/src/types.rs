// js2zig-core/src/types.rs
// Shared type definitions used across the crate.
// Extracted from native_proto.rs to reduce coupling and improve discoverability.

// ── Zig type system ──────────────────────────────────────

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
    /// Named struct referenced by name. Covers four sources:
    /// - Host-defined structs (via `HostStructDef`, e.g. `"UserInfo"`)
    /// - Built-in runtime types (`"Map"`, `"Set"`, `"Date"`)
    /// - User-defined JS classes (e.g. `"MyClass"`)
    /// - JSDoc `@typedef` types (e.g. `"User"`, `"Point"`)
    NamedStruct(String),
    /// anytype (for non-export function parameters)
    Anytype,
    /// Dynamic JSON value (JsAny in generated code).
    /// Used for JSON.parse() return type and dynamic property access.
    /// Allows runtime type coercion via asI64(), asF64(), asBool(), asString().
    JsAny,
    /// Symbol value (JsSymbol in generated code).
    /// Represents a unique identifier with optional description.
    JsSymbol,
    /// Arbitrary-precision integer (JsBigInt in generated code).
    /// Wraps `std.math.big.int.Managed`.
    BigInt,
    /// JS Error object (js_error.JsError in generated code).
    /// Has `.name`, `.message`, `.stack` fields.
    JsError,
    /// Return type depends on anytype parameters (non-export functions only).
    /// The Emitter emits `@TypeOf(return_expr)` instead of a concrete Zig type.
    /// This allows Zig to infer the return type at compile time from the
    /// actual concrete types of anytype parameters.
    AnytypeReturn,
}

impl ZigType {
    /// Get the Zig type string for code generation.
    /// NOTE: This method does NOT add "host." prefix for NamedStruct.
    /// If the type refers to a host-defined struct, the caller must add "host."
    /// prefix manually (e.g., in the Emitter when generating non-host module code).
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
            ZigType::JsSymbol => "JsSymbol".to_string(),
            ZigType::BigInt => "js_bigint.JsBigInt".to_string(),
            ZigType::JsError => "js_error.JsError".to_string(),
            ZigType::AnytypeReturn => "anytype".to_string(), // placeholder — Emitter replaces with @TypeOf
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
            ZigType::ArrayList(_) => "void".to_string(), // ArrayLists cannot be directly exported via C ABI
            ZigType::Struct(_) => "struct".to_string(), // Anonymous struct - not directly supported in C ABI
            ZigType::NamedStruct(_) => "struct".to_string(), // Named struct - C ABI name depends on HostStructDef
            ZigType::Anytype => "i64".to_string(), // Default for anytype (not used in C ABI)
            ZigType::JsAny => "JsAny".to_string(), // JsAny is not directly supported in C ABI
            ZigType::JsSymbol => "JsSymbol".to_string(), // JsSymbol is not directly supported in C ABI
            ZigType::BigInt => "i64".to_string(),        // C ABI: degrade to i64
            ZigType::JsError => "i64".to_string(),       // C ABI: degrade to i64
            ZigType::AnytypeReturn => "i64".to_string(), // C ABI: shouldn't be used (exports can't be AnytypeReturn)
        }
    }

    /// Map ZigType to JS typeof string.
    /// Returns None for dynamic types (JsAny, Anytype) that need runtime dispatch.
    pub fn to_js_typeof(&self) -> Option<&'static str> {
        match self {
            ZigType::I64 | ZigType::F64 => Some("number"),
            ZigType::Bool => Some("boolean"),
            ZigType::Str => Some("string"),
            ZigType::JsSymbol => Some("symbol"),
            ZigType::Void => Some("undefined"),
            ZigType::Struct(_) | ZigType::NamedStruct(_) | ZigType::ArrayList(_) => Some("object"),
            ZigType::BigInt => Some("bigint"),
            ZigType::JsError => Some("object"),
            // Dynamic types — need runtime typeof helper
            ZigType::JsAny | ZigType::Anytype | ZigType::AnytypeReturn => None,
        }
    }

    /// Parse a Zig type string back into `ZigType`.
    ///
    /// This is the inverse of `to_zig_type()` for primitive and named types.
    /// Complex types (ArrayList, Struct) are not supported by this method;
    /// they should be constructed directly.
    ///
    /// Replaces the previously duplicated `parse_type_str()` (host.rs)
    /// and `zig_str_to_type()` (infer/fn_types.rs).
    pub fn from_zig_str(s: &str) -> ZigType {
        match s {
            "i64" => ZigType::I64,
            "i32" => ZigType::I64, // i32 widens to i64
            "f64" => ZigType::F64,
            "bool" => ZigType::Bool,
            "[]const u8" => ZigType::Str,
            "void" => ZigType::Void,
            "bigint" => ZigType::BigInt,
            "jsany" | "JsAny" => ZigType::JsAny,
            "jssymbol" | "JsSymbol" => ZigType::JsSymbol,
            "jserror" | "JsError" => ZigType::JsError,
            "anytype" => ZigType::Anytype,
            // Host JSON config uses "string" and "any" as type names
            "string" => ZigType::Str,
            "any" | "jsvalue" => ZigType::JsAny,
            // struct: prefix from host JSON config
            other if other.starts_with("struct:") => ZigType::NamedStruct(other[7..].to_string()),
            // Named struct (fallback for types not in typedefs — no spaces or brackets)
            other if !other.contains(' ') && !other.contains('[') => {
                ZigType::NamedStruct(other.to_string())
            }
            _ => ZigType::Anytype, // default fallback for unknown types
        }
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

// ── Export metadata ──────────────────────────────────────

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
}

// ── Transpile result ─────────────────────────────────────

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
    /// @compileError nodes in the generated IR — not blocking, but inform the user
    /// which JS features are unsupported.  Zig's lazy analysis may never trigger
    /// these at compile time, so we surface them at transpile time instead.
    pub compile_errors: Vec<String>,
    /// Inferred variable types (for cross-file type propagation, future use).
    pub var_types: std::collections::HashMap<String, ZigType>,
    /// C ABI exports metadata (for bridge macro to generate Rust FFI bindings).
    /// Uses `NativeCabiExport` for compatibility with the pipeline.
    pub cabi_exports: Vec<NativeCabiExport>,
}

// ── JSDoc data ───────────────────────────────────────────

/// JSDoc 解析结果，传递给 Lowerer
#[derive(Debug, Clone)]
pub struct JSDocData {
    /// @typedef 定义：类型名 → TypedefDef
    pub typedefs: std::collections::HashMap<String, crate::jsdoc::TypedefDef>,
    /// @type 注解：变量名 → 类型名
    pub type_annotations: std::collections::HashMap<String, String>,
    /// @returns 注解：函数名 → 类型名
    pub return_types: std::collections::HashMap<String, String>,
    /// @param 注解：函数名 → [(参数名, 类型名)]
    pub param_types: std::collections::HashMap<String, Vec<(String, String)>>,
}

// ── Name generator ───────────────────────────────────────

/// Unique-name generation counters.
///
/// Each `next_*` method atomically reads and increments its counter,
/// replacing the manual read-then-increment pattern that was previously
/// scattered across `self.<counter>` / `self.<counter> += 1` pairs.
#[derive(Debug)]
pub struct NameGen {
    task_counter: u32,
    try_label_counter: u32,
    arrow_counter: u32,
    oc_counter: u32,
    destructure_counter: u32,
    fn_expr_counter: u32,
    label_counter: u32,
    shadow_counter: u32,
}

impl Default for NameGen {
    fn default() -> Self {
        Self::new()
    }
}

impl NameGen {
    pub fn new() -> Self {
        Self {
            task_counter: 0,
            try_label_counter: 0,
            arrow_counter: 0,
            oc_counter: 0,
            destructure_counter: 0,
            fn_expr_counter: 0,
            label_counter: 0,
            shadow_counter: 0,
        }
    }

    /// Return the current task id and advance the counter.
    /// Produces `_t{N}` or `_arr_lit_{N}` temp variable names.
    pub fn next_task(&mut self) -> u32 {
        let n = self.task_counter;
        self.task_counter += 1;
        n
    }

    /// Return the current try-label id and advance the counter.
    /// Produces `_js_try_blk_{N}` / `_js_try_{N}` names.
    pub fn next_try_label(&mut self) -> u32 {
        let n = self.try_label_counter;
        self.try_label_counter += 1;
        n
    }

    /// Return the current arrow-function id and advance the counter.
    /// Produces `Closure_{N}` struct or `_arrow_fn_{N}` function names.
    pub fn next_arrow(&mut self) -> u32 {
        let n = self.arrow_counter;
        self.arrow_counter += 1;
        n
    }

    /// Return the current optional-chaining id and advance the counter.
    /// Produces `_oc{N}` temp variable names.
    pub fn next_oc(&mut self) -> u32 {
        let n = self.oc_counter;
        self.oc_counter += 1;
        n
    }

    /// Return the current destructuring id and advance the counter.
    /// Produces `_js_dest_{N}` temp variable names.
    pub fn next_destructure(&mut self) -> u32 {
        let n = self.destructure_counter;
        self.destructure_counter += 1;
        n
    }

    /// Return the current function-expression id and advance the counter.
    /// Produces `_fn_expr_{N}` names for anonymous function expressions.
    pub fn next_fn_expr(&mut self) -> u32 {
        let n = self.fn_expr_counter;
        self.fn_expr_counter += 1;
        n
    }

    /// Return a unique block label string `blk_{N}` and advance the counter.
    /// Replaces the manual `let id = self.label_counter; self.label_counter += 1; format!("blk_{}", id)`.
    pub fn next_label(&mut self) -> String {
        let id = self.label_counter;
        self.label_counter += 1;
        format!("blk_{}", id)
    }

    /// Return the id of the most recently generated label (since the last `next_label` call).
    /// Equivalent to the old `self.label_counter - 1` idiom for temp var suffixes.
    pub fn last_label_id(&self) -> u32 {
        self.label_counter - 1
    }

    /// Return the current shadow id and advance the counter.
    /// Produces `{name}_shadow_{N}` variable renames.
    pub fn next_shadow(&mut self) -> u32 {
        let n = self.shadow_counter;
        self.shadow_counter += 1;
        n
    }

    /// Peek at the current label counter value *before* incrementing.
    /// Used when the caller needs to capture the id for later use but
    /// will call `next_label()` separately.
    pub fn peek_label_id(&self) -> u32 {
        self.label_counter
    }
}

// ── Closure manager ──────────────────────────────────────

/// Closure-related state.
///
/// Groups the four fields that track closure (arrow function) capture
/// semantics: what variables are captured, which runtime instances are
/// closures, and the struct definitions that must be prepended to output.
#[derive(Debug)]
pub struct ClosureManager {
    /// Captured variables for the current arrow function: (name, type, is_mutable)
    pub current_captured: Vec<(String, ZigType, bool)>,
    /// Map: closure variable name → list of (captured_name, type, is_mutable)
    /// Used to generate struct instance at assignment and .call() at call site.
    pub closure_vars: std::collections::HashMap<String, Vec<(String, ZigType, bool)>>,
    /// Set of variable names that are closure instances (assigned from arrow functions with captures).
    /// Used to rewrite `fn(args)` to `fn.call(args)` in emit_call.
    pub closure_instances: std::collections::HashSet<String>,
    /// Closure struct definitions to be prepended to output (module-level).
    pub closure_defs: Vec<String>,
}

impl Default for ClosureManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ClosureManager {
    pub fn new() -> Self {
        Self {
            current_captured: Vec::new(),
            closure_vars: std::collections::HashMap::new(),
            closure_instances: std::collections::HashSet::new(),
            closure_defs: Vec::new(),
        }
    }

    /// Take the current captured variables, leaving an empty vec in their place.
    /// Used at the start of emit_fn / emit_closure_struct to save old state.
    pub fn take_captured(&mut self) -> Vec<(String, ZigType, bool)> {
        std::mem::take(&mut self.current_captured)
    }

    /// Restore previously saved captured variables.
    pub fn restore_captured(&mut self, saved: Vec<(String, ZigType, bool)>) {
        self.current_captured = saved;
    }
}
