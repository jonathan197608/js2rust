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

use crate::native_proto::ZigType;
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
    pub fns: Vec<HostFnDef>,
    pub structs: Vec<HostStructDef>,
}

impl HostFnRegistry {
    pub fn new() -> Self {
        Self {
            fns: Vec::new(),
            structs: Vec::new(),
        }
    }

    /// Check if the registry is empty (no host functions registered).
    pub fn is_empty(&self) -> bool {
        self.fns.is_empty()
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
            ret_type: ZigType::NamedStruct(struct_zig_name),
            is_async: true,
        });
    }

    /// Register an async host function with a simple (non-struct) return type.
    pub fn register_async_simple(
        &mut self,
        name: &str,
        c_name: &str,
        params: Vec<(String, ZigType)>,
        ret_type: ZigType,
    ) {
        self.fns.push(HostFnDef {
            name: name.to_string(),
            c_name: c_name.to_string(),
            params,
            ret_type,
            is_async: true,
        });
    }

    /// Return the JS-side names of all registered async functions.
    pub fn async_fn_names(&self) -> Vec<String> {
        self.fns
            .iter()
            .filter(|f| f.is_async)
            .map(|f| f.name.clone())
            .collect()
    }

    /// Return struct field types for all registered host structs.
    /// Key: Zig struct name, Value: Vec<(field_name, field_zig_type)>
    pub fn struct_fields_map(&self) -> std::collections::HashMap<String, Vec<(String, ZigType)>> {
        let mut map = std::collections::HashMap::new();
        for s in &self.structs {
            let fields: Vec<(String, ZigType)> = s
                .fields
                .iter()
                .map(|f| {
                    let zig_type = match f.zig_type.as_str() {
                        "i64" | "i32" => ZigType::I64,
                        "f64" => ZigType::F64,
                        "bool" => ZigType::Bool,
                        "[]const u8" => ZigType::Str,
                        _ => ZigType::Void,
                    };
                    (f.name.clone(), zig_type)
                })
                .collect();
            map.insert(s.zig_name.clone(), fields);
        }
        map
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
    /// - Wrapper functions for string params/returns (converts `[]const u8` ↔ `[*:0]const u8`)
    ///
    /// For functions with string parameters or string return types, a `_cabi` extern
    /// declaration is emitted alongside a wrapper function with the original name.
    /// The wrapper converts Zig-level `[]const u8` to null-terminated `[*:0]const u8`
    /// via `dupeZ`, and converts the return value back via `std.mem.span`.
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
        out.push_str("const std = @import(\"std\");\n");
        out.push_str("const Io = std.Io;\n");
        out.push_str("const js_allocator = @import(\"js_runtime/js_allocator.zig\");\n");
        out.push_str("const StrRet = @import(\"js_runtime/string.zig\").StrRet;\n\n");

        // Emit C ABI struct definitions (extern structs).
        // String fields are represented as ptr+len in C ABI (matches JsStrField layout).
        // Field order and types must exactly match Rust #[repr(C)] layout.
        // Rust JsStrField = { ptr: *const u8, len: usize } (16 bytes, two fields).
        for s in &self.structs {
            out.push_str(&format!("pub const {} = extern struct {{\n", s.c_name));
            for f in &s.fields {
                if f.zig_type == "[]const u8" {
                    // Emit ptr+len pair matching JsStrField memory layout.
                    // Field names: {name}_ptr, {name}_len (so async wrapper can access them).
                    out.push_str(&format!(
                        "    {}_ptr: [*]const u8,\n    {}_len: usize,\n",
                        f.name, f.name
                    ));
                } else {
                    out.push_str(&format!("    {}: {},\n", f.name, f.c_type));
                }
            }
            out.push_str("};\n\n");
        }

        // Emit clean Zig struct definitions (for async wrapper return types)
        for s in &self.structs {
            out.push_str(&format!("pub const {} = struct {{\n", s.zig_name));
            for f in &s.fields {
                out.push_str(&format!("    {}: {},\n", f.name, f.zig_type));
            }
            out.push_str("};\n\n");
        }

        // Emit extern "c" function declarations and wrappers
        for def in &self.fns {
            // ── Async functions: emit extern + async wrapper, skip _wrap ──
            if def.is_async {
                // Build C ABI params: string params expand to ptr+len (zero-copy)
                let mut cabi_param_parts = Vec::new();
                for (n, t) in &def.params {
                    for (cabi_name, cabi_type) in Self::to_c_abi_param_types(n, t) {
                        cabi_param_parts.push(format!("{}: {}", cabi_name, cabi_type));
                    }
                }
                let params_cabi = cabi_param_parts.join(", ");

                let ret_cabi = match &def.ret_type {
                    ZigType::NamedStruct(name) => self
                        .structs
                        .iter()
                        .find(|s| &s.zig_name == name)
                        .map(|s| s.c_name.clone())
                        .unwrap_or_else(|| name.clone()),
                    other => Self::to_c_abi_ret_type(other),
                };

                // Extern declaration (C ABI symbol defined in Rust)
                out.push_str(&format!(
                    "extern \"c\" fn {}({}) callconv(.c) {};\n",
                    def.c_name,
                    params_cabi,
                    ret_cabi
                ));

                // Async wrapper function (callable from Zig via host.{name}_async)
                let ret_type_str = def.ret_type.to_zig_type(true);
                let params_zig: Vec<String> = def
                    .params
                    .iter()
                    .map(|(n, t)| format!("{}: {}", n, t.to_zig_type(true)))
                    .collect();

                out.push_str(&format!(
                    "pub fn {}_async(io: Io, {}) !{} {{\n",
                    def.name,
                    params_zig.join(", "),
                    ret_type_str
                ));
                out.push_str("    _ = io;\n");

                // Build call args: string params pass .ptr and .len (zero-copy)
                let mut call_args = Vec::new();
                for (n, t) in &def.params {
                    if *t == ZigType::Str {
                        call_args.push(format!("{}.ptr", n));
                        call_args.push(format!("{}.len", n));
                    } else {
                        call_args.push(n.clone());
                    }
                }

                // Call extern and convert return
                if let ZigType::NamedStruct(ref zig_name) = def.ret_type {
                    if let Some(s) = self.structs.iter().find(|s| &s.zig_name == zig_name) {
                        out.push_str(&format!(
                            "    const raw = {}({});\n",
                            def.c_name,
                            call_args.join(", ")
                        ));
                        // Convert string fields: zero-copy ptr+len from Zig Arena
                        let field_inits: Vec<String> = s.fields.iter().map(|f| {
                            if f.zig_type == "[]const u8" {
                                format!("    .{} = raw.{}_ptr[0..raw.{}_len]", f.name, f.name, f.name)
                            } else {
                                format!("    .{} = raw.{}", f.name, f.name)
                            }
                        }).collect();
                        out.push_str(&format!(
                            "    return .{{\n{}\n    }};\n",
                            field_inits.join(",\n")
                        ));
                    }
                } else if def.ret_type == ZigType::Str {
                    // Zero-copy string return: result is in Zig Arena
                    out.push_str(&format!(
                        "    const result = {}({});\n",
                        def.c_name,
                        call_args.join(", ")
                    ));
                    out.push_str("    return result.toSlice();\n");
                } else {
                    out.push_str(&format!(
                        "    return {}({});\n",
                        def.c_name,
                        call_args.join(", ")
                    ));
                }

                out.push_str("}\n\n");
                continue;
            }

            // ── Sync functions ──
            // Build C ABI params: string params expand to ptr+len (zero-copy)
            let mut cabi_param_parts = Vec::new();
            for (n, t) in &def.params {
                for (cabi_name, cabi_type) in Self::to_c_abi_param_types(n, t) {
                    cabi_param_parts.push(format!("{}: {}", cabi_name, cabi_type));
                }
            }
            let params_cabi = cabi_param_parts.join(", ");

            // Return type in C ABI representation
            let ret_cabi = match &def.ret_type {
                ZigType::NamedStruct(name) => self
                    .structs
                    .iter()
                    .find(|s| &s.zig_name == name)
                    .map(|s| s.c_name.clone())
                    .unwrap_or_else(|| name.clone()),
                other => Self::to_c_abi_ret_type(other),
            };

            // Check if this function needs string conversion wrappers
            let has_string_params = def.params.iter().any(|(_, t)| *t == ZigType::Str);
            let has_string_return = def.ret_type == ZigType::Str;
            let is_void = def.ret_type == ZigType::Void;

            // Build call args for C ABI: string params pass .ptr and .len
            let mut call_args = Vec::new();
            for (n, t) in &def.params {
                if *t == ZigType::Str {
                    call_args.push(format!("{}.ptr", n));
                    call_args.push(format!("{}.len", n));
                } else {
                    call_args.push(n.clone());
                }
            }

            // ── String params or return: generate wrapper ──
            if has_string_params || has_string_return {
                // Extern declaration with original name (matches Rust symbol)
                out.push_str(&format!(
                    "extern \"c\" fn {}({}) callconv(.c) {};\n",
                    def.c_name,
                    params_cabi,
                    ret_cabi
                ));

                // Wrapper function with _wrap suffix (Zig-level types)
                let wrap_name = format!("{}_wrap", def.c_name);
                let params_zig: Vec<String> = def
                    .params
                    .iter()
                    .map(|(n, t)| format!("{}: {}", n, t.to_zig_type(true)))
                    .collect();
                let ret_zig = def.ret_type.to_zig_type(true);

                out.push_str(&format!(
                    "pub fn {}({}) {} {{\n",
                    wrap_name,
                    params_zig.join(", "),
                    ret_zig
                ));

                // Call extern (zero-copy: pass .ptr/.len, receive StrRet)
                if has_string_return {
                    out.push_str(&format!(
                        "    const result = {}({});\n",
                        def.c_name,
                        call_args.join(", ")
                    ));
                    out.push_str("    return result.toSlice();\n");
                } else if is_void {
                    out.push_str(&format!(
                        "    {}({});\n",
                        def.c_name,
                        call_args.join(", ")
                    ));
                } else {
                    out.push_str(&format!(
                        "    return {}({});\n",
                        def.c_name,
                        call_args.join(", ")
                    ));
                }

                out.push_str("}\n\n");
            } else {
                // No string conversion needed: direct extern with original name
                out.push_str(&format!(
                    "pub extern \"c\" fn {}({}) callconv(.c) {};\n",
                    def.c_name,
                    params_cabi,
                    ret_cabi
                ));
            }
        }

        // ── Aliases for native_proto codegen ──
        // The native_proto codegen strips the `host_` prefix from host function
        // calls (e.g. host_add → host.add). Add short-name aliases so that
        // `host.add(...)` resolves to the correct extern/wrapper function.
        let has_aliases = self.fns.iter().any(|f| !f.is_async && f.name.starts_with("host_"));
        if has_aliases {
            out.push_str("\n// Aliases for native_proto codegen (strips host_ prefix)\n");
            for def in &self.fns {
                if def.is_async {
                    continue; // async aliases are generated in lib.zig
                }
                let Some(short) = def.name.strip_prefix("host_") else {
                    continue;
                };
                let has_string = def.params.iter().any(|(_, t)| *t == ZigType::Str)
                    || def.ret_type == ZigType::Str;
                if has_string {
                    out.push_str(&format!("pub const {} = {}_wrap;\n", short, def.name));
                } else {
                    out.push_str(&format!("pub const {} = {};\n", short, def.name));
                }
            }
            out.push('\n');
        }

        out
    }

    /// Convert a ZigType to the corresponding C ABI type string (for return types).
    /// String returns use StrRet (zero-copy: memory is in Zig Arena).
    fn to_c_abi_ret_type(ty: &ZigType) -> String {
        match ty {
            ZigType::Str => "StrRet".to_string(),
            other => other.to_zig_type(true),
        }
    }

    /// Expand a single Zig-level param into one or more C ABI params.
    /// String params expand to `ptr: [*]const u8, len_name: usize` (zero-copy).
    /// Other types pass through as-is.
    fn to_c_abi_param_types(
        param_name: &str,
        ty: &ZigType,
    ) -> Vec<(String, String)> {
        match ty {
            ZigType::Str => vec![
                (format!("{}_ptr", param_name), "[*]const u8".to_string()),
                (format!("{}_len", param_name), "usize".to_string()),
            ],
            other => vec![(param_name.to_string(), other.to_zig_type(true))],
        }
    }

    /// Generate async wrapper functions for all registered async host functions.
    ///
    /// **Deprecated**: Async wrappers are now generated in `generate_zig_header()`
    /// as part of host.zig. This method returns an empty string.
    pub fn generate_async_wrappers(&self) -> String {
        String::new()
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
                        let t =
                            Self::parse_type_str(p["type"].as_str().ok_or("missing param type")?);
                        Ok((n.into(), t))
                    })
                    .collect::<Result<Vec<_>, &str>>()?;
                let is_async = f["async"].as_bool().unwrap_or(false);

                if is_async {
                    let c_name = f["c_name"].as_str().ok_or("async fn missing c_name")?;
                    let ret_struct_name = f["ret_struct"]
                        .as_str()
                        .ok_or("async fn missing ret_struct")?;
                    let ret_struct = struct_map
                        .get(ret_struct_name)
                        .cloned()
                        .ok_or_else(|| format!("undefined struct: {}", ret_struct_name))?;
                    registry.register_async(name, c_name, params, ret_struct);
                } else {
                    let ret_type =
                        Self::parse_type_str(f["ret_type"].as_str().ok_or("missing ret_type")?);
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
            "string" => ZigType::Str,
            "any" => ZigType::Void,
            "jsvalue" => ZigType::Void,
            "jsany" => ZigType::Anytype,
            "void" => ZigType::Void,
            other if other.starts_with("struct:") => ZigType::NamedStruct(other[7..].to_string()),
            _ => ZigType::Void,
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
                        "zig_type": t.to_zig_type(false) // JSON metadata is consumed by pipeline.rs (lib.zig)
                    })
                })
                .collect();
            imports.push(serde_json::json!({
                "name": def.c_name,
                "params": params,
                "ret_type": def.ret_type.to_zig_type(false), // JSON metadata is consumed by pipeline.rs (lib.zig)
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
