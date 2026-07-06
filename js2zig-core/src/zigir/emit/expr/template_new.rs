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
        use crate::zigir::emit::helpers::escape_zig_string;
        // Zig template literal → std.fmt.allocPrint
        if exprs.is_empty() {
            // No expressions: just a string literal
            self.write(&format!("\"{}\"", escape_zig_string(&parts[0])));
        } else {
            // Build the format string: parts[0] + {spec0} + parts[1] + {spec1} + ...
            let mut fmt = String::new();
            for (i, part) in parts.iter().enumerate() {
                if i > 0 && i - 1 < format_specs.len() {
                    fmt.push_str(&format_specs[i - 1]);
                }
                fmt.push_str(&escape_zig_string(part));
            }
            // Emit args as a separate pass to get their string representations
            let arg_strs: Vec<String> = exprs
                .iter()
                .map(|arg| {
                    let saved = std::mem::take(self.output_mut());
                    self.emit_expr(arg);
                    let rendered = std::mem::take(self.output_mut());
                    *self.output_mut() = saved;
                    rendered
                })
                .collect();
            let args_str = format!(".{{{}}}", arg_strs.join(", "));
            self.write(&format!(
                "std.fmt.allocPrint(js_allocator.allocator(), \"{}\", {}) catch @panic(\"OOM: template literal allocPrint\")",
                fmt, args_str
            ));
        }
    }

    pub(super) fn emit_new_expr(&mut self, new_expr: &crate::zigir::types::IrNewExpr) {
        use crate::zigir::emit::helpers::escape_zig_string;
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
                    if let Some(arg) = new_expr.args.first() {
                        self.emit_expr(arg);
                    }
                    self.write(")");
                }
                DateConstructorKind::FromString => {
                    // new Date("2024-01-01") → js_date.JsDate.fromMillis(js_date.parse("2024-01-01"))
                    self.write("js_date.JsDate.fromMillis(js_date.parse(");
                    if let Some(arg) = new_expr.args.first() {
                        self.emit_expr(arg);
                    }
                    self.write("))");
                }
                DateConstructorKind::FromComponents => {
                    // new Date(y, m, d, h, min, s, ms)
                    // Defaults for missing args: d=1, h=0, min=0, s=0, ms=0
                    self.write("js_date.JsDate.fromComponents(");
                    let n_args = new_expr.args.len();
                    let defaults = ["1", "0", "0", "0", "0"]; // d, h, min, s, ms
                    // First two args (y, m) are always required
                    for (i, arg) in new_expr.args.iter().enumerate() {
                        if i > 0 {
                            self.write(", ");
                        }
                        self.emit_expr(arg);
                    }
                    // Fill remaining with defaults
                    for i in n_args..7 {
                        self.write(&format!(", {}", defaults[i - 2]));
                    }
                    self.write(")");
                }
            },
            NewConstructor::RegExp => {
                // new RegExp(pat) → try js_regexp.JsRegExp.init(js_allocator.allocator(), pat)
                self.write("try js_regexp.JsRegExp.init(js_allocator.allocator(), ");
                for (i, arg) in new_expr.args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.emit_expr(arg);
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
                if let Some(arg) = new_expr.args.first() {
                    // The arg is typically an IrExpr::ArrayLiteral or similar
                    self.emit_expr(arg);
                }
                self.write("})");
            }
            NewConstructor::Class(class_name) => {
                self.write(&format!("{}.init(", class_name));
                for (i, arg) in new_expr.args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.emit_expr(arg);
                }
                self.write(")");
            }
            NewConstructor::Error(msg) => {
                self.write(&format!("JsAny.fromError(\"{}\")", escape_zig_string(msg)));
            }
            NewConstructor::Unsupported(name) => {
                self.write(&format!(
                    "@compileError(\"new {}() is not supported\")",
                    name
                ));
            }
        }
    }
}
