---
AIGC:
  ContentProducer: '001191110102MAD55U9H0F10002'
  ContentPropagator: '001191110102MAD55U9H0F10002'
  Label: '1'
  ProduceID: '055f0674-21da-4859-acc2-a47d622ac330'
  PropagateID: '055f0674-21da-4859-acc2-a47d622ac330'
  ReservedCode1: 'afcd94b5-193e-420c-801e-cb8c30ccad19'
  ReservedCode2: 'afcd94b5-193e-420c-801e-cb8c30ccad19'
---

# R47 审计报告：emit 层与 lower 层 Bug

**审计范围**: `js2zig-core/src/zigir/emit/` 和 `js2zig-core/src/zigir/lower/`  
**审计日期**: 2026-08-02  
**审计人**: TeleAgent  

---

## Bug 汇总

| Bug ID | 优先级 | 文件 | 简述 |
|--------|--------|------|------|
| BUG-1 | P1 | `emit/builtins/string.rs:221-222` | startsWith/endsWith 丢弃 position 参数 |
| BUG-2 | P1 | `emit/builtins/object.rs:326-350` | Object.groupBy 非字符串 key 类型不匹配 |
| BUG-3 | P1 | `emit/builtins/array_callback.rs` (多处) | 数组回调方法硬编码 `.items`，rest 参数切片上调用时编译错误 |
| BUG-4 | P2 | `emit/builtins/object.rs:340-341` | Object.groupBy 中 items_arg 硬编码 `.items`，rest 参数场景编译错误 |

---

## BUG-1 (P1): startsWith/endsWith 丢弃 position 参数

### 文件位置
- `emit/builtins/string.rs:221-222` — emit 层配置
- `runtime/js_string.zig:449-456` — runtime 签名

### 问题描述
JS 的 `String.prototype.startsWith(searchString, position)` 和 `endsWith(searchString, endPosition)` 支持第二个可选参数指定搜索起始位置。但转译器将这两个方法的 `max_args` 设为 `1`，导致 position 参数被静默丢弃。

**JS 语义**:
```js
"hello".startsWith("ell", 1)  // → true (从索引 1 开始搜索)
"hello".endsWith("llo", 4)   // → true (到索引 4 结束)
```

**当前生成的 Zig 代码**:
```zig
js_string.startsWith("hello", "ell")  // position 参数被丢弃，永远从头搜索
```

### 根因分析

`emit/builtins/string.rs` 行 221-222:
```rust
"startsWith" => ("startsWith", false, false, 1, 1, &[], "js_string"),
"endsWith" => ("endsWith", false, false, 1, 1, &[], "js_string"),
```

第 5 个参数 `max_args=1` 限制了迭代到第 1 个参数为止，position 参数（arg[1]）永远不会被传递。

Runtime 签名 (`js_string.zig:449-456`) 也只有 2 个参数：
```zig
pub fn startsWith(s: []const u8, prefix: []const u8) bool {
    return std.mem.startsWith(u8, s, prefix);
}
pub fn endsWith(s: []const u8, suffix: []const u8) bool {
    return std.mem.endsWith(u8, s, suffix);
}
```

### 修复建议

**方案 A（推荐）：emit 层 slice 后再调用**

修改 `string.rs` 的 emit 配置，将 `startsWith`/`endsWith` 改为特殊处理：当有 position 参数时，先对字符串做 slice，再调用无 position 版本。

```rust
// 在 string.rs 的 match 块中，在通用表之前添加特殊处理：
"startsWith" => {
    if args.len() >= 2 {
        // startsWith(str, position) → js_string.startsWith(js_string.slice(s, position, std.math.maxInt(i64)), prefix)
        self.write("js_string.startsWith(js_string.slice(");
        self.emit_expr(&obj_expr);
        self.write(", ");
        self.emit_i64_coerced(&args[1]);
        self.write(", std.math.maxInt(i64)), ");
        self.emit_expr(&args[0]);
        self.write(")");
    } else {
        // 无 position 参数：走通用路径
        // (保持原有表配置，但 max_args 改为 1)
    }
    return;
}
"endsWith" => {
    if args.len() >= 2 {
        // endsWith(str, endPosition) → js_string.endsWith(js_string.slice(s, 0, endPosition), suffix)
        self.write("js_string.endsWith(js_string.slice(");
        self.emit_expr(&obj_expr);
        self.write(", 0, ");
        self.emit_i64_coerced(&args[1]);
        self.write("), ");
        self.emit_expr(&args[0]);
        self.write(")");
    } else {
        // 无 position 参数
    }
    return;
}
```

**方案 B：更新 runtime 签名**

修改 `js_string.zig` 增加 position 参数：
```zig
pub fn startsWith(s: []const u8, prefix: []const u8, position: ?i64) bool {
    if (position) |pos| {
        const start = if (pos < 0) 0 else @as(usize, @intCast(pos));
        if (start > s.len) return false;
        return std.mem.startsWith(u8, s[start..], prefix);
    }
    return std.mem.startsWith(u8, s, prefix);
}
```
并将 emit 配置改为 `max_args=2`。

---

## BUG-2 (P1): Object.groupBy 非字符串 key 类型不匹配

### 文件位置
- `emit/builtins/object.rs:326-350`

### 问题描述
`Object.groupBy(items, callbackFn)` 使用 `StringArrayHashMap` 作为分组容器（key 类型为 `[]const u8`）。但 JS 回调可以返回任意值（数字、布尔值等），当回调返回非字符串类型时，生成的 Zig 代码会因为 key 类型不匹配而编译失败。

**JS 语义**:
```js
Object.groupBy([6.1, 4.2, 6.3], Math.floor)
// 回调返回数字 6, 4, 6 → key 应被转为字符串 "6", "4"
```

**当前生成的 Zig 代码**:
```zig
// _key 的类型是 i64（Math.floor 的返回值）
if (_grp_map.getPtr(_key)) |_grp_list| { ... }  // 编译错误：i64 ≠ []const u8
_grp_map.put(_key, _grp_new)  // 编译错误：i64 ≠ []const u8
```

### 根因分析

`object.rs:337` 创建了 `StringArrayHashMap`:
```rust
self.write(&format!(
    "{blk}: {{ var {0} = js_runtime.StringArrayHashMap(std.ArrayList(JsAny)).init(...); ",
    _map
));
```

行 343-347 中，回调返回值直接赋给 `_key`，然后用于 map 的 `getPtr` 和 `put`：
```rust
self.emit_group_by_callback(&args[1], &_item, &_key);
// ...
self.write(&format!("; if ({0}.getPtr({1})) |_grp_list| {{ ... }} else {{ ... {0}.put({1}, {3}) ..."));
```

当回调返回 i64/f64/bool 时，`_key` 的 Zig 类型不是 `[]const u8`，导致 HashMap 操作编译失败。

### 修复建议

在 `_key` 赋值后，将其转换为字符串。修改 `emit_group_by_callback` 或在调用后插入转换代码：

```rust
// 在 emit_group_by_callback 调用之后，添加 key 转换：
// const _grp_key_str = JsAny.from(_key).toString(js_allocator.allocator()) catch @panic("OOM");
// 然后用 _grp_key_str 替代 _key 用于 map 操作

// 修改 object.rs 行 343-347：
if args.len() >= 2 {
    self.emit_group_by_callback(&args[1], &_item, &_key);
} else {
    self.write(&format!("const {0} = {1}", _key, _item));
}
// 新增：将 key 转为字符串
self.write(&format!(
    "; const {0}_str = JsAny.from({1}).toString(js_allocator.allocator()) catch @panic(\"OOM: groupBy key\"); ",
    _key, _key
));
// 后续 map 操作使用 _key_str 而非 _key
self.write(&format!(
    "; if ({0}.getPtr({1}_str)) |_grp_list| {{ _grp_list.append(js_allocator.allocator(), JsAny.from({2})) catch @panic(\"OOM\"); }} else {{ var {3}: std.ArrayList(JsAny) = .empty; {3}.append(js_allocator.allocator(), JsAny.from({2})) catch @panic(\"OOM\"); {0}.put({1}_str, {3}) catch @panic(\"OOM\"); }} }} ",
    _map, _key, _item, _new
));
```

---

## BUG-3 (P1): 数组回调方法硬编码 `.items`，rest 参数切片上调用时编译错误

### 文件位置
- `emit/builtins/array_callback.rs` — 多处硬编码 `.items`

### 问题描述

`items_path()` 函数（行 16-22）已正确定义，用于区分 ArrayList（需要 `.items`）和 rest 参数切片 `[]const JsAny`（不需要 `.items`）。`emit_reduce_inline` 和 `emit_reduce_right_inline` 正确使用了此函数。

但以下 **9 个函数** 绕过了 `items_path()`，直接硬编码 `.items`：

| 函数 | 硬编码行号 | 影响 |
|------|-----------|------|
| `emit_for_each_inline` | 行 75 传入 `".items"` | forEach on rest params |
| `emit_short_circuit_inline` (some/every) | 行 217, 221 | some/every on rest params |
| `emit_filter_inline` | 行 270 | filter on rest params |
| `emit_find_like_inline` (find/findLast) | 行 309 | find/findLast on rest params |
| `emit_find_index_like_inline` (findIndex/findLastIndex) | 行 385 | findIndex/findLastIndex on rest params |
| `emit_collect_inline` (map/flatMap) | 行 469, 472 | map/flatMap on rest params |
| `emit_reverse_loop_header` | 行 878 | findLast/findLastIndex reverse loop |
| `emit_sort_callback_inline` | 行 815, 826 | sort on rest params |
| `emit_to_sorted_callback_inline` | 行 857 | toSorted on rest params |

### 根本原因

`IrArrayCallbackInline` 结构体（`types.rs:1026-1051`）**没有** `receiver_is_slice` 字段，而 `IrArrayMethodInline`（行 1088-1104）有此字段。

`try_inline_array_callback`（`cabi.rs:583-634`）不检查 receiver 是否为 rest 参数切片。当 receiver 是 rest 参数（`[]const JsAny` 类型）时，`get_var_type` 返回 None（或 ArrayList(JsAny)），默认 `CollectionKind::Array`，回调内联路径被触发。

生成的代码在 `[]const JsAny` 上使用 `.items`，但 Zig 的切片类型没有 `.items` 字段，导致编译错误。

**复现场景**:
```js
function test(...args) {
    return args.some(x => x > 0);  // 生成 args.items → 编译错误
}
```

**当前生成的 Zig 代码**:
```zig
for (args.items) |x| {  // 错误：[]const JsAny 没有 .items 字段
    if (js_runtime.isTruthy(...)) break :blk true;
}
```

**期望的 Zig 代码**:
```zig
for (args) |x| {  // 切片直接迭代，不需要 .items
    if (js_runtime.isTruthy(...)) break :blk true;
}
```

### 修复建议

将所有硬编码的 `.items` 替换为 `self.items_path(&receiver)` 调用。以下是逐函数的修复 diff：

**1. `emit_for_each_inline` (行 72-80)**:
```rust
// Before:
CollectionKind::Array => {
    self.emit_for_each_simple_loop(
        &binding, &receiver, ".items", // 硬编码
        ...
    );
}

// After:
CollectionKind::Array => {
    let items = self.items_path(&receiver).to_string();
    self.emit_for_each_simple_loop(
        &binding, &receiver, &items, // 动态获取
        ...
    );
}
```

**2. `emit_short_circuit_inline` (行 215-221)**:
```rust
// Before:
self.write(&format!("for ({}.items, 0..) |{}, {}| ", receiver, ...));
self.write(&format!("for ({}.items) |{}| ", receiver, ...));

// After:
let items = self.items_path(&receiver);
self.write(&format!("for ({}{}, 0..) |{}, {}| ", receiver, items, ...));
self.write(&format!("for ({}{}) |{}| ", receiver, items, ...));
```

**3. `emit_filter_inline` (行 270)**:
```rust
// Before:
self.write(&format!("for ({}.items) |{}| ", receiver, loop_elem));

// After:
self.write(&format!("for ({}{}) |{}| ", receiver, self.items_path(&receiver), loop_elem));
```

**4. `emit_find_like_inline` (行 309)**:
```rust
// Before:
self.write(&format!("for ({}.items) |{}| ", receiver, data.elem_param));

// After:
self.write(&format!("for ({}{}) |{}| ", receiver, self.items_path(&receiver), data.elem_param));
```

**5. `emit_find_index_like_inline` (行 384-386)**:
```rust
// Before:
self.write(&format!("for ({}.items, 0..) |{}, {}| ", receiver, ...));

// After:
self.write(&format!("for ({}{}, 0..) |{}, {}| ", receiver, self.items_path(&receiver), ...));
```

**6. `emit_collect_inline` (行 468-472)**:
```rust
// Before:
self.write(&format!(
    "{}.ensureTotalCapacity(js_allocator.allocator(), {}.items.len) catch ...", var, receiver, ...));
self.write(&format!("for ({}.items) |{}| ", receiver, loop_elem));

// After:
let items = self.items_path(&receiver);
let len_expr = if items.is_empty() { format!("{}.len", receiver) } else { format!("{}.items.len", receiver) };
self.write(&format!(
    "{}.ensureTotalCapacity(js_allocator.allocator(), {}) catch ...", var, len_expr, ...));
self.write(&format!("for ({}{}) |{}| ", receiver, items, loop_elem));
```

**7. `emit_reverse_loop_header` (行 877-880)**:
```rust
// Before:
self.write(&format!(
    "var {}: usize = {}.items.len; while ({} > 0) {{ {} -= 1; const {} = {}.items[{}]; {}",
    loop_var, receiver, loop_var, loop_var, elem_param, receiver, loop_var, extra
));

// After:
let items = self.items_path(receiver);
let len_expr = if items.is_empty() { format!("{}.len", receiver) } else { format!("{}.items.len", receiver) };
let idx_expr = if items.is_empty() { format!("{}[{}]", receiver, loop_var) } else { format!("{}.items[{}]", receiver, loop_var) };
self.write(&format!(
    "var {}: usize = {}; while ({} > 0) {{ {} -= 1; const {} = {}; {}",
    loop_var, len_expr, loop_var, loop_var, elem_param, idx_expr, extra
));
```

**8. `emit_sort_callback_inline` (行 815, 826)** 和 **9. `emit_to_sorted_callback_inline` (行 857)**:
```rust
// Before:
&format!("{}.items", receiver)

// After:
let items = self.items_path(&receiver);
let target = if items.is_empty() { receiver.clone() } else { format!("{}{}", receiver, items) };
// 用 target 替代 format!("{}.items", receiver)
```

---

## BUG-4 (P2): Object.groupBy 中 items_arg 硬编码 `.items`

### 文件位置
- `emit/builtins/object.rs:340-341`

### 问题描述

`Object.groupBy(items, callbackFn)` 中，`items` 参数硬编码使用 `.items` 后缀来迭代。当 `items` 是 rest 参数（`[]const JsAny` 切片）时，`.items` 字段不存在，导致编译错误。

**复现场景**:
```js
function test(...args) {
    return Object.groupBy(args, x => x > 0 ? "big" : "small");
}
```

**当前生成的 Zig 代码**:
```zig
for (args.items) |_grp_item| {  // 错误：[]const JsAny 没有 .items
    ...
}
```

### 根因分析

`object.rs:339-341`:
```rust
self.write("for (");
self.emit_expr(items_arg);
self.write(&format!(".items) |{0}| {{ ", _item));
```

无条件追加 `.items`，未考虑 `items_arg` 可能是切片类型。

### 修复建议

检查 `items_arg` 是否为 rest 参数标识符，如果是则不追加 `.items`：

```rust
self.write("for (");
self.emit_expr(items_arg);
// 检查 items_arg 是否是 rest 参数
let is_rest = match items_arg {
    IrExpr::Ident(ident) if self.rest_param_names.contains(&ident.zig_name) => true,
    IrExpr::TypedIdent { ident, .. } if self.rest_param_names.contains(&ident.zig_name) => true,
    _ => false,
};
if is_rest {
    self.write(&format!(") |{0}| {{ ", _item));
} else {
    self.write(&format!(".items) |{0}| {{ ", _item));
}
```

---

## 审计覆盖范围

### 已深入审计的文件
**emit 层**:
- `builtins/string.rs` ✓ (found BUG-1)
- `builtins/object.rs` ✓ (found BUG-2, BUG-4)
- `builtins/array_callback.rs` ✓ (found BUG-3)
- `builtins/array_method.rs` ✓ (已检查 format! 占位符匹配)
- `builtins/collections.rs` ✓
- `builtins/math.rs` ✓
- `builtins/regexp.rs` ✓
- `stmt/control_flow.rs` ✓
- `stmt/decl.rs` ✓
- `stmt/destructure_assign.rs` ✓
- `expr/binary.rs` ✓
- `expr/call_member.rs` ✓
- `expr/container.rs` ✓
- `expr/template_new.rs` ✓
- `expr/mod.rs` ✓ (部分)

**lower 层**:
- `cabi.rs` ✓ (found BUG-3 根因)
- `closure.rs` ✓
- `decl.rs` ✓
- `class.rs` ✓
- `expr/mod.rs` ✓
- `expr/operators.rs` ✓
- `expr/member.rs` ✓
- `expr/optional.rs` ✓
- `expr/call.rs` ✓ (部分)

**类型定义**:
- `types.rs` ✓ (确认 IrArrayCallbackInline 无 receiver_is_slice 字段)

### 已检查无重大问题的方面
- **format! 占位符匹配**: 手动验证了关键 format! 调用的占位符与参数数量匹配
- **needs_deinit 与 JsAny**: JsAny 在 arena allocator 下 deinit 是 no-op，设计意图正确
- **闭包所有权转移**: non-mut 捕获按值拷贝、mut 捕获用 `&` 指针，基本正确
- **Conditional (ternary)**: 类型推断处理了 I64+F64→F64 coercion
- **Try/catch/finally**: 双 labeled-block 模式正确处理 error 传播
- **Switch 字符串**: 正确降级为 if/else-if 链
- **Do-while**: 唯一 flag 变量名避免 shadowing
- **For-of 字符串迭代**: 使用 Utf8View 正确迭代 Unicode code points
- **Math 方法**: 正确处理 f64 coercion 和 JS 语义差异（如 Math.round 的 half-up）
- **BigInt 运算**: 正确使用方法调用和 error 处理