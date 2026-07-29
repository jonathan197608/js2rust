// zigir/emit/expr/call_member.rs
// Call expression, field access, index access, and computed field emission.

use crate::zigir::emit::Emitter;
use crate::zigir::emit::helpers::{EmitterHelpers, zig_ident};
use crate::zigir::kinds::{CallKind, ComputedKeyKind, FieldKind, IndexKind};

impl Emitter {
    pub(super) fn emit_call_expr(&mut self, call: &crate::zigir::types::IrCallExpr) {
        match &call.call_kind {
            CallKind::Direct | CallKind::Method { .. } => {
                self.emit_expr(&call.callee);
                self.emit_args(&call.args);
            }
            CallKind::Closure => {
                // Struct literal + method call needs parentheses in Zig:
                // (StructName{ .field = val }).call(args)
                if matches!(*call.callee, crate::zigir::types::IrExpr::Closure(_)) {
                    self.write("(");
                    self.emit_expr(&call.callee);
                    self.write(")");
                } else {
                    self.emit_expr(&call.callee);
                }
                self.write(".call(");
                // Check if any arg is a Spread — if so, generate a temporary
                // ArrayList(JsAny) builder block to flatten the mixed args
                // into a single []const JsAny slice.
                let has_spread = call
                    .args
                    .iter()
                    .any(|a| matches!(a, crate::zigir::types::IrExpr::Spread(_)));
                if has_spread {
                    self.emit_closure_spread_args(&call.args);
                } else {
                    self.emit_inline_args(&call.args);
                }
                self.write(")");
            }
        }
    }

    /// Emit spread args for a Closure call by building a temporary
    /// ArrayList(JsAny) in a labeled block, then returning `.items`.
    ///
    /// Pattern: (blk: { var __arr_N: std.ArrayList(JsAny) = .empty;
    ///   append non-spread args; for-loop spread sources;
    ///   break :blk __arr_N.items; })
    fn emit_closure_spread_args(&mut self, args: &[crate::zigir::types::IrExpr]) {
        use crate::zigir::types::IrExpr;
        let blk = self.next_label();
        let arr = self.next_array_var();
        self.write(&format!(
            "({}: {{ var {}: std.ArrayList(JsAny) = .empty; ",
            blk, arr
        ));
        for arg in args.iter() {
            match arg {
                IrExpr::Spread(inner) => {
                    // Spread: iterate the source and append each element
                    // wrapped via JsAny.from(...)
                    let is_rest = match inner.as_ref() {
                        IrExpr::Ident(ident) if self.rest_param_names.contains(&ident.zig_name) => {
                            true
                        }
                        IrExpr::TypedIdent { ident, .. }
                            if self.rest_param_names.contains(&ident.zig_name) =>
                        {
                            true
                        }
                        _ => false,
                    };
                    // Check if spread source is Map.entries() — each element
                    // is ArrayList(JsAny), needs fromArrayList not from.
                    let is_entries = matches!(
                        inner.as_ref(),
                        IrExpr::BuiltinCall(bc)
                            if bc.method == "entries"
                                && matches!(bc.module, crate::zigir::builtins::BuiltinModule::JsCollections)
                    );
                    self.write("for (");
                    if is_rest {
                        self.emit_expr(inner);
                    } else {
                        let needs_parens =
                            !matches!(inner.as_ref(), IrExpr::Ident(_) | IrExpr::TypedIdent { .. });
                        if needs_parens {
                            self.write("(");
                        }
                        self.emit_expr(inner);
                        if needs_parens {
                            self.write(")");
                        }
                        self.write(".items");
                    }
                    if is_entries {
                        self.write(&format!(") |__spread_item| {}.append(js_allocator.allocator(), JsAny.fromArrayList(js_allocator.allocator(), __spread_item) catch @panic(\"OOM: fromArrayList\")) catch @panic(\"OOM: Array.spread\"); ", arr));
                    } else {
                        self.write(&format!(") |__spread_item| {}.append(js_allocator.allocator(), JsAny.from(__spread_item)) catch @panic(\"OOM: Array.spread\"); ", arr));
                    }
                }
                _ => {
                    // Non-spread arg: append with JsAny.from() wrap
                    self.write(&format!(
                        "{}.append(js_allocator.allocator(), JsAny.from(",
                        arr
                    ));
                    self.emit_expr(arg);
                    self.write(")) catch @panic(\"OOM: Array.push append\"); ");
                }
            }
        }
        self.write(&format!("break :{} {}.items; }})", blk, arr));
    }

    pub(super) fn emit_field_access(
        &mut self,
        object: &crate::zigir::types::IrExpr,
        field: &str,
        kind: &FieldKind,
    ) {
        match kind {
            // Direct field access: obj.field (same for StructField, Namespace, Private)
            FieldKind::StructField | FieldKind::Namespace | FieldKind::Private => {
                self.emit_dot_access(object, field);
            }
            FieldKind::ArrayListLen => {
                // Wrap in parens for complex expressions that may contain `catch`.
                // Simple identifiers don't need parens: arr.items.len is correct.
                if matches!(
                    *object,
                    crate::zigir::types::IrExpr::Ident(_)
                        | crate::zigir::types::IrExpr::TypedIdent { .. }
                ) {
                    self.emit_expr(object);
                    self.write(".items.len");
                } else {
                    self.write("(");
                    self.emit_expr(object);
                    self.write(").items.len");
                }
            }
            FieldKind::StringLen => {
                // JS string.length returns UTF-16 code unit count, not byte count
                self.write("js_string.utf16Len(");
                self.emit_expr(object);
                self.write(")");
            }
            FieldKind::SliceLen => {
                // Slice/TypedArray length: element count.
                // Wrap in parens for complex expressions that may contain `catch`
                // (e.g. split() catch @panic(...)) — without parens, .len would
                // bind to the catch fallback instead of the result.
                // Simple identifiers don't need parens: arr.len is correct.
                if matches!(
                    *object,
                    crate::zigir::types::IrExpr::Ident(_)
                        | crate::zigir::types::IrExpr::TypedIdent { .. }
                ) {
                    self.emit_expr(object);
                    self.write(".len");
                } else {
                    self.write("(");
                    self.emit_expr(object);
                    self.write(").len");
                }
            }
            FieldKind::JsAnyLen => {
                // JsAny is a union(enum); .length needs runtime dispatch.
                // Arrays: a.items.len, strings: utf16Len, objects: count(), else: 0.
                self.write("(switch (");
                self.emit_expr(object);
                self.write(") { .array => |a| @as(i64, @intCast(a.items.len)), .value => |v| switch (v) { .string => |s| @as(i64, @intCast(js_string.utf16Len(s))), else => @as(i64, 0) }, .object => |o| @as(i64, @intCast(o.count())), .null => @as(i64, 0) })");
            }
            FieldKind::ArgumentsLen => {
                // arguments.length: JS .length is i64, but []const JsAny .len is usize.
                // Cast to i64 for correct JS semantics.
                self.write("@as(i64, @intCast(");
                self.emit_expr(object);
                self.write(".len))");
            }
            FieldKind::MapSetSize => {
                self.emit_expr(object);
                self.write(".size()");
            }
            FieldKind::MathConstant(val) => {
                // Zig 0.16.0: std.math constants are comptime_float with higher
                // precision than f64. Wrap in @as(f64, ...) so string formatting
                // and template literals produce JS-compatible output (e.g. PI =>
                // "3.141592653589793", not the full-precision comptime value).
                match val.as_str() {
                    "PI" => self.write("@as(f64, std.math.pi)"),
                    "E" => self.write("@as(f64, std.math.e)"),
                    "LN2" => self.write("@as(f64, std.math.ln2)"),
                    "LN10" => self.write("@as(f64, std.math.ln10)"),
                    "LOG2E" => self.write("@as(f64, std.math.log2e)"),
                    "LOG10E" => self.write("@as(f64, std.math.log10e)"),
                    "SQRT1_2" => self.write("@as(f64, std.math.sqrt1_2)"),
                    "SQRT2" => self.write("@as(f64, std.math.sqrt2)"),
                    _ => self.write(&format!("@as(f64, std.math.{})", val.to_lowercase())),
                }
            }
            FieldKind::NumberConstant(val) => {
                // Map JS Number constants to Zig std.math equivalents
                match val.as_str() {
                    "MAX_VALUE" => self.write("std.math.floatMax(f64)"),
                    "MIN_VALUE" => self.write("std.math.floatMin(f64)"),
                    "NaN" => self.write("std.math.nan(f64)"),
                    "NEGATIVE_INFINITY" => self.write("-std.math.inf(f64)"),
                    "POSITIVE_INFINITY" => self.write("std.math.inf(f64)"),
                    "EPSILON" => self.write("std.math.floatEps(f64)"),
                    "MAX_SAFE_INTEGER" => self.write("9007199254740991"),
                    "MIN_SAFE_INTEGER" => self.write("-9007199254740991"),
                    _ => self.write(&format!("std.math.{}", val)),
                }
            }
            FieldKind::SymbolWellKnown(val) => {
                // Symbol well-known properties: Symbol.iterator → js_symbol.symbolIterator()
                // All well-known symbol accessors are prefixed with "symbol" in the runtime
                let zig_name = match val.as_str() {
                    "iterator" => "symbolIterator".to_string(),
                    "asyncIterator" => "symbolAsyncIterator".to_string(),
                    "hasInstance" => "symbolHasInstance".to_string(),
                    "isConcatSpreadable" => "symbolIsConcatSpreadable".to_string(),
                    "species" => "symbolSpecies".to_string(),
                    "toPrimitive" => "symbolToPrimitive".to_string(),
                    "toStringTag" => "symbolToStringTag".to_string(),
                    "unscopables" => "symbolUnscopables".to_string(),
                    "match" => "symbolMatch".to_string(),
                    "matchAll" => "symbolMatchAll".to_string(),
                    "replace" => "symbolReplace".to_string(),
                    "search" => "symbolSearch".to_string(),
                    "split" => "symbolSplit".to_string(),
                    "dispose" => "symbolDispose".to_string(),
                    other => {
                        // Fallback: capitalize first letter and prepend "symbol"
                        let mut chars = other.chars();
                        match chars.next() {
                            None => "symbol".to_string(),
                            Some(c) => format!("symbol{}{}", c.to_uppercase(), chars.as_str()),
                        }
                    }
                };
                self.write(&format!("js_symbol.{}()", zig_name));
            }
            FieldKind::TypedArrayProp { prop, type_suffix } => {
                if let Some(suffix) = type_suffix {
                    self.write(&format!("js_runtime.js_typedarray.{}{}(", prop, suffix));
                    self.emit_expr(object);
                    self.write(")");
                } else {
                    self.emit_dot_access(object, prop);
                }
            }
            FieldKind::PointerDeref => {
                self.emit_expr(object);
                self.write(".*");
            }
            FieldKind::RegExpProp { prop } => {
                // regex.source → regex.pattern; others map directly (regex.flags → .flags, etc.)
                if prop == "source" {
                    self.emit_dot_access(object, "pattern");
                } else {
                    self.emit_dot_access(object, prop);
                }
            }
            FieldKind::StaticField { class_name } => {
                // ClassName.field → __ClassName_field module-scope var
                self.emit_static_field(class_name, field);
            }
        }
    }

    pub(super) fn emit_index_access(
        &mut self,
        object: &crate::zigir::types::IrExpr,
        index: &crate::zigir::types::IrExpr,
        kind: &IndexKind,
    ) {
        match kind {
            IndexKind::ArrayListItem => {
                self.emit_arraylist_item(object, index);
            }
            IndexKind::SliceIndex => {
                self.emit_slice_index(object, index);
            }
            IndexKind::JsAnyIndex => {
                self.emit_jsany_index(object, index);
            }
            IndexKind::MapPut => {
                // MapPut is only for assignment, not reads
                unreachable!("MapPut in read context");
            }
        }
    }

    pub(super) fn emit_computed_field(
        &mut self,
        object: &crate::zigir::types::IrExpr,
        key: &crate::zigir::types::IrExpr,
        kind: &ComputedKeyKind,
    ) {
        use crate::zigir::emit::helpers;
        match kind {
            ComputedKeyKind::StructField => {
                self.write("@field(");
                self.emit_expr(object);
                self.write(", ");
                self.emit_expr(key);
                self.write(")");
            }
            ComputedKeyKind::MapGet => {
                self.emit_expr(object);
                self.write(".get(");
                self.emit_expr(key);
                self.write(")");
            }
            ComputedKeyKind::JsAnyGetByKey => {
                self.emit_expr(object);
                self.write(".getByKey(");
                self.emit_expr(key);
                self.write(", js_allocator.allocator())");
            }
            ComputedKeyKind::ArrayListItem => {
                self.emit_arraylist_item(object, key);
            }
            ComputedKeyKind::StringCharAt => {
                // str[idx] in JS returns a single-character substring (charAt
                // semantics). Use js_string.charAt which returns `[]const u8`.
                // Match the fallible string-method pattern used elsewhere:
                // `catch @panic("OOM: string method")` (avoids forcing an
                // error-union return type on the enclosing function).
                self.write("js_string.charAt(js_allocator.allocator(), ");
                self.emit_expr(object);
                self.write(", ");
                self.emit_expr(key);
                self.write(") catch @panic(\"OOM: string method\")");
            }
            ComputedKeyKind::CompileError(msg) => {
                self.write(&helpers::compile_error(msg));
            }
            ComputedKeyKind::ObjectMapGet => {
                // Parenthesize so method calls on the result bind correctly,
                // e.g. (obj.get(k) orelse JsAny.fromUndefined()).asF64()
                self.write("(");
                self.emit_expr(object);
                self.write(".get(");
                self.emit_expr(key);
                self.write(") orelse JsAny.fromUndefined())");
            }
        }
    }

    // ── Shared index/field helpers ────────────────────────
    // Used by emit_index_access, emit_computed_field, and emit_assign_target_inner.

    /// Emit `object.field` — dot-access on an expression.
    pub(super) fn emit_dot_access(&mut self, object: &crate::zigir::types::IrExpr, field: &str) {
        self.emit_expr(object);
        self.write(&format!(".{}", zig_ident(field)));
    }

    /// Emit `object.items[...]` — ArrayList element access.
    /// Uses a labeled block to guard against negative runtime indices:
    /// `@intCast` on a negative i64 panics with an opaque message, whereas
    /// the labeled block produces a clear "negative index" panic.
    pub(super) fn emit_arraylist_item(
        &mut self,
        object: &crate::zigir::types::IrExpr,
        index: &crate::zigir::types::IrExpr,
    ) {
        let lbl = self.next_label();
        self.emit_expr(object);
        self.write(&format!(".items[{}: {{ const __idx = ", lbl));
        self.emit_expr(index);
        self.write(&format!(
            "; break :{} if (__idx < 0) @panic(\"array index out of bounds: negative index\") else @as(usize, @intCast(__idx)); }}]",
            lbl
        ));
    }

    /// Emit `object[...]` — Slice/array index access.
    /// Same negative-index guard as `emit_arraylist_item`.
    pub(super) fn emit_slice_index(
        &mut self,
        object: &crate::zigir::types::IrExpr,
        index: &crate::zigir::types::IrExpr,
    ) {
        let lbl = self.next_label();
        self.emit_expr(object);
        self.write(&format!("[{}: {{ const __idx = ", lbl));
        self.emit_expr(index);
        self.write(&format!(
            "; break :{} if (__idx < 0) @panic(\"array index out of bounds: negative index\") else @as(usize, @intCast(__idx)); }}]",
            lbl
        ));
    }

    /// Emit `obj.at(@as(usize, @intCast(idx)))` — JsAny array element access.
    /// Uses the same negative-index guard pattern as other index methods.
    pub(super) fn emit_jsany_index(
        &mut self,
        object: &crate::zigir::types::IrExpr,
        index: &crate::zigir::types::IrExpr,
    ) {
        let lbl = self.next_label();
        self.emit_expr(object);
        self.write(&format!(".at({}: {{ const __idx = ", lbl));
        self.emit_expr(index);
        self.write(&format!(
            "; break :{} if (__idx < 0) @panic(\"array index out of bounds: negative index\") else @as(usize, @intCast(__idx)); }})",
            lbl
        ));
    }

    /// Emit `__ClassName_field` — static field access on a class.
    pub(super) fn emit_static_field(&mut self, class_name: &str, field: &str) {
        self.write(&format!("__{}_{}", class_name, field));
    }
}
