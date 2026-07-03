// codegen/tables.rs
// Pure lookup tables for builtin dispatch — no Codegen state required.

use crate::native_builtins::BuiltinCall;

// ── Math 1-arg descriptors ─────────────────────────────

/// Descriptor for a simple 1-arg Math builtin mapped to a Zig call.
pub(crate) struct MathOneArgDesc {
    /// JS method name for error messages (e.g. "abs").
    pub(crate) name: &'static str,
    /// Zig format string with `{arg}` placeholder.
    pub(crate) format: &'static str,
}

pub(crate) fn math_one_arg_desc(b: &BuiltinCall) -> Option<MathOneArgDesc> {
    match b {
        // Direct Zig builtins
        BuiltinCall::MathAbs => Some(MathOneArgDesc {
            name: "abs",
            format: "@abs({arg})",
        }),
        BuiltinCall::MathFloor => Some(MathOneArgDesc {
            name: "floor",
            format: "@floor({arg})",
        }),
        BuiltinCall::MathCeil => Some(MathOneArgDesc {
            name: "ceil",
            format: "@ceil({arg})",
        }),
        BuiltinCall::MathRound => Some(MathOneArgDesc {
            name: "round",
            format: "@round({arg})",
        }),
        BuiltinCall::MathSqrt => Some(MathOneArgDesc {
            name: "sqrt",
            format: "@sqrt({arg})",
        }),
        // Trig: @fn(@as(f64, @floatFromInt(x)))
        BuiltinCall::MathSin => Some(MathOneArgDesc {
            name: "sin",
            format: "@sin(@as(f64, @floatFromInt({arg})))",
        }),
        BuiltinCall::MathCos => Some(MathOneArgDesc {
            name: "cos",
            format: "@cos(@as(f64, @floatFromInt({arg})))",
        }),
        BuiltinCall::MathTan => Some(MathOneArgDesc {
            name: "tan",
            format: "@tan(@as(f64, @floatFromInt({arg})))",
        }),
        BuiltinCall::MathAtan => Some(MathOneArgDesc {
            name: "atan",
            format: "@atan(@as(f64, @floatFromInt({arg})))",
        }),
        // Log: @fn(@as(f64, @floatFromInt(x)))
        BuiltinCall::MathLog => Some(MathOneArgDesc {
            name: "log",
            format: "@log(@as(f64, @floatFromInt({arg})))",
        }),
        BuiltinCall::MathLog10 => Some(MathOneArgDesc {
            name: "log10",
            format: "@log10(@as(f64, @floatFromInt({arg})))",
        }),
        BuiltinCall::MathLog2 => Some(MathOneArgDesc {
            name: "log2",
            format: "@log2(@as(f64, @floatFromInt({arg})))",
        }),
        BuiltinCall::MathExp => Some(MathOneArgDesc {
            name: "exp",
            format: "@exp(@as(f64, @floatFromInt({arg})))",
        }),
        // std.math with f64 wrapping
        BuiltinCall::MathAsin => Some(MathOneArgDesc {
            name: "asin",
            format: "std.math.asin(@as(f64, @floatFromInt({arg})))",
        }),
        BuiltinCall::MathAcos => Some(MathOneArgDesc {
            name: "acos",
            format: "std.math.acos(@as(f64, @floatFromInt({arg})))",
        }),
        // std.math without wrapping
        BuiltinCall::MathTrunc => Some(MathOneArgDesc {
            name: "trunc",
            format: "@trunc(@as(f64, @floatFromInt({arg})))",
        }),
        BuiltinCall::MathCbrt => Some(MathOneArgDesc {
            name: "cbrt",
            format: "std.math.cbrt({arg})",
        }),
        BuiltinCall::MathExpm1 => Some(MathOneArgDesc {
            name: "expm1",
            format: "std.math.expm1({arg})",
        }),
        BuiltinCall::MathSinh => Some(MathOneArgDesc {
            name: "sinh",
            format: "std.math.sinh({arg})",
        }),
        BuiltinCall::MathCosh => Some(MathOneArgDesc {
            name: "cosh",
            format: "std.math.cosh({arg})",
        }),
        BuiltinCall::MathTanh => Some(MathOneArgDesc {
            name: "tanh",
            format: "std.math.tanh({arg})",
        }),
        BuiltinCall::MathAsinh => Some(MathOneArgDesc {
            name: "asinh",
            format: "std.math.asinh({arg})",
        }),
        BuiltinCall::MathAcosh => Some(MathOneArgDesc {
            name: "acosh",
            format: "std.math.acosh({arg})",
        }),
        BuiltinCall::MathAtanh => Some(MathOneArgDesc {
            name: "atanh",
            format: "std.math.atanh({arg})",
        }),
        BuiltinCall::MathLog1p => Some(MathOneArgDesc {
            name: "log1p",
            format: "std.math.log1p({arg})",
        }),
        _ => None,
    }
}

// ── String runtime descriptors ─────────────────────────

/// Descriptor for simple String runtime forwarding calls.
pub(crate) struct StringRuntimeDesc {
    /// Zig function name (e.g. "trim", "toUpper").
    pub(crate) method: &'static str,
    /// Whether the call needs `js_allocator.allocator()` as first arg.
    pub(crate) needs_allocator: bool,
    /// Whether the Zig runtime function returns an error union (`!T`).
    /// If true, `try` is prepended to the call expression.
    pub(crate) is_fallible: bool,
    /// Minimum number of JS-level arguments required.
    pub(crate) min_args: usize,
    /// Maximum number of JS-level arguments accepted.
    pub(crate) max_args: usize,
    /// Default Zig expressions for optional argument slots beyond min_args.
    /// One entry per optional slot (e.g. min=1,max=2 → 1 entry for 2nd arg).
    /// Empty entries mean the slot is simply omitted when the arg is missing.
    pub(crate) opt_defaults: &'static [&'static str],
}

pub(crate) fn string_runtime_desc(b: &BuiltinCall) -> Option<StringRuntimeDesc> {
    match b {
        // ── No Allocator, 0 args, non-fallible ──
        BuiltinCall::StringTrim => Some(StringRuntimeDesc {
            method: "trim",
            needs_allocator: false,
            is_fallible: false,
            min_args: 0,
            max_args: 0,
            opt_defaults: &[],
        }),
        BuiltinCall::StringTrimStart => Some(StringRuntimeDesc {
            method: "trimStart",
            needs_allocator: false,
            is_fallible: false,
            min_args: 0,
            max_args: 0,
            opt_defaults: &[],
        }),
        BuiltinCall::StringTrimEnd => Some(StringRuntimeDesc {
            method: "trimEnd",
            needs_allocator: false,
            is_fallible: false,
            min_args: 0,
            max_args: 0,
            opt_defaults: &[],
        }),
        // ── No Allocator, 1 arg, non-fallible ──
        BuiltinCall::StringIndexOf => Some(StringRuntimeDesc {
            method: "indexOf",
            needs_allocator: false,
            is_fallible: false,
            min_args: 1,
            max_args: 1,
            opt_defaults: &[],
        }),
        BuiltinCall::StringIncludes => Some(StringRuntimeDesc {
            method: "includes",
            needs_allocator: false,
            is_fallible: false,
            min_args: 1,
            max_args: 1,
            opt_defaults: &[],
        }),
        BuiltinCall::StringStartsWith => Some(StringRuntimeDesc {
            method: "startsWith",
            needs_allocator: false,
            is_fallible: false,
            min_args: 1,
            max_args: 1,
            opt_defaults: &[],
        }),
        BuiltinCall::StringEndsWith => Some(StringRuntimeDesc {
            method: "endsWith",
            needs_allocator: false,
            is_fallible: false,
            min_args: 1,
            max_args: 1,
            opt_defaults: &[],
        }),
        BuiltinCall::StringLastIndexOf => Some(StringRuntimeDesc {
            method: "lastIndexOf",
            needs_allocator: false,
            is_fallible: false,
            min_args: 1,
            max_args: 1,
            opt_defaults: &[],
        }),
        BuiltinCall::StringCharCodeAt => Some(StringRuntimeDesc {
            method: "charCodeAt",
            needs_allocator: false,
            is_fallible: false,
            min_args: 1,
            max_args: 1,
            opt_defaults: &[],
        }),
        BuiltinCall::StringCodePointAt => Some(StringRuntimeDesc {
            method: "codePointAt",
            needs_allocator: false,
            is_fallible: false,
            min_args: 1,
            max_args: 1,
            opt_defaults: &[],
        }),
        // ── No Allocator, 1-2 args, non-fallible ──
        BuiltinCall::StringSlice => Some(StringRuntimeDesc {
            method: "slice",
            needs_allocator: false,
            is_fallible: false,
            min_args: 1,
            max_args: 2,
            opt_defaults: &["std.math.maxInt(i64)"],
        }),
        BuiltinCall::StringSubstring => Some(StringRuntimeDesc {
            method: "substring",
            needs_allocator: false,
            is_fallible: false,
            min_args: 1,
            max_args: 2,
            opt_defaults: &["std.math.maxInt(i64)"],
        }),
        // ── No Allocator, 0-1 arg, non-fallible ──
        BuiltinCall::StringLocaleCompare => Some(StringRuntimeDesc {
            method: "localeCompare",
            needs_allocator: false,
            is_fallible: false,
            min_args: 0,
            max_args: 1,
            opt_defaults: &[],
        }),
        // ── With Allocator, 0 args, fallible (returns ![]const u8) ──
        BuiltinCall::StringToUpperCase => Some(StringRuntimeDesc {
            method: "toUpper",
            needs_allocator: true,
            is_fallible: true,
            min_args: 0,
            max_args: 0,
            opt_defaults: &[],
        }),
        BuiltinCall::StringToLocaleUpperCase => Some(StringRuntimeDesc {
            method: "toLocaleUpper",
            needs_allocator: true,
            is_fallible: true,
            min_args: 0,
            max_args: 0,
            opt_defaults: &[],
        }),
        BuiltinCall::StringToLowerCase => Some(StringRuntimeDesc {
            method: "toLower",
            needs_allocator: true,
            is_fallible: true,
            min_args: 0,
            max_args: 0,
            opt_defaults: &[],
        }),
        BuiltinCall::StringToLocaleLowerCase => Some(StringRuntimeDesc {
            method: "toLocaleLower",
            needs_allocator: true,
            is_fallible: true,
            min_args: 0,
            max_args: 0,
            opt_defaults: &[],
        }),
        // ── With Allocator, 1 arg, fallible ──
        BuiltinCall::StringCharAt => Some(StringRuntimeDesc {
            method: "charAt",
            needs_allocator: true,
            is_fallible: true,
            min_args: 1,
            max_args: 1,
            opt_defaults: &[],
        }),
        BuiltinCall::StringAt => Some(StringRuntimeDesc {
            method: "at",
            needs_allocator: true,
            is_fallible: true,
            min_args: 1,
            max_args: 1,
            opt_defaults: &[],
        }),
        BuiltinCall::StringConcat => Some(StringRuntimeDesc {
            method: "concat",
            needs_allocator: true,
            is_fallible: true,
            min_args: 1,
            max_args: 1,
            opt_defaults: &[],
        }),
        BuiltinCall::StringRepeat => Some(StringRuntimeDesc {
            method: "repeat",
            needs_allocator: true,
            is_fallible: true,
            min_args: 1,
            max_args: 1,
            opt_defaults: &[],
        }),
        // ── With Allocator, 1 arg, fallible (returns ![][]const u8) ──
        BuiltinCall::StringSplit => Some(StringRuntimeDesc {
            method: "split",
            needs_allocator: true,
            is_fallible: true,
            min_args: 1,
            max_args: 1,
            opt_defaults: &[],
        }),
        // ── With Allocator, 2 args, fallible ──
        BuiltinCall::StringPadStart => Some(StringRuntimeDesc {
            method: "padStart",
            needs_allocator: true,
            is_fallible: true,
            min_args: 2,
            max_args: 2,
            opt_defaults: &[],
        }),
        BuiltinCall::StringPadEnd => Some(StringRuntimeDesc {
            method: "padEnd",
            needs_allocator: true,
            is_fallible: true,
            min_args: 2,
            max_args: 2,
            opt_defaults: &[],
        }),
        BuiltinCall::StringReplace => Some(StringRuntimeDesc {
            method: "replace",
            needs_allocator: true,
            is_fallible: true,
            min_args: 2,
            max_args: 2,
            opt_defaults: &[],
        }),
        BuiltinCall::StringReplaceAll => Some(StringRuntimeDesc {
            method: "replaceAll",
            needs_allocator: true,
            is_fallible: true,
            min_args: 2,
            max_args: 2,
            opt_defaults: &[],
        }),
        // ── With Allocator, 0-1 arg, fallible ──
        BuiltinCall::StringNormalize => Some(StringRuntimeDesc {
            method: "normalize",
            needs_allocator: true,
            is_fallible: true,
            min_args: 0,
            max_args: 1,
            opt_defaults: &["\"NFC\""],
        }),
        _ => None,
    }
}
