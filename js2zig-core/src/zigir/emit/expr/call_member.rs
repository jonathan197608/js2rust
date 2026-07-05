// zigir/emit/expr/call_member.rs
// Call expression, field access, index access, and computed field emission.

use crate::zigir::emit::Emitter;
use crate::zigir::emit::helpers::EmitterHelpers;
use crate::zigir::kinds::{CallKind, ComputedKeyKind, FieldKind, IndexKind};

impl Emitter {
    pub(super) fn emit_call_expr(&mut self, call: &crate::zigir::types::IrCallExpr) {
        match &call.call_kind {
            CallKind::Direct => {
                self.emit_expr(&call.callee);
                self.emit_args(&call.args);
            }
            CallKind::Method { object_type: _ } => {
                self.emit_expr(&call.callee);
                self.emit_args(&call.args);
            }
            CallKind::Closure => {
                self.emit_expr(&call.callee);
                self.write(".call(");
                for (i, arg) in call.args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.emit_expr(arg);
                }
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
            FieldKind::StructField => {
                self.emit_expr(object);
                self.write(&format!(".{}", field));
            }
            FieldKind::Namespace => {
                self.emit_expr(object);
                self.write(&format!(".{}", field));
            }
            FieldKind::ArrayListLen => {
                self.emit_expr(object);
                self.write(".items.len");
            }
            FieldKind::StringLen => {
                self.emit_expr(object);
                self.write(".len");
            }
            FieldKind::MapSetSize => {
                self.emit_expr(object);
                self.write(".size()");
            }
            FieldKind::MathConstant(val) => {
                self.write(&format!("std.math.{}", val));
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
                    self.emit_expr(object);
                    self.write(&format!(".{}", prop));
                }
            }
            FieldKind::Private => {
                self.emit_expr(object);
                self.write(&format!(".{}", field));
            }
            FieldKind::PointerDeref => {
                self.emit_expr(object);
                self.write(".*");
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
                self.emit_expr(object);
                self.write(".items[");
                self.emit_expr(index);
                self.write("]");
            }
            IndexKind::SliceIndex => {
                self.emit_expr(object);
                self.write("[");
                self.emit_expr(index);
                self.write("]");
            }
        }
    }

    pub(super) fn emit_computed_field(
        &mut self,
        object: &crate::zigir::types::IrExpr,
        key: &crate::zigir::types::IrExpr,
        kind: &ComputedKeyKind,
    ) {
        use crate::zigir::emit::helpers::escape_zig_string;
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
                self.emit_expr(object);
                self.write(".items[");
                self.emit_expr(key);
                self.write("]");
            }
            ComputedKeyKind::CompileError(msg) => {
                self.write(&format!("@compileError(\"{}\")", escape_zig_string(msg)));
            }
        }
    }
}
