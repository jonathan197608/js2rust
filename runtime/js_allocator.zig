const std = @import("std");

/// Global allocator storage.
/// Set by `init_js2rust(allocator)` via `setGlobalAllocator`.
/// All generated code that needs heap allocation should call `g_alloc()`
/// instead of hard-coding `std.heap.page_allocator`.
var g_allocator: ?std.mem.Allocator = null;

/// Store the allocator for later use by `g_alloc()`.
/// Called once from `init_js2rust(allocator)`.
pub fn setGlobalAllocator(alloc: std.mem.Allocator) void {
    g_allocator = alloc;
}

/// Retrieve the global allocator.
/// Panics if `setGlobalAllocator` has not been called.
pub fn g_alloc() std.mem.Allocator {
    return g_allocator orelse @panic("js2rust: allocator not initialized (call init_js2rust first)");
}
