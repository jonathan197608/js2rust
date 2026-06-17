# js2rust

**JavaScript → Zig 源码翻译器**

将 JavaScript（ES2022 子集）编译为可读、可编译的 Zig 源码，附带完整的运行时库。Rust 实现，~10,000 行代码。

---

## 快速开始

**前置条件：** Rust 1.80+, Zig 0.16.0

```bash
# 1. 在 in/ 目录放入 JS 文件
echo 'export function add(a, b) { return a + b; }' > in/main.js

# 2. 运行翻译流水线
cargo run

# 输出：
#   Generated: out/js2rust (single Zig library)
#   Zig build: OK
#   Zig tests: PASSED
```

`in/` 目录已包含覆盖性测试用例，可直接运行：
```bash
cargo run
# 27 个 smoke test 全部通过
```

---

## 架构

```
 in/*.js                 out/
 ───────                 ─────
  │  预处理              输出目录 ← zig build
  │  import/export      │
  │  模块合并            │   cabi_exports.json
  │                     │
  ▼                     │
 解析 (oxc)             │   js2rust/
  │                     │   ├── build.zig
  ▼                     │   ├── build.zig.zon
 类型推导                │   ├── src/
  │  12 种 Zig 类型      │   │   ├── lib.zig
  │  Union/Optional/     │   │   └── js_runtime/*
  │  Object/Array        │   └── tests/
  │                     │
  ▼                     ▼
 代码生成               zig build
  │  语句/表达式/         │
  │  闭包/类/内置函数     ▼
  │                     zig build test
  ▼
 测试生成
  test_* 变量 → Zig test blocks
```

**7 阶段流水线：**

| # | 阶段 | 职责 |
|---|------|------|
| 1 | 预处理 | import/export 解析、模块合并、命名冲突处理 |
| 2 | 解析 | oxc AST（支持 ES2022 + TS 类型标注） |
| 3 | 类型推导 | 推断 Zig 类型（i64/f64/String/Array/Object/Union 等） |
| 4 | 代码生成 | JS AST → 整洁可读的 Zig 源码 |
| 5 | 测试生成 | `test_*` 变量 → Zig test 块（冒烟测试） |
| 6 | 项目输出 | 生成 `build.zig` + `build.zig.zon` + `lib.zig` |
| 7 | 构建+测试 | 自动调用 `zig build` 和 `zig build test` |

---

## 语法支持

### ✅ 支持

| 类别 | 覆盖 |
|------|------|
| **变量** | `const`/`let`/`var`，含解构（对象/数组/嵌套） |
| **函数** | 声明、参数、递归、闭包（箭头函数捕获） |
| **控制流** | `if/else`、`switch`、`for(;;)`、`for-of`、`for-in`、`while`、`do-while`、`break`/`continue` |
| **错误** | `try/catch/finally`、`throw` |
| **类** | `class` 声明、`constructor`、`this` 方法 |
| **表达式** | 算术/比较/逻辑/位运算、三元、模板字符串 |
| **对象/数组** | 字面量、`.`/`[]` 访问、spread、赋值 |
| **异步** | `async`/`await`（→ Zig `io.async`） |
| **模块** | `import`/`export` 多文件支持 |
| **类型标注** | TS `as`/`!`/`satisfies` 类型表达式 |
| **可选链** | `?.` |
| **正则** | 字面量、`test`/`exec` |

### 内置函数

| 命名空间 | 方法 |
|----------|------|
| `Math` | abs/ceil/floor/round/max/min/pow/sqrt/random/sin/cos/PI 等 |
| `console` | log/warn/error |
| `JSON` | stringify/parse |
| `Date` | now/getTime/getFullYear/getMonth 等 |
| `String` | indexOf/slice/split/toUpperCase/startsWith/replace 等 |
| `Array` | push/pop/shift/unshift/indexOf/includes/map/filter/slice/splice/sort/reverse |
| `Map`/`Set` | new/get/set/has/delete/keys/values |
| `RegExp` | test/exec |
| `Number` | parseInt/parseFloat/isNaN/isFinite |
| `URI` | encodeURIComponent/decodeURIComponent |
| Global | parseInt/parseFloat/isNaN/isFinite |

### 🚫 不实现（边缘语法）

- tagged template literals
- class expression / private fields
- generator / yield
- dynamic import
- JSX
- for-await-of

### 🔧 设计限制

- 内联箭头函数 / 函数表达式 → 必须改写为命名函数（`@compileError` 提示）
- 字符串返回函数需 C ABI 兼容处理（运行时内存管理）

---

## 运行时

`runtime/` 目录包含 15 个 Zig 运行时文件：

| 模块 | 用途 |
|------|------|
| `jsvalue.zig` | JsValue tagged union |
| `js_allocator.zig` | 全局分配器 |
| `js_string.zig` | 字符串操作 |
| `js_array.zig` | 数组操作 |
| `js_object.zig` | 对象键枚举（HashMap） |
| `js_map.zig` / `js_set.zig` | Map/Set |
| `js_console.zig` | 控制台日志 |
| `js_json.zig` | JSON  |
| `js_date.zig` / `js_regexp.zig` | Date/RegExp |
| `js_number.zig` / `js_error.zig` | Number/Error |
| `js_uri.zig` | URI 编解码 |
| `js_runtime.zig` | 运行时入口 |

---

## 项目结构

```
js2rust/
├── in/                  # JS 输入文件
│   ├── main.js          # 主编译文件（含 27 个测试）
│   ├── math.js          # 数学实用函数模块
│   ├── string_utils.js  # 字符串实用函数模块
│   ├── builtins.js      # 内置函数测试
│   └── classes.js       # 类声明测试
├── js2rustc/            # Rust 核心库 + CLI
│   └── src/
│       ├── main.rs      # 7 阶段流水线编排
│       ├── parser.rs    # oxc 解析器
│       ├── preprocess.rs # import/export 模块系统
│       ├── infer.rs     # 类型推导
│       ├── codegen/     # Zig 代码生成
│       │   ├── mod.rs   # 入口 + helpers
│       │   ├── stmt.rs  # 语句生成
│       │   ├── expr.rs  # 表达式生成
│       │   ├── fn_decl.rs # 函数声明
│       │   ├── closure.rs # 闭包
│       │   └── builtins.rs # 内置函数
│       ├── testgen.rs   # 测试用例提取
│       ├── project.rs   # Zig 项目文件生成
│       ├── builtins.rs  # 内置函数注册表
│       ├── host.rs      # Host 函数 FFI
│       └── analyzer.rs  # 文件组分析
├── js2rust-bridge/      # Rust ↔ Zig FFI 桥接
├── runtime/             # Zig 运行时库
├── out/                 # 编译输出（自动生成）
├── tests/               # Rust 集成测试
└── ROADMAP.md           # 开发路线图
```

---

## Host 函数

JS 可以通过 C ABI 调用 Rust 函数：

```js
// JS 侧
const result = hostAdd(10, 20);          // → Rust host::hostAdd
const user = await fetchUser("Alice");    // → Rust host::hostFetchUser
```

当前注册的 host 函数（`main.rs`）：

| JS 函数 | Rust 函数 | 签名 |
|---------|-----------|------|
| `hostAdd` | `host::hostAdd` | (i64, i64) → i64 |
| `hostMultiply` | `host::hostMultiply` | (i64, i64) → i64 |
| `fetchUser` | `host::hostFetchUser` | (name: string) → UserInfo { id, name } |

---

## 许可

MIT
