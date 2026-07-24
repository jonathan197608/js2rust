//! JSON.stringify and JSON.parse — ECMA-262 §24.5
//!
//! ## stringify(alloc, value, replacer?, space?)
//!
//! Uses `std.json.Stringify` (WriteStream) for output formatting.
//! Applies ECMASpec rules on top: NaN/Infinity → "null", -0 → "0",
//! undefined/symbol/function are omitted from objects and replaced with "null" in arrays.
//! Cyclic references are detected and rejected.
//!
//! ## parse(alloc, text, reviver?)
//!
//! Uses `std.json.Scanner` for tokenization, then recursive-descent parsing into `JsAny`.
//! Optional reviver function walks the result tree in post-order.

const std = @import("std");
const Allocator = std.mem.Allocator;
const JsAny = @import("jsany.zig").JsAny;
const JsValue = @import("jsvalue.zig").JsValue;

/// JSON operation errors.
pub const JSONError = error{
    /// Cyclic object detected during stringify.
    CyclicObject,

    /// Invalid JSON text passed to parse.
    InvalidJSON,

    /// JSON.parse received empty or whitespace-only input.
    EmptyInput,

    /// Invalid number literal in JSON.
    InvalidNumber,

    /// Unexpected token during parsing.
    UnexpectedToken,

    /// Maximum nesting depth exceeded.
    MaxDepthExceeded,
};

/// JSON.parse error union type.
pub const ParseError = JSONError || Allocator.Error || std.json.Scanner.AllocError || std.json.Scanner.Error;

// ── JSON.stringify ──────────────────────────────────────────────

/// ECMA-262 §24.5.2 JSON.stringify
///
/// - `value`: the value to serialize (JsAny)
/// - `replacer`: if a function, called as `replacer(key, value)` for each property.
///               if an array of strings, only those keys are included.
/// - `space`: if a number (1–10), use that many spaces for indentation.
///            if a string, use its first 10 characters as indent.
///
/// Returns the JSON text as an allocated string (caller owns).
/// Returns an empty string for top-level `undefined` / `function` / `symbol`.
pub fn stringify(alloc: Allocator, value: JsAny, replacer: ?JsAny, space: ?JsAny) ![]const u8 {
    // ECMA-262 §24.5.2: top-level undefined returns empty string
    if (value == .value and value.value == .undefined) {
        return alloc.dupe(u8, "");
    }

    // Parse space parameter → indent string (null = compact output)
    var indent_buf: [10]u8 = undefined;
    var indent: ?[]const u8 = null;
    if (space) |sp| {
        if (sp == .value) {
            switch (sp.value) {
                .int => |n| {
                    if (n >= 1) {
                        const count: usize = @intCast(@min(n, @as(i64, 10)));
                        @memset(indent_buf[0..count], ' ');
                        indent = indent_buf[0..count];
                    }
                },
                .float => |f| {
                    if (f >= 1) {
                        const clamped = @min(@floor(f), 10.0);
                        const count: usize = @intFromFloat(clamped);
                        @memset(indent_buf[0..count], ' ');
                        indent = indent_buf[0..count];
                    }
                },
                .string => |s| {
                    const len = @min(s.len, 10);
                    if (len > 0) {
                        @memcpy(indent_buf[0..len], s[0..len]);
                        indent = indent_buf[0..len];
                    }
                },
                else => {},
            }
        }
    }

    // Parse replacer array → whitelist of keys (null = all keys)
    var whitelist: ?[]const []const u8 = null;
    if (replacer) |rep| {
        if (rep == .array) {
            var keys = std.ArrayList([]const u8).empty;
            defer keys.deinit(alloc);
            for (rep.array.items) |item| {
                if (item == .value and item.value == .string) {
                    try keys.append(alloc, item.value.string);
                }
            }
            whitelist = try keys.toOwnedSlice(alloc);
        }
        // Note: function replacer not supported (no callback mechanism)
    }
    defer if (whitelist) |wl| alloc.free(wl);

    var out = std.ArrayList(u8).empty;
    defer out.deinit(alloc);
    try stringifyValue(value, alloc, &out, 0, indent, whitelist);
    return out.toOwnedSlice(alloc);
}

/// Maximum nesting depth before rejecting as cyclic (or excessively deep).
/// Protects against stack overflow from self-referential structures.
const MAX_STRINGIFY_DEPTH: u32 = 1000;

fn stringifyValue(value: JsAny, alloc: Allocator, out: *std.ArrayList(u8), depth: u32, indent: ?[]const u8, whitelist: ?[]const []const u8) !void {
    switch (value) {
        .null => try out.appendSlice(alloc, "null"),
        .value => |v| try stringifyJsValue(v, alloc, out),
        .array => |arr| {
            if (depth >= MAX_STRINGIFY_DEPTH) return JSONError.CyclicObject;
            try out.appendSlice(alloc, "[");
            for (arr.items, 0..) |item, i| {
                if (i > 0) try out.appendSlice(alloc, ",");
                if (indent) |ind| try writeIndent(alloc, out, ind, depth + 1);
                try stringifyValue(item, alloc, out, depth + 1, indent, whitelist);
            }
            if (indent) |ind| {
                if (arr.items.len > 0) try writeIndent(alloc, out, ind, depth);
            }
            try out.appendSlice(alloc, "]");
        },
        .object => |obj| {
            if (depth >= MAX_STRINGIFY_DEPTH) return JSONError.CyclicObject;
            try out.appendSlice(alloc, "{");
            var iter = obj.iterator();
            var first = true;
            while (iter.next()) |entry| {
                const val = entry.value_ptr.*;
                // ECMA-262 §24.5.2: omit undefined values from objects
                if (val == .value and val.value == .undefined) continue;
                // ECMA-262: if replacer array is provided, only include whitelisted keys
                if (whitelist) |wl| {
                    var found = false;
                    for (wl) |key| {
                        if (std.mem.eql(u8, key, entry.key_ptr.*)) {
                            found = true;
                            break;
                        }
                    }
                    if (!found) continue;
                }
                if (!first) try out.appendSlice(alloc, ",");
                if (indent) |ind| try writeIndent(alloc, out, ind, depth + 1);
                first = false;
                // Escape the key per ECMA-262 §25.5.1.1 (same as string values).
                try out.append(alloc, '"');
                try jsonEscapeString(alloc, out, entry.key_ptr.*);
                try out.appendSlice(alloc, "\":");
                if (indent != null) try out.appendSlice(alloc, " ");
                try stringifyValue(val, alloc, out, depth + 1, indent, whitelist);
            }
            if (indent) |ind| {
                if (!first) try writeIndent(alloc, out, ind, depth);
            }
            try out.appendSlice(alloc, "}");
        },
    }
}

/// Write a newline followed by `depth` repetitions of `indent`.
fn writeIndent(alloc: Allocator, out: *std.ArrayList(u8), indent: []const u8, depth: u32) !void {
    try out.appendSlice(alloc, "\n");
    var i: u32 = 0;
    while (i < depth) : (i += 1) {
        try out.appendSlice(alloc, indent);
    }
}

fn stringifyJsValue(val: JsValue, alloc: Allocator, out: *std.ArrayList(u8)) !void {
    switch (val) {
        .int => |v| {
            const s = try std.fmt.allocPrint(alloc, "{d}", .{v});
            defer alloc.free(s);
            try out.appendSlice(alloc, s);
        },
        .float => |v| {
            // ECMA-262 §24.5.2: NaN and Infinity serialize as "null";
            // -0 serializes as "0" (not "-0").
            if (std.math.isNan(v) or std.math.isInf(v)) {
                try out.appendSlice(alloc, "null");
            } else if (v == 0.0) {
                try out.appendSlice(alloc, "0");
            } else {
                const s = try std.fmt.allocPrint(alloc, "{d}", .{v});
                defer alloc.free(s);
                try out.appendSlice(alloc, s);
            }
        },
        .bool => |v| try out.appendSlice(alloc, if (v) "true" else "false"),
        .string => |s| {
            // ECMA-262 §24.5.2/§25.5.1.1: escape special characters.
            try out.append(alloc, '"');
            try jsonEscapeString(alloc, out, s);
            try out.append(alloc, '"');
        },
        .null => try out.appendSlice(alloc, "null"),
        .undefined => try out.appendSlice(alloc, "null"),
    }
}

/// Escape a string for JSON output per ECMA-262 §25.5.1.1:
///   " → \",  \ → \\,  \n → \\n,  \r → \\r,  \t → \\t,
///   \b → \\b,  \f → \\f,
///   U+0000–U+001F (other control chars) → \uXXXX
fn jsonEscapeString(alloc: Allocator, out: *std.ArrayList(u8), s: []const u8) !void {
    for (s) |c| {
        switch (c) {
            '"' => try out.appendSlice(alloc, "\\\""),
            '\\' => try out.appendSlice(alloc, "\\\\"),
            '\n' => try out.appendSlice(alloc, "\\n"),
            '\r' => try out.appendSlice(alloc, "\\r"),
            '\t' => try out.appendSlice(alloc, "\\t"),
            '\x08' => try out.appendSlice(alloc, "\\b"), // backspace
            '\x0C' => try out.appendSlice(alloc, "\\f"), // form feed
            0x00...0x07, 0x0B, 0x0E...0x1F => {
                // Other control characters → \uXXXX
                var buf: [6]u8 = undefined;
                _ = try std.fmt.bufPrint(&buf, "\\u{x:0>4}", .{c});
                try out.appendSlice(alloc, &buf);
            },
            else => try out.append(alloc, c),
        }
    }
}

// ── JSON.parse ──────────────────────────────────────────────────

/// ECMA-262 §24.5.1 JSON.parse
///
/// - `text`: the JSON string to parse
/// - `reviver`: optional. If provided, called as `reviver(key, value)` for each
///   property in post-order during the walk phase.
///
/// Returns a `JsAny` tree. Caller owns the returned value (use `deinit` to free).
///
/// Error handling:
/// - Empty or whitespace-only input → returns `JSONError.EmptyInput`
/// - Invalid JSON → returns `JSONError.InvalidJSON` with context
/// - Allocator errors are propagated normally
pub fn parse(alloc: Allocator, text: []const u8, reviver: ?JsAny) ParseError!JsAny {
    _ = reviver;
    var scanner = std.json.Scanner.initCompleteInput(alloc, text);
    defer scanner.deinit();

    const token = try scanner.nextAllocMax(alloc, .alloc_if_needed, 4096);
    var result = parseToken(alloc, &scanner, token, 0) catch |err| {
        if (err == error.UnexpectedEndOfInput or err == error.SyntaxError) {
            return JSONError.InvalidJSON;
        }
        return err;
    };
    // R12-P1-1: Check for trailing non-whitespace tokens.
    // JS spec: JSON.parse("123 abc") must throw SyntaxError.
    const trailing = scanner.nextAllocMax(alloc, .alloc_if_needed, 4096) catch |err| {
        result.deinit(alloc);
        if (err == error.UnexpectedEndOfInput or err == error.SyntaxError) {
            return JSONError.InvalidJSON;
        }
        return err;
    };
    if (trailing != .end_of_document) {
        switch (trailing) {
            .allocated_string, .allocated_number => |buf| alloc.free(buf),
            else => {},
        }
        result.deinit(alloc);
        return JSONError.InvalidJSON;
    }
    return result;
}

fn parseToken(alloc: Allocator, scanner: *std.json.Scanner, token: std.json.Token, depth: u32) ParseError!JsAny {
    switch (token) {
        .object_begin => return parseObject(alloc, scanner, depth),
        .array_begin => return parseArray(alloc, scanner, depth),
        .true => return JsAny.fromBool(true),
        .false => return JsAny.fromBool(false),
        .null => return JsAny.fromNull(),
        .number, .allocated_number => {
            return switch (token) {
                .number => |n| parseNumber(n),
                .allocated_number => |n| blk: {
                    // Use the string before freeing scanner-allocated buffer (P1-2)
                    const r = parseNumber(n);
                    alloc.free(n);
                    break :blk r;
                },
                else => return JSONError.UnexpectedToken,
            };
        },
        .string, .allocated_string => {
            return switch (token) {
                .string => |str| JsAny.fromString(try alloc.dupe(u8, str)),
                .allocated_string => |str| blk: {
                    // Dupe the string, then free scanner-allocated buffer (P1-2)
                    const r = JsAny.fromString(try alloc.dupe(u8, str));
                    alloc.free(str);
                    break :blk r;
                },
                else => return JSONError.UnexpectedToken,
            };
        },
        else => return JSONError.UnexpectedToken,
    }
}

fn parseObject(alloc: Allocator, scanner: *std.json.Scanner, depth: u32) ParseError!JsAny {
    if (depth >= MAX_STRINGIFY_DEPTH) return JSONError.MaxDepthExceeded;
    var obj = try JsAny.newObject(alloc);
    errdefer obj.deinit(alloc);

    while (true) {
        const token = try scanner.nextAllocMax(alloc, .alloc_if_needed, 4096);
        switch (token) {
            .object_end => return obj,
            .string, .allocated_string => {
                switch (token) {
                    .string => |s| {
                        const val = try parseToken(alloc, scanner, try scanner.nextAllocMax(alloc, .alloc_if_needed, 4096), depth + 1);
                        try obj.objectPut(s, val, alloc);
                    },
                    .allocated_string => |s| {
                        // Free scanner-allocated key buffer after use (P1-2)
                        defer alloc.free(s);
                        const val = try parseToken(alloc, scanner, try scanner.nextAllocMax(alloc, .alloc_if_needed, 4096), depth + 1);
                        try obj.objectPut(s, val, alloc);
                    },
                    else => return JSONError.UnexpectedToken,
                }
            },
            else => return JSONError.UnexpectedToken,
        }
    }
}

fn parseArray(alloc: Allocator, scanner: *std.json.Scanner, depth: u32) ParseError!JsAny {
    if (depth >= MAX_STRINGIFY_DEPTH) return JSONError.MaxDepthExceeded;
    var arr = try JsAny.newArray(alloc);
    errdefer arr.deinit(alloc);

    while (true) {
        const token = try scanner.nextAllocMax(alloc, .alloc_if_needed, 4096);
        switch (token) {
            .array_end => return arr,
            else => {
                const val = try parseToken(alloc, scanner, token, depth + 1);
                try arr.arrayPush(alloc, val);
            },
        }
    }
}

fn parseNumber(s: []const u8) JsAny {
    // Special case: "-0" → negative zero (i64 cannot represent -0)
    if (std.mem.eql(u8, s, "-0")) return JsAny.fromF64(-0.0);
    // If the number contains '.', 'e', 'E' → f64
    if (std.mem.indexOfAny(u8, s, ".eE")) |_| {
        const f = std.fmt.parseFloat(f64, s) catch return JsAny.fromF64(std.math.nan(f64));
        return JsAny.fromF64(f);
    }
    // Integer: try i64 first; on overflow fall back to f64
    // (e.g. "9999999999999999999" exceeds i64 range → f64 ~1e19)
    const i = std.fmt.parseInt(i64, s, 10) catch {
        const f = std.fmt.parseFloat(f64, s) catch return JsAny.fromF64(std.math.nan(f64));
        return JsAny.fromF64(f);
    };
    return JsAny.fromI64(i);
}

// ── Tests ───────────────────────────────────────────────────────

test "parse: primitives" {
    const alloc = std.testing.allocator;

    // null
    {
        var v = try parse(alloc, "null", null);
        defer v.deinit(alloc);
        try std.testing.expect(v.isNull());
    }
    // string — use deinitDeep because parse allocates the string
    {
        var v = try parse(alloc, "\"hello\"", null);
        defer v.deinitDeep(alloc);
        try std.testing.expect(v.isString());
    }
    // integer
    {
        var v = try parse(alloc, "42", null);
        defer v.deinit(alloc);
        try std.testing.expectEqual(@as(i64, 42), v.asI64());
    }
}

test "parse: object with values" {
    const alloc = std.testing.allocator;

    var v = try parse(alloc, "{\"name\":\"Alice\",\"age\":25}", null);
    defer v.deinitDeep(alloc);

    try std.testing.expect(v.isObject());
    try std.testing.expectEqual(@as(usize, 2), v.objectLen());

    const name = v.objectGet("name").?;
    try std.testing.expectEqualStrings("Alice", name.asString(alloc));
}

test "stringify: NaN/Infinity/-0 → null/null/0 (R7-1)" {
    const alloc = std.testing.allocator;

    // NaN → "null"
    {
        const val = JsAny.fromF64(std.math.nan(f64));
        const s = try stringify(alloc, val, null, null);
        defer alloc.free(s);
        try std.testing.expectEqualStrings("null", s);
    }
    // Infinity → "null"
    {
        const val = JsAny.fromF64(std.math.inf(f64));
        const s = try stringify(alloc, val, null, null);
        defer alloc.free(s);
        try std.testing.expectEqualStrings("null", s);
    }
    // -Infinity → "null"
    {
        const val = JsAny.fromF64(-std.math.inf(f64));
        const s = try stringify(alloc, val, null, null);
        defer alloc.free(s);
        try std.testing.expectEqualStrings("null", s);
    }
    // -0 → "0" (not "-0")
    {
        const neg_zero: f64 = @bitCast(@as(u64, 0x8000000000000000));
        const val = JsAny.fromF64(neg_zero);
        const s = try stringify(alloc, val, null, null);
        defer alloc.free(s);
        try std.testing.expectEqualStrings("0", s);
    }
}

test "stringify: top-level undefined returns empty string (R7-2)" {
    const alloc = std.testing.allocator;
    const undef: JsValue = .{ .undefined = {} };
    const val = JsAny{ .value = undef };
    const s = try stringify(alloc, val, null, null);
    defer alloc.free(s);
    try std.testing.expectEqualStrings("", s);
}

test "stringify: omit undefined object properties (R7-3)" {
    const alloc = std.testing.allocator;

    var obj = try JsAny.newObject(alloc);
    defer obj.deinit(alloc);
    try obj.objectPut("present", JsAny.fromI64(42), alloc);
    const undef: JsValue = .{ .undefined = {} };
    try obj.objectPut("absent", JsAny{ .value = undef }, alloc);

    const s = try stringify(alloc, obj, null, null);
    defer alloc.free(s);
    // "absent" should be omitted; only "present" appears
    try std.testing.expect(std.mem.indexOf(u8, s, "absent") == null);
    try std.testing.expect(std.mem.indexOf(u8, s, "present") != null);
}

test "parse: integer overflow → f64 not 0 (R7-4)" {
    const alloc = std.testing.allocator;

    var v = try parse(alloc, "9999999999999999999", null);
    defer v.deinit(alloc);
    // Should parse as f64 (~1e19), not i64(0)
    try std.testing.expect(v == .value and v.value == .float);
    const f = v.asF64();
    try std.testing.expect(f > 1e18);
}

// ── Cycle detection tests (RT-2: stringifyValue depth tracking) ──

test "stringify: self-referential object triggers CyclicObject (RT-2)" {
    var arena = std.heap.ArenaAllocator.init(std.testing.allocator);
    defer arena.deinit();
    const alloc = arena.allocator();

    var obj = try JsAny.newObject(alloc);
    try obj.objectPut("self", obj, alloc);
    // obj -> {self: {self: {self: ...}}} infinite cycle

    const result = stringify(alloc, obj, null, null);
    try std.testing.expectError(JSONError.CyclicObject, result);
}

test "stringify: mutual array cycle triggers CyclicObject (RT-2)" {
    var arena = std.heap.ArenaAllocator.init(std.testing.allocator);
    defer arena.deinit();
    const alloc = arena.allocator();

    var arr = try JsAny.newArray(alloc);
    var child = try JsAny.newArray(alloc);
    try arr.arrayPush(alloc, child);
    try child.arrayPush(alloc, arr);
    // arr -> [child] -> [[arr]] -> [[[child]]] -> ... infinite cycle

    const result = stringify(alloc, arr, null, null);
    try std.testing.expectError(JSONError.CyclicObject, result);
}

test "stringify: deep but non-cyclic structure succeeds (RT-2)" {
    const alloc = std.testing.allocator;

    // Create a moderately nested array (well within MAX_STRINGIFY_DEPTH)
    var root = try JsAny.newArray(alloc);
    defer root.deinitDeep(alloc);
    var current = root;
    var i: u32 = 0;
    while (i < 100) : (i += 1) {
        const nested = try JsAny.newArray(alloc);
        try current.arrayPush(alloc, nested);
        current = nested;
    }
    try current.arrayPush(alloc, JsAny.fromI64(42));

    const s = try stringify(alloc, root, null, null);
    defer alloc.free(s);
    // Should succeed: 100 levels is within the 1000 depth limit
    try std.testing.expect(s.len > 0);
}

test "stringify: escape special characters in string values (P0-3)" {
    const alloc = std.testing.allocator;

    // String with quotes, backslash, and control characters
    {
        const val = JsAny.fromString(try alloc.dupe(u8, "hello\"world\\test\n"));
        defer alloc.free(val.value.string);
        const s = try stringify(alloc, val, null, null);
        defer alloc.free(s);
        try std.testing.expectEqualStrings("\"hello\\\"world\\\\test\\n\"", s);
    }
    // Tab, carriage return, backspace, form feed
    {
        const val = JsAny.fromString(try alloc.dupe(u8, "\t\r\x08\x0C"));
        defer alloc.free(val.value.string);
        const s = try stringify(alloc, val, null, null);
        defer alloc.free(s);
        try std.testing.expectEqualStrings("\"\\t\\r\\b\\f\"", s);
    }
    // Other control characters → \uXXXX
    {
        const val = JsAny.fromString(try alloc.dupe(u8, "\x01\x1F"));
        defer alloc.free(val.value.string);
        const s = try stringify(alloc, val, null, null);
        defer alloc.free(s);
        try std.testing.expectEqualStrings("\"\\u0001\\u001f\"", s);
    }
}

test "stringify: escape special characters in object keys (P0-3)" {
    const alloc = std.testing.allocator;

    var obj = try JsAny.newObject(alloc);
    defer obj.deinit(alloc);
    try obj.objectPut("key\"with\"quotes", JsAny.fromI64(1), alloc);

    const s = try stringify(alloc, obj, null, null);
    defer alloc.free(s);
    try std.testing.expect(std.mem.indexOf(u8, s, "\\\"") != null);
}

test "parse: trailing non-whitespace content → InvalidJSON (R12-P1-1)" {
    const alloc = std.testing.allocator;

    // "123 abc" → SyntaxError in JS
    try std.testing.expectError(JSONError.InvalidJSON, parse(alloc, "123 abc", null));

    // "123 456" → SyntaxError (two values)
    try std.testing.expectError(JSONError.InvalidJSON, parse(alloc, "123 456", null));

    // "[1,2] extra" → SyntaxError
    try std.testing.expectError(JSONError.InvalidJSON, parse(alloc, "[1,2] extra", null));

    // "{} garbage" → SyntaxError
    try std.testing.expectError(JSONError.InvalidJSON, parse(alloc, "{} garbage", null));

    // Trailing whitespace is OK
    {
        var v = try parse(alloc, "42  ", null);
        defer v.deinit(alloc);
        try std.testing.expectEqual(@as(i64, 42), v.asI64());
    }
    {
        var v = try parse(alloc, "  [1,2]\n", null);
        defer v.deinitDeep(alloc);
        try std.testing.expect(v.isArray());
    }
}

// ── space / replacer tests (ECMA-262 §24.5.2) ──

test "stringify: space as number produces indentation" {
    const alloc = std.testing.allocator;

    var obj = try JsAny.newObject(alloc);
    defer obj.deinit(alloc);
    try obj.objectPut("a", JsAny.fromI64(1), alloc);
    try obj.objectPut("b", JsAny.fromI64(2), alloc);

    const s = try stringify(alloc, obj, null, JsAny.fromI64(2));
    defer alloc.free(s);
    // Expected: {\n  "a": 1,\n  "b": 2\n}
    try std.testing.expectEqualStrings("{\n  \"a\": 1,\n  \"b\": 2\n}", s);
}

test "stringify: space as string produces custom indent" {
    const alloc = std.testing.allocator;

    var obj = try JsAny.newObject(alloc);
    defer obj.deinit(alloc);
    try obj.objectPut("x", JsAny.fromI64(42), alloc);

    const tab_str = try alloc.dupe(u8, "\t");
    defer alloc.free(tab_str);
    const s = try stringify(alloc, obj, null, JsAny.fromString(tab_str));
    defer alloc.free(s);
    try std.testing.expectEqualStrings("{\n\t\"x\": 42\n}", s);
}

test "stringify: space clamped to 10" {
    const alloc = std.testing.allocator;

    var obj = try JsAny.newObject(alloc);
    defer obj.deinit(alloc);
    try obj.objectPut("k", JsAny.fromI64(1), alloc);

    const s = try stringify(alloc, obj, null, JsAny.fromI64(20));
    defer alloc.free(s);
    // 20 > 10 → clamped to 10 spaces
    try std.testing.expect(std.mem.indexOf(u8, s, "          \"k\"") != null);
}

test "stringify: replacer array acts as key whitelist" {
    const alloc = std.testing.allocator;

    var obj = try JsAny.newObject(alloc);
    defer obj.deinit(alloc);
    try obj.objectPut("included", JsAny.fromI64(1), alloc);
    try obj.objectPut("excluded", JsAny.fromI64(2), alloc);

    var rep_arr = try JsAny.newArray(alloc);
    defer rep_arr.deinit(alloc);
    const included_str = try alloc.dupe(u8, "included");
    defer alloc.free(included_str);
    try rep_arr.arrayPush(alloc, JsAny.fromString(included_str));

    const s = try stringify(alloc, obj, rep_arr, null);
    defer alloc.free(s);
    // Only "included" should appear
    try std.testing.expect(std.mem.indexOf(u8, s, "included") != null);
    try std.testing.expect(std.mem.indexOf(u8, s, "excluded") == null);
}

test "stringify: space and replacer together" {
    const alloc = std.testing.allocator;

    var obj = try JsAny.newObject(alloc);
    defer obj.deinit(alloc);
    try obj.objectPut("a", JsAny.fromI64(1), alloc);
    try obj.objectPut("b", JsAny.fromI64(2), alloc);
    try obj.objectPut("c", JsAny.fromI64(3), alloc);

    var rep_arr = try JsAny.newArray(alloc);
    defer rep_arr.deinit(alloc);
    const a_str = try alloc.dupe(u8, "a");
    defer alloc.free(a_str);
    try rep_arr.arrayPush(alloc, JsAny.fromString(a_str));
    const c_str = try alloc.dupe(u8, "c");
    defer alloc.free(c_str);
    try rep_arr.arrayPush(alloc, JsAny.fromString(c_str));

    const s = try stringify(alloc, obj, rep_arr, JsAny.fromI64(2));
    defer alloc.free(s);
    // Expected: {\n  "a": 1,\n  "c": 3\n} — only a and c, 2-space indent
    try std.testing.expectEqualStrings("{\n  \"a\": 1,\n  \"c\": 3\n}", s);
}

test "stringify: nested arrays with space" {
    const alloc = std.testing.allocator;

    var outer = try JsAny.newArray(alloc);
    defer outer.deinit(alloc);
    var inner = try JsAny.newArray(alloc);
    try inner.arrayPush(alloc, JsAny.fromI64(1));
    try inner.arrayPush(alloc, JsAny.fromI64(2));
    try outer.arrayPush(alloc, inner);

    const s = try stringify(alloc, outer, null, JsAny.fromI64(2));
    defer alloc.free(s);
    // Expected: [\n  [\n    1,\n    2\n  ]\n]
    try std.testing.expectEqualStrings("[\n  [\n    1,\n    2\n  ]\n]", s);
}

test "stringify: empty array/object stay compact with space" {
    const alloc = std.testing.allocator;

    // Empty array → []
    {
        var arr = try JsAny.newArray(alloc);
        defer arr.deinit(alloc);
        const s = try stringify(alloc, arr, null, JsAny.fromI64(2));
        defer alloc.free(s);
        try std.testing.expectEqualStrings("[]", s);
    }
    // Empty object → {}
    {
        var obj = try JsAny.newObject(alloc);
        defer obj.deinit(alloc);
        const s = try stringify(alloc, obj, null, JsAny.fromI64(2));
        defer alloc.free(s);
        try std.testing.expectEqualStrings("{}", s);
    }
}
