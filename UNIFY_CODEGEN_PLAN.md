# js2rust 代码生成统一方案 v3.0（最终版）

## 一、核心决策

| 决策点 | 选择 | 说明 |
|--------|------|------|
| **实施层级** | 层级 2（推荐注解） | 复杂类型必须注解，基础类型可推断 |
| **推断失败处理** | 混合模式 | 复杂类型失败 → 编译错误；基础类型失败 → 默认 `[]const u8` |
| **实施顺序** | 先推断，后 JSDoc | 先实现类型推断，再扩展 JSDoc 解析 |
| **基础类型默认** | `[]const u8` | 基础类型推断失败时，默认字符串类型 |
| **数组类型实现** | 静态分析可变性 | 只读 → `[]const T`，可写 → `std.ArrayList(T)` |
| **浮点数推断** | 支持科学计数法 | `1e3` → `f64`，`1.5e2` → `f64` |
| **Allocator 方案** | A（Rust 端创建 GPA） | Rust 端创建 GeneralPurposeAllocator，传入 Zig |
| **js_allocator fallback** | 移除 | 强制要求初始化，移除 fallback |
| **C ABI wrapper allocator** | B（使用 js_allocator） | 使用 `js_allocator.g_alloc()`，修改 Rust 端释放逻辑 |

---

## 二、类型推断规则（完整版）

### 2.1 基础类型推断

| JS 类型 | Zig 类型 | 推断条件 | 失败处理 |
|---------|---------|---------|---------|
| `number`（整数） | `i64` | 数字字面量无小数点、无科学计数法 | 默认 `[]const u8` |
| `number`（浮点数） | `f64` | 数字字面量有小数点、科学计数法 | 默认 `[]const u8` |
| `number`（除法） | `f64` | 除法运算（即使能整除） | 默认 `[]const u8` |
| `string` | `[]const u8` | 字符串字面量、模板字符串 | 默认 `[]const u8` |
| `boolean` | `bool` | 布尔字面量、比较运算 | 默认 `[]const u8` |
| `undefined` | `?T` | 可选参数 | 默认 `[]const u8` |
| `null` | `?T` | 可空类型 | 默认 `[]const u8` |

#### 浮点数科学计数法支持

```rust
fn infer_number_literal(lit: &str) -> ZigType {
    // 检查科学计数法：1e3, 1.5e2, 1E-3 等
    if lit.contains('e') || lit.contains('E') {
        return ZigType::F64;  // 科学计数法 → f64
    }
    
    // 检查小数点
    if lit.contains('.') {
        return ZigType::F64;  // 带小数点 → f64
    }
    
    // 整数
    ZigType::I64
}
```

**示例**：
```javascript
const a = 1e3;      // → f64 (1000.0)
const b = 1.5e2;    // → f64 (150.0)
const c = 1E-3;     // → f64 (0.001)
const d = 1.23;     // → f64
const e = 42;       // → i64
```

---

### 2.2 数组可变性静态分析

#### 分析规则

| 操作 | 可变性 | Zig 类型 |
|------|--------|---------|
| `arr.push(x)` | 可变 | `std.ArrayList(T)` |
| `arr.pop()` | 可变 | `std.ArrayList(T)` |
| `arr.shift()` | 可变 | `std.ArrayList(T)` |
| `arr.unshift(x)` | 可变 | `std.ArrayList(T)` |
| `arr[i] = x` | 可变 | `std.ArrayList(T)` |
| `arr.length` | 只读 | `[]const T` 或 `std.ArrayList(T)` |
| `arr[i]` | 只读 | `[]const T` 或 `std.ArrayList(T)` |

#### 分析算法

```rust
fn analyze_array_mutability(arr_name: &str, program: &Program) -> ArrayMutability {
    let mut is_mutable = false;
    
    // 遍历 AST
    for stmt in &program.body {
        // 检查数组变量的方法调用
        if let Some(call_expr) = is_method_call(stmt, arr_name) {
            match call_expr.callee.property.name {
                "push" | "pop" | "shift" | "unshift" | "splice" => {
                    is_mutable = true;
                }
                _ => {}
            }
        }
        
        // 检查数组赋值
        if let Some(assign_expr) = is_array_assignment(stmt, arr_name) {
            is_mutable = true;
        }
    }
    
    if is_mutable {
        ArrayMutability::Mutable  // → std.ArrayList(T)
    } else {
        ArrayMutability::Immutable  // → []const T
    }
}
```

#### 生成代码示例

**示例 1**：只读数组 → 切片

```javascript
// JS 代码
export function sum(arr) {
    let total = 0;
    for (let i = 0; i < arr.length; i++) {
        total = total + arr[i];
    }
    return total;
}
```

**生成代码**：
```zig
pub fn sum(arr: []const i64) i64 {
    var total: i64 = 0;
    for (arr) |item| {
        total = total + item;
    }
    return total;
}
```

---

**示例 2**：可写数组 → ArrayList

```javascript
// JS 代码
export function addNumbers() {
    let arr = [1, 2, 3];
    arr.push(4);
    return arr;
}
```

**生成代码**：
```zig
pub fn addNumbers() std.ArrayList(i64) {
    var arr = std.ArrayList(i64).init(js_allocator.g_alloc());
    arr.append(1) catch unreachable;
    arr.append(2) catch unreachable;
    arr.append(3) catch unreachable;
    arr.append(4) catch unreachable;
    return arr;
}
```

---

**示例 3**：`.length` 属性处理

```javascript
// JS 代码
export function getLength(arr) {
    return arr.length;
}
```

**生成代码**（根据数组可变性）：
```zig
// 如果 arr 是 []const i64
pub fn getLength(arr: []const i64) i64 {
    return @intCast(arr.len);
}

// 如果 arr 是 std.ArrayList(i64)
pub fn getLength(arr: std.ArrayList(i64)) i64 {
    return @intCast(arr.items.len);
}
```

---

## 三、堆内存分配模式方案

### 3.1 Allocator 选择方案

#### 决策：方案 A（Rust 端创建 GPA，传入 Zig）

**理由**：
1. 性能最好（GPA 比 page_allocator 快很多）
2. 内存安全（GPA 检测内存泄漏，调试模式）
3. 统一管理（一个分配器管理所有内存）
4. 生命周期清晰（Rust 端控制分配器生命周期）

---

### 3.2 Allocator 实施方案

#### 步骤 1：修改 Rust 端代码

**目标**：Rust 端创建 `GeneralPurposeAllocator`，传入 Zig

**实现**：
```rust
// js2rust-bridge/src/lib.rs 或类似文件

#[no_mangle]
pub extern "C" fn js2rust_init() {
    // 创建 Zig GeneralPurposeAllocator
    let mut gpa = zig_general_purpose_allocator_new();
    let alloc = zig_general_purpose_allocator_get(&gpa);
    
    // 传入 Zig
    unsafe {
        init_js2rust(alloc);
    }
    
    // 保存 gpa 到全局变量，确保生命周期
    // ...
}

#[no_mangle]
pub extern "C" fn js2rust_deinit() {
    // 清理 GPA
    // ...
}
```

**注意**：需要使用 Zig 的 C ABI 来创建和管理 GPA。可能需要写一个小的 Zig 辅助函数。

---

#### 步骤 2：修改 Zig 端代码

**文件**：`runtime/js_allocator.zig`

```zig
const std = @import("std");

var g_allocator: ?std.mem.Allocator = null;

/// 由 Rust 端调用，传入 GPA
pub fn setGlobalAllocator(alloc: std.mem.Allocator) void {
    g_allocator = alloc;
}

/// 获取全局分配器
pub fn g_alloc() std.mem.Allocator {
    if (g_allocator) |a| return a;
    // 移除 fallback，强制要求初始化
    @compileError("g_allocator not initialized. Call setGlobalAllocator() first.");
}
```

**改进**：
1. 移除 fallback（强制要求初始化）
2. 添加调试断言（`std.debug.assert(g_allocator != null)`）

---

#### 步骤 3：修改 C ABI wrapper 的 allocator

**当前**：使用 `std.heap.page_allocator`
**改为**：使用 `js_allocator.g_alloc()`

**文件**：`js2zig-core/src/pipeline.rs`（orchestrator 生成代码）

```zig
// 当前生成代码
pub export fn greet_cabi(name: [*:0]const u8, result_len: *usize) [*:0]u8 {
    const name_slice: []const u8 = std.mem.span(name);
    const _result = main.greet_impl(name_slice);
    const _result_cstr = allocator.dupeZ(u8, _result) catch unreachable;  // allocator = page_allocator
    result_len.* = _result.len;
    return _result_cstr;
}
```

**改为**：
```zig
pub export fn greet_cabi(name: [*:0]const u8, result_len: *usize) [*:0]u8 {
    const name_slice: []const u8 = std.mem.span(name);
    const _result = main.greet_impl(name_slice);
    const alloc = js_allocator.g_alloc();
    const _result_cstr = alloc.dupeZ(u8, _result) catch unreachable;
    result_len.* = _result.len;
    return _result_cstr;
}
```

**Rust 端释放逻辑修改**：
```rust
// 当前：使用 libc::free (page_allocator 分配)
#[no_mangle]
pub extern "C" fn free_string(ptr: *mut u8, len: usize) {
    unsafe {
        libc::free(ptr as *mut libc::c_void);
    }
}

// 改为：使用 js_allocator 释放
#[no_mangle]
pub extern "C" fn free_string(ptr: *mut u8, len: usize) {
    // 需要传入 allocator 给 Zig，让 Zig 自己释放
    // 或者：Rust 端保存 allocator 引用，用于释放
    // ...
}
```

**注意**：这需要修改 Rust 和 Zig 两端的释放逻辑，确保使用同一个 allocator。

---

### 3.3 内存管理规则

| 规则 | 说明 | 示例 |
|------|------|------|
| **规则 1** | 所有分配使用 `js_allocator.g_alloc()` | `const ptr = try g_alloc().create(T);` |
| **规则 2** | 临时分配使用 `defer` 释放 | `defer g_alloc().destroy(ptr);` |
| **规则 3** | 字符串拼接使用 `allocPrint` + `defer free` | `const s = try std.fmt.allocPrint(g_alloc(), "...", .{}); defer g_alloc().free(s);` |
| **规则 4** | C ABI wrapper 使用 `js_allocator.g_alloc()` | `const cstr = g_alloc().dupeZ(u8, s) catch unreachable;` |
| **规则 5** | 闭包返回使用 `create` 堆分配 | `const ptr = try g_alloc().create(Counter);` |
| **规则 6** | Rust 端释放使用同一个 allocator | 需要确保 Rust 端有 allocator 的引用 |

---

## 四、实施计划（最终版）

### 阶段 0：Allocator 改造（2-3 天）⭐ 新增

**目标**：实现 Allocator 方案 A（Rust 端创建 GPA，传入 Zig）

**任务**：
1. 修改 `js_allocator.zig`：移除 fallback，强制要求初始化
2. 修改 Rust 端代码：创建 GPA，传入 Zig
3. 修改 C ABI wrapper：使用 `js_allocator.g_alloc()`
4. 修改 Rust 端释放逻辑：使用同一个 allocator
5. 测试验证：确保内存分配/释放正常工作

**交付物**：
- 支持 GPA 的 `js_allocator.zig`
- 修改后
- 所有测试通过

---

### 阶段 1：实现基础类型推断（4-5 天）

**任务**：
1. 支持整数推断（`i64`）
2. 支持浮点数推断（`f64`）
3. 支持科学计数法（`1e3` → `f64`）
4. 支持字符串推断（`[]const u8`）
5. 支持布尔推断（`bool`）
6. 实现推断失败默认处理（默认 `[]const u8`）

**交付物**：
- 支持基础类型推断的 `native_proto` 模块
- 基础类型推断测试用例

---

### 阶段 2：实现数组可变性分析（2-3 天）

**任务**：
1. 实现数组可变性静态分析
2. 只读数组 → `[]const T`
3. 可写数组 → `std.ArrayList(T)`
4. 处理 `.length` 属性生成（`.len` vs `.items.len`）

**交付物**：
- 支持数组可变性分析的 `native_proto` 模块
- 数组类型推断测试用例

---

### 阶段 3：实现复杂类型推断（5-7 天）

**任务**：
1. 实现结构体类型推断（`struct`）
2. 实现 Map/Set 类型推断（`std.AutoHashMap`, `std.AutoArrayHashMap`）
3. 实现 TypedArray 类型推断（`[]T`）
4. 添加编译错误（复杂类型推断失败时）

**交付物**：
- 支持复杂类型推断的 `native_proto` 模块
- 复杂类型推断测试用例

---

### 阶段 4：实现堆内存分配模式（3-4 天）

**任务**：
1. 实现分配模式自动选择
2. 实现值语义返回
3. 实现堆分配返回（闭包、大对象）
4. 实现临时分配（字符串拼接）
5. 实现 C ABI 兼容分配（字符串返回）

**交付物**：
- 支持统一内存分配模式的 `native_proto` 模块
- 内存分配测试用例

---

### 阶段 5：扩展 JSDoc 解析（2-3 天）

**任务**：
1. 扩展 `@param` 解析
2. 扩展 `@returns` 解析
3. 扩展 `@type` 解析
4. 添加 `@callback` 解析（可选）

**交付物**：
- 支持完整 JSDoc 注解的 `native_proto` 模块
- JSDoc 注解测试用例

---

### 阶段 6：集成到主流程（1-2 天）

**任务**：
1. 修改 `pipeline.rs` 切换到 `native_proto`
2. 删除 `codegen` 目录
3. 更新文档

**交付物**：
- 使用 `native_proto` 的主流程
- 删除 `codegen` 目录

---

### 阶段 7：测试与验证（2-3 天）

**任务**：
1. 运行现有测试
2. 添加 JSDoc 注解
3. 性能测试

**交付物**：
- 所有测试通过
- 示例项目 JSDoc 注解完整
- 性能测试报告

---

## 五、时间预估（最终版）

| 阶段 | 内容 | 时间 | 依赖 |
|------|------|------|------|
| **阶段 0** | **Allocator 改造** | **2-3 天** | **无** |
| 阶段 1 | 基础类型推断 | 4-5 天 | 阶段 0 |
| 阶段 2 | 数组可变性分析 | 2-3 天 | 阶段 0、1 |
| 阶段 3 | 复杂类型推断 | 5-7 天 | 阶段 0、1、2 |
| 阶段 4 | 堆内存分配模式 | 3-4 天 | 阶段 0、1、2、3 |
| 阶段 5 | 扩展 JSDoc 解析 | 2-3 天 | 阶段 0、1、2、3 |
| 阶段 6 | 集成到主流程 | 1-2 天 | 阶段 0、4、5 |
| 阶段 7 | 测试与验证 | 2-3 天 | 阶段 6 |
| **总计** | | **21-30 天** | |

---

## 六、风险控制

| 风险 | 严重程度 | 缓解措施 |
|------|---------|---------|
| **Allocator 改造复杂** | 🔴 高 | 分阶段实施，先实现基础功能，再优化 |
| **类型推断不准确** | 🔴 高 | 添加详细的错误信息，提示添加 JSDoc 注解 |
| **复杂类型实现困难** | 🟡 中 | 参考 `codegen` 模式的实现 |
| **向后兼容性** | 🔴 高 | 提供迁移指南和自动化工具 |
| **测试覆盖不足** | 🟡 中 | 增加测试案例，确保覆盖所有语法 |

---

## 七、后续工作

### 7.1 性能优化

- 减少 `allocPrint` 使用（预计算字符串）
- 使用 `ArrayList` 替代动态字符串拼接
- 优化闭包内存分配

### 7.2 开发者体验

- 提供 JSDoc 注解模板
- 提供 VS Code 插件（类型检查）
- 提供迁移工具（自动添加 JSDoc 注解）

### 7.3 文档

- 编写《JSDoc 注解规范》
- 编写《类型推断规则》
- 编写《从 JavaScript 到 Zig 迁移指南》
- 编写《Allocator 使用指南》

---

## 八、决策总结

鹏哥，以下是所有决策的总结：

| 决策点 | 选择 | 说明 |
|--------|------|------|
| **实施层级** | 层级 2 | 复杂类型必须注解，基础类型可推断 |
| **推断失败处理** | 混合模式 | 复杂类型 → 编译错误；基础类型 → 默认 `[]const u8` |
| **实施顺序** | 先推断，后 JSDoc | 先实现类型推断，再扩展 JSDoc 解析 |
| **基础类型默认** | `[]const u8` | 基础类型推断失败时，默认字符串类型 |
| **数组类型实现** | 静态分析可变性 | 只读 → `[]const T`，可写 → `std.ArrayList(T)` |
| **浮点数推断** | 支持科学计数法 | `1e3` → `f64`，`1.5e2` → `f64` |
| **Allocator 方案** | A | Rust 端创建 GPA，传入 Zig |
| **js_allocator fallback** | 移除 | 强制要求初始化 |
| **C ABI wrapper allocator** | B | 使用 `js_allocator.g_alloc()` |

---

## 九、下一步行动

**请确认以上方案，我将开始实施阶段 0（Allocator 改造）。**

如果有任何修改，请告诉我。

---

**文档版本**：v3.0（最终版）
**创建时间**：2026-06-21
**作者**：AI Assistant
**审核者**：鹏哥
