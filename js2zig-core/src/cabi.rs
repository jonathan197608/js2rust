//! C ABI wrapper generation and metadata serialization.
//!
//! This module contains all logic for generating `pub export fn` C ABI wrapper
//! code (consumed by `project::generate` as part of `lib.zig`) and for writing
//! C ABI export/import JSON metadata files (consumed by `js2rust-bridge-macro`).

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use crate::host::HostFnRegistry;
use crate::types::{NativeCabiExport, ZigType};

// ── Helpers ─────────────────────────────────────────────

/// Format conversion statements: empty → empty string, otherwise joined with newlines.
fn format_conversions(convs: &[String]) -> String {
    if convs.is_empty() {
        String::new()
    } else {
        format!("{}\n", convs.join("\n"))
    }
}

/// Emit `comptime { @export(...) }` line for a CABI wrapper.
fn emit_comptime_export(out: &mut String, name: &str) {
    out.push_str(&format!(
        "comptime {{ @export(&{name}_cabi, .{{ .name = \"{name}\", .linkage = .strong }}); }}\n",
        name = name,
    ));
}

/// Emit a `pub const` alias for exports that bypass C ABI wrapping (JsAny,
/// ArrayList, or functions with unsupported parameter types).
fn emit_const_alias(out: &mut String, name: &str, bare_name: &str, module: &str) {
    out.push_str(&format!(
        "pub const {name} = {mod}.{bare};\n\n",
        name = name,
        bare = bare_name,
        mod = module,
    ));
}

/// Disambiguate a name by appending the module name if it collides.
pub(super) fn disambiguate_name(
    name: &str,
    module: &str,
    is_colliding: impl Fn(&str) -> bool,
) -> String {
    if is_colliding(name) {
        format!("{}_{}", name, module)
    } else {
        name.to_string()
    }
}

/// Emit an async Zig-friendly adapter function.
fn emit_async_adapter(
    out: &mut String,
    name: &str,
    bare: &str,
    params: &str,
    ret_type: &str,
    module: &str,
    args: &str,
) {
    out.push_str(&format!(
        "pub fn {name}({params}) {ret_type} {{\n    return {module}.{bare}({args}) catch @panic(\"async error in {name}\");\n}}\n",
        name = name,
        bare = bare,
        params = params,
        ret_type = ret_type,
        module = module,
        args = args,
    ));
}

// ── Public API ──────────────────────────────────────────

/// Generate `pub export fn` wrapper code for lib.zig.
/// Each wrapper calls the per-file module function and lives in the root lib.zig,
/// so Zig correctly propagates the symbols into the final .lib.
///
/// For string-returning functions, ALSO generate a Zig-friendly adapter
/// (`pub fn greet(s: []const u8) []const u8`) so test code can call
/// the function with idiomatic Zig string types.
///
/// `cabi_rename` maps disambiguated CABI names → bare function names.
/// When an export name collides across modules, the CABI wrapper gets the
/// disambiguated name (`{fn}_{module}`) as its public symbol, but calls the
/// original bare-named function inside the per-file module.
pub fn gen_cabi_wrappers(
    name_to_module: &HashMap<String, String>,
    name_to_cabi: &HashMap<String, &NativeCabiExport>,
    cabi_rename: &HashMap<String, String>,
) -> String {
    let mut out = String::new();
    let mut emitted: HashSet<&str> = HashSet::new();

    for (cabi_name, exp) in name_to_cabi {
        if !emitted.insert(cabi_name.as_str()) {
            continue;
        }
        let Some(module) = name_to_module.get(cabi_name) else {
            continue;
        };
        // Prefix module name with _ to match orchestrator import (const _mod = @import(...))
        let module = format!("_{}", module);
        // Bare function name inside the per-file module (may differ from cabi_name when collision)
        let bare_name = cabi_rename
            .get(cabi_name)
            .map(|s| s.as_str())
            .unwrap_or(cabi_name.as_str());
        // `name` = public/disambiguated name (used for wrapper declarations, @export, etc.)
        let name = cabi_name.as_str();

        let returns_string = exp.ret_type == ZigType::Str;
        let ret_is_js_any = exp.ret_type == ZigType::Anytype;
        let ret_is_arraylist = matches!(exp.ret_type, ZigType::ArrayList(_));

        // JsAny/ArrayList returns: re-export as const alias (no CABI export).
        // This lets Zig test code call the function, but no C ABI symbol is emitted.
        if ret_is_js_any || ret_is_arraylist {
            emit_const_alias(&mut out, name, bare_name, &module);
            continue;
        }

        // Skip functions with JsValue/JsAny parameters (C ABI doesn't support unions)
        let has_js_obj_param = exp
            .params
            .iter()
            .any(|(_, ty)| *ty == ZigType::Void || *ty == ZigType::Anytype);
        if has_js_obj_param {
            emit_const_alias(&mut out, name, bare_name, &module);
            continue;
        }

        // Build parameter lists for all function types
        let mut cabi_params: Vec<String> = Vec::new();
        let mut zig_params: Vec<String> = Vec::new();
        let mut arg_names: Vec<String> = Vec::new();
        let mut cabi_to_zig_conversions: Vec<String> = Vec::new();

        for (pname, ptype) in &exp.params {
            arg_names.push(pname.clone());
            if *ptype == ZigType::Str {
                cabi_params.push(format!("{}: [*:0]const u8", pname));
                zig_params.push(format!("{}: []const u8", pname));
                cabi_to_zig_conversions.push(format!(
                    "    const {p}_slice: []const u8 = std.mem.span({p});",
                    p = pname
                ));
            } else {
                let zig_ty = ptype.to_zig_type();
                cabi_params.push(format!("{}: {}", pname, zig_ty));
                zig_params.push(format!("{}: {}", pname, zig_ty));
            }
        }

        // Build call args: for CABI wrapper, string params use _slice version
        let zig_call_args: String = arg_names.join(", ");
        let cabi_call_args: String = exp
            .params
            .iter()
            .map(|(pname, ptype)| {
                if *ptype == ZigType::Str {
                    format!("{}_slice", pname)
                } else {
                    pname.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(", ");

        // ── Async exports: call _impl with js_runtime.getIo(), catch errors ──
        if exp.is_async {
            let async_zig_args = if zig_call_args.is_empty() {
                "js_runtime.getIo()".to_string()
            } else {
                format!("js_runtime.getIo(), {}", zig_call_args)
            };
            let async_cabi_args = if cabi_call_args.is_empty() {
                "js_runtime.getIo()".to_string()
            } else {
                format!("js_runtime.getIo(), {}", cabi_call_args)
            };

            if returns_string {
                // Zig-friendly adapter (for tests) — calls _impl directly
                emit_async_adapter(
                    &mut out,
                    name,
                    bare_name,
                    &zig_params.join(", "),
                    "[]const u8",
                    &module,
                    &async_zig_args,
                );
                // C ABI wrapper (StrRet — zero-copy, panic via negative len)
                let conversions = format_conversions(&cabi_to_zig_conversions);
                out.push_str(&format!(
                    "pub export fn {name}_cabi({cabi_params}) StrRet {{\n{conv}    return StrRet.from({mod}.{bare}({args}) catch |err| return StrRet.from_panic(err));\n}}\n",
                    name = name,
                    bare = bare_name,
                    cabi_params = cabi_params.join(", "),
                    conv = conversions,
                    mod = module,
                    args = async_cabi_args,
                ));
                emit_comptime_export(&mut out, name);
            } else if let ZigType::NamedStruct(ref sn) = exp.ret_type {
                // Async struct return: use out-pointer C ABI wrapper
                let struct_name = format!("host.{}", sn);
                let conversions = format_conversions(&cabi_to_zig_conversions);

                // Zig-friendly adapter (for tests)
                emit_async_adapter(
                    &mut out,
                    name,
                    bare_name,
                    &zig_params.join(", "),
                    &struct_name,
                    &module,
                    &async_zig_args,
                );

                // C ABI wrapper: add *<struct_name> out-pointer parameter
                let mut cabi_params_with_out = cabi_params.clone();
                cabi_params_with_out.push(format!("result: *{}", struct_name));
                let cabi_params_str = cabi_params_with_out.join(", ");

                let cabi_call = format!(
                    "{mod}.{bare}({args})",
                    mod = module,
                    bare = bare_name,
                    args = async_cabi_args,
                );
                out.push_str(&format!(
                    "pub export fn {name}_cabi({params}) void {{\n{conv}    const _result = {cabi_call} catch @panic(\"async error in {name}\");\n    result.* = _result;\n}}\n",
                    name = name,
                    params = cabi_params_str,
                    conv = conversions,
                    cabi_call = cabi_call,
                ));
                emit_comptime_export(&mut out, name);
            } else {
                // Async non-string, non-struct return (e.g., i64, bool)
                let ret_zig = exp.ret_type.to_zig_type();
                let conversions = format_conversions(&cabi_to_zig_conversions);

                // Zig-friendly adapter (for tests)
                emit_async_adapter(
                    &mut out,
                    name,
                    bare_name,
                    &zig_params.join(", "),
                    &ret_zig,
                    &module,
                    &async_zig_args,
                );

                // C ABI wrapper
                let cabi_params_with_runtime = {
                    let mut p = cabi_params.clone();
                    p.push("js_runtime: *JSRuntime".to_string());
                    p
                };
                let cabi_params_str = cabi_params_with_runtime.join(", ");

                let cabi_call = format!(
                    "{mod}.{bare}({args})",
                    mod = module,
                    bare = bare_name,
                    args = async_cabi_args,
                );
                out.push_str(&format!(
                    "pub export fn {name}_cabi({params}) {ret} {{\n{conv}    return {cabi_call} catch @panic(\"async error in {name}\");\n}}\n",
                    name = name,
                    params = cabi_params_str,
                    ret = ret_zig,
                    conv = conversions,
                    cabi_call = cabi_call,
                ));
                emit_comptime_export(&mut out, name);
            }

            out.push('\n');
            continue;
        }

        if returns_string {
            // ── Zig-friendly adapter (for tests) — calls _impl directly, no conversion ──
            let test_call = if exp.can_throw {
                format!(
                    "{mod}.{bare}({args}) catch @panic(\"error in {name}\")",
                    mod = module,
                    bare = bare_name,
                    name = name,
                    args = zig_call_args,
                )
            } else {
                format!(
                    "{mod}.{bare}({args})",
                    mod = module,
                    bare = bare_name,
                    args = zig_call_args
                )
            };
            out.push_str(&format!(
                "pub fn {name}({params}) []const u8 {{\n    return {test_call};\n}}\n",
                name = name,
                params = zig_params.join(", "),
                test_call = test_call,
            ));

            // ── C ABI wrapper (StrRet — zero-copy, error via sign-bit) ──
            let conversions = format_conversions(&cabi_to_zig_conversions);
            let cabi_call = if exp.can_throw {
                format!(
                    "{mod}.{bare}({args}) catch |err| return StrRet.from_panic(err)",
                    mod = module,
                    bare = bare_name,
                    args = cabi_call_args,
                )
            } else {
                format!(
                    "{mod}.{bare}({args})",
                    mod = module,
                    bare = bare_name,
                    args = cabi_call_args,
                )
            };
            out.push_str(&format!(
                "pub export fn {name}_cabi({cabi_params}) StrRet {{\n{conv}    return StrRet.from({cabi_call});\n}}\n",
                name = name,
                cabi_params = cabi_params.join(", "),
                conv = conversions,
                cabi_call = cabi_call,
            ));
            emit_comptime_export(&mut out, name);
        } else {
            let ret_zig = exp.ret_type.to_cabi_str();
            let exp_ret_is_js_value = exp.ret_type == ZigType::Void;

            // Build C ABI param list: add _err out-param for can_throw non-string exports
            let mut cabi_params_with_err = cabi_params.clone();
            if exp.can_throw {
                cabi_params_with_err.push("err_out: *?[*:0]const u8".to_string());
            }
            let cabi_params_str = cabi_params_with_err.join(", ");

            // Build the call expression with error handling for can_throw
            let (call_expr, err_setup) = if exp.can_throw {
                let err_handle = if ret_zig == "void" {
                    // Void: call without assignment
                    format!(
                        "    {mod}.{bare}({args}) catch |err| {{\n        err_out.* = @errorName(err);\n        return;\n    }};",
                        mod = module,
                        bare = bare_name,
                        args = cabi_call_args,
                    )
                } else {
                    // Use type-appropriate zero value for the catch fallback
                    let ret_zero = if ret_zig == "bool" { "false" } else { "0" };
                    format!(
                        "    const _result = {mod}.{bare}({args}) catch |err| {{\n        err_out.* = @errorName(err);\n        return {ret_zero};\n    }};",
                        mod = module,
                        bare = bare_name,
                        args = cabi_call_args,
                        ret_zero = ret_zero,
                    )
                };
                let setup = if ret_zig == "void" {
                    "    err_out.* = null;\n".to_string()
                } else {
                    String::new()
                };
                (err_handle, setup)
            } else {
                (
                    format!(
                        "{mod}.{bare}({args})",
                        mod = module,
                        bare = bare_name,
                        args = cabi_call_args,
                    ),
                    String::new(),
                )
            };

            let conversions = format_conversions(&cabi_to_zig_conversions);

            if ret_zig == "void" {
                if exp.can_throw {
                    out.push_str(&format!(
                        "pub export fn {name}({params}) void {{\n{conv}{err_setup}{call_expr}\n}}\n",
                        name = name,
                        params = cabi_params_str,
                        conv = conversions,
                        err_setup = err_setup,
                        call_expr = call_expr,
                    ));
                } else {
                    out.push_str(&format!(
                        "pub export fn {name}({params}) void {{\n{conv}    {mod}.{bare}({args});\n}}\n",
                        name = name,
                        bare = bare_name,
                        params = cabi_params_str,
                        conv = conversions,
                        mod = module,
                        args = cabi_call_args,
                    ));
                }
            } else if exp_ret_is_js_value {
                // JsValue: extract .int for C ABI (i64)
                if exp.can_throw {
                    out.push_str(&format!(
                        "pub export fn {name}({params}) i64 {{\n{conv}{call_expr}\n    return _result.int;\n}}\n",
                        name = name,
                        params = cabi_params_str,
                        conv = conversions,
                        call_expr = call_expr,
                    ));
                } else {
                    out.push_str(&format!(
                        "pub export fn {name}({params}) i64 {{\n{conv}    const _result = {mod}.{bare}({args});\n    return _result.int;\n}}\n",
                        name = name,
                        bare = bare_name,
                        params = cabi_params_str,
                        conv = conversions,
                        mod = module,
                        args = cabi_call_args,
                    ));
                }
            } else {
                // Use type-appropriate zero value for void fallback
                let rz = if ret_zig == "bool" { "false" } else { "0" };
                if exp.can_throw {
                    out.push_str(&format!(
                        "pub export fn {name}({params}) {ret} {{\n{conv}{call_expr}\n    if (@TypeOf(_result) == void) {{\n        return {rz};\n    }} else {{\n        return _result;\n    }}\n}}\n",
                        name = name,
                        params = cabi_params_str,
                        conv = conversions,
                        ret = ret_zig,
                        call_expr = call_expr,
                        rz = rz,
                    ));
                } else {
                    out.push_str(&format!(
                        "pub export fn {name}({params}) {ret} {{\n{conv}    const _result = {mod}.{bare}({args});\n    if (@TypeOf(_result) == void) {{\n        return {rz};\n    }} else {{\n        return _result;\n    }}\n}}\n",
                        name = name,
                        bare = bare_name,
                        params = cabi_params_str,
                        ret = ret_zig,
                        conv = conversions,
                        mod = module,
                        args = cabi_call_args,
                        rz = rz,
                    ));
                }
            }
        }

        out.push('\n');
    }

    out
}

/// Write C ABI exports/imports JSON metadata.
///
/// `cabi_exports` is a list of (module_name, export) pairs.
/// `cabi_rename` maps disambiguated CABI names → bare function names.
/// When a name collides across modules, the JSON "name" field uses the
/// disambiguated form (`{fn}_{module}`) so that the bridge macro generates
/// unique Rust function definitions.
pub fn write_cabi_metadata(
    out_dir: &Path,
    project_name: &str,
    cabi_exports: &[(String, NativeCabiExport)],
    host_fns: &HostFnRegistry,
    include_init: bool,
    cabi_rename: &HashMap<String, String>,
) {
    let project_dir = out_dir.join(project_name);

    // cabi_exports.json — filter out exports with Anytype returns or params (no C ABI export generated)
    let exports_path = project_dir.join("cabi_exports.json");
    let mut exports_value: Vec<serde_json::Value> = cabi_exports
        .iter()
        .filter(|(_, exp)| {
            exp.ret_type != ZigType::Anytype
                && !exp.params.iter().any(|(_, ty)| *ty == ZigType::Anytype)
        })
        .map(|(mod_name, exp)| {
            // Build params list
            let params: Vec<serde_json::Value> = exp
                .params
                .iter()
                .map(|(name, ty)| {
                    serde_json::json!({
                        "name": name,
                        "zig_type": ty.to_zig_type() // JSON metadata is consumed by macro (lib.zig)
                    })
                })
                .collect();

            // Determine ret_type string for C ABI
            let ret_type_str = exp.ret_type.to_cabi_str();

            // For NamedStruct returns, look up struct fields from host_fns
            let (ret_struct_name, ret_struct_fields) =
                if let ZigType::NamedStruct(ref struct_name) = exp.ret_type {
                    // Look up the struct definition from host_fns
                    let struct_fields: Option<Vec<serde_json::Value>> = host_fns
                        .structs
                        .iter()
                        .find(|s| &s.zig_name == struct_name)
                        .map(|s| {
                            s.fields
                                .iter()
                                .map(|f| {
                                    serde_json::json!({
                                        "name": f.name,
                                        "zig_type": f.zig_type,
                                        "cabi_type": f.c_type,
                                    })
                                })
                                .collect()
                        });
                    (Some(struct_name.clone()), struct_fields)
                } else {
                    (None, None)
                };

            // Use disambiguated name if this export collides across modules
            let disambiguated = format!("{}_{}", exp.name, mod_name);
            let export_name = if cabi_rename.contains_key(&disambiguated) {
                disambiguated
            } else {
                exp.name.clone()
            };

            let mut json_obj = serde_json::json!({
                "name": export_name,
                "params": params,
                "ret_type": ret_type_str,
                "can_throw": exp.can_throw,
            });

            // Add struct info if returning a NamedStruct
            if let Some(sn) = ret_struct_name {
                json_obj["ret_struct_name"] = serde_json::json!(sn);
            }
            if let Some(sf) = ret_struct_fields {
                json_obj["ret_struct_fields"] = serde_json::json!(sf);
            }

            json_obj
        })
        .collect();

    // Deduplicate exports by name — when multiple JS files produce identically-named
    // C ABI exports, the bridge macro would generate duplicate Rust function definitions,
    // causing E0428 compilation errors. With collision disambiguation, duplicates are
    // resolved by the {fn}_{module} naming, but we still deduplicate as a safety net.
    {
        let mut seen = HashSet::new();
        exports_value.retain(|exp| {
            let name = exp["name"].as_str().unwrap_or("");
            seen.insert(name.to_string())
        });
    }

    // Only include js2rust_init and js2rust_deinit for non-test projects
    if include_init {
        exports_value.push(serde_json::json!({
            "name": "js2rust_init",
            "params": [],
            "ret_type": "void",
        }));
        exports_value.push(serde_json::json!({
            "name": "js2rust_deinit",
            "params": [],
            "ret_type": "void",
        }));
    }

    if let Ok(json_str) = serde_json::to_string_pretty(&exports_value) {
        let _ = fs::write(&exports_path, &json_str);
    }

    // cabi_imports.json
    let imports_path = project_dir.join("cabi_imports.json");
    let imports_value = host_fns.to_json_value();
    if let Ok(json_str) = serde_json::to_string_pretty(&imports_value) {
        let _ = fs::write(&imports_path, &json_str);
    }
}
