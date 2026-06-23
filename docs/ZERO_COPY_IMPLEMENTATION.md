# Host 函数零复制模式 — 详细实施方案 (v3.0)

> **目标**: 消除 Host 函数调用中参数和返回值的不必要复制
> **日期**: 2026-06-23
> **状态**: 待确认实施
> **冷却期**: 600 秒（10 分钟），支持慢速异步调用
> **设计简化**: 所有内存由 Zig Arena 管理，不需要 `owned_by_zig` 标记

---

## 1. 设计总览

### 1.1 核心思想

**强制 Arena 分配 — 所有内存由 Zig 管理**

```
┌─────────────────────────────────────────────────────────────────────┐
│  Zig → Rust (参数零复制)                                         │
│  ──────────────────────────────────────────────────────────────────│
│  当前: []const u8 → dupeZ → [*:0]const u8 → Rust CStr        │
│  修改: []const u8 → 直接传递 ptr + len → Rust &[u8] (借用)    │
│  复制: 1 次 → 0 次 ✅                                          │
└─────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────┐
│  Rust → Zig (返回值零复制)                                       │
│  ──────────────────────────────────────────────────────────────────│
│  当前: Rust CString → Zig dupe → Zig Arena → Rust free          │
│  修改: Rust 调用 js_allocator_alloc → Zig Arena → 直接返回 ptr   │
│  复制: 1 次 → 0 次 ✅                                          │
│  内存管理: Rust 分配 + Rust free → Zig Arena 管理 (自动释放)     │
└─────────────────────────────────────────────────────────────────────┘
```

### 1.2 为什么不需要 `owned_by_zig` 标记

**用户反馈**: "目前内存都是 zig 拥有"

**分析**:
- 当前实现：Rust 分配 CString → Zig `dupe` 复制到 Arena → Zig 调用 `host_free` 释放 Rust 分配
- 最终内存都是 Zig Arena 管理（复制后的副本）
- 零复制目标：Rust 直接分配到 Zig Arena，连复制都不需要

**结论**:
- 强制所有 Rust Host 函数都使用 `js_allocator_alloc` 分配到 Zig Arena
- 所有返回的字符串指针都在 Zig Arena 中
- Zig 侧直接使用，不需要 `free`（Arena 自动管理）
- **不需要 `owned_by_zig` 标记**，因为永远是 `true`

### 1.3 冷却期调整

**修改**: `DEFAULT_GRACE_MS` 从 `5000` (5 秒) 调整为 `600000` (600 秒 = 10 分钟)

**理由**:
- 支持慢速异步调用（async host function 可能需要几秒到几分钟）
- Arena 内存上限 `DEFAULT_MAX_ARENA_SIZE_MB` = 100MB，10 分钟内足够轮换一次
- 可通过环境变量 `JS2RUST_ARENA_GRACE_MS` 调整

---

## 2. 修改清单

### 2.1 文件修改总览

| 文件 | 修改内容 | Breaking Change |
|------|----------|-----------------|
| `runtime/js_allocator.zig` | 调整 `DEFAULT_GRACE_MS` 到 600000 (10 分钟) | ❌ |
| `runtime/js_allocator.zig` | 导出 `js_allocator_alloc(size) -> [*]u8` 到 C ABI | ❌ |
| ~~`runtime/string.zig`~~ | ~~修改 `StrRet`，添加 `owned` 字段~~ → **不需要** | ❌ |
| `js2zig-core/src/host.rs` | 修改 `to_c_abi_type()`：字符串参数用 `ptr + len` | ✅ |
| `js2zig-core/src/host.rs` | 修改 `generate_zig_header()`：生成新签名 + 零复制返回值处理 | ✅ |
| `js2zig-core/src/native_proto/codegen/` | 修改字符串参数 codegen | ✅ |
| `js2rust-bridge/src/lib.rs` | 修改宏：生成新 C ABI 签名 + 零复制 wrapper | ✅ |
| `examples/test-bin-project/` | 更新 Host 函数示例（使用 `js_allocator_alloc`） | ✅ |
| 移除 `host_free` | Rust 不再自己分配内存，不需要释放 | ✅ |

---

## 3. 详细实施步骤

### Phase 0: 调整冷却期（10 分钟）

**文件**: `runtime/js_allocator.zig`

**修改**:

```zig
// 修改前
const DEFAULT_GRACE_MS: u64 = 5000;

// 修改后
const DEFAULT_GRACE_MS: u64 = 600000; // 600 秒 = 10 分钟
```

**验证**:
```bash
cd runtime && zig build test
```

---

### Phase 1: Zig → Rust 参数零复制

#### Step 1.1: 修改 `host.rs` 字符串参数类型

**文件**: `js2zig-core/src/host.rs`

**问题**: `to_c_abi_type()` 当前返回单个类型，但 `ptr + len` 需要两个参数。

**重构方案**:

```rust
// HostType 添加方法
impl HostType {
    /// C ABI 参数类型（可能多个）
    fn to_c_abi_params(&self) -> Vec<String> {
        match self {
            HostType::Str => vec!["[*]const u8".to_string(), "usize".to_string()],
            HostType::I64 => vec!["i64".to_string()],
            HostType::F64 => vec!["f64".to_string()],
            HostType::Bool => vec!["bool".to_string()],
            // ...
        }
    }
    
    /// C ABI 返回值类型
    fn to_c_abi_return(&self) -> String {
        match self {
            HostType::Str => "StrRet".to_string(),
            HostType::I64 => "i64".to_string(),
            // ...
        }
    }
}
```

#### Step 1.2: 修改 `generate_zig_header()` — 字符串参数

**文件**: `js2zig-core/src/host.rs`

**修改同步函数 wrapper 生成**:

```rust
// 修改前：生成 dupeZ + defer free
// pub fn host_func_wrap(param: []const u8) void {
//     const c_param = js_allocator.g_alloc().dupeZ(u8, param) catch return;
//     defer js_allocator.g_alloc().free(c_param);
//     host_func(c_param);
// }

// 修改后：直接传递 ptr + len
// pub fn host_func_wrap(param: []const u8) void {
//     host_func(param.ptr, param.len);
// }
```

**代码修改**:

```rust
fn generate_zig_header(host: &HostFunction, is_async: bool) -> String {
    // ...
    let mut params = vec![];
    for param in &host.params {
        if param.ty == HostType::Str {
            // 参数列表添加 ptr 和 len
            params.push(format!("{}_ptr: [*]const u8", param.name));
            params.push(format!("{}_len: usize", param.name));
        } else {
            params.push(format!("{}: {}", param.name, param.ty.to_c_abi_return()));
        }
    }
    // ...
}
```

#### Step 1.3: 修改 Rust 侧 Host 函数签名

**Breaking Change**: 所有字符串参数的 Host 函数需要修改签名。

**示例**:

```rust
// 修改前
#[no_mangle]
pub extern "C" fn host_print(s: *const c_char) {
    let s = unsafe { CStr::from_ptr(s) }.to_str().unwrap();
    println!("{}", s);
}

// 修改后
#[no_mangle]
pub extern "C" fn host_print(ptr: *const u8, len: usize) {
    let s = unsafe { slice::from_raw_parts(ptr, len) };
    // s 是借用，函数返回后无效
    // 如果需要保存，自行复制：let owned = String::from_utf8_lossy(s).to_string();
    println!("{}", String::from_utf8_lossy(s));
}
```

**迁移指南**:
1. 搜索所有 `#[no_mangle] pub extern "C"` 函数
2. 找到字符串参数（`*const c_char` 或 `CStr`）
3. 修改为 `ptr: *const u8, len: usize`
4. 修改函数体：使用 `slice::from_raw_parts` 替代 `CStr::from_ptr`

---

### Phase 2: Rust → Zig 返回值零复制

#### Step 2.1: 导出 Zig Arena 分配器到 C ABI

**文件**: `runtime/js_allocator.zig`

**新增函数**:

```zig
/// Export Zig Arena allocator to C ABI.
/// Rust Host functions call this to allocate memory in Zig's Arena,
/// enabling zero-copy returns.
///
/// Usage in Rust:
/// ```rust
/// extern "C" {
///     fn js_allocator_alloc(size: usize) -> *mut u8;
/// }
///
/// #[no_mangle]
/// pub extern "C" fn host_func() -> StrRet {
///     let data = "hello".as_bytes();
///     let ptr = js_allocator_alloc(data.len());
///     std::ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len());
///     StrRet { ptr, len: data.len() as isize }
/// }
/// ```
pub export fn js_allocator_alloc(size: usize) [*]u8 {
    const alloc = getAllocator();
    const slice = alloc.alloc(u8, size) catch @panic("js_allocator_alloc failed");
    return slice.ptr;
}
```

**注意**: 
- 这个函数分配的内存由 Arena 管理，不需要手动释放
- Arena 在 `js2rust_deinit()` 或轮换时自动释放

#### Step 2.2: 修改 Rust 侧 Host 函数宏

**文件**: `js2rust-bridge/src/lib.rs`

**修改宏生成**: 生成使用 `js_allocator_alloc` 的代码。

**示例**:

```rust
// 修改前：Rust 分配 CString
// #[no_mangle]
// pub extern "C" fn host_func() -> *const c_char {
//     let s = func();
//     CString::new(s).unwrap().into_raw()
// }

// 修改后：Rust 分配到 Zig Arena
#[no_mangle]
pub extern "C" fn host_func() -> StrRet {
    // 声明外部函数
    extern "C" {
        fn js_allocator_alloc(size: usize) -> *mut u8;
    }
    
    let data = func().into_bytes();
    let ptr = unsafe { js_allocator_alloc(data.len()) };
    unsafe { std::ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len()) };
    
    StrRet {
        ptr: ptr as *const u8,
        len: data.len() as isize,
    }
}
```

**问题**: `StrRet` 在 Rust 侧的定义需要更新（添加 `owned_by_zig` 字段？不，用户说不需要）。

**简化**: `StrRet` 不需要修改，因为所有返回值都是 Arena 分配的，Zig 侧直接使用。

#### Step 2.3: 修改 Zig 侧返回值处理

**文件**: `js2zig-core/src/host.rs`

**修改返回值 wrapper**:

```zig
// 修改前：复制返回值
// pub fn host_func_wrap() []const u8 {
//     const raw = host_func(...);
//     const span = std.mem.span(raw);
//     const owned = js_allocator.g_alloc().dupe(u8, span) catch return "";
//     host_free(@ptrCast(@constCast(raw)));
//     return owned;
// }

// 修改后：零复制，直接使用
// pub fn host_func_wrap() []const u8 {
//     const result = host_func(...);
//     // result.ptr 在 Zig Arena 中，直接使用
//     return result.toSlice();
// }
```

**注意**: 
- 不再调用 `host_free`（因为内存是 Arena 分配的）
- 如果 Rust Host 函数没有使用 `js_allocator_alloc`，会导致内存泄漏（Rust 分配的内存没有被释放）

**强制措施**: 
- 文档明确说明：所有 Rust Host 函数必须使用 `js_allocator_alloc`
- 提供 lint 或静态分析检查（未来）

#### Step 2.4: 移除 `host_free`

**文件**: `js2rust-bridge/src/lib.rs` 和 Rust Host 函数

**修改**:
- 移除 `host_free` 的声明和定义
- 所有 Rust Host 函数不再调用 `host_free`

**理由**: 
- 零复制模式下，Rust 不再自己分配内存
- 所有内存都是 Zig Arena 分配的，由 Arena 管理

---

### Phase 3: 完整的零复制示例

#### 3.1 同步 Host 函数（参数 + 返回值零复制）

**Rust 侧**:

```rust
use std::os::raw::c_void;
use std::slice;

// 声明 Zig Arena 分配器
extern "C" {
    fn js_allocator_alloc(size: usize) -> *mut u8;
}

/// 零复制参数 + 零复制返回值
#[no_mangle]
pub extern "C" fn host_concat(ptr: *const u8, len: usize, suffix_ptr: *const u8, suffix_len: usize) -> StrRet {
    // 参数：零复制（借用 Zig Arena 内存）
    let s1 = unsafe { slice::from_raw_parts(ptr, len) };
    let s2 = unsafe { slice::from_raw_parts(suffix_ptr, suffix_len) };
    
    // 拼接
    let result = format!("{}{}", String::from_utf8_lossy(s1), String::from_utf8_lossy(s2));
    
    // 返回值：零复制（分配到 Zig Arena）
    let bytes = result.as_bytes();
    let out_ptr = unsafe { js_allocator_alloc(bytes.len()) };
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), out_ptr, bytes.len());
    
    StrRet {
        ptr: out_ptr as *const u8,
        len: bytes.len() as isize,
    }
}
```

**Zig 侧**（由 `js2zig-core` 自动生成）:

```zig
// 自动生成的 Zig wrapper
pub fn host_concat(a: []const u8, b: []const u8) []const u8 {
    const result = c.host_concat(a.ptr, a.len, b.ptr, b.len);
    // result.ptr 在 Arena 中，直接使用
    return result.toSlice();
}
```

#### 3.2 异步 Host 函数

**Rust 侧**:

```rust
#[no_mangle]
pub extern "C" fn async_fetch_user(name_ptr: *const u8, name_len: usize, callback: ...) {
    let name = unsafe { slice::from_raw_parts(name_ptr, name_len) };
    
    // 异步操作...
    let result = format!("User: {}", String::from_utf8_lossy(name));
    
    // 分配到 Zig Arena
    let bytes = result.as_bytes();
    let out_ptr = unsafe { js_allocator_alloc(bytes.len()) };
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), out_ptr, bytes.len());
    
    // 调用回调...
}
```

---

## 4. 兼容性迁移计划

### 4.1 强制迁移（推荐）

**理由**: 零复制模式更简单、更高效，不需要兼容层。

**迁移步骤**:
1. 更新 `js2rust-bridge` 宏，生成使用 `js_allocator_alloc` 的代码
2. 更新所有示例 Host 函数
3. 文档说明：所有 Rust Host 函数必须使用 `js_allocator_alloc`

### 4.2 提供迁移工具

**脚本**: `tools/migrate_host_functions.sh`

```bash
#!/bin/bash
# 自动迁移 Host 函数签名
# 1. 将 *const c_char 改为 ptr: *const u8, len: usize
# 2. 添加 js_allocator_alloc 声明
# 3. 修改函数体

find . -name "*.rs" -exec sed -i 's/.../.../g' {} \;
```

**注意**: 自动迁移脚本容易出错，建议手动迁移 + 提供详细指南。

---

## 5. 测试计划

### 5.1 单元测试

**文件**: `js2zig-core/src/native_proto/tests.rs`

**新增测试**:

```rust
#[test]
fn test_zero_copy_string_param() {
    // 测试字符串参数零复制
    // 1. 注册 Host 函数（新签名：ptr + len）
    // 2. 调用 Host 函数
    // 3. 验证正确性（无复制）
}

#[test]
fn test_zero_copy_string_return() {
    // 测试字符串返回值零复制
    // 1. 注册 Host 函数（使用 js_allocator_alloc）
    // 2. 调用 Host 函数
    // 3. 验证零复制（指针相同，无复制）
}
```

### 5.2 集成测试

**文件**: `examples/test-bin-project/`

**修改示例**:
1. 更新 `host_functions.rs`：使用新签名 + `js_allocator_alloc`
2. 添加零复制示例 Host 函数
3. 运行 `zig build test` 验证

---

## 6. 风险评估

### 6.1 技术风险

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| Arena 在 Host 函数执行期间轮换 | Use-after-free | 单线程保证 + cooling 10 分钟宽限期 |
| Rust 侧错误使用 ptr + len | Segfault | 文档 + 宏生成安全 wrapper |
| `js_allocator_alloc` 线程安全 | 数据竞争 | 当前单线程，未来加锁 |
| Breaking Change 导致用户代码失效 | 迁移成本 | 提供迁移指南 + 工具 |

### 6.2 性能风险

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| 零复制收益不明显 | 修改成本高 | Benchmark 验证 |
| Arena 分配压力大 | 内存占用高 | 100MB 上限 + 10 分钟宽限期 |

---

## 7. 实施时间表

| 阶段 | 任务 | 预计时间 | 状态 |
|------|------|----------|------|
| Phase 0 | 调整冷却期到 10 分钟 | 0.5 小时 | 待实施 |
| Phase 1 | Zig → Rust 参数零复制 | 4 小时 | 待实施 |
| Phase 2 | Rust → Zig 返回值零复制 | 4 小时 | 待实施 |
| Phase 3 | 示例更新 + 测试 | 2 小时 | 待实施 |
| **总计** | | **10.5 小时** | |

---

## 8. 确认清单

实施前需要确认：

- [ ] 接受 Breaking Change（修改 Host 函数签名）
- [ ] 冷却期 10 分钟可接受
- [ ] 强制零复制模式（所有 Host 函数必须使用 `js_allocator_alloc`）
- [ ] 移除 `host_free`（不再需要）
- [ ] 不需要兼容层（或者接受手动迁移）
- [ ] Benchmark 计划已制定

---

## 9. 附录：StrRet 结构体（不需要修改）

**文件**: `runtime/string.zig`

**当前定义**:

```zig
pub const StrRet = extern struct {
    ptr: [*c]const u8,
    len: isize,
};
```

**说明**:
- 不需要添加 `owned_by_zig` 字段
- 所有返回值都是 Zig Arena 分配的
- Zig 侧直接使用 `ptr`，不需要 `free`

---

**作者**: Jonathan Huang  
**日期**: 2026-06-23  
**版本**: 3.0 (Simplified — No `owned_by_zig` flag)
