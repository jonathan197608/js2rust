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
}
