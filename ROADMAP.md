# js2rust — 开发路线图 v3.0

> JS → Zig 源码转译器，Rust 实现。
> ~13,200 行 Rust + 19 个 Zig 运行时文件。
> `cargo test --workspace` 107 通过，0 clippy 警告，Zig 0.16 目标。
>
> **更新时间**: 2026-06-23 | 基于 1cf3348

---

## 执行概况

| 优先级 | 总任务 | 已完成 | 剩余 |
|--------|--------|--------|------|
| P0 — 阻塞项 | 6 | 6 | 0 |
| P1 — 工程质量 | 5 | 5 | 0 |
| P2 — 远期 | 3 | 3 | 0 |
| **P3 — 新增功能** | **18** | **18** | **0** |
| **代码 TODO** | **4** | **0** | **4** |

---

## 核心流水线（7 阶段，全部完成）

| 阶段 | 模块 | 状态 | 行数 |
|------|------|------|------|
| 1 | 预处理（import/export 解析、模块合并、命名冲突） | ✅ | 1186 |
| 2 | 类型推导（8-rule simplified + JSDoc annotations） | ✅ | 2365 |
| 3 | 代码生成（native_proto，语句/表达式/闭包/内置/类） | ✅ | ~3400 |
| 4 | 测试生成（AST 提取用例 → Zig test 块） | ✅ | 236 |
| 5 | Zig 项目输出（build.zig/build.zig.zon/lib.zig） | ✅ | 390 |
| 6 | Zig 构建 + 测试（自动 zig build/test） | ✅ | js2zig-build |
| 7 | 多文件分组（per-file codegen，独立 Zig 项目） | ✅ | js2zig-core |

---

## 语法覆盖率

### 语句 — 14/15 已支持（1 项故意不支持）

| 语法 | 状态 |
|------|------|
| const/let/var（含解构，按函数隔离 mutated_vars） | ✅ |
| function declaration（含 async、类方法、C ABI export） | ✅ |
| class declaration（→ Zig struct + methods） | ✅ |
| 表达式语句 | ✅ |
| return（含 try/catch 上下文） | ✅ |
| if/else if/else | ✅ |
| block `{}` | ✅ |
| for (init; test; update) | ✅ |
| for-of（含解构 for-of） | ✅ |
| for-in（→ HashMap iterator，含 static/dynamic 分派） | ✅ |
| while | ✅ |
| do-while | ✅ |
| break / continue | ✅ |
| switch | ✅ |
| try/catch/finally（→ Zig error union + defer，含 finally body inline） | ✅ |
| throw（→ error propagation: `return error.Error(msg)`） | ✅ |
| for-await-of | 🚫 `@compileError` |

### 表达式 — 30/36 已支持（6 项故意不支持）

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
| 函数调用（builtin 感知、闭包感知、宿主函数感知） | ✅ |
| new 表达式（Map/Set/类实例） | ✅ |
| 静态成员 `.`（含 `.length` → `.len`、动态对象 HashMap） | ✅ |
| 计算成员 `[]`（动态数组 .items、HashMap .get、struct 字段） | ✅ |
| 赋值 =, +=, -= 等（含动态对象 HashMap.put） | ✅ |
| 条件（三元）`? :` → `if/else` | ✅ |
| 数组字面量 `[_]T{}` | ✅ |
| 对象字面量 `.{}`（含 spread/override） | ✅ |
| 模板字符串（含插值 → `allocPrint`，类型感知 `{f}` vs `{}`） | ✅ |
| 箭头函数（闭包 struct 实例化，值/引用捕获） | ✅ |
| await → `io.async/defer/cancel/await` | ✅ |
| 括号/逗号/可选链 `?.`（real null check） | ✅ |
| 正则字面量 | ✅ |
| TypeScript 类型表达式（as/assert/!./satisfies/instantiation） | ✅ |
| 乘方 `**` → `std.math.pow(f64, ...)` | ✅ |
| NullLiteral → `null` | ✅ |
| 箭头函数单表达式 → `return expr;` | ✅ |
| 内联箭头函数 | 🚫 设计限制（要求用命名函数） |
| 内联函数表达式 | 🚫 设计限制（要求用命名函数） |
| tagged template | 🚫 不实现 |
| private field | 🚫 不实现 |
| class expression | 🚫 不实现 |
| generator (yield) | 🚫 不实现 |
| dynamic import | 🚫 不实现 |
| JSX | 🚫 不实现 |

### 内置函数与运行时（19 个 .zig 文件）

| 类别 | 状态 | 文件 |
|------|------|------|
| Math.*（abs/ceil/floor/round/max/min/pow/sqrt/random/PI/sin/cos/tan/log 等） | ✅ | builtins |
| console.*（log/warn/error） | ✅ | js_console.zig |
| JSON.*（stringify/parse，含 @type 注解自动反序列化） | ✅ | js_json.zig |
| Date（含 now/ISO/get 方法） | ✅ | js_date.zig |
| String 方法（indexOf/slice/split/toUpperCase/toLowerCase/includes/startsWith 等） | ✅ | js_string.zig |
| Array 静态（indexOf/includes/map/filter/reduce/forEach/join/slice/some/every） | ✅ | js_array.zig |
| Array 动态（push/pop/shift/unshift/splice/sort/reverse） | ✅ | js_array.zig |
| TypedArray（Uint8Array, fromU8, .length） | ✅ | js_typedarray.zig |
| Map / Set（new/init/get/set/has/delete/size/keys/values/entries） | ✅ | js_map.zig / js_set.zig |
| RegExp（test/exec） | ✅ | js_regexp.zig |
| Error（含 message/stack） | ✅ | js_error.zig |
| Number（parseInt/parseFloat/isNaN/isFinite） | ✅ | js_number.zig |
| URI（encodeURIComponent/decodeURIComponent） | ✅ | js_uri.zig |
| Object（动态 HashMap，静态 struct） | ✅ | js_object.zig |
| JsValue/JsAny（类型擦除 + format() 方法） | ✅ | jsvalue.zig / jsany.zig |
| Promise | ✅ | js_promise.zig |
| Allocator（双区 Arena，热/冷自动切换，600s 宽限期） | ✅ | js_allocator.zig |
| String ops（dupeZ/slice/span/format） | ✅ | string.zig |
| 全局 setTimeout/setInterval | 🔶 占位 | js_runtime.zig |

---

## 🔴 P0 — 阻塞项 ✅ 全部完成

- [x] **JS 测试用例** — 5 个 JS 文件、27 个 smoke test、zig build + test 通过
- [x] **README.md** — 项目介绍、快速开始、架构图、限制说明
- [x] **架构重构** — 删除 CLI，4 个 crate 发布 crates.io (v0.3.1)
- [x] **mutated_vars 按函数隔离** — 修复 const/var 作用域错误
- [x] **Codegen 修复** — infer.rs TemplateLiteral / stmt.rs for-loop type / testgen.rs expr_to_string / bridge lib.rs
- [x] **语法补齐** — for-in / splice-sort-reverse / 边缘语法标注

### 已发布 crate（v0.3.1）

- `js2zig-core` — 核心转译库
- `js2rust-bridge` — FFI 桥接 runtime（含 `build()` 单次构建 API）
- `js2rust-bridge-macro` — proc-macro（生成 FFI 绑定 + Host 函数桩）

---

## 🔵 P1 — 工程质量 ✅ 全部完成

- [x] **CI/CD** — GitHub Actions：`cargo check`、`cargo clippy`、Zig 构建测试
- [x] **Rust 单元测试** — 107 测试通过（含 87 native_proto + 14 旧测试 + 2 bridge + 2 test-lib）
- [x] **Host 函数配置化** — `HostFnRegistry` + `HostConfig` struct
- [x] **错误信息改进** — `Diagnostic::with_span()` + `format_with_source()`
- [x] **性能基准** — criterion 基准：parse 10.8µs / preprocess 101.5µs / pipeline 79.1µs

---

## ⚪ P2 — 远期 ✅ 全部完成

- [x] **源码映射 (Source Map)** — `sourcemap.rs` + 内联 `// @src(file:line)` + `source_map.json`
- [x] **增量编译** — `.build_cache.json` 哈希缓存，`--force` 强制重建
- [x] **WASM 目标** — `zig build wasm` (wasm32-wasi)，builtins.wasm 1015KB

---

## 🟢 P3 — 新增功能 ✅ 全部完成

以下功能在 2026-06-19 至 2026-06-23 期间实现：

### 语言特性

| 功能 | 状态 | commit | 描述 |
|------|------|--------|------|
| throw→error 传播 | ✅ | `881e333` | `return error.Error(msg)` 端到端 |
| `**` 指数运算符 | ✅ | `643d61a` | `std.math.pow(f64, ...)` |
| 箭头函数（单表达式） | ✅ | `2817ce1` | `return expr;` |
| 箭头函数闭包 | ✅ | `25b3794` | closure struct + 值/引用捕获 |
| 可选链 `?.` | ✅ | `ddde803` | real null checking |
| 不支持特性 → @compileError | ✅ | `b2d666a` | 优雅降级 |
| 字符串转义（控制字符） | ✅ | `1840d7f` | `\n` `\t` `\\` `\"` 正确转义 |
| for-in（static/dynamic） | ✅ | `b2d666a` | HashMap iterator 分派 |

### 类型系统

| 功能 | 状态 | commit | 描述 |
|------|------|--------|------|
| JSDoc 解析器 | ✅ | `05366bd` | @param/@returns/@typedef/@property |
| @typedef struct 生成 | ✅ | `d077a3c` | 自定义类型 struct 定义 |
| @type JSON.parse | ✅ | `78c59a8` | 自动反序列化到强类型 |
| toJson() 序列化 | ✅ | `9ddc14a` | `std.json.fmt()` 序列化 |
| 8-rule 简化类型推断 | ✅ | `e8e8f31` | 含 fn_return_types 缓存 |
| async 宿主返回类型推断 | ✅ | `2a9094d` | struct 字段类型解析 |
| TypedArray 类型支持 | ✅ | `1840d7f` | .length 属性访问 |

### 宿主函数 (Host Functions)

| 功能 | 状态 | commit | 描述 |
|------|------|--------|------|
| 零拷贝 Host 调用 | ✅ | `7c8822b` | 参数 ptr+len 直传，返回值 Arena 分配 |
| 双区 Arena 分配器 | ✅ | `6920b11` | 热/冷自动切换，600s 宽限期 |
| js_allocator_alloc C ABI | ✅ | `6920b11` | Rust → Zig Arena 直写 |
| StrRet sign-bit panic | ✅ | `a8036a6` | 错误名通过 sign-bit 传递 |
| 单次 cargo build | ✅ | `7c8822b` | `js2rust_bridge::build()` 消除二次构建 |
| 多 JS 文件 bridge | ✅ | `babbd6a` | `js2rust_bridge!("group1", "group2")` |

### 内置对象

| 功能 | 状态 | commit | 描述 |
|------|------|--------|------|
| Math 完整方法 | ✅ | `f560ffc` | sin/cos/tan/log/PI 等 |
| Array push/pop/shift 等 | ✅ | `b682750` | 动态数组全系列 |
| Array indexOf/includes 等 | ✅ | `70194ab` | 静态数组全系列 |
| Array map/filter/reduce/forEach | ✅ | `077648f` | 回调闭包支持 |
| Map/Set 全系列 | ✅ | `dee776a` | new/set/get/has/delete 等 |
| try/catch/finally | ✅ | `7795992` | Zig error union + defer + finally inline |

### 架构优化

| 功能 | 状态 | commit | 描述 |
|------|------|--------|------|
| native_proto/ 模块拆分 | ✅ | `f4ec1bb`/`8dd5c37` | codegen/ + infer/ 子模块 |
| 删除旧 codegen/ | ✅ | `4ca440b` | 完全切换到 native_proto |
| 消除双解析 | ✅ | `2a46890` | AST 从 analyzer 传给 transpile_js |
| Arena 自动重置 | ✅ | `67dcee2` | 线程安全 + 内存上限 + 环境变量 |
| Zig 0.16.0 兼容 | ✅ | `7795992` | ArrayList API 变更适配 |

---

## 🔶 剩余任务 — 代码 TODO（4 项）

### TODO-1: catch 参数映射 `e` → `err` 
- 位置：`js2zig-core/src/native_proto/codegen/stmt.rs:629`
- 问题：catch body 中引用 `e` 时未映射到 Zig 的 `err`
- 影响：`try { ... } catch(e) { console.log(e) }` — `e` 未定义
- 难度：低

### TODO-2: 箭头闭包变异检测
- 位置：`js2zig-core/src/native_proto/codegen/stmt.rs:1333`
- 问题：箭头函数中 `is_mut` 硬编码为 `false`
- 影响：闭包中修改捕获变量时生成错误代码
- 难度：中（需要分析箭头函数体内的赋值）

### TODO-3: 箭头函数返回类型推断
- 位置：`js2zig-core/src/native_proto/codegen/stmt.rs:1481`
- 问题：返回值类型硬编码为 `i64`
- 影响：箭头函数返回字符串/f64/bool 时类型错误
- 难度：中（需要 `fn_return_types` 查找 ArrowFunctionExpression）

### TODO-4: for-of 循环类型推断
- 位置：`js2zig-core/src/native_proto/tests.rs:447`
- 问题：for-of 循环变量类型推断不完整
- 影响：复杂 for-of 场景下类型可能错误
- 难度：中

---

## 🔶 当前不支持的场景（降级为 @compileError）

| 场景 | 位置 | 说明 |
|------|------|------|
| 成员函数调用 `obj.method()` | expr.rs:570 | 用户自定义 class/object 的方法调用 |
| Spread 参数 `fn(...args)` | expr.rs:1400 | 展开运算符作为函数参数 |
| 空值类型推断 `null` | expr.rs:1805 | null literal 的简化类型系统限制 |
| exports `{}` 语法 | mod.rs:126 | `export { foo, bar }` 重新导出语法 |

---

## 📋 下一步建议（按优先级排序）

### Phase A: 代码 TODO 修复（P0 等价）
```
1. TODO-1: catch e→err 映射 ────── 低难度，直接修改
2. TODO-3: 箭头函数返回类型推断 ── 需要类型信息集成
3. TODO-4: for-of 类型推断 ─────── 需要类型信息集成
4. TODO-2: 箭头闭包 is_mut 检测 ── 需要赋值分析
```
预计工作量：中等。优先 1→3→4→2 顺序。

### Phase B: 降低 @compileError 覆盖率（P1）
```
5. Member function calls ── 支持 obj.method() 转译为 struct 方法调用
6. Spread arguments ──────── 支持 fn(...args) 展开
7. exports {} syntax ─────── 支持 export 重新导出
```
预计工作量：高。member function calls 是最有价值的一项。

### Phase C: 版本发布与文档（P2）
```
8. 版本 bump: 0.3.1 → 0.4.0（重大功能累积）
9. API 文档：js2rust_bridge::build() / HostConfig / BuildConfig
10. 用户指南：零拷贝 Host 函数编写指南
```

### Phase D: 远期特性（P3）
```
11. setTimeout/setInterval 真实实现（非占位）
12. Class 方法调用支持（当前只有静态方法）
13. 零拷贝 vs 拷贝模式性能对比基准
14. WASM 目标端到端测试
```

---

## 项目结构（当前）

```
js2rust/                          # workspace root
├── Cargo.toml                    # workspace 配置
├── ROADMAP.md                    # 本文件
├── README.md
│
├── js2zig-core/          [发布]  # 核心转译库 v0.3.1
│   ├── Cargo.toml
│   ├── build.rs                  # 嵌入 runtime/*.zig
│   └── src/
│       ├── lib.rs                # 公开 API
│       ├── pipeline.rs           # 编排逻辑
│       ├── project.rs            # lib.zig/build.zig 生成
│       ├── host.rs               # host.zig 生成
│       ├── analyzer.rs           # JSDoc + AST 分析
│       ├── parser.rs / testgen.rs / sourcemap.rs
│       ├── native_proto/         # 主 codegen（已替代旧 codegen/）
│       │   ├── mod.rs            # 入口 + export 检测
│       │   ├── codegen/
│       │   │   ├── mod.rs
│       │   │   ├── stmt.rs       # 语句生成
│       │   │   ├── expr.rs       # 表达式生成
│       │   │   └── helpers.rs
│       │   ├── infer/
│       │   │   ├── mod.rs        # 8-rule 类型推断
│       │   │   ├── expr.rs
│       │   │   ├── fn_types.rs
│       │   │   ├── passes.rs
│       │   │   └── helpers.rs
│       │   ├── builtins.rs       # 内置函数调度
│       │   ├── jsdoc.rs          # JSDoc 解析器
│       │   └── tests.rs          # 87 测试
│
├── js2rust-bridge/       [发布]  # FFI 桥接 v0.3.1
│   ├── Cargo.toml
│   └── src/lib.rs                # build() / link() / BuildConfig
│
├── js2rust-bridge-macro/ [发布]  # proc-macro v0.3.1
│   ├── Cargo.toml
│   └── src/lib.rs                # js2rust_bridge!() + Host 函数桩
│
├── runtime/               [源]   # Zig 运行时源文件（19 个 .zig）
│   ├── js_allocator.zig          # 双区 Arena 分配器
│   ├── js_array.zig / js_map.zig / js_set.zig
│   ├── js_console.zig / js_date.zig / js_error.zig
│   ├── js_json.zig / js_number.zig / js_object.zig
│   ├── js_promise.zig / js_regexp.zig / js_runtime.zig
│   ├── js_string.zig / js_typedarray.zig / js_uri.zig
│   ├── jsvalue.zig / jsany.zig / string.zig
│
├── examples/
│   ├── test-bin-project/         # 零拷贝 Host 函数端到端
│   ├── test-lib-project/         # C ABI 导出
│   └── showcase-project/         # 40+ 语法特性展示
│
└── docs-archive/                 # 历史文档归档
```

---

## 版本历史

| 版本 | 日期 | 关键变更 |
|------|------|----------|
| v0.1.0 | 2026-06-18 | 初始发布：4 crate，基础转译 |
| v0.2.0 | 2026-06-19 | async/string host functions，Source Map，WASM |
| v0.3.0 | 2026-06-20 | native_proto 切换，JSDoc 类型注解 |
| v0.3.1 | 2026-06-22 | 双区 Arena，零拷贝 Host 函数，单次构建 |
| v0.4.0 | 待定 | Phase B 完成后发布（成员函数调用 + spread + exports {}） |
