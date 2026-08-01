// zigir/emit/builtins/object.rs
// Object, JSON, Number, Symbol, Console builtin method emission.

use crate::types::ZigType;
use crate::zigir::emit::helpers::{self, EmitterHelpers};
use crate::zigir::types::IrExpr;

use crate::zigir::emit::Emitter;

/// Type kind for Object.is argument dispatch.
#[derive(Clone, Copy)]
enum ObjectIsKind {
    Integer, // i64 (never NaN, no signbit issues)
    Float,   // f64 (NaN === NaN, +0 vs -0)
    String,  // []const u8
    Bool,    // bool
    Unknown, // anytype / JsAny — use runtime
}

/// Result of Object.is argument kind analysis: merged kind + individual kinds.
struct ObjectIsKinds {
    merged: ObjectIsKind,
    a: ObjectIsKind,
    b: ObjectIsKind,
}

/// Determine the comparison kind for Object.is(a, b) by inspecting the IrExpr
/// variants and any type annotations attached during lowering.
/// If the two args disagree (e.g. one numeric, one string), the result is
/// `Unknown` which falls through to the runtime JsAny.sameValue path.
fn object_is_arg_kind(args: &[IrExpr]) -> ObjectIsKinds {
    let a = match args.first() {
        Some(e) => e,
        None => {
            return ObjectIsKinds {
                merged: ObjectIsKind::Unknown,
                a: ObjectIsKind::Unknown,
                b: ObjectIsKind::Unknown,
            };
        }
    };
    let b = match args.get(1) {
        Some(e) => e,
        None => {
            return ObjectIsKinds {
                merged: ObjectIsKind::Unknown,
                a: ObjectIsKind::Unknown,
                b: ObjectIsKind::Unknown,
            };
        }
    };
    let ka = expr_type_kind(a);
    let kb = expr_type_kind(b);
    // Merge Integer+Float into the appropriate kind:
    // - Both Integer → Integer (simple ==, no NaN/signbit concerns)
    // - Both Float → Float (NaN, +0/-0)
    // - Mixed Integer+Float → Float (treat as float for correctness)
    // - Otherwise → Unknown
    let merged = match (ka, kb) {
        (ObjectIsKind::Integer, ObjectIsKind::Integer) => ObjectIsKind::Integer,
        (ObjectIsKind::Float, ObjectIsKind::Float)
        | (ObjectIsKind::Integer, ObjectIsKind::Float)
        | (ObjectIsKind::Float, ObjectIsKind::Integer) => ObjectIsKind::Float,
        (ObjectIsKind::String, ObjectIsKind::String) => ObjectIsKind::String,
        (ObjectIsKind::Bool, ObjectIsKind::Bool) => ObjectIsKind::Bool,
        _ => ObjectIsKind::Unknown,
    };
    ObjectIsKinds {
        merged,
        a: ka,
        b: kb,
    }
}

/// Inspect a single IrExpr to determine its type kind for Object.is dispatch.
fn expr_type_kind(expr: &IrExpr) -> ObjectIsKind {
    match expr {
        IrExpr::IntLiteral(_) => ObjectIsKind::Integer,
        IrExpr::FloatLiteral(_) => ObjectIsKind::Float,
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
        IrExpr::TypedIdent { ty, .. } => zig_type_to_kind(ty),
        _ => ObjectIsKind::Unknown,
    }
}

fn zig_type_to_kind(t: &ZigType) -> ObjectIsKind {
    match t {
        ZigType::I64 => ObjectIsKind::Integer,
        ZigType::F64 => ObjectIsKind::Float,
        ZigType::Str => ObjectIsKind::String,
        ZigType::Bool => ObjectIsKind::Bool,
        _ => ObjectIsKind::Unknown,
    }
}

impl Emitter {
    /// Emit an Object.is argument coerced to f64.
    /// For Integer-typed args, wrap in `@as(f64, @floatFromInt(...))`.
    /// For Float-typed args, emit directly.
    fn emit_object_is_f64_arg(&mut self, arg: &IrExpr, kind: ObjectIsKind) {
        match kind {
            ObjectIsKind::Integer => {
                self.write("@as(f64, @floatFromInt(");
                self.emit_expr(arg);
                self.write("))");
            }
            _ => {
                self.emit_expr(arg);
            }
        }
    }

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
                let kinds = object_is_arg_kind(args);
                match kinds.merged {
                    ObjectIsKind::Integer => {
                        // Both args are i64 — simple equality. Integers are never NaN,
                        // and +0/-0 distinction doesn't exist in i64 (both are 0).
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
                    ObjectIsKind::Float => {
                        // Both args are f64 (or mixed i64/f64).  std.math.isNan
                        // works on f64.  For +0/-0 we add a signbit guard.
                        // Bind args to temporaries to avoid repeated evaluation.
                        let n = self.label_counter;
                        self.label_counter += 1;
                        let blk = format!("__is_blk_{}", n);
                        let a_name = format!("__is_a_{}", n);
                        let b_name = format!("__is_b_{}", n);
                        self.write(&format!("({}: {{ const {} = ", blk, a_name));
                        if let Some(a) = args.first() {
                            self.emit_object_is_f64_arg(a, kinds.a);
                        }
                        self.write(&format!("; const {} = ", b_name));
                        if args.len() >= 2 {
                            self.emit_object_is_f64_arg(&args[1], kinds.b);
                        }
                        self.write(&format!("; break :{} ((std.math.isNan({}) and std.math.isNan({})) or ({} == {} and ({} != 0 or std.math.signbit({}) == std.math.signbit({})))); }})", blk, a_name, b_name, a_name, b_name, a_name, a_name, b_name));
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
                    let ident_name = match &args[0] {
                        IrExpr::Ident(ident) => Some(&ident.zig_name),
                        IrExpr::TypedIdent { ident, .. } => Some(&ident.zig_name),
                        _ => None,
                    };
                    if let (Some(ident_name), IrExpr::StringLiteral(key)) = (ident_name, &args[1]) {
                        self.write(&format!(
                            "@hasField(@TypeOf({}), \"{}\")",
                            ident_name,
                            helpers::escape_zig_string(key)
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
            // ── Object.keysMap/getOwnPropertyNamesMap — runtime key collection for JsObjectMap ──
            "keysMap" | "getOwnPropertyNamesMap" => {
                self.write(&format!("js_object.{}(js_allocator.allocator(), ", method));
                if let Some(arg) = args.first() {
                    self.write("&");
                    self.emit_expr(arg);
                }
                self.write(&format!(") catch @panic(\"OOM: Object.{}\")", method));
            }
            "groupBy" => {
                // Object.groupBy(items, callbackFn) — fully inline emission
                // Uses StringArrayHashMap (managed wrapper, insertion-order-preserving)
                // so that Object.keys on the result iterates in insertion order.
                let blk = self.next_label();
                let _map = format!("_grp_map_{}", blk);
                let _di = format!("_grp_di_{}", blk);
                let _e = format!("_grp_e_{}", blk);
                let _item = format!("_grp_item_{}", blk);
                let _key = format!("_grp_key_{}", blk);
                let _new = format!("_grp_new_{}", blk);
                self.write(&format!("{blk}: {{ var {0} = js_runtime.StringArrayHashMap(std.ArrayList(JsAny)).init(js_allocator.allocator()); errdefer {{ var {1} = {0}.iterator(); while ({1}.next()) |{2}| {{ {2}.value_ptr.deinit(js_allocator.allocator()); }} {0}.deinit(); }} ", _map, _di, _e));
                if let Some(items_arg) = args.first() {
                    self.write("for (");
                    self.emit_expr(items_arg);
                    self.write(&format!(".items) |{0}| {{ ", _item));
                    if args.len() >= 2 {
                        self.emit_group_by_callback(&args[1], &_item, &_key);
                    } else {
                        self.write(&format!("const {0} = {1}", _key, _item));
                    }
                    self.write(&format!("; if ({0}.getPtr({1})) |_grp_list| {{ _grp_list.append(js_allocator.allocator(), JsAny.from({2})) catch @panic(\"OOM\"); }} else {{ var {3}: std.ArrayList(JsAny) = .empty; {3}.append(js_allocator.allocator(), JsAny.from({2})) catch @panic(\"OOM\"); {0}.put({1}, {3}) catch @panic(\"OOM\"); }} }} ", _map, _key, _item, _new));
                }
                self.write(&format!("break :{blk} {}; }}", _map));
            }
            // ── Object.getOwnPropertyDescriptor — needs allocator prefix ──
            "getOwnPropertyDescriptor" => {
                self.write("js_object.getOwnPropertyDescriptor(js_allocator.allocator(), ");
                self.emit_inline_args(args);
                self.write(") catch @panic(\"OOM: Object.getOwnPropertyDescriptor\")");
            }
            // ── Default: js_object.method(args) ──
            _ => {
                self.emit_module_call("js_object", method, args);
            }
        }
    }

    /// Emit the callback parameter binding and key expression for Object.groupBy.
    /// Handles ArrowFn, Closure (all stmts + last as value), and fallback call.
    fn emit_group_by_callback(&mut self, callback: &IrExpr, item: &str, key: &str) {
        match callback {
            IrExpr::ArrowFn(arrow) => {
                let param_name = arrow
                    .params
                    .first()
                    .map(|p| p.name.zig_name.clone())
                    .unwrap_or_else(|| "_".to_string());
                self.write(&format!("const {} = {}; ", param_name, item));
                let stmts = &arrow.body.stmts;
                for stmt in stmts.iter().take(stmts.len().saturating_sub(1)) {
                    self.emit_stmt(stmt);
                }
                self.write(&format!("const {} = ", key));
                if let Some(stmt) = stmts.last() {
                    self.emit_stmt_value(stmt, item);
                }
            }
            IrExpr::Closure(closure) => {
                let param_name = closure
                    .fn_params
                    .first()
                    .map(|p| p.name.zig_name.clone())
                    .unwrap_or_else(|| "_".to_string());
                self.write(&format!("const {} = {}; ", param_name, item));
                let stmts = &closure.body.stmts;
                for stmt in stmts.iter().take(stmts.len().saturating_sub(1)) {
                    self.emit_stmt(stmt);
                }
                self.write(&format!("const {} = ", key));
                if let Some(stmt) = stmts.last() {
                    self.emit_stmt_value(stmt, item);
                }
            }
            _ => {
                self.write(&format!("const {} = ", key));
                self.emit_expr(callback);
                self.write(&format!("({})", item));
            }
        }
    }

    /// Extract the value expression from a Return or Expr statement, or emit a fallback.
    fn emit_stmt_value(&mut self, stmt: &crate::zigir::types::IrStmt, fallback: &str) {
        match stmt {
            crate::zigir::types::IrStmt::Return { value: Some(v) } => self.emit_expr(v),
            crate::zigir::types::IrStmt::Expr(e) => self.emit_expr(e),
            _ => self.write(fallback),
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
                } else if self.in_function && self.fn_can_throw {
                    self.write(") catch return error.JsThrow");
                } else {
                    self.write(") catch @panic(\"JSON.parse failed\")");
                }
            }
            "stringify" => {
                // Check if the first argument is a struct literal (ObjectLiteral) or
                // a variable of struct type — these need stringifyStruct which
                // accepts anytype and converts struct fields to JsAny.
                // For JsAny values, use the regular stringify function.
                let is_struct_arg = match args.first() {
                    Some(crate::zigir::types::IrExpr::ObjectLiteral(_)) => true,
                    Some(crate::zigir::types::IrExpr::Ident(ident)) => {
                        self.struct_var_names.contains(&ident.zig_name)
                    }
                    Some(crate::zigir::types::IrExpr::TypedIdent { ident, ty }) => {
                        // TypedIdent carries type info — if it's a Struct type, use stringifyStruct.
                        // Also check struct_var_names as a fallback for variables detected by emit_var_decl.
                        matches!(ty, ZigType::Struct(_))
                            || self.struct_var_names.contains(&ident.zig_name)
                    }
                    Some(crate::zigir::types::IrExpr::BuiltinCall(bc)) => {
                        bc.method == "spreadMerge"
                    }
                    _ => false,
                };
                let func_name = if is_struct_arg {
                    "stringifyStruct"
                } else {
                    "stringify"
                };
                self.write(&format!("js_json.{}(js_allocator.allocator(), ", func_name));
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
                if let Some(label) = &self.inside_try_block {
                    self.write(&format!(
                        ") catch |err| break :{} @as(anyerror!void, err)",
                        label
                    ));
                } else if self.in_function && self.fn_can_throw {
                    self.write(") catch return error.JsThrow");
                } else {
                    self.write(") catch @panic(\"OOM: JSON.stringify\")");
                }
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
                // digit counts, plus OOM). Use inside_try_block to propagate
                // errors to catch handler; otherwise @panic with RangeError msg.
                self.write(&format!("js_number.{}(js_allocator.allocator(), ", method));
                let mut first = true;
                if let Some(name) = obj {
                    self.write(name);
                    first = false;
                }
                for arg in args.iter() {
                    if !first {
                        self.write(", ");
                    }
                    first = false;
                    self.emit_expr(arg);
                }
                if let Some(label) = &self.inside_try_block {
                    self.write(&format!(
                        ") catch |err| break :{} @as(anyerror!void, err)",
                        label
                    ));
                } else {
                    self.write(") catch @panic(\"RangeError: Number method failed\")");
                }
            }
            // R8-NumberToString: js_number.toString(allocator, val, radix).
            // Zig runtime requires all three args (no Zig default-param
            // support); ECMA-262 21.1.3.7 says the default radix is 10.
            // The emitter always emits `, 10` when the JS call omits it,
            // matching the slice/substring/parseInt convention.
            "toString" => {
                self.write("js_number.toString(js_allocator.allocator(), ");
                let mut first = true;
                if let Some(name) = obj {
                    self.write(name);
                    first = false;
                }
                for arg in args.iter() {
                    if !first {
                        self.write(", ");
                    }
                    first = false;
                    self.emit_expr(arg);
                }
                if args.is_empty() {
                    if !first {
                        self.write(", 10");
                    } else {
                        self.write("10");
                    }
                }
                // Same fallible-call convention as toFixed/toExponential/
                // toPrecision above: toString returns ![]const u8 (RangeError
                // on radix outside 2..36, plus OOM). Route via inside_try_block
                // to propagate errors to catch handler.
                if let Some(label) = &self.inside_try_block {
                    self.write(&format!(
                        ") catch |err| break :{} @as(anyerror!void, err)",
                        label
                    ));
                } else {
                    self.write(") catch @panic(\"RangeError: Number method failed\")");
                }
            }
            "parseInt" => {
                // parseInt is a static method on Number (Number.parseInt(str)),
                // not an instance method. The `obj` (e.g. "Number") is the
                // constructor name, not a valid Zig value — ignore it.
                self.write("js_number.parseInt(");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
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
            // Symbol.keyFor returns ?[]const u8 — null when symbol not registered
            // via Symbol.for(). Wrap in JsAny to return undefined for unregistered symbols.
            "keyFor" => {
                let lbl = self.next_label();
                let _k = format!("_k_{}", lbl);
                self.write(&format!("({lbl}: {{ const {} = ", _k));
                self.emit_module_call("js_symbol", "symbolKeyFor", args);
                self.write(&format!(
                    "; break :{lbl} if ({}) |v| JsAny.fromString(v) else JsAny.fromUndefined(); }})",
                    _k
                ));
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
