# js2zig 集成方案（简化版）

> **设计目标**：外部 Rust 项目通过 `build.rs` 一行调用，实现 JS → Zig → 静态库 → Rust FFI 的全自动编译集成。
>
> **简化要点**：
> 1. 不保留 CLI（`js2rustc` 删除）
> 2. 只支持多文件项目转译
> 3. Bridge Macro 固定使用 `$OUT_DIR/js2zig/` 前缀，用户只需指定组名

---

## 架构总览

### 最终 workspace 结构

```
js2rust/                          # workspace root
├── Cargo.toml                    # workspace 配置
│
├── js2zig-core/           [核心] # 转译库 (从 js2rustc 提取)
│   ├── Cargo.toml
│   ├── build.rs                # 自动生成 runtime/embed.rs
│   └── src/
│       ├── lib.rs                # 公开 API
│       ├── pipeline.rs           # 编排逻辑 (js2rustc/main.rs → 此处)
│       ├── bridge.rs             # Bridge lib.rs 生成
│       └── runtime/
│           └── embed.rs          # auto-generated: include_str!() 嵌入运行时
│
├── js2zig-build/         [发布] # build.rs helper (外部项目用)
│   ├── Cargo.toml
│   └── src/lib.rs                # compile_js() — build.rs 一键集成
│
├── js2rust-bridge/        [发布] # FFI 桥接 (保持现有结构)
├── js2rust-bridge-macro/ [发布] # proc-macro (固定 $OUT_DIR 前缀)
│
├── runtime/               [源]   # Zig 运行时源文件 (被 js2zig-core 嵌入)
├── in/                    [测试] # JS 测试文件
├── host_config.json       [配置] # Host 函数配置
└── README.md
```

### 依赖关系

```
外部项目 Cargo.toml
  [build-dependencies]
  js2zig-build = "..."      ← build.rs 中调用

js2zig-build
  ↓ 依赖
js2zig-core
  ↓ 依赖 (compile-time: embed runtime/*.zig)
runtime/
  ↓ 生成 (写入 $OUT_DIR/js2zig/)
cabi_exports.json
  ↓ 读取 (compile time)
js2rust-bridge-macro
  ↓ 生成 (compile time)
FFI 绑定代码
```

---

## 外部项目集成示例

### 项目结构

```
my-app/
├── Cargo.toml
├── build.rs
├── js/
│   ├── math.js          # function add(a, b) { return a + b; }
│   └── string_utils.js  # function greet(name) { return "Hello, " + name; }
├── host_config.json     # (可选) Rust host 函数声明
└── src/
    ├── main.rs
    └── host.rs           # (可选) Rust host 函数实现
```

### Cargo.toml

```toml
[package]
name = "my-app"
version = "0.1.0"
edition = "2021"

[build-dependencies]
js2zig-build = { git = "https://github.com/aspect-building/js2rust.git" }

[dependencies]
js2rust-bridge = { git = "https://github.com/aspect-building/js2rust.git" }
```

### build.rs（一行调用）

```rust
fn main() {
    // 使用默认配置（js/ 目录）
    js2zig_build::compile_js(Default::default());
}
```

或自定义配置：

```rust
fn main() {
    js2zig_build::compile_js(js2zig_build::JsBuildConfig {
        js_dir: "js/".into(),
        host_config: Some("host_config.json".into()),
    });
}
```

### src/main.rs（极简）

```rust
use js2rust_bridge::js2rust_bridge;

// 只需指定组名，自动查找 $OUT_DIR/js2zig/main/cabi_exports.json
js2rust_bridge!("main");

fn main() {
    // 调用转译后的 JS 函数（已编译为 Zig 静态库并链接）
    let sum = unsafe { add_main(3, 5) };
    println!("3 + 5 = {}", sum);  // 输出: 3 + 5 = 8
}
```

---

## 核心 API

### js2zig-core API

```rust
/// 多文件项目配置
pub struct ProjectConfig {
    /// 项目名称（也是输出目录名和 Zig 库名）
    pub name: String,
    /// JS 源文件目录路径
    pub js_dir: PathBuf,
    /// 输出目录路径（通常是 $OUT_DIR）
    pub out_dir: PathBuf,
    /// Host 函数配置文件路径（可选）
    pub host_config: Option<PathBuf>,
}

/// 项目转译结果
pub struct ProjectResult {
    /// 每个组的结果
    pub groups: Vec<GroupResult>,
    /// 全局诊断信息
    pub diagnostics: Vec<Diagnostic>,
}

/// 多文件项目转译：JS 目录 → Zig 项目 + cabi_exports.json
pub fn transpile_project(config: &ProjectConfig) -> Result<ProjectResult, Error>;
```

### js2zig-build API

```rust
/// build.rs 集成配置（简化：只保留必要字段）
pub struct JsBuildConfig {
    /// JS 源文件目录（相对于 CARGO_MANIFEST_DIR）
    pub js_dir: PathBuf,
    /// Host 函数配置文件（可选，相对于 CARGO_MANIFEST_DIR）
    pub host_config: Option<PathBuf>,
}

impl Default for JsBuildConfig {
    fn default() -> Self {
        Self {
            js_dir: PathBuf::from("js"),
            host_config: None,
        }
    }
}

/// build.rs 一键集成入口
///
/// 执行完整的 JS → Zig → 静态库 流水线，并输出 cargo link 指令。
/// 输出目录固定为 `$OUT_DIR/js2zig/`，Bridge Macro 自动使用此路径。
///
/// # Panics
/// 构建失败时 panic（build.rs 约定）
pub fn compile_js(config: JsBuildConfig);
```

---

## 发布到 crates.io

### 发布顺序

```bash
# 1. 发布 js2zig-core (无依赖)
cd js2zig-core && cargo publish

# 2. 发布 js2rust-bridge-macro (依赖 serde)
cd ../js2rust-bridge-macro && cargo publish

# 3. 发布 js2rust-bridge (无 proc-macro 依赖)
cd ../js2rust-bridge && cargo publish

# 4. 最后发布 js2zig-build (依赖 js2zig-core)
cd ../js2zig-build && cargo publish
```

### 外部项目使用（发布后）

```toml
# 外部项目 Cargo.toml (发布到 crates.io 后)
[build-dependencies]
js2zig-build = "0.1.0"

[dependencies]
js2rust-bridge = { version = "0.1.0" }
```

详细发布步骤见 `INTEGRATION_BUILDRS_SIMPLE.md` §6。

---

## 实施步骤

### Step 1: 新建 js2zig-core crate (2-3h)

1. `cargo init --lib js2zig-core`
2. 添加到 workspace members
3. 将 `js2rustc/src/` 中的模块移动到 `js2zig-core/src/`
4. 定义 `ProjectConfig`, `ProjectResult`
5. 实现 `runtime/embed.rs`（`include_str!()` 嵌入所有 runtime 文件）

### Step 2: 删除 js2rustc CLI (0.5h)

1. 从 workspace 移除 `js2rustc`
2. 删除 `js2rustc/` 目录

### Step 3: 新建 js2zig-build crate (2h)

1. `cargo init --lib js2zig-build`
2. 实现 `compile_js()`
3. 实现 Zig 编译器查找、cargo link 指令输出、rerun-if-changed
4. 设置 `JS2ZIG_OUT_DIR` 环境变量

### Step 4: 简化 bridge macro (1h)

1. `resolve_json_path()` 固定使用 `JS2ZIG_OUT_DIR` 环境变量
2. 用户只需指定组名

### Step 5: 发布到 crates.io (1h)

1. 更新所有 `Cargo.toml` (`authors`, `license`, `description`, etc.)
2. `cargo publish` 按顺序发布 4 个 crate

### Step 6: 验证 (1h)

1. `cargo test` — 所有现有测试不变
2. `cargo clippy` — 零警告
3. 创建外部项目测试：`cargo add js2zig-build --build`，验证端到端流程
4. 提交

**总预估：7-9h（~2 天）**

---

## 详细设计文档

- **简化版详细设计**：`INTEGRATION_BUILDRS_SIMPLE.md`
  - js2zig-core API 详细设计
  - js2zig-build 实现细节
  - Bridge Macro 简化方案
  - 发布到 crates.io 详细步骤
  - 风险与缓解措施

---

## 参考资料

- **当前实现状态**：`ROADMAP.md`
- **语法覆盖率**：`SYNTAX_IMPLEMENTATION.md`
- **原始设计（已废弃）**：`INTEGRATION_BUILDRS.md`（多模式方案）
