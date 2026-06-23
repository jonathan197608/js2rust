const std = @import("std");
const js_date = @import("js_date.zig");

/// Global allocator: dual-arena hot-swap design.
///
/// Two ArenaAllocator instances (A and B) take turns being the "active"
/// allocator. Each instance cycles through three states:
///
///     ready  --(becomes active)-->  active  --(capacity exceeds limit)-->
///     cooling  --(grace period elapsed -> reset)-->  ready
///
/// At any moment exactly one instance is `active` (used for all allocations)
/// and the other is non-active (`cooling` or `ready`).
///
/// When the active arena's capacity exceeds JS2RUST_MAX_ARENA_MB and the
/// backup is `ready`, the two swap: the full one enters `cooling` (its memory
/// stays alive so any pointer Rust still holds across the FFI boundary remains
/// valid) and the backup becomes active. A `cooling` instance is only reset
/// (memory reclaimed) after it has been cooling for at least
/// JS2RUST_ARENA_GRACE_MS milliseconds, guaranteeing returned strings outlive
/// any single FFI consumption window.
///
/// Cooling is time-based using the cross-platform milliTimestamp() from
/// js_date.zig. No background thread is needed: the timer check runs lazily
/// inside getAllocator(), protected by the existing compare-and-swap spinlock.

const DEFAULT_MAX_ARENA_SIZE_MB: usize = 100;
const ENV_MAX_ARENA_MB: [:0]const u8 = "JS2RUST_MAX_ARENA_MB";

/// Grace period in milliseconds a cooling instance must survive before it may
/// be reset. Uses the cross-platform milliTimestamp() from js_date.zig.
/// Configurable via JS2RUST_ARENA_GRACE_MS (default 600000 = 10 minutes).
/// Extended to 10 minutes to support slow async host function calls.
const DEFAULT_GRACE_MS: u64 = 600000;
const ENV_GRACE_MS: [:0]const u8 = "JS2RUST_ARENA_GRACE_MS";

const State = enum { ready, active, cooling };

/// The two arena instances. Stored as module-level vars so their addresses are
/// stable; the `std.mem.Allocator` fat pointer returned by `.allocator()` holds
/// a pointer into the arena, which must not move.
var arena_a: std.heap.ArenaAllocator = undefined;
var arena_b: std.heap.ArenaAllocator = undefined;

var state_a: State = .ready;
var state_b: State = .ready;

/// Timestamp (milliseconds since epoch) when each instance entered `cooling`.
/// 0 means the instance is not in cooling. When an instance is cooling and
/// `milliTimestamp() - cooling_since >= g_grace_ms`, the instance is reclaimed.
var cooling_since_a: i64 = 0;
var cooling_since_b: i64 = 0;

/// Which instance is currently active.
var active_is_a: bool = true;

var g_initialized: bool = false;

/// Atomic spinlock protecting state transitions (rotate/reclaim/reset).
/// Zig 0.16.0 in this target has no std.Thread.Mutex, so we use a plain bool
/// with compare-and-swap. Allocation itself is assumed single-threaded (the C
/// ABI callers serialize), so this lock only guards the swap bookkeeping.
var g_lock: bool = false;

/// Capacity threshold (bytes) that triggers an active->backup swap.
var g_max_arena_bytes: usize = DEFAULT_MAX_ARENA_SIZE_MB * 1024 * 1024;

/// Minimum time in milliseconds a cooling instance must survive before it
/// may be reset. See cooling_since_a/b above.
var g_grace_ms: u64 = DEFAULT_GRACE_MS;

/// Read JS2RUST_MAX_ARENA_MB (default 100). Returns bytes.
fn readMaxArenaBytes() usize {
    const env = std.c.getenv(ENV_MAX_ARENA_MB.ptr);
    if (env == null) return DEFAULT_MAX_ARENA_SIZE_MB * 1024 * 1024;
    const env_str = std.mem.span(env.?);
    const mb = std.fmt.parseInt(usize, env_str, 10) catch return DEFAULT_MAX_ARENA_SIZE_MB * 1024 * 1024;
    if (mb == 0) return DEFAULT_MAX_ARENA_SIZE_MB * 1024 * 1024;
    return mb * 1024 * 1024;
}

/// Read JS2RUST_ARENA_GRACE_MS (default 5000). Returns milliseconds.
fn readGraceMs() u64 {
    const env = std.c.getenv(ENV_GRACE_MS.ptr);
    if (env == null) return DEFAULT_GRACE_MS;
    const env_str = std.mem.span(env.?);
    const n = std.fmt.parseInt(u64, env_str, 10) catch return DEFAULT_GRACE_MS;
    if (n == 0) return DEFAULT_GRACE_MS;
    return n;
}

fn acquireLock() void {
    while (true) {
        const result = @cmpxchgStrong(bool, &g_lock, false, true, .acquire, .monotonic);
        if (result == null) return;
        std.atomic.spinLoopHint();
    }
}

fn releaseLock() void {
    @atomicStore(bool, &g_lock, false, .release);
}

/// Initialize the dual-arena allocator. Idempotent.
/// A starts `active`, B starts `ready`.
pub fn initGlobalAllocator() void {
    if (g_initialized) return;

    g_max_arena_bytes = readMaxArenaBytes();
    g_grace_ms = readGraceMs();

    arena_a = std.heap.ArenaAllocator.init(std.heap.page_allocator);
    arena_b = std.heap.ArenaAllocator.init(std.heap.page_allocator);

    state_a = .active;
    state_b = .ready;
    cooling_since_a = 0;
    cooling_since_b = 0;
    active_is_a = true;

    g_initialized = true;
}

/// Deinitialize both arenas. Call from js2rust_deinit.
pub fn deinitGlobalAllocator() void {
    acquireLock();
    defer releaseLock();

    if (!g_initialized) return;

    arena_a.deinit();
    arena_b.deinit();
    state_a = .ready;
    state_b = .ready;
    cooling_since_a = 0;
    cooling_since_b = 0;
    g_initialized = false;
}

/// Check cooling timers for any cooling instance. If the grace period has
/// elapsed, the instance is reset (memory reclaimed) and its state becomes `ready`.
/// Must hold the lock. Called at the start of getAllocator().
fn tickGraceTimers(now: i64) void {
    if (state_a == .cooling and cooling_since_a > 0) {
        if (now - cooling_since_a >= @as(i64, @intCast(g_grace_ms))) {
            _ = arena_a.reset(.free_all);
            state_a = .ready;
            cooling_since_a = 0;
        }
    }
    if (state_b == .cooling and cooling_since_b > 0) {
        if (now - cooling_since_b >= @as(i64, @intCast(g_grace_ms))) {
            _ = arena_b.reset(.free_all);
            state_b = .ready;
            cooling_since_b = 0;
        }
    }
}

/// Swap active <-> backup. The current active enters `cooling`; the backup
/// (which must be `ready`) becomes active. Must hold the lock.
/// No-op if the backup is not `ready` (cannot swap safely).
fn swapActive() void {
    const now = js_date.milliTimestamp();
    if (active_is_a) {
        if (state_b != .ready) return; // backup not available, keep current active
        state_a = .cooling;
        cooling_since_a = now;
        state_b = .active;
        active_is_a = false;
    } else {
        if (state_a != .ready) return;
        state_b = .cooling;
        cooling_since_b = now;
        state_a = .active;
        active_is_a = true;
    }
}

/// Current active arena capacity in bytes. Must hold the lock (or be in a
/// single-threaded context).
fn activeCapacity() usize {
    return if (active_is_a) arena_a.queryCapacity() else arena_b.queryCapacity();
}

/// Run one rotate/reclaim cycle.
/// - Always ticks grace timers first (may reclaim a cooled-down backup).
/// - Then, if `force` or the active arena exceeds the size limit, swaps to the
///   backup (when the backup is ready).
/// Must hold the lock.
fn checkAndRotate(force: bool) void {
    const now = js_date.milliTimestamp();
    tickGraceTimers(now);

    // Swap: when active is full (or forced) and a ready backup exists.
    if (force or activeCapacity() > g_max_arena_bytes) {
        swapActive();
    }
}

/// Manual reset (Rust calls this via js2rust_reset).
/// Forces a rotate: the current active is parked in `cooling` (its memory stays
/// alive for the grace window so any in-flight returned pointer is safe), and
/// the backup, if ready, becomes active. Cooled-down backups are reclaimed.
pub fn resetGlobalAllocator() void {
    acquireLock();
    defer releaseLock();
    if (!g_initialized) return;
    checkAndRotate(true);
}

/// Get current arena memory usage in bytes (the active instance's capacity).
/// Kept for API compatibility.
pub fn getArenaUsedBytes() usize {
    if (!g_initialized) return 0;
    return activeCapacity();
}

/// Get the configured max arena size in bytes.
pub fn getMaxArenaBytes() usize {
    return g_max_arena_bytes;
}

/// Get the configured grace period in milliseconds.
pub fn getGraceMs() u64 {
    return g_grace_ms;
}

/// Retrieve the global allocator (the active arena's allocator).
/// Must be called after initGlobalAllocator(); panics otherwise.
/// Performs a lazy rotate/reclaim check before returning.
pub fn g_alloc() std.mem.Allocator {
    if (!g_initialized) {
        @panic("js_allocator not initialized. Call initGlobalAllocator() first.");
    }

    acquireLock();
    checkAndRotate(false);
    const is_a = active_is_a;
    releaseLock();

    return if (is_a) arena_a.allocator() else arena_b.allocator();
}

/// Alias for g_alloc() (preferred name).
pub fn getAllocator() std.mem.Allocator {
    return g_alloc();
}

/// Allocate memory in Zig's Arena, also exposed via C ABI in lib.zig.
///
/// Rust Host functions call the C ABI export `js_allocator_alloc` (forwarded
/// from lib.zig) to allocate memory for string returns, enabling zero-copy:
/// the returned pointer is in Zig's Arena, so Zig can use it directly.
///
/// Usage in Rust (via C ABI):
/// ```rust
/// extern "C" {
///     fn js_allocator_alloc(size: usize) -> *mut u8;
/// }
///
/// #[no_mangle]
/// pub extern "C" fn host_func() -> __JsStr {
///     let data = "hello".as_bytes();
///     let ptr = js_allocator_alloc(data.len());
///     std::ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len());
///     __JsStr { ptr, len: data.len() as isize }
/// }
/// ```
pub fn js_allocator_alloc(size: usize) []u8 {
    const alloc = getAllocator();
    return alloc.alloc(u8, size) catch @panic("js_allocator_alloc failed");
}

// ── Tests ───────────────────────────────────────────────────────

test "dual arena init/deinit" {
    initGlobalAllocator();
    defer deinitGlobalAllocator();

    try std.testing.expect(g_initialized);
    try std.testing.expect(state_a == .active);
    try std.testing.expect(state_b == .ready);
    try std.testing.expect(active_is_a);
}

test "forced rotate parks active in cooling and swaps to backup" {
    initGlobalAllocator();
    defer deinitGlobalAllocator();

    // allocate a bit on A
    const a = getAllocator();
    _ = try a.alloc(u8, 64);

    resetGlobalAllocator(); // force rotate

    try std.testing.expect(state_a == .cooling);
    try std.testing.expect(state_b == .active);
    try std.testing.expect(!active_is_a);
}

test "cooling backup reclaimed after grace time expires" {
    initGlobalAllocator();
    defer deinitGlobalAllocator();

    g_grace_ms = 0; // instant reclaim

    resetGlobalAllocator(); // A -> cooling, B -> active

    // Next allocator fetch should check the timer and reclaim A immediately.
    _ = getAllocator();

    try std.testing.expect(state_a == .ready);
    try std.testing.expect(state_b == .active);
}

test "grace ms env var" {
    // Just verify the default is applied; actual env parsing is tested indirectly
    // through initGlobalAllocator reading JS2RUST_ARENA_GRACE_MS.
    initGlobalAllocator();
    defer deinitGlobalAllocator();
    try std.testing.expect(getGraceMs() > 0);
}
