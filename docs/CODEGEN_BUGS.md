---
AIGC:
  ContentProducer: '001191110102MAD55U9H0F10002'
  ContentPropagator: '001191110102MAD55U9H0F10002'
  Label: '1'
  ProduceID: '474a1152-f040-4432-913d-b527cfa10d15'
  PropagateID: '474a1152-f040-4432-913d-b527cfa10d15'
  ReservedCode1: 'bb614588-a147-4133-9b54-cab400a4b4bc'
  ReservedCode2: 'bb614588-a147-4133-9b54-cab400a4b4bc'
---

---
AIGC:
  ContentProducer: '001191110102MAD55U9H0F10002'
  ContentPropagator: '001191110102MAD55U9H0F10002'
  Label: '1'
  ProduceID: 'bf3d6cdc-435c-435b-87dd-ac671a03e740'
  PropagateID: 'bf3d6cdc-435c-435b-87dd-ac671a03e740'
  ReservedCode1: '6937a675-1e73-47dc-95e4-1c4b328b8464'
  ReservedCode2: '6937a675-1e73-47dc-95e4-1c4b328b8464'
---

---
AIGC:
  ContentProducer: '001191110102MAD55U9H0F10002'
  ContentPropagator: '001191110102MAD55U9H0F10002'
  Label: '1'
  ProduceID: '353f7c8d-efaa-48c6-be16-c5ad85a2cf74'
  PropagateID: '353f7c8d-efaa-48c6-be16-c5ad85a2cf74'
  ReservedCode1: 'f55500ab-7d15-47a8-b75d-ab7736f22296'
  ReservedCode2: 'f55500ab-7d15-47a8-b75d-ab7736f22296'
---

---
AIGC:
  ContentProducer: '001191110102MAD55U9H0F10002'
  ContentPropagator: '001191110102MAD55U9H0F10002'
  Label: '1'
  ProduceID: '96d95a23-1a74-4887-984f-994588b4f88c'
  PropagateID: '96d95a23-1a74-4887-984f-994588b4f88c'
  ReservedCode1: '576b07a7-9e52-4508-a58b-402e09ad1a05'
  ReservedCode2: '576b07a7-9e52-4508-a58b-402e09ad1a05'
---

---
AIGC:
  ContentProducer: '001191110102MAD55U9H0F10002'
  ContentPropagator: '001191110102MAD55U9H0F10002'
  Label: '1'
  ProduceID: '3d41e8c3-d73f-46bf-a9f0-3f0f8e2340f9'
  PropagateID: '3d41e8c3-d73f-46bf-a9f0-3f0f8e2340f9'
  ReservedCode1: '83abcf8f-63df-4e4a-866e-baa00160df4e'
  ReservedCode2: '83abcf8f-63df-4e4a-866e-baa00160df4e'
---

---
AIGC:
  ContentProducer: '001191110102MAD55U9H0F10002'
  ContentPropagator: '001191110102MAD55U9H0F10002'
  Label: '1'
  ProduceID: '18373494-e488-4051-8242-2357cd1788ca'
  PropagateID: '18373494-e488-4051-8242-2357cd1788ca'
  ReservedCode1: '8299073d-212e-45db-8605-97a5f82febbe'
  ReservedCode2: '8299073d-212e-45db-8605-97a5f82febbe'
---

---
AIGC:
  ContentProducer: '001191110102MAD55U9H0F10002'
  ContentPropagator: '001191110102MAD55U9H0F10002'
  Label: '1'
  ProduceID: '7ccc77a4-cdcb-424a-a41a-2bb3c9b82b31'
  PropagateID: '7ccc77a4-cdcb-424a-a41a-2bb3c9b82b31'
  ReservedCode1: 'b8e49185-003b-4364-90d4-4ebe1c5b5fd9'
  ReservedCode2: 'b8e49185-003b-4364-90d4-4ebe1c5b5fd9'
---

# Codegen Bug Tracker

> e2e 测试补充过程中发现的代码生成器 Bug 清单。
> 这些特性在 Rust 侧（推断/lowering）已实现，但生成的 Zig 代码无法编译。
> 所有特性均有 Rust 单元测试覆盖（494 passed），仅缺少端到端（e2e）验证。

## 严重程度定义

| 级别 | 含义 |
|------|------|
| **P0** | 生成无法编译的 Zig 代码，任何使用该特性的 JS 代码都会构建失败 |
| **P1** | 功能正确性问题：生成的 Zig 可编译但运行时行为不符合 JS 语义 |
| **P2** | 诊断/易用性问题：不影响功能但影响开发体验 |

---

## BUG-01: `in` operator on Map/Set 生成不存在的 `.contains()` 方法

- **严重程度**: P0
- **文件**: `js2zig-core/src/zigir/emit/expr/binary.rs:254-260`
- **Pending e2e 测试**: `examples/showcase-project/js_src_pending/test_in_operator.js`
- **复现**: `"key" in mapVariable` 或 `value in setVariable`
- **现象**: 生成 `m.contains("key")`，但 `JsCollection(JsAny)` / `JsCollection(void)` 没有 `.contains()` 方法。`has()` 方法存在但签名不同（`has` 接受 `JsAny` 参数，`contains` 接受具体类型）。
- **生成代码示例**:
  ```zig
  // 期望: m.has(JsAny.from("key"))
  // 实际: m.contains("key")  // compile error: no member named 'contains'
  ```
- **修复方向**: 将 `in` operator 的 emit 从 `.contains(key)` 改为 `.has(JsAny.from(key))`，或在 `JsCollection` 上添加 `contains` 便捷方法。
- **Workaround**: 使用 `m.has("key")` 替代 `"key" in m`。

---

## BUG-02: `arguments` 对象生成 ArrayList 但类型签名不匹配（已修复）

- **严重程度**: P0
- **状态**: FIXED
- **文件**: `js2zig-core/src/infer/fn_types.rs`（检测 `arguments` 使用），`js2zig-core/src/zigir/lower/decl.rs`（合成 rest 参数 + `__arguments` VarDecl），`js2zig-core/src/zigir/emit/expr/call_member.rs`（`ArgumentsLen` FieldKind），`js2zig-core/src/zigir/lower/expr/member.rs`（`.length` → `ArgumentsLen`），`js2zig-core/src/zigir/lower/expr/call.rs`（rest 参数打包）
- **e2e 测试**: `examples/showcase-project/js_src/test_arguments.js`
- **修复方案**:
  - `infer/fn_types.rs`: 检测函数体中使用 `arguments` 的非导出函数，注册到 `functions_needing_synthetic_rest`
  - `lower/decl.rs`: 为这些函数自动注入 `...__arguments` 合成 rest 参数，使 `arguments` 捕获所有运行时实参
  - `lower/expr/member.rs`: `arguments.length` → `FieldKind::ArgumentsLen`（`@as(i64, @intCast(__arguments.len))`）
  - `lower/expr/member.rs`: `arguments[i]` → `IndexAccess` with `SliceIndex`（`__arguments[@as(usize, @intCast(i))]`）
  - `lower/expr/call.rs`: `pack_rest_args_if_needed` 将多余实参打包为 spread 传递给合成 rest 参数
  - `emit/expr/call_member.rs`: 新增 `FieldKind::ArgumentsLen` 变体，emit `@as(i64, @intCast(obj.len))`
  - `__arguments` 声明为 `[]const JsAny` const 切片，`needs_deinit=false`
  - 导出函数（C ABI）不支持 `arguments`，需用户显式使用 `...args` rest 参数
- **限制**: 导出函数中使用 `arguments` 会走旧路径（仅包含已声明参数），建议使用 `...args` rest 参数语法

---

## BUG-03: `for...of` Map 解构变量类型为 JsAny 而非推断的具体类型

- **严重程度**: P0
- **文件**: `js2zig-core/src/zigir/emit/stmt/control_flow.rs:300-331`
- **Pending e2e 测试**: `examples/showcase-project/js_src_pending/test_for_of_collections.js`
- **复现**: `for (const [k, v] of mapInstance) { sum += v; }` 其中 `sum` 是 `i64`
- **现象**: 解构变量 `v` 的类型为 `JsAny`（从 `__kv.value_ptr.*` 推断），而不是 Map 值的具体类型（如 `i64`）。当与 `i64` 做算术时报类型不兼容错误。
- **生成代码示例**:
  ```zig
  const v = __kv.value_ptr.*;  // type: JsAny
  sum = sum + v;                // error: incompatible types: i64 and JsAny
  ```
- **修复方向**: 降低 Map `for...of` 解构时，应根据 Map 的 value 类型推断 `v` 的类型，并在 emit 时插入 `JsAny.toI64()` 或 `.asInt()` 转换。
- **Workaround**: 仅对解构变量做不涉及算术的操作（如计数），避免类型混合。

---

## BUG-04: `for...of` Set 迭代变量类型为 JsAny

- **严重程度**: P0
- **文件**: `js2zig-core/src/zigir/emit/stmt/control_flow.rs`
- **Pending e2e 测试**: `examples/showcase-project/js_src_pending/test_for_of_collections.js`
- **复现**: `for (const val of setInstance) { sum += val; }` 其中 `sum` 是 `i64`
- **现象**: 同 BUG-03，`val` 类型为 `JsAny`，与 `i64` 算术不兼容。
- **生成代码示例**:
  ```zig
  const val = __kv.key_ptr.*;  // type: JsAny
  sum = sum + val;              // error: incompatible types: i64 and JsAny
  ```
- **修复方向**: 同 BUG-03，需要类型转换。
- **Workaround**: 仅对迭代变量做计数操作。

---

## BUG-05: `for...of` String 生成 `for (str) |ch|` 但捕获变量未使用时报错

- **严重程度**: P2
- **文件**: `js2zig-core/src/zigir/emit/stmt/control_flow.rs`
- **Pending e2e 测试**: `examples/showcase-project/js_src_pending/test_for_of_collections.js`
- **复现**: `for (const ch of "ABC") { count++; }`（未在循环体中使用 `ch`）
- **现象**: Zig 0.16 将未使用的 for 循环捕获变量视为错误。应生成 `for ("ABC") |_ch|` 或 `for ("ABC") |_, _|` 来显式忽略。
- **生成代码示例**:
  ```zig
  for ("ABC") |ch| {    // error: unused capture
      count = count + 1;
  }
  ```
- **修复方向**: 当循环变量在循环体中未被引用时，emit 为 `|_|` 代替 `|varName|`。
- **Workaround**: 在循环体中引用变量（如 `let x = ch;`）。

---

## BUG-06: Array ES2023 方法返回 ArrayList 缺少后续操作支持

- **严重程度**: P0
- **文件**: `js2zig-core/src/zigir/emit/builtins/array_method.rs`
- **Pending e2e 测试**: `examples/showcase-project/js_src_pending/test_builtins_es2023.js`
- **复现**: `arr.toReversed()[0]`、`arr.toSorted().length`、`for (const v of arr.toSpliced()) {}`
- **现象**: `.toReversed()`/`.toSorted()`/`.toSpliced()`/`.with()` 返回 `ArrayList(i64)` 类型，但：
  1. **索引访问** `newArr[0]` 生成 `newArr[0]`，但 `ArrayList` 不支持 `[]`（需 `.items[0]`）
  2. **`.length`** 生成 `js_string.utf16Len(newArr)`，但参数类型不匹配（`ArrayList` ≠ `[]const u8`）
  3. **`for...of`** 生成 `for (newArr) |v|`，但 `ArrayList` 不是可迭代类型
  4. **赋值后未使用** 生成 `const reversed = ...` 后未引用，Zig 报 unused local constant
- **生成代码示例**:
  ```zig
  const reversed = arr.toReversed();  // type: ArrayList(i64)
  reversed[0]                         // error: ArrayList not indexable
  js_string.utf16Len(reversed)        // error: expected []const u8
  for (reversed) |v| {}               // error: not indexable and not a range
  ```
- **修复方向**: 返回值类型推断应为 `ZigType::ArrayList(Anytype)`（已做），但后续 emit 需要识别 `ArrayList` 类型：
  - 索引访问 → `.items[@intCast(idx)]`
  - `.length` → `.items.len`
  - `for...of` → `for (list.items) |v| {}`
  - 未使用 → 加 `_ = reversed;`
- **Workaround**: Rust 单元测试覆盖 `test_array_to_reversed` 等。

---

## BUG-07: Date setter 方法缺少可选参数传递

- **严重程度**: P0
- **文件**: `js2zig-core/src/zigir/emit/builtins/collections.rs`
- **Pending e2e 测试**: `examples/showcase-project/js_src_pending/test_date_setters.js`
- **复现**: `d.setFullYear(2025)`、`d.setMonth(5)`、`d.setHours(14)`
- **现象**: JS 中 Date setter 的后续参数是可选的（如 `setFullYear(year, month?, date?)`），但生成代码将所有参数位置传递，Zig 运行时签名要求精确参数数量。
- **生成代码示例**:
  ```zig
  // JS: d.setFullYear(2025)
  // 生成: _ = d.setFullYear(2025);
  // 但 Zig 签名: pub fn setFullYear(self: JsDate, year: i64, month: ?i64, date: ?i64) i64
  // error: member function expected 3 argument(s), found 1
  ```
- **修复方向**: emit 时为缺失的可选参数传递 `null`。
- **Workaround**: 无。Rust 单元测试覆盖。

---

## BUG-08: String 方法在字面量上生成 `.deinit()` 调用

- **严重程度**: P0
- **文件**: `js2zig-core/src/zigir/emit/`（自动 cleanup 逻辑）
- **Pending e2e 测试**: `examples/showcase-project/js_src_pending/test_string_methods.js`
- **复现**: `const s = "hello"; const x = s.slice(0, 3);`（`s` 是字符串字面量，编译时类型为 `[:0]const u8`）
- **现象**: 字符串字面量被标记为需要 `defer deinit()`，但 `[:0]const u8` 没有 `.deinit()` 方法。类似地，对字面量调用 `.items` 也会失败。
- **生成代码示例**:
  ```zig
  const s = "  hello  ";   // type: *const [9:0]u8
  defer s.deinit(...);      // error: no member named 'deinit'
  s.items[6..11]            // error: no member named 'items'
  ```
- **修复方向**: 区分字面量类型（`[:0]const u8`）和动态字符串类型（`ArrayList`），仅对后者生成 cleanup 和 `.items` 访问。字面量的 `slice`/`substring` 应直接使用 Zig 切片语法 `s[start..end]`。
- **Workaround**: 将字符串作为函数参数传入（参数类型为 `string` → 生成 `[]const u8` 切片）。

---

## BUG-09: Symbol 等性比较生成不支持的操作符

- **严重程度**: P0
- **文件**: `js2zig-core/src/zigir/emit/expr/binary.rs`
- **Pending e2e 测试**: `examples/showcase-project/js_src_pending/test_symbol.js`
- **复现**: `a !== b`（`a`, `b` 是 `Symbol()` 返回值）
- **现象**: `JsSymbol` 是一个 Zig struct，不支持 `==` / `!=` 操作符。需要实现 `eql` 方法或使用 `ptrEqual`。
- **生成代码示例**:
  ```zig
  if (a != b) {}   // error: operator != not allowed for type 'JsSymbol'
  if (a == b) {}   // error: operator == not allowed for type 'JsSymbol'
  ```
- **修复方向**: 在 `js_symbol.zig` 的 `JsSymbol` 上添加 `pub fn eql(self, other) bool` 方法，emit 侧对 `JsSymbol` 类型的 `===`/`!==` 改为调用 `.eql()`。
- **Workaround**: 无。Rust 单元测试覆盖。

---

## BUG-10: `Symbol.keyFor()` 返回 optional 类型但代码未处理

- **严重程度**: P1
- **文件**: `js2zig-core/src/zigir/emit/expr/call_member.rs`
- **Pending e2e 测试**: `examples/showcase-project/js_src_pending/test_symbol.js`
- **复现**: `const key = Symbol.keyFor(sym); key === "my.key"`
- **现象**: `Symbol.keyFor()` 返回 `?[]const u8`（optional），但生成代码直接将其作为 `[]const u8` 传给 `std.mem.eql`。
- **生成代码示例**:
  ```zig
  const key = js_symbol.keyFor(sym);  // type: ?[]const u8
  std.mem.eql(u8, key, "my.key")      // error: expected []const u8, found ?[]const u8
  ```
- **修复方向**: emit 侧对 optional 返回值插入 `.?` 或 `orelse` 解包，或在 runtime 侧保证 `keyFor` 对注册过的 symbol 总返回非 null。
- **Workaround**: 无。

---

## BUG-11: `new RegExp()` 在非 error 函数中使用 `try`

- **严重程度**: P0
- **文件**: `js2zig-core/src/zigir/emit/expr/new.rs`（或相关 emit）
- **Pending e2e 测试**: `examples/showcase-project/js_src_pending/test_regexp.js`
- **复现**: `const re = new RegExp("test", "g");` 在返回 `i64` 的函数中
- **现象**: `JsRegExp.init()` 返回 `!JsRegExp`（error union），但生成代码在非 error 返回函数中直接使用 `try`。
- **生成代码示例**:
  ```zig
  pub fn testRegExpGlobal() i64 {
      const re = try js_regexp.JsRegExp.init(...);  // error: function cannot return an error
  }
  ```
- **修复方向**: 类似 `JSON.parse` 的处理方式，检测 `RegExp` 构造并设置 `has_catchable_error` 标志，使函数返回类型变为 `!i64`。
- **Workaround**: 无。

---

## BUG-12: `delete obj.prop` on Map 生成 `deleteByKey` 缺少 allocator 参数

- **严重程度**: P0
- **文件**: `js2zig-core/src/zigir/emit/expr/operators.rs`（delete emit 逻辑）
- **Pending e2e 测试**: `examples/showcase-project/js_src_pending/test_delete_operator.js`
- **复现**: `delete mapVariable["key"]`
- **现象**: 对 Map 的 `delete obj[expr]` 生成 `m.deleteByKey(_dk, alloc)` 但 `alloc` 变量在作用域中不存在。
- **生成代码示例**:
  ```zig
  _ = m.deleteByKey(_dk, alloc);  // error: use of undeclared identifier 'alloc'
  ```
- **修复方向**: emit 时使用 `js_allocator.allocator()` 替代 `alloc`，与 Map 其他方法的 emit 方式保持一致。
- **Workaround**: 使用 `m.delete("key")` 显式方法调用替代 `delete` 运算符。

---

## BUG-13: Map.forEach / Set.forEach 回调参数类型为 JsAny

- **严重程度**: P0
- **文件**: `js2zig-core/src/zigir/emit/builtins/array_callback.rs`
- **Pending e2e 测试**: `examples/showcase-project/js_src_pending/test_for_of_collections.js`
- **复现**: `m.forEach((value, key) => { sum += value; })`（`sum` 是 `i64`）
- **现象**: forEach 回调的 `value` 参数类型为 `JsAny`，与 `i64` 算术不兼容。同 BUG-03/04 的根本原因。
- **生成代码示例**:
  ```zig
  // forEach callback body
  sum = sum + value;  // error: incompatible types: i64 and JsAny
  ```
- **修复方向**: 同 BUG-03，需要类型转换。
- **Workaround**: 无。Rust 单元测试覆盖。

---

## BUG-14: `class extends` 的 `super` 调用未实现

- **严重程度**: P1
- **文件**: `js2zig-core/src/zigir/lower/class.rs:175-181`
- **Pending e2e 测试**: `examples/showcase-project/js_src_pending/test_extends.js`
- **复现**: `class Child extends Parent { constructor() { super(); } }`
- **现象**: `super` 明确不支持，生成 `@compileError("super not supported")`。`extends` 仅用于 `instanceof` 链追踪，不生成字段/方法继承。
- **修复方向**: 需要完整的原型链继承 emit 策略（字段展开、方法委托、super 调用映射），属于较大的架构性工作。
- **Workaround**: 使用组合模式替代继承。

---

## 优先级排序建议

| 优先级 | Bug 编号 | 理由 |
|--------|----------|------|
| **高** | BUG-06 | Array ES2023 是最常用的新特性之一，影响面广 |
| **高** | BUG-08 | 字符串方法是最常用的内置方法，字面量场景极常见 |
| **高** | BUG-07 | Date setter 是基础功能，缺少可选参数传递阻塞所有使用 |
| **高** | BUG-03/04/13 | Map/Set 迭代是核心功能，JsAny 类型推断问题影响多个场景 |
| **中** | BUG-01 | `in` operator 有 `has()` 替代方案 |
| **中** | BUG-02 | `arguments` 使用场景有限 |
| **中** | BUG-11 | RegExp 构造可复用 `has_catchable_error` 机制 |
| **中** | BUG-12 | `delete` 有显式方法替代方案 |
| **低** | BUG-05 | 未使用捕获变量可通过代码调整规避 |
| **低** | BUG-09/10 | Symbol 使用场景较少 |
| **低** | BUG-14 | `super` 是已知的架构限制，非 Bug |

---

## 测试覆盖状态

| Bug 编号 | Rust 单元测试 | e2e 测试 (showcase) | e2e 测试 (MDN) | 状态 | e2e 文件 |
|----------|:---:|:---:|:---:|:---:|------|
| BUG-01 | ✅ | ✅ | ❌ | FIXED | `js_src/test_in_operator.js` |
| BUG-02 | ✅ | ✅ | ❌ | FIXED | `js_src/test_arguments.js` |
| BUG-03 | ✅ | ✅ | ❌ | FIXED | `js_src/test_for_of_collections.js` |
| BUG-04 | ✅ | ✅ | ❌ | FIXED | `js_src/test_for_of_collections.js` |
| BUG-05 | ✅ | ✅ | ❌ | FIXED | `js_src/test_for_of_collections.js` |
| BUG-06 | ✅ | ✅ | ❌ | FIXED | `js_src/test_builtins_es2023.js` |
| BUG-07 | ✅ | ✅ | ❌ | FIXED | `js_src/test_date_setters.js` |
| BUG-08 | ✅ | ✅ | ❌ | N/A (已工作) | `js_src/test_string_methods.js` |
| BUG-09 | ✅ | ✅ | ❌ | FIXED | `js_src/test_symbol.js` |
| BUG-10 | ✅ | ✅ | ❌ | FIXED | `js_src/test_symbol.js` |
| BUG-11 | ✅ | ✅ | ❌ | FIXED | `js_src/test_regexp.js` |
| BUG-12 | ✅ | ✅ | ❌ | FIXED | `js_src/test_delete_operator.js` |
| BUG-13 | ✅ | ✅ | ❌ | FIXED | `js_src/test_for_of_collections.js` |
| BUG-14 | ✅ | N/A | N/A | SKIP (架构限制) | `js_src_pending/test_extends.js` |

### 已修复 (13/14):
- **BUG-01/07/08/11/12**: 已在先前提交修复并启用 e2e
- **BUG-02**: `arguments` 完整支持 — 自动注入合成 rest 参数 `...__arguments` 捕获所有运行时实参，`arguments.length` 走 `ArgumentsLen` FieldKind（`@as(i64, @intCast(.len))`）、`arguments[i]` 走 `SliceIndex`、`__arguments` 声明为 `[]const JsAny` const 切片。新增 `FieldKind::ArgumentsLen` 变体避免影响其他 `SliceLen` 用例。导出函数需显式使用 `...args` rest 参数。
- **BUG-03/04**: Map/Set for-of 迭代变量设为 JsAny + JsAny 算术转换 (`.asI64()`) + 未使用解构变量抑制
- **BUG-05**: String for-of 新增 `IrForOfKind::Str` 变体，未使用捕获生成 `|_|`，修复 `ast_expr_uses_ident` 缺少 `AssignmentExpression`/`UpdateExpression`/`LogicalExpression` 的问题
- **BUG-06**: ArrayList `needs_deinit` + ES2023 方法返回类型推断
- **BUG-09**: Symbol `===`/`!==` 生成 `.eql()` 而非 `==`/`!=`
- **BUG-10**: `Symbol.keyFor()` 返回值 `.?` 解包
- **BUG-13**: Map/Set forEach 回调参数 var_types 预设 + Set.forEach 改用 iterator 模式

### 跳过 (1/14):
- **BUG-14**: `class extends` / `super` 是架构限制，非 Bug，建议使用组合模式

剩余 e2e 测试文件位于 `examples/showcase-project/js_src_pending/`（仅 `test_extends.js`）。

**额外修复**：JSON.parse 顶层作用域 `catch return error.JsThrow` 编译错误 — 新增 `Emitter.in_function` 字段，顶层使用 `catch @panic()` 替代。`emit_slice_index` 所有切片索引添加 `@as(usize, @intCast())` 转换。

**测试状态**：501 passed, clippy clean, fmt clean, 4 个 e2e 项目全部通过。