//! JsAny — general-purpose JS value type that covers primitives, dynamic arrays, and objects.
//! Arrays use std.ArrayList(JsAny), objects use StringArrayHashMap(JsAny) (insertion-order-preserving).
//! Both containers allow nesting (JsAny can contain JsAny).
//! Updated for Zig 0.16.0 ArrayList API (.empty, deinit(alloc), append(alloc, item)).

const std = @import("std");
const Allocator = std.mem.Allocator;
const JsValue = @import("jsvalue.zig").JsValue;
const js_allocator = @import("js_allocator.zig");
const StringArrayHashMap = @import("string_array_hash_map.zig").StringArrayHashMap;

/// Type alias for a dynamic JS array.
pub const JsArrayList = std.ArrayList(JsAny);

/// Type alias for a dynamic JS object (string-keyed, insertion-order map).
pub const JsObjectMap = StringArrayHashMap(JsAny);

/// General-purpose JS value that can hold primitives, arrays, or objects.
pub const JsAny = union(enum) {
    value: JsValue,
    array: *JsArrayList,
    object: *JsObjectMap,
    null: void,

    // === Constructors ===

    pub fn fromI64(v: i64) JsAny {
        return .{ .value = .{ .int = v } };
    }

    pub fn fromF64(v: f64) JsAny {
        return .{ .value = .{ .float = v } };
    }

    pub fn fromBool(v: bool) JsAny {
        return .{ .value = .{ .bool = v } };
    }

    pub fn fromString(v: []const u8) JsAny {
        return .{ .value = .{ .string = v } };
    }

    pub fn fromValue(v: JsValue) JsAny {
        return .{ .value = v };
    }

    pub fn fromNull() JsAny {
        return .{ .null = {} };
    }

    pub fn fromUndefined() JsAny {
        return .{ .value = .{ .undefined = {} } };
    }

    /// Generic constructor: auto-wrap primitives into JsAny.
    /// Accepts i64, f64, bool, []const u8, JsAny, JsValue, comptime_int, comptime_float,
    /// and string literals (*const [N:0]u8).
    pub fn from(value: anytype) JsAny {
        const T = @TypeOf(value);
        // Exact type matches (fast path, no cast)
        if (T == JsAny) return value;
        if (T == JsValue) return fromValue(value);
        if (T == bool) return fromBool(value);
        if (T == i64) return fromI64(value);
        if (T == f64) return fromF64(value);
        if (T == []const u8) return fromString(value);
        // comptime_int (e.g. `42`) → i64
        if (switch (@typeInfo(T)) {
            .comptime_int => true,
            else => false,
        }) {
            return fromI64(@intCast(value));
        }
        // comptime_float (e.g. `3.14`) → f64
        if (switch (@typeInfo(T)) {
            .comptime_float => true,
            else => false,
        }) {
            return fromF64(@floatCast(value));
        }
        // String literals: *const [N:0]u8 → []const u8
        if (switch (@typeInfo(T)) {
            .pointer => |ptr| blk: {
                const child_info = @typeInfo(ptr.child);
                break :blk switch (child_info) {
                    .array => |arr| arr.child == u8,
                    else => false,
                };
            },
            else => false,
        }) {
            // value is *const [N:0]u8, convert to []const u8 via slicing
            const slice: []const u8 = value[0..];
            return fromString(slice);
        }
        // Other integer types (u32, etc.) → i64
        if (switch (@typeInfo(T)) {
            .int => true,
            else => false,
        }) {
            return fromI64(@intCast(value));
        }
        // Other float types → f64
        if (switch (@typeInfo(T)) {
            .float => true,
            else => false,
        }) {
            return fromF64(@floatCast(value));
        }
        @compileError("Unsupported type for JsAny.from: " ++ @typeName(T));
    }

    /// JsAny.undefined constant.
    pub const undefined_value: JsAny = .{ .value = .{ .undefined = {} } };

    /// Create a new empty array on the heap.
    pub fn newArray(alloc: Allocator) !JsAny {
        const arr = try alloc.create(JsArrayList);
        arr.* = .empty;
        return .{ .array = arr };
    }

    /// Create a new empty object on the heap.
    pub fn newObject(alloc: Allocator) !JsAny {
        const obj = try alloc.create(JsObjectMap);
        obj.* = JsObjectMap.init(alloc);
        return .{ .object = obj };
    }

    // === Type checks ===

    pub fn isValue(self: JsAny) bool {
        return self == .value;
    }

    pub fn isArray(self: JsAny) bool {
        return self == .array;
    }

    pub fn isObject(self: JsAny) bool {
        return self == .object;
    }

    pub fn isNull(self: JsAny) bool {
        return self == .null;
    }

    pub fn isUndefined(self: JsAny) bool {
        return switch (self) {
            .value => |v| v.isUndefined(),
            else => false,
        };
    }

    pub fn isNullish(self: JsAny) bool {
        return self.isNull() or self.isUndefined();
    }

    pub fn isString(self: JsAny) bool {
        return switch (self) {
            .value => |v| v == .string,
            else => false,
        };
    }

    pub fn isNumber(self: JsAny) bool {
        return switch (self) {
            .value => |v| v == .int or v == .float,
            else => false,
        };
    }

    /// JS truthiness: false, 0, -0, 0n, "", null, undefined, NaN → false; everything else → true.
    pub fn toBool(self: JsAny) bool {
        return switch (self) {
            .null => false,
            .value => |v| switch (v) {
                .undefined => false,
                .bool => |b| b,
                .int => |i| i != 0,
                .float => |f| f != 0.0 and !std.math.isNan(f),
                .string => |s| s.len > 0,
                .null => false,
            },
            .array => true,
            .object => true,
        };
    }

    // === std.fmt integration (for template literals) ===

    /// Custom formatter so `std.fmt.allocPrint("{f}", .{jsany})` outputs the JS string representation.
    /// Note: in Zig 0.16.0, only the `{f}` specifier dispatches to this method;
    /// `{}`/`{any}` emit the debug representation for tagged unions.
    /// R12-P1-2: Arrays use Array.toString() semantics (comma-join, no brackets,
    /// null/undefined elements → empty string). Objects use "[object Object]".
    /// No allocator needed — writes directly to the writer.
    pub fn format(self: JsAny, writer: *std.Io.Writer) std.Io.Writer.Error!void {
        switch (self) {
            .value => |v| try v.format(writer),
            .array => |a| {
                for (a.items, 0..) |item, i| {
                    if (i > 0) try writer.writeAll(",");
                    switch (item) {
                        .null => {},
                        .value => |iv| switch (iv) {
                            .null, .undefined => {},
                            else => try item.format(writer),
                        },
                        else => try item.format(writer),
                    }
                }
            },
            .object => try writer.writeAll("[object Object]"),
            .null => try writer.writeAll("null"),
        }
    }

    // === Coercion to primitive ===

    pub fn asI64(self: JsAny) i64 {
        return switch (self) {
            .value => |v| v.asI64(),
            .array => |a| @intCast(a.items.len),
            .object => |o| @intCast(o.count()),
            .null => 0,
        };
    }

    pub fn asF64(self: JsAny) f64 {
        return switch (self) {
            .value => |v| v.asF64(),
            .array => |a| @floatFromInt(a.items.len),
            .object => |o| @floatFromInt(o.count()),
            .null => 0.0,
        };
    }

    pub fn asBool(self: JsAny) bool {
        return switch (self) {
            .value => |v| v.asBool(),
            .array => |a| a.items.len > 0,
            .object => |o| o.count() > 0,
            .null => false,
        };
    }

    pub fn asString(self: JsAny, alloc: Allocator) []const u8 {
        return switch (self) {
            .value => |v| v.asString(alloc),
            .array => |a| blk: {
                var buf: std.ArrayList(u8) = .empty;
                defer buf.deinit(alloc);
                for (a.items, 0..) |item, i| {
                    if (i > 0) buf.append(alloc, ',') catch break :blk "";
                    switch (item) {
                        .null => {},
                        .value => |iv| switch (iv) {
                            .null, .undefined => {},
                            else => {
                                const tmp = std.fmt.allocPrint(alloc, "{f}", .{item}) catch break :blk "";
                                defer alloc.free(tmp);
                                buf.appendSlice(alloc, tmp) catch break :blk "";
                            },
                        },
                        else => {
                            const tmp = std.fmt.allocPrint(alloc, "{f}", .{item}) catch break :blk "";
                            defer alloc.free(tmp);
                            buf.appendSlice(alloc, tmp) catch break :blk "";
                        },
                    }
                }
                break :blk buf.toOwnedSlice(alloc) catch "";
            },
            // R12-P1-2: JS Object.toString → "[object Object]"
            .object => "[object Object]",
            .null => "null",
        };
    }

    // === Downgrade to JsValue ===

    pub fn toValue(self: JsAny) JsValue {
        return switch (self) {
            .value => |v| v,
            .array => |a| .{ .int = @intCast(a.items.len) },
            .object => |o| .{ .int = @intCast(o.count()) },
            .null => .null,
        };
    }

    // === Arithmetic (JS semantics) ===

    pub fn add(self: JsAny, other: JsAny, alloc: Allocator) JsAny {
        // String concat: if either side is a string, concatenate
        // R8-P1-10: Use {f} format to avoid asString() temporary allocations.
        // Previously self.asString(alloc) + other.asString(alloc) leaked
        // heap-allocated temporaries for int/float/array/object operands.
        if (self.isString() or other.isString()) {
            const result = std.fmt.allocPrint(alloc, "{f}{f}", .{ self, other }) catch return JsAny.undefined_value;
            return .{ .value = .{ .string = result } };
        }
        // Numeric: int + int = int (with overflow → f64), otherwise float.
        // JS Number arithmetic always returns IEEE-754 double — when i64 +
        // i64 overflows, fall back to f64 instead of panicking (R6-5).
        if (self.isNumber() and other.isNumber()) {
            if (self == .value and self.value == .int and other == .value and other.value == .int) {
                const sum, const overflow = @addWithOverflow(self.value.int, other.value.int);
                if (overflow == 0) {
                    return .{ .value = .{ .int = sum } };
                }
                return .{ .value = .{ .float =
                    @as(f64, @floatFromInt(self.value.int)) +
                    @as(f64, @floatFromInt(other.value.int)) } };
            }
            return .{ .value = .{ .float = self.asF64() + other.asF64() } };
        }
        // Array + Array → concat (JS semantics: converts to string first)
        // Fallback: string concat — same {f} pattern avoids temp allocations.
        const result = std.fmt.allocPrint(alloc, "{f}{f}", .{ self, other }) catch return JsAny.undefined_value;
        return .{ .value = .{ .string = result } };
    }

    pub fn sub(self: JsAny, other: JsAny) JsAny {
        if (self.isNumber() and other.isNumber()) {
            if (self == .value and self.value == .int and other == .value and other.value == .int) {
                const diff, const overflow = @subWithOverflow(self.value.int, other.value.int);
                if (overflow == 0) {
                    return .{ .value = .{ .int = diff } };
                }
                return .{ .value = .{ .float =
                    @as(f64, @floatFromInt(self.value.int)) -
                    @as(f64, @floatFromInt(other.value.int)) } };
            }
        }
        return .{ .value = .{ .float = self.asF64() - other.asF64() } };
    }

    pub fn mul(self: JsAny, other: JsAny) JsAny {
        if (self.isNumber() and other.isNumber()) {
            if (self == .value and self.value == .int and other == .value and other.value == .int) {
                const prod, const overflow = @mulWithOverflow(self.value.int, other.value.int);
                if (overflow == 0) {
                    return .{ .value = .{ .int = prod } };
                }
                return .{ .value = .{ .float =
                    @as(f64, @floatFromInt(self.value.int)) *
                    @as(f64, @floatFromInt(other.value.int)) } };
            }
        }
        return .{ .value = .{ .float = self.asF64() * other.asF64() } };
    }

    pub fn div(self: JsAny, other: JsAny) JsAny {
        const num = self.asF64();
        const denom = other.asF64();
        if (denom == 0.0) {
            // IEEE-754 / JS: 0/0 (any sign) → NaN; x/±0 → ±Infinity with
            // sign(num) XOR sign(denom). Zig panics on f64 division by zero
            // in safe modes, so we must guard explicitly. We use `copysign`
            // (rather than direct `< 0` comparison) so signed zero is handled:
            // `copysign(1.0, -0.0)` returns `-1.0`, distinguishing -0 from +0.
            if (num == 0.0 or std.math.isNan(num)) {
                return .{ .value = .{ .float = std.math.nan(f64) } };
            }
            const sign_num = std.math.copysign(@as(f64, 1.0), num);
            const sign_denom = std.math.copysign(@as(f64, 1.0), denom);
            const inf_sign = sign_num * sign_denom;
            return .{ .value = .{ .float = std.math.copysign(std.math.inf(f64), inf_sign) } };
        }
        return .{ .value = .{ .float = num / denom } };
    }

    pub fn rem(self: JsAny, other: JsAny) JsAny {
        const d = other.asF64();
        if (d == 0.0) return .{ .value = .{ .float = std.math.nan(f64) } };
        // JS `%` uses truncated division semantics (sign follows dividend);
        // `@rem` matches that. `@mod` is floored (Python-like) and was used
        // here by mistake — this is a regression from R5-8 (R8-P1-9).
        return .{ .value = .{ .float = @rem(self.asF64(), d) } };
    }

    pub fn neg(self: JsAny) JsAny {
        return switch (self) {
            .value => |v| switch (v) {
                // -minInt(i64) has no positive representation in two's
                // complement — promoting to f64 avoids the overflow (R6-5).
                .int => |i| if (i == std.math.minInt(i64))
                    .{ .value = .{ .float = -@as(f64, @floatFromInt(i)) } }
                else
                    .{ .value = .{ .int = -i } },
                else => .{ .value = .{ .float = -self.asF64() } },
            },
            else => .{ .value = .{ .float = -self.asF64() } },
        };
    }

    pub fn not(self: JsAny) JsAny {
        return .{ .value = .{ .bool = !self.asBool() } };
    }

    // === Comparison ===

    /// Loose equality (==): JS spec §7.2.13.
    /// R8-P1-15: Previously reduced arrays/objects to .length via toValue(),
    /// making distinct arrays with same length "equal". Now uses reference
    /// identity for arrays/objects (two distinct objects are never ==).
    pub fn eq(self: JsAny, other: JsAny) bool {
        // Reference identity for heap-allocated containers
        switch (self) {
            .array => |a| {
                switch (other) {
                    .array => |b| return a == b, // pointer identity
                    .object, .null => return false,
                    .value => |v| switch (v) {
                        .null, .undefined => return false,
                        else => return false, // array == primitive → false
                    },
                }
            },
            .object => |o| {
                switch (other) {
                    .object => |b| return o == b, // pointer identity
                    .array, .null => return false,
                    .value => |v| switch (v) {
                        .null, .undefined => return false,
                        else => return false, // object == primitive → false
                    },
                }
            },
            .null => {
                switch (other) {
                    .null => return true,
                    .value => |v| switch (v) {
                        .null, .undefined => return true,
                        else => return false,
                    },
                    else => return false,
                }
            },
            .value => |v| switch (v) {
                .null, .undefined => {
                    switch (other) {
                        .null => return true,
                        .value => |ov| switch (ov) {
                            .null, .undefined => return true,
                            else => return false,
                        },
                        else => return false, // null/undefined == array/object → false
                    }
                },
                else => {
                    // value primitive vs array/object/null → false (no ToPrimitive conversion)
                    switch (other) {
                        .array, .object, .null => return false,
                        else => {}, // both are .value primitives — fall through to JsValue.eq
                    }
                },
            },
        }
        // Both are .value primitives — delegate to JsValue.eq (loose ==)
        return self.value.eq(other.value);
    }

    /// Strict equality (===): same type AND same value. No coercion.
    /// R8-P1-15: Reference identity for arrays/objects (two distinct
    /// objects are never ===). Different JsAny tags → always false.
    pub fn strictEq(self: JsAny, other: JsAny) bool {
        // Different top-level tags → always false (strict requires same type)
        const self_tag: std.meta.Tag(JsAny) = self;
        const other_tag: std.meta.Tag(JsAny) = other;
        if (self_tag != other_tag) {
            // Exception: .value(.null) and .null are both JS null
            if (self.isNull() and other.isNull()) return true;
            return false;
        }
        return switch (self) {
            .value => |v| v.strictEq(other.value),
            .array => |a| a == other.array, // pointer identity
            .object => |o| o == other.object, // pointer identity
            .null => true,
        };
    }

    /// SameValue (ECMA-262 §7.2.10): used by Object.is.
    /// Differs from === in two cases:
    ///   - NaN sameValue NaN → true  (=== returns false)
    ///   - +0 sameValue -0  → false  (=== returns true)
    pub fn sameValue(self: JsAny, other: JsAny) bool {
        // Different top-level tags → check null/undefined equivalence
        const self_tag: std.meta.Tag(JsAny) = self;
        const other_tag: std.meta.Tag(JsAny) = other;
        if (self_tag != other_tag) {
            // .null and .value(.null) are both JS null
            if (self.isNull() and other.isNull()) return true;
            return false;
        }
        return switch (self) {
            .value => |v| v.sameValue(other.value),
            .array => |a| a == other.array, // pointer identity
            .object => |o| o == other.object, // pointer identity
            .null => true,
        };
    }

    pub fn neq(self: JsAny, other: JsAny) bool {
        return !self.eq(other);
    }

    /// Ordering comparison (<): JS spec §7.2.14 Abstract Relational Comparison.
    /// R8-P1-16: Both-strings → lexicographic; otherwise → numeric via asF64().
    /// Previously always used asF64(), making string<string always return false.
    pub fn lt(self: JsAny, other: JsAny) bool {
        if (self.isString() and other.isString()) {
            return std.mem.order(u8, self.value.string, other.value.string) == .lt;
        }
        return self.asF64() < other.asF64();
    }

    pub fn le(self: JsAny, other: JsAny) bool {
        if (self.isString() and other.isString()) {
            const ord = std.mem.order(u8, self.value.string, other.value.string);
            return ord == .lt or ord == .eq;
        }
        return self.asF64() <= other.asF64();
    }

    pub fn gt(self: JsAny, other: JsAny) bool {
        if (self.isString() and other.isString()) {
            return std.mem.order(u8, self.value.string, other.value.string) == .gt;
        }
        return self.asF64() > other.asF64();
    }

    pub fn ge(self: JsAny, other: JsAny) bool {
        if (self.isString() and other.isString()) {
            const ord = std.mem.order(u8, self.value.string, other.value.string);
            return ord == .gt or ord == .eq;
        }
        return self.asF64() >= other.asF64();
    }

    /// Optional equality comparison for Map.get() results.
    /// Returns false if the optional is null.
    pub fn optionalEq(opt: ?JsAny, other: anytype) bool {
        return if (opt) |v| v.eq(from(other)) else false;
    }

    // === Array operations ===

    /// Append an item to the array. Auto-upgrades non-arrays to arrays.
    pub fn arrayPush(self: *JsAny, alloc: Allocator, item: JsAny) !void {
        switch (self.*) {
            .array => |a| try a.append(alloc, item),
            else => {
                var new_arr = try alloc.create(JsArrayList);
                new_arr.* = .empty;
                try new_arr.append(alloc, self.*);
                try new_arr.append(alloc, item);
                self.* = .{ .array = new_arr };
            },
        }
    }

    /// Get array element by index, returns undefined_value if out of bounds.
    /// Preferred method for codegen (avoids null handling).
    pub fn at(self: JsAny, index: usize) JsAny {
        return switch (self) {
            .array => |a| if (index < a.items.len) a.items[index] else .undefined_value,
            else => .undefined_value,
        };
    }

    /// Get array element by index. Returns null if out of bounds.
    /// Lower-level method, prefer `at()` for codegen.
    pub fn arrayGet(self: JsAny, index: usize) ?JsAny {
        return switch (self) {
            .array => |a| if (index < a.items.len) a.items[index] else null,
            else => null,
        };
    }

    /// Set array element by index. Does nothing if not an array.
    pub fn arraySet(self: *JsAny, index: usize, item: JsAny) void {
        switch (self.*) {
            .array => |a| {
                if (index < a.items.len) a.items[index] = item;
            },
            else => {},
        }
    }

    /// Remove and return last element. Returns null if empty or not array.
    pub fn arrayPop(self: *JsAny) ?JsAny {
        return switch (self.*) {
            .array => |a| a.pop(),
            else => null,
        };
    }

    /// Get array length.
    pub fn arrayLen(self: JsAny) usize {
        return switch (self) {
            .array => |a| a.items.len,
            else => 0,
        };
    }

    // === Object operations ===

    /// Get object property by key. Returns JsAny (undefined_value if not found).
    /// This is the preferred method for codegen (avoids null handling).
    pub fn get(self: JsAny, key: []const u8) JsAny {
        return switch (self) {
            .object => |o| if (o.get(key)) |v| v else .undefined_value,
            else => .undefined_value,
        };
    }

    /// Set object property by key. Allocates a copy of key.
    /// This is the preferred method for codegen.
    pub fn set(self: *JsAny, key: []const u8, val: JsAny, alloc: Allocator) !void {
        switch (self.*) {
            .object => |o| {
                const key_dupe = try alloc.dupe(u8, key);
                try o.put(key_dupe, val);
            },
            else => {},
        }
    }

    /// Get object property by dynamic key (JsAny). Key is converted to string via asString().
    /// For codegen of `obj[key]` syntax.
    pub fn getByKey(self: JsAny, key: JsAny, alloc: Allocator) JsAny {
        const key_str = key.asString(alloc);
        const result = self.get(key_str);
        freeAsStringKey(key, key_str, alloc);
        return result;
    }

    /// Set object property by dynamic key (JsAny). Key is converted to string via asString().
    /// For codegen of `obj[key] = value` syntax.
    pub fn setByKey(self: *JsAny, key: JsAny, val: JsAny, alloc: Allocator) !void {
        const key_str = key.asString(alloc);
        try self.set(key_str, val, alloc);
        freeAsStringKey(key, key_str, alloc);
    }

    /// Get object property by key. Returns null if not found or not object.
    pub fn objectGet(self: JsAny, key: []const u8) ?JsAny {
        return switch (self) {
            .object => |o| o.get(key),
            else => null,
        };
    }

    /// Set object property by key. Duplicates key internally.
    /// This is the lower-level method; prefer `set()` for codegen.
    pub fn objectPut(self: *JsAny, key: []const u8, val: JsAny, alloc: Allocator) !void {
        switch (self.*) {
            .object => |o| {
                const key_dupe = try alloc.dupe(u8, key);
                try o.put(key_dupe, val);
            },
            else => {},
        }
    }

    /// Check if object has a key.
    pub fn objectHas(self: JsAny, key: []const u8) bool {
        return switch (self) {
            .object => |o| o.contains(key),
            else => false,
        };
    }

    /// Get object key count.
    pub fn objectLen(self: JsAny) usize {
        return switch (self) {
            .object => |o| o.count(),
            else => 0,
        };
    }

    /// Delete a property by string key. Returns true if the key existed.
    /// For codegen of `delete obj.prop`.
    pub fn deleteKey(self: *JsAny, key: []const u8) bool {
        return switch (self.*) {
            .object => |*o| o.remove(key),
            else => false,
        };
    }

    /// Delete a property by dynamic JsAny key. Returns true if the key existed.
    /// For codegen of `delete obj[expr]`.
    pub fn deleteByKey(self: *JsAny, key: JsAny, alloc: Allocator) bool {
        const key_str = key.asString(alloc);
        defer {
            switch (key) {
                .value => |v| switch (v) {
                    .int, .float => alloc.free(key_str),
                    else => {},
                },
                .array, .object => alloc.free(key_str),
                .null => {},
            }
        }
        return switch (self.*) {
            .object => |*o| o.remove(key_str),
            else => false,
        };
    }

    // === Cleanup ===

    /// Free the result of asString() if it was heap-allocated.
    /// asString allocates for .value.int, .value.float (non-NaN/Inf), .array, .object.
    /// It returns a borrowed slice or literal for everything else.
    /// R8-P1-10: .float NaN/Infinity/-Infinity return literals — must NOT be freed.
    fn freeAsStringKey(key: JsAny, key_str: []const u8, alloc: Allocator) void {
        switch (key) {
            .value => |v| switch (v) {
                .int => alloc.free(key_str),
                .float => |f| {
                    // NaN and Infinity return string literals from asString — not heap-allocated.
                    if (!std.math.isNan(f) and !std.math.isInf(f)) {
                        alloc.free(key_str);
                    }
                },
                else => {},
            },
            .array, .object => alloc.free(key_str),
            .null => {},
        }
    }

    /// Deinit heap-allocated array or object.
    /// Does NOT free .value.string — use deinitDeep() for that.
    ///
    /// Under the multi-arena allocator, free()/destroy() are no-ops, so the
    /// entire traversal + cleanup is wasted CPU. isNoOpFree() short-circuits
    /// with a single pointer comparison, skipping all work.
    pub fn deinit(self: *JsAny, alloc: Allocator) void {
        if (js_allocator.isNoOpFree(alloc)) {
            self.* = .{ .null = {} };
            return;
        }
        switch (self.*) {
            .array => |a| {
                for (a.items) |*item| item.deinit(alloc);
                a.deinit(alloc);
                alloc.destroy(a);
            },
            .object => |o| {
                var iter = o.iterator();
                while (iter.next()) |entry| {
                    alloc.free(entry.key_ptr.*); // Free duplicated key string
                    var val = entry.value_ptr.*;
                    val.deinit(alloc);
                }
                o.deinit();
                alloc.destroy(o);
            },
            else => {},
        }
        self.* = .{ .null = {} };
    }

    /// Deinit heap-allocated array, object, AND recursively free .value.string.
    /// Use this variant when JsAny may contain heap-allocated strings
    /// (e.g., values from JSON.parse). NOT safe for string literals
    /// (e.g., JsAny.fromString("hello")).
    ///
    /// Under the multi-arena allocator, all free()/destroy() are no-ops.
    /// isNoOpFree() short-circuits with a single pointer comparison.
    pub fn deinitDeep(self: *JsAny, alloc: Allocator) void {
        if (js_allocator.isNoOpFree(alloc)) {
            self.* = .{ .null = {} };
            return;
        }
        switch (self.*) {
            .value => |v| {
                if (v == .string) alloc.free(v.string);
            },
            .array => |a| {
                for (a.items) |*item| item.deinitDeep(alloc);
                a.deinit(alloc);
                alloc.destroy(a);
            },
            .object => |o| {
                var iter = o.iterator();
                while (iter.next()) |entry| {
                    alloc.free(entry.key_ptr.*);
                    var val = entry.value_ptr.*;
                    val.deinitDeep(alloc);
                }
                o.deinit();
                alloc.destroy(o);
            },
            .null => {},
        }
        self.* = .{ .null = {} };
    }
};

/// JS typeof operator — returns the JS typeof string for a JsAny value.
/// Maps JsAny tagged union variants to JS typeof semantics:
///   .value.int / .value.float → "number"
///   .value.bool                → "boolean"
///   .value.string              → "string"
///   .value.null                → "object"  (JS quirk: typeof null === "object")
///   .value.undefined           → "undefined"
///   .array / .object           → "object"
///   .null                      → "object"
pub fn jsTypeof(val: JsAny) []const u8 {
    return switch (val) {
        .value => |v| switch (v) {
            .int, .float => "number",
            .bool => "boolean",
            .string => "string",
            .null => "object",
            .undefined => "undefined",
        },
        .array, .object => "object",
        .null => "object",
    };
}

/// JS `instanceof` operator for JsAny dynamic type checking.
/// Since the Zig runtime has no prototype chain, this uses a tag-based
/// approach that maps JsAny variants to JS class names:
///   - .array → matches "Array" and "Object"
///   - .object → matches "Object" and any custom class stored in __jsClass__
///   - .value.int / .value.float → "Number" is false (primitives aren't objects)
///   - .value.string → "String" is false (primitive strings aren't String objects)
///   - .value.bool → "Boolean" is false (primitive booleans aren't Boolean objects)
///   - .null → always false
///
/// For custom class instances stored as .object, the object may contain a
/// __jsClass__ key with the class name string. This allows basic prototype-like
/// matching for `obj instanceof ClassName` checks.
pub fn instanceOf(val: JsAny, type_name: []const u8) bool {
    return switch (val) {
        .array => std.mem.eql(u8, type_name, "Array") or std.mem.eql(u8, type_name, "Object"),
        .object => |o| blk: {
            // Direct Object match
            if (std.mem.eql(u8, type_name, "Object")) break :blk true;
            // Check if this object has a __jsClass__ field for custom class matching
            const class_opt = o.get("__jsClass__");
            if (class_opt) |class_tag| {
                if (class_tag.isString()) {
                    if (std.mem.eql(u8, class_tag.value.string, type_name)) break :blk true;
                    // Walk parent chain via __jsExtends__
                    const parent_opt = o.get("__jsExtends__");
                    if (parent_opt) |parent_tag| {
                        if (parent_tag.isString()) {
                            if (std.mem.eql(u8, parent_tag.value.string, type_name)) break :blk true;
                        }
                    }
                }
            }
            break :blk false;
        },
        .value => |v| switch (v) {
            // JS primitives are never instanceof their wrapper types
            .int, .float => false,
            .bool => false,
            .string => false,
            // typeof null === "object" but null instanceof Object is false in JS
            .null => false,
            .undefined => false,
        },
        .null => false, // null is not instanceof anything
    };
}

// ── Tests ──

test "JsAny primitive constructors" {
    const a = JsAny.fromI64(42);
    try std.testing.expect(a.isValue());
    try std.testing.expect(a.isNumber());
    try std.testing.expectEqual(@as(i64, 42), a.asI64());

    const b = JsAny.fromF64(3.14);
    try std.testing.expect(b.isNumber());

    const c = JsAny.fromBool(true);
    try std.testing.expect(c.asBool());

    const d = JsAny.fromString("hello");
    try std.testing.expect(d.isString());

    const e = JsAny.fromNull();
    try std.testing.expect(e.isNull());
}

test "JsAny numeric arithmetic" {
    const a = JsAny.fromI64(10);
    const b = JsAny.fromI64(3);

    const sum = a.add(b, std.testing.allocator);
    try std.testing.expectEqual(@as(i64, 13), sum.value.int);

    const diff = a.sub(b);
    try std.testing.expectEqual(@as(i64, 7), diff.value.int);

    const product = a.mul(b);
    try std.testing.expectEqual(@as(i64, 30), product.value.int);
}

test "JsAny arithmetic overflow promotes to f64 (R6-5)" {
    const allocator = std.testing.allocator;

    // add: maxInt(i64) + 1 overflows i64 — must promote to f64, not panic.
    const sum = JsAny.fromI64(std.math.maxInt(i64)).add(JsAny.fromI64(1), allocator);
    try std.testing.expectEqual(
        @as(f64, @floatFromInt(std.math.maxInt(i64))) + 1.0,
        sum.value.float,
    );

    // sub: minInt(i64) - 1 underflows i64 — promote to f64.
    const diff = JsAny.fromI64(std.math.minInt(i64)).sub(JsAny.fromI64(1));
    try std.testing.expectEqual(
        @as(f64, @floatFromInt(std.math.minInt(i64))) - 1.0,
        diff.value.float,
    );

    // mul: 5_000_000_000 * 5_000_000_000 = 2.5e19 > maxInt(i64) — promote to f64.
    const product = JsAny.fromI64(5_000_000_000).mul(JsAny.fromI64(5_000_000_000));
    try std.testing.expectEqual(@as(f64, 5_000_000_000.0) * 5_000_000_000.0, product.value.float);

    // neg: -minInt(i64) has no positive i64 representation — promote to f64.
    const neg_min = JsAny.fromI64(std.math.minInt(i64)).neg();
    try std.testing.expectEqual(
        -@as(f64, @floatFromInt(std.math.minInt(i64))),
        neg_min.value.float,
    );
}

test "JsAny float arithmetic" {
    const a = JsAny.fromF64(10.0);
    const b = JsAny.fromF64(4.0);

    const quotient = a.div(b);
    try std.testing.expectEqual(@as(f64, 2.5), quotient.value.float);
}

test "JsAny div by zero and 0/0 (R8-P1-8)" {
    // 0/0 → NaN (was Infinity before fix).
    const zero_div = JsAny.fromI64(0).div(JsAny.fromI64(0));
    try std.testing.expect(std.math.isNan(zero_div.value.float));

    // 1/0 → +Infinity (was Infinity, but verify sign correctness).
    const pos_div = JsAny.fromI64(1).div(JsAny.fromI64(0));
    try std.testing.expect(std.math.isInf(pos_div.value.float));
    try std.testing.expect(pos_div.value.float > 0);

    // 1/-0 → -Infinity (was +Infinity before fix: signed-zero handling).
    const neg_zero = JsAny.fromF64(std.math.nan(f64)); // placeholder
    _ = neg_zero;
    const neg_div = JsAny.fromI64(1).div(JsAny.fromF64(-0.0));
    try std.testing.expect(std.math.isInf(neg_div.value.float));
    try std.testing.expect(neg_div.value.float < 0);

    // -1/0 → -Infinity.
    const neg_num_div = JsAny.fromI64(-1).div(JsAny.fromI64(0));
    try std.testing.expect(std.math.isInf(neg_num_div.value.float));
    try std.testing.expect(neg_num_div.value.float < 0);
}

test "JsAny rem truncated not floored (R8-P1-9)" {
    // -5 % 3 → JS: -2 (truncated). @mod (floored) gives 1.
    const rem_neg = JsAny.fromI64(-5).rem(JsAny.fromI64(3));
    try std.testing.expectEqual(@as(f64, -2.0), rem_neg.value.float);

    // 5 % -3 → JS: 2 (truncated). @mod (floored) gives -1.
    const rem_pos = JsAny.fromI64(5).rem(JsAny.fromI64(-3));
    try std.testing.expectEqual(@as(f64, 2.0), rem_pos.value.float);

    // x % 0 → NaN.
    const rem_zero = JsAny.fromI64(5).rem(JsAny.fromI64(0));
    try std.testing.expect(std.math.isNan(rem_zero.value.float));
}

test "JsAny string concat" {
    const alloc = std.testing.allocator;
    const a = JsAny.fromString("Hello, ");
    const b = JsAny.fromString("World!");
    const result = a.add(b, alloc);
    defer alloc.free(result.value.string);
    try std.testing.expectEqualStrings("Hello, World!", result.value.string);
}

test "JsAny comparison" {
    const a = JsAny.fromI64(5);
    const b = JsAny.fromI64(10);
    try std.testing.expect(a.lt(b));
    try std.testing.expect(b.gt(a));
    try std.testing.expect(a.le(a));
    try std.testing.expect(a.ge(a));
}

test "JsAny array operations" {
    const alloc = std.testing.allocator;
    var arr = try JsAny.newArray(alloc);
    defer arr.deinit(alloc);

    try arr.arrayPush(alloc, JsAny.fromI64(1));
    try arr.arrayPush(alloc, JsAny.fromI64(2));
    try arr.arrayPush(alloc, JsAny.fromI64(3));

    try std.testing.expectEqual(@as(usize, 3), arr.arrayLen());

    const first = arr.arrayGet(0).?;
    try std.testing.expectEqual(@as(i64, 1), first.value.int);

    const popped = arr.arrayPop().?;
    try std.testing.expectEqual(@as(i64, 3), popped.value.int);
    try std.testing.expectEqual(@as(usize, 2), arr.arrayLen());
}

test "JsAny object operations" {
    const alloc = std.testing.allocator;
    var obj = try JsAny.newObject(alloc);
    defer obj.deinit(alloc);

    try obj.objectPut("name", JsAny.fromString("Zig"), alloc);
    try obj.objectPut("version", JsAny.fromI64(0), alloc);

    try std.testing.expect(obj.objectHas("name"));
    try std.testing.expectEqual(@as(usize, 2), obj.objectLen());

    const name = obj.objectGet("name").?;
    try std.testing.expectEqualStrings("Zig", name.value.string);
}

test "JsAny nested array in object" {
    const alloc = std.testing.allocator;
    var obj = try JsAny.newObject(alloc);
    defer obj.deinit(alloc);

    var inner = try JsAny.newArray(alloc);
    try inner.arrayPush(alloc, JsAny.fromI64(1));
    try inner.arrayPush(alloc, JsAny.fromI64(2));
    try obj.objectPut("data", inner, alloc);

    const got = obj.objectGet("data").?;
    try std.testing.expect(got.isArray());
    try std.testing.expectEqual(@as(usize, 2), got.arrayLen());
}

test "JsAny get() convenience method" {
    const alloc = std.testing.allocator;
    var obj = try JsAny.newObject(alloc);
    defer obj.deinit(alloc);

    try obj.set("name", JsAny.fromString("Alice"), alloc);
    try obj.set("age", JsAny.fromI64(30), alloc);

    // get() returns JsAny (not ?JsAny)
    const name = obj.get("name");
    try std.testing.expect(name.isString());
    try std.testing.expectEqualStrings("Alice", name.asString(alloc));

    const age = obj.get("age");
    try std.testing.expectEqual(@as(i64, 30), age.asI64());

    // get() returns undefined_value for missing key (not null)
    const missing = obj.get("missing");
    try std.testing.expect(missing.isUndefined());
}

test "JsAny at() array index access" {
    const alloc = std.testing.allocator;
    var arr = try JsAny.newArray(alloc);
    defer arr.deinit(alloc);

    try arr.arrayPush(alloc, JsAny.fromI64(10));
    try arr.arrayPush(alloc, JsAny.fromI64(20));
    try arr.arrayPush(alloc, JsAny.fromI64(30));

    // at() returns JsAny (not ?JsAny)
    const first = arr.at(0);
    try std.testing.expectEqual(@as(i64, 10), first.asI64());

    const second = arr.at(1);
    try std.testing.expectEqual(@as(i64, 20), second.asI64());

    // at() returns undefined_value for out-of-bounds (not null)
    const out_of_bounds = arr.at(100);
    try std.testing.expect(out_of_bounds.isUndefined());
}

test "JsAny getByKey() dynamic key access" {
    const alloc = std.testing.allocator;
    var obj = try JsAny.newObject(alloc);
    defer obj.deinit(alloc);

    try obj.set("name", JsAny.fromString("Bob"), alloc);
    try obj.set("score", JsAny.fromI64(100), alloc);

    // getByKey() uses key.asString() to convert key to property name
    const key = JsAny.fromString("name");
    const name = obj.getByKey(key, alloc);
    try std.testing.expectEqualStrings("Bob", name.asString(alloc));

    const score_key = JsAny.fromString("score");
    const score = obj.getByKey(score_key, alloc);
    try std.testing.expectEqual(@as(i64, 100), score.asI64());
}

test "JsAny setByKey() dynamic key assignment" {
    const alloc = std.testing.allocator;
    var obj = try JsAny.newObject(alloc);
    defer obj.deinit(alloc);

    const key = JsAny.fromString("dynamic");
    try obj.setByKey(key, JsAny.fromI64(42), alloc);

    const val = obj.get("dynamic");
    try std.testing.expectEqual(@as(i64, 42), val.asI64());
}

test "JsAny get() on non-object returns undefined" {
    // Calling get() on a non-object returns undefined_value
    const num = JsAny.fromI64(42);
    const result = num.get("any_key");
    try std.testing.expect(result.isUndefined());
}

test "JsAny at() on non-array returns undefined" {
    // Calling at() on a non-array returns undefined_value
    var obj = try JsAny.newObject(std.testing.allocator);
    defer obj.deinit(std.testing.allocator);

    try obj.set("x", JsAny.fromI64(1), std.testing.allocator);

    const result = obj.at(0);
    try std.testing.expect(result.isUndefined());
}

test "jsTypeof primitives" {
    try std.testing.expectEqualStrings("number", jsTypeof(JsAny.fromI64(42)));
    try std.testing.expectEqualStrings("number", jsTypeof(JsAny.fromF64(3.14)));
    try std.testing.expectEqualStrings("boolean", jsTypeof(JsAny.fromBool(true)));
    try std.testing.expectEqualStrings("boolean", jsTypeof(JsAny.fromBool(false)));
    try std.testing.expectEqualStrings("string", jsTypeof(JsAny.fromString("hello")));
    try std.testing.expectEqualStrings("object", jsTypeof(JsAny.fromNull()));
    try std.testing.expectEqualStrings("undefined", jsTypeof(JsAny.undefined_value));
}

test "jsTypeof array and object" {
    const alloc = std.testing.allocator;
    var arr = try JsAny.newArray(alloc);
    defer arr.deinit(alloc);
    try std.testing.expectEqualStrings("object", jsTypeof(arr));

    var obj = try JsAny.newObject(alloc);
    defer obj.deinit(alloc);
    try std.testing.expectEqualStrings("object", jsTypeof(obj));
}

test "instanceOf array checks" {
    const alloc = std.testing.allocator;
    var arr = try JsAny.newArray(alloc);
    defer arr.deinit(alloc);

    // [].instanceof Array === true
    try std.testing.expect(instanceOf(arr, "Array"));
    // [].instanceof Object === true
    try std.testing.expect(instanceOf(arr, "Object"));
    // [].instanceof String === false
    try std.testing.expect(!instanceOf(arr, "String"));
}

test "instanceOf object checks" {
    const alloc = std.testing.allocator;
    var obj = try JsAny.newObject(alloc);
    defer obj.deinit(alloc);

    // {}.instanceof Object === true
    try std.testing.expect(instanceOf(obj, "Object"));
    // {}.instanceof Array === false
    try std.testing.expect(!instanceOf(obj, "Array"));
}

test "instanceOf custom class object" {
    const alloc = std.testing.allocator;
    var obj = try JsAny.newObject(alloc);
    defer obj.deinit(alloc);

    // Mark this object as an instance of "Dog"
    try obj.set("__jsClass__", JsAny.fromString("Dog"), alloc);

    // dog instanceof Dog === true
    try std.testing.expect(instanceOf(obj, "Dog"));
    // dog instanceof Object === true
    try std.testing.expect(instanceOf(obj, "Object"));
    // dog instanceof Cat === false
    try std.testing.expect(!instanceOf(obj, "Cat"));
}

test "instanceOf custom class with extends" {
    const alloc = std.testing.allocator;
    var obj = try JsAny.newObject(alloc);
    defer obj.deinit(alloc);

    // Mark as "Husky" extends "Dog"
    try obj.set("__jsClass__", JsAny.fromString("Husky"), alloc);
    try obj.set("__jsExtends__", JsAny.fromString("Dog"), alloc);

    // husky instanceof Husky === true
    try std.testing.expect(instanceOf(obj, "Husky"));
    // husky instanceof Object === true
    try std.testing.expect(instanceOf(obj, "Object"));
    // husky instanceof Dog === true (via __jsExtends__)
    try std.testing.expect(instanceOf(obj, "Dog"));
    // husky instanceof Cat === false
    try std.testing.expect(!instanceOf(obj, "Cat"));
}

test "instanceOf primitives are always false" {
    // In JS, primitive values are never instanceof their wrapper types
    try std.testing.expect(!instanceOf(JsAny.fromI64(42), "Number"));
    try std.testing.expect(!instanceOf(JsAny.fromF64(3.14), "Number"));
    try std.testing.expect(!instanceOf(JsAny.fromString("hello"), "String"));
    try std.testing.expect(!instanceOf(JsAny.fromBool(true), "Boolean"));
    // Primitives are not instanceof Object either
    try std.testing.expect(!instanceOf(JsAny.fromI64(42), "Object"));
    try std.testing.expect(!instanceOf(JsAny.fromString("hello"), "Object"));
}

test "instanceOf null and undefined" {
    try std.testing.expect(!instanceOf(JsAny.fromNull(), "Object"));
    try std.testing.expect(!instanceOf(JsAny.undefined_value, "Object"));
}

test "JsAny.format array and object output (R12-P1-2)" {
    const alloc = std.testing.allocator;

    // Array: JS Array.toString() → comma-join, no brackets
    var arr = try JsAny.newArray(alloc);
    defer arr.deinit(alloc);
    try arr.arrayPush(alloc, JsAny.fromI64(1));
    try arr.arrayPush(alloc, JsAny.fromI64(2));
    try arr.arrayPush(alloc, JsAny.fromI64(3));
    const arr_str = try std.fmt.allocPrint(alloc, "{f}", .{arr});
    defer alloc.free(arr_str);
    try std.testing.expectEqualStrings("1,2,3", arr_str);

    // Array with null/undefined elements → empty string in join
    var arr2 = try JsAny.newArray(alloc);
    defer arr2.deinit(alloc);
    try arr2.arrayPush(alloc, JsAny.fromI64(1));
    try arr2.arrayPush(alloc, JsAny.fromNull());
    try arr2.arrayPush(alloc, JsAny.fromI64(3));
    const arr2_str = try std.fmt.allocPrint(alloc, "{f}", .{arr2});
    defer alloc.free(arr2_str);
    try std.testing.expectEqualStrings("1,,3", arr2_str);

    // Object: JS Object.toString() → "[object Object]"
    var obj = try JsAny.newObject(alloc);
    defer obj.deinit(alloc);
    try obj.set("x", JsAny.fromI64(42), alloc);
    const obj_str = try std.fmt.allocPrint(alloc, "{f}", .{obj});
    defer alloc.free(obj_str);
    try std.testing.expectEqualStrings("[object Object]", obj_str);

    // Null
    const null_str = try std.fmt.allocPrint(alloc, "{f}", .{JsAny.fromNull()});
    defer alloc.free(null_str);
    try std.testing.expectEqualStrings("null", null_str);
}

test "JsAny.add string concat with non-string operand (R8-P1-10)" {
    const alloc = std.testing.allocator;

    // string + int → no leak with std.testing.allocator
    const result1 = JsAny.fromString("x").add(JsAny.fromI64(42), alloc);
    defer alloc.free(result1.value.string);
    try std.testing.expectEqualStrings("x42", result1.value.string);

    // int + string → no leak
    const result2 = JsAny.fromI64(7).add(JsAny.fromString("y"), alloc);
    defer alloc.free(result2.value.string);
    try std.testing.expectEqualStrings("7y", result2.value.string);

    // string + string → no alloc for operands, only for result
    const result3 = JsAny.fromString("hello").add(JsAny.fromString(" world"), alloc);
    defer alloc.free(result3.value.string);
    try std.testing.expectEqualStrings("hello world", result3.value.string);
}

test "JsAny.add fallback concat with array operand (R8-P1-10)" {
    const alloc = std.testing.allocator;

    // array + string (fallback path) — no leak
    var arr = try JsAny.newArray(alloc);
    defer arr.deinit(alloc);
    try arr.arrayPush(alloc, JsAny.fromI64(1));
    try arr.arrayPush(alloc, JsAny.fromI64(2));
    const result = arr.add(JsAny.fromString("x"), alloc);
    defer alloc.free(result.value.string);
    try std.testing.expectEqualStrings("1,2x", result.value.string);
}

test "JsAny.eq reference identity for arrays and objects (R8-P1-15)" {
    const alloc = std.testing.allocator;

    // Two distinct arrays with same contents are NOT equal (reference identity)
    var arr1 = try JsAny.newArray(alloc);
    defer arr1.deinit(alloc);
    try arr1.arrayPush(alloc, JsAny.fromI64(1));
    try arr1.arrayPush(alloc, JsAny.fromI64(2));
    try arr1.arrayPush(alloc, JsAny.fromI64(3));

    var arr2 = try JsAny.newArray(alloc);
    defer arr2.deinit(alloc);
    try arr2.arrayPush(alloc, JsAny.fromI64(1));
    try arr2.arrayPush(alloc, JsAny.fromI64(2));
    try arr2.arrayPush(alloc, JsAny.fromI64(3));

    try std.testing.expect(!arr1.eq(arr2)); // distinct → false
    try std.testing.expect(!arr1.strictEq(arr2)); // distinct → false

    // Same reference IS equal
    try std.testing.expect(arr1.eq(arr1)); // same pointer → true
    try std.testing.expect(arr1.strictEq(arr1)); // same pointer → true

    // Two distinct empty objects are NOT equal
    var obj1 = try JsAny.newObject(alloc);
    defer obj1.deinit(alloc);
    var obj2 = try JsAny.newObject(alloc);
    defer obj2.deinit(alloc);

    try std.testing.expect(!obj1.eq(obj2)); // distinct → false
    try std.testing.expect(!obj1.strictEq(obj2)); // distinct → false
    try std.testing.expect(obj1.eq(obj1)); // same pointer → true

    // Array vs object → false
    try std.testing.expect(!arr1.eq(obj1));
    try std.testing.expect(!arr1.strictEq(obj1));

    // Array vs primitive → false
    try std.testing.expect(!arr1.eq(JsAny.fromI64(3)));
    try std.testing.expect(!arr1.strictEq(JsAny.fromI64(3)));

    // Object vs null → false
    try std.testing.expect(!obj1.eq(JsAny.fromNull()));
    try std.testing.expect(!obj1.strictEq(JsAny.fromNull()));

    // JsAny.null == JsAny.null → true; JsAny.null == JsAny.value(.null) → true
    try std.testing.expect(JsAny.fromNull().eq(JsAny.fromNull()));
    try std.testing.expect(JsAny.fromNull().eq(JsAny.fromValue(.null)));
}

test "JsAny.lt/le/gt/ge string lexicographic comparison (R8-P1-16)" {
    // String < string → lexicographic
    const a = JsAny.fromString("apple");
    const b = JsAny.fromString("banana");
    try std.testing.expect(a.lt(b)); // "apple" < "banana"
    try std.testing.expect(!b.lt(a)); // "banana" < "apple" → false
    try std.testing.expect(a.le(b));
    try std.testing.expect(b.gt(a));
    try std.testing.expect(b.ge(a));

    // Equal strings
    const c = JsAny.fromString("hello");
    const d = JsAny.fromString("hello");
    try std.testing.expect(!c.lt(d)); // equal → not less
    try std.testing.expect(c.le(d)); // equal → le is true
    try std.testing.expect(c.ge(d));
    try std.testing.expect(!c.gt(d));

    // Prefix: "abc" < "abcd"
    const e = JsAny.fromString("abc");
    const f = JsAny.fromString("abcd");
    try std.testing.expect(e.lt(f));
    try std.testing.expect(!f.lt(e));

    // Non-string: numeric comparison still works
    const x = JsAny.fromI64(3);
    const y = JsAny.fromI64(5);
    try std.testing.expect(x.lt(y));
    try std.testing.expect(!y.lt(x));
    try std.testing.expect(x.le(y));
    try std.testing.expect(y.gt(x));
}
