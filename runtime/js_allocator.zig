const std = @import("std");

/// Global allocator storage.
/// Uses ArenaAllocator (Zig 0.16.0: lock-free, thread-safe).
/// All generated code that needs heap allocation should call `g_alloc()`.
var g_allocator: ?std.mem.Allocator = null;
var g_arena: ?std.heap.ArenaAllocator = null;

/// Initialize the global allocator with an ArenaAllocator.
/// Call this once at startup (from js2rust_init).
/// The arena uses page_allocator as the backing allocator.
/// Safe to call multiple times (idempotent).
pub fn initGlobalAllocator() void {
    // If already initialized, skip (idempotent)
    if (g_arena != null) return;

    g_arena = std.heap.ArenaAllocator.init(std.heap.page_allocator);
    g_allocator = g_arena.?.allocator();
}

/// Deinitialize the global allocator.
/// Frees all memory allocated through this allocator at once.
/// Call this when shutting down (from js2rust_deinit).
pub fn deinitGlobalAllocator() void {
    if (g_arena) |*arena| {
        arena.deinit();
        g_arena = null;
        g_allocator = null;
    }
}

/// Retrieve the global allocator.
/// Must be called after `initGlobalAllocator()`.
/// Will panic if called before initialization.
pub fn g_alloc() std.mem.Allocator {
    if (g_allocator) |a| return a;
    @panic("js_allocator not initialized. Call initGlobalAllocator() first.");
}

/// Alias for g_alloc() (preferred name).
pub fn getAllocator() std.mem.Allocator {
    return g_alloc();
}
