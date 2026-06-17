# js2rust — 开发路线图

> JS → Zig 源码翻译器，Rust 实现。
> ~10,000 行 Rust + 15 个 Zig 运行时文件。
> `cargo check` / `cargo clippy` 零新警告，Zig 0.16 目标。

---

## 核心流水线（7 阶段）

| 阶段 | 模块 | 状态 | 行数 |
|------|------|------|------|
| 1 | 预处理（import/export 解析、模块合并、命名冲突） | ✅ 完成 | 1186 |
| 2 | 类型推导（12 种 Zig 类型，含 Union/Optional/Object） | ✅ 完成 | 2365 |
| 3 | 代码生成（语句、表达式、闭包、内置函数、类） | ✅ 核心完成 | ~3400 |
| 4 | 测试生成（AST 提取用例 → Zig test 块） | ✅ 完成 | 236 |
| 5 | Zig 项目输出（build.zig/build.zig.zon/lib.zig） | ✅ 完成 | 390 |
| 6 | Zig 构建 + 测试（自动调用 `zig build` / `zig build test`） | ✅ 完成 | 内嵌 main.rs |
| 7 | 多文件分组（每组独立 Zig 项目，per-file codegen） | ✅ 完成 | 内嵌 main.rs |

---

## 语法覆盖率

### 语句 — 14/15 已支持

| 语法 | 状态 |
|------|------|
| const/let/var（含解构） | ✅ |
| function declaration（含 async、类方法、C ABI export） | ✅ |
| class declaration（→ Zig struct + methods） | ✅ |
| 表达式语句 | ✅ |
| return（含 try/catch 上下文） | ✅ |
| if/else if/else | ✅ |
| block `{}` | ✅ |
| for (init; test; update) | ✅ |
| for-of（含解构 for-of） | ✅ |
| for-in（→ HashMap iterator） | ✅ |
| while | ✅ |
| do-while | ✅ |
| break / continue | ✅ |
| switch | ✅ |
| try/catch/finally（→ Zig error union + defer） | ✅ |
| throw（→ error propagation） | ✅ |
| for-await-of | 🚫 不实现 |

### 表达式 — 28/36 已支持（8 项标为不实现）

| 语法 | 状态 |
|------|------|
| 数值/字符串/布尔/null/BigInt 字面量 | ✅ |
| 标识符（含 Zig 关键字转义） | ✅ |
| this → self | ✅ |
| 二元运算（全部运算符，含 `**` → `std.math.pow`） | ✅ |
| 字符串拼接 `+` → `allocPrint` | ✅ |
| 位移运算 → `@intCast(u6, ...)` | ✅ |
| 逻辑运算 and/or/orelse | ✅ |
| 一元运算（含 typeof/delete/void/位非→@as(i64)） | ✅ |
| update ++/-- | ✅ |
| 函数调用（builtin 感知、闭包感知） | ✅ |
| new 表达式（Map/Set/类实例） | ✅ |
| 静态成员 `.`（含 `.length` → `.len`、动态对象 HashMap） | ✅ |
| 计算成员 `[]`（动态数组 .items、HashMap .get、struct 字段） | ✅ |
| 赋值 =, +=, -= 等（含动态对象 HashMap.put） | ✅ |
| 条件（三元）`? :` → `if/else` | ✅ |
| 数组字面量 `[_]T{}` | ✅ |
| 对象字面量 `.{}`（含 spread/override） | ✅ |
| 模板字符串（含插值 → `allocPrint`） | ✅ |
| 箭头函数（闭包 struct 实例化） | ✅ |
| await → `io.async/defer/cancel/await` | ✅ |
| 括号/逗号/可选链表达式 | ✅ |
| 正则字面量 | ✅ |
| TypeScript 类型表达式（as/assert/!./satisfies/instantiation） | ✅ |
| 内联箭头函数 | 🚫 设计限制（要求用命名函数） |
| 内联函数表达式 | 🚫 设计限制（要求用命名函数） |
| tagged template | 🚫 不实现 |
| private field | 🚫 不实现 |
| class expression | 🚫 不实现 |
| generator (yield) | 🚫 不实现 |
| dynamic import | 🚫 不实现 |
| JSX | 🚫 不实现 |

### 内置函数与运行时

| 类别 | 状态 |
|------|------|
| Math.*（abs/ceil/floor/round/max/min/pow/sqrt/random/PI 等） | ✅ |
| console.*（log/warn/error） | ✅ |
| JSON.*（stringify/parse） | ✅ |
| Date（含 now/ISO/get 方法） | ✅ |
| String 方法（indexOf/slice/split/toUpperCase 等） | ✅ |
| Array 方法 — 静态数组：indexOf/includes/map/filter/reduce/join/slice | ✅ |
| Array 方法 — 动态数组：push/pop/shift/unshift | ✅ |
| Array 方法 — 动态数组：splice/sort/reverse | ✅ |
| Map / Set（new/init/get/set/has/delete/size/keys/values/entries） | ✅ |
| RegExp（test/exec） | ✅ |
| Error（含 message/stack） | ✅ |
| Number（parseInt/parseFloat/isNaN/isFinite） | ✅ |
| URI（encodeURIComponent/decodeURIComponent） | ✅ |
| 全局 setTimeout/setInterval → 占位 | 🔶 |

---

## 🔴 P0 — 阻塞项

- [x] **添加 JS 测试用例** — `in/` 目录已有完整测试文件
  - ✅ 基础类型：数字/字符串/布尔/null
  - ✅ 变量声明（含解构）
  - ✅ 函数（普通/递归/闭包）
  - ✅ 控制流：if/for/while/do-while/switch/try-catch
  - ✅ for-of（含解构）
  - ✅ 对象/数组操作（字段访问、索引访问）
  - ✅ 闭包（makeAdder → 箭头函数捕获外部变量）
  - ✅ 多模块交互（import/export）
  - ✅ builtins：Math.round/Math.sign/parseInt
  - ✅ class declaration
  - 文件清单：`main.js`, `math.js`, `string_utils.js`, `builtins.js`, `classes.js`
  - ✅ `cargo run` → `zig build` → `zig build test` 全部通过（27 个测试）

- [x] **README.md** — 项目介绍、快速开始、架构图、限制说明

---

## 🔵 P1 — 工程质量

- [x] **CI/CD** — GitHub Actions：`cargo check`、`cargo clippy`、Zig 构建测试
- [x] **Rust 单元测试** — parser (7 个)、preprocess (3 个) + bridge (3 个)、testgen (3 个) → 14 passed
- [x] **Host 函数配置化** — `host_config.json` + `HostFnRegistry::load_from_file()`
- [x] **错误信息改进** — `Diagnostic::with_span()` + `format_with_source()` 输出 `[line:col]`
- [x] **性能基准** — criterion 基准：parse 10.8µs / preprocess 101.5µs / pipeline 79.1µs

---

## ⚪ P2 — 远期

- [ ] **源码映射 (Source Map)** — Zig → JS 行号对应
- [ ] **增量编译** — 只重译变更文件
- [ ] **WASM 目标** — 生成 Zig → WASM
