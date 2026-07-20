// zigir/emit/expr/container.rs
// Array and object literal emission.

use crate::zigir::emit::Emitter;
use crate::zigir::emit::helpers::EmitterHelpers;

impl Emitter {
    pub(super) fn emit_array_literal(&mut self, arr: &crate::zigir::types::IrArrayLiteral) {
        if arr.elements.is_empty() {
            self.write("std.ArrayList(JsAny).empty");
            return;
        }

        // Determine element type: if any spread is present, force JsAny (mixed types guaranteed).
        // Otherwise, scan ALL elements — if any differ from the first, use JsAny.
        let elem_type = if !arr.spread_indices.is_empty() {
            "JsAny"
        } else {
            let first_type = arr
                .elements
                .first()
                .map(|e| match e {
                    crate::zigir::types::IrExpr::IntLiteral(_) => "i64",
                    crate::zigir::types::IrExpr::FloatLiteral(_) => "f64",
                    crate::zigir::types::IrExpr::StringLiteral(_) => "[]const u8",
                    crate::zigir::types::IrExpr::BoolLiteral(_) => "bool",
                    // Non-literal elements (Ident, Call, FieldAccess, etc.)
                    // have no statically-known type at the emit layer. Fall
                    // back to JsAny rather than i64: this guarantees the
                    // generated ArrayList is type-compatible with whatever
                    // the expression evaluates to (JsAny.from polymorphism
                    // handles all element types). Forcing i64 here would
                    // produce Zig compile errors like "expected i64, found
                    // f64" for cases like `const arr = [someF64Var]`.
                    _ => "JsAny",
                })
                .unwrap_or("JsAny");

            let all_same = arr.elements.iter().all(|e| match e {
                crate::zigir::types::IrExpr::IntLiteral(_) => first_type == "i64",
                crate::zigir::types::IrExpr::FloatLiteral(_) => first_type == "f64",
                crate::zigir::types::IrExpr::StringLiteral(_) => first_type == "[]const u8",
                crate::zigir::types::IrExpr::BoolLiteral(_) => first_type == "bool",
                // Non-literal matches only when first_type has already
                // degraded to JsAny — preserving the conservative fallback.
                _ => first_type == "JsAny",
            });

            if all_same { first_type } else { "JsAny" }
        };
        let needs_jsany_wrap = elem_type == "JsAny";

        // Emit as labeled block with ArrayList builder:
        // (blk: { var __arr: std.ArrayList(Type) = .empty; append...; break :blk __arr; })
        let blk = self.next_label();
        self.write(&format!(
            "({}: {{ var __arr: std.ArrayList({}) = .empty; ",
            blk, elem_type
        ));
        for (i, elem) in arr.elements.iter().enumerate() {
            if arr.spread_indices.contains(&i) {
                // Spread: iterate the source's `.items` slice and append each
                // element wrapped via `JsAny.from(...)`. This handles ANY
                // ArrayList element type uniformly (i64, f64, []const u8, JsAny,
                // bool, comptime_int, etc.).
                //
                // Why not appendSlice: the receiver __arr is always
                // `std.ArrayList(JsAny)` when a spread is present (see the
                // `elem_type` decision above), so `appendSlice` would require
                // the source's `.items` to be `[]const JsAny`. That fails to
                // compile for the common `ArrayList(i64)` / `ArrayList(bool)` /
                // `ArrayList([]const u8)` cases. The for-loop + JsAny.from
                // approach is type-polymorphic via anytype.
                //
                // The loop variable name (`__spread_item`) is scoped to each
                // for-loop body, so reusing it across multiple spreads in the
                // same literal is safe.
                if let crate::zigir::types::IrExpr::Spread(inner) = elem {
                    self.write("for (");
                    self.emit_expr(inner);
                    self.write(
                        ".items) |__spread_item| __arr.append(js_allocator.allocator(), \
                         JsAny.from(__spread_item)) catch @panic(\"OOM: Array.spread\"); ",
                    );
                }
            } else {
                self.write("__arr.append(js_allocator.allocator(), ");
                if needs_jsany_wrap {
                    self.write("JsAny.from(");
                }
                self.emit_expr(elem);
                if needs_jsany_wrap {
                    self.write(")");
                }
                self.write(") catch @panic(\"OOM: Array.push append\"); ");
            }
        }
        self.write(&format!("break :{} __arr; }})", blk));
    }

    /// Emit object field pairs (`.key = value` or `@"key" = value`), preceded by
    /// separator handling. Shared between the inline-property path and the
    /// spreadMerge inline-text path.
    fn emit_object_fields<'a, I>(&mut self, fields: I)
    where
        I: Iterator<Item = &'a crate::zigir::types::IrObjectField>,
    {
        let mut first = true;
        for field in fields {
            if !first {
                self.write(", ");
            }
            first = false;
            if field.is_computed {
                self.write(&format!("@\"{}\" = ", field.key));
            } else {
                self.write(&format!(".{} = ", field.key));
            }
            self.emit_expr(&field.value);
        }
    }

    pub(super) fn emit_object_literal(&mut self, obj: &crate::zigir::types::IrObjectLiteral) {
        use crate::zigir::types::IrObjectItem;

        // Empty object → JsObjectMap.init(allocator)
        // JsObjectMap is StringArrayHashMap(JsAny) — insertion-order-preserving.
        if obj.items.is_empty() {
            self.write("JsObjectMap.init(js_allocator.allocator())");
            return;
        }

        // Check if any spread items exist
        let has_spread = obj
            .items
            .iter()
            .any(|item| matches!(item, IrObjectItem::Spread(_)));

        // Extract inline fields once — reused in both branches below
        let inline_fields: Vec<_> = obj
            .items
            .iter()
            .filter_map(|item| match item {
                IrObjectItem::Field(f) => Some(f),
                _ => None,
            })
            .collect();

        if !has_spread {
            // Pure inline properties — emit directly as .{ ... }
            self.write(".{ ");
            self.emit_object_fields(inline_fields.iter().copied());
            self.write(" }");
            return;
        }

        // Has spread: build a left-fold spreadMerge(...) chain.
        // Strategy:
        //   { ...a }                       → a
        //   { ...a, ...b }                 → js_runtime.spreadMerge(a, b)
        //   { ...a, b: 1 }                 → js_runtime.spreadMerge(a, .{ .b = 1 })
        //   { ...a, ...b, c: 1 }           → js_runtime.spreadMerge(spreadMerge(a, b), .{ .c = 1 })

        // Collect spread expression texts
        let mut parts: Vec<String> = obj
            .items
            .iter()
            .filter_map(|item| match item {
                IrObjectItem::Spread(expr) => Some(self.expr_to_string(expr)),
                _ => None,
            })
            .collect();

        // Append inline fields as a single .{ .key = val } text, if any
        if !inline_fields.is_empty() {
            let saved_output = std::mem::take(&mut self.output);
            self.write(".{ ");
            self.emit_object_fields(inline_fields.iter().copied());
            self.write(" }");
            let text = std::mem::take(&mut self.output);
            self.output = saved_output;
            parts.push(text);
        }

        match parts.len() {
            0 => unreachable!("emit_array_with_spread: parts is empty but has_spread=true"),
            1 => {
                self.write(&parts[0]);
            }
            _ => {
                let result = parts[1..].iter().fold(parts[0].clone(), |acc, next| {
                    format!("js_runtime.spreadMerge({}, {})", acc, next)
                });
                self.write(&result);
            }
        }
    }
}
