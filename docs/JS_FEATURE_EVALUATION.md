# JS 语言特性实现评估

> **项目**: js2rust (JS → Zig 转译器)
> **评估日期**: 2026-06-23
> **代码版本**: main branch (1651070)
> **测试覆盖**: 153 个 Rust 测试 (135 native_proto + 9 jsdoc + 7 parser + 4 sourcemap + 3 testgen 等) + 3 个示例项目

---

## 1. 执行总结

| 指标 | 数值 | 占比 |
|------|------|------|
| **JS 语法特性总数** | ~150+ | - |
| **完全实现** | ~85 | ~57% |
| **部分实现** | ~5 | ~3% |
| **未实现（@compileError）** | ~60 | ~40% |
| **内置对象有效覆盖率** | 57/219 | ~26% |
| **测试覆盖** | 153 个 Rust 测试 (135 native_proto + 23 其他) | - |

**更新说明** (2026-06-25):
- **内置对象覆盖重新评估**: 第 4 节全部重写，用三层分析（检测/发射/运行时）替代原来过于乐观的单层标记。有效覆盖率从声称的 80-100% 修正为实际 ~26%。
- 21 个 runtime 函数已实现但 codegen 未连线（最大低挂果实）。
- console.log/error/warn 完全缺失（0/3），是最高频使用的空缺。

**更新说明** (2026-06-24):
- 修正了 5 个不准确的状态标记（`instanceof`, `void`, `delete`, `obj[key]`, `Date.UTC()`）
- 添加了 8 个遗漏的特性（`function*`, `async function*`, `import.meta`, 逻辑赋值运算符, `**=`, `arguments`, `Symbol`, `WeakMap`/`WeakSet`）
- 文档准确性提升，实际未实现特性数量高于之前估计

---

## 2. 表达式 (Expressions)

### 2.1 基本字面量 (Primary Literals) - ✅ 100% 实现

| 特性 | 状态 | Zig 输出 | 测试 |
|------|------|----------|------|
| 数字字面量 | ✅ | `42`, `3.14` | `test_native_proto_literals` |
| 字符串字面量 | ✅ | `"hello"` | 同上 |
| 布尔字面量 | ✅ | `true` / `false` | 同上 |
| `null` 字面量 | ✅ | `null` | 同上 |
| `undefined` | ✅ | `null` (映射) | 隐式测试 |
| `this` | ✅ | `self` | showcase-project |
| `NaN` | ✅ | `std.math.nan(f64)` | 隐式测试 |
| `Infinity` | ✅ | `std.math.inf(f64)` | 隐式测试 |
| BigInt 字面量 | ✅ | raw 值 | 未测试 |

### 2.2 算术运算符 (Arithmetic Operators) - ✅ 100% 实现

| 特性 | 状态 | Zig 输出 | 测试 |
|------|------|----------|------|
| `+` (加法/字符串拼接) | ✅ | `a + b` 或 `++` | `test_native_proto_operators` |
| `-` (减法) | ✅ | `a - b` | 同上 |
| `*` (乘法) | ✅ | `a * b` | 同上 |
| `/` (除法) | ✅ | `@divTrunc(a, b)` | 同上 |
| `%` (取模) | ✅ | `@rem(a, b)` | 同上 |
| `**` (指数) | ✅ | `std.math.pow(f64, ...)` | `test_native_proto_exponential_*` |
| `++` (自增) | ✅ | `+= 1` | 隐式测试 |
| `--` (自减) | ✅ | `-= 1` | 隐式测试 |
| `+=` `-=` `*=` `/=` `%=` | ✅ | 对应 Zig 运算符 | 隐式测试 |

### 2.3 比较运算符 (Comparison Operators) - ✅ 100% 实现

| 特性 | 状态 | Zig 输出 | 测试 |
|------|------|----------|------|
| `===` (严格相等) | ✅ | `==` 或 `.eq()` | `test_native_proto_operators` |
| `!==` (严格不等) | ✅ | `!=` 或 `.neq()` | 同上 |
| `==` (宽松相等) | ✅ | `==` (同 `===`) | 未区分 |
| `!=` (宽松不等) | ✅ | `!=` (同 `!==`) | 未区分 |
| `<` `>` `<=` `>=` | ✅ | `a < b` 或 `.lt()` | `test_native_proto_operators` |

### 2.4 逻辑运算符 (Logical Operators) - ✅ 100% 实现

| 特性 | 状态 | Zig 输出 | 测试 |
|------|------|----------|------|
| `&&` (与) | ✅ | `and` | `test_native_proto_operators` |
| `\|\|` (或) | ✅ | `or` | 同上 |
| `??` (空值合并) | ✅ | `orelse` | 隐式测试 |

### 2.5 位运算 (Bitwise Operators) - ✅ 100% 实现

| 特性 | 状态 | Zig 输出 | 测试 |
|------|------|----------|------|
| `&` `\|` `^` `~` `<<` `>>` `>>>` | ✅ | 对应 Zig 运算符 | `test_native_proto_operators` |

### 2.6 一元运算符 (Unary Operators) - ⚠️ 部分实现

| 特性 | 状态 | Zig 输出 | 测试 |
|------|------|----------|------|
| `-` (取负) | ✅ | `-x` | `test_native_proto_operators` |
| `+` (取正) | ✅ | 忽略（Zig 无一元加） | 隐式测试 |
| `!` (逻辑非) | ✅ | `!x` | 同上 |
| `~` (位非) | ✅ | `~@as(i64, x)` | 同上 |
| `typeof` | ⚠️ | `@typeName(@TypeOf(x))` | 隐式测试 |
| `void` | ❌ | `@compileError("Unsupported unary operator")` | - |
| `delete` | ❌ | `@compileError("Unsupported unary operator")` | - |

**注意**:
- `typeof` 生成 Zig 类型名（如 `"i64"`），而非 JS `typeof` 的字符串（如 `"number"`）
- `void` 和 `delete` 在 JS 中是有效运算符，但当前实现不支持
- `null` 字面量的类型推断返回 `None`（不确定类型），可能导致类型推断错误
- `undefined` 生成为 `JsAny{ .undefined = {} }`（tagged union），处理正确
- 闭包可变捕获生成 `self.x.*`（指针解引用），多次调用闭包时可能导致 Zig 借用检查器错误
- 大型数组/对象字面量（1000+ 元素）可能导致 Zig 编译器栈溢出，建议使用动态分配
- 深层嵌套函数调用（如 `a(b(c(d(e(f())))))`）可能导致 Zig 编译器递归深度超限
- Unicode 标识符（如中文变量名）应由 oxc 解析器和 Zig 编译器支持，但未经完整测试
- `try-catch` 嵌套资源释放未验证：使用 labeled block + catch handler，finally 内联 emit，需验证嵌套场景下资源是否正确释放
- 模板字符串 `allocPrint` 使用 arena allocator（`js_allocator`），内存由 arena 自动管理，不会泄漏

### 2.7 条件（三元）运算符 - ✅ 100% 实现

| 特性 | 状态 | Zig 输出 | 测试 |
|------|------|----------|------|
| `cond ? a : b` | ✅ | `if (cond) a else b` | `test_native_proto_operators` |

### 2.8 赋值运算符 (Assignment Operators) - ⚠️ 部分实现

| 特性 | 状态 | Zig 输出 | 测试 |
|------|------|----------|------|
| `=` `+=` `-=` `*=` `/=` `%=` | ✅ | 对应 Zig 语法 | 隐式测试 |
| `<<=` `>>=` `>>>=` `&=` `|=` `^=` | ✅ | 对应 Zig 语法 | 未测试 |
| `**=` (指数赋值) | ❌ | 未实现 | ES2016 |
| `&&=` (逻辑与赋值) | ❌ | 未实现 | ES2021 |
| `||=` (逻辑或赋值) | ❌ | 未实现 | ES2021 |
| `??=` (空值合并赋值) | ❌ | 未实现 | ES2021 |

### 2.9 对象/数组访问 - ⚠️ 部分实现

| 特性 | 状态 | Zig 输出 | 测试 |
|------|------|----------|------|
| `obj.prop` (属性访问) | ✅ | `obj.prop` | showcase-project |
| `obj[key]` (计算属性) | ❌ | `@compileError("Dynamic property access")` | - |
| `arr[idx]` (数组索引) | ✅ | `arr[idx]` (仅支持数字字面量) | showcase-project |
| `.length` → `.len` | ✅ | 自动转换 | 同上 |

**注意**:
- `obj[key]` 动态属性访问当前不支持，仅支持数字字面量索引（如 `arr[0]`）
- 字符串 key 访问（如 `obj["key"]`）会生成编译错误

### 2.10 函数调用 - ✅ 100% 实现

| 特性 | 状态 | Zig 输出 | 测试 |
|------|------|----------|------|
| `fn(args)` | ✅ | `fn(args)` | 所有测试 |
| 内置函数调用 | ✅ | 走 `BuiltinRegistry` | 同上 |
| 方法调用 `obj.method()` | ✅ | `obj.method()` | showcase-project |

### 2.11 对象/数组字面量 - ✅ 100% 实现

| 特性 | 状态 | Zig 输出 | 测试 |
|------|------|----------|------|
| `[...]` (数组字面量) | ✅ | `[_]T{ ... }` | `test_native_proto_literals` |
| `{...}` (对象字面量) | ✅ | `.{ .k = v }` | 同上 |
| 对象展开 `{ ...base, k: v }` | ✅ | `blk: { var _tmp = base; ... }` | 隐式测试 |
| Getter 属性 `{ get x() { ... } }` | ✅ | `.x = <return expr>` | `test_native_proto_getter` |
| Setter 属性 `{ set x(v) { ... } }` | ✅ | 跳过（不贡献字段） | `test_native_proto_setter_skipped` |
| 多 spread 合并 `{ ...a, ...b }` | ✅ | `spreadMerge(spreadMerge({}, a), b)` | `testSpreadSingle/Multi/Triple/WithInline/Override` |

### 2.12 模板字面量 - ✅ 100% 实现

| 特性 | 状态 | Zig 输出 | 测试 |
|------|------|----------|------|
| `` `text` `` (无插值) | ✅ | `"text"` | `test_native_proto_template_*` |
| `` `hello ${name}` `` (有插值) | ✅ | `allocPrint(...)` | 同上 |
| 复杂嵌套 | ✅ | 递归生成 | 同上 |
| 标签模板 `` tag`...` `` | ❌ | `@compileError` | - |

### 2.13 箭头函数 - ✅ 100% 实现

| 特性 | 状态 | Zig 输出 | 测试 |
|------|------|----------|------|
| 单表达式 `(x, y) => x + y` | ✅ | 生成独立 `fn` | `test_native_proto_arrow_*` |
| 块语句 `(x, y) => { return x + y; }` | ✅ | 生成独立 `fn` | 同上 |
| 单参数 `x => expr` | ✅ | 生成独立 `fn` | 同上 |
| 无捕获箭头函数 | ✅ | 函数指针 | 隐式测试 |
| 闭包值捕获 `(y) => x + y` | ✅ | 生成 `Closure_X` 结构体 + `call()` | `test_native_proto_closure_basic` |
| 闭包可变捕获 `() => { x++; }` | ✅ | 生成 `Closure_X` 结构体 + `*i64` 指针 | `test_native_proto_closure_mutable` |

**实现方式**: 检测箭头函数中引用的外层变量，自动生成闭包结构体：
- 不可变捕获 (`const` 外层变量) → 值复制到结构体字段
- 可变捕获 (`let`/`var` 外层变量) → 指针字段 (`*T`)，通过 `self.x.*` 解引用

### 2.14 `new` 表达式 - ✅ 90% 实现

| 特性 | 状态 | Zig 输出 | 测试 |
|------|------|----------|------|
| `new Map()` | ✅ | `js_map.JsMap.init(alloc)` | showcase-project |
| `new Set()` | ✅ | `js_set.JsSet.init(alloc)` | 同上 |
| `new Error(msg)` | ✅ | `js_error.JsError.init(alloc, msg)` | `test_native_proto_throw_*` |
| `new ClassName(args)` | ✅ | `ClassName.init(args)` | showcase-project |
| `new Promise(...)` | ❌ | `@compileError` | - |
| 其他构造函数 | ✅ | 自动映射 | 隐式测试 |

### 2.15 `await` 表达式 - ✅ 100% 实现

| 特性 | 状态 | Zig 输出 | 测试 |
|------|------|----------|------|
| `await expr` | ✅ | `io.async(fn, .{io, args}).await(io)` | test-bin-project |

### 2.16 其他表达式 - ⚠️ 部分实现

| 特性 | 状态 | Zig 输出 | 测试 |
|------|------|----------|------|
| `instanceof` | ❌ | `@compileError("instanceof operator is not supported")` | - |
| `"key" in obj` | ✅ | `@hasField(...)` 或 `.contains(key)` | 未测试 |
| 正则表达式 `/pattern/` | ✅ | `"pattern"` (提取 pattern) | 未测试 |
| 可选链 `obj?.prop` | ✅ | `if (obj) |v| v.prop else null` | 5 个测试 |
| 非空断言 `x!` (TS) | ✅ | `x.?` | 未测试 |
| 类型断言 `x as T` (TS) | ✅ | `@as(T, expr)` | 未测试 |
| 序列表达式 `a, b` | ✅ | `a, b` | 未测试 |

**注意**:
- `instanceof` 在 JS 中用于检查对象原型链，但当前实现不支持

### 2.17 不支持的表达式 - ❌ @compileError

| 特性 | 错误信息 |
|------|----------|
| 类表达式 `const X = class {}` | `Unsupported NewExpression` |
| `function*` (生成器函数) | `Unsupported expression type: Function` (注: 需添加生成器支持) |
| `yield` / `yield*` (生成器) | `Unsupported expression type` |
| `async function*` (异步生成器) | 未测试 |
| 动态 `import()` | 需使用静态 `import` |
| 私有字段 `#field` | 不支持 |
| `new.target` | meta property not supported |
| Spread 参数 `fn(...args)` | `Spread argument not supported` |
| `for await...of` | `Promise.{}() not supported` |
| 标签模板 `` tag`...` `` | `Unsupported expression type` |
| `import.meta` | 未实现 (ES 模块元数据) |

---

## 3. 语句 (Statements)

### 3.1 变量声明 - ✅ 100% 实现

| 特性 | 状态 | Zig 输出 | 测试 |
|------|------|----------|------|
| `var x = val` | ✅ | `var x: T = val;` | 所有测试 |
| `let x = val` | ✅ | `const x = val;` (如未修改) | 同上 |
| `const x = val` | ✅ | `const x = val;` | 同上 |
| 解构 `const {a, b} = obj` | ✅ | 展平为逐字段访问 | showcase-project |
| 解构默认值 `const {a = 1} = obj` | 🚧 | 跳过默认值 | 未测试 |

### 3.2 函数声明 - ✅ 100% 实现

| 特性 | 状态 | Zig 输出 | 测试 |
|------|------|----------|------|
| `function fn(params) {}` | ✅ | `pub fn fn(params) ret {}` | 所有测试 |
| `export function fn(params) {}` | ✅ | 生成 C ABI wrapper（arena 自动管理内存） | 同上 |
| `async function fn(params) {}` | ✅ | 添加 `io: Io` 参数 | test-bin-project |
| 默认参数 `function fn(a = 1) {}` | ✅ | `a: i64 = 1` | 隐式测试 |
| Rest 参数 `function fn(...args) {}` | ✅ | `args: []const i64` | showcase-project |
| 嵌套函数声明 | ❌ | 报错（需重构为顶层） | - |
| `arguments` 对象 | ❌ | 未实现 | 传统函数参数对象 |

**注意**:
- `arguments` 是传统函数（非箭头函数）内部的类数组对象，包含调用时传入的所有参数

### 3.3 类声明 - ✅ 90% 实现

| 特性 | 状态 | Zig 输出 | 测试 |
|------|------|----------|------|
| `class Name { ... }` | ✅ | `const Name = struct { ... };` | showcase-project |
| 构造函数 `constructor()` | ✅ | `pub fn init() Name { ... }` | 同上 |
| 实例方法 | ✅ | `pub fn method(self: *const Name) {}` | 同上 |
| 静态方法 | ✅ | `pub fn method() {}` (无 `self`) | 同上 |
| 静态属性 | ✅ | `pub const prop = val;` | 同上 |
| Getter/Setter | ✅ | `pub fn get_prop() T {}` / `pub fn set_prop(v: T) {}` | `test_native_proto_getter_*` |
| `extends` 继承 | ✅ | 内嵌 `base: ParentType` 字段 | showcase-project |
| `super` 调用 | ✅ | `self.base.method()` | 同上 |
| 私有字段 `#field` | ❌ | `@compileError` | - |
| 类表达式 `const X = class {}` | ❌ | `@compileError` | - |
| 静态初始化块 `static {}` | ❌ | 未实现 | - |

### 3.4 控制流语句 - ✅ 95% 实现

| 特性 | 状态 | Zig 输出 | 测试 |
|------|------|----------|------|
| `if...else` | ✅ | `if (cond) {} else {}` | `test_native_proto_control_flow` |
| `if...else if...else` | ✅ | 嵌套 `if...else` | 同上 |
| `switch` | ✅ | `_ = switch (val) { ... }` | 同上 |
| `for (init; test; update)` | ✅ | `{ init; while (test) : (update) {} }` | showcase-project |
| `for...of` | ✅ | `for (iterable) \|item\| {}` | 同上 |
| `for...in` (动态对象) | ✅ | HashMap iterator | `test_native_proto_for_in` |
| `for...in` (静态 struct) | ✅ | 字段展开循环 | `test_native_proto_for_in_static` |
| `while` | ✅ | `while (cond) {}` | showcase-project |
| `do...while` | ✅ | `while (true) { ... if (!cond) break; }` | `test_native_proto_do_while` |
| `break` / `continue` | ✅ | `break` / `continue` | showcase-project |
| 标签语句 `label: while` | ✅ | `label: while {}` | 未测试 |
| `for await...of` | ❌ | `@compileError` | - |

### 3.5 错误处理 - ✅ 100% 实现

| 特性 | 状态 | Zig 输出 | 测试 |
|------|------|----------|------|
| `throw expr` | ✅ | `return error.JsThrow` 或 `break :_try error.JsThrow` | `test_native_proto_throw_*` |
| `try { ... } catch (e) { ... }` | ✅ | `defer { ... } _ = _try0: { ... } catch { ... }` | 同上 |
| `try { ... } finally { ... }` | ✅ | `defer { cleanup }` | 同上 |
| 嵌套 try-catch | ✅ | 支持 | 同上 |

### 3.6 其他语句 - 🚧 部分实现

| 特性 | 状态 | Zig 输出 | 测试 |
|------|------|----------|------|
| 表达式语句 | ✅ | `expr;` | 所有测试 |
| 块语句 `{ }` | ✅ | `{ }` | 同上 |
| 空语句 `;` | ✅ | 忽略 | 未测试 |
| `with` 语句 | ❌ | JS 严格模式已废弃 | - |
| `debugger` 语句 | ❌ | 不支持 | - |
| 声明 + 表达式混合 | 🚧 | 可能产生未使用变量警告 | 隐式测试 |

---

## 4. 内置对象 (Built-in Objects)

> **评估方法**: 内置对象经过三层流水线才能正常工作：
> 1. **检测 (Detect)** — `native_builtins.rs` 的 `BuiltinCall` 枚举 + `detect_builtin_call()`
> 2. **发射 (Emit)** — `codegen/expr.rs` 的 `emit_builtin_call()` 生成 Zig 代码
> 3. **运行时 (Runtime)** — `runtime/*.zig` 提供 Zig 侧实现
>
> 三层全部 ✅ 才算"有效覆盖"。仅 runtime 有但检测/发射缺失 → 实际不可用。

### 4.1 `Math` — 11/43 (26%)

| 方法/属性 | 检测 | 发射 | 运行时 | 状态 |
|----------|------|------|--------|------|
| `Math.abs/ceil/floor/round/sqrt` | ✅ | ✅ | Zig 内置 | ✅ |
| `Math.random()` | ✅ | ✅ | `@random` | ✅ |
| `Math.pow(b,e)` | ✅ | ✅ | `@pow` | ✅ |
| `Math.max/min(...)` | ✅ | ✅ | `@max/@min(args)` | ✅ |
| `Math.PI` | ✅ | ✅ | `std.math.pi` | ✅ |
| `Math.hypot(a,b)` | ✅ | ✅ | `@compileError` | 🔶 故意不支持 |
| `Math.sin/cos/tan` | ❌ | ❌ | — | ❌ |
| `Math.asin/acos/atan/atan2` | ❌ | ❌ | — | ❌ |
| `Math.log/log10/log2/exp` | ❌ | ❌ | — | ❌ |
| `Math.sign/trunc/cbrt` | ❌ | ❌ | — | ❌ |
| `Math.sinh/cosh/tanh` 等 | ❌ | ❌ | — | ❌ |
| `Math.E/LN2/LN10/LOG2E/SQRT2` 等常量 | ❌ | ❌ | — | ❌ |

### 4.2 `Array` — 14/32 (44%, 含 5 stub)

| 方法 | 检测 | 发射 | 运行时 | 状态 |
|------|------|------|--------|------|
| `.push/pop/shift/unshift` | ✅ | ✅ | 内联 ArrayList | ✅ |
| `.reverse/sort` | ✅ | ✅ | 内联 swap/sort | ✅ |
| `.indexOf/includes/join/slice/splice` | ✅ | ✅ | 内联操作 | ✅ |
| `.forEach/map/reduce` | ✅ | ✅ | 内联 for + 闭包 | ✅ |
| `.filter/some/every` | ✅ | ✅ | 返回原数组/true (stub) | 🔶 stub |
| `.flat/flatMap` | ✅ | ✅ | 返回原数组 (stub) | 🔶 stub |
| `.concat/find/findIndex/fill/lastIndexOf/at/copyWithin/keys/values/entries/reduceRight` | ❌ | ❌ | — | ❌ |

### 4.3 `String` — 8/26 (31%)

| 方法 | 检测 | 发射 | 运行时 | 状态 |
|------|------|------|--------|------|
| `.indexOf/includes/startsWith/endsWith/trim/split/padStart/padEnd` | ✅ | ✅ | ✅ js_string | ✅ |
| `.charAt/charCodeAt/concat/slice/replace/repeat` | ❌ | ❌ | ✅ runtime 已实现 | ❌ 未连线 |
| `.toUpperCase/toLowerCase` | ❌ | ❌ | ✅ toUpper/toLower | ❌ 未连线 |
| `.substring/trimStart/trimEnd/match/search/localeCompare/at/codePointAt` | ❌ | ❌ | ❌ | ❌ |
| `String.fromCharCode/fromCodePoint/raw` | ❌ | ❌ | ❌ | ❌ |

### 4.4 `Map` — 5/11 (45%)

| 方法 | 检测 | 发射 | 运行时 | 状态 |
|------|------|------|--------|------|
| `.set/get/has/delete` | ✅ | ✅ | ✅ JsMap | ✅ |
| `new Map()` | ✅ | ✅ | ✅ JsMap.init | ✅ |
| `.clear/size` | ❌ | ❌ | ✅ runtime 已实现 | ❌ 未连线 |
| `.keys/values/entries/forEach` | ❌ | ❌ | ❌ | ❌ |

### 4.5 `Set` — 2/9 (22%)

| 方法 | 检测 | 发射 | 运行时 | 状态 |
|------|------|------|--------|------|
| `.add` | ✅ | ✅ | ✅ JsSet.add | ✅ |
| `new Set()` | ✅ | ✅ | ✅ JsSet.init | ✅ |
| `.has/delete` | ❌ | ❌ | ✅ runtime 已实现 | ❌ (路由到 Map 场景) |
| `.clear/size` | ❌ | ❌ | ✅ runtime 已实现 | ❌ |
| `.keys/values/entries/forEach` | ❌ | ❌ | ❌ | ❌ |

### 4.6 `Object` — 5/12 (42%)

| 方法 | 检测 | 发射 | 运行时 | 状态 |
|------|------|------|--------|------|
| `Object.keys/values/entries/assign` | ✅ | ✅ | ✅ js_object | ✅ |
| `Object.freeze` | ✅ | ✅ | no-op | 🔶 Zig 默认不可变 |
| `Object.create/defineProperty/getOwnPropertyNames/hasOwn/is/seal/fromEntries` | ❌ | ❌ | ❌ | ❌ |

### 4.7 `JSON` — 2/2 (100%) ✅

| 方法 | 检测 | 发射 | 运行时 | 状态 |
|------|------|------|--------|------|
| `JSON.stringify` | ✅ | ✅ | ✅ js_json.stringify | ✅ |
| `JSON.parse` | ✅ | ✅ | ✅ js_json.parse | ✅ |

### 4.8 `Date` — 11/55+ (~20%)

| 方法 | 检测 | 发射 | 运行时 | 状态 |
|------|------|------|--------|------|
| `Date.now()` | ✅ | ✅ | ✅ js_date.now | ✅ |
| `Date.parse(s)` | ✅ | ✅ | ✅ (stub 返回 0) | 🔶 |
| `.getTime/getFullYear/getMonth/getDate/getDay/getHours/getMinutes/getSeconds` | ✅ | ✅ | ✅ JsDate | ✅ |
| `.getMilliseconds` | ❌ | ❌ | ❌ | ❌ |
| UTC getter 系列 (8) / setter 系列 (~7) / `.toISOString/toString/valueOf` | ❌ | ❌ | ❌ | ❌ |
| `new Date()` / `new Date(ms)` | ❌ | ❌ | ❌ | ❌ |
| `Date.UTC(y,m,d)` | ✅ | ✅ | `@compileError` | 🔶 |

**已知限制**: 所有 getHours/getMinutes/getSeconds 返回 UTC 时间（等效 JS `getUTCHours()`）；calcMonth/calcDate 使用简化近似。

### 4.9 全局函数 — 1/8 (13%)

| 函数 | 检测 | 发射 | 运行时 | 状态 |
|------|------|------|--------|------|
| `parseInt(s)` | ✅ | ✅ | `std.fmt.parseInt` | ✅ |
| `parseFloat(s)` | ❌ | ❌ | ✅ runtime 已实现 | ❌ |
| `isNaN/isFinite` | ❌ | ❌ | ✅ runtime 已实现 | ❌ |
| `encodeURIComponent/decodeURIComponent` | ❌ | ❌ | ✅ runtime 已实现 | ❌ |
| `encodeURI/decodeURI/eval` | ❌ | ❌ | ❌ | ❌ |

### 4.10 `Number` — 0/5+ (0%)

| 方法 | 检测 | 发射 | 运行时 | 状态 |
|------|------|------|--------|------|
| `Number.isNaN/isFinite/isInteger/parseInt/parseFloat` | ❌ | ❌ | ✅ runtime 已实现 | ❌ |
| `Number.MAX_VALUE` 等常量 | ❌ | ❌ | ❌ | ❌ |

### 4.11 `console` — 0/3 (0%) ❌

| 方法 | 检测 | 发射 | 运行时 | 状态 |
|------|------|------|--------|------|
| `console.log/error/warn` | ❌ | ❌ | ✅ js_console | ❌ 完全缺失 |

**这是最高频使用的缺失项**，runtime 已实现但 codegen 未连线。

### 4.12 `RegExp` — 0/2 (0%)

| 方法 | 检测 | 发射 | 运行时 | 状态 |
|------|------|------|--------|------|
| 正则字面量 `/pat/` | ✅ | ✅ | 字符串提取 | ✅ 语法可用 |
| `.test/.exec` | ❌ | ❌ | ✅ runtime 已实现 | ❌ 未连线 |

### 4.13 `TypedArray` — 10/11 (91%) ✅

| 特性 | 检测 | 发射 | 运行时 | 状态 |
|------|------|------|--------|------|
| `Int8Array` ~ `Float64Array` / `.length` / 构造 | ✅ | ✅ | ✅ | ✅ |
| `.get/.set/.subarray/.copyWithin/.fill/.buffer/.byteLength/.byteOffset` | ✅ | ✅ | ✅ js_typedarray | ✅ |
| `.slice()` | ❌ | ❌ | ❌ | ❌ (用 `.subarray()` 替代) |

### 4.14 `Promise` — 0/x (0%)

| 特性 | 检测 | 发射 | 运行时 | 状态 |
|------|------|------|--------|------|
| `new Promise()/.then/.catch` | ❌ | ❌ | — | ❌ native_proto 显式拒绝 |

**建议**: 使用 `async/await` + `Io` 模式替代 Promise API。

### 4.15 `Error` — 1/1 (100%) ✅

| 特性 | 状态 |
|------|------|
| `throw new Error(msg)` → `error.JsThrow` | ✅ |

### 4.16 未实现类别

| 类别 | 状态 |
|------|------|
| `Symbol` | ❌ 完全缺失 |
| `WeakMap` | ❌ 完全缺失 |
| `WeakSet` | ❌ 完全缺失 |

### 4.17 汇总

| 类别 | 总方法数 | 有效覆盖 | 比例 | 备注 |
|------|---------|---------|------|------|
| Math | 43 | 11 | 26% | |
| Array | 32 | 14 | 44% | 含 5 stub |
| String | 26 | 8 | 31% | 8 runtime 已实现但未连线 |
| Map | 11 | 5 | 45% | |
| Set | 9 | 2 | 22% | |
| Date | 55+ | 11 | ~20% | |
| Object | 12 | 5 | 42% | |
| JSON | 2 | 2 | 100% | |
| Global | 8 | 1 | 13% | 5 runtime 已实现但未连线 |
| console | 3 | 0 | 0% | 最高频缺失 |
| Number | 5+ | 0 | 0% | |
| RegExp | 2 | 0 | 0% | 字面量可用 |
| TypedArray | 11 | 10 | 91% | |
| **总计** | **~219** | **57** | **~26%** | |

> ⚠️ **最大低挂果实**: 21 个 runtime 函数已实现但 codegen 检测/发射路径未连接（见 `runtime/` 目录下 `js_string.toUpper/toLower/charAt/charCodeAt/concat/slice/replace/repeat`、`js_number.*`、`js_uri.*`、`js_console.*`、`js_map.clear`、`js_set.clear/has/delete`），只需添加 `BuiltinCall` 变体即可工作。

---

## 5. 模块系统 (Modules)

### 5.1 `import` / `export` - ✅ 100% 实现

| 特性 | 状态 | 说明 | 测试 |
|------|------|------|------|
| `import { name } from './file.js'` | ✅ | AST 驱动提取 | showcase-project |
| `import defaultExport from './file.js'` | ✅ | 同上 | 同上 |
| `import * as ns from './file.js'` | ✅ | 同上 | 未测试 |
| `export function fn() {}` | ✅ | 生成 C ABI wrapper（arena 自动管理内存） | 所有测试 |
| `export const x = val` | ✅ | 导出为 C ABI 函数 | 同上 |
| `export default expr` | ✅ | 标记为 default 导出 | 未测试 |
| 多文件分组 | ✅ | DFS 依赖排序 | showcase-project |

### 5.2 C ABI 内存管理 - ✅ 100% 实现

**设计**：双 Arena 全局分配器（主备热切换），所有 Zig 侧内存分配通过全局 arena 进行，调用方无需手动释放内存。

#### 核心机制：`js_allocator.zig`

**双 Arena 状态机**：

```
Arena A:  ready  --(成为 active)-->  active  --(容量超限)-->  cooling  --(冷却期结束)-->  ready
Arena B:  ready  --(成为 active)-->  active  --(容量超限)-->  cooling  --(冷却期结束)-->  ready
```

- 任意时刻只有一个 arena 是 `active`（用于所有分配），另一个是非激活状态（`cooling` 或 `ready`）
- 当 `active` arena 容量超过 `JS2RUST_MAX_ARENA_MB`（默认 100MB）且备用 arena 是 `ready` 时，两者交换
- 退出的 arena 进入 `cooling` 状态，保持存活 `JS2RUST_ARENA_GRACE_MS`（默认 5000ms = 5 秒），确保已返回的指针在 FFI 消费窗口内保持有效
- `cooling` 到期后，arena 被 `reset`（内存回收），状态回到 `ready`

**关键设计**：

1. **惰性回收**：冷却定时器检查在 `getAllocator()` 内部惰性执行，无需后台线程
2. **线程安全**：使用原子自旋锁（`g_lock`）保护状态转换
3. **环境配置**：
   - `JS2RUST_MAX_ARENA_MB`（默认 100）：触发主备交换的容量阈值
   - `JS2RUST_ARENA_GRACE_MS`（默认 5000）：冷却期毫秒数

**C ABI 导出函数**：

| 函数 | 说明 |
|------|------|
| `js2rust_init()` | 初始化全局分配器 + Io（在 Rust 侧调用） |
| `js2rust_deinit()` | 释放全局分配器 + Io（在 Rust 侧调用） |
| `js2rust_reset()` | 强制主备交换（将当前 active 送入冷却，备用变为 active） |

#### 字符串返回：`string.zig`

**`StrRet` 结构体**（C ABI 兼容）：

```zig
pub const StrRet = extern struct {
    ptr: [*c]const u8,
    len: isize,  // >= 0: 字符串长度; < 0: 错误标志，|len| = 错误名长度
};
```

**符号位约定**：

- `len >= 0`：正常字符串（arena 分配，Zig 侧拥有所有权）
- `len < 0`：异步错误传播（ `@errorName(err)` 静态字符串，无需释放）

**Rust 侧对应**：`#[repr(C)] struct __JsStr { ptr: *const u8, len: isize }`

#### Host 函数字符串处理：`host.rs`

**字符串参数（Zig → Rust）**：

1. Zig 侧调用 `js_allocator.g_alloc().dupeZ(u8, str)` 创建以 `\0` 结尾的 C 字符串
2. 调用 `defer js_allocator.g_alloc().free(c_str)` 确保在函数返回后释放 C 字符串
3. 将 `c_str` 传递给 Rust（`[*:0]const u8`）

**字符串返回（Rust → Zig）**：

1. Rust 侧用 `CString::into_raw()` 分配内存并返回指针
2. Zig 侧用 `std.mem.span(raw)` 获取切片长度
3. Zig 侧用 `js_allocator.g_alloc().dupe(u8, span)` 复制到 arena
4. Zig 侧调用 `host_free(@ptrCast(raw))` 释放 Rust 分配的内存
5. 返回 arena 分配的副本（由双 Arena 自动管理生命周期）

**内存所有权**：

- Rust 分配 → Zig 复制 → Rust 释放 → Zig arena 拥有副本
- 调用方（Rust）无需手动释放，arena 在冷却期后自动回收

#### 示例：完整调用流程

```
Rust: js2rust_init()  // 初始化 Arena A (active), Arena B (ready)

Rust: call greet("Alice")  // C ABI 调用
  └─ Zig: 使用 getAllocator() (Arena A)
  └─ Zig: 生成字符串 "Hello, Alice" (Arena A 分配)
  └─ Zig: 返回 StrRet { .ptr = arena_ptr, .len = 13 }
  └─ Rust: 使用字符串 (指针有效，因为 Arena A 仍 active)
  └─ Rust: 下一次 FFI 调用前 / js2rust_reset() 后:
       - 如果 Arena A 超过 100MB → 交换 → Arena A 进入 cooling (5 秒)
       - Arena B 变为 active
       - 5 秒后 Arena A 被 reset (指针失效，但 Rust 已消费完毕)

Rust: js2rust_deinit()  // 释放两个 arena
```

| 特性 | 状态 | 说明 |
|------|------|------|
| 双 Arena 分配器 | ✅ | 主备热切换 + 冷却期保证指针有效性 |
| 自动内存释放 | ✅ | Arena 统一回收，调用方无需手动释放 |
| 字符串返回 | ✅ | `StrRet` 结构体 + 符号位约定 |
| Host 函数字符串参数 | ✅ | `dupeZ` + `defer free`（Zig → Rust） |
| Host 函数字符串返回 | ✅ | `span` + `dupe` + `host_free`（Rust → Zig） |
| 异步 Host 函数 | ✅ | `Io.Threaded` + `io.async()` 模式 |

---

## 6. 类型系统 (Type System)

### 6.1 设计概览

类型系统采用**两遍分离架构**：第一遍 `TypeInferrer` 遍历完整 AST 生成 `TypeCheckResult`，第二遍 `Codegen` 只读 `TypeCheckResult` 生成 Zig 代码。推断与代码生成完全解耦。

**核心数据结构：**

```
TypeInferrer  →  (推断阶段)  收集所有类型信息
TypeCheckResult  →  (只读快照)  传递给 Codegen
ZigType  →  (类型枚举)  表示推断出的 Zig 类型
InferResult  →  Definite(ZigType) | Indeterminate
```

### 6.2 类型推断规则（8 条简化规则）

| 规则 | 说明 | 示例 |
|------|------|------|
| 1. 字面量精确推断 | 字面量表达式 → 确定类型（有 JSDoc 则用 JSDoc） | `42` → `i64`, `"hi"` → `[]const u8` |
| 2. 二元表达式 | 仅当**两个**操作数都是字面量时才确定类型 | `2 + 3` → `i64`, `x + y` → Indeterminate |
| 3. 其他表达式 | 一律 Indeterminate | 函数调用、成员访问等 |
| 4. `const` 声明 | 不生成类型注解，让 Zig 自行推断 | `const x = expr;` |
| 5. 局部变量 | 检查**所有**赋值，至少一个确定 → 使用该类型 | `let x = 1; x = 2;` → `i64` |
| 6. 返回类型 | 检查**所有** return 表达式，至少一个确定 → 使用该类型 | `if (c) return 1; return 2;` → `i64` |
| 7. 非导出函数参数 | Indeterminate → `anytype` | `function f(x)` → `f(x: anytype)` |
| 8. Indeterminate 报错 | 导出函数参数 / CABI 返回类型若为 Indeterminate → 编译错误 | 要求 JSDoc 标注 |

**特殊推断：**
- 箭头函数闭包：自动生成 `Closure` 结构体类型（value capture / reference capture）
- Host 函数返回类型：`host_return_types` + `host_struct_fields` 查表
- 可选链 `?.`：返回 `InferResult::Indeterminate`（Zig 从 `else null` 自动推导 `?T`）
- `JSON.parse(@type)`：通过 `has_json_parse_types` 标记，生成类型转换代码

### 6.3 `ZigType` 类型枚举

| 变体 | Zig 类型 | 说明 |
|------|----------|------|
| `Void` | `void` | 无返回值 |
| `I64` | `i64` | 整数 |
| `F64` | `f64` | 浮点数 |
| `Bool` | `bool` | 布尔值 |
| `Str` | `[]const u8` | 字符串 |
| `ArrayList(inner)` | `std.ArrayList(T)` | 动态数组，T 为元素类型 |
| `Struct(fields)` | 匿名结构体 | `.{ .field1 = T1, .field2 = T2 }` |
| `NamedStruct(name)` | 命名结构体 | 由 `HostStructDef` 定义（如 `"UserInfo"`） |
| `Anytype` | `anytype` | 非导出函数参数，留待 Zig 推断 |

**类型兼容性：** `I64` 可宽化到 `F64`（`is_compatible_with`），其他组合同类型才兼容。

**C ABI 类型映射：** `Str` → `StrRet`（extern struct `{ ptr, len }`），`Struct`/`NamedStruct` → 对应 C ABI struct。

### 6.4 类型映射（JS → Zig）

| JS 类型 | Zig 类型 | 备注 |
|---------|----------|------|
| `number`（整数运算） | `i64` | `/` 运算符触发 `F64` 宽化 |
| `number`（浮点/除法） | `f64` | |
| `string` | `[]const u8` | C ABI 返回时用 `StrRet` |
| `boolean` | `bool` | |
| `null` / `undefined` | `void` | 用作返回类型时 |
| `object`（已知字段） | 匿名 `struct` | `.{ .name = []const u8, .age = i64 }` |
| `object`（Host 定义） | `NamedStruct` | `HostStructDef` 中定义 |
| `object`（动态） | `std.StringHashMap(ZigType)` | 通过 `Map` 模拟 |
| `array`（字面量） | `[_]T{ ... }` | 元素类型统一推断 |
| `array`（动态） | `std.ArrayList(T)` | |
| `function` | 函数类型 或 闭包结构体 | 闭包自动生成 `Closure` 泛型结构体 |
| `any` | `anytype` | 非导出函数参数 |
| TypedArray | `[]T`（Zig 切片） | 完整支持 method/accessor (.get/.set/.subarray/.buffer 等) |

### 6.5 JSDoc 类型标注

| 注解 | 作用 |
|------|------|
| `@type {type}` | 变量类型强制标注 |
| `@param {type} name` | 函数参数类型（解决 Rule 8 错误） |
| `@returns {type}` | 函数返回类型 |
| `@typedef {field: type}` | 定义命名结构体类型 |

---

## 7. 测试覆盖 (Test Coverage)

### 7.1 Rust 单元测试 - 145 个测试

| 测试模块 | 测试数量 | 覆盖特性 |
|----------|----------|----------|
| `native_proto::tests` | 122 | 所有核心语法、内置对象、闭包、错误处理 |
| `native_proto::jsdoc` | 9 | JSDoc 解析与类型标注 |
| `parser` | 7 | oxc_ast 解析器集成 |
| `sourcemap` | 4 | Source Map 生成 |
| `testgen` | 3 | Zig 测试代码生成 |

### 7.2 测试覆盖情况

已覆盖所有完全实现特性的核心路径。

---

**文档版本**: 2.4  
**最后更新**: 2026-06-23  
**作者**: jonathan197608
