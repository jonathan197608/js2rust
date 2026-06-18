//! Code generation from JS AST to Zig source code.
//! Main module: struct definitions, helpers, public API.

use std::collections::{HashMap, HashSet};

use oxc_ast::ast::*;

use crate::builtins::BuiltinRegistry;
use crate::infer::{TypeInferrer, ZigType};

mod builtins;
mod closure;
mod expr;
mod fn_decl;
mod stmt;

/// Information about a closure generated from an inline arrow function
#[derive(Debug, Clone)]
struct ClosureInfo {
    /// Struct name, e.g. "_Closure_makeAdder"
    struct_name: String,
    /// Captured (free) variable names and their Zig types
    captured: Vec<(String, String)>,
    /// Arrow function parameter names and types
    params: Vec<(String, String)>,
    /// Return type as Zig string
    return_type: String,
    /// Pre-generated struct definition string (filled during record_closure)
    struct_def: String,
}

/// Generate a unique closure struct name from a function name
fn closure_name(fn_name: &str) -> String {
    format!("_Closure_{}", fn_name)
}

/// A single leaf binding extracted from a destructuring pattern.
/// For `const { a: { b, c } } = obj`, we get two leaf bindings:
///   LeafBinding { name: "b", access: "_tmp.a.b", default: None }
///   LeafBinding { name: "c", access: "_tmp.a.c", default: None }
struct LeafBinding<'a> {
    name: &'a str,
    /// Full access path from the temp variable (e.g. "_tmp.a.b" or "_tmp[0].x")
    access: String,
}

/// Recursively flatten a BindingPattern into a list of leaf bindings.
/// `prefix` is the access path from the temp variable (e.g. "_tmp").
fn flatten_binding_pattern<'a>(
    pattern: &'a BindingPattern<'a>,
    prefix: &str,
    result: &mut Vec<LeafBinding<'a>>,
) {
    match pattern {
        BindingPattern::BindingIdentifier(id) => {
            result.push(LeafBinding {
                name: id.name.as_str(),
                access: prefix.to_string(),
            });
        }
        BindingPattern::ObjectPattern(obj) => {
            for prop in &obj.properties {
                let key_str = property_key_name(&prop.key);
                let new_prefix = format!("{}.{}", prefix, key_str);
                flatten_binding_pattern(&prop.value, &new_prefix, result);
            }
            // rest: ...rest — skip for now (requires runtime support)
        }
        BindingPattern::ArrayPattern(arr) => {
            for (i, elem) in arr.elements.iter().enumerate().filter_map(|(i, opt)| opt.as_ref().map(|e| (i, e))) {
                let new_prefix = format!("{}[{}]", prefix, i);
                flatten_binding_pattern(elem, &new_prefix, result);
            }
            // rest: ...rest — skip for now
        }
        BindingPattern::AssignmentPattern(assign) => {
            // Default value: for simplicity, skip the default in the first pass
            // The leaf binding gets the access path; default is not emitted.
            flatten_binding_pattern(&assign.left, prefix, result);
        }
    }
}

/// Extract a string representation of a PropertyKey for use in access paths.
/// E.g., `{ a: x }` → "a"
fn property_key_name(key: &PropertyKey) -> String {
    match key {
        PropertyKey::StaticIdentifier(id) => id.name.to_string(),
        _ => "_computed_key".to_string(),
    }
}

/// Collect all leaf identifier names from a BindingPattern.
/// Used by preprocess and infer to register variable names without access paths.
pub fn collect_binding_names(pattern: &BindingPattern, names: &mut Vec<String>) {
    match pattern {
        BindingPattern::BindingIdentifier(id) => {
            names.push(id.name.to_string());
        }
        BindingPattern::ObjectPattern(obj) => {
            for prop in &obj.properties {
                collect_binding_names(&prop.value, names);
            }
            if let Some(rest) = &obj.rest {
                collect_binding_names(&rest.argument, names);
            }
        }
        BindingPattern::ArrayPattern(arr) => {
            for elem in arr.elements.iter().flatten() {
                collect_binding_names(elem, names);
            }
            if let Some(rest) = &arr.rest {
                collect_binding_names(&rest.argument, names);
            }
        }
        BindingPattern::AssignmentPattern(assign) => {
            collect_binding_names(&assign.left, names);
        }
    }
}

/// Collect all leaf identifier names and their spans from a BindingPattern.
/// Used by preprocess for rename edits.
pub fn collect_binding_names_with_spans(
    pattern: &BindingPattern,
    names: &mut Vec<(String, oxc_span::Span)>,
) {
    match pattern {
        BindingPattern::BindingIdentifier(id) => {
            names.push((id.name.to_string(), id.span));
        }
        BindingPattern::ObjectPattern(obj) => {
            for prop in &obj.properties {
                collect_binding_names_with_spans(&prop.value, names);
            }
            if let Some(rest) = &obj.rest {
                collect_binding_names_with_spans(&rest.argument, names);
            }
        }
        BindingPattern::ArrayPattern(arr) => {
            for elem in arr.elements.iter().flatten() {
                collect_binding_names_with_spans(elem, names);
            }
            if let Some(rest) = &arr.rest {
                collect_binding_names_with_spans(&rest.argument, names);
            }
        }
        BindingPattern::AssignmentPattern(assign) => {
            collect_binding_names_with_spans(&assign.left, names);
        }
    }
}

/// Check whether a BindingPattern is just a simple identifier (no destructuring).
pub fn is_simple_binding(pattern: &BindingPattern) -> bool {
    matches!(pattern, BindingPattern::BindingIdentifier(_))
}

/// Return type for codegen::generate: (zig_code, diagnostics, closure_fns, fn_return_types, cabi_exports)
pub type CodegenResult = (
    String,
    Vec<crate::infer::Diagnostic>,
    std::collections::HashSet<String>,
    std::collections::HashMap<String, ZigType>,
    Vec<CabiExport>,
);

pub fn generate(
    program: &Program,
    builtins: &BuiltinRegistry,
    exports: &HashSet<String>,
) -> CodegenResult {
    let mut inferrer = TypeInferrer::new();
    inferrer.infer_program(program);

    // Extract fn_return_types before inferrer is consumed by ZigCodegen
    let fn_return_types = inferrer.all_fn_return_types();

    let mut diagnostics = inferrer.diagnostics().to_vec();

    let mut cg = ZigCodegen::new(inferrer, &mut diagnostics, builtins, exports.clone());
    // Header is added by project.rs/generate_lib_zig() — do NOT emit here.

    // Pre-scan: find functions that return inline arrow functions
    cg.pre_scan_closures(program);

    // Push all pre-generated closure struct defs to the output buffer
    for def in cg.closure_struct_defs.values() {
        cg.closure_structs.push(def.clone());
    }

    // Top-level: only VariableDeclaration and FunctionDeclaration
    // Rejected statements produce Error diagnostics inside emit_stmt()
    cg.in_top_level = true;
    for stmt in &program.body {
        cg.emit_stmt(stmt);
    }

    // Emit buffered closure struct definitions
    for def in &cg.closure_structs {
        cg.output.push_str(def);
    }

    // Emit buffered object type struct definitions
    for def in &cg.object_type_defs {
        cg.output.push_str(def);
    }

    // Emit C ABI export wrappers (after all functions, before tests)
    for wrapper in &cg.cabi_wrappers {
        cg.output.push_str(wrapper);
    }

    // Emit init_js2rust() function if there are dynamic access variables
    cg.emit_init_js2rust();
    // Emit deinit_js2rust() function for cleanup
    cg.emit_deinit_js2rust();

    let closure_fns: HashSet<String> = cg.fn_closure_spans.keys().cloned().collect();

    let cabi_exports = cg.cabi_exports;

    (cg.output, diagnostics, closure_fns, fn_return_types, cabi_exports)
}

/// Metadata for a C ABI exported function, used by sys crate to generate Rust FFI bindings.
#[derive(Debug, Clone)]
pub struct CabiExport {
    /// C ABI exported function name (e.g. "chineseAdd")
    pub name: String,
    /// (param_name, ZigType) pairs
    pub params: Vec<(String, ZigType)>,
    /// Return type (ZigType)
    pub ret_type: ZigType,
    /// Whether a corresponding free_xxx function exists
    pub has_free_func: bool,
}

struct ZigCodegen<'a> {
    output: String,
    indent: usize,
    inferrer: TypeInferrer,
    diagnostics: &'a mut Vec<crate::infer::Diagnostic>,
    in_top_level: bool,
    task_counter: usize,
    builtins: &'a BuiltinRegistry,
    /// arrow.span.start → ClosureInfo for ALL detected inline arrow functions
    closure_map: HashMap<u32, ClosureInfo>,
    /// arrow.span.start → pre-generated struct definition string
    closure_struct_defs: HashMap<u32, String>,
    /// fn_name → arrow.span.start (for functions that return a closure)
    fn_closure_spans: HashMap<String, u32>,
    /// Counter for generating unique synthetic closure names
    closure_counter: usize,
    /// Buffered closure struct definitions to emit after all functions
    closure_structs: Vec<String>,
    /// Buffered C ABI export wrappers to emit after all functions
    cabi_wrappers: Vec<String>,
    /// C ABI export metadata for sys crate FFI bindings
    cabi_exports: Vec<CabiExport>,
    /// Set of function names that need free_xxx functions (return strings)
    string_return_fns: HashSet<String>,
    /// Set of variable names that hold closure struct instances
    closure_vars: HashSet<String>,
    /// Current function name for closure lookup
    current_fn: Option<String>,
    /// Set of exported function/var names (from preprocess)
    exports: HashSet<String>,
    /// Current try-block label (for translating `throw` to `break :label error.X`)
    try_label: Option<String>,
    /// Current catch-block label (for translating `return` in catch to `break :label expr`)
    catch_label: Option<String>,
    /// Counter for generating unique try-block labels
    try_counter: usize,
    /// Counter for generating unique temp variable names (destructuring)
    temp_counter: usize,
    /// Pending body prelude statements for destructured function parameters
    /// Cleared after each function body is emitted.
    destructure_prelude: Vec<String>,
    /// Current class context: (struct_name, field_names) for `self` tracking
    current_class: Option<(String, Vec<String>)>,
    /// Buffered struct type definitions for top-level object literals
    object_type_defs: Vec<String>,
    /// Per-function: struct name for each Object-typed parameter (indexed by param position).
    /// Populated at the start of emit_fn_decl, consumed by emit_params, cleared after the function.
    current_obj_structs: Vec<Option<String>>,
    /// Buffered initialization code for init_globals() function.
    /// Each string is a line of code to be inserted into init_globals().
    init_globals_code: Vec<String>,
}


impl<'a> ZigCodegen<'a> {
    pub(super) fn new(
        inferrer: TypeInferrer,
        diagnostics: &'a mut Vec<crate::infer::Diagnostic>,
        builtins: &'a BuiltinRegistry,
        exports: HashSet<String>,
    ) -> Self {
        Self {
            output: String::new(),
            indent: 0,
            inferrer,
            diagnostics,
            in_top_level: false,
            task_counter: 0,
            builtins,
            closure_map: HashMap::new(),
            closure_struct_defs: HashMap::new(),
            fn_closure_spans: HashMap::new(),
            closure_counter: 0,
            closure_structs: Vec::new(),
            cabi_wrappers: Vec::new(),
            cabi_exports: Vec::new(),
            string_return_fns: HashSet::new(),
            closure_vars: HashSet::new(),
            current_fn: None,
            exports,
            try_label: None,
            catch_label: None,
            try_counter: 0,
            temp_counter: 0,
            destructure_prelude: Vec::new(),
            current_class: None,
            object_type_defs: Vec::new(),
            current_obj_structs: Vec::new(),
            init_globals_code: Vec::new(),
        }
    }

    pub(super) fn emit_indent(&mut self) {
        for _ in 0..self.indent {
            self.output.push_str("    ");
        }
    }

    pub(super) fn get_indent_str(&self, level: usize) -> String {
        "    ".repeat(level)
    }

    pub(super) fn push(&mut self, s: &str) {
        self.output.push_str(s);
    }

    pub(super) fn push_line(&mut self, s: &str) {
        self.emit_indent();
        self.push(s);
        self.push("\n");
    }

    pub(super) fn binding_name<'b>(&self, pattern: &BindingPattern<'b>) -> &'b str {
        match pattern {
            BindingPattern::BindingIdentifier(id) => id.name.as_str(),
            _ => "_unsupported_pattern",
        }
    }

    pub(super) fn escape_keyword(name: &str) -> String {
        let keywords: &[&str] = &[
            "align", "allowzero", "and", "anyframe", "anytype", "asm", "async",
            "await", "break", "callconv", "catch", "comptime", "const", "continue",
            "defer", "else", "enum", "errdefer", "error", "export", "extern",
            "false", "fn", "for", "if", "inline", "linksection", "noalias",
            "noinline", "noreturn", "nosuspend", "null", "opaque", "or", "orelse", "packed",
            "pub", "resume", "return", "struct", "suspend", "switch", "test",
            "threadlocal", "true", "try", "type", "undefined", "union", "unreachable",
            "usingnamespace", "var", "volatile", "while",
        ];
        if keywords.contains(&name) {
            format!("@\"{}\"", name)
        } else {
            name.to_string()
        }
    }

    /// Wrap an init expression with the appropriate constructor for JsValue/JsAny types.
    /// For precise Zig types (I64, F64, Bool, String), the expression is emitted as-is.
    pub(super) fn emit_typed_init(&mut self, init: &Expression, target_type: &ZigType) {
        match target_type {
            ZigType::JsValue => {
                let expr_type = self.inferrer.infer_expr(init);
                match expr_type {
                    ZigType::I64 | ZigType::I32 | ZigType::Usize => {
                        self.push("JsValue.fromI64(");
                        self.emit_expr(init);
                        self.push(")");
                    }
                    ZigType::F64 | ZigType::F32 => {
                        self.push("JsValue.fromF64(");
                        self.emit_expr(init);
                        self.push(")");
                    }
                    ZigType::Bool => {
                        self.push("JsValue.fromBool(");
                        self.emit_expr(init);
                        self.push(")");
                    }
                    ZigType::String => {
                        self.push("JsValue.fromString(");
                        self.emit_expr(init);
                        self.push(")");
                    }
                    ZigType::Null => {
                        self.push("JsValue.fromNull()");
                    }
                    _ => {
                        self.emit_expr(init);
                    }
                }
            }
            ZigType::JsAny => {
                let expr_type = self.inferrer.infer_expr(init);
                match expr_type {
                    ZigType::I64 | ZigType::I32 | ZigType::Usize => {
                        self.push("JsAny.fromI64(");
                        self.emit_expr(init);
                        self.push(")");
                    }
                    ZigType::F64 | ZigType::F32 => {
                        self.push("JsAny.fromF64(");
                        self.emit_expr(init);
                        self.push(")");
                    }
                    ZigType::Bool => {
                        self.push("JsAny.fromBool(");
                        self.emit_expr(init);
                        self.push(")");
                    }
                    ZigType::String => {
                        self.push("JsAny.fromString(");
                        self.emit_expr(init);
                        self.push(")");
                    }
                    ZigType::Null => {
                        self.push("JsAny.fromNull()");
                    }
                    _ => {
                        self.emit_expr(init);
                    }
                }
            }
            _ => {
                self.emit_expr(init);
            }
        }
    }

    // ========== Statements ==========

    pub(super) fn emit_dynamic_access_var_decl(&mut self, name: &str) {
        self.emit_indent();
        self.push("var ");
        self.push(name);
        self.push(": std.StringHashMap(JsAny) = undefined;\n");
    }

    /// Generate initialization code for a dynamic access variable.
    /// The code is buffered in init_globals_code and emitted in init_js2rust().
    pub(super) fn emit_dynamic_access_var_init_code(&mut self, name: &str, obj: &ObjectExpression) {
        // Add initialization code: name = std.StringHashMap(JsAny).init(allocator);
        self.init_globals_code.push(format!(
            "    {} = std.StringHashMap(JsAny).init(allocator);\n",
            name
        ));

        // Add put() calls for each property
        for prop in &obj.properties {
            if let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(p) = prop {
                let field_name = match &p.key {
                    oxc_ast::ast::PropertyKey::StaticIdentifier(id) => id.name.to_string(),
                    oxc_ast::ast::PropertyKey::StringLiteral(s) => s.value.to_string(),
                    _ => continue,
                };

                // Generate JsAny literal based on expression type
                let js_any_lit = self.emit_js_any_literal(&p.value);
                self.init_globals_code.push(format!(
                    "    {}.put(\"{}\", {}) catch @panic(\"OOM\");\n",
                    name, field_name, js_any_lit
                ));
            }
        }
    }

    /// Generate a JsAny literal from a constant expression.
    /// For non-constant expressions, wraps in JsAny.fromValue(JsValue{...}).
    pub(super) fn emit_js_any_literal(&self, expr: &Expression) -> String {
        match expr {
            Expression::NumericLiteral(lit) => {
                if lit.value.is_finite() && lit.value == lit.value.trunc() {
                    format!("JsAny.fromI64({})", lit.value as i64)
                } else {
                    format!("JsAny.fromF64({})", lit.value)
                }
            }
            Expression::StringLiteral(s) => {
                format!("JsAny.fromString(\"{}\")", Self::escape_zig_string(&s.value))
            }
            Expression::BooleanLiteral(b) => {
                format!("JsAny.fromBool({})", b.value)
            }
            Expression::NullLiteral(_) => {
                "JsAny.fromNull()".to_string()
            }
            _ => {
                // For non-literal expressions, fall back to JsValue wrapping
                let js_val = self.emit_js_value_literal(expr);
                format!("JsAny.fromValue({})", js_val)
            }
        }
    }

    /// Escape a string for Zig string literal context.
    fn escape_zig_string(s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    }

    pub(super) fn emit_js_value_literal(&self, expr: &Expression) -> String {
        match expr {
            Expression::NumericLiteral(lit) => {
                let val_str = lit.value.to_string();
                // Use is_finite() + trunc() to reliably detect integer literals.
                // f64::fract() can be unreliable due to floating-point precision.
                if lit.value.is_finite() && lit.value == lit.value.trunc() {
                    format!(".{{ .int = {} }}", val_str)
                } else {
                    format!(".{{ .float = {} }}", val_str)
                }
            }
            Expression::StringLiteral(lit) => {
                format!(".{{ .string = \"{}\" }}", lit.value)
            }
            Expression::BooleanLiteral(lit) => {
                format!(".{{ .bool = {} }}", if lit.value { "true" } else { "false" })
            }
            Expression::NullLiteral(_) => {
                ".{ .null = {} }".to_string()
            }
            _ => {
                // Unsupported - store as int 0
                ".{ .int = 0 }".to_string()
            }
        }
    }

    /// Emit JsValue construction code for an expression (used in HashMap.put() calls).
    /// For literals, emit inline JsValue literal.
    /// For complex expressions, emit a simple wrap (assumes i64).
    pub(super) fn emit_js_value_construction(&mut self, expr: &Expression) {
        match expr {
            Expression::NumericLiteral(lit) => {
                let val_str = lit.value.to_string();
                if lit.value.is_finite() && lit.value == lit.value.trunc() {
                    self.push(&format!("JsValue{{ .int = {} }}", val_str));
                } else {
                    self.push(&format!("JsValue{{ .float = {} }}", val_str));
                }
            }
            Expression::StringLiteral(lit) => {
                self.push(&format!("JsValue{{ .string = \"{}\" }}", lit.value));
            }
            Expression::BooleanLiteral(lit) => {
                let b = if lit.value { "true" } else { "false" };
                self.push(&format!("JsValue{{ .bool = {} }}", b));
            }
            Expression::NullLiteral(_) => {
                self.push("JsValue{ .null = {} }");
            }
            _ => {
                // TODO: use runtime helper for proper type conversion
                self.push("JsValue{ .int = ");
                self.emit_expr(expr);
                self.push(" }");
            }
        }
    }

    /// Return the correct JsValue variant field accessor for a HashMap field lookup.
    /// Look up the field type from the original object type stored in the type inferrer.
    pub(super) fn dynamic_field_accessor(&self, obj_expr: &Expression, prop: &str) -> String {
        if let Expression::Identifier(id) = obj_expr {
            let obj_type = self.inferrer.get_var_type(id.name.as_str());
            if let ZigType::Object { fields } = obj_type
                && let Some((_, field_type)) = fields.iter().find(|(n, _)| n == prop)
            {
                return match field_type {
                    ZigType::String => ".string".to_string(),
                    ZigType::F64 | ZigType::F32 => ".float".to_string(),
                    ZigType::Bool => ".bool".to_string(),
                    ZigType::Null => ".null".to_string(),
                    _ => ".asI64()".to_string(),
                };
            }
        }
        ".asI64()".to_string()
    }

    pub(super) fn emit_init_js2rust(&mut self) {
        // Always generate init_js2rust() — empty if no dynamic access vars
        self.push("\n");
        self.push("/// Initialize global allocator and all objects that use dynamic property access.\n");
        self.push("pub fn init_js2rust(allocator: std.mem.Allocator) void {\n");
        self.push("    js_allocator.setGlobalAllocator(allocator);\n");
        // Collect init code to avoid borrowing self twice
        let init_code: String = self.init_globals_code.iter().cloned().collect();
        if !init_code.is_empty() {
            self.push(&init_code);
        }
        self.push("}\n");
    }

    /// Emit deinit_js2rust() that frees all global HashMaps.
    pub(super) fn emit_deinit_js2rust(&mut self) {
        // Extract HashMap variable names from init code (format: "    varname = std.StringHashMap...")
        let mut hashmap_names: Vec<String> = Vec::new();
        for line in &self.init_globals_code {
            if line.contains(".init(")
                && let Some(name) = line.trim().split(" =").next()
            {
                hashmap_names.push(name.to_string());
            }
        }
        // Always generate deinit — noop if no HashMaps, but needed for defer
        self.push("\n");
        self.push("/// Deinitialize all global objects created by init_js2rust().\n");
        self.push("pub fn deinit_js2rust() void {\n");
        for name in &hashmap_names {
            self.push(&format!("    {}.deinit();\n", name));
        }
        self.push("}\n");
    }

    pub(super) fn gen_obj_struct_def(struct_name: &str, fields: &[(String, ZigType)]) -> String {
        let mut s = format!("const {} = struct {{\n", struct_name);
        for (fname, ftype) in fields {
            s.push_str(&format!("    {}: {},\n", fname, ftype.to_zig_str()));
        }
        s.push_str("};\n\n");
        s
    }

    /// Capitalize the first letter for struct naming (e.g., "person" → "Person")
    pub(super) fn capitalize_first(s: &str) -> String {
        let mut chars = s.chars();
        match chars.next() {
            None => String::new(),
            Some(c) => {
                let upper = c.to_uppercase().collect::<String>();
                upper + chars.as_str()
            }
        }
    }

    pub(super) fn is_string_literal_expr(expr: &Expression) -> bool {
        matches!(expr, Expression::StringLiteral(_))
    }

    // ========== Expression Helpers ==========

    pub(super) fn emit_arg(&mut self, arg: &Argument) {
        match arg {
            Argument::SpreadElement(spread) => {
                self.push("...");
                self.emit_expr(&spread.argument);
            }
            _ => {
                if let Some(expr) = arg.as_expression() {
                    self.emit_expr(expr);
                }
            }
        }
    }

    /// Emit an argument wrapped in JsAny constructor for dynamic array methods.
    pub(super) fn emit_jsany_arg(&mut self, arg: &Argument) {
        match arg {
            Argument::SpreadElement(_) => {
                // Spread not supported in dynamic array method args
                self.push("JsAny.fromNull()");
            }
            _ => {
                if let Some(expr) = arg.as_expression() {
                    let lit = self.emit_js_any_literal(expr);
                    self.push(&lit);
                }
            }
        }
    }

    pub(super) fn emit_array_element(&mut self, elem: &ArrayExpressionElement) {
        match elem {
            ArrayExpressionElement::SpreadElement(spread) => {
                self.push("...");
                self.emit_expr(&spread.argument);
            }
            ArrayExpressionElement::Elision(_) => {
                self.push("undefined");
            }
            _ => {
                if let Some(expr) = elem.as_expression() {
                    self.emit_expr(expr);
                }
            }
        }
    }

    /// Emit an array element wrapped in JsAny constructor for dynamic arrays.
    pub(super) fn emit_jsany_array_element(&mut self, elem: &ArrayExpressionElement) {
        match elem {
            ArrayExpressionElement::SpreadElement(_) => {
                self.push("JsAny.fromNull()"); // spread not supported in array literal
            }
            ArrayExpressionElement::Elision(_) => {
                self.push("JsAny.fromNull()");
            }
            _ => {
                if let Some(expr) = elem.as_expression() {
                    let lit = self.emit_js_any_literal(expr);
                    self.push(&lit);
                }
            }
        }
    }

    pub(super) fn emit_assign_target(&mut self, target: &AssignmentTarget) {
        match target {
            AssignmentTarget::AssignmentTargetIdentifier(id) => {
                self.push(&Self::escape_keyword(id.name.as_str()));
            }
            AssignmentTarget::ArrayAssignmentTarget(_) => {
                self.push("_/* array destructure */");
            }
            AssignmentTarget::ObjectAssignmentTarget(_) => {
                self.push("_/* object destructure */");
            }
            AssignmentTarget::StaticMemberExpression(mem) => {
                self.emit_expr(&mem.object);
                self.push(".");
                self.push(mem.property.name.as_str());
            }
            AssignmentTarget::ComputedMemberExpression(mem) => {
                let obj_type = self.inferrer.infer_expr(&mem.object);
                if matches!(&obj_type, ZigType::Object { .. })
                    && let Expression::StringLiteral(s) = &mem.expression
                {
                    self.emit_expr(&mem.object);
                    self.push(".");
                    self.push(s.value.as_str());
                } else {
                    self.emit_expr(&mem.object);
                    self.push("[");
                    self.emit_expr(&mem.expression);
                    self.push("]");
                }
            }
            AssignmentTarget::PrivateFieldExpression(_) => {
                self.push("_/* private field */");
            }
            AssignmentTarget::TSAsExpression(ts) => {
                self.emit_expr(&ts.expression);
            }
            AssignmentTarget::TSSatisfiesExpression(ts) => {
                self.emit_expr(&ts.expression);
            }
            AssignmentTarget::TSNonNullExpression(ts) => {
                self.emit_expr(&ts.expression);
                self.push(".?");
            }
            AssignmentTarget::TSTypeAssertion(ts) => {
                self.emit_expr(&ts.expression);
            }
        }
    }

    pub(super) fn emit_assign_target_from_simple(&mut self, target: &SimpleAssignmentTarget) {
        match target {
            SimpleAssignmentTarget::AssignmentTargetIdentifier(id) => {
                self.push(&Self::escape_keyword(id.name.as_str()));
            }
            SimpleAssignmentTarget::StaticMemberExpression(mem) => {
                self.emit_expr(&mem.object);
                self.push(".");
                self.push(mem.property.name.as_str());
            }
            SimpleAssignmentTarget::ComputedMemberExpression(mem) => {
                let obj_type = self.inferrer.infer_expr(&mem.object);
                if matches!(&obj_type, ZigType::Object { .. })
                    && let Expression::StringLiteral(s) = &mem.expression
                {
                    self.emit_expr(&mem.object);
                    self.push(".");
                    self.push(s.value.as_str());
                } else {
                    self.emit_expr(&mem.object);
                    self.push("[");
                    self.emit_expr(&mem.expression);
                    self.push("]");
                }
            }
            SimpleAssignmentTarget::PrivateFieldExpression(_) => {
                self.push("_/* private */");
            }
            _ => {
                self.push("_/* ts assign target */");
            }
        }
    }

    // ========== Operator Mappings ==========

    pub(super) fn map_binary_op(&self, op: &BinaryOperator) -> &'static str {
        match op {
            BinaryOperator::StrictEquality => "==",
            BinaryOperator::StrictInequality => "!=",
            BinaryOperator::Equality => "==",
            BinaryOperator::Inequality => "!=",
            BinaryOperator::Addition => "+",
            BinaryOperator::Subtraction => "-",
            BinaryOperator::Multiplication => "*",
            BinaryOperator::Division => "/",
            BinaryOperator::Remainder => "%",
            BinaryOperator::Exponential => "**",
            BinaryOperator::LessThan => "<",
            BinaryOperator::LessEqualThan => "<=",
            BinaryOperator::GreaterThan => ">",
            BinaryOperator::GreaterEqualThan => ">=",
            BinaryOperator::ShiftLeft => "<<",
            BinaryOperator::ShiftRight => ">>",
            BinaryOperator::ShiftRightZeroFill => ">>",
            BinaryOperator::BitwiseOR => "|",
            BinaryOperator::BitwiseXOR => "^",
            BinaryOperator::BitwiseAnd => "&",
            BinaryOperator::In => "== /* in */",
            BinaryOperator::Instanceof => "== /* instanceof */",
        }
    }

    pub(super) fn map_logical_op(&self, op: &LogicalOperator) -> &'static str {
        match op {
            LogicalOperator::And => "and",
            LogicalOperator::Or => "or",
            LogicalOperator::Coalesce => "orelse",
        }
    }

    pub(super) fn map_unary_op(&self, op: &UnaryOperator) -> &'static str {
        match op {
            UnaryOperator::UnaryPlus => "",  // Zig doesn't support unary plus
            UnaryOperator::UnaryNegation => "-",
            UnaryOperator::LogicalNot => "!",
            UnaryOperator::BitwiseNot => "~",
            UnaryOperator::Typeof => "@TypeOf",
            UnaryOperator::Void => "",
            UnaryOperator::Delete => "",
        }
    }

    pub(super) fn map_assign_op(&self, op: &AssignmentOperator) -> &'static str {
        match op {
            AssignmentOperator::Assign => "=",
            AssignmentOperator::Addition => "+=",
            AssignmentOperator::Subtraction => "-=",
            AssignmentOperator::Multiplication => "*=",
            AssignmentOperator::Division => "/=",
            AssignmentOperator::Remainder => "%=",
            AssignmentOperator::Exponential => "**=",
            AssignmentOperator::ShiftLeft => "<<=",
            AssignmentOperator::ShiftRight => ">>=",
            AssignmentOperator::ShiftRightZeroFill => ">>>=",
            AssignmentOperator::BitwiseOR => "|=",
            AssignmentOperator::BitwiseXOR => "^=",
            AssignmentOperator::BitwiseAnd => "&=",
            AssignmentOperator::LogicalOr => "|=",
            AssignmentOperator::LogicalAnd => "&=",
            AssignmentOperator::LogicalNullish => "=?=",
        }
    }
}
