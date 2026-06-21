const std = @import("std");

/// Configuration for the global allocator.
/// Memory limit can be set via environment variable JS2RUST_MAX_ARENA_MB (default: 100 MB).
/// When the arena size exceeds this limit, an automatic reset is triggered.
const DEFAULT_MAX_ARENA_SIZE_MB: usize = 100;
const ENV_MAX_ARENA_MB: [:0]const u8 = "JS2RUST_MAX_ARENA_MB";

/// Global allocator storage.
/// Uses ArenaAllocator (Zig 0.16.0: lock-free, thread-safe for allocations).
/// All generated code that needs heap allocation should call `g_alloc()`.
var g_allocator: ?std.mem.Allocator = null;
var g_arena: ?std.heap.ArenaAllocator = null;

/// Atomic lock for thread-safe reset operations.
/// Uses spinlock pattern (compare-and-swap) for simplicity.
/// In Zig 0.16.0, std.Thread.Mutex is not available, so we implement our own.
/// Use a plain bool with atomic operations (Zig 0.16.0 built-in functions).
var g_reset_lock: bool = false;

/// Maximum arena size in bytes (configurable via environment variable).
/// When queryCapacity() exceeds this limit, the arena is automatically reset.
var g_max_arena_bytes: usize = DEFAULT_MAX_ARENA_SIZE_MB * 1024 * 1024;

/// Read the maximum arena size from environment variable.
/// Format: JS2RUST_MAX_ARENA_MB=<MB> (default: 100)
/// Returns max size in bytes.
/// Requires libc (already linked in js2rust generated code).
fn readMaxArenaBytes() usize {
    // std.c.getenv expects [*:0]const u8, string literals in Zig are null-terminated
    const env = std.c.getenv(ENV_MAX_ARENA_MB.ptr);
    if (env == null) return DEFAULT_MAX_ARENA_SIZE_MB * 1024 * 1024;
    const env_str = std.mem.span(env.?);
    const mb = std.fmt.parseInt(usize, env_str, 10) catch return DEFAULT_MAX_ARENA_SIZE_MB * 1024 * 1024;
    if (mb == 0) return DEFAULT_MAX_ARENA_SIZE_MB * 1024 * 1024; // Disallow 0
    return mb * 1024 * 1024; // Convert MB to bytes
}

/// Acquire the reset lock (spinlock pattern).
/// This ensures only one thread can reset the arena at a time.
/// Uses Zig 0.16.0 built-in atomic functions.
fn acquireResetLock() void {
    while (true) {
        // Try to acquire the lock (swap false -> true)
        const result = @cmpxchgStrong(bool, &g_reset_lock, false, true, .acquire, .monotonic);
        if (result == null) {
            // Lock acquired successfully
            return;
        }
        // Lock is held by another thread, spin-wait
        std.atomic.spinLoopHint();
    }
}

/// Release the reset lock.
fn releaseResetLock() void {
    @atomicStore(bool, &g_reset_lock, false, .release);
}

/// Initialize the global allocator with an ArenaAllocator.
/// Call this once at startup (from js2rust_init).
/// The arena uses page_allocator as the backing allocator.
/// Safe to call multiple times (idempotent).
pub fn initGlobalAllocator() void {
    // If already initialized, skip (idempotent)
    if (g_arena != null) return;

    // Read max arena size from environment
    g_max_arena_bytes = readMaxArenaBytes();

    // Create arena
    g_arena = std.heap.ArenaAllocator.init(std.heap.page_allocator);
    g_allocator = g_arena.?.allocator();
}

/// Deinitialize the global allocator.
/// Frees all memory allocated through this allocator at once.
/// Call this when shutting down (from js2rust_deinit).
pub fn deinitGlobalAllocator() void {
    acquireResetLock();
    defer releaseResetLock();

    if (g_arena) |*arena| {
        arena.deinit();
        g_arena = null;
        g_allocator = null;
    }
}

/// Reset the arena (free all allocated memory, keep allocator active).
/// Thread-safe: uses atomic spinlock to protect the reset operation.
/// This is the MANUAL reset API - it ALWAYS resets the arena.
/// After reset, memory usage returns to near zero.
/// Call this from Rust via `js2rust_reset()`.
pub fn resetGlobalAllocator() void {
    acquireResetLock();
    defer releaseResetLock();

    // Unconditional reset (manual API)
    if (g_arena) |*arena| {
        arena.deinit();
    }

    // Re-initialize arena
    g_arena = std.heap.ArenaAllocator.init(std.heap.page_allocator);
    g_allocator = g_arena.?.allocator();
}

/// Try to reset the arena (only if memory usage exceeds the limit).
/// Thread-safe: uses atomic spinlock and double-check pattern.
/// This is the AUTOMATIC reset API - it checks the limit again after acquiring the lock.
/// Returns true if reset was performed.
fn tryResetGlobalAllocator() bool {
    // First check (outside the lock, fast path)
    const used_before = getArenaUsedBytes();
    if (used_before <= g_max_arena_bytes) {
        return false; // Under limit, no reset needed
    }

    // Acquire lock for reset
    acquireResetLock();
    defer releaseResetLock();

    // Double-check (inside the lock, after possible reset by another thread)
    const used_after = getArenaUsedBytes();
    if (used_after <= g_max_arena_bytes) {
        return false; // Another thread already reset, memory is under limit now
    }

    // Memory still exceeds limit, perform reset
    if (g_arena) |*arena| {
        arena.deinit();
    }

    // Re-initialize arena
    g_arena = std.heap.ArenaAllocator.init(std.heap.page_allocator);
    g_allocator = g_arena.?.allocator();

    return true;
}

/// Get current arena memory usage in bytes.
/// Uses ArenaAllocator.queryCapacity() for accurate tracking.
/// Returns 0 if allocator is not initialized.
pub fn getArenaUsedBytes() usize {
    if (g_arena) |*arena| {
        return arena.queryCapacity();
    }
    return 0;
}

/// Get maximum arena size in bytes (from env or default).
pub fn getMaxArenaBytes() usize {
    return g_max_arena_bytes;
}

/// Check if arena should be reset (memory usage exceeds max).
/// If so, automatically reset the arena (with double-check for thread safety).
/// Returns true if reset was performed.
fn maybeAutoReset() bool {
    return tryResetGlobalAllocator();
}

/// Retrieve the global allocator.
/// Must be called after `initGlobalAllocator()`.
/// Will panic if called before initialization.
/// Before returning the allocator, checks if auto-reset is needed.
pub fn g_alloc() std.mem.Allocator {
    if (g_allocator == null) {
        @panic("js_allocator not initialized. Call initGlobalAllocator() first.");
    }

    // Check if we need to auto-reset (before returning allocator)
    _ = maybeAutoReset();

    return g_allocator.?;
}

/// Alias for g_alloc() (preferred name).
pub fn getAllocator() std.mem.Allocator {
    return g_alloc();
}
