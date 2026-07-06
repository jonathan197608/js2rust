// zigir/emit/builtins/math.rs
// Math builtin method emission.

use crate::zigir::emit::helpers::EmitterHelpers;

use crate::zigir::emit::Emitter;

impl Emitter {
    pub(super) fn emit_math_builtin(&mut self, method: &str, args: &[crate::zigir::types::IrExpr]) {
        // Many Math methods map to Zig builtin functions (@sqrt, @floor, etc.)
        // rather than std.math.*().
        // NOTE: We emit args manually (without emit_args which adds parens).
        match method {
            // Direct Zig builtins
            "abs" => {
                self.write("@abs(");
                self.emit_inline_args(args);
                self.write(")");
            }
            "floor" => {
                self.write("@floor(");
                self.emit_inline_args(args);
                self.write(")");
            }
            "ceil" => {
                self.write("@ceil(");
                self.emit_inline_args(args);
                self.write(")");
            }
            "round" => {
                self.write("@round(");
                self.emit_inline_args(args);
                self.write(")");
            }
            "sqrt" => {
                self.write("@sqrt(");
                self.emit_inline_args(args);
                self.write(")");
            }
            // Trig: @fn(@as(f64, @floatFromInt(arg)))
            "sin" => {
                self.write("@sin(@as(f64, @floatFromInt(");
                self.emit_inline_args(args);
                self.write(")))");
            }
            "cos" => {
                self.write("@cos(@as(f64, @floatFromInt(");
                self.emit_inline_args(args);
                self.write(")))");
            }
            "tan" => {
                self.write("@tan(@as(f64, @floatFromInt(");
                self.emit_inline_args(args);
                self.write(")))");
            }
            "atan" => {
                self.write("@atan(@as(f64, @floatFromInt(");
                self.emit_inline_args(args);
                self.write(")))");
            }
            // Log
            "log" => {
                self.write("@log(@as(f64, @floatFromInt(");
                self.emit_inline_args(args);
                self.write(")))");
            }
            "log10" => {
                self.write("@log10(@as(f64, @floatFromInt(");
                self.emit_inline_args(args);
                self.write(")))");
            }
            "log2" => {
                self.write("@log2(@as(f64, @floatFromInt(");
                self.emit_inline_args(args);
                self.write(")))");
            }
            "exp" => {
                self.write("@exp(@as(f64, @floatFromInt(");
                self.emit_inline_args(args);
                self.write(")))");
            }
            "trunc" => {
                self.write("@trunc(");
                self.emit_inline_args(args);
                self.write(")");
            }
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
            // min/max — use blk expansion pattern
            "min" => {
                let blk = self.next_label();
                match args.len() {
                    0 => {
                        self.write("@as(i64, 9223372036854775807)");
                    }
                    1 => {
                        self.write("@as(i64, ");
                        self.emit_expr(&args[0]);
                        self.write(")");
                    }
                    _ => {
                        // (blk_N: { var __min = @as(i64, a); if (@as(i64, b) < __min) __min = @as(i64, b);  break :blk_N __min; })
                        self.write(&format!("({}: {{ var __min = @as(i64, ", blk));
                        self.emit_expr(&args[0]);
                        self.write("); ");
                        for arg in &args[1..] {
                            let arg_str = {
                                let saved = std::mem::take(self.output_mut());
                                self.emit_expr(arg);
                                let rendered = std::mem::take(self.output_mut());
                                *self.output_mut() = saved;
                                rendered
                            };
                            self.write(&format!(
                                "if (@as(i64, {}) < __min) __min = @as(i64, {}); ",
                                arg_str, arg_str
                            ));
                        }
                        self.write(&format!(" break :{} __min; }})", blk));
                    }
                }
            }
            "max" => {
                let blk = self.next_label();
                match args.len() {
                    0 => {
                        self.write("@as(i64, -9223372036854775808)");
                    }
                    1 => {
                        self.write("@as(i64, ");
                        self.emit_expr(&args[0]);
                        self.write(")");
                    }
                    _ => {
                        // (blk_N: { var __max = @as(i64, a); if (@as(i64, b) > __max) __max = @as(i64, b);  break :blk_N __max; })
                        self.write(&format!("({}: {{ var __max = @as(i64, ", blk));
                        self.emit_expr(&args[0]);
                        self.write("); ");
                        for arg in &args[1..] {
                            let arg_str = {
                                let saved = std::mem::take(self.output_mut());
                                self.emit_expr(arg);
                                let rendered = std::mem::take(self.output_mut());
                                *self.output_mut() = saved;
                                rendered
                            };
                            self.write(&format!(
                                "if (@as(i64, {}) > __max) __max = @as(i64, {}); ",
                                arg_str, arg_str
                            ));
                        }
                        self.write(&format!(" break :{} __max; }})", blk));
                    }
                }
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
                        // @as(f64, @floatFromInt(arg)) * @as(f64, @floatFromInt(arg))
                        self.write("@as(f64, @floatFromInt(");
                        self.emit_expr(arg);
                        self.write("))*@as(f64, @floatFromInt(");
                        self.emit_expr(arg);
                        self.write("))");
                    }
                    self.write(")");
                }
            },
            // std.math one-arg functions: expm1, sinh, cosh, tanh, asinh, acosh, atanh
            "expm1" => {
                self.write("std.math.expm1(");
                self.emit_inline_args(args);
                self.write(")");
            }
            "sinh" => {
                self.write("std.math.sinh(");
                self.emit_inline_args(args);
                self.write(")");
            }
            "cosh" => {
                self.write("std.math.cosh(");
                self.emit_inline_args(args);
                self.write(")");
            }
            "tanh" => {
                self.write("std.math.tanh(");
                self.emit_inline_args(args);
                self.write(")");
            }
            "asinh" => {
                self.write("std.math.asinh(");
                self.emit_inline_args(args);
                self.write(")");
            }
            "acosh" => {
                self.write("std.math.acosh(");
                self.emit_inline_args(args);
                self.write(")");
            }
            "atanh" => {
                self.write("std.math.atanh(");
                self.emit_inline_args(args);
                self.write(")");
            }
            "log1p" => {
                self.write("std.math.log1p(");
                self.emit_inline_args(args);
                self.write(")");
            }
            "asin" => {
                self.write("std.math.asin(@as(f64, @floatFromInt(");
                self.emit_inline_args(args);
                self.write(")))");
            }
            "acos" => {
                self.write("std.math.acos(@as(f64, @floatFromInt(");
                self.emit_inline_args(args);
                self.write(")))");
            }
            "cbrt" => {
                self.write("std.math.cbrt(");
                self.emit_inline_args(args);
                self.write(")");
            }
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
                self.write(&format!("js_math.{}(", method));
                self.emit_inline_args(args);
                self.write(")");
            }
        }
    }
}
