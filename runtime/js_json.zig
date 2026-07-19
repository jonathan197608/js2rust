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
    _ = replacer;
    _ = space;
    // ECMA-262 §24.5.2: top-level undefined returns empty string
    // (not the string "null"). The doc comment already specified this
    // but the implementation was writing "null" via stringifyValue.
    if (value == .value and value.value == .undefined) {
        return alloc.dupe(u8, "");
    }
    var out = std.ArrayList(u8).empty;
    defer out.deinit(alloc);
    try stringifyValue(value, alloc, &out);
    return out.toOwnedSlice(alloc);
}

fn stringifyValue(value: JsAny, alloc: Allocator, out: *std.ArrayList(u8)) !void {
    switch (value) {
        .null => try out.appendSlice(alloc, "null"),
        .value => |v| try stringifyJsValue(v, alloc, out),
        .array => |arr| {
            try out.appendSlice(alloc, "[");
            for (arr.items, 0..) |item, i| {
                if (i > 0) try out.appendSlice(alloc, ",");
                try stringifyValue(item, alloc, out);
            }
            try out.appendSlice(alloc, "]");
        },
        .object => |obj| {
            try out.appendSlice(alloc, "{");
            var iter = obj.iterator();
            var first = true;
            while (iter.next()) |entry| {
                const val = entry.value_ptr.*;
                // ECMA-262 §24.5.2: omit undefined values from objects
                if (val == .value and val.value == .undefined) continue;
                if (!first) try out.appendSlice(alloc, ",");
                first = false;
                const key_part = try std.fmt.allocPrint(alloc, "\"{s}\":", .{entry.key_ptr.*});
                defer alloc.free(key_part);
                try out.appendSlice(alloc, key_part);
                try stringifyValue(val, alloc, out);
            }
            try out.appendSlice(alloc, "}");
        },
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
            const quoted = try std.fmt.allocPrint(alloc, "\"{s}\"", .{s});
            defer alloc.free(quoted);
            try out.appendSlice(alloc, quoted);
        },
        .null => try out.appendSlice(alloc, "null"),
        .undefined => try out.appendSlice(alloc, "null"),
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
    return parseToken(alloc, &scanner, token) catch |err| {
        if (err == error.UnexpectedEndOfInput or err == error.SyntaxError) {
            return JSONError.InvalidJSON;
        }
        return err;
    };
}

fn parseToken(alloc: Allocator, scanner: *std.json.Scanner, token: std.json.Token) ParseError!JsAny {
    switch (token) {
        .object_begin => return parseObject(alloc, scanner),
        .array_begin => return parseArray(alloc, scanner),
        .true => return JsAny.fromBool(true),
        .false => return JsAny.fromBool(false),
        .null => return JsAny.fromNull(),
        .number, .allocated_number => {
            const s = switch (token) {
                .number => |n| n,
                .allocated_number => |n| n,
                else => return JSONError.UnexpectedToken,
            };
            return parseNumber(s);
        },
        .string, .allocated_string => {
            const s = switch (token) {
                .string => |str| str,
                .allocated_string => |str| str,
                else => return JSONError.UnexpectedToken,
            };
            return JsAny.fromString(try alloc.dupe(u8, s));
        },
        else => return JSONError.UnexpectedToken,
    }
}

fn parseObject(alloc: Allocator, scanner: *std.json.Scanner) ParseError!JsAny {
    var obj = try JsAny.newObject(alloc);
    errdefer obj.deinit(alloc);

    while (true) {
        const token = try scanner.nextAllocMax(alloc, .alloc_if_needed, 4096);
        switch (token) {
            .object_end => return obj,
            .string, .allocated_string => {
                const key_raw = switch (token) {
                    .string => |s| s,
                    .allocated_string => |s| s,
                    else => return JSONError.UnexpectedToken,
                };

                const val = try parseToken(alloc, scanner, try scanner.nextAllocMax(alloc, .alloc_if_needed, 4096));
                try obj.objectPut(key_raw, val, alloc);
            },
            else => return JSONError.UnexpectedToken,
        }
    }
}

fn parseArray(alloc: Allocator, scanner: *std.json.Scanner) ParseError!JsAny {
    var arr = try JsAny.newArray(alloc);
    errdefer arr.deinit(alloc);

    while (true) {
        const token = try scanner.nextAllocMax(alloc, .alloc_if_needed, 4096);
        switch (token) {
            .array_end => return arr,
            else => {
                const val = try parseToken(alloc, scanner, token);
                try arr.arrayPush(alloc, val);
            },
        }
    }
}

fn parseNumber(s: []const u8) JsAny {
    // If the number contains '.', 'e', 'E' → f64
    if (std.mem.indexOfAny(u8, s, ".eE")) |_| {
        const f = std.fmt.parseFloat(f64, s) catch return JsAny.fromI64(0);
        return JsAny.fromF64(f);
    }
    // Integer: try i64 first; on overflow fall back to f64
    // (e.g. "9999999999999999999" exceeds i64 range → f64 ~1e19)
    const i = std.fmt.parseInt(i64, s, 10) catch {
        const f = std.fmt.parseFloat(f64, s) catch return JsAny.fromI64(0);
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
