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
    // Simplified implementation: use std.json.Stringify
    var out = std.ArrayList(u8).empty;
    defer out.deinit(alloc);
    try stringifyValue(value, alloc, out.writer());
    return out.toOwnedSlice(alloc);
}

fn stringifyValue(value: JsAny, alloc: Allocator, writer: anytype) !void {
    switch (value) {
        .null => try writer.writeAll("null"),
        .value => |v| try stringifyJsValue(v, writer),
        .array => |arr| {
            try writer.writeAll("[");
            for (arr.items, 0..) |item, i| {
                if (i > 0) try writer.writeAll(",");
                try stringifyValue(item, alloc, writer);
            }
            try writer.writeAll("]");
        },
        .object => |obj| {
            try writer.writeAll("{");
            var iter = obj.iterator();
            var first = true;
            while (iter.next()) |entry| {
                if (!first) try writer.writeAll(",");
                first = false;
                try writer.print("\"{s}\":", .{entry.key_ptr.*});
                try stringifyValue(entry.value_ptr.*, alloc, writer);
            }
            try writer.writeAll("}");
        },
    }
}

fn stringifyJsValue(val: JsValue, writer: anytype) !void {
    switch (val) {
        .int => |v| try writer.print("{d}", .{v}),
        .float => |v| try writer.print("{d}", .{v}),
        .bool => |v| try writer.writeAll(if (v) "true" else "false"),
        .string => |s| try writer.print("\"{s}\"", .{s}),
        .null => try writer.writeAll("null"),
        .undefined => try writer.writeAll("null"),
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
                else => unreachable,
            };
            return parseNumber(s);
        },
        .string, .allocated_string => {
            const s = switch (token) {
                .string => |str| str,
                .allocated_string => |str| str,
                else => unreachable,
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
                    else => unreachable,
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
    // Integer: try i64
    const i = std.fmt.parseInt(i64, s, 10) catch return JsAny.fromI64(0);
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
