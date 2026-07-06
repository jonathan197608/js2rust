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
            | "getUTCMinutes" | "getUTCSeconds" | "getUTCMilliseconds" | "setFullYear"
            | "setMonth" | "setDate" | "setHours" | "setMinutes" | "setSeconds"
            | "setMilliseconds" | "setUTCFullYear" | "setUTCMonth" | "setUTCDate"
            | "setUTCHours" | "setUTCMinutes" | "setUTCSeconds" | "setUTCMilliseconds"
            | "valueOf" => {
                if let Some(name) = obj {
                    self.write(&format!("{}.{}(", name, method));
                } else {
                    self.write(&format!("js_date.{}(", method));
                }
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.emit_expr(arg);
                }
                self.write(")");
            }
            // Instance methods that need allocator
            "toISOString" | "toString" | "toDateString" | "toTimeString" | "toJSON"
            | "toLocaleString" => {
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
                let n_args = args.len();
                // Defaults: [y=1970, m=0, d=1, h=0, min=0, s=0, ms=0]
                let defaults = ["1970", "0", "1", "0", "0", "0", "0"];
                for (i, default) in defaults.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    if i < n_args {
                        self.emit_expr(&args[i]);
                    } else {
                        self.write(default);
                    }
                }
                self.write(")");
            }
            _ => {
                // Static methods: js_date.method(args)
                self.write(&format!("js_date.{}(", method));
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
            self.write(&format!("js_typedarray.{}(", method));
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    self.write(", ");
                }
                self.emit_expr(arg);
            }
            self.write(")");
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
            "parseInt" => {
                self.write("js_uri.parseInt(");
                self.emit_inline_args(args);
                if args.len() < 2 {
                    self.write(", null");
                }
                self.write(")");
            }
            "parseFloat" => {
                self.write("js_uri.parseFloat(");
                self.emit_inline_args(args);
                self.write(")");
            }
            "isNaN" => {
                self.write("js_uri.isNaN(");
                self.emit_inline_args(args);
                self.write(")");
            }
            "isFinite" => {
                self.write("js_uri.isFinite(");
                self.emit_inline_args(args);
                self.write(")");
            }
            _ => {
                self.write(&format!("js_uri.{}(", method));
                self.emit_inline_args(args);
                self.write(")");
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
            // BigInt(x) → js_bigint.JsBigInt.fromI64(allocator, x) catch @panic("OOM: BigInt fromI64")
            "bigIntConstructor" => {
                self.write("(js_bigint.JsBigInt.fromI64(js_allocator.allocator(), ");
                self.emit_inline_args(args);
                self.write(") catch @panic(\"OOM: BigInt fromI64\"))");
            }
            // -bigintExpr → (expr).neg(allocator) catch @panic("OOM: BigInt neg")
            "bigIntNeg" => {
                self.write("(");
                if let Some(o) = obj {
                    self.write(o);
                }
                self.write(".neg(js_allocator.allocator()) catch @panic(\"BigInt neg OOM\"))");
            }
            // ~bigintExpr → (expr).bitwiseNot(allocator) catch @panic("OOM: BigInt bitwiseNot")
            "bigIntBitwiseNot" => {
                self.write("(");
                if let Some(o) = obj {
                    self.write(o);
                }
                self.write(".bitwiseNot(js_allocator.allocator()) catch @panic(\"BigInt bitwiseNot OOM\"))");
            }
            _ => {
                self.write(&format!("js_bigint.{}(", method));
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
    pub(super) fn emit_collections_builtin(
        &mut self,
        method: &str,
        obj: Option<&str>,
        args: &[crate::zigir::types::IrExpr],
    ) {
        match method {
            // ── Map instance methods (use receiver) ──
            "set" => {
                // map.set(key, val) → map.set(JsAny.from(key), JsAny.from(val)) catch @panic("OOM: allocation")
                if let Some(name) = obj {
                    self.write(&format!("{}.set(", name));
                } else {
                    self.write("js_collections.set(");
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
                self.write(") catch @panic(\"OOM: allocation\")");
            }
            "get" => {
                // map.get(key) → map.get(JsAny.from(key))
                if let Some(name) = obj {
                    self.write(&format!("{}.get(", name));
                } else {
                    self.write("js_collections.get(");
                }
                if let Some(key) = args.first() {
                    self.write("JsAny.from(");
                    self.emit_expr(key);
                    self.write(")");
                }
                self.write(")");
            }
            "has" => {
                // map.has(key) → map.has(JsAny.from(key))
                if let Some(name) = obj {
                    self.write(&format!("{}.has(", name));
                } else {
                    self.write("js_collections.has(");
                }
                if let Some(key) = args.first() {
                    self.write("JsAny.from(");
                    self.emit_expr(key);
                    self.write(")");
                }
                self.write(")");
            }
            "delete" => {
                // map.delete(key) → map.delete(JsAny.from(key))
                if let Some(name) = obj {
                    self.write(&format!("{}.delete(", name));
                } else {
                    self.write("js_collections.delete(");
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
                    self.write(&format!("{}.clear()", name));
                } else {
                    self.write("js_collections.clear()");
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
                // set.add(val) → set.add(JsAny.from(val)) catch @panic("OOM: allocation")
                if let Some(name) = obj {
                    self.write(&format!("{}.add(", name));
                } else {
                    self.write("js_collections.add(");
                }
                if let Some(val) = args.first() {
                    self.write("JsAny.from(");
                    self.emit_expr(val);
                    self.write(")");
                }
                self.write(") catch @panic(\"OOM: allocation\")");
            }
            // ── forEach — handled by IrArrayCallbackInline, not here ──
            "forEach" => {
                // Fallback: should be handled by callback inline, not here
                self.write(&format!("js_collections.{}(", method));
                self.emit_inline_args(args);
                self.write(")");
            }
            // ── Default ──
            _ => {
                self.write(&format!("js_collections.{}(", method));
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
                let blk = self.next_label();
                self.write(&format!("{blk}: {{ const _dk = "));
                if let Some(arg) = args.first() {
                    self.emit_expr(arg);
                }
                self.write("; _ = ");
                if let Some(name) = obj {
                    self.write(name);
                }
                self.write(&format!(".deleteByKey(_dk, alloc); break :{blk} true; }}"));
            }
            _ => {
                // Default: js_runtime.method(args)
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
    }
}
