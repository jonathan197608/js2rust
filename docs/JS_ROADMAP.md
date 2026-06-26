# JS2Zig 项目路线图与任务跟踪

> **项目**: js2rust/js2zig (JS → Zig 转译器)
> **创建日期**: 2026-06-21
> **最后更新**: 2026-06-25
> **维护者**: jonathan197608

---

## 当前状态

项目 Phase 5 已完成，进入 Phase 6（String 高级方法，排除正则相关）。221 测试通过，0 clippy 警告。

**✅ 2026-06-24 内置对象补齐完成**: 有效覆盖率从 ~22% 提升至 ~53%（~138/260）。P0/P1/P2/P3(Phase 3) 共 ~58 个方法全部接入 BuiltinCall 检测/发射流水线。
**✅ 2026-06-25 Class 隐式字段推断 + codegen 审计完成**: #613-625 全部完成，206 测试通过。
**✅ 2026-06-25 #628 Map/Set 迭代器完成**: `JsSet` 重构为 `std.HashMap(JsAny, void, JsAnyHashMapContext, ...)`（SameValueZero 语义），`set.keys()/values()/entries()` 类型推断和 codegen 全部接通，24 个 Zig 测试通过。
**✅ 2026-06-26 Phase 4 全部完成**: #626-#639 共 14 个任务全部完成（含 #639 stub），覆盖率提升至 ~65%（~170/260）。
- #626 Number.toExponential/toPrecision ✅
- #627 String.at ✅
- #628 Map/Set 迭代器 ✅
- #629 Date.toString/toDateString/toTimeString ✅
- #630 String.codePointAt ✅
- #631 Array.findLast/findLastIndex/reduceRight ✅
- #632 String.replaceAll/fromCharCode/fromCodePoint ✅
- #633 Math 高级方法（已全部实现）✅
- #634 Number 常量 ✅
- #635 Array.keys/values/entries ✅
- #636 Date constructor 重载 ✅
- #637 Date UTC getter 系列 ✅
- #638 Object.keys/values/entries ✅
- #639 String.matchAll/localeCompare/normalize（stub）✅

**✅ 2026-06-26 Phase 5 完成 — Array.from/of/isArray、Object 剩余方法、Date 剩余方法**: 实现 `Array.from()/of()/isArray()`、`Object.create()/seal()/defineProperty()/getPrototypeOf()`、`Date.toJSON()/valueOf()` + 15 个 setter 方法，覆盖率提升至 ~73%（~190/260）。
**🔧 2026-06-26 Phase 6 进行中 — String 高级方法**: 
- 添加 10 个 Phase 6 测试（startsWith, endsWith, includes, slice, concat, repeat, substring, normalize, toUpperCase, toLowerCase），全部通过
- 修复 ~15 个字符串方法的 codegen（使用 `callee_object_repr()` 替代 `callee_object_name()`，支持字符串字面量）
- 修复 8 个字符串方法的 codegen（使用 `js_string.xxx()` 替代 `std.mem.xxx()`）：startsWith, endsWith, includes, indexOf, trim, split, charAt, at, charCodeAt, codePointAt, concat, slice, replace, replaceAll, repeat, substring
- TrimStart/TrimEnd/LastIndexOf 暂用 `std.mem`（runtime 函数待实现）
- 进度：codegen 修复完成，测试全部通过（221 tests ✅）

详细特性实现状态请参考 [JS_FEATURE_EVALUATION.md](./JS_FEATURE_EVALUATION.md)。

---

## 0. 内置对象补齐计划 (MDN 对齐版 — 2026-06-24)

> 背景: `docs/JS_FEATURE_EVALUATION.md` 第 4 节已按 MDN 标准对齐重评估。P0/P1 全部完成，有效覆盖率 ~53% (~138/260)。
> 策略: P0 连线已有 runtime ✅ → P1 补齐 Zig 内置映射 ✅ → P2 实现新 runtime → P3 修复 stub → P4 远期。
> 每个方法均对照 [MDN Global Objects](https://developer.mozilla.org/zh-CN/docs/Web/JavaScript/Reference/Global_Objects) 标准。
> 测试用例来自 MDN 官方示例，存放于 `examples/builtins-mdn-tests/js_src/`。

### `examples/builtins-mdn-tests` 项目设计

```
examples/builtins-mdn-tests/
├── js2rust.toml          # 项目配置: name="builtins-mdn-tests", type="test"
├── Cargo.toml             # Rust 依赖 js2rust-bridge
├── build.rs               # 调用 js2zig-core 转译全部 JS 文件
├── src/
│   └── main.rs            # 调用各 C ABI 导出函数并断言结果
└── js_src/
    ├── math.js            # Math 三角/对数/常量 (21 方法)
    ├── string.js          # String charAt/concat/slice/replace/toUpper/toLower/substring (10 方法)
    ├── array.js           # Array concat/find/findIndex/fill/flat/filter/some/every (9 方法)
    ├── map_set.js         # Map.clear/size/forEach + Set.has/delete/clear/size (8 方法)
    ├── console.js         # console.log/error/warn (3 方法)
    ├── number.js          # Number.isNaN/isFinite/isInteger/parseInt/parseFloat/toFixed (7 方法)
    ├── global_functions.js # parseFloat/isNaN/isFinite/encodeURIComponent/decodeURIComponent (5 方法)
    ├── date.js            # Date 构造函数/getMilliseconds/toISOString (5 方法)
    ├── object.js          # Object.hasOwn/is/getOwnPropertyNames (4 方法)
    └── regexp.js          # RegExp.test/exec (2 方法)
```

**测试策略**: 每个 JS 文件中的 `export function` 对应一个 MDN 官方示例，Rust 侧用 `assert_eq!` 验证输出匹配 MDN 文档预期结果。

---

### 0.1 Phase 0 — 快速连线 (P0, 27 方法) ✅ 已完成 (2026-06-24)

**目标**: 添加 `BuiltinCall` 变体 + `detect_builtin_call()` 匹配 + `emit_builtin_call()` 生成，调用已有 runtime 函数。覆盖率 22% → ~33%。

#### 0.1.1 `console.log/error/warn` (3 方法)

| 方法 | 检测位置 | 发射 Zig 代码 | Runtime | 测试文件 |
|------|---------|-------------|---------|---------|
| `console.log(...args)` | `detect_builtin_call()` 新增 `StaticMemberExpression { object: Identifier("console") }` 分支 | `js_console.log(.{...args})` | `js_console.log()` | `console.js` |
| `console.error(...args)` | 同上, property="error" | `js_console.err(.{...args})` | `js_console.err()` | `console.js` |
| `console.warn(...args)` | 同上, property="warn" | `js_console.warn(.{...args})` | `js_console.warn()` | `console.js` |

**注意**: console 与其他内置对象不同 — receiver 是 `Identifier("console")`，不是方法链上的对象。需在 `detect_builtin_call()` 中新增独立检测分支。

#### 0.1.2 `String` 实例方法 (8 方法)

| 方法 | BuiltinCall 变体 | 发射 Zig 代码 | Runtime 函数 | 测试文件 |
|------|-----------------|-------------|-------------|---------|
| `.toUpperCase()` | `StringToUpperCase` | `js_string.toUpper(s, alloc)` | `js_string.toUpper()` | `string.js` |
| `.toLowerCase()` | `StringToLowerCase` | `js_string.toLower(s, alloc)` | `js_string.toLower()` | `string.js` |
| `.charAt(i)` | `StringCharAt` | `js_string.charAt(s, @intCast(i))` | `js_string.charAt()` | `string.js` |
| `.charCodeAt(i)` | `StringCharCodeAt` | `js_string.charCodeAt(s, @intCast(i))` | `js_string.charCodeAt()` → `u16` | `string.js` |
| `.concat(...s)` | `StringConcat` | `js_string.concat(alloc, s, .{...})` | `js_string.concat()` | `string.js` |
| `.slice(s,e)` | `StringSlice` | `js_string.slice(s, start, end)` | `js_string.slice()` | `string.js` |
| `.replace(p,r)` | `StringReplace` | `js_string.replace(alloc, s, pattern, replacement)` | `js_string.replace()` | `string.js` |
| `.repeat(n)` | `StringRepeat` | `js_string.repeat(alloc, s, @intCast(n))` | `js_string.repeat()` | `string.js` |

**类型注意**: `charCodeAt` 返回 `u16`（UTF-16 码元），与大多数返回 `i64` 的方法不同。

#### 0.1.3 全局函数 + Number 静态方法 (5+5=10 方法)

| 方法 | BuiltinCall 变体 | 发射 Zig 代码 | Runtime | 测试文件 |
|------|-----------------|-------------|---------|---------|
| `parseFloat(s)` | `GlobalParseFloat` | `js_number.parseFloat(s)` | `js_number.parseFloat()` | `global_functions.js` |
| `isNaN(v)` | `GlobalIsNaN` | `js_number.isNaN(v)` | `js_number.isNaN()` | `global_functions.js` |
| `isFinite(v)` | `GlobalIsFinite` | `js_number.isFinite(v)` | `js_number.isFinite()` | `global_functions.js` |
| `encodeURIComponent(s)` | `GlobalEncodeURIComponent` | `try js_uri.encodeURIComponent(alloc, s)` | `js_uri.encodeURIComponent()` | `global_functions.js` |
| `decodeURIComponent(s)` | `GlobalDecodeURIComponent` | `try js_uri.decodeURIComponent(alloc, s)` | `js_uri.decodeURIComponent()` | `global_functions.js` |
| `Number.isNaN(v)` | `NumberIsNaN` | `js_number.isNaN(v)` | `js_number.isNaN()` | `number.js` |
| `Number.isFinite(v)` | `NumberIsFinite` | `js_number.isFinite(v)` | `js_number.isFinite()` | `number.js` |
| `Number.isInteger(v)` | `NumberIsInteger` | `js_number.isInteger(v)` | `js_number.isInteger()` | `number.js` |
| `Number.parseInt(s,r)` | `NumberParseInt` | `std.fmt.parseInt(i64, s, r)` | `std.fmt.parseInt` | `number.js` |
| `Number.parseFloat(s)` | `NumberParseFloat` | `js_number.parseFloat(s)` | `js_number.parseFloat()` | `number.js` |

**检测方式**: `Number.isNaN()` 是 `StaticMemberExpression { object: Identifier("Number"), property: "isNaN" }`，需在 `detect_builtin_call()` 中新增分支。`isNaN()` 是 `Identifier("isNaN")`，直接匹配全局函数名。

#### 0.1.4 `Map` / `Set` 属性访问 (4+2=6 方法)

| 方法/属性 | BuiltinCall 变体 | 发射 Zig 代码 | Runtime | 测试文件 |
|----------|-----------------|-------------|---------|---------|
| `map.clear()` | `MapClear` | `map.clear()` | `JsMap.clear()` | `map_set.js` |
| `map.size` | 属性访问（非 BuiltinCall） | `map.count()` | `JsMap.count()` | `map_set.js` |
| `set.has(v)` | `SetHas` | `set.has(v)` | `JsSet.has()` | `map_set.js` |
| `set.delete(v)` | `SetDelete` | `set.delete(v)` | `JsSet.delete()` | `map_set.js` |
| `set.clear()` | `SetClear` | `set.clear()` | `JsSet.clear()` | `map_set.js` |
| `set.size` | 属性访问 | `set.count()` | `JsSet.count()` | `map_set.js` |

**检测冲突解决**: 当前 `.has()`/`.delete()` 统一路由到 `MapHas`/`MapDelete`。需在 `detect_builtin_call()` 中增加 receiver 类型判断：
- 通过 `type_info` / `typedarray_vars` 类比，新增 `set_vars` / `map_vars` 集合
- 或统一简化为 `MapHas`/`MapDelete` 兼容两种 receiver（Map 和 Set 的 `has`/`delete` 语义相同，Zig 侧 runtime 已区分）

**Phase 0 预估**: ~27 方法，新增 ~15 个 `BuiltinCall` 变体，~120 行 codegen 检测/发射代码。

---

### 0.2 Phase 1 — 补齐简短 Runtime (P1, 21 方法, ~150-200 行) ✅ 已完成 (2026-06-24)

**目标**: Math 三角函数/对数/常量直接映射 Zig 内置函数，零额外 runtime。覆盖率 33% → ~42%。

#### 0.2.1 Math 三角函数 (7 方法)

| 方法 | BuiltinCall 变体 | Zig 发射 | 测试文件 |
|------|-----------------|----------|---------|
| `Math.sin(x)` | `MathSin` | `@sin(@as(f64, @floatFromInt(x)))` | `math.js` |
| `Math.cos(x)` | `MathCos` | `@cos(@as(f64, @floatFromInt(x)))` | `math.js` |
| `Math.tan(x)` | `MathTan` | `@tan(@as(f64, @floatFromInt(x)))` | `math.js` |
| `Math.asin(x)` | `MathAsin` | `std.math.asin(@as(f64, @floatFromInt(x)))` | `math.js` |
| `Math.acos(x)` | `MathAcos` | `std.math.acos(@as(f64, @floatFromInt(x)))` | `math.js` |
| `Math.atan(x)` | `MathAtan` | `@atan(@as(f64, @floatFromInt(x)))` | `math.js` |
| `Math.atan2(y,x)` | `MathAtan2` | `std.math.atan2(f64, y, x)` | `math.js` |

**参数处理**: 所有三角函数参数需 `@floatCast` 或 `@floatFromInt` 转 `f64`（因为 JS number 推断为 `i64`）。

#### 0.2.2 Math 对数/指数/其他 (7 方法)

| 方法 | BuiltinCall 变体 | Zig 发射 | 测试文件 |
|------|-----------------|----------|---------|
| `Math.log(x)` | `MathLog` | `@log(@floatCast(x))` | `math.js` |
| `Math.log10(x)` | `MathLog10` | `@log10(@floatCast(x))` | `math.js` |
| `Math.log2(x)` | `MathLog2` | `@log2(@floatCast(x))` | `math.js` |
| `Math.exp(x)` | `MathExp` | `@exp(@floatCast(x))` | `math.js` |
| `Math.sign(x)` | `MathSign` | `std.math.sign(x)` | `math.js` |
| `Math.trunc(x)` | `MathTrunc` | `@trunc(@floatCast(x))` | `math.js` |
| `Math.cbrt(x)` | `MathCbrt` | `std.math.cbrt(@floatCast(x))` | `math.js` |

#### 0.2.3 Math 静态常量 (7 个)

| 常量 | 检测方式 | Zig 发射 | 测试文件 |
|------|---------|----------|---------|
| `Math.PI` | 已实现 ✅ | `std.math.pi` | `math.js` |
| `Math.E` | 已实现 ✅ | `std.math.e` | `math.js` |
| `Math.LN2` | 已实现 ✅ | `std.math.ln2` | `math.js` |
| `Math.LN10` | 已实现 ✅ | `std.math.ln10` | `math.js` |
| `Math.LOG2E` | 已实现 ✅ | `std.math.log2e` | `math.js` |
| `Math.LOG10E` | 已实现 ✅ | `std.math.log10e` | `math.js` |
| `Math.SQRT1_2` | 已实现 ✅ | `std.math.sqrt1_2` | `math.js` |
| `Math.SQRT2` | 已实现 ✅ | `std.math.sqrt2` | `math.js` |

**检测方式**: Math 常量不是函数调用，是 `StaticMemberExpression`。需在 `emit_static_member` 或类似路径中检测 `object: Identifier("Math")` → 映射到 `std.math.*` 常量。

**Phase 1 预估**: 21 方法，新增 ~12 个 `BuiltinCall` 变体 + 7 个常量映射，~100 行检测/发射代码。

---

### 0.3 Phase 2 — Runtime 实现 (P1/P2, ~20 方法) ✅ 已完成 (2026-06-24)

**目标**: Array/String 高频缺失方法 + stub 修复，覆盖率 42% → ~52%。

#### 0.3.1 Array 缺失 + stub (12 方法)

| 方法 | 优先级 | Zig 实现策略 | 测试文件 |
|------|--------|-------------|---------|
| `.concat(...arrs)` | P1 | `for` 循环 `appendSlice` 到新 ArrayList | `array.js` |
| `.find(fn)` | P1 | `for` + 闭包调用，首次匹配返回 | `array.js` |
| `.findIndex(fn)` | P1 | `for` + 闭包调用，返回索引或 -1 | `array.js` |
| `.fill(v,s,e)` | P1 | `for` 循环赋值 `[s..e]` | `array.js` |
| `.filter(fn)` — 修复 stub | P1 | `for` + 闭包 + `append` 匹配元素到新 ArrayList | `array.js` |
| `.some(fn)` — 修复 stub | P1 | `for` + 闭包，首次匹配返回 `true` | `array.js` |
| `.every(fn)` — 修复 stub | P1 | `for` + 闭包，首次不匹配返回 `false` | `array.js` |
| `.flat(depth)` — 修复 stub | P2 | 递归展平 ArrayList，depth 控制深度 | `array.js` |
| `.flatMap(fn)` — 修复 stub | P2 | `map(fn)` + `flat(1)` 组合 | `array.js` |
| `.lastIndexOf(item)` | P2 | 反向 `for` 循环 | `array.js` |
| `.at(index)` | P2 | 负值索引处理 `i = if (i < 0) len + i else i` | `array.js` |
| `.copyWithin(t,s,e)` | P2 | `for` 循环内联复制 | `array.js` |

#### 0.3.2 String 缺失方法 (5 方法)

| 方法 | 优先级 | Zig 实现策略 | 测试文件 |
|------|--------|-------------|---------|
| `.substring(s,e)` | P1 | 参数交换 (s>e) + 切片 | `string.js` |
| `.trimStart()` | P2 | 去除左侧空白 (`std.mem.trimLeft`) | `string.js` |
| `.trimEnd()` | P2 | 去除右侧空白 (`std.mem.trimRight`) | `string.js` |
| `.match(re)` | P2 | 需 RegExp 引擎 | `string.js` |
| `.search(re)` | P2 | 需 RegExp 引擎 | `string.js` |

#### 0.3.3 其他补齐 (3 方法)

| 方法 | 优先级 | 实现策略 | 测试文件 |
|------|--------|---------|---------|
| `Object.hasOwn(obj,k)` | P1 | `@hasField()` 或 `std.meta.hasField` | `object.js` |
| `Object.is(v1,v2)` | P2 | 实现 `SameValueZero` 算法 | `object.js` |
| `Object.getOwnPropertyNames(obj)` | P2 | `std.meta.fieldNames` + `comptime` | `object.js` |

**Phase 2 预估**: ~20 方法，新增 ~8 个 `BuiltinCall` 变体 + runtime 实现，~300 行。

---

### 0.4 Phase 3 — Date/Number/MapForEach 补齐 (P2, ~20 方法) ✅ 已完成 (2026-06-25)

| 方法 | 优先级 | 实现策略 | 测试文件 |
|------|--------|---------|---------|
| `new Date()` / `new Date(ms)` / `new Date(str)` | P2 | `JsDate.init()` 构造函数，解析 ms/string | `date.js` |
| `.getMilliseconds()` | P2 | `@mod(ms, 1000)` | `date.js` |
| `.getTimezoneOffset()` | P2 | 本地时区计算 | `date.js` |
| UTC getter 系列 (8) | P2 | 同现有 getter，明确标注 UTC | `date.js` |
| `.toISOString()` | P2 | 格式化 ISO 8601 `"YYYY-MM-DDTHH:mm:ss.sssZ"` | `date.js` |
| `Number.isSafeInteger(v)` | P2 | `std.math.minInt(i53) <= v <= std.math.maxInt(i53)` | `number.js` |
| `Number.MAX/MIN_VALUE/NaN/Infinity/EPSILON` (6) | P2 | 映射到 `std.math.*` 常量 | `number.js` |
| `Number.MAX/MIN_SAFE_INTEGER` (2) | P2 | `9007199254740991` / `-9007199254740991` | `number.js` |
| `.toFixed(d)` | P2 | `std.fmt.format` 浮点格式 | `number.js` |
| `Map.forEach(fn)` | P2 | `for` 遍历 `items()` + 闭包 | `map_set.js` |

---

### 0.5 Phase 4 — 内置对象补齐 (P3, ~80+ 方法) 📋 待开始

**目标**: 补齐剩余 ~80+ 方法，覆盖率从 ~53% 提升至 ~85%+。按优先级分批实现。

#### 0.5.1 String 高级方法 (#627, #630, #632, #639)

| 方法 | 任务 # | 优先级 | 实现策略 | 测试文件 |
|------|--------|--------|----------|---------|
| `.at(index)` | #627 | P3 | 负值索引处理，参考 Array.at | `string.js` |
| `.codePointAt(index)` | #630 | P3 | UTF-16 代理对解析，返回 `u32` | `string.js` |
| `.replaceAll(p,r)` | #632 | P3 | 全局替换，循环 `std.mem.replaceSequence` | `string.js` |
| `.fromCharCode(...codes)` | #632 | P3 | `std.unicode.utf16LeFromString` 或手动构建 | `string.js` |
| `.fromCodePoint(...points)` | #632 | P3 | `std.unicode.utf8Encode` + UTF-8 验证 | `string.js` |
| `.matchAll(re)` | #639 | P4 | 返回迭代器，需 RegExp 引擎支持 | `string.js` |
| `.localeCompare(other)` | #639 | P4 | 简化实现（字节比较）或引入 ICU | `string.js` |
| `.normalize(form)` | #639 | P4 | Unicode 规范化，需 Unicode 数据 | `string.js` |

#### 0.5.2 Array 高级方法 (#631, #635)

| 方法 | 任务 # | 优先级 | 实现策略 | 测试文件 |
|------|--------|--------|----------|---------|
| `.findLast(fn)` | #631 | P3 | 反向 `for` + 闭包调用 | `array.js` |
| `.findLastIndex(fn)` | #631 | P3 | 反向 `for` + 闭包，返回索引 | `array.js` |
| `.reduceRight(fn,init)` | #631 | P3 | 反向 `reduce`，从右到左累积 | `array.js` |
| `.keys()` | #635 | P3 | 返回索引迭代器 `{ value: i, done }` | `array.js` |
| `.values()` | #635 | P3 | 返回元素迭代器 `{ value: v, done }` | `array.js` |
| `.entries()` | #635 | P3 | 返回 `[i, v]` 迭代器 | `array.js` |

#### 0.5.3 Number 高级 (#626, #634)

| 方法/常量 | 任务 # | 优先级 | 实现策略 | 测试文件 |
|----------|--------|--------|----------|---------|
| `.toExponential(digits)` | #626 | P3 | `std.fmt.format` 科学计数法 | `number.js` |
| `.toPrecision(precision)` | #626 | P3 | `std.fmt.format` 指定精度 | `number.js` |
| `Number.EPSILON` | #634 | P3 | `1e-52` (2^-52) | `number.js` |
| `Number.MAX_SAFE_INTEGER` | #634 | P3 | `9007199254740991` (2^53-1) | `number.js` |
| `Number.MIN_SAFE_INTEGER` | #634 | P3 | `-9007199254740991` | `number.js` |
| `Number.MAX_VALUE` | #634 | P3 | `1.7976931348623157e+308` | `number.js` |
| `Number.MIN_VALUE` | #634 | P3 | `5e-324` | `number.js` |
| `Number.POSITIVE_INFINITY` | #634 | P3 | `std.math.inf(f64)` | `number.js` |
| `Number.NEGATIVE_INFINITY` | #634 | P3 | `-std.math.inf(f64)` | `number.js` |
| `Number.NaN` | #634 | P3 | `std.math.nan(f64)` | `number.js` |

#### 0.5.4 Date 高级 (#629, #636, #637)

| 方法 | 任务 # | 优先级 | 实现策略 | 测试文件 |
|------|--------|--------|----------|---------|
| `new Date()` | #636 | P3 | `std.time.timestamp()` 当前时间 | `date.js` |
| `new Date(timestamp)` | #636 | P3 | 毫秒时间戳 → `JsDate` | `date.js` |
| `new Date(dateStr)` | #636 | P3 | 字符串解析（简化 ISO 8601） | `date.js` |
| `.toString()` | #629 | P3 | 本地时区格式化 | `date.js` |
| `.toDateString()` | #629 | P3 | 本地日期部分 | `date.js` |
| `.toTimeString()` | #629 | P3 | 本地时间部分 | `date.js` |
| `.toUTCString()` | #629 | P3 | UTC 格式化 | `date.js` |
| `.toLocaleString()` | #629 | P4 | 简化实现（同 toString） | `date.js` |
| `.valueOf()` | #629 | P3 | 返回时间戳（ms） | `date.js` |
| `.toJSON()` | #629 | P3 | 调用 `toISOString()` | `date.js` |
| UTC getter 系列 (8) | #637 | P3 | `getUTCFullYear/getUTCMonth/...` | `date.js` |

#### 0.5.5 Map/Set 迭代器 (#628) ✅ 已完成 (2026-06-25)

| 方法 | 任务 # | 优先级 | 实现策略 | 测试文件 |
|------|--------|--------|----------|---------|
| `map.keys()` | #628 | P3 | 返回 key 迭代器 | `map_set.js` |
| `map.values()` | #628 | P3 | 返回 value 迭代器 | `map_set.js` |
| `map.entries()` | #628 | P3 | 返回 `[k,v]` 迭代器 | `map_set.js` |
| `set.keys()` → `set.values()` | #628 | P3 | Set 迭代器（alias） | `map_set.js` |
| `set.entries()` | #628 | P3 | 返回 `[v,v]` 迭代器 | `map_set.js` |

#### 0.5.6 Math 高级 (#633)

| 方法 | 任务 # | 优先级 | Zig 映射 | 测试文件 |
|------|--------|--------|----------|---------|
| `Math.expm1(x)` | #633 | P3 | `std.math.expm1(@floatCast(x))` | `math.js` |
| `Math.sinh(x)` | #633 | P3 | `std.math.sinh(@floatCast(x))` | `math.js` |
| `Math.cosh(x)` | #633 | P3 | `std.math.cosh(@floatCast(x))` | `math.js` |
| `Math.tanh(x)` | #633 | P3 | `std.math.tanh(@floatCast(x))` | `math.js` |
| `Math.asinh(x)` | #633 | P3 | `std.math.asinh(@floatCast(x))` | `math.js` |
| `Math.acosh(x)` | #633 | P3 | `std.math.acosh(@floatCast(x))` | `math.js` |
| `Math.atanh(x)` | #633 | P3 | `std.math.atanh(@floatCast(x))` | `math.js` |
| `Math.clz32(x)` | #633 | P3 | `@clz(@as(u32, @intCast(x)))` | `math.js` |
| `Math.fround(x)` | #633 | P3 | `@as(f32, @floatCast(x))` | `math.js` |
| `Math.imul(a,b)` | #633 | P3 | `@as(i32, @intCast(a)) * @as(i32, @intCast(b))` | `math.js` |
| `Math.log1p(x)` | #633 | P3 | `std.math.log1p(@floatCast(x))` | `math.js` |

#### 0.5.7 Object 静态方法 (#638)

| 方法 | 任务 # | 优先级 | 实现策略 | 测试文件 |
|------|--------|--------|----------|---------|
| `Object.keys(obj)` | #638 | P3 | `std.meta.fieldNames` + 过滤 | `object.js` |
| `Object.values(obj)` | #638 | P3 | 遍历 keys 提取值 | `object.js` |
| `Object.entries(obj)` | #638 | P3 | 返回 `[k,v]` 数组 | `object.js` |
| `Object.create(proto)` | #638 | P4 | 原型链操作，需 JsObject 支持 | `object.js` |
| `Object.freeze(obj)` | #638 | P4 | 标记只读（简化实现） | `object.js` |
| `Object.seal(obj)` | #638 | P4 | 标记不可扩展（简化实现） | `object.js` |

**Phase 4 预估**: ~80+ 方法，新增 ~40 个 `BuiltinCall` 变体 + runtime 实现，~1500 行。

---

### 0.6 Phase 5 — 修复 Stub + 剩余高优先级方法 (P2/P3, ~30 方法) 🔧 进行中

**目标**: 实现 Phase 4 遗留的 P2/P3 方法（Object 剩余、Date 剩余、String 高级），覆盖率从 ~65% 提升至 ~80%+。

#### 0.6.1 Object 剩余方法 (#653) ✅ 已完成 (2026-06-26)

| 方法 | 任务 # | 优先级 | Zig 实现策略 | 测试文件 |
|------|--------|--------|----------|---------|
| `Object.create(proto)` | #653 | P3 | 创建新 HashMap，可选复制原型属性（简化） | `object.js` |
| `Object.seal(obj)` | #653 | P3 | no-op（Zig HashMap 默认不可扩展） | `object.js` |
| `Object.defineProperty(obj, key, desc)` | #653 | P3 | `put()` 设置值（忽略 descriptor） | `object.js` |
| `Object.getPrototypeOf(obj)` | #653 | P3 | 返回 null（无原型链支持） | `object.js` |

**实现说明**: 以上均为简化实现，完整原型链支持需后续版本。新增 5 个 Zig 测试。

#### 0.6.2 Date 剩余方法 (#655) ✅ 已完成 (2026-06-26)

| 方法 | 任务 # | 优先级 | Zig 实现策略 | 测试文件 |
|------|--------|--------|----------|---------|
| `.toJSON()` | #655 | P3 | 调用 `toISOString()` | `date.js` |
| `.valueOf()` | #655 | P3 | 返回 `self.millis` | `date.js` |
| `.setFullYear(y, m?, d?)` | #655 | P3 | 修改日期部分毫秒 | `date.js` |
| `.setMonth(m, d?)` | #655 | P3 | 修改月份部分毫秒 | `date.js` |
| `.setDate(d)` | #655 | P3 | 修改日期部分毫秒 | `date.js` |
| `.setHours(h, m?, s?, ms?)` | #655 | P3 | 修改时间部分毫秒 | `date.js` |
| `.setMinutes(m, s?, ms?)` | #655 | P3 | 修改分钟部分毫秒 | `date.js` |
| `.setSeconds(s, ms?)` | #655 | P3 | 修改秒部分毫秒 | `date.js` |
| `.setMilliseconds(ms)` | #655 | P3 | 修改毫秒部分 | `date.js` |
| UTC setter 系列 (8) | #655 | P3 | 同 local setter（UTC-only 实现） | `date.js` |

**实现说明**: 所有 setter 返回新的毫秒时间戳。新增 18 个 Zig 测试。

#### 0.6.3 String 高级方法 (#654, #650) 📋 待实现

| 方法 | 任务 # | 优先级 | Zig 实现策略 | 测试文件 |
|------|--------|--------|----------|---------|
| `.normalize(form)` | #654 | P2 | Unicode 规范化（stub 已实现，需完整实现） | `string.js` |
| `.match(re)` | #650 | P2 | 需 RegExp 引擎 | `string.js` |
| `.search(re)` | #650 | P2 | 需 RegExp 引擎 | `string.js` |
| `.matchAll(re)` | #650 | P3 | 返回迭代器，需 RegExp 引擎 | `string.js` |

**注意**: `normalize()` 当前为 stub（返回原字符串），完整实现需 Unicode 数据。`.match/search/matchAll` 依赖正则引擎，暂不实现。

---

### 0.7 风险点与已知问题

| # | 风险 | 影响范围 | 缓解方案 |
|---|------|---------|---------|
| 1 | **`console.*` detection path** | 3 方法 | console receiver 是 `Identifier("console")` 非类名 — 需在 `detect_builtin_call()` 新增独立分支 |
| 2 | **`str.slice()` vs `arr.slice()` 歧义** | `.slice()` | 需通过 receiver 类型路由（`type_info` 或 `typedarray_vars` 类比） |
| 3 | **`map.has()` vs `set.has()` 歧义** | `.has()/.delete()` | 需区分 receiver 变量类型 → `SetHas`/`MapHas` 独立变体 |
| 4 | **`charAt/charCodeAt` UTF-16 vs UTF-8** | 2 方法 | Zig 字符串为 UTF-8，需处理多字节字符索引 |
| 5 | **闭包回调类型推断** | `filter/some/every/find` | 箭头函数闭包已实现，需验证在 `for` 循环中的调用 |
| 6 | **URI 函数需 `try` + allocator** | 2 方法 | `encodeURIComponent/decodeURIComponent` 可能返回 error |
| 7 | **Math 常量不是函数调用** | 7 常量 | 需在 `emit_static_member` 路径检测 `object: Identifier("Math")` |
| 8 | **`Number.isNaN` vs 全局 `isNaN` 语义不同** | 2 方法 | `Number.isNaN` 不做类型转换，全局 `isNaN` 先 `ToNumber` |
| 9 | **Date 构造函数多态** | 3 重载 | `new Date()/new Date(ms)/new Date(str)` 需在 `new` 表达式中路由 |
| 10 | **`sort()` 无 compareFn 时的默认行为** | `Array.sort` | JS 默认按字符串排序，Zig 默认按数值排序 |

---

### 0.8 实施里程碑

| 里程碑 | 方法数 | 累计覆盖率 | 代码量 | 测试 | 状态 |
|--------|--------|-----------|--------|------|------|
| **M0: 当前基线** | 57 | 22% | — | 169 tests | ✅ |
| **M1: Phase 0 完成** | 84 (+27) | 33% | +150LOC | builtins 6 组 | ✅ (2026-06-24) |
| **M2: Phase 1 完成** | 108 (+24) | 42% | +100LOC | Math 三角/对数/常量 21 方法 | ✅ (2026-06-24) |
| **M3: Phase 2 完成** | 138 (+30) | 53% | +300LOC | Array/String/Object 补齐 | ✅ (2026-06-24) |
| **M4: Phase 3 完成** | 158 (+20) | 61% | +200LOC | Date/Number/MapForEach 补齐 | ✅ (2026-06-25) |
| **M5: Phase 4 完成** | 190 (+32) | 73% | +500LOC | Array.from/of/isArray + Object/Date 剩余 | ✅ (2026-06-26) |
| **M6: Phase 5 完成** | 225+ | ~86% | +300LOC | String 高级 + console | 📋 |

---

### 0.9 相关文件索引

| 文件 | 作用 | 修改阶段 |
|------|------|---------|
| `js2zig-core/src/native_builtins.rs` | `BuiltinCall` 枚举 + `detect_builtin_call()` | Phase 0-4 |
| `js2zig-core/src/codegen/expr.rs` | `emit_builtin_call()` + 常量 emit | Phase 0-4 |
| `runtime/js_console.zig` | `log/err/warn` (已实现) | Phase 0 (连线) |
| `runtime/js_string.zig` | 8 方法 (已实现) + `substring/trimStart/trimEnd` | Phase 0 (连线) + Phase 2 |
| `runtime/js_number.zig` | 5 方法 (已实现) + `isSafeInteger/toFixed` | Phase 0 (连线) + Phase 3 |
| `runtime/js_uri.zig` | `encodeURIComponent/decodeURIComponent` (已实现) | Phase 0 (连线) |
| `runtime/js_map.zig` | `clear/size/forEach` (部分已实现) | Phase 0-2 |
| `runtime/js_set.zig` | `has/delete/clear/size` (已实现) | Phase 0 (连线) |
| `runtime/js_date.zig` | 现有 9 getter (已实现) + 构造函数/toISOString 等 | Phase 3 |
| `runtime/js_regexp.zig` | `test/exec` (已实现, 需迷你引擎) | Phase 4 |
| `examples/builtins-mdn-tests/` | 10 个 JS 测试文件 | Phase 0-4 |
| `docs/JS_FEATURE_EVALUATION.md` | 第 4 节: 方法状态表 | 持续更新 |
| `docs/JS_ROADMAP.md` | 本节: 实施计划 | 持续更新 |

---

## 1. 任务优先级

### 1.1 P2 (当前阶段 — 语言完整性与测试覆盖)

| # | 任务 | 说明 | 预估复杂度 | 状态 | 优先级 |
|---|------|------|------------|------|--------|
| 1 | **解构默认值** | `const {a = 1} = obj` → `const a = obj.a orelse 1` | 低/中 | ✅ 已完成 | ⭐⭐⭐ |
| 2 | **多 spread 合并** | `{ ...a, ...b }` 对象合并 | 中 | ✅ 已完成 | ⭐⭐ |
| 3 | **测试覆盖补充** | 合并项：`instanceof`/`in`/`Date`/`Object`/标签语句 测试 | 低 | ✅ 已完成 | ⭐⭐⭐ |
| 4 | **Class 字段类型推断** | 根据构造函数推断字段类型（替代硬编码 `i64`） | 高 | 📋 待开始 | ⭐⭐ |
| 5 | **嵌套函数声明** | 支持函数内定义函数（含捕获变量） | 中 | ✅ 已完成 | ⭐⭐ |
| 6 | **`for-in` 静态 struct 集成** | 集成到 showcase-project 做端到端验证 | 低 | ✅ 已完成 | ⭐ |
| 7 | **正则表达式引擎** | 引入 C 库（如 `pcre2`）或实现迷你引擎 | 很高 | 📋 待开始 | ⭐ |

**合并说明**：原先"未覆盖特性测试"、`instanceof`/`in` 运算测试、`Date` 方法测试、`Object` 方法测试、标签语句测试 合并为任务 #3，一次性补充测试覆盖。

### 1.2 P2 补充 — 不确定项核实（15 项）✅ 全部完成

> 来源：2026-06-24 JS_FEATURE_EVALUATION.md 文档审计。全部 15 项已于 git log 中逐项验证完成。

| # | 类别 | 任务 | 说明 | 复杂度 | 状态 |
|---|------|------|------|--------|------|
| 1 | 类型推断 | `typeof` 运算符 | 验证 `@typeName(@TypeOf(x))` 输出是否与 JS `typeof` 行为一致 | 低 | ✅ 已完成 |
| 2 | 类型推断 | `null` 字面量 | 验证 `NullLiteral` 返回 `None` 类型是否导致推断错误 | 低 | ✅ 已完成 |
| 3 | 类型推断 | `undefined` → `null` | 验证转换后运行时行为（Zig `null` vs JS `undefined`） | 低 | ✅ 已完成 |
| 4 | 内置对象 | `Math.hypot()` | 验证实现 `@sqrt(a*a + b*b)` 是否等价于标准 `hypot`（精度/溢出） | 中 | ✅ 已完成 |
| 5 | 内置对象 | `String.prototype.charAt()` | 验证 `s[@intCast(i)]` 是否等价于 JS `charAt`（UTF-16 vs UTF-8） | 中 | ✅ 已完成 |
| 6 | 内置对象 | `Date` 时区处理 | 验证 `getFullYear()`/`getMonth()` 等是否使用正确时区 | 高 | ✅ 已完成 |
| 7 | 语句 | `try-catch` 嵌套 | 验证嵌套 try-catch 资源释放是否正确（无重复释放/泄漏） | 中 | ✅ 已完成 |
| 8 | 语句 | `for-in` 静态 struct | 验证静态 struct 展开循环是否正确处理所有字段类型（忽略方法） | 低 | ✅ 已完成 |
| 9 | 语句 | 标签语句 | 编写测试验证 `break label` / `continue label` 行为 | 低 | ✅ 已完成 |
| 10 | 代码生成 | 可选链 `?.` | 验证生成代码 `if (obj) |v| v.prop else null` 是否存在空指针解引用 | 中 | ✅ 已完成 |
| 11 | 代码生成 | 闭包可变捕获 | 验证 `self.x.*` 解引用是否触发 Zig 借用检查器错误 | 高 | ✅ 已完成 |
| 12 | 代码生成 | 模板字符串 `allocPrint` | 验证内存是否正确释放（arena reset 时机） | 中 | ✅ 已完成 |
| 13 | 边缘情况 | 大型字面量 | 创建 1000+ 元素数组/对象测试是否触发编译器栈溢出 | 低 | ✅ 已完成 |
| 14 | 边缘情况 | 深层嵌套调用 | 创建 `a(b(c(d(e(f())))))` 测试编译行为 | 低 | ✅ 已完成 |
| 15 | 边缘情况 | Unicode 标识符 | 验证中文变量名是否被 oxc 解析器和代码生成正确处理 | 低 | ✅ 已完成 |

**执行策略**：
- 低复杂度（#1, #2, #3, #8, #9, #13, #14, #15）→ 可批量执行，每个 15-30 分钟
- 中复杂度（#4, #5, #7, #10, #12）→ 需要编写针对性测试用例
- 高复杂度（#6, #11）→ 需要深入理解 Zig 标准库和借用规则

### 1.3 P3 (长期)

| 任务 | 说明 | 理由 | 状态 | 优先级 |
|------|------|------|------|--------|
| Generator / `yield` | 支持高级异步模式 | 语言完整性 | 📋 待开始 | 低 |
| TypeScript 泛型 | 支持复杂类型推断 | 类型安全 | 📋 待开始 | 低 |
| `interface` / `type` alias | 支持 TypeScript 完整语法 | 语言完整性 | 📋 待开始 | 低 |
| 复杂联合类型支持 | 导出函数参数支持联合类型 | 类型安全 | 📋 待开始 | 低 |
| 错误信息改进 | 附加源位置 + 建议 | 开发体验 | ✅ 已完成 (2026-06-24) | 低 |
| 转译器性能优化 | 支持大文件 JS | 性能 | 📋 待开始 | 低 |
| `Array.prototype.flat/flatMap` | 完整 Array 方法集 | 语言完整性 | ✅ 已完成 (2026-06-24) | 低 |
| `String.prototype.padStart/padEnd` | 完整 String 方法集 | 语言完整性 | ✅ 已完成 (2026-06-24) | 低 |
| Promise API | 不支持，使用 `async/await` 替代 | 语言完整性 | 📋 待开始 | 低 |
| 动态 `import()` | 不支持，使用静态 `import` | 语言完整性 | 📋 待开始 | 低 |
| 私有字段 `#field` | 不支持 | 语言完整性 | 📋 待开始 | 低 |
| 类表达式 | 不支持 | 语言完整性 | 📋 待开始 | 低 |
| 标签模板 | 不支持 | 语言完整性 | 📋 待开始 | 低 |
| JSX | 不支持，使用 `createElement()` 调用 | 语言完整性 | 📋 待开始 | 低 |
| `Math.random()` 安全性改进 | 使用 `std.crypto.random`，但为真随机 | 安全性 | 📋 待开始 | 低 |

---

## 2. 任务状态说明

| 状态 | 说明 |
|------|------|
| ✅ 已完成 | 任务已完成并通过测试 |
| 🚧 进行中 | 任务正在开发中 |
| 📋 待开始 | 任务已规划但未开始 |
| ⏸️ 暂停 | 任务暂停，等待依赖或决策 |
| ❌ 已取消 | 任务已取消 |

---

## 3. 近期成就

### 3.1 2026-06-18 ~ 2026-06-23

| 特性 | 之前状态 | 现在状态 | 完成日期 |
|------|----------|----------|----------|
| 箭头函数闭包（捕获外层变量） | ❌ 不支持 | ✅ 支持 value/reference capture | 2026-06-23 |
| `for-in` 静态对象 | ❌ 仅 HashMap | ✅ 支持 struct 字段展开 | 2026-06-23 |
| Getter/Setter | 🚧 已实现未测试 | ✅ 测试覆盖 | 2026-06-23 |
| `splice` 多参数插入 | 🚧 仅删除 | ✅ 支持删除+插入 | 2026-06-23 |
| 可选链 `?.` | 🚧 简化为直接访问 | ✅ 真正 null 检查 | 2026-06-23 |
| 双 Arena 全局分配器 | ❌ | ✅ 主备自动切换，自动释放内存 | 2026-06-23 |
| 异步 host 函数返回类型推断 | 🚧 有 bug | ✅ 修复 | 2026-06-23 |
| oxc_ast 0.135 兼容 | - | ✅ ForStatementLeft API 适配 | 2026-06-23 |
| test-lib-project 并发转译竞态 | 🐛 | ✅ 修复 | 2026-06-23 |
| showcase-project 闭包集成测试 | 🚧 进行中 | ✅ map/reduce/forEach/every/some/forEach 闭包全通过 | 2026-06-23 |
| 双 Arena 分配器集成到 showcase | 📋 待开始 | ✅ Map/Set/Array 压力测试 + js2rust_reset 验证 | 2026-06-23 |
| TypedArray 完整支持 | 🚧 40% | ✅ `.get()`/`.set()`/`.subarray()`/`.copyWithin()`/`.fill()`/`.buffer`/`.byteLength`/`.byteOffset` (12 tests) | 2026-06-23 |
| 字符串返回值内存管理 | 🚧 有 bug | ✅ `StrRet` C ABI 零拷贝 + arena 分配器 (`std.fmt.allocPrint`) | 2026-06-23 |
| clippy 清理 | 7 个警告 | ✅ 0 警告 | 2026-06-23 |

### 3.2 2026-06-24

| 特性 | 之前状态 | 现在状态 | 完成日期 |
|------|----------|----------|----------|
| 解构默认值 `const {a = 1} = obj` | 📋 待开始 | ✅ 支持 struct 直接访问 + HashMap `.get()` + 类型感知转换 | 2026-06-24 |
| 测试覆盖补充（`instanceof`/`in`/`Date`/`Object`/标签语句） | 📋 待开始 | ✅ 全部有测试覆盖（P1 系列） | 2026-06-24 |
| `Object.fromEntries()` 未实现标记 | - | ✅ 生成 `@compileError`，测试覆盖 | 2026-06-24 |
| P2 不确定项核实（15 项） | 📋 待开始 | ✅ 全部验证完成 | 2026-06-24 |
| `for-in` 静态 struct 集成 | 📋 待开始 | ✅ codegen 验证测试完成（test_p2_for_in_static_codegen）+ 159 测试全通 | 2026-06-24 |
| 嵌套函数声明（含捕获变量） | 📋 待开始 | ✅ 支持函数内定义函数，捕获变量自动生成 struct 字段 + `self.xxx` 重写 | 2026-06-24 |
| Class 声明支持（struct 代码生成） | 📋 待开始 | ✅ struct 定义 + init 构造函数 + 方法 + 字段类型推断 | 2026-06-24 |
| Array.flat/flatMap | 📋 待开始 | ✅ flat identity + flatMap identity (Zig 运行时实现) | 2026-06-24 |
| String.padStart/padEnd | 📋 待开始 | ✅ padStart/padEnd 运行时 + codegen 集成 | 2026-06-24 |
| `@compileError` 源位置 | 📋 待开始 | ✅ 19 个 compileError 调用附加 JS 源位置（file:line:col），codegen 新增 `source` 字段 + 4 个 helper 方法 | 2026-06-24 |
| 内置对象评估 MDN 对齐 | 📋 待开始 | ✅ JS_FEATURE_EVALUATION.md §4 全部重写 (12 类别 260 方法，含 MDN 签名/参数/返回值/Zig 等效)，ROADMAP.md §0 扩展为 8 子节完整实施计划 | 2026-06-24 |
| **P0 内置对象连线 (27 方法)** | ❌ 未连线 | ✅ console (3) + String (8) + Global (5) + Number (5) + Map/Set (6) 全部接入 BuiltinCall 流水线 | 2026-06-24 |
| **P1 内置对象补齐 (21 方法)** | ❌ 未连线 | ✅ Math 三角 (7) + 对数/指数/其他 (7) + 静态常量 (7) 全部接入 | 2026-06-24 |
| **P2 内置对象补齐 (30 方法)** | 📋 待开始 | ✅ Array (12) + String (5) + Object (3) + 其他补齐 + Map/Set 方法全部实现 | 2026-06-24 |
| **FEATURE/ROADMAP 文档更新** | 📋 待开始 | ✅ 内置对象覆盖率 22%→53%，Math 11→31/44，String 8→21/35，Array 14→26/35，Map/Set/Object/Global/Number/console 全部更新 | 2026-06-24 |

---

## 4. 下一步计划

### 4.1 短期（1-2 周）

~~1. **P2: 解构默认值** — 支持 `const {a = 1, b = 2} = obj` 完整 ES6 语法~~ ✅
~~2. **P2: 嵌套函数声明** — 自动提升到模块顶层~~ ✅
~~3. **P2: Class 字段类型推断** — 根据构造函数推断字段类型~~ ✅
~~4. **P2: 测试覆盖补充** — 补充 `instanceof`/`in`/`Date`/`Object`/标签语句 测试~~ ✅
~~5. **P2: `for-in` 静态 struct 集成** — 集成到 showcase-project~~ ✅
~~6. **P3 实用方法** — `Array.flat/flatMap` + `String.padStart/padEnd`~~ ✅
~~7. **P3: 错误信息改进** — 附加源位置 + 建议~~ ✅
~~8. **内置对象补齐 Phase 0/1/2** — 48 方法 P0/P1/P2 连线~~ ✅ (已全部完成，覆盖率 ~53%)

1. **P3: Phase 3 Date/Number 补齐** — 20 方法 (~53% → ~61%)
2. **P3: 正则表达式引擎** — 引入 pcre2 或实现迷你引擎
3. **P3: Symbol/WeakMap/WeakSet 支持** — 语法和 API

### 4.2 中期（1-2 月）

~~1. **P2: Class 字段类型推断** — 根据构造函数推断字段类型（替代硬编码 i64）~~ ✅
~~2. **P2: 嵌套函数声明** — 自动提升到模块顶层~~ ✅
1. **P3: 正则表达式引擎** — 引入 pcre2 或实现迷你引擎
2. **P3: Date/Number 补齐** — Phase 3 约 20 方法

### 4.3 长期（3+ 月）

1. **P3: 开发体验** — 错误信息改进、性能优化
2. **P3: TypeScript 支持** — 泛型、`interface`、`type` alias
3. **P3: 高级特性** — Generator、Promise API

---

## 5. 贡献指南

### 6.1 任务认领流程

1. 查看本文档的 P1/P2/P3 任务列表
2. 选择任务并在状态列标记为 `🚧 进行中`
3. 在任务说明中添加认领日期和预计完成日期
4. 完成后更新状态为 `✅ 已完成` 并添加完成日期

### 6.2 提交要求

- 每个任务完成后需通过所有现有测试（201 个测试）
- 新功能需添加测试用例（Rust 测试或示例项目）
- 提交信息需包含任务编号和说明（如 `feat: implement TypedArray.set() (P1)`）
- `cargo clippy` 零警告

---

## 6. 更新日志

| 日期 | 更新内容 | 更新人 |
|------|----------|--------|
| 2026-06-25 | P0/P1/P2 内置对象连线全部完成 (覆盖率 22%→53%)，FEATURE/ROADMAP 文档同步 | jonathan197608 |
| 2026-06-25 | #628 Map/Set 迭代器完成: JsSet 重构为 JsAny HashMap + SameValueZero 语义，Set iterator codegen 接通 | jonathan197608 |
| 2026-06-24 | 添加 P2 补充不确定项核实任务（15 项），来自 JS_FEATURE_EVALUATION.md 文档审计 | jonathan197608 |
| 2026-06-24 | 清理已完成的 P0/P1 任务；移除与 JS_FEATURE_EVALUATION.md 重叠内容（核心能力、已知限制）；文档结构去重 | jonathan197608 |
| 2026-06-24 | 内置对象补齐计划 MDN 对齐重写: Section 0 扩展为 8 子节 (Phase 0-4 + 风险评估 + 里程碑 + 文件索引 + examples/builtins-mdn-tests 项目设计) | jonathan197608 |
| 2026-06-23 | 初始版本，创建任务规划文档 | jonathan197608 |
| 2026-06-23 | P0 任务全部标记为已完成 | jonathan197608 |
| 2026-06-23 | P1 闭包集成测试标记为已完成，测试计数更新 101→111 | jonathan197608 |
| 2026-06-23 | P1 双 Arena 分配器集成到 showcase 完成：Map/Set/Array 压力测试 + js2rust_reset 验证 | jonathan197608 |
| 2026-06-23 | P1 TypedArray 完整支持完成：`.get()`/`.set()`/`.subarray()`/`.copyWithin()`/`.fill()`/`.buffer`/`.byteLength`/`.byteOffset` (12 个专用测试) | jonathan197608 |
| 2026-06-23 | P0/P1 全部完成，P1 新增字符串返回值内存管理任务；测试计数 111→145，clippy 7→0 警告 | jonathan197608 |
| 2026-06-23 | 文档架构重构：README 拆分中英文、FEATURE 与 ROADMAP 去重解耦、v0.3.2 版本 bump | jonathan197608 |

---

**文档版本**: 2.0  
**最后更新**: 2026-06-25  
**维护者**: jonathan197608
