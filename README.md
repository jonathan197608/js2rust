# js2rust — JS-to-Zig Transpiler for Rust FFI

`js2rust` is a JS-to-Zig transpiler that enables seamless integration of JavaScript code into Rust projects via automatic FFI bridge generation.

## Features

- **JS-to-Zig transpilation**: Automatically converts JS source files to Zig code
- **Proc-macro FFI bridge**: `js2rust_bridge!()` transpiles and generates Rust FFI bindings in one step
- **Host functions**: Call Rust functions from JS via C ABI
  - Synchronous: `i64`, `f64`, `bool`, `str` parameters and return values
  - **Async** (new in 0.2): `async fn` with struct return types, bridged via tokio
- **Async export functions** (new in 0.2): `export async function` generates a C ABI blocking wrapper using a global Zig `Io` instance
- **String host functions** (new in 0.2): Automatic `[*:0]const u8` ↔ `[]const u8` conversion with heap-allocated returns
- **Source Map** (new in 0.2): `// @src(file:line)` inline comments + `source_map.json`
- **Incremental compilation** (new in 0.2): Hash-based cache — unchanged files are skipped on rebuild (`--force` to override)
- **WASM target** (new in 0.2): `zig build wasm` (wasm32-wasi) support
- **Multi-file project support**: Transpile entire JS project directories
- **Type inference**: Automatic JS type inference (number -> i64/f64, string -> []u8, etc.)
- **No build.rs code generation**: Everything happens in the proc-macro — IDE-friendly

## Quick Start

### 1. Add dependencies to your `Cargo.toml`

```toml
[dependencies]
js2rust-bridge = "0.2"

[build-dependencies]
js2rust-bridge = "0.2"
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

### Async host functions (0.2)

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

async fn fetch_user_from_db(name: &str) -> User {
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    // ... database lookup ...
}

#[no_mangle]
pub extern "C" fn fetch_user(name: *const std::ffi::c_char) -> HostFetchUserResult {
    let name = unsafe { std::ffi::CStr::from_ptr(name).to_string_lossy() };
    let user = runtime().block_on(fetch_user_from_db(&name));
    // ... pack into HostFetchUserResult ...
}
```

Call from JS with `await`:

```js
async function getUserInfo(name) {
    const user = await fetch_user(name);
    return user.name;  // Access struct fields
}
```

### Async export functions (0.2)

`export async function` is exported via C ABI as a blocking wrapper. The transpiler generates a `getUserInfo_impl` async function and a `getUserInfo_cabi` wrapper that obtains a global `Io` instance and blocks on the async result:

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
    println!("User: {}", name);  // "Alice Smith"
    js2rust_deinit();
}
```

## Architecture

```
js2rust/
├── js2zig-core/            # Core transpiler library (parser, type inference, codegen)
├── js2rust-bridge/         # Facade crate (re-exports the proc-macro + link helper)
├── js2rust-bridge-macro/   # Proc-macro: transpile + generate FFI bindings
├── runtime/                # Zig runtime (js_runtime.zig, allocator, builtins)
└── examples/
    ├── test-bin-project/   # Binary project with sync + async host functions
    └── test-lib-project/   # Library project
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

## Changelog

### 0.2.0

- **Async host functions**: `async fn` with struct return types, tokio `block_on` bridge
- **Async export functions**: `export async function` generates C ABI blocking wrapper via global `Io`
- **String host functions**: Automatic C string ↔ Zig string conversion with heap-allocated returns
- **Source Map**: `// @src(file:line)` inline comments + `source_map.json`
- **Incremental compilation**: Hash-based build cache, `--force` flag for full rebuild
- **WASM target**: `zig build wasm` (wasm32-wasi) support
- Global `js_runtime.initIo()` / `js2rust_init()` for async export support
- Use-after-free fix: async host string returns now heap-allocated via `dupe(u8, ...)`

### 0.1.0

- Initial release
- JS-to-Zig transpilation with proc-macro FFI bridge
- Synchronous host functions (i64, f64, bool, string)
- Multi-file project support
- Type inference
- 12 Zig test groups, 90+ Rust tests

## License

Dual-licensed under MIT or Apache-2.0.
