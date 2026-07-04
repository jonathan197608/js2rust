// zigir/emit/builtins.rs
// Builtin method emission: routes BuiltinModule + method name to Zig code.
//
// This module handles `IrBuiltinCall` — calls to JS runtime library methods
// like Array.push, String.split, Math.floor, etc.
// Each BuiltinModule variant routes to a specialized emission function.

use crate::types::ZigType;
use crate::zigir::builtins::BuiltinModule;
use crate::zigir::emit::Emitter;
use crate::zigir::emit::helpers::EmitterHelpers;

// ═══════════════════════════════════════════════════════
//  Builtin call dispatch
// ═══════════════════════════════════════════════════════

impl Emitter {
    pub(crate) fn emit_builtin_call(&mut self, bc: &crate::zigir::types::IrBuiltinCall) {
        let obj = bc.obj_name.as_deref();
        match bc.module {
            BuiltinModule::JsArray => self.emit_array_builtin(&bc.method, obj, &bc.args),
            BuiltinModule::JsString => self.emit_string_builtin(&bc.method, obj, &bc.args),
            BuiltinModule::JsDate => self.emit_date_builtin(&bc.method, &bc.args),
            BuiltinModule::JsJson => self.emit_json_builtin(&bc.method, &bc.args),
            BuiltinModule::JsObject => self.emit_object_builtin(&bc.method, &bc.args),
            BuiltinModule::JsNumber => self.emit_number_builtin(&bc.method, &bc.args),
            BuiltinModule::JsSymbol => self.emit_symbol_builtin(&bc.method, &bc.args),
            BuiltinModule::JsConsole => self.emit_console_builtin(&bc.method, &bc.args),
            BuiltinModule::JsMath => self.emit_math_builtin(&bc.method, &bc.args),
            BuiltinModule::JsRegExp => self.emit_regexp_builtin(&bc.method, &bc.args),
            BuiltinModule::JsTypedArray => self.emit_typedarray_builtin(&bc.method, &bc.args),
            BuiltinModule::JsUri => self.emit_uri_builtin(&bc.method, obj, &bc.args),
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
    fn emit_array_builtin(
        &mut self,
        method: &str,
        obj: Option<&str>,
        args: &[crate::zigir::types::IrExpr],
    ) {
        // Some array methods are direct ArrayList operations when we have the object name.
        match method {
            "pop" => {
                // arr.pop() — direct ArrayList method, not js_array.pop()
                if let Some(name) = obj {
                    self.write(&format!("{}.pop()", name));
                } else {
                    self.write(&format!("js_array.{}(", method));
                    self.emit_inline_args(args);
                    self.write(")");
                }
            }
            _ => {
                // Fallback: js_array.method(args)
                self.write(&format!("js_array.{}(", method));
                self.emit_inline_args(args);
                self.write(")");
            }
        }
    }

    fn emit_string_builtin(
        &mut self,
        method: &str,
        obj: Option<&str>,
        args: &[crate::zigir::types::IrExpr],
    ) {
        // Method dispatch: JS name → Zig runtime name + allocator + fallible.
        // Mirrors Codegen's StringRuntimeDesc table (tables.rs).
        let (zig_method, needs_allocator, is_fallible, min_args, max_args, opt_defaults): (
            &str,
            bool,
            bool,
            usize,
            usize,
            &[&str],
        ) = match method {
            // ── No allocator, 0 args, non-fallible ──
            "trim" => ("trim", false, false, 0, 0, &[]),
            "trimStart" => ("trimStart", false, false, 0, 0, &[]),
            "trimEnd" => ("trimEnd", false, false, 0, 0, &[]),
            // ── No allocator, 1 arg, non-fallible ──
            "indexOf" => ("indexOf", false, false, 1, 1, &[]),
            "includes" => ("includes", false, false, 1, 1, &[]),
            "startsWith" => ("startsWith", false, false, 1, 1, &[]),
            "endsWith" => ("endsWith", false, false, 1, 1, &[]),
            "lastIndexOf" => ("lastIndexOf", false, false, 1, 1, &[]),
            "charCodeAt" => ("charCodeAt", false, false, 1, 1, &[]),
            "codePointAt" => ("codePointAt", false, false, 1, 1, &[]),
            // ── No allocator, 1-2 args, non-fallible ──
            "slice" => ("slice", false, false, 1, 2, &["std.math.maxInt(i64)"]),
            "substring" => ("substring", false, false, 1, 2, &["std.math.maxInt(i64)"]),
            // ── No allocator, 0-1 arg, non-fallible ──
            "localeCompare" => ("localeCompare", false, false, 0, 1, &[]),
            // ── With allocator, 0 args, fallible ──
            "toUpperCase" => ("toUpper", true, true, 0, 0, &[]),
            "toLocaleUpperCase" => ("toLocaleUpper", true, true, 0, 0, &[]),
            "toLowerCase" => ("toLower", true, true, 0, 0, &[]),
            "toLocaleLowerCase" => ("toLocaleLower", true, true, 0, 0, &[]),
            // ── With allocator, 1 arg, fallible ──
            "charAt" => ("charAt", true, true, 1, 1, &[]),
            "at" => ("at", true, true, 1, 1, &[]),
            "concat" => ("concat", true, true, 1, 1, &[]),
            "repeat" => ("repeat", true, true, 1, 1, &[]),
            // ── With allocator, 1 arg, fallible (returns ![][]const u8) ──
            "split" => ("split", true, true, 1, 1, &[]),
            // ── With allocator, 2 args, fallible ──
            "padStart" => ("padStart", true, true, 2, 2, &[]),
            "padEnd" => ("padEnd", true, true, 2, 2, &[]),
            "replace" => ("replace", true, true, 2, 2, &[]),
            "replaceAll" => ("replaceAll", true, true, 2, 2, &[]),
            // ── With allocator, 0-1 arg, fallible ──
            "normalize" => ("normalize", true, true, 0, 1, &["\"NFC\""]),
            // ── Fallback ──
            _ => {
                // Unknown string method — naive emission
                self.write(&format!("js_string.{}(", method));
                self.emit_inline_args(args);
                self.write(")");
                return;
            }
        };

        // Emit: [try ]js_string.zig_method([js_allocator.allocator(), ]obj[, arg1[, arg2...]])
        if is_fallible {
            self.write("try ");
        }
        self.write(&format!("js_string.{}(", zig_method));
        if needs_allocator {
            self.write("js_allocator.allocator(), ");
        }
        // Receiver object
        if let Some(name) = obj {
            self.write(name);
        }
        // Arguments (fill to max_args with opt_defaults for missing slots)
        let n_args = args.len();
        let total_slots = max_args;
        for slot in 0..total_slots {
            if slot < n_args {
                self.write(", ");
                self.emit_expr(&args[slot]);
            } else {
                let opt_idx = slot - min_args;
                if let Some(default) = opt_defaults.get(opt_idx)
                    && !default.is_empty()
                {
                    self.write(&format!(", {}", default));
                }
            }
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
        match method {
            "toFixed" => {
                // js_number.toFixed(js_allocator.allocator(), obj, digits)
                self.write("js_number.toFixed(js_allocator.allocator(), ");
                self.emit_inline_args(args);
                self.write(")");
            }
            _ => {
                self.write(&format!("js_number.{}(", method));
                self.emit_inline_args(args);
                self.write(")");
            }
        }
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
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 {
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
            "imul" => {
                self.write("@mulWithOverflow(i32, ");
                self.emit_inline_args(args);
                self.write(")");
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

    fn emit_uri_builtin(
        &mut self,
        method: &str,
        _obj: Option<&str>,
        args: &[crate::zigir::types::IrExpr],
    ) {
        // URI methods: all need js_allocator.allocator() and specific catch patterns.
        // encodeURI/encodeURIComponent: catch @panic("OOM: ...")
        // decodeURI/decodeURIComponent: catch "" (outside try block)
        match method {
            "encodeURI" => {
                self.write("js_uri.encodeURI(js_allocator.allocator(), ");
                self.emit_inline_args(args);
                self.write(") catch @panic(\"OOM: encodeURI\")");
            }
            "encodeURIComponent" => {
                self.write("js_uri.encodeURIComponent(js_allocator.allocator(), ");
                self.emit_inline_args(args);
                self.write(") catch @panic(\"OOM: encodeURIComponent\")");
            }
            "decodeURI" => {
                self.write("js_uri.decodeURI(js_allocator.allocator(), ");
                self.emit_inline_args(args);
                self.write(") catch \"\"");
            }
            "decodeURIComponent" => {
                self.write("js_uri.decodeURIComponent(js_allocator.allocator(), ");
                self.emit_inline_args(args);
                self.write(") catch \"\"");
            }
            _ => {
                self.write(&format!("js_uri.{}(", method));
                self.emit_inline_args(args);
                self.write(")");
            }
        }
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

    // ═══════════════════════════════════════════════════════
    //  Array callback inlining
    // ═══════════════════════════════════════════════════════

    /// Emit an inlined array callback method (forEach, some, every, filter,
    /// find, findIndex, findLast, findLastIndex, map, reduce) as a Zig loop.
    ///
    /// This mirrors the Codegen's callback inlining patterns from
    /// `codegen/builtins.rs` lines 2318–2901, but operates on the IR
    /// `IrArrayCallbackInline` data instead of raw AST.
    pub(crate) fn emit_array_callback_inline(
        &mut self,
        data: &crate::zigir::types::IrArrayCallbackInline,
    ) {
        use crate::zigir::types::ArrayCallbackKind as K;

        match data.kind {
            K::ForEach => self.emit_for_each_inline(data),
            K::Some => self.emit_some_inline(data),
            K::Every => self.emit_every_inline(data),
            K::Filter => self.emit_filter_inline(data),
            K::Find => self.emit_find_inline(data),
            K::FindIndex => self.emit_find_index_inline(data),
            K::FindLast => self.emit_find_last_inline(data),
            K::FindLastIndex => self.emit_find_last_index_inline(data),
            K::Map => self.emit_map_inline(data),
            K::Reduce => self.emit_reduce_inline(data),
        }
    }

    // ── forEach ────────────────────────────────────────
    //
    //  for (obj.items) |elem| {
    //      <body stmts>
    //  }
    //
    fn emit_for_each_inline(&mut self, data: &crate::zigir::types::IrArrayCallbackInline) {
        self.write(&format!(
            "for ({}.items) |{}| ",
            data.obj_name, data.elem_param
        ));
        self.write("{\n");
        self.indent_push();
        for stmt in &data.body {
            self.writeln("");
            self.emit_stmt(stmt);
        }
        self.indent_pop();
        self.writeln("");
        self.write("}");
    }

    // ── some ───────────────────────────────────────────
    //
    //  (blk_N: {
    //      for (obj.items[, 0..]) |elem[, idx]| {
    //          if (<pred>) break :blk_N true;
    //      }
    //      break :blk_N false;
    //  })
    //
    fn emit_some_inline(&mut self, data: &crate::zigir::types::IrArrayCallbackInline) {
        let blk = self.next_label();
        self.write(&format!("({}: {{ ", blk));
        if data.has_idx_param {
            self.write(&format!(
                "for ({}.items, 0..) |{}, {}| ",
                data.obj_name, data.elem_param, data.idx_param
            ));
        } else {
            self.write(&format!(
                "for ({}.items) |{}| ",
                data.obj_name, data.elem_param
            ));
        }
        self.write("{\n");
        self.indent_push();
        for stmt in &data.body {
            self.writeln("");
            match stmt {
                crate::zigir::types::IrStmt::Return { value: Some(expr) } => {
                    self.write("if (");
                    self.emit_expr(expr);
                    self.write(&format!(") break :{} true;", blk));
                }
                crate::zigir::types::IrStmt::Expr(expr) => {
                    self.write("if (");
                    self.emit_expr(expr);
                    self.write(&format!(") break :{} true;", blk));
                }
                _ => self.emit_stmt(stmt),
            }
        }
        self.indent_pop();
        self.writeln("");
        self.write("}");
        self.write(&format!(" break :{} false; }})", blk));
    }

    // ── every ──────────────────────────────────────────
    //
    //  (blk_N: {
    //      for (obj.items[, 0..]) |elem[, idx]| {
    //          if (!(<pred>)) break :blk_N false;
    //      }
    //      break :blk_N true;
    //  })
    //
    fn emit_every_inline(&mut self, data: &crate::zigir::types::IrArrayCallbackInline) {
        let blk = self.next_label();
        self.write(&format!("({}: {{ ", blk));
        if data.has_idx_param {
            self.write(&format!(
                "for ({}.items, 0..) |{}, {}| ",
                data.obj_name, data.elem_param, data.idx_param
            ));
        } else {
            self.write(&format!(
                "for ({}.items) |{}| ",
                data.obj_name, data.elem_param
            ));
        }
        self.write("{\n");
        self.indent_push();
        for stmt in &data.body {
            self.writeln("");
            match stmt {
                crate::zigir::types::IrStmt::Return { value: Some(expr) } => {
                    self.write("if (!(");
                    self.emit_expr(expr);
                    self.write(&format!(")) break :{} false;", blk));
                }
                crate::zigir::types::IrStmt::Expr(expr) => {
                    self.write("if (!(");
                    self.emit_expr(expr);
                    self.write(&format!(")) break :{} false;", blk));
                }
                _ => self.emit_stmt(stmt),
            }
        }
        self.indent_pop();
        self.writeln("");
        self.write("}");
        self.write(&format!(" break :{} true; }})", blk));
    }

    // ── filter ─────────────────────────────────────────
    //
    //  (blk_N: {
    //      var __filter: std.ArrayList(elem_type) = .empty;
    //      for (obj.items) |elem| {
    //          if (<pred>) __filter.append(js_allocator.allocator(), elem) catch @panic("OOM: Array.filter append");
    //      }
    //      break :blk_N __filter;
    //  })
    //
    fn emit_filter_inline(&mut self, data: &crate::zigir::types::IrArrayCallbackInline) {
        let blk = self.next_label();
        let elem_type_str = data.elem_type.to_zig_type();
        self.write(&format!("({}: {{ ", blk));
        self.write(&format!(
            "var __filter: std.ArrayList({}) = .empty; ",
            elem_type_str
        ));
        self.write(&format!(
            "for ({}.items) |{}| ",
            data.obj_name, data.elem_param
        ));
        self.write("{\n");
        self.indent_push();
        for stmt in &data.body {
            self.writeln("");
            match stmt {
                crate::zigir::types::IrStmt::Return { value: Some(expr) } => {
                    self.write("if (");
                    self.emit_expr(expr);
                    self.write(&format!(
                        ") {{ __filter.append(js_allocator.allocator(), {}) catch @panic(\"OOM: Array.filter append\"); }}",
                        data.elem_param
                    ));
                }
                crate::zigir::types::IrStmt::Expr(expr) => {
                    self.write("if (");
                    self.emit_expr(expr);
                    self.write(&format!(
                        ") {{ __filter.append(js_allocator.allocator(), {}) catch @panic(\"OOM: Array.filter append\"); }}",
                        data.elem_param
                    ));
                }
                _ => self.emit_stmt(stmt),
            }
        }
        self.indent_pop();
        self.writeln("");
        self.write("}");
        self.write(&format!(" break :{} __filter; }})", blk));
    }

    // ── find ───────────────────────────────────────────
    //
    //  (blk_N: {
    //      for (obj.items) |elem| {
    //          if (<pred>) break :blk_N elem;
    //      }
    //      break :blk_N undefined;
    //  })
    //
    fn emit_find_inline(&mut self, data: &crate::zigir::types::IrArrayCallbackInline) {
        let blk = self.next_label();
        self.write(&format!("({}: {{ ", blk));
        self.write(&format!(
            "for ({}.items) |{}| ",
            data.obj_name, data.elem_param
        ));
        self.write("{\n");
        self.indent_push();
        for stmt in &data.body {
            self.writeln("");
            match stmt {
                crate::zigir::types::IrStmt::Return { value: Some(expr) } => {
                    self.write("if (");
                    self.emit_expr(expr);
                    self.write(&format!(") break :{} {};", blk, data.elem_param));
                }
                crate::zigir::types::IrStmt::Expr(expr) => {
                    self.write("if (");
                    self.emit_expr(expr);
                    self.write(&format!(") break :{} {};", blk, data.elem_param));
                }
                _ => self.emit_stmt(stmt),
            }
        }
        self.indent_pop();
        self.writeln("");
        self.write("}");
        self.write(&format!(" break :{} undefined; }})", blk));
    }

    // ── findIndex ──────────────────────────────────────
    //
    //  (blk_N: {
    //      for (obj.items, 0..) |elem, __i| {
    //          const __idx: i64 = @intCast(__i);
    //          if (<pred>) break :blk_N __idx;
    //      }
    //      break :blk_N -1;
    //  })
    //
    fn emit_find_index_inline(&mut self, data: &crate::zigir::types::IrArrayCallbackInline) {
        let blk = self.next_label();
        let index_name = format!("__{}_i", data.elem_param);
        let idx_name = format!("__{}_idx", data.elem_param);
        self.write(&format!("({}: {{ ", blk));
        self.write(&format!(
            "for ({}.items, 0..) |{}, {}| ",
            data.obj_name, data.elem_param, index_name
        ));
        self.write("{\n");
        self.indent_push();
        self.writeln(&format!(
            "const {}: i64 = @intCast({});",
            idx_name, index_name
        ));
        for stmt in &data.body {
            self.writeln("");
            match stmt {
                crate::zigir::types::IrStmt::Return { value: Some(expr) } => {
                    self.write("if (");
                    self.emit_expr(expr);
                    self.write(&format!(") break :{} {};", blk, idx_name));
                }
                crate::zigir::types::IrStmt::Expr(expr) => {
                    self.write("if (");
                    self.emit_expr(expr);
                    self.write(&format!(") break :{} {};", blk, idx_name));
                }
                _ => self.emit_stmt(stmt),
            }
        }
        self.indent_pop();
        self.writeln("");
        self.write("}");
        self.write(&format!(" break :{} -1; }})", blk));
    }

    // ── findLast ───────────────────────────────────────
    //
    //  (blk_N: {
    //      var __i: usize = obj.items.len;
    //      while (__i > 0) {
    //          __i -= 1;
    //          const elem = obj.items[__i];
    //          if (<pred>) break :blk_N elem;
    //      }
    //      break :blk_N undefined;
    //  })
    //
    fn emit_find_last_inline(&mut self, data: &crate::zigir::types::IrArrayCallbackInline) {
        let blk = self.next_label();
        self.write(&format!(
            "({}: {{ var __i: usize = {}.items.len; while (__i > 0) {{ __i -= 1; const {} = {}.items[__i]; ",
            blk, data.obj_name, data.elem_param, data.obj_name
        ));
        self.indent_push();
        for stmt in &data.body {
            self.writeln("");
            match stmt {
                crate::zigir::types::IrStmt::Return { value: Some(expr) } => {
                    self.write("if (");
                    self.emit_expr(expr);
                    self.write(&format!(") break :{} {};", blk, data.elem_param));
                }
                crate::zigir::types::IrStmt::Expr(expr) => {
                    self.write("if (");
                    self.emit_expr(expr);
                    self.write(&format!(") break :{} {};", blk, data.elem_param));
                }
                _ => self.emit_stmt(stmt),
            }
        }
        self.indent_pop();
        self.writeln("");
        self.write(&format!("}} break :{} undefined; }})", blk));
    }

    // ── findLastIndex ──────────────────────────────────
    //
    //  (blk_N: {
    //      var __i: usize = obj.items.len;
    //      while (__i > 0) {
    //          __i -= 1;
    //          const elem = obj.items[__i];
    //          const __idx: i64 = @intCast(__i);
    //          if (<pred>) break :blk_N __idx;
    //      }
    //      break :blk_N -1;
    //  })
    //
    fn emit_find_last_index_inline(&mut self, data: &crate::zigir::types::IrArrayCallbackInline) {
        let blk = self.next_label();
        let idx_name = format!("__{}_idx", data.elem_param);
        self.write(&format!(
            "({}: {{ var __i: usize = {}.items.len; while (__i > 0) {{ __i -= 1; const {} = {}.items[__i]; const {}: i64 = @intCast(__i); ",
            blk, data.obj_name, data.elem_param, data.obj_name, idx_name
        ));
        self.indent_push();
        for stmt in &data.body {
            self.writeln("");
            match stmt {
                crate::zigir::types::IrStmt::Return { value: Some(expr) } => {
                    self.write("if (");
                    self.emit_expr(expr);
                    self.write(&format!(") break :{} {};", blk, idx_name));
                }
                crate::zigir::types::IrStmt::Expr(expr) => {
                    self.write("if (");
                    self.emit_expr(expr);
                    self.write(&format!(") break :{} {};", blk, idx_name));
                }
                _ => self.emit_stmt(stmt),
            }
        }
        self.indent_pop();
        self.writeln("");
        self.write(&format!("}} break :{} -1; }})", blk));
    }

    // ── map (identity stub) ────────────────────────────
    //
    //  Codegen just returns the object name — map is not fully implemented.
    //
    fn emit_map_inline(&mut self, data: &crate::zigir::types::IrArrayCallbackInline) {
        self.write(&data.obj_name);
    }

    // ── reduce ─────────────────────────────────────────
    //
    //  (blk_N: {
    //      var acc: <type> = <init>;
    //      for (obj.items) |elem| {
    //          acc = <body_expr>;
    //      }
    //      break :blk_N acc;
    //  })
    //
    fn emit_reduce_inline(&mut self, data: &crate::zigir::types::IrArrayCallbackInline) {
        let blk = self.next_label();
        // Determine init value and accumulator type
        let init_expr_str = match &data.reduce_init {
            Some(expr) => {
                let saved = std::mem::take(self.output_mut());
                self.emit_expr(expr);
                let rendered = std::mem::take(self.output_mut());
                *self.output_mut() = saved;
                rendered
            }
            None => "0".to_string(),
        };
        let acc_type = if init_expr_str.contains('.') {
            "f64"
        } else {
            "i64"
        };
        self.write(&format!("({}: {{ ", blk));
        self.write(&format!("var acc: {} = {}; ", acc_type, init_expr_str));
        self.write(&format!(
            "for ({}.items) |{}| ",
            data.obj_name, data.elem_param
        ));
        self.write("{\n");
        self.indent_push();
        for stmt in &data.body {
            self.writeln("");
            match stmt {
                crate::zigir::types::IrStmt::Return { value: Some(expr) } => {
                    self.write("acc = ");
                    self.emit_expr(expr);
                    self.write(";");
                }
                crate::zigir::types::IrStmt::Expr(expr) => {
                    self.write("acc = ");
                    self.emit_expr(expr);
                    self.write(";");
                }
                _ => self.emit_stmt(stmt),
            }
        }
        self.indent_pop();
        self.writeln("");
        self.write("}");
        self.write(&format!(" break :{} acc; }})", blk));
    }

    // ═══════════════════════════════════════════════════════
    //  Array non-callback method inlining
    // ═══════════════════════════════════════════════════════

    /// Emit an inlined array non-callback method as a Zig block expression or
    /// statement. This mirrors the Codegen's inline patterns for includes,
    /// indexOf, lastIndexOf, join, slice, splice, at, concat, copyWithin, fill.
    pub(crate) fn emit_array_method_inline(
        &mut self,
        data: &crate::zigir::types::IrArrayMethodInline,
    ) {
        use crate::zigir::types::ArrayMethodKind as K;

        match data.kind {
            K::Includes => self.emit_includes_inline(data),
            K::IndexOf => self.emit_index_of_inline(data),
            K::LastIndexOf => self.emit_last_index_of_inline(data),
            K::Join => self.emit_join_inline(data),
            K::Slice => self.emit_slice_inline(data),
            K::Splice => self.emit_splice_inline(data),
            K::At => self.emit_at_inline(data),
            K::Concat => self.emit_concat_inline(data),
            K::CopyWithin => self.emit_copy_within_inline(data),
            K::Fill => self.emit_fill_inline(data),
        }
    }

    // ── includes ───────────────────────────────────────
    // For string arrays: (std.mem.indexOf(u8, obj, target) != null)
    // For i64 arrays: (blk: { for (obj.items) |item| { if (item == target) break :blk true; } break :blk false; })
    fn emit_includes_inline(&mut self, data: &crate::zigir::types::IrArrayMethodInline) {
        let blk = self.next_label();
        // If the array is a string type, use std.mem.indexOf
        if matches!(data.elem_type, ZigType::Str) {
            self.write("(std.mem.indexOf(u8, ");
            self.write(&data.obj_name);
            self.write(", ");
            if let Some(arg) = data.args.first() {
                self.emit_expr(arg);
            }
            self.write(") != null)");
        } else {
            self.write(&format!("({}: {{ ", blk));
            self.write(&format!("for ({}.items) |item| ", data.obj_name));
            self.write("{\n");
            self.indent_push();
            self.writeln("if (item == ");
            if let Some(arg) = data.args.first() {
                self.emit_expr(arg);
            }
            self.write(&format!(") break :{} true;", blk));
            self.indent_pop();
            self.writeln("");
            self.write("}");
            self.write(&format!(" break :{} false; }})", blk));
        }
    }

    // ── indexOf ────────────────────────────────────────
    // For string: (if (std.mem.indexOf(u8, obj, target)) |idx| @as(i64, @intCast(idx)) else @as(i64, -1))
    // For i64: (blk: { for (obj.items, 0..) |item, i| { if (item == target) break :blk @as(i64, @intCast(i)); } break :blk @as(i64, -1); })
    fn emit_index_of_inline(&mut self, data: &crate::zigir::types::IrArrayMethodInline) {
        let blk = self.next_label();
        if matches!(data.elem_type, ZigType::Str) {
            self.write("(if (std.mem.indexOf(u8, ");
            self.write(&data.obj_name);
            self.write(", ");
            if let Some(arg) = data.args.first() {
                self.emit_expr(arg);
            }
            self.write(")) |idx| @as(i64, @intCast(idx)) else @as(i64, -1))");
        } else {
            self.write(&format!("({}: {{ ", blk));
            self.write(&format!("for ({}.items, 0..) |item, i| ", data.obj_name));
            self.write("{\n");
            self.indent_push();
            self.writeln("if (item == ");
            if let Some(arg) = data.args.first() {
                self.emit_expr(arg);
            }
            self.write(&format!(") break :{} @as(i64, @intCast(i));", blk));
            self.indent_pop();
            self.writeln("");
            self.write("}");
            self.write(&format!(" break :{} @as(i64, -1); }})", blk));
        }
    }

    // ── lastIndexOf ────────────────────────────────────
    // (blk: { var __i: isize = @as(isize, @intCast(obj.items.len)) - 1; while (__i >= 0) : (__i -= 1) { if (obj.items[@as(usize, @intCast(__i))] == target) break :blk @as(i64, __i); } break :blk @as(i64, -1); })
    fn emit_last_index_of_inline(&mut self, data: &crate::zigir::types::IrArrayMethodInline) {
        let blk = self.next_label();
        self.write(&format!(
            "({}: {{ var __i: isize = @as(isize, @intCast({}.items.len)) - 1; while (__i >= 0) : (__i -= 1) {{ if ({}.items[@as(usize, @intCast(__i))] == ",
            blk, data.obj_name, data.obj_name
        ));
        if let Some(arg) = data.args.first() {
            self.emit_expr(arg);
        }
        self.write(&format!(
            ") break :{} @as(i64, __i); }} break :{} @as(i64, -1); }})",
            blk, blk
        ));
    }

    // ── join ───────────────────────────────────────────
    // (blk: { var __join_buf = std.io.Writer.Allocating.init(js_allocator.allocator());
    //   for (obj.items, 0..) |__item, __i| { if (__i > 0) __join_buf.writer().writeAll(sep) catch break :blk "";
    //     __join_buf.writer().print("{fmt}", .{__item}) catch break :blk ""; }
    //   break :blk __join_buf.toOwnedSlice() catch ""; })
    fn emit_join_inline(&mut self, data: &crate::zigir::types::IrArrayMethodInline) {
        let blk = self.next_label();
        self.write(&format!("({}: {{ ", blk));
        self.write("var __join_buf = std.io.Writer.Allocating.init(js_allocator.allocator()); ");
        self.write(&format!(
            "for ({}.items, 0..) |__item, __i| ",
            data.obj_name
        ));
        self.write("{\n");
        self.indent_push();
        let sep = if let Some(arg) = data.args.first() {
            let saved = std::mem::take(self.output_mut());
            self.emit_expr(arg);
            let rendered = std::mem::take(self.output_mut());
            *self.output_mut() = saved;
            rendered
        } else {
            ",".to_string()
        };
        self.writeln(&format!(
            "if (__i > 0) __join_buf.writer().writeAll(\"{}\") catch break :{} \"\";",
            sep.replace('\\', "\\\\").replace('"', "\\\""),
            blk
        ));
        self.writeln(&format!(
            "__join_buf.writer().print(\"{{fmt}}\", .{{__item}}) catch break :{} \"\";",
            blk
        ));
        self.indent_pop();
        self.writeln("");
        self.write("}");
        self.write(&format!(
            " break :{} __join_buf.toOwnedSlice() catch \"\"; }})",
            blk
        ));
    }

    // ── slice ──────────────────────────────────────────
    // (blk: { var __slice: std.ArrayList(elem_type) = .empty;
    //   __slice.appendSlice(js_allocator.allocator(), obj.items[start..end]) catch @panic("OOM");
    //   break :blk __slice; })
    fn emit_slice_inline(&mut self, data: &crate::zigir::types::IrArrayMethodInline) {
        let blk = self.next_label();
        let elem_type_str = data.elem_type.to_zig_type();
        self.write(&format!("({}: {{ ", blk));
        self.write(&format!(
            "var __slice: std.ArrayList({}) = .empty; ",
            elem_type_str
        ));

        // Build the slice expression: obj.items, obj.items[start..], or obj.items[start..end]
        self.write("__slice.appendSlice(js_allocator.allocator(), ");
        self.write(&data.obj_name);
        self.write(".items");
        match data.args.len() {
            0 => {}
            1 => {
                self.write("[");
                self.emit_expr(&data.args[0]);
                self.write("..]");
            }
            _ => {
                self.write("[");
                self.emit_expr(&data.args[0]);
                self.write("..");
                self.emit_expr(&data.args[1]);
                self.write("]");
            }
        }
        self.write(") catch @panic(\"OOM: Array.slice appendSlice\"); ");
        self.write(&format!("break :{} __slice; }})", blk));
    }

    // ── splice ─────────────────────────────────────────
    // (blk: { var __spliced: std.ArrayList(elem_type) = .empty;
    //   const __start = @as(usize, @intCast(@max(0, start)));
    //   const __cnt = @as(usize, @intCast(@min(@max(0, count), obj.items.len -| __start)));
    //   var __i: usize = 0; while (__i < __cnt) : (__i += 1) { __spliced.append(allocator, obj.orderedRemove(__start)) catch @panic("OOM"); }
    //   [insert items if args > 2]
    //   break :blk __spliced; })
    fn emit_splice_inline(&mut self, data: &crate::zigir::types::IrArrayMethodInline) {
        let blk = self.next_label();
        let elem_type_str = data.elem_type.to_zig_type();
        self.write(&format!("({}: {{ ", blk));
        self.write(&format!(
            "var __spliced: std.ArrayList({}) = .empty; ",
            elem_type_str
        ));
        self.write("const __start = @as(usize, @intCast(@max(0, ");
        if let Some(arg) = data.args.first() {
            self.emit_expr(arg);
        } else {
            self.write("0");
        }
        self.write("))); ");
        self.write("const __cnt = @as(usize, @intCast(@min(@max(0, ");
        if data.args.len() >= 2 {
            self.emit_expr(&data.args[1]);
        } else {
            self.write("0");
        }
        self.write(&format!("), {}.items.len -| __start))); ", data.obj_name));
        self.write("var __i: usize = 0; while (__i < __cnt) : (__i += 1) { ");
        self.write(&format!(
            "__spliced.append(js_allocator.allocator(), {}.orderedRemove(__start)) catch @panic(\"OOM: Array.splice\"); }} ",
            data.obj_name
        ));
        // Insert items if provided (args beyond start and count)
        for item_arg in data.args.iter().skip(2) {
            self.write(&format!("{}.insert(__start, ", data.obj_name));
            self.emit_expr(item_arg);
            self.write(") catch @panic(\"OOM: Array.splice insert\"); ");
        }
        self.write(&format!("break :{} __spliced; }})", blk));
    }

    // ── at ─────────────────────────────────────────────
    // (blk: { const __idx = arg; const __at_idx = if (__idx < 0) @as(usize, @intCast(@as(isize, @intCast(obj.items.len)) + @as(isize, @intCast(__idx)))) else @as(usize, @intCast(__idx)); break :blk obj.items[__at_idx]; })
    fn emit_at_inline(&mut self, data: &crate::zigir::types::IrArrayMethodInline) {
        let blk = self.next_label();
        self.write(&format!("({}: {{ ", blk));
        self.write("const __idx = ");
        if let Some(arg) = data.args.first() {
            self.emit_expr(arg);
        } else {
            self.write("0");
        }
        self.write("; ");
        self.write(&format!(
            "const __at_idx = if (__idx < 0) @as(usize, @intCast(@as(isize, @intCast({}.items.len)) + @as(isize, @intCast(__idx)))) else @as(usize, @intCast(__idx)); ",
            data.obj_name
        ));
        self.write(&format!(
            "break :{} {}.items[__at_idx]; }})",
            blk, data.obj_name
        ));
    }

    // ── concat ─────────────────────────────────────────
    // (blk: { var __concat: std.ArrayList(elem_type) = .empty;
    //   __concat.appendSlice(allocator, obj.items) catch @panic("OOM");
    //   [for each arg:] __concat.appendSlice(allocator, arg.items) catch @panic("OOM");
    //   break :blk __concat; })
    fn emit_concat_inline(&mut self, data: &crate::zigir::types::IrArrayMethodInline) {
        let blk = self.next_label();
        let elem_type_str = data.elem_type.to_zig_type();
        self.write(&format!("({}: {{ ", blk));
        self.write(&format!(
            "var __concat: std.ArrayList({}) = .empty; ",
            elem_type_str
        ));
        self.write(&format!(
            "__concat.appendSlice(js_allocator.allocator(), {}.items) catch @panic(\"OOM: Array.concat appendSlice\"); ",
            data.obj_name
        ));
        for arg in &data.args {
            self.write("__concat.appendSlice(js_allocator.allocator(), ");
            self.emit_expr(arg);
            self.write(".items) catch @panic(\"OOM: Array.concat appendSlice\"); ");
        }
        self.write(&format!("break :{} __concat; }})", blk));
    }

    // ── copyWithin ─────────────────────────────────────
    // Simplified: for (obj.items[@intCast(start)..@intCast(end)]) |*elem, i| { elem.* = obj.items[@intCast(target) + i]; }
    // Full Codegen version has reverse copy logic when target > start.
    // For now, emit a simpler forward-only version.
    fn emit_copy_within_inline(&mut self, data: &crate::zigir::types::IrArrayMethodInline) {
        let blk = self.next_label();
        self.write(&format!("({}: {{ ", blk));
        self.write("const __cpw_target = @as(isize, @intCast(");
        if let Some(arg) = data.args.first() {
            self.emit_expr(arg);
        } else {
            self.write("0");
        }
        self.write(")); ");
        self.write("const __cpw_start = @as(isize, @intCast(");
        if data.args.len() >= 2 {
            self.emit_expr(&data.args[1]);
        } else {
            self.write("0");
        }
        self.write(")); ");
        self.write("const __cpw_end = @as(isize, @intCast(");
        if data.args.len() >= 3 {
            self.emit_expr(&data.args[2]);
        } else {
            self.write(&format!("@as(i64, @intCast({}.items.len))", data.obj_name));
        }
        self.write(")); ");
        self.write("const __cpw_cnt = __cpw_end - __cpw_start; ");
        self.write("if (__cpw_cnt > 0) { ");
        self.write("const __src = @as(usize, @intCast(__cpw_start)); const __dst = @as(usize, @intCast(__cpw_target)); ");
        self.write(&format!(
            "for (0..@as(usize, @intCast(__cpw_cnt))) |__j| {{ {}.items[__dst + __j] = {}.items[__src + __j]; }} }} ",
            data.obj_name, data.obj_name
        ));
        self.write(&format!("break :{} &{}; }})", blk, data.obj_name));
    }

    // ── fill ───────────────────────────────────────────
    // for (obj.items[@intCast(start)..@intCast(end)]) |*elem| { elem.* = val; }
    fn emit_fill_inline(&mut self, data: &crate::zigir::types::IrArrayMethodInline) {
        self.write(&format!("for ({}.items", data.obj_name));
        match data.args.len() {
            0 => {
                // fill entire array
                self.write(") |*elem| { elem.* = ");
                self.write("undefined"); // no value arg
                self.write("; }");
            }
            1 => {
                // fill(value) — fill entire array
                self.write(") |*elem| { elem.* = ");
                self.emit_expr(&data.args[0]);
                self.write("; }");
            }
            2 => {
                // fill(value, start)
                self.write("[@intCast(");
                self.emit_expr(&data.args[1]);
                self.write(")..]) |*elem| { elem.* = ");
                self.emit_expr(&data.args[0]);
                self.write("; }");
            }
            _ => {
                // fill(value, start, end)
                self.write("[@intCast(");
                self.emit_expr(&data.args[1]);
                self.write(")..@intCast(");
                self.emit_expr(&data.args[2]);
                self.write(")]) |*elem| { elem.* = ");
                self.emit_expr(&data.args[0]);
                self.write("; }");
            }
        }
    }
}
