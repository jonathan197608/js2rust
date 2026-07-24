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
        let obj_inline: Option<String> = if bc.obj_name.is_none() {
            if let Some(obj_expr) = &bc.obj_expr {
                let (rendered, new_counter) =
                    Self::emit_expr_inline_with_label_offset(obj_expr, self.label_counter);
                self.label_counter = new_counter;
                Some(rendered)
            } else {
                None
            }
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
            BuiltinModule::JsRuntime => self.emit_runtime_builtin(&bc.method, obj, &bc.args),
            BuiltinModule::JsError => {
                // JsError is not directly callable as a builtin method;
                // it's constructed in catch dispatch (emit_try_stmt).
                self.write("js_error.JsError");
            }
        }
    }
}

// ═══════════════════════════════════════════════════════
//  Chain-receiver resolution
// ═══════════════════════════════════════════════════════

/// Resolve the receiver for an inline array callback/method, fixing two
/// method-chaining bugs:
///
/// **Bug A — Label conflict**: `emit_expr_inline()` starts each sub-emitter
/// with `label_counter = 0`, so nested chains like `filter().map()` both
/// emit `blk_0`. We pass `label_offset` so inner blocks use higher labels.
///
/// **Bug B — Double evaluation**: Methods like `map` call the receiver
/// expression twice (e.g. for `ensureTotalCapacity` and `for`), re-running
/// the entire inner chain. We emit a `const __chain_N = <expr>;` binding
/// once and return the variable name for all references.
///
/// Returns `(receiver_string, optional_binding, updated_label_counter)`.
/// The caller must emit the binding (if any) inside the enclosing block
/// before the first use of `receiver_string`.
pub(super) fn resolve_chain_receiver(
    obj_expr: &Option<Box<crate::zigir::types::IrExpr>>,
    obj_name: &str,
    label_offset: u32,
) -> (String, Option<String>, u32) {
    if let Some(expr) = obj_expr {
        use crate::zigir::types::IrExpr;
        let (rendered, new_counter) =
            Emitter::emit_expr_inline_with_label_offset(expr, label_offset);
        match expr.as_ref() {
            IrExpr::ArrayCallbackInline(_) | IrExpr::ArrayMethodInline(_) => {
                let chain_var = format!("__chain_{}", label_offset);
                let binding = format!("const {} = {}; ", chain_var, rendered);
                (chain_var, Some(binding), new_counter)
            }
            _ => (format!("({})", rendered), None, new_counter),
        }
    } else {
        (obj_name.to_string(), None, label_offset)
    }
}

// ═══════════════════════════════════════════════════════
//  Shared helpers
// ═══════════════════════════════════════════════════════

impl Emitter {
    pub(super) fn emit_inline_args(&mut self, args: &[crate::zigir::types::IrExpr]) {
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            self.emit_expr(arg);
        }
    }

    /// Emit the first argument if present, otherwise emit `default`.
    pub(super) fn emit_first_arg_or_default(
        &mut self,
        args: &[crate::zigir::types::IrExpr],
        default: &str,
    ) {
        if let Some(arg) = args.first() {
            self.emit_expr(arg);
        } else {
            self.write(default);
        }
    }

    /// Emit `total_slots` comma-separated values, filling missing args with defaults.
    /// `defaults[i]` is the default for slot `i` when `i >= args.len()`.
    pub(super) fn emit_args_with_defaults(
        &mut self,
        args: &[crate::zigir::types::IrExpr],
        total_slots: usize,
        defaults: &[&str],
    ) {
        self.emit_args_with_defaults_coerced(args, total_slots, defaults, false);
    }

    /// Same as `emit_args_with_defaults` but coerces each provided argument
    /// to i64 via `emit_i64_coerced`. Used by Date setter methods whose Zig
    /// runtime signatures expect `i64` parameters.
    pub(super) fn emit_args_with_defaults_i64(
        &mut self,
        args: &[crate::zigir::types::IrExpr],
        total_slots: usize,
        defaults: &[&str],
    ) {
        self.emit_args_with_defaults_coerced(args, total_slots, defaults, true);
    }

    fn emit_args_with_defaults_coerced(
        &mut self,
        args: &[crate::zigir::types::IrExpr],
        total_slots: usize,
        defaults: &[&str],
        coerce_i64: bool,
    ) {
        let n_args = args.len();
        #[allow(clippy::needless_range_loop)]
        for i in 0..total_slots {
            if i > 0 {
                self.write(", ");
            }
            if i < n_args {
                if coerce_i64 {
                    self.emit_i64_coerced(&args[i]);
                } else {
                    self.emit_expr(&args[i]);
                }
            } else if let Some(default) = defaults.get(i) {
                self.write(default);
            }
        }
    }

    /// Emit `module_prefix.method(args)` — the common fallback pattern.
    pub(super) fn emit_module_call(
        &mut self,
        module_prefix: &str,
        method: &str,
        args: &[crate::zigir::types::IrExpr],
    ) {
        self.write(&format!("{}.{}(", module_prefix, method));
        self.emit_inline_args(args);
        self.write(")");
    }

    /// Emit `obj.method(args)` if `obj` is Some, else `module_prefix.method(args)`.
    pub(super) fn emit_receiver_or_module_call(
        &mut self,
        obj: Option<&str>,
        module_prefix: &str,
        method: &str,
        args: &[crate::zigir::types::IrExpr],
    ) {
        if let Some(name) = obj {
            self.write(&format!("{}.{}(", name, method));
            self.emit_inline_args(args);
            self.write(")");
        } else {
            self.emit_module_call(module_prefix, method, args);
        }
    }

    /// Resolve chain receiver and update label_counter.
    /// Returns (receiver_string, optional_binding).
    pub(super) fn resolve_receiver(
        &mut self,
        obj_expr: &Option<Box<crate::zigir::types::IrExpr>>,
        obj_name: &str,
    ) -> (String, Option<String>) {
        let (receiver, binding, new_lc) =
            resolve_chain_receiver(obj_expr, obj_name, self.label_counter);
        self.label_counter = new_lc;
        (receiver, binding)
    }

    /// Emit the opening of a labeled block: `(blk_N: { [binding]`.
    /// Returns the label name for the caller to use in break statements.
    pub(super) fn begin_labeled_block(&mut self, binding: &Option<String>) -> String {
        let blk = self.next_label();
        self.write(&format!("({}: {{ ", blk));
        if let Some(b) = binding {
            self.write(b);
        }
        blk
    }

    /// Render an expression to a string by temporarily swapping the output buffer.
    /// Delegates to `expr_to_string` in expr/mod.rs (the canonical implementation).
    pub(super) fn render_expr_to_string(&mut self, expr: &crate::zigir::types::IrExpr) -> String {
        self.expr_to_string(expr)
    }

    /// Emit `if (<expr>) break :<blk> <value>` or `if (!(<expr>)) break :<blk> <value>`.
    /// Shared by short-circuit (some/every), find-like, and find-index-like inlining.
    pub(super) fn emit_if_break_pred(
        &mut self,
        expr: &crate::zigir::types::IrExpr,
        blk: &str,
        value: &str,
        negate: bool,
    ) {
        if negate {
            self.write("if (!(js_runtime.isTruthy(");
            self.emit_expr(expr);
            self.write(&format!("))) break :{} {};", blk, value));
        } else {
            self.write("if (js_runtime.isTruthy(");
            self.emit_expr(expr);
            self.write(&format!(")) break :{} {};", blk, value));
        }
    }

    /// Emit callback body statements, fusing `IrStmt::Return { value }` and
    /// `IrStmt::Expr` into a single predicate via `emit_pred`. Other statements
    /// are emitted normally.
    pub(super) fn emit_callback_body<F>(
        &mut self,
        stmts: &[crate::zigir::types::IrStmt],
        mut emit_pred: F,
    ) where
        F: FnMut(&mut Self, &crate::zigir::types::IrExpr),
    {
        let last_idx = stmts.len().saturating_sub(1);
        for (i, stmt) in stmts.iter().enumerate() {
            self.writeln("");
            match stmt {
                crate::zigir::types::IrStmt::Return { value: Some(expr) } => {
                    emit_pred(self, expr);
                }
                crate::zigir::types::IrStmt::Expr(expr) if i == last_idx => {
                    emit_pred(self, expr);
                }
                _ => self.emit_stmt(stmt),
            }
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
                if let Some(name) = obj {
                    self.write(&format!("{}.pop()", name));
                } else {
                    self.emit_module_call("js_array", method, args);
                }
            }
            "shift" => {
                if let Some(name) = obj {
                    self.write(&format!("{}.orderedRemove(0)", name));
                } else {
                    self.emit_module_call("js_array", method, args);
                }
            }
            "reverse" => {
                if let Some(name) = obj {
                    let blk = self.next_label();
                    self.write(&format!(
                        "({}: {{ std.mem.reverse(@TypeOf({}.items[0]), {}.items); break :{} {}; }})",
                        blk, name, name, blk, name
                    ));
                } else {
                    self.emit_module_call("js_array", method, args);
                }
            }
            "sort" => {
                // ECMA-262: Default sort (no compareFn) converts elements to
                // strings and compares by UTF-16 code unit sequence. JsAny
                // already does this via .lt(). For i64/f64, format as strings;
                // other types fall back to numeric <.
                if let Some(name) = obj {
                    let blk = self.next_label();
                    self.write(&format!(
                        "({}: {{ const T = @TypeOf({}.items[0]); ",
                        blk, name
                    ));
                    self.write(&format!(
                        "std.mem.sort(T, {}.items, {{}}, struct {{ fn lessThan(_: void, a: T, b: T) bool {{",
                        name
                    ));
                    self.write(" if (T == JsAny) return a.lt(b);");
                    self.write(" if (T == i64 or T == f64) {");
                    self.write(" var __sa: [64]u8 = undefined; var __sb: [64]u8 = undefined;");
                    self.write(" const __stra = std.fmt.bufPrint(&__sa, \"{d}\", .{a}) catch return a < b;");
                    self.write(" const __strb = std.fmt.bufPrint(&__sb, \"{d}\", .{b}) catch return a < b;");
                    self.write(" return std.mem.order(u8, __stra, __strb) == .lt;");
                    self.write(" } return a < b; } }.lessThan); ");
                    self.write(&format!("break :{} {}; }})", blk, name));
                } else {
                    self.emit_module_call("js_array", method, args);
                }
            }
            _ => {
                self.emit_module_call("js_array", method, args);
            }
        }
    }
}
