# js2rustc 语法实现说明

> JS → Zig 转译器，Rust 实现。基于 oxc_ast 解析 JavaScript，经类型推断后生成 Zig 0.16 源码。

---

## 1. 架构总览

```
JS Source Files (.js)
        │
        ▼
  ┌─────────────────┐
  │   analyzer.rs    │   分组、import/export 提取
  └────────┬────────┘
           ▼
  ┌─────────────────┐
  │   parser.rs      │   oxc_allocator + oxc_parser → AST
  └────────┬────────┘
           ▼
  ┌─────────────────┐
  │   infer.rs       │   类型推断（3 层规则）
  └────────┬────────┘
           ▼
  ┌─────────────────┐
  │   codegen/*      │   AST → Zig 源码
  │  ├ mod.rs        │   结构体定义、辅助函数、generate() 入口
  │  ├ expr.rs       │   表达式代码生成
  │  ├ stmt.rs       │   语句代码生成
  │  ├ fn_decl.rs    │   函数/类/方法声明
  │  ├ builtins.rs   │   内置 API 调用分发
  │  └ closure.rs    │   闭包扫描与结构体生成
  └────────┬────────┘
           ▼
  ┌─────────────────┐
  │   testgen.rs     │   test_* 变量 → Zig test 代码
  └────────┬────────┘
           ▼
  ┌─────────────────┐
  │   project.rs     │   Zig 项目脚手架（build.zig、lib.zig、多文件模块）
  └────────┬────────┘
           ▼
    out/<group>/
    ├── build.zig
    ├── src/
    │   ├── lib.zig          (编排器，re-export + C ABI wrappers)
    │   ├── <module>.zig     (每个 JS 文件一个 Zig 模块)
    │   └── main.zig         (测试入口)
    └── cabi_exports.json    (C ABI 元数据，供 Rust sys crate 使用)
```

**Pipeline**: `cargo run` → 读取 `in/*.js` → 分组 → 逐文件 parse → infer → codegen → testgen → 输出 `out/<group>/` → `zig build` → `zig build test`

---

## 2. 类型系统 (ZigType)

### 2.1 类型枚举

| ZigType | Zig 输出 | 说明 |
|---------|----------|------|
| `I64` | `i64` | 默认整数类型 |
| `I32` | `i32` | 32 位整数 |
| `Usize` | `usize` | 索引类型 |
| `F64` | `f64` | 默认浮点 |
| `F32` | `f32` | 32 位浮点 |
| `Bool` | `bool` | 布尔 |
| `String` | `[]const u8` | Zig 字符串切片 |
| `Null` | `null` | 空值 |
| `Void` | `void` | 无返回值 |
| `Array(Box<ZigType>)` | `[_]T` | 编译期数组 |
| `Slice(Box<ZigType>)` | `[]const T` | 运行时切片（函数参数） |
| `Optional(Box<ZigType>)` | `?T` | 可空类型（JS `T \| null`） |
| `FunctionPtr(ZigFuncSig)` | `fn (params) ret` | 函数指针 |
| `Struct(String)` | `StructName` | 命名结构体（class） |
| `Object { fields }` | 匿名结构体 / `JsAny` | 对象字面量 |
| `Union(Vec<ZigType>)` | `JsValue` | 联合类型 fallback |
| `JsValue` | `JsValue` | 动态值 union enum（int/float/string/bool/null） |
| `JsAny` | `JsAny` | 最通用容器（ArrayList 元素、HashMap 值） |

### 2.2 类型拓宽 (widen)

```
JsAny > JsValue > Union > F64 > F32 > I64 > I32 / Usize
```

- `T | null` → `?T`（Optional 合并）
- 所有 numeric → 取最宽
- 混合 → `JsValue`；涉及 `JsAny` → `JsAny`

---

## 3. 类型推断 (infer.rs)

### 3.1 三层规则体系

**Layer 1 — 精确推断（编译期类型）**

| 规则 | 条件 | 推断结果 |
|------|------|----------|
| 1.1 | 数字字面量（整数） | `I64` |
| 1.2 | 数字字面量（浮点） | `F64` |
| 1.3 | 字符串字面量 | `String` |
| 1.4 | 布尔字面量 | `Bool` |
| 1.5 | null 字面量 | `Null` |
| 1.6 | 数组字面量 `[...]` | `Array(elem_type)` |
| 1.7 | 对象字面量 `{...}` | `Object { fields }` |
| 1.8 | 模板字面量（无插值） | `String` |
| 1.9 | 二元表达式（算术） | 操作数 widen |
| 1.10 | 比较/逻辑 | `Bool` |
| 1.11 | `typeof` | `String` |

**Layer 2 — 变量推断**

| 规则 | 条件 | 推断结果 |
|------|------|----------|
| 2.1 | `const` + 常量表达式 | Layer 1 精确类型 |
| 2.2 | `const` + 函数调用 | 追踪函数返回类型 |
| 2.3 | `const` + `new ClassName()` | `Struct(ClassName)` |
| 2.4 | `const` + 非常量/非 new | `JsAny` |
| 2.5 | `var`/`let` + 值类型 | `JsValue`（需运行时修改） |
| 2.6 | `var`/`let` + 非值类型 | Layer 1 精确类型 |

**Layer 3 — 函数参数推断**

采用 **约束收集 + 求解** 模式：

1. 收集约束（`ParamConstraint`）：
   - `BinaryWith(other_type, op)` — 二元运算
   - `CallArg(callee, idx)` — 作为函数参数
   - `UnaryOp(op)` — 一元运算
   - `Update` — `++/--`
   - `Condition` — if/while 条件
   - `IteratedInto` — for-of 目标
2. 求解规则：
   - 纯算术约束 → `I64`
   - 字符串拼接 → `String`
   - 布尔条件 → `Bool`
   - 数组迭代 → `Slice(elem_type)`
   - 混合/无约束 → `JsValue`
3. 默认值类型：参数有默认值时，从默认值推断

### 3.2 函数返回类型推断

- 扫描函数体所有 `return expr` → `infer_expr` → `widen` 合并
- 无 return → `Void`
- 多分支返回不同类型 → `widen` 或 `Union`
- `new ClassName()` 返回 → `Struct(ClassName)`

### 3.3 动态特性检测

| 特性 | 检测方式 | 结果 |
|------|----------|------|
| 动态属性访问 | `obj[variable]`（非字面量 key） | `dynamic_access_vars` → HashMap |
| 动态数组 | `arr.push()/pop()/splice()` 调用 | `dynamic_arrays` → ArrayList |

---

## 4. 表达式代码生成 (expr.rs)

### 4.1 已实现表达式

| JS 表达式 | Zig 输出 | 备注 |
|-----------|----------|------|
| 数字字面量 | `42`, `3.14` | 保留 raw 源文本（含十六进制） |
| 字符串字面量 | `"hello"` | |
| 布尔字面量 | `true` / `false` | |
| null 字面量 | `null` | |
| BigInt 字面量 | raw 值 | |
| 标识符 | `name` / `@"keyword"` | Zig 关键字自动转义 |
| `this` | `self` | |
| `NaN` | `std.math.nan(f64)` | |
| `Infinity` | `std.math.inf(f64)` | |
| 算术 `+` | `a + b` 或 `.add()` | 根据类型选择（静态 vs JsValue） |
| 字符串拼接 `+` | `"a" ++ "b"` 或 `allocPrint` | 编译期 vs 运行时 |
| 比较 `<` `>` `<=` `>=` | `a < b` 或 `.lt()/.gt()` | 同上 |
| 相等 `===` `!==` | `==` / `!=` 或 `.eq()/.neq()` | |
| 位运算 `& \| ^ ~ << >>` | 对应 Zig 运算符 | `<<` 右操作数转 `u6` |
| 逻辑 `&& \|\| ??` | `and` / `or` / `orelse` | |
| 一元 `-` `!` `~` | `-` / `!` / `~@as(i64, x)` | `~` 需显式类型转换 |
| 一元 `typeof` | `@TypeOf(x)` | |
| 一元 `void` / `delete` | 忽略，仅 emit 子表达式 | |
| 一元 `+` | 忽略（Zig 无一元加） | |
| `++` / `--` | `+= 1` / `-= 1` 或 `.add()/.sub()` | |
| 赋值 `=` `+=` `-=` 等 | 对应 Zig 运算符或方法调用 | JsValue/JsAny 展开 |
| 三元 `? :` | `if (cond) a else b` | |
| 数组字面量 `[...]` | `[_]T{ ... }` | 元素类型推断 |
| 对象字面量 `{...}` | `.{ .k = v }` | 匿名结构体 |
| 对象展开 `{ ...base, k: v }` | `blk: { var _tmp = base; _tmp.k = v; break :blk _tmp; }` | |
| 模板字面量 `` `...` `` | `"str"` 或 `allocPrint(...)` | 无插值 vs 有插值 |
| 属性访问 `obj.prop` | `obj.prop` | `.length` → `.len` |
| 计算属性 `obj[key]` | `obj[key]` 或 `.get(key).?` | HashMap vs 数组 |
| 函数调用 `fn(args)` | `fn(args)` | 内置函数走 BuiltinRegistry |
| `new Cls(args)` | `Cls.init(args)` | Map/Set/Error 特殊处理 |
| `new Map()` | `js_map.JsMap.init(alloc)` | |
| `new Set()` | `js_set.JsSet.init(alloc)` | |
| `new Error(msg)` | `js_error.JsError.init(alloc, msg)` | |
| 箭头函数 `() => {}` | 闭包结构体字面量 | 详见 §8 |
| `await expr` | `io.async(fn, .{io, args})` + `.await(io)` | Zig 0.16 Io 模式 |
| `(expr)` | `(expr)` | |
| 序列 `a, b` | `a, b` | |
| 链式 `a?.b` | `a.b` | 可选链简化为直接访问 |
| `super` | `self.base` | extends 继承 |
| 正则 `/pattern/` | `"pattern"` | 提取 pattern 为字符串 |
| `instanceof` | `@TypeOf(x) == Y` | |
| `"key" in obj` | `@hasField(...)` 或 `.contains(key)` | 静态 vs HashMap |
| 幂运算 `**` | `std.math.pow(f64, ...)` | |
| TS as/类型断言 | `@as(T, expr)` | |
| TS 非空断言 `x!` | `x.?` | |
| TS satisfies / instantiation | 透传子表达式 | |

### 4.2 未实现表达式（编译错误提示）

| JS 表达式 | 错误信息 |
|-----------|----------|
| 类表达式 `const X = class {}` | `class expression not yet implemented` |
| `yield` | `generators not yet implemented` |
| 动态 `import()` | `use static import instead` |
| 私有字段 `#field` | `private field access not supported` |
| `new.target` | `meta property not supported` |
| JSX | `use createElement() calls instead` |
| 标签模板 `` tag`...` `` | `tagged template not supported` |

---

## 5. 语句代码生成 (stmt.rs)

### 5.1 已实现语句

| JS 语句 | Zig 输出 | 备注 |
|---------|----------|------|
| `var`/`let`/`const` 声明 | `var x: T = val;` / `const x = val;` | `var` 加类型注解 |
| 解构赋值 `const {a, b} = obj` | `const _tmp = obj; const a = _tmp.a;` | 展平为逐字段访问 |
| 函数声明 | `pub fn name(params) ret { ... }` | 详见 §6 |
| 类声明 | `const Name = struct { ... };` | 详见 §6 |
| 表达式语句 | `expr;` | |
| `return expr` | `return expr;` | try/catch 内 → `break :label` |
| `if...else if...else` | `if (cond) { } else if { } else { }` | Optional 条件自动 `!= null` |
| `for (init; test; update)` | `{ init; while (test) : (update) { } }` | |
| `for...in` | HashMap iterator | 仅支持 dynamic access 对象 |
| `for...of` | `for (iterable) \|item\| { }` | 支持解构 |
| `while` | `while (cond) { }` | |
| `do...while` | `while (true) { ... if (!cond) break; }` | |
| `switch` | `_ = switch (val) { ... }` | `break` 自动跳过 |
| `try...catch...finally` | `defer { finally } _ = _try: { } catch { }` | 详见下方 |
| `throw` | `return error.Unexpected` 或 `break :_try error.Unexpected` | |
| `break` / `continue` | `break` / `continue` | 支持标签 |
| 标签语句 `label:` | `label: while/{ }` | 循环标签直接附加 |
| 块语句 `{ }` | `{ }` | |
| 空语句 `;` | （忽略） | |

### 5.2 try-catch-finally 映射策略

```javascript
// JS
try { body } catch (e) { handler } finally { cleanup }
```

```zig
// Zig 输出
defer { cleanup }                          // finally → defer
_ = _try0: {                              // try block
    body                                   // throw → break :_try0 error.Unexpected
                                           // return → break :_try0 value
} catch _catch0: {                         // catch block
    handler                                // return → break :_catch0 value
};
```

### 5.3 变量声明 — 特殊路径

| 场景 | 处理 |
|------|------|
| 箭头函数赋值 `const f = () => {}` | 走 `emit_arrow_fn`（命名函数） |
| 函数表达式赋值 `const f = function() {}` | 走 `emit_fn_from_func_expr` |
| 对象字面量（顶层） | 生成命名结构体 `const StructName = struct { ... };` |
| 动态对象 | `var obj: std.StringHashMap(JsAny) = undefined;` + init 代码 |
| 动态数组 | `var arr = std.ArrayList(JsAny).empty;` + appendSlice 初始元素 |
| `test_*` 变量 | 跳过（测试元数据，由 testgen 处理） |

---

## 6. 函数与类声明 (fn_decl.rs)

### 6.1 函数声明

```javascript
// JS
function add(a, b) { return a + b; }
export function greet(name) { return `Hello ${name}`; }
```

```zig
// Zig
pub fn add(a: i64, b: i64) i64 { return a + b; }
// export → C ABI wrapper：
pub fn greet_impl(name: []const u8) []const u8 { ... }
pub fn greet(name: [*:0]const u8) callconv(.c) [*:0]const u8 { ... }
```

**参数类型推断**：约束收集 → 求解（见 §3.3）

**返回类型推断**：扫描所有 `return` 表达式 → widen

**C ABI wrapper 生成条件**（export 函数）：
- 参数含 `String` 类型 → `[*:0]const u8` 转换
- 返回 `String` → 指针返回 + `free_xxx` 函数
- 返回闭包 → `*anyopaque` + `free_xxx`
- 返回 `JsValue/JsAny` → 提取 `.int` 为 `i64`

### 6.2 async 函数

```javascript
async function fetchData(url) { const data = await fetch(url); return data; }
```

```zig
pub fn fetchData(io: Io, url: []const u8) ![]const u8 {
    var _t0 = io.async(fetch, .{ io, url });
    defer _t0.cancel(io) catch {};
    const data = try _t0.await(io);
    return data;
}
```

- 自动添加 `io: Io` 参数
- 返回类型前加 `!`（error union）
- `await` 展开为 `io.async` + `.await(io)` 模式

### 6.3 参数处理

| 场景 | 处理 |
|------|------|
| 简单参数 `a` | `a: InferredType` |
| 默认值 `a = 10` | `a: i64 = 10` |
| 解构 `{a, b}` | `_arg0: JsValue` + body prelude 展开 |
| rest `...args` | `args: []const i64` |
| Object 类型参数 | 生成命名结构体 `FnNameArgName = struct { ... }` |

### 6.4 类声明

```javascript
class Point {
  constructor(x, y) { this.x = x; this.y = y; }
  distance() { return Math.sqrt(this.x ** 2 + this.y ** 2); }
  static origin() { return new Point(0, 0); }
}
```

```zig
const Point = struct {
    x: i64,
    y: i64,

    pub fn init(x: i64, y: i64) Point {
        var self: Point = undefined;
        self.x = x;
        self.y = y;
        return self;
    }

    pub fn distance(self: *const Point) i64 {
        return @sqrt(self.x ** 2 + self.y ** 2);
    }

    pub fn origin() Point {
        return Point.init(0, 0);
    }
};
```

**类特性支持**：

| 特性 | 支持 | 说明 |
|------|------|------|
| 构造函数 | ✅ | → `init()` 方法，返回 `Self` |
| 实例方法 | ✅ | 首参 `self: *const StructName` |
| 静态方法 | ✅ | 无 `self` 参数 |
| 属性定义 | ✅ | 含默认值 |
| 静态属性 | ✅ | → `pub const` |
| getter/setter | ✅ | → `get_xxx` / `set_xxx` |
| extends | ✅ | → 内嵌 `base: ParentType` 字段 |
| `this.field = val` | ✅ | → `self.field = val`（构造函数中 `var self = undefined;`） |
| `super` | ✅ | → `self.base` |
| 私有字段 `#field` | ❌ | 编译错误提示 |

---

## 7. 内置 API 映射 (builtins.rs)

### 7.1 Math

| JS | Zig |
|----|-----|
| `Math.PI` | `std.math.pi` |
| `Math.E` | `std.math.e` |
| `Math.abs(x)` | `@abs(x)` |
| `Math.ceil/floor/trunc/round(x)` | `@ceil/@floor/@trunc/@round(x)` |
| `Math.sqrt(x)` | `@sqrt(x)` |
| `Math.sin/cos/tan/asin/acos/atan(x)` | `@sin/@cos/@tan/@asin/@acos/@atan(x)` |
| `Math.atan2(y, x)` | `@atan2(y, x)` |
| `Math.exp/log/log2/log10(x)` | `@exp/@log/@log2/@log10(x)` |
| `Math.min/max(a, b)` | `@min/@max(a, b)` |
| `Math.pow(b, e)` | `std.math.pow(f64, b, e)` |
| `Math.random()` | `std.crypto.random.float(f64)` |
| `Math.sign(x)` | 三路 if 表达式 |
| `Math.hypot(a, b)` | `@sqrt(a*a + b*b)` |

### 7.2 全局函数

| JS | Zig |
|----|-----|
| `parseInt(s)` | `std.fmt.parseInt(i64, s, 10) catch 0` |
| `parseFloat(s)` | `std.fmt.parseFloat(f64, s) catch 0.0` |
| `isNaN(x)` | `std.math.isNan(@as(f64, x))` |
| `isFinite(x)` | `!std.math.isInf(x)` |
| `encodeURIComponent(s)` | `js_uri.encodeURIComponent(alloc, s)` |
| `decodeURIComponent(s)` | `js_uri.decodeURIComponent(alloc, s)` |

### 7.3 String 方法

| JS | Zig 运行时 |
|----|-----------|
| `.length` | `.len` |
| `.toUpperCase()` | `js_string.toUpper(s)` |
| `.toLowerCase()` | `js_string.toLower(s)` |
| `.charAt(i)` | `js_string.charAt(s, i)` |
| `.charCodeAt(i)` | `s[@intCast(i)]` |
| `.concat(other)` | `js_string.concat(s, other)` |
| `.includes(sub)` | `js_string.includes(s, sub)` |
| `.indexOf(sub)` | `js_string.indexOf(s, sub)` |
| `.startsWith(pre)` | `js_string.startsWith(s, pre)` |
| `.endsWith(suf)` | `js_string.endsWith(s, suf)` |
| `.slice(start, end)` | `js_string.slice(s, start, end)` |
| `.split(sep)` | `js_string.split(s, sep)` |
| `.replace(old, new)` | `js_string.replace(s, old, new)` |
| `.trim()` | `js_string.trim(s)` |
| `.repeat(n)` | `js_string.repeat(s, n)` |

### 7.4 Array 方法

**静态数组（编译期）** → `js_array.*` 运行时函数

| JS | Zig |
|----|-----|
| `.length` | `.len` |
| `.push(val)` | `js_array.push(arr, val)` |
| `.pop()` | `js_array.pop(arr)` |
| `.shift()` | `js_array.shift(arr)` |
| `.unshift(val)` | `js_array.unshift(arr, val)` |
| `.indexOf(val)` | `js_array.indexOf(arr, val)` |
| `.includes(val)` | `js_array.includes(arr, val)` |
| `.join(sep)` | `js_array.join(arr, sep)` |
| `.reverse()` | `js_array.reverse(arr)` |
| `.sort()` | `js_array.sort(arr)` |
| `.slice(s, e)` | `js_array.slice(arr, s, e)` |
| `.concat(other)` | `js_array.concat(arr, other)` |
| `.map(fn)` | `js_array.map(arr, fn)` |
| `.filter(fn)` | `js_array.filter(arr, fn)` |
| `Array.isArray(x)` | `js_array.isArray(x)` |

**动态数组（ArrayList）** → 直接 ArrayList 方法

| JS | Zig |
|----|-----|
| `.push(val)` | `arr.append(alloc, JsAny.fromXxx(val))` |
| `.pop()` | `arr.pop() orelse JsAny.fromNull()` |
| `.shift()` | `arr.orderedRemove(0)` |
| `.unshift(val)` | `arr.insert(alloc, 0, JsAny.fromXxx(val))` |
| `.reverse()` | `std.mem.reverse(JsAny, arr.items)` |
| `.sort()` | `std.mem.sort(JsAny, arr.items, ...)` |
| `.splice(s, n)` | orderedRemove 循环 |

### 7.5 其他内置对象

| 对象 | 已实现方法 |
|------|-----------|
| `console` | `.log()`, `.error()`, `.warn()` |
| `JSON` | `.stringify()`, `.parse()` |
| `Object` | `.keys()`, `.values()`, `.assign()`, `.entries()` |
| `Number` | `.isNaN()`, `.isFinite()`, `.isInteger()`, `.parseInt()`, `.parseFloat()` |
| `Date` | `.now()`, `.getTime()`, `.getFullYear()`, `.getMonth()`, `.getDate()`, `.getDay()`, `.getHours()`, `.getMinutes()`, `.getSeconds()` |
| `Map` | `.get()`, `.set()`, `.has()`, `.delete()`, `.clear()`, `.size` |
| `Set` | `.add()`, `.has()`, `.delete()`, `.clear()` |
| `RegExp` | `.test()`, `.exec()` |
| `Boolean` | `.toString()` |

---

## 8. 闭包 (closure.rs)

### 8.1 闭包实现策略

JS 箭头函数 / 函数表达式如果捕获了外层变量，转换为 **闭包结构体**：

```javascript
function makeAdder(x) {
    return (y) => x + y;   // 捕获 x
}
```

```zig
const _Closure_makeAdder = struct {
    x: i64,

    pub fn call(self: @This(), y: i64) i64 {
        return self.x + y;
    }
};

pub fn makeAdder(x: i64) _Closure_makeAdder {
    return _Closure_makeAdder{ .x = x };
}
```

### 8.2 闭包检测流程

1. **Pre-scan**：`pre_scan_closures()` 遍历所有函数体
2. **记录**：`record_closure()` 收集捕获变量、参数、返回类型
3. **生成**：`generate_closure_struct_def()` 立即生成结构体定义
4. **使用**：
   - 返回闭包 → 函数返回类型为结构体名
   - 变量赋值 → `const __cl_name = StructName{ .cap = cap };`
   - 调用 → `__cl_name.call(args)`

### 8.3 捕获变量识别

- 收集箭头函数体内所有标识符
- 减去箭头函数自身参数
- 减去箭头函数体内局部声明
- 剩余 = 捕获变量（自动推断类型）

---

## 9. Zig 运行时

### 9.1 JsValue (jsvalue.zig)

```zig
pub const JsValue = union(enum) {
    int: i64,
    float: f64,
    string: []const u8,
    boolean: bool,
    null: void,
};
```

方法：`fromI64`, `fromF64`, `fromBool`, `fromString`, `fromNull`, `add`, `sub`, `mul`, `div`, `rem`, `lt`, `le`, `gt`, `ge`, `eq`, `neq`

### 9.2 JsAny (jsany.zig)

```zig
pub const JsAny = union(enum) {
    int: i64,
    float: f64,
    string: []const u8,
    boolean: bool,
    null: void,
    object: *std.StringHashMap(JsAny),
    array: *std.ArrayList(JsAny),
    function: *const anyopaque,
};
```

方法：同 JsValue + `asI64`, `asF64`, `asBool`, `asString` 提取器

### 9.3 运行时模块

| 文件 | 提供功能 |
|------|----------|
| `js_string.zig` | 字符串方法 |
| `js_array.zig` | 数组方法 |
| `js_object.zig` | Object.keys/values/entries/assign |
| `js_runtime.zig` | 运行时注册表 |
| `js_allocator.zig` | 全局 allocator 管理 |

---

## 10. 测试生成 (testgen.rs)

### 10.1 约定

JS 文件中以 `test_` 开头的变量被视为测试用例：

```javascript
const test_add = add(3, 5);       // => 8
const test_greet = greet("Zig");  // => "Hello Zig"
const test_smoke = factorial(10); // （无期望值 → 冒烟测试）
```

### 10.2 期望值解析

`// => value` 注释被解析为期望值：

| 格式 | Zig 断言 |
|------|----------|
| `// => 8` | `expectEqual(@as(i64, 8), ...)` |
| `// => 3.14` | `expectEqual(@as(f64, 3.14), ...)` |
| `// => "hello"` | `expectEqualSlices(u8, "hello", ...)` |
| `// => true` | `expectEqual(true, ...)` |
| （无注释） | `_ = expr;`（冒烟测试） |

### 10.3 JsValue/JsAny 提取

当函数返回 `JsValue` 或 `JsAny` 时，testgen 自动追加字段访问：

- 数值：`.int` (JsValue) / `.value.int` (JsAny)
- 字符串：`.string` (JsValue) / `.value.string` (JsAny)

---

## 11. C ABI 桥接

### 11.1 导出流

```
JS export function → codegen (fn_impl + C ABI wrapper) → lib.zig re-export → .lib/.dll/.so
```

### 11.2 类型映射

| Zig 类型 | C ABI 类型 |
|----------|-----------|
| `i64` | `i64` |
| `f64` | `f64`  |
| `bool` | `bool` |
| `[]const u8` | `[*:0]const u8`（C 字符串） |
| `JsValue` / `JsAny` | `i64`（提取 `.int`） |
| 闭包结构体 | `*anyopaque` |
| `void` | `void` |

### 11.3 导入流（Host Functions）

`host_config.json` 定义 Rust 提供的函数 → 注册到 `BuiltinRegistry` → JS 中直接调用 → Zig 通过 `extern "c"` 调用。

---

## 12. 已知限制与未实现

### 12.1 语句级

| 特性 | 状态 | 说明 |
|------|------|------|
| 嵌套函数声明 | ❌ | 报错，需重构为顶层 |
| `with` 语句 | ❌ | JS 严格模式已废弃 |
| `debugger` | ❌ | |
| `for-in` 静态对象 | ❌ | 仅支持 HashMap 对象 |
| `for await...of` | ❌ | 跳过 |

### 12.2 表达式级

| 特性 | 状态 | 说明 |
|------|------|------|
| Generator / `yield` | ❌ | |
| 动态 `import()` | ❌ | |
| 私有字段 `#field` | ❌ | |
| 类表达式 | ❌ | |
| 标签模板 | ❌ | |
| 可选链 `?.` | ⚠️ | 简化为直接访问（无 null 检查） |
| 解构赋值默认值 | ⚠️ | 跳过默认值 |
| 多 spread 合并 | ❌ | `{ ...a, ...b }` 不支持 |
| `splice` 带插入 | ❌ | 仅支持删除操作 |

### 12.3 类型系统

| 限制 | 说明 |
|------|------|
| class 字段仅 `i64` | 所有字段类型硬编码为 `i64` |
| 泛型缺失 | 无 TypeScript 泛型支持 |
| interface / type alias | 不支持 |
| 联合类型 fallback | 复杂联合 → `JsValue` |
