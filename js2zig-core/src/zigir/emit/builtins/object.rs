// zigir/emit/builtins/object.rs
// Object, JSON, Number, Symbol, Console builtin method emission.

use crate::zigir::emit::helpers::EmitterHelpers;
use crate::zigir::types::IrExpr;

use crate::zigir::emit::Emitter;

impl Emitter {
    /// Emit `js_object.method(js_allocator.allocator(), args) catch @panic("OOM: Object.method")`.
    /// Shared by keys, values, entries, getOwnPropertyNames.
    fn emit_object_alloc_method(&mut self, method: &str, args: &[IrExpr]) {
        self.write(&format!("js_object.{}(js_allocator.allocator(), ", method));
        self.emit_inline_args(args);
        self.write(&format!(") catch @panic(\"OOM: Object.{}\")", method));
    }

    /// Emit `js_object.methodStruct(@TypeOf(args))`.
    /// Shared by keysStruct, getOwnPropertyNamesStruct.
    fn emit_object_struct_method(&mut self, method: &str, args: &[IrExpr]) {
        self.write(&format!("js_object.{}Struct(@TypeOf(", method));
        self.emit_inline_args(args);
        self.write("))");
    }

    pub(super) fn emit_object_builtin(&mut self, method: &str, args: &[IrExpr]) {
        match method {
            // ── No-op methods (Zig is immutable by default) ──
            "freeze" | "seal" | "preventExtensions" => {
                // Object.freeze(obj) → obj (no-op, Zig structs are immutable)
                // Emit the first argument directly
                if let Some(arg) = args.first() {
                    self.emit_expr(arg);
                } else {
                    self.write(&format!("js_object.{}(", method));
                    self.emit_inline_args(args);
                    self.write(")");
                }
            }
            // ── Always-true / Always-false (Zig is sealed/frozen by default) ──
            "isSealed" | "isFrozen" => {
                // Object.isSealed(obj) → true (Zig structs are always sealed)
                self.write("true");
            }
            "isExtensible" => {
                // Object.isExtensible(obj) → false (Zig structs cannot be extended)
                self.write("false");
            }
            // ── Object.is — NaN-safe SameValue comparison ──
            "is" => {
                // Object.is(a, b) → (std.math.isNan(a) and std.math.isNan(b)) or (a == b)
                self.write("((std.math.isNan(");
                if let Some(a) = args.first() {
                    self.emit_expr(a);
                }
                self.write(") and std.math.isNan(");
                if args.len() >= 2 {
                    self.emit_expr(&args[1]);
                }
                self.write(")) or (");
                if let Some(a) = args.first() {
                    self.emit_expr(a);
                }
                self.write(" == ");
                if args.len() >= 2 {
                    self.emit_expr(&args[1]);
                }
                self.write("))");
            }
            // ── Object.hasOwn — comptime @hasField for struct+string, else runtime ──
            "hasOwn" => {
                // If args are (Ident, StringLiteral), emit comptime @hasField
                if args.len() == 2 {
                    if let (IrExpr::Ident(ident), IrExpr::StringLiteral(key)) = (&args[0], &args[1])
                    {
                        self.write(&format!(
                            "@hasField(@TypeOf({}), \"{}\")",
                            ident.zig_name, key
                        ));
                    } else {
                        self.write("js_object.hasOwn(");
                        self.emit_inline_args(args);
                        self.write(")");
                    }
                } else {
                    self.write("js_object.hasOwn(");
                    self.emit_inline_args(args);
                    self.write(")");
                }
            }
            // ── Object.keys/values/entries/getOwnPropertyNames — need allocator prefix ──
            "keys" | "values" | "entries" | "getOwnPropertyNames" => {
                self.emit_object_alloc_method(method, args);
            }
            // ── Object.keysStruct/getOwnPropertyNamesStruct — comptime reflection ──
            "keysStruct" | "getOwnPropertyNamesStruct" => {
                self.emit_object_struct_method(method, args);
            }
            "groupBy" => {
                // Object.groupBy(items, callbackFn) — fully inline emission
                // Generates: blk: { var _grp_map = std.StringHashMap(std.ArrayList(JsAny)).init(alloc);
                //   for (items_arg.items) |_grp_item| {
                //     const <param> = _grp_item; const _grp_key = <callback body expr>;
                //     // insert into map
                //   }
                //   break :blk _grp_map }
                self.write("blk: { var _grp_map = std.StringHashMap(std.ArrayList(JsAny)).init(js_allocator.allocator()); errdefer _grp_map.deinit(); ");
                if let Some(items_arg) = args.first() {
                    self.write("for (");
                    self.emit_expr(items_arg);
                    self.write(".items) |_grp_item| { ");
                    if args.len() >= 2 {
                        match &args[1] {
                            IrExpr::ArrowFn(arrow) => {
                                let param_name = arrow
                                    .params
                                    .first()
                                    .map(|p| p.name.zig_name.clone())
                                    .unwrap_or_else(|| "_".to_string());
                                self.write(&format!(
                                    "const {} = _grp_item; const _grp_key = ",
                                    param_name
                                ));
                                // Concise arrow body is IrStmt::Return { value: Some(expr) }
                                if let Some(first_stmt) = arrow.body.stmts.first() {
                                    match first_stmt {
                                        crate::zigir::types::IrStmt::Return { value: Some(v) } => {
                                            self.emit_expr(v)
                                        }
                                        crate::zigir::types::IrStmt::Expr(e) => self.emit_expr(e),
                                        _ => self.write("_grp_item"),
                                    }
                                }
                            }
                            IrExpr::Closure(closure) => {
                                let param_name = closure
                                    .fn_params
                                    .first()
                                    .map(|p| p.name.zig_name.clone())
                                    .unwrap_or_else(|| "_".to_string());
                                self.write(&format!(
                                    "const {} = _grp_item; const _grp_key = ",
                                    param_name
                                ));
                                if let Some(last) = closure.body.stmts.last() {
                                    match last {
                                        crate::zigir::types::IrStmt::Return { value: Some(v) } => {
                                            self.emit_expr(v)
                                        }
                                        crate::zigir::types::IrStmt::Expr(e) => self.emit_expr(e),
                                        _ => self.write("_grp_item"),
                                    }
                                }
                            }
                            _ => {
                                self.write("const _grp_key = ");
                                self.emit_expr(&args[1]);
                                self.write("(_grp_item)");
                            }
                        }
                    } else {
                        self.write("const _grp_key = _grp_item");
                    }
                    self.write("; if (_grp_map.getPtr(_grp_key)) |_grp_list| { _grp_list.append(js_allocator.allocator(), JsAny.from(_grp_item)) catch @panic(\"OOM\"); } else { var _grp_new_list = std.ArrayList(JsAny).init(js_allocator.allocator()); _grp_new_list.append(js_allocator.allocator(), JsAny.from(_grp_item)) catch @panic(\"OOM\"); _grp_map.put(_grp_key, _grp_new_list) catch @panic(\"OOM\"); } } ");
                }
                self.write("break :blk _grp_map; }");
            }
            // ── Object.getOwnPropertyDescriptor — needs allocator prefix ──
            "getOwnPropertyDescriptor" => {
                self.write("js_object.getOwnPropertyDescriptor(js_allocator.allocator(), ");
                self.emit_inline_args(args);
                self.write(")");
            }
            // ── Default: js_object.method(args) ──
            _ => {
                self.emit_module_call("js_object", method, args);
            }
        }
    }

    pub(super) fn emit_json_builtin(&mut self, method: &str, args: &[IrExpr]) {
        match method {
            "parse" => {
                self.write("js_json.parse(js_allocator.allocator(), ");
                if let Some(first_arg) = args.first() {
                    self.emit_expr(first_arg);
                } else {
                    self.write("\"\"");
                }
                if args.len() >= 2 {
                    self.write(", ");
                    self.emit_expr(&args[1]);
                } else {
                    self.write(", null");
                }
                self.write(") catch @panic(\"JSON.parse error\")");
            }
            "stringify" => {
                self.write("try js_json.stringify(js_allocator.allocator(), ");
                if let Some(first_arg) = args.first() {
                    self.emit_expr(first_arg);
                } else {
                    self.write("JsAny.fromUndefined()");
                }
                if args.len() >= 2 {
                    self.write(", ");
                    self.emit_expr(&args[1]);
                } else {
                    self.write(", null");
                }
                if args.len() >= 3 {
                    self.write(", ");
                    self.emit_expr(&args[2]);
                } else {
                    self.write(", null");
                }
                self.write(") catch @panic(\"OOM: JSON.stringify\")");
            }
            _ => {
                self.emit_module_call("js_json", method, args);
            }
        }
    }

    pub(super) fn emit_number_builtin(&mut self, method: &str, obj: Option<&str>, args: &[IrExpr]) {
        match method {
            "toFixed" | "toExponential" | "toPrecision" => {
                // js_number.toFixed(js_allocator.allocator(), obj, digits)
                self.write(&format!("js_number.{}(js_allocator.allocator(), ", method));
                if let Some(name) = obj {
                    self.write(name);
                }
                for arg in args.iter() {
                    self.write(", ");
                    self.emit_expr(arg);
                }
                self.write(")");
            }
            "parseInt" => {
                self.write("js_number.parseInt(");
                if let Some(name) = obj {
                    self.write(name);
                }
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 || obj.is_some() {
                        self.write(", ");
                    }
                    self.emit_expr(arg);
                }
                // parseInt requires (value, radix) — add null if only value provided
                if args.len() < 2 {
                    self.write(", null");
                }
                self.write(")");
            }
            _ => {
                self.emit_module_call("js_number", method, args);
            }
        }
    }

    pub(super) fn emit_symbol_builtin(&mut self, method: &str, obj: Option<&str>, args: &[IrExpr]) {
        // Avoid Zig keyword conflicts: Symbol.for → symbolFor, Symbol.keyFor → symbolKeyFor
        let zig_method = match method {
            "for" => "symbolFor",
            "keyFor" => "symbolKeyFor",
            other => other,
        };

        match method {
            // Symbol() / Symbol(desc) — constructor
            "constructor" => {
                if args.is_empty() {
                    // Symbol() → js_symbol.JsSymbol.initAnonymous()
                    self.write("js_symbol.JsSymbol.initAnonymous()");
                } else {
                    // Symbol("desc") → js_symbol.JsSymbol.init("desc")
                    self.write("js_symbol.JsSymbol.init(");
                    self.emit_inline_args(args);
                    self.write(")");
                }
            }
            // Instance methods that use the receiver: sym.toString(), sym.description, etc.
            "toString" => {
                if let Some(name) = obj {
                    self.write(&format!("{}.toString(js_allocator.allocator())", name));
                } else {
                    self.write(&format!(
                        "js_symbol.{}(js_allocator.allocator())",
                        zig_method
                    ));
                }
            }
            "description" => {
                if let Some(name) = obj {
                    self.write(&format!("{}.description", name));
                } else {
                    self.write(&format!("js_symbol.{}", zig_method));
                }
            }
            // Static methods: js_symbol.symbolFor(key), js_symbol.symbolKeyFor(sym), etc.
            _ => {
                self.emit_module_call("js_symbol", zig_method, args);
            }
        }
    }

    pub(super) fn emit_console_builtin(&mut self, method: &str, args: &[IrExpr]) {
        if args.len() <= 1 {
            // Single-arg: js_console.log(msg), js_console.err(msg), js_console.warn(msg)
            self.write(&format!("js_console.{}(", method));
            if let Some(arg) = args.first() {
                self.emit_expr(arg);
            }
            self.write(")");
        } else {
            // Multi-arg: js_console.logMulti(.{ arg1, arg2, ... })
            let multi_method = match method {
                "log" => "logMulti",
                "err" => "errMulti",
                "warn" => "warnMulti",
                other => {
                    // Fallback: append "Multi" for unknown methods
                    self.write(&format!("js_console.{}Multi(", other));
                    self.write(".{");
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 {
                            self.write(", ");
                        }
                        self.emit_expr(arg);
                    }
                    self.write("})");
                    return;
                }
            };
            self.write(&format!("js_console.{}(", multi_method));
            self.write(".{");
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    self.write(", ");
                }
                self.emit_expr(arg);
            }
            self.write("})");
        }
    }
}
