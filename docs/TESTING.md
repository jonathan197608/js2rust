---
AIGC:
  ContentProducer: '001191110102MAD55U9H0F10002'
  ContentPropagator: '001191110102MAD55U9H0F10002'
  Label: '1'
  ProduceID: '40f94572-d6f4-4945-8ba3-4c6740e9099d'
  PropagateID: '40f94572-d6f4-4945-8ba3-4c6740e9099d'
  ReservedCode1: 'dc4445ca-feba-46b4-a7f0-c2af165f6231'
  ReservedCode2: 'dc4445ca-feba-46b4-a7f0-c2af165f6231'
---

# js2rust 测试说明文档

> 本文档描述项目的测试体系、运行方式与回归验证流程，供代码重构或优化时参考。

## 1. 测试体系总览

项目包含两层测试：

| 层级 | 位置 | 测试数量 | 验证内容 | 运行依赖 |
|------|------|----------|----------|----------|
| **Rust 单元测试** | `js2zig-core/src/tests/`（10 子模块）+ 内联测试 | 368 + 130 = 498 | 转译器正确性（JS → Zig 代码生成 + `zig ast-check`） | `zig.exe` 在 PATH |
| **Zig runtime 测试** | `runtime/jsany.zig` + `runtime/js_string.zig` 等 | ~202 | 运行时函数正确性（UTF-16 helpers、字符串方法、instanceOf 动态类型检查、日期、集合等） | `zig.exe` 在 PATH |
| **MDN 端到端测试** | `examples/mdn-test-project/` | 204 | 真实 JS 片段转译后运行结果与 Node.js 对比 | `zig.exe` + `node` 在 PATH |

### 基线指标（2026-07-10）

- Rust 单元测试：**498 passed, 0 failed**（368 在 `tests/` 子模块 + 130 内联在源文件中）
- Zig runtime 测试：**~202 passed, 0 failed**（分布在 15 个文件：js_string.zig 43、js_date.zig 35、jsany.zig 22、js_collections.zig 19、js_array.zig 14、js_object.zig 12、js_symbol.zig 11、js_uri.zig 10、js_number.zig 9、js_allocator.zig 8、js_console.zig 7、js_typedarray.zig 5、js_regexp.zig 3、js_json.zig 2、js_error.zig 2）
- Clippy：**0 warnings**
- MDN 端到端：**203 match / 1 mismatch / 0 error**（匹配率 99.5%，204 total）
- 1 个 mismatch 为已知限制，详见下方表格
- Example 项目：test-lib `cargo test` 2 passed / test-bin `cargo run` 0 errors / showcase `cargo run` **0 errors（全部输出正确）**
- UTF-16/UTF-8 差异处理：String `.length`/`charAt`/`slice`/`substring`/`indexOf`/`lastIndexOf`/`padStart`/`padEnd` 已正确实现 UTF-16 索引语义（`.length` → `utf16Len()`，切片 → `utf16IndexToByteOffset()`，查找 → `byteOffsetToUtf16Index()`）

---

## 2. Rust 单元测试

### 2.1 文件位置

```
js2zig-core/src/tests/
├── mod.rs                          # 模块入口，声明 10 个子模块
├── common.rs                       # 共享 helper 函数（0 个测试）
├── basic.rs                        # 基础转译：运算符/控制流/循环/switch（29 个测试）
├── builtins_basic.rs               # 内置方法基础：Math/Array/String/JSON（31 个测试）
├── advanced_builtins.rs            # 高级内置：Number/Map/Set/URI/RegExp/Symbol（69 个测试）
├── destructure_class_arrays.rs     # 解构/Class/String 方法/Array 高阶（51 个测试）
├── not_implemented_and_fixes.rs    # 未实现特性占位/回归修复（63 个测试）
├── shadowing_chaining_array.rs     # 变量遮蔽/方法链/Array 高阶方法/flatMap/sort（33 个测试）
├── objects_and_types.rs            # 对象/JSDoc/类型签名/JSON E2E（20 个测试）
├── phase1.rs                       # P1 特性：in/instanceof/Date/Object/spread（40 个测试）
├── collision.rs                    # 标识符冲突/保留字转义（3 个测试）
└── try_catch_and_closures.rs       # try-catch/throw/箭头函数/闭包/可选链（29 个测试）
```

另有 **130 个内联测试**分布在源文件中：

| 文件 | 测试数 | 说明 |
|------|--------|------|
| `zigir/types.rs` | 20 | 类型系统单元测试 |
| `zigir/passes/constant_fold.rs` | 12 | 常量折叠 pass |
| `zigir/ident.rs` | 9 | IrIdent 测试 |
| `zigir/lower/mod.rs` | 10 | Lower 层测试 |
| `zigir/emit/helpers.rs` | 7 | Emit helper 测试 |
| `zigir/passes/mod.rs` | 7 | Pass 框架测试 |
| `zigir/passes/dead_code.rs` | 5 | 死代码消除 |
| `zigir/emit/mod.rs` | 10 | Emit 框架测试 |
| `zigir/passes/validate.rs` | 4 | 验证 pass |
| `zigir/kinds.rs` | 4 | IR kind 测试 |
| `zigir/source_span.rs` | 4 | 源码位置测试 |
| `jsdoc.rs` | 13 | JSDoc 解析测试 |
| `parser.rs` | 7 | 解析器测试（无 `test_` 前缀） |
| `sourcemap.rs` | 4 | Source map 测试 |
| 其他 | 13 | lower/idents、lower/helpers、builtins、ops、testgen |

**总计：498 个测试**（368 + 130）

### 2.2 测试分类

测试按功能域分组，命名前缀标识所属批次：

| 前缀 | 功能域 | 数量 | 示例 |
|------|--------|------|------|
| `test_native_proto_` | 核心转译（语句/表达式/运算符/类型/内置方法） | 172 | `test_native_proto_basic`, `test_native_proto_if_else` |
| `test_not_implemented_` | 未实现特性的占位测试（验证错误提示） | 37 | `test_not_implemented_generator_function` |
| `test_p1_` | P1 优先级特性（in/instanceof/Date/Object/labeled/spread） | 36 | `test_p1_date_now`, `test_p1_spread_multi` |
| `test_p6_` | P6 String 方法全覆盖 | 24 | `test_p6_string_split`, `test_p6_string_replace` |
| `test_p2_` | P2 优先级特性（for-of/Map/Set/解构/嵌套函数） | 18 | `test_p2_destructure_object_basic` |
| `test_p8_` | P8 RegExp/Object.isSealed | 17 | `test_p8_regex_test`, `test_p8_regexp_exec_literal` |
| `test_p7_` | P7 Set/URI/Object 方法 | 11 | `test_p7_set_add_has`, `test_p7_encode_uri` |
| `test_p3_` | P3 优先级特性（String.matchAll/混合声明） | 6 | `test_p3_string_match_all_ast_check` |
| `test_method_chaining_` | 方法链 codegen | 4 | `test_method_chaining_array_filter_map` |
| `test_bigint_` | BigInt 运算 | 3 | `test_bigint_add` |
| `test_shadowing_` | 变量遮蔽场景 | 3 | `test_shadowing_let_in_block` |
| `test_cross_type_` | 跨类型比较 | 2 | `test_cross_type_number_string_eq` |
| `test_dynamic_` | 动态数组索引 | 2 | `test_dynamic_array_index_assign` |
| `test_for_loop_` | 非零起始 for 循环 | 1 | `test_for_loop_non_zero_init` |
| `test_update_expr_` | 更新表达式在索引中 | 1 | `test_update_expr_in_index` |

### 2.3 测试工具函数

所有 helper 定义在 `tests/common.rs`，各子模块通过 `use super::common::*;` 导入。

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
- 自动检测需要的 runtime import（js_allocator/js_array/js_string/js_date/js_json/js_collections/js_uri/js_regexp/js_object/js_number/js_runtime/JsAny/js_symbol/js_bigint/js_error 等 15+ 模块），注入 `@import` 声明
- 如果 `zig.exe` 不在 PATH，优雅跳过（不 fail）
- ast-check 失败时 panic，打印生成的代码和 stderr

#### `transpile_and_assert(js, test_name) -> String`

转译 + 打印生成的 Zig 代码，**不执行 ast-check**。用于仅验证代码内容的情况：

```rust
let zig = transpile_and_assert(js, "test_name");
assert!(zig.contains("pub fn add"));
```

#### `transpile_and_check(js, test_name) -> String`

转译 + 打印 + ast-check，无自定义 exports。最常用的验证模式：

```rust
let zig = transpile_and_check(js, "test_name");
assert!(zig.contains("pub fn add"));
```

#### `transpile_and_check_with_exports(js, test_name, exports) -> String`

转译 + 打印 + ast-check，支持自定义 exports 参数：

```rust
let exports = HashSet::from(["foo".to_string()]);
let zig = transpile_and_check_with_exports(js, "test_name", exports);
assert!(zig.contains("pub fn foo"));
```

#### `assert_not_implemented(js, feature_name)`

验证未实现特性能正确产生编译错误：

```rust
assert_not_implemented("function* gen() { yield 1; }", "generator");
```

### 2.4 测试编写模式

**模式 A：代码生成验证（最常见）**

```rust
#[test]
fn test_native_proto_basic() {
    let js = "export function add(a, b) { return a + b; }";
    let zig = transpile_and_check(js, "test_native_proto_basic");
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
    assert_not_implemented("function* gen() { yield 1; }", "generator");
}
```

**模式 D：带 exports 的生成验证**

```rust
#[test]
fn test_with_custom_exports() {
    let js = "export function foo() { return 42; }";
    let exports = HashSet::from(["foo".to_string()]);
    let zig = transpile_and_check_with_exports(js, "test_with_custom_exports", exports);
    assert!(zig.contains("pub fn foo"));
}
```

### 2.5 运行命令

```bash
# 全部 js2zig-core 测试
cargo test -p js2zig-core --lib

# 仅运行特定前缀的测试
cargo test -p js2zig-core --lib test_native_proto_
cargo test -p js2zig-core --lib test_p6_string_

# 运行单个测试
cargo test -p js2zig-core --lib test_native_proto_basic

# 显示 println! 输出（生成的 Zig 代码）
cargo test -p js2zig-core --lib -- --nocapture test_native_proto_basic

# Clippy 检查（必须零警告）
cargo clippy -p js2zig-core -- -D warnings

# 格式化检查
cargo fmt -p js2zig-core -- --check
```

---

## 3. MDN 端到端测试

### 3.1 项目位置

```
examples/mdn-test-project/
├── Cargo.toml              # 依赖 js2rust-bridge
├── build.rs                # 构建时调用 js2rust_bridge::build(true)
├── src/main.rs             # CLI 入口 + 204 个 fragment 分发与对比
├── js_src/                 # JS 源文件（424 个 .js + 424 个 .node.js + 1 个 app.js）
├── pass_fragments.json     # 通过转译的 204 个 fragment 列表
├── comparison_results.json # 上次对比结果快照（203 match / 1 mismatch）
├── compare_outputs.py      # Node.js vs Zig 输出对比脚本（已过时，main.rs 内置对比逻辑）
└── _check_results.py       # 快速查看 comparison_results.json
```

### 3.2 测试数据来源

从 MDN Web Docs 抓取的 JS 代码片段。磁盘上共 424 个 fragment 文件，其中 **204 个**通过转译纳入测试（`ALL_FRAGMENTS` 列表）：

| 类别 | 磁盘总数 | 通过转译 | 来源 |
|------|----------|----------|------|
| statements | 40 | 7 | MDN Statements 参考 |
| expressions | 161 | 124 | MDN Expressions 参考 |
| builtins | 223 | 73 | MDN Built-in Objects 参考 |
| **总计** | **424** | **204** | |

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
    └── 遍历 ALL_FRAGMENTS（204 个）
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

### 3.5 已知 mismatch（1 个）

| Fragment | 类型 | 问题 | 优先级 | 说明 |
|----------|------|------|--------|------|
| `test_expressions_frag_112` | MISMATCH | `-4 % 2` 输出 `0` 而非 `-0` | WONTFIX | i64 无法表示 `-0` |

---

## 4. Example 项目

除 MDN 测试外，还有 3 个 example 项目验证 bridge 集成：

| 项目 | 路径 | 类型 | 验证命令 | 验证内容 |
|------|------|------|----------|----------|
| test-lib-project | `examples/test-lib-project/` | lib | `cargo test` | 基础库导出（C ABI → Rust lib），2 个单元测试（greet + add） |
| test-bin-project | `examples/test-bin-project/` | bin | `cargo run` | sync/async host 函数、try-catch 嵌套、Date 方法，`assert_eq!` 断言 |
| showcase-project | `examples/showcase-project/` | bin | `cargo run` | 185 个导出函数覆盖 Array/Math/String/Date/Object/Class/Spread/解构，println 对比 expected 值 |

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
- **Date 方法**：9 个测试（getFullYear/getMonth/getDate/getDay/getHours/getMinutes/getSeconds 等），`assert_eq!` 验证

### 4.3 showcase-project

```bash
cd examples/showcase-project
cargo run     # 运行 main()，打印 185 个函数结果
```

覆盖范围最广的集成测试，10 个 JS 文件共 185 个导出函数：

| JS 文件 | 导出数 | 覆盖内容 |
|---------|--------|----------|
| `app.js` | 47 | C ABI 导出、循环、错误处理、集合(Map/Set)、位运算、解构默认值 |
| `utils.js` | 41 | Math 内置、算术/位/比较/一元运算、String/Array 方法、类型推断 |
| `helpers.js` | 29 | Class(stub)、闭包(stub)、对象字面量、模板字符串、控制流、import 使用 |
| `phase5.js` | 13 | Array 高阶方法（pop/shift/reverse/sort/slice/map/filter/reduce/some/every/forEach） |
| `phase6.js` | 37 | String 实例方法、Math 静态方法、Date 全量、parseInt、Object.keys、Spread merge |
| `phase_memory.js` | 5 | Memory 压力测试（Map/Set/Array 突变 + Arena 轮转） |
| `test_throw.js` | 5 | Throw/Error 传播、try-finally |
| `test_classes.js` | 4 | Class 声明（Rectangle/User 构造器 + 方法） |
| `test_optional.js` | 2 | Optional chaining（未纳入构建） |
| `for_in_struct.js` | 2 | For-in static struct（未纳入构建） |

> 注：`test_optional.js` 和 `for_in_struct.js` 存在于 `js_src/` 但未在 `js2rust.toml` 中声明，暂不参与构建。

---

## 5. 回归测试流程

### 5.1 重构/优化前

```bash
# 1. 确认基线
cargo test -p js2zig-core --lib                                      # 应全绿（498 passed）
cargo clippy -p js2zig-core -- -D warnings                           # 零警告
cargo fmt -p js2zig-core -- --check                                   # 无变更
cargo run -p mdn-test-project -- --all                                # 记录 match/mismatch 基线
```

### 5.2 重构/优化后

```bash
# 1. Rust 单元测试 — 必须全绿
cargo test -p js2zig-core --lib

# 2. Clippy — 必须零警告
cargo clippy -p js2zig-core -- -D warnings

# 3. 代码格式化
cargo fmt -p js2zig-core -- --check

# 4. MDN 端到端 — match 数不降，mismatch 数不增
cd examples/mdn-test-project
cargo run -- --all                  # 运行对比（exit code 恒为 0，需检查 Summary 输出）

# 5. Example 项目 — 运行验证（非仅构建）
cd examples/test-lib-project && cargo test    # 2 tests passed
cd examples/test-bin-project && cargo run     # assert_eq! 断言通过
cd examples/showcase-project && cargo run     # 185 个函数输出正确
```

### 5.3 验收标准

| 检查项 | 要求 | 当前结果 |
|--------|------|----------|
| `cargo test -p js2zig-core --lib` | 498 passed, 0 failed | 498 passed |
| `cargo clippy -p js2zig-core -- -D warnings` | 0 warnings | 0 warnings |
| `cargo fmt -p js2zig-core -- --check` | 无变更 | clean |
| MDN match 数 | >= 203（不低于基线） | 203 |
| MDN mismatch 数 | <= 1（不增加已知 mismatch） | 1 |
| MDN error 数 | 0（BigInt div/0 已修复为可恢复 throw） | 0 |
| test-lib-project `cargo test --lib` | 2 passed, 0 failed | 2 passed |
| test-bin-project `cargo run` | exit code 0（所有 assert_eq! 通过） | PASS |
| showcase-project `cargo run` | exit code 0（所有输出匹配 expected 值） | PASS — 0 codegen errors |

#### MDN 已知 mismatch（1 个）

| Fragment | 类型 | 问题 | 优先级 |
|----------|------|------|--------|
| `test_expressions_frag_112` | MISMATCH | `-4 % 2` 输出 `0` 而非 `-0`（i64 无法表示 -0） | WONTFIX |

### 5.4 新增测试

重构时如果发现未覆盖的边界情况：

1. 确定应归入哪个测试子模块（basic/builtins_basic/advanced_builtins/destructure_class_arrays/not_implemented_and_fixes/objects_and_types/phase1/try_catch_and_closures），在对应文件末尾添加
2. 使用 `transpile_and_check` 或 `transpile_and_assert` 函数完成转译 + 验证
3. `assert!` 验证生成的 Zig 代码包含关键模式
4. 运行 `cargo test -p js2zig-core --lib <新测试名>` 确认通过
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

### Q: Python 对比脚本还能用吗？

`compare_outputs.py` 和 `_check_results.py` 仍存在但已过时：脚本内置 153 条 fragment 计数，与当前 204 条不匹配。对比逻辑已内置到 `main.rs` 的 `run_all()` 中，推荐直接使用 `cargo run -- --all`。旧版 `compare_results.json` 已删除，以 `comparison_results.json` 为准。

### Q: 测试文件如何导航？

测试已拆分为 10 个子模块，每个聚焦一个功能域。用 IDE 的结构视图或搜索 `fn test_` 快速定位。各子模块按功能组织，不再按添加时间排列。

> AI生成