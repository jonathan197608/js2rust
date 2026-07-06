---
AIGC:
  ContentProducer: '001191110102MAD55U9H0F10002'
  ContentPropagator: '001191110102MAD55U9H0F10002'
  Label: '1'
  ProduceID: 'f5b8cc36-c093-4470-8881-d47e43bb00e7'
  PropagateID: 'f5b8cc36-c093-4470-8881-d47e43bb00e7'
  ReservedCode1: '4235d542-2897-4772-a181-2922ddd8ad81'
  ReservedCode2: '4235d542-2897-4772-a181-2922ddd8ad81'
---

# js2rust 测试说明文档

> 本文档描述项目的测试体系、运行方式与回归验证流程，供代码重构或优化时参考。

## 1. 测试体系总览

项目包含两层测试：

| 层级 | 位置 | 测试数量 | 验证内容 | 运行依赖 |
|------|------|----------|----------|----------|
| **Rust 单元测试** | `js2zig-core/src/tests.rs` | 361 | 转译器正确性（JS → Zig 代码生成 + `zig ast-check`） | `zig.exe` 在 PATH |
| **MDN 端到端测试** | `examples/mdn-test-project/` | 153 | 真实 JS 片段转译后运行结果与 Node.js 对比 | `zig.exe` + `node` 在 PATH |

### 基线指标（2026-07-06）

- Rust 单元测试：**455 passed, 0 failed**
- Clippy：**0 warnings**
- MDN 端到端：**200 match / 3 mismatch / 1 error**（匹配率 98.0%，204 total）
- 3 个 mismatch + 1 个 error 均为已知限制，详见下方表格
- Example 项目：test-lib `cargo test` 2 passed / test-bin `cargo run` 0 errors / showcase `cargo run` **19 个 pre-existing codegen 错误**

---

## 2. Rust 单元测试

### 2.1 文件位置

```
js2zig-core/src/tests.rs
```

单文件包含全部 360 个测试，组织在 `native_proto_tests` 模块内。

### 2.2 测试分类

测试按功能域分组，命名前缀标识所属批次：

| 前缀 | 功能域 | 示例 |
|------|--------|------|
| `test_native_proto_` | 核心转译（语句/表达式/运算符/类型） | `test_native_proto_basic`, `test_native_proto_if_else` |
| `test_p1_` | P1 优先级特性（in/instanceof/Date/Object/labeled/spread） | `test_p1_date_now`, `test_p1_spread_multi` |
| `test_p2_` | P2 优先级特性（for-of Map/Set/解构/嵌套函数） | `test_p2_destructure_object_basic` |
| `test_p3_` | P3 优先级特性（String.matchAll/混合声明） | `test_p3_string_match_all_ast_check` |
| `test_p6_` | P6 String 方法全覆盖 | `test_p6_string_split`, `test_p6_string_replace` |
| `test_p7_` | P7 Set/URI/Object 方法 | `test_p7_set_add_has`, `test_p7_encode_uri` |
| `test_p8_` | P8 RegExp/Object.isSealed | `test_p8_regex_test`, `test_p8_regexp_exec_literal` |
| `test_not_implemented_` | 未实现特性的占位测试（验证错误提示） | `test_not_implemented_generator_function` |
| `test_bigint_` | BigInt 运算 | `test_bigint_add` |

### 2.3 测试工具函数

#### `parse_and_transpile(js, exports) -> Result<TranspileResult, String>`

核心 helper：用 oxc 解析 JS，调用 `transpile_js` 生成 Zig 代码。

```rust
fn parse_and_transpile(
    js: &str,
    exports: Option<std::collections::HashSet<String>>,
) -> Result<TranspileResult, String>
```

#### `assert_zig_ast_check(zig_code, test_name)`

将生成的 Zig 代码写入临时文件，运行 `zig ast-check` 验证语法正确性。
- 自动检测需要的 runtime import（`js_array`/`js_string`/`js_date` 等），注入 `@import` 声明
- 如果 `zig.exe` 不在 PATH，优雅跳过（不 fail）
- ast-check 失败时 panic，打印生成的代码和 stderr

#### `transpile_and_assert!` 宏

组合 `parse_and_transpile` + `assert_zig_ast_check`，一行完成转译 + 语法验证：

```rust
let zig = transpile_and_assert!(js, "test_name");
assert!(zig.contains("pub fn add"));
```

### 2.4 测试编写模式

**模式 A：代码生成验证（最常见）**

```rust
#[test]
fn test_native_proto_basic() {
    let js = "export function add(a, b) { return a + b; }";
    let zig = transpile_and_assert!(js, "test_native_proto_basic");
    assert!(zig.contains("pub fn add(a: anytype, b: anytype) i64 {"));
    assert!(zig.contains("return a + b;"));
}
```

**模式 B：仅验证转译不报错（不检查 ast-check）**

```rust
#[test]
fn test_native_proto_toplevel_var_error() {
    let js = "var x = 10;";
    let result = parse_and_transpile(js, None);
    // 顶层 var 应该报错
    assert!(result.is_err());
}
```

**模式 C：未实现特性占位**

```rust
#[test]
fn test_not_implemented_generator_function() {
    let js = "function* gen() { yield 1; }";
    let result = parse_and_transpile(js, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not implemented"));
}
```

### 2.5 运行命令

```bash
# 全部测试
cargo test

# 仅运行特定前缀的测试
cargo test test_native_proto_
cargo test test_p6_string_

# 运行单个测试
cargo test test_native_proto_basic

# 显示 println! 输出（生成的 Zig 代码）
cargo test -- --nocapture test_native_proto_basic

# Clippy 检查（必须零警告）
cargo clippy --all-targets -- -D warnings
```

---

## 3. MDN 端到端测试

### 3.1 项目位置

```
examples/mdn-test-project/
├── Cargo.toml          # 依赖 js2rust-bridge
├── build.rs            # 构建时调用 js2rust_bridge::build(true)
├── src/main.rs         # CLI 入口 + 153 个 fragment 分发
├── js_src/             # JS 源文件（.js + .node.js 参考文件）
├── pass_fragments.json # 通过转译的 fragment 列表
├── compare_results.json # 上次对比结果
├── comparison_results.json # Python 脚本对比结果
├── compare_outputs.py  # Node.js vs Zig 输出对比脚本
└── _check_results.py   # 快速查看对比结果
```

### 3.2 测试数据来源

从 MDN Web Docs 抓取的 JS 代码片段，分为三类：

| 类别 | 数量 | 来源 |
|------|------|------|
| statements | 7 | MDN Statements 参考 |
| expressions | 124 | MDN Expressions 参考 |
| builtins | 73 | MDN Built-in Objects 参考 |
| **总计** | **153** | |

每个 fragment 有两个文件：
- `test_<category>_frag_<N>.js` — 原始 JS 片段（供转译器处理）
- `test_<category>_frag_<N>.node.js` — Node.js 参考文件（带 try/catch 包装，产出期望输出）

### 3.3 构建与运行流程

```
cargo build                    ← 触发 build.rs
    └── js2rust_bridge::build(true)
        └── 1. 调用 js2zig-core 转译 js_src/*.js → Zig 代码
            2. 生成 build.zig + runtime/*.zig
            3. zig build → 编译为 .lib
            4. 链接到 Rust 二进制

cargo run                       ← 运行所有 fragment
    └── 遍历 ALL_FRAGMENTS
        ├── 运行 Zig 二进制 (子进程，crash 隔离)
        ├── 运行 Node.js (获取参考输出)
        └── 逐行对比 stderr/stdout
```

### 3.4 CLI 用法

```bash
cd examples/mdn-test-project

# 构建（转译 + Zig 编译 + 链接）
cargo build

# 运行所有 fragment，与 Node.js 对比
# 注意：exit code 恒为 0，需检查 stderr 的 Summary 输出判断 match/mismatch/error
cargo run

# 列出所有 fragment
cargo run -- --list

# 运行单个 fragment
cargo run -- test_expressions_frag_4

# 运行所有 fragment（显式）
cargo run -- --all
```

### 3.5 Python 对比脚本

```bash
cd examples/mdn-test-project

# 完整对比：Node.js vs Zig，输出到 comparison_results.json
python compare_outputs.py

# 快速查看上次对比结果
python _check_results.py
```

### 3.6 已知 mismatch（4 个）

| Fragment | 问题 | 优先级 | 说明 |
|----------|------|--------|------|
| `test_statements_frag_11` | const 重新赋值未报错 | WONTFIX | Zig 无法在运行时检测 const 重赋值 |
| `test_expressions_frag_109` | BigInt `2n/0n` CRASH | ACCEPTABLE | 未捕获的 RangeError，行为正确 |
| `test_expressions_frag_112` | `-4 % 2` 输出 `0` 而非 `-0` | WONTFIX | i64 无法表示 `-0` |
| `test_builtins_frag_202` | stack trace 格式差异 | WONTFIX | Zig stack trace 格式不同于 Node.js |

---

## 4. Example 项目

除 MDN 测试外，还有 3 个 example 项目验证 bridge 集成：

| 项目 | 路径 | 类型 | 验证命令 | 验证内容 |
|------|------|------|----------|----------|
| test-lib-project | `examples/test-lib-project/` | lib | `cargo test` | 基础库导出（C ABI → Rust lib），2 个单元测试（greet + add） |
| test-bin-project | `examples/test-bin-project/` | bin | `cargo run` | 二进制项目（含 sync/async host 函数、try-catch 嵌套、Date 方法），`main()` 内含 `assert_eq!` 断言 |
| showcase-project | `examples/showcase-project/` | bin | `cargo run` | for-in static struct 集成演示，覆盖 60+ 函数调用（Array/Math/String/Date/Object/Class/Spread/解构），打印期望值验证 |

### 4.1 test-lib-project

```bash
cd examples/test-lib-project
cargo test    # 2 tests: test_greet, test_add
```

`src/lib.rs` 内含 `#[cfg(test)] mod tests`，验证转译后的 `greet_main` 和 `add_main` 返回正确值。

### 4.2 test-bin-project

```bash
cd examples/test-bin-project
cargo run     # 运行 main()，含 assert_eq! 断言
```

验证内容：
- **sync JS 函数**：`greet_main`（字符串返回）、`add_main`（整数返回）
- **sync host 函数**：`useHostAdd_main`、`useHostMultiply_main`（整数）、`useHostConcat_main`（字符串，验证 `js_allocator_dupe` FFI）、`useHostStrlen_main`
- **async host 函数**：`getUserInfo_main`（tokio runtime + `JsStrField` 返回）
- **try-catch 嵌套**：4 个测试（嵌套/重抛/资源管理），`assert_eq!` 验证
- **Date 方法**：9 个测试（getFullYear/getDay/getHours/...），`assert_eq!` 验证

### 4.3 showcase-project

```bash
cd examples/showcase-project
cargo run     # 运行 main()，打印 60+ 函数结果
```

覆盖范围最广的集成测试，包含：
- Phase 0/5/6：算术、Array 方法（pop/reduce/forEach/map/filter/some/every）
- Throw/Error 传播、try-finally
- Memory 压力测试（Map/Set/Array 突变 + Arena 轮转）
- String/Math/Date/Number/Object 内置方法
- Spread merge（单/多/三重/内联/覆盖）
- 类型推断验证（除法/取模/位运算/Map.delete/Set.delete）
- 解构默认值（对象/数组 + 空对象/空数组）
- Class 支持（Rect area/perimeter、User id/nameLength）

---

## 5. 回归测试流程

### 5.1 重构/优化前

```bash
# 1. 确认基线
cargo test                    # 应全绿（361 passed）
cargo clippy --all-targets -- -D warnings  # 零警告
cd examples/mdn-test-project && cargo run   # 记录 match/mismatch 基线
```

### 5.2 重构/优化后

```bash
# 1. Rust 单元测试 — 必须全绿
cargo test

# 2. Clippy — 必须零警告
cargo clippy --all-targets -- -D warnings

# 3. 代码格式化
cargo fmt

# 4. MDN 端到端 — match 数不降，mismatch 数不增
cd examples/mdn-test-project
cargo build                   # 转译 + 编译
cargo run                     # 运行对比（exit code 恒为 0，需检查 Summary 输出）
# 或用 Python 脚本获取详细对比
python compare_outputs.py

# 5. Example 项目 — 运行验证（非仅构建）
cd examples/test-lib-project && cargo test    # 2 tests passed
cd examples/test-bin-project && cargo run     # assert_eq! 断言通过
cd examples/showcase-project && cargo run     # 60+ 函数输出正确
```

### 5.3 验收标准

| 检查项 | 要求 | 当前结果 |
|--------|------|----------|
| `cargo test -p js2zig-core --lib` | 455 passed, 0 failed | 455 passed |
| `cargo clippy -p js2zig-core -- -D warnings` | 0 warnings | 0 warnings |
| `cargo fmt -p js2zig-core -- --check` | 无变更 | clean |
| MDN match 数 | >= 200（不低于基线） | 200 |
| MDN mismatch 数 | <= 4（不增加已知 mismatch） | 3 |
| MDN error 数 | <= 1（frag_109 BigInt/0 为已知 CRASH） | 1 |
| test-lib-project `cargo test --lib` | 2 passed, 0 failed | 2 passed |
| test-bin-project `cargo run` | exit code 0（所有 assert_eq! 通过） | PASS |
| showcase-project `cargo run` | exit code 0（所有输出匹配 expected 值） | **FAIL — 19 pre-existing codegen bugs** |

#### MDN 已知 mismatch/error（4 个）

| Fragment | 类型 | 问题 | 优先级 |
|----------|------|------|--------|
| `test_statements_frag_11` | MISMATCH | const 重赋值无运行时 TypeError（Zig 根本限制） | WONTFIX |
| `test_expressions_frag_109` | CRASH | BigInt `2n / 0n` → 未捕获 RangeError（行为正确） | ACCEPTABLE |
| `test_expressions_frag_112` | MISMATCH | `-4 % 2` 输出 `0` 而非 `-0`（i64 无法表示 -0） | WONTFIX |
| `test_builtins_frag_202` | MISMATCH | stack trace 格式差异（运行时格式不可调合） | WONTFIX |

#### showcase-project pre-existing codegen 错误（19 个）

| 类别 | 数量 | 示例 |
|------|------|------|
| js_date member 错误 | 10 | `js_date.getFullYear()` 应为 `d.getFullYear()` 实例调用 |
| 方法调用参数不匹配 | 7 | `js_array.shift()` 缺少 ArrayList 参数、`parseInt()` 缺少 radix |
| 内联回调 codegen bug | 2 | `_` 标识符和 `x` undeclared（filter/reduce inline） |

### 5.4 新增测试

重构时如果发现未覆盖的边界情况：

1. 在 `js2zig-core/src/tests.rs` 末尾添加测试函数
2. 使用 `transpile_and_assert!` 宏确保 ast-check 通过
3. `assert!` 验证生成的 Zig 代码包含关键模式
4. 运行 `cargo test <新测试名>` 确认通过
5. 更新本文件的测试计数

---

## 6. 常见问题

### Q: `zig ast-check` 被跳过了？

`assert_zig_ast_check` 在 `zig.exe` 不在 PATH 时会打印 warning 并跳过，测试不会 fail。确认 `zig.exe` 可用：

```bash
zig version
```

### Q: MDN 测试构建失败（Zig 编译错误）？

自 2026-07-01 起，Zig 构建失败不再静默吞掉。`js2rust-bridge/src/lib.rs` 的 `build()` 在 `transpile_project` 返回 `Err` 时会 `panic!`，暴露 codegen bug。检查 `cargo build` 的完整错误信息。

### Q: MDN 测试运行时 Node.js 不在 PATH？

`run_all` 会检测 `node` 是否可用。不可用时降级为仅检查 Zig 退出码模式（不做输出对比）。

### Q: 测试文件太大不好导航？

`tests.rs` 是单文件，测试按功能域前缀分组。用 IDE 的结构视图或搜索 `fn test_` 快速定位。测试按添加时间排列，P1/P2/P3/P6/P7/P8 分批添加。

> AI生成