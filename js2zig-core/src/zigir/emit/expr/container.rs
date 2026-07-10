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
        // Otherwise, infer from first element.
        let elem_type = if !arr.spread_indices.is_empty() {
            "JsAny"
        } else {
            arr.elements
                .first()
                .map(|e| match e {
                    crate::zigir::types::IrExpr::IntLiteral(_) => "i64",
                    crate::zigir::types::IrExpr::FloatLiteral(_) => "f64",
                    crate::zigir::types::IrExpr::StringLiteral(_) => "[]const u8",
                    crate::zigir::types::IrExpr::BoolLiteral(_) => "bool",
                    _ => "i64",
                })
                .unwrap_or("i64")
        };

        // Emit as labeled block with ArrayList builder:
        // (blk: { var __arr: std.ArrayList(Type) = .empty; append...; break :blk __arr; })
        let blk = self.next_label();
        self.write(&format!(
            "({}: {{ var __arr: std.ArrayList({}) = .empty; ",
            blk, elem_type
        ));
        for (i, elem) in arr.elements.iter().enumerate() {
            if arr.spread_indices.contains(&i) {
                // Spread element: use appendSlice
                if let crate::zigir::types::IrExpr::Spread(inner) = elem {
                    self.write("__arr.appendSlice(js_allocator.allocator(), ");
                    self.emit_expr(inner);
                    self.write(".items) catch @panic(\"OOM: Array.spread\"); ");
                }
            } else {
                self.write("__arr.append(js_allocator.allocator(), ");
                self.emit_expr(elem);
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

        // Empty object → StringHashMap(JsAny).init(allocator)
        if obj.items.is_empty() {
            self.write("std.StringHashMap(JsAny).init(js_allocator.allocator())");
            return;
        }

        // Check if any spread items exist
        let has_spread = obj
            .items
            .iter()
            .any(|item| matches!(item, IrObjectItem::Spread(_)));

        if !has_spread {
            // Pure inline properties — emit directly as .{ ... }
            self.write(".{ ");
            self.emit_object_fields(obj.items.iter().filter_map(|item| {
                if let IrObjectItem::Field(f) = item {
                    Some(f)
                } else {
                    None
                }
            }));
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
        let mut spread_texts: Vec<String> = Vec::new();
        for item in &obj.items {
            if let IrObjectItem::Spread(expr) = item {
                spread_texts.push(self.expr_to_string(expr));
            }
        }

        // Collect inline fields as .{ .key = val } string
        let inline_fields: Vec<_> = obj
            .items
            .iter()
            .filter_map(|item| {
                if let IrObjectItem::Field(f) = item {
                    Some(f)
                } else {
                    None
                }
            })
            .collect();

        let inline_text = if inline_fields.is_empty() {
            None
        } else {
            let saved_output = std::mem::take(&mut self.output);
            self.write(".{ ");
            self.emit_object_fields(inline_fields.iter().copied());
            self.write(" }");
            let text = std::mem::take(&mut self.output);
            self.output = saved_output;
            Some(text)
        };

        match (spread_texts.len(), &inline_text) {
            (0, _) => unreachable!(), // has_spread is true, so spread_texts is non-empty
            (1, None) => {
                // Single spread, no inline → identity
                self.write(&spread_texts[0]);
            }
            _ => {
                // Multi-spread or spread + inline → build spreadMerge chain
                let mut result = spread_texts[0].clone();
                for text in &spread_texts[1..] {
                    result = format!("js_runtime.spreadMerge({}, {})", result, text);
                }
                if let Some(ref inline) = inline_text {
                    result = format!("js_runtime.spreadMerge({}, {})", result, inline);
                }
                self.write(&result);
            }
        }
    }
}
