# js2rust — JS 转 Zig 转译器（Rust FFI 集成）

`js2rust` 是一个 JS 到 Zig 的源码级转译器，可将 JavaScript 代码无缝集成到 Rust 项目中，通过自动生成 FFI 桥接代码实现 JS ↔ Rust 互调用。

> [English Version](README_EN.md)

## 项目状态

| 指标 | 数值 |
|------|------|
| Rust 测试 | 361 (361 pass, 0 ignore) |
| Zig 测试 | 27 |
| Clippy 警告 | 0 |
| JS 表达式覆盖率 | 78/91 (~86%) |
| JS 语句覆盖率 | 43/49 (~88%) |
| JS 内置对象覆盖率 | 194/217 (~89%，含 5 个简化实现) |
| Crate 版本 | [js2zig-core 0.7](https://crates.io/crates/js2zig-core) · [js2rust-bridge 0.8](https://crates.io/crates/js2rust-bridge) · [js2rust-bridge-macro 0.8](https://crates.io/crates/js2rust-bridge-macro) |

> 详细特性评估见 [JS 语言特性实现说明](docs/JS_FEATURE_EVALUATION.md)。

## 核心特性

- **JS → Zig 转译**：自动将 JS 源文件转换为 Zig 代码
- **Proc-macro FFI 桥接**：`js2rust_bridge!()` 一步完成转译和 Rust FFI 绑定生成
- **Host 函数**：从 JS 中直接调用 Rust 函数（通过 C ABI）
  - 同步：`i64`、`f64`、`bool`、`str` 参数及返回值
  - **异步**：`async fn` 带 struct 返回类型，通过 tokio bridge
- **异步导出函数**：`export async function` 生成 C ABI 阻塞包装器（利用全局 Zig `Io` 实例）
- **字符串宿主函数**：自动 `[*:0]const u8` ↔ `[]const u8` 转换，堆分配返回值
- **Source Map**：`// @src(file:line)` 行内注释 + `source_map.json`
- **增量编译**：基于哈希的缓存，未修改文件跳过重建（`--force` 强制重建）
- **WASM 目标**：`zig build wasm`（wasm32-wasi）支持
- **多文件项目支持**：可转译整个 JS 项目目录，DFS 依赖排序
- **类型推断**：自动 JS 类型推断（number → i64/f64，string → `[]u8` 等）
- **零代码生成**：所有逻辑在 proc-macro 中完成，IDE 友好

## 快速开始

### 1. 添加依赖

```toml
[dependencies]
js2rust-bridge = "0.8"

[build-dependencies]
js2rust-bridge = "0.8"
```

### 2. 编写 JS 代码 `js_src/main.js`

```javascript
export function greet(name) {
    return "Hello, " + name + "!";
}

export function add(a, b) {
    return a + b;
}
```

### 3. 在 `src/main.rs` 中使用宏

```rust
js2rust_bridge!("js_src/main.js");

fn main() {
    let result = greet_main("World");
    println!("{}", result); // "Hello, World!"

    let sum = add_main(1, 2);
    println!("1 + 2 = {}", sum); // 3
}
```

### 4. 添加 `build.rs` 用于静态库链接

```rust
fn main() {
    js2rust_bridge::link();
}
```

## Host 函数（Rust → JS）

在宏中声明 Host 函数，即可从 JS 中调用 Rust 函数：

### 同步 Host 函数

```rust
js2rust_bridge! {
    "js_src/main.js",
    host_add(i64, i64) -> i64,
    host_concat(str, str) -> str,
}
```

Rust 实现：

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

### 异步 Host 函数

```rust
js2rust_bridge! {
    "js_src/main.js",
    async fetch_user(str) -> { id: i64, name: str },
}
```

```rust
use tokio::runtime::Runtime;
use std::sync::OnceLock;

static RUNTIME: OnceLock<Runtime> = OnceLock::new();
fn runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| tokio::runtime::Builder::new_current_thread()
        .enable_all().build().expect("tokio runtime"))
}

#[repr(C)]
pub struct HostFetchUserResult { pub id: i64, pub name: [u8; 256] }

#[no_mangle]
pub extern "C" fn fetch_user(name: *const std::ffi::c_char) -> HostFetchUserResult {
    let name = unsafe { std::ffi::CStr::from_ptr(name).to_string_lossy() };
    runtime().block_on(fetch_user_from_db(&name))
}
```

JS 中使用 `await`：

```js
export async function getUserInfo(name) {
    const user = await fetch_user(name);
    return user.name;
}
```

### 异步导出函数

`export async function` 通过 C ABI 导出为阻塞包装器：

```js
export async function getUserInfo(name) {
    const user = await fetch_user(name);
    return user.name;
}
```

Rust 侧同步调用：

```rust
fn main() {
    js2rust_init();  // 初始化全局 Io（异步导出函数需要）
    let name = getUserInfo_main("alice");
    println!("User: {}", name);
    js2rust_deinit();
}
```

## 项目架构

```
js2rust/
├── js2zig-core/            # 核心转译库（解析、类型推断、代码生成）
├── js2rust-bridge/         # 外观 crate（重导出 proc-macro + link 辅助函数）
├── js2rust-bridge-macro/   # Proc-macro：转译 + 生成 FFI 绑定
├── runtime/                # Zig 运行时（js_array/js_string/js_map/js_date/js_regexp 等）
├── native_proto/           # 代码生成器（expr → Zig、stmt → Zig、内置对象调用）
└── examples/
    ├── test-bin-project/   # 二进制项目（同步+异步 host 函数）
    ├── test-lib-project/   # 库项目
    ├── showcase-project/   # 多文件综合示例
    └── mdn-test-project/   # MDN 语义一致性测试集（149+ cases）
```

### 工作原理

1. `js2rust_bridge!("js_src/main.js")` 宏调用 `js2zig_core::transpile_project()`
2. 核心 JS 文件及其传递导入被转译为 Zig，输出到 `.js2zig-cache/main/`
3. 宏读取 `cabi_exports.json` 并生成 `unsafe extern "C"` + 安全 Rust 包装器
4. 异步导出生成 `_impl` 异步函数 + C ABI 阻塞包装器（使用全局 `Io`）
5. 宏运行 `zig build` 编译静态库
6. `build.rs` 链接静态库（扫描 `.js2zig-cache/`）
7. 调用生成的安全包装函数（如 `greet_main()`、`getUserInfo_main()`）

### 异步调用链

```
Rust: getUserInfo_main("alice")
  → Zig C ABI: getUserInfo_cabi(name)
    → Zig async: getUserInfo_impl(io, name)
      → Zig: io.async(fetch_user, .{ io, name })
        → Zig wrapper: fetch_user_async(io, name) → extern "c" fetch_user(name)
          → Rust: fetch_user(name) → tokio runtime block_on(async_fn)
```

## 环境要求

- Rust 1.85+（edition 2024）
- Zig 0.16.0+（用于编译转译后的 Zig 代码）

## 文档

- [JS 语言特性实现说明](docs/JS_FEATURE_EVALUATION.md) — 逐特性实现状态，覆盖 140 个语法特性 + 217 行内置对象方法

## 许可证

MIT 或 Apache-2.0 双许可。
