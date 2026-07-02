// native_proto/codegen/expr.rs
// Expression-level code generation: expr, call, binary, template, array, object, builtin.

use super::Codegen;
use crate::native_proto::ZigType;
use crate::native_proto::builtins;
use oxc_ast::ast::*;
use oxc_span::GetSpan;

/// Escape a string for use in a Zig string literal.
/// Handles control characters (0x00-0x1F, 0x7F) by converting them to \\xNN hex escapes,
/// and standard escapes for \\, \", \n, \r, \t.
pub(crate) fn escape_zig_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 16);
    for byte in s.bytes() {
        match byte {
            b'\\' => result.push_str("\\\\"),
            b'"' => result.push_str("\\\""),
            b'\n' => result.push_str("\\n"),
            b'\r' => result.push_str("\\r"),
            b'\t' => result.push_str("\\t"),
            // Control characters and DEL — must use \\xNN escape in Zig
            c @ 0x00..=0x1F | c @ 0x7F..=0xFF => {
                result.push_str(&format!("\\x{:02X}", c));
            }
            _ => result.push(byte as char),
        }
    }
    result
}

// ── Expressions ─────────────────────────────────────

impl Codegen {
    /// Emit an expression that must produce a value (not a statement).
    /// Temporarily sets `in_expr_stmt = false` so that UpdateExpression
    /// (e.g., `i++`) emits a block expression instead of `i += 1`.
    /// Used for array indices, function arguments, and other value positions.
    pub(crate) fn emit_value_expr(&mut self, expr: &Expression) {
        let saved = self.in_expr_stmt;
        self.in_expr_stmt = false;
        self.emit_expr(expr);
        self.in_expr_stmt = saved;
    }

    pub(crate) fn emit_expr(&mut self, expr: &Expression) {
        match expr {
            Expression::NumericLiteral(n) => {
                // Zig considers `-0` ambiguous (could be integer 0 or float -0.0).
                // JS `-0` is always the float -0.0, so emit `-0.0` explicitly.
                if n.value == -0.0 && n.value.is_sign_negative() {
                    self.write("-0.0");
                } else {
                    self.write(&n.value.to_string());
                }
            }
            Expression::StringLiteral(s) => {
                self.write(&format!("\"{}\"", escape_zig_string(&s.value)));
            }
            Expression::BooleanLiteral(b) => {
                self.write(if b.value { "true" } else { "false" });
            }
            Expression::Identifier(id) => {
                let var_name = id.name.as_str();
                // 🔘 arguments object: not supported — generate @compileError
                if var_name == "arguments" {
                    self.compile_error(
                        id.span,
                        "arguments object is not supported. Use rest parameters (...args) instead.",
                    );
                    return;
                }
                // JS global constants: NaN, Infinity, undefined
                if var_name == "NaN" {
                    self.write("std.math.nan(f64)");
                    return;
                }
                if var_name == "Infinity" {
                    self.write("std.math.inf(f64)");
                    return;
                }
                if var_name == "undefined" {
                    self.write("JsAny.fromUndefined()");
                    return;
                }
                // Check if this identifier is a captured variable in the current closure.
                // If so, rewrite to self.var_name (value capture) or self.var_name.* (ref capture).
                if !self.current_captured.is_empty()
                    && let Some((_, _, is_mut)) = self
                        .current_captured
                        .iter()
                        .find(|(n, _, _)| n.as_str() == var_name)
                {
                    let safe_name = self.zig_safe_name(var_name);
                    if *is_mut {
                        self.write(&format!("self.{}.*", safe_name));
                    } else {
                        self.write(&format!("self.{}", safe_name));
                    }
                    return;
                }
                self.write(&self.zig_safe_name(var_name));
            }
            Expression::ThisExpression(te) => {
                // When inside a class method, `this` maps to `self`.
                if self.current_class.is_some() {
                    self.write("self");
                } else {
                    self.errors
                        .push("`this` used outside of a class method".to_string());
                    self.compile_error(te.span, "`this` used outside of a class method");
                }
            }
            Expression::BinaryExpression(be) => {
                self.emit_binary(be);
            }
            Expression::CallExpression(ce) => {
                self.emit_call(ce);
            }
            Expression::AssignmentExpression(ae) => {
                self.emit_assignment(ae);
            }
            Expression::UnaryExpression(ue) => {
                self.emit_unary(ue);
            }
            Expression::LogicalExpression(le) => {
                // Zig `and`/`or` require bool operands. Coerce non-bool operands
                // with `!= 0` (JS truthiness). This preserves JS short-circuit
                // semantics for if-conditions; see emit_expr_as_bool.
                let left_bool = self.expr_is_definitely_bool(&le.left);
                let right_bool = self.expr_is_definitely_bool(&le.right);
                self.write("(");
                if left_bool {
                    self.emit_expr(&le.left);
                } else {
                    self.emit_expr_as_bool(&le.left);
                }
                self.write(&format!(" {} ", Self::logical_op(le.operator)));
                if right_bool {
                    self.emit_expr(&le.right);
                } else {
                    self.emit_expr_as_bool(&le.right);
                }
                self.write(")");
            }
            Expression::ParenthesizedExpression(pe) => {
                self.write("(");
                self.emit_expr(&pe.expression);
                self.write(")");
            }
            Expression::ConditionalExpression(ce) => {
                self.emit_conditional(ce);
            }
            Expression::ArrayExpression(ae) => {
                self.emit_array(ae);
            }
            Expression::ObjectExpression(oe) => {
                self.emit_object(oe);
            }
            Expression::StaticMemberExpression(mem) => {
                // Check for Math constants
                if let Expression::Identifier(id) = &mem.object
                    && id.name.as_str() == "Math"
                {
                    match mem.property.name.as_str() {
                        "PI" => {
                            self.write("@as(f64, std.math.pi)");
                            return;
                        }
                        "E" => {
                            self.write("@as(f64, std.math.e)");
                            return;
                        }
                        "LN2" => {
                            self.write("@as(f64, std.math.ln2)");
                            return;
                        }
                        "LN10" => {
                            self.write("@as(f64, std.math.ln10)");
                            return;
                        }
                        "LOG2E" => {
                            self.write("@as(f64, std.math.log2e)");
                            return;
                        }
                        "LOG10E" => {
                            self.write("@as(f64, std.math.log10e)");
                            return;
                        }
                        "SQRT1_2" => {
                            self.write("@as(f64, std.math.sqrt1_2)");
                            return;
                        }
                        "SQRT2" => {
                            self.write("@as(f64, std.math.sqrt2)");
                            return;
                        }
                        _ => {}
                    }
                }
                // Check for Number constants
                if let Expression::Identifier(id) = &mem.object
                    && id.name.as_str() == "Number"
                {
                    match mem.property.name.as_str() {
                        "MAX_VALUE" => {
                            self.write("std.math.floatMax(f64)");
                            return;
                        }
                        "MIN_VALUE" => {
                            self.write("std.math.floatMin(f64)");
                            return;
                        }
                        "NaN" => {
                            self.write("std.math.nan(f64)");
                            return;
                        }
                        "NEGATIVE_INFINITY" => {
                            self.write("-std.math.inf(f64)");
                            return;
                        }
                        "POSITIVE_INFINITY" => {
                            self.write("std.math.inf(f64)");
                            return;
                        }
                        "EPSILON" => {
                            self.write("std.math.floatEps(f64)");
                            return;
                        }
                        "MAX_SAFE_INTEGER" => {
                            self.write("9007199254740991");
                            return;
                        }
                        "MIN_SAFE_INTEGER" => {
                            self.write("-9007199254740991");
                            return;
                        }
                        _ => {}
                    }
                }
                // Symbol well-known symbols (Symbol.iterator, Symbol.asyncIterator, etc.)
                if let Expression::Identifier(id) = &mem.object
                    && id.name.as_str() == "Symbol"
                {
                    match mem.property.name.as_str() {
                        "iterator" => {
                            self.write("js_symbol.symbolIterator()");
                            return;
                        }
                        "asyncIterator" => {
                            self.write("js_symbol.symbolAsyncIterator()");
                            return;
                        }
                        "hasInstance" => {
                            self.write("js_symbol.symbolHasInstance()");
                            return;
                        }
                        "isConcatSpreadable" => {
                            self.write("js_symbol.symbolIsConcatSpreadable()");
                            return;
                        }
                        "species" => {
                            self.write("js_symbol.symbolSpecies()");
                            return;
                        }
                        "toPrimitive" => {
                            self.write("js_symbol.symbolToPrimitive()");
                            return;
                        }
                        "toStringTag" => {
                            self.write("js_symbol.symbolToStringTag()");
                            return;
                        }
                        "unscopables" => {
                            self.write("js_symbol.symbolUnscopables()");
                            return;
                        }
                        "match" => {
                            self.write("js_symbol.symbolMatch()");
                            return;
                        }
                        "matchAll" => {
                            self.write("js_symbol.symbolMatchAll()");
                            return;
                        }
                        "replace" => {
                            self.write("js_symbol.symbolReplace()");
                            return;
                        }
                        "search" => {
                            self.write("js_symbol.symbolSearch()");
                            return;
                        }
                        "split" => {
                            self.write("js_symbol.symbolSplit()");
                            return;
                        }
                        "dispose" => {
                            self.write("js_symbol.symbolDispose()");
                            return;
                        }
                        _ => {}
                    }
                }
                // TypedArray .buffer / .byteLength / .byteOffset
                let prop_name = mem.property.name.as_str();
                if let Expression::Identifier(id) = &mem.object {
                    let ta_type = self.typedarray_vars.get(id.name.as_str()).cloned();
                    if let Some(ta_type) = ta_type {
                        match prop_name {
                            "buffer" => {
                                self.write(&format!(
                                    "js_runtime.js_typedarray.buffer{}({})",
                                    ta_type, id.name
                                ));
                                return;
                            }
                            "byteLength" => {
                                self.write(&format!(
                                    "js_runtime.js_typedarray.byteLength{}({})",
                                    ta_type, id.name
                                ));
                                return;
                            }
                            "byteOffset" => {
                                self.write("js_runtime.js_typedarray.byteOffset()");
                                return;
                            }
                            _ => {}
                        }
                    }
                }
                self.emit_expr(&mem.object);
                // Map/Set .size is a method call, not a field access
                let is_map_set_size_method = prop_name == "size"
                    && if let Expression::Identifier(id) = &mem.object {
                        self.type_info.var_types.get(id.name.as_str()).is_some_and(
                            |t| matches!(t, ZigType::NamedStruct(s) if s == "Map" || s == "Set"),
                        )
                    } else {
                        false
                    };
                if is_map_set_size_method {
                    self.write(".size()");
                } else if prop_name == "length" {
                    // .length → .items.len for ArrayList, .len for slices/strings
                    if let Expression::Identifier(id) = &mem.object {
                        if self
                            .type_info
                            .var_types
                            .get(id.name.as_str())
                            .is_some_and(|t| matches!(t, ZigType::ArrayList(_)))
                        {
                            self.write(".items.len");
                        } else {
                            self.write(".len");
                        }
                    } else {
                        self.write(".len");
                    }
                } else {
                    self.write(".");
                    self.write(prop_name);
                }
            }
            Expression::ComputedMemberExpression(mem) => {
                // Check if this is array indexing (numeric literal) or dynamic property access.
                match &mem.expression {
                    Expression::NumericLiteral(n) => {
                        // Array indexing with numeric literal: arr[0]
                        let idx = n.value as i64;
                        let obj_type = self.infer_expr_type(&mem.object);
                        match obj_type {
                            Some(ZigType::ArrayList(_)) => {
                                // ArrayList: arr.items[0]
                                self.emit_expr(&mem.object);
                                self.write(&format!(".items[{}]", idx));
                            }
                            _ => {
                                // Other types: arr[0] (JsAny, unknown, etc.)
                                self.emit_expr(&mem.object);
                                self.write(&format!("[{}]", idx));
                            }
                        }
                    }
                    Expression::StringLiteral(s) => {
                        // obj["key"] → dispatch based on obj type
                        let key = s.value.as_str();
                        let obj_type = self.infer_expr_type(&mem.object);
                        match obj_type {
                            Some(ZigType::Struct(_)) => {
                                // Anonymous struct: @field(obj, "key")
                                self.write("@field(");
                                self.emit_expr(&mem.object);
                                self.write(&format!(", \"{}\")", key));
                            }
                            Some(ZigType::NamedStruct(ref name)) if name == "Map" => {
                                // Map: obj.get("key") returns JsAny (undefined if not found)
                                self.emit_expr(&mem.object);
                                self.write(&format!(".get(\"{}\")", key));
                            }
                            Some(ZigType::NamedStruct(_)) => {
                                // Named struct (host/class/JSDoc): @field(obj, "key")
                                self.write("@field(");
                                self.emit_expr(&mem.object);
                                self.write(&format!(", \"{}\")", key));
                            }
                            _ => {
                                // JsAny or unknown: obj.get("key") (static key, no alloc)
                                self.emit_expr(&mem.object);
                                self.write(&format!(".get(\"{}\")", key));
                            }
                        }
                    }
                    _ => {
                        // obj[expr] → dynamic key lookup
                        let obj_type = self.infer_expr_type(&mem.object);
                        match obj_type {
                            Some(ZigType::JsAny) | Some(ZigType::Anytype) => {
                                // JsAny.getByKey(key, alloc)
                                self.emit_expr(&mem.object);
                                self.write(".getByKey(");
                                self.emit_value_expr(&mem.expression);
                                self.write(", js_allocator.getAllocator())");
                            }
                            Some(ZigType::NamedStruct(ref name)) if name == "Map" => {
                                // Map: obj.get(key) returns JsAny (undefined if not found)
                                self.emit_expr(&mem.object);
                                self.write(".get(");
                                self.emit_value_expr(&mem.expression);
                                self.write(")");
                            }
                            Some(ZigType::ArrayList(_)) => {
                                // ArrayList: arr.items[expr]
                                // Zig requires usize for array indexing; convert i64 if needed.
                                let idx_type = self.infer_expr_type(&mem.expression);
                                self.emit_expr(&mem.object);
                                self.write(".items[");
                                if idx_type == Some(ZigType::I64)
                                    || idx_type == Some(ZigType::BigInt)
                                {
                                    self.write("@as(usize, @intCast(");
                                    self.emit_value_expr(&mem.expression);
                                    self.write("))");
                                } else {
                                    self.emit_value_expr(&mem.expression);
                                }
                                self.write("]");
                            }
                            Some(ZigType::Struct(_)) | Some(ZigType::NamedStruct(_)) => {
                                // Struct: @field(obj, expr) — expr must be comptime-known
                                // (e.g., for-in loop variable unrolled to string literal)
                                self.write("@field(");
                                self.emit_expr(&mem.object);
                                self.write(", ");
                                self.emit_value_expr(&mem.expression);
                                self.write(")");
                            }
                            None => {
                                // Unknown type → fallback to JsAny.getByKey
                                self.emit_expr(&mem.object);
                                self.write(".getByKey(");
                                self.emit_value_expr(&mem.expression);
                                self.write(", js_allocator.getAllocator())");
                            }
                            _ => {
                                // Other non-indexable types → compile error
                                self.errors.push(
                                    "Dynamic property access on non-object type. \
                                     Use static property access (obj.prop) for structs."
                                        .to_string(),
                                );
                                self.write(
                                    "@compileError(\"dynamic property access on non-object type\")",
                                );
                            }
                        }
                    }
                }
            }
            Expression::AwaitExpression(ae) => {
                let task_var = format!("_t{}", self.task_counter);
                self.task_counter += 1;

                // emit: (blk_N: { var _tN = io.async(fn_async, .{io, args...}); defer _ = _tN.cancel(io) catch undefined; break :blk_N try _tN.await(io); })
                let blk = self.next_label();
                self.write(&format!("({}: {{\n", blk));
                self.indent += 1;

                self.write_indent();
                self.write(&format!("var {} = io.async(", task_var));

                match &ae.argument {
                    Expression::CallExpression(call) => {
                        // Check if this is an async host function → use host.{name}_async wrapper
                        if let Expression::Identifier(id) = &call.callee {
                            let name = id.name.as_str();
                            if self.async_host_fns.contains(name) {
                                self.write(&format!("host.{}_async", name));
                            } else {
                                self.emit_expr(&call.callee);
                            }
                        } else {
                            self.emit_expr(&call.callee);
                        }
                        self.write(", .{ io");
                        for arg in &call.arguments {
                            self.write(", ");
                            if let Some(expr) = arg.as_expression() {
                                self.emit_expr(expr);
                            } else {
                                self.write("undefined");
                            }
                        }
                        self.write(" }");
                    }
                    _ => {
                        self.emit_expr(&ae.argument);
                        self.write(", .{ io }");
                    }
                }

                self.write(");\n");

                self.write_indent();
                self.write(&format!(
                    "defer _ = {}.cancel(io) catch undefined;\n",
                    task_var
                ));

                self.write_indent();
                self.write(&format!("break :{} try {}.await(io);\n", blk, task_var));

                self.indent -= 1;
                self.write_indent();
                self.write("})");
            }
            Expression::NewExpression(ne) => {
                // Check if this is new Int32Array(...) or new Uint8Array(...)
                if let Expression::Identifier(id) = &ne.callee {
                    let obj_name = id.name.as_str();

                    // TypedArray constructors
                    if let Some(ta) = Self::typedarray_info(obj_name) {
                        self.write(ta.func);
                        self.write("(");
                        if let Some(first_arg) = ne.arguments.first()
                            && let Some(expr) = first_arg.as_expression()
                            && let Expression::ArrayExpression(ae) = expr
                        {
                            self.write(ta.array_prefix);
                            self.emit_comma_separated_array_elements(&ae.elements);
                            self.write("}");
                        }
                        self.write(")");
                        return;
                    }

                    // Error constructors (used with throw statements; value is discarded)
                    if Self::is_error_constructor(obj_name) {
                        // new Error(msg) → {} (throw immediately follows with error.JsThrow)
                        self.write("{}");
                        return;
                    }

                    // Wrapper constructors — emit the argument value directly
                    if obj_name == "String" {
                        // new String(x) → x (wrapper semantics rarely needed)
                        if let Some(first_arg) = ne.arguments.first()
                            && let Some(expr) = first_arg.as_expression()
                        {
                            self.emit_expr(expr);
                        } else {
                            self.write("\"\"");
                        }
                        return;
                    } else if obj_name == "Number" {
                        // new Number(x) → x
                        if let Some(first_arg) = ne.arguments.first()
                            && let Some(expr) = first_arg.as_expression()
                        {
                            self.emit_expr(expr);
                        } else {
                            self.write("0");
                        }
                        return;
                    } else if obj_name == "Boolean" {
                        // new Boolean(x) → x
                        if let Some(first_arg) = ne.arguments.first()
                            && let Some(expr) = first_arg.as_expression()
                        {
                            self.emit_expr(expr);
                        } else {
                            self.write("false");
                        }
                        return;
                    }

                    // Known unsupported constructors — specific error messages
                    if matches!(
                        obj_name,
                        "ArrayBuffer"
                            | "SharedArrayBuffer"
                            | "Function"
                            | "Promise"
                            | "WeakMap"
                            | "WeakSet"
                            | "DataView"
                    ) {
                        let msg = match obj_name {
                            "Function" => "Function constructor is not supported in Zig",
                            "Promise" => "Promise constructor is not supported in Zig",
                            other => other, // will be used in format! below
                        };
                        let full_msg = if obj_name == "Function" || obj_name == "Promise" {
                            msg.to_string()
                        } else {
                            format!("{obj_name} is not supported in Zig")
                        };
                        self.errors.push(full_msg.clone());
                        self.compile_error(ne.span, &full_msg);
                        return;
                    }

                    if obj_name == "Map" {
                        // new Map() → js_collections.JsMap.init(js_allocator.getAllocator())
                        self.write("js_collections.JsMap.init(js_allocator.getAllocator())");
                        return;
                    } else if obj_name == "Set" {
                        // new Set() → js_collections.JsSet.init(js_allocator.getAllocator())
                        self.write("js_collections.JsSet.init(js_allocator.getAllocator())");
                        return;
                    } else if obj_name == "Date" {
                        // new Date() → js_date.JsDate.init()
                        // new Date(millis) → js_date.JsDate.fromMillis(millis)
                        // new Date(str) → js_date.JsDate.fromMillis(js_date.parse(str))
                        // new Date(y, m, d?, h?, min?, s?, ms?) → js_date.JsDate.fromComponents(y, m, d, h, min, s, ms)
                        if ne.arguments.is_empty() {
                            self.write("js_date.JsDate.init()");
                        } else if ne.arguments.len() >= 2 {
                            // Multi-arg constructor with default padding
                            // JS defaults: day=1, hours/minutes/seconds/ms=0
                            self.write("js_date.JsDate.fromComponents(");
                            for (i, arg) in ne.arguments.iter().enumerate() {
                                if i > 0 {
                                    self.write(", ");
                                }
                                self.emit_expr_arg(arg);
                            }
                            // Pad remaining args with defaults
                            // Position: 0=year, 1=month, 2=day, 3=hour, 4=min, 5=sec, 6=ms
                            const DEFAULTS: [&str; 5] = ["1", "0", "0", "0", "0"];
                            let emitted = ne.arguments.len();
                            for i in emitted..7 {
                                self.write(", ");
                                self.write(DEFAULTS[i - 2]);
                            }
                            self.write(")");
                        } else if let Some(first_arg) = ne.arguments.first()
                            && let Some(expr) = first_arg.as_expression()
                        {
                            // Detect if argument is a string (literal or inferred type)
                            let is_string = match expr {
                                Expression::StringLiteral(_) => true,
                                Expression::Identifier(id) => self
                                    .type_info
                                    .var_types
                                    .get(id.name.as_str())
                                    .is_some_and(|t| matches!(t, ZigType::Str)),
                                _ => false,
                            };
                            if is_string {
                                self.write("js_date.JsDate.fromMillis(js_date.parse(");
                                self.emit_expr(expr);
                                self.write("))");
                            } else {
                                self.write("js_date.JsDate.fromMillis(");
                                self.emit_expr(expr);
                                self.write(")");
                            }
                        } else {
                            self.write("js_date.JsDate.init()");
                        }
                        return;
                    } else if obj_name == "RegExp" {
                        // new RegExp(pattern) → try js_regexp.JsRegExp.init(alloc, pattern)
                        self.write("try js_regexp.JsRegExp.init(js_allocator.getAllocator(), ");
                        if let Some(first_arg) = ne.arguments.first()
                            && let Some(expr) = first_arg.as_expression()
                        {
                            self.emit_expr(expr);
                        } else {
                            // new RegExp() with no args → default empty pattern
                            self.write("\"\"");
                        }
                        self.write(")");
                        return;
                    } else if self.class_names.contains(obj_name) {
                        // new ClassName(args) → ClassName.init(args)
                        self.write(&format!("{}.init(", obj_name));
                        for (i, arg) in ne.arguments.iter().enumerate() {
                            if i > 0 {
                                self.write(", ");
                            }
                            self.emit_expr_arg(arg);
                        }
                        self.write(")");
                        return;
                    }
                }
                // Unsupported NewExpression
                self.errors.push(
                    "Unsupported NewExpression (supported: Int32Array, Uint8Array, Float64Array, Map, Set, Date, RegExp, class names)"
                        .to_string(),
                );
                self.compile_error(ne.span, "Unsupported NewExpression");
            }
            Expression::TemplateLiteral(tpl) => self.emit_template_literal(tpl),
            Expression::UpdateExpression(ue) => {
                // i++/i-- → i += 1 / i -= 1
                let op_str = match ue.operator {
                    UpdateOperator::Increment => " += 1",
                    UpdateOperator::Decrement => " -= 1",
                };
                match &ue.argument {
                    SimpleAssignmentTarget::AssignmentTargetIdentifier(id) => {
                        let name = self.zig_safe_name(id.name.as_str());
                        if self.in_expr_stmt {
                            // Statement context (for-loop update, expr statement):
                            // emit `i += 1` directly (void, no return value needed)
                            self.write(&name);
                            self.write(op_str);
                        } else {
                            // Expression context (array index, function arg, etc.):
                            // emit a block expression that returns the appropriate value
                            let lbl = self.next_label();
                            if ue.prefix {
                                // ++i → (blk_N: { i += 1; break :blk_N i; })
                                self.write(&format!("({}: {{ ", lbl));
                                self.write(&name);
                                self.write(op_str);
                                self.write(&format!("; break :{} {}; }})", lbl, name));
                            } else {
                                // i++ → (blk_N: { const _old = i; i += 1; break :blk_N _old; })
                                let tmp = format!("_old_{}", lbl);
                                self.write(&format!("({}: {{ const {} = {}; ", lbl, tmp, name));
                                self.write(&name);
                                self.write(op_str);
                                self.write(&format!("; break :{} {}; }})", lbl, tmp));
                            }
                        }
                    }
                    _ => {
                        self.errors.push(
                            "Unsupported UpdateExpression target (only simple identifiers)"
                                .to_string(),
                        );
                        self.compile_error(ue.span, "Unsupported UpdateExpression target");
                    }
                }
            }
            Expression::ChainExpression(chain) => {
                self.emit_optional_chain(chain);
            }
            Expression::PrivateFieldExpression(mem) => {
                // `obj.#field` → `obj.field` (strip #, no pub needed in Zig)
                self.emit_expr(&mem.object);
                self.write(".");
                self.write(mem.field.name.as_str());
            }
            Expression::NullLiteral(_) => {
                // JS `null` → JsAny.fromNull()
                self.write("JsAny.fromNull()");
            }
            Expression::RegExpLiteral(re) => {
                // JS regexp literal `/pattern/flags` → try js_regexp.JsRegExp.init(alloc, pattern)
                self.write("try js_regexp.JsRegExp.init(js_allocator.getAllocator(), ");
                let pattern = re.regex.pattern.text.as_str();
                let escaped = pattern.replace("\\", "\\\\").replace("\"", "\\\"");
                self.write(&format!("\"{}\"", escaped));
                self.write(")");
            }
            Expression::BigIntLiteral(bigint) => {
                // BigInt literal: 9n → js_bigint.JsBigInt.init(alloc, "9")
                // bigint.value is the decimal string without trailing `n`
                let s = bigint.value.as_str();
                self.write(&format!(
                    "js_bigint.JsBigInt.init(js_allocator.getAllocator(), \"{}\") catch @panic(\"OOM: BigInt init\")",
                    s
                ));
            }
            Expression::ArrowFunctionExpression(arrow) => {
                // Arrow function as expression value — generate definition deferred
                // so it appears before the current statement.
                let saved_output = std::mem::take(&mut self.output);
                let saved_indent = self.indent;
                let fn_name = self.emit_arrow_function(arrow);
                // emit_arrow_function wrote the fn definition to (now-saved) output,
                // adjusted indent in the process. Restore and capture.
                let fn_def = std::mem::take(&mut self.output);
                self.output = saved_output;
                self.indent = saved_indent;
                self.pending_expr_fns.push(fn_def);
                self.write(&fn_name);
            }
            Expression::FunctionExpression(func) => {
                // Function expression as value — similar to arrow, defer definition.
                let saved_output = std::mem::take(&mut self.output);
                let saved_indent = self.indent;
                let fn_name = self.emit_fn_expr(func);
                let fn_def = std::mem::take(&mut self.output);
                self.output = saved_output;
                self.indent = saved_indent;
                self.pending_expr_fns.push(fn_def);
                self.write(&fn_name);
            }
            Expression::TaggedTemplateExpression(tte) => {
                // Tagged template literals (e.g. tag`str`) are not supported.
                self.compile_error(
                    tte.span,
                    "Tagged template literals (tag`str`) are not supported in native_proto mode",
                );
            }
            other => {
                // Unsupported expression type
                self.errors.push(format!(
                    "Unsupported expression type: {:?}",
                    std::mem::discriminant(other)
                ));
                self.compile_error(GetSpan::span(other), "Unsupported expression type");
            }
        }
    }
}

// ── Binary / Call / Assignment / Unary / Conditional / Array ──

/// TypedArray constructor codegen info.
struct TypedArrayInfo {
    func: &'static str,
    array_prefix: &'static str,
}

impl Codegen {
    // Binary expression with string-concat special case

    /// Recursively collect all operands in a string concatenation chain.
    /// Takes &BinaryExpression directly (avoids type wrapping issues).
    pub(crate) fn collect_concat_from_be<'a>(
        be: &'a BinaryExpression<'a>,
        out: &mut Vec<&'a Expression<'a>>,
    ) {
        // Left side
        if let Expression::BinaryExpression(ref left_be) = be.left {
            if left_be.operator == BinaryOperator::Addition {
                Self::collect_concat_from_be(left_be, out);
            } else {
                out.push(&be.left);
            }
        } else {
            out.push(&be.left);
        }

        // Right side
        if let Expression::BinaryExpression(ref right_be) = be.right {
            if right_be.operator == BinaryOperator::Addition {
                Self::collect_concat_from_be(right_be, out);
            } else {
                out.push(&be.right);
            }
        } else {
            out.push(&be.right);
        }
    }

    /// Emit an expression converted to f64.
    /// For i64 variables, uses @floatFromInt (Zig doesn't allow @as(f64, i64_var)).
    /// For comptime_int literals, uses @as(f64, expr).
    /// For f64 expressions, emits directly without conversion.
    fn emit_float_conversion(&mut self, expr: &Expression, ty: Option<ZigType>) {
        match ty {
            Some(ZigType::F64) => {
                self.emit_expr_for_arithmetic(expr);
            }
            Some(ZigType::I64) | Some(ZigType::BigInt) => {
                self.write("@as(f64, @floatFromInt(");
                self.emit_expr_for_arithmetic(expr);
                self.write("))");
            }
            _ => {
                // comptime_int or unknown — @as(f64, expr) works for comptime_int
                self.write("@as(f64, ");
                self.emit_expr_for_arithmetic(expr);
                self.write(")");
            }
        }
    }

    /// Emit an expression that will be used in an arithmetic context (+, -, *, /, %, **
    /// and bitwise operators). Boolean literals are coerced to integer via `@intFromBool`
    /// because Zig forbids `bool + comptime_int`.
    fn emit_expr_for_arithmetic(&mut self, expr: &Expression) {
        if matches!(expr, Expression::BooleanLiteral(_)) {
            self.write("@intFromBool(");
            self.emit_expr(expr);
            self.write(")");
        } else {
            self.emit_expr(expr);
        }
    }

    /// Emit a string concatenation using std.fmt.allocPrint (Zig 0.16.0: ++ requires comptime-known slices).
    fn emit_string_concat(&mut self, be: &BinaryExpression) {
        let mut operands: Vec<&Expression> = Vec::new();
        Self::collect_concat_from_be(be, &mut operands);

        // Build format string and arguments.
        // For string literals: include verbatim (escape { and }).
        // For expressions: use {s} placeholder, collect expression code as argument.
        let mut fmt = String::new();
        let mut args: Vec<String> = Vec::new();

        for op in &operands {
            if let Expression::StringLiteral(sl) = op {
                // Escape for a Zig format string literal:
                // backslash, double-quote, and {/} must be escaped.
                for ch in sl.value.chars() {
                    match ch {
                        '\\' => fmt.push_str("\\\\"),
                        '"' => fmt.push_str("\\\""),
                        '\n' => fmt.push_str("\\n"),
                        '\r' => fmt.push_str("\\r"),
                        '\t' => fmt.push_str("\\t"),
                        '{' => fmt.push_str("{{"),
                        '}' => fmt.push_str("}}"),
                        c => fmt.push(c),
                    }
                }
            } else {
                // Pick format specifier based on inferred type (match emit_template_literal logic).
                // ConditionalExpression: use {s} only if both branches are strings
                // Unwrap ParenthesizedExpression first (oxc wraps ternary in parens in concat)
                let spec_expr = match op {
                    Expression::ParenthesizedExpression(pe) => &pe.expression,
                    other => other,
                };
                let placeholder = match spec_expr {
                    Expression::ConditionalExpression(ce) => {
                        // Check if both branches are definitely strings
                        let cons_str = match &ce.consequent {
                            Expression::StringLiteral(_) => true,
                            _ => self.expr_is_string(&ce.consequent),
                        };
                        let alt_str = match &ce.alternate {
                            Expression::StringLiteral(_) => true,
                            _ => self.expr_is_string(&ce.alternate),
                        };
                        if cons_str && alt_str {
                            "{s}"
                        } else {
                            // Fallback to infer_expr_type
                            match self.infer_expr_type(spec_expr) {
                                Some(ZigType::Str) => "{s}",
                                Some(ZigType::I64) | Some(ZigType::F64) => "{d}",
                                Some(ZigType::Bool) => "{}",
                                _ => "{}",
                            }
                        }
                    }
                    _ => {
                        if self.expr_is_string(spec_expr) {
                            "{s}"
                        } else {
                            match self.infer_expr_type(spec_expr) {
                                Some(ZigType::Str) => "{s}",
                                Some(ZigType::I64) | Some(ZigType::F64) => "{d}",
                                Some(ZigType::Bool) => "{}",
                                _ => "{}",
                            }
                        }
                    }
                };
                fmt.push_str(placeholder);
                let arg_str = self.emit_expr_to_string(op);
                args.push(arg_str);
            }
        }

        // Generate: std.fmt.allocPrint(js_allocator.getAllocator(), "fmt", .{args}) catch @panic("OOM: template literal allocPrint")
        self.emit_format_string(&fmt, &args);
    }

    /// Emit a template literal `\`a=${x}\`` using std.fmt.allocPrint.
    /// Text segments form the format string (with `{`/`}` doubled and special
    /// chars escaped for a Zig string literal). Each interpolation picks a
    /// placeholder from the inferred type: Str→{s}, I64/F64→{d}, Bool→{},
    /// otherwise expr_is_string ? {s} : {}. Pure-text templates (no
    /// interpolation) degrade to a plain string literal (no allocation).
    /// Allocates from the global arena via js_allocator.getAllocator().
    fn emit_template_literal(&mut self, tpl: &TemplateLiteral) {
        let mut fmt = String::new();
        let mut args: Vec<String> = Vec::new();

        for (i, quasi) in tpl.quasis.iter().enumerate() {
            // Text segment: prefer cooked (JS escapes resolved), fallback to raw.
            let text: String = quasi
                .value
                .cooked
                .as_ref()
                .map(|c| c.as_str().to_string())
                .unwrap_or_else(|| quasi.value.raw.as_str().to_string());
            // Escape for a Zig string literal that is also a fmt template.
            for ch in text.chars() {
                match ch {
                    '\\' => fmt.push_str("\\\\"),
                    '"' => fmt.push_str("\\\""),
                    '\n' => fmt.push_str("\\n"),
                    '\r' => fmt.push_str("\\r"),
                    '\t' => fmt.push_str("\\t"),
                    '{' => fmt.push_str("{{"),
                    '}' => fmt.push_str("}}"),
                    c => fmt.push(c),
                }
            }

            // Interpolation following this text segment (if any).
            if i < tpl.expressions.len() {
                let expr = &tpl.expressions[i];
                let placeholder = match self.infer_expr_type(expr) {
                    Some(ZigType::Str) => "{s}",
                    Some(ZigType::I64) | Some(ZigType::F64) => "{d}",
                    Some(ZigType::Bool) => "{}",
                    _ => {
                        if self.expr_is_string(expr) {
                            "{s}"
                        } else {
                            "{}"
                        }
                    }
                };
                fmt.push_str(placeholder);
                let arg_str = self.emit_expr_to_string(expr);
                args.push(arg_str);
            }
        }

        self.emit_format_string(&fmt, &args);
    }

    fn emit_binary(&mut self, be: &BinaryExpression) {
        // Sub-expressions of a binary expression are always in expression position,
        // never in expression-statement position. Save/restore in_expr_stmt so that
        // void/delete/assignment sub-expressions don't incorrectly inherit the
        // statement-level flag from the enclosing ExpressionStatement.
        let saved_expr_stmt = self.in_expr_stmt;
        self.in_expr_stmt = false;

        // Object(BigInt) wrapper creates a BigInt wrapper object.
        // In JS: Object(0n) === 0n → false, Object(0n) === Object(0n) → false
        // (different objects). For ===/!== we can emit compile-time constants.
        let left_is_obj_bigint = self.expr_is_object_bigint(&be.left);
        let right_is_obj_bigint = self.expr_is_object_bigint(&be.right);
        if (left_is_obj_bigint || right_is_obj_bigint)
            && matches!(
                be.operator,
                BinaryOperator::StrictEquality | BinaryOperator::StrictInequality
            )
        {
            self.write(if be.operator == BinaryOperator::StrictEquality {
                "false"
            } else {
                "true"
            });
            return;
        }

        // Check for BigInt operations FIRST (before string check).
        // Object(BigInt) is treated as BigInt for non-strict comparisons
        // and arithmetic (e.g. Object(0n) > 0n coerces to BigInt comparison).
        let left_is_bigint = self.expr_is_bigint(&be.left) || left_is_obj_bigint;
        let right_is_bigint = self.expr_is_bigint(&be.right) || right_is_obj_bigint;
        if left_is_bigint || right_is_bigint {
            if left_is_bigint && right_is_bigint {
                self.emit_bigint_binary(be);
            } else if be.operator == BinaryOperator::Addition {
                // String + BigInt → string concatenation in JS.
                // Use JsBigInt.toString() to convert the BigInt side.
                let other_is_str = if left_is_bigint {
                    self.expr_is_string(&be.right)
                } else {
                    self.expr_is_string(&be.left)
                };
                if other_is_str {
                    // Build format string and args in left-to-right order.
                    let mut fmt = String::new();
                    let mut concat_args: Vec<String> = Vec::new();
                    let sides: [(&Expression, bool); 2] =
                        [(&be.left, left_is_bigint), (&be.right, right_is_bigint)];
                    for (side_expr, is_bigint) in &sides {
                        if *is_bigint {
                            fmt.push_str("{s}");
                            let bigint_code = self.emit_expr_to_string(side_expr);
                            concat_args.push(format!(
                                "({}).toString(js_allocator.getAllocator()) catch @panic(\"OOM: BigInt toString\")",
                                bigint_code
                            ));
                        } else if let Expression::StringLiteral(sl) = side_expr {
                            for ch in sl.value.chars() {
                                match ch {
                                    '\\' => fmt.push_str("\\\\"),
                                    '"' => fmt.push_str("\\\""),
                                    '\n' => fmt.push_str("\\n"),
                                    '\r' => fmt.push_str("\\r"),
                                    '\t' => fmt.push_str("\\t"),
                                    '{' => fmt.push_str("{{"),
                                    '}' => fmt.push_str("}}"),
                                    c => fmt.push(c),
                                }
                            }
                        } else {
                            fmt.push_str("{s}");
                            concat_args.push(self.emit_expr_to_string(side_expr));
                        }
                    }
                    self.emit_format_string(&fmt, &concat_args);
                } else {
                    // BigInt + non-string in addition → TypeError in JS.
                    self.write("if (true) { return; } else {}");
                }
            } else if Self::is_comparison_operator(be.operator) {
                // BigInt-to-number comparison: valid in JS, not a TypeError.
                // Convert BigInt side to f64 and compare numerically.
                // === with mixed types is always false, !== always true.
                //
                // But if the non-BigInt side is a string, we can't generate
                // valid Zig code (string-to-number conversion is required per JS
                // spec). Fall back to the panic for these edge cases.
                let other_is_str = if left_is_bigint {
                    self.expr_is_string(&be.right)
                } else {
                    self.expr_is_string(&be.left)
                };
                if other_is_str {
                    // String/BigInt comparison: parse string to f64,
                    // convert BigInt to f64, compare numerically.
                    self.emit_bigint_string_compare(be, left_is_bigint);
                } else {
                    self.emit_bigint_mixed_compare(be, left_is_bigint);
                }
            } else {
                // JS throws TypeError at runtime when mixing BigInt with other
                // types in arithmetic operations (+, -, *, /, %, **).
                // Use if-else expression so that `return;` does not make
                // subsequent statements unreachable at compile time.
                self.write("if (true) { return; } else {}");
            }
            return;
        }

        // Check if either operand is a string type
        let left_is_string = self.expr_is_string(&be.left);
        let right_is_string = self.expr_is_string(&be.right);

        if be.operator == BinaryOperator::Addition && (left_is_string || right_is_string) {
            // Use std.fmt.allocPrint for runtime string concatenation
            // (Zig 0.16.0: ++ requires comptime-known slices)
            self.emit_string_concat(be);
        } else if (be.operator == BinaryOperator::Equality
            || be.operator == BinaryOperator::StrictEquality)
            && left_is_string
            && right_is_string
        {
            // String equality: use std.mem.eql(u8, a, b)
            self.write("std.mem.eql(u8, ");
            self.emit_expr(&be.left);
            self.write(", ");
            self.emit_expr(&be.right);
            self.write(")");
        } else if (be.operator == BinaryOperator::Inequality
            || be.operator == BinaryOperator::StrictInequality)
            && left_is_string
            && right_is_string
        {
            // String inequality: !std.mem.eql(u8, a, b)
            self.write("!std.mem.eql(u8, ");
            self.emit_expr(&be.left);
            self.write(", ");
            self.emit_expr(&be.right);
            self.write(")");
        } else if left_is_string && right_is_string && Self::is_comparison_operator(be.operator) {
            // String lexical comparison: use std.mem.order
            self.write("std.mem.order(u8, ");
            self.emit_expr(&be.left);
            self.write(", ");
            self.emit_expr(&be.right);
            self.write(") ");
            self.write(match be.operator {
                BinaryOperator::LessThan => "== .lt",
                BinaryOperator::LessEqualThan => "!= .gt",
                BinaryOperator::GreaterThan => "== .gt",
                BinaryOperator::GreaterEqualThan => "!= .lt",
                _ => unreachable!("only order comparisons reach here"),
            });
        } else if (be.operator == BinaryOperator::Equality
            || be.operator == BinaryOperator::StrictEquality
            || be.operator == BinaryOperator::Inequality
            || be.operator == BinaryOperator::StrictInequality)
            && (left_is_string || right_is_string)
        {
            // Mixed string/non-string comparison: one side is string, other is not.
            // Use JsAny comparison for type-safe Zig code (JS allows cross-type
            // comparison like `3 != "3"` → false, `"1" != 1` → false).
            // NOTE: Must compute actual JsAny status — strings are NOT JsAny,
            // so we can't pass left_is_string/right_is_string as JsAny flags.
            let lt = self.infer_expr_type(&be.left);
            let rt = self.infer_expr_type(&be.right);
            self.emit_jsany_comparison(be, lt == Some(ZigType::JsAny), rt == Some(ZigType::JsAny));
        } else if be.operator == BinaryOperator::Division {
            let left_type = self.infer_expr_type(&be.left);
            let right_type = self.infer_expr_type(&be.right);
            if left_type == Some(ZigType::F64) || right_type == Some(ZigType::F64) {
                // Float division: use `/` operator (JS `/` always returns float)
                self.write("(");
                self.emit_expr_for_arithmetic(&be.left);
                self.write(" / ");
                self.emit_expr_for_arithmetic(&be.right);
                self.write(")");
            } else {
                // Integer division: use @divTrunc
                self.write("@divTrunc(");
                self.emit_expr_for_arithmetic(&be.left);
                self.write(", ");
                self.emit_expr_for_arithmetic(&be.right);
                self.write(")");
            }
        } else if be.operator == BinaryOperator::Remainder {
            let left_type = self.infer_expr_type(&be.left);
            let right_type = self.infer_expr_type(&be.right);
            if left_type == Some(ZigType::F64) || right_type == Some(ZigType::F64) {
                // Float remainder: use `%` operator
                self.write("(");
                self.emit_expr_for_arithmetic(&be.left);
                self.write(" % ");
                self.emit_expr_for_arithmetic(&be.right);
                self.write(")");
            } else {
                // Integer remainder: use @rem
                self.write("@rem(");
                self.emit_expr_for_arithmetic(&be.left);
                self.write(", ");
                self.emit_expr_for_arithmetic(&be.right);
                self.write(")");
            }
        } else if be.operator == BinaryOperator::Exponential {
            // ** operator: JS exponentiation
            // JS `**` always returns number (f64), even for integer operands.
            // Use std.math.pow(f64, ...) with temporary f64 variables.
            // Use a unique suffix to avoid variable shadowing in nested `**` expressions.
            // For i64 variables, use @floatFromInt (Zig doesn't allow @as(f64, i64_var)).
            let pow_id = self.label_counter;
            let blk = self.next_label();
            self.write(&format!("({blk}: {{ "));
            // Left operand
            let left_type = self.infer_expr_type(&be.left);
            self.write(&format!("const _base_f64_{pow_id}: f64 = "));
            self.emit_float_conversion(&be.left, left_type);
            self.write(&format!("; const _exp_f64_{pow_id}: f64 = "));
            let right_type = self.infer_expr_type(&be.right);
            self.emit_float_conversion(&be.right, right_type);
            self.write(&format!(
                "; break :{blk} std.math.pow(f64, _base_f64_{pow_id}, _exp_f64_{pow_id}); }})",
            ));
        } else if be.operator == BinaryOperator::In {
            // `key in obj` → obj.contains(key)
            // Right side is the object, left side is the key
            self.emit_expr(&be.right);
            self.write(".contains(");
            self.emit_expr(&be.left);
            self.write(")");
        } else if be.operator == BinaryOperator::ShiftRightZeroFill {
            // JS >>> (unsigned right shift): treat left as u32, shift right with zero fill.
            // JS semantics: ToUint32(left) >>> (ToUint32(right) & 31)
            // Zig has no >>> for signed ints, so we bit-cast through u32.
            // Use @intCast (not @bitCast) for u32→i64 widening (zero-extend).
            self.write("@as(i64, @intCast(@as(u32, @bitCast(@as(i32, @truncate(");
            self.emit_expr_for_arithmetic(&be.left);
            self.write(")))) >> @intCast(");
            self.emit_expr_for_arithmetic(&be.right);
            self.write(" & 31)))");
        } else if be.operator == BinaryOperator::Instanceof {
            // `x instanceof Constructor` — not directly supported in Zig.
            // Emit a compile error with source location info.
            self.compile_error(be.span, "instanceof operator is not supported in Zig");
        } else {
            // Determine whether this is an arithmetic operator (where BooleanLiteral
            // must be coerced to integer) or a comparison (where bool is fine and
            // JsAny type incompatibility checks apply).
            let is_arithmetic = matches!(
                be.operator,
                BinaryOperator::Addition
                    | BinaryOperator::Subtraction
                    | BinaryOperator::Multiplication
                    | BinaryOperator::BitwiseAnd
                    | BinaryOperator::BitwiseOR
                    | BinaryOperator::BitwiseXOR
                    | BinaryOperator::ShiftLeft
                    | BinaryOperator::ShiftRight
            );

            if is_arithmetic {
                // Arithmetic: coerce BooleanLiteral to integer, emit directly.
                // Do NOT route through JsAny comparison — cross-type arithmetic
                // (e.g. bool + i64) should be handled via @intFromBool coercion,
                // not via JsAny method calls.
                self.emit_expr_for_arithmetic(&be.left);
                self.write(" ");
                self.write(Self::binary_op(be.operator));
                self.write(" ");
                self.emit_expr_for_arithmetic(&be.right);
            } else {
                // Check if either side is JsAny — need to use method calls for comparison
                let left_type = self.infer_expr_type(&be.left);
                let right_type = self.infer_expr_type(&be.right);
                // Only JsAny (not Anytype) should use .eq()/.asI64() methods.
                // Anytype means "inferred at call site" — could be i64 or JsAny,
                // so generate direct comparison (Zig will type-check at call site).
                let left_is_jsany = left_type == Some(ZigType::JsAny);
                let right_is_jsany = right_type == Some(ZigType::JsAny);

                // Check for incompatible types (e.g. I64 vs Bool, Str vs I64 for >=).
                // JS allows cross-type comparisons via coercion, but Zig doesn't.
                // Route these through JsAny comparison for type-safe code.
                // Anytype is excluded: it resolves at the call site, not here.
                let left_is_anytype = left_type == Some(ZigType::Anytype);
                let right_is_anytype = right_type == Some(ZigType::Anytype);
                let both_numeric = matches!(left_type, Some(ZigType::I64) | Some(ZigType::F64))
                    && matches!(right_type, Some(ZigType::I64) | Some(ZigType::F64));
                let types_incompatible = left_type.is_some()
                    && right_type.is_some()
                    && !both_numeric
                    && left_type != right_type
                    && !left_is_anytype
                    && !right_is_anytype;

                let should_use_jsany = left_is_jsany || right_is_jsany || types_incompatible;

                if should_use_jsany {
                    self.emit_jsany_comparison(be, left_is_jsany, right_is_jsany);
                } else {
                    self.emit_expr(&be.left);
                    self.write(" ");
                    self.write(Self::binary_op(be.operator));
                    self.write(" ");
                    self.emit_expr(&be.right);
                }
            }
        }

        self.in_expr_stmt = saved_expr_stmt;
    }

    /// Check if a binary operator is a comparison operator.
    /// Comparison operators are valid between BigInt and Number in JS,
    /// unlike arithmetic operators which throw TypeError.
    fn is_comparison_operator(op: BinaryOperator) -> bool {
        matches!(
            op,
            BinaryOperator::Equality
                | BinaryOperator::StrictEquality
                | BinaryOperator::Inequality
                | BinaryOperator::StrictInequality
                | BinaryOperator::LessThan
                | BinaryOperator::LessEqualThan
                | BinaryOperator::GreaterThan
                | BinaryOperator::GreaterEqualThan
        )
    }

    /// Emit comparison between BigInt and non-BigInt (number/bool/etc).
    /// JS allows these comparisons; we convert the BigInt side to f64.
    /// For strict equality/inequality with mixed types, emit constant
    /// false/true since BigInt !== Number in JS.
    fn emit_bigint_mixed_compare(&mut self, be: &BinaryExpression, left_is_bigint: bool) {
        match be.operator {
            BinaryOperator::StrictEquality => {
                // BigInt === non-BigInt → false (different types in JS)
                self.write("false");
            }
            BinaryOperator::StrictInequality => {
                // BigInt !== non-BigInt → true
                self.write("true");
            }
            _ => {
                // Numeric comparison: convert BigInt to f64, then compare.
                // Generate: @floatFromInt((bigint_side).toI64() catch @panic(...))
                self.write("@as(f64, @floatFromInt((");
                if left_is_bigint {
                    self.emit_expr(&be.left);
                } else {
                    self.emit_expr(&be.right);
                }
                self.write(").toI64() catch @panic(\"BigInt too large for comparison\"))) ");
                // Zig comparison operator
                self.write(match be.operator {
                    BinaryOperator::Equality => "==",
                    BinaryOperator::Inequality => "!=",
                    BinaryOperator::LessThan => "<",
                    BinaryOperator::LessEqualThan => "<=",
                    BinaryOperator::GreaterThan => ">",
                    BinaryOperator::GreaterEqualThan => ">=",
                    _ => unreachable!("unexpected comparison operator in mixed BigInt compare"),
                });
                self.write(" ");
                if left_is_bigint {
                    self.emit_expr(&be.right);
                } else {
                    self.emit_expr(&be.left);
                }
            }
        }
    }

    /// Emit comparison between BigInt and String.
    /// JS coerces the string to a number (via ToNumber) before comparing.
    /// We use std.fmt.parseFloat to convert the string side to f64.
    fn emit_bigint_string_compare(&mut self, be: &BinaryExpression, left_is_bigint: bool) {
        match be.operator {
            BinaryOperator::StrictEquality => {
                // BigInt === string → false (different types in JS)
                self.write("false");
            }
            BinaryOperator::StrictInequality => {
                // BigInt !== string → true
                self.write("true");
            }
            _ => {
                // Numeric comparison: parse string to f64, convert BigInt to f64.
                let cmp_op = match be.operator {
                    BinaryOperator::Equality => "==",
                    BinaryOperator::Inequality => "!=",
                    BinaryOperator::LessThan => "<",
                    BinaryOperator::LessEqualThan => "<=",
                    BinaryOperator::GreaterThan => ">",
                    BinaryOperator::GreaterEqualThan => ">=",
                    _ => unreachable!("unexpected comparison operator in String/BigInt compare"),
                };
                if left_is_bigint {
                    // BigInt cmp String
                    self.write("@as(f64, @floatFromInt((");
                    self.emit_expr(&be.left);
                    self.write(").toI64() catch @panic(\"BigInt too large for comparison\"))) ");
                    self.write(cmp_op);
                    self.write(" (std.fmt.parseFloat(f64, ");
                    self.emit_expr(&be.right);
                    self.write(") catch std.math.nan(f64))");
                } else {
                    // String cmp BigInt
                    self.write("(std.fmt.parseFloat(f64, ");
                    self.emit_expr(&be.left);
                    self.write(") catch std.math.nan(f64)) ");
                    self.write(cmp_op);
                    self.write(" @as(f64, @floatFromInt((");
                    self.emit_expr(&be.right);
                    self.write(").toI64() catch @panic(\"BigInt too large for comparison\")))");
                }
            }
        }
    }

    /// Emit BigInt binary operation.
    /// Both operands are known to be BigInt.
    fn emit_bigint_binary(&mut self, be: &BinaryExpression) {
        // >>> on BigInt throws TypeError in JS (no unsigned right shift for BigInt).
        // Wrap @panic in an if-else expression so Zig does not treat it
        // as unconditionally noreturn (which would make subsequent
        // statements unreachable at compile time).
        if be.operator == BinaryOperator::ShiftRightZeroFill {
            // JS throws TypeError for BigInt >>> at runtime.
            // Use if-else expression so that `return;` does not make
            // subsequent statements unreachable at compile time.
            self.write("if (true) { return; } else {}");
            return;
        }

        // Shift operators need a usize shift amount, not &JsBigInt.
        // Generate: (blk: { var _a = left; var _b = right; break :blk _a.shiftLeft(_b.toU64() catch ..., alloc) catch ...; })
        if matches!(
            be.operator,
            BinaryOperator::ShiftLeft | BinaryOperator::ShiftRight
        ) {
            let blk = self.next_label();
            let var_suffix = self.label_counter - 1;
            let a_name = format!("_a{}", var_suffix);
            let b_name = format!("_b{}", var_suffix);
            let op = match be.operator {
                BinaryOperator::ShiftLeft => "shiftLeft",
                BinaryOperator::ShiftRight => "shiftRight",
                _ => unreachable!("only shift operators reach this branch"),
            };
            self.write(&format!("({}: {{ var {} = ", blk, a_name));
            self.emit_expr(&be.left);
            self.write(&format!("; var {} = ", b_name));
            self.emit_expr(&be.right);
            self.write(&format!(
                "; break :{} {}.{}({}.toU64() catch @panic(\"BigInt shift count too large\"), \
                js_allocator.getAllocator()) catch @panic(\"OOM: BigInt shift\"); }})",
                blk, a_name, op, b_name
            ));
            return;
        }

        // All other BigInt operators: _a.op(&_b, alloc) pattern.
        // Generate: (blk_N: { var _a_N = <left>; var _b_N = <right>; break :blk_N _a_N.op(&_b_N, alloc) catch @panic(...); })
        // Use unique variable names based on label counter to avoid shadowing in nested BigInt expressions.
        let blk = self.next_label();
        let var_suffix = self.label_counter - 1; // label_counter was already incremented by next_label()
        let a_name = format!("_a{}", var_suffix);
        let b_name = format!("_b{}", var_suffix);
        self.write(&format!("({}: {{ var {} = ", blk, a_name));
        self.emit_expr(&be.left);
        self.write(&format!("; var {} = ", b_name));
        self.emit_expr(&be.right);
        // Division handles its own break with inline error handling.
        if !matches!(be.operator, BinaryOperator::Division) {
            self.write(&format!("; break :{} ", blk));
        }
        match be.operator {
            BinaryOperator::Addition => {
                self.write(&format!(
                    "{}.add(&{}, js_allocator.getAllocator())",
                    a_name, b_name
                ));
            }
            BinaryOperator::Subtraction => {
                self.write(&format!(
                    "{}.sub(&{}, js_allocator.getAllocator())",
                    a_name, b_name
                ));
            }
            BinaryOperator::Multiplication => {
                self.write(&format!(
                    "{}.mul(&{}, js_allocator.getAllocator())",
                    a_name, b_name
                ));
            }
            BinaryOperator::Division => {
                // BigInt division by zero throws RangeError in JS.
                // Zig's Managed.divTrunc asserts on zero — not an error return.
                // Check for zero divisor first and return early (matching JS try/catch).
                self.write(&format!(
                    "; if ({b}.isZero()) return; \
                    const _dr = {a}.div(&{b}, js_allocator.getAllocator()) catch @panic(\"OOM: BigInt div\"); \
                    break :{blk} _dr; }})",
                    a = a_name,
                    b = b_name,
                    blk = blk,
                ));
                // Division handled inline — don't add catch in post-match.
                return;
            }
            BinaryOperator::Exponential => {
                // BigInt ** requires exponent to be u64
                // JS: exponent is converted via ToUint64 (same as ToIntegerOrInfinity then mod 2^32)
                // Use catch instead of try — the enclosing break expression already
                // uses catch, and try would propagate the error past the catch to
                // the containing void function.
                self.write(&format!(
                    "{}.pow({}.toU64() catch @panic(\"OOM: BigInt toU64\"), js_allocator.getAllocator())",
                    a_name, b_name
                ));
            }
            BinaryOperator::Remainder => {
                self.write(&format!(
                    "{}.rem(&{}, js_allocator.getAllocator())",
                    a_name, b_name
                ));
            }
            BinaryOperator::BitwiseAnd => {
                self.write(&format!(
                    "{}.bitwiseAnd(&{}, js_allocator.getAllocator())",
                    a_name, b_name
                ));
            }
            BinaryOperator::BitwiseOR => {
                self.write(&format!(
                    "{}.bitwiseOr(&{}, js_allocator.getAllocator())",
                    a_name, b_name
                ));
            }
            BinaryOperator::BitwiseXOR => {
                self.write(&format!(
                    "{}.bitwiseXor(&{}, js_allocator.getAllocator())",
                    a_name, b_name
                ));
            }
            BinaryOperator::Equality | BinaryOperator::StrictEquality => {
                self.write(&format!("{}.eq(&{})", a_name, b_name));
            }
            BinaryOperator::Inequality | BinaryOperator::StrictInequality => {
                self.write(&format!("!{}.eq(&{})", a_name, b_name));
            }
            BinaryOperator::LessThan => {
                self.write(&format!("{}.order(&{}) == .lt", a_name, b_name));
            }
            BinaryOperator::LessEqualThan => {
                self.write(&format!("{}.order(&{}) != .gt", a_name, b_name));
            }
            BinaryOperator::GreaterThan => {
                self.write(&format!("{}.order(&{}) == .gt", a_name, b_name));
            }
            BinaryOperator::GreaterEqualThan => {
                self.write(&format!("{}.order(&{}) != .lt", a_name, b_name));
            }
            // ShiftRightZeroFill is handled in the early guard above.
            // ShiftLeft / ShiftRight are handled in the separate shift block above.
            _ => unreachable!("BigInt operator should have been caught by early guard"),
        }
        if matches!(
            be.operator,
            BinaryOperator::Addition
                | BinaryOperator::Subtraction
                | BinaryOperator::Multiplication
                | BinaryOperator::Exponential
                | BinaryOperator::Remainder
                | BinaryOperator::BitwiseAnd
                | BinaryOperator::BitwiseOR
                | BinaryOperator::BitwiseXOR
        ) {
            self.write(" catch @panic(\"OOM: BigInt op\"); })");
        } else {
            // Comparison operators return bool (not error union)
            self.write("; })");
        }
    }

    /// Emit comparison code for JsAny values.
    /// At least one side is JsAny. We generate method calls on the JsAny side.
    fn emit_jsany_comparison(
        &mut self,
        be: &BinaryExpression,
        left_is_jsany: bool,
        right_is_jsany: bool,
    ) {
        match be.operator {
            BinaryOperator::Equality
            | BinaryOperator::StrictEquality
            | BinaryOperator::Inequality
            | BinaryOperator::StrictInequality => {
                let negate = matches!(
                    be.operator,
                    BinaryOperator::Inequality | BinaryOperator::StrictInequality
                );
                // Strict (===, !==) uses strictEq (no type coercion);
                // Loose (==, !=) uses eq (JS coercion semantics).
                let method = if matches!(
                    be.operator,
                    BinaryOperator::StrictEquality | BinaryOperator::StrictInequality
                ) {
                    "strictEq"
                } else {
                    "eq"
                };
                if negate {
                    self.write("!");
                }
                // left.eq(right) — prefer left as receiver if it's JsAny, otherwise wrap left.
                if left_is_jsany {
                    self.emit_expr(&be.left);
                    self.write(&format!(".{method}("));
                    self.emit_as_jsany(&be.right, right_is_jsany);
                    self.write(")");
                } else {
                    self.write("JsAny.from(");
                    self.emit_expr(&be.left);
                    self.write(&format!(").{method}("));
                    self.emit_as_jsany(&be.right, right_is_jsany);
                    self.write(")");
                }
            }
            BinaryOperator::LessThan
            | BinaryOperator::LessEqualThan
            | BinaryOperator::GreaterThan
            | BinaryOperator::GreaterEqualThan => {
                // Numeric comparison: use .asI64() on the JsAny side(s).
                let op_str = Self::binary_op(be.operator);
                if left_is_jsany {
                    self.emit_expr(&be.left);
                    self.write(".asI64() ");
                    self.write(op_str);
                    self.write(" ");
                    if right_is_jsany {
                        self.emit_expr(&be.right);
                        self.write(".asI64()");
                    } else {
                        self.emit_expr(&be.right);
                    }
                } else {
                    // left is primitive → wrap left, then compare as i64.
                    // Right may or may not be JsAny — use emit_as_jsany to handle both.
                    self.write("JsAny.from(");
                    self.emit_expr(&be.left);
                    self.write(").asI64() ");
                    self.write(op_str);
                    self.write(" ");
                    self.emit_as_jsany(&be.right, right_is_jsany);
                    self.write(".asI64()");
                }
            }
            _ => {
                // Default: emit as-is (may cause compile error)
                self.emit_expr(&be.left);
                self.write(" ");
                self.write(Self::binary_op(be.operator));
                self.write(" ");
                self.emit_expr(&be.right);
            }
        }
    }

    /// Emit an expression as a JsAny value, wrapping with `JsAny.from()` if it's not already JsAny.
    fn emit_as_jsany(&mut self, expr: &Expression, is_jsany: bool) {
        if is_jsany {
            self.emit_expr(expr);
        } else {
            self.write("JsAny.from(");
            self.emit_expr(expr);
            self.write(")");
        }
    }

    /// Map TypedArray constructor name to codegen info.
    fn typedarray_info(name: &str) -> Option<TypedArrayInfo> {
        match name {
            "Int8Array" => Some(TypedArrayInfo {
                func: "js_runtime.js_typedarray.fromI64AsI8",
                array_prefix: "&[_]i64{",
            }),
            "Uint8Array" => Some(TypedArrayInfo {
                func: "js_runtime.js_typedarray.fromI64AsU8",
                array_prefix: "&[_]i64{",
            }),
            "Uint8ClampedArray" => Some(TypedArrayInfo {
                func: "js_runtime.js_typedarray.fromI64AsU8",
                array_prefix: "&[_]i64{",
            }),
            "Int16Array" => Some(TypedArrayInfo {
                func: "js_runtime.js_typedarray.fromI64AsI16",
                array_prefix: "&[_]i64{",
            }),
            "Uint16Array" => Some(TypedArrayInfo {
                func: "js_runtime.js_typedarray.fromI64AsU16",
                array_prefix: "&[_]i64{",
            }),
            "Int32Array" => Some(TypedArrayInfo {
                func: "js_runtime.js_typedarray.fromI64AsI32",
                array_prefix: "&[_]i64{",
            }),
            "Uint32Array" => Some(TypedArrayInfo {
                func: "js_runtime.js_typedarray.fromI64AsU32",
                array_prefix: "&[_]i64{",
            }),
            "Float32Array" => Some(TypedArrayInfo {
                func: "js_runtime.js_typedarray.fromF64AsF32",
                array_prefix: "&[_]f64{",
            }),
            "Float64Array" => Some(TypedArrayInfo {
                func: "js_runtime.js_typedarray.fromF64",
                array_prefix: "&[_]f64{",
            }),
            _ => None,
        }
    }

    /// Check if name is an Error constructor.
    fn is_error_constructor(name: &str) -> bool {
        matches!(
            name,
            "Error"
                | "TypeError"
                | "RangeError"
                | "ReferenceError"
                | "SyntaxError"
                | "EvalError"
                | "URIError"
                | "AggregateError"
                | "SuppressedError"
                | "InternalError"
        )
    }

    /// Check if an expression evaluates to a string type
    fn expr_is_string(&mut self, expr: &Expression) -> bool {
        match expr {
            Expression::StringLiteral(_) => true,
            Expression::TemplateLiteral(_) => true,
            Expression::Identifier(id) => {
                self.type_info.var_types.get(id.name.as_str()) == Some(&ZigType::Str)
            }
            // Handle nested binary expressions: if it's string concatenation, result is string
            Expression::BinaryExpression(be) if be.operator == BinaryOperator::Addition => {
                self.expr_is_string(&be.left) || self.expr_is_string(&be.right)
            }
            // ConditionalExpression (ternary): result is string if both branches are strings
            Expression::ConditionalExpression(ce) => {
                // Check if both branches are definitely strings
                let cons_is_str = match &ce.consequent {
                    Expression::StringLiteral(_) => true,
                    _ => self.expr_is_string(&ce.consequent),
                };
                let alt_is_str = match &ce.alternate {
                    Expression::StringLiteral(_) => true,
                    _ => self.expr_is_string(&ce.alternate),
                };
                cons_is_str && alt_is_str
            }
            // ParenthesizedExpression: unwrap and recurse
            Expression::ParenthesizedExpression(pe) => self.expr_is_string(&pe.expression),
            // CallExpression — check if return type is Str (e.g. jsTypeof(), String())
            Expression::CallExpression(_ce) => self.infer_expr_type(expr) == Some(ZigType::Str),
            _ => false,
        }
    }

    /// Check if an expression evaluates to a BigInt type
    fn expr_is_bigint(&self, expr: &Expression) -> bool {
        match expr {
            Expression::BigIntLiteral(_) => true,
            Expression::Identifier(id) => {
                self.type_info.var_types.get(id.name.as_str()) == Some(&ZigType::BigInt)
            }
            Expression::ParenthesizedExpression(pe) => self.expr_is_bigint(&pe.expression),
            // BigInt(x) constructor call produces a BigInt
            Expression::CallExpression(ce) => {
                if let Expression::Identifier(id) = &ce.callee {
                    id.name == "BigInt"
                } else {
                    false
                }
            }
            // BigInt op BigInt → BigInt
            Expression::BinaryExpression(be) => {
                self.expr_is_bigint(&be.left) && self.expr_is_bigint(&be.right)
            }
            // -bigint → BigInt, ~bigint → BigInt
            Expression::UnaryExpression(ue)
                if ue.operator == UnaryOperator::UnaryNegation
                    || ue.operator == UnaryOperator::BitwiseNot =>
            {
                self.expr_is_bigint(&ue.argument)
            }
            _ => false,
        }
    }

    /// Check if an expression is Object(bigint_expr).
    /// Object() wraps a BigInt primitive into a BigInt wrapper object.
    /// In JS, Object(BigInt) !== BigInt (different types), and two
    /// Object(BigInt) calls always create distinct objects (=== false).
    fn expr_is_object_bigint(&self, expr: &Expression) -> bool {
        match expr {
            Expression::CallExpression(ce) => {
                if let Expression::Identifier(id) = &ce.callee
                    && id.name == "Object"
                    && ce.arguments.len() == 1
                    && let Some(arg) = ce.arguments.first()
                    && let Some(e) = arg.as_expression()
                {
                    return self.expr_is_bigint(e);
                }
                false
            }
            Expression::ParenthesizedExpression(pe) => self.expr_is_object_bigint(&pe.expression),
            _ => false,
        }
    }

    /// Check if an expression is known to produce a Bool result.
    /// Used by control-flow statements (if/while/for) to decide whether
    /// `!= 0` coercion is needed.
    /// Check whether an expression is statically known to produce a `bool` result.
    /// Used by `emit_condition` and `LogicalExpression` codegen to avoid
    /// redundant `((expr) != 0)` coercion (which is illegal in Zig for `bool`).
    pub(crate) fn expr_is_definitely_bool(&mut self, expr: &Expression) -> bool {
        match expr {
            Expression::BooleanLiteral(_) => true,
            // LogicalExpression always produces Bool (operands are coerced in codegen)
            Expression::LogicalExpression(_) => true,
            Expression::ParenthesizedExpression(pe) => self.expr_is_definitely_bool(&pe.expression),
            Expression::UnaryExpression(ue) => {
                matches!(
                    ue.operator,
                    UnaryOperator::LogicalNot | UnaryOperator::Delete
                )
            }
            Expression::BinaryExpression(be) => {
                matches!(
                    be.operator,
                    BinaryOperator::Equality
                        | BinaryOperator::StrictEquality
                        | BinaryOperator::Inequality
                        | BinaryOperator::StrictInequality
                        | BinaryOperator::LessThan
                        | BinaryOperator::LessEqualThan
                        | BinaryOperator::GreaterThan
                        | BinaryOperator::GreaterEqualThan
                        | BinaryOperator::In
                        | BinaryOperator::Instanceof
                )
            }
            Expression::Identifier(id) => {
                let ty = self.type_info.var_types.get(id.name.as_str());
                // Bool and Anytype both pass through without coercion.
                // Anytype resolves at call site; Zig will check there.
                matches!(ty, Some(ZigType::Bool) | Some(ZigType::Anytype))
            }
            Expression::ConditionalExpression(ce) => {
                self.expr_is_definitely_bool(&ce.consequent)
                    && self.expr_is_definitely_bool(&ce.alternate)
            }
            // Fallback: ask the type inference system. Covers CallExpression
            // (e.g. `A()` where A returns bool), StaticMemberExpression
            // (e.g. `obj.isReady` field of type bool), etc.
            _ => self.infer_expr_type(expr) == Some(ZigType::Bool),
        }
    }

    /// Emit an expression with truthiness coercion for Zig `bool` context.
    ///
    /// When a non-bool expression appears in a position that Zig requires `bool`
    /// (e.g. `if` condition, `and`/`or` operand), we coerce it via JS truthiness:
    /// - `bool` expressions → emitted directly (no coercion needed)
    /// - `Str` expressions → `.len != 0` (empty string is falsy in JS)
    /// - numeric/other → `((expr) != 0)` (0 is falsy in JS)
    pub(crate) fn emit_expr_as_bool(&mut self, expr: &Expression) {
        if self.expr_is_definitely_bool(expr) {
            // Already bool; just emit the expression directly.
            // This also handles the case where infer_expr_type returns Bool
            // for function calls that return bool.
            self.emit_expr(expr);
        } else if let Some(ZigType::Str) = self.infer_expr_type(expr) {
            // String truthiness: non-empty → true, empty → false
            self.write("(");
            self.emit_expr(expr);
            self.write(".len != 0)");
        } else {
            // Default numeric truthiness: 0 → false, non-zero → true
            self.write("((");
            self.emit_expr(expr);
            self.write(") != 0)");
        }
    }

    // Call expression (all calls get `try`)
    fn emit_call(&mut self, ce: &CallExpression) {
        // Check if this is a Promise .then() or .catch() call (not supported in native_proto)
        if let Expression::StaticMemberExpression(ref mem) = ce.callee {
            let prop_name = mem.property.name.as_str();
            if prop_name == "then" || prop_name == "catch" {
                self.errors.push(format!(
                    "Promise.{}() is not supported. Use 'await' instead of '.{}()'",
                    prop_name, prop_name
                ));
                self.compile_error_fmt(
                    ce.span,
                    format!("Promise.{}() not supported, use 'await' instead", prop_name),
                );
                return;
            }
        }

        // Check if this is a Promise.resolve() or Promise.reject() call
        if let Expression::StaticMemberExpression(ref mem) = ce.callee
            && let Expression::Identifier(ref obj) = mem.object
            && obj.name == "Promise"
        {
            let method = mem.property.name.as_str();
            if method == "resolve" || method == "reject" {
                self.errors.push(format!(
                            "Promise.{}() is not supported in native_proto mode. Use 'await' with async functions instead.",
                            method
                        ));
                self.compile_error_fmt(ce.span, format!("Promise.{}() not supported", method));
                return;
            }
        }

        // Check if this is a built-in object call (Math.xxx(), arr.xxx(), str.xxx())
        // Route regexpVar.isMatch(str) / regexpVar.exec(str) to RegExp builtins even though
        // detect_builtin_call doesn't handle Identifier receivers (only RegExpLiteral).
        if let Expression::StaticMemberExpression(ref mem) = ce.callee
            && let Expression::Identifier(ref obj_id) = mem.object
            && self.regexp_vars.contains(obj_id.name.as_str())
        {
            let builtin = match mem.property.name.as_str() {
                "test" => builtins::BuiltinCall::RegExpTest,
                "exec" => builtins::BuiltinCall::RegExpExec,
                _ => return,
            };
            if self.emit_builtin_call(&builtin, ce) {
                return;
            }
        }
        // Handle ArrayExpression method calls: ['a','b'].includes('a')
        // Generate a temporary array variable, then dispatch to builtin.
        if let Expression::StaticMemberExpression(ref mem) = ce.callee
            && matches!(mem.object, Expression::ArrayExpression(_))
        {
            // Generate temp variable name
            let tmp_name = format!("_arr_lit_{}", self.task_counter);
            self.task_counter += 1;
            // Emit temp variable declaration
            let blk_lit = self.next_label();
            self.write(&format!("({}: {{ const {} = ", blk_lit, tmp_name));
            self.emit_expr(&mem.object);
            self.write(";\n");
            // We can't easily rewrite CE here; instead, manually emit the builtin call
            // by constructing a new StaticMemberExpression with Identifier temp name.
            // For simplicity, just emit the array method directly:
            let method_name = mem.property.name.as_str();
            match method_name {
                "includes" => {
                    // ['a','b'].includes(x) → (blk: { ... })
                    if ce.arguments.len() != 1 {
                        self.errors
                            .push("Array.includes() requires 1 argument".to_string());
                        return;
                    }
                    let arg_expr =
                        self.emit_expr_to_string(ce.arguments[0].as_expression().unwrap());
                    self.write(&format!(
                        "if (std.mem.indexOf(u8, {tmp}.items, {arg}) != null) true else false;",
                        tmp = tmp_name,
                        arg = arg_expr
                    ));
                    self.write(" })");
                    return;
                }
                "indexOf" => {
                    if ce.arguments.len() != 1 {
                        self.errors
                            .push("Array.indexOf() requires 1 argument".to_string());
                        return;
                    }
                    let arg_expr =
                        self.emit_expr_to_string(ce.arguments[0].as_expression().unwrap());
                    self.write(&format!(
                        "((std.mem.indexOf(u8, {tmp}.items, {arg}) orelse -1) catch -1); }}",
                        tmp = tmp_name,
                        arg = arg_expr
                    ));
                    return;
                }
                _ => {
                    self.errors.push(format!(
                        "Array literal method '.{}()' not yet supported",
                        method_name
                    ));
                    return;
                }
            }
        }
        if let Some(mut builtin) = builtins::detect_builtin_call(ce) {
            // Override: if detect_builtin_call returns ArrayAt but object is a string, use StringAt
            if matches!(builtin, builtins::BuiltinCall::ArrayAt)
                && let Expression::StaticMemberExpression(ref mem) = ce.callee
                && let Expression::Identifier(ref obj_id) = mem.object
            {
                let obj_name = obj_id.name.as_str();
                // Check if obj is a string variable (from type_info)
                if let Some(ZigType::Str) = self.type_info.var_types.get(obj_name) {
                    builtin = builtins::BuiltinCall::StringAt;
                }
            }
            // Override: if detect_builtin_call returns MapKeys/MapValues/MapEntries
            // but object is a Set variable, use SetKeys/SetValues/SetEntries
            if let Expression::StaticMemberExpression(ref mem) = ce.callee
                && let Expression::Identifier(ref obj_id) = mem.object
            {
                let obj_name = obj_id.name.as_str();
                if self.type_info.set_vars.contains(obj_name) {
                    match builtin {
                        builtins::BuiltinCall::MapKeys => {
                            builtin = builtins::BuiltinCall::SetKeys;
                        }
                        builtins::BuiltinCall::MapValues => {
                            builtin = builtins::BuiltinCall::SetValues;
                        }
                        builtins::BuiltinCall::MapEntries => {
                            builtin = builtins::BuiltinCall::SetEntries;
                        }
                        builtins::BuiltinCall::ArrayForEach => {
                            builtin = builtins::BuiltinCall::SetForEach;
                        }
                        _ => {}
                    }
                }
                // Override: if detect_builtin_call returns MapKeys/MapValues/MapEntries
                // but object is an array variable, use ArrayKeys/ArrayValues/ArrayEntries
                if self.type_info.array_element_types.contains_key(obj_name) {
                    match builtin {
                        builtins::BuiltinCall::MapKeys => {
                            builtin = builtins::BuiltinCall::ArrayKeys;
                        }
                        builtins::BuiltinCall::MapValues => {
                            builtin = builtins::BuiltinCall::ArrayValues;
                        }
                        builtins::BuiltinCall::MapEntries => {
                            builtin = builtins::BuiltinCall::ArrayEntries;
                        }
                        _ => {}
                    }
                }
            }
            if self.emit_builtin_call(&builtin, ce) {
                return;
            }
        }
        // If emit_builtin_call returns false, fall through to normal call handling

        // Get callee name.
        let callee_name = match &ce.callee {
            Expression::Identifier(id) => Some(id.name.to_string()),
            _ => None,
        };

        // Check if this is a closure variable call (e.g., `fn(args)` where fn is a closure struct instance)
        // or a nested function call (e.g., `inner(args)` where inner is a hoisted struct type).
        if let Some(ref name) = callee_name
            && (self.closure_instances.contains(name.as_str())
                || self.nested_fn_names.contains(name.as_str()))
        {
            // Rewrite to `variable.call(args)` or `NestedFn.call(args)`
            // Use zig_safe_name to handle Zig reserved keywords (e.g. test → _test)
            let safe = self.zig_safe_name(name);
            self.write(&safe);
            self.write(".call(");
            self.emit_comma_separated_args(&ce.arguments);
            self.write(")");
            return;
        }

        // Emit function call (no `try` by default, only for error-returning functions).
        if let Some(ref name) = callee_name {
            // Check if this is a host function call (host_xxx)
            if let Some(host_func_name) = name.strip_prefix("host_") {
                // Convert host_add(...) to host.add(...)
                self.write(&format!("host.{}(", host_func_name));
                self.emit_comma_separated_args(&ce.arguments);
                self.write(")");
                return;
            }
            self.write(&self.zig_safe_name(name));
        } else if let Expression::StaticMemberExpression(ref mem) = ce.callee {
            // Member function call: obj.method(args)
            // Check if obj is a class instance → emit obj.method(args) directly
            let obj_name = if let Expression::Identifier(ref obj_id) = mem.object {
                Some(obj_id.name.to_string())
            } else {
                None
            };
            if let Some(ref obj) = obj_name
                && self
                    .type_info
                    .var_types
                    .get(obj)
                    .is_some_and(|t| matches!(t, ZigType::NamedStruct(_)))
            {
                // Class method call: rect.area() → rect.area(args)
                self.write(obj);
                self.write(".");
                self.write(mem.property.name.as_str());
                self.write("(");
                self.emit_comma_separated_args(&ce.arguments);
                self.write(")");
                return;
            }
            // Fallback for non-Identifier objects (e.g., Temporal.Now.instant(),
            // new DataView(buf).setInt16(), null.f()): emit the call directly
            // and let the Zig compiler decide if the method is valid.
            // For Identifier objects (e.g., Atomics.load, registry.register),
            // keep the @compileError behavior so test_not_implemented_* tests pass.
            if obj_name.is_none() {
                // Non-Identifier object — emit directly
                self.emit_expr(&mem.object);
                self.write(&format!(".{}(", mem.property.name.as_str()));
                self.emit_comma_separated_args(&ce.arguments);
                self.write(")");
                return;
            }
            // Identifier object with unrecognized method — generate compile error
            let callee_str = format!("{:?}", ce.callee);
            self.errors.push(format!(
                "Member function calls (obj.method()) are not fully supported in native_proto mode: callee = {}",
                callee_str
            ));
            self.compile_error(ce.span, "Member function calls not supported");
            return;
        } else if let Expression::ParenthesizedExpression(pe) = &ce.callee {
            // IIFE: (function(){})() or (()=>{})() — unwrap and handle the inner function
            match &pe.expression {
                Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_) => {
                    // emit_expr will generate the function definition (deferred)
                    // and write the function name as the expression value.
                    self.emit_expr(&pe.expression);
                    // For FunctionExpression, call via .call(args)
                    // For ArrowFunctionExpression, call directly (args)
                    if matches!(&pe.expression, Expression::FunctionExpression(_)) {
                        self.write(".call(");
                    } else {
                        self.write("(");
                    }
                    self.emit_comma_separated_args(&ce.arguments);
                    self.write(")");
                    return;
                }
                _ => {
                    // Unsupported callee type inside parentheses
                    let callee_str = format!("{:?}", ce.callee);
                    self.errors.push(format!(
                        "Unsupported callee type in native_proto mode: callee = {}",
                        callee_str
                    ));
                    self.compile_error(ce.span, "Unsupported callee type");
                    return;
                }
            }
        } else if matches!(&ce.callee, Expression::FunctionExpression(_))
            || matches!(&ce.callee, Expression::ArrowFunctionExpression(_))
        {
            // Direct function expression as callee: function(){}() or ()=>{})()
            self.emit_expr(&ce.callee);
            if matches!(&ce.callee, Expression::FunctionExpression(_)) {
                self.write(".call(");
            } else {
                self.write("(");
            }
            self.emit_comma_separated_args(&ce.arguments);
            self.write(")");
            return;
        } else {
            // Other unsupported callee types
            let callee_str = format!("{:?}", ce.callee);
            self.errors.push(format!(
                "Unsupported callee type in native_proto mode: callee = {}",
                callee_str
            ));
            self.compile_error(ce.span, "Unsupported callee type");
            return;
        }
        self.write("(");
        self.emit_comma_separated_args(&ce.arguments);
        self.write(")");
    }

    /// Emit an expression to a temporary string (preserves self.output and all state).
    pub(crate) fn emit_expr_to_string(&mut self, expr: &Expression) -> String {
        let saved = std::mem::take(&mut self.output);
        self.emit_expr(expr);
        let result = std::mem::take(&mut self.output);
        self.output = saved;
        result
    }

    // ── Builtin lookup tables (table-driven dispatch helpers) ──────

    /// Descriptor for a simple 1-arg Math builtin mapped to a Zig call.
    fn math_one_arg_desc(b: &builtins::BuiltinCall) -> Option<MathOneArgDesc> {
        match b {
            // Direct Zig builtins
            builtins::BuiltinCall::MathAbs => Some(MathOneArgDesc {
                name: "abs",
                format: "@abs({arg})",
            }),
            builtins::BuiltinCall::MathFloor => Some(MathOneArgDesc {
                name: "floor",
                format: "@floor({arg})",
            }),
            builtins::BuiltinCall::MathCeil => Some(MathOneArgDesc {
                name: "ceil",
                format: "@ceil({arg})",
            }),
            builtins::BuiltinCall::MathRound => Some(MathOneArgDesc {
                name: "round",
                format: "@round({arg})",
            }),
            builtins::BuiltinCall::MathSqrt => Some(MathOneArgDesc {
                name: "sqrt",
                format: "@sqrt({arg})",
            }),
            // Trig: @fn(@as(f64, @floatFromInt(x)))
            builtins::BuiltinCall::MathSin => Some(MathOneArgDesc {
                name: "sin",
                format: "@sin(@as(f64, @floatFromInt({arg})))",
            }),
            builtins::BuiltinCall::MathCos => Some(MathOneArgDesc {
                name: "cos",
                format: "@cos(@as(f64, @floatFromInt({arg})))",
            }),
            builtins::BuiltinCall::MathTan => Some(MathOneArgDesc {
                name: "tan",
                format: "@tan(@as(f64, @floatFromInt({arg})))",
            }),
            builtins::BuiltinCall::MathAtan => Some(MathOneArgDesc {
                name: "atan",
                format: "@atan(@as(f64, @floatFromInt({arg})))",
            }),
            // Log: @fn(@as(f64, @floatFromInt(x)))
            builtins::BuiltinCall::MathLog => Some(MathOneArgDesc {
                name: "log",
                format: "@log(@as(f64, @floatFromInt({arg})))",
            }),
            builtins::BuiltinCall::MathLog10 => Some(MathOneArgDesc {
                name: "log10",
                format: "@log10(@as(f64, @floatFromInt({arg})))",
            }),
            builtins::BuiltinCall::MathLog2 => Some(MathOneArgDesc {
                name: "log2",
                format: "@log2(@as(f64, @floatFromInt({arg})))",
            }),
            builtins::BuiltinCall::MathExp => Some(MathOneArgDesc {
                name: "exp",
                format: "@exp(@as(f64, @floatFromInt({arg})))",
            }),
            // std.math with f64 wrapping
            builtins::BuiltinCall::MathAsin => Some(MathOneArgDesc {
                name: "asin",
                format: "std.math.asin(@as(f64, @floatFromInt({arg})))",
            }),
            builtins::BuiltinCall::MathAcos => Some(MathOneArgDesc {
                name: "acos",
                format: "std.math.acos(@as(f64, @floatFromInt({arg})))",
            }),
            // std.math without wrapping
            builtins::BuiltinCall::MathTrunc => Some(MathOneArgDesc {
                name: "trunc",
                format: "@trunc(@as(f64, @floatFromInt({arg})))",
            }),
            builtins::BuiltinCall::MathCbrt => Some(MathOneArgDesc {
                name: "cbrt",
                format: "std.math.cbrt({arg})",
            }),
            builtins::BuiltinCall::MathExpm1 => Some(MathOneArgDesc {
                name: "expm1",
                format: "std.math.expm1({arg})",
            }),
            builtins::BuiltinCall::MathSinh => Some(MathOneArgDesc {
                name: "sinh",
                format: "std.math.sinh({arg})",
            }),
            builtins::BuiltinCall::MathCosh => Some(MathOneArgDesc {
                name: "cosh",
                format: "std.math.cosh({arg})",
            }),
            builtins::BuiltinCall::MathTanh => Some(MathOneArgDesc {
                name: "tanh",
                format: "std.math.tanh({arg})",
            }),
            builtins::BuiltinCall::MathAsinh => Some(MathOneArgDesc {
                name: "asinh",
                format: "std.math.asinh({arg})",
            }),
            builtins::BuiltinCall::MathAcosh => Some(MathOneArgDesc {
                name: "acosh",
                format: "std.math.acosh({arg})",
            }),
            builtins::BuiltinCall::MathAtanh => Some(MathOneArgDesc {
                name: "atanh",
                format: "std.math.atanh({arg})",
            }),
            builtins::BuiltinCall::MathLog1p => Some(MathOneArgDesc {
                name: "log1p",
                format: "std.math.log1p({arg})",
            }),
            _ => None,
        }
    }

    /// Emit a simple 1-arg Math builtin call using a format string.
    fn emit_math_one_arg(&mut self, desc: &MathOneArgDesc, ce: &CallExpression) -> bool {
        if ce.arguments.len() != 1 {
            self.errors
                .push(format!("Math.{}() requires exactly 1 argument", desc.name));
            return false;
        }
        let (prefix, suffix) = desc
            .format
            .split_once("{arg}")
            .expect("invalid math format — missing {arg}");
        self.write(prefix);
        self.emit_first_arg(&ce.arguments);
        self.write(suffix);
        true
    }

    /// Returns the descriptor for simple String runtime forwarding builtins.
    fn string_runtime_desc(b: &builtins::BuiltinCall) -> Option<StringRuntimeDesc> {
        match b {
            // ── No allocator, 0 args, non-fallible ──
            builtins::BuiltinCall::StringTrim => Some(StringRuntimeDesc {
                method: "trim",
                needs_allocator: false,
                is_fallible: false,
                min_args: 0,
                max_args: 0,
                opt_defaults: &[],
            }),
            builtins::BuiltinCall::StringTrimStart => Some(StringRuntimeDesc {
                method: "trimStart",
                needs_allocator: false,
                is_fallible: false,
                min_args: 0,
                max_args: 0,
                opt_defaults: &[],
            }),
            builtins::BuiltinCall::StringTrimEnd => Some(StringRuntimeDesc {
                method: "trimEnd",
                needs_allocator: false,
                is_fallible: false,
                min_args: 0,
                max_args: 0,
                opt_defaults: &[],
            }),
            // ── No allocator, 1 arg, non-fallible ──
            builtins::BuiltinCall::StringIndexOf => Some(StringRuntimeDesc {
                method: "indexOf",
                needs_allocator: false,
                is_fallible: false,
                min_args: 1,
                max_args: 1,
                opt_defaults: &[],
            }),
            builtins::BuiltinCall::StringIncludes => Some(StringRuntimeDesc {
                method: "includes",
                needs_allocator: false,
                is_fallible: false,
                min_args: 1,
                max_args: 1,
                opt_defaults: &[],
            }),
            builtins::BuiltinCall::StringStartsWith => Some(StringRuntimeDesc {
                method: "startsWith",
                needs_allocator: false,
                is_fallible: false,
                min_args: 1,
                max_args: 1,
                opt_defaults: &[],
            }),
            builtins::BuiltinCall::StringEndsWith => Some(StringRuntimeDesc {
                method: "endsWith",
                needs_allocator: false,
                is_fallible: false,
                min_args: 1,
                max_args: 1,
                opt_defaults: &[],
            }),
            builtins::BuiltinCall::StringLastIndexOf => Some(StringRuntimeDesc {
                method: "lastIndexOf",
                needs_allocator: false,
                is_fallible: false,
                min_args: 1,
                max_args: 1,
                opt_defaults: &[],
            }),
            builtins::BuiltinCall::StringCharCodeAt => Some(StringRuntimeDesc {
                method: "charCodeAt",
                needs_allocator: false,
                is_fallible: false,
                min_args: 1,
                max_args: 1,
                opt_defaults: &[],
            }),
            builtins::BuiltinCall::StringCodePointAt => Some(StringRuntimeDesc {
                method: "codePointAt",
                needs_allocator: false,
                is_fallible: false,
                min_args: 1,
                max_args: 1,
                opt_defaults: &[],
            }),
            // ── No allocator, 1-2 args, non-fallible ──
            builtins::BuiltinCall::StringSlice => Some(StringRuntimeDesc {
                method: "slice",
                needs_allocator: false,
                is_fallible: false,
                min_args: 1,
                max_args: 2,
                opt_defaults: &["std.math.maxInt(i64)"],
            }),
            builtins::BuiltinCall::StringSubstring => Some(StringRuntimeDesc {
                method: "substring",
                needs_allocator: false,
                is_fallible: false,
                min_args: 1,
                max_args: 2,
                opt_defaults: &["std.math.maxInt(i64)"],
            }),
            // ── No allocator, 0-1 arg, non-fallible ──
            builtins::BuiltinCall::StringLocaleCompare => Some(StringRuntimeDesc {
                method: "localeCompare",
                needs_allocator: false,
                is_fallible: false,
                min_args: 0,
                max_args: 1,
                opt_defaults: &[],
            }),
            // ── With allocator, 0 args, fallible (returns ![]const u8) ──
            builtins::BuiltinCall::StringToUpperCase => Some(StringRuntimeDesc {
                method: "toUpper",
                needs_allocator: true,
                is_fallible: true,
                min_args: 0,
                max_args: 0,
                opt_defaults: &[],
            }),
            builtins::BuiltinCall::StringToLocaleUpperCase => Some(StringRuntimeDesc {
                method: "toLocaleUpper",
                needs_allocator: true,
                is_fallible: true,
                min_args: 0,
                max_args: 0,
                opt_defaults: &[],
            }),
            builtins::BuiltinCall::StringToLowerCase => Some(StringRuntimeDesc {
                method: "toLower",
                needs_allocator: true,
                is_fallible: true,
                min_args: 0,
                max_args: 0,
                opt_defaults: &[],
            }),
            builtins::BuiltinCall::StringToLocaleLowerCase => Some(StringRuntimeDesc {
                method: "toLocaleLower",
                needs_allocator: true,
                is_fallible: true,
                min_args: 0,
                max_args: 0,
                opt_defaults: &[],
            }),
            // ── With allocator, 1 arg, fallible ──
            builtins::BuiltinCall::StringCharAt => Some(StringRuntimeDesc {
                method: "charAt",
                needs_allocator: true,
                is_fallible: true,
                min_args: 1,
                max_args: 1,
                opt_defaults: &[],
            }),
            builtins::BuiltinCall::StringAt => Some(StringRuntimeDesc {
                method: "at",
                needs_allocator: true,
                is_fallible: true,
                min_args: 1,
                max_args: 1,
                opt_defaults: &[],
            }),
            builtins::BuiltinCall::StringConcat => Some(StringRuntimeDesc {
                method: "concat",
                needs_allocator: true,
                is_fallible: true,
                min_args: 1,
                max_args: 1,
                opt_defaults: &[],
            }),
            builtins::BuiltinCall::StringRepeat => Some(StringRuntimeDesc {
                method: "repeat",
                needs_allocator: true,
                is_fallible: true,
                min_args: 1,
                max_args: 1,
                opt_defaults: &[],
            }),
            // ── With allocator, 1 arg, fallible (returns ![][]const u8) ──
            builtins::BuiltinCall::StringSplit => Some(StringRuntimeDesc {
                method: "split",
                needs_allocator: true,
                is_fallible: true,
                min_args: 1,
                max_args: 1,
                opt_defaults: &[],
            }),
            // ── With allocator, 2 args, fallible ──
            builtins::BuiltinCall::StringPadStart => Some(StringRuntimeDesc {
                method: "padStart",
                needs_allocator: true,
                is_fallible: true,
                min_args: 2,
                max_args: 2,
                opt_defaults: &[],
            }),
            builtins::BuiltinCall::StringPadEnd => Some(StringRuntimeDesc {
                method: "padEnd",
                needs_allocator: true,
                is_fallible: true,
                min_args: 2,
                max_args: 2,
                opt_defaults: &[],
            }),
            builtins::BuiltinCall::StringReplace => Some(StringRuntimeDesc {
                method: "replace",
                needs_allocator: true,
                is_fallible: true,
                min_args: 2,
                max_args: 2,
                opt_defaults: &[],
            }),
            builtins::BuiltinCall::StringReplaceAll => Some(StringRuntimeDesc {
                method: "replaceAll",
                needs_allocator: true,
                is_fallible: true,
                min_args: 2,
                max_args: 2,
                opt_defaults: &[],
            }),
            // ── With allocator, 0-1 arg, fallible ──
            builtins::BuiltinCall::StringNormalize => Some(StringRuntimeDesc {
                method: "normalize",
                needs_allocator: true,
                is_fallible: true,
                min_args: 0,
                max_args: 1,
                opt_defaults: &["\"NFC\""],
            }),
            _ => None,
        }
    }

    /// Emit a simple String runtime forwarding call: `js_string.method(alloc?, obj, ...)`.
    fn emit_string_runtime_call(&mut self, desc: &StringRuntimeDesc, ce: &CallExpression) -> bool {
        let n_args = ce.arguments.len();
        if n_args < desc.min_args || n_args > desc.max_args {
            self.errors.push(if desc.min_args == desc.max_args {
                format!(
                    "String.{}() requires exactly {} argument(s)",
                    desc.method, desc.min_args
                )
            } else {
                format!(
                    "String.{}() requires {}-{} argument(s)",
                    desc.method, desc.min_args, desc.max_args
                )
            });
            return false;
        }
        let Some(obj_repr) = self.callee_object_repr_mut(&ce.callee) else {
            return false;
        };
        if desc.is_fallible {
            self.write("try ");
        }
        self.write(&format!("js_string.{}(", desc.method));
        if desc.needs_allocator {
            self.write("js_allocator.getAllocator(), ");
        }
        self.write(&obj_repr);

        let total_slots = desc.max_args;
        for slot in 0..total_slots {
            if slot < n_args {
                // Emit actual argument
                if let Some(expr) = ce.arguments[slot].as_expression() {
                    self.write(", ");
                    self.emit_expr(expr);
                }
            } else {
                // Emit default for optional slot
                let opt_idx = slot - desc.min_args;
                if let Some(default) = desc.opt_defaults.get(opt_idx)
                    && !default.is_empty()
                {
                    self.write(&format!(", {}", default));
                }
            }
        }
        self.write(")");
        true
    }

    /// Returns the Zig method name for Date setter builtins.
    fn date_setter_method_name(b: &builtins::BuiltinCall) -> Option<&'static str> {
        match b {
            builtins::BuiltinCall::DateSetFullYear => Some("setFullYear"),
            builtins::BuiltinCall::DateSetMonth => Some("setMonth"),
            builtins::BuiltinCall::DateSetDate => Some("setDate"),
            builtins::BuiltinCall::DateSetHours => Some("setHours"),
            builtins::BuiltinCall::DateSetMinutes => Some("setMinutes"),
            builtins::BuiltinCall::DateSetSeconds => Some("setSeconds"),
            builtins::BuiltinCall::DateSetMilliseconds => Some("setMilliseconds"),
            builtins::BuiltinCall::DateSetUTCFullYear => Some("setUTCFullYear"),
            builtins::BuiltinCall::DateSetUTCMonth => Some("setUTCMonth"),
            builtins::BuiltinCall::DateSetUTCDate => Some("setUTCDate"),
            builtins::BuiltinCall::DateSetUTCHours => Some("setUTCHours"),
            builtins::BuiltinCall::DateSetUTCMinutes => Some("setUTCMinutes"),
            builtins::BuiltinCall::DateSetUTCSeconds => Some("setUTCSeconds"),
            builtins::BuiltinCall::DateSetUTCMilliseconds => Some("setUTCMilliseconds"),
            _ => None,
        }
    }

    /// Emit a Date setter method call: `obj.setXxx(args...)`.
    fn emit_date_setter_method(&mut self, method: &str, ce: &CallExpression) -> bool {
        if let Expression::StaticMemberExpression(mem) = &ce.callee {
            self.emit_expr(&mem.object);
            self.write(&format!(".{method}("));
            self.emit_comma_separated_args(&ce.arguments);
            self.write(")");
            true
        } else {
            self.errors.push(format!(
                "Date.{method}() called on non-static-member expression"
            ));
            false
        }
    }

    /// Emit Zig code for a built-in object call
    /// Returns true if the call was handled, false otherwise
    fn emit_builtin_call(&mut self, builtin: &builtins::BuiltinCall, ce: &CallExpression) -> bool {
        match builtin {
            // ── Math methods ─────────────────────────────
            builtins::BuiltinCall::MathRandom => {
                // Math.random() → @as(f64, @floatFromInt(std.crypto.random.int(u64))) / @as(f64, std.math.maxInt(u64))
                // Simplified: use std.time.timestamp() for now
                if !ce.arguments.is_empty() {
                    self.errors
                        .push("Math.random() requires no arguments".to_string());
                    return false;
                }
                self.write("(@as(f64, @floatFromInt(std.crypto.random.int(u32))) / @as(f64, 4294967295.0))");
                true
            }

            builtins::BuiltinCall::MathPow => {
                // Math.pow(base, exp) → std.math.pow(f64, base, exp)
                if ce.arguments.len() != 2 {
                    self.errors
                        .push("Math.pow() requires exactly 2 arguments".to_string());
                    return false;
                }
                self.write("std.math.pow(f64, ");
                self.emit_first_arg(&ce.arguments);
                self.write(", ");
                if let Some(arg) = ce.arguments.get(1)
                    && let Some(expr) = arg.as_expression()
                {
                    self.emit_expr(expr);
                }
                self.write(")");
                true
            }

            builtins::BuiltinCall::MathMax => {
                match ce.arguments.len() {
                    // Math.max() → -Infinity (minInt(i64))
                    0 => {
                        self.write("@as(i64, -9223372036854775808)");
                        true
                    }
                    // Math.max(x) or Math.max(...args) → if spread, iterate; else return x
                    1 => {
                        let single = &ce.arguments[0];
                        if matches!(single, Argument::SpreadElement(_)) {
                            let blk = self.next_label();
                            self.write(&format!(
                                "({}: {{ var __max: i64 = @as(i64, -9223372036854775808); for (",
                                blk
                            ));
                            self.emit_expr_arg(single);
                            self.write(&format!(") |item| {{ if (item > __max) __max = item; }} break :{} __max; }})", blk));
                        } else {
                            self.write("@as(i64, ");
                            self.emit_expr_arg(single);
                            self.write(")");
                        }
                        true
                    }
                    // Math.max(a, b, ...) → find maximum of all arguments
                    _ => {
                        let blk = self.next_label();
                        self.write(&format!("({}: {{ var __max = @as(i64, ", blk));
                        self.emit_first_arg(&ce.arguments);
                        self.write("); ");
                        for (i, arg) in ce.arguments.iter().enumerate() {
                            if i == 0 {
                                continue;
                            }
                            if let Some(expr) = arg.as_expression() {
                                self.write("if (");
                                let arg_str = self.emit_expr_to_string(expr);
                                self.write(&format!(
                                    "@as(i64, {}) > __max) __max = @as(i64, {}); ",
                                    arg_str, arg_str
                                ));
                            }
                        }
                        self.write(&format!(" break :{} __max; }})", blk));
                        true
                    }
                }
            }

            builtins::BuiltinCall::MathMin => {
                match ce.arguments.len() {
                    // Math.min() → +Infinity (maxInt(i64))
                    0 => {
                        self.write("@as(i64, 9223372036854775807)");
                        true
                    }
                    // Math.min(x) or Math.min(...args) → if spread, iterate; else return x
                    1 => {
                        let single = &ce.arguments[0];
                        if matches!(single, Argument::SpreadElement(_)) {
                            let blk = self.next_label();
                            self.write(&format!(
                                "({}: {{ var __min: i64 = @as(i64, 9223372036854775807); for (",
                                blk
                            ));
                            self.emit_expr_arg(single);
                            self.write(&format!(") |item| {{ if (item < __min) __min = item; }} break :{} __min; }})", blk));
                        } else {
                            self.write("@as(i64, ");
                            self.emit_expr_arg(single);
                            self.write(")");
                        }
                        true
                    }
                    // Math.min(a, b, ...) → find minimum of all arguments
                    _ => {
                        let blk = self.next_label();
                        self.write(&format!("({}: {{ var __min = @as(i64, ", blk));
                        self.emit_first_arg(&ce.arguments);
                        self.write("); ");
                        for (i, arg) in ce.arguments.iter().enumerate() {
                            if i == 0 {
                                continue;
                            }
                            if let Some(expr) = arg.as_expression() {
                                self.write("if (");
                                let arg_str = self.emit_expr_to_string(expr);
                                self.write(&format!(
                                    "@as(i64, {}) < __min) __min = @as(i64, {}); ",
                                    arg_str, arg_str
                                ));
                            }
                        }
                        self.write(&format!(" break :{} __min; }})", blk));
                        true
                    }
                }
            }

            builtins::BuiltinCall::MathHypot => {
                // Math.hypot(...args) → sqrt(sum of squares)
                // JS semantics: Math.hypot() = 0, Math.hypot(x) = |x|,
                // Math.hypot(x, y, ...) = sqrt(x² + y² + ...)
                if ce.arguments.is_empty() {
                    self.write("0");
                } else if ce.arguments.len() == 1 {
                    // Math.hypot(x) → @abs(@as(f64, x))
                    self.write("@abs(@as(f64, ");
                    self.emit_first_arg(&ce.arguments);
                    self.write("))");
                } else {
                    // Math.hypot(x, y, ...) → @sqrt(@as(f64,x)*@as(f64,x) + ...)
                    self.write("@sqrt(");
                    for (i, arg) in ce.arguments.iter().enumerate() {
                        if i > 0 {
                            self.write(" + ");
                        }
                        if let Some(expr) = arg.as_expression() {
                            let arg_str = self.emit_expr_to_string(expr);
                            self.write(&format!("@as(f64, {0})*@as(f64, {0})", arg_str));
                        }
                    }
                    self.write(")");
                }
                true
            }

            // ── Math trig ─────────────────────────────
            builtins::BuiltinCall::MathAtan2 => {
                // Math.atan2(y, x) → std.math.atan2(f64, y, x)
                if ce.arguments.len() != 2 {
                    self.errors
                        .push("Math.atan2() requires exactly 2 arguments".to_string());
                    return false;
                }
                self.write("std.math.atan2(f64, ");
                self.emit_first_arg(&ce.arguments);
                self.write(", ");
                if let Some(arg) = ce.arguments.get(1)
                    && let Some(expr) = arg.as_expression()
                {
                    self.emit_expr(expr);
                }
                self.write(")");
                true
            }

            // ── Math log / other ──────────────────────
            builtins::BuiltinCall::MathSign => {
                // Math.sign(x) → @select(f64, @as(f64, @floatFromInt(x)) > 0, 1.0, ...)
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("Math.sign() requires exactly 1 argument".to_string());
                    return false;
                }
                // Equivalent JS: if x>0 return 1, if x<0 return -1, else return x
                self.write("(if (@as(f64, @floatFromInt(");
                self.emit_first_arg(&ce.arguments);
                self.write(")) > 0) @as(f64, 1.0) else if (@as(f64, @floatFromInt(");
                self.emit_first_arg(&ce.arguments);
                self.write(")) < 0) @as(f64, -1.0) else @as(f64, @floatFromInt(");
                self.emit_first_arg(&ce.arguments);
                self.write(")))");
                true
            }
            // ── Phase 4 Math methods ─────────────────────
            builtins::BuiltinCall::MathClz32 => {
                // Math.clz32(x) → @clz(@as(u32, @bitCast(@as(i32, @intFromFloat(x)))))
                // JavaScript: convert to 32-bit int, then count leading zeros
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("Math.clz32() requires exactly 1 argument".to_string());
                    return false;
                }
                self.write("@clz(@as(u32, @bitCast(@as(i32, @intFromFloat(");
                self.emit_first_arg(&ce.arguments);
                self.write(")))))");
                true
            }

            builtins::BuiltinCall::MathFround => {
                // Math.fround(x) → @as(f32, @floatFromInt(x))
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("Math.fround() requires exactly 1 argument".to_string());
                    return false;
                }
                self.write("@as(f32, @floatFromInt(");
                self.emit_first_arg(&ce.arguments);
                self.write("))");
                true
            }

            builtins::BuiltinCall::MathImul => {
                // Math.imul(a, b) → @as(i32, @intCast((@as(u32, @bitCast(@as(i32, a)))) *% (@as(u32, @bitCast(@as(i32, b))))))
                if ce.arguments.len() != 2 {
                    self.errors
                        .push("Math.imul() requires exactly 2 arguments".to_string());
                    return false;
                }
                self.write("@as(i32, @intCast((");
                // First argument: convert to i32, then to u32 for wrapping multiplication
                self.write("@as(u32, @bitCast(@as(i32, ");
                self.emit_first_arg(&ce.arguments);
                self.write("))) *% (");
                // Second argument: same conversion
                self.write("@as(u32, @bitCast(@as(i32, ");
                if let Some(arg1) = ce.arguments.get(1)
                    && let Some(expr) = arg1.as_expression()
                {
                    self.emit_expr(expr);
                }
                self.write("))))");
                true
            }

            // ── Math 1-arg (table-driven, 25 methods) ─────
            b if Self::math_one_arg_desc(b).is_some() => {
                let desc = Self::math_one_arg_desc(b).unwrap();
                self.emit_math_one_arg(&desc, ce)
            }

            // ── String runtime forwarding (27 methods) ────
            b if Self::string_runtime_desc(b).is_some() => {
                let desc = Self::string_runtime_desc(b).unwrap();
                self.emit_string_runtime_call(&desc, ce)
            }

            builtins::BuiltinCall::ArrayPush => {
                // arr.push(x) → arr.append(alloc, x) catch @panic("OOM")
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    self.write(&format!(
                        "{}.append(js_allocator.getAllocator(), ",
                        obj_name
                    ));
                    self.emit_comma_separated_args(&ce.arguments);
                    self.write(") catch @panic(\"OOM: Array.push\")");
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArrayPop => {
                // arr.pop() → _ = arr.pop(); (Zig 0.16.0: pop() returns ?T, no popOrNull)
                // In return context, skip the _ = prefix.
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    if !self.in_return_expr {
                        self.write("_ = ");
                    }
                    self.write(&format!("{}.pop()", obj_name));
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArrayShift => {
                // arr.shift() → if (arr.items.len > 0) _ = arr.orderedRemove(0);
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    self.write(&format!(
                        "if ({obj}.items.len > 0) _ = {obj}.orderedRemove(0)",
                        obj = obj_name
                    ));
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArrayUnshift => {
                // arr.unshift(x) → arr.insert(alloc, 0, x) catch @panic("OOM: Array.unshift insert")
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    self.write(&format!(
                        "{}.insert(js_allocator.getAllocator(), 0, ",
                        obj_name
                    ));
                    self.emit_comma_separated_args(&ce.arguments);
                    self.write(") catch @panic(\"OOM: allocation\")");
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArrayReverse => {
                // arr.reverse() → std.mem.reverse(T, arr.items);
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    let elem_ty = self
                        .type_info
                        .array_element_types
                        .get(obj_name)
                        .map(|t| match t {
                            ZigType::I64 => "i64",
                            ZigType::F64 => "f64",
                            ZigType::Bool => "bool",
                            _ => "i64",
                        })
                        .unwrap_or("i64");
                    self.write(&format!("std.mem.reverse({}, {}.items)", elem_ty, obj_name));
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArraySort => {
                // arr.sort() → std.mem.sort(T, arr.items, {{}}, comptime std.sort.asc(T));
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    let elem_ty = self
                        .type_info
                        .array_element_types
                        .get(obj_name)
                        .map(|t| match t {
                            ZigType::I64 => "i64",
                            ZigType::F64 => "f64",
                            ZigType::Bool => "bool",
                            _ => "i64",
                        })
                        .unwrap_or("i64");
                    self.write(&format!(
                        "std.mem.sort({}, {}.items, {{}}, comptime std.sort.asc({}))",
                        elem_ty, obj_name, elem_ty
                    ));
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArrayIndexOf => {
                // arr.indexOf(x) → labeled block with loop
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("Array.indexOf() requires exactly 1 argument".to_string());
                    return false;
                }
                // Redirect to String.indexOf if the object variable is a string type
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    if self.type_info.var_types.get(obj_name) == Some(&ZigType::Str) {
                        // Treat as string indexOf
                        let arg_expr = self.first_arg_string(&ce.arguments);
                        self.write(&format!(
                            "(if (std.mem.indexOf(u8, {obj}, {arg})) |idx| @as(i64, @intCast(idx)) else @as(i64, -1))",
                            obj = obj_name,
                            arg = arg_expr
                        ));
                        return true;
                    }
                    let arg_expr = self.first_arg_string(&ce.arguments);
                    let blk = self.next_label();
                    self.write(&format!(
                            "({blk}: {{ for ({obj}.items, 0..) |item, i| {{ if (item == {arg}) break :{blk} @as(i64, @intCast(i)); }} break :{blk} @as(i64, -1); }})",
                            blk = blk,
                            obj = obj_name,
                            arg = arg_expr
                        ));
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArrayIncludes => {
                // arr.includes(x) → labeled block with loop
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("Array.includes() requires exactly 1 argument".to_string());
                    return false;
                }
                // Try Identifier fast path first, then fallback to complex expression
                let obj_repr = match self.callee_object_name(&ce.callee) {
                    Some(name) => Some(name.to_string()),
                    None => self.callee_object_repr_mut(&ce.callee),
                };
                if let Some(obj_name) = obj_repr {
                    // Redirect to String.includes if the object variable is a string type
                    if self.type_info.var_types.get(obj_name.as_str()) == Some(&ZigType::Str) {
                        // Treat as string includes
                        let arg_expr = self.first_arg_string(&ce.arguments);
                        self.write(&format!(
                            "(std.mem.indexOf(u8, {obj}, {arg}) != null)",
                            obj = obj_name,
                            arg = arg_expr
                        ));
                        return true;
                    }
                    let arg_expr = self.first_arg_string(&ce.arguments);
                    let blk = self.next_label();
                    self.write(&format!(
                            "({blk}: {{ for ({obj}.items) |item| {{ if (item == {arg}) break :{blk} true; }} break :{blk} false; }})",
                            blk = blk,
                            obj = obj_name,
                            arg = arg_expr
                        ));
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArrayJoin => {
                // arr.join(sep) → labeled block with std.io.Writer.Allocating
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("Array.join() requires exactly 1 argument".to_string());
                    return false;
                }
                // Try Identifier fast path first, then fallback to complex expression
                // (e.g., arr.map(fn).join(',') where object is a CallExpression)
                let obj_repr = match self.callee_object_name(&ce.callee) {
                    Some(name) => Some(name.to_string()),
                    None => self.callee_object_repr_mut(&ce.callee),
                };
                if let Some(obj_name) = obj_repr {
                    let sep_expr = self.first_arg_string(&ce.arguments);
                    // Determine format specifier from array element type
                    let fmt_spec = match self.type_info.array_element_types.get(obj_name.as_str()) {
                        Some(ZigType::I64) => "{d}",
                        Some(ZigType::F64) => "{d}",
                        Some(ZigType::Bool) => "{}",
                        Some(ZigType::Str) => "{s}",
                        _ => "{any}",
                    };
                    let blk = self.next_label();
                    self.write(&format!(
                            "({blk}: {{ var __join_buf = std.io.Writer.Allocating.init(js_allocator.getAllocator()); for ({obj}.items, 0..) |__item, __i| {{ if (__i > 0) __join_buf.writer().writeAll({sep}) catch break :{blk} \"\"; __join_buf.writer().print(\"{fmt}\", .{{__item}}) catch break :{blk} \"\"; }} break :{blk} __join_buf.toOwnedSlice() catch \"\"; }})",
                            blk = blk,
                            obj = obj_name,
                            sep = sep_expr,
                            fmt = fmt_spec
                        ));
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArraySlice => {
                // Check if this is a TypedArray slice
                let obj_name = self.callee_object_name(&ce.callee);
                if let (Some(obj_name), Some(ta_type)) = (
                    obj_name,
                    obj_name.and_then(|n| self.typedarray_vars.get(n).cloned()),
                ) {
                    // TypedArray slice: js_typedarray.sliceXXX(arr, start, end)
                    let start_expr = if !ce.arguments.is_empty() {
                        self.first_arg_string_or(&ce.arguments, "0")
                    } else {
                        "0".to_string()
                    };
                    let end_expr = if ce.arguments.len() >= 2 {
                        if let Some(arg) = ce.arguments.get(1)
                            && let Some(expr) = arg.as_expression()
                        {
                            self.emit_expr_to_string(expr)
                        } else {
                            "std.math.maxInt(i64)".to_string()
                        }
                    } else {
                        "std.math.maxInt(i64)".to_string()
                    };
                    self.write(&format!(
                        "js_runtime.js_typedarray.slice{}({}, {}, {})",
                        ta_type, obj_name, start_expr, end_expr
                    ));
                    return true;
                }
                // arr.slice(start, end) → new ArrayList with appended slice
                // arr.slice(start) → new ArrayList with appended slice
                // arr.slice() → new ArrayList clone
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    // Get element type for the new ArrayList
                    let elem_type = self
                        .type_info
                        .array_element_types
                        .get(obj_name)
                        .map(|t| t.to_zig_type())
                        .unwrap_or_else(|| "i64".to_string());
                    let slice_expr = match ce.arguments.len() {
                        0 => format!("{}.items", obj_name),
                        1 => {
                            let arg_expr = self.first_arg_string_or(&ce.arguments, "0");
                            format!("{}.items[{}..]", obj_name, arg_expr)
                        }
                        2 => {
                            let start_expr = self.first_arg_string_or(&ce.arguments, "0");
                            let end_expr = if let Some(arg) = ce.arguments.get(1) {
                                if let Some(expr) = arg.as_expression() {
                                    self.emit_expr_to_string(expr)
                                } else {
                                    "0".to_string()
                                }
                            } else {
                                "0".to_string()
                            };
                            format!("{}.items[{}..{}]", obj_name, start_expr, end_expr)
                        }
                        _ => {
                            self.errors
                                .push("Array.slice() requires 0-2 arguments".to_string());
                            return false;
                        }
                    };
                    let blk = self.next_label();
                    self.write(&format!(
                        "({0}: {{ var __slice: std.ArrayList({1}) = .empty; __slice.appendSlice(js_allocator.getAllocator(), {2}) catch @panic(\"OOM: Array.slice appendSlice\"); break :{0} __slice; }})",
                        blk, elem_type, slice_expr
                    ));
                    return true;
                }
                false
            }

            // ── TypedArray methods (non-overlapping) ────
            builtins::BuiltinCall::TypedArraySubarray => {
                // arr.subarray(start, end) → js_typedarray.subarrayXXX(arr, start, end)
                // subarray is an alias for slice in the runtime
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    let ta_type = self.typedarray_vars.get(obj_name).cloned();
                    if let Some(ta_type) = ta_type {
                        let start_expr = self.first_arg_string_or(&ce.arguments, "0");
                        let end_expr = if ce.arguments.len() >= 2 {
                            if let Some(arg) = ce.arguments.get(1)
                                && let Some(expr) = arg.as_expression()
                            {
                                self.emit_expr_to_string(expr)
                            } else {
                                "std.math.maxInt(i64)".to_string()
                            }
                        } else {
                            "std.math.maxInt(i64)".to_string()
                        };
                        self.write(&format!(
                            "js_runtime.js_typedarray.subarray{}({}, {}, {})",
                            ta_type, obj_name, start_expr, end_expr
                        ));
                        return true;
                    }
                }
                false
            }

            builtins::BuiltinCall::ArraySplice => {
                // arr.splice(start, deleteCount, ...items)
                // Returns removed elements as a new ArrayList.
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    if ce.arguments.len() < 2 {
                        self.errors.push(
                            "Array.splice() requires at least 2 arguments (start, deleteCount)"
                                .to_string(),
                        );
                        return false;
                    }
                    let elem_type = self
                        .type_info
                        .array_element_types
                        .get(obj_name)
                        .map(|t| t.to_zig_type())
                        .unwrap_or_else(|| "i64".to_string());

                    let start_expr = self.first_arg_string_or(&ce.arguments, "0");
                    let count_expr = if let Some(arg) = ce.arguments.get(1) {
                        if let Some(e) = arg.as_expression() {
                            self.emit_expr_to_string(e)
                        } else {
                            "0".to_string()
                        }
                    } else {
                        "0".to_string()
                    };

                    let blk = self.next_label();
                    self.write(&format!(
                        "({0}: {{ var __spliced: std.ArrayList({1}) = .empty; const __start = @as(usize, @intCast(@max(0, {2}))); const __cnt = @as(usize, @intCast(@min(@max(0, {3}), {4}.items.len -| __start))); var __i: usize = 0; while (__i < __cnt) : (__i += 1) {{ __spliced.append(js_allocator.getAllocator(), {4}.orderedRemove(__start)) catch @panic(\"OOM: Array.splice\"); }}", 
                        blk, elem_type, start_expr, count_expr, obj_name
                    ));
                    // Insert new items if any (args beyond index 1)
                    if ce.arguments.len() > 2 {
                        // Use insertSlice for better performance
                        self.write(&format!(
                            " {0}.insertSlice(js_allocator.getAllocator(), __start, &[_]{1}{{",
                            obj_name, elem_type
                        ));
                        for (i, arg) in ce.arguments.iter().enumerate() {
                            if i < 2 {
                                continue;
                            }
                            if i > 2 {
                                self.write(", ");
                            }
                            if let Some(expr) = arg.as_expression() {
                                self.emit_expr(expr);
                            }
                        }
                        self.write("}) catch @panic(\"OOM: Array.splice insertSlice\");");
                    }
                    self.write(&format!(" break :{} __spliced; }})", blk));
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArrayConcat => {
                // arr.concat(other) → new ArrayList with items from both
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    let elem_type = self
                        .type_info
                        .array_element_types
                        .get(obj_name)
                        .map(|t| t.to_zig_type())
                        .unwrap_or_else(|| "i64".to_string());
                    // Generate: (blk_N: {
                    //   var __concat: std.ArrayList(T) = .empty;
                    //   __concat.appendSlice(alloc, arr.items) catch @panic("OOM");
                    //   __concat.appendSlice(alloc, other.items) catch @panic("OOM");
                    //   break :blk_N __concat;
                    // })
                    let blk = self.next_label();
                    self.write(&format!("({}: {{ ", blk));
                    self.write(&format!(
                        "var __concat: std.ArrayList({0}) = .empty; ",
                        elem_type
                    ));
                    // Append original array's items
                    self.write(&format!(
                        "__concat.appendSlice(js_allocator.getAllocator(), {}.items) catch @panic(\"OOM: Array.concat appendSlice\"); ",
                        obj_name
                    ));
                    // Append each additional argument's items
                    for arg in &ce.arguments {
                        if let Some(expr) = arg.as_expression() {
                            let arg_str = self.emit_expr_to_string(expr);
                            self.write(&format!(
                                "__concat.appendSlice(js_allocator.getAllocator(), {}.items) catch @panic(\"OOM: Array.concat appendSlice\"); ",
                                arg_str
                            ));
                        }
                    }
                    self.write(&format!("break :{} __concat; }})", blk));
                    return true;
                }
                false
            }

            // ── Array iterator methods ─────────────────────────────
            builtins::BuiltinCall::ArrayKeys => {
                // arr.keys() → js_runtime.js_array.keys(js_allocator.getAllocator(), &arr) catch @panic(...)
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    self.write(&format!(
                        "js_runtime.js_array.keys(js_allocator.getAllocator(), &{}) catch @panic(\"OOM: allocation\")",
                        obj_name
                    ));
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArrayValues => {
                // arr.values() → js_runtime.js_array.values(js_allocator.getAllocator(), &arr) catch @panic(...)
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    self.write(&format!(
                        "js_runtime.js_array.values(js_allocator.getAllocator(), &{}) catch @panic(\"OOM: allocation\")",
                        obj_name
                    ));
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArrayEntries => {
                // arr.entries() → js_runtime.js_array.entries(js_allocator.getAllocator(), &arr) catch @panic(...)
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    self.write(&format!(
                        "js_runtime.js_array.entries(js_allocator.getAllocator(), &{}) catch @panic(\"OOM: allocation\")",
                        obj_name
                    ));
                    return true;
                }
                false
            }

            // ── Map methods (also TypedArray .get/.set) ──
            builtins::BuiltinCall::MapSet => {
                // TypedArray.set(idx, val) → js_typedarray.setXXX(arr, idx, val)
                if let Expression::StaticMemberExpression(mem) = &ce.callee
                    && let Expression::Identifier(id) = &mem.object
                {
                    let ta_type = self.typedarray_vars.get(id.name.as_str()).cloned();
                    if let Some(ta_type) = ta_type {
                        if ce.arguments.len() != 2 {
                            self.errors
                                .push("TypedArray.set() requires exactly 2 arguments".to_string());
                            return false;
                        }
                        // Note: `_ =` prefix is handled by the expression statement handler.
                        self.write(&format!("js_runtime.js_typedarray.set{}(", ta_type));
                        self.emit_expr(&mem.object);
                        self.write(", ");
                        self.emit_first_arg(&ce.arguments);
                        self.write(", ");
                        if let Some(arg) = ce.arguments.get(1)
                            && let Some(expr) = arg.as_expression()
                        {
                            self.emit_expr(expr);
                        }
                        self.write(")");
                        return true;
                    }
                }
                // map.set(key, value) → map.set(JsAny.from(key), JsAny.from(value)) catch @panic("OOM: Map.set")
                if ce.arguments.len() != 2 {
                    self.errors
                        .push("Map.set() requires exactly 2 arguments".to_string());
                    return false;
                }
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    self.write(&format!("{}.set(JsAny.from(", obj_name));
                    // Emit key
                    self.emit_first_arg(&ce.arguments);
                    self.write("), JsAny.from(");
                    // Emit value
                    if let Some(arg) = ce.arguments.get(1)
                        && let Some(expr) = arg.as_expression()
                    {
                        self.emit_expr(expr);
                    }
                    self.write(")) catch @panic(\"OOM: allocation\")");
                    return true;
                }
                false
            }

            builtins::BuiltinCall::MapGet => {
                // TypedArray.get(idx) → js_typedarray.getXXX(arr, idx)
                if let Expression::StaticMemberExpression(mem) = &ce.callee
                    && let Expression::Identifier(id) = &mem.object
                {
                    let ta_type = self.typedarray_vars.get(id.name.as_str()).cloned();
                    if let Some(ta_type) = ta_type {
                        if ce.arguments.len() != 1 {
                            self.errors
                                .push("TypedArray.get() requires exactly 1 argument".to_string());
                            return false;
                        }
                        self.write(&format!("js_runtime.js_typedarray.get{}(", ta_type));
                        self.emit_expr(&mem.object);
                        self.write(", ");
                        self.emit_first_arg(&ce.arguments);
                        self.write(")");
                        return true;
                    }
                }
                // map.get(key) → map.get(JsAny.from(key))  (returns ?JsAny)
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("Map.get() requires exactly 1 argument".to_string());
                    return false;
                }
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    // map.get(key) → map.get(JsAny.from(key))  (returns JsAny, not ?JsAny)
                    self.write(&format!("{}.get(JsAny.from(", obj_name));
                    self.emit_first_arg(&ce.arguments);
                    self.write("))");
                    return true;
                }
                false
            }

            builtins::BuiltinCall::MapHas => {
                // map.has(key) → map.has(JsAny.from(key))
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("Map.has() requires exactly 1 argument".to_string());
                    return false;
                }
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    self.write(&format!("{}.has(JsAny.from(", obj_name));
                    self.emit_first_arg(&ce.arguments);
                    self.write("))");
                    return true;
                }
                false
            }

            builtins::BuiltinCall::MapDelete => {
                // map.delete(key) → map.delete(JsAny.from(key))
                // Note: `_ =` prefix is handled by the expression statement handler.
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("Map.delete() requires exactly 1 argument".to_string());
                    return false;
                }
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    self.write(&format!("{}.delete(JsAny.from(", obj_name));
                    self.emit_first_arg(&ce.arguments);
                    self.write("))");
                    return true;
                }
                false
            }
            // ── Map iterator methods ──
            builtins::BuiltinCall::MapKeys => {
                // map.keys() → map.keys(js_allocator.getAllocator()) catch @panic(...)
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    self.write(&format!(
                        "{}.keys(js_allocator.getAllocator()) catch @panic(\"OOM: allocation\")",
                        obj_name
                    ));
                    return true;
                }
                false
            }
            builtins::BuiltinCall::MapValues => {
                // map.values() → map.values(js_allocator.getAllocator()) catch @panic(...)
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    self.write(&format!(
                        "{}.values(js_allocator.getAllocator()) catch @panic(\"OOM: allocation\")",
                        obj_name
                    ));
                    return true;
                }
                false
            }
            builtins::BuiltinCall::MapEntries => {
                // map.entries() → map.entries(js_allocator.getAllocator()) catch @panic(...)
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    self.write(&format!(
                        "{}.entries(js_allocator.getAllocator()) catch @panic(\"OOM: allocation\")",
                        obj_name
                    ));
                    return true;
                }
                false
            }

            // ── Set methods ─────────────────────────────
            builtins::BuiltinCall::SetAdd => {
                // set.add(value) → set.add(JsAny.from(value)) catch @panic("OOM: Set.add")
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("Set.add() requires exactly 1 argument".to_string());
                    return false;
                }
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    self.write(&format!("{}.add(JsAny.from(", obj_name));
                    self.emit_first_arg(&ce.arguments);
                    self.write(")) catch @panic(\"OOM: allocation\")");
                    return true;
                }
                false
            }
            // ── Set iterator methods ──
            builtins::BuiltinCall::SetForEach => {
                // set.forEach(fn) → for (set.items.items) |value| { ... }
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    if !ce.arguments.is_empty()
                        && let Some(arg) = ce.arguments.first()
                        && let Some(Expression::ArrowFunctionExpression(arrow)) =
                            arg.as_expression()
                    {
                        // Set.forEach((value) => { ... }) — JS callback: value, value, set
                        let val_param = arrow
                            .params
                            .items
                            .first()
                            .and_then(|p| crate::native_proto::infer::binding_name(&p.pattern));

                        let val_name = val_param.unwrap_or("_item");
                        self.write(&format!(
                            "for ({obj}.items.items) |{val}| {{\n",
                            obj = obj_name,
                            val = val_name
                        ));
                        self.indent += 1;

                        // Emit arrow function body
                        for stmt in &arrow.body.statements {
                            self.write_indent();
                            self.emit_fn_stmt(stmt);
                        }

                        if let Some(vp) = &val_param {
                            self.write_indent();
                            self.write(&format!("_ = &{};\n", vp));
                        }

                        self.indent -= 1;
                        self.write_indent();
                        self.write("}");
                        return true;
                    }
                    // Fallback: empty Set.forEach
                    self.write(&format!("for ({}.items.items) |_| {{}}", obj_name));
                    return true;
                }
                false
            }
            builtins::BuiltinCall::SetKeys => {
                // set.keys() → set.keys(js_allocator.getAllocator()) catch @panic(...)
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    self.write(&format!(
                        "{}.keys(js_allocator.getAllocator()) catch @panic(\"OOM: allocation\")",
                        obj_name
                    ));
                    return true;
                }
                false
            }
            builtins::BuiltinCall::SetValues => {
                // set.values() → set.values(js_allocator.getAllocator()) catch @panic(...)
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    self.write(&format!(
                        "{}.values(js_allocator.getAllocator()) catch @panic(\"OOM: allocation\")",
                        obj_name
                    ));
                    return true;
                }
                false
            }
            builtins::BuiltinCall::SetEntries => {
                // set.entries() → set.entries(js_allocator.getAllocator()) catch @panic(...)
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    self.write(&format!(
                        "{}.entries(js_allocator.getAllocator()) catch @panic(\"OOM: allocation\")",
                        obj_name
                    ));
                    return true;
                }
                false
            }

            // ── Array methods (with closure) ─────────────────────────────
            // ForEach: generate for/while loop that inlines the callback body
            builtins::BuiltinCall::ArrayForEach => {
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    // Check if this is a Map.forEach call (detected by variable type)
                    let is_map = self
                        .type_info
                        .var_types
                        .get(obj_name)
                        .is_some_and(|t| matches!(t, ZigType::NamedStruct(s) if s == "Map"));

                    if is_map {
                        // Map.forEach(fn) → while-iterator loop with key/value pairs
                        if !ce.arguments.is_empty()
                            && let Some(arg) = ce.arguments.first()
                            && let Some(Expression::ArrowFunctionExpression(arrow)) =
                                arg.as_expression()
                        {
                            // Map.forEach((value, key) => {...}) — JS callback order
                            let val_param =
                                arrow.params.items.first().and_then(|p| {
                                    crate::native_proto::infer::binding_name(&p.pattern)
                                });
                            let key_param =
                                arrow.params.items.get(1).and_then(|p| {
                                    crate::native_proto::infer::binding_name(&p.pattern)
                                });

                            self.write(&format!("var iter = {}.inner.iterator();\n", obj_name));
                            self.write_indent();
                            self.write("while (iter.next()) |entry| {\n");
                            self.indent += 1;
                            // Bind value and key from entry
                            if let Some(vp) = &val_param {
                                self.write_indent();
                                self.write(&format!("const {} = entry.value_ptr.*;\n", vp));
                            }
                            if let Some(kp) = &key_param {
                                self.write_indent();
                                self.write(&format!("const {} = entry.key_ptr.*;\n", kp));
                            }
                            // Emit callback body
                            for stmt in &arrow.body.statements {
                                self.write_indent();
                                self.emit_fn_stmt(stmt);
                            }
                            // Suppress unused variable warnings: _ = &param
                            // (harmless no-op if param is used; required for zig ast-check)
                            if let Some(kp) = &key_param {
                                self.write_indent();
                                self.write(&format!("_ = &{};\n", kp));
                            }
                            if let Some(vp) = &val_param {
                                self.write_indent();
                                self.write(&format!("_ = &{};\n", vp));
                            }
                            self.indent -= 1;
                            self.write_indent();
                            self.write("}");
                            return true;
                        }
                        // Fallback: empty Map.forEach
                        self.write(&format!("var iter = {}.inner.iterator();\n", obj_name));
                        self.write_indent();
                        self.write("while (iter.next()) |_| {}\n");
                        return true;
                    }

                    // Array.forEach(fn) → for (arr.items) |item| { /* fn body */ }
                    if !ce.arguments.is_empty()
                        && let Some(arg) = ce.arguments.first()
                        && let Some(Expression::ArrowFunctionExpression(arrow)) =
                            arg.as_expression()
                    {
                        // Generate for loop
                        self.write(&format!("for ({}.items) |", obj_name));
                        // Print arrow function parameters
                        if arrow.params.items.len() == 1 {
                            if let Some(param_name) = crate::native_proto::infer::binding_name(
                                &arrow.params.items[0].pattern,
                            ) {
                                self.write(&format!("{}| {{ ", param_name));
                            } else {
                                self.write("_| {{ ");
                            }
                        } else {
                            self.write("_| {{ ");
                        }
                        // Print arrow function body (statements)
                        self.indent += 1;
                        for stmt in &arrow.body.statements {
                            self.write_indent();
                            self.emit_fn_stmt(stmt);
                        }
                        self.indent -= 1;
                        self.write_indent();
                        self.write("}");
                        return true;
                    }
                    // Fallback: empty for loop
                    self.write(&format!("for ({}.items) |_| {{}}", obj_name));
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArrayMap => {
                // arr.map(fn) → arr (simplified: return original array)
                let obj_repr = match self.callee_object_name(&ce.callee) {
                    Some(name) => Some(name.to_string()),
                    None => self.callee_object_repr_mut(&ce.callee),
                };
                if let Some(obj_name) = obj_repr {
                    self.write(&obj_name);
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArrayFilter => {
                // arr.filter(fn) → generate inline for-loop with predicate check
                let obj_repr = match self.callee_object_name(&ce.callee) {
                    Some(name) => Some(name.to_string()),
                    None => self.callee_object_repr_mut(&ce.callee),
                };
                if let Some(obj_name) = obj_repr {
                    let elem_type = self
                        .type_info
                        .array_element_types
                        .get(obj_name.as_str())
                        .map(|t| t.to_zig_type())
                        .unwrap_or_else(|| "i64".to_string());
                    if !ce.arguments.is_empty()
                        && let Some(arg) = ce.arguments.first()
                        && let Some(Expression::ArrowFunctionExpression(arrow)) =
                            arg.as_expression()
                    {
                        // Generate: (blk_N: {
                        //   var __filter: std.ArrayList(T) = .empty;
                        //   for (arr.items) |elem| {
                        //     if (predicate) __filter.append(alloc, elem) catch @panic("OOM");
                        //   }
                        //   break :blk_N __filter;
                        // })
                        let blk = self.next_label();
                        self.write(&format!("({}: {{ ", blk));
                        self.write(&format!(
                            "var __filter: std.ArrayList({0}) = .empty; ",
                            elem_type
                        ));
                        self.write(&format!("for ({}.items) |", obj_name));
                        let param_name = if !arrow.params.items.is_empty() {
                            crate::native_proto::infer::binding_name(&arrow.params.items[0].pattern)
                                .unwrap_or("_")
                                .to_string()
                        } else {
                            "_".to_string()
                        };
                        self.write(&format!("{}| {{ ", param_name));
                        self.indent += 1;
                        for stmt in &arrow.body.statements {
                            self.write_indent();
                            if let Statement::ReturnStatement(ret) = stmt {
                                // Block body: { return predicate; }
                                if let Some(expr) = &ret.argument {
                                    self.write("if (");
                                    self.emit_expr(expr);
                                    self.write(") { __filter.append(js_allocator.getAllocator(), ");
                                    self.write(&param_name);
                                    self.write(") catch @panic(\"OOM: Array.filter append\"); }");
                                }
                            } else if let Statement::ExpressionStatement(es) = stmt {
                                // Concise body: x => predicate
                                self.write("if (");
                                self.emit_expr(&es.expression);
                                self.write(") { __filter.append(js_allocator.getAllocator(), ");
                                self.write(&param_name);
                                self.write(") catch @panic(\"OOM: Array.filter append\"); }");
                            }
                        }
                        self.indent -= 1;
                        self.write_indent();
                        self.write("}");
                        self.write_indent();
                        self.write(&format!("break :{} __filter; }})", blk));
                        return true;
                    }
                    // Fallback: no arrow function argument → return original array
                    self.write(&obj_name);
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArrayReduce => {
                // arr.reduce(fn, init) → generate for loop with accumulator
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    // Get initial value from second argument
                    let init_expr = if ce.arguments.len() >= 2
                        && let Some(arg) = ce.arguments.get(1)
                        && let Some(expr) = arg.as_expression()
                    {
                        self.emit_expr_to_string(expr)
                    } else {
                        "0".to_string()
                    };
                    // Check if first argument is an arrow function
                    if !ce.arguments.is_empty()
                        && let Some(arg) = ce.arguments.first()
                        && let Some(Expression::ArrowFunctionExpression(arrow)) =
                            arg.as_expression()
                    {
                        // Generate labeled block with accumulator
                        let blk = self.next_label();
                        self.write(&format!("({}: {{ ", blk));
                        // Determine accumulator type from init expression
                        let acc_type = if init_expr.contains(".") {
                            "f64"
                        } else {
                            "i64"
                        };
                        self.write(&format!("var acc: {} = {}; ", acc_type, init_expr));
                        // Generate for loop
                        self.write(&format!("for ({}.items) |", obj_name));
                        // Print arrow function parameters
                        if arrow.params.items.len() >= 2 {
                            // First param is accumulator, second is current item
                            if let Some(param_name) = crate::native_proto::infer::binding_name(
                                &arrow.params.items[1].pattern,
                            ) {
                                self.write(&format!("{}| {{ ", param_name));
                            } else {
                                self.write("_| { ");
                            }
                        } else {
                            self.write("_| { ");
                        }
                        // Print arrow function body
                        // arrow.body is FunctionBody, which has .statements field
                        // For concise body (acc, x) => acc + x, oxc converts it to
                        // FunctionBody with a single ExpressionStatement
                        self.indent += 1;
                        for stmt in &arrow.body.statements {
                            self.write_indent();
                            // For return statements in reduce callback, replace "return expr;" with "acc = expr;"
                            if let Statement::ReturnStatement(ret) = stmt {
                                if let Some(expr) = &ret.argument {
                                    self.write("acc = ");
                                    self.emit_expr(expr);
                                    self.write(";");
                                }
                            } else if let Statement::ExpressionStatement(es) = stmt {
                                // Concise body: (acc, x) => acc + x
                                // oxc converts to ExpressionStatement
                                // For reduce, we need to assign the expression to acc
                                self.write("acc = ");
                                self.emit_expr(&es.expression);
                                self.write(";");
                            } else {
                                self.emit_fn_stmt(stmt);
                            }
                        }
                        self.indent -= 1;
                        self.write_indent();
                        self.write("}");
                        self.write_indent();
                        self.write(&format!("break :{} acc; }})", blk));
                        return true;
                    }
                    // Fallback: just return initial value
                    if ce.arguments.len() >= 2
                        && let Some(arg) = ce.arguments.get(1)
                        && let Some(expr) = arg.as_expression()
                    {
                        self.emit_expr(expr);
                        return true;
                    }
                    self.write("0");
                    true
                } else {
                    false
                }
            }

            builtins::BuiltinCall::ArraySome => {
                // arr.some(fn) → generate inline for-loop with short-circuit predicate
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    if !ce.arguments.is_empty()
                        && let Some(arg) = ce.arguments.first()
                        && let Some(Expression::ArrowFunctionExpression(arrow)) =
                            arg.as_expression()
                    {
                        // Generate: (blk: {
                        //   for (arr.items) |elem| { ... }              — 1-param callback
                        //   for (arr.items, 0..) |elem, i| { ... }      — 2-param callback (elem, index)
                        let idx_param = if arrow.params.items.len() >= 2 {
                            crate::native_proto::infer::binding_name(&arrow.params.items[1].pattern)
                                .unwrap_or("_")
                                .to_string()
                        } else {
                            String::new()
                        };
                        let has_idx = !idx_param.is_empty();
                        let blk = self.next_label();
                        self.write(&format!("({}: {{ ", blk));
                        if has_idx {
                            self.write(&format!("for ({}.items, 0..) |", obj_name));
                        } else {
                            self.write(&format!("for ({}.items) |", obj_name));
                        }
                        let param_name = if !arrow.params.items.is_empty() {
                            crate::native_proto::infer::binding_name(&arrow.params.items[0].pattern)
                                .unwrap_or("_")
                                .to_string()
                        } else {
                            "_".to_string()
                        };
                        // If param_name unused in body, replace with "_"
                        let param_name = if !arrow_body_uses_ident(&param_name, arrow) {
                            "_".to_string()
                        } else {
                            param_name
                        };
                        if has_idx {
                            self.write(&format!("{}, {}| {{ ", param_name, idx_param));
                        } else {
                            self.write(&format!("{}| {{ ", param_name));
                        }
                        self.indent += 1;
                        for stmt in &arrow.body.statements {
                            self.write_indent();
                            if let Statement::ReturnStatement(ret) = stmt {
                                if let Some(expr) = &ret.argument {
                                    self.write("if (");
                                    self.emit_expr(expr);
                                    self.write(&format!(") break :{} true;", blk));
                                }
                            } else if let Statement::ExpressionStatement(es) = stmt {
                                self.write("if (");
                                self.emit_expr(&es.expression);
                                self.write(&format!(") break :{} true;", blk));
                            }
                        }
                        self.indent -= 1;
                        self.write_indent();
                        self.write("}");
                        self.write_indent();
                        self.write(&format!("break :{} false; }})", blk));
                        return true;
                    }
                    // Fallback: no arrow function → return false
                    self.write("false");
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArrayEvery => {
                // arr.every(fn) → generate inline for-loop with short-circuit predicate
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    if !ce.arguments.is_empty()
                        && let Some(arg) = ce.arguments.first()
                        && let Some(Expression::ArrowFunctionExpression(arrow)) =
                            arg.as_expression()
                    {
                        // Generate: (blk: {
                        //   for (arr.items) |elem| { ... }              — 1-param callback
                        //   for (arr.items, 0..) |elem, i| { ... }      — 2-param callback (elem, index)
                        let idx_param = if arrow.params.items.len() >= 2 {
                            crate::native_proto::infer::binding_name(&arrow.params.items[1].pattern)
                                .unwrap_or("_")
                                .to_string()
                        } else {
                            String::new()
                        };
                        let has_idx = !idx_param.is_empty();
                        // If idx_param unused in body, replace with "_"
                        let idx_param = if has_idx && !arrow_body_uses_ident(&idx_param, arrow) {
                            "_".to_string()
                        } else {
                            idx_param
                        };
                        let has_idx = !idx_param.is_empty();
                        let blk = self.next_label();
                        self.write(&format!("({}: {{ ", blk));
                        if has_idx {
                            self.write(&format!("for ({}.items, 0..) |", obj_name));
                        } else {
                            self.write(&format!("for ({}.items) |", obj_name));
                        }
                        let param_name = if !arrow.params.items.is_empty() {
                            crate::native_proto::infer::binding_name(&arrow.params.items[0].pattern)
                                .unwrap_or("_")
                                .to_string()
                        } else {
                            "_".to_string()
                        };
                        // If param_name unused in body, replace with "_"
                        let param_name = if !arrow_body_uses_ident(&param_name, arrow) {
                            "_".to_string()
                        } else {
                            param_name
                        };
                        if has_idx {
                            self.write(&format!("{}, {}| {{ ", param_name, idx_param));
                        } else {
                            self.write(&format!("{}| {{ ", param_name));
                        }
                        self.indent += 1;
                        for stmt in &arrow.body.statements {
                            self.write_indent();
                            if let Statement::ReturnStatement(ret) = stmt {
                                if let Some(expr) = &ret.argument {
                                    self.write("if (!(");
                                    self.emit_expr(expr);
                                    self.write(&format!(")) break :{} false;", blk));
                                }
                            } else if let Statement::ExpressionStatement(es) = stmt {
                                self.write("if (!(");
                                self.emit_expr(&es.expression);
                                self.write(&format!(")) break :{} false;", blk));
                            }
                        }
                        self.indent -= 1;
                        self.write_indent();
                        self.write("}");
                        self.write_indent();
                        self.write(&format!("break :{} true; }})", blk));
                        return true;
                    }
                    // Fallback: no arrow function → return true
                    self.write("true");
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArrayFlat => {
                // arr.flat() → arr (identity for i64 arrays)
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    self.write(obj_name);
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArrayFlatMap => {
                // arr.flatMap(fn) → arr (simplified: return original array)
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    self.write(obj_name);
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArrayFind => {
                // arr.find(fn) → inline for-loop, break with element
                // (blk: {
                //   for (arr.items) |elem| {
                //     if (predicate) break :blk elem;
                //   }
                //   break :blk undefined;
                // })
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    if !ce.arguments.is_empty()
                        && let Some(arg) = ce.arguments.first()
                        && let Some(Expression::ArrowFunctionExpression(arrow)) =
                            arg.as_expression()
                    {
                        let blk = self.next_label();
                        self.write(&format!("({}: {{ ", blk));
                        self.write(&format!("for ({}.items) |", obj_name));
                        let param_name = if !arrow.params.items.is_empty() {
                            crate::native_proto::infer::binding_name(&arrow.params.items[0].pattern)
                                .unwrap_or("_")
                                .to_string()
                        } else {
                            "_".to_string()
                        };
                        self.write(&format!("{}| {{ ", param_name));
                        self.indent += 1;
                        for stmt in &arrow.body.statements {
                            self.write_indent();
                            if let Statement::ReturnStatement(ret) = stmt {
                                if let Some(expr) = &ret.argument {
                                    self.write("if (");
                                    self.emit_expr(expr);
                                    self.write(&format!(") break :{} {};", blk, param_name));
                                }
                            } else if let Statement::ExpressionStatement(es) = stmt {
                                self.write("if (");
                                self.emit_expr(&es.expression);
                                self.write(&format!(") break :{} {};", blk, param_name));
                            }
                        }
                        self.indent -= 1;
                        self.write_indent();
                        self.write("}");
                        self.write_indent();
                        self.write(&format!("break :{} undefined; }})", blk));
                        return true;
                    }
                    // Fallback: no arrow function → return undefined
                    self.write("undefined");
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArrayFindIndex => {
                // arr.findIndex(fn) → inline for-loop, break with index
                // (blk: {
                //   for (arr.items, 0..) |param, __i| {
                //     const __idx: i64 = @intCast(__i);
                //     if (predicate) break :blk __idx;
                //   }
                //   break :blk -1;
                // })
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    if !ce.arguments.is_empty()
                        && let Some(arg) = ce.arguments.first()
                        && let Some(Expression::ArrowFunctionExpression(arrow)) =
                            arg.as_expression()
                    {
                        let blk = self.next_label();
                        self.write(&format!("({}: {{ ", blk));
                        self.write(&format!("for ({}.items, 0..) |", obj_name));
                        let param_name = if !arrow.params.items.is_empty() {
                            crate::native_proto::infer::binding_name(&arrow.params.items[0].pattern)
                                .unwrap_or("_")
                                .to_string()
                        } else {
                            "_".to_string()
                        };
                        let index_name = format!("__{}_i", param_name);
                        let idx_name = format!("__{}_idx", param_name);
                        self.write(&format!("{}, {}| {{ ", param_name, index_name));
                        self.indent += 1;
                        self.write_indent();
                        self.write(&format!(
                            "const {}: i64 = @intCast({});\n",
                            idx_name, index_name
                        ));
                        for stmt in &arrow.body.statements {
                            self.write_indent();
                            if let Statement::ReturnStatement(ret) = stmt {
                                if let Some(expr) = &ret.argument {
                                    self.write("if (");
                                    self.emit_expr(expr);
                                    self.write(&format!(") break :{} {};", blk, idx_name));
                                }
                            } else if let Statement::ExpressionStatement(es) = stmt {
                                self.write("if (");
                                self.emit_expr(&es.expression);
                                self.write(&format!(") break :{} {};", blk, idx_name));
                            }
                        }
                        self.indent -= 1;
                        self.write_indent();
                        self.write("}");
                        self.write_indent();
                        self.write(&format!("break :{} -1; }})", blk));
                        return true;
                    }
                    // Fallback: no arrow function → return -1
                    self.write("-1");
                    return true;
                }
                false
            }

            // ── ArrayFindLast ────────────────────────────
            builtins::BuiltinCall::ArrayFindLast => {
                // arr.findLast(fn) → reverse iterator, break with element
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    if !ce.arguments.is_empty()
                        && let Some(arg) = ce.arguments.first()
                        && let Some(Expression::ArrowFunctionExpression(arrow)) =
                            arg.as_expression()
                    {
                        let param_name = if !arrow.params.items.is_empty() {
                            crate::native_proto::infer::binding_name(&arrow.params.items[0].pattern)
                                .unwrap_or("_")
                                .to_string()
                        } else {
                            "_".to_string()
                        };
                        // Generate reverse loop
                        let blk = self.next_label();
                        self.write(&format!("({}: {{ var __i: usize = {}.items.len; while (__i > 0) {{ __i -= 1; const {} = {}.items[__i]; ", blk, obj_name, param_name, obj_name));
                        self.indent += 1;
                        for stmt in &arrow.body.statements {
                            self.write_indent();
                            if let Statement::ReturnStatement(ret) = stmt {
                                if let Some(expr) = &ret.argument {
                                    self.write("if (");
                                    self.emit_expr(expr);
                                    self.write(&format!(") break :{} {};", blk, param_name));
                                }
                            } else if let Statement::ExpressionStatement(es) = stmt {
                                self.write("if (");
                                self.emit_expr(&es.expression);
                                self.write(&format!(") break :{} {};", blk, param_name));
                            }
                        }
                        self.indent -= 1;
                        self.write_indent();
                        self.write(&format!("}} break :{} undefined; }})", blk));
                        return true;
                    }
                    self.write("undefined");
                    return true;
                }
                false
            }

            // ── ArrayFindLastIndex ────────────────────────────
            builtins::BuiltinCall::ArrayFindLastIndex => {
                // arr.findLastIndex(fn) → reverse iterator, break with index
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    if !ce.arguments.is_empty()
                        && let Some(arg) = ce.arguments.first()
                        && let Some(Expression::ArrowFunctionExpression(arrow)) =
                            arg.as_expression()
                    {
                        let param_name = if !arrow.params.items.is_empty() {
                            crate::native_proto::infer::binding_name(&arrow.params.items[0].pattern)
                                .unwrap_or("_")
                                .to_string()
                        } else {
                            "_".to_string()
                        };
                        let idx_name = format!("__{}_idx", param_name);
                        // Generate reverse loop with index
                        let blk = self.next_label();
                        self.write(&format!("({}: {{ var __i: usize = {}.items.len; while (__i > 0) {{ __i -= 1; const {} = {}.items[__i]; const {}: i64 = @intCast(__i); ", blk, obj_name, param_name, obj_name, idx_name));
                        self.indent += 1;
                        for stmt in &arrow.body.statements {
                            self.write_indent();
                            if let Statement::ReturnStatement(ret) = stmt {
                                if let Some(expr) = &ret.argument {
                                    self.write("if (");
                                    self.emit_expr(expr);
                                    self.write(&format!(") break :{} {};", blk, idx_name));
                                }
                            } else if let Statement::ExpressionStatement(es) = stmt {
                                self.write("if (");
                                self.emit_expr(&es.expression);
                                self.write(&format!(") break :{} {};", blk, idx_name));
                            }
                        }
                        self.indent -= 1;
                        self.write_indent();
                        self.write(&format!("}} break :{} -1; }})", blk));
                        return true;
                    }
                    self.write("-1");
                    return true;
                }
                false
            }

            // ── ArrayReduceRight ────────────────────────────
            builtins::BuiltinCall::ArrayReduceRight => {
                // arr.reduceRight(fn, init) → reverse reduce
                // TODO: Implement proper reverse reduce
                // For now, return undefined (placeholder)
                self.write("undefined");
                true
            }

            builtins::BuiltinCall::ArrayFill => {
                // arr.fill(val, start, end)
                // If object is a TypedArray, delegate to TypedArrayFill logic.
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    // Check if it's a TypedArray first
                    if self.typedarray_vars.contains_key(obj_name) {
                        // Delegate to TypedArrayFill logic
                        let ta_type = self.typedarray_vars.get(obj_name).cloned();
                        if let Some(ta_type) = ta_type {
                            if ce.arguments.is_empty() {
                                self.errors.push(
                                    "TypedArray.fill() requires at least 1 argument (value)"
                                        .to_string(),
                                );
                                return false;
                            }
                            let val_expr = self.first_arg_string(&ce.arguments);
                            let start_expr = if ce.arguments.len() >= 2 {
                                if let Some(arg) = ce.arguments.get(1)
                                    && let Some(expr) = arg.as_expression()
                                {
                                    self.emit_expr_to_string(expr)
                                } else {
                                    "0".to_string()
                                }
                            } else {
                                "0".to_string()
                            };
                            let end_expr = if ce.arguments.len() >= 3 {
                                if let Some(arg) = ce.arguments.get(2)
                                    && let Some(expr) = arg.as_expression()
                                {
                                    self.emit_expr_to_string(expr)
                                } else {
                                    "std.math.maxInt(i64)".to_string()
                                }
                            } else {
                                "std.math.maxInt(i64)".to_string()
                            };
                            self.write(&format!(
                                "js_runtime.js_typedarray.fill{}({}, {}, {}, {})",
                                ta_type, obj_name, val_expr, start_expr, end_expr
                            ));
                            return true;
                        }
                    }

                    // Regular Array.fill
                    // for (arr.items[start..end]) |*elem| { elem.* = val; }
                    if ce.arguments.is_empty() {
                        self.errors
                            .push("Array.fill() requires at least 1 argument (value)".to_string());
                        return false;
                    }
                    let val_str = self.first_arg_string(&ce.arguments);
                    let start_str = if ce.arguments.len() >= 2 {
                        if let Some(arg) = ce.arguments.get(1)
                            && let Some(expr) = arg.as_expression()
                        {
                            self.emit_expr_to_string(expr)
                        } else {
                            "0".to_string()
                        }
                    } else {
                        "0".to_string()
                    };
                    let end_str = if ce.arguments.len() >= 3 {
                        if let Some(arg) = ce.arguments.get(2)
                            && let Some(expr) = arg.as_expression()
                        {
                            self.emit_expr_to_string(expr)
                        } else {
                            format!("{}.items.len", obj_name)
                        }
                    } else {
                        format!("{}.items.len", obj_name)
                    };

                    self.write(&format!(
                        "for ({}.items[@intCast({})..@intCast({})]) |*elem| {{ elem.* = {}; }}",
                        obj_name, start_str, end_str, val_str
                    ));
                    return true;
                }
                false
            }

            // ── Global functions ─────────────────────────
            // ── Date methods (static) ──────────────────────
            builtins::BuiltinCall::DateNow => {
                // Date.now() → js_date.now()
                self.write("js_date.now()");
                true
            }
            builtins::BuiltinCall::DateParse => {
                // Date.parse(str) → js_date.parse(str)
                self.write("js_date.parse(");
                self.emit_first_arg(&ce.arguments);
                self.write(")");
                true
            }
            builtins::BuiltinCall::DateUTC => {
                // Date.UTC(y, m, d?, h?, min?, s?, ms?) → js_date.utc(y, m, d, h, min, s, ms)
                // Defaults: d=1, h=0, min=0, s=0, ms=0
                self.write("js_date.utc(");
                for (i, arg) in ce.arguments.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.emit_expr_arg(arg);
                }
                // Fill in defaults for missing arguments
                // Default: day = 1, hours/minutes/seconds/ms = 0
                match ce.arguments.len() {
                    0 => self.write("1970, 0, 1, 0, 0, 0, 0"),
                    1 => self.write(", 0, 1, 0, 0, 0, 0"),
                    2 => self.write(", 1, 0, 0, 0, 0"),
                    3 => self.write(", 0, 0, 0, 0"),
                    4 => self.write(", 0, 0, 0"),
                    5 => self.write(", 0, 0"),
                    6 => self.write(", 0"),
                    7 => {} // all args provided
                    _ => {
                        // More than 7 args — just emit all of them
                    }
                }
                self.write(")");
                true
            }

            // ── Date methods (instance) ────────────────────
            builtins::BuiltinCall::DateGetTime => self.emit_date_instance_method("getTime", ce),
            builtins::BuiltinCall::DateGetFullYear => {
                self.emit_date_instance_method("getFullYear", ce)
            }
            builtins::BuiltinCall::DateGetMonth => self.emit_date_instance_method("getMonth", ce),
            builtins::BuiltinCall::DateGetDate => self.emit_date_instance_method("getDate", ce),
            builtins::BuiltinCall::DateGetDay => self.emit_date_instance_method("getDay", ce),
            builtins::BuiltinCall::DateGetHours => self.emit_date_instance_method("getHours", ce),
            builtins::BuiltinCall::DateGetMinutes => {
                self.emit_date_instance_method("getMinutes", ce)
            }
            builtins::BuiltinCall::DateGetSeconds => {
                self.emit_date_instance_method("getSeconds", ce)
            }
            builtins::BuiltinCall::DateGetMilliseconds => {
                self.emit_date_instance_method("getMilliseconds", ce)
            }
            builtins::BuiltinCall::DateGetTimezoneOffset => {
                self.emit_date_instance_method("getTimezoneOffset", ce)
            }
            builtins::BuiltinCall::DateToISOString => {
                // date.toISOString() → try date.toISOString(js_allocator.getAllocator())
                if let Expression::StaticMemberExpression(mem) = &ce.callee {
                    self.write("try ");
                    self.emit_expr(&mem.object);
                    self.write(".toISOString(js_allocator.getAllocator())");
                    true
                } else {
                    self.errors.push(
                        "Date.toISOString() called on non-static-member expression".to_string(),
                    );
                    false
                }
            }

            // ── Date string methods ────────────────────────
            builtins::BuiltinCall::DateToString => {
                // date.toString() → try date.toString(js_allocator.getAllocator())
                if let Expression::StaticMemberExpression(mem) = &ce.callee {
                    self.write("try ");
                    self.emit_expr(&mem.object);
                    self.write(".toString(js_allocator.getAllocator())");
                    true
                } else {
                    self.errors
                        .push("Date.toString() called on non-static-member expression".to_string());
                    false
                }
            }

            builtins::BuiltinCall::DateToDateString => {
                // date.toDateString() → try date.toDateString(js_allocator.getAllocator())
                if let Expression::StaticMemberExpression(mem) = &ce.callee {
                    self.write("try ");
                    self.emit_expr(&mem.object);
                    self.write(".toDateString(js_allocator.getAllocator())");
                    true
                } else {
                    self.errors.push(
                        "Date.toDateString() called on non-static-member expression".to_string(),
                    );
                    false
                }
            }

            builtins::BuiltinCall::DateToTimeString => {
                // date.toTimeString() → try date.toTimeString(js_allocator.getAllocator())
                if let Expression::StaticMemberExpression(mem) = &ce.callee {
                    self.write("try ");
                    self.emit_expr(&mem.object);
                    self.write(".toTimeString(js_allocator.getAllocator())");
                    true
                } else {
                    self.errors.push(
                        "Date.toTimeString() called on non-static-member expression".to_string(),
                    );
                    false
                }
            }

            builtins::BuiltinCall::DateToLocaleString => {
                // date.toLocaleString() → try date.toLocaleString(js_allocator.getAllocator())
                if let Expression::StaticMemberExpression(mem) = &ce.callee {
                    self.write("try ");
                    self.emit_expr(&mem.object);
                    self.write(".toLocaleString(js_allocator.getAllocator())");
                    true
                } else {
                    self.errors.push(
                        "Date.toLocaleString() called on non-static-member expression".to_string(),
                    );
                    false
                }
            }

            // ── Date UTC getters ─────────────────────────
            builtins::BuiltinCall::DateGetUTCFullYear => {
                self.emit_date_instance_method("getUTCFullYear", ce)
            }
            builtins::BuiltinCall::DateGetUTCMonth => {
                self.emit_date_instance_method("getUTCMonth", ce)
            }
            builtins::BuiltinCall::DateGetUTCDate => {
                self.emit_date_instance_method("getUTCDate", ce)
            }
            builtins::BuiltinCall::DateGetUTCDay => self.emit_date_instance_method("getUTCDay", ce),
            builtins::BuiltinCall::DateGetUTCHours => {
                self.emit_date_instance_method("getUTCHours", ce)
            }
            builtins::BuiltinCall::DateGetUTCMinutes => {
                self.emit_date_instance_method("getUTCMinutes", ce)
            }
            builtins::BuiltinCall::DateGetUTCSeconds => {
                self.emit_date_instance_method("getUTCSeconds", ce)
            }
            builtins::BuiltinCall::DateGetUTCMilliseconds => {
                self.emit_date_instance_method("getUTCMilliseconds", ce)
            }

            // ── Date toJSON/valueOf ─────────────────────
            builtins::BuiltinCall::DateToJSON => {
                // date.toJSON() → try obj.toJSON(alloc)
                if ce.arguments.is_empty() {
                    if let Expression::StaticMemberExpression(mem) = &ce.callee {
                        self.write("try ");
                        self.emit_expr(&mem.object);
                        self.write(".toJSON(js_allocator.getAllocator())");
                    } else {
                        self.compile_error(
                            ce.span,
                            "Date.toJSON() called on non-static-member expression",
                        );
                    }
                } else {
                    self.compile_error(ce.span, "Date.toJSON() takes no arguments");
                }
                true
            }
            builtins::BuiltinCall::DateValueOf => {
                // date.valueOf() → obj.valueOf()
                if ce.arguments.is_empty() {
                    if let Expression::StaticMemberExpression(mem) = &ce.callee {
                        self.emit_expr(&mem.object);
                        self.write(".valueOf()");
                    } else {
                        self.compile_error(
                            ce.span,
                            "Date.valueOf() called on non-static-member expression",
                        );
                    }
                } else {
                    self.compile_error(ce.span, "Date.valueOf() takes no arguments");
                }
                true
            }

            // ── Date setters (table-driven, 14 methods) ──
            b if Self::date_setter_method_name(b).is_some() => {
                let method = Self::date_setter_method_name(b).unwrap();
                self.emit_date_setter_method(method, ce)
            }

            // ── Object methods (static) ────────────────────
            builtins::BuiltinCall::ObjectKeys => {
                // Object.keys(obj) → js_object.keys(alloc, obj) for JsValueHashMap,
                // or inline keys array for anonymous struct literals
                if !ce.arguments.is_empty()
                    && let Some(expr) = ce.arguments[0].as_expression()
                {
                    // Check if the argument is a variable with struct type
                    if let Expression::Identifier(id) = expr
                        && let Some(ZigType::Struct(fields)) =
                            self.type_info.var_types.get(id.name.as_str())
                    {
                        let obj_name = id.name.as_str();
                        let keys: Vec<String> = fields
                            .iter()
                            .map(|(name, _)| format!("\"{}\"", name))
                            .collect();
                        // Use a block that references the original variable (to prevent
                        // Zig "unused local constant" errors) and returns the keys inline.
                        let blk = self.next_label();
                        if keys.is_empty() {
                            self.write(&format!(
                                "({blk}: {{ _ = {obj}; break :{blk} (&[_][]const u8{{}}); }})",
                                blk = blk,
                                obj = obj_name
                            ));
                        } else {
                            self.write(&format!(
                                "({blk}: {{ _ = {obj}; break :{blk} (&[_][]const u8{{ {keys} }}); }})",
                                blk = blk,
                                obj = obj_name,
                                keys = keys.join(", ")
                            ));
                        }
                        return true;
                    }
                    // Check if the argument is an inline object literal
                    if let Expression::ObjectExpression(oe) = expr {
                        let mut keys: Vec<String> = Vec::new();
                        for prop in &oe.properties {
                            if let ObjectPropertyKind::ObjectProperty(p) = prop {
                                let field_name = match &p.key {
                                    PropertyKey::StaticIdentifier(id) => id.name.to_string(),
                                    PropertyKey::StringLiteral(s) => s.value.to_string(),
                                    _ => continue,
                                };
                                keys.push(format!("\"{}\"", field_name));
                            }
                        }
                        if keys.is_empty() {
                            self.write("(&[_][]const u8{})");
                        } else {
                            self.write(&format!("(&[_][]const u8{{ {} }})", keys.join(", ")));
                        }
                        return true;
                    }
                }
                // Default: pass to js_object.keys (for JsValueHashMap etc.)
                self.write("js_object.keys(js_allocator.getAllocator(), ");
                self.emit_first_arg(&ce.arguments);
                self.write(")");
                true
            }
            builtins::BuiltinCall::ObjectValues => {
                // Object.values(obj) → js_object.values(alloc, obj)
                self.write("js_object.values(js_allocator.getAllocator(), ");
                self.emit_first_arg(&ce.arguments);
                self.write(")");
                true
            }
            builtins::BuiltinCall::ObjectEntries => {
                // Object.entries(obj) → js_object.entries(alloc, obj)
                self.write("js_object.entries(js_allocator.getAllocator(), ");
                self.emit_first_arg(&ce.arguments);
                self.write(")");
                true
            }
            builtins::BuiltinCall::ObjectFromEntries => {
                // Object.fromEntries(iterable) → js_object.fromEntries(alloc, iterable)
                self.write("js_object.fromEntries(js_allocator.getAllocator(), ");
                self.emit_first_arg(&ce.arguments);
                self.write(")");
                true
            }
            builtins::BuiltinCall::ObjectAssign => {
                // Object.assign(target, source) → js_object.assign(target, source)
                if ce.arguments.len() >= 2 {
                    self.write("js_object.assign(&");
                    self.emit_expr_arg(&ce.arguments[0]);
                    self.write(", &");
                    self.emit_expr_arg(&ce.arguments[1]);
                    self.write(")");
                } else {
                    self.compile_error(ce.span, "Object.assign requires at least 2 arguments");
                }
                true
            }
            builtins::BuiltinCall::ObjectFreeze => {
                // Object.freeze(obj) — no-op in Zig (immutable by default)
                self.emit_first_arg(&ce.arguments);
                true
            }
            builtins::BuiltinCall::ObjectSeal => {
                // Object.seal(obj) — no-op in Zig (simplified)
                self.emit_first_arg(&ce.arguments);
                true
            }
            builtins::BuiltinCall::ObjectPreventExtensions => {
                // Object.preventExtensions(obj) — no-op in Zig (immutable by default)
                self.emit_first_arg(&ce.arguments);
                true
            }
            builtins::BuiltinCall::ObjectIsSealed => {
                // Object.isSealed(obj) — always true in Zig
                self.write("true");
                true
            }
            builtins::BuiltinCall::ObjectIsFrozen => {
                // Object.isFrozen(obj) — always true in Zig
                self.write("true");
                true
            }
            builtins::BuiltinCall::ObjectIsExtensible => {
                // Object.isExtensible(obj) — always false in Zig
                self.write("false");
                true
            }
            builtins::BuiltinCall::ObjectCreate => {
                // Object.create(proto) → js_object.create(alloc, proto)
                if ce.arguments.is_empty() {
                    self.compile_error(ce.span, "Object.create() requires at least 1 argument");
                    return true;
                }
                self.write("js_object.create(js_allocator.getAllocator(), ");
                let first_arg = ce.arguments[0].as_expression();
                if let Some(Expression::NullLiteral(_)) = first_arg {
                    self.write("null");
                } else if let Some(expr) = first_arg {
                    self.emit_expr(expr);
                } else {
                    self.write("null");
                }
                self.write(")");
                true
            }
            builtins::BuiltinCall::ObjectDefineProperty => {
                // Object.defineProperty(obj, key, value) → js_object.defineProperty(obj, key, value)
                if ce.arguments.len() < 3 {
                    self.compile_error(ce.span, "Object.defineProperty() requires 3 arguments");
                    return true;
                }
                self.write("js_object.defineProperty(");
                self.emit_expr_arg(&ce.arguments[0]);
                self.write(", ");
                self.emit_expr_arg(&ce.arguments[1]);
                self.write(", ");
                self.emit_expr_arg(&ce.arguments[2]);
                self.write(")");
                true
            }
            builtins::BuiltinCall::ObjectGetPrototypeOf => {
                // Object.getPrototypeOf(obj) → js_object.getPrototypeOf(obj)
                if ce.arguments.is_empty() {
                    self.compile_error(ce.span, "Object.getPrototypeOf() requires 1 argument");
                    return true;
                }
                self.write("js_object.getPrototypeOf(");
                self.emit_first_arg(&ce.arguments);
                self.write(")");
                true
            }
            builtins::BuiltinCall::ObjectDefineProperties => {
                // Object.defineProperties(obj, props) → js_object.defineProperties(obj, props)
                if ce.arguments.len() < 2 {
                    self.compile_error(ce.span, "Object.defineProperties() requires 2 arguments");
                    return true;
                }
                self.write("js_object.defineProperties(");
                self.emit_expr_arg(&ce.arguments[0]);
                self.write(", ");
                self.emit_expr_arg(&ce.arguments[1]);
                self.write(")");
                true
            }
            builtins::BuiltinCall::ObjectGetOwnPropertyDescriptor => {
                // Object.getOwnPropertyDescriptor(obj, key) → ?JsValueHashMap
                if ce.arguments.len() < 2 {
                    self.compile_error(
                        ce.span,
                        "Object.getOwnPropertyDescriptor() requires 2 arguments",
                    );
                    return true;
                }
                self.write("js_object.getOwnPropertyDescriptor(js_allocator.getAllocator(), ");
                self.emit_expr_arg(&ce.arguments[0]);
                self.write(", ");
                self.emit_expr_arg(&ce.arguments[1]);
                self.write(")");
                true
            }
            builtins::BuiltinCall::ObjectSetPrototypeOf => {
                // Object.setPrototypeOf(obj, proto) → js_object.setPrototypeOf(obj, proto)
                if ce.arguments.len() < 2 {
                    self.compile_error(ce.span, "Object.setPrototypeOf() requires 2 arguments");
                    return true;
                }
                self.write("js_object.setPrototypeOf(");
                self.emit_expr_arg(&ce.arguments[0]);
                self.write(", ");
                self.emit_expr_arg(&ce.arguments[1]);
                self.write(")");
                true
            }
            builtins::BuiltinCall::ObjectHasOwn => {
                // Object.hasOwn(obj, key) → bool
                // For statically-known struct types + string literal key: @hasField
                // Otherwise: js_object.hasOwn(obj, key) runtime
                if ce.arguments.len() != 2 {
                    self.compile_error(ce.span, "Object.hasOwn requires exactly 2 arguments");
                    return true;
                }
                let obj_arg = ce.arguments[0].as_expression();
                let key_arg = ce.arguments[1].as_expression();

                // Check if we can use comptime @hasField
                if let (Some(Expression::Identifier(id)), Some(Expression::StringLiteral(key_lit))) =
                    (obj_arg, key_arg)
                    && let Some(ty) = self.type_info.var_types.get(id.name.as_str())
                    && matches!(ty, ZigType::Struct(_))
                {
                    self.write(&format!(
                        "@hasField(@TypeOf({}), \"{}\")",
                        id.name.as_str(),
                        key_lit.value.as_str()
                    ));
                    return true;
                }

                // Fallback: runtime hasOwn
                self.write("js_object.hasOwn(");
                self.emit_expr_arg(&ce.arguments[0]);
                self.write(", ");
                self.emit_expr_arg(&ce.arguments[1]);
                self.write(")");
                true
            }

            builtins::BuiltinCall::ParseInt => {
                // parseInt(s) → js_number.parseInt(s, null)
                // parseInt(s, radix) → js_number.parseInt(s, radix)
                if let Some(arg) = ce.arguments.first()
                    && arg.as_expression().is_some()
                {
                    self.write("js_number.parseInt(");
                    self.emit_expr_arg(&ce.arguments[0]);
                    if ce.arguments.len() >= 2
                        && let Some(radix_expr) = ce.arguments[1].as_expression()
                    {
                        self.write(", ");
                        self.emit_expr(radix_expr);
                    } else {
                        self.write(", null");
                    }
                    self.write(")");
                    return true;
                }
                false
            }

            // ── JSON methods ─────────────────────────────
            builtins::BuiltinCall::JsonStringify => {
                // JSON.stringify(value, replacer?, space?) → try js_json.stringify(js_allocator.getAllocator(), value, replacer, space)
                self.write("try js_json.stringify(js_allocator.getAllocator(), ");
                if let Some(first_arg) = ce.arguments.first() {
                    self.emit_expr_arg(first_arg);
                } else {
                    self.write("JsAny.fromUndefined()");
                }
                // Pass replacer (default null)
                if ce.arguments.len() >= 2 {
                    self.write(", ");
                    self.emit_expr_arg(&ce.arguments[1]);
                } else {
                    self.write(", null");
                }
                // Pass space (default null)
                if ce.arguments.len() >= 3 {
                    self.write(", ");
                    self.emit_expr_arg(&ce.arguments[2]);
                } else {
                    self.write(", null");
                }
                self.write(") catch @panic(\"OOM: JSON.stringify\")");
                true
            }

            builtins::BuiltinCall::JsonParse => {
                // JSON.parse(text, reviver?) → try js_json.parse(js_allocator.getAllocator(), text, reviver)
                self.write("try js_json.parse(js_allocator.getAllocator(), ");
                if let Some(first_arg) = ce.arguments.first() {
                    self.emit_expr_arg(first_arg);
                } else {
                    self.write("\"\"");
                }
                // Pass reviver (default null)
                if ce.arguments.len() >= 2 {
                    self.write(", ");
                    self.emit_expr_arg(&ce.arguments[1]);
                } else {
                    self.write(", null");
                }
                self.write(") catch @panic(\"JSON.parse error\")");
                true
            }

            // ── Console methods ─────────────────────────────
            builtins::BuiltinCall::ConsoleLog => {
                if ce.arguments.len() <= 1 {
                    self.write("js_console.log(");
                    self.emit_first_arg(&ce.arguments);
                    self.write(")");
                } else {
                    self.write("js_console.logMulti(.{ ");
                    for (i, arg) in ce.arguments.iter().enumerate() {
                        if i > 0 {
                            self.write(", ");
                        }
                        if let Some(expr) = arg.as_expression() {
                            self.emit_expr(expr);
                        }
                    }
                    self.write(" })");
                }
                true
            }

            builtins::BuiltinCall::ConsoleError => {
                if ce.arguments.len() <= 1 {
                    self.write("js_console.err(");
                    self.emit_first_arg(&ce.arguments);
                    self.write(")");
                } else {
                    self.write("js_console.errMulti(.{ ");
                    for (i, arg) in ce.arguments.iter().enumerate() {
                        if i > 0 {
                            self.write(", ");
                        }
                        if let Some(expr) = arg.as_expression() {
                            self.emit_expr(expr);
                        }
                    }
                    self.write(" })");
                }
                true
            }

            builtins::BuiltinCall::ConsoleWarn => {
                if ce.arguments.len() <= 1 {
                    self.write("js_console.warn(");
                    self.emit_first_arg(&ce.arguments);
                    self.write(")");
                } else {
                    self.write("js_console.warnMulti(.{ ");
                    for (i, arg) in ce.arguments.iter().enumerate() {
                        if i > 0 {
                            self.write(", ");
                        }
                        if let Some(expr) = arg.as_expression() {
                            self.emit_expr(expr);
                        }
                    }
                    self.write(" })");
                }
                true
            }

            // ── Global functions (extended) ────────────────────
            builtins::BuiltinCall::ParseFloat => {
                // parseFloat(s) → std.fmt.parseFloat(f64, s) catch 0.0
                if let Some(arg) = ce.arguments.first()
                    && let Some(expr) = arg.as_expression()
                {
                    self.write("std.fmt.parseFloat(f64, ");
                    self.emit_expr(expr);
                    self.write(") catch 0.0");
                    return true;
                }
                false
            }

            builtins::BuiltinCall::IsNaN => {
                // isNaN(v) → std.math.isNan(@as(f64, v))
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("isNaN() requires exactly 1 argument".to_string());
                    return false;
                }
                self.write("std.math.isNan(@as(f64, ");
                self.emit_first_arg(&ce.arguments);
                self.write("))");
                true
            }

            builtins::BuiltinCall::IsFinite => {
                // isFinite(v) → !std.math.isInf(@as(f64, v)) && !std.math.isNan(@as(f64, v))
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("isFinite() requires exactly 1 argument".to_string());
                    return false;
                }
                self.write("(!std.math.isInf(@as(f64, ");
                self.emit_first_arg(&ce.arguments);
                self.write(")) and !std.math.isNan(@as(f64, ");
                self.emit_first_arg(&ce.arguments);
                self.write(")))");
                true
            }

            builtins::BuiltinCall::EncodeURIComponent => {
                // encodeURIComponent(s) → js_uri.encodeURIComponent(alloc, s)
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("encodeURIComponent() requires exactly 1 argument".to_string());
                    return false;
                }
                self.write("js_uri.encodeURIComponent(js_allocator.getAllocator(), ");
                self.emit_first_arg(&ce.arguments);
                self.write(") catch @panic(\"OOM: encodeURIComponent\")");
                true
            }

            builtins::BuiltinCall::EncodeURI => {
                // encodeURI(uri) → js_uri.encodeURI(alloc, uri)
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("encodeURI() requires exactly 1 argument".to_string());
                    return false;
                }
                self.write("js_uri.encodeURI(js_allocator.getAllocator(), ");
                self.emit_first_arg(&ce.arguments);
                self.write(") catch @panic(\"OOM: encodeURI\")");
                true
            }

            builtins::BuiltinCall::DecodeURIComponent => {
                // decodeURIComponent(s) → js_uri.decodeURIComponent(alloc, s)
                // On invalid encoding, fall back to "" (mirrors JS URIError → caught by outer try/catch).
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("decodeURIComponent() requires exactly 1 argument".to_string());
                    return false;
                }
                self.write("js_uri.decodeURIComponent(js_allocator.getAllocator(), ");
                self.emit_first_arg(&ce.arguments);
                self.write(") catch \"\"");
                true
            }

            builtins::BuiltinCall::DecodeURI => {
                // decodeURI(encodedURI) → js_uri.decodeURI(alloc, encodedURI)
                // On invalid encoding, fall back to "" (mirrors JS URIError → caught by outer try/catch).
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("decodeURI() requires exactly 1 argument".to_string());
                    return false;
                }
                self.write("js_uri.decodeURI(js_allocator.getAllocator(), ");
                self.emit_first_arg(&ce.arguments);
                self.write(") catch \"\"");
                true
            }

            // ── Number static methods ──────────────────────────
            builtins::BuiltinCall::NumberIsNaN => {
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("Number.isNaN() requires exactly 1 argument".to_string());
                    return false;
                }
                self.write("js_number.isNaN(");
                self.emit_first_arg(&ce.arguments);
                self.write(")");
                true
            }

            builtins::BuiltinCall::NumberIsFinite => {
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("Number.isFinite() requires exactly 1 argument".to_string());
                    return false;
                }
                self.write("js_number.isFinite(");
                self.emit_first_arg(&ce.arguments);
                self.write(")");
                true
            }

            builtins::BuiltinCall::NumberIsInteger => {
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("Number.isInteger() requires exactly 1 argument".to_string());
                    return false;
                }
                self.write("js_number.isInteger(");
                self.emit_first_arg(&ce.arguments);
                self.write(")");
                true
            }

            builtins::BuiltinCall::NumberParseInt => {
                if ce.arguments.is_empty() {
                    self.errors
                        .push("Number.parseInt() requires at least 1 argument".to_string());
                    return false;
                }
                self.write("js_number.parseInt(");
                self.emit_first_arg(&ce.arguments);
                if ce.arguments.len() >= 2
                    && let Some(radix_expr) = ce.arguments[1].as_expression()
                {
                    self.write(", ");
                    self.emit_expr(radix_expr);
                } else {
                    self.write(", null");
                }
                self.write(")");
                true
            }

            builtins::BuiltinCall::NumberParseFloat => {
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("Number.parseFloat() requires exactly 1 argument".to_string());
                    return false;
                }
                self.write("js_number.parseFloat(");
                self.emit_first_arg(&ce.arguments);
                self.write(")");
                true
            }

            builtins::BuiltinCall::NumberIsSafeInteger => {
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("Number.isSafeInteger() requires exactly 1 argument".to_string());
                    return false;
                }
                self.write("js_number.isSafeInteger(");
                self.emit_first_arg(&ce.arguments);
                self.write(")");
                true
            }

            // ── Number instance methods ────────────────────────
            builtins::BuiltinCall::NumberToFixed => {
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("toFixed() requires exactly 1 argument (digits)".to_string());
                    return false;
                }
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    self.write(&format!(
                        "js_number.toFixed(js_allocator.getAllocator(), {}, ",
                        obj_name
                    ));
                    self.emit_first_arg(&ce.arguments);
                    self.write(")");
                    return true;
                }
                // Handle numeric literal: (77.1234).toFixed(2)
                // AST: StaticMemberExpression { object: ParenthesizedExpression(NumericLiteral), ... }
                if self.emit_numeric_receiver(&ce.callee, "toFixed", &ce.arguments, true) {
                    return true;
                }
                false
            }

            builtins::BuiltinCall::NumberToExponential => {
                // num.toExponential(fractionDigits) → js_number.toExponential(allocator, num, fractionDigits)
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    self.write(&format!(
                        "js_number.toExponential(js_allocator.getAllocator(), {}, ",
                        obj_name
                    ));
                    if ce.arguments.is_empty() {
                        self.write("null");
                    } else {
                        self.emit_first_arg(&ce.arguments);
                    }
                    self.write(")");
                    return true;
                }
                // Handle numeric literal: (77.1234).toExponential(2)
                if self.emit_numeric_receiver(&ce.callee, "toExponential", &ce.arguments, false) {
                    return true;
                }
                false
            }

            builtins::BuiltinCall::NumberToPrecision => {
                // num.toPrecision(precision) → js_number.toPrecision(allocator, num, precision)
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    self.write(&format!(
                        "js_number.toPrecision(js_allocator.getAllocator(), {}, ",
                        obj_name
                    ));
                    if ce.arguments.is_empty() {
                        self.write("null");
                    } else {
                        self.emit_first_arg(&ce.arguments);
                    }
                    self.write(")");
                    return true;
                }
                // Handle numeric literal: (5.123456).toPrecision(3)
                if self.emit_numeric_receiver(&ce.callee, "toPrecision", &ce.arguments, false) {
                    return true;
                }
                false
            }

            // ── Map/Set clear ────────────────────────────────
            builtins::BuiltinCall::MapClear => {
                // map.clear() / set.clear() → obj.clear()
                if !ce.arguments.is_empty() {
                    self.errors
                        .push("Map/Set.clear() requires no arguments".to_string());
                    return false;
                }
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    self.write(&format!("{}.clear()", obj_name));
                    return true;
                }
                false
            }

            // ── Array methods (P2) ─────────────────────────────
            builtins::BuiltinCall::ArrayAt => {
                // arr.at(index) → arr.items[@intCast(clamped_index)]
                // Negative indices count from the end
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("Array.at() requires exactly 1 argument".to_string());
                    return false;
                }
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    let arg_expr = self.first_arg_string(&ce.arguments);
                    let blk = self.next_label();
                    self.write(&format!(
                        "({blk}: {{ const __idx = {arg}; const __at_idx = if (__idx < 0) @as(usize, @intCast(@as(isize, @intCast({obj}.items.len)) + @as(isize, @intCast(__idx)))) else @as(usize, @intCast(__idx)); break :{blk} {obj}.items[__at_idx]; }})",
                        blk = blk,
                        obj = obj_name,
                        arg = arg_expr
                    ));
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArrayLastIndexOf => {
                // arr.lastIndexOf(x) → backward labeled loop
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("Array.lastIndexOf() requires exactly 1 argument".to_string());
                    return false;
                }
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    let arg_expr = self.first_arg_string(&ce.arguments);
                    let blk = self.next_label();
                    self.write(&format!(
                        "({blk}: {{ var __i: isize = @as(isize, @intCast({obj}.items.len)) - 1; while (__i >= 0) : (__i -= 1) {{ if ({obj}.items[@as(usize, @intCast(__i))] == {arg}) break :{blk} @as(i64, __i); }} break :{blk} @as(i64, -1); }})",
                        blk = blk,
                        obj = obj_name,
                        arg = arg_expr
                    ));
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArrayCopyWithin => {
                // arr.copyWithin(target, start, end?) — inline copy elements
                // Check for TypedArray first (route to runtime)
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    if let Some(ta_type) = self.typedarray_vars.get(obj_name).cloned() {
                        if ce.arguments.len() < 2 {
                            self.errors.push(
                                "TypedArray.copyWithin() requires at least 2 arguments (target, start)".to_string(),
                            );
                            return false;
                        }
                        let target_expr = self.first_arg_string(&ce.arguments);
                        let start_expr = if let Some(arg) = ce.arguments.get(1)
                            && let Some(expr) = arg.as_expression()
                        {
                            self.emit_expr_to_string(expr)
                        } else {
                            "0".to_string()
                        };
                        let end_expr = if ce.arguments.len() >= 3 {
                            if let Some(arg) = ce.arguments.get(2)
                                && let Some(expr) = arg.as_expression()
                            {
                                self.emit_expr_to_string(expr)
                            } else {
                                "std.math.maxInt(i64)".to_string()
                            }
                        } else {
                            "std.math.maxInt(i64)".to_string()
                        };
                        self.write(&format!(
                            "js_runtime.js_typedarray.copyWithin{}({}, {}, {}, {})",
                            ta_type, obj_name, target_expr, start_expr, end_expr
                        ));
                        return true;
                    }

                    // Regular ArrayList copyWithin
                    if ce.arguments.len() < 2 {
                        self.errors.push(
                            "Array.copyWithin() requires at least 2 arguments (target, start)"
                                .to_string(),
                        );
                        return false;
                    }
                    let target_expr = self.first_arg_string(&ce.arguments);
                    let start_expr = if let Some(arg) = ce.arguments.get(1)
                        && let Some(expr) = arg.as_expression()
                    {
                        self.emit_expr_to_string(expr)
                    } else {
                        "0".to_string()
                    };
                    let end_expr = if ce.arguments.len() >= 3 {
                        if let Some(arg) = ce.arguments.get(2)
                            && let Some(expr) = arg.as_expression()
                        {
                            self.emit_expr_to_string(expr)
                        } else {
                            format!("{}.items.len", obj_name)
                        }
                    } else {
                        format!("{}.items.len", obj_name)
                    };
                    let blk = self.next_label();
                    self.write(&format!(
                        "({blk}: {{ \
                            const __cpw_target = @as(isize, @intCast({t})); \
                            const __cpw_start = @as(isize, @intCast({s})); \
                            const __cpw_end = @as(isize, @intCast({e})); \
                            const __cpw_cnt = __cpw_end - __cpw_start; \
                            if (__cpw_cnt > 0) {{ \
                                if (__cpw_target > __cpw_start) {{ \
                                    var __cpw_i: isize = __cpw_cnt - 1; \
                                    while (__cpw_i >= 0) : (__cpw_i -= 1) {{ \
                                        {obj}.items[@as(usize, @intCast(__cpw_target + __cpw_i))] = {obj}.items[@as(usize, @intCast(__cpw_start + __cpw_i))]; \
                                    }} \
                                }} else if (__cpw_target < __cpw_start) {{ \
                                    var __cpw_i: isize = 0; \
                                    while (__cpw_i < __cpw_cnt) : (__cpw_i += 1) {{ \
                                        {obj}.items[@as(usize, @intCast(__cpw_target + __cpw_i))] = {obj}.items[@as(usize, @intCast(__cpw_start + __cpw_i))]; \
                                    }} \
                                }} \
                            }} \
                            break :{blk} &{obj}; \
                        }})",
                        blk = blk,
                        obj = obj_name,
                        t = target_expr,
                        s = start_expr,
                        e = end_expr,
                    ));
                    return true;
                }
                false
            }

            // ── String methods (P2) ─────────────────────────────
            builtins::BuiltinCall::RegExpTest => {
                // /pattern/.test(str) → host.regex_test("pattern", str)
                // regexpVar.isMatch(str) → regexpVar.isMatch(str) (method call on JsRegExp)
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("RegExp.isMatch() requires exactly 1 argument".to_string());
                    return false;
                }
                // Extract pattern from the receiver (RegExp literal or RegExp variable)
                if let Expression::StaticMemberExpression(ref mem) = ce.callee {
                    if let Expression::RegExpLiteral(re) = &mem.object {
                        let escaped = escape_zig_string(&re.regex.pattern.text);
                        self.write(&format!("host.regex_test(\"{}\", ", escaped));
                        self.emit_first_arg(&ce.arguments);
                        self.write(")");
                        return true;
                    }
                    // Dynamic RegExp variable: emit .isMatch() method call
                    if let Expression::Identifier(id) = &mem.object
                        && self.regexp_vars.contains(id.name.as_str())
                    {
                        self.emit_expr(&mem.object);
                        self.write(".isMatch(");
                        self.emit_first_arg(&ce.arguments);
                        self.write(")");
                        return true;
                    }
                }
                self.compile_error(
                    ce.span,
                    "RegExp.isMatch() receiver must be a regex literal or RegExp variable",
                );
                true
            }

            builtins::BuiltinCall::StringMatch => {
                // str.match(/pattern/) → js_string.matchString(alloc, str, "pattern")
                // str.match(/pattern/g) → js_string.matchStringGlobal(alloc, str, "pattern")
                // str.match(regexpVar) → js_string.matchString(alloc, str, regexpVar.pattern)
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("String.match() requires exactly 1 argument".to_string());
                    return false;
                }
                if let Some(first_arg) = ce.arguments.first()
                    && let Some(expr) = first_arg.as_expression()
                {
                    let Some(obj_repr) = self.callee_object_repr_mut(&ce.callee) else {
                        return false;
                    };
                    match expr {
                        Expression::RegExpLiteral(re) => {
                            let pattern = re.regex.pattern.text.as_str().to_string();
                            let escaped = pattern.replace("\\", "\\\\").replace("\"", "\\\"");
                            // Parse flags from raw regex literal (e.g., "/abc/g" → "g")
                            // re.raw is Option<Str>, so we need to handle the Option
                            let has_global = re
                                .raw
                                .as_ref()
                                .map(|raw| {
                                    let raw_str = raw.as_str();
                                    // Find the last '/' and extract flags
                                    raw_str.rfind('/').is_some_and(|idx| {
                                        let flags_part = &raw_str[idx + 1..];
                                        flags_part.contains('g')
                                    })
                                })
                                .unwrap_or(false);
                            if has_global {
                                self.write(&format!(
                                    "js_string.matchStringGlobal(js_allocator.getAllocator(), {}, \"{}\") catch @panic(\"OOM: allocation\")",
                                    obj_repr, escaped
                                ));
                            } else {
                                self.write(&format!(
                                    "js_string.matchString(js_allocator.getAllocator(), {}, \"{}\") catch @panic(\"OOM: allocation\")",
                                    obj_repr, escaped
                                ));
                            }
                        }
                        Expression::Identifier(id)
                            if self.regexp_vars.contains(id.name.as_str()) =>
                        {
                            self.write(&format!(
                                "js_string.matchString(js_allocator.getAllocator(), {}, {}.pattern) catch @panic(\"OOM: allocation\")",
                                obj_repr, id.name.as_str()
                            ));
                        }
                        _ => {
                            self.compile_error(
                                ce.span,
                                "String.match() requires a regex literal or RegExp variable argument",
                            );
                        }
                    }
                    return true;
                }
                self.compile_error(
                    ce.span,
                    "String.match() requires a regex literal or RegExp variable argument",
                );
                true
            }

            builtins::BuiltinCall::StringSearch => {
                // str.search(/pattern/) → host.regex_search("pattern", str)
                // str.search(regexpVar) → host.regex_search(regexpVar.pattern, str)
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("String.search() requires exactly 1 argument".to_string());
                    return false;
                }
                if let Some(first_arg) = ce.arguments.first()
                    && let Some(expr) = first_arg.as_expression()
                {
                    let Some(obj_repr) = self.callee_object_repr_mut(&ce.callee) else {
                        return false;
                    };
                    match expr {
                        Expression::RegExpLiteral(re) => {
                            let pattern = re.regex.pattern.text.as_str().to_string();
                            let escaped = pattern.replace("\\", "\\\\").replace("\"", "\\\"");
                            self.write(&format!(
                                "host.regex_search(\"{}\", {})",
                                escaped, obj_repr
                            ));
                        }
                        Expression::Identifier(id)
                            if self.regexp_vars.contains(id.name.as_str()) =>
                        {
                            self.write(&format!(
                                "host.regex_search({}.pattern, {})",
                                id.name.as_str(),
                                obj_repr
                            ));
                        }
                        _ => {
                            self.compile_error(
                                ce.span,
                                "String.search() requires a regex literal or RegExp variable argument",
                            );
                        }
                    }
                    return true;
                }
                self.compile_error(ce.span, "String.search() requires a regex literal argument");
                true
            }

            // ── String methods (matchAll) ─────────────────────
            builtins::BuiltinCall::StringMatchAll => {
                // str.matchAll(/pattern/g) → js_string.matchAllString(alloc, str, "pattern")
                // str.matchAll(regexpVar) → js_string.matchAllString(alloc, str, regexpVar.pattern)
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("String.matchAll() requires exactly 1 argument".to_string());
                    return false;
                }
                if let Some(first_arg) = ce.arguments.first()
                    && let Some(expr) = first_arg.as_expression()
                {
                    let Some(obj_repr) = self.callee_object_repr_mut(&ce.callee) else {
                        return false;
                    };
                    match expr {
                        Expression::RegExpLiteral(re) => {
                            let pattern = re.regex.pattern.text.as_str().to_string();
                            let escaped = pattern.replace("\\", "\\\\").replace("\"", "\\\"");
                            self.write(&format!(
                                "js_string.matchAllString(js_allocator.getAllocator(), {}, \"{}\") catch @panic(\"OOM: allocation\")",
                                obj_repr, escaped
                            ));
                        }
                        Expression::Identifier(id)
                            if self.regexp_vars.contains(id.name.as_str()) =>
                        {
                            self.write(&format!(
                                "js_string.matchAllString(js_allocator.getAllocator(), {}, {}.pattern) catch @panic(\"OOM: allocation\")",
                                obj_repr, id.name.as_str()
                            ));
                        }
                        _ => {
                            self.compile_error(
                                ce.span,
                                "String.matchAll() requires a regex literal or RegExp variable argument",
                            );
                        }
                    }
                    return true;
                }
                self.compile_error(
                    ce.span,
                    "String.matchAll() requires a regex literal or RegExp variable argument",
                );
                true
            }
            // ── Object methods (P2) ─────────────────────────────
            builtins::BuiltinCall::ObjectIs => {
                // Object.is(a, b) → SameValue comparison
                // JS SameValue: NaN === NaN (true), +0 !== -0 (false)
                // Zig: NaN != NaN, 0 == -0 — we approximate with NaN check
                if ce.arguments.len() != 2 {
                    self.compile_error(ce.span, "Object.is() requires exactly 2 arguments");
                    return true;
                }
                // Generate: (std.math.isNan(a) and std.math.isNan(b)) or (a == b)
                self.write("(");
                let a_expr =
                    if let Some(arg0) = ce.arguments.first().and_then(|a| a.as_expression()) {
                        self.emit_expr_to_string(arg0)
                    } else {
                        self.compile_error(
                            ce.span,
                            "Object.is(): first argument must be an expression",
                        );
                        return true;
                    };
                let b_expr = if let Some(arg1) = ce.arguments.get(1).and_then(|a| a.as_expression())
                {
                    self.emit_expr_to_string(arg1)
                } else {
                    self.compile_error(
                        ce.span,
                        "Object.is(): second argument must be an expression",
                    );
                    return true;
                };
                self.write(&format!(
                    "(std.math.isNan({a}) and std.math.isNan({b})) or ({a} == {b})",
                    a = a_expr,
                    b = b_expr,
                ));
                self.write(")");
                true
            }

            builtins::BuiltinCall::ObjectGetOwnPropertyNames => {
                // Object.getOwnPropertyNames(obj) → not yet implemented
                self.compile_error(
                    ce.span,
                    "Object.getOwnPropertyNames() is not yet implemented in js2zig",
                );
                true
            }

            // ── String static methods ─────────────────────────────
            builtins::BuiltinCall::StringFromCharCode => {
                // String.fromCharCode(...codes) → js_string.fromCharCode(alloc, codes)
                self.write("js_string.fromCharCode(js_allocator.getAllocator()");
                if !ce.arguments.is_empty() {
                    self.write(", &[_]u16{");
                    for (i, arg) in ce.arguments.iter().enumerate() {
                        if i > 0 {
                            self.write(", ");
                        }
                        if let Some(expr) = arg.as_expression() {
                            self.emit_expr(expr);
                        }
                    }
                    self.write("}");
                }
                self.write(")");
                true
            }

            builtins::BuiltinCall::StringFromCodePoint => {
                // String.fromCodePoint(...codePoints) → js_string.fromCodePoint(alloc, codePoints)
                self.write("js_string.fromCodePoint(js_allocator.getAllocator()");
                if !ce.arguments.is_empty() {
                    self.write(", &[_]u32{");
                    for (i, arg) in ce.arguments.iter().enumerate() {
                        if i > 0 {
                            self.write(", ");
                        }
                        if let Some(expr) = arg.as_expression() {
                            self.emit_expr(expr);
                        }
                    }
                    self.write("}");
                }
                self.write(")");
                true
            }

            // ── Array static methods ─────────────────────────────
            builtins::BuiltinCall::ArrayFrom => {
                // Array.from(arrayLike[, mapFn[, thisArg]]) → js_array.from(alloc, arrayLike)
                self.write("js_array.from(js_allocator.getAllocator()");
                if !ce.arguments.is_empty() {
                    self.write(", ");
                    if let Some(first) = ce.arguments.first()
                        && let Some(expr) = first.as_expression()
                    {
                        self.emit_expr(expr);
                    }
                }
                self.write(")");
                true
            }

            builtins::BuiltinCall::ArrayOf => {
                // Array.of(...items) → js_array.of(alloc, items)
                self.write("js_array.of(js_allocator.getAllocator()");
                if !ce.arguments.is_empty() {
                    self.write(", &[_]JsAny{");
                    for (i, arg) in ce.arguments.iter().enumerate() {
                        if i > 0 {
                            self.write(", ");
                        }
                        if let Some(expr) = arg.as_expression() {
                            self.emit_expr(expr);
                        }
                    }
                    self.write("}");
                }
                self.write(")");
                true
            }

            builtins::BuiltinCall::ArrayIsArray => {
                // Array.isArray(obj) → js_array.isArray(obj)
                self.write("js_array.isArray(");
                if let Some(first) = ce.arguments.first()
                    && let Some(expr) = first.as_expression()
                {
                    self.emit_expr(expr);
                }
                self.write(")");
                true
            }

            builtins::BuiltinCall::RegExpExec => {
                // /pattern/.exec(str) → js_regexp.execLiteral(alloc, str, "pattern")
                // regexpVar.exec(str) → regexpVar.exec(alloc, str)
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("RegExp.exec() requires exactly 1 argument".to_string());
                    return false;
                }
                if let Expression::StaticMemberExpression(ref mem) = ce.callee {
                    if let Expression::RegExpLiteral(re) = &mem.object {
                        let pattern = re.regex.pattern.text.as_str().to_string();
                        let escaped = pattern.replace("\\", "\\\\").replace("\"", "\\\"");
                        self.write("js_regexp.execLiteral(js_allocator.getAllocator(), ");
                        self.emit_first_arg(&ce.arguments);
                        self.write(&format!(
                            ", \"{}\") catch @panic(\"OOM: allocation\")",
                            escaped
                        ));
                        return true;
                    }
                    // Dynamic RegExp variable: emit .exec() method call
                    if let Expression::Identifier(id) = &mem.object
                        && self.regexp_vars.contains(id.name.as_str())
                    {
                        self.emit_expr(&mem.object);
                        self.write(".exec(js_allocator.getAllocator(), ");
                        self.emit_first_arg(&ce.arguments);
                        self.write(")");
                        return true;
                    }
                }
                self.compile_error(
                    ce.span,
                    "RegExp.exec() receiver must be a regex literal or RegExp variable",
                );
                true
            }

            // ── Symbol methods ────────────────────────────
            builtins::BuiltinCall::SymbolConstructor => {
                // Symbol(description?) → js_symbol.JsSymbol.init(description)
                // or js_symbol.JsSymbol.initAnonymous()
                if ce.arguments.is_empty() {
                    self.write("js_symbol.JsSymbol.initAnonymous()");
                } else {
                    self.write("js_symbol.JsSymbol.init(");
                    self.emit_first_arg(&ce.arguments);
                    self.write(")");
                }
                true
            }

            builtins::BuiltinCall::SymbolFor => {
                // Symbol.for(key) → js_symbol.symbolFor(key)
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("Symbol.for() requires exactly 1 argument".to_string());
                    return false;
                }
                self.write("js_symbol.symbolFor(");
                self.emit_first_arg(&ce.arguments);
                self.write(")");
                true
            }

            builtins::BuiltinCall::SymbolKeyFor => {
                // Symbol.keyFor(sym) → js_symbol.symbolKeyFor(sym)
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("Symbol.keyFor() requires exactly 1 argument".to_string());
                    return false;
                }
                self.write("js_symbol.symbolKeyFor(");
                self.emit_first_arg(&ce.arguments);
                self.write(")");
                true
            }
            // ── Global type constructors (used as functions) ──
            builtins::BuiltinCall::NumberConstructor => {
                // Number(x) — type-aware conversion:
                //   string → std.fmt.parseFloat(f64, x) catch 0.0
                //   float  → @as(f64, @floatCast(x))
                //   int    → @as(f64, @floatFromInt(x))
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("Number() requires exactly 1 argument".to_string());
                    return false;
                }
                if let Some(arg) = ce.arguments.first()
                    && let Some(expr) = arg.as_expression()
                {
                    match self.infer_expr_type(expr) {
                        Some(ZigType::Str) => {
                            self.write("std.fmt.parseFloat(f64, ");
                            self.emit_expr(expr);
                            self.write(") catch 0.0");
                        }
                        Some(ZigType::F64) => {
                            self.write("@as(f64, @floatCast(");
                            self.emit_expr(expr);
                            self.write("))");
                        }
                        Some(ZigType::JsAny) => {
                            // Number(JsAny) → .asF64() (NaN for undefined, 0 for null, etc.)
                            self.emit_expr(expr);
                            self.write(".asF64()");
                        }
                        Some(ZigType::Bool) => {
                            // Number(true) → 1.0, Number(false) → 0.0
                            self.write("if (");
                            self.emit_expr(expr);
                            self.write(") @as(f64, 1.0) else @as(f64, 0.0)");
                        }
                        Some(ZigType::BigInt) => {
                            // Number(BigInt) → @floatFromInt(bigint.toI64())
                            self.write("@as(f64, @floatFromInt((");
                            self.emit_expr(expr);
                            self.write(
                                ").toI64() catch @panic(\"BigInt too large for Number()\")))",
                            );
                        }
                        _ => {
                            // Default: integer conversion
                            self.write("@as(f64, @floatFromInt(");
                            self.emit_expr(expr);
                            self.write("))");
                        }
                    }
                } else {
                    self.errors
                        .push("Number() requires an expression argument".to_string());
                    return false;
                }
                true
            }
            builtins::BuiltinCall::StringConstructor => {
                // String(x) — string coercion
                // For numeric types, use std.fmt.bufPrint or similar.
                // TODO: full string coercion for all types.
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("String() requires exactly 1 argument".to_string());
                    return false;
                }
                let blk = self.next_label();
                self.write(&format!("({}: {{ const _val = ", blk));
                self.emit_first_arg(&ce.arguments);
                self.write("; ");
                self.write(&format!("break :{} std.fmt.allocPrint(js_allocator.getAllocator(), \"{{d}}\", .{{_val}}) catch @panic(\"OOM\"); }})", blk));
                true
            }
            builtins::BuiltinCall::BooleanConstructor => {
                // Boolean(x) → bool, type-dependent conversion:
                //   Bool: identity (already bool)
                //   Str:  str.len != 0 (empty string is falsy)
                //   I64/F64: x != 0
                //   undefined/null: always falsy
                //   other: x != 0 (best-effort fallback)
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("Boolean() requires exactly 1 argument".to_string());
                    return false;
                }
                let arg = &ce.arguments[0];
                if let Some(expr) = arg.as_expression() {
                    match self.infer_expr_type(expr) {
                        Some(ZigType::Bool) => {
                            // Boolean(bool) → identity
                            self.emit_expr(expr);
                        }
                        Some(ZigType::Str) => {
                            // Boolean("") → "".len != 0 (empty string is falsy)
                            self.write("((");
                            self.emit_expr(expr);
                            self.write(").len != 0)");
                        }
                        Some(ZigType::I64) | Some(ZigType::F64) => {
                            // Boolean(0) → 0 != 0 (falsy); Boolean(42) → 42 != 0 (truthy)
                            self.write("(");
                            self.emit_expr(expr);
                            self.write(" != 0)");
                        }
                        _ => {
                            // Unknown type: check for special JS falsy values
                            if let Expression::NullLiteral(_) = expr {
                                // Boolean(null) → false (null is always falsy)
                                self.write("false");
                                return true;
                            }
                            if let Expression::Identifier(id) = expr
                                && id.name.as_str() == "undefined"
                            {
                                // Boolean(undefined) → false (undefined is always falsy)
                                self.write("false");
                                return true;
                            }
                            // Fallback: x != 0 (works for numbers, may fail for JsAny)
                            self.write("(");
                            self.emit_expr(expr);
                            self.write(" != 0)");
                        }
                    }
                } else {
                    self.errors
                        .push("Boolean() argument is not an expression".to_string());
                    return false;
                }
                true
            }
            builtins::BuiltinCall::BigIntConstructor => {
                // BigInt(x) → (js_bigint.JsBigInt.fromI64(alloc, x) catch @panic(...))
                // Parens are needed for correct parsing in if/else branches (Zig catch precedence).
                self.write("(js_bigint.JsBigInt.fromI64(js_allocator.getAllocator(), ");
                self.emit_first_arg(&ce.arguments);
                self.write(") catch @panic(\"OOM: BigInt fromI64\"))");
                true
            }
            builtins::BuiltinCall::ObjectConstructor => {
                // Object(x) → x (identity in Zig — everything is a value)
                // EXCEPT BigInt: emit identity BigInt literal as-is, because JsAny
                // does not yet have BigInt support. Object(BigInt) === BigInt is
                // handled by emit_binary's mixed-type comparison path (returns false).
                if ce.arguments.is_empty() {
                    // Object() → empty object, not supported in native mode
                    self.compile_error(ce.span, "Object() without arguments would create an empty object which is not supported in native_proto mode. Use struct literal {} instead.");
                } else {
                    self.emit_first_arg(&ce.arguments);
                }
                true
            }

            builtins::BuiltinCall::Eval => {
                // eval() is not supported — emit @compileError
                self.compile_error(ce.span, "eval() is not supported (security risk, cannot dynamically execute at compile time)");
                true
            }
            _ => false,
        }
    }

    /// Emit argument expression (handles spread etc.).
    pub(crate) fn emit_expr_arg(&mut self, arg: &Argument) {
        // Arguments are in expression position, never statement position.
        let saved = self.in_expr_stmt;
        self.in_expr_stmt = false;

        // Argument inherits Expression variants via inherit_variants! macro.
        // SpreadElement is a variant: Argument::SpreadElement(Box<SpreadElement>).
        match arg {
            Argument::SpreadElement(se) => {
                // foo(...args) → pass args.items (the underlying slice)
                // The callee must accept []const JsAny (rest parameter).
                // se is Box<SpreadElement>, so se.argument is the expression being spread.
                self.emit_expr(&se.argument);
                self.write(".items");
            }
            _ => {
                // Expression argument: use as_expression() to get the Expression.
                if let Some(e) = arg.as_expression() {
                    self.emit_expr(e);
                } else {
                    self.errors.push("Unknown argument type".to_string());
                    self.compile_error(GetSpan::span(arg), "Unknown argument type");
                }
            }
        }

        self.in_expr_stmt = saved;
    }

    /// Emit a Date instance method call.
    /// JS: `date.getTime()` → Zig: `date.getTime()` (direct instance method call)
    fn emit_date_instance_method(&mut self, method: &str, ce: &CallExpression) -> bool {
        // Extract the receiver object from the callee
        if let Expression::StaticMemberExpression(mem) = &ce.callee {
            self.emit_expr(&mem.object);
            self.write(&format!(".{method}()"));
            true
        } else {
            self.errors.push(format!(
                "Date.{method}() called on non-static-member expression"
            ));
            false
        }
    }

    // Assignment
    fn emit_assignment(&mut self, ae: &AssignmentExpression) {
        // Handle ComputedMemberExpression assignment: obj[key] = val
        if let AssignmentTarget::ComputedMemberExpression(ref mem) = ae.left {
            match &mem.expression {
                Expression::NumericLiteral(n) => {
                    // arr[0] = val → dispatch based on obj type
                    let idx = n.value as i64;
                    let obj_type = self.infer_expr_type(&mem.object);
                    match obj_type {
                        Some(ZigType::ArrayList(_)) => {
                            // ArrayList: arr.items[0] = val
                            self.emit_expr(&mem.object);
                            self.write(&format!(".items[{}] = ", idx));
                            self.emit_expr(&ae.right);
                        }
                        Some(ZigType::JsAny) | None => {
                            // JsAny: (blk_N: { obj.setByKey(JsAny.from(idx), val, alloc) catch undefined; break :blk_N val; })
                            let blk = self.next_label();
                            self.write(&format!("({}: {{ ", blk));
                            self.emit_expr(&mem.object);
                            self.write(&format!(".setByKey(JsAny.from({}), ", idx));
                            self.emit_expr(&ae.right);
                            self.write(&format!(
                                ", js_allocator.getAllocator()) catch undefined; break :{} ",
                                blk
                            ));
                            self.emit_expr(&ae.right);
                            self.write("; })");
                        }
                        _ => {
                            self.errors
                                .push("Numeric indexing on non-indexable type".to_string());
                            self.write("@compileError(\"numeric indexing on non-indexable type\")");
                        }
                    }
                    return;
                }
                Expression::StringLiteral(s) => {
                    // obj["key"] = val → dispatch based on obj type
                    let key = s.value.as_str();
                    let obj_type = self.infer_expr_type(&mem.object);
                    match obj_type {
                        Some(ZigType::Struct(_)) | Some(ZigType::NamedStruct(_)) => {
                            // Struct: @field(obj, "key") = val (Zig returns val)
                            self.write("@field(");
                            self.emit_expr(&mem.object);
                            self.write(&format!(", \"{}\") = ", key));
                            self.emit_expr(&ae.right);
                            return;
                        }
                        _ => {
                            // JsAny/Map/unknown: block expr returning val
                            let blk = self.next_label();
                            self.write(&format!("({}: {{ ", blk));
                            self.emit_expr(&mem.object);
                            self.write(&format!(".set(\"{}\", ", key));
                            self.emit_expr(&ae.right);
                            self.write(&format!(") catch undefined; break :{} ", blk));
                            self.emit_expr(&ae.right);
                            self.write("; })");
                            return;
                        }
                    }
                }
                _ => {
                    // obj[expr] = val → dynamic key assignment
                    let obj_type = self.infer_expr_type(&mem.object);
                    match obj_type {
                        Some(ZigType::ArrayList(_)) => {
                            // ArrayList: arr.items[expr] = val
                            // Zig requires usize for array indexing; convert i64 if needed.
                            self.emit_expr(&mem.object);
                            self.write(".items[");
                            let idx_type = self.infer_expr_type(&mem.expression);
                            if idx_type == Some(ZigType::I64) || idx_type == Some(ZigType::BigInt) {
                                self.write("@as(usize, @intCast(");
                                self.emit_value_expr(&mem.expression);
                                self.write("))");
                            } else {
                                self.emit_value_expr(&mem.expression);
                            }
                            self.write("] = ");
                            self.emit_expr(&ae.right);
                            return;
                        }
                        Some(ZigType::NamedStruct(ref name)) if name == "Map" => {
                            // Map: (blk_N: { obj.set(key, val) catch undefined; break :blk_N val; })
                            let blk = self.next_label();
                            self.write(&format!("({}: {{ ", blk));
                            self.emit_expr(&mem.object);
                            self.write(".set(");
                            self.emit_value_expr(&mem.expression);
                            self.write(", ");
                            self.emit_expr(&ae.right);
                            self.write(&format!(") catch undefined; break :{} ", blk));
                            self.emit_expr(&ae.right);
                            self.write("; })");
                            return;
                        }
                        Some(ZigType::Struct(_)) | Some(ZigType::NamedStruct(_)) => {
                            // Struct: @field(obj, expr) = val (expr must be comptime-known)
                            self.write("@field(");
                            self.emit_expr(&mem.object);
                            self.write(", ");
                            self.emit_value_expr(&mem.expression);
                            self.write(") = ");
                            self.emit_expr(&ae.right);
                            return;
                        }
                        Some(ZigType::JsAny) => {
                            // JsAny: (blk_N: { obj.setByKey(key, val, alloc) catch undefined; break :blk_N val; })
                            let blk = self.next_label();
                            self.write(&format!("({}: {{ ", blk));
                            self.emit_expr(&mem.object);
                            self.write(".setByKey(");
                            self.emit_value_expr(&mem.expression);
                            self.write(", ");
                            self.emit_expr(&ae.right);
                            self.write(&format!(
                                ", js_allocator.getAllocator()) catch undefined; break :{} ",
                                blk
                            ));
                            self.emit_expr(&ae.right);
                            self.write("; })");
                            return;
                        }
                        None => {
                            // Unknown type → fallback to JsAny.setByKey
                            let blk = self.next_label();
                            self.write(&format!("({}: {{ ", blk));
                            self.emit_expr(&mem.object);
                            self.write(".setByKey(");
                            self.emit_value_expr(&mem.expression);
                            self.write(", ");
                            self.emit_expr(&ae.right);
                            self.write(&format!(
                                ", js_allocator.getAllocator()) catch undefined; break :{} ",
                                blk
                            ));
                            self.emit_expr(&ae.right);
                            self.write("; })");
                            return;
                        }
                        _ => {
                            self.errors
                                .push("Dynamic property assignment on non-object type".to_string());
                            self.write(
                                "@compileError(\"dynamic property assignment on non-object type\")",
                            );
                            return;
                        }
                    }
                }
            }
        }

        // Zig 0.16+: signed integer division requires @divTrunc/@rem
        if ae.operator == AssignmentOperator::Division
            || ae.operator == AssignmentOperator::Remainder
        {
            let op_fn = if ae.operator == AssignmentOperator::Division {
                "@divTrunc"
            } else {
                "@rem"
            };
            if self.in_expr_stmt {
                self.emit_assignment_target(&ae.left);
                self.write(&format!(" = {}(", op_fn));
                self.emit_assignment_target(&ae.left);
                self.write(", ");
                self.emit_expr(&ae.right);
                self.write(")");
            } else {
                // Wrap in labeled block so assignment returns the assigned value (JS semantics)
                let blk = self.next_label();
                self.write(&format!("({blk}: {{ "));
                self.emit_assignment_target(&ae.left);
                self.write(&format!(" = {}(", op_fn));
                self.emit_assignment_target(&ae.left);
                self.write(", ");
                self.emit_expr(&ae.right);
                self.write(&format!("); break :{blk} "));
                self.emit_assignment_target(&ae.left);
                self.write("; })");
            }
            return;
        }

        // **= exponentiation assignment: a **= b → a = a ** b
        // JS `**` always returns f64, but if `a` is i64, convert result back.
        if ae.operator == AssignmentOperator::Exponential {
            // Infer target type (for simple identifiers, look up var_types)
            let target_type = match &ae.left {
                AssignmentTarget::AssignmentTargetIdentifier(id) => {
                    self.type_info.var_types.get(id.name.as_str()).cloned()
                }
                _ => None,
            };
            let needs_int_cast = matches!(target_type, Some(ZigType::I64) | Some(ZigType::BigInt));
            let needs_float_from_int =
                matches!(target_type, Some(ZigType::I64) | Some(ZigType::BigInt));
            if self.in_expr_stmt {
                self.emit_assignment_target(&ae.left);
                self.write(" = ");
                if needs_int_cast {
                    self.write("@as(i64, @intFromFloat(std.math.pow(f64, ");
                } else {
                    self.write("std.math.pow(f64, ");
                }
                // Left operand: convert to f64
                if needs_float_from_int {
                    self.write("@as(f64, @floatFromInt(");
                    self.emit_assignment_target(&ae.left);
                    self.write("))");
                } else {
                    self.write("@as(f64, ");
                    self.emit_assignment_target(&ae.left);
                    self.write(")");
                }
                self.write(", ");
                let right_type = self.infer_expr_type(&ae.right);
                self.emit_float_conversion(&ae.right, right_type);
                if needs_int_cast {
                    self.write(")))");
                } else {
                    self.write(")");
                }
            } else {
                let blk = self.next_label();
                let inner_blk = self.next_label();
                self.write(&format!("({blk}: {{ "));
                self.emit_assignment_target(&ae.left);
                self.write(&format!(" = ({inner_blk}: {{ const _b: f64 = "));
                if needs_float_from_int {
                    self.write("@as(f64, @floatFromInt(");
                    self.emit_assignment_target(&ae.left);
                    self.write("))");
                } else {
                    self.write("@as(f64, ");
                    self.emit_assignment_target(&ae.left);
                    self.write(")");
                }
                self.write("; const _e: f64 = ");
                let right_type = self.infer_expr_type(&ae.right);
                self.emit_float_conversion(&ae.right, right_type);
                if needs_int_cast {
                    self.write(&format!(
                        "; break :{inner_blk} @as(i64, @intFromFloat(std.math.pow(f64, _b, _e))); }}); break :{blk} "
                    ));
                } else {
                    self.write(&format!(
                        "; break :{inner_blk} std.math.pow(f64, _b, _e); }}); break :{blk} "
                    ));
                }
                self.emit_assignment_target(&ae.left);
                self.write("; })");
            }
            return;
        }

        // &&= logical AND assignment
        if ae.operator == AssignmentOperator::LogicalAnd {
            if self.in_expr_stmt {
                self.emit_assignment_target(&ae.left);
                self.write(" = if (");
                self.emit_assignment_target(&ae.left);
                self.write(".toBool()) ");
                self.emit_expr(&ae.right);
                self.write(" else ");
                self.emit_assignment_target(&ae.left);
            } else {
                let blk = self.next_label();
                self.write(&format!("({blk}: {{ "));
                self.emit_assignment_target(&ae.left);
                self.write(" = if (");
                self.emit_assignment_target(&ae.left);
                self.write(".toBool()) ");
                self.emit_expr(&ae.right);
                self.write(" else ");
                self.emit_assignment_target(&ae.left);
                self.write(&format!("; break :{blk} "));
                self.emit_assignment_target(&ae.left);
                self.write("; })");
            }
            return;
        }

        // ||= logical OR assignment
        if ae.operator == AssignmentOperator::LogicalOr {
            if self.in_expr_stmt {
                self.emit_assignment_target(&ae.left);
                self.write(" = if (!");
                self.emit_assignment_target(&ae.left);
                self.write(".toBool()) ");
                self.emit_expr(&ae.right);
                self.write(" else ");
                self.emit_assignment_target(&ae.left);
            } else {
                let blk = self.next_label();
                self.write(&format!("({blk}: {{ "));
                self.emit_assignment_target(&ae.left);
                self.write(" = if (!");
                self.emit_assignment_target(&ae.left);
                self.write(".toBool()) ");
                self.emit_expr(&ae.right);
                self.write(" else ");
                self.emit_assignment_target(&ae.left);
                self.write(&format!("; break :{blk} "));
                self.emit_assignment_target(&ae.left);
                self.write("; })");
            }
            return;
        }

        // ??= nullish coalescing assignment
        if ae.operator == AssignmentOperator::LogicalNullish {
            if self.in_expr_stmt {
                self.emit_assignment_target(&ae.left);
                self.write(" = if (");
                self.emit_assignment_target(&ae.left);
                self.write(".isNullish()) ");
                self.emit_expr(&ae.right);
                self.write(" else ");
                self.emit_assignment_target(&ae.left);
            } else {
                let blk = self.next_label();
                self.write(&format!("({blk}: {{ "));
                self.emit_assignment_target(&ae.left);
                self.write(" = if (");
                self.emit_assignment_target(&ae.left);
                self.write(".isNullish()) ");
                self.emit_expr(&ae.right);
                self.write(" else ");
                self.emit_assignment_target(&ae.left);
                self.write(&format!("; break :{blk} "));
                self.emit_assignment_target(&ae.left);
                self.write("; })");
            }
            return;
        }

        // >>>= unsigned right shift assignment
        if ae.operator == AssignmentOperator::ShiftRightZeroFill {
            if self.in_expr_stmt {
                self.emit_assignment_target(&ae.left);
                self.write(" = @as(i64, @intCast(@as(u32, @bitCast(@as(i32, @truncate(");
                self.emit_assignment_target(&ae.left);
                self.write(")))) >> @intCast(");
                self.emit_expr(&ae.right);
                self.write(" & 31)))");
            } else {
                let blk = self.next_label();
                self.write(&format!("({blk}: {{ "));
                self.emit_assignment_target(&ae.left);
                self.write(" = @as(i64, @intCast(@as(u32, @bitCast(@as(i32, @truncate(");
                self.emit_assignment_target(&ae.left);
                self.write(")))) >> @intCast(");
                self.emit_expr(&ae.right);
                self.write(" & 31)))");
                self.write(&format!("; break :{blk} "));
                self.emit_assignment_target(&ae.left);
                self.write("; })");
            }
            return;
        }

        // Default: += -= *= <<= >>= &= |= ^=
        {
            if self.in_expr_stmt {
                self.emit_assignment_target(&ae.left);
                self.write(&format!(" {} ", Self::assignment_op(ae.operator)));
                self.emit_expr(&ae.right);
            } else {
                let blk = self.next_label();
                self.write(&format!("({blk}: {{ "));
                self.emit_assignment_target(&ae.left);
                self.write(&format!(" {} ", Self::assignment_op(ae.operator)));
                self.emit_expr(&ae.right);
                self.write(&format!("; break :{blk} "));
                self.emit_assignment_target(&ae.left);
                self.write("; })");
            }
        }
    }

    fn emit_assignment_target(&mut self, target: &AssignmentTarget) {
        match target {
            AssignmentTarget::AssignmentTargetIdentifier(id) => {
                let var_name = id.name.as_str();
                // Check if this is a captured variable in the current closure.
                if !self.current_captured.is_empty()
                    && let Some((_, _, is_mut)) = self
                        .current_captured
                        .iter()
                        .find(|(n, _, _)| n.as_str() == var_name)
                {
                    // Captured variable: rewrite to self.var_name (value capture)
                    // or self.var_name.* (reference capture, dereference for assignment)
                    let safe_name = self.zig_safe_name(var_name);
                    if *is_mut {
                        self.write(&format!("self.{}.*", safe_name));
                    } else {
                        self.write(&format!("self.{}", safe_name));
                    }
                    return;
                }
                self.write(&self.zig_safe_name(var_name));
            }
            AssignmentTarget::StaticMemberExpression(mem) => {
                self.emit_expr(&mem.object);
                self.write(".");
                self.write(mem.property.name.as_str());
            }
            AssignmentTarget::ComputedMemberExpression(_mem) => {
                self.errors.push(
                    "Dynamic property assignment (obj[key] = value) is not allowed. Use static property assignment (obj.prop = value).".to_string()
                );
                self.write("/* error: dynamic property assignment */");
            }
            _ => {
                self.errors
                    .push("Unsupported assignment target".to_string());
                self.write("/* unsupported assign target */");
            }
        }
    }

    // Unary expression
    fn emit_unary(&mut self, ue: &UnaryExpression) {
        // Check for BigInt unary ops first (neg, bitwise NOT)
        if self.expr_is_bigint(&ue.argument) {
            match ue.operator {
                UnaryOperator::UnaryNegation => {
                    let blk = self.next_label();
                    let var_suffix = self.label_counter - 1;
                    let a_name = format!("_a{}", var_suffix);
                    self.write(&format!("({0}: {{ var {1} = ", blk, a_name));
                    self.emit_expr(&ue.argument);
                    self.write(&format!("; break :{0} {1}.neg(js_allocator.getAllocator()) catch @panic(\"OOM: BigInt neg\"); }})", blk, a_name));
                    return;
                }
                UnaryOperator::BitwiseNot => {
                    let blk = self.next_label();
                    let var_suffix = self.label_counter - 1;
                    let a_name = format!("_a{}", var_suffix);
                    self.write(&format!("({0}: {{ var {1} = ", blk, a_name));
                    self.emit_expr(&ue.argument);
                    self.write(&format!("; break :{0} {1}.bitwiseNot(js_allocator.getAllocator()) catch @panic(\"OOM: BigInt bitwiseNot\"); }})", blk, a_name));
                    return;
                }
                _ => {}
            }
        }

        match ue.operator {
            UnaryOperator::UnaryNegation => {
                // Zig considers `-0` ambiguous (integer 0 or float -0.0).
                // This happens because JS parses `-0` literally as UnaryNegation(NumericLiteral(0)).
                // Emit `-0.0` explicitly to resolve the ambiguity.
                if let Expression::NumericLiteral(n) = &ue.argument
                    && n.value == 0.0
                {
                    self.write("-0.0");
                } else {
                    self.write("-");
                    self.emit_expr(&ue.argument);
                }
            }
            UnaryOperator::UnaryPlus => {
                self.write(Self::unary_prefix(ue.operator));
                self.emit_expr(&ue.argument);
            }
            UnaryOperator::LogicalNot => {
                // Zig's ! requires bool operand. For non-bool types (i64, comptime_int),
                // convert to bool via `expr != 0` (JS truthiness for numbers).
                // Anytype is excluded: it resolves at the call site (Zig will check there).
                let operand_type = self.infer_expr_type(&ue.argument);
                if operand_type == Some(ZigType::Str) {
                    // !"" → true (empty string is falsy in JS)
                    // !"hello" → false (non-empty string is truthy in JS)
                    self.write("(");
                    self.emit_expr(&ue.argument);
                    self.write(".len == 0)");
                } else if operand_type == Some(ZigType::Bool)
                    || operand_type == Some(ZigType::Anytype)
                    || operand_type.is_none()
                {
                    self.write("!");
                    self.emit_expr(&ue.argument);
                } else {
                    // !(expr != 0) — JS `!number` semantics
                    self.write("!(");
                    self.emit_expr(&ue.argument);
                    self.write(" != 0)");
                }
            }
            UnaryOperator::BitwiseNot => {
                // Zig 0.16.0 doesn't allow ~ on comptime_int.
                // Wrap operand with @as(i64, ...) to give it a concrete type.
                let operand_type = self.infer_expr_type(&ue.argument);
                let needs_wrap = !matches!(operand_type, Some(ZigType::F64));
                if needs_wrap {
                    self.write("~@as(i64, ");
                    self.emit_expr(&ue.argument);
                    self.write(")");
                } else {
                    self.write("~");
                    self.emit_expr(&ue.argument);
                }
            }
            UnaryOperator::Typeof => {
                // Use inferred Zig type to emit the JS typeof string at compile time.
                // For dynamic types (JsAny/Anytype), call the runtime jsTypeof() helper.
                if let Some(ty) = self.infer_expr_type(&ue.argument) {
                    if let Some(js_typeof) = ty.to_js_typeof() {
                        self.write(js_typeof);
                    } else {
                        self.write("js_runtime.jsTypeof(");
                        self.emit_expr(&ue.argument);
                        self.write(")");
                    }
                } else {
                    self.write("js_runtime.jsTypeof(");
                    self.emit_expr(&ue.argument);
                    self.write(")");
                }
            }
            UnaryOperator::Void => {
                // void expr: evaluate expr for side effects, return undefined.
                // When in expression-statement position, prefix with `_ = blk: {};`
                // to discard the result value (Zig forbids unused labeled-block values).
                //
                // Function/arrow expressions are already emitted as type declarations
                // (struct or closure struct) by emit_expr. The "side effect" _ = name
                // incorrectly discards the type, breaking subsequent .call() usage.
                // For these, fall through to emit the expression (which generates the
                // declaration) but wrap only with undefined, not a discard.
                if matches!(
                    &ue.argument,
                    Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_)
                ) {
                    // Emit the function expression first (creates declaration)
                    self.emit_expr(&ue.argument);
                    // Then produce undefined as the void result, without discarding
                    // the function name (it's a type, not a value).
                    if self.in_expr_stmt {
                        self.write("; _ = ");
                    }
                    self.write("JsAny.fromUndefined()");
                } else {
                    let blk = self.next_label();
                    self.write(&format!("{blk}: {{ _ = "));
                    self.emit_expr(&ue.argument);
                    self.write(&format!("; break :{blk} JsAny.fromUndefined(); }}"));
                }
            }
            UnaryOperator::Delete => {
                // delete obj.prop / delete obj[expr] — remove property, return bool
                match &ue.argument {
                    Expression::StaticMemberExpression(mem) => {
                        // delete obj.prop → _ = obj.deleteKey("prop"); true
                        let blk = self.next_label();
                        self.write(&format!("{blk}: {{ _ = "));
                        self.emit_expr(&mem.object);
                        self.write(".deleteKey(\"");
                        self.write(&mem.property.name);
                        self.write(&format!("\"); break :{blk} true; }}"));
                    }
                    Expression::ComputedMemberExpression(mem) => {
                        // delete obj[expr] → _ = obj.deleteByKey(expr, alloc); true
                        let blk = self.next_label();
                        self.write(&format!("{blk}: {{ const _dk = "));
                        self.emit_value_expr(&mem.expression);
                        self.write("; _ = ");
                        self.emit_expr(&mem.object);
                        self.write(&format!(".deleteByKey(_dk, alloc); break :{blk} true; }}"));
                    }
                    _ => {
                        self.errors
                            .push("delete operator requires property access".to_string());
                        self.write("/* unsupported: delete */");
                    }
                }
            }
        }
    }

    // Conditional (ternary)
    fn emit_conditional(&mut self, ce: &ConditionalExpression) {
        self.write("if (");
        self.emit_condition(&ce.test);
        self.write(") ");
        self.emit_expr(&ce.consequent);
        self.write(" else ");
        self.emit_expr(&ce.alternate);
    }

    // Array expression
    fn emit_array(&mut self, ae: &ArrayExpression) {
        if ae.elements.is_empty() {
            self.write("std.ArrayList(JsAny).empty");
        } else {
            // Determine element type.
            // If there is ANY spread element, element types can no longer be
            // uniform, so we must use JsAny.
            let has_spread = ae
                .elements
                .iter()
                .any(|e| matches!(e, ArrayExpressionElement::SpreadElement(_)));
            let elem_type = if has_spread {
                "JsAny".to_string()
            } else {
                ae.elements
                    .iter()
                    .find_map(|e| e.as_expression())
                    .map(|expr| match expr {
                        Expression::NumericLiteral(n) => {
                            let s = n.value.to_string();
                            if s.contains('.') || s.contains('e') || s.contains('E') {
                                "f64"
                            } else {
                                "i64"
                            }
                        }
                        Expression::StringLiteral(_) => "[]const u8",
                        Expression::BooleanLiteral(_) => "bool",
                        _ => "i64",
                    })
                    .unwrap_or("i64")
                    .to_string()
            };
            let blk = self.next_label();
            self.write(&format!(
                "({}: {{ var __arr: std.ArrayList({}) = .empty; ",
                blk, elem_type
            ));
            for elem in ae.elements.iter() {
                match elem {
                    ArrayExpressionElement::SpreadElement(se) => {
                        self.write("__arr.appendSlice(js_allocator.getAllocator(), ");
                        self.emit_expr(&se.argument);
                        self.write(".items) catch @panic(\"OOM: Array.spread\"); ");
                    }
                    ArrayExpressionElement::Elision(_) => {
                        self.write("__arr.append(js_allocator.getAllocator(), JsAny{ .undefined = {} }) catch @panic(\"OOM: Array.elision\"); ");
                    }
                    _ => {
                        if let Some(e) = elem.as_expression() {
                            self.write("__arr.append(js_allocator.getAllocator(), ");
                            self.emit_expr(e);
                            self.write(") catch @panic(\"OOM: Array.push append\"); ");
                        }
                    }
                }
            }
            self.write(&format!("break :{} __arr; }})", blk));
        }
    }

    /// Emit an object literal as a Zig anonymous struct.
    /// Supports multi-spread: { ...a, ...b, c: 1 } → js_runtime.spreadMerge(spreadMerge(a, b), .{ .c = 1 })
    fn emit_object(&mut self, oe: &ObjectExpression) {
        if oe.properties.is_empty() {
            // Empty object → StringHashMap(JsAny).init(js_allocator.getAllocator())
            self.write("std.StringHashMap(JsAny).init(js_allocator.getAllocator())");
            return;
        }

        let has_spread = oe
            .properties
            .iter()
            .any(|p| matches!(p, ObjectPropertyKind::SpreadProperty(_)));

        if !has_spread {
            // Pure inline properties — emit directly as .{ ... }
            self.write(".{ ");
            let mut first = true;
            for prop in &oe.properties {
                if let ObjectPropertyKind::ObjectProperty(p) = prop {
                    self.emit_inline_prop(p, &mut first);
                }
            }
            self.write(" }");
            return;
        }

        // Has spread elements: build a left-fold spreadMerge(...) chain.
        // Strategy:
        //   { ...a }                       → a
        //   { ...a, ...b }                 → js_runtime.spreadMerge(a, b)
        //   { ...a, b: 1 }                 → js_runtime.spreadMerge(a, .{ .b = 1 })
        //   { ...a, ...b, c: 1 }           → js_runtime.spreadMerge(spreadMerge(a, b), .{ .c = 1 })

        let mut spread_texts: Vec<String> = Vec::new();
        for prop in &oe.properties {
            if let ObjectPropertyKind::SpreadProperty(s) = prop {
                spread_texts.push(self.capture_expr(&s.argument));
            }
        }

        let inline_text = self.capture_inline_struct(oe);

        match (spread_texts.len(), &inline_text) {
            (0, _) => unreachable!(), // has_spread is true, so spread_texts is non-empty
            (1, None) => {
                // Single spread, no inline → identity (the whole object IS the spread value)
                self.write(&spread_texts[0]);
            }
            _ => {
                // Multi-spread or spread + inline → build spreadMerge chain
                let mut result = spread_texts[0].clone();
                for text in &spread_texts[1..] {
                    result = format!("js_runtime.spreadMerge({}, {})", result, text);
                }
                if let Some(ref inline) = inline_text {
                    result = format!("js_runtime.spreadMerge({}, {})", result, inline);
                }
                self.write(&result);
            }
        }
    }

    /// Capture the output of an expression to a string, leaving self.output unchanged.
    pub(crate) fn capture_expr(&mut self, expr: &Expression) -> String {
        let saved = self.output.len();
        self.emit_expr(expr);
        let result = self.output[saved..].to_string();
        self.output.truncate(saved);
        result
    }

    /// Capture inline (non-spread) properties as a Zig anonymous struct literal string.
    /// Returns None if there are no inline ObjectProperty items.
    fn capture_inline_struct(&mut self, oe: &ObjectExpression) -> Option<String> {
        let has_inline = oe
            .properties
            .iter()
            .any(|p| matches!(p, ObjectPropertyKind::ObjectProperty(_)));
        if !has_inline {
            return None;
        }

        let saved = self.output.len();
        self.write(".{ ");
        let mut first = true;
        for prop in &oe.properties {
            if let ObjectPropertyKind::ObjectProperty(p) = prop {
                self.emit_inline_prop(p, &mut first);
            }
        }
        self.write(" }");
        let result = self.output[saved..].to_string();
        self.output.truncate(saved);
        Some(result)
    }

    /// Emit a single inline object property (shared by emit_object and capture_inline_struct).
    fn emit_inline_prop(&mut self, p: &ObjectProperty, first: &mut bool) {
        let field_name = match &p.key {
            PropertyKey::StaticIdentifier(id) => id.name.to_string(),
            PropertyKey::StringLiteral(s) => s.value.to_string(),
            _ => return,
        };
        match p.kind {
            PropertyKind::Init => {
                if !*first {
                    self.write(", ");
                }
                *first = false;
                self.write(&format!(".{} = ", field_name));
                self.emit_expr(&p.value);
            }
            PropertyKind::Get => {
                // Getter: extract return expression from function body
                // { get x() { return expr; } } → .x = expr
                if let Expression::FunctionExpression(func) = &p.value
                    && let Some(body) = &func.body
                    && let Some(return_expr) = Self::extract_return_expr_from_body(body)
                {
                    if !*first {
                        self.write(", ");
                    }
                    *first = false;
                    self.write(&format!(".{} = ", field_name));
                    self.emit_expr(return_expr);
                }
            }
            PropertyKind::Set => {
                // Setter: skip, doesn't contribute a field to struct initialization
            }
        }
    }

    /// Extract the return expression from a function body (single return statement)
    fn extract_return_expr_from_body<'a>(body: &'a FunctionBody<'a>) -> Option<&'a Expression<'a>> {
        if body.statements.len() == 1
            && let Statement::ReturnStatement(ret) = &body.statements[0]
        {
            return ret.argument.as_ref();
        }
        None
    }

    // ── Optional chaining (?. ) ───────────────────────

    /// Emit an optional chain expression (?. ).
    /// Generates null-check if the object might be null; otherwise emits direct access.
    fn emit_optional_chain(&mut self, chain: &ChainExpression) {
        // Pre-check: handle TSNonNullExpression and PrivateFieldExpression early.
        match &chain.expression {
            ChainElement::TSNonNullExpression(non_null) => {
                // TSNonNullExpression.expression is the inner Expression — no null check
                self.emit_expr(&non_null.expression);
                return;
            }
            ChainElement::PrivateFieldExpression(_) => {
                self.errors.push(
                    "Optional chaining on private fields (?. #field) is not supported".to_string(),
                );
                self.compile_error(chain.span, "optional chaining on private fields");
                return;
            }
            _ => {}
        }

        let oc_var = format!("_oc{}", self.oc_counter);
        self.oc_counter += 1;

        match &chain.expression {
            ChainElement::StaticMemberExpression(mem) => {
                let needs_check = self.expr_might_be_null(&mem.object);
                if needs_check {
                    self.write("(if (");
                    self.emit_expr(&mem.object);
                    self.write(") |");
                    self.write(&oc_var);
                    self.write("| ");
                    self.write(&oc_var);
                    self.write(".");
                    self.write(mem.property.name.as_str());
                    self.write(" else null)");
                } else {
                    self.emit_expr(&mem.object);
                    self.write(".");
                    self.write(mem.property.name.as_str());
                }
            }
            ChainElement::ComputedMemberExpression(mem) => {
                let needs_check = self.expr_might_be_null(&mem.object);
                if needs_check {
                    self.write("(if (");
                    self.emit_expr(&mem.object);
                    self.write(") |");
                    self.write(&oc_var);
                    self.write("| ");
                    self.write(&oc_var);
                    self.write("[");
                    self.emit_value_expr(&mem.expression);
                    self.write("]");
                    self.write(" else null)");
                } else {
                    self.emit_expr(&mem.object);
                    self.write("[");
                    self.emit_value_expr(&mem.expression);
                    self.write("]");
                }
            }
            ChainElement::CallExpression(call) => {
                // For obj?.method(), the null check should be on obj, not the callee.
                // Extract the receiver from the callee expression.
                let (check_expr, emit_full_call) = match &call.callee {
                    Expression::StaticMemberExpression(mem) => {
                        // obj?.greet() → check obj, then obj.greet()
                        (&mem.object, false)
                    }
                    Expression::ComputedMemberExpression(mem) => {
                        // obj?.[key]() → check obj, then obj[key]()
                        (&mem.object, false)
                    }
                    _ => {
                        // obj?.() or other → check the callee itself
                        (&call.callee, true)
                    }
                };
                let needs_check = self.expr_might_be_null(check_expr);
                if emit_full_call && !needs_check {
                    // callee is non-nullable, just call it
                    self.emit_expr(&call.callee);
                    self.write("(");
                    self.emit_comma_separated_args(&call.arguments);
                    self.write(")");
                } else if needs_check {
                    self.write("(if (");
                    self.emit_expr(check_expr);
                    self.write(") |");
                    self.write(&oc_var);
                    self.write("| ");
                    if emit_full_call {
                        // obj?.() style: call the captured value
                        self.write(&oc_var);
                        self.write("(");
                        self.emit_comma_separated_args(&call.arguments);
                        self.write(")");
                    } else {
                        // obj?.greet() style: access .greet(args) on captured value
                        // Re-emit the property access path
                        match &call.callee {
                            Expression::StaticMemberExpression(mem) => {
                                self.write(&oc_var);
                                self.write(".");
                                self.write(mem.property.name.as_str());
                                self.write("(");
                                self.emit_comma_separated_args(&call.arguments);
                                self.write(")");
                            }
                            Expression::ComputedMemberExpression(mem) => {
                                self.write(&oc_var);
                                self.write("[");
                                self.emit_value_expr(&mem.expression);
                                self.write("](");
                                self.emit_comma_separated_args(&call.arguments);
                                self.write(")");
                            }
                            _ => unreachable!(),
                        }
                    }
                    self.write(" else null)");
                } else {
                    // Non-nullable → emit full call directly
                    self.emit_expr(&call.callee);
                    self.write("(");
                    self.emit_comma_separated_args(&call.arguments);
                    self.write(")");
                }
            }
            _ => unreachable!(),
        }
    }

    /// Returns true if the expression might evaluate to null at runtime.
    fn expr_might_be_null(&self, expr: &Expression) -> bool {
        match expr {
            // Literals: known non-null
            Expression::NumericLiteral(_)
            | Expression::StringLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::BigIntLiteral(_)
            | Expression::ArrayExpression(_)
            | Expression::ObjectExpression(_)
            | Expression::TemplateLiteral(_) => false,

            // null literal → it IS null
            Expression::NullLiteral(_) => true,

            // Identifier: check type
            Expression::Identifier(id) => match self.type_info.var_types.get(id.name.as_str()) {
                Some(ZigType::Struct(_))
                | Some(ZigType::NamedStruct(_))
                | Some(ZigType::ArrayList(_))
                | Some(ZigType::I64)
                | Some(ZigType::F64)
                | Some(ZigType::Bool)
                | Some(ZigType::Str)
                | Some(ZigType::JsSymbol)
                | Some(ZigType::BigInt)
                | Some(ZigType::AnytypeReturn) => false,
                Some(ZigType::Void) | Some(ZigType::Anytype) | Some(ZigType::JsAny) => true,
                None => true,
            },

            // Chain expression result is always optional (from else null)
            Expression::ChainExpression(_) => true,

            // Member access on unknown objects might return null
            Expression::StaticMemberExpression(mem) => self.expr_might_be_null(&mem.object),

            // Call results might return null
            Expression::CallExpression(_) => true,

            // Parenthesized: check inner
            Expression::ParenthesizedExpression(pe) => self.expr_might_be_null(&pe.expression),

            _ => true,
        }
    }
}

// ============================================================
// Phase A: Type inference has been moved to infer.rs.
// Codegen is now purely generative — it reads from
// self.type_info (TypeCheckResult) pre-computed by TypeInferrer.
// ============================================================

/// Descriptor for a simple 1-arg Math builtin mapped to a Zig call.
struct MathOneArgDesc {
    /// JS method name for error messages (e.g. "abs").
    name: &'static str,
    /// Zig format string with `{arg}` placeholder.
    format: &'static str,
}

/// Descriptor for simple String runtime forwarding calls.
struct StringRuntimeDesc {
    /// Zig function name (e.g. "trim", "toUpper").
    method: &'static str,
    /// Whether the call needs `js_allocator.getAllocator()` as first arg.
    needs_allocator: bool,
    /// Whether the Zig runtime function returns an error union (`!T`).
    /// If true, `try` is prepended to the call expression.
    is_fallible: bool,
    /// Minimum number of JS-level arguments required.
    min_args: usize,
    /// Maximum number of JS-level arguments accepted.
    max_args: usize,
    /// Default Zig expressions for optional argument slots beyond min_args.
    /// One entry per optional slot (e.g. min=1,max=2 → 1 entry for 2nd arg).
    /// Empty entries mean the slot is simply omitted when the arg is missing.
    opt_defaults: &'static [&'static str],
}

impl Codegen {
    /// Infer the type of an expression. Returns ZigType.
    /// If the type cannot be inferred, reports an error to self.errors
    /// and returns I64 as a fallback (the generated code will be invalid).
    /// Infer the type of an expression.
    /// Returns `Some(ZigType)` if the type can be determined (literal or binary with both literals),
    /// `None` if the type is indeterminate (Rule 1-3).
    /// Rule 1: Literal expressions → definite type
    /// Rule 2: Binary expressions → definite only if BOTH operands are literals
    /// Rule 3: Other expressions → indeterminate
    pub(crate) fn infer_expr_type(&mut self, expr: &Expression) -> Option<ZigType> {
        match expr {
            // Rule 1: Literals → definite type
            Expression::NumericLiteral(n) => {
                let s = n.value.to_string();
                if s.contains('.') || s.contains('e') || s.contains('E') {
                    Some(ZigType::F64)
                } else {
                    Some(ZigType::I64)
                }
            }
            Expression::StringLiteral(_) => Some(ZigType::Str),
            Expression::TemplateLiteral(_) => Some(ZigType::Str),
            Expression::BooleanLiteral(_) => Some(ZigType::Bool),
            Expression::BigIntLiteral(_) => Some(ZigType::BigInt),
            // NullLiteral → generates JsAny.fromNull() in codegen
            Expression::NullLiteral(_) => Some(ZigType::JsAny),
            Expression::UnaryExpression(ue) => {
                // -1, +1, !true → type can be determined from operand
                match ue.operator {
                    UnaryOperator::UnaryNegation | UnaryOperator::UnaryPlus => {
                        // -x or +x: type is same as x (if x is literal)
                        if Self::is_literal(&ue.argument) {
                            self.infer_expr_type(&ue.argument)
                        } else {
                            None
                        }
                    }
                    UnaryOperator::LogicalNot => {
                        // !x → Bool
                        if Self::is_literal(&ue.argument) {
                            Some(ZigType::Bool)
                        } else {
                            None
                        }
                    }
                    UnaryOperator::Void => {
                        // void expr → always returns undefined (JsAny)
                        Some(ZigType::JsAny)
                    }
                    _ => None,
                }
            }

            // Rule 2: Binary expression → definite only if BOTH operands are literals
            // Special case: Addition (+) with a string operand → result is Str (string concatenation)
            Expression::BinaryExpression(be) => {
                if Self::is_literal(&be.left) && Self::is_literal(&be.right) {
                    // Both are literals: infer types and compute result.
                    // Safety: is_literal() returning true guarantees
                    // infer_expr_type() returns Some(...).
                    let left_ty = self
                        .infer_expr_type(&be.left)
                        .expect("is_literal() true → infer_expr_type() returns Some");
                    let right_ty = self
                        .infer_expr_type(&be.right)
                        .expect("is_literal() true → infer_expr_type() returns Some");
                    Some(Self::infer_binary_type(be.operator, left_ty, right_ty))
                } else if be.operator == BinaryOperator::Addition
                    && (self.expr_is_string(&be.left) || self.expr_is_string(&be.right))
                {
                    // String concatenation: if either operand is a string, result is Str
                    Some(ZigType::Str)
                } else {
                    // Rule 3: Cannot infer type
                    None
                }
            }

            // Identifier: look up variable type from var_types (Rule 5)
            // Also handle known global constants not in var_types
            Expression::Identifier(id) => match id.name.as_str() {
                "Infinity" | "NaN" => Some(ZigType::F64),
                "undefined" => Some(ZigType::JsAny),
                _ => self.type_info.var_types.get(id.name.as_str()).cloned(),
            },

            // StaticMemberExpression: look up field type from struct type (Rule 5)
            Expression::StaticMemberExpression(mem) => {
                // Math.PI → f64
                if let Expression::Identifier(id) = &mem.object
                    && id.name.as_str() == "Math"
                    && mem.property.name.as_str() == "PI"
                {
                    return Some(ZigType::F64);
                }
                // Number.* constants → typed
                if let Expression::Identifier(id) = &mem.object
                    && id.name.as_str() == "Number"
                {
                    match mem.property.name.as_str() {
                        "MAX_VALUE" | "MIN_VALUE" | "NaN" | "NEGATIVE_INFINITY"
                        | "POSITIVE_INFINITY" | "EPSILON" => return Some(ZigType::F64),
                        "MAX_SAFE_INTEGER" | "MIN_SAFE_INTEGER" => return Some(ZigType::I64),
                        _ => {}
                    }
                }
                let obj_ty = self.infer_expr_type(&mem.object);
                if let Some(ZigType::Struct(fields)) = obj_ty {
                    let field_name = mem.property.name.as_str();
                    for (name, ty) in fields {
                        if name == field_name {
                            return Some(ty.clone());
                        }
                    }
                    // Field not found: indeterminate
                    None
                } else {
                    // Object type is indeterminate: cannot infer field type
                    None
                }
            }

            // CallExpression: look up function return type from cache (Rule 5-6)
            Expression::CallExpression(ce) => {
                // Map.get(key) / Set.has(key) etc. — StaticMemberExpression callee (obj.method(...))
                if let Expression::StaticMemberExpression(mem) = &ce.callee
                    && let Expression::Identifier(obj) = &mem.object
                {
                    let obj_name = obj.name.as_str();
                    if let Some(ty) = self.type_info.var_types.get(obj_name) {
                        match (ty, mem.property.name.as_str()) {
                            (ZigType::NamedStruct(name), "get") if name == "Map" => {
                                return Some(ZigType::JsAny);
                            }
                            (ZigType::NamedStruct(name), "has")
                                if name == "Map" || name == "Set" =>
                            {
                                return Some(ZigType::Bool);
                            }
                            // String methods returning Bool
                            (ZigType::Str, "includes" | "startsWith" | "endsWith") => {
                                return Some(ZigType::Bool);
                            }
                            // String methods returning I64
                            (ZigType::Str, "indexOf" | "lastIndexOf") => {
                                return Some(ZigType::I64);
                            }
                            // String methods returning Str
                            (
                                ZigType::Str,
                                "trim" | "trimStart" | "trimEnd" | "split" | "padStart" | "padEnd"
                                | "charAt" | "at" | "toUpperCase" | "toLowerCase" | "slice"
                                | "substring" | "replace" | "replaceAll" | "concat" | "repeat",
                            ) => {
                                return Some(ZigType::Str);
                            }
                            _ => {}
                        }
                    }
                }
                // Fallback: ComputedMemberExpression callee (obj[key]()) — kept for completeness
                if let Expression::ComputedMemberExpression(mem) = &ce.callee
                    && let Expression::Identifier(obj) = &mem.object
                {
                    let obj_name = obj.name.as_str();
                    if let Some(ZigType::NamedStruct(name)) = self.type_info.var_types.get(obj_name)
                        && name == "Map"
                    {
                        return Some(ZigType::JsAny);
                    }
                }
                // Get callee name for non-builtin calls
                if let Expression::Identifier(id) = &ce.callee {
                    let fn_name = id.name.as_str();
                    // Global builtin return types
                    if fn_name == "parseInt" {
                        return Some(ZigType::I64);
                    }
                    // Look up return type from cache
                    if let Some(ret_ty) = self.type_info.fn_return_types.get(fn_name) {
                        return Some(ret_ty.clone());
                    }
                }
                // Cannot determine return type
                None
            }

            // ArrayExpression: if all elements are literals, infer element type
            Expression::ArrayExpression(ae) => {
                if ae.elements.is_empty() {
                    // Empty array: cannot infer element type
                    None
                } else {
                    // Infer element type from first element (if it's a literal)
                    if let Some(first_elem) = ae.elements.first() {
                        if let Some(first) = first_elem.as_expression() {
                            let elem_ty = self.infer_expr_type(first);
                            // Check all elements have the same definite type
                            for elem in ae.elements.iter().skip(1) {
                                if let Some(e) = elem.as_expression() {
                                    let et = self.infer_expr_type(e);
                                    match (&elem_ty, &et) {
                                        (Some(t1), Some(t2)) => {
                                            if *t1 != *t2 {
                                                // Type mismatch: indeterminate
                                                return None;
                                            }
                                        }
                                        _ => {
                                            // Indeterminate element: cannot infer array type
                                            return None;
                                        }
                                    }
                                } else {
                                    // Spread or other: cannot infer
                                    return None;
                                }
                            }
                            // All elements have definite, matching types
                            elem_ty.map(|t| ZigType::ArrayList(Box::new(t)))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
            }

            // ObjectExpression: if all field values are literals, infer field types
            Expression::ObjectExpression(obj) => {
                if obj.properties.is_empty() {
                    // Empty object: cannot infer type
                    None
                } else {
                    // Infer field types from literal values
                    let mut fields: Vec<(String, ZigType)> = Vec::new();
                    for prop in &obj.properties {
                        if let ObjectPropertyKind::ObjectProperty(p) = prop {
                            let field_name = match &p.key {
                                PropertyKey::StaticIdentifier(id) => id.name.to_string(),
                                PropertyKey::StringLiteral(s) => s.value.to_string(),
                                _ => {
                                    // Cannot infer field name: indeterminate
                                    return None;
                                }
                            };
                            let field_ty = self.infer_expr_type(&p.value);
                            match field_ty {
                                Some(t) => {
                                    fields.push((field_name, t));
                                }
                                None => {
                                    // Indeterminate field value: cannot infer object type
                                    return None;
                                }
                            }
                        } else {
                            // Spread property: cannot infer
                            return None;
                        }
                    }
                    Some(ZigType::Struct(fields))
                }
            }

            // ConditionalExpression (ternary: a ? b : c):
            // return type = common type of both branches.
            // If both branches have the same definite type, return that.
            // If one is I64 and the other F64, return F64 (JS numeric coercion).
            // Otherwise None.
            Expression::ConditionalExpression(ce) => {
                let cons_ty = self.infer_expr_type(&ce.consequent);
                let alt_ty = self.infer_expr_type(&ce.alternate);
                match (cons_ty, alt_ty) {
                    (Some(t1), Some(t2)) => {
                        if t1 == t2 {
                            Some(t1)
                        } else {
                            // Numeric coercion: I64 + F64 → F64
                            match (t1, t2) {
                                (ZigType::I64, ZigType::F64) => Some(ZigType::F64),
                                (ZigType::F64, ZigType::I64) => Some(ZigType::F64),
                                _ => None,
                            }
                        }
                    }
                    _ => None,
                }
            }

            // ParenthesizedExpression: unwrap and recurse
            Expression::ParenthesizedExpression(pe) => self.infer_expr_type(&pe.expression),

            // Rule 3: Other expressions → indeterminate
            _ => None,
        }
    }

    /// Check if an expression is a literal (Rule 1, Rule 2).
    fn is_literal(expr: &Expression) -> bool {
        matches!(
            expr,
            Expression::NumericLiteral(_)
                | Expression::StringLiteral(_)
                | Expression::BooleanLiteral(_)
                | Expression::NullLiteral(_)
                | Expression::BigIntLiteral(_)
        )
    }

    /// Infer binary expression result type (both operands are literals).
    fn infer_binary_type(op: BinaryOperator, left: ZigType, right: ZigType) -> ZigType {
        match op {
            // Arithmetic operators
            BinaryOperator::Addition
            | BinaryOperator::Subtraction
            | BinaryOperator::Multiplication
            | BinaryOperator::Division
            | BinaryOperator::Remainder => {
                if left == ZigType::F64 || right == ZigType::F64 {
                    ZigType::F64
                } else {
                    ZigType::I64
                }
            }
            // Exponential: JS `**` always returns number (f64)
            BinaryOperator::Exponential => ZigType::F64,
            // Comparison operators → Bool
            BinaryOperator::Equality
            | BinaryOperator::Inequality
            | BinaryOperator::StrictEquality
            | BinaryOperator::StrictInequality
            | BinaryOperator::LessThan
            | BinaryOperator::LessEqualThan
            | BinaryOperator::GreaterThan
            | BinaryOperator::GreaterEqualThan => ZigType::Bool,
            // Logical operators (for BinaryExpression, these are bitwise)
            BinaryOperator::BitwiseAnd => ZigType::I64,
            BinaryOperator::BitwiseOR => ZigType::I64,
            BinaryOperator::BitwiseXOR => ZigType::I64,
            // Shift operators
            BinaryOperator::ShiftLeft
            | BinaryOperator::ShiftRight
            | BinaryOperator::ShiftRightZeroFill => ZigType::I64,
            // Default
            _ => ZigType::I64,
        }
    }
}

// ── Identifier usage detection ─────────────────────────────────────
// Used to generate `_` for unused for-loop captures (Zig 0.16 compat).
// Returns true if `ident` appears as a free identifier in `expr`.
// Unhandled Expression variants: conservatively return true.

fn expr_uses_ident(ident: &str, expr: &Expression) -> bool {
    match expr {
        Expression::Identifier(id) => id.name.as_str() == ident,
        Expression::BinaryExpression(b) => {
            expr_uses_ident(ident, &b.left) || expr_uses_ident(ident, &b.right)
        }
        Expression::UnaryExpression(u) => expr_uses_ident(ident, &u.argument),
        Expression::StaticMemberExpression(m) => expr_uses_ident(ident, &m.object),
        Expression::ComputedMemberExpression(m) => {
            expr_uses_ident(ident, &m.object) || expr_uses_ident(ident, &m.expression)
        }
        Expression::CallExpression(c) => {
            expr_uses_ident(ident, &c.callee)
                || c.arguments.iter().any(|a| match a.as_expression() {
                    Some(e) => expr_uses_ident(ident, e),
                    None => false,
                })
        }
        Expression::ParenthesizedExpression(p) => expr_uses_ident(ident, &p.expression),
        Expression::ConditionalExpression(c) => {
            expr_uses_ident(ident, &c.test)
                || expr_uses_ident(ident, &c.consequent)
                || expr_uses_ident(ident, &c.alternate)
        }
        // Literals: no identifiers
        Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::RegExpLiteral(_) => false,
        // Conservative: assume identifier MAY appear in unhandled variants
        _ => true,
    }
}

fn stmt_uses_ident(ident: &str, stmt: &Statement) -> bool {
    match stmt {
        Statement::ReturnStatement(r) => r
            .argument
            .as_ref()
            .is_some_and(|e| expr_uses_ident(ident, e)),
        Statement::ExpressionStatement(e) => expr_uses_ident(ident, &e.expression),
        Statement::BlockStatement(b) => b.body.iter().any(|s| stmt_uses_ident(ident, s)),
        _ => false,
    }
}

/// Check if `ident` appears anywhere in the arrow function body.
fn arrow_body_uses_ident(ident: &str, arrow: &ArrowFunctionExpression) -> bool {
    arrow
        .body
        .statements
        .iter()
        .any(|stmt| stmt_uses_ident(ident, stmt))
}
