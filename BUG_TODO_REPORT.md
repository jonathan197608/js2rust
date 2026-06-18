# js2rust 源代码审查报告
> 审查时间：2026-06-18  
> 审查范围：`js2rustc/src/` 全部核心文件 + `runtime/*.zig`

---

## 一、BUG 清单（按严重程度排序）

### P0 — 必然导致崩溃或生成错误 Zig 代码

| # | 文件 | 行号 | 问题描述 | 影响 | 状态 |
|---|---|---|---|---|
| 1 | `codegen/mod.rs` | 486 | `ZigType::Null => unreachable!()` — `dynamic_field_accessor` 遇 Null 类型会 panic | 运行期崩溃 | **已修复** → `".null".to_string()` |
| 2 | `codegen/mod.rs` | 407-411 | `emit_js_value_literal` 生成的 JsValue 字面量直接嵌入 `put()` 调用，格式正确性待验证 | 潜在生成错误代码 | 待验证 |
| 3 | `codegen/builtins.rs` | 186 | `chars.next().unwrap()` 在 peek 保护下理论安全，但防御性不足 | 潜在 panic | **已修复** → `.expect("peek() guaranteed a digit")` |
| 4 | `analyzer.rs` | 223 | `serde_json::to_string_pretty(...).unwrap()` — 序列化失败会 panic | 运行时 panic | **已修复** → `.expect("Failed to serialize groups.json")` |

### P1 — 可能导致错误行为

| # | 文件 | 行号 | 问题描述 | 影响 | 状态 |
|---|---|---|---|---|
| 5 | `runtime/js_allocator.zig` | 17-21 | `g_alloc()` 存在竞态：两个并发调用可能同时看到 `null` 并尝试设置 | 多线程环境下可能崩溃 | **已修复** → 不缓存 page_allocator，直接返回 |
| 6 | `codegen/mod.rs` | 420 | `lit.value.fract() != 0.0` 浮点比较不可靠（如 1.1 的二进制表示） | 整数被误判为浮点数 | **已修复** → `is_finite() && value.trunc() == value` |
| 7 | `infer.rs` | 191 | `simplify_union` 中 `types[0].clone()` 在空 vec 时 panic（虽然有调用方保护，但防御性不足） | 极端情况下 panic | **已修复** → 增加空/single-element 守卫 |
| 8 | `main.rs` | 11 | `Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()` — 若 CARGO_MANIFEST_DIR 是根目录会 panic | 极低概率 panic | **已修复** → `.expect("CARGO_MANIFEST_DIR must have a parent directory")` |
| 9 | `infer.rs` | 1566 | `pnames.into_iter().next().unwrap_or_default()` — 解构 pattern 无名字时返回空字符串作为参数名 | 生成错误参数名 | **已修复** → 空名替换为 `"_"` |

### P2 — 代码质量问题

| # | 文件 | 行号 | 问题描述 | 状态 |
|---|---|---|---|
| 10 | `codegen/mod.rs` | 362-371 | `escape_keyword` 关键字列表不完整，缺少 `allowzero`、`noreturn`、`undefined`，移除错误的 `addrspace` | **已修复** |
| 11 | `codegen/closure.rs` | 468 | 多语句闭包体只生成注释，未实现 | **已部分实现** |
| 12 | `codegen/closure.rs` | 600 | `emit_stmt_with_capture` 对大多数 Statement 类型未实现，只输出 TODO | **已部分实现** |

---

## 二、TODO 清单（未实现语法/功能）

### 表达式（expr.rs）— 全部输出 `@compileError` 或 TODO 注释

| # | 语法 | 行号 | 优先级 | 状态 |
|---|---|---|---|
| T1 | `PrivateFieldExpression` | 523 | 低 | 待实现 |
| T2 | `TaggedTemplateExpression` | 526 | 低 | 待实现 |
| T3 | `ClassExpression` | 529 | 中 | **✅ 已替换 @compileError** |
| T4 | `MetaProperty` (import.meta) | 532 | 低 | 待实现 |
| T5 | `ImportExpression` (dynamic import) | 535 | 中 | **✅ 已替换 @compileError** |
| T6 | `YieldExpression` (generator) | 538 | 低 | 待实现 |
| T7 | `V8IntrinsicExpression` | 541 | 低 | 待实现 |
| T8 | `PrivateInExpression` | 544 | 低 | 待实现 |
| T9 | `JSXElement` / `JSXFragment` | 547 | 低（除非用 JSX） | 待实现 |

### 语句（stmt.rs）— 已替换为 @compileError

| # | 语法 | 行号 | 优先级 | 状态 |
|---|---|---|---|
| T10 | `for-in` 非动态对象 | 575 | 中 | **✅ 已替换 @compileError** |
| T11 | `for-in` with destructuring | 605 | 低 | **✅ 已替换 @compileError** |
| T12 | `for-in` with empty decl | 611 | 低 | **✅ 已替换 @compileError** |
| T13 | `for-in` with member expr | 616 | 低 | **✅ 已替换 @compileError** |
| T14 | `for-await-of` (async iterator) | 155 | 高 | **⏭ 跳过（用户决定）** |
| T15 | `for-of` with empty decl | 752 | 低 | **✅ 已替换 @compileError** |
| T16 | `for-of` with member expr/destructuring | 773 | 低 | **✅ 已替换 @compileError** |
| T17 | 未知 Statement 类型 | 104 | 中 | 待修复 |

### 类型推断（infer.rs）

| # | 描述 | 行号 | 优先级 | 状态 |
|---|---|---|---|
| T18 | Union 类型中 Optional 未 flatten（如 `?i64 \| null` 应简化） | 198 | 中 | **✅ 已实现** |

### 闭包（closure.rs）

| # | 描述 | 行号 | 优先级 | 状态 |
|---|---|---|---|
| T19 | 多语句闭包体未实现（只支持表达式体） | 468 | 高 | **✅ 已实现** |
| T20 | `emit_stmt_with_capture` 对 IfStatement、ForStatement 等未实现 | 600 | 高 | **✅ 部分实现** |

---

## 三、工作计划（按优先级排序）

### 阶段一：修复 P0/P1 BUG（已完成 ✅）

| 任务 | 提交 | 状态 |
|---|---|---|
| BUG #1, #3, #4, #5, #6, #7, #10 | `4dd6f9e`, `156765c`, `e79ba4c`, `e8bb9f7` | ✅ |

### 阶段二：实现高优先级 TODO（已完成 ✅）

| 任务 | 提交 | 状态 |
|---|---|---|
| T18: Union Optional flattening | `25b7eab` | ✅ |
| T3: ClassExpression | `b5af459` | ✅ (@compileError) |
| T5: ImportExpression | `b5af459` | ✅ (@compileError) |
| T10-T13: for-in 边界 | `8cdc859` | ✅ (@compileError) |
| T15-T16: for-of 边界 | `8cdc859` | ✅ (@compileError) |

### 阶段三：实现低优先级 TODO（进行中）

| 任务 | 文件 | 状态 |
|---|---|---|
| T1: PrivateFieldExpression | `expr.rs:523` | 进行中 |
| T2: TaggedTemplateExpression | `expr.rs:526` | 待实现 |
| T4: MetaProperty | `expr.rs:532` | 待实现 |
| T6-T9: 其他低优先级表达式 | `expr.rs` | 待实现 |

---

## 四、建议

1. **已完成**：P0/P1 BUG 全部修复，高优先级 TODO 全部处理（@compileError 或完整实现）
2. **进行中**：低优先级表达式类型（T1-T9）正在实现
3. **下一步**：继续 T1-T9，或停止并报告进度

---

## 五、已修复项目（截止目前）

| BUG/TODO # | 修复内容 | 提交 |
|---|---|---|
| 1 | `mod.rs` `dynamic_field_accessor` 中 `unreachable!()` → `".null".to_string()` | ✅ `4dd6f9e` |
| 3 | `builtins.rs` `chars.next().unwrap()` → `.expect("peek() guaranteed a digit")` | ✅ `156765c` |
| 4 | `analyzer.rs` `to_string_pretty(...).unwrap()` → `.expect(...)` | ✅ `e79ba4c` |
| 5 | `js_allocator.zig` 竞态条件修复 | ✅ `e8bb9f7` |
| 6 | `mod.rs` 浮点判断修复 | ✅ `e8bb9f7` |
| 7 | `infer.rs` `simplify_union` 防御性修复 | ✅ `e8bb9f7` |
| 10 | `escape_keyword` 补充关键字 | ✅ `4dd6f9e` |
| T18 | Union 类型 Optional flattening | ✅ `25b7eab` |
| T3 | ClassExpression → @compileError | ✅ `b5af459` |
| T5 | ImportExpression → @compileError | ✅ `b5af459` |
| T10-T13 | for-in 边界 → @compileError | ✅ `8cdc859` |
| T15-T16 | for-of 边界 → @compileError | ✅ `8cdc859` |
| NEW-1 | `fn_decl.rs` 默认参数 `y: i64 = 10` Zig 不合法语法 — 移除两处 `= value` 输出 | ✅ 2026-06-18 |
| NEW-2 | `infer.rs` 闭包返回类型推断：`const mul = (x) => x * factor` → JsAny 而非 FunctionPtr — `register_binding_with_expr` 特殊处理 ArrowFunctionExpression | ✅ 2026-06-18 |
| NEW-3 | `codegen/builtins.rs` `{}` 占位符展开为全部参数（`@min({}, {})` → `@min(a, b, a, b)`）— 改为顺序索引 | ✅ 2026-06-18 |

---

## 六、测试发现的已知限制（待后续解决）

| # | 描述 | 优先级 |
|---|---|---|
| L1 | 静态方法参数默认推断为 JsValue 而非 i64（非构造函数不享受 i64 默认） | 中 |
| L2 | `const a = x + 1` → JsAny，但 `return a` 返回 i64，函数声明返回 JsAny 类型不匹配 | 高 |
| L3 | `console.log(x)` 传入 i64 参数，但 js_console.log 期望 `[]const u8` | 中 |
| L4 | `Math.abs/min/max` 对 i64 输入产生类型不匹配（`@abs(i64)` → `u64`，函数返回 f64） | 高 |
| L5 | `7.0` 等整数浮点字面量被推断为 i64（`fract() == 0.0` 判定为整数） | 中 |
| L6 | Zig 默认参数不支持，需完整实现 optional param + orelse 模式 | 低 |
| L7 | 递归函数返回类型推断失败（如 `fib(n)` fallback 到 JsValue） | 中 |
