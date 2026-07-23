// zigir/emit/builtins/math.rs
// Math builtin method emission.

use crate::zigir::emit::helpers::EmitterHelpers;

use crate::zigir::emit::Emitter;

// ── Data-driven tables ──────────────────────────────────
// Direct Zig builtins: emit `@fn(args)`.
const ZIG_BUILTINS: &[&str] = &["abs", "floor", "ceil", "round", "sqrt", "trunc"];

// Float builtin Zig builtins: emit `@fn(emit_f64_coerced(arg))`.
// Coercion handles both int and float inputs via type inspection.
// NOTE: `@atan` is NOT a Zig builtin — single-arg atan uses `std.math.atan`
// (see STD_MATH_FLOAT). Only `@sin`/`@cos`/`@tan`/`@log`/`@log10`/`@log2`/`@exp`
// are real `@`-builtins in Zig 0.16.
const ZIG_FLOAT_BUILTINS: &[&str] = &["sin", "cos", "tan", "log", "log10", "log2", "exp"];

// std.math direct calls: emit `std.math.fn(args)`.
const STD_MATH_DIRECT: &[&str] = &[
    "expm1", "sinh", "cosh", "tanh", "asinh", "acosh", "atanh", "log1p", "cbrt",
];

// Float builtin std.math calls: emit `std.math.fn(emit_f64_coerced(arg))`.
// `atan` is here (not a `@`-builtin); `asin`/`acos` likewise use std.math.
const STD_MATH_FLOAT: &[&str] = &["asin", "acos", "atan"];

/// Determine whether an IrExpr likely produces an f64 value.
/// Used to decide between `@as(f64, expr)` (identity cast for floats)
/// and `@as(f64, @floatFromInt(expr))` (int→float conversion).
pub(super) fn expr_is_float(expr: &crate::zigir::types::IrExpr) -> bool {
    use crate::types::ZigType;
    use crate::zigir::ops::BinOp;
    use crate::zigir::types::IrExpr;
    match expr {
        IrExpr::FloatLiteral(_) => true,
        IrExpr::Binary {
            op,
            left_type,
            right_type,
            ..
        } => {
            // Division always produces f64 in JS semantics
            *op == BinOp::Div
                || left_type.as_ref() == Some(&ZigType::F64)
                || right_type.as_ref() == Some(&ZigType::F64)
        }
        IrExpr::Unary { operand_type, .. } => operand_type.as_ref() == Some(&ZigType::F64),
        IrExpr::BuiltinCall(bc) => bc.return_type == ZigType::F64,
        // TypedIdent with F64 type — the lowerer annotates known-type variables
        IrExpr::TypedIdent {
            ty: ZigType::F64, ..
        } => true,
        // DivExpr / RemExpr always produce f64 (JS `/` and `%` return float)
        IrExpr::DivExpr { .. } | IrExpr::RemExpr { .. } => true,
        // PowExpr produces f64 when result_type is None (not coerced to i64)
        IrExpr::PowExpr { result_type, .. } => result_type.is_none(),
        _ => false,
    }
}

impl Emitter {
    /// Emit an argument coerced to f64, handling both int and float inputs.
    /// - Float expressions/literals: `@as(f64, expr)` (identity/comptime cast)
    /// - Int literals: `@as(f64, literal)` (comptime coercion, no @floatFromInt)
    /// - JsAny expressions: `expr.asF64()` (preserves float payload)
    /// - Int variables/expressions: `@as(f64, @floatFromInt(expr))` (int→float)
    fn emit_f64_coerced(&mut self, arg: &crate::zigir::types::IrExpr) {
        // JsAny-typed expressions: use .asF64() to preserve float payload
        if let crate::zigir::types::IrExpr::TypedIdent {
            ty: crate::types::ZigType::JsAny,
            ..
        } = arg
        {
            self.emit_expr(arg);
            self.write(".asF64()");
            return;
        }
        if expr_is_float(arg) || matches!(arg, crate::zigir::types::IrExpr::IntLiteral(_)) {
            self.write("@as(f64, ");
            self.emit_expr(arg);
            self.write(")");
        } else {
            self.write("@as(f64, @floatFromInt(");
            self.emit_expr(arg);
            self.write("))");
        }
    }

    /// Render `emit_f64_coerced(arg)` to a String without writing to the main
    /// output buffer. Used by `emit_min_max` where each coerced arg text is
    /// needed twice (in the `if` condition and the assignment).
    fn render_f64_coerced_to_string(&mut self, arg: &crate::zigir::types::IrExpr) -> String {
        let saved_output = std::mem::take(&mut self.output);
        self.emit_f64_coerced(arg);
        let result = std::mem::take(&mut self.output);
        self.output = saved_output;
        result
    }

    pub(super) fn emit_math_builtin(&mut self, method: &str, args: &[crate::zigir::types::IrExpr]) {
        // ── Data-driven: direct Zig builtins ──
        if ZIG_BUILTINS.contains(&method) {
            self.write(&format!("@{}(", method));
            self.emit_inline_args(args);
            self.write(")");
            return;
        }

        // ── Data-driven: float builtin Zig builtins ──
        if ZIG_FLOAT_BUILTINS.contains(&method) {
            self.write(&format!("@{}(", method));
            if let Some(a) = args.first() {
                self.emit_f64_coerced(a);
            }
            self.write(")");
            return;
        }

        // ── Data-driven: std.math direct calls ──
        if STD_MATH_DIRECT.contains(&method) {
            self.write(&format!("std.math.{}(", method));
            self.emit_inline_args(args);
            self.write(")");
            return;
        }

        // ── Data-driven: float builtin std.math calls ──
        if STD_MATH_FLOAT.contains(&method) {
            self.write(&format!("std.math.{}(", method));
            if let Some(a) = args.first() {
                self.emit_f64_coerced(a);
            }
            self.write(")");
            return;
        }

        // ── Special-case methods ──
        match method {
            // atan2: std.math.atan2(f64, x, y)
            "atan2" => {
                self.write("std.math.atan2(f64, ");
                self.emit_inline_args(args);
                self.write(")");
            }
            // pow: std.math.pow(f64, a, b)
            "pow" => {
                self.write("std.math.pow(f64, ");
                self.emit_inline_args(args);
                self.write(")");
            }
            // min/max — unified block expansion pattern
            "min" | "max" => {
                self.emit_min_max(method, args);
            }
            // random: inline expression using std.crypto.random
            "random" => {
                self.write(
                    "(@as(f64, @floatFromInt(std.crypto.random.int(u32))) / @as(f64, 4294967295.0))",
                );
            }
            // hypot: inline @sqrt(a*a + b*b + ...) expression
            "hypot" => match args.len() {
                0 => {
                    self.write("0");
                }
                1 => {
                    self.write("@abs(");
                    self.emit_f64_coerced(&args[0]);
                    self.write(")");
                }
                _ => {
                    self.write("@sqrt(");
                    for (_i, arg) in args.iter().enumerate() {
                        if _i > 0 {
                            self.write(" + ");
                        }
                        self.emit_f64_coerced(arg);
                        self.write("*");
                        self.emit_f64_coerced(arg);
                    }
                    self.write(")");
                }
            },
            "fround" => {
                // Math.fround(x) → nearest f32 representation.
                // @floatFromInt only works for int; @floatCast for float→f32.
                if let Some(a) = args.first() {
                    if expr_is_float(a) {
                        self.write("@as(f32, @floatCast(");
                        self.emit_expr(a);
                        self.write("))");
                    } else if matches!(a, crate::zigir::types::IrExpr::IntLiteral(_)) {
                        self.write("@as(f32, ");
                        self.emit_expr(a);
                        self.write(")");
                    } else {
                        self.write("@as(f32, @floatFromInt(");
                        self.emit_expr(a);
                        self.write("))");
                    }
                } else {
                    self.write("@as(f32, 0)");
                }
            }
            "imul" => {
                // Math.imul(a, b) → @as(i32, @intCast(@as(u32, @bitCast(@as(i32, a))) *% @as(u32, @bitCast(@as(i32, b)))))
                self.write("@as(i32, @intCast(@as(u32, @bitCast(@as(i32, ");
                if let Some(a) = args.first() {
                    self.emit_expr(a);
                }
                self.write("))) *% @as(u32, @bitCast(@as(i32, ");
                if let Some(b) = args.get(1) {
                    self.emit_expr(b);
                }
                self.write("))))");
            }
            "clz32" => {
                // Math.clz32(x): convert x to Uint32, count leading zero bits.
                // @clz requires an integer; float args must be converted first.
                if let Some(a) = args.first() {
                    if expr_is_float(a) {
                        self.write("@clz(@as(u32, @intFromFloat(");
                        self.emit_expr(a);
                        self.write(")))");
                    } else if matches!(a, crate::zigir::types::IrExpr::IntLiteral(_)) {
                        self.write("@clz(@as(u32, ");
                        self.emit_expr(a);
                        self.write("))");
                    } else {
                        self.write("@clz(@as(u32, @intCast(");
                        self.emit_expr(a);
                        self.write(")))");
                    }
                } else {
                    self.write("@clz(@as(u32, 0))");
                }
            }
            "sign" => {
                // Math.sign(x) → block with cached value to avoid re-evaluation.
                // JS semantics: +1 if x>0, -1 if x<0, 0 if x==0, NaN otherwise.
                let blk = self.next_label();
                self.write(&format!("({}: {{ const __sign_v = ", blk));
                if let Some(a) = args.first() {
                    self.emit_f64_coerced(a);
                } else {
                    self.write("@as(f64, 0)");
                }
                self.write("; break :");
                self.write(&blk);
                self.write(" if (__sign_v > 0) @as(f64, 1.0) else if (__sign_v < 0) @as(f64, -1.0) else if (__sign_v == 0) @as(f64, 0.0) else std.math.nan(f64); })");
            }
            // Global NaN constant → std.math.nan(f64)
            "nan_f64" => {
                self.write("std.math.nan(f64)");
            }
            // Global Infinity constant → std.math.inf(f64)
            "inf_f64" => {
                self.write("std.math.inf(f64)");
            }
            // Unknown Math method: emit compile error instead of referencing non-existent js_math module
            _ => {
                self.write(&format!(
                    "@compileError(\"Math.{method} is not implemented\")"
                ));
            }
        }
    }

    /// Unified min/max block expansion.
    ///
    /// The inferred return type of `Math.max`/`Math.min` is `f64` (see
    /// `builtin_return_type` in `native_builtins.rs` and the third tuple element
    /// in `cabi::builtin_call_to_ir`). To match that, we emit a value of type
    /// `f64` when:
    /// - there are zero args (JS spec: `Math.max()` -> `-Infinity`,
    ///   `Math.min()` -> `+Infinity`); or
    /// - any arg is float-shaped — a float literal, a division result, an F64
    ///   unary, or an f64-returning BuiltinCall (detected via `expr_is_float`).
    ///   In this branch every arg is coerced to f64 via `emit_f64_coerced` so
    ///   mixed int/float args compile (Zig cannot compare/assign i64 vs f64).
    ///
    /// Otherwise -- the common case where every arg is i64-shaped -- we preserve
    /// the existing i64-typed block: it round-trips cleanly through the inferred
    /// `f64` return type at call sites via Zig's implicit int->float coercion on
    /// assignment, and the integer literal/`number`-JSDoc args are i64.
    fn emit_min_max(&mut self, method: &str, args: &[crate::zigir::types::IrExpr]) {
        let is_min = method == "min";
        let blk = self.next_label();
        let var = if is_min { "__min" } else { "__max" };
        let cmp_op = if is_min { "<" } else { ">" };

        // Any float-shaped arg ⇒ use f64 emit for all args (Zig cannot
        // `@as(i64, x)` a non-int value, and cannot compare i64 vs f64).
        let any_float = args.iter().any(expr_is_float);

        match args.len() {
            0 => {
                // JS spec: empty Math.max() → -Infinity; Math.min() → +Infinity.
                if is_min {
                    self.write("std.math.inf(f64)");
                } else {
                    self.write("-std.math.inf(f64)");
                }
            }
            _ if any_float => {
                self.write(&format!("({}: {{ var {} = ", blk, var));
                let first = self.render_f64_coerced_to_string(&args[0]);
                self.write(&first);
                self.write("; ");
                for (i, arg) in args[1..].iter().enumerate() {
                    // Cache each arg in a temp to avoid double-evaluation and
                    // duplicate label names when the expression contains blocks.
                    let tmp = format!("__{}_{}", var, i + 1);
                    let arg_str = self.render_f64_coerced_to_string(arg);
                    self.write(&format!("const {} = {}; ", tmp, arg_str));
                    self.write(&format!(
                        "if ({} {} {}) {} = {}; ",
                        tmp, cmp_op, var, var, tmp
                    ));
                }
                self.write(&format!(" break :{} {}; }})", blk, var));
            }
            _ => {
                self.write(&format!("({}: {{ var {} = @as(i64, ", blk, var));
                self.emit_expr(&args[0]);
                self.write("); ");
                for (i, arg) in args[1..].iter().enumerate() {
                    // Cache each arg in a temp to avoid double-evaluation and
                    // duplicate label names when the expression contains blocks.
                    let tmp = format!("__{}_{}", var, i + 1);
                    let arg_str = self.render_expr_to_string(arg);
                    self.write(&format!("const {} = @as(i64, {}); ", tmp, arg_str));
                    self.write(&format!(
                        "if ({} {} {}) {} = {}; ",
                        tmp, cmp_op, var, var, tmp
                    ));
                }
                self.write(&format!(" break :{} {}; }})", blk, var));
            }
        }
    }
}
