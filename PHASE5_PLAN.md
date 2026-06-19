# Phase 5 实施计划：高级数组方法

## 目标
实现 JS 高级数组方法的转译：pop, shift, join, reverse, sort, slice, map, filter

## 当前状态（2026-06-19）
- **Phase 1-4**: 已完成，测试通过
- **Phase 5**: 禁用（未添加测试到 `app.js`）
- **构建状态**: 通过

## 遇到的问题

### 1. 动态数组索引语法错误
**问题**: 转译器为动态数组（`ArrayList(JsAny)`）生成 `arr[0]`，但 Zig 中应该是 `arr.items[0]`

**需要修复**:
- `js2zig-core/src/codegen/expr.rs` - 数组索引代码生成
- 检查 `emit_expr` 中 `Identifier` + 索引的处理

### 2. 动态数组迭代语法错误
**问题**: 转译器为 `for...of` 循环生成错误语法

**需要修复**:
- `js2zig-core/src/codegen/stmt.rs` - `for...of` 循环代码生成
- 为动态数组生成 `for (arr.items) |item|` 而不是错误的语法

### 3. `catch` 语法错误
**问题**: `catch &[_]i64` 应该是 `catch &[_]i64{}`

**需要修复**:
- `js2zig-core/src/codegen/builtins.rs` - 检查 `map`/`filter` 的 `catch` 语法

### 4. 运行时函数类型不匹配
**问题**: `js_array.zig` 中的函数期望 `[]const i64`，但动态数组是 `[]JsAny`

**需要添加**:
- `js_array.joinAny(alloc, arr, sep)` - 处理 `ArrayList(JsAny)`
- `js_array.sliceAny(alloc, arr, start, end)` - 处理 `ArrayList(JsAny)`
- `js_array.mapAny(alloc, arr, scalar)` - 处理 `ArrayList(JsAny)`
- `js_array.filterAny(alloc, arr, threshold)` - 处理 `ArrayList(JsAny)`

### 5. `buf.writer()` 兼容性问题
**问题**: Zig 0.16.0 中 `buf.writer()` 的使用方式有变化

**需要修复**:
- `runtime/js_array.zig` - `joinAny` 函数的实现
- 可能需要使用 `std.fmt.allocPrint` 替代 `buf.writer()`

### 6. 类型推断逻辑
**问题**: `pop`/`shift` 导致数组被标记为动态数组，但 `reverse`/`sort` 可以返回新数组

**需要修改**:
- `js2zig-core/src/infer.rs` - `detect_dynamic_arrays_expr` 函数
- 考虑只对 `push`/`unshift` 标记为动态数组

## 实施步骤

### 步骤 1: 修复动态数组索引
- [ ] 修改 `expr.rs` 中的数组索引代码生成
- [ ] 为动态数组生成 `arr.items[idx]`
- [ ] 测试：添加简单的动态数组索引测试

### 步骤 2: 修复动态数组迭代
- [ ] 修改 `stmt.rs` 中的 `for...of` 代码生成
- [ ] 为动态数组生成 `for (arr.items) |item|`
- [ ] 测试：添加 `for...of` 迭代动态数组的测试

### 步骤 3: 添加 JsAny 运行时函数
- [ ] 在 `js_array.zig` 中添加 `joinAny`, `sliceAny`, `mapAny`, `filterAny`
- [ ] 修复 `buf.writer()` 兼容性问题
- [ ] 测试：单独测试这些函数

### 步骤 4: 更新 builtins.rs
- [ ] 更新 `join`, `slice`, `map`, `filter` 的代码生成
- [ ] 调用新增的 `Any` 版本函数
- [ ] 修复 `catch` 语法

### 步骤 5: 修复 reverse 和 sort
- [ ] 确保 `reverse` 和 `sort` 返回数组（使用 `blk:` 语法）
- [ ] 为动态数组调用 `sortInPlace` 辅助函数

### 步骤 6: 添加 Phase 5 测试
- [ ] 在 `app.js` 中添加 Phase 5 测试函数
- [ ] 在 `main.rs` 中添加对应的 Rust 测试调用
- [ ] 运行构建并修复所有编译错误

### 步骤 7: 验证和提交
- [ ] 运行所有测试并验证输出
- [ ] 修复发现的问题
- [ ] 提交并推送到远程仓库

## 参考资料

### 相关文件
- `js2zig-core/src/codegen/expr.rs` - 表达式代码生成
- `js2zig-core/src/codegen/stmt.rs` - 语句代码生成
- `js2zig-core/src/codegen/builtins.rs` - 内置函数代码生成
- `js2zig-core/src/infer.rs` - 类型推断
- `runtime/js_array.zig` - 数组运行时函数
- `examples/showcase-project/js_src/app.js` - JS 测试文件
- `examples/showcase-project/src/main.rs` - Rust 测试调用

### 测试 JS 函数（参考）
```javascript
// -- Array.pop --
export function testArrayPop() {
    const arr = [10, 20, 30];
    const last = arr.pop();
    if (last !== 30) return -1;
    if (arr.length !== 2) return -2;
    return 0;
}

// -- Array.join --
export function testArrayJoin() {
    const arr = [1, 2, 3];
    const s = arr.join("-");
    if (s !== "1-2-3") return -1;
    return 0;
}

// -- Array.reverse --
export function testArrayReverse() {
    const arr = [1, 2, 3];
    const rev = arr.reverse();
    if (rev[0] !== 3) return -1;
    if (rev[2] !== 1) return -2;
    return 0;
}

// -- Array.slice --
export function testArraySlice() {
    const arr = [10, 20, 30, 40, 50];
    const sub = arr.slice(1, 4);
    if (sub.length !== 3) return -1;
    if (sub[0] !== 20) return -2;
    return 0;
}

// -- Array.map --
export function testArrayMap() {
    const arr = [1, 2, 3];
    const doubled = arr.map(function(x) { return x * 2; });
    if (doubled.length !== 3) return -1;
    if (doubled[0] !== 2) return -2;
    return 0;
}

// -- Array.filter --
export function testArrayFilter() {
    const arr = [1, 2, 3, 4, 5];
    const evens = arr.filter(function(x) { return x % 2 === 0; });
    if (evens.length !== 2) return -1;
    if (evens[0] !== 2) return -2;
    return 0;
}
```

## 备注
- 优先修复静态数组的代码生成（更简单）
- 动态数组的代码生成需要更多修改
- 考虑先让静态数组的测试通过，再处理动态数组
