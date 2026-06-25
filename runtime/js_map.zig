//! JS Map implementation for Zig.
//! Uses std.StringHashMap(JsAny) internally (maps string keys to JsAny values).
//! All allocating functions take `alloc: std.mem.Allocator` as first parameter.

const std = @import("std");
const Allocator = std.mem.Allocator;
const JsAny = @import("jsany.zig").JsAny;

pub const JsMap = struct {
    inner: std.StringHashMap(JsAny),
    alloc: Allocator,

    pub fn init(alloc: Allocator) JsMap {
        return JsMap{
            .inner = std.StringHashMap(JsAny).init(alloc),
            .alloc = alloc,
        };
    }

    pub fn deinit(self: *JsMap) void {
        var iter = self.inner.iterator();
        while (iter.next()) |entry| {
            self.alloc.free(entry.key_ptr.*);
            var val = entry.value_ptr.*;
            val.deinit(self.alloc);
        }
        self.inner.deinit();
    }

    /// Set a key-value pair. Key is duplicated.
    /// If the key already exists, the old key string and old value are freed.
    pub fn set(self: *JsMap, key: []const u8, value: JsAny) !void {
        // If key already exists, fetchRemove returns the old kv so we can free them.
        if (self.inner.fetchRemove(key)) |old_kv| {
            self.alloc.free(old_kv.key);
            var old_val = old_kv.value;
            old_val.deinit(self.alloc);
        }
        const key_copy = try self.alloc.dupe(u8, key);
        try self.inner.put(key_copy, value);
    }

    /// Get value by key. Returns null if key not found.
    pub fn get(self: *const JsMap, key: []const u8) ?JsAny {
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
            var val = kv.value;
            val.deinit(self.alloc);
            return true;
        }
        return false;
    }

    /// Remove all entries.
    pub fn clear(self: *JsMap) void {
        var iter = self.inner.iterator();
        while (iter.next()) |entry| {
            self.alloc.free(entry.key_ptr.*);
            var val = entry.value_ptr.*;
            val.deinit(self.alloc);
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
    pub fn values(self: *const JsMap) ![]JsAny {
        var result = try self.alloc.alloc(JsAny, self.inner.count());
        var i: usize = 0;
        var iter = self.inner.iterator();
        while (iter.next()) |entry| {
            result[i] = entry.value_ptr.*;
            i += 1;
        }
        return result;
    }

    /// Return array of entries (key-value pairs).
    /// Caller must free each string in the returned array.
    pub fn entries(self: *const JsMap) ![][][]const u8 {
        var result = try self.alloc.alloc([][]const u8, self.inner.count());
        var i: usize = 0;
        var iter = self.inner.iterator();
        while (iter.next()) |entry| {
            result[i] = try self.alloc.alloc([]const u8, 2);
            result[i][0] = try self.alloc.dupe(u8, entry.key_ptr.*);
            // asString() may return a borrowed slice or an allocated string.
            // We duplicate it to ensure the caller can always free it.
            const val_str = entry.value_ptr.*.asString(self.alloc);
            result[i][1] = try self.alloc.dupe(u8, val_str);
            i += 1;
        }
        return result;
    }
};

// ── Tests ──

test "JsMap set/get/has" {
    var m = JsMap.init(std.testing.allocator);
    defer m.deinit();

    try m.set("a", JsAny.fromI64(1));
    try m.set("b", JsAny.fromI64(2));
    if (m.get("a")) |v| {
        try std.testing.expect(v.eq(JsAny.fromI64(1)));
    } else {
        try std.testing.expect(false); // should not happen
    }
    try std.testing.expect(m.has("b"));
    try std.testing.expect(!m.has("c"));
}

test "JsMap delete" {
    var m = JsMap.init(std.testing.allocator);
    defer m.deinit();

    try m.set("x", JsAny.fromI64(10));
    try std.testing.expect(m.delete("x"));
    try std.testing.expect(!m.has("x"));
    try std.testing.expect(!m.delete("x"));
}

test "JsMap clear" {
    var m = JsMap.init(std.testing.allocator);
    defer m.deinit();

    try m.set("a", JsAny.fromI64(1));
    try m.set("b", JsAny.fromI64(2));
    m.clear();
    try std.testing.expectEqual(@as(usize, 0), m.size());
}

test "JsMap size" {
    var m = JsMap.init(std.testing.allocator);
    defer m.deinit();

    try std.testing.expectEqual(@as(usize, 0), m.size());
    try m.set("a", JsAny.fromI64(1));
    try std.testing.expectEqual(@as(usize, 1), m.size());
}
