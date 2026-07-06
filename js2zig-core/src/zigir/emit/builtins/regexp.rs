// zigir/emit/builtins/regexp.rs
// RegExp builtin method emission.

use crate::zigir::emit::helpers::EmitterHelpers;

use crate::zigir::emit::Emitter;

impl Emitter {
    pub(super) fn emit_regexp_builtin(
        &mut self,
        method: &str,
        obj: Option<&str>,
        args: &[crate::zigir::types::IrExpr],
        regex_info: Option<&crate::zigir::types::IrRegexInfo>,
    ) {
        match method {
            // regexp.test(str) — instance method on a variable
            "test" => {
                if let Some(info) = regex_info {
                    if !info.is_var_ref {
                        // Literal regex: /pattern/.test(str) → host.regex_test("pattern", str)
                        if let Some(pattern) = &info.pattern {
                            self.write(&format!("host.regex_test(\"{}\", ", pattern));
                            self.emit_inline_args(args);
                            self.write(")");
                            return;
                        }
                    } else {
                        // Variable regex: r.test(str) → r.isMatch(str)
                        if let Some(name) = &info.var_name {
                            self.write(&format!("{}.isMatch(", name));
                            self.emit_inline_args(args);
                            self.write(")");
                            return;
                        }
                    }
                }
                // Fallback: use obj_name as variable (shouldn't normally reach here)
                if let Some(name) = obj {
                    self.write(&format!("{}.isMatch(", name));
                    self.emit_inline_args(args);
                    self.write(")");
                } else {
                    self.write(&format!("js_regexp.{}(", method));
                    self.emit_inline_args(args);
                    self.write(")");
                }
            }
            // regexp.exec(str) — instance method on a variable
            "exec" => {
                if let Some(info) = regex_info {
                    if !info.is_var_ref {
                        // Literal regex: /pattern/.exec(str) → js_regexp.execLiteral(alloc, str, "pattern") catch @panic(...)
                        if let Some(pattern) = &info.pattern {
                            self.write("js_regexp.execLiteral(js_allocator.allocator(), ");
                            self.emit_inline_args(args);
                            self.write(&format!(
                                ", \"{}\") catch @panic(\"OOM: allocation\")",
                                pattern
                            ));
                            return;
                        }
                    } else {
                        // Variable regex: r.exec(str) → r.exec(allocator, str)
                        if let Some(name) = &info.var_name {
                            self.write(&format!("{}.exec(js_allocator.allocator(), ", name));
                            self.emit_inline_args(args);
                            self.write(")");
                            return;
                        }
                    }
                }
                // Fallback
                if let Some(name) = obj {
                    self.write(&format!("{}.exec(js_allocator.allocator(), ", name));
                    self.emit_inline_args(args);
                    self.write(")");
                } else {
                    self.write(&format!("js_regexp.{}(", method));
                    self.emit_inline_args(args);
                    self.write(")");
                }
            }
            _ => {
                self.write(&format!("js_regexp.{}(", method));
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
