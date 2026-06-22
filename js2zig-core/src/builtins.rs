use crate::host::HostFnRegistry;
use crate::native_proto::ZigType;
use std::collections::HashMap;

/// Result of looking up a JS built-in call
#[derive(Debug, Clone)]
pub struct BuiltinTranslation {
    /// The Zig code template, {} placeholders for arguments
    pub template: String,
    /// Return type of the builtin function (if known)
    pub return_type: Option<crate::native_proto::ZigType>,
}

impl BuiltinTranslation {
    /// Create a new BuiltinTranslation with template only (return type unknown).
    pub fn new(template: String) -> Self {
        Self {
            template,
            return_type: None,
        }
    }

    /// Create a new BuiltinTranslation with known return type.
    pub fn with_return_type(template: String, return_type: crate::native_proto::ZigType) -> Self {
        Self {
            template,
            return_type: Some(return_type),
        }
    }
}

/// Central registry for JS → Zig built-in mappings
pub struct BuiltinRegistry {
    /// Method calls: key = "object.method" → template
    methods: HashMap<String, BuiltinTranslation>,
    /// Global function calls: key = "functionName" → template
    globals: HashMap<String, BuiltinTranslation>,
    /// Static property access: key = "object.property" → template
    properties: HashMap<String, String>,
}

impl BuiltinRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            methods: HashMap::new(),
            globals: HashMap::new(),
            properties: HashMap::new(),
        };

        // ── Math constants (static properties) ──
        registry.add_property("Math", "PI", "std.math.pi");
        registry.add_property("Math", "E", "std.math.e");

        // ── Tier 1: Direct Zig builtins ──
        registry.add_method("Math", "abs", "@abs({})");
        registry.add_method("Math", "ceil", "@ceil({})");
        registry.add_method("Math", "floor", "@floor({})");
        registry.add_method("Math", "trunc", "@trunc({})");
        registry.add_method("Math", "round", "@round({})");
        registry.add_method("Math", "sqrt", "@sqrt({})");
        registry.add_method("Math", "cbrt", "@sqrt(@sqrt({}))"); // approximation
        registry.add_method("Math", "sin", "@sin({})");
        registry.add_method("Math", "cos", "@cos({})");
        registry.add_method("Math", "tan", "@tan({})");
        registry.add_method("Math", "asin", "@asin({})");
        registry.add_method("Math", "acos", "@acos({})");
        registry.add_method("Math", "atan", "@atan({})");
        registry.add_method("Math", "atan2", "@atan2({}, {})");
        registry.add_method("Math", "exp", "@exp({})");
        registry.add_method("Math", "log", "@log({})");
        registry.add_method("Math", "log2", "@log2({})");
        registry.add_method("Math", "log10", "@log10({})");
        registry.add_method("Math", "min", "@min({}, {})");
        registry.add_method("Math", "max", "@max({}, {})");
        registry.add_method(
            "Math",
            "sign",
            "if ({0} > 0) @as(i64, 1) else if ({0} < 0) @as(i64, -1) else @as(i64, 0)",
        );
        registry.add_method("Math", "hypot", "@sqrt({0} * {0} + {1} * {1})");

        // ── Tier 2: std lib ──
        registry.add_method("Math", "pow", "std.math.pow(f64, {}, {})");
        registry.add_method("Math", "random", "std.crypto.random.float(f64)");

        // Global functions (Tier 2)
        registry.add_global(
            "parseInt",
            "std.fmt.parseInt(i64, {}, 10) catch @as(i64, 0)",
        );
        registry.add_global(
            "parseFloat",
            "std.fmt.parseFloat(f64, {}) catch @as(f64, 0.0)",
        );
        registry.add_global("isNaN", "std.math.isNan(@as(f64, {}))");
        registry.add_global("isFinite", "!std.math.isInf({})");
        registry.add_global("Number", "std.fmt.parseInt(i64, {}, 10) catch @as(i64, 0)");

        // ── Tier 3: URI encoding ──
        registry.add_global(
            "encodeURIComponent",
            "js_uri.encodeURIComponent(js_allocator.g_alloc(), {})",
        );
        registry.add_global(
            "decodeURIComponent",
            "js_uri.decodeURIComponent(js_allocator.g_alloc(), {})",
        );

        // ── Tier 3: Runtime ──
        // All runtime functions use js_allocator.g_alloc() where Zig needs an allocator.
        // For type-dispatched methods, {} is sequential: {0}=receiver, {1}=arg0, {2}=arg1, ...
        // String methods
        registry.add_method("String", "length", "{}.len");
        registry.add_method_runtime("string", "length", "{}.len", "js_string");
        registry.add_method_runtime(
            "string",
            "toUpperCase",
            "(js_string.toUpper(js_allocator.g_alloc(), {}) catch \"\")",
            "js_string",
        );
        registry.add_method_runtime(
            "string",
            "toLowerCase",
            "(js_string.toLower(js_allocator.g_alloc(), {}) catch \"\")",
            "js_string",
        );
        registry.add_method_runtime(
            "string",
            "charAt",
            "(js_string.charAt(js_allocator.g_alloc(), {}, {}) catch \"\")",
            "js_string",
        );
        registry.add_method_runtime("string", "charCodeAt", "{0}[@intCast({1})]", "js_string");
        registry.add_method_runtime(
            "string",
            "concat",
            "(js_string.concat(js_allocator.g_alloc(), {}, {}) catch \"\")",
            "js_string",
        );
        registry.add_method_runtime(
            "string",
            "includes",
            "js_string.includes({}, {})",
            "js_string",
        );
        registry.add_method_runtime(
            "string",
            "indexOf",
            "js_string.indexOf({}, {})",
            "js_string",
        );
        registry.add_method_runtime(
            "string",
            "startsWith",
            "js_string.startsWith({}, {})",
            "js_string",
        );
        registry.add_method_runtime(
            "string",
            "endsWith",
            "js_string.endsWith({}, {})",
            "js_string",
        );
        registry.add_method_runtime(
            "string",
            "slice",
            "js_string.slice({}, {}, {})",
            "js_string",
        );
        registry.add_method_runtime(
            "string",
            "split",
            "(js_string.split(js_allocator.g_alloc(), {}, {}) catch &[_][]const u8{})",
            "js_string",
        );
        registry.add_method_runtime(
            "string",
            "replace",
            "(js_string.replace(js_allocator.g_alloc(), {}, {}, {}) catch \"\")",
            "js_string",
        );
        registry.add_method_runtime("string", "trim", "js_string.trim({})", "js_string");
        registry.add_method_runtime(
            "string",
            "repeat",
            "(js_string.repeat(js_allocator.g_alloc(), {}, {}) catch \"\")",
            "js_string",
        );

        // Console
        registry.add_method_runtime("console", "log", "js_console.log({})", "js_console");
        registry.add_method_runtime("console", "error", "js_console.err({})", "js_console");
        registry.add_method_runtime("console", "warn", "js_console.warn({})", "js_console");

        // JSON
        registry.add_method_runtime(
            "JSON",
            "stringify",
            "js_json.stringifyI64(js_allocator.g_alloc(), {})",
            "js_json",
        );
        registry.add_method_runtime(
            "JSON",
            "parse",
            "js_json.parse(js_allocator.g_alloc(), {})",
            "js_json",
        );

        // Array
        registry.add_method_runtime("Array", "isArray", "js_array.isArray({})", "js_array");
        registry.add_method_runtime("array", "length", "{}.len", "js_array");
        registry.add_method_runtime(
            "array",
            "push",
            "(js_array.push(js_allocator.g_alloc(), {}, {}) catch &[_]i64{})",
            "js_array",
        );
        registry.add_method_runtime("array", "pop", "js_array.pop({})", "js_array");
        registry.add_method_runtime("array", "shift", "js_array.shift({})", "js_array");
        registry.add_method_runtime(
            "array",
            "unshift",
            "(js_array.unshift(js_allocator.g_alloc(), {}, {}) catch &[_]i64{})",
            "js_array",
        );
        registry.add_method_runtime(
            "array",
            "join",
            "(js_array.join(js_allocator.g_alloc(), {}, {}) catch \"\")",
            "js_array",
        );
        registry.add_method_runtime(
            "array",
            "map",
            "(js_array.map(js_allocator.g_alloc(), {}, {}) catch &[_]i64{})",
            "js_array",
        );
        registry.add_method_runtime(
            "array",
            "filter",
            "(js_array.filter(js_allocator.g_alloc(), {}, {}) catch &[_]i64{})",
            "js_array",
        );
        registry.add_method_runtime("array", "indexOf", "js_array.indexOf({}, {})", "js_array");
        registry.add_method_runtime("array", "includes", "js_array.includes({}, {})", "js_array");
        registry.add_method_runtime(
            "array",
            "reverse",
            "(js_array.reverse(js_allocator.g_alloc(), {}) catch &[_]i64{})",
            "js_array",
        );
        registry.add_method_runtime("array", "slice", "js_array.slice({}, {}, {})", "js_array");
        registry.add_method_runtime(
            "array",
            "concat",
            "(js_array.concat(js_allocator.g_alloc(), {}, {}) catch &[_]i64{})",
            "js_array",
        );
        registry.add_method_runtime(
            "array",
            "sort",
            "(js_array.sort(js_allocator.g_alloc(), {}) catch &[_]i64{})",
            "js_array",
        );

        // ── Tier 3: Object ──
        registry.add_method_runtime(
            "Object",
            "keys",
            "js_object.keys(js_allocator.g_alloc(), {})",
            "js_object",
        );
        registry.add_method_runtime(
            "Object",
            "values",
            "js_object.values(js_allocator.g_alloc(), {})",
            "js_object",
        );
        registry.add_method_runtime("Object", "assign", "js_object.assign({}, {})", "js_object");
        registry.add_method_runtime(
            "Object",
            "entries",
            "js_object.entries(js_allocator.g_alloc(), {})",
            "js_object",
        );

        // ── Tier 3: Number ──
        registry.add_method_runtime(
            "Number",
            "isNaN",
            "js_number.isNaN(@as(f64, @floatFromInt({})))",
            "js_number",
        );
        registry.add_method_runtime(
            "Number",
            "isFinite",
            "js_number.isFinite(@as(f64, @floatFromInt({})))",
            "js_number",
        );
        registry.add_method_runtime(
            "Number",
            "isInteger",
            "js_number.isInteger(@as(f64, @floatFromInt({})))",
            "js_number",
        );
        registry.add_method_runtime("Number", "parseInt", "js_number.parseInt({})", "js_number");
        registry.add_method_runtime(
            "Number",
            "parseFloat",
            "js_number.parseFloat({})",
            "js_number",
        );

        // ── Tier 3: Boolean ──
        registry.add_method_runtime(
            "Boolean",
            "toString",
            "if ({}) \"true\" else \"false\"",
            "js_number",
        );

        // ── Tier 3: Promise (minimal synchronous support) ──
        // Promise.resolve(value) → js_runtime.Promise.resolve(value)
        // Promise.reject(reason) → js_runtime.Promise.reject(reason)
        // js_runtime exports Promise (via js_promise)
        registry.add_method("Promise", "resolve", "js_runtime.Promise.resolve({})");
        registry.add_method("Promise", "reject", "js_runtime.Promise.reject({})");

        // ── Tier 3: Date ──
        registry.add_method_runtime("Date", "now", "js_date.now()", "js_date");
        registry.add_method_runtime("Date", "getTime", "js_date.getTime({})", "js_date");
        registry.add_method_runtime("Date", "getFullYear", "js_date.getFullYear({})", "js_date");
        registry.add_method_runtime("Date", "getMonth", "js_date.getMonth({})", "js_date");
        registry.add_method_runtime("Date", "getDate", "js_date.getDate({})", "js_date");
        registry.add_method_runtime("Date", "getDay", "js_date.getDay({})", "js_date");
        registry.add_method_runtime("Date", "getHours", "js_date.getHours({})", "js_date");
        registry.add_method_runtime("Date", "getMinutes", "js_date.getMinutes({})", "js_date");
        registry.add_method_runtime("Date", "getSeconds", "js_date.getSeconds({})", "js_date");
        registry.add_method_runtime(
            "Date",
            "parse",
            "std.fmt.parseInt(i64, {}, 10) catch @as(i64, 0)",
            "js_date",
        );

        // ── Tier 3: Map (instance methods) ──
        registry.add_method_runtime("map", "get", "{0}.get({1})", "js_map");
        registry.add_method_runtime(
            "map",
            "set",
            "{0}.set({1}, {2}) catch unreachable",
            "js_map",
        );
        registry.add_method_runtime("map", "has", "{0}.has({1})", "js_map");
        registry.add_method_runtime("map", "delete", "{0}.delete({1})", "js_map");
        registry.add_method_runtime("map", "clear", "{0}.clear()", "js_map");
        registry.add_method_runtime("map", "size", "{0}.size()", "js_map");

        // ── Tier 3: Set (instance methods) ──
        registry.add_method_runtime("set", "add", "{0}.add({1}) catch unreachable", "js_set");
        registry.add_method_runtime("set", "has", "{0}.has({1})", "js_set");
        registry.add_method_runtime("set", "delete", "{0}.delete({1})", "js_set");
        registry.add_method_runtime("set", "clear", "{0}.clear()", "js_set");
        registry.add_method_runtime("set", "size", "{0}.size()", "js_set");

        // ── Tier 3: RegExp ──
        registry.add_method_runtime("RegExp", "test", "js_regexp.test_({}, {})", "js_regexp");
        registry.add_method_runtime(
            "RegExp",
            "exec",
            "js_regexp.exec(js_allocator.g_alloc(), {}, {})",
            "js_regexp",
        );

        // ── Tier 3: TypedArray ──
        // Int32Array
        registry.add_method_runtime("Int32Array", "from", "(js_runtime.js_typedarray.fromI64AsI32(js_allocator.g_alloc(), {}) catch js_runtime.js_typedarray.emptyI32())", "js_typedarray");
        registry.add_method_runtime("int32array", "length", "{}.len", "js_typedarray");
        registry.add_method_runtime(
            "int32array",
            "get",
            "js_runtime.js_typedarray.getI32({}, {})",
            "js_typedarray",
        );
        registry.add_method_runtime(
            "int32array",
            "set",
            "js_runtime.js_typedarray.setI32({}, {}, {})",
            "js_typedarray",
        );
        registry.add_method_runtime(
            "int32array",
            "slice",
            "js_runtime.js_typedarray.sliceI32({}, {}, {})",
            "js_typedarray",
        );
        registry.add_method_runtime(
            "int32array",
            "subarray",
            "js_runtime.js_typedarray.subarrayI32({}, {}, {})",
            "js_typedarray",
        );
        registry.add_method_runtime(
            "int32array",
            "copyWithin",
            "js_runtime.js_typedarray.copyWithinI32({}, {}, {}, {})",
            "js_typedarray",
        );
        registry.add_method_runtime(
            "int32array",
            "fill",
            "js_runtime.js_typedarray.fillI32({}, {}, {}, {})",
            "js_typedarray",
        );

        // Uint8Array
        registry.add_method_runtime("Uint8Array", "from", "(js_runtime.js_typedarray.fromI64AsU8(js_allocator.g_alloc(), {}) catch js_runtime.js_typedarray.emptyU8())", "js_typedarray");
        registry.add_method_runtime("uint8array", "length", "{}.len", "js_typedarray");
        registry.add_method_runtime(
            "uint8array",
            "get",
            "js_runtime.js_typedarray.getU8({}, {})",
            "js_typedarray",
        );
        registry.add_method_runtime(
            "uint8array",
            "set",
            "js_runtime.js_typedarray.setU8({}, {}, {})",
            "js_typedarray",
        );
        registry.add_method_runtime(
            "uint8array",
            "slice",
            "js_runtime.js_typedarray.sliceU8({}, {}, {})",
            "js_typedarray",
        );
        registry.add_method_runtime(
            "uint8array",
            "subarray",
            "js_runtime.js_typedarray.subarrayU8({}, {}, {})",
            "js_typedarray",
        );

        // Float64Array
        registry.add_method_runtime("Float64Array", "from", "(js_runtime.js_typedarray.fromF64(js_allocator.g_alloc(), {}) catch js_runtime.js_typedarray.emptyF64())", "js_typedarray");
        registry.add_method_runtime("float64array", "length", "{}.len", "js_typedarray");
        registry.add_method_runtime(
            "float64array",
            "get",
            "js_runtime.js_typedarray.getF64({}, {})",
            "js_typedarray",
        );
        registry.add_method_runtime(
            "float64array",
            "set",
            "js_runtime.js_typedarray.setF64({}, {}, {})",
            "js_typedarray",
        );
        registry.add_method_runtime(
            "float64array",
            "slice",
            "js_runtime.js_typedarray.sliceF64({}, {}, {})",
            "js_typedarray",
        );

        // ── Tier 4: Rust via C ABI (reserved for custom host functions) ──

        registry
    }

    /// Register host functions from a HostFnRegistry so they can be called
    /// from JS code. These functions are defined in Rust with `#[no_mangle] pub extern "C"`.
    ///
    /// For functions with string params/returns, the template calls the `_wrap` variant
    /// (generated in host.zig) which handles `[]const u8` ↔ `[*:0]const u8` conversion.
    pub fn register_host_fns(&mut self, host_fns: &HostFnRegistry) {
        for def in host_fns.sync_fns() {
            // Build the template: host.fnName(arg1, arg2, ...)
            let args: String = (0..def.params.len())
                .map(|i| format!("{{{}}}", i))
                .collect::<Vec<_>>()
                .join(", ");

            // Check if this function needs the string conversion wrapper
            let has_string =
                def.params.iter().any(|(_, t)| *t == ZigType::Str) || def.ret_type == ZigType::Str;
            let fn_name = if has_string {
                format!("{}_wrap", def.name)
            } else {
                def.name.clone()
            };
            let template = format!("host.{}({})", fn_name, args);

            // Store return type
            let return_type = def.ret_type.clone();

            self.globals.insert(
                def.name.clone(),
                BuiltinTranslation::with_return_type(template, return_type),
            );
        }
    }

    fn add_method(&mut self, object: &str, method: &str, template: &str) {
        let key = format!("{}.{}", object, method);
        self.methods
            .insert(key, BuiltinTranslation::new(template.to_string()));
    }

    fn add_method_runtime(&mut self, object: &str, method: &str, template: &str, _dep: &str) {
        self.add_method(object, method, template);
    }

    fn add_global(&mut self, name: &str, template: &str) {
        self.globals.insert(
            name.to_string(),
            BuiltinTranslation::new(template.to_string()),
        );
    }

    fn add_property(&mut self, object: &str, property: &str, zig_expr: &str) {
        let key = format!("{}.{}", object, property);
        self.properties.insert(key, zig_expr.to_string());
    }

    /// Look up a method call: `object.method(...)` → Zig translation
    pub fn lookup_method(&self, object: &str, method: &str) -> Option<&BuiltinTranslation> {
        let key = format!("{}.{}", object, method);
        self.methods.get(&key)
    }

    /// Look up a global function call: `func(...)` → Zig translation
    pub fn lookup_global(&self, name: &str) -> Option<&BuiltinTranslation> {
        self.globals.get(name)
    }

    /// Look up the return type of a global function (if known).
    pub fn lookup_global_return_type(&self, name: &str) -> Option<&crate::native_proto::ZigType> {
        self.globals
            .get(name)
            .and_then(|trans| trans.return_type.as_ref())
    }

    /// Look up a static property: `Obj.prop` → Zig expression
    pub fn lookup_property(&self, object: &str, property: &str) -> Option<&str> {
        let key = format!("{}.{}", object, property);
        self.properties.get(&key).map(|s| s.as_str())
    }
}

impl Default for BuiltinRegistry {
    fn default() -> Self {
        Self::new()
    }
}
