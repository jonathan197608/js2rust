# js2rustc 全面测试方案

> 基于 SYNTAX_IMPLEMENTATION.md 文档，覆盖所有已声明的功能点。

---

## 1. 测试架构

### 1.1 三层测试体系

| 层级 | 工具 | 范围 | 文件 |
|------|------|------|------|
| L1: Rust 单元测试 | `cargo test` | parser / infer / testgen 内部逻辑 | `js2rustc/src/*.rs` `#[cfg(test)]` |
| L2: Rust 集成测试 | `cargo test` | 端到端 pipeline（JS → Zig 源码 → zig build） | `js2rustc/tests/pipeline_tests.rs` |
| L3: Zig 端到端测试 | `cargo run` + `zig build test` | JS 输入 → 转译 → 编译 → 运行时断言 | `in/*.js` → `out/*/` → zig test |

### 1.2 测试命名约定

- JS 文件中 `test_` 前缀变量 → 自动生成 Zig `test "xxx" {}` 块
- `// => value` 注释 → 自动生成断言（`expectEqual` / `expectEqualSlices`）
- 无 `// =>` → 冒烟测试（仅验证编译 + 运行不崩溃）

### 1.3 当前覆盖率

| 类别 | 已测试 | 文档声明 | 覆盖率 |
|------|--------|----------|--------|
| 表达式 | 10/35+ | 35+ 种 | ~29% |
| 语句 | 10/15 | 15 种 | ~67% |
| 内置 API | 3/60+ | 60+ 种 | ~5% |
| 类型推断规则 | 3/14 | 14 条 | ~21% |
| 类特性 | 2/10 | 10 种 | ~20% |
| 闭包 | 0/4 | 4 种 | 0% |
| C ABI 桥接 | 1/6 | 6 种 | ~17% |

---

## 2. 测试用例清单

### 2.1 类型推断 (infer.rs) — 14 个测试

#### T-INF-01: Layer 1 精确推断 — 字面量类型

```javascript
// in/test_types.js
function intLiteral() { return 42; }
function floatLiteral() { return 3.14; }
function stringLiteral() { return "hello"; }
function boolLiteral() { return true; }
function nullLiteral() { return null; }

const test_int = intLiteral(); // => 42
const test_float = floatLiteral(); // => 3.14
const test_string = stringLiteral(); // => "hello"
const test_bool = boolLiteral(); // => true
```

**验证**: 各函数返回类型分别推断为 i64, f64, []const u8, bool, null

#### T-INF-02: Layer 1 — 数组字面量推断

```javascript
function getArray() {
    const arr = [1, 2, 3];
    return arr[0];
}
const test_array_elem = getArray(); // => 1
```

**验证**: 数组元素类型推断为 i64，`arr[0]` 返回 i64

#### T-INF-03: Layer 1 — 二元表达式类型推断

```javascript
function addInts(a, b) { return a + b; }       // i64 + i64 → i64
function addFloats(a, b) { return a + b + 0.5; } // widen → f64
function compare(a, b) { return a > b; }        // → bool

const test_add_ints = addInts(3, 5); // => 8
const test_compare = compare(5, 3); // => true
```

**验证**: 算术 widen 规则正确，比较返回 bool

#### T-INF-04: Layer 2 — const + 函数调用追踪 (Rule 2.2)

```javascript
function getNumber() { return 42; }
function wrapper() {
    const result = getNumber();
    return result + 1;
}
const test_wrapper = wrapper(); // => 43
```

**验证**: `result` 类型通过 `getNumber()` 追踪到 i64

#### T-INF-05: Layer 2 — const + new ClassName (Rule 2.3)

```javascript
class Counter {
    constructor(n) { this.count = n; }
    get() { return this.count; }
}
function makeCounter() {
    const c = new Counter(10);
    return c.get();
}
const test_counter = makeCounter(); // => 10
```

**验证**: `c` 类型保持为 `Struct("Counter")`，方法调用链正确

#### T-INF-06: Layer 2 — var/let 可变变量 (Rule 2.5/2.6)

```javascript
function mutableVar() {
    let sum = 0;
    sum = sum + 10;
    return sum;
}
const test_mutable = mutableVar(); // => 10
```

**验证**: `let sum = 0` 推断为 JsValue（Rule 2.5），操作通过 JsValue 方法

#### T-INF-07: Layer 3 — 参数约束求解

```javascript
function arithmetic(a, b) { return (a * 2) + (b - 1); }
function stringParam(s) { return s + " world"; }

const test_arith = arithmetic(5, 3); // => 12
const test_str_param = stringParam("hello"); // => "hello world"
```

**验证**: `a,b` 通过算术约束推断为 i64；`s` 通过字符串拼接推断为 []const u8

#### T-INF-08: 函数返回类型推断 — 多分支

```javascript
function multiReturn(x) {
    if (x > 0) { return 1; }
    if (x < 0) { return -1; }
    return 0;
}
const test_multi_ret = multiReturn(5); // => 1
```

**验证**: 所有分支返回 i64，widen 后仍为 i64

#### T-INF-09: 函数返回类型推断 — void

```javascript
function noReturn(x) {
    const y = x + 1;
}
const test_void_fn = noReturn(5);
```

**验证**: 无 return 语句 → 返回类型为 void，冒烟测试通过

#### T-INF-10: 类型拓宽 — Optional

```javascript
function maybeNull(x) {
    if (x > 0) { return x; }
    return null;
}
const test_maybe = maybeNull(5);
```

**验证**: 返回 i64 和 null → widen 为 ?i64 或 JsValue

#### T-INF-11: 动态属性检测 → HashMap

```javascript
function dynamicObj() {
    const obj = {};
    obj["key1"] = 10;
    obj["key2"] = 20;
    return obj["key1"];
}
const test_dyn_obj = dynamicObj();
```

**验证**: 动态 `obj[variable]` 检测 → StringHashMap 生成

#### T-INF-12: 动态数组检测 → ArrayList

```javascript
function dynamicArray() {
    const arr = [];
    arr.push(1);
    arr.push(2);
    arr.push(3);
    return arr;
}
const test_dyn_arr = dynamicArray();
```

**验证**: `arr.push()` 检测 → ArrayList 生成

#### T-INF-13: 默认值类型推断

```javascript
function withDefault(x, y = 10) {
    return x + y;
}
const test_default = withDefault(5); // => 15
```

**验证**: 参数 `y` 从默认值 `10` 推断为 i64

#### T-INF-14: typeof 表达式推断

```javascript
function checkType(x) {
    const t = typeof x;
    return t;
}
const test_typeof = checkType(42);
```

**验证**: `typeof` 返回 `String` 类型

---

### 2.2 表达式代码生成 (expr.rs) — 28 个测试

#### T-EXPR-01: 数字字面量（整数 + 浮点 + 十六进制）

```javascript
function hexNum() { return 0xFF; }
function negNum() { return -42; }

const test_hex = hexNum(); // => 255
const test_neg = negNum(); // => -42
```

#### T-EXPR-02: 模板字面量 — 无插值

```javascript
function templateSimple() { return `hello world`; }
const test_tpl_simple = templateSimple(); // => "hello world"
```

#### T-EXPR-03: 模板字面量 — 有插值

```javascript
function templateInterp(name) {
    return `Hello ${name}!`;
}
const test_tpl_interp = templateInterp("Zig"); // => "Hello Zig!"
```

**验证**: 生成 `std.fmt.allocPrint` 调用

#### T-EXPR-04: 三元运算符

```javascript
function ternary(x) { return x > 0 ? 1 : -1; }
const test_ternary_pos = ternary(5); // => 1
const test_ternary_neg = ternary(-3); // => -1
```

#### T-EXPR-05: 逻辑运算符 (&&, ||, ??)

```javascript
function logicAnd(a, b) { return a > 0 && b > 0; }
function logicOr(a, b) { return a > 0 || b > 0; }

const test_and_tt = logicAnd(1, 2); // => true
const test_and_tf = logicAnd(1, -1); // => false
const test_or_tf = logicOr(-1, 2); // => true
const test_or_ff = logicOr(-1, -2); // => false
```

**验证**: 生成 `and` / `or` Zig 关键字

#### T-EXPR-06: 位运算全集

```javascript
function bitOps(x) {
    const a = x << 2;
    const b = x >> 1;
    const c = x & 0xFF;
    const d = x | 0x0F;
    const e = x ^ 0xAA;
    const f = ~x;
    return a;
}
const test_bitops = bitOps(4); // => 16
```

**验证**: 所有位运算符正确映射，`<<` 右操作数转 u6

#### T-EXPR-07: 赋值运算符 (+=, -=, *=)

```javascript
function compoundAssign(x) {
    let a = x;
    a += 10;
    a -= 3;
    a *= 2;
    return a;
}
const test_compound = compoundAssign(5); // => 24
```

#### T-EXPR-08: 幂运算 (**)

```javascript
function power(base, exp) {
    return base ** exp;
}
const test_power = power(2, 10);
```

**验证**: 生成 `std.math.pow(f64, ...)`

#### T-EXPR-09: 对象字面量 → 匿名结构体

```javascript
function makeObj() {
    const obj = { x: 1, y: 2 };
    return obj.x + obj.y;
}
const test_obj = makeObj(); // => 3
```

#### T-EXPR-10: 对象展开 (Spread)

```javascript
function spreadObj() {
    const base = { x: 1, y: 2 };
    const extended = { ...base, y: 10 };
    return extended.y;
}
const test_spread = spreadObj(); // => 10
```

**验证**: 生成 `blk: { var _tmp = base; _tmp.y = 10; break :blk _tmp; }`

#### T-EXPR-11: 数组字面量

```javascript
function arrayLiteral() {
    const arr = [10, 20, 30];
    return arr[1];
}
const test_arr_lit = arrayLiteral(); // => 20
```

#### T-EXPR-12: 属性访问 (.length → .len)

```javascript
function getLength(s) {
    return s.length;
}
const test_str_len = getLength("hello"); // => 5
```

#### T-EXPR-13: new Map / Set / Error

```javascript
function useMap() {
    const m = new Map();
    m.set("key", 42);
    return m.get("key");
}
const test_map = useMap();
```

**验证**: 生成 `js_map.JsMap.init(alloc)` + `.put()`/`.get()` 调用

#### T-EXPR-14: instanceof

```javascript
// Smoke test — 编译通过即可
function checkInstance(x) {
    return x instanceof Point;
}
```

**验证**: 生成 `@TypeOf(x) == Point`

#### T-EXPR-15: "key" in obj（静态对象）

```javascript
function hasKey() {
    const obj = { name: "test", age: 25 };
    return "name" in obj;
}
const test_has_key = hasKey();
```

**验证**: 生成 `@hasField(...)` 编译期检查

#### T-EXPR-16: NaN / Infinity

```javascript
function getNaN() { return NaN; }
function getInf() { return Infinity; }
const test_nan = getNaN();
const test_inf = getInf();
```

**验证**: 生成 `std.math.nan(f64)` / `std.math.inf(f64)`

#### T-EXPR-17: Update 表达式 (++/--)

```javascript
function increment(x) {
    let a = x;
    a++;
    return a;
}
function decrement(x) {
    let a = x;
    a--;
    return a;
}
const test_inc = increment(5); // => 6
const test_dec = decrement(5); // => 4
```

**验证**: 生成 `+= 1` / `-= 1` 或 `.add()/.sub()`

#### T-EXPR-18: 链式可选访问 (?.)

```javascript
function optChain(obj) {
    return obj?.name;
}
```

**验证**: 简化为 `obj.name`（当前限制）

#### T-EXPR-19: 正则表达式

```javascript
function getPattern() {
    const re = /hello/;
    return re;
}
const test_regex = getPattern();
```

**验证**: 提取 pattern 为字符串 `"hello"`

#### T-EXPR-20: 序列表达式

```javascript
function seqExpr() {
    return (1, 2, 3);
}
const test_seq = seqExpr();
```

#### T-EXPR-21: 括号表达式

```javascript
function parenExpr(a, b) {
    return (a + b) * 2;
}
const test_paren = parenExpr(3, 4); // => 14
```

#### T-EXPR-22: 字符串拼接 — 编译期 vs 运行时

```javascript
function constConcat() {
    return "hello" + " " + "world";
}
function runtimeConcat(a, b) {
    return a + " " + b;
}
const test_const_concat = constConcat(); // => "hello world"
const test_runtime_concat = runtimeConcat("foo", "bar"); // => "foo bar"
```

#### T-EXPR-23: 相等运算符 (=== / !==)

```javascript
function strictEq(a, b) { return a === b; }
function strictNeq(a, b) { return a !== b; }
const test_eq_true = strictEq(5, 5); // => true
const test_eq_false = strictEq(5, 3); // => false
const test_neq = strictNeq(5, 3); // => true
```

#### T-EXPR-24: 比较运算符全集

```javascript
function lt(a, b) { return a < b; }
function le(a, b) { return a <= b; }
function gt(a, b) { return a > b; }
function ge(a, b) { return a >= b; }

const test_lt = lt(3, 5); // => true
const test_le = le(5, 5); // => true
const test_gt = gt(5, 3); // => true
const test_ge = ge(3, 5); // => false
```

#### T-EXPR-25: 一元运算符 (-, !, ~)

```javascript
function unaryNeg(x) { return -x; }
function unaryNot(x) { return !x; }
function unaryBitNot(x) { return ~x; }

const test_uneg = unaryNeg(5); // => -5
const test_unot = unaryNot(true); // => false
const test_ubnot = unaryBitNot(0); // => -1
```

#### T-EXPR-26: super / extends

```javascript
class Animal {
    constructor(name) { this.name = name; }
    speak() { return 0; }
}
class Dog extends Animal {
    constructor(name) { super(name); }
    speak() { return 1; }
}
function makeDog() {
    const d = new Dog("Rex");
    return d.speak();
}
const test_dog = makeDog(); // => 1
```

**验证**: extends → 内嵌 `base: Animal`，super → `self.base`

#### T-EXPR-27: TS as 类型断言

```javascript
function tsAs(x) {
    return (x as number) + 1;
}
const test_as = tsAs(5);
```

**验证**: 生成 `@as(T, expr)`

#### T-EXPR-28: 计算属性访问 (obj[key])

```javascript
function indexAccess() {
    const arr = [10, 20, 30];
    return arr[1];
}
const test_index = indexAccess(); // => 20
```

---

### 2.3 语句代码生成 (stmt.rs) — 12 个测试

#### T-STMT-01: 解构赋值 — 对象

```javascript
function destructObj() {
    const obj = { x: 10, y: 20 };
    const { x, y } = obj;
    return x + y;
}
const test_destruct = destructObj(); // => 30
```

**验证**: 展平为 `const _tmp = obj; const x = _tmp.x; const y = _tmp.y;`

#### T-STMT-02: for...in (HashMap)

```javascript
function forInObj() {
    const obj = {};
    obj["a"] = 1;
    obj["b"] = 2;
    let sum = 0;
    for (const key in obj) {
        sum = sum + obj[key];
    }
    return sum;
}
const test_for_in = forInObj();
```

**验证**: 生成 HashMap iterator

#### T-STMT-03: for...of

```javascript
function forOfSum(items) {
    let total = 0;
    for (const item of items) {
        total = total + item;
    }
    return total;
}
const test_for_of = forOfSum([1, 2, 3, 4, 5]);
```

#### T-STMT-04: throw 语句

```javascript
function throwError() {
    throw new Error("something went wrong");
}
function catchThrow() {
    try {
        throwError();
        return 0;
    } catch (e) {
        return -1;
    }
}
const test_throw = catchThrow(); // => -1
```

**验证**: throw → `return error.Unexpected` 或 `break :_try error.Unexpected`

#### T-STMT-05: try-catch-finally 完整映射

```javascript
function tryCatchFinally(x) {
    let result = 0;
    try {
        if (x === 0) {
            throw new Error("zero");
        }
        result = 100 / x;
    } catch (e) {
        result = -1;
    } finally {
        result = result + 1;
    }
    return result;
}
const test_tcf_ok = tryCatchFinally(10);
const test_tcf_err = tryCatchFinally(0);
```

**验证**: finally → `defer { }`，try → `_try0: { ... }`

#### T-STMT-06: 标签语句 + 标签 break

```javascript
function labeledBreak() {
    let sum = 0;
    outer: for (let i = 0; i < 5; i++) {
        for (let j = 0; j < 5; j++) {
            if (i + j > 3) {
                break outer;
            }
            sum = sum + 1;
        }
    }
    return sum;
}
const test_labeled = labeledBreak();
```

**验证**: 标签附加到外层循环

#### T-STMT-07: do...while 循环

```javascript
function doWhileCount() {
    let count = 0;
    let i = 0;
    do {
        count = count + 1;
        i = i + 1;
    } while (i < 5);
    return count;
}
const test_dowhile = doWhileCount(); // => 5
```

#### T-STMT-08: switch-case — 多值

```javascript
function switchMulti(x) {
    switch (x) {
        case 1: return 10;
        case 2: return 20;
        case 3: return 30;
        case 4: return 40;
        case 5: return 50;
        default: return 0;
    }
}
const test_sw1 = switchMulti(3); // => 30
const test_sw_def = switchMulti(99); // => 0
```

#### T-STMT-09: 变量声明 — 箭头函数赋值

```javascript
const square = (x) => { return x * x; };
const test_arrow_var = square(5); // => 25
```

**验证**: 走 `emit_arrow_fn` 路径，生成命名函数

#### T-STMT-10: 块语句 + 空语句

```javascript
function blockStmt(x) {
    {
        const a = x + 1;
        return a;
    }
}
const test_block = blockStmt(5); // => 6
```

#### T-STMT-11: 多变量声明

```javascript
function multiDecl() {
    const a = 1, b = 2, c = 3;
    return a + b + c;
}
const test_multi_decl = multiDecl(); // => 6
```

#### T-STMT-12: 表达式语句（副作用调用）

```javascript
function sideEffect(x) {
    console.log(x);
    return x + 1;
}
const test_side_effect = sideEffect(5); // => 6
```

---

### 2.4 函数与类声明 (fn_decl.rs) — 14 个测试

#### T-FN-01: 基础函数声明

```javascript
function simpleAdd(a, b) { return a + b; }
const test_fn_basic = simpleAdd(10, 20); // => 30
```

#### T-FN-02: export 函数 → C ABI wrapper

```javascript
export function exportedFn(x) { return x * 2; }
const test_export = exportedFn(5); // => 10
```

**验证**: 生成 `callconv(.c)` wrapper + `pub fn` 实现

#### T-FN-03: 默认参数

```javascript
function withDefaults(a, b = 10, c = 20) {
    return a + b + c;
}
const test_def_all = withDefaults(1); // => 31
const test_def_some = withDefaults(1, 2); // => 23
```

#### T-FN-04: Rest 参数

```javascript
function sum(...args) {
    let total = 0;
    for (const arg of args) {
        total = total + arg;
    }
    return total;
}
const test_rest = sum(1, 2, 3, 4, 5);
```

**验证**: 生成 `args: []const i64`

#### T-FN-05: 类 — 构造函数 + 实例方法

```javascript
class Rectangle {
    constructor(w, h) {
        this.width = w;
        this.height = h;
    }
    area() {
        return this.width * this.height;
    }
}
function testRect() {
    const r = new Rectangle(3, 4);
    return r.area();
}
const test_rect = testRect(); // => 12
```

#### T-FN-06: 类 — 静态方法

```javascript
class MathUtils {
    static double(x) { return x * 2; }
    static triple(x) { return x * 3; }
}
function testStatic() {
    return MathUtils.double(5) + MathUtils.triple(3);
}
const test_static = testStatic(); // => 19
```

**验证**: 静态方法无 `self` 参数

#### T-FN-07: 类 — 静态属性

```javascript
class Config {
    static MAX_SIZE = 100;
    static MIN_SIZE = 1;
}
function getMax() {
    return Config.MAX_SIZE;
}
const test_static_prop = getMax(); // => 100
```

**验证**: `static` 属性 → `pub const`

#### T-FN-08: 类 — getter / setter

```javascript
class Temperature {
    constructor(celsius) { this.celsius = celsius; }
    get fahrenheit() {
        return this.celsius * 9 / 5 + 32;
    }
}
function testGetter() {
    const t = new Temperature(100);
    return t.fahrenheit;
}
const test_getter = testGetter();
```

**验证**: getter → `get_fahrenheit` 方法

#### T-FN-09: 类 — extends / super

```javascript
class Shape {
    constructor(name) { this.name = name; }
    sides() { return 0; }
}
class Triangle extends Shape {
    constructor() { super("triangle"); }
    sides() { return 3; }
}
function testExtends() {
    const t = new Triangle();
    return t.sides();
}
const test_extends = testExtends(); // => 3
```

**验证**: extends → `base: Shape` 字段，super → 初始化 base

#### T-FN-10: 类 — 含默认值的属性定义

```javascript
class Settings {
    constructor() {
        this.volume = 50;
        this.brightness = 75;
    }
    getVolume() { return this.volume; }
}
function testSettings() {
    const s = new Settings();
    return s.getVolume();
}
const test_settings = testSettings(); // => 50
```

#### T-FN-11: async 函数（冒烟测试）

```javascript
// 仅验证代码生成格式正确，不实际执行 async
// async function fetchData(url) {
//     const data = await fetch(url);
//     return data;
// }
```

**验证**: 生成 `io: Io` 参数、`!` 返回类型、`io.async` + `.await(io)` 模式（注释测试，手动检查生成代码）

#### T-FN-12: 多返回类型 → widen

```javascript
function flexReturn(x) {
    if (x > 0) { return 1; }
    if (x < 0) { return -1.0; }
    return 0;
}
const test_flex = flexReturn(5);
```

**验证**: i64 + f64 → widen 为 f64

#### T-FN-13: 递归函数

```javascript
function fib(n) {
    if (n <= 1) { return n; }
    return fib(n - 1) + fib(n - 2);
}
const test_fib = fib(10); // => 55
```

#### T-FN-14: 函数表达式赋值

```javascript
const cube = function(x) { return x * x * x; };
const test_cube = cube(3); // => 27
```

**验证**: 走 `emit_fn_from_func_expr` 路径

---

### 2.5 内置 API 映射 (builtins.rs) — 33 个测试

#### T-BUILTIN-01: Math 常量

```javascript
function getPi() { return Math.PI; }
function getE() { return Math.E; }
const test_pi = getPi(); // => 3.141592653589793
const test_e = getE(); // => 2.718281828459045
```

#### T-BUILTIN-02: Math 一元函数

```javascript
function testAbs(x) { return Math.abs(x); }
function testCeil(x) { return Math.ceil(x); }
function testFloor(x) { return Math.floor(x); }
function testRound(x) { return Math.round(x); }
function testTrunc(x) { return Math.trunc(x); }
function testSqrt(x) { return Math.sqrt(x); }

const test_abs = testAbs(-7);
const test_ceil = testCeil(3.2);
const test_floor = testFloor(3.8);
const test_round = testRound(3.5);
const test_trunc = testTrunc(3.9);
const test_sqrt = testSqrt(9.0);
```

**验证**: 各自映射到 `@abs/@ceil/@floor/@round/@trunc/@sqrt`

#### T-BUILTIN-03: Math 三角函数

```javascript
function testSin(x) { return Math.sin(x); }
function testCos(x) { return Math.cos(x); }
function testTan(x) { return Math.tan(x); }
function testAsin(x) { return Math.asin(x); }
function testAcos(x) { return Math.acos(x); }
function testAtan(x) { return Math.atan(x); }
function testAtan2(y, x) { return Math.atan2(y, x); }

const test_sin = testSin(0.0);
const test_cos = testCos(0.0);
```

**验证**: 映射到 `@sin/@cos/@tan/@asin/@acos/@atan/@atan2`

#### T-BUILTIN-04: Math 高级函数

```javascript
function testExp(x) { return Math.exp(x); }
function testLog(x) { return Math.log(x); }
function testLog2(x) { return Math.log2(x); }
function testLog10(x) { return Math.log10(x); }
function testPow(b, e) { return Math.pow(b, e); }
function testMin(a, b) { return Math.min(a, b); }
function testMax(a, b) { return Math.max(a, b); }
function testHypot(a, b) { return Math.hypot(a, b); }

const test_pow = testPow(2, 10);
const test_min = testMin(3, 7); // => 3
const test_max = testMax(3, 7); // => 7
```

#### T-BUILTIN-05: Math.random / Math.sign

```javascript
function testRandom() { return Math.random(); }
function testSign(x) { return Math.sign(x); }
const test_random = testRandom();
const test_sign_pos = testSign(5);
const test_sign_neg = testSign(-3);
```

#### T-BUILTIN-06: 全局函数 — parseInt / parseFloat

```javascript
function testParseI(s) { return parseInt(s); }
function testParseF(s) { return parseFloat(s); }

const test_parse_int = testParseI("42"); // => 42
const test_parse_float = testParseF("3.14");
```

#### T-BUILTIN-07: 全局函数 — isNaN / isFinite

```javascript
function testIsNaN(x) { return isNaN(x); }
function testIsFinite(x) { return isFinite(x); }

const test_isnan = testIsNaN(0);
const test_isfinite = testIsFinite(42);
```

#### T-BUILTIN-08: String 方法 — 大小写转换

```javascript
function testToUpper(s) { return s.toUpperCase(); }
function testToLower(s) { return s.toLowerCase(); }

const test_upper = testToUpper("hello"); // => "HELLO"
const test_lower = testToLower("HELLO"); // => "hello"
```

**验证**: 映射到 `js_string.toUpper()` / `js_string.toLower()`

#### T-BUILTIN-09: String 方法 — 查找

```javascript
function testCharAt(s, i) { return s.charAt(i); }
function testIncludes(s, sub) { return s.includes(sub); }
function testIndexOf(s, sub) { return s.indexOf(sub); }
function testStartsWith(s, pre) { return s.startsWith(pre); }
function testEndsWith(s, suf) { return s.endsWith(suf); }

const test_charat = testCharAt("hello", 1);
const test_includes = testIncludes("hello world", "world"); // => true
const test_starts = testStartsWith("hello", "hel"); // => true
const test_ends = testEndsWith("hello", "llo"); // => true
```

#### T-BUILTIN-10: String 方法 — 变换

```javascript
function testSlice(s, start, end) { return s.slice(start, end); }
function testSplit(s, sep) { return s.split(sep); }
function testReplace(s, old, n) { return s.replace(old, n); }
function testTrim(s) { return s.trim(); }
function testRepeat(s, n) { return s.repeat(n); }
function testConcat(a, b) { return a.concat(b); }

const test_trim = testTrim("  hello  "); // => "hello"
const test_repeat = testRepeat("ab", 3); // => "ababab"
```

#### T-BUILTIN-11: Array 方法 — 静态数组基础

```javascript
function testArrLen() {
    const arr = [1, 2, 3, 4, 5];
    return arr.length;
}
function testArrIncludes() {
    const arr = [1, 2, 3, 4, 5];
    return arr.includes(3);
}
function testArrIndexOf() {
    const arr = [10, 20, 30];
    return arr.indexOf(20);
}

const test_arr_len = testArrLen(); // => 5
const test_arr_includes = testArrIncludes(); // => true
const test_arr_indexof = testArrIndexOf(); // => 1
```

#### T-BUILTIN-12: Array 方法 — 高阶函数

```javascript
function testArrMap() {
    const arr = [1, 2, 3];
    return arr.map((x) => x * 2);
}
function testArrFilter() {
    const arr = [1, 2, 3, 4, 5];
    return arr.filter((x) => x > 3);
}

const test_arr_map = testArrMap();
const test_arr_filter = testArrFilter();
```

**验证**: 映射到 `js_array.map(arr, fn)` / `js_array.filter(arr, fn)`

#### T-BUILTIN-13: Array 方法 — 变异操作

```javascript
function testArrJoin() {
    const arr = [1, 2, 3];
    return arr.join("-");
}
function testArrReverse() {
    const arr = [1, 2, 3];
    return arr.reverse();
}
function testArrSort() {
    const arr = [3, 1, 2];
    return arr.sort();
}
function testArrSlice() {
    const arr = [10, 20, 30, 40, 50];
    return arr.slice(1, 3);
}
function testArrConcat() {
    const arr1 = [1, 2];
    const arr2 = [3, 4];
    return arr1.concat(arr2);
}

const test_arr_join = testArrJoin();
const test_arr_reverse = testArrReverse();
const test_arr_sort = testArrSort();
const test_arr_slice = testArrSlice();
const test_arr_concat = testArrConcat();
```

#### T-BUILTIN-14: Array 方法 — 动态数组 (ArrayList)

```javascript
function testDynPush() {
    const arr = [];
    arr.push(1);
    arr.push(2);
    arr.push(3);
    return arr;
}
function testDynPop() {
    const arr = [];
    arr.push(10);
    arr.push(20);
    arr.pop();
    return arr;
}

const test_dyn_push = testDynPush();
const test_dyn_pop = testDynPop();
```

**验证**: 动态数组 → ArrayList 方法

#### T-BUILTIN-15: console.log / error / warn

```javascript
function testConsoleLog(msg) {
    console.log(msg);
    return 1;
}
function testConsoleError(msg) {
    console.error(msg);
    return 1;
}
function testConsoleWarn(msg) {
    console.warn(msg);
    return 1;
}

const test_console_log = testConsoleLog("test");
const test_console_error = testConsoleError("error");
const test_console_warn = testConsoleWarn("warn");
```

**验证**: 映射到 `js_console.log/error/warn`

#### T-BUILTIN-16: JSON.stringify / parse

```javascript
function testStringify() {
    const obj = { x: 1, y: 2 };
    return JSON.stringify(obj);
}
const test_json = testStringify();
```

**验证**: 映射到 `js_json.stringify/parse`

#### T-BUILTIN-17: Object.keys / values / entries / assign

```javascript
function testObjectKeys() {
    const obj = { a: 1, b: 2, c: 3 };
    return Object.keys(obj);
}
const test_obj_keys = testObjectKeys();
```

**验证**: 映射到 `js_object.keys/values/entries/assign`

#### T-BUILTIN-18: Number 方法

```javascript
function testNumIsNaN(x) { return Number.isNaN(x); }
function testNumIsFinite(x) { return Number.isFinite(x); }
function testNumIsInteger(x) { return Number.isInteger(x); }

const test_num_isnan = testNumIsNaN(0);
const test_num_isfinite = testNumIsFinite(42);
const test_num_isint = testNumIsInteger(5);
```

#### T-BUILTIN-19: Date.now

```javascript
function testDateNow() {
    return Date.now();
}
const test_date_now = testDateNow();
```

**验证**: 映射到 `js_date.now()`

#### T-BUILTIN-20: Map 方法

```javascript
function testMapOps() {
    const m = new Map();
    m.set("a", 1);
    m.set("b", 2);
    const hasA = m.has("a");
    m.delete("b");
    return hasA;
}
const test_map_ops = testMapOps();
```

**验证**: 映射到 `js_map.JsMap` 的 get/set/has/delete/clear

#### T-BUILTIN-21: Set 方法

```javascript
function testSetOps() {
    const s = new Set();
    s.add(1);
    s.add(2);
    s.add(1); // duplicate
    const hasOne = s.has(1);
    return hasOne;
}
const test_set_ops = testSetOps();
```

**验证**: 映射到 `js_set.JsSet` 的 add/has/delete/clear

#### T-BUILTIN-22: Array.isArray

```javascript
function testIsArray(x) {
    return Array.isArray(x);
}
const test_is_array = testIsArray([1, 2, 3]);
```

#### T-BUILTIN-23: encodeURIComponent / decodeURIComponent

```javascript
function testEncode(s) { return encodeURIComponent(s); }
function testDecode(s) { return decodeURIComponent(s); }

const test_encode = testEncode("hello world");
const test_decode = testDecode("hello%20world");
```

**验证**: 映射到 `js_uri.encodeURIComponent/decodeURIComponent`

---

### 2.6 闭包 (closure.rs) — 5 个测试

#### T-CLS-01: 基础闭包 — 捕获单个变量

```javascript
function makeAdder(x) {
    return (y) => x + y;
}
const test_adder = makeAdder(10)(5); // => 15
```

**验证**: 生成 `_Closure_makeAdder` 结构体，`.call()` 调用

#### T-CLS-02: 闭包 — 捕获多个变量

```javascript
function makeRange(min, max) {
    return (x) => x >= min && x <= max;
}
const test_range = makeRange(1, 10)(5); // => true
```

#### T-CLS-03: 闭包 — 变量赋值

```javascript
function createMultiplier(factor) {
    const mul = (x) => x * factor;
    return mul(5);
}
const test_closure_var = createMultiplier(3); // => 15
```

#### T-CLS-04: 闭包 — 复杂捕获

```javascript
function counter(start) {
    let count = start;
    const inc = () => {
        count = count + 1;
        return count;
    };
    return inc();
}
const test_counter_closure = counter(0); // => 1
```

#### T-CLS-05: 闭包 — 嵌套

```javascript
function outer(a) {
    return (b) => {
        return a * b;
    };
}
const test_nested = outer(3)(4); // => 12
```

---

### 2.7 C ABI 桥接 — 6 个测试

#### T-CABI-01: 基础 i64 导出

```javascript
export function cabiAdd(a, b) { return a + b; }
const test_cabi_add = cabiAdd(3, 5); // => 8
```

**验证**: `callconv(.c)` + 参数类型不变

#### T-CABI-02: String 参数 → [*:0]const u8

```javascript
export function cabiGreet(name) { return "Hello " + name; }
const test_cabi_greet = cabiGreet("World"); // => "Hello World"
```

**验证**: 参数转为 `[*:0]const u8`，内部转 `[]const u8`

#### T-CABI-03: String 返回 → 指针 + free

```javascript
export function cabiReverse(s) {
    return s;
}
```

**验证**: 返回 `[*:0]const u8`，生成 `free_cabiReverse` 函数

#### T-CABI-04: f64 导出

```javascript
export function cabiDivide(a, b) {
    if (b === 0) { return 0; }
    return a / b;
}
const test_cabi_div = cabiDivide(10, 3);
```

#### T-CABI-05: bool 导出

```javascript
export function cabiIsPositive(x) {
    return x > 0;
}
const test_cabi_bool = cabiIsPositive(5); // => true
```

#### T-CABI-06: void 导出

```javascript
export function cabiLog(x) {
    console.log(x);
}
const test_cabi_void = cabiLog(42);
```

---

### 2.8 测试生成 (testgen.rs) — 6 个测试

#### T-TGEN-01: 整数期望值

```javascript
const test_int_exp = add(3, 5); // => 8
```

**验证**: 生成 `expectEqual(@as(i64, 8), ...)`

#### T-TGEN-02: 浮点期望值

```javascript
const test_float_exp = divide(10, 3); // => 3.3333
```

**验证**: 生成 `expectEqual(@as(f64, 3.3333), ...)`

#### T-TGEN-03: 字符串期望值

```javascript
const test_str_exp = greet("World"); // => "Hello, World!"
```

**验证**: 生成 `expectEqualSlices(u8, "Hello, World!", ...)`

#### T-TGEN-04: 布尔期望值

```javascript
const test_bool_exp = isPositive(5); // => true
```

**验证**: 生成 `expectEqual(true, ...)`

#### T-TGEN-05: 冒烟测试（无期望值）

```javascript
const test_smoke = someFunction(1, 2);
```

**验证**: 生成 `_ = ...;`（仅验证不崩溃）

#### T-TGEN-06: JsValue/JsAny 字段提取

```javascript
// 函数返回 JsValue 时，数值比较自动追加 .int
// 函数返回 JsAny 时，字符串比较自动追加 .value.string
```

**验证**: 已在 main.js 的 forLoop/stringConcat 测试中覆盖

---

### 2.9 项目脚手架 (project.rs) — 4 个测试

#### T-PROJ-01: 单文件项目生成

**验证**: 生成 build.zig + src/lib.zig + src/xxx.zig + src/main.zig

#### T-PROJ-02: 多文件项目生成

**验证**: 每个 JS 文件 → 独立 .zig 模块，lib.zig 编排 re-export

#### T-PROJ-03: 运行时文件复制

**验证**: runtime/*.zig → out/group/src/js_runtime/ 完整复制

#### T-PROJ-04: cabi_exports.json 生成

**验证**: 导出函数元数据正确写入 JSON

---

### 2.10 错误处理 — 7 个测试

#### T-ERR-01: 未实现的类表达式

```javascript
const X = class {};  // 应报错 "class expression not yet implemented"
```

#### T-ERR-02: yield / Generator

```javascript
function* gen() { yield 1; }  // 应报错 "generators not yet implemented"
```

#### T-ERR-03: 动态 import()

```javascript
const m = import("./foo");  // 应报错 "use static import instead"
```

#### T-ERR-04: 私有字段

```javascript
class Foo { #bar = 1; }  // 应报错 "private field access not supported"
```

#### T-ERR-05: new.target

```javascript
function Foo() { return new.target; }  // 应报错 "meta property not supported"
```

#### T-ERR-06: JSX

```javascript
const el = <div>hello</div>;  // 应报错 "use createElement() calls instead"
```

#### T-ERR-07: 标签模板

```javascript
const x = tag`hello`;  // 应报错 "tagged template not supported"
```

---

## 3. 测试执行矩阵

### 3.1 E2E 测试（in/*.js → zig build test）

| 测试文件 | 覆盖的测试 ID | 优先级 |
|----------|---------------|--------|
| `in/main.js` (现有) | T-EXPR-04/06/21/23/24/25, T-STMT-07/08, T-FN-01/13 | P0 ✅ |
| `in/math.js` (现有) | T-FN-01, T-CABI-01 | P0 ✅ |
| `in/string_utils.js` (现有) | T-EXPR-22 | P0 ✅ |
| `in/builtins.js` (现有) | T-BUILTIN-02/05/06 | P0 ✅ |
| `in/classes.js` (现有) | T-FN-05, T-INF-05 | P0 ✅ |
| `in/test_types.js` (新建) | T-INF-01~04, T-INF-08/09/13 | **P0** |
| `in/test_expressions.js` (新建) | T-EXPR-01~03/08~12/16~17/22/28 | **P0** |
| `in/test_closures.js` (新建) | T-CLS-01~05 | **P0** |
| `in/test_classes_adv.js` (新建) | T-FN-06~10, T-EXPR-26 | **P1** |
| `in/test_builtins_math.js` (新建) | T-BUILTIN-01~05 | **P1** |
| `in/test_builtins_string.js` (新建) | T-BUILTIN-08~10 | **P1** |
| `in/test_builtins_array.js` (新建) | T-BUILTIN-11~14 | **P1** |
| `in/test_builtins_misc.js` (新建) | T-BUILTIN-15~23 | **P2** |
| `in/test_control_flow.js` (新建) | T-STMT-01~06 | **P1** |
| `in/test_cabi.js` (新建) | T-CABI-01~06 | **P1** |

### 3.2 Rust 单元测试（cargo test）

| 模块 | 覆盖的测试 ID | 优先级 |
|------|---------------|--------|
| `infer.rs` 类型推断规则 | T-INF-01~14 | **P0** |
| `testgen.rs` 期望值解析 | T-TGEN-01~06 | P0 ✅ |
| `codegen/expr.rs` 表达式输出 | T-EXPR-* 快照测试 | **P1** |
| `codegen/stmt.rs` 语句输出 | T-STMT-* 快照测试 | **P1** |
| `codegen/builtins.rs` 内置映射 | T-BUILTIN-* 快照测试 | **P2** |

### 3.3 Rust 集成测试（pipeline_tests.rs）

| 测试 | 覆盖 | 优先级 |
|------|------|--------|
| `test_full_pipeline` (现有) | 基础 E2E | P0 ✅ |
| `test_pipeline_with_classes` (新建) | 类声明 E2E | **P1** |
| `test_pipeline_with_closures` (新建) | 闭包 E2E | **P1** |
| `test_pipeline_error_diagnostics` (新建) | 错误提示 T-ERR-* | **P1** |

---

## 4. 优先级与执行顺序

### P0 — 必须通过（核心功能验证）

1. **T-INF-01~04**: 基础类型推断
2. **T-EXPR-01~03**: 字面量、模板字面量
3. **T-CLS-01~03**: 基础闭包
4. **T-FN-01/05/13**: 函数声明、类、递归
5. 运行现有 5 个 JS 文件 → 3 组 zig build test 全通过

### P1 — 应该通过（重要功能）

6. **T-FN-06~10**: 类高级特性（静态、getter、extends）
7. **T-BUILTIN-01~14**: Math + String + Array 内置 API
8. **T-STMT-01~06**: 解构、for-in、try-catch-finally、标签
9. **T-CABI-01~06**: C ABI 完整桥接
10. **T-EXPR-05/08~12**: 逻辑、幂、对象、数组

### P2 — 锦上添花

11. **T-BUILTIN-15~23**: console、JSON、Object、Number、Date、Map、Set、URI
12. **T-ERR-01~07**: 错误信息测试
13. **T-PROJ-01~04**: 项目脚手架验证
14. **T-INF-10~14**: 边缘类型推断

---

## 5. 执行方法

### 5.1 E2E 测试执行

```bash
# 1. 添加新测试 JS 文件到 in/ 目录
# 2. 运行完整 pipeline
cargo run --manifest-path js2rustc/Cargo.toml

# 3. 检查每组生成的 Zig 代码
# 4. 各组 zig build + zig build test 自动执行

# 如果某组失败，查看具体错误：
cd out/<group_name>
zig build test 2>&1
```

### 5.2 Rust 测试执行

```bash
# 运行所有 Rust 测试
cd js2rustc
cargo test

# 运行特定测试
cargo test test_full_pipeline
cargo test infer::tests
cargo test testgen::tests

# 带输出
cargo test -- --nocapture
```

### 5.3 验证清单

每次修改后，按以下顺序验证：

| 步骤 | 命令 | 期望结果 |
|------|------|----------|
| 1. 编译 | `cargo check` | 零错误 |
| 2. Clippy | `cargo clippy` | 零新警告 |
| 3. Rust 测试 | `cargo test` | 18/18+ 通过 |
| 4. Pipeline | `cargo run` | 所有组 zig build OK |
| 5. Zig 测试 | 自动 `zig build test` | 所有组 PASSED |

---

## 6. 当前已测试 vs 新增需求一览

### 6.1 现有测试（31 个 zig test）

| 文件 | 测试数 | 覆盖 |
|------|--------|------|
| main.js | 27 个 | 算术、位运算、字符串、控制流、循环、递归、try-catch |
| builtins.js | 3 个 | Math.round、Math.sign、parseInt |
| classes.js | 1 个 | class + constructor + method |

### 6.2 需要新增的测试文件

| 文件 | 预估测试数 | 新覆盖 |
|------|-----------|--------|
| test_types.js | ~10 个 | 类型推断规则全集 |
| test_expressions.js | ~15 个 | 模板字面量、对象、数组、幂、NaN/Infinity、++ |
| test_closures.js | ~5 个 | 闭包全场景 |
| test_classes_adv.js | ~8 个 | 静态、getter、extends、属性默认值 |
| test_builtins_math.js | ~15 个 | Math 全量 |
| test_builtins_string.js | ~12 个 | String 方法全量 |
| test_builtins_array.js | ~12 个 | Array 方法全量 |
| test_builtins_misc.js | ~10 个 | console、JSON、Object、Number、Date、Map、Set |
| test_control_flow.js | ~8 个 | 解构、for-in、throw、标签 |
| test_cabi.js | ~6 个 | C ABI 桥接全量 |
| **合计** | **~101 个** | — |

### 6.3 总测试预算

| 类别 | 现有 | 新增 | 合计 |
|------|------|------|------|
| Zig E2E test | 31 | ~101 | ~132 |
| Rust unit test | 10 | ~20 | ~30 |
| Rust integration test | 8 | ~3 | ~11 |
| **总计** | **49** | **~124** | **~173** |

---

## 附录 A: 已知限制（无需测试，仅文档记录）

| 特性 | 状态 | 说明 |
|------|------|------|
| 嵌套函数声明 | ❌ | 报错，需重构为顶层 |
| `with` 语句 | ❌ | JS 严格模式已废弃 |
| `debugger` | ❌ | 无运行时支持 |
| `for-in` 静态对象 | ❌ | 仅支持 HashMap |
| `for await...of` | ❌ | 跳过 |
| Generator / yield | ❌ | 不支持 |
| 动态 import() | ❌ | 不支持 |
| 私有字段 #field | ❌ | 不支持 |
| 类表达式 | ❌ | 不支持 |
| 标签模板 | ❌ | 不支持 |
| 多 spread 合并 | ❌ | `{ ...a, ...b }` 不支持 |
| splice 带插入 | ❌ | 仅删除 |
| class 字段仅 i64 | ⚠️ | 硬编码 |
| 泛型 | ❌ | 无 TS 泛型支持 |
| interface / type alias | ❌ | 不支持 |
