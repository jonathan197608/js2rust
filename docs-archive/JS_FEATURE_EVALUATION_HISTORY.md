# JS 语言特性实现评估 - 历史归档

> 本文件归档了 JS_FEATURE_EVALUATION.md 的修改历史和任务实施记录。
> 归档日期: 2026-06-27

---

## 更新日志

| 日期 | 版本 | 主要变更 |
|------|------|----------|
| 2026-06-27 | v2.28 | String.matchAll() 完成：`host_regex_match_all` (Rust fancy-regex captures_from_pos) + `matchAllString` (Zig runtime, JsAny 数组) + codegen + 2 测试。P3 剩余 3 项 |
| 2026-06-27 | v2.27 | Symbol well-known symbols 完成：14 个 `Symbol.<name>` 静态属性访问 codegen + 类型推断 + runtime 14 工厂函数 + project.rs 导入 + 4 测试。P2 全部清零。Rust 测试 302 |
| 2026-06-27 | v2.26 | 从严评估修正（Phase A）：4 个 String 方法 -> 简化实现（localeCompare/normalize/toLocaleUpperCase/toLocaleLowerCase，因 ICU 依赖不可行）；全局统计更新；4.3 String 28+4/35 |
| 2026-06-27 | v2.23 | `#field` 私有字段 实现（ES2022 class 私有字段，`#` 前缀剥离 -> Zig 结构体字段 + 默认值保留） |
| 2026-06-27 | v2.19 | `test_p3_mixed_decl_expr_unused_var` -> known-expected：Zig "unused local constant" 是特性，不应抑制；3.6 "其他语句" -> 完全实现 |
| 2026-06-27 深夜 | v2.17 | 全面重算 4.17 汇总表：P2 ~25->~7, P3 ~16->~8, String 30->32, Global 7->8, RegExp P3->0, 8.3 重构为 P2/P3 双表, 9 重写为剩余计划 |
| 2026-06-27 深夜 | v2.18 | typeof 完成：静态类型映射 JS typeof 字符串 + 动态类型 jsTypeof() runtime helper |
| 2026-06-27 晚间 | v2.15 | 测试计数同步 275->281、Math 44/44 100%、Section 7.1 测试模块分解 |
| 2026-06-27 #4 | - | `Symbol` + iterable 协议决策 |
| 2026-06-27 #3 | - | `new Date()` 构造函数重载完成（8 个测试） |
| 2026-06-27 #2 | - | `obj[key]` 计算属性 + `String.match()` Phase 1-3（+17 测试） |
| 2026-06-26 | - | Phase 8(`encodeURI/decodeURI`)、Phase 7(`Set.forEach`)、Phase 6(String 高级方法)、Phase 5(Object/Date setter)、Phase 4(14 任务全部完成) |
| 2026-06-25 | - | 内置对象覆盖重新评估（~22% 修正） |
| 2026-06-24 | - | P0/P1 内置对象全部连线（~53%）、MDN 标准对齐、5 个状态修正 + 8 个遗漏特性 |

---

## 特征实现重新评估 (2026-06-27)

### 评估原则

对所有未实现特征进行重新评估，按**应用价值**细分为三类：

| 分类 | 标记 | 定义 | 策略 |
|------|------|------|------|
| **高价值（P2）** | P2 | 常用 JS 特征，实际项目需要 | 全部完成（Symbol well-known symbols 已实现） |
| **中价值（P3）** | P3 | 偶尔使用，有 workaround | 全部结案：已实现或降级为不实现 |
| **不实现（低价值）** | 不实现 | 很少用，或 Zig 有更好替代 | 永不实现（~63 项） |

### 不实现特征清单

以下特征因**应用价值低**或**已有更好替代方案**，标记为不实现：

**语法（表达式/语句）：**
- `with` 语句 - JS 严格模式已废弃，绝不实现
- `debugger` 语句 - 调试用，Zig 有自身调试工具
- 标签模板 - 很少使用
- 类表达式 `const X = class {}` - 很少使用，可用 `class X {}` 替代
- 静态初始化块 `static {}` - ES2022，使用较少
- `for await...of` - 异步迭代，当前项目聚焦同步代码
- `eval(s)` - 安全风险，编译时无法动态执行
- `new.target` - meta property，niche 场景
- `import.meta` - ES 模块元数据，niche
- `arguments` 对象 - 传统函数，箭头函数已替代

**内置对象（低价值）：**
- Array `.with()/.toReversed()/.toSorted()/.toSpliced()` - ES2023 不可变方法，有可变版本替代
- Set ES2025 操作（`.difference/.intersection/...`） - 很新，使用较少
- `String.raw` - 标签模板，很少使用
- `Object.getOwnPropertySymbols` - Symbol 属性名，很少使用
- `RegExp` `.source/.flags` 等属性 - 高级正则用法，很少用
- `Promise` `new Promise()/.then/.catch` - 建议用 `async/await` + `Io` 模式替代
- `WeakMap` / `WeakSet` - 弱引用，Zig 内存管理不同
- `Reflect` - 反射 API，Zig 不需要
- `Intl` - 国际化，可调用 Zig/C 库
- `Atomics` - 共享内存原子操作，niche 场景
- `Map.groupBy()` / `Object.groupBy()` - ES2024 静态分组方法，应用层逻辑（可用 Map + for 循环替代）
- `BigInt` - 大整数类型，Zig 原生 i64/i128 已提供等价能力

### P3 结案（全部完成或降级）

#### P2 高价值 - 全部完成

| 特征 | 说明 | 状态 |
|------|------|------|
| **Symbol well-known symbols** | `Symbol.iterator/asyncIterator/hasInstance/isConcatSpreadable/species/toPrimitive/toStringTag/unscopables/match/matchAll/replace/search/split/dispose` 等 14 个静态属性访问 | 完成 - codegen `StaticMemberExpression` 检测 + runtime 14 个工厂函数 + 类型推断 + 4 个测试 |

> 所有 P2 高价值项已全部完成。Symbol well-known symbols 通过 `codegen/expr.rs` 的 `StaticMemberExpression` handler 检测 `Symbol.<name>`，映射到 `js_symbol.symbolXxx()` 运行时调用。类型推断返回 `ZigType::JsSymbol`。

#### P3 结案明细

| 特征 | 类别 | 说明 | 最终状态 |
|------|------|------|----------|
| `String.matchAll()` | 内置对象 | 正则全局匹配迭代器 | 已实现 - `host_regex_match_all` (Rust) + `matchAllString` (Zig) + codegen + 2 测试 |
| `Map.groupBy()` | 内置对象 | ES2024 静态分组 | 降级为不实现 - 应用层逻辑，可用 Map + for 循环替代 |
| `Object.groupBy()` | 内置对象 | ES2024 静态分组 | 降级为不实现 - 应用层逻辑，可用 Object + for 循环替代 |
| `BigInt` | 内置对象 | 大整数类型 + 5+ 方法 | 降级为不实现 - Zig 原生 i64/i128 替代，无需独立 BigInt 类型 |

### 覆盖率变化

| 指标 | 之前 (v2.7) | 上次更新 (v2.15) | 当前 (v2.29) | 变化 |
|------|----------|----------|----------|------|
| 语法完全实现 | ~109 (~71%) | ~113 (~74%) | ~130 (~86%) | +17 |
| 语法部分实现 | ~10 (~7%) | ~8 (~5%) | ~4 (~3%) | -4 |
| 语法计划实现 (P3) | ~19 (~13%) | ~18 (~12%) | ~0 (~0%) | -18 |
| 语法不实现 | ~13 (~9%) | ~13 (~9%) | ~16 (~11%) | +4 |
| 内置对象有效覆盖率 | ~193/310 (~62%) | ~242/310 (~78%) | ~240+4/310 (~79%) | +2 |
| 内置对象 P2 剩余 | - | ~25 (~8%) | ~5 (~2%) | -20 |
| 内置对象 P3 剩余 | - | ~16 (~5%) | ~0 (~0%) | -16 |
| 内置对象 简化实现 | - | ~0 | ~4 (~1%) | +4 |
| 内置对象 不实现 | - | ~40 (~13%) | ~63 (~20%) | +23 |
| Rust 测试 | 246 | 281 | 304 | +23 |
| Math 覆盖率 | ~98% | 98% | 100% (44/44) | - |

---

## P3 结案报告 (2026-06-27 晚间)

> **状态**: 所有 P0/P1/P2/P3 特征已全部结案。P3 项要么已实现，要么降级为不实现。

### P2: Symbol well-known symbols - 已完成

**实现内容**:
- `codegen/expr.rs`: `StaticMemberExpression` handler 检测 `Symbol.<name>`，映射到 `js_symbol.symbolXxx()` 运行时调用
- `infer/expr.rs`: `Symbol.<well-known>` -> `ZigType::JsSymbol` 类型推断
- `runtime/js_symbol.zig`: 14 个 well-known symbol 工厂函数完整实现
- `project.rs`: 添加 `js_symbol` / `JsSymbol` 导入到项目输出
- 4 个测试: `test_native_proto_symbol_well_known_iterator` / `_async_iterator` / `_multiple` / `_to_string_tag`

> **注意**: for-of Map/Set 使用 direct `.inner.iterator()` 模式（基于类型推导），不通过 `Symbol.iterator` 协议。这是务实选择：类型信息已足够，无需完整的 Symbol iterable 抽象层。

### P3 结案明细

| # | 特征 | 类别 | 最终状态 | 降级理由 |
|---|------|------|----------|----------|
| 1 | `String.matchAll()` | 内置对象 | 已实现 | `host_regex_match_all` + `matchAllString` + codegen + 2 测试 |
| 2 | `Map.groupBy()` | 内置对象 | 不实现 | 应用层逻辑 - ES2024 静态分组方法，可用 `Map` + `for` 循环 3 行代码替代，不值得引入 BuiltinCall 变体 |
| 3 | `Object.groupBy()` | 内置对象 | 不实现 | 同上 - 返回普通对象 vs Map 的区别，同样可用 `for` 循环替代 |
| 4 | `BigInt` | 内置对象 | 不实现 | Zig 原生 `i64`/`i128` 已提供大整数能力；JS BigInt 的任意精度语义与 Zig 固定宽度整数模型不兼容，实现完整 BigInt 类型工作量过大且价值低 |

> **从严评估 (2026-06-27)**: `instanceof`（Zig 无运行时原型链）、`function*`/`yield`（状态机变换极复杂，Zig 无等价物）、动态 `import()`（Zig `@import()` 仅 comptime）3 项从 P3->不实现。Map.groupBy/Object.groupBy/BigInt 同理降级。**P3 队列已清空。**

---

文档版本: 2.29
最后更新: 2026-06-27 (P3 全部结案)
作者: jonathan197608
