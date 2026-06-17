const std = @import("std");

/// Global allocator storage.
/// Set once by `init_js2rust(allocator)` via `setGlobalAllocator`.
/// All generated code that needs heap allocation should call `g_alloc()`.
var g_allocator: ?std.mem.Allocator = null;

/// Store the allocator for later use by `g_alloc()`.
/// Should be called once at startup (before any `g_alloc()` calls).
pub fn setGlobalAllocator(alloc: std.mem.Allocator) void {
    g_allocator = alloc;
}

/// Retrieve the global allocator.
/// If `setGlobalAllocator` was never called, falls back to `page_allocator`.
/// NOTE: No race condition because we never mutate `g_allocator` on the
///       fallback path — each call to `page_allocator` returns the same
///       singleton, and the `?` check is read-only.
pub fn g_alloc() std.mem.Allocator {
    if (g_allocator) |a| return a;
    // No mutation → no race. Caller gets page_allocator directly.
    return std.heap.page_allocator;
}
