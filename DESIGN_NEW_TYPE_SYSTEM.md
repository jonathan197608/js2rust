# 新类型系统设计文档

## 分支：feature/zig-native-types

---

## 一、设计原则

### 核心思想
取消转译器端的复杂类型推断，让 Zig 编译器原生类型系统承担类型检查职责。
转译器只做**最小必要的语法转换**，生成的 Zig 代码尽量利用 `anytype` 和 `@TypeOf` 让 Zig 编译器自动推导。

### 禁止的 JS 特性（编译错误）

| JS 特性 | 处理方式 |
|---------|---------|
| `undefined` | 编译错误 |
| `null` | 编译错误 |
| `BigInt` | 编译错误 |
| `Symbol` | 编译错误 |
| 顶层 `var` / `let` | 编译错误（只允许 `const`） |
| 动态 Object 作为函数返回值 | 编译错误 |
| 动态 Object 作为函数入参 | 允许（但入参类型是 `anytype`） |

---

## 二、类型映射

### 2.1 基本类型（直接映射）

| JS 类型 | Zig 类型 | 说明 |
|---------|---------|------|
| `Boolean` 字面量 / 常量 | `bool` | 无需转换 |
| `Number` 整数字面量 / 常量 | `i64` | 转译器识别整数 vs 浮点 |
| `Number` 浮点字面量 / 常量 | `f64` | |
| `String` 字面量 / 常量 | `[]const u8` | 无需转换 |
| `Boolean` 变量 | `bool` | |
| `Number` 变量（整数） | `i64` | 转译器根据使用场景推断 |
| `Number` 变量（浮点） | `f64` | |
| `String` 变量 | `[]const u8` 或 `std.ArrayList(u8)` | 取决于是否修改 |

**整数 vs 浮点判断规则**（转译器端）：
- 字面量：`42` → `i64`，`3.14` → `f64`
- 算术运算：含 `/` 或其中一个操作数是浮点 → `f64`，否则 `i64`
- 函数返回值：收集所有 `return` 表达式，取统一类型

### 2.2 特殊类型

| 名称 | Zig 类型 | 使用场景 |
|------|---------|---------|
| `JsAny` | `js_runtime.JsAny` | 动态类型场景的唯一 union 类型 |

`JsAny` 定义（已有，保留）：
```zig
pub const JsAny = union(enum) {
    int: i64,
    float: f64,
    bool: bool,
    string: []const u8,
    // 复合类型用指针
    array: *std.ArrayList(JsAny),
    object: *std.StringHashMap(JsAny),
    none, // 对应 undefined（禁止，但运行时可能需要）
};
```

### 2.3 复合类型

| JS 构造 | Zig 类型 | 生成代码 |
|---------|---------|---------|
| 常量数组 `[1, 2, 3]` | `[3]i64` | `const arr = [3]i64{ 1, 2, 3 };` |
| 空数组 `[]` | `std.ArrayList(JsAny)` | `var arr = std.ArrayList(JsAny).init(alloc);` |
| 含混合类型数组 `[1, "a"]` | `std.ArrayList(JsAny)` | 同上 |
| 常量 Object `{x: 1, y: 2}` | `struct { x: i64, y: i64 }` | 自动生成匿名 struct |
| 空 Object `{}` | `std.StringHashMap(JsAny)` | `var obj = std.StringHashMap(JsAny).init(alloc);` |
| 动态 Object（运行时添加属性） | `std.StringHashMap(JsAny)` | 同上 |

---

## 三、代码生成规则

### 3.1 顶层常量赋值

**规则**：只允许 `const`，`var`/`let` 报编译错误。

```javascript
// JS 源码
const x = 42;
const name = "Alice";
const flag = true;
```

```zig
// 生成 Zig 代码
const x: i64 = 42;
const name: []const u8 = "Alice";
const flag: bool = true;
```

**类型推断**：根据字面量直接确定，无需额外注解。

### 3.2 函数定义

**核心规则**：入参用 `anytype`，返回值用 `@TypeOf` 根据 `return` 表达式推导。

```javascript
// JS 源码
function add(a, b) {
    return a + b;
}

function abs(x) {
    if (x < 0) {
        return -x;
    }
    return x;
}
```

```zig
// 生成 Zig 代码
fn add(x: anytype, y: anytype) !@TypeOf(x + y) {
    return x + y;
}

fn abs(x: anytype) !@TypeOf(x) {
    if (x < 0) {
        return -x;
    }
    return x;
}
```

**多 return 语句处理**：
- 收集所有 `return` 表达式的类型
- 取兼容的公共类型（如 `i64` 和 `f64` → `f64`）
- 如果有中间变量，使用中间变量的初始化表达式

```javascript
// JS 源码
function foo(a, b) {
    if (a > b) {
        var r = a - b;
        return r;
    }
    return a + b;
}
```

```zig
// 生成 Zig 代码（@TypeOf 使用 return 表达式，不用中间变量 r）
fn foo(a: anytype, b: anytype) !@TypeOf(a - b, a + b) {
    if (a > b) {
        var r: @TypeOf(a - b) = a - b;
        return r;
    }
    return a + b;
}
```

**无 return 语句**：返回值类型 `!void`

```javascript
function log(msg) {
    console.log(msg);
}
```

```zig
fn log(msg: anytype) !void {
    // ...
}
```

### 3.3 函数体内变量赋值

**规则**：转译器根据赋值表达式推断变量类型，生成 Zig 类型声明。

**推断流程**：
1. 扫描函数体内所有赋值语句（`var x = expr` 或 `x = expr`）
2. 根据 `expr` 的类型确定 `x` 的类型
3. 如果同一变量多次赋值，取所有类型的兼容超类型（如 `i64` → `f64`）
4. 无法推断时报编译错误

```javascript
// JS 源码
function compute() {
    var total = 0;        // → i64
    var tax = 0.08;       // → f64
    var name = "Alice";    // → []const u8
    total = total + 1;
    return total;
}
```

```zig
// 生成 Zig 代码
fn compute() !i64 {
    var total: i64 = 0;
    var tax: f64 = 0.08;
    var name: []const u8 = "Alice";
    total = total + 1;
    return total;
}
```

**特殊处理**：
- 变量先声明后赋值（`var x; x = 1;`）→ 根据首次赋值推断
- 变量声明时不赋值（`var x = undefined;`）→ 必须显式标注类型（未来可以考虑 JS 类型注解）

### 3.4 数组

**常量数组**：
```javascript
const nums = [1, 2, 3];
```

```zig
const nums = [3]i64{ 1, 2, 3 };
```

**空数组（动态数组）**：
```javascript
var dynamic = [];
```

```zig
var dynamic = std.ArrayList(JsAny).init(alloc);
```

**数组方法调用**：
- `push/pop/shift/unshift` → 生成 `ArrayList` 对应方法
- `length` → `.items.len`（动态数组）或 `.len`（常量数组）

```javascript
dynamic.push(1);
var len = dynamic.length;
```

```zig
try dynamic.append(alloc, JsAny{ .int = 1 });
var len = dynamic.items.len;
```

### 3.5 Object

**静态 Object**（属性名编译期已知）：
```javascript
const point = { x: 1, y: 2 };
const px = point.x;
```

```zig
const point = struct { x: i64, y: i64 }{ .x = 1, .y = 2 };
const px = point.x;
```

**动态 Object**（运行时添加属性）：
```javascript
var obj = {};
obj.name = "Alice";
obj.age = 30;
var n = obj.name;
```

```zig
var obj = std.StringHashMap(JsAny).init(alloc);
try obj.put("name", JsAny{ .string = "Alice" });
try obj.put("age", JsAny{ .int = 30 });
var n = (try obj.get("name")).?;
```

**属性访问类型转换**：
当 `StringHashMap(JsAny)` 的值被使用时，根据使用场景自动生成类型转换：

```javascript
var obj = {};
obj.val = "42";
var num = parseInt(obj.val);  // obj.val 是 JsAny，需要转为 []const u8
```

```zig
var obj = std.StringHashMap(JsAny).init(alloc);
try obj.put("val", JsAny{ .string = "42" });
var num = std.fmt.parseInt(i64, (try obj.get("val")).?.string) catch unreachable;
```

（注：`parseInt` 是内置函数，转译器知道它期望 `[]const u8` 入参）

### 3.6 运算符

**基本规则**：直接映射到 Zig 运算符，不做操作数类型转换（让 Zig 编译器报错）。

| JS 运算符 | Zig 运算符 | 说明 |
|-----------|------------|------|
| `+` | `+` | 特殊处理（见下） |
| `-` | `-` | |
| `*` | `*` | |
| `/` | `/` | 整数除法 → 自动转 `f64` |
| `%` | `%` | |
| `===` | `==` | Zig 无类型 coercion |
| `!==` | `!=` | |
| `<` `>` `<=` `>=` | 同左 | |
| `&&` | `and` | |
| `\|\|` | `or` | |

**`+` 特殊处理**：
1. 如果两个操作数都是字面量且至少一个是字符串 → 生成字符串拼接 `++`
2. 如果一个是变量（字符串类型），另一个是其他类型 → 生成 `try std.fmt.allocPrint` 或 `ArrayList(u8)` 拼接
3. 如果两个都是数值类型 → 算术加法

```javascript
var s = "Hello, " + name + "!";
```

```zig
var s = std.ArrayList(u8).init(alloc);
try s.appendSlice("Hello, ");
try s.appendSlice(name);
try s.appendSlice("!");
// s 是 ArrayList(u8)，如果需要 []const u8，调用 s.items
```

**类型不兼容**（如 `string + object`）→ 报编译错误，要求用户修改 JS 写法。

### 3.7 函数调用

**规则**：所有函数调用都加 `try`（因为返回值是 `!T`）。

```javascript
var result = add(1, 2);
```

```zig
var result = try add(1, 2);
```

**主机函数调用**：和普通函数一样，加 `try`。

### 3.8 控制流

**if/else**：
```javascript
if (x > 0) {
    return x;
} else {
    return -x;
}
```

```zig
if (x > 0) {
    return x;
} else {
    return -x;
}
```

**while 循环**：
```javascript
var i = 0;
while (i < 10) {
    i = i + 1;
}
```

```zig
var i: i64 = 0;
while (i < 10) {
    i = i + 1;
}
```

**for 循环**：
```javascript
for (var i = 0; i < arr.length; i = i + 1) {
    console.log(arr[i]);
}
```

```zig
var i: i64 = 0;
while (i < arr.len) : (i = i + 1) {
    std.debug.print("{}\n", .{arr[i]});
}
```

### 3.9 错误和异常

**`throw`**：
```javascript
if (x < 0) {
    throw "error: negative";
}
```

```zig
if (x < 0) {
    return error.Negative;
}
```

**`try-catch`**：
```javascript
try {
    var r = riskyFunc();
} catch (e) {
    console.log(e);
}
```

```zig
var r = riskyFunc() catch |e| {
    std.debug.print("{}\n", .{e});
};
```

---

## 四、转译器实现

### 4.1 架构调整

**当前架构**：
```
JS 源码 → oxc_parser → AST → infer.rs (类型推断) → codegen (生成 Zig)
```

**新架构**：
```
JS 源码 → oxc_parser → AST → minimal_type_check.rs (最小类型检查) → new_codegen (生成 Zig)
```

**关键变化**：
1. 移除/禁用 `infer.rs`（保留但不在新分支使用）
2. 新增 `minimal_type_check.rs`：只做禁止特性检查和最基本类型收集
3. 新增 `new_codegen` 模块：生成使用 `anytype` 的 Zig 代码
4. 通过 feature flag 或命令行参数切换新旧 codegen

### 4.2 最小类型检查（`minimal_type_check.rs`）

**职责**：
1. 检查禁止特性（`undefined`/`null`/`BigInt`/`Symbol`/顶层 `var`）
2. 收集函数签名（参数名 + return 表达式）
3. 收集变量赋值语句（用于生成类型声明）
4. 识别常量数组 vs 动态数组
5. 识别静态 Object vs 动态 Object

**输出**（传递给 new_codegen）：
```rust
struct TypeInfo {
    functions: Vec<FunctionInfo>,
    variables: Vec<VariableInfo>,
    constants: Vec<ConstantInfo>,
    // ...
}

struct FunctionInfo {
    name: String,
    params: Vec<String>,  // 参数名，类型都是 anytype
    return_expressions: Vec<String>,  // return 表达式（用于 @TypeOf）
    return_type: ZigType,  // 推断出的返回值类型
}

struct VariableInfo {
    name: String,
    inferred_type: ZigType,
    assignments: Vec<Expression>,  // 所有赋值表达式
}
```

### 4.3 新代码生成（`new_codegen` 模块）

**核心函数**：
```rust
pub fn generate_zig(ast: &Program, type_info: &TypeInfo) -> String {
    // 生成完整的 Zig 代码
}
```

**生成顺序**：
1. 文件头（`const std = @import("std");` 等）
2. 顶层常量（`const x = ...`）
3. 函数定义
4. 辅助类型定义（如静态 Object 的 struct）

### 4.4 类型推断（转译器端，最小化）

虽然大部分类型推导交给 Zig 编译器，但转译器仍需做**最基本**的推断来生成变量声明。

**变量类型推断**：
```rust
fn infer_variable_type(assignments: &[Expression]) -> ZigType {
    // 收集所有赋值表达式的类型
    let types: Vec<ZigType> = assignments.iter()
        .map(|expr| infer_expr_type(expr))
        .collect();
    // 取兼容的超类型
    unify_types(&types)
}
```

**`infer_expr_type` 规则**（简化版）：
- 字面量 `42` → `i64`
- 字面量 `"abc"` → `[]const u8`
- 字面量 `true` → `bool`
- 二元运算 `a + b` → 根据操作数类型确定
- 函数调用 `f(x)` → 需要知道 `f` 的返回值类型（查 FunctionInfo）

---

## 五、分阶段实施计划

### Phase 1: 基础设施（1-2 天）

- [ ] 创建 `js2zig-core/src/new_codegen/` 模块
- [ ] 实现 `minimal_type_check.rs`：
  - [ ] 检查禁止特性
  - [ ] 收集函数信息
  - [ ] 收集变量赋值信息
- [ ] 实现基本的 Zig 代码生成：
  - [ ] 文件头
  - [ ] 常量赋值（`const x = ...`）
  - [ ] 简单函数定义（无 return）
- [ ] 添加 feature flag 或命令行参数切换新旧 codegen

### Phase 2: 函数和变量（2-3 天）

- [ ] 实现函数定义生成：
  - [ ] `anytype` 入参
  - [ ] `@TypeOf` 返回值类型
  - [ ] `!T` 错误返回
- [ ] 实现变量赋值生成：
  - [ ] 类型推断
  - [ ] `var x: Type = ...` 生成
- [ ] 实现 return 语句
- [ ] 实现基本控制流（if/while/for）

### Phase 3: 复合类型（2-3 天）

- [ ] 实现数组生成：
  - [ ] 常量数组 `[N]T{ ... }`
  - [ ] 动态数组 `ArrayList(JsAny)`
  - [ ] 数组方法（`push`/`pop`/`length`）
- [ ] 实现 Object 生成：
  - [ ] 静态 Object → struct
  - [ ] 动态 Object → `StringHashMap(JsAny)`
  - [ ] 属性访问 `.key` 或 `.get("key")`
- [ ] 实现运算符生成（含 `+` 特殊处理）

### Phase 4: 内置函数和主机函数（1-2 天）

- [ ] 适配内置函数（`Math`/`Array`/`Object`/`console`）
- [ ] 适配主机函数调用（加 `try`）
- [ ] 实现 `throw`/`try-catch` 生成

### Phase 5: 验证和测试（2-3 天）

- [ ] 创建测试用例（覆盖所有语法特性）
- [ ] 验证生成的 Zig 代码能编译运行
- [ ] 对比新旧方案的编译错误友好度
- [ ] 性能对比（编译速度、运行速度）
- [ ] 文档更新

---

## 六、风险和注意事项

### 6.1 `anytype` 的限制

- `anytype` 只能在函数参数使用，不能用于变量声明
- 如果同一个函数被用不同类型调用，Zig 会生成多个实例化（类似 C++ 模板）
- 过度使用 `anytype` 可能导致编译错误不够友好

### 6.2 `@TypeOf` 的复杂性

- `@TypeOf(x + y, x - y)` 需要所有表达式都有相同类型
- 如果 return 语句有不同分支返回不同类型，需要手动统一

### 6.3 与现有代码的兼容性

- 新方案生成的 Zig 代码和旧方案不兼容
- 需要保留旧方案作为 fallback，或制定迁移计划

### 6.4 性能影响

- `anytype` + `@TypeOf` 是编译期特性，不影响运行期性能
- 但可能增加编译时间和二进制大小（模板实例化）

---

## 七、示例对比

### 示例 1：简单函数

**JS 源码**：
```javascript
function add(a, b) {
    return a + b;
}
```

**旧方案生成**：
```zig
fn add(a: JsAny, b: JsAny) !JsAny {
    // 复杂的类型检查和转换
    if (a.isInt() and b.isInt()) {
        return JsAny{ .int = a.asI64() + b.asI64() };
    }
    // ...
}
```

**新方案生成**：
```zig
fn add(a: anytype, b: anytype) !@TypeOf(a + b) {
    return a + b;
}
```

### 示例 2：变量赋值

**JS 源码**：
```javascript
function compute() {
    var total = 0;
    var name = "Alice";
    total = total + 1;
    return total;
}
```

**旧方案生成**：
```zig
fn compute() !JsAny {
    var total = JsAny{ .int = 0 };
    var name = JsAny{ .string = "Alice" };
    total = JsAny{ .int = total.asI64() + 1 };
    return total;
}
```

**新方案生成**：
```zig
fn compute() !i64 {
    var total: i64 = 0;
    var name: []const u8 = "Alice";
    total = total + 1;
    return total;
}
```

---

## 八、决策点

1. **是否完全移除 `infer.rs`**？
   - 建议：暂时保留，新分支使用新 codegen，旧 codegen 作为 fallback

2. **如何切换新旧 codegen**？
   - 建议：命令行参数 `--experimental-native-types` 或环境变量

3. **是否需要 JS 类型注解支持**？
   - 建议：未来可以考虑，当前不强制

4. **`JsAny` 是否足够表达所有动态类型场景**？
   - 需要验证：Array/Object 嵌套、函数作为值等

---

## 九、下一步

1. 确认设计方案
2. 开始 Phase 1 实施
3. 每周同步进度和调整方案
