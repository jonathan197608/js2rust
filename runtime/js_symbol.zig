//! JS Symbol type for Zig runtime.
//! Implements:
//!   - Symbol(description) — unique symbol with optional description
//!   - Symbol.for(key)    — global symbol registry (shared across all code)
//!   - Symbol.keyFor(sym) — retrieve the key for a registry symbol
//!   - Well-known symbols  — Symbol.iterator, Symbol.asyncIterator, etc.
//!
//! Threading note: the global registry and id counter are NOT thread-safe.
//! This is intentional — generated JS→Zig code runs single-threaded.
//!
//! MDN: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Symbol

const std = @import("std");
const Allocator = std.mem.Allocator;

/// Global symbol counter for unique Symbol() creation.
var next_symbol_id: u64 = 0;

/// Reserved ID range for well-known symbols (0..99).
/// User-created symbols start from 100.
const FIRST_USER_SYMBOL_ID: u64 = 100;

/// Well-known symbol IDs.
pub const WELL_KNOWN = struct {
    pub const iterator: u64 = 0;
    pub const async_iterator: u64 = 1;
    pub const has_instance: u64 = 2;
    pub const is_concat_spreadable: u64 = 3;
    pub const species: u64 = 4;
    pub const to_primitive: u64 = 5;
    pub const to_string_tag: u64 = 6;
    pub const unscopables: u64 = 7;
    pub const match: u64 = 8;
    pub const match_all: u64 = 9;
    pub const replace: u64 = 10;
    pub const search: u64 = 11;
    pub const split: u64 = 12;
    pub const dispose: u64 = 13;
};

/// JsSymbol — represents a JavaScript Symbol value.
/// Each symbol has a unique id and optional description.
pub const JsSymbol = struct {
    /// Unique identifier for this symbol.
    id: u64,

    /// Optional description string (may be null for anonymous symbols).
    description: ?[]const u8,

    /// Create a new unique Symbol with optional description.
    /// Equivalent to JS: Symbol([description])
    /// Requires initRegistry() to have been called first.
    pub fn init(description: ?[]const u8) JsSymbol {
        const alloc = getGlobalAlloc();
        const new_id = getNextId();
        // Duplicate description so the symbol owns its memory
        const desc: ?[]const u8 = if (description) |d|
            alloc.dupe(u8, d) catch @panic("JsSymbol.init: out of memory")
        else
            null;
        return JsSymbol{ .id = new_id, .description = desc };
    }

    /// Create a new unique Symbol without description.
    /// Equivalent to JS: Symbol()
    pub fn initAnonymous() JsSymbol {
        return JsSymbol{ .id = getNextId(), .description = null };
    }

    /// Get the description string, or null if none.
    pub fn getDescription(self: JsSymbol) ?[]const u8 {
        return self.description;
    }

    /// Return a string representation of this symbol.
    /// JS: symbol.toString() → "Symbol(description)" or "Symbol()"
    pub fn toString(self: JsSymbol, alloc: Allocator) ![]const u8 {
        if (self.description) |d| {
            return try std.fmt.allocPrint(alloc, "Symbol({s})", .{d});
        } else {
            return try alloc.dupe(u8, "Symbol()");
        }
    }

    /// Free the symbol's owned description (if any).
    pub fn deinit(self: *JsSymbol, alloc: Allocator) void {
        if (self.description) |d| {
            alloc.free(d);
            self.description = null;
        }
    }

    /// Equality comparison: two symbols are equal iff they have the same id.
    pub fn eql(self: JsSymbol, other: JsSymbol) bool {
        return self.id == other.id;
    }
};

/// Generate the next unique symbol id.
/// Must be called after initRegistry() to ensure FIRST_USER_SYMBOL_ID baseline.
fn getNextId() u64 {
    // Ensure user symbols start from FIRST_USER_SYMBOL_ID
    if (next_symbol_id < FIRST_USER_SYMBOL_ID) {
        next_symbol_id = FIRST_USER_SYMBOL_ID;
    }
    const id = next_symbol_id;
    next_symbol_id += 1;
    return id;
}

// ── Global Symbol Registry ──────────────────────────────────────
// Maps string keys → symbol ids for Symbol.for() / Symbol.keyFor().
// NOT thread-safe — generated code is single-threaded.
// ─────────────────────────────────────────────────────────────────

var registry: ?std.StringHashMap(u64) = null;
var registry_alloc: ?Allocator = null;

/// Initialize the global symbol registry. Must be called once before
/// using Symbol.for(), Symbol.keyFor(), or JsSymbol.init().
pub fn initRegistry(alloc: Allocator) void {
    if (registry != null) return;
    registry = std.StringHashMap(u64).init(alloc);
    registry_alloc = alloc;
}

/// Deinitialize the global symbol registry. Frees all stored keys.
pub fn deinitRegistry() void {
    if (registry) |*r| {
        const alloc = registry_alloc.?;
        // Free all keys stored in the hash map (we own them)
        var iter = r.iterator();
        while (iter.next()) |entry| {
            alloc.free(entry.key_ptr.*);
        }
        r.deinit();
        registry = null;
    }
    registry_alloc = null;
}

/// Get the global allocator reference.
fn getGlobalAlloc() Allocator {
    return registry_alloc orelse @panic("js_symbol: registry not initialized. Call initRegistry() first.");
}

/// Symbol.for(key) — returns an existing symbol for the given key,
/// or creates a new one if none exists.
/// JS: Symbol.for("key") → Symbol
pub fn symbolFor(key: []const u8) JsSymbol {
    if (registry) |*r| {
        const alloc = registry_alloc orelse @panic("js_symbol: registry allocator not set.");

        if (r.get(key)) |existing_id| {
            return JsSymbol{ .id = existing_id, .description = alloc.dupe(u8, key) catch @panic("js_symbol.symbolFor: out of memory") };
        }

        // Create new symbol for this key
        const id = getNextId();
        const key_copy = alloc.dupe(u8, key) catch @panic("js_symbol.symbolFor: out of memory");
        r.put(key_copy, id) catch @panic("js_symbol.symbolFor: out of memory");
        const desc = alloc.dupe(u8, key) catch @panic("js_symbol.symbolFor: out of memory");
        return JsSymbol{ .id = id, .description = desc };
    }
    @panic("js_symbol: registry not initialized. Call initRegistry() first.");
}

/// Symbol.keyFor(sym) — returns the key for a symbol in the global registry,
/// or null if the symbol was not created by Symbol.for().
/// JS: Symbol.keyFor(sym) → string | undefined
pub fn symbolKeyFor(sym: JsSymbol) ?[]const u8 {
    const r = registry orelse return null;

    var iter = r.iterator();
    while (iter.next()) |entry| {
        if (entry.value_ptr.* == sym.id) {
            // Return the key pointer — caller must not free it
            return entry.key_ptr.*;
        }
    }
    return null;
}

// ── Well-known Symbol helpers ───────────────────────────────────
// These return pre-constructed JsSymbol values for the well-known symbols.

/// Symbol.iterator
pub fn symbolIterator() JsSymbol {
    return JsSymbol{ .id = WELL_KNOWN.iterator, .description = null };
}

/// Symbol.asyncIterator
pub fn symbolAsyncIterator() JsSymbol {
    return JsSymbol{ .id = WELL_KNOWN.async_iterator, .description = null };
}

/// Symbol.hasInstance
pub fn symbolHasInstance() JsSymbol {
    return JsSymbol{ .id = WELL_KNOWN.has_instance, .description = null };
}

/// Symbol.isConcatSpreadable
pub fn symbolIsConcatSpreadable() JsSymbol {
    return JsSymbol{ .id = WELL_KNOWN.is_concat_spreadable, .description = null };
}

/// Symbol.species
pub fn symbolSpecies() JsSymbol {
    return JsSymbol{ .id = WELL_KNOWN.species, .description = null };
}

/// Symbol.toPrimitive
pub fn symbolToPrimitive() JsSymbol {
    return JsSymbol{ .id = WELL_KNOWN.to_primitive, .description = null };
}

/// Symbol.toStringTag
pub fn symbolToStringTag() JsSymbol {
    return JsSymbol{ .id = WELL_KNOWN.to_string_tag, .description = null };
}

/// Symbol.unscopables
pub fn symbolUnscopables() JsSymbol {
    return JsSymbol{ .id = WELL_KNOWN.unscopables, .description = null };
}

// ── Tests ────────────────────────────────────────────────────────

test "JsSymbol init with description" {
    const alloc = std.testing.allocator;
    initRegistry(alloc);
    defer deinitRegistry();

    const sym = JsSymbol.init("test");
    defer {
        var s = sym;
        s.deinit(alloc);
    }
    try std.testing.expect(sym.id >= FIRST_USER_SYMBOL_ID);
    try std.testing.expectEqualStrings("test", sym.description.?);
}

test "JsSymbol init anonymous" {
    const alloc = std.testing.allocator;
    initRegistry(alloc);
    defer deinitRegistry();

    const sym = JsSymbol.initAnonymous();
    try std.testing.expect(sym.id >= FIRST_USER_SYMBOL_ID);
    try std.testing.expect(sym.description == null);
}

test "JsSymbol unique ids" {
    const alloc = std.testing.allocator;
    initRegistry(alloc);
    defer deinitRegistry();

    const a = JsSymbol.initAnonymous();
    const b = JsSymbol.initAnonymous();
    try std.testing.expect(a.id != b.id);
}

test "JsSymbol eql" {
    const a = JsSymbol{ .id = 100, .description = null };
    const b = JsSymbol{ .id = 100, .description = null };
    const c = JsSymbol{ .id = 101, .description = null };
    try std.testing.expect(a.eql(b));
    try std.testing.expect(!a.eql(c));
}

test "JsSymbol toString" {
    const alloc = std.testing.allocator;
    initRegistry(alloc);
    defer deinitRegistry();

    var sym = JsSymbol.init("foo");
    defer sym.deinit(alloc);

    const s = try sym.toString(alloc);
    defer alloc.free(s);
    try std.testing.expectEqualStrings("Symbol(foo)", s);
}

test "JsSymbol toString anonymous" {
    const alloc = std.testing.allocator;
    const sym = JsSymbol{ .id = 200, .description = null };
    const s = try sym.toString(alloc);
    defer alloc.free(s);
    try std.testing.expectEqualStrings("Symbol()", s);
}

test "Symbol.for basic" {
    const alloc = std.testing.allocator;
    initRegistry(alloc);
    defer deinitRegistry();

    const sym1 = symbolFor("shared");
    defer {
        var s = sym1;
        s.deinit(alloc);
    }
    const sym2 = symbolFor("shared");
    defer {
        var s = sym2;
        s.deinit(alloc);
    }
    try std.testing.expect(sym1.eql(sym2));
}

test "Symbol.for different keys" {
    const alloc = std.testing.allocator;
    initRegistry(alloc);
    defer deinitRegistry();

    const sym_a = symbolFor("a");
    defer {
        var s = sym_a;
        s.deinit(alloc);
    }
    const sym_b = symbolFor("b");
    defer {
        var s = sym_b;
        s.deinit(alloc);
    }
    try std.testing.expect(!sym_a.eql(sym_b));
}

test "Symbol.keyFor returns null for non-registered symbol" {
    const alloc = std.testing.allocator;
    initRegistry(alloc);
    defer deinitRegistry();

    // Create a symbol that's NOT in the registry
    const local_sym = JsSymbol.initAnonymous();
    const result = symbolKeyFor(local_sym);
    try std.testing.expect(result == null);
}

test "Symbol.keyFor finds registered symbol" {
    const alloc = std.testing.allocator;
    initRegistry(alloc);
    defer deinitRegistry();

    const sym = symbolFor("registered");
    defer {
        var s = sym;
        s.deinit(alloc);
    }
    const key = symbolKeyFor(sym);
    try std.testing.expect(key != null);
    if (key) |k| {
        try std.testing.expectEqualStrings("registered", k);
    }
}

test "well-known symbols have distinct ids" {
    const a = symbolIterator();
    const b = symbolAsyncIterator();
    try std.testing.expect(!a.eql(b));
    try std.testing.expectEqual(@as(u64, WELL_KNOWN.iterator), a.id);
    try std.testing.expectEqual(@as(u64, WELL_KNOWN.async_iterator), b.id);
}
