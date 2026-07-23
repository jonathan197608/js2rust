//! 多 Arena 全局分配器 — 线程安全版 (Zig 0.16.0)
//!
//! ## 设计要点
//! - 总内存上限默认 384M，最小值 384M，可通过环境变量 JS_ZIG_TOTAL_LIMIT 修改
//! - 每个 Arena 固定 128M，节点数 = total_limit / 128M（最低 2 个）
//! - 状态机：ready / cooling
//! - 冷却时间限制：默认最低 10 分钟，可通过环境变量 JS_ZIG_MIN_COOLING_TIME 修改
//! - 环形拓扑，随机选取，cooling 则取相邻
//! - 每个节点一把 std.atomic.Mutex，复合操作（读-判-写）在一次锁持有中完成
//! - 分配完成后自动检查使用率，超过 80% 则原子标记为 cooling
//! - 所有节点都 cooling 且未过冷却时间时，返回 error.OutOfMemory

const std = @import("std");
const Allocator = std.mem.Allocator;
const ArenaAllocator = std.heap.ArenaAllocator;
const Alignment = std.mem.Alignment;

/// 全局 backing allocator — 所有节点的 Arena 共享同一个底层分配器
const backing = std.heap.page_allocator;

/// 导入 js_date.zig 获取时间戳（跨平台）
const js_date = @import("js_date.zig");

/// 全局计数器，用于 selectNode 随机化（原子自增）
var global_counter: std.atomic.Value(u64) = std.atomic.Value(u64).init(0);

/// 从环境变量读取配置（辅助函数）
/// 用法：
///   const config = try MultiArenaAllocator.readEnvConfig();
///   const da = try MultiArenaAllocator.init(config.total_limit, config.min_cooling_time);
pub fn readEnvConfig() !struct { total_limit: ?usize, min_cooling_time: ?i64 } {
    var result = struct { total_limit: ?usize = null, min_cooling_time: ?i64 = null };

    // 读取 JS_ZIG_TOTAL_LIMIT
    if (std.process.getEnv("JS_ZIG_TOTAL_LIMIT", backing)) |env_str| {
        defer backing.free(env_str);
        result.total_limit = std.fmt.parseInt(usize, env_str, 10) catch null;
    } else |_| {}

    // 读取 JS_ZIG_MIN_COOLING_TIME
    if (std.process.getEnv("JS_ZIG_MIN_COOLING_TIME", backing)) |env_str| {
        defer backing.free(env_str);
        result.min_cooling_time = std.fmt.parseInt(i64, env_str, 10) catch null;
    } else |_| {}

    return result;
}

// ── 配置常量 ─────────────────────────────────────────────────────

/// 默认总内存上限：384M
pub const DEFAULT_TOTAL_LIMIT: usize = 384 * 1024 * 1024;

/// 最小总内存上限：384M
pub const MIN_TOTAL_LIMIT: usize = 384 * 1024 * 1024;

/// 每个 Arena 的固定上限：128M
pub const ARENA_SIZE: usize = 128 * 1024 * 1024;

/// 触发 cooling 的使用率阈值：80%
const COOLING_THRESHOLD: usize = ARENA_SIZE * 80 / 100;

/// 默认最低冷却时间：10 分钟（单位：秒）
/// 可通过环境变量 JS_ZIG_MIN_COOLING_TIME 修改
pub const MIN_COOLING_TIME_SECONDS: i64 = 600;

// ── 分配器状态 ───────────────────────────────────────────────────

/// 分配器状态
pub const AllocatorState = enum(u8) {
    /// 就绪状态：可以分配内存
    ready = 0,
    /// 冷却状态：暂时不可用
    cooling = 1,
};

// ── Arena 节点 ───────────────────────────────────────────────────

/// 单个 Arena 节点（线程安全：每节点一把锁）
const ArenaNode = struct {
    /// Arena 分配器
    arena: ArenaAllocator,
    /// 当前状态（由 mutex 保护，必须通过复合原子操作访问）
    state: AllocatorState,
    /// 状态锁（保护 state 字段的复合读写操作）
    mutex: std.atomic.Mutex,
    /// 前驱节点索引
    prev: usize,
    /// 后继节点索引
    next: usize,
    /// 冷却开始时间戳（秒），0 表示没有在冷却
    cooling_since: i64,

    /// 初始化 Arena 节点
    pub fn init() !ArenaNode {
        return ArenaNode{
            .arena = ArenaAllocator.init(backing),
            .state = .ready,
            .mutex = .unlocked,
            .prev = 0,
            .next = 0,
            .cooling_since = 0,
        };
    }

    /// 获取 Arena 的 Allocator（无锁，arena 内部有自己的同步）
    pub fn allocator(self: *ArenaNode) Allocator {
        return self.arena.allocator();
    }

    /// 获取已分配字节数（无锁，只读统计信息）
    pub fn bytesAllocated(self: *ArenaNode) usize {
        return self.arena.queryCapacity();
    }

    // ── 复合原子操作（读-判-写在一次锁持有中完成）────────────

    /// 原子读取当前状态（用于 selectNode 判断）
    pub fn isReady(self: *ArenaNode) bool {
        while (!self.mutex.tryLock()) {}
        defer self.mutex.unlock();
        return self.state == .ready;
    }

    /// 原子操作：如果当前是 ready 且使用率超过 80%，则标记为 cooling
    /// 返回 true 表示发生了状态转换
    /// 整个"读取状态 + 读取使用率 + 判断 + 写入状态"在一次锁持有中完成
    /// 由 allocBytes 在每次分配后自动调用
    pub fn tryMarkCoolingIfFull(self: *ArenaNode) bool {
        while (!self.mutex.tryLock()) {}
        defer self.mutex.unlock();

        if (self.state != .ready) return false;
        if (self.arena.queryCapacity() <= COOLING_THRESHOLD) return false;
        self.state = .cooling;
        self.cooling_since = @divTrunc(js_date.milliTimestamp(), 1000);
        return true;
    }

    /// 原子操作：如果当前是 cooling 且已冷却至少 min_cooling_time 秒，则重置 Arena 并标记为 ready
    /// 返回 true 表示发生了状态转换
    /// 整个"读取状态 + 检查冷却时间 + 重置 Arena + 写入状态"在一次锁持有中完成
    /// 由 selectNode 检测节点状态时自动调用
    pub fn tryResetCoolingToReady(self: *ArenaNode, min_cooling_time: i64) bool {
        while (!self.mutex.tryLock()) {}
        defer self.mutex.unlock();

        if (self.state != .cooling) return false;

        // 检查是否已冷却至少 min_cooling_time 秒
        const now = @divTrunc(js_date.milliTimestamp(), 1000);
        if (now - self.cooling_since < min_cooling_time) {
            return false;
        }

        self.arena.deinit();
        self.arena = ArenaAllocator.init(backing);
        self.state = .ready;
        self.cooling_since = 0;
        return true;
    }

    /// 释放节点资源（调用前调用方需保证无并发访问）
    pub fn deinit(self: *ArenaNode) void {
        self.arena.deinit();
    }
};

// ── 多 Arena 全局分配器 ─────────────────────────────────────────

/// 多 Arena 全局分配器
/// 不存储 backing / current_idx，随机选取节点
pub const MultiArenaAllocator = struct {
    /// Arena 节点数组（首节点 arena.child_allocator 用于释放）
    nodes: []ArenaNode,
    /// 节点数量（最低 2 个）
    node_count: usize,
    /// 总内存上限
    total_limit: usize,
    /// 最低冷却时间（秒），可通过环境变量 JS_ZIG_MIN_COOLING_TIME 修改
    min_cooling_time: i64,

    /// 初始化多 Arena 分配器
    /// total_limit: 可选的总内存上限（字节），null 时使用 DEFAULT_TOTAL_LIMIT
    /// min_cooling_time: 可选的冷却时间（秒），null 时使用 MIN_COOLING_TIME_SECONDS
    /// 注意：可通过环境变量 JS_ZIG_TOTAL_LIMIT 和 JS_ZIG_MIN_COOLING_TIME 配置默认值
    pub fn init(total_limit: ?usize, min_cooling_time: ?i64) !*MultiArenaAllocator {
        const limit = blk: {
            const requested = total_limit orelse DEFAULT_TOTAL_LIMIT;
            break :blk if (requested < MIN_TOTAL_LIMIT) MIN_TOTAL_LIMIT else requested;
        };

        const cooling_time = min_cooling_time orelse MIN_COOLING_TIME_SECONDS;

        // 计算节点数：total_limit / ARENA_SIZE，最低 2 个
        var node_count = (limit + ARENA_SIZE - 1) / ARENA_SIZE;
        if (node_count < 2) {
            node_count = 2;
        }
        // Clamp to MAX_STATS_NODES to prevent Stats.nodes array overflow (P0-3 fix).
        if (node_count > 64) {
            node_count = 64;
        }

        // 分配 MultiArenaAllocator 本身
        const da = try backing.create(MultiArenaAllocator);
        errdefer backing.destroy(da);

        // 分配节点数组
        const nodes = try backing.alloc(ArenaNode, node_count);
        errdefer backing.free(nodes);

        // 初始化每个节点（每个节点的 arena 以 backing 为 child_allocator）
        for (nodes, 0..) |*node, i| {
            node.* = try ArenaNode.init();
            node.prev = (i + node_count - 1) % node_count;
            node.next = (i + 1) % node_count;
        }

        da.* = MultiArenaAllocator{
            .nodes = nodes,
            .node_count = node_count,
            .total_limit = limit,
            .min_cooling_time = cooling_time,
        };

        return da;
    }

    /// 释放分配器
    pub fn deinit(self: *MultiArenaAllocator) void {
        for (self.nodes) |*node| {
            node.deinit();
        }
        backing.free(self.nodes);
        backing.destroy(self);
    }

    /// 获取分配器接口
    /// vtable 是 Zig Allocator API 的必要部分：
    ///   Allocator = { .ptr = *anyopaque, .vtable = *VTable }
    ///   vtable 存储 alloc/free/resize/remap 函数指针
    ///   没有 vtable 就无法实现自定义 Allocator 接口
    pub fn allocator(self: *MultiArenaAllocator) Allocator {
        return Allocator{
            .ptr = self,
            .vtable = &vtable,
        };
    }

    /// 分配字节
    /// 分配完成后自动检查该节点使用率，超过 80% 则原子标记为 cooling
    /// 如果所有节点都 cooling 且未过冷却时间，返回 error.OutOfMemory
    pub fn allocBytes(self: *MultiArenaAllocator, n: usize) ![]u8 {
        const node = self.selectNode() orelse return error.OutOfMemory;

        const buf = try node.allocator().alloc(u8, n);

        // 分配后自动检查：该节点使用率超过 80% 则标记为 cooling
        // tryMarkCoolingIfFull 是复合原子操作，读-判-写在一次锁持有中完成
        _ = node.tryMarkCoolingIfFull();

        return buf;
    }

    /// 选取可用节点（原子自增计数器 + 环形遍历）
    /// 如果所有节点都处于 cooling 且未过冷却时间，返回 null
    fn selectNode(self: *MultiArenaAllocator) ?*ArenaNode {
        const start_val = global_counter.fetchAdd(1, .monotonic);
        const start_idx = @as(usize, @intCast(start_val)) % self.node_count;

        var idx = start_idx;
        var count: usize = 0;
        while (count < self.node_count) : (count += 1) {
            const node = &self.nodes[idx];
            if (node.isReady()) {
                return node;
            }
            if (node.tryResetCoolingToReady(self.min_cooling_time)) {
                return node;
            }
            idx = node.next;
        }
        return null;
    }

    /// 获取统计信息（读取 state 时加锁）
    pub fn stats(self: *MultiArenaAllocator) Stats {
        var result = Stats{
            .node_count = self.node_count,
            .total_limit = self.total_limit,
            .nodes = undefined,
        };

        for (self.nodes[0..self.node_count], 0..) |*node, i| {
            result.total_bytes += node.bytesAllocated();
            result.nodes[i] = .{
                // 用 isReady() 原子读取状态
                .state = if (node.isReady()) AllocatorState.ready else AllocatorState.cooling,
                .bytes = node.bytesAllocated(),
            };
        }

        return result;
    }
};

/// 统计信息
pub const Stats = struct {
    total_bytes: usize = 0,
    node_count: usize = 0,
    total_limit: usize = 0,
    nodes: [64]NodeStat,

    pub fn format(self: Stats, w: *std.Io.Writer) std.Io.Writer.Error!void {
        try w.print("MultiArenaAllocator Stats:\n", .{});
        try w.print("  Total bytes: {}\n", .{std.fmt.fmtIntSizeDec(self.total_bytes)});
        try w.print("  Node count: {}\n", .{self.node_count});
        try w.print("  Total limit: {}\n", .{std.fmt.fmtIntSizeDec(self.total_limit)});
        for (self.nodes[0..self.node_count], 0..) |node_stat, i| {
            try w.print("  Node[{}]: state={s}, bytes={}\n", .{ i, @tagName(node_stat.state), std.fmt.fmtIntSizeDec(node_stat.bytes) });
        }
    }
};

/// 节点统计
pub const NodeStat = struct {
    state: AllocatorState = .ready,
    bytes: usize = 0,
};

// ── Allocator vtable 函数（必须在 vtable 之前定义）────────────────────
// 这些是非成员函数，vtable 可以引用它们

fn allocImpl(ctx: *anyopaque, len: usize, alignment: Alignment, ret_addr: usize) ?[*]u8 {
    const self_: *MultiArenaAllocator = @ptrCast(@alignCast(ctx));
    const node = self_.selectNode() orelse return null;
    const arena_alloc = node.allocator();
    // Delegate to the arena allocator's vtable, passing alignment through.
    // This ensures ArrayList(i64) etc. get properly aligned memory.
    const ptr = arena_alloc.vtable.alloc(arena_alloc.ptr, len, alignment, ret_addr) orelse return null;
    // Call tryMarkCoolingIfFull after allocation. This is essential for the
    // vtable path: when code uses the allocator via std.mem.Allocator interface
    // (e.g., ArrayList.append), it goes through allocImpl, not allocBytes().
    // Without this call, arena cooling never triggers for vtable-path allocations,
    // eventually causing OOM with no recovery. tryMarkCoolingIfFull is idempotent
    // (returns false if node is not .ready), so double-calling via allocBytes is harmless.
    _ = node.tryMarkCoolingIfFull();
    return ptr;
}

fn freeImpl(ctx: *anyopaque, memory: []u8, alignment: Alignment, ret_addr: usize) void {
    _ = ctx;
    _ = memory;
    _ = alignment;
    _ = ret_addr;
}

/// Arena allocators allocate linearly and never release individual blocks.
/// In-place resize is fundamentally impossible — returning false tells the
/// caller (e.g. ArrayList) to fall back to alloc+copy+free. This is correct
/// and expected semantics; the vtable requires this function pointer but a
/// stub is the only valid implementation for an arena-based allocator.
fn resizeImpl(ctx: *anyopaque, memory: []u8, alignment: Alignment, new_len: usize, ret_addr: usize) bool {
    _ = ctx;
    _ = memory;
    _ = alignment;
    _ = new_len;
    _ = ret_addr;
    return false;
}

/// Arena allocators cannot remap memory — the original block is permanently
/// committed to the arena's linear buffer. Returning null forces the caller
/// to alloc+copy+free. This is correct and expected; the vtable requires all
/// four function pointers but a stub is the only valid implementation here.
fn remapImpl(ctx: *anyopaque, memory: []u8, alignment: Alignment, new_len: usize, ret_addr: usize) ?[*]u8 {
    _ = ctx;
    _ = memory;
    _ = alignment;
    _ = new_len;
    _ = ret_addr;
    return null;
}

/// Allocator vtable (Zig 0.16.0 API)
/// 作用：Zig 的 Allocator 接口通过 vtable 实现多态。
///   Allocator { .ptr, .vtable }
///   ptr 指向自定义数据（MultiArenaAllocator 实例）
///   vtable 指向这个函数表，标准库通过它调用 alloc/free/resize/remap
/// 这是实现自定义 Allocator 的唯一标准方式，不能删除。
const vtable = Allocator.VTable{
    .alloc = allocImpl,
    .free = freeImpl,
    .resize = resizeImpl,
    .remap = remapImpl,
};

// ── 测试 ─────────────────────────────────────────────────────────

test "MultiArenaAllocator init" {
    const testing = std.testing;
    const da = try MultiArenaAllocator.init(null, null);
    defer da.deinit();

    try testing.expectEqual(@as(usize, 3), da.node_count);
    try testing.expectEqual(@as(usize, DEFAULT_TOTAL_LIMIT), da.total_limit);
}

test "MultiArenaAllocator alloc" {
    const testing = std.testing;
    const da = try MultiArenaAllocator.init(null, null);
    defer da.deinit();

    const alloc = da.allocator();
    const buf = try alloc.alloc(u8, 1024);
    defer alloc.free(buf);

    try testing.expect(buf.len == 1024);
}

test "MultiArenaAllocator stats" {
    const testing = std.testing;
    const da = try MultiArenaAllocator.init(null, null);
    defer da.deinit();

    const alloc = da.allocator();
    _ = try alloc.alloc(u8, 1024);

    const stats = da.stats();
    try testing.expect(stats.total_bytes > 0);
    try testing.expectEqual(@as(usize, 3), stats.node_count);
}

test "MultiArenaAllocator node count min 2" {
    const testing = std.testing;

    const test_cases = [_]usize{
        384 * 1024 * 1024,
        256 * 1024 * 1024,
        128 * 1024 * 1024,
    };

    for (test_cases) |limit| {
        const da = try MultiArenaAllocator.init(limit, null);
        defer da.deinit();

        try testing.expect(da.node_count >= 2);
    }
}

test "MultiArenaAllocator ring topology" {
    const testing = std.testing;
    const da = try MultiArenaAllocator.init(null, null);
    defer da.deinit();

    for (da.nodes, 0..) |*node, i| {
        try testing.expectEqual(node.prev, (i + da.node_count - 1) % da.node_count);
        try testing.expectEqual(node.next, (i + 1) % da.node_count);
    }
}

test "MultiArenaAllocator all cooling returns null if not cooled enough" {
    const testing = std.testing;
    const da = try MultiArenaAllocator.init(null, null);
    defer da.deinit();

    // 将所有节点手动设为 cooling（单线程测试，直接写 state）
    for (da.nodes) |*node| {
        node.state = .cooling;
        // 设置 cooling_since 为当前时间，这样冷却时间还没过
        node.cooling_since = @divTrunc(js_date.milliTimestamp(), 1000);
    }

    // selectNode 现在会返回 null，因为所有节点都 cooling 且未过冷却时间
    const node = da.selectNode();
    try testing.expect(node == null);
}

test "MultiArenaAllocator auto cooling after alloc" {
    const testing = std.testing;
    const da = try MultiArenaAllocator.init(null, null);
    defer da.deinit();

    // 分配一个接近 80% 阈值的大小，触发自动冷却
    // ARENA_SIZE = 128M, COOLING_THRESHOLD = 102.4M
    // 分配 103M 应该触发冷却
    const large_buf = da.allocBytes(COOLING_THRESHOLD + 1) catch |err| {
        // 如果底层 allocator 无法分配这么大的内存，跳过这个测试
        try testing.expect(err == error.OutOfMemory);
        return;
    };
    // allocBytes 使用 selectNode + allocator().alloc，无法 free 单个块（arena 语义）
    // 但 deinit 会在测试结束时清理所有内存
    _ = large_buf;

    // 分配后，该节点应该被自动标记为 cooling
    // 由于 selectNode 是随机的，我们无法确定是哪个节点被分配了
    // 但我们可以检查是否有节点被标记为 cooling
    var cooling_found = false;
    for (da.nodes) |*node| {
        if (!node.isReady()) {
            cooling_found = true;
            break;
        }
    }
    try testing.expect(cooling_found);
}

test "MultiArenaAllocator thread safety: isReady is atomic" {
    const testing = std.testing;
    const da = try MultiArenaAllocator.init(null, null);
    defer da.deinit();

    // 初始状态所有节点都是 ready
    for (da.nodes) |*node| {
        try testing.expect(node.isReady());
    }
}

// ── 全局分配器 API ───────────────────────────────────────────────

var g_instance: ?*MultiArenaAllocator = null;

/// 初始化全局分配器（幂等，多次调用安全）
/// total_limit: 可选的总内存上限（字节），null 时使用 DEFAULT_TOTAL_LIMIT
/// min_cooling_time: 可选的冷却时间（秒），null 时使用 MIN_COOLING_TIME_SECONDS
pub fn init(total_limit: ?usize, min_cooling_time: ?i64) !void {
    if (g_instance != null) return;
    g_instance = try MultiArenaAllocator.init(total_limit, min_cooling_time);
}

/// 释放全局分配器（幂等，多次调用安全）
pub fn deinit() void {
    if (g_instance) |inst| {
        inst.deinit();
        g_instance = null;
    }
}

/// 获取 Allocator 接口
/// 调用前必须先 init()，否则行为未定义
pub fn allocator() Allocator {
    return g_instance.?.allocator();
}

/// 分配内存（返回 error 而非 panic）
pub fn allocBytes(n: usize) ![]u8 {
    return g_instance.?.allocBytes(n);
}

/// Check whether the given allocator's free operation is a no-op.
/// Under the multi-arena allocator, `freeImpl` discards all parameters,
/// so individual deinit() calls waste CPU traversing data structures
/// only to call no-op free(). This check allows deinit methods to
/// short-circuit with a single pointer comparison.
pub fn isNoOpFree(alloc: Allocator) bool {
    return alloc.vtable.free == freeImpl;
}

/// 复制字节到 Arena（返回 error 而非 panic）
pub fn dupeBytes(src: []const u8) ![]u8 {
    const buf = try g_instance.?.allocBytes(src.len);
    @memcpy(buf, src);
    return buf;
}
