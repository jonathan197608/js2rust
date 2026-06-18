# 类型推断重新设计方案

> 作者：Cargo
> 日期：2026-06-18
> 状态：设计阶段

---

## 一、设计目标

### 核心原则

| 规则 | 描述 |
|------|------|
| 1. const 决定类型 | `const` 声明 + 常量表达式初始化 → 精确的 Zig 原生类型 |
| 2. var 按使用场景 | `var` 声明或混合表达式 → 按使用场景选择 JsValue 或 JsAny |
| 2.1 值类型 | 字符串、数值、布尔值 → JsValue (现有 union enum) |
| 2.2 静态数组 | 静态长度 + 常量初始值 → `[_]T` |
| 2.3 常量对象 | 属性全常量 → `struct { ... }` |
| 2.4 通用容器 | 新设计 JsAny，涵盖值类型、ArrayList 数组、HashMap 对象 |
| 3. 运算兼容 | JsValue 和 JsAny 参与所有运算符，函数调用自动转换 |
| 4. 返回值推断 | return 语句决定，多个 return 取最宽泛类型 |

---

## 二、三层类型系统

### Layer 1: ZigType（编译期静态类型）

**适用条件：** `const` 声明 + 初始化表达式为常量（字面量、纯算术运算、常量数组、常量对象）

**生成的 Zig 类型：**

| JS 模式 | Zig 类型 | 示例 |
|---------|----------|------|
| `const x = 42` | `i64` | `const x: i64 = 42;` |
| `const x = 3.14` | `f64` | `const x: f64 = 3.14;` |
| `const x = true` | `bool` | `const x: bool = true;` |
| `const x = "hello"` | `[]const u8` | `const x: []const u8 = "hello";` |
| `const arr = [1, 2, 3]` | `[_]i64` | `const arr = [_]i64{ 1, 2, 3 };` |
| `const obj = { x: 1, y: 2 }` | `struct` | `const obj = .{ .x = 1, .y = 2 };` |

**判定规则（常量表达式）：**
- 字面量：NumericLiteral, StringLiteral, BooleanLiteral, NullLiteral
- 纯算术运算：两个常量操作数的 BinaryExpression
- 常量数组：所有元素都是常量表达式
- 常量对象：所有属性值都是常量表达式

### Layer 2: JsValue（运行时动态值类型）

**适用条件：** `var` 声明的值类型变量，或需要参与混合运算的场景

**增强后的 JsValue（现有 + 新增运算符方法）：**

```zig
pub const JsValue = union(enum) {
    int: i64,
    float: f64,
    bool: bool,
    string: []const u8,
    null: void,

    // 现有方法保持不变：asI64, asF64, asString, asBool
    // 现有运算保持不变：add, sub, mul, div, rem, neg, not, eq, lt, le, gt, ge

    // 新增：从任意 Zig 值构造
    pub fn fromI64(v: i64) JsValue { return .{ .int = v }; }
    pub fn fromF64(v: f64) JsValue { return .{ .float = v }; }
    pub fn fromBool(v: bool) JsValue { return .{ .bool = v }; }
    pub fn fromString(v: []const u8) JsValue { return .{ .string = v }; }

    // 新增：类型判断
    pub fn isInt(self: JsValue) bool { return self == .int; }
    pub fn isFloat(self: JsValue) bool { return self == .float; }
    pub fn isString(self: JsValue) bool { return self == .string; }
    pub fn isBool(self: JsValue) bool { return self == .bool; }
    pub fn isNull(self: JsValue) bool { return self == .null; }

    // 新增：类型标签（用于 typeof 模拟）
    pub fn typeName(self: JsValue) []const u8 {
        return switch (self) {
            .int => "number",
            .float => "number",
            .bool => "boolean",
            .string => "string",
            .null => "object",
        };
    }
};
```

**生成代码示例：**
```zig
// JS: var x = 42; x = "hello";
// Zig:
var x = JsValue.fromI64(42);
x = JsValue.fromString("hello");
```

### Layer 3: JsAny（通用容器类型，新增）

**适用条件：**
- 动态数组（调用 push/pop/shift/unshift/splice/sort/reverse）
- 动态对象（使用计算键访问 `obj[variableKey]`）
- 需要嵌套的数组/对象结构
- const 但初始化表达式不是常量

**JsAny 设计：**

```zig
pub const JsAny = union(enum) {
    value: JsValue,
    array: *JsArrayList,
    object: *JsObjectMap,
    null: void,

    // === 构造方法 ===
    pub fn fromValue(v: JsValue) JsAny { return .{ .value = v }; }
    pub fn fromArray(arr: *JsArrayList) JsAny { return .{ .array = arr }; }
    pub fn fromObject(obj: *JsObjectMap) JsAny { return .{ .object = obj }; }

    // === 类型判断 ===
    pub fn isValue(self: JsAny) bool { return self == .value; }
    pub fn isArray(self: JsAny) bool { return self == .array; }
    pub fn isObject(self: JsAny) bool { return self == .object; }
    pub fn isNull(self: JsAny) bool { return self == .null; }

    // === 值转换（自动 coercion）===
    pub fn asI64(self: JsAny) i64 {
        return switch (self) {
            .value => |v| v.asI64(),
            .array => |a| @intCast(a.items.len),
            .object => |o| @intCast(o.count()),
            .null => 0,
        };
    }

    pub fn asF64(self: JsAny) f64 {
        return switch (self) {
            .value => |v| v.asF64(),
            .array => |a| @floatFromInt(a.items.len),
            .object => |o| @floatFromInt(o.count()),
            .null => 0.0,
        };
    }

    pub fn asString(self: JsAny, alloc: Allocator) []const u8 {
        return switch (self) {
            .value => |v| v.asString(alloc),
            .array => |a| blk: {
                // JSON 风格的数组字符串表示
                var buf = std.ArrayList(u8).init(alloc);
                defer buf.deinit();
                buf.append('[') catch break :blk "";
                for (a.items, 0..) |item, i| {
                    if (i > 0) buf.append(',') catch break :blk "";
                    const s = item.asString(alloc);
                    buf.appendSlice(s) catch break :blk "";
                }
                buf.append(']') catch break :blk "";
                break :blk buf.toOwnedSlice() catch "";
            },
            .object => |o| blk: {
                var buf = std.ArrayList(u8).init(alloc);
                defer buf.deinit();
                buf.append('{') catch break :blk "";
                var iter = o.iterator();
                var first = true;
                while (iter.next()) |entry| {
                    if (!first) buf.append(',') catch break :blk "";
                    first = false;
                    buf.append('"') catch break :blk "";
                    buf.appendSlice(entry.key_ptr.*) catch break :blk "";
                    buf.append('"') catch break :blk "";
                    buf.append(':') catch break :blk "";
                    const s = entry.value_ptr.asString(alloc);
                    buf.appendSlice(s) catch break :blk "";
                }
                buf.append('}') catch break :blk "";
                break :blk buf.toOwnedSlice() catch "";
            },
            .null => "null",
        };
    }

    pub fn asBool(self: JsAny) bool {
        return switch (self) {
            .value => |v| v.asBool(),
            .array => |a| a.items.len > 0,
            .object => |o| o.count() > 0,
            .null => false,
        };
    }

    // === 运算符（委托给 JsValue 的值部分）===
    pub fn add(self: JsAny, other: JsAny, alloc: Allocator) JsAny {
        // 字符串拼接优先
        if (self.isValue() and self.value.isString()) {
            return .{ .value = self.value.add(other.toValue(), alloc) };
        }
        if (other.isValue() and other.value.isString()) {
            return .{ .value = self.toValue().add(other.value, alloc) };
        }
        // 数值加法
        return .{ .value = .{ .float = self.asF64() + other.asF64() } };
    }

    pub fn sub(self: JsAny, other: JsAny) JsAny {
        return .{ .value = .{ .float = self.asF64() - other.asF64() } };
    }

    pub fn mul(self: JsAny, other: JsAny) JsAny {
        return .{ .value = .{ .float = self.asF64() * other.asF64() } };
    }

    pub fn div(self: JsAny, other: JsAny) JsAny {
        const denom = other.asF64();
        if (denom == 0.0) return .{ .value = .{ .float = std.math.inf(f64) } };
        return .{ .value = .{ .float = self.asF64() / denom } };
    }

    pub fn eq(self: JsAny, other: JsAny) bool {
        return self.toValue().eq(other.toValue());
    }

    pub fn lt(self: JsAny, other: JsAny) bool {
        return self.asF64() < other.asF64();
    }

    // === 转换为 JsValue（用于运算时降级）===
    pub fn toValue(self: JsAny) JsValue {
        return switch (self) {
            .value => |v| v,
            .array => |a| .{ .int = @intCast(a.items.len) },
            .object => |o| .{ .int = @intCast(o.count()) },
            .null => .null,
        };
    }

    // === 数组操作 ===
    pub fn arrayPush(self: *JsAny, alloc: Allocator, item: JsAny) !void {
        switch (self.*) {
            .array => |a| try a.append(item),
            else => {
                // 自动升级为数组
                var new_arr = try alloc.create(JsArrayList);
                new_arr.* = JsArrayList.init(alloc);
                try new_arr.append(self.*);
                try new_arr.append(item);
                self.* = .{ .array = new_arr };
            },
        }
    }

    pub fn arrayGet(self: JsAny, index: usize) ?JsAny {
        return switch (self) {
            .array => |a| if (index < a.items.len) a.items[index] else null,
            else => null,
        };
    }

    // === 对象操作 ===
    pub fn objectGet(self: JsAny, key: []const u8) ?JsAny {
        return switch (self) {
            .object => |o| o.get(key),
            else => null,
        };
    }

    pub fn objectPut(self: *JsAny, key: []const u8, value: JsAny) !void {
        switch (self.*) {
            .object => |o| try o.put(key, value),
            else => {},
        }
    }
};

// 类型别名
pub const JsArrayList = std.ArrayList(JsAny);
pub const JsObjectMap = std.StringHashMap(JsAny);
```

---

## 三、类型推断算法

### 3.1 变量类型推断流程

```
infer_var_type(decl):
  is_const = (decl.kind == Const)
  init_type = infer_expr(decl.init)
  is_constant_expr = is_constant(decl.init)

  if is_const and is_constant_expr:
    # Rule 1: const + 常量 → ZigType
    return init_type  # i64, f64, bool, []const u8, [_]T, struct

  if is_const and not is_constant_expr:
    # Rule 2.4: const 但非常量 → JsAny
    return ZigType::JsAny

  # var 声明
  if is_value_type(init_type):
    # Rule 2.1: var + 值类型 → JsValue
    # 但需要检查后续是否被用于动态数组/对象操作
    if will_be_dynamic_array(decl.name) or will_be_dynamic_object(decl.name):
      return ZigType::JsAny
    return ZigType::JsValue

  if is_array_type(init_type) and not will_be_dynamic_array(decl.name):
    # Rule 2.2: var + 静态数组 → 保持 [_]T
    return init_type

  if is_object_type(init_type) and not will_be_dynamic_object(decl.name):
    # Rule 2.3: var + 常量属性对象 → 保持 struct
    return init_type

  # 默认：动态数组/对象 → JsAny
  return ZigType::JsAny
```

### 3.2 常量表达式判定

```rust
fn is_constant_expr(expr: &Expression) -> bool {
    match expr {
        // 字面量
        Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_) => true,

        // 纯算术运算（两个操作数都是常量）
        Expression::BinaryExpression(bin) => {
            is_constant_expr(&bin.left) && is_constant_expr(&bin.right)
        }

        // 一元运算
        Expression::UnaryExpression(un) => is_constant_expr(&un.argument),

        // 常量数组（所有元素都是常量）
        Expression::ArrayExpression(arr) => {
            arr.elements.iter().all(|elem| {
                elem.as_expression().map(is_constant_expr).unwrap_or(false)
            })
        }

        // 常量对象（所有属性值都是常量）
        Expression::ObjectExpression(obj) => {
            obj.properties.iter().all(|prop| {
                match prop {
                    ObjectPropertyKind::ObjectProperty(p) => is_constant_expr(&p.value),
                    ObjectPropertyKind::SpreadProperty(_) => false,
                }
            })
        }

        // 括号表达式
        Expression::ParenthesizedExpression(p) => is_constant_expr(&p.expression),

        // 其他都不是常量
        _ => false,
    }
}
```

### 3.3 函数返回值类型推断

```
infer_return_type(fn_body):
  return_types = collect_all_return_types(fn_body)

  if return_types.is_empty():
    return Void

  # 所有返回类型相同
  if all_same(return_types):
    return return_types[0]

  # 数值类型混合 → widen
  if all_numeric(return_types):
    return widen_all(return_types)  # i64 + f64 → f64

  # 含 JsAny → JsAny（最宽泛）
  if any_is_jsany(return_types):
    return JsAny

  # 含 JsValue → JsValue
  if any_is_jsvalue(return_types):
    return JsValue

  # 异构类型 → Union
  return make_union(return_types)
```

### 3.4 运算符自动转换规则

| 左操作数 | 右操作数 | 结果类型 | 转换方式 |
|---------|---------|---------|---------|
| ZigType | ZigType | ZigType (widen) | 直接运算 |
| ZigType | JsValue | JsValue | ZigType → JsValue |
| JsValue | ZigType | JsValue | ZigType → JsValue |
| JsValue | JsValue | JsValue | 直接运算 |
| JsAny | 任意 | JsAny | 其他 → JsAny |
| 任意 | JsAny | JsAny | 其他 → JsAny |

**Codegen 策略：**
- 如果两边都是 ZigType：直接生成 Zig 原生运算
- 如果任一边是 JsValue：生成 `JsValue.add(a, b, alloc)` 调用
- 如果任一边是 JsAny：生成 `JsAny.add(a, b, alloc)` 调用

### 3.5 函数参数自动转换

```
convert_arg(arg_type, param_type):
  if arg_type == param_type:
    return identity  # 无需转换

  if param_type is ZigType and arg_type is ZigType:
    return implicit_cast  # Zig 隐式转换

  if param_type is JsValue:
    if arg_type is ZigType:
      return JsValue.fromXxx(value)  # 包装
    if arg_type is JsAny:
      return arg.toValue()  # 降级

  if param_type is JsAny:
    if arg_type is ZigType:
      return JsAny.fromValue(JsValue.fromXxx(value))  # 包装
    if arg_type is JsValue:
      return JsAny.fromValue(value)  # 包装

  # 不兼容转换 → 编译错误
  return error
```

---

## 四、ZigType 枚举修改

```rust
pub enum ZigType {
    // === Layer 1: 静态类型 ===
    I64,
    I32,
    Usize,
    F64,
    F32,
    Bool,
    String,
    Null,
    Void,
    Array(Box<ZigType>),      // [_]T
    Slice(Box<ZigType>),      // []const T
    Optional(Box<ZigType>),   // ?T
    FunctionPtr(Box<ZigFuncSig>),
    Struct(String),           // 命名结构体
    Object { fields: Vec<(String, ZigType)> },  // 匿名结构体

    // === Layer 2: 动态值类型 ===
    JsValue,  // 替换原来的 Any

    // === Layer 3: 通用容器 ===
    JsAny,    // 新增

    // === 保留 ===
    Union(Vec<ZigType>),  // 联合类型（用于多 return）
}
```

**`to_zig_str()` 映射：**

| ZigType | Zig 代码 |
|---------|---------|
| `JsValue` | `JsValue` |
| `JsAny` | `JsAny` |
| `Union(...)` | 按最宽泛成员决定 |

---

## 五、实现计划

### Phase 1: Zig 运行时（runtime/）

| 任务 | 文件 | 描述 |
|------|------|------|
| 1.1 | `jsany.zig` | 新建 JsAny 类型定义 |
| 1.2 | `jsvalue.zig` | 增强 JsValue：添加 fromXxx/isXxx/typeName 方法 |
| 1.3 | `js_runtime.zig` | 导出 JsAny |
| 1.4 | `js_array.zig` | 重写为基于 JsAny 的 ArrayList 操作 |
| 1.5 | `js_object.zig` | 重写为基于 JsAny 的 HashMap 操作 |

### Phase 2: Rust 类型推断（infer.rs）

| 任务 | 描述 |
|------|------|
| 2.1 | ZigType 枚举：将 `Any` 改名为 `JsValue`，新增 `JsAny` |
| 2.2 | 新增 `is_constant_expr()` 函数 |
| 2.3 | 修改变量类型推断：const + 常量 → ZigType；var 值类型 → JsValue；动态 → JsAny |
| 2.4 | 修改 `to_zig_str()`：JsValue → "JsValue"，JsAny → "JsAny" |
| 2.5 | 修改返回值推断：含 JsAny → JsAny，含 JsValue → JsValue |
| 2.6 | 修改 `widen()`：添加 JsValue/JsAny 的 widen 规则 |

### Phase 3: Codegen 适配（codegen/）

| 任务 | 文件 | 描述 |
|------|------|------|
| 3.1 | `mod.rs` | 变量声明生成：JsValue/JsAny 变量用对应构造器 |
| 3.2 | `expr.rs` | 运算符生成：按操作数类型选择原生运算/JsValue方法/JsAny方法 |
| 3.3 | `expr.rs` | 函数调用：参数自动转换代码生成 |
| 3.4 | `stmt.rs` | 赋值语句：类型转换 |
| 3.5 | `fn_decl.rs` | 函数声明：返回类型和参数类型 |

### Phase 4: 测试验证

| 任务 | 描述 |
|------|------|
| 4.1 | Zig 单元测试：JsAny 所有运算符 |
| 4.2 | Zig 单元测试：JsValue 增强 API |
| 4.3 | JS→Zig 转换测试：const 常量场景 |
| 4.4 | JS→Zig 转换测试：var 值类型场景 |
| 4.5 | JS→Zig 转换测试：动态数组/对象场景 |
| 4.6 | JS→Zig 转换测试：函数返回值多类型 |
| 4.7 | cargo test --workspace 全绿 |

---

## 六、影响分析

### 破坏性变更

| 变更 | 影响 | 迁移策略 |
|------|------|---------|
| `ZigType::Any` → `ZigType::JsValue` | 全代码库引用 `Any` 的地方 | 全局替换 |
| 新增 `ZigType::JsAny` | to_zig_str、widen、make_union | 添加新分支 |
| dynamic_access_vars → JsAny | codegen 中 HashMap 代码生成 | 改为 JsAny(.object) |
| dynamic_arrays → JsAny | codegen 中 ArrayList 代码生成 | 改为 JsAny(.array) |

### 向后兼容

- 现有测试用例的 JS 代码不需要修改
- 生成的 Zig 代码结构变化，但语义等价
- JsValue 现有 API 保持不变，只是新增方法
