// zigir/emit/expr/call_member.rs
// Call expression, field access, index access, and computed field emission.

use crate::zigir::emit::Emitter;
use crate::zigir::emit::helpers::EmitterHelpers;
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
                self.emit_inline_args(&call.args);
                self.write(")");
            }
        }
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
                self.emit_expr(object);
                self.write(".items.len");
            }
            FieldKind::StringLen => {
                // JS string.length returns UTF-16 code unit count, not byte count
                self.write("js_string.utf16Len(");
                self.emit_expr(object);
                self.write(")");
            }
            FieldKind::SliceLen => {
                // Slice/TypedArray length: element count.
                self.emit_expr(object);
                self.write(".len");
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
                // Zig 0.16.0: math constants are lowercase (pi, e, tau, etc.)
                match val.as_str() {
                    "PI" => self.write("std.math.pi"),
                    "E" => self.write("std.math.e"),
                    "LN2" => self.write("std.math.ln2"),
                    "LN10" => self.write("std.math.ln10"),
                    "LOG2E" => self.write("std.math.log2e"),
                    "LOG10E" => self.write("std.math.log10e"),
                    "SQRT1_2" => self.write("std.math.sqrt1_2"),
                    "SQRT2" => self.write("std.math.sqrt2"),
                    _ => self.write(&format!("std.math.{}", val.to_lowercase())),
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
        }
    }

    // ── Shared index/field helpers ────────────────────────
    // Used by emit_index_access, emit_computed_field, and emit_assign_target_inner.

    /// Emit `object.field` — dot-access on an expression.
    pub(super) fn emit_dot_access(&mut self, object: &crate::zigir::types::IrExpr, field: &str) {
        self.emit_expr(object);
        self.write(&format!(".{}", field));
    }

    /// Emit `object.items[@as(usize, @intCast(index))]` — ArrayList element access.
    pub(super) fn emit_arraylist_item(
        &mut self,
        object: &crate::zigir::types::IrExpr,
        index: &crate::zigir::types::IrExpr,
    ) {
        self.emit_expr(object);
        self.write(".items[@as(usize, @intCast(");
        self.emit_expr(index);
        self.write("))]");
    }

    /// Emit `object[@as(usize, @intCast(index))]` — Slice/array index access.
    /// The @intCast is needed because JS indices are i64 but Zig indexing requires usize.
    pub(super) fn emit_slice_index(
        &mut self,
        object: &crate::zigir::types::IrExpr,
        index: &crate::zigir::types::IrExpr,
    ) {
        self.emit_expr(object);
        self.write("[@as(usize, @intCast(");
        self.emit_expr(index);
        self.write("))]");
    }

    /// Emit `__ClassName_field` — static field access on a class.
    pub(super) fn emit_static_field(&mut self, class_name: &str, field: &str) {
        self.write(&format!("__{}_{}", class_name, field));
    }
}
