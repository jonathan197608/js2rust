use crate::host::HostFnRegistry;
use std::collections::HashMap;

/// Built-in mapping tier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinTier {
    /// Direct Zig builtin: Math.abs → @abs
    Tier1Direct,
    /// Zig std lib: isNaN → std.math.isNan
    Tier2Std,
    /// Runtime library: "s".split → js_string.split
    Tier3Runtime,
    /// Rust via C ABI (stub)
    Tier4Rust,
}

/// Result of looking up a JS built-in call
#[derive(Debug, Clone)]
pub struct BuiltinTranslation {
    /// The Zig code template, {} placeholders for arguments
    pub template: String,
    /// Which tier was used
    #[allow(dead_code)]
    pub tier: BuiltinTier,
    /// Name of the runtime module this depends on (e.g. "js_string"), None if not needed
    #[allow(dead_code)]
    pub runtime_dep: Option<String>,
}

/// Central registry for JS → Zig built-in mappings
pub struct BuiltinRegistry {
    /// Method calls: key = "object.method" → template
    methods: HashMap<String, BuiltinTranslation>,
    /// Global function calls: key = "functionName" → template
    globals: HashMap<String, BuiltinTranslation>,
}

impl BuiltinRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            methods: HashMap::new(),
            globals: HashMap::new(),
        };

        // ── Tier 1: Direct Zig builtins ──
        registry.add_method("Math", "abs", "@abs({})", BuiltinTier::Tier1Direct, None);
        registry.add_method("Math", "ceil", "@ceil({})", BuiltinTier::Tier1Direct, None);
        registry.add_method("Math", "floor", "@floor({})", BuiltinTier::Tier1Direct, None);
        registry.add_method("Math", "trunc", "@trunc({})", BuiltinTier::Tier1Direct, None);
        registry.add_method("Math", "round", "@round({})", BuiltinTier::Tier1Direct, None);
        registry.add_method("Math", "sqrt", "@sqrt({})", BuiltinTier::Tier1Direct, None);
        registry.add_method("Math", "cbrt", "@sqrt(@sqrt({}))", BuiltinTier::Tier1Direct, None); // approximation
        registry.add_method("Math", "sin", "@sin({})", BuiltinTier::Tier1Direct, None);
        registry.add_method("Math", "cos", "@cos({})", BuiltinTier::Tier1Direct, None);
        registry.add_method("Math", "tan", "@tan({})", BuiltinTier::Tier1Direct, None);
        registry.add_method("Math", "exp", "@exp({})", BuiltinTier::Tier1Direct, None);
        registry.add_method("Math", "log", "@log({})", BuiltinTier::Tier1Direct, None);
        registry.add_method("Math", "log2", "@log2({})", BuiltinTier::Tier1Direct, None);
        registry.add_method("Math", "log10", "@log10({})", BuiltinTier::Tier1Direct, None);
        registry.add_method("Math", "min", "@min({}, {})", BuiltinTier::Tier1Direct, None);
        registry.add_method("Math", "max", "@max({}, {})", BuiltinTier::Tier1Direct, None);
        registry.add_method("Math", "sign", "if ({0} > 0) @as(i64, 1) else if ({0} < 0) @as(i64, -1) else @as(i64, 0)", BuiltinTier::Tier1Direct, None);

        // ── Tier 2: std lib ──
        registry.add_method("Math", "pow", "std.math.pow(f64, {}, {})", BuiltinTier::Tier2Std, None);
        registry.add_method(
            "Math", "random",
            "std.crypto.random.float(f64)",
            BuiltinTier::Tier2Std, None,
        );

        // Global functions (Tier 2)
        registry.add_global("parseInt", "std.fmt.parseInt(i64, {}, 10) catch @as(i64, 0)", BuiltinTier::Tier2Std, None);
        registry.add_global("parseFloat", "std.fmt.parseFloat(f64, {}) catch @as(f64, 0.0)", BuiltinTier::Tier2Std, None);
        registry.add_global("isNaN", "std.math.isNan(@as(f64, {}))", BuiltinTier::Tier2Std, None);
        registry.add_global("isFinite", "!std.math.isInf({})", BuiltinTier::Tier2Std, None);
        registry.add_global("Number", "std.fmt.parseInt(i64, {}, 10) catch @as(i64, 0)", BuiltinTier::Tier2Std, None);

        // ── Tier 3: Runtime ──
        // All runtime functions use std.heap.page_allocator internally.
        // String methods
        registry.add_method("String", "length", "{}.len", BuiltinTier::Tier3Runtime, None);
        registry.add_method_runtime("string", "length", "{}.len", "js_string");
        registry.add_method_runtime("string", "toUpperCase", "js_string.toUpper({})", "js_string");
        registry.add_method_runtime("string", "toLowerCase", "js_string.toLower({})", "js_string");
        registry.add_method_runtime("string", "charAt", "js_string.charAt({}, {})", "js_string");
        registry.add_method_runtime("string", "charCodeAt", "{0}[@intCast({1})]", "js_string");
        registry.add_method_runtime("string", "concat", "js_string.concat({}, {})", "js_string");
        registry.add_method_runtime("string", "includes", "js_string.includes({}, {})", "js_string");
        registry.add_method_runtime("string", "indexOf", "js_string.indexOf({}, {})", "js_string");
        registry.add_method_runtime("string", "startsWith", "js_string.startsWith({}, {})", "js_string");
        registry.add_method_runtime("string", "endsWith", "js_string.endsWith({}, {})", "js_string");
        registry.add_method_runtime("string", "slice", "js_string.slice({}, {}, {})", "js_string");
        registry.add_method_runtime("string", "split", "js_string.split({}, {})", "js_string");
        registry.add_method_runtime("string", "replace", "js_string.replace({}, {}, {})", "js_string");
        registry.add_method_runtime("string", "trim", "js_string.trim({})", "js_string");
        registry.add_method_runtime("string", "repeat", "js_string.repeat({}, {})", "js_string");

        // Console
        registry.add_method_runtime("console", "log", "js_console.log({})", "js_console");
        registry.add_method_runtime("console", "error", "js_console.err({})", "js_console");
        registry.add_method_runtime("console", "warn", "js_console.warn({})", "js_console");

        // JSON
        registry.add_method_runtime("JSON", "stringify", "js_json.stringifyI64({})", "js_json");
        registry.add_method_runtime("JSON", "parse", "js_json.parse({})", "js_json");

        // Array
        registry.add_method_runtime("Array", "isArray", "js_array.isArray({})", "js_array");
        registry.add_method_runtime("array", "length", "{}.len", "js_array");
        registry.add_method_runtime("array", "push", "js_array.push({}, {})", "js_array");
        registry.add_method_runtime("array", "pop", "js_array.pop({})", "js_array");
        registry.add_method_runtime("array", "join", "js_array.join({}, {})", "js_array");
        registry.add_method_runtime("array", "map", "js_array.map({}, {})", "js_array");
        registry.add_method_runtime("array", "filter", "js_array.filter({}, {})", "js_array");

        // ── Tier 4: Rust via C ABI (stubs) ──
        registry.add_method("RegExp", "test", "js_regexp.test(alloc, {}, {})", BuiltinTier::Tier4Rust, Some("js_regexp".into()));
        registry.add_method("RegExp", "exec", "js_regexp.exec(alloc, {}, {})", BuiltinTier::Tier4Rust, Some("js_regexp".into()));
        registry.add_method("Date", "now", "js_date.now()", BuiltinTier::Tier4Rust, Some("js_date".into()));
        registry.add_method("Date", "getTime", "js_date.getTime({})", BuiltinTier::Tier4Rust, Some("js_date".into()));

        registry
    }

    /// Register host functions from a HostFnRegistry so they can be called
    /// from JS code. These functions are defined in Rust with `#[no_mangle] pub extern "C"`.
    pub fn register_host_fns(&mut self, host_fns: &HostFnRegistry) {
        for def in host_fns.sync_fns() {
            // Build the template: host.fnName(arg1, arg2, ...)
            let args: String = (0..def.params.len())
                .map(|i| format!("{{{}}}", i))
                .collect::<Vec<_>>()
                .join(", ");
            let template = format!("host.{}({})", def.name, args);

            self.globals.insert(
                def.name.clone(),
                BuiltinTranslation {
                    template,
                    tier: BuiltinTier::Tier4Rust,
                    runtime_dep: None,
                },
            );
        }
    }

    fn add_method(&mut self, object: &str, method: &str, template: &str, tier: BuiltinTier, runtime_dep: Option<String>) {
        let key = format!("{}.{}", object, method);
        self.methods.insert(key, BuiltinTranslation {
            template: template.to_string(),
            tier,
            runtime_dep,
        });
    }

    fn add_method_runtime(&mut self, object: &str, method: &str, template: &str, dep: &str) {
        self.add_method(object, method, template, BuiltinTier::Tier3Runtime, Some(dep.to_string()));
    }

    fn add_global(&mut self, name: &str, template: &str, tier: BuiltinTier, runtime_dep: Option<String>) {
        self.globals.insert(name.to_string(), BuiltinTranslation {
            template: template.to_string(),
            tier,
            runtime_dep,
        });
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

    /// Collect all unique runtime dependencies needed
    #[allow(dead_code)]
    pub fn collect_runtime_deps<'a>(&'a self, translations: impl Iterator<Item = &'a BuiltinTranslation>) -> Vec<&'a str> {
        let mut deps: Vec<&str> = Vec::new();
        for t in translations {
            if let Some(ref dep) = t.runtime_dep
                && !deps.contains(&dep.as_str())
            {
                deps.push(dep.as_str());
            }
        }
        deps
    }
}

impl Default for BuiltinRegistry {
    fn default() -> Self {
        Self::new()
    }
}
