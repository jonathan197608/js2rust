# JS 语言特性实现评估

> **项目**: js2rust (JS → Zig 转译器)
> **评估日期**: 2026-06-23
> **代码版本**: main branch (1651070)
> **测试覆盖**: 145 个 Rust 测试 (122 native_proto + 9 jsdoc + 7 parser + 4 sourcemap + 3 testgen) + 3 个示例项目

---

## 1. 执行总结

| 指标 | 数值 | 占比 |
|------|------|------|
| **JS 语法特性总数** | ~150+ | - |
| **完全实现** | ~85 | ~57% |
| **部分实现** | ~5 | ~3% |
| **未实现（@compileError）** | ~60 | ~40% |
| **测试覆盖** | 145 个 Rust 测试 (122 native_proto + 23 其他) | - |

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
| 多 spread 合并 `{ ...a, ...b }` | ❌ | 不支持 | - |

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

### 4.1 `Math` - ✅ 100% 实现

| 方法/属性 | 状态 | Zig 输出 | 测试 |
|----------|------|----------|------|
| `Math.PI`, `Math.E` | ✅ | `std.math.pi`, `std.math.e` | 隐式测试 |
| `Math.abs(x)` | ✅ | `@abs(x)` | `test_native_proto_math_*` |
| `Math.ceil/floor/trunc/round(x)` | ✅ | `@ceil/@floor/@trunc/@round(x)` | 同上 |
| `Math.sqrt(x)` | ✅ | `@sqrt(x)` | 同上 |
| `Math.sin/cos/tan/asin/acos/atan(x)` | ✅ | `@sin/@cos/...` | 同上 |
| `Math.atan2(y, x)` | ✅ | `@atan2(y, x)` | 同上 |
| `Math.exp/log/log2/log10(x)` | ✅ | `@exp/@log/...` | 同上 |
| `Math.min/max(a, b)` | ✅ | `@min/@max(a, b)` | 同上 |
| `Math.pow(b, e)` | ✅ | `std.math.pow(f64, b, e)` | 同上 |
| `Math.random()` | ✅ | `std.crypto.random.float(f64)` | 同上 |
| `Math.sign(x)` | ✅ | 三路 if 表达式 | 同上 |
| `Math.hypot(a, b)` | ✅ | `@sqrt(a*a + b*b)` | 同上 |

### 4.2 `String` - ✅ 95% 实现

| 方法 | 状态 | Zig 输出 | 测试 |
|------|------|----------|------|
| `.length` | ✅ | `.len` | showcase-project |
| `.toUpperCase()` | ✅ | `js_string.toUpper(s)` | 隐式测试 |
| `.toLowerCase()` | ✅ | `js_string.toLower(s)` | 同上 |
| `.charAt(i)` | ✅ | `js_string.charAt(s, i)` | 同上 |
| `.charCodeAt(i)` | ✅ | `s[@intCast(i)]` | 同上 |
| `.concat(other)` | ✅ | `js_string.concat(s, other)` | `test_native_proto_string_concat_*` |
| `.includes(sub)` | ✅ | `js_string.includes(s, sub)` | 同上 |
| `.indexOf(sub)` | ✅ | `js_string.indexOf(s, sub)` | 同上 |
| `.startsWith(pre)` | ✅ | `js_string.startsWith(s, pre)` | 同上 |
| `.endsWith(suf)` | ✅ | `js_string.endsWith(s, suf)` | 同上 |
| `.slice(start, end)` | ✅ | `js_string.slice(s, start, end)` | 同上 |
| `.split(sep)` | ✅ | `js_string.split(s, sep)` | 同上 |
| `.replace(old, new)` | ✅ | `js_string.replace(s, old, new)` | 同上 |
| `.trim()` | ✅ | `js_string.trim(s)` | 同上 |
| `.repeat(n)` | ✅ | `js_string.repeat(s, n)` | 同上 |
| `.padStart/padEnd` | ❌ | 未实现 | - |

### 4.3 `Array` - ✅ 95% 实现

| 方法 | 状态 | Zig 输出 | 测试 |
|------|------|----------|------|
| `.length` | ✅ | `.len` | showcase-project |
| `.push(val)` | ✅ | `js_array.push(arr, val)` | Phase 5 测试 |
| `.pop()` | ✅ | `js_array.pop(arr)` | 同上 |
| `.shift()` | ✅ | `js_array.shift(arr)` | 同上 |
| `.unshift(val)` | ✅ | `js_array.unshift(arr, val)` | 同上 |
| `.indexOf(val)` | ✅ | `js_array.indexOf(arr, val)` | `test_native_proto_array_indexof` |
| `.includes(val)` | ✅ | `js_array.includes(arr, val)` | `test_native_proto_array_includes` |
| `.join(sep)` | ✅ | `js_array.join(arr, sep)` | `test_native_proto_array_join` |
| `.reverse()` | ✅ | `js_array.reverse(arr)` | 同上 |
| `.sort()` | ✅ | `js_array.sort(arr)` | 同上 |
| `.slice(s, e)` | ✅ | `js_array.slice(arr, s, e)` | `test_native_proto_array_slice` |
| `.splice(s, n, ...)` | ✅ | 支持删除+插入 | `test_native_proto_array_splice*` |
| `.concat(other)` | ✅ | `js_array.concat(arr, other)` | 同上 |
| `.map(fn)` | ✅ | `js_array.map(arr, fn)` | Phase 5 测试 |
| `.filter(fn)` | ✅ | `js_array.filter(arr, fn)` | 同上 |
| `.reduce(fn, init)` | ✅ | `js_array.reduce(arr, fn, init)` | 同上 |
| `.forEach(fn)` | ✅ | `js_array.forEach(arr, fn)` | 同上 |
| `.some(fn)` / `.every(fn)` | ✅ | `js_array.some/every(arr, fn)` | 同上 |
| `.flat()` / `.flatMap()` | ❌ | 未实现 | - |
| `Array.isArray(x)` | ✅ | `js_array.isArray(x)` | 未测试 |

### 4.4 `Map` - ✅ 100% 实现

| 方法 | 状态 | Zig 输出 | 测试 |
|------|------|----------|------|
| `.get(key)` | ✅ | `map.get(key)` | showcase-project |
| `.set(key, val)` | ✅ | `map.set(key, val)` | 同上 |
| `.has(key)` | ✅ | `map.has(key)` | 同上 |
| `.delete(key)` | ✅ | `map.delete(key)` | 同上 |
| `.clear()` | ✅ | `map.clear()` | 未测试 |
| `.size` | ✅ | `map.size()` | showcase-project |

### 4.5 `Set` - ✅ 100% 实现

| 方法 | 状态 | Zig 输出 | 测试 |
|------|------|----------|------|
| `.add(val)` | ✅ | `set.add(val)` | showcase-project |
| `.has(val)` | ✅ | `set.has(val)` | 同上 |
| `.delete(val)` | ✅ | `set.delete(val)` | 同上 |
| `.clear()` | ✅ | `set.clear()` | 未测试 |
| `.size` | ✅ | `set.size()` | showcase-project |

### 4.6 `Object` - ✅ 80% 实现

| 方法 | 状态 | Zig 输出 | 测试 |
|------|------|----------|------|
| `Object.keys(obj)` | ✅ | `js_object.keys(obj)` | 未测试 |
| `Object.values(obj)` | ✅ | `js_object.values(obj)` | 同上 |
| `Object.entries(obj)` | ✅ | `js_object.entries(obj)` | 同上 |
| `Object.assign(target, source)` | ✅ | `js_object.assign(target, source)` | 同上 |
| `Object.freeze/seal/preventExtensions` | ❌ | 未实现（Zig 无运行时冻结） | - |

### 4.7 `JSON` - ✅ 100% 实现

| 方法 | 状态 | Zig 输出 | 测试 |
|------|------|----------|------|
| `JSON.stringify(obj)` | ✅ | `js_json.stringify(alloc, obj)` | 隐式测试 |
| `JSON.parse(str)` | ✅ | `js_json.parse(alloc, str)` | `parseUserJson` 测试 |

### 4.8 `Date` - ✅ 100% 实现

| 方法 | 状态 | Zig 输出 | 测试 |
|------|------|----------|------|
| `Date.now()` | ✅ | `std.time.milliTimestamp()` (Zig 0.16.0 兼容) | showcase-project |
| `date.getTime()` | ✅ | `date.getTime()` | 未测试 |
| `date.getFullYear()` | ✅ | `date.getFullYear()` | 同上 |
| `date.getMonth()` | ✅ | `date.getMonth()` | 同上 |
| `date.getDate()` | ✅ | `date.getDate()` | 同上 |
| `date.getDay()` | ✅ | `date.getDay()` | 同上 |
| `date.getHours()` | ✅ | `date.getHours()` | 同上 |
| `date.getMinutes()` | ✅ | `date.getMinutes()` | 同上 |
| `date.getSeconds()` | ✅ | `date.getSeconds()` | 同上 |
| `Date.UTC(y, m, d, ...)` | ❌ | `@compileError("Date.UTC is not yet implemented")` | - |

**注意**:
- `Date.UTC()` 是静态方法，当前生成编译错误

### 4.9 `Number` - ✅ 100% 实现

| 方法/属性 | 状态 | Zig 输出 | 测试 |
|----------|------|----------|------|
| `Number.isNaN(x)` | ✅ | `std.math.isNan(@as(f64, x))` | 隐式测试 |
| `Number.isFinite(x)` | ✅ | `!std.math.isInf(x)` | 同上 |
| `Number.isInteger(x)` | ✅ | `@as(bool, ...)` | 同上 |
| `Number.parseInt(s)` / `parseInt(s)` | ✅ | `std.fmt.parseInt(i64, s, 10)` | 隐式测试 |
| `Number.parseFloat(s)` / `parseFloat(s)` | ✅ | `std.fmt.parseFloat(f64, s)` | 同上 |

### 4.10 `console` - ✅ 100% 实现

| 方法 | 状态 | Zig 输出 | 测试 |
|------|------|----------|------|
| `console.log(...)` | ✅ | `std.debug.print(...)` | 所有测试 |
| `console.error(...)` | ✅ | `std.debug.print(...)` (stderr) | 未测试 |
| `console.warn(...)` | ✅ | `std.debug.print(...)` (stderr) | 未测试 |

### 4.11 `RegExp` - 🚧 20% 实现

| 方法 | 状态 | Zig 输出 | 测试 |
|------|------|----------|------|
| 正则表达式字面量 `/pattern/` | ✅ | `"pattern"` (提取 pattern 字符串) | 未测试 |
| `.test(s)` | 🚧 | 需手动实现匹配逻辑 | 未测试 |
| `.exec(s)` | 🚧 | 需手动实现匹配逻辑 | 未测试 |
| 完整正则引擎 | ❌ | 不支持（需引入 C 库） | - |

### 4.12 `Promise` - ❌ 不支持

| 特性 | 状态 | 说明 |
|------|------|------|
| `new Promise((resolve, reject) => { ... })` | ❌ | `@compileError` (使用 `async/await` 替代) |
| `.then()` / `.catch()` / `.finally()` | ❌ | 不支持 |
| `Promise.resolve()` / `Promise.reject()` | ❌ | 不支持 |
| `Promise.all()` / `Promise.race()` | ❌ | 不支持 |

**建议**: JS 的 `Promise` 对应 Zig 的 `Io` 模式 + `async/await`，无需直接翻译 `Promise` API。

### 4.13 `TypedArray` - ✅ 100% 实现

| 特性 | 状态 | Zig 输出 | 测试 |
|------|------|----------|------|
| `Int8Array` / `Uint8Array` / ... / `Float64Array` | ✅ | Zig 切片 `[]T` | `test_native_proto_typedarray_*` |
| `.length` | ✅ | `.len` | 同上 |
| 构造 `new Uint8Array([...])` | ✅ | Zig 数组字面量 | 同上 |
| `.get(idx)` | ✅ | `js_typedarray.get{Type}(slice, idx)` | 同上 |
| `.set(idx, val)` | ✅ | `js_typedarray.set{Type}(slice, idx, val)` | 同上 |
| `.subarray(start, end)` | ✅ | `js_typedarray.subarray{Type}(slice, start, end)` | 同上 |
| `.copyWithin(target, start, end)` | ✅ | `js_typedarray.copyWithin{Type}(slice, target, start, end)` | 同上 |
| `.fill(val, start, end)` | ✅ | `js_typedarray.fill{Type}(slice, val, start, end)` | 同上 |
| `.buffer` | ✅ | `js_typedarray.buffer{Type}(slice)` | 同上 |
| `.byteLength` | ✅ | `js_typedarray.byteLength{Type}(slice)` | 同上 |
| `.byteOffset` | ✅ | `js_typedarray.byteOffset()` (fixed 0) | 同上 |
| `.slice()` | ❌ | `@compileError` (使用 `.subarray()` 替代) | - |

### 4.14 `Symbol` - ❌ 不支持

| 方法/属性 | 状态 | 说明 |
|----------|------|------|
| `Symbol(description)` | ❌ | ES6 符号，未实现 |
| `Symbol.for(key)` / `Symbol.keyFor(sym)` | ❌ | 全局符号注册表，未实现 |
| `Symbol.iterator` / `Symbol.asyncIterator` | ❌ | 迭代器协议，未实现 |

### 4.15 `WeakMap` - ❌ 不支持

| 方法 | 状态 | 说明 |
|------|------|------|
| `.set(key, val)` | ❌ | 弱引用 Map，未实现 |
| `.get(key)` | ❌ | 未实现 |
| `.has(key)` | ❌ | 未实现 |
| `.delete(key)` | ❌ | 未实现 |

### 4.16 `WeakSet` - ❌ 不支持

| 方法 | 状态 | 说明 |
|------|------|------|
| `.add(val)` | ❌ | 弱引用 Set，未实现 |
| `.has(val)` | ❌ | 未实现 |
| `.delete(val)` | ❌ | 未实现 |

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
