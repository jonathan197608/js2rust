// zigir/emit/builtins/object.rs
// Object, JSON, Number, Symbol, Console builtin method emission.

use crate::types::ZigType;
use crate::zigir::emit::helpers::EmitterHelpers;
use crate::zigir::types::IrExpr;

use crate::zigir::emit::Emitter;

/// Type kind for Object.is argument dispatch.
enum ObjectIsKind {
    Numeric, // i64 or f64
    String,  // []const u8
    Bool,    // bool
    Unknown, // type not determinable at emit time (Ident, etc.)
}

/// Determine the comparison kind for Object.is(a, b) by inspecting the IrExpr
/// variants and any type annotations attached during lowering.
/// If the two args disagree (e.g. one numeric, one string), the result is
/// `Unknown` which falls through to the runtime JsAny.sameValue path.
fn object_is_arg_kind(args: &[IrExpr]) -> ObjectIsKind {
    let a = match args.first() {
        Some(e) => e,
        None => return ObjectIsKind::Unknown,
    };
    let b = match args.get(1) {
        Some(e) => e,
        None => return ObjectIsKind::Unknown,
    };
    let ka = expr_type_kind(a);
    let kb = expr_type_kind(b);
    match (ka, kb) {
        (ObjectIsKind::Numeric, ObjectIsKind::Numeric) => ObjectIsKind::Numeric,
        (ObjectIsKind::String, ObjectIsKind::String) => ObjectIsKind::String,
        (ObjectIsKind::Bool, ObjectIsKind::Bool) => ObjectIsKind::Bool,
        _ => ObjectIsKind::Unknown,
    }
}

/// Inspect a single IrExpr to determine its type kind for Object.is dispatch.
fn expr_type_kind(expr: &IrExpr) -> ObjectIsKind {
    match expr {
        IrExpr::IntLiteral(_) | IrExpr::FloatLiteral(_) => ObjectIsKind::Numeric,
        IrExpr::StringLiteral(_) => ObjectIsKind::String,
        IrExpr::BoolLiteral(_) => ObjectIsKind::Bool,
        IrExpr::Binary {
            left_type,
            right_type,
            ..
        } => {
            if let Some(t) = left_type.as_ref().or(right_type.as_ref()) {
                return zig_type_to_kind(t);
            }
            ObjectIsKind::Unknown
        }
        IrExpr::Unary { operand_type, .. } => {
            if let Some(t) = operand_type.as_ref() {
                return zig_type_to_kind(t);
            }
            ObjectIsKind::Unknown
        }
        IrExpr::BuiltinCall(bc) => zig_type_to_kind(&bc.return_type),
        _ => ObjectIsKind::Unknown,
    }
}

fn zig_type_to_kind(t: &ZigType) -> ObjectIsKind {
    match t {
        ZigType::I64 | ZigType::F64 => ObjectIsKind::Numeric,
        ZigType::Str => ObjectIsKind::String,
        ZigType::Bool => ObjectIsKind::Bool,
        _ => ObjectIsKind::Unknown,
    }
}

impl Emitter {
    /// Emit `js_object.method(js_allocator.allocator(), args) catch @panic("OOM: Object.method")`.
    /// Shared by keys, values, entries, getOwnPropertyNames.
    fn emit_object_alloc_method(&mut self, method: &str, args: &[IrExpr]) {
        self.write(&format!("js_object.{}(js_allocator.allocator(), ", method));
        self.emit_inline_args(args);
        self.write(&format!(") catch @panic(\"OOM: Object.{}\")", method));
    }

    /// Emit `js_object.method(@TypeOf(args))`.
    /// Shared by keysStruct, getOwnPropertyNamesStruct.
    /// Note: `method` already includes the "Struct" suffix (set by lower/call.rs),
    /// so we must NOT append it again.
    fn emit_object_struct_method(&mut self, method: &str, args: &[IrExpr]) {
        self.write(&format!("js_object.{}(@TypeOf(", method));
        self.emit_inline_args(args);
        self.write("))");
    }

    pub(super) fn emit_object_builtin(&mut self, method: &str, args: &[IrExpr]) {
        match method {
            // ── No-op methods (Zig is immutable by default) — return first arg per JS spec ──
            "freeze" | "seal" | "preventExtensions" => {
                // Object.freeze/seal/preventExtensions(obj) → obj
                // Zig structs are immutable, so these are no-ops that return the input.
                if let Some(arg) = args.first() {
                    self.emit_expr(arg);
                } else {
                    self.write(&format!("js_object.{}(", method));
                    self.emit_inline_args(args);
                    self.write(")");
                }
            }
            // ── Mutating methods that return obj per JS spec ──
            // Runtime functions have been updated to return the receiver pointer.
            // R8-P1-27: assign/defineProperty/defineProperties now deep-copy
            // keys (alloc.dupe), so they require an allocator parameter.
            "assign" => {
                self.write("js_object.assign(js_allocator.allocator(), ");
                self.emit_inline_args(args);
                self.write(") catch @panic(\"OOM: Object.assign\")");
            }
            "defineProperty" | "defineProperties" => {
                // Object.defineProperty/defineProperties(obj, ...) → obj
                // Runtime returns !*JsValueHashMap — must catch error.
                self.write(&format!("js_object.{}(js_allocator.allocator(), ", method));
                self.emit_inline_args(args);
                self.write(&format!(") catch @panic(\"OOM: Object.{}\")", method));
            }
            // ── Object.create — needs allocator (deep-copies keys from proto) ──
            "create" => {
                self.write("js_object.create(js_allocator.allocator(), ");
                self.emit_inline_args(args);
                self.write(") catch @panic(\"OOM: Object.create\")");
            }
            "setPrototypeOf" => {
                // Object.setPrototypeOf(obj, proto) → obj
                // Runtime returns *JsValueHashMap (no error possible).
                self.emit_module_call("js_object", method, args);
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
                // Object.is(a, b) implements ECMA-262 §7.2.10 SameValue:
                //   NaN === NaN → true (unlike ===)
                //   +0 vs -0  → false (unlike ===)
                //   Otherwise same as ===
                //
                // Dispatch based on the inferred type of each argument so that
                // the generated Zig code is type-correct.  The previous code
                // unconditionally emitted `std.math.isNan(a)…` which fails to
                // compile for `[]const u8` (strings) and `bool` arguments.
                let kind = object_is_arg_kind(args);
                match kind {
                    ObjectIsKind::Numeric => {
                        // Both args are numeric (i64 or f64).  std.math.isNan
                        // works on comptime_int, i64, and f64.  For +0/-0 we
                        // add a signbit guard.
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
                        // +0 vs -0: when both are zero, also check signbit
                        self.write(" and (");
                        if let Some(a) = args.first() {
                            self.emit_expr(a);
                        }
                        self.write(" != 0 or std.math.signbit(");
                        if let Some(a) = args.first() {
                            self.emit_expr(a);
                        }
                        self.write(") == std.math.signbit(");
                        if args.len() >= 2 {
                            self.emit_expr(&args[1]);
                        }
                        self.write(")))");
                        self.write(")");
                    }
                    ObjectIsKind::String => {
                        // Both args are []const u8 — use content comparison.
                        self.write("std.mem.eql(u8, ");
                        if let Some(a) = args.first() {
                            self.emit_expr(a);
                        }
                        self.write(", ");
                        if args.len() >= 2 {
                            self.emit_expr(&args[1]);
                        }
                        self.write(")");
                    }
                    ObjectIsKind::Bool => {
                        // Both args are bool — direct ==.
                        self.write("(");
                        if let Some(a) = args.first() {
                            self.emit_expr(a);
                        }
                        self.write(" == ");
                        if args.len() >= 2 {
                            self.emit_expr(&args[1]);
                        }
                        self.write(")");
                    }
                    ObjectIsKind::Unknown => {
                        // Type not known at emit time — wrap in JsAny and use
                        // the runtime sameValue method which handles all cases.
                        self.write("JsAny.from(");
                        if let Some(a) = args.first() {
                            self.emit_expr(a);
                        }
                        self.write(").sameValue(JsAny.from(");
                        if args.len() >= 2 {
                            self.emit_expr(&args[1]);
                        }
                        self.write("))");
                    }
                }
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
                // Uses StringArrayHashMap (managed wrapper, insertion-order-preserving)
                // so that Object.keys on the result iterates in insertion order.
                self.write("blk: { var _grp_map = StringArrayHashMap(std.ArrayList(JsAny)).init(js_allocator.allocator()); errdefer _grp_map.deinit(); ");
                if let Some(items_arg) = args.first() {
                    self.write("for (");
                    self.emit_expr(items_arg);
                    self.write(".items) |_grp_item| { ");
                    if args.len() >= 2 {
                        self.emit_group_by_callback(&args[1]);
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

    /// Emit the callback parameter binding and key expression for Object.groupBy.
    /// Handles ArrowFn (first stmt), Closure (last stmt), and fallback call.
    fn emit_group_by_callback(&mut self, callback: &IrExpr) {
        match callback {
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
                if let Some(stmt) = arrow.body.stmts.first() {
                    self.emit_stmt_value(stmt);
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
                if let Some(stmt) = closure.body.stmts.last() {
                    self.emit_stmt_value(stmt);
                }
            }
            _ => {
                self.write("const _grp_key = ");
                self.emit_expr(callback);
                self.write("(_grp_item)");
            }
        }
    }

    /// Extract the value expression from a Return or Expr statement, or emit a fallback.
    fn emit_stmt_value(&mut self, stmt: &crate::zigir::types::IrStmt) {
        match stmt {
            crate::zigir::types::IrStmt::Return { value: Some(v) } => self.emit_expr(v),
            crate::zigir::types::IrStmt::Expr(e) => self.emit_expr(e),
            _ => self.write("_grp_item"),
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
                if let Some(label) = &self.inside_try_block {
                    self.write(&format!(
                        ") catch |err| break :{} @as(anyerror!void, err)",
                        label
                    ));
                } else if self.in_function {
                    self.write(") catch return error.JsThrow");
                } else {
                    self.write(") catch @panic(\"JSON.parse failed\")");
                }
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
                // These runtime methods return ![]const u8 (RangeError on bad
                // digit counts, plus OOM). We append `catch @panic` to coerce
                // the error union back to []const u8, consistent with the
                // fallible string-method convention (string.rs): `try` would
                // require the enclosing fn to return an error union, which the
                // transpiler does not currently propagate from builtin calls.
                self.write(&format!("js_number.{}(js_allocator.allocator(), ", method));
                if let Some(name) = obj {
                    self.write(name);
                }
                for arg in args.iter() {
                    self.write(", ");
                    self.emit_expr(arg);
                }
                self.write(") catch @panic(\"Number method failed\")");
            }
            // R8-NumberToString: js_number.toString(allocator, val, radix).
            // Zig runtime requires all three args (no Zig default-param
            // support); ECMA-262 21.1.3.7 says the default radix is 10.
            // The emitter always emits `, 10` when the JS call omits it,
            // matching the slice/substring/parseInt convention.
            "toString" => {
                self.write("js_number.toString(js_allocator.allocator(), ");
                if let Some(name) = obj {
                    self.write(name);
                }
                for arg in args.iter() {
                    self.write(", ");
                    self.emit_expr(arg);
                }
                if args.is_empty() {
                    self.write(", 10");
                }
                // Same fallible-call convention as toFixed/toExponential/
                // toPrecision above: toString returns ![]const u8 (RangeError
                // on radix outside 2..36, plus OOM). `catch @panic` coerces it
                // to []const u8 without requiring error-union propagation.
                self.write(") catch @panic(\"Number method failed\")");
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
                    // init returns !JsSymbol (can fail with OOM) — unwrap with catch.
                    self.write("(js_symbol.JsSymbol.init(");
                    self.emit_inline_args(args);
                    self.write(") catch @panic(\"Symbol init OOM\"))");
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
            // Symbol.for(key) → js_symbol.symbolFor(key)
            // symbolFor returns !JsSymbol (can fail with OOM) — unwrap with catch.
            "for" => {
                self.write("(js_symbol.symbolFor(");
                self.emit_inline_args(args);
                self.write(") catch @panic(\"Symbol.for OOM\"))");
            }
            // Symbol.keyFor returns ?[]const u8 — unwrap with .? (caller ensures symbol
            // was created via Symbol.for, so key is always present).
            "keyFor" => {
                self.emit_module_call("js_symbol", "symbolKeyFor", args);
                self.write(".?");
            }
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
            let multi_method: String = match method {
                "log" => "logMulti".to_string(),
                "err" => "errMulti".to_string(),
                "warn" => "warnMulti".to_string(),
                other => format!("{}Multi", other),
            };
            self.emit_console_multi(&multi_method, args);
        }
    }

    /// Emit `js_console.method(.{ arg1, arg2, ... })`.
    fn emit_console_multi(&mut self, method: &str, args: &[IrExpr]) {
        self.write(&format!("js_console.{}(", method));
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
