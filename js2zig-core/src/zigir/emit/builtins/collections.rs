// zigir/emit/builtins/collections.rs
// Date, Map/Set, TypedArray, URI, BigInt, Runtime builtin method emission.

use crate::zigir::emit::helpers::EmitterHelpers;

use crate::zigir::emit::Emitter;

impl Emitter {
    pub(super) fn emit_date_builtin(
        &mut self,
        method: &str,
        obj: Option<&str>,
        args: &[crate::zigir::types::IrExpr],
    ) {
        match method {
            // Instance methods: receiver.method() — not js_date.method()
            "getTime" | "getFullYear" | "getMonth" | "getDate" | "getDay" | "getHours"
            | "getMinutes" | "getSeconds" | "getMilliseconds" | "getTimezoneOffset"
            | "getUTCFullYear" | "getUTCMonth" | "getUTCDate" | "getUTCDay" | "getUTCHours"
            | "getUTCMinutes" | "getUTCSeconds" | "getUTCMilliseconds" | "setDate"
            | "setMilliseconds" | "setUTCDate" | "setUTCMilliseconds" | "setTime" | "valueOf" => {
                if method.starts_with("set") {
                    self.emit_date_setter_simple(obj, method, args);
                } else {
                    self.emit_receiver_or_module_call(obj, "js_date", method, args);
                }
            }
            // Date setters with optional params — pad missing args with null:
            // setFullYear(year, month?, date?) → 3 slots
            "setFullYear" | "setUTCFullYear" => {
                if let Some(name) = obj {
                    self.write(&format!("{}.{}(", name, method));
                } else {
                    self.write(&format!("js_date.{}(", method));
                }
                let defaults = ["0", "null", "null"];
                self.emit_args_with_defaults_i64(args, 3, &defaults);
                self.write(")");
            }
            // setMonth(month, date?) → 2 slots
            "setMonth" | "setUTCMonth" => {
                if let Some(name) = obj {
                    self.write(&format!("{}.{}(", name, method));
                } else {
                    self.write(&format!("js_date.{}(", method));
                }
                let defaults = ["0", "null"];
                self.emit_args_with_defaults_i64(args, 2, &defaults);
                self.write(")");
            }
            // setHours(hours, min?, sec?, ms?) → 4 slots
            "setHours" | "setUTCHours" => {
                if let Some(name) = obj {
                    self.write(&format!("{}.{}(", name, method));
                } else {
                    self.write(&format!("js_date.{}(", method));
                }
                let defaults = ["0", "null", "null", "null"];
                self.emit_args_with_defaults_i64(args, 4, &defaults);
                self.write(")");
            }
            // setMinutes(min, sec?, ms?) → 3 slots
            "setMinutes" | "setUTCMinutes" => {
                if let Some(name) = obj {
                    self.write(&format!("{}.{}(", name, method));
                } else {
                    self.write(&format!("js_date.{}(", method));
                }
                let defaults = ["0", "null", "null"];
                self.emit_args_with_defaults_i64(args, 3, &defaults);
                self.write(")");
            }
            // setSeconds(sec, ms?) → 2 slots
            "setSeconds" | "setUTCSeconds" => {
                if let Some(name) = obj {
                    self.write(&format!("{}.{}(", name, method));
                } else {
                    self.write(&format!("js_date.{}(", method));
                }
                let defaults = ["0", "null"];
                self.emit_args_with_defaults_i64(args, 2, &defaults);
                self.write(")");
            }
            // Instance methods that need allocator
            "toISOString" | "toString" | "toDateString" | "toTimeString" | "toJSON"
            | "toLocaleString" | "toLocaleDateString" | "toLocaleTimeString" | "toUTCString" => {
                if let Some(name) = obj {
                    self.write(&format!(
                        "try {}.{}(js_allocator.allocator())",
                        name, method
                    ));
                } else {
                    self.write(&format!("try js_date.{}(js_allocator.allocator())", method));
                }
            }
            // UTC with default argument filling:
            // Date.UTC(y) → js_date.utc(y, 0, 1, 0, 0, 0, 0)
            // Date.UTC(y, m) → js_date.utc(y, m, 1, 0, 0, 0, 0)
            // etc.
            "utc" => {
                self.write("js_date.utc(");
                // Defaults: [y=1970, m=0, d=1, h=0, min=0, s=0, ms=0]
                let defaults = ["1970", "0", "1", "0", "0", "0", "0"];
                self.emit_args_with_defaults_i64(args, 7, &defaults);
                self.write(")");
            }
            _ => {
                // Static methods: js_date.method(args)
                self.emit_module_call("js_date", method, args);
            }
        }
    }

    /// Emit a simple Date setter (single i64 argument): `obj.method(arg)`.
    /// Coerces the argument to i64 to handle JsAny/F64 variables.
    fn emit_date_setter_simple(
        &mut self,
        obj: Option<&str>,
        method: &str,
        args: &[crate::zigir::types::IrExpr],
    ) {
        if let Some(name) = obj {
            self.write(&format!("{}.{}(", name, method));
        } else {
            self.write(&format!("js_date.{}(", method));
        }
        if let Some(arg) = args.first() {
            self.emit_i64_coerced(arg);
        } else {
            self.write("0");
        }
        self.write(")");
    }
    pub(super) fn emit_typedarray_builtin(
        &mut self,
        method: &str,
        obj: Option<&str>,
        args: &[crate::zigir::types::IrExpr],
        ta_type_suffix: Option<&str>,
    ) {
        if let Some(suffix) = ta_type_suffix {
            self.write(&format!("js_runtime.js_typedarray.{}{}(", method, suffix));
            if let Some(name) = obj {
                self.write(name);
                for arg in args.iter() {
                    self.write(", ");
                    self.emit_expr(arg);
                }
            } else {
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.emit_expr(arg);
                }
            }
            self.write(")");
        } else {
            // No type suffix: use js_runtime.js_typedarray prefix (js_typedarray
            // is not imported as a standalone module).
            self.emit_module_call("js_runtime.js_typedarray", method, args);
        }
    }
    pub(super) fn emit_uri_builtin(
        &mut self,
        method: &str,
        _obj: Option<&str>,
        args: &[crate::zigir::types::IrExpr],
    ) {
        // URI methods: all need js_allocator.allocator() and specific catch patterns.
        // encodeURI/encodeURIComponent: catch @panic("OOM: ...")
        // decodeURI/decodeURIComponent: catch "" (outside try block)
        match method {
            "encodeURI" | "encodeURIComponent" => {
                self.write(&format!("js_uri.{}(js_allocator.allocator(), ", method));
                self.emit_inline_args(args);
                self.write(&format!(") catch @panic(\"OOM: {}\")", method));
            }
            "decodeURI" | "decodeURIComponent" => {
                self.write(&format!("js_uri.{}(js_allocator.allocator(), ", method));
                self.emit_inline_args(args);
                self.write(")");
                if let Some(label) = &self.inside_try_block {
                    // Inside a try block: propagate the original error so
                    // the catch dispatch can map it to the correct JsError name
                    self.write(&format!(
                        " catch |err| break :{} @as(anyerror!void, err)",
                        label
                    ));
                } else {
                    // Not inside a try block: swallow error
                    self.write(" catch \"\"");
                }
            }
            "parseInt" => {
                self.write("js_uri.parseInt(");
                self.emit_inline_args(args);
                if args.len() < 2 {
                    self.write(", null");
                }
                self.write(")");
            }
            _ => {
                self.emit_module_call("js_uri", method, args);
            }
        }
    }
    pub(super) fn emit_bigint_builtin(
        &mut self,
        method: &str,
        obj: Option<&str>,
        args: &[crate::zigir::types::IrExpr],
    ) {
        match method {
            // BigInt(x) → js_bigint.JsBigInt.fromValue(allocator, x) catch @panic("OOM: BigInt fromValue")
            // fromValue uses comptime dispatch: string → init(), i64 → fromI64()
            "bigIntConstructor" => {
                self.write("(js_bigint.JsBigInt.fromValue(js_allocator.allocator(), ");
                self.emit_inline_args(args);
                self.write(") catch @panic(\"OOM: BigInt fromValue\"))");
            }
            "bigIntNeg" | "bigIntBitwiseNot" => {
                // (-expr).method(allocator) catch @panic(...)
                let (zig_method, panic_ctx) = if method == "bigIntNeg" {
                    ("neg", "BigInt neg OOM")
                } else {
                    ("bitwiseNot", "BigInt bitwiseNot OOM")
                };
                self.write("(");
                if let Some(o) = obj {
                    self.write(o);
                }
                self.write(&format!(
                    ".{}(js_allocator.allocator()) catch @panic(\"{}\"))",
                    zig_method, panic_ctx
                ));
            }
            "toString" => {
                // bigint.toString([radix]) → bigint.toString(allocator, radix) catch @panic(...)
                // R8-P1-4: radix defaults to 10 per ECMA-262. Previously the
                // radix argument was silently dropped. Fallible builtin call
                // convention: catch @panic coerces the error union to []u8.
                self.write("(");
                if let Some(o) = obj {
                    self.write(o);
                }
                self.write(".toString(js_allocator.allocator(), ");
                if !args.is_empty() {
                    self.emit_expr(&args[0]);
                } else {
                    self.write("10");
                }
                self.write(") catch @panic(\"OOM: BigInt toString\"))");
            }
            "toLocaleString" => {
                // toLocaleString() ignores any radix and is equivalent to
                // toString(10) per JS spec (no radix parameter).
                self.write("(");
                if let Some(o) = obj {
                    self.write(o);
                }
                self.write(".toString(js_allocator.allocator(), 10) catch @panic(\"OOM: BigInt toString\"))");
            }
            "valueOf" => {
                // bigint.valueOf() → returns self (identity)
                if let Some(o) = obj {
                    self.write(o);
                } else if !args.is_empty() {
                    self.emit_expr(&args[0]);
                }
            }
            "asIntN" | "asUintN" => {
                // BigInt.asIntN(width, bigint) / BigInt.asUintN(width, bigint)
                // → js_bigint.asIntN(width, &bigint, allocator) / js_bigint.asUintN(...)
                let zig_method = if method == "asIntN" {
                    "asIntN"
                } else {
                    "asUintN"
                };
                self.write(&format!("(js_bigint.{}(", zig_method));
                if args.len() >= 2 {
                    self.emit_expr(&args[0]);
                    self.write(", &");
                    self.emit_expr(&args[1]);
                }
                self.write(", js_allocator.allocator()) catch @panic(\"OOM: BigInt ");
                self.write(zig_method);
                self.write("\"))");
            }
            _ => {
                self.emit_module_call("js_bigint", method, args);
            }
        }
    }
    pub(super) fn emit_collections_builtin(
        &mut self,
        method: &str,
        obj: Option<&str>,
        args: &[crate::zigir::types::IrExpr],
    ) {
        match method {
            // ── Map instance methods (use receiver) ──
            "set" => {
                // map.set(key, val) → (blk_N: { map.set(alloc, JsAny.from(key), JsAny.from(val)) catch @panic("OOM: allocation"); break :blk_N map })
                // Returns the map object for chaining (JS semantics).
                let blk = self.next_label();
                if let Some(name) = obj {
                    self.write(&format!(
                        "({blk}: {{ {}.set(js_allocator.allocator(), ",
                        name
                    ));
                } else {
                    self.write(&format!(
                        "({blk}: {{ js_collections.set(js_allocator.allocator(), "
                    ));
                }
                if let Some(key) = args.first() {
                    self.write("JsAny.from(");
                    self.emit_expr(key);
                    self.write(")");
                }
                if args.len() >= 2 {
                    self.write(", JsAny.from(");
                    self.emit_expr(&args[1]);
                    self.write(")");
                }
                self.write(") catch @panic(\"OOM: allocation\"); ");
                if let Some(name) = obj {
                    self.write(&format!("break :{blk} {}; }})", name));
                } else {
                    // Degenerate path: no receiver name, cannot return the map.
                    self.write(&format!("break :{blk} JsAny.undefined_value; }})"));
                }
            }
            "get" | "has" => {
                // map.get(key)/map.has(key) → map.method(JsAny.from(key))
                if let Some(name) = obj {
                    self.write(&format!("{}.{}(", name, method));
                } else {
                    self.write(&format!("js_collections.{}(", method));
                }
                if let Some(key) = args.first() {
                    self.write("JsAny.from(");
                    self.emit_expr(key);
                    self.write(")");
                }
                self.write(")");
            }
            "delete" => {
                // map.delete(key) → map.delete(js_allocator.allocator(), JsAny.from(key))
                if let Some(name) = obj {
                    self.write(&format!("{}.delete(js_allocator.allocator(), ", name));
                } else {
                    self.write("js_collections.delete(js_allocator.allocator(), ");
                }
                if let Some(key) = args.first() {
                    self.write("JsAny.from(");
                    self.emit_expr(key);
                    self.write(")");
                }
                self.write(")");
            }
            "clear" => {
                if let Some(name) = obj {
                    self.write(&format!("{}.clear(js_allocator.allocator())", name));
                } else {
                    self.write("js_collections.clear(js_allocator.allocator())");
                }
            }
            "keys" | "values" | "entries" => {
                // map.keys() etc. — need allocator
                if let Some(name) = obj {
                    self.write(&format!(
                        "{}.{}(js_allocator.allocator()) catch @panic(\"OOM: allocation\")",
                        name, method
                    ));
                } else {
                    self.write(&format!("js_collections.{}(js_allocator.allocator()) catch @panic(\"OOM: allocation\")", method));
                }
            }
            // ── Set instance methods ──
            "add" => {
                // set.add(val) → (blk_N: { set.add(JsAny.from(val)) catch @panic("OOM: allocation"); break :blk_N set })
                // Returns the set object for chaining (JS semantics).
                let blk = self.next_label();
                if let Some(name) = obj {
                    self.write(&format!("({blk}: {{ {}.add(", name));
                } else {
                    self.write(&format!("({blk}: {{ js_collections.add("));
                }
                if let Some(val) = args.first() {
                    self.write("JsAny.from(");
                    self.emit_expr(val);
                    self.write(")");
                }
                self.write(") catch @panic(\"OOM: allocation\"); ");
                if let Some(name) = obj {
                    self.write(&format!("break :{blk} {}; }})", name));
                } else {
                    // Degenerate path: no receiver name, cannot return the set.
                    self.write(&format!("break :{blk} JsAny.undefined_value; }})"));
                }
            }
            // ── forEach — handled by IrArrayCallbackInline, not here ──
            "forEach" => {
                // Fallback: should be handled by callback inline, not here
                self.emit_module_call("js_collections", method, args);
            }
            // ── Default ──
            _ => {
                self.emit_module_call("js_collections", method, args);
            }
        }
    }
    pub(super) fn emit_runtime_builtin(
        &mut self,
        method: &str,
        obj: Option<&str>,
        args: &[crate::zigir::types::IrExpr],
    ) {
        match method {
            "deleteKey" => {
                // obj.deleteKey("prop") → true
                let blk = self.next_label();
                self.write(&format!("{blk}: {{ _ = "));
                if let Some(name) = obj {
                    self.write(name);
                }
                self.write(".deleteKey(");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.emit_expr(arg);
                }
                self.write(&format!("); break :{blk} true; }}"));
            }
            "deleteByKey" => {
                // obj.deleteByKey(expr, alloc) → true
                // JsCollection.delete(alloc, JsAny.from(key)) returns bool
                let blk = self.next_label();
                self.write(&format!("{blk}: {{ const _dk = "));
                if let Some(arg) = args.first() {
                    self.emit_expr(arg);
                }
                self.write("; _ = ");
                if let Some(name) = obj {
                    self.write(name);
                }
                self.write(&format!(
                    ".delete(js_allocator.allocator(), JsAny.from(_dk)); break :{blk} true; }}"
                ));
            }
            "instanceOf" => {
                // instanceOf(value, "TypeName") → js_runtime.instanceOf(value, "TypeName")
                // The value comes from obj_expr (not obj_name), already resolved by
                // emit_builtin_call as `obj`. Fall through to emit_module_call with
                // the receiver prepended as the first argument.
                self.write("js_runtime.instanceOf(");
                if let Some(name) = obj {
                    self.write(name);
                    self.write(", ");
                }
                self.emit_inline_args(args);
                self.write(")");
            }
            _ => {
                // Default: js_runtime.method(args)
                self.emit_module_call("js_runtime", method, args);
            }
        }
    }
}
