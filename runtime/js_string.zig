//! JS String method implementations for Zig.
//! All allocating functions take `alloc: std.mem.Allocator` as first parameter.

const std = @import("std");
const Allocator = std.mem.Allocator;
const JsAny = @import("jsany.zig").JsAny;

// ── Host C ABI function declarations ──
// Regex-dependent extern declarations have been moved to js_string_regex.zig.
// They are only needed when needs_regex = true.

// ── Internal UTF-8/UTF-16 helpers ──

/// Decode a single UTF-8 code point starting at position i.
/// Returns the decoded code point and the byte sequence length, or null if invalid.
fn decodeUtf8CodePoint(s: []const u8, i: usize) ?struct { code_point: u32, len: u8 } {
    if (i >= s.len) return null;
    const c = s[i];
    var code_point: u32 = 0;
    var seq_len: u8 = 1;

    if (c & 0x80 == 0) {
        // 1-byte: 0xxxxxxx (ASCII)
        code_point = c;
        seq_len = 1;
    } else if (c & 0xE0 == 0xC0) {
        // 2-byte: 110xxxxx 10xxxxxx
        if (i + 1 >= s.len) return null;
        code_point = (@as(u32, c & 0x1F) << 6) | @as(u32, s[i + 1] & 0x3F);
        seq_len = 2;
    } else if (c & 0xF0 == 0xE0) {
        // 3-byte: 1110xxxx 10xxxxxx 10xxxxxx
        if (i + 2 >= s.len) return null;
        code_point = (@as(u32, c & 0x0F) << 12) | (@as(u32, s[i + 1] & 0x3F) << 6) | @as(u32, s[i + 2] & 0x3F);
        seq_len = 3;
    } else if (c & 0xF8 == 0xF0) {
        // 4-byte: 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx
        if (i + 3 >= s.len) return null;
        code_point = (@as(u32, c & 0x07) << 18) | (@as(u32, s[i + 1] & 0x3F) << 12) | (@as(u32, s[i + 2] & 0x3F) << 6) | @as(u32, s[i + 3] & 0x3F);
        seq_len = 4;
    } else {
        return null;
    }

    return .{ .code_point = code_point, .len = seq_len };
}

/// Number of UTF-16 code units needed to represent a Unicode code point.
fn utf16CodeUnitCount(code_point: u32) usize {
    return if (code_point <= 0xFFFF) 1 else 2;
}

// ── Public UTF-16 helper functions ──

/// Count the number of UTF-16 code units in a UTF-8 string.
/// JS string.length returns UTF-16 code unit count, not byte count.
pub fn utf16Len(s: []const u8) usize {
    var len: usize = 0;
    var i: usize = 0;
    while (i < s.len) {
        const decoded = decodeUtf8CodePoint(s, i) orelse {
            i += 1;
            continue;
        };
        len += utf16CodeUnitCount(decoded.code_point);
        i += decoded.len;
    }
    return len;
}

/// Convert a UTF-16 code unit index to a byte offset in the UTF-8 string.
/// Returns null if the index is out of bounds (>= utf16Len).
/// Passing idx == utf16Len returns s.len (one-past-end, for slicing).
/// For supplementary characters, the low surrogate index maps to the same
/// byte offset as the high surrogate (both are part of the same code point).
pub fn utf16IndexToByteOffset(s: []const u8, idx: usize) ?usize {
    var utf16_idx: usize = 0;
    var i: usize = 0;
    while (i < s.len) {
        const code_point_start = i;
        const decoded = decodeUtf8CodePoint(s, i) orelse {
            i += 1;
            continue;
        };
        const cu_count = utf16CodeUnitCount(decoded.code_point);
        // Check if idx falls within this code point's UTF-16 code units
        if (idx >= utf16_idx and idx < utf16_idx + cu_count) {
            return code_point_start; // Return byte offset of the code point
        }
        utf16_idx += cu_count;
        i += decoded.len;
    }
    // idx is exactly at the end (one-past-end)
    if (idx == utf16_idx) return i;
    return null;
}

/// Convert a byte offset in a UTF-8 string to the corresponding UTF-16 code unit index.
pub fn byteOffsetToUtf16Index(s: []const u8, byte_off: usize) usize {
    var utf16_idx: usize = 0;
    var i: usize = 0;
    while (i < byte_off and i < s.len) {
        const decoded = decodeUtf8CodePoint(s, i) orelse {
            i += 1;
            continue;
        };
        utf16_idx += utf16CodeUnitCount(decoded.code_point);
        i += decoded.len;
    }
    return utf16_idx;
}

/// Return the byte slice containing exactly the first `n` UTF-16 code units of `s`.
/// Used by padStart/padEnd for partial padding truncation.
pub fn firstUtf16CodeUnits(s: []const u8, n: usize) []const u8 {
    var utf16_idx: usize = 0;
    var i: usize = 0;
    while (i < s.len and utf16_idx < n) {
        const decoded = decodeUtf8CodePoint(s, i) orelse {
            i += 1;
            continue;
        };
        const cu_count = utf16CodeUnitCount(decoded.code_point);
        if (utf16_idx + cu_count > n) break; // would overshoot
        utf16_idx += cu_count;
        i += decoded.len;
    }
    return s[0..i];
}

/// Encode a single UTF-16 code unit as a UTF-8 string.
/// For BMP characters this produces standard UTF-8.
/// For surrogate code points (0xD800-0xDFFF) this produces CESU-8 (3-byte encoding),
/// which matches JS semantics where charAt can return a lone surrogate.
fn encodeCodeUnit(alloc: Allocator, cu: u16) ![]const u8 {
    const cp: u32 = cu;
    if (cp <= 0x7F) {
        const result = try alloc.alloc(u8, 1);
        result[0] = @intCast(cp);
        return result;
    } else if (cp <= 0x7FF) {
        const result = try alloc.alloc(u8, 2);
        result[0] = @intCast(0xC0 | (cp >> 6));
        result[1] = @intCast(0x80 | (cp & 0x3F));
        return result;
    } else {
        // BMP including surrogates → 3-byte encoding (CESU-8 for surrogates)
        const result = try alloc.alloc(u8, 3);
        result[0] = @intCast(0xE0 | (cp >> 12));
        result[1] = @intCast(0x80 | ((cp >> 6) & 0x3F));
        result[2] = @intCast(0x80 | (cp & 0x3F));
        return result;
    }
}

/// Convert string to uppercase. Returns newly allocated string.
pub fn toUpper(alloc: Allocator, s: []const u8) ![]const u8 {
    const result = try alloc.alloc(u8, s.len);
    for (s, 0..) |c, i| {
        result[i] = std.ascii.toUpper(c);
    }
    return result;
}

/// Convert string to lowercase. Returns newly allocated string.
pub fn toLower(alloc: Allocator, s: []const u8) ![]const u8 {
    const result = try alloc.alloc(u8, s.len);
    for (s, 0..) |c, i| {
        result[i] = std.ascii.toLower(c);
    }
    return result;
}

/// Get character at index, returned as a string.
/// Uses UTF-16 code unit indexing (JS semantics).
/// For supplementary characters, charAt at the high surrogate index returns the
/// high surrogate as a CESU-8 string; charAt at the low surrogate index returns
/// the low surrogate as a CESU-8 string.
pub fn charAt(alloc: Allocator, s: []const u8, idx: i64) ![]const u8 {
    if (idx < 0) return &[0]u8{};
    const target: usize = @intCast(idx);
    var utf16_idx: usize = 0;
    var i: usize = 0;
    while (i < s.len) {
        const decoded = decodeUtf8CodePoint(s, i) orelse {
            i += 1;
            continue;
        };
        if (decoded.code_point <= 0xFFFF) {
            // BMP character: 1 UTF-16 code unit
            if (utf16_idx == target) {
                return encodeCodeUnit(alloc, @intCast(decoded.code_point));
            }
            utf16_idx += 1;
        } else {
            // Supplementary character: 2 UTF-16 code units (surrogate pair)
            const high: u16 = @intCast(0xD800 + ((decoded.code_point - 0x10000) >> 10));
            const low: u16 = @intCast(0xDC00 + ((decoded.code_point - 0x10000) & 0x3FF));
            if (utf16_idx == target) {
                return encodeCodeUnit(alloc, high);
            }
            if (utf16_idx + 1 == target) {
                return encodeCodeUnit(alloc, low);
            }
            utf16_idx += 2;
        }
        i += decoded.len;
    }
    return &[0]u8{}; // Out of bounds
}

/// Get character at index (supports negative indices), returned as a string.
/// Negative indices count from the end: at(-1) returns the last character.
/// Uses UTF-16 code unit indexing (JS semantics).
pub fn at(alloc: Allocator, s: []const u8, idx: i64) ![]const u8 {
    const len: i64 = @intCast(utf16Len(s));
    const adjusted_idx: i64 = if (idx < 0) len + idx else idx;
    if (adjusted_idx < 0 or adjusted_idx >= len) return &[0]u8{};
    return charAt(alloc, s, adjusted_idx);
}

/// Get UTF-16 code unit at index (JS charCodeAt behavior).
/// Returns the i-th UTF-16 code unit (0-65535).
/// If idx is out of bounds, returns 0 (JS returns NaN, but we return 0 for type simplicity).
pub fn charCodeAt(s: []const u8, idx: i64) u16 {
    const target: usize = @intCast(@max(0, idx));
    var utf16_idx: usize = 0;
    var i: usize = 0;

    while (i < s.len) {
        const decoded = decodeUtf8CodePoint(s, i) orelse {
            i += 1;
            continue;
        };

        if (decoded.code_point <= 0xFFFF) {
            // BMP character: 1 UTF-16 code unit
            if (utf16_idx == target) {
                return @intCast(decoded.code_point);
            }
            utf16_idx += 1;
        } else {
            // Supplementary plane character: 2 UTF-16 code units (surrogate pair)
            const high: u16 = @intCast(0xD800 + ((decoded.code_point - 0x10000) >> 10));
            const low: u16 = @intCast(0xDC00 + ((decoded.code_point - 0x10000) & 0x3FF));
            if (utf16_idx == target) {
                return high;
            }
            if (utf16_idx + 1 == target) {
                return low;
            }
            utf16_idx += 2;
        }

        i += decoded.len;
    }

    return 0; // Out of bounds
}

/// Get Unicode code point at index (JS codePointAt behavior).
/// Returns the Unicode code point (U+0000 to U+10FFFF) as i64.
/// If idx is out of bounds, returns 0 (JS returns undefined, but we return 0 for type simplicity).
/// Unlike charCodeAt(), this correctly handles surrogate pairs:
/// - If the index points to a high surrogate (0xD800-0xDBFF) and the next code unit
///   is a low surrogate (0xDC00-0xDFFF), decodes the pair and returns the full code point.
pub fn codePointAt(s: []const u8, idx: i64) i64 {
    const target: usize = @intCast(@max(0, idx));
    var utf16_idx: usize = 0;
    var i: usize = 0;

    while (i < s.len) {
        const decoded = decodeUtf8CodePoint(s, i) orelse {
            i += 1;
            continue;
        };

        if (decoded.code_point <= 0xFFFF) {
            // BMP character: 1 UTF-16 code unit
            if (utf16_idx == target) {
                return @intCast(decoded.code_point);
            }
            utf16_idx += 1;
        } else {
            // Supplementary plane character: 2 UTF-16 code units (surrogate pair)
            if (utf16_idx == target) {
                // Return the full code point (not just the high surrogate)
                return @intCast(decoded.code_point);
            }
            // Skip the low surrogate (it's part of the same code point)
            utf16_idx += 2;
        }

        i += decoded.len;
    }

    return 0; // Out of bounds
}

/// Concatenate two strings. Returns newly allocated string.
pub fn concat(alloc: Allocator, a: []const u8, b: []const u8) ![]const u8 {
    const result = try alloc.alloc(u8, a.len + b.len);
    @memcpy(result[0..a.len], a);
    @memcpy(result[a.len..], b);
    return result;
}

/// Check if haystack contains needle.
pub fn includes(haystack: []const u8, needle: []const u8) bool {
    return std.mem.indexOf(u8, haystack, needle) != null;
}

/// Find index of needle in haystack starting at UTF-16 index `from_index`,
/// or -1 if not found. Returns UTF-16 code unit index (JS semantics).
///
/// R8-P1-19: support optional `from_index` (ECMA-262 22.1.3.18).
/// - `from_index < 0` → clamped to 0 (search from the start)
/// - `from_index > len` → return -1 (or `len` if needle is empty)
/// - empty needle → returns the clamped start position
/// The emitter always supplies `from_index` (default `0` when JS omits it),
/// matching the `slice` convention; direct Zig callers must pass it too.
pub fn indexOf(haystack: []const u8, needle: []const u8, from_index: i64) i64 {
    const hay_len: i64 = @intCast(utf16Len(haystack));
    // start = clamp(from_index, 0, len)
    const start: i64 = if (from_index < 0) 0 else if (from_index > hay_len) hay_len else from_index;
    if (needle.len == 0) return start;
    if (start >= hay_len) return -1;
    // start is now in [0, hay_len), so the offset is non-null.
    const start_byte: usize = utf16IndexToByteOffset(haystack, @intCast(start)).?;
    if (std.mem.indexOf(u8, haystack[start_byte..], needle)) |pos| {
        return @intCast(byteOffsetToUtf16Index(haystack, start_byte + pos));
    }
    return -1;
}

/// Check if s starts with prefix.
pub fn startsWith(s: []const u8, prefix: []const u8) bool {
    return std.mem.startsWith(u8, s, prefix);
}

/// Check if s ends with suffix.
pub fn endsWith(s: []const u8, suffix: []const u8) bool {
    return std.mem.endsWith(u8, s, suffix);
}

/// Extract substring from start (inclusive) to end (exclusive).
/// Negative indices count from the end. Returns borrowed slice (no allocation).
/// Uses UTF-16 code unit indexing (JS semantics).
pub fn slice(s: []const u8, start: i64, end: i64) []const u8 {
    const len: i64 = @intCast(utf16Len(s));
    var st: i64 = start;
    var en: i64 = end;

    if (st < 0) st = @max(0, len + st);
    if (en < 0) en = @max(0, len + en);

    st = @min(@max(0, st), len);
    en = @min(@max(0, en), len);
    if (st >= en) return &[0]u8{};

    const byte_start = utf16IndexToByteOffset(s, @intCast(st)) orelse s.len;
    const byte_end = utf16IndexToByteOffset(s, @intCast(en)) orelse s.len;
    return s[byte_start..byte_end];
}

/// Extract substring from startIndex to endIndex (JS substring semantics).
/// - If either arg is negative or NaN, treat as 0.
/// - If either arg > length, treat as length.
/// - If startIndex > endIndex, swap them.
/// Returns borrowed slice (no allocation).
/// Uses UTF-16 code unit indexing (JS semantics).
pub fn substring(s: []const u8, start: i64, end: i64) []const u8 {
    const len: i64 = @intCast(utf16Len(s));
    var st: i64 = start;
    var en: i64 = end;

    // Clamp negative → 0
    if (st < 0) st = 0;
    if (en < 0) en = 0;

    // Clamp > length → length
    st = @min(st, len);
    en = @min(en, len);

    // Swap if start > end
    if (st > en) {
        const tmp = st;
        st = en;
        en = tmp;
    }

    const byte_start = utf16IndexToByteOffset(s, @intCast(st)) orelse s.len;
    const byte_end = utf16IndexToByteOffset(s, @intCast(en)) orelse s.len;
    return s[byte_start..byte_end];
}

/// Split string by separator. Returns newly allocated array of strings.
///
/// Per ECMAScript spec, `split("")` splits the string into individual code units.
/// Since the runtime strings are UTF-8, we split into UTF-8 code point sequences
/// (equivalent for BMP characters). An empty separator must NOT be fed to
/// `std.mem.indexOf`, which would match at position 0 forever and hang.
pub fn split(alloc: Allocator, s: []const u8, sep: []const u8) ![][]const u8 {
    var parts = std.ArrayList([]const u8).empty;
    errdefer parts.deinit(alloc);

    if (sep.len == 0) {
        // Split into individual UTF-8 code point sequences (matches JS `split("")`
        // for BMP characters; the runtime stores strings as UTF-8). We walk by
        // leading-byte pattern rather than std.unicode.Utf8View to avoid the
        // error-returning `init` and to keep allocations as borrowed slices of
        // the original string buffer.
        var i: usize = 0;
        while (i < s.len) {
            const cp_len: usize = switch (s[i] >> 4) {
                0x0...0x7 => 1,
                0xC...0xD => 2,
                0xE => 3,
                0xF => 4,
                else => 1, // invalid leading byte; consume one byte
            };
            const end = @min(i + cp_len, s.len);
            try parts.append(alloc, s[i..end]);
            i = end;
        }
        return parts.toOwnedSlice(alloc);
    }

    var remaining = s;
    while (std.mem.indexOf(u8, remaining, sep)) |pos| {
        try parts.append(alloc, remaining[0..pos]);
        remaining = remaining[pos + sep.len ..];
    }
    try parts.append(alloc, remaining);

    return parts.toOwnedSlice(alloc);
}

    /// Expand replacement string for plain-string replace per ECMA-262 Table 52.
    ///
    /// For plain-string (non-RegExp) replacement, only these patterns apply:
    /// - `$$` → literal `$`
    /// - `$&` → the matched substring (= `matched`)
    /// - `` $` `` → text before match (= `before`)
    /// - `$'` → text after match (= `after`)
    /// - `$n` / `$<name>` → literal (no capture groups exist)
    ///
    /// Any `$` followed by an unrecognised escape, or a trailing `$`, is
    /// emitted as a literal `$`.
    fn expandReplacementPlain(
        alloc: Allocator,
        replacement: []const u8,
        matched: []const u8,
        before: []const u8,
        after: []const u8,
    ) ![]const u8 {
        var result = std.ArrayList(u8).empty;
        defer result.deinit(alloc);

        var i: usize = 0;
        while (i < replacement.len) {
            if (replacement[i] == '$') {
                if (i + 1 >= replacement.len) {
                    // Trailing $ — literal
                    try result.append(alloc, '$');
                    i += 1;
                    continue;
                }
                switch (replacement[i + 1]) {
                    '$' => {
                        try result.append(alloc, '$');
                        i += 2;
                    },
                    '&' => {
                        try result.appendSlice(alloc, matched);
                        i += 2;
                    },
                    '`' => {
                        try result.appendSlice(alloc, before);
                        i += 2;
                    },
                    '\'' => {
                        try result.appendSlice(alloc, after);
                        i += 2;
                    },
                    '<' => {
                        // $<name> with no RegExp → literal "$<name>"
                        // Check for closing >
                        if (std.mem.indexOfScalar(u8, replacement[i + 2 ..], '>')) |close_rel| {
                            try result.appendSlice(alloc, replacement[i .. i + 2 + close_rel + 1]);
                            i += 2 + close_rel + 1;
                        } else {
                            try result.append(alloc, '$');
                            i += 1;
                        }
                    },
                    else => {
                        // Unrecognised → literal $
                        try result.append(alloc, '$');
                        i += 1;
                    },
                }
            } else {
                // Copy literal run until next $
                const start = i;
                while (i < replacement.len and replacement[i] != '$') {
                    i += 1;
                }
                try result.appendSlice(alloc, replacement[start..i]);
            }
        }

        return result.toOwnedSlice(alloc);
    }

    /// Replace the first occurrence of old with new. Returns newly allocated string.
    /// R8-P1-22: JS String.prototype.replace() (without /g) replaces only the
    /// first occurrence. Previously used std.mem.replaceOwned which replaced ALL
    /// occurrences — identical to replaceAll. Now uses indexOf to find only the
    /// first match, then builds the replacement via allocPrint.
    /// P1-24: Now expands $$ $& $` $' patterns in the replacement string per
    /// ECMA-262 Table 52 for plain-string matches.
    pub fn replace(alloc: Allocator, s: []const u8, old: []const u8, new: []const u8) ![]const u8 {
        if (old.len == 0) {
            // Empty search string → prepend replacement before first char.
            // matched="", before="", after=s
            const expanded = try expandReplacementPlain(alloc, new, "", "", s);
            defer alloc.free(expanded);
            return std.fmt.allocPrint(alloc, "{s}{s}", .{ expanded, s });
        }
        const idx = std.mem.indexOf(u8, s, old) orelse return alloc.dupe(u8, s);
        const matched = s[idx .. idx + old.len];
        const before = s[0..idx];
        const after = s[idx + old.len ..];
        const expanded = try expandReplacementPlain(alloc, new, matched, before, after);
        defer alloc.free(expanded);
        return std.fmt.allocPrint(alloc, "{s}{s}{s}", .{ before, expanded, after });
    }

/// Trim whitespace from both ends. Returns borrowed slice.
pub fn trim(s: []const u8) []const u8 {
    return std.mem.trim(u8, s, &std.ascii.whitespace);
}

/// Trim whitespace from the start (left) of a string.
pub fn trimStart(s: []const u8) []const u8 {
    return std.mem.trimStart(u8, s, &std.ascii.whitespace);
}

/// Trim whitespace from the end (right) of a string.
pub fn trimEnd(s: []const u8) []const u8 {
    return std.mem.trimEnd(u8, s, &std.ascii.whitespace);
}

/// Find the last index of needle in haystack.
/// Returns UTF-16 code unit index as i64, or -1 if not found.
///
/// R8-P1-19: support optional `from_index` (ECMA-262 22.1.3.19).
/// Per spec, the search scans backward starting at position `start` where:
///   - if `from_index >= 0`: `start = min(from_index, len)`
///   - if `from_index <  0`: `start = len + from_index` (may be negative → -1)
/// Empty needle matches at `start` (which may equal `len` when
/// `from_index >= len`).
pub fn lastIndexOf(haystack: []const u8, needle: []const u8, from_index: i64) i64 {
    const hay_len: i64 = @intCast(utf16Len(haystack));
    const start: i64 = if (from_index < 0) hay_len + from_index else @min(from_index, hay_len);
    if (start < 0) return -1;
    if (needle.len == 0) return start;
    if (needle.len > haystack.len) return -1;

    // Scan backward from UTF-16 index `start` to 0, comparing byte slices.
    // Converting UTF-16 idx → byte offset each iteration is O(N) overall for
    // typical inputs; this trades raw speed for correctness under multi-byte
    // sequences (a hot loop optimisation can come later if needed).
    var i: i64 = start;
    while (i >= 0) : (i -= 1) {
        // i is in [0, start] ⊆ [0, hay_len], so the offset is non-null.
        const i_byte: usize = utf16IndexToByteOffset(haystack, @intCast(i)).?;
        if (i_byte + needle.len <= haystack.len) {
            if (std.mem.eql(u8, haystack[i_byte .. i_byte + needle.len], needle)) {
                return i;
            }
        }
    }
    return -1;
}

/// Repeat string n times. Returns newly allocated string.
pub fn repeat(alloc: Allocator, s: []const u8, n: i64) ![]const u8 {
    const count: usize = @intCast(@max(0, n));
    if (count == 0 or s.len == 0) return try alloc.dupe(u8, "");
    // Overflow check: s.len * count must fit in usize.
    // Without this, the multiplication wraps silently and alloc.alloc
    // succeeds with a tiny buffer, causing a heap buffer overflow in the
    // @memcpy loop below.
    if (s.len > std.math.maxInt(usize) / count) return error.Overflow;
    const result = try alloc.alloc(u8, s.len * count);
    var i: usize = 0;
    while (i < count) : (i += 1) {
        @memcpy(result[i * s.len .. (i + 1) * s.len], s);
    }
    return result;
}

/// Pad the start of a string to reach target_len using pad_str repeated.
/// Uses UTF-16 code unit indexing (JS semantics): target_len is in UTF-16 code units.
pub fn padStart(alloc: Allocator, s: []const u8, target_len: i64, pad_str: []const u8) ![]const u8 {
    const s_utf16_len: usize = utf16Len(s);
    const pad_utf16_len: usize = utf16Len(pad_str);
    const target: usize = @intCast(@max(0, target_len));
    if (s_utf16_len >= target or pad_utf16_len == 0) return try alloc.dupe(u8, s);

    const pad_needed: usize = target - s_utf16_len; // in UTF-16 code units
    // Calculate how many full repetitions + partial prefix of pad_str we need
    const full_reps = pad_needed / pad_utf16_len;
    const partial_units = pad_needed % pad_utf16_len;

    // Byte size of the partial prefix of pad_str (first `partial_units` UTF-16 code units)
    const partial_slice = if (partial_units > 0) firstUtf16CodeUnits(pad_str, partial_units) else &[0]u8{};

    const total_pad_bytes = full_reps * pad_str.len + partial_slice.len;
    const result = try alloc.alloc(u8, total_pad_bytes + s.len);
    var pos: usize = 0;

    // Write full repetitions of pad_str
    for (0..full_reps) |_| {
        @memcpy(result[pos..][0..pad_str.len], pad_str);
        pos += pad_str.len;
    }

    // Write partial prefix
    if (partial_units > 0) {
        @memcpy(result[pos..][0..partial_slice.len], partial_slice);
        pos += partial_slice.len;
    }

    // Write original string
    @memcpy(result[pos..], s);
    return result;
}

/// Pad the end of a string to reach target_len using pad_str repeated.
/// Uses UTF-16 code unit indexing (JS semantics): target_len is in UTF-16 code units.
pub fn padEnd(alloc: Allocator, s: []const u8, target_len: i64, pad_str: []const u8) ![]const u8 {
    const s_utf16_len: usize = utf16Len(s);
    const pad_utf16_len: usize = utf16Len(pad_str);
    const target: usize = @intCast(@max(0, target_len));
    if (s_utf16_len >= target or pad_utf16_len == 0) return try alloc.dupe(u8, s);

    const pad_needed: usize = target - s_utf16_len; // in UTF-16 code units
    const full_reps = pad_needed / pad_utf16_len;
    const partial_units = pad_needed % pad_utf16_len;

    const partial_slice = if (partial_units > 0) firstUtf16CodeUnits(pad_str, partial_units) else &[0]u8{};

    const total_pad_bytes = full_reps * pad_str.len + partial_slice.len;
    const result = try alloc.alloc(u8, s.len + total_pad_bytes);

    // Write original string
    @memcpy(result[0..s.len], s);
    var pos: usize = s.len;

    // Write full repetitions of pad_str
    for (0..full_reps) |_| {
        @memcpy(result[pos..][0..pad_str.len], pad_str);
        pos += pad_str.len;
    }

    // Write partial prefix
    if (partial_units > 0) {
        @memcpy(result[pos..][0..partial_slice.len], partial_slice);
        pos += partial_slice.len;
    }

    return result;
}

test "toUpper" {
    const result = try toUpper(std.testing.allocator, "hello");
    defer std.testing.allocator.free(result);
    try std.testing.expectEqualStrings("HELLO", result);
}

test "toLower" {
    const result = try toLower(std.testing.allocator, "HELLO");
    defer std.testing.allocator.free(result);
    try std.testing.expectEqualStrings("hello", result);
}

test "charAt" {
    const result = try charAt(std.testing.allocator, "abc", 1);
    defer std.testing.allocator.free(result);
    try std.testing.expectEqualStrings("b", result);
}

test "concat" {
    const result = try concat(std.testing.allocator, "hello", " world");
    defer std.testing.allocator.free(result);
    try std.testing.expectEqualStrings("hello world", result);
}

test "includes" {
    try std.testing.expect(includes("hello world", "world"));
    try std.testing.expect(!includes("hello world", "xyz"));
}

test "indexOf" {
    try std.testing.expectEqual(@as(i64, 6), indexOf("hello world", "world", 0));
    try std.testing.expectEqual(@as(i64, -1), indexOf("hello world", "xyz", 0));
}

test "indexOf from_index (R8-P1-19)" {
    // Basic from_index usage
    try std.testing.expectEqual(@as(i64, 6), indexOf("hello hello", "hello", 1));
    try std.testing.expectEqual(@as(i64, 0), indexOf("hello hello", "hello", 0));
    // from_index beyond first occurrence skips it
    try std.testing.expectEqual(@as(i64, 6), indexOf("hello hello", "hello", 1));
    // from_index past a run of identical chars still finds later ones
    try std.testing.expectEqual(@as(i64, 2), indexOf("aaabb", "a", 2));
    // from_index beyond all occurrences returns -1
    try std.testing.expectEqual(@as(i64, -1), indexOf("aaabb", "a", 3));
    // Negative from_index clamped to 0
    try std.testing.expectEqual(@as(i64, 0), indexOf("abcabc", "a", -5));
    // from_index >= len returns -1 (non-empty needle)
    try std.testing.expectEqual(@as(i64, -1), indexOf("abc", "a", 3));
    try std.testing.expectEqual(@as(i64, -1), indexOf("abc", "a", 100));
    // Empty needle returns clamped start position
    try std.testing.expectEqual(@as(i64, 0), indexOf("abc", "", 0));
    try std.testing.expectEqual(@as(i64, 0), indexOf("abc", "", -5));
    try std.testing.expectEqual(@as(i64, 3), indexOf("abc", "", 3));
    try std.testing.expectEqual(@as(i64, 3), indexOf("abc", "", 100));
}

test "startsWith" {
    try std.testing.expect(startsWith("hello", "hel"));
    try std.testing.expect(!startsWith("hello", "xyz"));
}

test "endsWith" {
    try std.testing.expect(endsWith("hello", "llo"));
    try std.testing.expect(!endsWith("hello", "hel"));
}

test "slice" {
    try std.testing.expectEqualStrings("ell", slice("hello", 1, 4));
    try std.testing.expectEqualStrings("lo", slice("hello", -2, 5));
}

test "split" {
    const alloc = std.testing.allocator;
    const result = try split(alloc, "a,b,c", ",");
    defer alloc.free(result);
    try std.testing.expectEqual(@as(usize, 3), result.len);
    try std.testing.expectEqualStrings("a", result[0]);
    try std.testing.expectEqualStrings("c", result[2]);
}

test "split empty separator (R8 P0-1)" {
    const alloc = std.testing.allocator;
    // split("") must split into individual code points, not infinite-loop.
    const result = try split(alloc, "abc", "");
    defer alloc.free(result);
    try std.testing.expectEqual(@as(usize, 3), result.len);
    try std.testing.expectEqualStrings("a", result[0]);
    try std.testing.expectEqualStrings("b", result[1]);
    try std.testing.expectEqualStrings("c", result[2]);
}

test "split empty string empty separator (R8 P0-1)" {
    const alloc = std.testing.allocator;
    const result = try split(alloc, "", "");
    defer alloc.free(result);
    try std.testing.expectEqual(@as(usize, 0), result.len);
}

test "replace" {
    // Single occurrence — replaced
    const r1 = try replace(std.testing.allocator, "hello world", "world", "zig");
    defer std.testing.allocator.free(r1);
    try std.testing.expectEqualStrings("hello zig", r1);

    // R8-P1-22: Multiple occurrences — only FIRST is replaced (JS spec)
    const r2 = try replace(std.testing.allocator, "abcabc", "b", "X");
    defer std.testing.allocator.free(r2);
    try std.testing.expectEqualStrings("aXcabc", r2);

    // Not found — original string returned (newly allocated copy)
    const r3 = try replace(std.testing.allocator, "hello", "xyz", "Z");
    defer std.testing.allocator.free(r3);
    try std.testing.expectEqualStrings("hello", r3);

    // Empty search string — replacement prepended
    const r4 = try replace(std.testing.allocator, "abc", "", "X");
    defer std.testing.allocator.free(r4);
    try std.testing.expectEqualStrings("Xabc", r4);
}

test "trim" {
    try std.testing.expectEqualStrings("hello", trim("  hello  "));
}

test "repeat" {
    const result = try repeat(std.testing.allocator, "ab", 3);
    defer std.testing.allocator.free(result);
    try std.testing.expectEqualStrings("ababab", result);
}

test "charCodeAt ASCII" {
    // ASCII characters
    try std.testing.expectEqual(@as(u16, 72), charCodeAt("Hello", 0)); // 'H'
    try std.testing.expectEqual(@as(u16, 101), charCodeAt("Hello", 1)); // 'e'
    try std.testing.expectEqual(@as(u16, 108), charCodeAt("Hello", 2)); // 'l'
    try std.testing.expectEqual(@as(u16, 108), charCodeAt("Hello", 3)); // Second 'l'
    try std.testing.expectEqual(@as(u16, 111), charCodeAt("Hello", 4)); // 'o'
    try std.testing.expectEqual(@as(u16, 0), charCodeAt("Hello", 10)); // Out of bounds
}

test "charCodeAt UTF-8" {
    // Multi-byte UTF-8 characters
    // 'café' - 'c'=99, 'a'=97, 'f'=102, 'é'=U+00E9=233
    try std.testing.expectEqual(@as(u16, 99), charCodeAt("café", 0));
    try std.testing.expectEqual(@as(u16, 97), charCodeAt("café", 1));
    try std.testing.expectEqual(@as(u16, 233), charCodeAt("café", 3)); // 'é' (U+00E9)
}

test "charCodeAt surrogate pair" {
    // Supplementary plane character (U+1F600 = 😀)
    // UTF-16: surrogate pair 0xD83D 0xDE00
    const emoji = "😀";
    const high = charCodeAt(emoji, 0);
    const low = charCodeAt(emoji, 1);
    try std.testing.expectEqual(@as(u16, 0xD83D), high); // High surrogate
    try std.testing.expectEqual(@as(u16, 0xDE00), low); // Low surrogate
}

// ── UTF-16 helper tests ──

test "utf16Len ASCII" {
    try std.testing.expectEqual(@as(usize, 5), utf16Len("hello"));
    try std.testing.expectEqual(@as(usize, 0), utf16Len(""));
}

test "utf16Len multi-byte" {
    // 'café' = 4 UTF-16 code units (c, a, f, é)
    try std.testing.expectEqual(@as(usize, 4), utf16Len("café"));
    // '😀' = 2 UTF-16 code units (surrogate pair)
    try std.testing.expectEqual(@as(usize, 2), utf16Len("😀"));
    // Mixed: 'Hi😀!' = 5 UTF-16 code units (H, i, high, low, !)
    try std.testing.expectEqual(@as(usize, 5), utf16Len("Hi😀!"));
}

test "utf16IndexToByteOffset ASCII" {
    try std.testing.expectEqual(@as(usize, 0), utf16IndexToByteOffset("abc", 0).?);
    try std.testing.expectEqual(@as(usize, 1), utf16IndexToByteOffset("abc", 1).?);
    try std.testing.expectEqual(@as(usize, 2), utf16IndexToByteOffset("abc", 2).?);
    try std.testing.expectEqual(@as(usize, 3), utf16IndexToByteOffset("abc", 3).?); // one-past-end
    try std.testing.expect(utf16IndexToByteOffset("abc", 4) == null); // out of bounds
}

test "utf16IndexToByteOffset multi-byte" {
    // 'café' in UTF-8: c(1) a(1) f(1) é(2) = 5 bytes
    try std.testing.expectEqual(@as(usize, 0), utf16IndexToByteOffset("café", 0).?); // 'c'
    try std.testing.expectEqual(@as(usize, 1), utf16IndexToByteOffset("café", 1).?); // 'a'
    try std.testing.expectEqual(@as(usize, 2), utf16IndexToByteOffset("café", 2).?); // 'f'
    try std.testing.expectEqual(@as(usize, 3), utf16IndexToByteOffset("café", 3).?); // 'é'
    try std.testing.expectEqual(@as(usize, 5), utf16IndexToByteOffset("café", 4).?); // one-past-end
}

test "utf16IndexToByteOffset surrogate pair" {
    // '😀' in UTF-8: 4 bytes, 2 UTF-16 code units
    // Both high and low surrogate map to byte offset 0 (same code point)
    try std.testing.expectEqual(@as(usize, 0), utf16IndexToByteOffset("😀", 0).?); // high surrogate
    try std.testing.expectEqual(@as(usize, 0), utf16IndexToByteOffset("😀", 1).?); // low surrogate → same byte offset
    try std.testing.expectEqual(@as(usize, 4), utf16IndexToByteOffset("😀", 2).?); // one-past-end
    try std.testing.expect(utf16IndexToByteOffset("😀", 3) == null); // out of bounds
}

test "byteOffsetToUtf16Index" {
    // ASCII: 1:1 mapping
    try std.testing.expectEqual(@as(usize, 0), byteOffsetToUtf16Index("abc", 0));
    try std.testing.expectEqual(@as(usize, 1), byteOffsetToUtf16Index("abc", 1));
    try std.testing.expectEqual(@as(usize, 3), byteOffsetToUtf16Index("abc", 3));
    // 'café': byte 0→0, byte 1→1, byte 2→2, byte 3→3, byte 5→4
    try std.testing.expectEqual(@as(usize, 3), byteOffsetToUtf16Index("café", 3));
    try std.testing.expectEqual(@as(usize, 4), byteOffsetToUtf16Index("café", 5));
}

// ── Fixed method tests with multi-byte chars ──

test "charAt UTF-8" {
    const alloc = std.testing.allocator;
    // 'café': charAt(3) should return "é"
    const result = try charAt(alloc, "café", 3);
    defer alloc.free(result);
    try std.testing.expectEqualStrings("é", result);
}

test "charAt out of bounds" {
    const result = try charAt(std.testing.allocator, "abc", 10);
    try std.testing.expectEqualStrings("", result);
}

test "at UTF-8 positive" {
    const alloc = std.testing.allocator;
    const result = try at(alloc, "café", 3);
    defer alloc.free(result);
    try std.testing.expectEqualStrings("é", result);
}

test "at UTF-8 negative" {
    const alloc = std.testing.allocator;
    // "café" has 4 UTF-16 code units, at(-1) should return "é"
    const result = try at(alloc, "café", -1);
    defer alloc.free(result);
    try std.testing.expectEqualStrings("é", result);
}

test "slice UTF-8" {
    // "café": slice(1,3) should return "af"
    try std.testing.expectEqualStrings("af", slice("café", 1, 3));
    // "café": slice(3) should return "é"
    try std.testing.expectEqualStrings("é", slice("café", 3, std.math.maxInt(i64)));
}

test "substring UTF-8" {
    // "café": substring(1,3) should return "af"
    try std.testing.expectEqualStrings("af", substring("café", 1, 3));
}

test "indexOf UTF-8" {
    // "café": indexOf("é") should return 3 (UTF-16 index)
    try std.testing.expectEqual(@as(i64, 3), indexOf("café", "é", 0));
}

test "lastIndexOf UTF-8" {
    // "caféé": lastIndexOf("é") should return 4 (second é is UTF-16 index 4)
    try std.testing.expectEqual(@as(i64, 4), lastIndexOf("caféé", "é", std.math.maxInt(i64)));
}

test "indexOf UTF-8 from_index (R8-P1-19)" {
    // "caféé" UTF-16 indices: c=0,a=1,f=2,é=3,é=4 (len=5)
    // from_index=4 should find the second é at index 4
    try std.testing.expectEqual(@as(i64, 4), indexOf("caféé", "é", 4));
    // from_index=0 should find the first é at index 3
    try std.testing.expectEqual(@as(i64, 3), indexOf("caféé", "é", 0));
    // from_index beyond len → -1
    try std.testing.expectEqual(@as(i64, -1), indexOf("caféé", "é", 5));
}

test "lastIndexOf from_index (R8-P1-19)" {
    // "hello hello": UTF-16 len=11, lastIndexOf("hello", from_index)
    // from_index=5 → start=min(5,11)=5, scan backward from index 5
    //   finds "hello" at index 0
    try std.testing.expectEqual(@as(i64, 0), lastIndexOf("hello hello", "hello", 5));
    // from_index=11 (>= len) → start=11, but needle non-empty so scan from 10
    //   actually start=min(11,11)=11, scan backward from 11 meaning compare at
    //   position 11-5=6..11 → "hello" at index 6
    try std.testing.expectEqual(@as(i64, 6), lastIndexOf("hello hello", "hello", 11));
    // from_index=10 → start=10, scan backward, finds index 6
    try std.testing.expectEqual(@as(i64, 6), lastIndexOf("hello hello", "hello", 10));
    // from_index=5 should find the first "hello" at 0 (second one starts at 6)
    try std.testing.expectEqual(@as(i64, 0), lastIndexOf("hello hello", "hello", 5));
    // Negative from_index: len + from_index
    // len=11, from_index=-1 → start=10, finds index 6
    try std.testing.expectEqual(@as(i64, 6), lastIndexOf("hello hello", "hello", -1));
    // from_index very negative → start < 0 → -1
    try std.testing.expectEqual(@as(i64, -1), lastIndexOf("hello hello", "hello", -100));
    // Empty needle returns start (clamped)
    try std.testing.expectEqual(@as(i64, 0), lastIndexOf("abc", "", 0));
    try std.testing.expectEqual(@as(i64, 3), lastIndexOf("abc", "", 100));
}

test "padStart UTF-8" {
    const alloc = std.testing.allocator;
    // "café" has 4 UTF-16 code units; padStart(6, " ") should add 2 spaces
    const result = try padStart(alloc, "café", 6, " ");
    defer alloc.free(result);
    try std.testing.expectEqualStrings("  café", result);
}

test "padEnd UTF-8" {
    const alloc = std.testing.allocator;
    // "café" has 4 UTF-16 code units; padEnd(6, " ") should add 2 spaces
    const result = try padEnd(alloc, "café", 6, " ");
    defer alloc.free(result);
    try std.testing.expectEqualStrings("café  ", result);
}

test "padStart" {
    const result = try padStart(std.testing.allocator, "42", 5, "0");
    defer std.testing.allocator.free(result);
    try std.testing.expectEqualStrings("00042", result);
}

test "padStart no-op" {
    const result = try padStart(std.testing.allocator, "hello", 3, ".");
    defer std.testing.allocator.free(result);
    try std.testing.expectEqualStrings("hello", result);
}

test "padEnd" {
    const result = try padEnd(std.testing.allocator, "hello", 10, ".");
    defer std.testing.allocator.free(result);
    try std.testing.expectEqualStrings("hello.....", result);
}

test "padEnd no-op" {
    const result = try padEnd(std.testing.allocator, "abc", 3, ".");
    defer std.testing.allocator.free(result);
    try std.testing.expectEqualStrings("abc", result);
}

/// Replace all occurrences of old with new. Returns newly allocated string.
/// P1-24: Now expands $$ $& $` $' patterns in the replacement string per
/// ECMA-262 Table 52 for each match. Previously used std.mem.replaceOwned
/// which performed zero pattern processing — identical to a literal paste.
pub fn replaceAll(alloc: Allocator, s: []const u8, old: []const u8, new: []const u8) ![]const u8 {
    if (old.len == 0) {
        // Empty search string: insert replacement at every position
        // (before each char and at the end), per JS spec.
        // "abc".replaceAll("", "X") → "XaXbXcX"
        var result = std.ArrayList(u8).empty;
        defer result.deinit(alloc);
        var pos: usize = 0;
        while (pos <= s.len) : (pos += 1) {
            const expanded = try expandReplacementPlain(alloc, new, "", s[0..pos], s[pos..]);
            defer alloc.free(expanded);
            try result.appendSlice(alloc, expanded);
            if (pos < s.len) {
                try result.appendSlice(alloc, s[pos .. pos + 1]);
            }
        }
        return result.toOwnedSlice(alloc);
    }

    var result = std.ArrayList(u8).empty;
    defer result.deinit(alloc);

    var last_end: usize = 0;
    while (std.mem.indexOf(u8, s[last_end..], old)) |rel_idx| {
        const pos = last_end + rel_idx;
        const matched = s[pos .. pos + old.len];
        const before = s[0..pos];
        const after = s[pos + old.len ..];
        const expanded = try expandReplacementPlain(alloc, new, matched, before, after);
        defer alloc.free(expanded);
        try result.appendSlice(alloc, s[last_end..pos]);
        try result.appendSlice(alloc, expanded);
        last_end = pos + old.len;
    }
    try result.appendSlice(alloc, s[last_end..]);
    return result.toOwnedSlice(alloc);
}

/// Create string from character code(s). Takes UTF-16 code units and returns a UTF-8 string.
///
/// R8-P1-17: Each `code` is first converted via ToUint16 (ECMA-262 7.1.15),
/// i.e. `code mod 2^16` — so `String.fromCharCode(0x10000)` === "\u0000" and
/// `String.fromCharCode(-1)` === "\uFFFF". Previously this clamped with
/// `@max(0, @min(code, 0xFFFF))`, which incorrectly produced "\uFFFF" for
/// 0x10000 (instead of "\u0000") and "\u0000" for -1 (instead of "\uFFFF").
/// The cast `@as(u16, @truncate(@as(u64, @bitCast(code))))` yields the low
/// 16 bits of the i64, which is exactly `code mod 2^16` under two's complement.
pub fn fromCharCode(alloc: Allocator, codes: []const i64) ![]const u8 {
    // Calculate required buffer size
    var buf_size: usize = 0;
    for (codes) |code| {
        const c: u32 = @as(u32, @as(u16, @truncate(@as(u64, @bitCast(code)))));
        if (c <= 0x7F) {
            buf_size += 1;
        } else if (c <= 0x7FF) {
            buf_size += 2;
        } else {
            buf_size += 3;
        }
    }

    const result = try alloc.alloc(u8, buf_size);
    var pos: usize = 0;
    for (codes) |code| {
        const c: u32 = @as(u32, @as(u16, @truncate(@as(u64, @bitCast(code)))));
        if (c <= 0x7F) {
            result[pos] = @intCast(c);
            pos += 1;
        } else if (c <= 0x7FF) {
            result[pos] = @intCast(0xC0 | (c >> 6));
            result[pos + 1] = @intCast(0x80 | (c & 0x3F));
            pos += 2;
        } else {
            result[pos] = @intCast(0xE0 | (c >> 12));
            result[pos + 1] = @intCast(0x80 | ((c >> 6) & 0x3F));
            result[pos + 2] = @intCast(0x80 | (c & 0x3F));
            pos += 3;
        }
    }
    return result;
}

/// Create string from Unicode code point(s). Takes Unicode code points (U+0000
/// to U+10FFFF inclusive) and returns a UTF-8 string.
///
/// R8-P1-18: ECMA-262 `String.fromCodePoint` throws a `RangeError` for any
/// code point that is not in `0..=0x10FFFF`. Previously out-of-range or
/// negative inputs were silently coerced via `@max(0, cp)` and encoded as
/// garbage UTF-8. We now `return error.RangeError` for invalid inputs; the
/// emitter routes `error.RangeError` to `error.JsThrow` (JS-thrown TypeError)
/// so a JS try/catch can observe it.
pub fn fromCodePoint(alloc: Allocator, code_points: []const i64) ![]const u8 {
    // Validate + calculate required buffer size
    var buf_size: usize = 0;
    for (code_points) |cp| {
        if (cp < 0 or cp > 0x10FFFF) return error.RangeError;
        const c: u32 = @intCast(cp);
        if (c <= 0x7F) {
            buf_size += 1;
        } else if (c <= 0x7FF) {
            buf_size += 2;
        } else if (c <= 0xFFFF) {
            buf_size += 3;
        } else {
            buf_size += 4;
        }
    }

    const result = try alloc.alloc(u8, buf_size);
    var pos: usize = 0;
    for (code_points) |cp| {
        // Range check above guarantees cp is in 0..=0x10FFFF.
        const c: u32 = @intCast(cp);
        if (c <= 0x7F) {
            result[pos] = @intCast(c);
            pos += 1;
        } else if (c <= 0x7FF) {
            result[pos] = @intCast(0xC0 | (c >> 6));
            result[pos + 1] = @intCast(0x80 | (c & 0x3F));
            pos += 2;
        } else if (c <= 0xFFFF) {
            result[pos] = @intCast(0xE0 | (c >> 12));
            result[pos + 1] = @intCast(0x80 | ((c >> 6) & 0x3F));
            result[pos + 2] = @intCast(0x80 | (c & 0x3F));
            pos += 3;
        } else {
            result[pos] = @intCast(0xF0 | (c >> 18));
            result[pos + 1] = @intCast(0x80 | ((c >> 12) & 0x3F));
            result[pos + 2] = @intCast(0x80 | ((c >> 6) & 0x3F));
            result[pos + 3] = @intCast(0x80 | (c & 0x3F));
            pos += 4;
        }
    }
    return result;
}

test "replaceAll" {
    const result = try replaceAll(std.testing.allocator, "hello world world", "world", "zig");
    defer std.testing.allocator.free(result);
    try std.testing.expectEqualStrings("hello zig zig", result);
}

test "fromCharCode: ToUint16 wrapping (R8-P1-17)" {
    const a = std.testing.allocator;
    // In-range values unchanged.
    {
        const r = try fromCharCode(a, &[_]i64{ 65, 66, 67 });
        defer a.free(r);
        try std.testing.expectEqualStrings("ABC", r);
    }
    // 0x10000 wraps to 0x0000 per ToUint16 (mod 2^16).
    {
        const r = try fromCharCode(a, &[_]i64{0x10000});
        defer a.free(r);
        try std.testing.expectEqualStrings("\u{0000}", r);
    }
    // -1 wraps to 0xFFFF per ToUint16 (two's complement low 16 bits).
    {
        const r = try fromCharCode(a, &[_]i64{-1});
        defer a.free(r);
        try std.testing.expectEqualStrings("\u{FFFF}", r);
    }
    // 0x10041 wraps to 0x0041 == 'A'.
    {
        const r = try fromCharCode(a, &[_]i64{0x10041});
        defer a.free(r);
        try std.testing.expectEqualStrings("A", r);
    }
}

test "fromCodePoint: range validation (R8-P1-18)" {
    const a = std.testing.allocator;
    // Valid code points still encode correctly.
    {
        const r = try fromCodePoint(a, &[_]i64{ 0x41, 0x1F600 });
        defer a.free(r);
        try std.testing.expectEqualStrings("A\u{1F600}", r);
    }
    // Negative → error.RangeError.
    try std.testing.expectError(error.RangeError, fromCodePoint(a, &[_]i64{-1}));
    // Above 0x10FFFF → error.RangeError.
    try std.testing.expectError(error.RangeError, fromCodePoint(a, &[_]i64{0x110000}));
    // Boundary: 0x10FFFF is valid.
    {
        const r = try fromCodePoint(a, &[_]i64{0x10FFFF});
        defer a.free(r);
        try std.testing.expectEqualStrings("\u{10FFFF}", r);
    }
    // Boundary: 0 is valid.
    {
        const r = try fromCodePoint(a, &[_]i64{0});
        defer a.free(r);
        try std.testing.expectEqualStrings("\u{0000}", r);
    }
}

test "fromCharCode" {
    const result = try fromCharCode(std.testing.allocator, &[_]i64{ 72, 101, 108, 108, 111 });
    defer std.testing.allocator.free(result);
    try std.testing.expectEqualStrings("Hello", result);
}

test "fromCodePoint" {
    const result = try fromCodePoint(std.testing.allocator, &[_]i64{ 72, 101, 108, 108, 111 });
    defer std.testing.allocator.free(result);
    try std.testing.expectEqualStrings("Hello", result);
}

// ── P1-24: replace / replaceAll $ pattern tests ──

test "replace dollar-dollar" {
    const a = std.testing.allocator;
    const r = try replace(a, "hello", "l", "$$");
    defer a.free(r);
    try std.testing.expectEqualStrings("he$lo", r);
}

test "replace dollar-ampersand" {
    const a = std.testing.allocator;
    const r = try replace(a, "hello", "l", "[$&]");
    defer a.free(r);
    try std.testing.expectEqualStrings("he[l]lo", r);
}

test "replace dollar-backtick" {
    const a = std.testing.allocator;
    const r = try replace(a, "hello", "l", "[$`]");
    defer a.free(r);
    try std.testing.expectEqualStrings("he[he]lo", r);
}

test "replace dollar-quote" {
    const a = std.testing.allocator;
    const r = try replace(a, "hello", "l", "[$']");
    defer a.free(r);
    try std.testing.expectEqualStrings("he[lo]lo", r);
}

test "replace dollar-n literal (no capture groups)" {
    const a = std.testing.allocator;
    const r = try replace(a, "hello", "l", "$1");
    defer a.free(r);
    try std.testing.expectEqualStrings("he$1lo", r);
}

test "replace dollar-name literal (no named captures)" {
    const a = std.testing.allocator;
    const r = try replace(a, "hello", "l", "$<name>");
    defer a.free(r);
    try std.testing.expectEqualStrings("he$<name>lo", r);
}

test "replace unknown dollar escape" {
    const a = std.testing.allocator;
    const r = try replace(a, "hello", "l", "$X");
    defer a.free(r);
    try std.testing.expectEqualStrings("he$Xlo", r);
}

test "replace trailing dollar" {
    const a = std.testing.allocator;
    const r = try replace(a, "hello", "l", "end$");
    defer a.free(r);
    try std.testing.expectEqualStrings("heend$lo", r);
}

test "replace plain no dollar" {
    const a = std.testing.allocator;
    const r = try replace(a, "hello", "l", "L");
    defer a.free(r);
    try std.testing.expectEqualStrings("heLlo", r);
}

test "replace no match" {
    const a = std.testing.allocator;
    const r = try replace(a, "hello", "z", "$&");
    defer a.free(r);
    try std.testing.expectEqualStrings("hello", r);
}

test "replace empty search string" {
    const a = std.testing.allocator;
    const r = try replace(a, "abc", "", "X");
    defer a.free(r);
    try std.testing.expectEqualStrings("Xabc", r);
}

test "replace empty search with dollar-ampersand" {
    const a = std.testing.allocator;
    const r = try replace(a, "abc", "", "[$&]");
    defer a.free(r);
    try std.testing.expectEqualStrings("[]abc", r);
}

// ── replaceAll tests ──

test "replaceAll dollar-dollar" {
    const a = std.testing.allocator;
    const r = try replaceAll(a, "hello", "l", "$$");
    defer a.free(r);
    try std.testing.expectEqualStrings("he$$o", r);
}

test "replaceAll dollar-ampersand" {
    const a = std.testing.allocator;
    const r = try replaceAll(a, "hello", "l", "[$&]");
    defer a.free(r);
    try std.testing.expectEqualStrings("he[l][l]o", r);
}

test "replaceAll dollar-backtick" {
    const a = std.testing.allocator;
    const r = try replaceAll(a, "aXbXc", "X", "[$`]");
    defer a.free(r);
    try std.testing.expectEqualStrings("a[a]b[aXb]c", r);
}

test "replaceAll dollar-quote" {
    const a = std.testing.allocator;
    const r = try replaceAll(a, "aXbXc", "X", "[$']");
    defer a.free(r);
    // $' = full text after match in original string.
    // X at pos 1: $' = "bXc"; X at pos 3: $' = "c".
    try std.testing.expectEqualStrings("a[bXc]b[c]c", r);
}

test "replaceAll plain no dollar" {
    const a = std.testing.allocator;
    const r = try replaceAll(a, "hello", "l", "L");
    defer a.free(r);
    try std.testing.expectEqualStrings("heLLo", r);
}

test "replaceAll no match" {
    const a = std.testing.allocator;
    const r = try replaceAll(a, "hello", "z", "$&");
    defer a.free(r);
    try std.testing.expectEqualStrings("hello", r);
}

test "replaceAll empty search string" {
    const a = std.testing.allocator;
    const r = try replaceAll(a, "abc", "", "X");
    defer a.free(r);
    try std.testing.expectEqualStrings("XaXbXcX", r);
}

test "replaceAll empty search with dollar-ampersand" {
    const a = std.testing.allocator;
    const r = try replaceAll(a, "abc", "", "[$&]");
    defer a.free(r);
    try std.testing.expectEqualStrings("[]a[]b[]c[]", r);
}

test "replaceAll dollar-backtick empty search" {
    const a = std.testing.allocator;
    const r = try replaceAll(a, "ab", "", "[$`]");
    defer a.free(r);
    // pos=0: $` = "" → []; pos=1: $` = "a" → [a]; pos=2: $` = "ab" → [ab]
    try std.testing.expectEqualStrings("[]a[a]b[ab]", r);
}

// ── repeat overflow tests (RT-3: multiplication overflow check) ──

test "repeat: basic operation" {
    const a = std.testing.allocator;
    const r = try repeat(a, "abc", 3);
    defer a.free(r);
    try std.testing.expectEqualStrings("abcabcabc", r);
}

test "repeat: zero count returns empty string" {
    const a = std.testing.allocator;
    const r = try repeat(a, "abc", 0);
    defer a.free(r);
    try std.testing.expectEqualStrings("", r);
}

test "repeat: negative count returns empty string" {
    const a = std.testing.allocator;
    const r = try repeat(a, "abc", -5);
    defer a.free(r);
    try std.testing.expectEqualStrings("", r);
}

test "repeat: empty string returns empty string" {
    const a = std.testing.allocator;
    const r = try repeat(a, "", 1000);
    defer a.free(r);
    try std.testing.expectEqualStrings("", r);
}

test "repeat: overflow detection (s.len * count exceeds usize)" {
    const a = std.testing.allocator;
    // 3 * maxInt(i64) overflows usize on 64-bit
    const result = repeat(a, "abc", std.math.maxInt(i64));
    try std.testing.expectError(error.Overflow, result);
}
