// zigir/emit/expr/template_new.rs
// Template literal and new expression emission.

use crate::zigir::emit::Emitter;
use crate::zigir::emit::helpers::EmitterHelpers;
use crate::zigir::kinds::{DateConstructorKind, NewConstructor, TypedArrayKind};

impl Emitter {
    pub(super) fn emit_template_literal(
        &mut self,
        parts: &[String],
        exprs: &[crate::zigir::types::IrExpr],
        format_specs: &[String],
    ) {
        use crate::zigir::emit::helpers::escape_zig_format_string;
        // Zig template literal → std.fmt.allocPrint
        if exprs.is_empty() {
            // No expressions: just a string literal
            self.write(&format!("\"{}\"", escape_zig_format_string(&parts[0])));
        } else {
            // Build the format string: parts[0] + {spec0} + parts[1] + {spec1} + ...
            let mut fmt = String::new();
            for (i, part) in parts.iter().enumerate() {
                if i > 0 && i - 1 < format_specs.len() {
                    fmt.push_str(&format_specs[i - 1]);
                }
                fmt.push_str(&escape_zig_format_string(part));
            }
            // Emit args as a separate pass to get their string representations
            self.emit_alloc_print(&fmt, exprs);
        }
    }

    pub(super) fn emit_new_expr(&mut self, new_expr: &crate::zigir::types::IrNewExpr) {
        use crate::zigir::emit::helpers;
        match &new_expr.constructor {
            NewConstructor::Map => {
                if new_expr.args.is_empty() {
                    self.write("js_collections.JsMap.init(js_allocator.allocator())");
                } else {
                    // new Map([["a",1],["b",2],...]) → block with init + set calls
                    let blk = self.next_label();
                    self.write(&format!(
                        "({blk}: {{ var __map = js_collections.JsMap.init(js_allocator.allocator()); "
                    ));
                    // If the arg is an ArrayLiteral without spread, emit each
                    // pair directly to avoid type issues with intermediate
                    // ArrayLists (JsAny.from(ArrayList) is not supported, and
                    // JsAny does not support .at() for indexing).
                    match &new_expr.args[0] {
                        crate::zigir::types::IrExpr::ArrayLiteral(arr)
                            if arr.spread_indices.is_empty() =>
                        {
                            for elem in &arr.elements {
                                if let crate::zigir::types::IrExpr::ArrayLiteral(pair) = elem
                                    && pair.elements.len() == 2
                                {
                                    self.write("__map.set(js_allocator.allocator(), JsAny.from(");
                                    self.emit_expr(&pair.elements[0]);
                                    self.write("), JsAny.from(");
                                    self.emit_expr(&pair.elements[1]);
                                    self.write(")) catch @panic(\"OOM: Map init\"); ");
                                }
                            }
                        }
                        _ => {
                            // Fallback: build the array and iterate. Each entry
                            // is ArrayList(JsAny), use .at() for key/value.
                            self.write("for (");
                            self.emit_expr(&new_expr.args[0]);
                            self.write(
                                ".items) |__entry| { __map.set(js_allocator.allocator(), __entry.at(@as(usize, 0)), __entry.at(@as(usize, 1))) catch @panic(\"OOM: Map init\"); } ");
                        }
                    }
                    self.write(&format!("break :{blk} __map; }})"));
                }
            }
            NewConstructor::Set => {
                if new_expr.args.is_empty() {
                    self.write("js_collections.JsSet.init(js_allocator.allocator())");
                } else {
                    // new Set([1,2,3,...]) → block with init + add calls
                    let blk = self.next_label();
                    self.write(&format!(
                        "({blk}: {{ var __set = js_collections.JsSet.init(js_allocator.allocator()); "
                    ));
                    // If the arg is an ArrayLiteral without spread, emit each
                    // element directly with JsAny.from() wrap to handle any
                    // element type (i64, f64, []const u8, etc.).
                    match &new_expr.args[0] {
                        crate::zigir::types::IrExpr::ArrayLiteral(arr)
                            if arr.spread_indices.is_empty() =>
                        {
                            for elem in &arr.elements {
                                self.write("__set.add(js_allocator.allocator(), JsAny.from(");
                                self.emit_expr(elem);
                                self.write(")) catch @panic(\"OOM: Set init\"); ");
                            }
                        }
                        _ => {
                            // Fallback: build the array and iterate. The array
                            // will be ArrayList(JsAny) when spread is present.
                            self.write("for (");
                            self.emit_expr(&new_expr.args[0]);
                            self.write(
                                ".items) |__val| { __set.add(js_allocator.allocator(), __val) catch @panic(\"OOM: Set init\"); } ");
                        }
                    }
                    self.write(&format!("break :{blk} __set; }})"));
                }
            }
            NewConstructor::Date(kind) => match kind {
                DateConstructorKind::Now => {
                    self.write("js_date.JsDate.init()");
                }
                DateConstructorKind::FromMillis => {
                    self.write("js_date.JsDate.fromMillis(");
                    self.emit_first_arg_or_default(&new_expr.args, "");
                    self.write(")");
                }
                DateConstructorKind::FromString => {
                    // new Date("2024-01-01") → js_date.JsDate.fromMillis(js_date.parse("2024-01-01"))
                    self.write("js_date.JsDate.fromMillis(js_date.parse(");
                    self.emit_first_arg_or_default(&new_expr.args, "");
                    self.write("))");
                }
                DateConstructorKind::FromComponents => {
                    // new Date(y, m, d, h, min, s, ms)
                    // Defaults: d=1, h=0, min=0, s=0, ms=0
                    self.write("js_date.JsDate.fromComponents(");
                    // y and m are always required; provide defaults for all 7 slots
                    let defaults = ["0", "0", "1", "0", "0", "0", "0"];
                    self.emit_args_with_defaults(&new_expr.args, 7, &defaults);
                    self.write(")");
                }
            },
            NewConstructor::RegExp => {
                // new RegExp(pat, flags?) → js_regexp.JsRegExp.init(alloc, pat, flags_or_empty) catch @panic(...)
                self.write("js_regexp.JsRegExp.init(js_allocator.allocator(), ");
                self.emit_first_arg_or_default(&new_expr.args, "\"\"");
                self.write(", ");
                if new_expr.args.len() >= 2 {
                    self.emit_expr(&new_expr.args[1]);
                } else {
                    self.write("\"\"");
                }
                self.write(") catch @panic(\"OOM: RegExp init\")");
            }
            NewConstructor::TypedArray(kind) => {
                let (module, init_fn) = super::typed_array_init(kind);
                let is_float = matches!(
                    kind,
                    TypedArrayKind::Float32Array | TypedArrayKind::Float64Array
                );
                let elem_type = if is_float { "f64" } else { "i64" };
                let zero_val = if is_float { "0.0" } else { "0" };

                self.write(&format!("{}.{}(", module, init_fn));
                match new_expr.args.first() {
                    // Array literal: emit elements directly as &[_]T{ ... }
                    Some(crate::zigir::types::IrExpr::ArrayLiteral(arr)) => {
                        if !arr.spread_indices.is_empty() {
                            self.write(&helpers::compile_error(
                                "Spread elements in TypedArray constructor are not supported",
                            ));
                        } else {
                            self.write(&format!("&[_]{}{{ ", elem_type));
                            for (i, elem) in arr.elements.iter().enumerate() {
                                if i > 0 {
                                    self.write(", ");
                                }
                                self.emit_expr(elem);
                            }
                            self.write(" }");
                        }
                    }
                    // Integer literal: a positive, comptime-known length in
                    // a reasonable range means we can emit `[_]T{ zero } ** n`
                    // to get a zero-filled typed array.
                    Some(crate::zigir::types::IrExpr::IntLiteral(n)) if *n > 0 && *n <= 1024 => {
                        // Comptime-known length: use array repeat syntax
                        self.write(&format!("&[_]{}{{ {} }} ** {}", elem_type, zero_val, n));
                    }
                    // Zero-length or unrealistic length: emit an empty array
                    // (the runtime call is responsible for sizing).
                    Some(crate::zigir::types::IrExpr::IntLiteral(_)) => {
                        self.write(&format!("&[_]{}{{}}", elem_type));
                    }
                    // No args: empty array
                    None => {
                        self.write(&format!("&[_]{}{{}}", elem_type));
                    }
                    // Other expressions: not supported in this position
                    Some(_) => {
                        self.write(&helpers::compile_error(
                            "new TypedArray(expr) only supports array literal or integer length",
                        ));
                    }
                }
                self.write(")");
            }
            NewConstructor::Class(class_name) => {
                // P2-11: class_name is the JS name — convert to zig_name
                // to match the struct definition (which uses IrIdent.zig_name).
                let zig_name = crate::zigir::ident::zig_safe_name(class_name);
                self.write(&format!("{}.init(", zig_name));
                for (i, arg) in new_expr.args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.emit_one_arg(arg);
                }
                self.write(")");
            }
            NewConstructor::Error(msg) => {
                // Emit a labeled block that stores the error name and message
                // in thread-local vars, then returns a placeholder JsAny.
                // The throw statement discards this value and breaks to the
                // try-catch label with `error.JsThrow`. The catch handler's
                // `fromError()` reads the stored name and message.
                let lbl = self.next_label();
                self.write(&format!("{}: {{", lbl));
                self.write(&format!(
                    "js_error.setLastThrow(\"{}\", ",
                    helpers::escape_zig_string(msg)
                ));
                // Emit the message argument (first arg to new Error(msg))
                if let Some(msg_arg) = new_expr.args.first() {
                    self.emit_expr(msg_arg);
                } else {
                    self.write("\"\"");
                }
                self.write("); ");
                self.write(&format!("break :{} JsAny.fromUndefined(); }}", lbl));
            }
            NewConstructor::Unsupported(name) => {
                self.write(&helpers::compile_error(&format!(
                    "new {}() is not supported",
                    name
                )));
            }
        }
    }
}
