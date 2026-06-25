//! JS Map implementation for Zig.
//! Uses std.StringHashMap(i64) internally (maps string keys to i64 values).
//! All allocating functions take `alloc: std.mem.Allocator` as first parameter.

const std = @import("std");
const Allocator = std.mem.Allocator;

pub const JsMap = struct {
    inner: std.StringHashMap(i64),
    alloc: Allocator,

    pub fn init(alloc: Allocator) JsMap {
        return JsMap{
            .inner = std.StringHashMap(i64).init(alloc),
            .alloc = alloc,
        };
    }

    pub fn deinit(self: *JsMap) void {
        var iter = self.inner.iterator();
        while (iter.next()) |entry| {
            self.alloc.free(entry.key_ptr.*);
        }
        self.inner.deinit();
    }

    /// Set a key-value pair. Key is duplicated.
    pub fn set(self: *JsMap, key: []const u8, value: i64) !void {
        // Check if key already exists, free old key if replacing
        if (self.inner.getKey(key)) |existing_key| {
            self.alloc.free(existing_key);
        }
        const key_copy = try self.alloc.dupe(u8, key);
        try self.inner.put(key_copy, value);
    }

    /// Get value by key. Returns null if key not found.
    pub fn get(self: *const JsMap, key: []const u8) ?i64 {
        return self.inner.get(key);
    }

    /// Check if key exists.
    pub fn has(self: *const JsMap, key: []const u8) bool {
        return self.inner.contains(key);
    }

    /// Remove a key. Returns true if key was present.
    pub fn delete(self: *JsMap, key: []const u8) bool {
        if (self.inner.fetchRemove(key)) |kv| {
            self.alloc.free(kv.key);
            return true;
        }
        return false;
    }

    /// Remove all entries.
    pub fn clear(self: *JsMap) void {
        var iter = self.inner.iterator();
        while (iter.next()) |entry| {
            self.alloc.free(entry.key_ptr.*);
        }
        self.inner.clearAndFree();
    }

    /// Number of entries.
    pub fn size(self: *const JsMap) usize {
        return self.inner.count();
    }

    /// Return array of keys.
    pub fn keys(self: *const JsMap) ![][]const u8 {
        var result = try self.alloc.alloc([]const u8, self.inner.count());
        var i: usize = 0;
        var iter = self.inner.iterator();
        while (iter.next()) |entry| {
            result[i] = try self.alloc.dupe(u8, entry.key_ptr.*);
            i += 1;
        }
        return result;
    }

    /// Return array of values.
    pub fn values(self: *const JsMap) ![]i64 {
        var result = try self.alloc.alloc(i64, self.inner.count());
        var i: usize = 0;
        var iter = self.inner.iterator();
        while (iter.next()) |entry| {
            result[i] = entry.value_ptr.*;
            i += 1;
        }
        return result;
    }

    /// Return array of entries (key-value pairs).
    pub fn entries(self: *const JsMap) ![][][]const u8 {
        var result = try self.alloc.alloc([][]const u8, self.inner.count());
        var i: usize = 0;
        var iter = self.inner.iterator();
        while (iter.next()) |entry| {
            result[i] = try self.alloc.alloc([]const u8, 2);
            result[i][0] = try self.alloc.dupe(u8, entry.key_ptr.*);
            // Value is i64, need to convert to string
            result[i][1] = try std.fmt.allocPrint(self.alloc, "{}", .{entry.value_ptr.*});
            i += 1;
        }
        return result;
    }
};

// ── Tests ──

test "JsMap set/get/has" {
    var m = JsMap.init(std.testing.allocator);
    defer m.deinit();

    try m.set("a", 1);
    try m.set("b", 2);
    try std.testing.expectEqual(@as(?i64, 1), m.get("a"));
    try std.testing.expect(m.has("b"));
    try std.testing.expect(!m.has("c"));
}

test "JsMap delete" {
    var m = JsMap.init(std.testing.allocator);
    defer m.deinit();

    try m.set("x", 10);
    try std.testing.expect(m.delete("x"));
    try std.testing.expect(!m.has("x"));
    try std.testing.expect(!m.delete("x"));
}

test "JsMap clear" {
    var m = JsMap.init(std.testing.allocator);
    defer m.deinit();

    try m.set("a", 1);
    try m.set("b", 2);
    m.clear();
    try std.testing.expectEqual(@as(usize, 0), m.size());
}

test "JsMap size" {
    var m = JsMap.init(std.testing.allocator);
    defer m.deinit();

    try std.testing.expectEqual(@as(usize, 0), m.size());
    try m.set("a", 1);
    try std.testing.expectEqual(@as(usize, 1), m.size());
}
