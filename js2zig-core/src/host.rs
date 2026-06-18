//! Host function registry — allows JS code to call custom Rust functions.
//!
//! This module provides a registry where developers can register Rust functions
//! (marked `#[no_mangle] pub extern "C"`) that should be callable from
//! translated JS/Zig code via the C ABI.
//!
//! # Usage
//!
//! In `main.rs`, create a `HostFnRegistry`, register Rust functions, and pass it
//! to `project.rs` for metadata generation and to `builtins`/`codegen` for translation.

use crate::infer::ZigType;
use std::path::Path;

// ── Struct definitions ──

/// A field in a host struct (for C ABI ↔ Zig conversion).
#[derive(Debug, Clone)]
pub struct HostStructField {
    pub name: String,
    /// Zig type for the clean struct (e.g. "i64", "[]const u8")
    pub zig_type: String,
    /// C ABI type for the extern struct (e.g. "i64", "\[128\]u8")
    pub c_type: String,
}

/// Definition of a struct used by host functions.
#[derive(Debug, Clone)]
pub struct HostStructDef {
    /// Clean Zig struct name (e.g. "UserInfo")
    pub zig_name: String,
    /// C ABI extern struct name (e.g. "HostUserInfo")
    pub c_name: String,
    /// Fields
    pub fields: Vec<HostStructField>,
}

// ── Function definitions ──

/// Definition of a single host function that JS can call.
#[derive(Debug, Clone)]
pub struct HostFnDef {
    /// JS-side function name (e.g. "hostAdd" for sync, "fetchUser" for async)
    pub name: String,
    /// C ABI symbol name (e.g. "hostFetchUser"). Same as `name` for sync fns.
    pub c_name: String,
    /// Parameter names and Zig types (wrapper-level types)
    pub params: Vec<(String, ZigType)>,
    /// Return type (wrapper-level)
    pub ret_type: ZigType,
    /// Whether this is an async function (needs io.async wrapper)
    pub is_async: bool,
}

/// Registry of Rust host functions exposed to JS via C ABI.
pub struct HostFnRegistry {
    fns: Vec<HostFnDef>,
    structs: Vec<HostStructDef>,
}

impl HostFnRegistry {
    pub fn new() -> Self {
        Self {
            fns: Vec::new(),
            structs: Vec::new(),
        }
    }

    /// Register a sync host function that JS code can call directly.
    pub fn register(&mut self, name: &str, params: Vec<(String, ZigType)>, ret_type: ZigType) {
        self.fns.push(HostFnDef {
            name: name.to_string(),
            c_name: name.to_string(),
            params,
            ret_type,
            is_async: false,
        });
    }

    /// Register an async host function that JS code calls with `await`.
    ///
    /// The async wrapper `fn {name}(io: Io, ...) !{ret_type}` is generated
    /// automatically and calls the C ABI function `{c_name}`.
    pub fn register_async(
        &mut self,
        name: &str,
        c_name: &str,
        params: Vec<(String, ZigType)>,
        ret_struct: HostStructDef,
    ) {
        let struct_zig_name = ret_struct.zig_name.clone();
        self.structs.push(ret_struct);
        self.fns.push(HostFnDef {
            name: name.to_string(),
            c_name: c_name.to_string(),
            params,
            ret_type: ZigType::Struct(struct_zig_name),
            is_async: true,
        });
    }

    /// Look up a host function by JS name.
    pub fn lookup(&self, name: &str) -> Option<&HostFnDef> {
        self.fns.iter().find(|f| f.name == name)
    }

    /// Iterate over all registered host functions.
    pub fn iter(&self) -> impl Iterator<Item = &HostFnDef> {
        self.fns.iter()
    }

    /// Iterate over only sync host functions (for builtins registration).
    pub fn sync_fns(&self) -> impl Iterator<Item = &HostFnDef> {
        self.fns.iter().filter(|f| !f.is_async)
    }

    /// Generate the Zig `host.zig` file content.
    ///
    /// Emits:
    /// - `extern struct` definitions for C ABI types
    /// - `extern "c" fn` declarations for C ABI host functions defined in Rust
    ///
    /// Note: `zig build test` (standalone Zig tests) may fail to link if test code
    /// calls host functions. Host-dependent functions are tested via Rust FFI instead.
    pub fn generate_zig_header(&self) -> String {
        if self.fns.is_empty() {
            return String::new();
        }
        let mut out = String::new();
        out.push_str("// Auto-generated host function declarations (Rust via C ABI)\n");
        out.push_str("// These symbols are defined in Rust with #[no_mangle] pub extern \"C\".\n");
        out.push_str("// Allocator: uses the global allocator from the runtime.\n");
        out.push_str("const js_allocator = @import(\"js_runtime/js_allocator.zig\");\n\n");

        // Emit C ABI struct definitions
        for s in &self.structs {
            out.push_str(&format!("pub const {} = extern struct {{\n", s.c_name));
            for f in &s.fields {
                out.push_str(&format!("    {}: {},\n", f.name, f.c_type));
            }
            out.push_str("};\n\n");
        }

        // Emit extern "c" function declarations
        for def in &self.fns {
            let params_zig: Vec<String> = def
                .params
                .iter()
                .map(|(n, t)| format!("{}: {}", n, Self::to_c_abi_type(t)))
                .collect();
            let ret_name = match &def.ret_type {
                ZigType::Struct(name) => {
                    // Find the matching struct's C name
                    self.structs
                        .iter()
                        .find(|s| &s.zig_name == name)
                        .map(|s| s.c_name.clone())
                        .unwrap_or_else(|| name.clone())
                }
                other => other.to_zig_str(),
            };

            out.push_str(&format!(
                "pub extern \"c\" fn {}({}) callconv(.c) {};\n",
                def.c_name,
                params_zig.join(", "),
                ret_name
            ));
        }
        out
    }

    /// Convert a ZigType to the corresponding C ABI type string.
    fn to_c_abi_type(ty: &ZigType) -> String {
        match ty {
            ZigType::String => "[*:0]const u8".to_string(),
            other => other.to_zig_str(),
        }
    }

    /// Generate async wrapper functions for all registered async host functions.
    ///
    /// These wrappers are emitted into lib.zig alongside the translated JS code.
    /// Each wrapper:
    /// 1. Converts JS-level params to C ABI params (null-terminated strings)
    /// 2. Calls the C ABI function
    /// 3. Converts the C ABI return struct to a clean Zig struct
    pub fn generate_async_wrappers(&self) -> String {
        if !self.fns.iter().any(|f| f.is_async) {
            return String::new();
        }
        let mut out = String::new();
        out.push_str("// ── Async host function wrappers ──\n\n");

        // Collect unique struct defs used by async wrappers
        let mut emitted_structs: std::collections::HashSet<String> = std::collections::HashSet::new();

        for def in &self.fns {
            if !def.is_async {
                continue;
            }

            // Emit clean Zig struct definition
            if let ZigType::Struct(ref zig_name) = def.ret_type
                && !emitted_structs.contains(zig_name)
            {
                emitted_structs.insert(zig_name.clone());
                if let Some(s) = self.structs.iter().find(|s| &s.zig_name == zig_name) {
                    out.push_str(&format!("pub const {} = struct {{\n", s.zig_name));
                    for f in &s.fields {
                        out.push_str(&format!("    {}: {},\n", f.name, f.zig_type));
                    }
                    out.push_str("};\n\n");
                }
            }

            // Generate async wrapper function
            let ret_type_str = def.ret_type.to_zig_str();
            let params_str: Vec<String> = def
                .params
                .iter()
                .map(|(n, t)| format!("{}: {}", n, t.to_zig_str()))
                .collect();

            out.push_str(&format!(
                "fn {}(io: Io, {}) !{} {{\n",
                def.name,
                params_str.join(", "),
                ret_type_str
            ));

            // Emit body
            out.push_str("    _ = io;\n");

            // Convert string params to null-terminated C strings
            for (pname, ptype) in &def.params {
                if *ptype == ZigType::String {
                    out.push_str(&format!(
                        "    const c_{} = js_allocator.g_alloc().dupeZ(u8, {}) catch return error.OutOfMemory;\n",
                        pname, pname
                    ));
                }
            }

            // Defer-free string params
            for (pname, ptype) in &def.params {
                if *ptype == ZigType::String {
                    out.push_str(&format!("    defer js_allocator.g_alloc().free(c_{});\n", pname));
                }
            }
            out.push('\n');

            // Call C ABI function
            let call_args: Vec<String> = def
                .params
                .iter()
                .map(|(n, t)| {
                    if *t == ZigType::String {
                        format!("c_{}", n)
                    } else {
                        n.clone()
                    }
                })
                .collect();

            // Build the call and conversion
            if matches!(&def.ret_type, ZigType::Struct(_)) {
                // Return struct: call C function, convert to clean struct
                if let Some(s) = self.structs.iter().find(|s| s.zig_name == ret_type_str) {
                    out.push_str(&format!(
                        "    const raw = host.{}({});\n",
                        def.c_name,
                        call_args.join(", ")
                    ));

                    // Convert each string field from C buffer to []const u8
                    for f in &s.fields {
                        if f.zig_type == "[]const u8" {
                            // Need to find null terminator in fixed buffer
                            out.push_str(&format!(
                                "    const {}_len = std.mem.indexOfScalar(u8, &raw.{}, 0) orelse raw.{}.len;\n",
                                f.name, f.name, f.name
                            ));
                        }
                    }

                    // Build return struct
                    let field_inits: Vec<String> = s.fields.iter().map(|f| {
                        if f.zig_type == "[]const u8" {
                            format!("    .{} = raw.{}[0..{}_len]", f.name, f.name, f.name)
                        } else {
                            format!("    .{} = raw.{}", f.name, f.name)
                        }
                    }).collect();

                    out.push_str(&format!(
                        "    return .{{\n{}\n    }};\n",
                        field_inits.join(",\n")
                    ));
                }
            } else {
                // Simple return type
                out.push_str(&format!(
                    "    return host.{}({});\n",
                    def.c_name,
                    call_args.join(", ")
                ));
            }

            out.push_str("}\n\n");
        }
        out
    }

    /// Load host function definitions from a JSON config file.
    ///
    /// Format:
    /// ```json
    /// {
    ///   "host_functions": [
    ///     { "name": "hostAdd", "params": [{"name":"a","type":"i64"}], "ret_type": "i64" },
    ///     { "name": "fetchUser", "c_name": "hostFetchUser",
    ///       "params": [{"name":"name","type":"string"}],
    ///       "ret_type": "struct", "ret_struct": "UserInfo", "async": true }
    ///   ],
    ///   "host_structs": [
    ///     { "zig_name": "UserInfo", "c_name": "HostUserInfo",
    ///       "fields": [{"name":"id","zig_type":"i64","c_type":"i64"}] }
    ///   ]
    /// }
    /// ```
    pub fn load_from_file(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read '{}': {}", path.display(), e))?;
        let json: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| format!("invalid JSON in '{}': {}", path.display(), e))?;

        let mut registry = Self::new();
        let mut struct_map: std::collections::HashMap<String, HostStructDef> =
            std::collections::HashMap::new();

        // Parse struct definitions first (needed by async functions).
        // Store in a temporary map; register_async will add them to registry.
        if let Some(structs) = json["host_structs"].as_array() {
            for s in structs {
                let zig_name = s["zig_name"].as_str().ok_or("missing zig_name in struct")?;
                let c_name = s["c_name"].as_str().ok_or("missing c_name in struct")?;
                let fields: Vec<HostStructField> = s["fields"]
                    .as_array()
                    .ok_or("missing fields array in struct")?
                    .iter()
                    .map(|f| {
                        Ok(HostStructField {
                            name: f["name"].as_str().ok_or("missing field name")?.into(),
                            zig_type: f["zig_type"].as_str().ok_or("missing zig_type")?.into(),
                            c_type: f["c_type"].as_str().ok_or("missing c_type")?.into(),
                        })
                    })
                    .collect::<Result<Vec<_>, &str>>()?;
                struct_map.insert(
                    zig_name.to_string(),
                    HostStructDef {
                        zig_name: zig_name.into(),
                        c_name: c_name.into(),
                        fields,
                    },
                );
            }
        }

        // Parse function definitions
        if let Some(fns) = json["host_functions"].as_array() {
            for f in fns {
                let name = f["name"].as_str().ok_or("missing name in host function")?;
                let params: Vec<(String, ZigType)> = f["params"]
                    .as_array()
                    .ok_or("missing params array")?
                    .iter()
                    .map(|p| {
                        let n = p["name"].as_str().ok_or("missing param name")?;
                        let t = Self::parse_type_str(p["type"].as_str().ok_or("missing param type")?);
                        Ok((n.into(), t))
                    })
                    .collect::<Result<Vec<_>, &str>>()?;
                let is_async = f["async"].as_bool().unwrap_or(false);

                if is_async {
                    let c_name = f["c_name"].as_str().ok_or("async fn missing c_name")?;
                    let ret_struct_name = f["ret_struct"].as_str().ok_or("async fn missing ret_struct")?;
                    let ret_struct = struct_map
                        .get(ret_struct_name)
                        .cloned()
                        .ok_or_else(|| format!("undefined struct: {}", ret_struct_name))?;
                    registry.register_async(name, c_name, params, ret_struct);
                } else {
                    let ret_type = Self::parse_type_str(f["ret_type"].as_str().ok_or("missing ret_type")?);
                    registry.register(name, params, ret_type);
                }
            }
        }

        Ok(registry)
    }

    /// Parse a type string from JSON config into ZigType.
    fn parse_type_str(s: &str) -> ZigType {
        match s {
            "i64" => ZigType::I64,
            "f64" => ZigType::F64,
            "bool" => ZigType::Bool,
            "string" => ZigType::String,
            "any" => ZigType::JsValue,
            "jsvalue" => ZigType::JsValue,
            "jsany" => ZigType::JsAny,
            "void" => ZigType::Void,
            other if other.starts_with("struct:") => ZigType::Struct(other[7..].into()),
            _ => ZigType::JsValue,
        }
    }

    /// Generate JSON metadata for cabi_imports.json (consumed by sys/build.rs).
    pub fn to_json_value(&self) -> Vec<serde_json::Value> {
        let mut imports = Vec::new();
        for def in &self.fns {
            let params: Vec<serde_json::Value> = def
                .params
                .iter()
                .map(|(n, t)| {
                    serde_json::json!({
                        "name": n,
                        "zig_type": t.to_zig_str()
                    })
                })
                .collect();
            imports.push(serde_json::json!({
                "name": def.c_name,
                "params": params,
                "ret_type": def.ret_type.to_zig_str(),
            }));
        }
        imports
    }
}

impl Default for HostFnRegistry {
    fn default() -> Self {
        Self::new()
    }
}
