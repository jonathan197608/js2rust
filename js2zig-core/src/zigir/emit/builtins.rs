// zigir/emit/builtins.rs
// Builtin method emission: routes BuiltinModule + method name to Zig code.
//
// This module handles `IrBuiltinCall` — calls to JS runtime library methods
// like Array.push, String.split, Math.floor, etc.
// Each BuiltinModule variant routes to a specialized emission function.

use crate::zigir::builtins::BuiltinModule;
use crate::zigir::emit::Emitter;
use crate::zigir::emit::helpers::EmitterHelpers;

// ═══════════════════════════════════════════════════════
//  Builtin call dispatch
// ═══════════════════════════════════════════════════════

impl Emitter {
    pub(crate) fn emit_builtin_call(&mut self, bc: &crate::zigir::types::IrBuiltinCall) {
        match bc.module {
            BuiltinModule::JsArray => self.emit_array_builtin(&bc.method, &bc.args),
            BuiltinModule::JsString => self.emit_string_builtin(&bc.method, &bc.args),
            BuiltinModule::JsDate => self.emit_date_builtin(&bc.method, &bc.args),
            BuiltinModule::JsJson => self.emit_json_builtin(&bc.method, &bc.args),
            BuiltinModule::JsObject => self.emit_object_builtin(&bc.method, &bc.args),
            BuiltinModule::JsNumber => self.emit_number_builtin(&bc.method, &bc.args),
            BuiltinModule::JsSymbol => self.emit_symbol_builtin(&bc.method, &bc.args),
            BuiltinModule::JsConsole => self.emit_console_builtin(&bc.method, &bc.args),
            BuiltinModule::JsMath => self.emit_math_builtin(&bc.method, &bc.args),
            BuiltinModule::JsRegExp => self.emit_regexp_builtin(&bc.method, &bc.args),
            BuiltinModule::JsTypedArray => self.emit_typedarray_builtin(&bc.method, &bc.args),
            BuiltinModule::JsUri => self.emit_uri_builtin(&bc.method, &bc.args),
            BuiltinModule::JsBigInt => self.emit_bigint_builtin(&bc.method, &bc.args),
            BuiltinModule::JsCollections => self.emit_collections_builtin(&bc.method, &bc.args),
            BuiltinModule::JsRuntime => self.emit_runtime_builtin(&bc.method, &bc.args),
        }
    }
}

// ═══════════════════════════════════════════════════════
//  Per-module builtin emitters
// ═══════════════════════════════════════════════════════

macro_rules! _builtin_stub {
    ($self:ident, $module:expr, $method:expr) => {
        $self.write(&format!("js_{}.{}(", $module, $method));
    };
}

impl Emitter {
    fn emit_array_builtin(&mut self, method: &str, args: &[crate::zigir::types::IrExpr]) {
        self.write(&format!("js_array.{}(", method));
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            self.emit_expr(arg);
        }
        self.write(")");
    }

    fn emit_string_builtin(&mut self, method: &str, args: &[crate::zigir::types::IrExpr]) {
        self.write(&format!("js_string.{}(", method));
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            self.emit_expr(arg);
        }
        self.write(")");
    }

    fn emit_date_builtin(&mut self, method: &str, args: &[crate::zigir::types::IrExpr]) {
        self.write(&format!("js_date.{}(", method));
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            self.emit_expr(arg);
        }
        self.write(")");
    }

    fn emit_json_builtin(&mut self, method: &str, args: &[crate::zigir::types::IrExpr]) {
        match method {
            "parse" => {
                // JSON.parse(text, reviver?) → try js_json.parse(js_allocator.allocator(), text, reviver) catch @panic("JSON.parse error")
                self.write("try js_json.parse(js_allocator.allocator(), ");
                if let Some(first_arg) = args.first() {
                    self.emit_expr(first_arg);
                } else {
                    self.write("\"\"");
                }
                // Pass reviver (default null)
                if args.len() >= 2 {
                    self.write(", ");
                    self.emit_expr(&args[1]);
                } else {
                    self.write(", null");
                }
                self.write(") catch @panic(\"JSON.parse error\")");
            }
            "stringify" => {
                // JSON.stringify(value, replacer?, space?) → js_json.stringify(...)
                self.write("js_json.stringify(");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.emit_expr(arg);
                }
                self.write(")");
            }
            _ => {
                self.write(&format!("js_json.{}(", method));
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.emit_expr(arg);
                }
                self.write(")");
            }
        }
    }

    fn emit_object_builtin(&mut self, method: &str, args: &[crate::zigir::types::IrExpr]) {
        self.write(&format!("js_object.{}(", method));
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            self.emit_expr(arg);
        }
        self.write(")");
    }

    fn emit_number_builtin(&mut self, method: &str, args: &[crate::zigir::types::IrExpr]) {
        self.write(&format!("js_number.{}(", method));
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            self.emit_expr(arg);
        }
        self.write(")");
    }

    fn emit_symbol_builtin(&mut self, method: &str, args: &[crate::zigir::types::IrExpr]) {
        self.write(&format!("js_symbol.{}(", method));
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            self.emit_expr(arg);
        }
        self.write(")");
    }

    fn emit_console_builtin(&mut self, method: &str, args: &[crate::zigir::types::IrExpr]) {
        self.write(&format!("js_console.{}(", method));
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            self.emit_expr(arg);
        }
        self.write(")");
    }

    fn emit_math_builtin(&mut self, method: &str, args: &[crate::zigir::types::IrExpr]) {
        // Many Math methods map to Zig builtin functions (@sqrt, @floor, etc.)
        // rather than std.math.*(). This mirrors Codegen's tables.rs mapping.
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
            // pow: @pow(a, b)
            "pow" => {
                self.write("@pow(");
                self.emit_inline_args(args);
                self.write(")");
            }
            // min/max — use Codegen's blk expansion pattern
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
            // random, sign, etc: fall through to js_math module
            _ => {
                self.write(&format!("js_math.{}(", method));
                self.emit_inline_args(args);
                self.write(")");
            }
        }
    }

    /// Emit args as comma-separated list WITHOUT wrapping parentheses.
    /// Used where we need to place args inside an existing function call like @sqrt(...).
    fn emit_inline_args(&mut self, args: &[crate::zigir::types::IrExpr]) {
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            self.emit_expr(arg);
        }
    }

    fn emit_regexp_builtin(&mut self, method: &str, args: &[crate::zigir::types::IrExpr]) {
        self.write(&format!("js_regexp.{}(", method));
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            self.emit_expr(arg);
        }
        self.write(")");
    }

    fn emit_typedarray_builtin(&mut self, method: &str, args: &[crate::zigir::types::IrExpr]) {
        self.write(&format!("js_typedarray.{}(", method));
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            self.emit_expr(arg);
        }
        self.write(")");
    }

    fn emit_uri_builtin(&mut self, method: &str, args: &[crate::zigir::types::IrExpr]) {
        self.write(&format!("js_uri.{}(", method));
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            self.emit_expr(arg);
        }
        self.write(")");
    }

    fn emit_bigint_builtin(&mut self, method: &str, args: &[crate::zigir::types::IrExpr]) {
        self.write(&format!("js_bigint.{}(", method));
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            self.emit_expr(arg);
        }
        self.write(")");
    }

    fn emit_collections_builtin(&mut self, method: &str, args: &[crate::zigir::types::IrExpr]) {
        self.write(&format!("js_collections.{}(", method));
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            self.emit_expr(arg);
        }
        self.write(")");
    }

    fn emit_runtime_builtin(&mut self, method: &str, args: &[crate::zigir::types::IrExpr]) {
        // js_runtime helper methods like jsTypeof()
        self.write(&format!("js_runtime.{}(", method));
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            self.emit_expr(arg);
        }
        self.write(")");
    }
}
