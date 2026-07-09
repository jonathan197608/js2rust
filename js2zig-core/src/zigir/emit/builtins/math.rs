// zigir/emit/builtins/math.rs
// Math builtin method emission.

use crate::zigir::emit::helpers::EmitterHelpers;

use crate::zigir::emit::Emitter;

// ── Data-driven tables ──────────────────────────────────
// Direct Zig builtins: emit `@fn(args)`.
const ZIG_BUILTINS: &[&str] = &["abs", "floor", "ceil", "round", "sqrt", "trunc"];

// Float-wrap Zig builtins: emit `@fn(@as(f64, @floatFromInt(args)))`.
const ZIG_FLOAT_BUILTINS: &[&str] = &["sin", "cos", "tan", "atan", "log", "log10", "log2", "exp"];

// std.math direct calls: emit `std.math.fn(args)`.
const STD_MATH_DIRECT: &[&str] = &[
    "expm1", "sinh", "cosh", "tanh", "asinh", "acosh", "atanh", "log1p", "cbrt",
];

// Float-wrap std.math calls: emit `std.math.fn(@as(f64, @floatFromInt(args)))`.
const STD_MATH_FLOAT: &[&str] = &["asin", "acos"];

impl Emitter {
    pub(super) fn emit_math_builtin(&mut self, method: &str, args: &[crate::zigir::types::IrExpr]) {
        // ── Data-driven: direct Zig builtins ──
        if ZIG_BUILTINS.contains(&method) {
            self.write(&format!("@{}(", method));
            self.emit_inline_args(args);
            self.write(")");
            return;
        }

        // ── Data-driven: float-wrap Zig builtins ──
        if ZIG_FLOAT_BUILTINS.contains(&method) {
            self.write(&format!("@{}(@as(f64, @floatFromInt(", method));
            self.emit_inline_args(args);
            self.write(")))");
            return;
        }

        // ── Data-driven: std.math direct calls ──
        if STD_MATH_DIRECT.contains(&method) {
            self.write(&format!("std.math.{}(", method));
            self.emit_inline_args(args);
            self.write(")");
            return;
        }

        // ── Data-driven: float-wrap std.math calls ──
        if STD_MATH_FLOAT.contains(&method) {
            self.write(&format!("std.math.{}(@as(f64, @floatFromInt(", method));
            self.emit_inline_args(args);
            self.write(")))");
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
                    self.write("@abs(@as(f64, @floatFromInt(");
                    self.emit_expr(&args[0]);
                    self.write(")))");
                }
                _ => {
                    self.write("@sqrt(");
                    for (_i, arg) in args.iter().enumerate() {
                        if _i > 0 {
                            self.write(" + ");
                        }
                        self.write("@as(f64, @floatFromInt(");
                        self.emit_expr(arg);
                        self.write("))*@as(f64, @floatFromInt(");
                        self.emit_expr(arg);
                        self.write("))");
                    }
                    self.write(")");
                }
            },
            "fround" => {
                self.write("@as(f32, @floatFromInt(");
                self.emit_inline_args(args);
                self.write("))");
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
                self.write("@clz(");
                self.emit_inline_args(args);
                self.write(")");
            }
            "sign" => {
                self.write("js_math.sign(");
                self.emit_inline_args(args);
                self.write(")");
            }
            // Global NaN constant → std.math.nan(f64)
            "nan_f64" => {
                self.write("std.math.nan(f64)");
            }
            // Global Infinity constant → std.math.inf(f64)
            "inf_f64" => {
                self.write("std.math.inf(f64)");
            }
            // random, sign, etc: fall through to js_math module
            _ => {
                self.emit_module_call("js_math", method, args);
            }
        }
    }

    /// Unified min/max block expansion.
    /// min: (blk: { var __min = @as(i64, a); if (@as(i64, b) < __min) __min = @as(i64, b); break :blk __min; })
    /// max: same with > and __max
    fn emit_min_max(&mut self, method: &str, args: &[crate::zigir::types::IrExpr]) {
        let is_min = method == "min";
        let blk = self.next_label();
        let var = if is_min { "__min" } else { "__max" };
        let extreme = if is_min {
            "@as(i64, 9223372036854775807)"
        } else {
            "@as(i64, -9223372036854775808)"
        };
        let cmp_op = if is_min { "<" } else { ">" };

        match args.len() {
            0 => {
                self.write(extreme);
            }
            1 => {
                self.write("@as(i64, ");
                self.emit_expr(&args[0]);
                self.write(")");
            }
            _ => {
                self.write(&format!("({}: {{ var {} = @as(i64, ", blk, var));
                self.emit_expr(&args[0]);
                self.write("); ");
                for arg in &args[1..] {
                    let arg_str = self.render_expr_to_string(arg);
                    self.write(&format!(
                        "if (@as(i64, {}) {} {}) {} = @as(i64, {}); ",
                        arg_str, cmp_op, var, var, arg_str
                    ));
                }
                self.write(&format!(" break :{} {}; }})", blk, var));
            }
        }
    }
}
