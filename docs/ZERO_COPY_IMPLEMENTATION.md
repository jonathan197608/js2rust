# Host 函数零复制模式 — 详细实施方案

> **目标**: 消除 Host 函数调用中参数和返回值的不必要复制
> **日期**: 2026-06-23
> **状态**: 待确认实施
> **冷却期**: 600 秒（10 分钟），支持慢速异步调用

---

## 1. 设计总览

### 1.1 核心思想

**Arena 借用 + 所有权转移**

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
│  方案 A: Rust 分配到 Zig Arena → 直接返回 ptr (零复制)          │
│  方案 B: Rust 分配 → Zig 接管所有权 → Zig 负责 free (零复制)   │
│  复制: 1 次 → 0 次 ✅                                          │
└─────────────────────────────────────────────────────────────────────┘
```

### 1.2 冷却期调整

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
| `runtime/js_allocator.zig` | 调整 `DEFAULT_GRACE_MS` | ❌ |
| `runtime/js_allocator.zig` | 导出 `js_allocator_alloc` 到 C ABI | ❌ |
| `runtime/string.zig` | 修改 `StrRet`，添加 `owned: bool` 字段 | ✅ |
| `js2zig-core/src/host.rs` | 修改 `to_c_abi_type()`：字符串参数用 `ptr + len` | ✅ |
| `js2zig-core/src/host.rs` | 修改 `generate_zig_header()`：生成新签名 | ✅ |
| `js2zig-core/src/native_proto/codegen/` | 修改字符串参数 codegen | ✅ |
| `js2rust-bridge/src/lib.rs` | 修改宏：生成新 C ABI 签名 | ✅ |
| `examples/test-bin-project/` | 更新 Host 函数示例 | ✅ |
| `docs/ZERO_COPY_DESIGN.md` | 更新设计文档 | ❌ |

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

**修改 `to_c_abi_type()` 函数**:

```rust
// 修改前
fn to_c_abi_type(ty: &HostType) -> String {
    match ty {
        HostType::Str => "[*:0]const u8".to_string(),
        // ...
    }
}

// 修改后
fn to_c_abi_type(ty: &HostType, is_param: bool) -> String {
    match ty {
        HostType::Str => {
            if is_param {
                // 参数：传递 ptr + len，零复制
                // 注意：这个函数现在返回 (ptr_type, len_type)，需要重构
                todo!("refactor to return (ptr, len)")
            } else {
                // 返回值：仍然使用 StrRet
                "StrRet".to_string()
            }
        }
        // ...
    }
}
```

**问题**: `to_c_abi_type()` 当前返回单个类型，但 `ptr + len` 需要两个参数。

**重构方案**:

```rust
// 新设计：HostType 添加 to_c_abi_param() 方法
impl HostType {
    /// C ABI 参数类型（可能多个）
    fn to_c_abi_params(&self) -> Vec<String> {
        match self {
            HostType::Str => vec!["[*]const u8".to_string(), "usize".to_string()],
            HostType::I64 => vec!["i64".to_string()],
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
// 修改前：生成 [_:0]const u8 参数
// pub fn host_func_wrap(param: []const u8) void {
//     const c_param = js_allocator.g_alloc().dupeZ(u8, param) catch return;
//     defer js_allocator.g_alloc().free(c_param);
//     host_func(c_param);
// }

// 修改后：生成 ptr + len 参数
// pub fn host_func_wrap(param: []const u8) void {
//     host_func(param.ptr, param.len);
// }
```

**代码修改**:

```rust
fn generate_zig_header(host: &HostFunction, is_async: bool) -> String {
    // ...
    for (param in host.params) {
        if param.ty == HostType::Str {
            // 修改：不生成 _wrap 函数，直接传递 ptr + len
            // 参数列表添加 ptr 和 len
            params.push(format!("{}_ptr: [*]const u8", param.name));
            params.push(format!("{}_len: usize", param.name));
        } else {
            params.push(format!("{}: {}", param.name, to_c_abi_type(&param.ty)));
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

### Phase 2: Rust → Zig 返回值零复制（所有权转移）

#### Step 2.1: 修改 `StrRet` 结构体

**文件**: `runtime/string.zig`

**修改**:

```zig
pub const StrRet = extern struct {
    ptr: [*c]const u8,
    len: isize,
    
    /// 新增：内存归属标记
    /// false = Rust 分配，Zig 需要调用 host_free
    /// true  = Zig Arena 分配，Zig 不需要 free (Arena 管理)
    owned_by_zig: bool,
    
    /// Build from Zig Arena-allocated slice (zero-copy return)
    pub fn from_arena(s: []const u8) StrRet {
        return StrRet{
            .ptr = s.ptr,
            .len = @intCast(s.len),
            .owned_by_zig = true,  // Arena 管理，不需要 free
        };
    }
    
    /// Build from Rust-allocated CString (needs host_free)
    pub fn from_owned(ptr: [*c]const u8, len: usize) StrRet {
        return StrRet{
            .ptr = ptr,
            .len = @intCast(len),
            .owned_by_zig = false,  // Rust 分配，需要 free
        };
    }
    
    /// Build from Rust-allocated CString (needs host_free)
    pub fn from_cstring(cstr: [*:0]const u8) StrRet {
        const len = std.mem.len(cstr);
        return StrRet{
            .ptr = cstr,
            .len = @intCast(len),
            .owned_by_zig = false,
        };
    }
};
```

**Breaking Change**: `StrRet` 添加 `owned_by_zig` 字段，Rust 侧需要更新。

#### Step 2.2: 导出 Zig Arena 分配器到 C ABI

**文件**: `runtime/js_allocator.zig`

**新增函数**:

```zig
/// Export Zig Arena allocator to C ABI.
/// Rust Host functions can use this to allocate memory in Zig's Arena,
/// enabling zero-copy returns.
///
/// Usage in Rust:
/// ```rust
/// extern "C" {
///     fn js_allocator_alloc(size: usize) -> *mut u8;
///     fn js_allocator_free(ptr: *mut u8);
/// }
///
/// #[no_mangle]
/// pub extern "C" fn host_func() -> StrRet {
///     let data = "hello".as_bytes();
///     let ptr = js_allocator_alloc(data.len);
///     std::ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len());
///     StrRet { ptr, len: data.len(), owned_by_zig: true }
/// }
/// ```
pub export fn js_allocator_alloc(size: usize) [*]mut u8 {
    const alloc = getAllocator();
    const slice = alloc.alloc(u8, size) catch @panic("js_allocator_alloc failed");
    return slice.ptr;
}

/// Free memory (only needed for non-Arena allocations).
/// For Arena-allocated memory, this is a no-op (Arena reset handles it).
pub export fn js_allocator_free(ptr: [*]mut u8) void {
    // Arena 分配的内存不需要手动 free，这里留空
    // 如果未来使用通用分配器，这里需要实际实现
    _ = ptr;
}
```

**问题**: `alloc.alloc()` 返回 `[]u8`，但 C ABI 需要 `[*]mut u8`。需要类型转换。

**修改**:

```zig
pub export fn js_allocator_alloc(size: usize) [*]mut u8 {
    const alloc = getAllocator();
    const slice = alloc.alloc(u8, size) catch @panic("js_allocator_alloc failed");
    return slice.ptr;
}
```

#### Step 2.3: 修改 Rust 侧 Host 函数宏

**文件**: `js2rust-bridge/src/lib.rs`

**修改宏生成**:

```rust
// 修改前：生成返回 CString 的代码
// #[no_mangle]
// pub extern "C" fn host_func() -> *const c_char {
//     let s = func();
//     CString::new(s).unwrap().into_raw()
// }

// 修改后：生成返回 StrRet 的代码（支持零复制）
// #[no_mangle]
// pub extern "C" fn host_func() -> StrRet {
//     // 方案 A：分配到 Zig Arena (零复制)
//     let data = func();
//     let ptr = js_allocator_alloc(data.len());
//     std::ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len());
//     StrRet { ptr, len: data.len() as isize, owned_by_zig: true }
//
//     // 方案 B：Rust 分配，Zig 接管 (零复制)
//     let s = CString::new(func()).unwrap();
//     let ptr = s.into_raw();
//     StrRet { ptr, len: ..., owned_by_zig: false }
// }
```

**问题**: Rust 侧需要声明 `js_allocator_alloc` 外部函数。

**修改**: 在 `js2rust-bridge` 宏生成的代码中添加：

```rust
extern "C" {
    fn js_allocator_alloc(size: usize) -> *mut u8;
}
```

#### Step 2.4: 修改 Zig 侧返回值处理

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

// 修改后：根据 owned_by_zig 决定是否复制
// pub fn host_func_wrap() []const u8 {
//     const result = host_func(...);
//     if (result.owned_by_zig) {
//         // Arena 分配，直接使用，不需要 free
//         return result.toSlice();
//     } else {
//         // Rust 分配，需要复制 + free
//         const owned = js_allocator.g_alloc().dupe(u8, result.toSlice()) catch return "";
//         host_free(result.ptr);
//         return owned;
//     }
// }
```

**问题**: 这样仍然有复制（当 `owned_by_zig = false` 时）。

**优化**: 如果 Rust Host 函数都改为使用 `js_allocator_alloc`，就可以完全零复制。

---

### Phase 3: 完整的零复制示例

#### 3.1 同步 Host 函数（参数 + 返回值零复制）

**Rust 侧**:

```rust
use std::os::raw::c_void;

// 声明 Zig Arena 分配器
extern "C" {
    fn js_allocator_alloc(size: usize) -> *mut u8;
}

/// 零复制参数 + 零复制返回值
#[no_mangle]
pub extern "C" fn host_concat(ptr: *const u8, len: usize, suffix: *const u8, suffix_len: usize) -> StrRet {
    // 参数：零复制（借用 Zig Arena 内存）
    let s1 = unsafe { slice::from_raw_parts(ptr, len) };
    let s2 = unsafe { slice::from_raw_parts(suffix, suffix_len) };
    
    // 拼接
    let result = format!("{}{}", String::from_utf8_lossy(s1), String::from_utf8_lossy(s2));
    
    // 返回值：零复制（分配到 Zig Arena）
    let bytes = result.as_bytes();
    let out_ptr = unsafe { js_allocator_alloc(bytes.len()) };
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), out_ptr, bytes.len());
    
    StrRet {
        ptr: out_ptr,
        len: bytes.len() as isize,
        owned_by_zig: true,  // Arena 管理，不需要 free
    }
}
```

**Zig 侧**（由 `js2zig-core` 自动生成）:

```zig
// 自动生成的 Zig wrapper
pub fn host_concat(a: []const u8, b: []const u8) []const u8 {
    const result = c.host_concat(a.ptr, a.len, b.ptr, b.len);
    // result.owned_by_zig = true，直接使用
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

### 4.1 提供兼容层（可选）

如果用户不想立即修改 Host 函数，可以提供兼容层：

**文件**: `js2rust-bridge/src/lib.rs`

**生成兼容 wrapper**:

```rust
// 兼容旧签名（[*:0]const u8）的 wrapper
#[no_mangle]
pub extern "C" fn host_print_compat(s: *const c_char) {
    // 将 [*:0]const u8 转换为 ptr + len
    let len = unsafe { libc::strlen(s) };
    host_print(s as *const u8, len)
}
```

**问题**: 这需要用户显式选择兼容模式，或者宏自动生成两个版本。

### 4.2 迁移工具（推荐）

提供脚本，自动修改 Rust Host 函数签名：

**脚本**: `tools/migrate_host_functions.sh`

```bash
#!/bin/bash
# 自动迁移 Host 函数签名
# 将 *const c_char 改为 ptr: *const u8, len: usize

find . -name "*.rs" -exec sed -i 's/\(fn \w\+\)(\([^)]*\)\*const c_char\([^)]*\))/...\1...(ptr: *const u8, len: usize).../g' {} \;
```

**问题**: 自动迁移脚本容易出错，建议手动迁移 + 提供详细指南。

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
    // 3. 验证正确性
}

#[test]
fn test_zero_copy_string_return() {
    // 测试字符串返回值零复制
    // 1. 注册 Host 函数（返回 StrRet with owned_by_zig = true）
    // 2. 调用 Host 函数
    // 3. 验证零复制（指针相同）
}
```

### 5.2 集成测试

**文件**: `examples/test-bin-project/`

**修改示例**:
1. 更新 `host_functions.rs`：使用新签名
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
| Breaking Change 导致用户代码失效 | 迁移成本 | 提供迁移指南 + 兼容层（可选） |

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
| Phase 2 | Rust → Zig 返回值零复制 | 6 小时 | 待实施 |
| Phase 3 | 示例更新 + 测试 | 2 小时 | 待实施 |
| 总计 | | **12.5 小时** | |

---

## 8. 确认清单

实施前需要确认：

- [ ] 接受 Breaking Change（修改 Host 函数签名）
- [ ] 冷却期 10 分钟可接受
- [ ] 所有权转移方案（Phase 2）设计合理
- [ ] 不需要兼容层（或者接受手动迁移）
- [ ] Benchmark 计划已制定

---

**作者**: Jonathan Huang  
**日期**: 2026-06-23  
**版本**: 2.0 (Detailed Implementation Plan)
