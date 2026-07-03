// codegen/builtins.rs
// Per-category sub-functions for emit_builtin_call.
// Extracted from the 3,300-line monolith in expr.rs to improve readability and maintainability.

use super::Codegen;
use crate::native_builtins as builtins;
use crate::types::ZigType;
use oxc_ast::ast::*;

// ── Math ──────────────────────────────────────────────

impl Codegen {
    pub(crate) fn emit_builtin_math(
        &mut self,
        builtin: &builtins::BuiltinCall,
        ce: &CallExpression,
    ) -> bool {
        match builtin {
            builtins::BuiltinCall::MathRandom => {
                if !ce.arguments.is_empty() {
                    self.errors
                        .push("Math.random() requires no arguments".to_string());
                    return false;
                }
                self.write("(@as(f64, @floatFromInt(std.crypto.random.int(u32))) / @as(f64, 4294967295.0))");
                true
            }

            builtins::BuiltinCall::MathPow => {
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
                    0 => {
                        self.write("@as(i64, -9223372036854775808)");
                        true
                    }
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
                    0 => {
                        self.write("@as(i64, 9223372036854775807)");
                        true
                    }
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
                if ce.arguments.is_empty() {
                    self.write("0");
                } else if ce.arguments.len() == 1 {
                    self.write("@abs(@as(f64, ");
                    self.emit_first_arg(&ce.arguments);
                    self.write("))");
                } else {
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

            builtins::BuiltinCall::MathAtan2 => {
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

            builtins::BuiltinCall::MathSign => {
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("Math.sign() requires exactly 1 argument".to_string());
                    return false;
                }
                self.write("(if (@as(f64, @floatFromInt(");
                self.emit_first_arg(&ce.arguments);
                self.write(")) > 0) @as(f64, 1.0) else if (@as(f64, @floatFromInt(");
                self.emit_first_arg(&ce.arguments);
                self.write(")) < 0) @as(f64, -1.0) else @as(f64, @floatFromInt(");
                self.emit_first_arg(&ce.arguments);
                self.write(")))");
                true
            }

            builtins::BuiltinCall::MathClz32 => {
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
                if ce.arguments.len() != 2 {
                    self.errors
                        .push("Math.imul() requires exactly 2 arguments".to_string());
                    return false;
                }
                self.write("@as(i32, @intCast((");
                self.write("@as(u32, @bitCast(@as(i32, ");
                self.emit_first_arg(&ce.arguments);
                self.write("))) *% (");
                self.write("@as(u32, @bitCast(@as(i32, ");
                if let Some(arg1) = ce.arguments.get(1)
                    && let Some(expr) = arg1.as_expression()
                {
                    self.emit_expr(expr);
                }
                self.write("))))");
                true
            }

            // Math 1-arg (table-driven, 25 methods)
            b if super::tables::math_one_arg_desc(b).is_some() => {
                let desc = super::tables::math_one_arg_desc(b).unwrap();
                self.emit_math_one_arg(&desc, ce)
            }

            _ => false,
        }
    }
}

// ── Console ───────────────────────────────────────────

impl Codegen {
    pub(crate) fn emit_builtin_console(
        &mut self,
        builtin: &builtins::BuiltinCall,
        ce: &CallExpression,
    ) -> bool {
        let (fn_name, multi_fn_name) = match builtin {
            builtins::BuiltinCall::ConsoleLog => ("log", "logMulti"),
            builtins::BuiltinCall::ConsoleError => ("err", "errMulti"),
            builtins::BuiltinCall::ConsoleWarn => ("warn", "warnMulti"),
            _ => return false,
        };

        if ce.arguments.len() <= 1 {
            self.write(&format!("js_console.{fn_name}("));
            self.emit_first_arg(&ce.arguments);
            self.write(")");
        } else {
            self.write(&format!("js_console.{multi_fn_name}(.{{ "));
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
}

// ── JSON ───────────────────────────────────────────────

impl Codegen {
    pub(crate) fn emit_builtin_json(
        &mut self,
        builtin: &builtins::BuiltinCall,
        ce: &CallExpression,
    ) -> bool {
        match builtin {
            builtins::BuiltinCall::JsonStringify => {
                // JSON.stringify(value, replacer?, space?) → try js_json.stringify(js_allocator.allocator(), value, replacer, space)
                self.write("try js_json.stringify(js_allocator.allocator(), ");
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
                // JSON.parse(text, reviver?) → try js_json.parse(js_allocator.allocator(), text, reviver)
                self.write("try js_json.parse(js_allocator.allocator(), ");
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

            _ => false,
        }
    }
}

// ── Symbol ─────────────────────────────────────────────

impl Codegen {
    pub(crate) fn emit_builtin_symbol(
        &mut self,
        builtin: &builtins::BuiltinCall,
        ce: &CallExpression,
    ) -> bool {
        match builtin {
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

            _ => false,
        }
    }
}

// ── RegExp ─────────────────────────────────────────────

impl Codegen {
    pub(crate) fn emit_builtin_regexp(
        &mut self,
        builtin: &builtins::BuiltinCall,
        ce: &CallExpression,
    ) -> bool {
        match builtin {
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
                        let escaped = super::expr::escape_zig_string(&re.regex.pattern.text);
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
                        self.write("js_regexp.execLiteral(js_allocator.allocator(), ");
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
                        self.write(".exec(js_allocator.allocator(), ");
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

            _ => false,
        }
    }
}

// ── Number ─────────────────────────────────────────────

impl Codegen {
    pub(crate) fn emit_builtin_number(
        &mut self,
        builtin: &builtins::BuiltinCall,
        ce: &CallExpression,
    ) -> bool {
        match builtin {
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
                        "js_number.toFixed(js_allocator.allocator(), {}, ",
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
                        "js_number.toExponential(js_allocator.allocator(), {}, ",
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
                        "js_number.toPrecision(js_allocator.allocator(), {}, ",
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

            _ => false,
        }
    }
}

// ── Global ─────────────────────────────────────────────

impl Codegen {
    pub(crate) fn emit_builtin_global(
        &mut self,
        builtin: &builtins::BuiltinCall,
        ce: &CallExpression,
    ) -> bool {
        match builtin {
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
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("encodeURIComponent() requires exactly 1 argument".to_string());
                    return false;
                }
                self.write("js_uri.encodeURIComponent(js_allocator.allocator(), ");
                self.emit_first_arg(&ce.arguments);
                self.write(") catch @panic(\"OOM: encodeURIComponent\")");
                true
            }

            builtins::BuiltinCall::EncodeURI => {
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("encodeURI() requires exactly 1 argument".to_string());
                    return false;
                }
                self.write("js_uri.encodeURI(js_allocator.allocator(), ");
                self.emit_first_arg(&ce.arguments);
                self.write(") catch @panic(\"OOM: encodeURI\")");
                true
            }

            builtins::BuiltinCall::DecodeURIComponent => {
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("decodeURIComponent() requires exactly 1 argument".to_string());
                    return false;
                }
                if let Some(ref body_blk) = self.inside_try_block {
                    let body_blk = body_blk.clone();
                    self.call_generated_catch = true;
                    self.write("_ = js_uri.decodeURIComponent(js_allocator.allocator(), ");
                    self.emit_first_arg(&ce.arguments);
                    self.write(") catch |_| { break :");
                    self.write(&body_blk);
                    self.write(" error.JsThrow; }\n");
                    return true;
                } else {
                    self.write("js_uri.decodeURIComponent(js_allocator.allocator(), ");
                    self.emit_first_arg(&ce.arguments);
                    self.write(") catch \"\"");
                }
                true
            }

            builtins::BuiltinCall::DecodeURI => {
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("decodeURI() requires exactly 1 argument".to_string());
                    return false;
                }
                if let Some(ref body_blk) = self.inside_try_block {
                    let body_blk = body_blk.clone();
                    self.call_generated_catch = true;
                    self.write_indent();
                    self.write("_ = js_uri.decodeURI(js_allocator.allocator(), ");
                    self.emit_first_arg(&ce.arguments);
                    self.write(") catch |_| { break :");
                    self.write(&body_blk);
                    self.write(" error.JsThrow; }\n");
                    return true;
                } else {
                    self.write("js_uri.decodeURI(js_allocator.allocator(), ");
                    self.emit_first_arg(&ce.arguments);
                    self.write(") catch \"\"");
                }
                true
            }

            _ => false,
        }
    }
}

// ── Constructors ───────────────────────────────────────

impl Codegen {
    pub(crate) fn emit_builtin_constructors(
        &mut self,
        builtin: &builtins::BuiltinCall,
        ce: &CallExpression,
    ) -> bool {
        match builtin {
            builtins::BuiltinCall::NumberConstructor => {
                // Number(x) — type-aware conversion
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
                            self.emit_expr(expr);
                            self.write(".asF64()");
                        }
                        Some(ZigType::Bool) => {
                            self.write("if (");
                            self.emit_expr(expr);
                            self.write(") @as(f64, 1.0) else @as(f64, 0.0)");
                        }
                        Some(ZigType::BigInt) => {
                            self.write("@as(f64, @floatFromInt((");
                            self.emit_expr(expr);
                            self.write(
                                ").toI64() catch @panic(\"BigInt too large for Number()\")))",
                            );
                        }
                        _ => {
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
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("String() requires exactly 1 argument".to_string());
                    return false;
                }
                let blk = self.next_label();
                self.write(&format!("({}: {{ const _val = ", blk));
                self.emit_first_arg(&ce.arguments);
                self.write("; ");
                self.write(&format!("break :{} std.fmt.allocPrint(js_allocator.allocator(), \"{{d}}\", .{{_val}}) catch @panic(\"OOM\"); }})", blk));
                true
            }

            builtins::BuiltinCall::BooleanConstructor => {
                // Boolean(x) → bool, type-dependent conversion
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("Boolean() requires exactly 1 argument".to_string());
                    return false;
                }
                let arg = &ce.arguments[0];
                if let Some(expr) = arg.as_expression() {
                    match self.infer_expr_type(expr) {
                        Some(ZigType::Bool) => {
                            self.emit_expr(expr);
                        }
                        Some(ZigType::Str) => {
                            self.write("((");
                            self.emit_expr(expr);
                            self.write(").len != 0)");
                        }
                        Some(ZigType::I64) | Some(ZigType::F64) => {
                            self.write("(");
                            self.emit_expr(expr);
                            self.write(" != 0)");
                        }
                        _ => {
                            if let Expression::NullLiteral(_) = expr {
                                self.write("false");
                                return true;
                            }
                            if let Expression::Identifier(id) = expr
                                && id.name.as_str() == "undefined"
                            {
                                self.write("false");
                                return true;
                            }
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
                self.write("(js_bigint.JsBigInt.fromI64(js_allocator.allocator(), ");
                self.emit_first_arg(&ce.arguments);
                self.write(") catch @panic(\"OOM: BigInt fromI64\"))");
                true
            }

            builtins::BuiltinCall::ObjectConstructor => {
                if ce.arguments.is_empty() {
                    self.compile_error(ce.span, "Object() without arguments would create an empty object which is not supported in native_proto mode. Use struct literal {} instead.");
                } else {
                    self.emit_first_arg(&ce.arguments);
                }
                true
            }

            builtins::BuiltinCall::Eval => {
                self.compile_error(ce.span, "eval() is not supported (security risk, cannot dynamically execute at compile time)");
                true
            }

            _ => false,
        }
    }
}

// ── Date ───────────────────────────────────────────────

impl Codegen {
    pub(crate) fn emit_builtin_date(
        &mut self,
        builtin: &builtins::BuiltinCall,
        ce: &CallExpression,
    ) -> bool {
        match builtin {
            builtins::BuiltinCall::DateNow => {
                self.write("js_date.now()");
                true
            }
            builtins::BuiltinCall::DateParse => {
                self.write("js_date.parse(");
                self.emit_first_arg(&ce.arguments);
                self.write(")");
                true
            }
            builtins::BuiltinCall::DateUTC => {
                self.write("js_date.utc(");
                for (i, arg) in ce.arguments.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.emit_expr_arg(arg);
                }
                match ce.arguments.len() {
                    0 => self.write("1970, 0, 1, 0, 0, 0, 0"),
                    1 => self.write(", 0, 1, 0, 0, 0, 0"),
                    2 => self.write(", 1, 0, 0, 0, 0"),
                    3 => self.write(", 0, 0, 0, 0"),
                    4 => self.write(", 0, 0, 0"),
                    5 => self.write(", 0, 0"),
                    6 => self.write(", 0"),
                    7 => {}
                    _ => {}
                }
                self.write(")");
                true
            }

            // ── Date instance methods ────────────────────
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
                if let Expression::StaticMemberExpression(mem) = &ce.callee {
                    self.write("try ");
                    self.emit_expr(&mem.object);
                    self.write(".toISOString(js_allocator.allocator())");
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
                if let Expression::StaticMemberExpression(mem) = &ce.callee {
                    self.write("try ");
                    self.emit_expr(&mem.object);
                    self.write(".toString(js_allocator.allocator())");
                    true
                } else {
                    self.errors
                        .push("Date.toString() called on non-static-member expression".to_string());
                    false
                }
            }
            builtins::BuiltinCall::DateToDateString => {
                if let Expression::StaticMemberExpression(mem) = &ce.callee {
                    self.write("try ");
                    self.emit_expr(&mem.object);
                    self.write(".toDateString(js_allocator.allocator())");
                    true
                } else {
                    self.errors.push(
                        "Date.toDateString() called on non-static-member expression".to_string(),
                    );
                    false
                }
            }
            builtins::BuiltinCall::DateToTimeString => {
                if let Expression::StaticMemberExpression(mem) = &ce.callee {
                    self.write("try ");
                    self.emit_expr(&mem.object);
                    self.write(".toTimeString(js_allocator.allocator())");
                    true
                } else {
                    self.errors.push(
                        "Date.toTimeString() called on non-static-member expression".to_string(),
                    );
                    false
                }
            }
            builtins::BuiltinCall::DateToLocaleString => {
                if let Expression::StaticMemberExpression(mem) = &ce.callee {
                    self.write("try ");
                    self.emit_expr(&mem.object);
                    self.write(".toLocaleString(js_allocator.allocator())");
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
                if ce.arguments.is_empty() {
                    if let Expression::StaticMemberExpression(mem) = &ce.callee {
                        self.write("try ");
                        self.emit_expr(&mem.object);
                        self.write(".toJSON(js_allocator.allocator())");
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

            // ── Date setters (table-driven) ──
            b if Self::date_setter_method_name(b).is_some() => {
                let method = Self::date_setter_method_name(b).unwrap();
                self.emit_date_setter_method(method, ce)
            }

            _ => false,
        }
    }
}

// ── Object ─────────────────────────────────────────────

impl Codegen {
    pub(crate) fn emit_builtin_object(
        &mut self,
        builtin: &builtins::BuiltinCall,
        ce: &CallExpression,
    ) -> bool {
        match builtin {
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
                self.write("js_object.keys(js_allocator.allocator(), ");
                self.emit_first_arg(&ce.arguments);
                self.write(")");
                true
            }
            builtins::BuiltinCall::ObjectValues => {
                // Object.values(obj) → js_object.values(alloc, obj)
                self.write("js_object.values(js_allocator.allocator(), ");
                self.emit_first_arg(&ce.arguments);
                self.write(")");
                true
            }
            builtins::BuiltinCall::ObjectEntries => {
                // Object.entries(obj) → js_object.entries(alloc, obj)
                self.write("js_object.entries(js_allocator.allocator(), ");
                self.emit_first_arg(&ce.arguments);
                self.write(")");
                true
            }
            builtins::BuiltinCall::ObjectFromEntries => {
                // Object.fromEntries(iterable) → js_object.fromEntries(alloc, iterable)
                self.write("js_object.fromEntries(js_allocator.allocator(), ");
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
                self.write("js_object.create(js_allocator.allocator(), ");
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
                self.write("js_object.getOwnPropertyDescriptor(js_allocator.allocator(), ");
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

            _ => false,
        }
    }

    /// Emit Map/Set builtin method calls.
    /// Handles: MapSet, MapGet, MapHas, MapDelete, MapKeys, MapValues, MapEntries,
    ///          MapClear, SetAdd, SetForEach, SetKeys, SetValues, SetEntries.
    /// Note: MapSet/MapGet include TypedArray .get/.set dispatch (checks typedarray_vars first).
    pub(crate) fn emit_builtin_map_set(
        &mut self,
        builtin: &builtins::BuiltinCall,
        ce: &CallExpression,
    ) -> bool {
        match builtin {
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
                // map.set(key, value) → map.set(JsAny.from(key), JsAny.from(value)) catch @panic("OOM")
                if ce.arguments.len() != 2 {
                    self.errors
                        .push("Map.set() requires exactly 2 arguments".to_string());
                    return false;
                }
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    self.write(&format!("{}.set(JsAny.from(", obj_name));
                    self.emit_first_arg(&ce.arguments);
                    self.write("), JsAny.from(");
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
                // map.get(key) → map.get(JsAny.from(key))
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("Map.get() requires exactly 1 argument".to_string());
                    return false;
                }
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
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

            builtins::BuiltinCall::MapKeys => {
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    self.write(&format!(
                        "{}.keys(js_allocator.allocator()) catch @panic(\"OOM: allocation\")",
                        obj_name
                    ));
                    return true;
                }
                false
            }
            builtins::BuiltinCall::MapValues => {
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    self.write(&format!(
                        "{}.values(js_allocator.allocator()) catch @panic(\"OOM: allocation\")",
                        obj_name
                    ));
                    return true;
                }
                false
            }
            builtins::BuiltinCall::MapEntries => {
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    self.write(&format!(
                        "{}.entries(js_allocator.allocator()) catch @panic(\"OOM: allocation\")",
                        obj_name
                    ));
                    return true;
                }
                false
            }

            builtins::BuiltinCall::MapClear => {
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

            // ── Set methods ─────────────────────────────
            builtins::BuiltinCall::SetAdd => {
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

            builtins::BuiltinCall::SetForEach => {
                // set.forEach(fn) → for (set.items.items) |value| { ... }
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    if !ce.arguments.is_empty()
                        && let Some(arg) = ce.arguments.first()
                        && let Some(Expression::ArrowFunctionExpression(arrow)) =
                            arg.as_expression()
                    {
                        let val_param = arrow
                            .params
                            .items
                            .first()
                            .and_then(|p| crate::infer::binding_name(&p.pattern));

                        let val_name = val_param.unwrap_or("_item");
                        self.write(&format!(
                            "for ({obj}.items.items) |{val}| {{\n",
                            obj = obj_name,
                            val = val_name
                        ));
                        self.indent += 1;

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
                    self.write(&format!("for ({}.items.items) |_| {{}}", obj_name));
                    return true;
                }
                false
            }
            builtins::BuiltinCall::SetKeys => {
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    self.write(&format!(
                        "{}.keys(js_allocator.allocator()) catch @panic(\"OOM: allocation\")",
                        obj_name
                    ));
                    return true;
                }
                false
            }
            builtins::BuiltinCall::SetValues => {
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    self.write(&format!(
                        "{}.values(js_allocator.allocator()) catch @panic(\"OOM: allocation\")",
                        obj_name
                    ));
                    return true;
                }
                false
            }
            builtins::BuiltinCall::SetEntries => {
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    self.write(&format!(
                        "{}.entries(js_allocator.allocator()) catch @panic(\"OOM: allocation\")",
                        obj_name
                    ));
                    return true;
                }
                false
            }

            _ => false,
        }
    }

    /// Emit String builtin method calls (non-runtime-table).
    /// Handles: StringMatch, StringSearch, StringMatchAll, StringFromCharCode, StringFromCodePoint.
    /// Note: The 27 string runtime table methods (via string_runtime_desc) are dispatched separately.
    pub(crate) fn emit_builtin_string(
        &mut self,
        builtin: &builtins::BuiltinCall,
        ce: &CallExpression,
    ) -> bool {
        match builtin {
            builtins::BuiltinCall::StringMatch => {
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
                            let has_global = re
                                .raw
                                .as_ref()
                                .map(|raw| {
                                    let raw_str = raw.as_str();
                                    raw_str.rfind('/').is_some_and(|idx| {
                                        let flags_part = &raw_str[idx + 1..];
                                        flags_part.contains('g')
                                    })
                                })
                                .unwrap_or(false);
                            if has_global {
                                self.write(&format!(
                                    "js_string.matchStringGlobal(js_allocator.allocator(), {}, \"{}\") catch @panic(\"OOM: allocation\")",
                                    obj_repr, escaped
                                ));
                            } else {
                                self.write(&format!(
                                    "js_string.matchString(js_allocator.allocator(), {}, \"{}\") catch @panic(\"OOM: allocation\")",
                                    obj_repr, escaped
                                ));
                            }
                        }
                        Expression::Identifier(id)
                            if self.regexp_vars.contains(id.name.as_str()) =>
                        {
                            self.write(&format!(
                                "js_string.matchString(js_allocator.allocator(), {}, {}.pattern) catch @panic(\"OOM: allocation\")",
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

            builtins::BuiltinCall::StringMatchAll => {
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
                                "js_string.matchAllString(js_allocator.allocator(), {}, \"{}\") catch @panic(\"OOM: allocation\")",
                                obj_repr, escaped
                            ));
                        }
                        Expression::Identifier(id)
                            if self.regexp_vars.contains(id.name.as_str()) =>
                        {
                            self.write(&format!(
                                "js_string.matchAllString(js_allocator.allocator(), {}, {}.pattern) catch @panic(\"OOM: allocation\")",
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

            builtins::BuiltinCall::StringFromCharCode => {
                self.write("js_string.fromCharCode(js_allocator.allocator()");
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
                self.write("js_string.fromCodePoint(js_allocator.allocator()");
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

            _ => false,
        }
    }
}

// ── Array / TypedArray ─────────────────────────────────

impl Codegen {
    pub(crate) fn emit_builtin_array(
        &mut self,
        builtin: &builtins::BuiltinCall,
        ce: &CallExpression,
    ) -> bool {
        match builtin {
            builtins::BuiltinCall::ArrayPush => {
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    self.write(&format!("{}.append(js_allocator.allocator(), ", obj_name));
                    self.emit_comma_separated_args(&ce.arguments);
                    self.write(") catch @panic(\"OOM: Array.push\")");
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArrayPop => {
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    self.write(&format!("{}.pop()", obj_name));
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArrayShift => {
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    self.write(&format!(
                        "if ({obj}.items.len > 0) {obj}.orderedRemove(0)",
                        obj = obj_name
                    ));
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArrayUnshift => {
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    self.write(&format!(
                        "{}.insert(js_allocator.allocator(), 0, ",
                        obj_name
                    ));
                    self.emit_comma_separated_args(&ce.arguments);
                    self.write(") catch @panic(\"OOM: allocation\")");
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArrayReverse => {
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
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("Array.indexOf() requires exactly 1 argument".to_string());
                    return false;
                }
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    if self.type_info.var_types.get(obj_name) == Some(&ZigType::Str) {
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
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("Array.includes() requires exactly 1 argument".to_string());
                    return false;
                }
                let obj_repr = match self.callee_object_name(&ce.callee) {
                    Some(name) => Some(name.to_string()),
                    None => self.callee_object_repr_mut(&ce.callee),
                };
                if let Some(obj_name) = obj_repr {
                    if self.type_info.var_types.get(obj_name.as_str()) == Some(&ZigType::Str) {
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
                if ce.arguments.len() != 1 {
                    self.errors
                        .push("Array.join() requires exactly 1 argument".to_string());
                    return false;
                }
                let obj_repr = match self.callee_object_name(&ce.callee) {
                    Some(name) => Some(name.to_string()),
                    None => self.callee_object_repr_mut(&ce.callee),
                };
                if let Some(obj_name) = obj_repr {
                    let sep_expr = self.first_arg_string(&ce.arguments);
                    let fmt_spec = match self.type_info.array_element_types.get(obj_name.as_str()) {
                        Some(ZigType::I64) => "{d}",
                        Some(ZigType::F64) => "{d}",
                        Some(ZigType::Bool) => "{}",
                        Some(ZigType::Str) => "{s}",
                        _ => "{any}",
                    };
                    let blk = self.next_label();
                    self.write(&format!(
                            "({blk}: {{ var __join_buf = std.io.Writer.Allocating.init(js_allocator.allocator()); for ({obj}.items, 0..) |__item, __i| {{ if (__i > 0) __join_buf.writer().writeAll({sep}) catch break :{blk} \"\"; __join_buf.writer().print(\"{fmt}\", .{{__item}}) catch break :{blk} \"\"; }} break :{blk} __join_buf.toOwnedSlice() catch \"\"; }})",
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
                let obj_name = self.callee_object_name(&ce.callee);
                if let (Some(obj_name), Some(ta_type)) = (
                    obj_name,
                    obj_name.and_then(|n| self.typedarray_vars.get(n).cloned()),
                ) {
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
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
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
                        "({0}: {{ var __slice: std.ArrayList({1}) = .empty; __slice.appendSlice(js_allocator.allocator(), {2}) catch @panic(\"OOM: Array.slice appendSlice\"); break :{0} __slice; }})",
                        blk, elem_type, slice_expr
                    ));
                    return true;
                }
                false
            }

            builtins::BuiltinCall::TypedArraySubarray => {
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
                        "({0}: {{ var __spliced: std.ArrayList({1}) = .empty; const __start = @as(usize, @intCast(@max(0, {2}))); const __cnt = @as(usize, @intCast(@min(@max(0, {3}), {4}.items.len -| __start))); var __i: usize = 0; while (__i < __cnt) : (__i += 1) {{ __spliced.append(js_allocator.allocator(), {4}.orderedRemove(__start)) catch @panic(\"OOM: Array.splice\"); }}", 
                        blk, elem_type, start_expr, count_expr, obj_name
                    ));
                    if ce.arguments.len() > 2 {
                        self.write(&format!(
                            " {0}.insertSlice(js_allocator.allocator(), __start, &[_]{1}{{",
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
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    let elem_type = self
                        .type_info
                        .array_element_types
                        .get(obj_name)
                        .map(|t| t.to_zig_type())
                        .unwrap_or_else(|| "i64".to_string());
                    let blk = self.next_label();
                    self.write(&format!("({}: {{ ", blk));
                    self.write(&format!(
                        "var __concat: std.ArrayList({0}) = .empty; ",
                        elem_type
                    ));
                    self.write(&format!(
                        "__concat.appendSlice(js_allocator.allocator(), {}.items) catch @panic(\"OOM: Array.concat appendSlice\"); ",
                        obj_name
                    ));
                    for arg in &ce.arguments {
                        if let Some(expr) = arg.as_expression() {
                            let arg_str = self.emit_expr_to_string(expr);
                            self.write(&format!(
                                "__concat.appendSlice(js_allocator.allocator(), {}.items) catch @panic(\"OOM: Array.concat appendSlice\"); ",
                                arg_str
                            ));
                        }
                    }
                    self.write(&format!("break :{} __concat; }})", blk));
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArrayKeys => {
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    self.write(&format!(
                        "js_runtime.js_array.keys(js_allocator.allocator(), &{}) catch @panic(\"OOM: allocation\")",
                        obj_name
                    ));
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArrayValues => {
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    self.write(&format!(
                        "js_runtime.js_array.values(js_allocator.allocator(), &{}) catch @panic(\"OOM: allocation\")",
                        obj_name
                    ));
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArrayEntries => {
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    self.write(&format!(
                        "js_runtime.js_array.entries(js_allocator.allocator(), &{}) catch @panic(\"OOM: allocation\")",
                        obj_name
                    ));
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArrayForEach => {
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    let is_map = self
                        .type_info
                        .var_types
                        .get(obj_name)
                        .is_some_and(|t| matches!(t, ZigType::NamedStruct(s) if s == "Map"));

                    if is_map {
                        if !ce.arguments.is_empty()
                            && let Some(arg) = ce.arguments.first()
                            && let Some(Expression::ArrowFunctionExpression(arrow)) =
                                arg.as_expression()
                        {
                            let val_param = arrow
                                .params
                                .items
                                .first()
                                .and_then(|p| crate::infer::binding_name(&p.pattern));
                            let key_param = arrow
                                .params
                                .items
                                .get(1)
                                .and_then(|p| crate::infer::binding_name(&p.pattern));

                            self.write(&format!("var iter = {}.inner.iterator();\n", obj_name));
                            self.write_indent();
                            self.write("while (iter.next()) |entry| {\n");
                            self.indent += 1;
                            if let Some(vp) = &val_param {
                                self.write_indent();
                                self.write(&format!("const {} = entry.value_ptr.*;\n", vp));
                            }
                            if let Some(kp) = &key_param {
                                self.write_indent();
                                self.write(&format!("const {} = entry.key_ptr.*;\n", kp));
                            }
                            for stmt in &arrow.body.statements {
                                self.write_indent();
                                self.emit_fn_stmt(stmt);
                            }
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
                        self.write(&format!("var iter = {}.inner.iterator();\n", obj_name));
                        self.write_indent();
                        self.write("while (iter.next()) |_| {}\n");
                        return true;
                    }

                    if !ce.arguments.is_empty()
                        && let Some(arg) = ce.arguments.first()
                        && let Some(Expression::ArrowFunctionExpression(arrow)) =
                            arg.as_expression()
                    {
                        self.write(&format!("for ({}.items) |", obj_name));
                        if arrow.params.items.len() == 1 {
                            if let Some(param_name) =
                                crate::infer::binding_name(&arrow.params.items[0].pattern)
                            {
                                self.write(&format!("{}| {{ ", param_name));
                            } else {
                                self.write("_| {{ ");
                            }
                        } else {
                            self.write("_| {{ ");
                        }
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
                    self.write(&format!("for ({}.items) |_| {{}}", obj_name));
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArrayMap => {
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
                        let blk = self.next_label();
                        self.write(&format!("({}: {{ ", blk));
                        self.write(&format!(
                            "var __filter: std.ArrayList({0}) = .empty; ",
                            elem_type
                        ));
                        self.write(&format!("for ({}.items) |", obj_name));
                        let param_name = if !arrow.params.items.is_empty() {
                            crate::infer::binding_name(&arrow.params.items[0].pattern)
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
                                    self.write(") { __filter.append(js_allocator.allocator(), ");
                                    self.write(&param_name);
                                    self.write(") catch @panic(\"OOM: Array.filter append\"); }");
                                }
                            } else if let Statement::ExpressionStatement(es) = stmt {
                                self.write("if (");
                                self.emit_expr(&es.expression);
                                self.write(") { __filter.append(js_allocator.allocator(), ");
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
                    self.write(&obj_name);
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArrayReduce => {
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    let init_expr = if ce.arguments.len() >= 2
                        && let Some(arg) = ce.arguments.get(1)
                        && let Some(expr) = arg.as_expression()
                    {
                        self.emit_expr_to_string(expr)
                    } else {
                        "0".to_string()
                    };
                    if !ce.arguments.is_empty()
                        && let Some(arg) = ce.arguments.first()
                        && let Some(Expression::ArrowFunctionExpression(arrow)) =
                            arg.as_expression()
                    {
                        let blk = self.next_label();
                        self.write(&format!("({}: {{ ", blk));
                        let acc_type = if init_expr.contains(".") {
                            "f64"
                        } else {
                            "i64"
                        };
                        self.write(&format!("var acc: {} = {}; ", acc_type, init_expr));
                        self.write(&format!("for ({}.items) |", obj_name));
                        if arrow.params.items.len() >= 2 {
                            if let Some(param_name) =
                                crate::infer::binding_name(&arrow.params.items[1].pattern)
                            {
                                self.write(&format!("{}| {{ ", param_name));
                            } else {
                                self.write("_| { ");
                            }
                        } else {
                            self.write("_| { ");
                        }
                        self.indent += 1;
                        for stmt in &arrow.body.statements {
                            self.write_indent();
                            if let Statement::ReturnStatement(ret) = stmt {
                                if let Some(expr) = &ret.argument {
                                    self.write("acc = ");
                                    self.emit_expr(expr);
                                    self.write(";");
                                }
                            } else if let Statement::ExpressionStatement(es) = stmt {
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
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    if !ce.arguments.is_empty()
                        && let Some(arg) = ce.arguments.first()
                        && let Some(Expression::ArrowFunctionExpression(arrow)) =
                            arg.as_expression()
                    {
                        let idx_param = if arrow.params.items.len() >= 2 {
                            crate::infer::binding_name(&arrow.params.items[1].pattern)
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
                            crate::infer::binding_name(&arrow.params.items[0].pattern)
                                .unwrap_or("_")
                                .to_string()
                        } else {
                            "_".to_string()
                        };
                        let param_name = if !super::expr::arrow_body_uses_ident(&param_name, arrow)
                        {
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
                    self.write("false");
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArrayEvery => {
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    if !ce.arguments.is_empty()
                        && let Some(arg) = ce.arguments.first()
                        && let Some(Expression::ArrowFunctionExpression(arrow)) =
                            arg.as_expression()
                    {
                        let idx_param = if arrow.params.items.len() >= 2 {
                            crate::infer::binding_name(&arrow.params.items[1].pattern)
                                .unwrap_or("_")
                                .to_string()
                        } else {
                            String::new()
                        };
                        let has_idx = !idx_param.is_empty();
                        let idx_param =
                            if has_idx && !super::expr::arrow_body_uses_ident(&idx_param, arrow) {
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
                            crate::infer::binding_name(&arrow.params.items[0].pattern)
                                .unwrap_or("_")
                                .to_string()
                        } else {
                            "_".to_string()
                        };
                        let param_name = if !super::expr::arrow_body_uses_ident(&param_name, arrow)
                        {
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
                    self.write("true");
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArrayFlat => {
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    self.write(obj_name);
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArrayFlatMap => {
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    self.write(obj_name);
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArrayFind => {
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
                            crate::infer::binding_name(&arrow.params.items[0].pattern)
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
                    self.write("undefined");
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArrayFindIndex => {
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
                            crate::infer::binding_name(&arrow.params.items[0].pattern)
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
                    self.write("-1");
                    return true;
                }
                false
            }

            builtins::BuiltinCall::ArrayFindLast => {
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    if !ce.arguments.is_empty()
                        && let Some(arg) = ce.arguments.first()
                        && let Some(Expression::ArrowFunctionExpression(arrow)) =
                            arg.as_expression()
                    {
                        let param_name = if !arrow.params.items.is_empty() {
                            crate::infer::binding_name(&arrow.params.items[0].pattern)
                                .unwrap_or("_")
                                .to_string()
                        } else {
                            "_".to_string()
                        };
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

            builtins::BuiltinCall::ArrayFindLastIndex => {
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    if !ce.arguments.is_empty()
                        && let Some(arg) = ce.arguments.first()
                        && let Some(Expression::ArrowFunctionExpression(arrow)) =
                            arg.as_expression()
                    {
                        let param_name = if !arrow.params.items.is_empty() {
                            crate::infer::binding_name(&arrow.params.items[0].pattern)
                                .unwrap_or("_")
                                .to_string()
                        } else {
                            "_".to_string()
                        };
                        let idx_name = format!("__{}_idx", param_name);
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

            builtins::BuiltinCall::ArrayReduceRight => {
                self.write("undefined");
                true
            }

            builtins::BuiltinCall::ArrayFill => {
                if let Some(obj_name) = self.callee_object_name(&ce.callee) {
                    if self.typedarray_vars.contains_key(obj_name) {
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

            builtins::BuiltinCall::ArrayAt => {
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

            builtins::BuiltinCall::ArrayFrom => {
                self.write("js_array.from(js_allocator.allocator()");
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
                self.write("js_array.of(js_allocator.allocator()");
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
                self.write("js_array.isArray(");
                if let Some(first) = ce.arguments.first()
                    && let Some(expr) = first.as_expression()
                {
                    self.emit_expr(expr);
                }
                self.write(")");
                true
            }

            _ => false,
        }
    }
}
