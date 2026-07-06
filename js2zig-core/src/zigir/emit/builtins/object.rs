// zigir/emit/builtins/object.rs
// Object, JSON, Number, Symbol, Console builtin method emission.

use crate::zigir::emit::helpers::EmitterHelpers;
use crate::zigir::types::IrExpr;

use crate::zigir::emit::Emitter;

impl Emitter {
    pub(super) fn emit_object_builtin(
        &mut self,
        method: &str,
        args: &[crate::zigir::types::IrExpr],
    ) {
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
            // ── Object.getOwnPropertyDescriptor — needs allocator prefix ──
            "getOwnPropertyDescriptor" => {
                self.write("js_object.getOwnPropertyDescriptor(js_allocator.allocator(), ");
                self.emit_inline_args(args);
                self.write(")");
            }
            // ── Default: js_object.method(args) ──
            _ => {
                self.write(&format!("js_object.{}(", method));
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

    pub(super) fn emit_json_builtin(&mut self, method: &str, args: &[crate::zigir::types::IrExpr]) {
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

    pub(super) fn emit_number_builtin(
        &mut self,
        method: &str,
        obj: Option<&str>,
        args: &[crate::zigir::types::IrExpr],
    ) {
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
                self.write(&format!("js_number.{}(", method));
                self.emit_inline_args(args);
                self.write(")");
            }
        }
    }

    pub(super) fn emit_symbol_builtin(
        &mut self,
        method: &str,
        obj: Option<&str>,
        args: &[crate::zigir::types::IrExpr],
    ) {
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
                self.write(&format!("js_symbol.{}(", zig_method));
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

    pub(super) fn emit_console_builtin(
        &mut self,
        method: &str,
        args: &[crate::zigir::types::IrExpr],
    ) {
        self.write(&format!("js_console.{}(", method));
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            self.emit_expr(arg);
        }
        self.write(")");
    }
}
