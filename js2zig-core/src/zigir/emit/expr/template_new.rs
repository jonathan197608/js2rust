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
                self.write("js_collections.JsMap.init(js_allocator.allocator())");
            }
            NewConstructor::Set => {
                self.write("js_collections.JsSet.init(js_allocator.allocator())");
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
                // new RegExp(pat, flags?) → try js_regexp.JsRegExp.init(alloc, pat, flags_or_empty)
                self.write("try js_regexp.JsRegExp.init(js_allocator.allocator(), ");
                self.emit_first_arg_or_default(&new_expr.args, "\"\"");
                self.write(", ");
                if new_expr.args.len() >= 2 {
                    self.emit_expr(&new_expr.args[1]);
                } else {
                    self.write("\"\"");
                }
                self.write(")");
            }
            NewConstructor::TypedArray(kind) => {
                let (module, init_fn) = super::typed_array_init(kind);
                let is_float = matches!(
                    kind,
                    TypedArrayKind::Float32Array | TypedArrayKind::Float64Array
                );
                let elem_type = if is_float { "f64" } else { "i64" };
                self.write(&format!("{}.{}(&[_]{}{{", module, init_fn, elem_type));
                // Emit array elements
                self.emit_first_arg_or_default(&new_expr.args, "");
                self.write("})");
            }
            NewConstructor::Class(class_name) => {
                self.write(&format!("{}.init(", class_name));
                self.emit_inline_args(&new_expr.args);
                self.write(")");
            }
            NewConstructor::Error(msg) => {
                self.write(&format!(
                    "JsAny.fromError(\"{}\")",
                    helpers::escape_zig_string(msg)
                ));
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
