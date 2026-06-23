# Host 函数零复制模式设计

> **目标**: 消除 Host 函数调用中参数和返回值的不必要复制
> **日期**: 2026-06-23
> **状态**: 设计讨论中

---

## 1. 当前问题

### 1.1 Zig → Rust（字符串参数复制）

**当前实现** (`host.rs` `generate_zig_header()`):

```zig
// 同步函数字符串参数处理
const c_param = js_allocator.g_alloc().dupeZ(u8, param) catch return "";
defer js_allocator.g_alloc().free(c_param);
// 调用 extern "c" fn host_func(c_param)
```

**复制次数**: 2 次
1. Zig: `dupeZ` — `[]const u8` → `[*:0]const u8` (添加 \0)
2. Rust: `CStr::to_str()` — `[*:0]const u8` → `&str` (可能需要 UTF-8 验证)

**根因**: C ABI 要求字符串是 null-terminated (`[*:0]const u8`)，但 Zig 侧字符串是 `[]const u8` (带长度前缀)。

### 1.2 Rust → Zig（字符串返回值复制）

**当前实现** (`host.rs` `generate_zig_header()`):

```zig
const raw = host_func(...);           // Rust 分配 CString
const span = std.mem.span(raw);       // 计算长度
const owned = js_allocator.g_alloc().dupe(u8, span) catch return "";  // 复制
host_free(@ptrCast(@constCast(raw))); // 释放 Rust 分配
return owned;                          // 返回 Zig 分配的副本
```

**复制次数**: 2 次
1. Rust: `CString::into_raw` — 分配内存
2. Zig: `dupe` — 复制到 Arena

**根因**: Rust 分配的内存不能被 Zig 的 Arena 管理，必须复制到 Zig 的分配器。

---

## 2. 零复制方案

### 方案 A：Arena 借用模式（推荐）

**核心思想**: 利用 Arena 的内存生命周期，避免在单次 Host 调用中复制。

#### 2.1 Zig → Rust（参数零复制）

**设计**:

```zig
// 新设计：直接传递 ptr + len，不需要 dupeZ
extern "c" fn host_func(ptr: [*]const u8, len: usize) callconv(.c) void;

// 调用方
host_func(param.ptr, param.len);  // 零复制！param 在 Arena 中
```

**Rust 侧修改**:

```rust
// 当前：接收 CStr
#[no_mangle]
pub extern "C" fn host_func(s: *const c_char) {
    let s = unsafe { CStr::from_ptr(s) }.to_str().unwrap();
    // ...
}

// 新设计：接收 ptr + len (借用)
#[no_mangle]
pub extern "C" fn host_func(ptr: *const u8, len: usize) {
    let s = unsafe { slice::from_raw_parts(ptr, len) };
    // s 是借用，函数返回后无效
    // 如果需要保存，调用方需要自行复制 to String
}
```

**生命周期保证**:
- Zig Arena 在 Host 函数调用期间不会 reset（单线程，Host 调用是同步的）
- 即使触发 Arena 轮换，旧 Arena 进入 cooling 状态，有 5 秒宽限期
- Host 函数必须在 5 秒内返回（合理假设）

**Breaking Change**: 所有同步 Host 函数的签名需要修改。

#### 2.2 Rust → Zig（返回值零复制）

**设计 1: Rust 分配到 Zig Arena**（彻底零复制）

```rust
// Rust 侧：使用 Zig 的 Arena 分配器
#[no_mangle]
pub extern "C" fn host_func(arena_alloc_func: extern "C" fn(usize) -> *mut u8, ...) -> StrRet {
    let data = "hello".as_bytes();
    let ptr = arena_alloc_func(data.len);  // 在 Zig Arena 中分配
    std::ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len());
    StrRet { ptr, len: data.len() }
}
```

**问题**: 需要导出 Zig Arena 的 `alloc` 函数到 C ABI，复杂度高。

---

**设计 2: 所有权转移**（部分零复制）

```zig
// Zig 侧：不复制，直接接管 Rust 分配的内存
extern "c" fn host_func(result_len: *usize) callconv(.c) [*]const u8;

pub fn host_func_wrapper() []const u8 {
    var result_len: usize = 0;
    const ptr = host_func(&result_len);
    // 注意：这里不复制！直接返回 ptr
    // 调用方负责在使用完后调用 host_free(ptr)
    return ptr[0..result_len];
}
```

**问题**: 内存管理复杂，调用方必须记得调用 `host_free`。

---

**设计 3: 双返回值 + Arena 借用**（推荐）

```zig
// 修改 StrRet：添加 borrower 标记
pub const StrRet = extern struct {
    ptr: [*c]const u8,
    len: isize,
    
    // 新设计：如果 ptr 在 Arena 中，borrowed = true
    // Zig 侧不需要复制，也不需要 free
    borrowed: bool,
};

// Rust 侧：如果字符串在 Zig Arena 中，设置 borrowed = true
#[no_mangle]
pub extern "C" fn host_func() -> StrRet {
    // 情况 1：Rust 分配到 Zig Arena (需要 FFI 导出 alloc)
    let ptr = zig_arena_alloc(len);
    // 填充数据...
    StrRet { ptr, len, borrowed: true }
    
    // 情况 2：Rust 自己分配，Zig 需要复制
    // let s = CString::new("hello").unwrap();
    // let ptr = s.into_raw();
    // StrRet { ptr, len, borrowed: false }
}
```

**问题**: 需要区分内存归属，实现复杂度中等。

---

### 方案 B：共享内存区域

**核心思想**: 预分配一块共享内存，Host 函数调用时直接读写该区域。

**设计**:

```zig
// 全局共享缓冲区（不在 Arena 中）
var g_shared_buf: [4096]u8 = undefined;
var g_shared_len: usize = 0;

pub fn host_func_wrapper(param: []const u8) []const u8 {
    // 复制参数到共享区域
    @memcpy(g_shared_buf[0..param.len], param);
    g_shared_len = param.len;
    
    // 调用 Host 函数（读取共享区域）
    const result_ptr = host_func(g_shared_buf[0..g_shared_len]);
    
    // 返回值也在共享区域中
    return result_ptr[0..result_len];
}
```

**问题**:
- 仍然需要复制（参数复制到共享区域）
- 共享区域大小有限
- 不是真正的零复制

---

### 方案 C：序列化到栈（小数据优化）

**核心思想**: 对于小字符串（< 256 字节），直接序列化到栈上，避免堆分配。

**设计**:

```zig
// 小字符串：栈分配
pub fn host_func_small(param: []const u8) void {
    var buf: [256]u8 = undefined;
    @memcpy(buf[0..param.len], param);
    // 调用 Host 函数...
}

// 大字符串：仍然使用 Arena
```

**问题**: 只优化了小字符串情况，不彻底。

---

## 3. 推荐方案：方案 A（Arena 借用模式）

### 3.1 修改后的调用流程

```
┌──────────────────────────────────────────────────────────────────┐
│  Zig → Rust (参数)                                             │
│  ───────────────────────────────────────────────────────────────│
│  修改前: []const u8 → dupeZ → [*:0]const u8 → Rust CStr      │
│         复制 1 次        复制 (添加 \0)                        │
│                                                                 │
│  修改后: []const u8 → 直接传递 ptr + len → Rust slice          │
│         零复制！                                                │
└──────────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────────┐
│  Rust → Zig (返回值)                                           │
│  ───────────────────────────────────────────────────────────────│
│  修改前: Rust CString → Zig dupe → Zig Arena → Rust free      │
│         复制 1 次                                               │
│                                                                 │
│  修改后: Rust 分配到 Zig Arena → 直接返回 ptr                   │
│         零复制！（需要导出 Zig Arena alloc 到 C ABI）           │
│                                                                 │
│  或者: Rust 分配 → Zig 接管所有权 → Zig 负责 free              │
│         零复制！（Zig 不复制，直接管理 Rust 分配的内存）        │
└──────────────────────────────────────────────────────────────────┘
```

### 3.2 实现步骤（分阶段）

#### Phase 1: Zig → Rust 参数零复制

**修改文件**:
1. `js2zig-core/src/host.rs` — 修改 `to_c_abi_type()` 和 `generate_zig_header()`
2. `js2rust-bridge/src/lib.rs` — 修改宏生成代码，使用 `ptr + len` 而不是 `[*:0]const u8`
3. 用户 Rust Host 函数 — 需要修改签名（Breaking Change）

**代码示例**:

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
    // s 是 &[u8]，函数返回后无效
    // 如果需要保存，自行复制：let owned = s.to_vec();
    println!("{}", String::from_utf8_lossy(s));
}
```

**收益**: 消除 Zig 侧的 `dupeZ` 复制（每次调用节省 1 次复制）。

#### Phase 2: Rust → Zig 返回值零复制（方案 1）

**设计**: 导出 Zig Arena 的 `alloc` 函数到 C ABI，让 Rust 可以直接分配到 Zig 的 Arena。

**修改文件**:
1. `runtime/js_allocator.zig` — 导出 `js_allocator_alloc(usize) -> *mut u8` 到 C ABI
2. `js2rust-bridge/src/lib.rs` — 修改 Rust 侧 Host 函数，使用 Zig 分配器
3. `js2zig-core/src/host.rs` — 修改返回值处理，不调用 `dupe`

**问题**: 
- 需要线程安全（Arena 分配需要锁）
- Zig Arena 可能在 Rust 持有指针时 reset

**解决方案**:
- Arena 分配函数获取锁（防止轮换）
- Host 函数返回后释放锁
- 或者：Rust 分配到 cooling Arena（5 秒宽限期足够）

---

#### Phase 2: Rust → Zig 返回值零复制（方案 2 — 更简单）

**设计**: Rust 分配内存，但通过 C ABI 返回 `StrRet`。Zig 侧不复制，直接使用，但在适当的时候调用 `host_free`。

**修改文件**:
1. `js2zig-core/src/host.rs` — 修改返回值处理，返回 `StrRet` 而不是 `[]const u8`
2. 调用方代码 — 需要管理 `StrRet` 的生命周期

**代码示例**:

```zig
// 修改前：复制返回值
pub fn host_func_wrap() []const u8 {
    const raw = host_func(...);
    const span = std.mem.span(raw);
    const owned = js_allocator.g_alloc().dupe(u8, span) catch return "";
    host_free(@ptrCast(@constCast(raw)));
    return owned;
}

// 修改后：零复制，但需要手动管理内存
pub fn host_func_wrap() StrRet {
    return host_func(...);  // 直接返回 StrRet
}

// 使用方
const result = host_func_wrap();
defer host_free(result.ptr);
const s = result.toSlice();  // 直接使用，不复制
```

**问题**: 需要调用方记得调用 `host_free`，容易内存泄漏。

---

### 3.3 综合方案（推荐）

考虑到实现复杂度和安全性，我推荐：

**Zig → Rust (参数)**: 采用 Phase 1 方案（修改 C ABI 签名，直接传递 ptr + len）

**Rust → Zig (返回值)**: 暂时保持现状（复制 1 次），因为：
1. 返回值的复制只在返回字符串时发生
2. 字符串返回值通常较小，复制开销可接受
3. 彻底零复制需要大量修改，且容易引入内存安全问题

**未来优化**: 如果返回值复制成为瓶颈，再实现 Phase 2 方案 2（所有权转移）。

---

## 4. 性能分析

### 4.1 当前复制开销

假设 Host 函数调用每秒 1000 次，每次传递 1KB 字符串：

- Zig → Rust 参数复制: 1000 次/秒 × 1KB = 1MB/秒
- Rust → Zig 返回值复制: 假设 50% 函数返回字符串，500 次/秒 × 1KB = 0.5MB/秒
- **总复制带宽**: 1.5MB/秒

### 4.2 零复制后开销

- Zig → Rust 参数: 0（零复制）
- Rust → Zig 返回值: 0.5MB/秒（暂时保持复制）
- **总复制带宽**: 0.5MB/秒（节省 66%）

### 4.3 延迟分析

复制 1KB 字符串的延迟（假设内存带宽 20GB/s）：
- 1KB 复制延迟: ~50ns
- 如果 Host 函数调用本身需要 1μs，复制占 5%
- **结论**: 对于小字符串，复制开销可能可以接受；对于大字符串（> 1MB），零复制更有价值

---

## 5. 风险和建议

### 5.1 风险

1. **生命周期错误**: 如果 Rust 侧保存了 Zig Arena 的指针，后续 Arena 轮换会导致 use-after-free
   - **缓解**: 文档明确说明指针是借用，函数返回后无效

2. **Breaking Change**: 修改 C ABI 签名会影响所有现有 Host 函数
   - **缓解**: 提供迁移指南，或者提供兼容层（同时支持新旧签名）

3. **线程安全**: 如果未来支持多线程，Arena 借用模式需要重新设计
   - **缓解**: 当前是单线程，未来多线程时可以采用其他方案

### 5.2 建议

1. **先实现 Zig → Rust 参数零复制**（Phase 1），因为：
   - 实现简单
   - 收益明显
   - 风险可控

2. **Rust → Zig 返回值暂时保持复制**，因为：
   - 实现复杂
   - 收益有限（返回值通常较小）
   - 容易引入内存安全问题

3. **提供 benchmark**，在修改前后测试性能，确保修改有价值

---

## 6. 下一步

1. **确认方案**: 与团队讨论，确认采用方案 A（Arena 借用模式）
2. **实现 Phase 1**: 修改 Zig → Rust 参数传递，消除 `dupeZ` 复制
3. **Benchmark**: 测试修改前后的性能差异
4. **文档更新**: 更新 `JS_FEATURE_EVALUATION.md` 和 `docs/`，描述新的零复制模式

---

**作者**: Jonathan Huang  
**日期**: 2026-06-23  
**版本**: 1.0 (Draft)
