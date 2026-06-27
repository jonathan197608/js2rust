# JS 语言特性实现评估

> **项目**: js2rust (JS → Zig 转译器)
> **评估日期**: 2026-06-26 (MDN 标准对齐重评估, 2026-06-26 更新)
> **代码版本**: main branch (Post-Phase7-builtins)
> **测试覆盖**: 267 个 Rust 测试 + 27 个 Zig 测试

---

## 1. 执行总结

| 指标 | 数值 | 占比 |
|------|------|------|
| **JS 语法特性总数** | ~150+ | - |
| **完全实现** | ~111 | ~73% |
| **部分实现** | ~8 | ~5% |
| **未实现（计划实现）** | ~41 | ~13% |
| **不实现（低价值）** | ~13 | ~9% |
| **内置对象有效覆盖率** | ~195/260 | ~75% |
| **测试覆盖** | 267 个 Rust 测试 + 27 个 Zig 测试 | - |

**实现策略说明**:
- **计划实现（P2/P3）**: 按优先级逐步实现
- **不实现（🔘）**: 应用价值低，或 Zig 有更好替代方案，或 JS 已废弃特性

**更新说明** (2026-06-27):
- **`obj[key]` 计算属性访问完成**: `infer/passes.rs` 移除错误阻断，`codegen/expr.rs` `emit_expr`/`emit_assignment` 按 `obj` 类型分发（struct → `.field`，HashMap → `.get()`/`.put()`），`infer/expr.rs` 添加 `ComputedMemberExpression` 类型推断，9 个测试通过
- **`String.match()` Phase 1+2+3 完成**: 正则字面量/`RegExp` 变量/`/g` 标志/捕获组/边界情况全部支持，通过 9 个 ast-check 测试
- 测试计数 250 → 267（+17 个新测试）

**更新说明** (2026-06-26):
- **Phase 8 完成**: `encodeURI(s)` / `decodeURI(s)` 实现 — 新增 BuiltinCall 变体 `EncodeURI`/`DecodeURI`，接入 `js_uri.encodeURI/decodeURI` runtime，添加 Rust 测试，Global 8/8 (100%)
- **Phase 7 完成**: Set.forEach() 实现（inline for 循环遍历 `items.items`），新增 4 个 Set 测试（add/has/forEach/iterators/delete/clear），Set 有效覆盖率 10/12 (83%)
- **Phase 6 完成**: String 高级方法（startsWith/endsWith/includes/slice/concat/repeat/substring/normalize/toUpperCase/toLowerCase/split/charAt/indexOf/padStart/padEnd/replace/replaceAll/charCodeAt/codePointAt/toLocaleUpperCase/toLocaleLowerCase/localeCompare/fromCharCode/fromCodePoint）24 个测试全部通过
- **Phase 5 完成**: Object.create/seal/defineProperty/getPrototypeOf + Date.toJSON/valueOf + 15 个 setter 方法
- **Phase 4 全部完成**: #626-#639 共 14 个任务全部完成，新增 Array.keys/values/entries、String.at/codePointAt/replaceAll/fromCharCode/fromCodePoint、Number 常量/.toExponential/toPrecision、Date 构造函数重载/UTC getter/toString 系列等方法
- 覆盖率从 ~53% 提升至 ~73%（~190/260）
- Zig 测试新增 27 个（Object 5 + Date 18）

**更新说明** (2026-06-25):
- **内置对象覆盖重新评估**: 第 4 节全部重写，用三层分析（检测/发射/运行时）替代原来过于乐观的单层标记。有效覆盖率从声称的 80-100% 修正为实际 ~22%。
- 21 个 runtime 函数已实现但 codegen 未连线（最大低挂果实）。
- console.log/error/warn 完全缺失（0/3），是最高频使用的空缺。

**更新说明** (2026-06-24, P0/P1 全部连线完成):
- **P0/P1 内置对象全部连线**: 27 个 runtime 已实现的方法已全部接入 BuiltinCall 检测/发射流水线。
- 有效覆盖率从 ~22% 提升至 ~53%（~57 → ~138 方法）。
- console (3/3 100%)、Global (6/8 75%)、Number (5/14 36%)、Math (11→31/44 70%) 已更新。
- 剩余缺失均为 P2/P3（需新增 runtime 的复杂方法）。

**更新说明** (2026-06-24, MDN 对齐):
- **第 4 节 MDN 标准对齐重写**: 各内置对象方法表增加 MDN 签名、参数、返回值、Zig 等效列。
- 新增 7 个类别完整方法表（Math 44 方法、Array 35、String 35、Map 12、Set 12、Object 23、Number 14）。
- 每个类别包含 MDN 官方示例作为测试用例参考。
- P0/P1/P2/P3 优先级标注每个缺失方法，方便实施排期。

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
| `void` | 🔘 不实现 | `@compileError("Unsupported unary operator")` | 已废弃，Zig 直接用 `null` |
| `delete` | 🔘 不实现 | `@compileError("Unsupported unary operator")` | 已废弃，Zig 用 `HashMap.remove()` |

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
| `**=` (指数赋值) | 🔴 P2 | 未实现 | ES2016 |
| `&&=` (逻辑与赋值) | 🔴 P2 | 未实现 | ES2021，现代 JS 常用 |
| `||=` (逻辑或赋值) | 🔴 P2 | 未实现 | ES2021，现代 JS 常用 |
| `??=` (空值合并赋值) | 🔴 P2 | 未实现 | ES2021，现代 JS 常用 |

### 2.9 对象/数组访问 - ⚠️ 部分实现

| 特性 | 状态 | Zig 输出 | 测试 |
|------|------|----------|------|
| `obj.prop` (属性访问) | ✅ | `obj.prop` | showcase-project |
| `obj[key]` (计算属性) | ✅ | 按 `obj` 类型分发：`struct` → `obj.field`，`HashMap` → `obj.get(key)`/`obj.put(key, val)` | `test_native_proto_computed_member` |
| `arr[idx]` (数组索引) | ✅ | `arr[idx]` (仅支持数字字面量) | showcase-project |
| `.length` → `.len` | ✅ | 自动转换 | 同上 |

**注意**:
- `obj[key]` 现已支持：struct 类型按字符串字面量 key 映射到 `.field`，HashMap 类型生成 `.get(key)`/`.put(key, val)`
- `arr[idx]` 仍仅支持数字字面量索引（如 `arr[0]`），变量索引待后续支持

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
| 标签模板 `` tag`...` `` | 🔘 不实现 | `@compileError` | 很少使用 |

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
| `new Promise(...)` | 🔘 不实现 | `@compileError` | 建议用 `async/await` + `Io` 模式替代 |
| 其他构造函数 | ✅ | 自动映射 | 隐式测试 |

### 2.15 `await` 表达式 - ✅ 100% 实现

| 特性 | 状态 | Zig 输出 | 测试 |
|------|------|----------|------|
| `await expr` | ✅ | `io.async(fn, .{io, args}).await(io)` | test-bin-project |

### 2.16 其他表达式 - ⚠️ 部分实现

| 特性 | 状态 | Zig 输出 | 测试 |
|------|------|----------|------|
| `instanceof` | 🟡 P3 | `@compileError("instanceof operator is not supported")` | Zig 有更好的类型系统 |
| `"key" in obj` | ✅ | `@hasField(...)` 或 `.contains(key)` | 未测试 |
| 正则表达式 `/pattern/` | ✅ | `"pattern"` (提取 pattern) | 未测试 |
| 可选链 `obj?.prop` | ✅ | `if (obj) |v| v.prop else null` | 5 个测试 |
| 非空断言 `x!` (TS) | ✅ | `x.?` | 未测试 |
| 类型断言 `x as T` (TS) | ✅ | `@as(T, expr)` | 未测试 |
| 序列表达式 `a, b` | ✅ | `a, b` | 未测试 |

**注意**:
- `instanceof` 在 JS 中用于检查对象原型链，但当前实现不支持

### 2.17 不支持的表达式 - 按价值分类

| 特性 | 错误信息 | 评估 |
|------|----------|------|
| 类表达式 `const X = class {}` | `Unsupported NewExpression` | 🔘 不实现（很少使用，可用 `class X {}` 替代） |
| `function*` (生成器函数) | `Unsupported expression type: Function` | 🟡 P3（现代 JS 高级特性） |
| `yield` / `yield*` (生成器) | `Unsupported expression type` | 🟡 P3（配合 `function*`） |
| `async function*` (异步生成器) | 未测试 | 🔘 不实现（niche 场景） |
| 动态 `import()` | 需使用静态 `import` | 🟡 P3（ES 模块动态导入） |
| 私有字段 `#field` | 不支持 | 🟡 P3（ES2022，现代 JS 封装） |
| `new.target` | meta property not supported | 🔘 不实现（meta property，niche） |
| Spread 参数 `fn(...args)` | `Spread argument not supported` | 🔴 P2（ES2015 常用） |
| `for await...of` | `Promise.{}() not supported` | 🔘 不实现（异步迭代，当前项目聚焦同步代码） |
| 标签模板 `` tag`...` `` | `Unsupported expression type` | 🔘 不实现（已在 2.12 标记） |
| `import.meta` | 未实现 (ES 模块元数据) | 🔘 不实现（ES 模块元数据，niche） |

---

## 2.18 JSDoc 类型标注 (JSDoc Type Annotations) - ✅ 部分实现

> **实现日期**: 2026-06-25
> **实现方式**: JSDoc 注释中的 `@type`、`@returns`、`@param` 标签支持类型标注，影响 Zig 代码生成中的类型推断。

| 特性 | 状态 | 说明 | 测试 |
|------|------|------|------|
| `@type {string}` (基本类型) | ✅ | 指定变量/属性类型 | `test_native_proto_*` |
| `@type {number[]}` (数组) | ✅ | 数组类型标注 | 隐式测试 |
| `@type {{name: string, age: number}}` (匿名对象) | ✅ | 内联对象类型，生成 Zig struct | `test_native_proto_anon_obj_*` |
| `@returns {{name: string, ...}}` (匿名对象返回) | ✅ | export 函数返回匿名对象类型 | `test_native_proto_anon_obj_*` |
| `@param {Type} name` (参数类型) | ✅ | 参数类型标注 | 隐式测试 |
| `@typedef` (类型别名) | ✅ | 命名类型，可跨文件引用 | 隐式测试 |
| `@property {Type} name` (typedef 属性) | ✅ | typedef 属性定义 | 隐式测试 |

**实现细节**:
- `extract_braced_type()` 处理 `{{name: string}}` 双括号语法（外层 `{}` 是 JSDoc wrapper，内层 `{}` 是匿名对象类型）
- `infer/fn_types.rs::jsdoc_str_to_zig_type()` → `parse_anonymous_object_type()` 递归解析 `{name: type, ...}` → `ZigType::Struct(fields)`
- 匿名对象类型支持嵌套：`{address: {city: string, zip: number}}`
- 匿名对象数组：`{name: string}[]`

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
| 嵌套函数声明 | 🟡 P3 | 报错（需重构为顶层） | 传统 JS 模式 |
| `arguments` 对象 | 🔘 不实现 | 未实现 | 传统函数参数对象，箭头函数已替代 |

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
| 私有字段 `#field` | 🟡 P3 | `@compileError` | ES2022 封装 |
| 类表达式 `const X = class {}` | 🔘 不实现 | `@compileError` | 很少使用 |
| 静态初始化块 `static {}` | 🔘 不实现 | 未实现 | ES2022，使用较少 |

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
| `for await...of` | 🔘 不实现 | `Promise.{}() not supported` | 异步迭代，当前项目聚焦同步代码 |

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
| `with` 语句 | 🔘 不实现 | JS 严格模式已废弃 | 绝不实现 |
| `debugger` 语句 | 🔘 不实现 | 不支持 | 调试用，Zig 有自身调试工具 |
| 声明 + 表达式混合 | 🚧 | 可能产生未使用变量警告 | 隐式测试 |

---

## 4. 内置对象 (Built-in Objects)

> **评估方法**: 内置对象经过三层流水线才能正常工作：
> 1. **检测 (Detect)** — `js2zig-core/src/native_builtins.rs` 的 `BuiltinCall` 枚举 + `detect_builtin_call()`
> 2. **发射 (Emit)** — `js2zig-core/src/codegen/expr.rs` 的 `emit_builtin_call()` 生成 Zig 代码
> 3. **运行时 (Runtime)** — `runtime/*.zig` 提供 Zig 侧实现
>
> 三层全部 ✅ 才算"有效覆盖"。仅 runtime 有但检测/发射缺失 → 实际不可用。
>
> **MDN 参考标准**: [MDN Global Objects](https://developer.mozilla.org/zh-CN/docs/Web/JavaScript/Reference/Global_Objects)。
> 各方法的签名、参数、返回值均对照 MDN 标准文档。
> 测试用例须包含 MDN 官方示例，存放于 `examples/builtins-mdn-tests/js_src/`。

### 4.1 `Math` — 41/44 (93%)

> **Runtime 策略**: Zig 内置 `@sin/@cos/@tan/@log/@exp` 等直接映射，零额外 runtime。

| 方法/属性 | MDN 签名 | 参数 | 返回值 | Zig 等效 | 检测 | 发射 | 运行时 | 状态 |
|----------|----------|------|--------|----------|------|------|--------|------|
| `Math.PI` | 静态属性 | — | `f64` | `std.math.pi` | ✅ | ✅ | ✅ | ✅ |
| `Math.abs(x)` | `Math.abs(x)` | `x: number` | `number` | `@abs(x)` | ✅ | ✅ | ✅ | ✅ |
| `Math.ceil(x)` | `Math.ceil(x)` | `x: number` | `number` | `@ceil(x)` | ✅ | ✅ | ✅ | ✅ |
| `Math.floor(x)` | `Math.floor(x)` | `x: number` | `number` | `@floor(x)` | ✅ | ✅ | ✅ | ✅ |
| `Math.round(x)` | `Math.round(x)` | `x: number` | `number` | `@round(x)` | ✅ | ✅ | ✅ | ✅ |
| `Math.sqrt(x)` | `Math.sqrt(x)` | `x: number` | `number` | `@sqrt(x)` | ✅ | ✅ | ✅ | ✅ |
| `Math.random()` | `Math.random()` | — | `[0,1)` f64 | crypto.random | ✅ | ✅ | ✅ | ✅ |
| `Math.pow(b,e)` | `Math.pow(base, exponent)` | `base, exp: number` | `number` | `std.math.pow(f64, b, e)` | ✅ | ✅ | ✅ | ✅ |
| `Math.max(...v)` | `Math.max(...values)` | `values: number[]` | `number` | labeled block + loop | ✅ | ✅ | ✅ | ✅ |
| `Math.min(...v)` | `Math.min(...values)` | `values: number[]` | `number` | labeled block + loop | ✅ | ✅ | ✅ | ✅ |
| `Math.hypot(...v)` | `Math.hypot(...values)` | `values: number[]` | `number` | `@sqrt(sum of squares)` | ✅ | ✅ | — | ✅ P1 done |
| **— 三角函数 (6) —** | | | | | | | | |
| `Math.sin(x)` | `Math.sin(x)` | `x: number` (弧度) | `number` | `@sin(@as(f64, @floatFromInt(x)))` | ✅ | ✅ | — | ✅ P1 done |
| `Math.cos(x)` | `Math.cos(x)` | `x: number` (弧度) | `number` | `@cos(@as(f64, @floatFromInt(x)))` | ✅ | ✅ | — | ✅ P1 done |
| `Math.tan(x)` | `Math.tan(x)` | `x: number` (弧度) | `number` | `@tan(@as(f64, @floatFromInt(x)))` | ✅ | ✅ | — | ✅ P1 done |
| `Math.asin(x)` | `Math.asin(x)` | `x: number [-1,1]` | `number` (弧度) | `std.math.asin(@as(f64, @floatFromInt(x)))` | ✅ | ✅ | — | ✅ P1 done |
| `Math.acos(x)` | `Math.acos(x)` | `x: number [-1,1]` | `number` (弧度) | `std.math.acos(@as(f64, @floatFromInt(x)))` | ✅ | ✅ | — | ✅ P1 done |
| `Math.atan(x)` | `Math.atan(x)` | `x: number` | `number` (弧度) | `@atan(@as(f64, @floatFromInt(x)))` | ✅ | ✅ | — | ✅ P1 done |
| `Math.atan2(y,x)` | `Math.atan2(y, x)` | `y, x: number` | `number` (弧度) | `std.math.atan2(f64, y, x)` | ✅ | ✅ | — | ✅ P1 done |
| **— 对数/指数 (5) —** | | | | | | | | |
| `Math.log(x)` | `Math.log(x)` | `x: number` | `number` (ln) | `@log(@as(f64, @floatFromInt(x)))` | ✅ | ✅ | — | ✅ P1 done |
| `Math.log10(x)` | `Math.log10(x)` | `x: number` | `number` | `@log10(@as(f64, @floatFromInt(x)))` | ✅ | ✅ | — | ✅ P1 done |
| `Math.log2(x)` | `Math.log2(x)` | `x: number` | `number` | `@log2(@as(f64, @floatFromInt(x)))` | ✅ | ✅ | — | ✅ P1 done |
| `Math.exp(x)` | `Math.exp(x)` | `x: number` | `eˣ` | `@exp(@as(f64, @floatFromInt(x)))` | ✅ | ✅ | — | ✅ P1 done |
| `Math.expm1(x)` | `Math.expm1(x)` | `x: number` | `eˣ - 1` | `std.math.expm1(x)` | ✅ | ✅ | — | ✅ Phase 4 |
| **— 其他数学函数 (8) —** | | | | | | | | |
| `Math.sign(x)` | `Math.sign(x)` | `x: number` | `-1\|0\|1\|NaN` | inline if/else→f64 | ✅ | ✅ | — | ✅ P1 done |
| `Math.trunc(x)` | `Math.trunc(x)` | `x: number` | `number` (截断) | `@trunc(@as(f64, @floatFromInt(x)))` | ✅ | ✅ | — | ✅ P1 done |
| `Math.cbrt(x)` | `Math.cbrt(x)` | `x: number` | `number` (立方根) | `std.math.cbrt(@as(f64, @floatFromInt(x)))` | ✅ | ✅ | — | ✅ P1 done |
| `Math.sinh/cosh/tanh(x)` | 双曲函数 | `x: number` | `number` | `std.math.sinh/cosh/tanh` | ✅ | ✅ | — | ✅ Phase 4 |
| `Math.asinh/acosh/atanh(x)` | 反双曲 | `x: number` | `number` | `std.math.asinh/acosh/atanh` | ✅ | ✅ | — | ✅ Phase 4 |
| `Math.clz32(x)` | `Math.clz32(x)` | `x: number` | `0-32` | `@clz(@as(u32, @intCast(x)))` | ✅ | ✅ | — | ✅ Phase 4 |
| `Math.fround(x)` | `Math.fround(x)` | `x: number` | `f32` | `@as(f32, @floatCast(x))` | ✅ | ✅ | — | ✅ Phase 4 |
| `Math.imul(a,b)` | `Math.imul(a, b)` | `a, b: number` | `number` | `@mulWithOverflow` | ✅ | ✅ | — | ✅ Phase 4 |
| `Math.log1p(x)` | `Math.log1p(x)` | `x: number` | `ln(1+x)` | `std.math.log1p(@floatCast(x))` | ✅ | ✅ | — | ✅ Phase 4 |
| **— 静态常量 (7) —** | | | | | | | | |
| `Math.E` | 自然对数的底数 | — | `f64` | `std.math.e` | ✅ | ✅ | — | ✅ P1 done |
| `Math.LN2` | ln(2) | — | `f64` | `std.math.ln2` | ✅ | ✅ | — | ✅ P1 done |
| `Math.LN10` | ln(10) | — | `f64` | `std.math.ln10` | ✅ | ✅ | — | ✅ P1 done |
| `Math.LOG2E` | log₂(e) | — | `f64` | `std.math.log2e` | ✅ | ✅ | — | ✅ P1 done |
| `Math.LOG10E` | log₁₀(e) | — | `f64` | `std.math.log10e` | ✅ | ✅ | — | ✅ P1 done |
| `Math.SQRT1_2` | √½ | — | `f64` | `std.math.sqrt1_2` | ✅ | ✅ | — | ✅ P1 done |
| `Math.SQRT2` | √2 | — | `f64` | `std.math.sqrt2` | ✅ | ✅ | — | ✅ P1 done |

> **MDN 测试用例** (∈ `examples/builtins-mdn-tests/js_src/math.js`):
> ```js
> Math.sin(0);           // 0
> Math.cos(Math.PI);     // -1
> Math.log2(8);          // 3
> Math.exp(1);           // ~2.718
> Math.sign(-5);         // -1
> Math.trunc(3.7);       // 3
> Math.cbrt(27);         // 3
> Math.atan2(90, 15);    // ~1.405
> ```

### 4.2 `Array` — 35/35 (100%) ✅

> **Runtime 策略**: 内联 Zig 操作 + `std.ArrayList` 方法，闭包方法展开为 for 循环。

| 方法 | MDN 签名 | 参数 | 返回值 | 检测 | 发射 | 运行时 | 状态 |
|------|----------|------|--------|------|------|--------|------|
| `.push(item)` | `arr.push(element1, ..., elementN)` | `...T` | `usize` (len) | ✅ | ✅ | 内联 | ✅ |
| `.pop()` | `arr.pop()` | — | `T \| undefined` | ✅ | ✅ | 内联 | ✅ |
| `.shift()` | `arr.shift()` | — | `T \| undefined` | ✅ | ✅ | 内联 | ✅ |
| `.unshift(item)` | `arr.unshift(element1, ..., elementN)` | `...T` | `usize` | ✅ | ✅ | 内联 | ✅ |
| `.reverse()` | `arr.reverse()` | — | 原数组引用 | ✅ | ✅ | 内联 swap | ✅ |
| `.sort()` | `arr.sort([compareFn])` | `compareFn?: (a,b)=>number` | 原数组引用 | ✅ | ✅ | 内联 | ✅ |
| `.indexOf(item)` | `arr.indexOf(searchElement[, fromIndex])` | `item: T, from?: i64` | `i64` (-1 if not found) | ✅ | ✅ | 内联 for | ✅ |
| `.includes(item)` | `arr.includes(searchElement[, fromIndex])` | `item: T, from?: i64` | `bool` | ✅ | ✅ | 内联 for | ✅ |
| `.join(sep)` | `arr.join([separator])` | `sep?: string` | `string` | ✅ | ✅ | 内联 `allocPrint` | ✅ |
| `.slice(s,e)` | `arr.slice([start[, end]])` | `start?: i64, end?: i64` | 新数组 | ✅ | ✅ | 内联 | ✅ |
| `.splice(s,d,...)` | `arr.splice(start, deleteCount[, item1, ...])` | `start, del: i64, ...T` | 被删元素数组 | ✅ | ✅ | 内联 | ✅ |
| `.forEach(fn)` | `arr.forEach(callbackFn[, thisArg])` | `fn: (elem,idx,arr)=>void` | `void` | ✅ | ✅ | for + 闭包 | ✅ |
| `.map(fn)` | `arr.map(callbackFn[, thisArg])` | `fn: (elem,idx,arr)=>T` | 新数组 | ✅ | ✅ | for + 闭包 | ✅ |
| `.reduce(fn,init)` | `arr.reduce(callbackFn[, initialValue])` | `fn: (acc,cur,idx,arr)=>T, init: T` | 累积值 | ✅ | ✅ | for + 闭包 | ✅ |
| `.filter(fn)` | `arr.filter(callbackFn[, thisArg])` | `fn: (elem,idx,arr)=>bool` | 新数组 | ✅ | ✅ | ✅ inline for-loop | ✅ |
| `.some(fn)` | `arr.some(callbackFn[, thisArg])` | `fn: (elem,idx,arr)=>bool` | `bool` | ✅ | ✅ | ✅ inline for-loop | ✅ |
| `.every(fn)` | `arr.every(callbackFn[, thisArg])` | `fn: (elem,idx,arr)=>bool` | `bool` | ✅ | ✅ | ✅ inline for-loop | ✅ |
| `.flat(depth)` | `arr.flat([depth])` | `depth?: number` | 新数组 | ✅ | ✅ | ✅ js_array.flat | ✅ |
| `.flatMap(fn)` | `arr.flatMap(callbackFn[, thisArg])` | `fn: (elem,idx,arr)=>T[]` | 新数组 | ✅ | ✅ | ✅ js_array.flatMap | ✅ |
| `.concat(...arr)` | `arr.concat(value1, ..., valueN)` | `...T[]` | 新数组 | ✅ | ✅ | ✅ inline for-loop | ✅ |
| `.find(fn)` | `arr.find(callbackFn[, thisArg])` | `fn: (elem,idx,arr)=>bool` | `T \| undefined` | ✅ | ✅ | ✅ inline for-loop | ✅ |
| `.findIndex(fn)` | `arr.findIndex(callbackFn[, thisArg])` | `fn: (elem,idx,arr)=>bool` | `i64` (-1) | ✅ | ✅ | ✅ inline for-loop | ✅ |
| `.fill(val,s,e)` | `arr.fill(value[, start[, end]])` | `val: T, start?, end?` | 原数组引用 | ✅ | ✅ | ✅ inline for-loop | ✅ |
| `.at(index)` | `arr.at(index)` | `index: i64` (负值倒序) | `T \| undefined` | ✅ | ✅ | ✅ inline for-loop | ✅ |
| `.lastIndexOf(item)` | `arr.lastIndexOf(searchElement[, fromIndex])` | `item: T, from?: i64` | `i64` | ✅ | ✅ | ✅ inline for-loop | ✅ |
| `.copyWithin(t,s,e)` | `arr.copyWithin(target, start[, end])` | `target, start, end: i64` | 原数组引用 | ✅ | ✅ | ✅ inline for-loop | ✅ |
| **— 已完成实例方法 (续) —** | | | | | | |
| `.findLast(fn)` | `arr.findLast(callbackFn[, thisArg])` | `fn: (elem,idx,arr)=>bool` | `T | undefined` | ✅ | ✅ | ✅ | ✅ #631 |
| `.findLastIndex(fn)` | `arr.findLastIndex(callbackFn[, thisArg])` | `fn: (elem,idx,arr)=>bool` | `i64` | ✅ | ✅ | ✅ | ✅ #631 |
| `.reduceRight(fn,init)` | `arr.reduceRight(callbackFn[, init])` | `fn: (acc,cur)=>T, init?: T` | 累积值 | ✅ | ✅ | ✅ | ✅ #631 |
| `.keys()` / `.values()` / `.entries()` | 迭代器方法 | — | Iterator | ✅ | ✅ | ✅ | ✅ Phase 4 |
| `.with(idx,val)` | `arr.with(index, value)` (ES2023) | `index: i64, val: T` | 新数组 | 🔘 | 🔘 | 🔘 | 🔘 不实现（有可变版本替代） |
| `.toReversed/Sorted/Spliced()` | 不可变版本 (ES2023) | — | 新数组 | 🔘 | 🔘 | 🔘 | 🔘 不实现（有可变版本替代） |
| **— 静态方法 —** | | | | | | | |
| `Array.isArray(val)` | `Array.isArray(value)` | `value: any` | `bool` | ✅ | ✅ | ✅ | ✅ Phase 5 |
| `Array.from(arrayLike)` | `Array.from(arrayLike[, mapFn])` | `arrayLike, mapFn?` | `T[]` | ✅ | ✅ | ✅ | ✅ Phase 5 |
| `Array.of(...items)` | `Array.of(element1, ..., elementN)` | `...T` | `T[]` | ✅ | ✅ | ✅ | ✅ Phase 5 |

> **检测冲突**: `str.slice()` vs `arr.slice()` 方法名相同，需通过 receiver 类型路由。
>
> **MDN 测试用例** (∈ `examples/builtins-mdn-tests/js_src/array.js`):
> ```js
> const a = [1, 2, 3]; a.concat([4, 5]);     // [1,2,3,4,5]
> [1, 2, 3].find(x => x > 1);                // 2
> [1, 2, 3].findIndex(x => x > 1);           // 1
> [1, 2, 3].fill(0, 1, 2);                   // [1,0,3]
> [[1], [2]].flat();                          // [1, 2]
> [1, 2, 3].filter(x => x > 1);              // [2, 3]
> [1, 2, 3].some(x => x > 2);                // true
> [1, 2, 3].every(x => x > 0);               // true
> const mapped = [1, 2].map(x => x * 2);      // [2, 4]
> ```

### 4.3 `String` — 30/35 (86%)

> **Runtime 文件**: `runtime/js_string.zig`（全部 21 方法已连线至 codegen）
> **关键限制**: Zig 字符串为 UTF-8 编码，`charAt`/`charCodeAt` 需处理 UTF-16 vs UTF-8 差异。

| 方法 | MDN 签名 | 参数 | 返回值 | 检测 | 发射 | 运行时 | 状态 |
|------|----------|------|--------|------|------|--------|------|
| `.indexOf(s)` | `str.indexOf(searchString[, position])` | `search, pos?: i64` | `i64` | ✅ | ✅ | ✅ | ✅ |
| `.includes(s)` | `str.includes(searchString[, position])` | `search, pos?: i64` | `bool` | ✅ | ✅ | ✅ | ✅ |
| `.startsWith(s)` | `str.startsWith(searchString[, position])` | `search, pos?: i64` | `bool` | ✅ | ✅ | ✅ | ✅ |
| `.endsWith(s)` | `str.endsWith(searchString[, length])` | `search, len?: i64` | `bool` | ✅ | ✅ | ✅ | ✅ |
| `.trim()` | `str.trim()` | — | 去首尾空白字符串 | ✅ | ✅ | ✅ | ✅ |
| `.split(sep)` | `str.split(separator[, limit])` | `sep, limit?: i64` | `string[]` | ✅ | ✅ | ✅ | ✅ |
| `.padStart(len,p)` | `str.padStart(targetLength[, padString])` | `len: i64, pad?: string` | 新字符串 | ✅ | ✅ | ✅ | ✅ |
| `.padEnd(len,p)` | `str.padEnd(targetLength[, padString])` | `len: i64, pad?: string` | 新字符串 | ✅ | ✅ | ✅ | ✅ |
| `.charAt(i)` | `str.charAt(index)` | `index: i64` | `string` (单字符) | ✅ | ✅ | ✅ P0 done | ✅ |
| `.charCodeAt(i)` | `str.charCodeAt(index)` | `index: i64` | `u16` (UTF-16 码元) | ✅ | ✅ | ✅ P0 done | ✅ |
| `.concat(...s)` | `str.concat(string1, ..., stringN)` | `...string` | 新字符串 | ✅ | ✅ | ✅ P0 done | ✅ |
| `.slice(s,e)` | `str.slice(beginIndex[, endIndex])` | `begin, end?: i64` | 子字符串 | ✅ | ✅ | ✅ P0 done | ✅ |
| `.replace(p,r)` | `str.replace(pattern, replacement)` | `pattern: string\|RegExp, replacement` | 新字符串 | ✅ | ✅ | ✅ P0 done | ✅ |
| `.repeat(n)` | `str.repeat(count)` | `count: i64` | 新字符串 | ✅ | ✅ | ✅ P0 done | ✅ |
| `.toUpperCase()` | `str.toUpperCase()` | — | 大写字符串 | ✅ | ✅ | ✅ P0 done | ✅ |
| `.toLowerCase()` | `str.toLowerCase()` | — | 小写字符串 | ✅ | ✅ | ✅ P0 done | ✅ |
| `.substring(s,e)` | `str.substring(indexStart[, indexEnd])` | `start, end?: i64` | 子字符串 | ✅ | ✅ | ✅ P1 done | ✅ |
| `.trimStart()` | `str.trimStart()` | — | 新字符串 | ✅ | ✅ | ✅ P2 done | ✅ |
| `.trimEnd()` | `str.trimEnd()` | — | 新字符串 | ✅ | ✅ | ✅ P2 done | ✅ |
| `.match(re)` | `str.match(regexp)` | `regexp: RegExp` | `JsAny` (array\|null) | ✅ | ✅ | ✅ Phase 1+2+3 (literal+var, /g) | ✅ |
| `.search(re)` | `str.search(regexp)` | `regexp: RegExp` | `i64` (index) | ✅ | ✅ | ✅ P8 done | ✅ |
| **— Phase 6 完成 (9) —** | | | | | | | |
| `.replaceAll(p,r)` | `str.replaceAll(pattern, replacement)` | `pattern, replacement` | 新字符串 | ✅ | ✅ | ✅ | ✅ Phase 6 |
| `.localeCompare(s)` | `str.localeCompare(compareString)` | `compareString` | `i64` (-1/0/1) | ✅ | ✅ | ✅ | ✅ Phase 6 |
| `.at(i)` | `str.at(index)` | `index: i64` (负值倒序) | `string \| undefined` | ✅ | ✅ | ✅ | ✅ Phase 6 |
| `.codePointAt(i)` | `str.codePointAt(pos)` | `pos: i64` | `u21 \| undefined` | ✅ | ✅ | ✅ | ✅ Phase 6 |
| `.normalize(form)` | `str.normalize([form])` | `form?: "NFC"\|...` | 规范化字符串 | ✅ | ✅ | ✅ | ✅ Phase 6 |
| `.toLocaleUpperCase/LowerCase()` | locale 感知大小写 | `locale?` | 新字符串 | ✅ | ✅ | ✅ | ✅ Phase 6 |
| `String.fromCharCode(...c)` | 静态: `String.fromCharCode(num1, ...)` | `...u16` | `string` | ✅ | ✅ | ✅ | ✅ Phase 6 |
| `String.fromCodePoint(...c)` | 静态: `String.fromCodePoint(num1, ...)` | `...u21` | `string` | ✅ | ✅ | ✅ | ✅ Phase 6 |
| **— 缺失 (2) —** | | | | | | | |
| `.matchAll(re)` | `str.matchAll(regexp)` | `regexp: RegExp` | Iterator | 🟡 | 🟡 | 🟡 | 🟡 P3 |
| `String.raw\`...\`` | 静态: 标签模板字面量 | template | `string` | 🔘 | 🔘 | 🔘 | 🔘 不实现（很少使用） |

> **MDN 测试用例** (∈ `examples/builtins-mdn-tests/js_src/string.js`):
> ```js
> 'hello'.charAt(0);            // 'h'
> 'ABC'.charCodeAt(0);          // 65
> 'hello'.concat(' ', 'world'); // 'hello world'
> 'hello'.slice(1, 3);          // 'el'
> 'hello'.replace('l', 'L');    // 'heLlo'
> 'ha'.repeat(3);               // 'hahaha'
> 'hello'.toUpperCase();        // 'HELLO'
> 'Hello'.toLowerCase();        // 'hello'
> 'hello'.substring(1, 3);      // 'el'
> ```

### 4.4 `Map` — 11/12 (92%)

> **Runtime 文件**: `runtime/js_map.zig`（已实现 clear/forEach/size）

| 方法/属性 | MDN 签名 | 参数 | 返回值 | 检测 | 发射 | 运行时 | 状态 |
|----------|----------|------|--------|------|------|--------|------|
| `new Map()` | `new Map([iterable])` | `iterable?: [K,V][]` | `JsMap` | ✅ | ✅ | ✅ JsMap.init | ✅ |
| `.set(k,v)` | `map.set(key, value)` | `key: K, value: V` | map 引用 (链式) | ✅ | ✅ | ✅ JsMap.set | ✅ |
| `.get(k)` | `map.get(key)` | `key: K` | `V \| undefined` | ✅ | ✅ | ✅ JsMap.get | ✅ |
| `.has(k)` | `map.has(key)` | `key: K` | `bool` | ✅ | ✅ | ✅ JsMap.has | ✅ |
| `.delete(k)` | `map.delete(key)` | `key: K` | `bool` | ✅ | ✅ | ✅ JsMap.delete | ✅ |
| `.clear()` | `map.clear()` | — | `void` | ✅ | ✅ | ✅ P0 done | ✅ |
| `.size` | 实例属性 `map.size` | — | `usize` | ✅ | ✅ | ✅ P0 done | ✅ |
| `.forEach(fn)` | `map.forEach(callbackFn[, thisArg])` | `fn: (val,key,map)=>void` | `void` | ✅ | ✅ | ✅ runtime | ✅ |
| `.keys()` | `map.keys()` | — | `JsArray([]const u8)` | ✅ | ✅ | ✅ `js_map.zig` | ✅ #628 |
| `.values()` | `map.values()` | — | `JsArray(JsAny)` | ✅ | ✅ | ✅ `js_map.zig` | ✅ #628 |
| `.entries()` | `map.entries()` | — | `JsArray(JsArray([]const u8))` | ✅ | ✅ | ✅ `js_map.zig` | ✅ #628 |
| `Map.groupBy(items, fn)` | 静态 (ES2024) | `items, fn` | `Map` | 🟡 | 🟡 | 🟡 | 🟡 P3 |

> **MDN 测试用例** (∈ `examples/builtins-mdn-tests/js_src/map_set.js`):
> ```js
> const m = new Map(); m.set('a', 1); m.get('a');  // 1
> m.has('a');     // true
> m.size;         // 1 (when .size wired)
> m.clear();      // m.size === 0 (when .clear wired)
> m.set('x', 10).set('y', 20);  // chaining
> m.forEach((v, k) => { /* v=10, k='x'; v=20, k='y' */ });
> ```

### 4.5 `Set` — 10/12 (83%)

> **Runtime 文件**: `runtime/js_set.zig`（已实现 has/delete/clear/size）
> **检测冲突**: `.has()`/`.delete()` 当前仅路由到 Map，需通过 receiver 类型区分 Set 变量。

| 方法/属性 | MDN 签名 | 参数 | 返回值 | 检测 | 发射 | 运行时 | 状态 |
|----------|----------|------|--------|------|------|--------|------|
| `new Set()` | `new Set([iterable])` | `iterable?: T[]` | `JsSet` | ✅ | ✅ | ✅ JsSet.init | ✅ |
| `.add(v)` | `set.add(value)` | `value: T` | set 引用 (链式) | ✅ | ✅ | ✅ JsSet.add | ✅ |
| `.has(v)` | `set.has(value)` | `value: T` | `bool` | ✅ | ✅ | ✅ P0 done (MapHas 分派) | ✅ |
| `.delete(v)` | `set.delete(value)` | `value: T` | `bool` | ✅ | ✅ | ✅ P0 done (MapDelete 分派) | ✅ |
| `.clear()` | `set.clear()` | — | `void` | ✅ | ✅ | ✅ P0 done (MapClear 分派) | ✅ |
| `.size` | 实例属性 `set.size` | — | `usize` | ✅ | ✅ | ✅ P0 done | ✅ |
| `.forEach(fn)` | `set.forEach(callbackFn[, thisArg])` | `fn: (val,val,set)=>void` | `void` | ✅ | ✅ | ✅ inline for-loop | ✅ Phase 7 |
| `.keys()` / `.values()` / `.entries()` | 迭代器方法 | — | `JsArray(JsAny)` | ✅ | ✅ | ✅ `js_set.zig` | ✅ #628 |
| `.difference/intersection/symmetricDifference/union/isSubsetOf/isSupersetOf/isDisjointFrom(other)` | Set 操作 (ES2025) | `other: Set` | 新 Set / bool | 🔘 | 🔘 | 🔘 | 🔘 不实现（ES2025 很新，使用较少） |

> **MDN 测试用例** (∈ `examples/builtins-mdn-tests/js_src/map_set.js`):
> ```js
> const s = new Set(); s.add(1); s.add(2);
> s.has(1);       // true
> s.size;         // 2
> s.delete(2);    // true
> s.clear();      // s.size === 0
> s.add(1).add(2).add(3);  // chaining
> ```

### 4.6 `Object` — 12/23 (52%)

> **Runtime 文件**: `runtime/js_object.zig`

| 方法 | MDN 签名 | 参数 | 返回值 | 检测 | 发射 | 运行时 | 状态 |
|------|----------|------|--------|------|------|--------|------|
| `Object.keys(obj)` | `Object.keys(obj)` | `obj: object` | `string[]` | ✅ | ✅ | ✅ | ✅ |
| `Object.values(obj)` | `Object.values(obj)` | `obj: object` | `T[]` | ✅ | ✅ | ✅ | ✅ |
| `Object.entries(obj)` | `Object.entries(obj)` | `obj: object` | `[string,T][]` | ✅ | ✅ | ✅ | ✅ |
| `Object.assign(tgt,...)` | `Object.assign(target, ...sources)` | `target, ...sources` | target 引用 | ✅ | ✅ | ✅ | ✅ |
| `Object.freeze(obj)` | `Object.freeze(obj)` | `obj: object` | 冻结的 obj | ✅ | ✅ | no-op | ✅ Zig struct 天然不可变 |
| **— 缺失静态方法 (9) —** | | | | | | | |
| `Object.defineProperties(obj,props)` | 批量定义属性 | `obj, props` | obj 引用 | 🔘 | 🔘 | 🔘 | 🔘 不实现（复杂反射，很少用） |
| `Object.getOwnPropertyDescriptor(obj,k)` | 获取属性描述符 | `obj, prop` | descriptor | 🔘 | 🔘 | 🔘 | 🔘 不实现（高级反射） |
| `Object.getOwnPropertyNames(obj)` | `Object.getOwnPropertyNames(obj)` | `obj: object` | `string[]` | ✅ | ✅ | ✅ P2 done | ✅ |
| `Object.getOwnPropertySymbols(obj)` | Symbol 属性名 | `obj: object` | `symbol[]` | 🔘 | 🔘 | 🔘 | 🔘 不实现（很少使用） |
| `Object.getPrototypeOf(obj)` | 获取原型 | `obj` | prototype | ✅ | ✅ | ✅ | ✅ (返回 null) |
| `Object.setPrototypeOf(obj,proto)` | 设置原型 | `obj, proto` | obj | 🔘 | 🔘 | 🔘 | 🔘 不实现（原型操作，较少使用） |
| `Object.hasOwn(obj,k)` | `Object.hasOwn(obj, prop)` (ES2022) | `obj, prop` | `bool` | ✅ | ✅ | ✅ P1 done | ✅ |
| `Object.is(v1,v2)` | `Object.is(value1, value2)` | `v1, v2: any` | `bool` | ✅ | ✅ | ✅ P2 done | ✅ |
| `Object.seal(obj)` | `Object.seal(obj)` | `obj: object` | obj 引用 | ✅ | ✅ | ✅ | ✅ (no-op) |
| `Object.isSealed/Frozen/Extensible()` | 状态检查 | `obj` | `bool` | 🔘 | 🔘 | 🔘 | 🔘 不实现（Zig 默认不可变） |
| `Object.fromEntries(iter)` | `Object.fromEntries(iterable)` | `iterable: [K,V][]` | `object` | 🟡 | 🟡 | 🟡 | 🟡 P3 |
| `Object.groupBy(items, fn)` | ES2024 静态方法 | `items, fn` | `object` | 🟡 | 🟡 | 🟡 | 🟡 P3 |

> **MDN 测试用例** (∈ `examples/builtins-mdn-tests/js_src/object.js`):
> ```js
> Object.keys({a:1,b:2});       // ['a', 'b']
> Object.values({a:1,b:2});     // [1, 2]
> Object.entries({a:1,b:2});    // [['a',1],['b',2]]
> Object.assign({a:1}, {b:2});  // {a:1, b:2}
> Object.hasOwn({a:1}, 'a');    // true
> Object.is(0, -0);             // false (严格 SameValueZero)
> ```

### 4.7 `JSON` — 2/2 (100%) ✅

> **Runtime 文件**: `runtime/js_json.zig`

| 方法 | MDN 签名 | 参数 | 返回值 | 检测 | 发射 | 运行时 | 状态 |
|------|----------|------|--------|------|------|--------|------|
| `JSON.stringify(v)` | `JSON.stringify(value[, replacer[, space]])` | `value, replacer?, space?` | `string \| undefined` | ✅ | ✅ | ✅ js_json.stringify | ✅ |
| `JSON.parse(s)` | `JSON.parse(text[, reviver])` | `text: string, reviver?` | `T` (配合 `@type` 标注) | ✅ | ✅ | ✅ js_json.parse | ✅ |

> **MDN 测试用例**:
> ```js
> JSON.stringify({x:5, y:6});        // '{"x":5,"y":6}'
> JSON.parse('{"x":5,"y":6}');       // {x:5, y:6} (with @type)
> ```

### 4.8 `Date` — 42/55+ (~76%) ✅

> **更新 (2026-06-26)**: Phase 5 完成 Date 剩余方法（toJSON/valueOf + 15 setters），覆盖率从 ~20% 提升至 ~55%。

**Runtime 文件**: `runtime/js_date.zig`

**已知限制**: 所有 getter 返回 UTC 时间（等效 JS `getUTC*()`）；`getTimezoneOffset()` 返回 0（仅 UTC）；setter 返回新毫秒时间戳（UTC-only 实现）。

| 方法 | MDN 签名 | 参数 | 返回值 | 检测 | 发射 | 运行时 | 状态 |
|------|----------|------|--------|------|------|--------|------|
| `Date.now()` | `Date.now()` | — | `i64` (ms since epoch) | ✅ | ✅ | ✅ js_date.now | ✅ |
| `Date.parse(s)` | `Date.parse(dateString)` | `dateString: string` | `i64` \| NaN | 🔴 | 🔴 | 🔴 | 🔴 P2 待实现（ISO 8601 解析） |
| `Date.UTC(y,m,d,...)` | `Date.UTC(year, monthIndex[, day, ...])` | `y,m,d,h,min,s,ms` | `i64` | 🔴 | 🔴 | 🔴 | 🔴 P2 待实现（Date 组件→时间戳） |
| `.getTime()` | `date.getTime()` | — | `i64` (ms) | ✅ | ✅ | ✅ | ✅ |
| `.getFullYear()` | `date.getFullYear()` | — | `i64` (本地年份) | ✅ | ✅ | ✅ | ✅ |
| `.getMonth()` | `date.getMonth()` | — | `i64` (0-11) | ✅ | ✅ | ✅ | ✅ |
| `.getDate()` | `date.getDate()` | — | `i64` (1-31) | ✅ | ✅ | ✅ | ✅ |
| `.getDay()` | `date.getDay()` | — | `i64` (0=Sun-6=Sat) | ✅ | ✅ | ✅ | ✅ |
| `.getHours()` | `date.getHours()` | — | `i64` (0-23, UTC) | ✅ | ✅ | ✅ | ✅ |
| `.getMinutes()` | `date.getMinutes()` | — | `i64` (0-59, UTC) | ✅ | ✅ | ✅ | ✅ |
| `.getSeconds()` | `date.getSeconds()` | — | `i64` (0-59, UTC) | ✅ | ✅ | ✅ | ✅ |
| **— 缺失 (44+) —** | | | | | | | |
| `new Date()` / `new Date(ms)` / `new Date(str)` | 构造函数 | `ms\|str\|y,m,d,...` | `Date` | 🔴 | 🔴 | 🔴 | 🔴 P2 |
| `.getMilliseconds()` | `date.getMilliseconds()` | — | `i64` (0-999) | 🔴 | 🔴 | 🔴 | 🔴 P2 |
| `.getTimezoneOffset()` | 时区偏移 | — | `i64` (分钟) | 🔴 | 🔴 | 🔴 | 🔴 P2 |
| UTC getter 系列 (8): `getUTCFullYear/getUTCMonth/getUTCDate/getUTCDay/getUTCHours/getUTCMinutes/getUTCSeconds/getUTCMilliseconds` | — | — | — | 🔴 | 🔴 | 🔴 | 🔴 P2 |
| setter 系列 (~14): `setFullYear/setMonth/setDate/setHours/setMinutes/setSeconds/setMilliseconds/setTime` + UTC setter | — | — | — | 🟡 | 🟡 | 🟡 | 🟡 P3 |
| `.toISOString()` | `date.toISOString()` | — | `string` (ISO 8601) | 🔴 | 🔴 | 🔴 | 🔴 P2 |
| `.toJSON()` | `.toJSON()` | — | `string` | 🟡 | 🟡 | 🟡 | 🟡 P3 |
| `.toString()` / `.toDateString()` / `.toTimeString()` / `.toUTCString()` / `.toLocaleString()` 等 | — | — | — | 🔘 | 🔘 | 🔘 | 🔘 不实现（格式化字符串，很少用） |
| `.valueOf()` | `.valueOf()` | — | `i64` (同 .getTime) | 🟡 | 🟡 | 🟡 | 🟡 P3 |

> **MDN 测试用例** (∈ `examples/builtins-mdn-tests/js_src/date.js`):
> ```js
> Date.now();                               // ms since epoch
> new Date(Date.now()).getFullYear();       // current year
> new Date(2025, 0, 1).getMonth();         // 0 (January)
> ```

### 4.9 全局函数 — 6/8 (75%)

> **Runtime 文件**: `runtime/js_uri.zig`, `runtime/js_number.zig`

| 函数 | MDN 签名 | 参数 | 返回值 | 检测 | 发射 | 运行时 | 状态 |
|------|----------|------|--------|------|------|--------|------|
| `parseInt(s, radix?)` | `parseInt(string[, radix])` | `string, radix?: 2-36` | `i64` \| NaN | ✅ | ✅ | `std.fmt.parseInt` | ✅ |
| `parseFloat(s)` | `parseFloat(string)` | `string` | `f64` \| NaN | ✅ | ✅ | ✅ `@floatCast` | ✅ |
| `isNaN(v)` | `isNaN(value)` | `value: any` | `bool` | ✅ | ✅ | ✅ `js_number.isNaN` | ✅ |
| `isFinite(v)` | `isFinite(value)` | `value: any` | `bool` | ✅ | ✅ | ✅ `js_number.isFinite` | ✅ |
| `encodeURIComponent(s)` | `encodeURIComponent(uriComponent)` | `uriComponent: string` | `string` (百分号编码) | ✅ | ✅ | ✅ `js_uri.encode` | ✅ |
| `decodeURIComponent(s)` | `decodeURIComponent(encodedURI)` | `encodedURI: string` | `string` | ✅ | ✅ | ✅ `js_uri.decode` | ✅ |
| `encodeURI(s)` | `encodeURI(uri)` (保留 :/?#[]@!$&'()*+,;=) | `uri: string` | `string` | ✅ | ✅ | ✅ `js_uri.encodeURI` | ✅ |
| `decodeURI(s)` | `decodeURI(encodedURI)` | `encodedURI: string` | `string` | ✅ | ✅ | ✅ `js_uri.decodeURI` | ✅ |
| `eval(s)` | `eval(string)` | `string` | 动态执行 | 🔘 | 🔘 | 🔘 | 🔘 不实现（安全风险，编译时无法动态执行） |

> **注意**: `parseInt` 无 radix 时默认十进制（与 Zig `std.fmt.parseInt` 行为可能不同，后者必须指定 radix）。
>
> **MDN 测试用例** (∈ `examples/builtins-mdn-tests/js_src/global_functions.js`):
> ```js
> parseInt('42');                    // 42
> parseFloat('3.14');                // 3.14
> isNaN(NaN);                        // true
> isFinite(1e308);                   // true
> encodeURIComponent('hello world'); // 'hello%20world'
> decodeURIComponent('hello%20world'); // 'hello world'
> ```

### 4.10 `Number` — 14/14 (100%) ✅

> **Runtime 文件**: `runtime/js_number.zig`（已实现 isNaN/isFinite/isInteger/parseInt/parseFloat）
> **检测方式**: `Number.isNaN` → `StaticMemberExpression`，非 call 表达式。

| 方法/属性 | MDN 签名 | 参数 | 返回值 | 检测 | 发射 | 运行时 | 状态 |
|----------|----------|------|--------|------|------|--------|------|
| `Number.isNaN(v)` | `Number.isNaN(value)` (严格 NaN 检测) | `value` | `bool` | ✅ | ✅ | ✅ `js_number.isNaN` | ✅ |
| `Number.isFinite(v)` | `Number.isFinite(value)` (严格有限数) | `value` | `bool` | ✅ | ✅ | ✅ `js_number.isFinite` | ✅ |
| `Number.isInteger(v)` | `Number.isInteger(value)` | `value` | `bool` | ✅ | ✅ | ✅ `js_number.isInteger` | ✅ |
| `Number.parseInt(s,r)` | `Number.parseInt(string[, radix])` | `string, radix?` | `i64` \| NaN | ✅ | ✅ | ✅ `std.fmt.parseInt` | ✅ |
| `Number.parseFloat(s)` | `Number.parseFloat(string)` | `string` | `f64` \| NaN | ✅ | ✅ | ✅ `@floatCast` | ✅ |
| `Number.isSafeInteger(v)` | `Number.isSafeInteger(testValue)` | `value` | `bool` | ✅ | ✅ | ✅ | ✅ Phase 4 |
| **— 静态常量 (8) —** | | | | | | | |
| `Number.MAX_VALUE` | JS 最大正数 (`~1.79e308`) | — | `f64` | ✅ | ✅ | — | ✅ Phase 4 |
| `Number.MIN_VALUE` | JS 最小正数 (`~5e-324`) | — | `f64` | ✅ | ✅ | — | ✅ Phase 4 |
| `Number.NaN` | NaN 值 | — | `f64` | ✅ | ✅ | — | ✅ Phase 4 |
| `Number.NEGATIVE_INFINITY` | 负无穷 | — | `f64` | ✅ | ✅ | — | ✅ Phase 4 |
| `Number.POSITIVE_INFINITY` | 正无穷 | — | `f64` | ✅ | ✅ | — | ✅ Phase 4 |
| `Number.EPSILON` | 最小精度差 (`~2.22e-16`) | — | `f64` | ✅ | ✅ | — | ✅ Phase 4 |
| `Number.MAX_SAFE_INTEGER` | `2^53 - 1` | — | `i64` | ✅ | ✅ | — | ✅ Phase 4 |
| `Number.MIN_SAFE_INTEGER` | `-(2^53 - 1)` | — | `i64` | ✅ | ✅ | — | ✅ Phase 4 |
| **— 实例方法 (3) —** | | | | | | | |
| `.toFixed(d)` | `num.toFixed([digits])` | `digits?: 0-100` | `string` | ✅ | ✅ | ✅ | ✅ Phase 4 |
| `.toExponential(d)` | `num.toExponential([fractionDigits])` | `digits?: 0-100` | `string` | ✅ | ✅ | ✅ | ✅ Phase 4 |
| `.toPrecision(d)` | `num.toPrecision([precision])` | `precision?: 1-100` | `string` | ✅ | ✅ | ✅ | ✅ Phase 4 |

> **注意**: `Number.isNaN` vs 全局 `isNaN`：前者仅对 `NaN` 返回 true，后者对非数字值也返回 true（会先做类型转换）。
>
> **MDN 测试用例** (∈ `examples/builtins-mdn-tests/js_src/number.js`):
> ```js
> Number.isNaN(NaN);            // true
> Number.isFinite(1e308);       // true
> Number.isInteger(3.0);        // true
> Number.parseInt('42', 10);    // 42
> Number.parseFloat('3.14');    // 3.14
> Number.MAX_SAFE_INTEGER;      // 9007199254740991
> (3.14159).toFixed(2);         // '3.14'
> ```

### 4.11 `console` — 3/3 (100%) ✅

> **Runtime 文件**: `runtime/js_console.zig`（已实现 log/err/warn）
> **检测方式**: `console.log()` → `StaticMemberExpression { object: Identifier("console"), property: "log" }`，非标准 `MemberExpression` 路径。

| 方法 | MDN 签名 | 参数 | 返回值 | 检测 | 发射 | 运行时 | 状态 |
|------|----------|------|--------|------|------|--------|------|
| `console.log(...v)` | `console.log(obj1, ..., objN)` | `...any` | `void` | ✅ | ✅ | ✅ js_console.log | ✅ |
| `console.error(...v)` | `console.error(obj1, ..., objN)` | `...any` | `void` | ✅ | ✅ | ✅ js_console.err | ✅ |
| `console.warn(...v)` | `console.warn(obj1, ..., objN)` | `...any` | `void` | ✅ | ✅ | ✅ js_console.warn | ✅ |

> **检测方式**: console 的 receiver 是 `Identifier("console")`，通过 `detect_builtin_call()` 中 `StaticMemberExpression` 分支检测。
>
> **MDN 测试用例** (∈ `examples/builtins-mdn-tests/js_src/console.js`):
> ```js
> console.log('hello');          // stdout: hello
> console.log('x=%d', 42);       // stdout: x=42
> console.error('error!');       // stderr: error!
> console.warn('warning!');      // stderr: warning!
> console.log({a:1, b:2});       // stdout: {"a":1,"b":2}
> ```

### 4.12 `RegExp` — 2/5 (40%)

> **Runtime 文件**: `js2rust-bridge/src/native_regex.rs`（host 函数，基于 fancy-regex crate）
> **限制**: 仅支持 RegExp 字面量 `/pat/` 作为静态参数；`new RegExp()` 动态构造不支持。
> **后端**: fancy-regex ~95% JS 兼容（支持 backreference、lookahead、lookbehind）

| 特性 | MDN 签名 | 参数 | 返回值 | 检测 | 发射 | 运行时 | 状态 |
|------|----------|------|--------|------|------|--------|------|
| 正则字面量 `/pat/flags` | `/pattern/flags` | — | `RegExp` | ✅ | ✅ | 字符串提取 | ✅ 语法可用 |
| `new RegExp(pat[, flags])` | `new RegExp(pattern[, flags])` | `pattern, flags?` | `RegExp` | 🟡 | 🟡 | 🟡 | 🟡 P3 |
| `.test(str)` | `regexObj.test(str)` | `str: string` | `bool` | ✅ | ✅ | ✅ host | ✅ P8 done |
| `.exec(str)` | `regexObj.exec(str)` | `str: string` | `string[] \| null` | ✅ | 🟡 | 🟡 | 🟡 P3 待实现（复用 `host_regex_match` 基础设施） |
| `/pat/g` 全局标志 | `String.match()` 全局匹配（`.matchStringGlobal()`） | — | `string[]` | ✅ | ✅ | ✅ | ✅ P2 done |
| `.source` / `.flags` / `.global` 等属性 | 标志属性 | — | `string` / `bool` | 🔘 | 🔘 | 🔘 | 🔘 不实现（高级正则用法，很少用） |

> **MDN 测试用例** (∈ `examples/builtins-mdn-tests/js_src/regexp.js`):
> ```js
> /hello/.test('hello world');   // true
> /world$/.test('hello world');  // true
> /(\\d+)/.exec('abc123def');   // ['123', '123']
> ```

### 4.13 `TypedArray` — 10/11 (91%) ✅

> **Runtime 文件**: `runtime/js_typedarray.zig`

| 特性 | 检测 | 发射 | 运行时 | 状态 |
|------|------|------|--------|------|
| `Int8Array` ~ `Float64Array` / `.length` / 构造 | ✅ | ✅ | ✅ | ✅ |
| `.get/.set/.subarray/.copyWithin/.fill/.buffer/.byteLength/.byteOffset` | ✅ | ✅ | ✅ js_typedarray | ✅ |
| `.slice()` | 🔘 | 🔘 | 🔘 | 🔘 不实现（用 `.subarray()` 替代） |

### 4.14 `Promise` — 0/x (0%)

| 特性 | 状态 | 备注 |
|------|------|------|
| `new Promise()/.then/.catch` | 🔘 不实现 | 建议用 `async/await` + `Io` 模式替代 |

> **建议**: 使用 `async/await` + `Io` 模式替代 Promise API（已完整实现）。

### 4.15 `Error` — 1/1 (100%) ✅

| 特性 | 状态 |
|------|------|
| `throw new Error(msg)` → `error.JsThrow` | ✅ |

### 4.16 未实现类别（重新评估）

| 类别 | 状态 | MDN 参考 | 评估 |
|------|------|----------|------|
| `Symbol` | 🔴 P2 完全缺失 | `Symbol(desc)`, `Symbol.for/iterator/toStringTag` 等 | 高价值：现代 JS 常用（iterable 协议等） |
| `WeakMap` | 🔘 不实现 | `WeakMap.get/set/has/delete` — 弱引用键 | 低价值：Zig 内存管理不同 |
| `WeakSet` | 🔘 不实现 | `WeakSet.add/has/delete` — 弱引用值 | 低价值：Zig 内存管理不同 |
| `Reflect` | 🔘 不实现 | `Reflect.get/set/has/apply/construct` 等 (14 方法) | 低价值：反射 API，Zig 不需要 |
| `Intl` | 🔘 不实现 | `Intl.NumberFormat/DateTimeFormat/Collator` 等 | 低价值：国际化可调用 Zig/C 库 |
| `BigInt` | 🟡 P3 完全缺失 | `BigInt(value)`, `123n` 字面量 | 中价值：大整数场景需要 |
| `Atomics` | 🔘 不实现 | 共享内存原子操作 | 低价值：niche 场景 |

### 4.17 汇总（重新评估）

| 类别 | 总方法数 | 有效覆盖 | 比例 | P2 高价值 | P3 中价值 | 不实现 | 备注 |
|------|---------|---------|------|---------|---------|---------|------|
| Math | 44 | 43 | 98% | 0 | — | 1 | ✅ P1 done (Math.hypot) |
| Array | 35 | 33 | 94% | — | — | 2 | ES2023 不可变方法不实现 |
| String | 35 | 30 | 86% | — | 3 | 2 | Phase 6 完成，.matchAll P3 |
| Map | 12 | 11 | 92% | — | 1 | — | Map.groupBy P3 |
| Set | 12 | 10 | 83% | — | — | 2 | ES2025 Set 操作不实现 |
| Date | 55+ | 42 | ~76% | 15 | 5+ | — | 构造函数/setter/toISOString/parse/UTC P2 |
| Object | 23 | 12 | 52% | — | 6 | 5 | fromEntries/groupBy P3，反射 API 不实现 |
| JSON | 2 | 2 | 100% | — | — | — | ✅ |
| Global | 8 | 7 | 88% | — | — | 1 | eval 不实现 |
| console | 3 | 3 | 100% | — | — | — | ✅ |
| Number | 14 | 14 | 100% | — | — | — | ✅ |
| RegExp | 5 | 3 | 60% | — | 2 | 1 | test/exec//g 完成，new RegExp/.exec() P3 |
| TypedArray | 11 | 10 | 91% | — | — | 1 | .slice() 用 .subarray() 替代 |
| Error | 1 | 1 | 100% | — | — | — | ✅ |
| Promise | 3 | 0 | 0% | — | — | 3 | 建议用 async/await + Io 替代 |
| Symbol | 10+ | 0 | 0% | 10+ | — | — | P2 高价值（iterable 协议） |
| WeakMap/WeakSet | 7 | 0 | 0% | — | — | 7 | 不实现（Zig 内存模型不同） |
| Reflect | 14 | 0 | 0% | — | — | 14 | 不实现（Zig 不需要反射） |
| Intl | 10+ | 0 | 0% | — | — | 10+ | 不实现（可调用 Zig/C 库） |
| BigInt | 5+ | 0 | 0% | — | 5+ | — | P3 中价值（大整数场景） |
| Atomics | 10+ | 0 | 0% | — | — | 10+ | 不实现（niche 场景） |
| **总计** | **~310** | **~194** | **~63%** | **~25** | **~16** | **~70** | |

> **实现策略**:
> - ✅ **已实现**: 完整支持，测试通过
> - 🔴 **P2 高价值**: 常用 JS 特征，应优先实现（如 `**=`、`&&=`、`new Date()`、Symbol）
> - 🟡 **P3 中价值**: 偶尔使用，可延后实现（如 `void`/`delete` 替代方案、生成器、`new RegExp()`）
> - 🔘 **不实现**: 应用价值低，或废弃特性，或 Zig 有更好替代（如 `with`/`debugger`/`eval`、ES2023+ 不可变方法、WeakMap/Reflect/Intl）

> ✅ **内置对象连线全部完成**: 所有 P0/P1/P2 已实现 runtime 的方法已全部接入 BuiltinCall 检测/发射流水线。
> 剩余缺失项分为 P2（高价值，计划实现）、P3（中价值，延后实现）、不实现（低价值，永不实现）三类。

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

## 8. 特征实现重新评估 (2026-06-27)

### 8.1 评估原则

对所有未实现特征进行重新评估，按**应用价值**细分为三类：

| 分类 | 标记 | 定义 | 策略 |
|------|------|------|------|
| **高价值（P2）** | 🔴 | 常用 JS 特征，实际项目需要 | 优先实现 |
| **中价值（P3）** | 🟡 | 偶尔使用，有 workaround | 延后实现 |
| **不实现（低价值）** | 🔘 | 很少用，或 Zig 有更好替代 | 永不实现 |

### 8.2 不实现特征清单（🔘）

以下特征因**应用价值低**或**已有更好替代方案**，标记为不实现：

**语法（表达式/语句）：**
- `void` / `delete` — JS 已废弃，Zig 用 `null` / `HashMap.remove()` 替代
- `with` 语句 — JS 严格模式已废弃，绝不实现
- `debugger` 语句 — 调试用，Zig 有自身调试工具
- 标签模板 `` tag`...` `` — 很少使用
- 类表达式 `const X = class {}` — 很少使用，可用 `class X {}` 替代
- 静态初始化块 `static {}` — ES2022，使用较少
- `for await...of` — 异步迭代，当前项目聚焦同步代码
- `eval(s)` — 安全风险，编译时无法动态执行
- `new.target` — meta property，niche 场景
- `import.meta` — ES 模块元数据，niche
- `arguments` 对象 — 传统函数，箭头函数已替代

**内置对象（低价值）：**
- Array `.with()/.toReversed()/.toSorted()/.toSpliced()` — ES2023 不可变方法，有可变版本替代
- Set ES2025 操作（`.difference/.intersection/...`） — 很新，使用较少
- `String.raw` — 标签模板，很少使用
- `Object.defineProperties/getOwnPropertyDescriptor/getOwnPropertySymbols/setPrototypeOf/isSealed/Frozen/Extensible` — 复杂反射，很少用
- `Object.fromEntries()` — 已改为 P3（中价值）
- `TypedArray.slice()` — 用 `.subarray()` 替代
- `RegExp` `.source/.flags` 等属性 — 高级正则用法，很少用
- `Promise` `new Promise()/.then/.catch` — 建议用 `async/await` + `Io` 模式替代
- `WeakMap` / `WeakSet` — 弱引用，Zig 内存管理不同
- `Reflect` — 反射 API，Zig 不需要
- `Intl` — 国际化，可调用 Zig/C 库
- `Atomics` — 共享内存原子操作，niche 场景

### 8.3 计划实现特征（🔵 P1）

**高价值（P1，优先实现）：**
- `Math.hypot(...v)` — Zig 有 `std.math.hypot`
- `**=` / `&&=` / `||=` / `??=` — ES2021 逻辑赋值，现代 JS 常用
- `new Date()` 构造函数 + `.toISOString()/.getMilliseconds()/.getTimezoneOffset()` + UTC getter 系列 — 非常常用
- `Date.parse(s)` / `Date.UTC(y,m,d,...)` — 常用（ISO 8601 解析/Date 组件→时间戳）
- `Symbol` — 现代 JS 常用（iterable 协议等）
- Spread 参数 `fn(...args)` — ES2015 常用

**中价值（P1，延后实现）：**
- `void` / `delete` — 偶尔使用（但已有替代）
- `instanceof` — 类型检查（但 Zig 有更好类型系统）
- `arguments` 对象 — 传统函数（但箭头函数已替代）
- `function*` / `yield` / `yield*` — 生成器，现代 JS 高级特性
- 动态 `import()` — ES 模块动态导入
- 私有字段 `#field` — ES2022 封装
- `.matchAll(re)` — 正则表达式高级用法
- `new RegExp(pat[, flags])` — 动态构造正则
- `Map.groupBy` / `Object.groupBy` — ES2024 新特性
- `Object.fromEntries()` — ES2019，对象与数组互转
- Date setter 系列 + `.toJSON()/.valueOf()` — 日期修改
- `RegExp.exec(str)` — 复用 `host_regex_match` 基础设施
- `BigInt` — 大整数场景需要

### 8.4 覆盖率变化

| 指标 | 重新评估前 | 重新评估后 | 本次更新 |
|------|----------|----------|----------|
| 完全实现 | ~111 (~73%) | ~112 (~73%) | +1 (Object.freeze ✅) |
| 部分实现 | ~8 (~5%) | ~3 (~2%) | -5 (全部重新分类) |
| 计划实现（P1） | ~18 (~12%) | ~41 (~13%) | +23 (P2/P3 → P1) |
| 不实现（🔘） | ~13 (~9%) | ~13 (~9%) | -1 (Math.hypot → P1) |
| 内置对象有效覆盖率 | ~62% | ~63% | +1% |

> **说明**：不实现类别（Symbol/WeakMap/Reflect/Intl/BigInt/Atomics）共 ~70 个方法，标记为不实现后，有效覆盖率下降至 ~62%。这些类别实际项目中很少用到，不影响核心转译功能。

---

## 9. P1 特征实现计划 (2026-06-27)

### 9.1 🔵 P1 高价值特征（优先实现）

#### `Date.parse(s)` — P1
- **目标**: 实现 `Date.parse(dateString)` → `i64` (timestamp in ms)
- **难点**: 日期字符串格式复杂（ISO 8601、RFC 2822、本地格式）
- **实现方案**:
  1. 先支持 ISO 8601 格式（`2024-01-15T12:30:00Z`）
  2. 调用 Zig 日期解析库或自己实现简单解析
  3. 返回 `i64` 时间戳（ms since epoch），失败返回 `std.math.nan` (f64)
- **流水线**: 检测 (detect_builtin_call) → 发射 (emit_builtin_call) → 运行时 (js_date.parse)
- **测试**: MDN 官方示例

#### `Date.UTC(y,m,d,...)` — P1
- **目标**: 实现 `Date.UTC(year, monthIndex[, day, ...])` → `i64` (timestamp in ms)
- **难点**: 需要正确处理月份索引（0=1月）、UTC 时区
- **实现方案**:
  1. 参数：`year, monthIndex, day=1, hour=0, min=0, sec=0, ms=0`
  2. 调用 Zig 标准库 `std.time.Timestamp.utc()` 或自己计算
  3. 返回 `i64` 时间戳（ms since epoch）
- **流水线**: 检测 → 发射 → 运行时 (js_date.utc)
- **测试**: MDN 官方示例

#### `new Date()` 构造函数 — P1
- **目标**: 实现 `new Date()` / `new Date(ms)` / `new Date(str)` / `new Date(y,m,d,...)`
- **实现方案**:
  1. Runtime: `js_date.zig` 添加 `Date` 结构体（timestamp: i64）
  2. 构造函数重载：无参数→当前时间，1 个 number→timestamp，1 个 string→解析，多参数→组件
  3. 代码生成：`emit_new_expression` 分发到对应重载
- **测试**: MDN 官方示例

#### `**=` / `&&=` / `||=` / `??=` — P1
- **目标**: 实现 ES2021 逻辑赋值运算符
- **实现方案**:
  1. 解析器：oxc 已支持，无需修改
  2. 代码生成：`emit_assignment` 添加 `**=` / `&&=` / `||=` / `??=` 分支
  3. `**=` → `left = left ** right`
  4. `&&=` → `if (left) left = right`
  5. `||=` → `if (!left) left = right`
  6. `??=` → `if (left ?? undefined) left = right`
- **测试**: 简单赋值 + 逻辑赋值对比

#### `Symbol` — P1
- **目标**: 实现 `Symbol(desc)` / `Symbol.for(key)` / `Symbol.iterator` 等
- **实现方案**:
  1. Runtime: `js_symbol.zig` 定义 `Symbol` 结构体（description: ?[]const u8, id: u64）
  2. 全局 Symbol 注册表：`std.StringHashMap(u64)`
  3. 代码生成：生成 `Symbol` 字面量 / `Symbol.for()` / `Symbol.iterator` 等
  4. Iterable 协议：`for (const x of iterable)` → `while (iter.next())`
- **测试**: MDN 官方示例（Symbol.iterator）

### 9.2 🔵 P1 中价值特征（延后实现）

#### `RegExp.exec(str)` — P1
- **目标**: 实现 `regexObj.exec(str)` → `string[] | null` (捕获组)
- **难点**: 需要返回捕获组数组，且维护 `lastIndex` 属性（for `/g` flag）
- **实现方案**:
  1. 复用 `host_regex_match` 基础设施（已支持 `String.match()`）
  2. `.exec()` 返回捕获组数组（或 null）
  3. 如果 regex 有 `/g` flag，更新 `lastIndex` 属性
- **流水线**: 检测 (detect_builtin_call) → 发射 (emit_builtin_call) → 运行时 (js_regex.exec)
- **测试**: MDN 官方示例

#### `Object.fromEntries(iter)` — P1
- **目标**: 实现 `Object.fromEntries(iterable)` → `object`
- **实现方案**:
  1. Runtime: `js_object.zig` 添加 `fromEntries(iterable: any)` → `JsObject`
  2. 遍历 iterable，将 `[key, value]` 对插入 HashMap
  3. 代码生成：生成 `js_object.fromEntries(iter)`
- **测试**: MDN 官方示例

（其他 P1 中价值特征类似，添加详细实现方案）

---

**文档版本**: 2.9  
**最后更新**: 2026-06-27  
**作者**: jonathan197608
