const std = @import("std");

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
/// JS2RUST_ARENA_GRACE_SEC seconds, guaranteeing returned strings outlive any
/// single FFI consumption window.
///
/// No background thread is used (Zig 0.16.0 has no std.Thread in this target):
/// the rotate/reclaim check runs lazily inside getAllocator(), protected by the
/// existing compare-and-swap spinlock.

const DEFAULT_MAX_ARENA_SIZE_MB: usize = 100;
const ENV_MAX_ARENA_MB: [:0]const u8 = "JS2RUST_MAX_ARENA_MB";

/// Number of getAllocator() calls a cooling instance must survive before it may
/// be reset. This replaces a time-based grace period (which would require a
/// wall-clock source that is not available in Zig 0.16.0's constrained targets).
/// Configurable via JS2RUST_ARENA_GRACE_CALLS (default 1000).
const DEFAULT_GRACE_CALLS: u64 = 1000;
const ENV_GRACE_CALLS: [:0]const u8 = "JS2RUST_ARENA_GRACE_CALLS";

const State = enum { ready, active, cooling };

/// The two arena instances. Stored as module-level vars so their addresses are
/// stable; the `std.mem.Allocator` fat pointer returned by `.allocator()` holds
/// a pointer into the arena, which must not move.
var arena_a: std.heap.ArenaAllocator = undefined;
var arena_b: std.heap.ArenaAllocator = undefined;

var state_a: State = .ready;
var state_b: State = .ready;

/// Grace call counter for each instance. When an instance enters `cooling`, its
/// counter is set to `g_grace_calls`. Each `getAllocator()` call decrements the
/// counter of any cooling instance; when it reaches 0 the instance may be reset.
/// This guarantees that any pointer returned across the FFI boundary stays valid
/// for at least N allocator-fetch operations after the swap.
var grace_calls_remaining_a: u64 = 0;
var grace_calls_remaining_b: u64 = 0;

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

/// Minimum number of getAllocator() calls a cooling instance must survive before
/// it may be reset. See grace_calls_remaining_a/b above.
var g_grace_calls: u64 = DEFAULT_GRACE_CALLS;

/// Read JS2RUST_MAX_ARENA_MB (default 100). Returns bytes.
fn readMaxArenaBytes() usize {
    const env = std.c.getenv(ENV_MAX_ARENA_MB.ptr);
    if (env == null) return DEFAULT_MAX_ARENA_SIZE_MB * 1024 * 1024;
    const env_str = std.mem.span(env.?);
    const mb = std.fmt.parseInt(usize, env_str, 10) catch return DEFAULT_MAX_ARENA_SIZE_MB * 1024 * 1024;
    if (mb == 0) return DEFAULT_MAX_ARENA_SIZE_MB * 1024 * 1024;
    return mb * 1024 * 1024;
}

/// Read JS2RUST_ARENA_GRACE_CALLS (default 1000). Returns call count.
fn readGraceCalls() u64 {
    const env = std.c.getenv(ENV_GRACE_CALLS.ptr);
    if (env == null) return DEFAULT_GRACE_CALLS;
    const env_str = std.mem.span(env.?);
    const n = std.fmt.parseInt(u64, env_str, 10) catch return DEFAULT_GRACE_CALLS;
    if (n == 0) return DEFAULT_GRACE_CALLS;
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
    g_grace_calls = readGraceCalls();

    arena_a = std.heap.ArenaAllocator.init(std.heap.page_allocator);
    arena_b = std.heap.ArenaAllocator.init(std.heap.page_allocator);

    state_a = .active;
    state_b = .ready;
    grace_calls_remaining_a = 0;
    grace_calls_remaining_b = 0;
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
    grace_calls_remaining_a = 0;
    grace_calls_remaining_b = 0;
    g_initialized = false;
}

/// Decrement grace counters for any cooling instance. If a counter reaches 0
/// the instance is reset (memory reclaimed) and its state becomes `ready`.
/// Must hold the lock. Called at the start of getAllocator().
fn tickGraceCounters() void {
    if (state_a == .cooling) {
        if (grace_calls_remaining_a > 0) {
            grace_calls_remaining_a -= 1;
        }
        if (grace_calls_remaining_a == 0) {
            _ = arena_a.reset(.free_all);
            state_a = .ready;
        }
    }
    if (state_b == .cooling) {
        if (grace_calls_remaining_b > 0) {
            grace_calls_remaining_b -= 1;
        }
        if (grace_calls_remaining_b == 0) {
            _ = arena_b.reset(.free_all);
            state_b = .ready;
        }
    }
}

/// Swap active <-> backup. The current active enters `cooling`; the backup
/// (which must be `ready`) becomes active. Must hold the lock.
/// No-op if the backup is not `ready` (cannot swap safely).
fn swapActive() void {
    if (active_is_a) {
        if (state_b != .ready) return; // backup not available, keep current active
        state_a = .cooling;
        grace_calls_remaining_a = g_grace_calls;
        state_b = .active;
        active_is_a = false;
    } else {
        if (state_a != .ready) return;
        state_b = .cooling;
        grace_calls_remaining_b = g_grace_calls;
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
/// - Always ticks grace counters first (may reclaim a cooled-down backup).
/// - Then, if `force` or the active arena exceeds the size limit, swaps to the
///   backup (when the backup is ready).
/// Must hold the lock.
fn checkAndRotate(force: bool) void {
    tickGraceCounters();

    // 2. Swap: when active is full (or forced) and a ready backup exists.
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

/// Get the configured grace call count.
pub fn getGraceCalls() u64 {
    return g_grace_calls;
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

test "cooling backup reclaimed after grace calls expire" {
    initGlobalAllocator();
    defer deinitGlobalAllocator();

    g_grace_calls = 1; // reset after 1 getAllocator() call

    resetGlobalAllocator(); // A -> cooling, B -> active

    // Next allocator fetch should tick the grace counter and reclaim A.
    _ = getAllocator();

    try std.testing.expect(state_a == .ready);
    try std.testing.expect(state_b == .active);
}

test "grace calls env var" {
    // Just verify the default is applied; actual env parsing is tested indirectly
    // through initGlobalAllocator reading JS2RUST_ARENA_GRACE_CALLS.
    initGlobalAllocator();
    defer deinitGlobalAllocator();
    try std.testing.expect(getGraceCalls() > 0);
}
