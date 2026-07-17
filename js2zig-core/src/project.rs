/// Generate a complete Zig library project from translated JS code.
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A single per-file Zig module with its dependency info.
#[derive(Debug, Clone)]
pub struct PerFileModule {
    /// Sanitized module name (e.g. "math", "string_utils", "main")
    pub mod_name: String,
    /// Translated Zig source code for this file
    pub zig_code: String,
    /// Dependency imports: Vec<(imported_name, source_module_name)>
    /// e.g. [("add", "math"), ("multiply", "math"), ("greet", "string_utils")]
    pub dep_imports: Vec<(String, String)>,
}

/// Project generation options
pub struct ProjectOptions {
    /// Name of the Zig library (also the output directory name)
    pub name: String,
    /// Output directory (e.g. "out")
    pub out_dir: PathBuf,
    /// Per-file Zig modules. Each entry is written as `{mod_name}.zig`, and lib.zig
    /// becomes a thin orchestrator that imports all per-file modules.
    pub per_file_code: Vec<PerFileModule>,
    /// Export names that should be re-exported from lib.zig.
    /// Each entry: (cabi_name, source_module_name, bare_fn_name).
    /// In multi-file mode, this should include ALL public exports from ALL modules.
    /// `cabi_name` is the disambiguated public name (may be `{fn}_{module}` on collision).
    /// `bare_fn_name` is the original function name inside the per-file module.
    pub external_exports: Vec<(String, String, String)>,
    /// Pre-generated `pub export fn` wrapper code for C ABI exports.
    /// Each wrapper calls a per-file module function and lives in the root lib.zig
    /// so that Zig propagates the symbols into the final .lib.
    pub cabi_wrapper_code: String,
    /// Names that have C ABI wrappers — these are skipped in the re-export section
    /// to avoid duplicate struct member errors.
    pub cabi_names: HashSet<String>,
    /// Auto-generated test code (from testgen)
    pub test_code: String,
    /// Runtime source directory (relative to project root, e.g. "runtime")
    pub runtime_dir: Option<PathBuf>,
    /// Host function extern "c" declarations (from host::HostFnRegistry)
    pub host_header: String,
    /// Names of async host functions (used to generate aliases in per-file modules)
    pub async_host_fn_names: Vec<String>,
    /// Whether the transpiled JS code uses RegExp/regex features
    pub needs_regex: bool,
    /// Whether the transpiled JS code uses ICU-dependent features
    /// (localeCompare, normalize, toLocaleUpperCase, toLocaleLowerCase).
    /// When true, js_string_icu.zig is overwritten with the ICU4X version
    /// and host_icu_stubs.zig is generated for zig test.
    pub needs_icu: bool,
}

/// Generate the full Zig library project.
pub fn generate(opts: &ProjectOptions) -> Result<(), String> {
    let project_dir = opts.out_dir.join(&opts.name);
    let src_dir = project_dir.join("src");

    // Create directories
    fs::create_dir_all(&src_dir).map_err(|e| format!("create {}: {}", project_dir.display(), e))?;

    // 1. build.zig.zon - first pass without fingerprint
    let zon_path = project_dir.join("build.zig.zon");
    let zon_no_fp = generate_zon(&opts.name, None);
    fs::write(&zon_path, &zon_no_fp).map_err(|e| format!("write build.zig.zon: {}", e))?;

    // 2. build.zig
    let build_zig = generate_build_zig(&opts.name, opts.needs_regex, opts.needs_icu);
    fs::write(project_dir.join("build.zig"), build_zig)
        .map_err(|e| format!("write build.zig: {}", e))?;

    // 3. Write per-file .zig files + orchestrator lib.zig
    for module in &opts.per_file_code {
        let mod_zig = generate_module_zig(module, &opts.async_host_fn_names, opts.needs_regex);
        let mod_path = src_dir.join(format!("{}.zig", module.mod_name));
        fs::write(&mod_path, mod_zig)
            .map_err(|e| format!("write {}.zig: {}", mod_path.display(), e))?;
    }

    // Generate orchestrator lib.zig
    let lib_zig = generate_orchestrator_lib(opts);
    fs::write(src_dir.join("lib.zig"), lib_zig).map_err(|e| format!("write lib.zig: {}", e))?;

    // 3.5 src/host.zig — user-defined host function extern "c" declarations.
    // Written when host_header is non-empty (i.e. the user registered custom host fns).
    if !opts.host_header.is_empty() {
        fs::write(src_dir.join("host.zig"), &opts.host_header)
            .map_err(|e| format!("write host.zig: {}", e))?;
    }

    // 3.5b src/host_regex.zig — regex host function extern "c" declarations.
    // Only generated when the transpiled JS code uses RegExp/regex features.
    if opts.needs_regex {
        let host_regex_content = r#"// Auto-generated host_regex.zig — regex host function declarations
// These symbols are defined in Rust with #[no_mangle] pub extern "C".
pub extern fn host_regex_test(
    pattern_ptr: [*]const u8, pattern_len: usize,
    subject_ptr: [*]const u8, subject_len: usize,
) callconv(.c) bool;
pub extern fn host_regex_search(
    pattern_ptr: [*]const u8, pattern_len: usize,
    subject_ptr: [*]const u8, subject_len: usize,
) callconv(.c) i64;

// Wrapper functions: the Emitter generates host_regex.regex_test(pattern, subject) (dot
// notation, 2 args as []const u8 slices), but extern declarations use
// host_regex_test with ptr+len pairs. These wrappers bridge the gap.
pub fn regex_test(pattern: []const u8, subject: []const u8) bool {
    return host_regex_test(pattern.ptr, pattern.len, subject.ptr, subject.len);
}
pub fn regex_search(pattern: []const u8, subject: []const u8) i64 {
    return host_regex_search(pattern.ptr, pattern.len, subject.ptr, subject.len);
}
"#;
        fs::write(src_dir.join("host_regex.zig"), host_regex_content)
            .map_err(|e| format!("write host_regex.zig: {}", e))?;
    }

    // 3.6 src/host_regex_stubs.zig — stub host regex C ABI implementations
    // for zig test (real implementations are in js2rust-bridge, linked by Rust).
    // Only generated when regex features are used.
    if opts.needs_regex {
        let stub_content = generate_host_regex_stubs();
        fs::write(src_dir.join("host_regex_stubs.zig"), stub_content)
            .map_err(|e| format!("write host_regex_stubs.zig: {}", e))?;
    }

    // 3.7 src/host_icu_stubs.zig — stub host ICU C ABI implementations
    // for zig test (real implementations are in js2rust-bridge, linked by Rust).
    // Only generated when ICU features are auto-detected in the transpiled code.
    if opts.needs_icu {
        let stub_content = generate_host_icu_stubs();
        fs::write(src_dir.join("host_icu_stubs.zig"), stub_content)
            .map_err(|e| format!("write host_icu_stubs.zig: {}", e))?;
    }

    // 4. Copy runtime/ if it exists (always overwrite to pick up runtime changes)
    if let Some(ref rt_dir) = opts.runtime_dir
        && rt_dir.exists()
        && rt_dir.is_dir()
    {
        let rt_dst = src_dir.join("js_runtime");
        // Always re-copy runtime files to pick up changes (e.g. js_console.zig updates)
        let _ = fs::remove_dir_all(&rt_dst);
        if let Err(e) = copy_dir_recursive(rt_dir, &rt_dst) {
            // If copy fails (e.g. concurrent process has the file), check if
            // another process already completed the copy.
            if !rt_dst.join("js_runtime.zig").exists() {
                return Err(format!("copy {}: {}", rt_dst.display(), e));
            }
        }

        // 4.1 When needs_icu=true (auto-detected), overwrite js_string_icu.zig with the
        // ICU4X-based version (extern fn declarations + host function calls).
        // The simplified version (copied from runtime/) is used when needs_icu=false.
        // The Cargo.toml `icu` feature on js2rust-bridge ensures the Rust-side
        // host_icu_* C ABI symbols are available for linking.
        if opts.needs_icu {
            let icu_runtime = generate_js_string_icu_icu();
            fs::write(rt_dst.join("js_string_icu.zig"), icu_runtime)
                .map_err(|e| format!("write js_string_icu.zig: {}", e))?;
        }
    }

    // 5. Auto-compute fingerprint via zig build
    if let Some(fp) = compute_fingerprint(&project_dir) {
        let zon_with_fp = generate_zon(&opts.name, Some(&fp));
        fs::write(&zon_path, zon_with_fp)
            .map_err(|e| format!("write build.zig.zon (with fingerprint): {}", e))?;
    }

    Ok(())
}

/// Run `zig build` in the project directory to get the suggested fingerprint.
fn compute_fingerprint(project_dir: &Path) -> Option<String> {
    let output = Command::new("zig")
        .arg("build")
        .current_dir(project_dir)
        .output()
        .ok()?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Parse: "suggested value: 0x8f0f7e479be9961b"
    let prefix = "suggested value: ";
    if let Some(pos) = stderr.find(prefix) {
        let after = &stderr[pos + prefix.len()..];
        let fp = after.split_whitespace().next()?;
        // Validate it looks like 0xHEX (length varies: 16–18 chars)
        if fp.starts_with("0x") && fp.len() >= 16 && fp[2..].chars().all(|c| c.is_ascii_hexdigit())
        {
            return Some(fp.to_string());
        }
    }
    None
}

fn generate_zon(name: &str, fingerprint: Option<&str>) -> String {
    let fp_line = match fingerprint {
        Some(fp) => format!("    .fingerprint = {},\n", fp),
        None => String::new(),
    };
    format!(
        r#".{{
    .name = .{name},
{fp_line}    .version = "0.1.0",
    .minimum_zig_version = "0.16.0",
    .paths = .{{"src", "build.zig", "build.zig.zon"}},
    .dependencies = .{{}},
}}
"#,
        name = name,
        fp_line = fp_line
    )
}

/// Generate `host_regex_stubs.zig` — stub implementations of the host C ABI
/// regex functions so that `zig build test` can link a standalone test binary.
/// The real implementations live in `js2rust-bridge` (native_regex.rs) and are
/// linked when Rust drives the final executable build.
pub fn generate_host_regex_stubs() -> String {
    r#"//! Stub implementations of host C ABI regex functions for zig test.
//! These provide linkable symbols so that zig test can produce a standalone
//! test binary without the Rust-side implementations.  The real implementations
//! live in js2rust-bridge (native_regex.rs) and are linked when Rust drives
//! the final executable build.  These stubs are only used by `zig build test`.

const JsStr = extern struct { ptr: [*]const u8, len: isize };

export fn host_regex_test(
    pattern_ptr: [*]const u8,
    pattern_len: usize,
    text_ptr: [*]const u8,
    text_len: usize,
) callconv(.c) bool {
    _ = pattern_ptr;
    _ = pattern_len;
    _ = text_ptr;
    _ = text_len;
    return false;
}

export fn host_regex_search(
    pattern_ptr: [*]const u8,
    pattern_len: usize,
    text_ptr: [*]const u8,
    text_len: usize,
) callconv(.c) i64 {
    _ = pattern_ptr;
    _ = pattern_len;
    _ = text_ptr;
    _ = text_len;
    return -1;
}

export fn host_regex_match(
    pattern_ptr: [*]const u8,
    pattern_len: usize,
    text_ptr: [*]const u8,
    text_len: usize,
    out_count: *usize,
) callconv(.c) JsStr {
    _ = pattern_ptr;
    _ = pattern_len;
    _ = text_ptr;
    _ = text_len;
    out_count.* = 0;
    return .{ .ptr = undefined, .len = 0 };
}

export fn host_regex_match_global(
    pattern_ptr: [*]const u8,
    pattern_len: usize,
    text_ptr: [*]const u8,
    text_len: usize,
    out_count: *usize,
) callconv(.c) JsStr {
    _ = pattern_ptr;
    _ = pattern_len;
    _ = text_ptr;
    _ = text_len;
    out_count.* = 0;
    return .{ .ptr = undefined, .len = 0 };
}

export fn host_regex_match_all(
    pattern_ptr: [*]const u8,
    pattern_len: usize,
    text_ptr: [*]const u8,
    text_len: usize,
    out_match_count: *usize,
    out_group_count: *usize,
) callconv(.c) JsStr {
    _ = pattern_ptr;
    _ = pattern_len;
    _ = text_ptr;
    _ = text_len;
    out_match_count.* = 0;
    out_group_count.* = 0;
    return .{ .ptr = undefined, .len = 0 };
}
"#
    .to_string()
}

pub fn generate_build_zig(lib_name: &str, needs_regex: bool, needs_icu: bool) -> String {
    let regex_stub_section = if needs_regex {
        r#"
    // Stub library: provides host_regex_* C ABI symbols for zig test.
    // When Rust drives the final executable build it links its own real
    // implementations (js2rust-bridge native_regex.rs).  The stubs only
    // exist so `zig build test` can produce a standalone test binary.
    const stub_mod = b.createModule(.{
        .root_source_file = b.path("src/host_regex_stubs.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    const stub_lib = b.addLibrary(.{
        .name = "host_regex_stubs",
        .linkage = .static,
        .root_module = stub_mod,
    });
"#
        .to_string()
    } else {
        String::new()
    };

    let icu_stub_section = if needs_icu {
        r#"
    // Stub library: provides host_icu_* C ABI symbols for zig test.
    // When Rust drives the final executable build it links its own real
    // implementations (js2rust-bridge native_icu.rs).  The stubs only
    // exist so `zig build test` can produce a standalone test binary.
    const icu_stub_mod = b.createModule(.{
        .root_source_file = b.path("src/host_icu_stubs.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    const icu_stub_lib = b.addLibrary(.{
        .name = "host_icu_stubs",
        .linkage = .static,
        .root_module = icu_stub_mod,
    });
"#
        .to_string()
    } else {
        String::new()
    };

    let test_link_stub = if needs_regex {
        "    test_mod.linkLibrary(stub_lib);\n"
    } else {
        ""
    };

    let test_link_icu_stub = if needs_icu {
        "    test_mod.linkLibrary(icu_stub_lib);\n"
    } else {
        ""
    };

    format!(
        r#"const std = @import("std");

pub fn build(b: *std.Build) void {{
    const target = b.standardTargetOptions(.{{}});
    const optimize = b.standardOptimizeOption(.{{}});

    const lib_mod = b.createModule(.{{
        .root_source_file = b.path("src/lib.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    }});

    // Build as static library.
    // bundle_compiler_rt = true: includes compiler-rt symbols in the .lib.
    // MSVC link.exe cannot parse COMDAT sections in compiler_rt.obj (LNK1143),
    // so users MUST use rust-lld as the linker:
    //   .cargo/config.toml: [target.x86_64-pc-windows-msvc] linker = "rust-lld.exe"
    const lib = b.addLibrary(.{{
        .name = "{name}",
        .linkage = .static,
        .root_module = lib_mod,
    }});
    lib.bundle_compiler_rt = true;
    b.installArtifact(lib);
{regex_stub_section}{icu_stub_section}
    // Test step
    // Create a test-only module that links the stub libraries, so the stub
    // symbols (host_regex_*, host_icu_*) are available during `zig build test` but
    // do NOT pollute the main library artifact linked by Rust.
    const test_mod = b.createModule(.{{
        .root_source_file = b.path("src/lib.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    }});
{test_link_stub}{test_link_icu_stub}    const tests = b.addTest(.{{
        .root_module = test_mod,
    }});
    const run_tests = b.addRunArtifact(tests);
    const test_step = b.step("test", "Run all library tests");
    test_step.dependOn(&run_tests.step);
}}
"#,
        name = lib_name
    )
}

/// Emit the standard runtime @import preamble (std, allocator, tier-3 runtime libs).
fn push_runtime_imports(out: &mut String, needs_regex: bool) {
    out.push_str("const std = @import(\"std\");\n");
    out.push_str("const builtin = @import(\"builtin\");\n");
    out.push_str("const Io = std.Io;\n");
    out.push_str("const Allocator = std.mem.Allocator;\n");
    out.push('\n');
    out.push_str("// Global allocator for generated code\n");
    out.push_str("const js_allocator = @import(\"js_runtime/js_allocator.zig\");\n");
    out.push_str("const StrRet = @import(\"js_runtime/string.zig\").StrRet;\n");
    out.push('\n');
    out.push_str("// Tier-3 runtime library imports\n");
    out.push_str("const js_string = @import(\"js_runtime/js_string.zig\");\n");
    out.push_str("const js_string_icu = @import(\"js_runtime/js_string_icu.zig\");\n");
    out.push_str("const js_console = @import(\"js_runtime/js_console.zig\");\n");
    out.push_str("const js_json = @import(\"js_runtime/js_json.zig\");\n");
    out.push_str("const js_array = @import(\"js_runtime/js_array.zig\");\n");
    out.push_str("const js_object = @import(\"js_runtime/js_object.zig\");\n");
    out.push_str("const js_number = @import(\"js_runtime/js_number.zig\");\n");
    out.push_str("const js_date = @import(\"js_runtime/js_date.zig\");\n");
    out.push_str("const js_error = @import(\"js_runtime/js_error.zig\");\n");
    out.push_str("const js_collections = @import(\"js_runtime/js_collections.zig\");\n");
    if needs_regex {
        out.push_str("const js_regexp = @import(\"js_runtime/js_regexp.zig\");\n");
        out.push_str("const js_string_regex = @import(\"js_runtime/js_string_regex.zig\");\n");
    }
    out.push_str("const js_uri = @import(\"js_runtime/js_uri.zig\");\n");
    out.push_str("const js_symbol = @import(\"js_runtime/js_symbol.zig\");\n");
    out.push_str("const JsSymbol = @import(\"js_runtime/js_symbol.zig\").JsSymbol;\n");
    out.push_str("const js_bigint = @import(\"js_runtime/js_bigint.zig\");\n");
    out.push_str("const JsValue = @import(\"js_runtime/jsvalue.zig\").JsValue;\n");
    out.push_str("const JsAny = @import(\"js_runtime/jsany.zig\").JsAny;\n");
    out.push_str("const js_runtime = @import(\"js_runtime/js_runtime.zig\");\n");
    out.push('\n');
}

/// Generate a per-file Zig module with dependency imports and full runtime imports.
///
/// Produces:
///   // Auto-generated from {mod_name}.js
///   const std = ...
///   const js_allocator = ...
///   // --- Dependency module imports ---
///   const math = @import("math.zig");
///   const string_utils = @import("string_utils.zig");
///   // --- Imported name aliases ---
///   const add = math.add;
///   ...
///   // Generated code
///   export fn add(...) ...
fn generate_module_zig(
    module: &PerFileModule,
    async_host_fn_names: &[String],
    needs_regex: bool,
) -> String {
    let mut out = String::new();
    out.push_str(&format!("// Auto-generated from {}.js\n", module.mod_name));
    push_runtime_imports(&mut out, needs_regex);

    // Host function imports (if this module calls host functions or async fns exist)
    let needs_host = module.zig_code.contains("host.") || !async_host_fn_names.is_empty();
    let needs_host_regex = needs_regex && module.zig_code.contains("host_regex.");
    if needs_host || needs_host_regex {
        out.push_str("// Host functions (Rust via C ABI)\n");
        if needs_host {
            out.push_str("const host = @import(\"host.zig\");\n");
        }
        if needs_host_regex {
            out.push_str("const host_regex = @import(\"host_regex.zig\");\n");
        }
        // Async function aliases: lets generated code call host.{name}_async as {name}
        for name in async_host_fn_names {
            out.push_str(&format!("const {} = host.{}_async;\n", name, name));
        }
        out.push('\n');
    }

    // Deduplicate dependency modules
    let mut dep_modules: Vec<String> = Vec::new();
    let mut seen_modules: HashSet<&str> = HashSet::new();
    for (_, src_mod) in &module.dep_imports {
        if seen_modules.insert(src_mod) {
            dep_modules.push(src_mod.clone());
        }
    }

    if !dep_modules.is_empty() {
        out.push_str("// --- Dependency module imports ---\n");
        for mod_name in &dep_modules {
            out.push_str(&format!(
                "const {} = @import(\"{}.zig\");\n",
                mod_name, mod_name
            ));
        }
        out.push('\n');

        out.push_str("// --- Imported name aliases ---\n");
        for (name, src_mod) in &module.dep_imports {
            out.push_str(&format!("const {name} = {mod}.{name};\n", name = name, mod = src_mod));
        }
        out.push('\n');
    }

    // NOTE: No Windows LdrRegisterDllNotification stub here — it's in the orchestrator lib.zig.
    // Per-file modules are only compiled as part of the orchestrator, so the stub is inherited.

    out.push_str(&module.zig_code);
    out
}

/// Generate the orchestrator lib.zig for multi-file mode.
/// Imports all per-file modules, re-exports ALL public exports from all modules,
/// includes test blocks.
fn generate_orchestrator_lib(opts: &ProjectOptions) -> String {
    let mut out = String::new();
    out.push_str("// Auto-generated by js2rust (orchestrator)\n");
    push_runtime_imports(&mut out, opts.needs_regex);

    // Host function declarations
    if !opts.host_header.is_empty() || !opts.async_host_fn_names.is_empty() {
        out.push_str("// User-defined host functions (Rust via C ABI)\n");
        out.push_str("const host = @import(\"host.zig\");\n");
        out.push('\n');
    }
    if opts.needs_regex {
        out.push_str("// Host regex functions (Rust via C ABI)\n");
        out.push_str("const host_regex = @import(\"host_regex.zig\");\n");
        out.push('\n');
    }

    // Windows stub (needed for Zig std.debug on Windows)
    // Uses platform-independent types so it compiles on all targets (native, WASM).
    // Always included because Zig's standard library references this symbol and
    // Windows COFF doesn't support weak linkage across object files.
    out.push_str("// Windows stub for Zig std.debug.SelfInfo.Windows\n");
    out.push_str("// Uses raw types (u32/i32) to avoid importing std.os.windows on non-Windows.\n");
    out.push_str("pub export fn LdrRegisterDllNotification(\n");
    out.push_str("    Flags: u32,\n");
    out.push_str("    NotificationFunction: ?*const anyopaque,\n");
    out.push_str("    Context: ?*anyopaque,\n");
    out.push_str("    Cookie: *?*anyopaque,\n");
    out.push_str(") callconv(.c) i32 {\n");
    out.push_str("    _ = Flags;\n");
    out.push_str("    _ = NotificationFunction;\n");
    out.push_str("    _ = Context;\n");
    out.push_str("    Cookie.* = @ptrFromInt(1);\n");
    out.push_str("    return 0; // STATUS_SUCCESS\n");
    out.push_str("}\n\n");

    // Async host function aliases (if any) — lets orchestrator call async wrappers
    if !opts.async_host_fn_names.is_empty() {
        out.push_str("// Async host function aliases\n");
        for name in &opts.async_host_fn_names {
            out.push_str(&format!("const {} = host.{}_async;\n", name, name));
        }
        out.push('\n');
    }

    // Per-file module imports (prefixed with _ to avoid duplicate name errors
    // when re-exporting pub const {name} = _{name}.{name}).
    out.push_str("// Per-file module imports\n");
    for module in &opts.per_file_code {
        out.push_str(&format!(
            "const _{} = @import(\"{}.zig\");\n",
            module.mod_name, module.mod_name
        ));
    }
    out.push('\n');

    // --- Global initialization / deinitialization ---
    out.push_str("/// Initialize the global allocator used by all generated functions.\n");
    out.push_str("/// The allocator is created internally using ArenaAllocator (Zig 0.16.0: lock-free, thread-safe).\n");
    out.push_str("pub fn init_js2rust() !void {\n");
    out.push_str("    try js_allocator.init(null, null);\n");
    out.push_str("    js_runtime.initIo(js_allocator.allocator());\n");
    // Also call init_js2rust on each per-file module that defines its own
    for module in &opts.per_file_code {
        if module.zig_code.contains("pub fn init_js2rust") {
            out.push_str(&format!("    try _{}.init_js2rust();\n", module.mod_name));
        }
    }
    out.push_str("}\n\n");
    // C ABI compatible init/deinit (callable from Rust via FFI)
    out.push_str("/// Initialize with ArenaAllocator (lock-free, thread-safe).\n");
    out.push_str("/// Call this from Rust via C ABI before using any function that allocates.\n");
    out.push_str("pub export fn js2rust_init() void {\n");
    out.push_str("    init_js2rust() catch @panic(\"init_js2rust failed\");\n");
    out.push_str("}\n\n");
    out.push_str("/// Release global resources. Call this when done.\n");
    out.push_str("pub export fn js2rust_deinit() void {\n");
    out.push_str("    deinit_js2rust();\n");
    out.push_str("}\n\n");
    out.push_str(
        "/// Allocate memory in Zig's Arena for zero-copy string returns from Rust host functions.\n",
    );
    out.push_str(
        "/// Called from Rust via extern \"C\" { fn js_allocator_alloc(size: usize) -> ?*mut u8; }\n",
    );
    out.push_str("/// Memory is managed by the multi-arena allocator — no free needed.\n");
    out.push_str("/// Returns null on OOM instead of panicking.\n");
    out.push_str("pub export fn js_allocator_alloc(size: usize) ?[*]u8 {\n");
    out.push_str("    const buf = js_allocator.allocBytes(size) catch return null;\n");
    out.push_str("    return buf.ptr;\n");
    out.push_str("}\n\n");
    out.push_str("/// Allocate + copy in Zig Arena — single call for zero-copy string returns.\n");
    out.push_str(
        "/// Called from Rust via extern \"C\" { fn js_allocator_dupe(src: *const u8, len: usize) -> ?*mut u8; }\n",
    );
    out.push_str("/// Prefer this over js_allocator_alloc + manual copy — avoids a separate memcpy in Rust.\n");
    out.push_str("/// Returns null on OOM instead of panicking.\n");
    out.push_str("pub export fn js_allocator_dupe(src: [*]const u8, len: usize) ?[*]u8 {\n");
    out.push_str("    const buf = js_allocator.dupeBytes(src[0..len]) catch return null;\n");
    out.push_str("    return buf.ptr;\n");
    out.push_str("}\n\n");
    out.push_str("/// Release global resources allocated via init_js2rust.\n");
    out.push_str("pub fn deinit_js2rust() void {\n");
    out.push_str("    js_runtime.deinitIo();\n");
    for module in &opts.per_file_code {
        if module.zig_code.contains("pub fn deinit_js2rust") {
            out.push_str(&format!("    _{}.deinit_js2rust();\n", module.mod_name));
        }
    }
    out.push_str("    js_allocator.deinit();\n");
    out.push_str("}\n\n");

    // Build a set of functions that return C ABI strings ([*:0]const u8),
    // so we can emit adapter wrappers instead of bare `pub const X = mod.X;`.
    let cabi_string_fns = collect_cabi_string_fns(&opts.per_file_code);

    // Re-export ALL public exports from ALL modules so tests can reference them
    if !opts.external_exports.is_empty() {
        out.push_str("// Re-export all public exports from per-file modules\n");
        let mut exported: HashSet<&str> = HashSet::new();

        for (cabi_name, mod_name, bare_name) in &opts.external_exports {
            if !exported.insert(cabi_name.as_str()) {
                continue;
            }
            // Skip names that already have C ABI pub export fn wrappers
            // (they'd cause duplicate struct member errors in Zig).
            if opts.cabi_names.contains(cabi_name.as_str()) {
                continue;
            }
            if cabi_string_fns.contains(bare_name.as_str()) {
                // C ABI string-returning function: generate adapter
                // The public name is cabi_name (possibly disambiguated),
                // but the internal call uses bare_name (original fn name in module).
                out.push_str(&format!(
                    "pub fn {cabi}(s: []const u8) []const u8 {{\n    return std.mem.sliceTo(_{mod}.{bare}(@ptrCast(s.ptr)), 0);\n}}\n",
                    cabi = cabi_name,
                    mod = mod_name,
                    bare = bare_name,
                ));
            } else {
                out.push_str(&format!(
                    "pub const {cabi} = _{mod}.{bare};\n",
                    cabi = cabi_name,
                    mod = mod_name,
                    bare = bare_name,
                ));
            }
        }
        out.push('\n');
    }

    // C ABI exports: pub export fn wrappers in root lib.zig calling per-file modules.
    // Zig only propagates root-module pub export into the final .lib binary.
    if !opts.cabi_wrapper_code.is_empty() {
        out.push_str("// C ABI exports (pub export fn wrappers)\n");
        out.push_str(&opts.cabi_wrapper_code);
        out.push('\n');
    }

    // Append test blocks
    if !opts.test_code.trim().is_empty() {
        out.push_str(&opts.test_code);
        out.push('\n');
    }

    out
}

/// Scan per-file modules' zig_code for functions that return C ABI strings
/// (signatures containing `[*:0]const u8`)
fn collect_cabi_string_fns(per_file_code: &[PerFileModule]) -> HashSet<String> {
    let mut fns = HashSet::new();
    for module in per_file_code {
        for line in module.zig_code.lines() {
            let trimmed = line.trim();
            // Match: pub export fn greet(name: ...) StrRet {
            if (trimmed.starts_with("pub export fn ") || trimmed.starts_with("pub fn "))
                && trimmed.contains("StrRet")
                && let Some(open_paren) = trimmed.find('(')
            {
                // Extract function name between `fn ` and `(`
                let prefix_end = if trimmed.starts_with("pub export fn ") {
                    "pub export fn ".len()
                } else {
                    "pub fn ".len()
                };
                let name = trimmed[prefix_end..open_paren].trim();
                fns.insert(name.to_string());
            }
        }
    }
    fns
}

/// Recursively copy a directory
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| format!("create dir {}: {}", dst.display(), e))?;

    for entry in fs::read_dir(src).map_err(|e| format!("read dir {}: {}", src.display(), e))? {
        let entry = entry.map_err(|e| format!("read entry: {}", e))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else if src_path.is_file() {
            fs::copy(&src_path, &dst_path)
                .map_err(|e| format!("copy {}: {}", src_path.display(), e))?;
        }
    }

    Ok(())
}

/// Generate `host_icu_stubs.zig` — stub implementations of the host C ABI
/// ICU functions so that `zig build test` can link a standalone test binary.
/// The real implementations live in `js2rust-bridge` (native_icu.rs) and are
/// linked when Rust drives the final executable build.
pub fn generate_host_icu_stubs() -> String {
    r#"//! Stub implementations of host C ABI ICU functions for zig test.
//! These provide linkable symbols so that zig test can produce a standalone
//! test binary without the Rust-side implementations.  The real implementations
//! live in js2rust-bridge (native_icu.rs) and are linked when Rust drives
//! the final executable build.  These stubs are only used by `zig build test`.

const JsStr = extern struct { ptr: [*]const u8, len: isize };

export fn host_icu_locale_compare(
    a_ptr: [*]const u8,
    a_len: usize,
    b_ptr: [*]const u8,
    b_len: usize,
) callconv(.c) i64 {
    _ = a_ptr;
    _ = a_len;
    _ = b_ptr;
    _ = b_len;
    return 0;
}

export fn host_icu_normalize(
    input_ptr: [*]const u8,
    input_len: usize,
    form_ptr: [*]const u8,
    form_len: usize,
) callconv(.c) JsStr {
    _ = input_ptr;
    _ = input_len;
    _ = form_ptr;
    _ = form_len;
    return .{ .ptr = undefined, .len = 0 };
}

export fn host_icu_to_locale_upper_case(
    input_ptr: [*]const u8,
    input_len: usize,
) callconv(.c) JsStr {
    _ = input_ptr;
    _ = input_len;
    return .{ .ptr = undefined, .len = 0 };
}

export fn host_icu_to_locale_lower_case(
    input_ptr: [*]const u8,
    input_len: usize,
) callconv(.c) JsStr {
    _ = input_ptr;
    _ = input_len;
    return .{ .ptr = undefined, .len = 0 };
}
"#
    .to_string()
}

/// Generate the ICU4X-based `js_string_icu.zig` that uses host_icu_* C ABI
/// functions for proper locale-sensitive string operations.
/// This version overwrites the simplified runtime version when needs_icu=true.
fn generate_js_string_icu_icu() -> String {
    r#"//! JS String ICU-dependent method implementations for Zig (ICU4X version).
//! These functions rely on host_icu_* C ABI symbols provided by
//! js2rust-bridge (native_icu.rs) at link time.
//!
//! This file overwrites the simplified runtime version when needs_icu = true.

const std = @import("std");
const Allocator = std.mem.Allocator;
const JsAny = @import("jsany.zig").JsAny;

// ── Host C ABI function declarations ──

extern fn host_icu_locale_compare(
    a_ptr: [*]const u8,
    a_len: usize,
    b_ptr: [*]const u8,
    b_len: usize,
) callconv(.c) i64;

extern fn host_icu_normalize(
    input_ptr: [*]const u8,
    input_len: usize,
    form_ptr: [*]const u8,
    form_len: usize,
) callconv(.c) extern struct { ptr: [*]const u8, len: isize };

extern fn host_icu_to_locale_upper_case(
    input_ptr: [*]const u8,
    input_len: usize,
) callconv(.c) extern struct { ptr: [*]const u8, len: isize };

extern fn host_icu_to_locale_lower_case(
    input_ptr: [*]const u8,
    input_len: usize,
) callconv(.c) extern struct { ptr: [*]const u8, len: isize };

/// Locale-sensitive string comparison via ICU4X Collator.
/// Returns -1 if self < other, 0 if equal, 1 if self > other.
pub fn localeCompare(a: []const u8, b: []const u8) i64 {
    return host_icu_locale_compare(a.ptr, a.len, b.ptr, b.len);
}

/// Normalize Unicode string using ICU4X Normalizer.
/// Supports NFC (default), NFD, NFKC, NFKD normalization forms.
pub fn normalize(alloc: Allocator, s: []const u8, form: []const u8) ![]const u8 {
    const result = host_icu_normalize(s.ptr, s.len, form.ptr, form.len);
    if (result.len == 0) return &[0]u8{};
    const bytes = result.ptr[0..@intCast(result.len)];
    return try alloc.dupe(u8, bytes);
}

/// Convert string to locale-specific uppercase via ICU4X CaseMapper.
pub fn toLocaleUpper(alloc: Allocator, s: []const u8) ![]const u8 {
    const result = host_icu_to_locale_upper_case(s.ptr, s.len);
    if (result.len == 0) return &[0]u8{};
    const bytes = result.ptr[0..@intCast(result.len)];
    return try alloc.dupe(u8, bytes);
}

/// Convert string to locale-specific lowercase via ICU4X CaseMapper.
pub fn toLocaleLower(alloc: Allocator, s: []const u8) ![]const u8 {
    const result = host_icu_to_locale_lower_case(s.ptr, s.len);
    if (result.len == 0) return &[0]u8{};
    const bytes = result.ptr[0..@intCast(result.len)];
    return try alloc.dupe(u8, bytes);
}
"#
    .to_string()
}
