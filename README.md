# js2rust — JS-to-Zig Transpiler for Rust FFI

`js2rust` is a JS-to-Zig transpiler that enables seamless integration of JavaScript code into Rust projects via automatic FFI bridge generation.

## Features

- **JS-to-Zig transpilation**: Automatically converts JS source files to Zig code
- **Automatic FFI bridge**: Generates Rust FFI bindings via proc-macro
- **Build.rs integration**: One-line build script integration
- **Multi-file project support**: Transpile entire JS project directories
- **Type inference**: Automatic JS type inference (number → i64/f64, string → []u8, etc.)

## Quick Start

### 1. Add dependencies to your `Cargo.toml`

```toml
[build-dependencies]
js2zig-build = "0.1"

[dependencies]
js2rust-bridge = "0.1"
```

### 2. Create `build.rs`

```rust
fn main() {
    // Transpile JS source files in "js_src/" directory
    js2zig-build::transpile("js_src");
}
```

### 3. Write JS code in `js_src/main.js`

```javascript
export function greet(name) {
    return "Hello, " + name + "!";
}

export function add(a, b) {
    return a + b;
}
```

### 4. Use the generated FFI bindings in `src/lib.rs`

```rust
// Generate FFI bindings for the "main" group
js2rust_bridge!(main);

// Now you can call the generated safe wrapper functions:
// - greet_main(name: &str) -> String
// - add_main(a: i64, b: i64) -> i64

fn main() {
    let result = greet_main("World");
    println!("{}", result); // "Hello, World!"

    let sum = add_main(1, 2);
    println!("1 + 2 = {}", sum); // 3
}
```

## Architecture

```
js2rust/
├── js2zig-core/          # Core transpiler library
├── js2zig-build/         # Build.rs helper (build-dependency)
├── js2rust-bridge/     # FFI bridge (runtime support)
└── js2rust-bridge-macro/ # Proc-macro for FFI binding generation
```

## How it works

1. `build.rs` calls `js2zig_build::transpile("js_src")`
2. `js2zig-core` transpiles JS to Zig, outputs to `$OUT_DIR/js2zig/`
3. `build.rs` runs `zig build` to compile Zig code to static library
4. `js2rust_bridge!(main)` macro reads `$OUT_DIR/js2zig/main/cabi_exports.json`
5. Macro generates `unsafe extern "C"` declarations + safe Rust wrappers
6. You call the generated safe wrapper functions (e.g., `greet_main()`, `add_main()`)

## Requirements

- Rust 1.75+ (edition 2021 or later)
- Zig 0.16.0+ (for compiling transpiled Zig code)

## License

Dual-licensed under MIT or Apache-2.0.
