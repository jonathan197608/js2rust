// zigir/emit/builtins/mod.rs
// Builtin method emission: dispatch and shared helpers.

pub mod array_callback;
pub mod array_method;
pub mod collections;
pub mod math;
pub mod object;
pub mod regexp;
pub mod string;

use crate::zigir::builtins::BuiltinModule;
use crate::zigir::emit::Emitter;
use crate::zigir::emit::helpers::EmitterHelpers;

// ═══════════════════════════════════════════════════════
//  Builtin call dispatch
// ═══════════════════════════════════════════════════════

impl Emitter {
    pub(crate) fn emit_builtin_call(&mut self, bc: &crate::zigir::types::IrBuiltinCall) {
        // When obj_name is None but obj_expr is set (method chaining), render the
        // receiver expression to a string and use it as the inline object reference.
        let obj_inline = if bc.obj_name.is_none() {
            bc.obj_expr
                .as_ref()
                .map(|obj_expr| Self::emit_expr_inline(obj_expr))
        } else {
            None
        };
        let obj = bc.obj_name.as_deref().or(obj_inline.as_deref());
        match bc.module {
            BuiltinModule::JsArray => self.emit_array_builtin(&bc.method, obj, &bc.args),
            BuiltinModule::JsString => {
                self.emit_string_builtin(&bc.method, obj, &bc.args, bc.regex_info.as_ref())
            }
            BuiltinModule::JsDate => self.emit_date_builtin(&bc.method, obj, &bc.args),
            BuiltinModule::JsJson => self.emit_json_builtin(&bc.method, &bc.args),
            BuiltinModule::JsObject => self.emit_object_builtin(&bc.method, &bc.args),
            BuiltinModule::JsNumber => self.emit_number_builtin(&bc.method, obj, &bc.args),
            BuiltinModule::JsSymbol => self.emit_symbol_builtin(&bc.method, obj, &bc.args),
            BuiltinModule::JsConsole => self.emit_console_builtin(&bc.method, &bc.args),
            BuiltinModule::JsMath => self.emit_math_builtin(&bc.method, &bc.args),
            BuiltinModule::JsRegExp => self.emit_regexp_builtin(
                &bc.method,
                bc.obj_name.as_deref(),
                &bc.args,
                bc.regex_info.as_ref(),
            ),
            BuiltinModule::JsTypedArray => self.emit_typedarray_builtin(
                &bc.method,
                bc.obj_name.as_deref(),
                &bc.args,
                bc.ta_type_suffix.as_deref(),
            ),
            BuiltinModule::JsUri => self.emit_uri_builtin(&bc.method, obj, &bc.args),
            BuiltinModule::JsBigInt => self.emit_bigint_builtin(&bc.method, obj, &bc.args),
            BuiltinModule::JsCollections => {
                self.emit_collections_builtin(&bc.method, bc.obj_name.as_deref(), &bc.args)
            }
            BuiltinModule::JsRuntime => {
                self.emit_runtime_builtin(&bc.method, bc.obj_name.as_deref(), &bc.args)
            }
            BuiltinModule::JsError => {
                // JsError is not directly callable as a builtin method;
                // it's constructed in catch dispatch (emit_try_stmt).
                self.write("js_error.JsError");
            }
        }
    }
}

// ═══════════════════════════════════════════════════════
//  Shared helpers
// ═══════════════════════════════════════════════════════

impl Emitter {
    fn emit_inline_args(&mut self, args: &[crate::zigir::types::IrExpr]) {
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            self.emit_expr(arg);
        }
    }

    fn emit_array_builtin(
        &mut self,
        method: &str,
        obj: Option<&str>,
        args: &[crate::zigir::types::IrExpr],
    ) {
        // Some array methods are direct ArrayList operations when we have the object name.
        match method {
            "pop" => {
                // arr.pop() — direct ArrayList method, not js_array.pop()
                if let Some(name) = obj {
                    self.write(&format!("{}.pop()", name));
                } else {
                    self.write(&format!("js_array.{}(", method));
                    self.emit_inline_args(args);
                    self.write(")");
                }
            }
            "shift" => {
                // arr.shift() — ArrayList orderedRemove(0)
                if let Some(name) = obj {
                    self.write(&format!("{}.orderedRemove(0)", name));
                } else {
                    self.write(&format!("js_array.{}(", method));
                    self.emit_inline_args(args);
                    self.write(")");
                }
            }
            "reverse" => {
                // arr.reverse() — in-place std.mem.reverse, returns the array
                if let Some(name) = obj {
                    let blk = self.next_label();
                    self.write(&format!(
                        "({}: {{ std.mem.reverse(@TypeOf({}.items[0]), {}.items); break :{} {}; }})",
                        blk, name, name, blk, name
                    ));
                } else {
                    self.write(&format!("js_array.{}(", method));
                    self.emit_inline_args(args);
                    self.write(")");
                }
            }
            "sort" => {
                // arr.sort() — in-place std.mem.sort, returns the array
                if let Some(name) = obj {
                    let blk = self.next_label();
                    self.write(&format!(
                        "({}: {{ std.mem.sort(@TypeOf({}.items[0]), {}.items, {{}}, comptime std.sort.asc(@TypeOf({}.items[0]))); break :{} {}; }})",
                        blk, name, name, name, blk, name
                    ));
                } else {
                    self.write(&format!("js_array.{}(", method));
                    self.emit_inline_args(args);
                    self.write(")");
                }
            }
            _ => {
                // Fallback: js_array.method(args)
                self.write(&format!("js_array.{}(", method));
                self.emit_inline_args(args);
                self.write(")");
            }
        }
    }
}
