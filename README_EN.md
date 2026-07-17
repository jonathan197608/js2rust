---
AIGC:
  ContentProducer: '001191110102MAD55U9H0F10002'
  ContentPropagator: '001191110102MAD55U9H0F10002'
  Label: '1'
  ProduceID: '717516a1-b890-4b3b-937f-1a4ade96d6ab'
  PropagateID: '717516a1-b890-4b3b-937f-1a4ade96d6ab'
  ReservedCode1: '6ab9056c-6127-4b2c-8a7f-248acb32dab7'
  ReservedCode2: '6ab9056c-6127-4b2c-8a7f-248acb32dab7'
---

---
AIGC:
  ContentProducer: '001191110102MAD55U9H0F10002'
  ContentPropagator: '001191110102MAD55U9H0F10002'
  Label: '1'
  ProduceID: '3c2dd8b2-0f87-4b51-a3cd-b41ff8984858'
  PropagateID: '3c2dd8b2-0f87-4b51-a3cd-b41ff8984858'
  ReservedCode1: '5343505b-f448-43bc-8bc1-3b518c839d18'
  ReservedCode2: '5343505b-f448-43bc-8bc1-3b518c839d18'
---

---
AIGC:
  ContentProducer: '001191110102MAD55U9H0F10002'
  ContentPropagator: '001191110102MAD55U9H0F10002'
  Label: '1'
  ProduceID: '1382e66d-519b-411f-8c6d-e25b392a232f'
  PropagateID: '1382e66d-519b-411f-8c6d-e25b392a232f'
  ReservedCode1: '85055633-41f1-4218-963b-3d17c17f53ca'
  ReservedCode2: '85055633-41f1-4218-963b-3d17c17f53ca'
---

---
AIGC:
  ContentProducer: '001191110102MAD55U9H0F10002'
  ContentPropagator: '001191110102MAD55U9H0F10002'
  Label: '1'
  ProduceID: '99ea6ed3-ffd6-482c-afc2-b02062d13360'
  PropagateID: '99ea6ed3-ffd6-482c-afc2-b02062d13360'
  ReservedCode1: '31ffc97e-ff08-47a6-a82a-ae4fdee898f2'
  ReservedCode2: '31ffc97e-ff08-47a6-a82a-ae4fdee898f2'
---

---
AIGC:
  ContentProducer: '001191110102MAD55U9H0F10002'
  ContentPropagator: '001191110102MAD55U9H0F10002'
  Label: '1'
  ProduceID: 'ea4d00d2-c6ff-42fb-8a89-8c51e4b1b4b2'
  PropagateID: 'ea4d00d2-c6ff-42fb-8a89-8c51e4b1b4b2'
  ReservedCode1: 'deef3230-8072-4dcc-8a9c-83c784b978e9'
  ReservedCode2: 'deef3230-8072-4dcc-8a9c-83c784b978e9'
---

# js2rust — JS-to-Zig Transpiler for Rust FFI

`js2rust` is a JS-to-Zig source-level transpiler that enables seamless integration of JavaScript code into Rust projects via automatic FFI bridge generation.

## Status

| Metric | Value |
|--------|-------|
| Rust tests | 506 (506 pass, 0 ignore) |
| Clippy warnings | 0 |
| MDN end-to-end tests | 236/237 (99.6% match, 1 WONTFIX mismatch, 0 error) |
| JS expression coverage | 82/91 (~90%) |
| JS statement coverage | 45/50 (~90%) |
| JS built-in coverage | 217/228 (~95%) |
| Crate versions | [js2zig-core 0.17.1](https://crates.io/crates/js2zig-core) · [js2rust-bridge 0.17.1](https://crates.io/crates/js2rust-bridge) · [js2rust-bridge-macro 0.17.1](https://crates.io/crates/js2rust-bridge-macro) |

> Detailed feature evaluation: [JS Language Feature Implementation Notes](docs/JS_FEATURE_EVALUATION.md) (Chinese).

## Features

- **JS-to-Zig transpilation**: Automatically converts JS source files to Zig code
- **Proc-macro FFI bridge**: `js2rust_bridge!()` transpiles and generates Rust FFI bindings in one step
- **Host functions**: Call Rust functions from JS via C ABI
  - Synchronous: `i64`, `f64`, `bool`, `str` parameters and return values
  - **Async**: `async fn` with struct return types, bridged via tokio
- **Async export functions**: `export async function` generates a C ABI blocking wrapper using a global Zig `Io` instance
- **String host functions**: Automatic `[*:0]const u8` ↔ `[]const u8` conversion with heap-allocated returns
- **Source Map**: `// @src(file:line)` inline comments + `source_map.json`
- **Incremental compilation**: Hash-based cache — unchanged files are skipped on rebuild (`--force` to override)
- **Multi-file project support**: Transpile entire JS project directories with DFS dependency ordering
- **Type inference**: Automatic JS type inference (number → i64/f64, string → `[]u8`, etc.)
- **Zero code generation**: Everything happens in the proc-macro — IDE-friendly

## Quick Start

### 1. Add dependencies to your `Cargo.toml`

```toml
[dependencies]
js2rust-bridge = "0.17"

[build-dependencies]
js2rust-bridge = "0.17"
```

### 2. Write JS code in `js_src/main.js`

```javascript
export function greet(name) {
    return "Hello, " + name + "!";
}

export function add(a, b) {
    return a + b;
}
```

### 3. Use the macro in `src/main.rs`

```rust
js2rust_bridge!("js_src/main.js");

fn main() {
    let result = greet_main("World");
    println!("{}", result); // "Hello, World!"

    let sum = add_main(1, 2);
    println!("1 + 2 = {}", sum); // 3
}
```

### 4. Add a minimal `build.rs` for linking

```rust
fn main() {
    js2rust_bridge::link();
}
```

## Host Functions

Call Rust functions from JS by declaring them in the macro:

### Synchronous host functions

```rust
js2rust_bridge! {
    "js_src/main.js",
    host_add(i64, i64) -> i64,
    host_concat(str, str) -> str,
}
```

Implement in Rust:

```rust
#[no_mangle]
pub extern "C" fn host_add(a: i64, b: i64) -> i64 { a + b }

#[no_mangle]
pub extern "C" fn host_concat(a: *const std::ffi::c_char, b: *const std::ffi::c_char) -> *mut std::ffi::c_char {
    let a = unsafe { std::ffi::CStr::from_ptr(a).to_string_lossy().into_owned() };
    let b = unsafe { std::ffi::CStr::from_ptr(b).to_string_lossy().into_owned() };
    std::ffi::CString::new(format!("{a}{b}")).unwrap().into_raw()
}
```

### Async host functions

Declare an async host function with a struct return type:

```rust
js2rust_bridge! {
    "js_src/main.js",
    async fetch_user(str) -> { id: i64, name: str },
}
```

Implement with tokio and bridge via `block_on`:

```rust
use tokio::runtime::Runtime;
use std::sync::OnceLock;

static RUNTIME: OnceLock<Runtime> = OnceLock::new();
fn runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to create tokio runtime")
    })
}

#[repr(C)]
pub struct HostFetchUserResult {
    pub id: i64,
    pub name: [u8; 256],
}

#[no_mangle]
pub extern "C" fn fetch_user(name: *const std::ffi::c_char) -> HostFetchUserResult {
    let name = unsafe { std::ffi::CStr::from_ptr(name).to_string_lossy() };
    runtime().block_on(fetch_user_from_db(&name))
}
```

Call from JS with `await`:

```js
export async function getUserInfo(name) {
    const user = await fetch_user(name);
    return user.name;
}
```

### Async export functions

`export async function` is exported via C ABI as a blocking wrapper:

```js
export async function getUserInfo(name) {
    const user = await fetch_user(name);
    return user.name;
}
```

Call from Rust as a regular synchronous function:

```rust
fn main() {
    js2rust_init();  // Initialize global Io (required for async exports)
    let name = getUserInfo_main("alice");
    println!("User: {}", name);
    js2rust_deinit();
}
```

## Architecture

```
js2rust/
├── js2zig-core/            # Core transpiler library (parser, type inference, codegen)
├── js2rust-bridge/         # Facade crate (re-exports the proc-macro + link helper)
├── js2rust-bridge-macro/   # Proc-macro: transpile + generate FFI bindings
├── runtime/                # Zig runtime (js_array, js_string, js_map, js_date, js_regexp, etc.)
├── native_proto/           # Code generator (expr → Zig, stmt → Zig, builtin calls)
└── examples/
    ├── test-bin-project/   # Binary project with sync + async host functions
    ├── test-lib-project/   # Library project
    ├── showcase-project/   # Multi-file demo
    └── mdn-test-project/   # MDN semantic conformance test suite (237 cases)
```

### How it works

1. `js2rust_bridge!("js_src/main.js")` macro calls `js2zig_core::transpile_project()`
2. The core JS file and its transitive imports are transpiled to Zig, output written to `.js2zig-cache/main/`
3. Macro reads `cabi_exports.json` and generates `unsafe extern "C"` + safe Rust wrappers
4. Async exports generate `_impl` async functions + C ABI blocking wrappers (using global `Io`)
5. Macro runs `zig build` to compile the static library
6. `build.rs` links the static library (scans `.js2zig-cache/`)
7. You call the generated safe wrapper functions (e.g., `greet_main()`, `getUserInfo_main()`)

### Async call chain

```
Rust: getUserInfo_main("alice")
  → Zig C ABI: getUserInfo_cabi(name)
    → Zig async: getUserInfo_impl(io, name)
      → Zig: io.async(fetch_user, .{ io, name })
        → Zig wrapper: fetch_user_async(io, name) → extern "c" fetch_user(name)
          → Rust: fetch_user(name) → tokio runtime block_on(async_fn)
```

## Requirements

- Rust 1.85+ (edition 2024)
- Zig 0.16.0+ (for compiling transpiled Zig code)

## Documentation

- [JS Language Feature Implementation Notes](docs/JS_FEATURE_EVALUATION.md) — Per-feature implementation status across 141 syntax features + 228 built-in method rows (Chinese)

## License

Dual-licensed under MIT or Apache-2.0.