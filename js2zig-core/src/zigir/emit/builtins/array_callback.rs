// zigir/emit/builtins/array_callback.rs
// Array callback method inlining (forEach, some, every, filter, find, map, reduce, etc.).

use crate::zigir::emit::helpers::EmitterHelpers;

use crate::zigir::emit::Emitter;

// ═══════════════════════════════════════════════════════
//  Array callback inlining
// ═══════════════════════════════════════════════════════

impl Emitter {
    /// Emit an inlined array callback method (forEach, some, every, filter,
    /// find, findIndex, findLast, findLastIndex, map, reduce) as a Zig loop.
    ///
    /// Inline callback methods operate on IR nodes rather than AST.
    /// `IrArrayCallbackInline` data instead of raw AST.
    pub(crate) fn emit_array_callback_inline(
        &mut self,
        data: &crate::zigir::types::IrArrayCallbackInline,
    ) {
        use crate::zigir::types::ArrayCallbackKind as K;

        match data.kind {
            K::ForEach => self.emit_for_each_inline(data),
            K::Some => self.emit_some_inline(data),
            K::Every => self.emit_every_inline(data),
            K::Filter => self.emit_filter_inline(data),
            K::Find => self.emit_find_inline(data),
            K::FindIndex => self.emit_find_index_inline(data),
            K::FindLast => self.emit_find_last_inline(data),
            K::FindLastIndex => self.emit_find_last_index_inline(data),
            K::Map => self.emit_map_inline(data),
            K::Reduce => self.emit_reduce_inline(data),
            K::Sort => self.emit_sort_callback_inline(data),
            K::ToSorted => self.emit_to_sorted_callback_inline(data),
            K::FlatMap => self.emit_flat_map_inline(data),
        }
    }

    // ── forEach ────────────────────────────────────────
    //
    //  Array: for (obj.items) |elem| { <body stmts> }
    //  Map:   var iter = m.inner.iterator(); while (iter.next()) |entry| { const val = entry.value_ptr.*; const key = entry.key_ptr.*; <body stmts> }
    //  Set:   for (s.items.items) |val| { <body stmts> }
    //
    //  When chaining (obj_expr is set), wraps in a block:
    //    { const __chain_N = <expr>; for (__chain_N.items) |elem| { ... } }
    //
    pub(super) fn emit_for_each_inline(
        &mut self,
        data: &crate::zigir::types::IrArrayCallbackInline,
    ) {
        use crate::zigir::types::CollectionKind;

        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        match data.collection_kind {
            CollectionKind::Array => {
                // When chaining, wrap in a block so the const binding is scoped.
                if let Some(b) = &binding {
                    self.write("{ ");
                    self.write(b);
                }
                self.write(&format!("for ({}.items) |{}| ", receiver, data.elem_param));
                self.write("{\n");
                self.indent_push();
                for stmt in &data.body {
                    self.writeln("");
                    self.emit_stmt(stmt);
                }
                self.indent_pop();
                self.writeln("");
                self.write("}");
                if binding.is_some() {
                    self.write(" }");
                }
            }
            CollectionKind::Map => {
                // Map.forEach → while-iterator over inner HashMap
                if let Some(b) = &binding {
                    self.write("{ ");
                    self.write(b);
                }
                self.writeln(&format!("var iter = {}.inner.iterator();", receiver));
                self.writeln("while (iter.next()) |entry| {");
                self.indent_push();
                // Bind val and key parameters
                if data.elem_param != "_" {
                    self.writeln(&format!("const {} = entry.value_ptr.*;", data.elem_param));
                }
                // idx_param serves as the key parameter for Map forEach
                if !data.idx_param.is_empty() && data.idx_param != "_" {
                    self.writeln(&format!("const {} = entry.key_ptr.*;", data.idx_param));
                }
                for stmt in &data.body {
                    self.emit_stmt(stmt);
                }
                // Suppress unused variable warnings
                if data.elem_param != "_" {
                    self.writeln(&format!("_ = &{};", data.elem_param));
                }
                if !data.idx_param.is_empty() && data.idx_param != "_" {
                    self.writeln(&format!("_ = &{};", data.idx_param));
                }
                self.indent_pop();
                self.write("}");
                if binding.is_some() {
                    self.write(" }");
                }
            }
            CollectionKind::Set => {
                // Set.forEach → for-loop over set.items.items
                if let Some(b) = &binding {
                    self.write("{ ");
                    self.write(b);
                }
                self.write(&format!(
                    "for ({}.items.items) |{}| ",
                    receiver, data.elem_param
                ));
                self.write("{\n");
                self.indent_push();
                for stmt in &data.body {
                    self.writeln("");
                    self.emit_stmt(stmt);
                }
                self.indent_pop();
                self.writeln("");
                self.write("}");
                // Suppress unused variable warning
                if data.elem_param != "_" {
                    // The _ = &val; is emitted inside the loop by the lower_stmt
                }
                if binding.is_some() {
                    self.write(" }");
                }
            }
        }
    }

    // ── some / every (shared) ─────────────────────────────
    //
    //  Both emit a labeled block with a for-loop that short-circuits:
    //    some:  if (pred)  break :blk true;  default: false
    //    every: if (!(pred)) break :blk false; default: true
    //
    fn emit_short_circuit_inline(
        &mut self,
        data: &crate::zigir::types::IrArrayCallbackInline,
        negate: bool,
        match_value: &str,
        default_value: &str,
    ) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        let blk = self.begin_labeled_block(&binding);
        if data.has_idx_param {
            self.write(&format!(
                "for ({}.items, 0..) |{}, {}| ",
                receiver, data.elem_param, data.idx_param
            ));
        } else {
            self.write(&format!("for ({}.items) |{}| ", receiver, data.elem_param));
        }
        self.write("{\n");
        self.indent_push();
        let blk_clone = blk.clone();
        let match_val = match_value.to_string();
        self.emit_callback_body(&data.body, |emitter, expr| {
            if negate {
                emitter.write("if (!(");
                emitter.emit_expr(expr);
                emitter.write(&format!(")) break :{} {};", blk_clone, match_val));
            } else {
                emitter.write("if (");
                emitter.emit_expr(expr);
                emitter.write(&format!(") break :{} {};", blk_clone, match_val));
            }
        });
        self.indent_pop();
        self.writeln("");
        self.write("}");
        self.write(&format!(" break :{} {}; }})", blk, default_value));
    }

    // ── some ───────────────────────────────────────────

    pub(super) fn emit_some_inline(&mut self, data: &crate::zigir::types::IrArrayCallbackInline) {
        self.emit_short_circuit_inline(data, false, "true", "false");
    }

    // ── every ──────────────────────────────────────────

    pub(super) fn emit_every_inline(&mut self, data: &crate::zigir::types::IrArrayCallbackInline) {
        self.emit_short_circuit_inline(data, true, "false", "true");
    }

    // ── filter ─────────────────────────────────────────
    //
    //  (blk_N: {
    //      var __filter: std.ArrayList(elem_type) = .empty;
    //      for (obj.items) |elem| {
    //          if (<pred>) __filter.append(js_allocator.allocator(), elem) catch @panic("OOM: Array.filter append");
    //      }
    //      break :blk_N __filter;
    //  })
    //
    pub(super) fn emit_filter_inline(&mut self, data: &crate::zigir::types::IrArrayCallbackInline) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        let blk = self.begin_labeled_block(&binding);
        let elem_type_str = data.elem_type.to_zig_type();
        // When elem_param is "_" (unused in body), we still need a real variable name
        // for the __filter.append() call — Zig's "_" is a discard, not an identifier.
        let append_elem = if data.elem_param == "_" {
            "__felem".to_string()
        } else {
            data.elem_param.clone()
        };
        self.write(&format!(
            "var __filter: std.ArrayList({}) = .empty; ",
            elem_type_str
        ));
        self.write(&format!("for ({}.items) |{}| ", receiver, append_elem));
        self.write("{\n");
        self.indent_push();
        let append_elem_clone = append_elem.clone();
        self.emit_callback_body(&data.body, |emitter, expr| {
            emitter.write("if (");
            emitter.emit_expr(expr);
            emitter.write(&format!(
                ") {{ __filter.append(js_allocator.allocator(), {}) catch @panic(\"OOM: Array.filter append\"); }}",
                append_elem_clone
            ));
        });
        self.indent_pop();
        self.writeln("");
        self.write("}");
        self.write(&format!(" break :{} __filter; }})", blk));
    }

    // ── find / findLast (shared) ────────────────────────
    //
    //  find:      for (obj.items) |elem| { if (pred) break :blk elem; } break :blk undefined;
    //  findLast:  var __i = len; while (__i > 0) { __i--; const elem = items[__i]; if (pred) break :blk elem; } break :blk undefined;
    //
    fn emit_find_like_inline(
        &mut self,
        data: &crate::zigir::types::IrArrayCallbackInline,
        reverse: bool,
    ) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        let blk = self.begin_labeled_block(&binding);
        let elem_param = data.elem_param.clone();
        let blk_clone = blk.clone();

        if reverse {
            self.write(&format!(
                "var __i: usize = {}.items.len; while (__i > 0) {{ __i -= 1; const {} = {}.items[__i]; ",
                receiver, data.elem_param, receiver
            ));
        } else {
            self.write(&format!("for ({}.items) |{}| ", receiver, data.elem_param));
            self.write("{\n");
        }
        self.indent_push();
        self.emit_callback_body(&data.body, |emitter, expr| {
            emitter.write("if (");
            emitter.emit_expr(expr);
            emitter.write(&format!(") break :{} {};", blk_clone, elem_param));
        });
        self.indent_pop();
        self.writeln("");

        if reverse {
            self.write(&format!("}} break :{} undefined; }})", blk));
        } else {
            self.write("}");
            self.write(&format!(" break :{} undefined; }})", blk));
        }
    }

    // ── find ───────────────────────────────────────────

    pub(super) fn emit_find_inline(&mut self, data: &crate::zigir::types::IrArrayCallbackInline) {
        self.emit_find_like_inline(data, false);
    }

    // ── findIndex / findLastIndex (shared) ──────────────
    //
    //  findIndex:     for (items, 0..) |elem, __i| { const __idx = @intCast(__i); if (pred) break :blk __idx; } break :blk -1;
    //  findLastIndex: var __i = len; while (__i > 0) { __i--; const elem = items[__i]; const __idx = @intCast(__i); if (pred) break :blk __idx; } break :blk -1;
    //
    fn emit_find_index_like_inline(
        &mut self,
        data: &crate::zigir::types::IrArrayCallbackInline,
        reverse: bool,
    ) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        let blk = self.begin_labeled_block(&binding);
        let idx_name = format!("__{}_idx", data.elem_param);
        let idx_name_clone = idx_name.clone();
        let blk_clone = blk.clone();

        if reverse {
            self.write(&format!(
                "var __i: usize = {}.items.len; while (__i > 0) {{ __i -= 1; const {} = {}.items[__i]; const {}: i64 = @intCast(__i); ",
                receiver, data.elem_param, receiver, idx_name
            ));
        } else {
            let index_name = format!("__{}_i", data.elem_param);
            self.write(&format!(
                "for ({}.items, 0..) |{}, {}| ",
                receiver, data.elem_param, index_name
            ));
            self.write("{\n");
            self.writeln(&format!(
                "const {}: i64 = @intCast({});",
                idx_name, index_name
            ));
        }
        self.indent_push();
        self.emit_callback_body(&data.body, |emitter, expr| {
            emitter.write("if (");
            emitter.emit_expr(expr);
            emitter.write(&format!(") break :{} {};", blk_clone, idx_name_clone));
        });
        self.indent_pop();
        self.writeln("");

        if reverse {
            self.write(&format!("}} break :{} -1; }})", blk));
        } else {
            self.write("}");
            self.write(&format!(" break :{} -1; }})", blk));
        }
    }

    // ── findIndex ──────────────────────────────────────

    pub(super) fn emit_find_index_inline(
        &mut self,
        data: &crate::zigir::types::IrArrayCallbackInline,
    ) {
        self.emit_find_index_like_inline(data, false);
    }

    // ── findLast ───────────────────────────────────────

    pub(super) fn emit_find_last_inline(
        &mut self,
        data: &crate::zigir::types::IrArrayCallbackInline,
    ) {
        self.emit_find_like_inline(data, true);
    }

    // ── findLastIndex ──────────────────────────────────

    pub(super) fn emit_find_last_index_inline(
        &mut self,
        data: &crate::zigir::types::IrArrayCallbackInline,
    ) {
        self.emit_find_index_like_inline(data, true);
    }

    // ── map / flatMap (shared) ────────────────────────────
    //
    //  Both emit the same pattern: create ArrayList → pre-allocate → for-loop
    //  append → break with list.  Only the variable prefix and method name
    //  differ (map → __map, flatMap → __fmap).
    //
    //  (blk_N: {
    //      var <var>: std.ArrayList(elem_type) = .empty;
    //      <var>.ensureTotalCapacity(allocator, obj.items.len) catch @panic("OOM");
    //      for (obj.items) |elem| {
    //          <var>.append(allocator, <body_expr>) catch @panic("OOM");
    //      }
    //      break :blk_N <var>;
    //  })
    //
    fn emit_collect_inline(
        &mut self,
        data: &crate::zigir::types::IrArrayCallbackInline,
        var_prefix: &str,
        method_name: &str,
    ) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        let blk = self.begin_labeled_block(&binding);
        let elem_type_str = data.elem_type.to_zig_type();
        let loop_elem = if data.elem_param == "_" {
            "__melem".to_string()
        } else {
            data.elem_param.clone()
        };
        self.write(&format!(
            "var {}: std.ArrayList({}) = .empty; ",
            var_prefix, elem_type_str
        ));
        self.write(&format!(
            "{}.ensureTotalCapacity(js_allocator.allocator(), {}.items.len) catch @panic(\"OOM: Array.{} capacity\"); ",
            var_prefix, receiver, method_name
        ));
        self.write(&format!("for ({}.items) |{}| ", receiver, loop_elem));
        self.write("{\n");
        self.indent_push();
        let var_clone = var_prefix.to_string();
        let method_clone = method_name.to_string();
        self.emit_callback_body(&data.body, move |emitter, expr| {
            emitter.write(&format!("{}.append(js_allocator.allocator(), ", var_clone));
            emitter.emit_expr(expr);
            emitter.write(&format!(
                ") catch @panic(\"OOM: Array.{} append\");",
                method_clone
            ));
        });
        self.indent_pop();
        self.writeln("");
        self.write("}");
        self.write(&format!(" break :{} {}; }})", blk, var_prefix));
    }

    // ── map ────────────────────────────────────────────

    pub(super) fn emit_map_inline(&mut self, data: &crate::zigir::types::IrArrayCallbackInline) {
        self.emit_collect_inline(data, "__map", "map");
    }

    // ── reduce ─────────────────────────────────────────
    //
    //  (blk_N: {
    //      var acc: <type> = <init>;
    //      for (obj.items) |elem| {
    //          acc = <body_expr>;
    //      }
    //      break :blk_N acc;
    //  })
    //
    pub(super) fn emit_reduce_inline(&mut self, data: &crate::zigir::types::IrArrayCallbackInline) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        let blk = self.begin_labeled_block(&binding);
        let acc_name = format!("_acc_{}", self.peek_label_id());
        // Determine init value and accumulator type
        let init_expr_str = match &data.reduce_init {
            Some(expr) => self.render_expr_to_string(expr),
            None => "0".to_string(),
        };
        let acc_type = if init_expr_str.contains('.') {
            "f64"
        } else {
            "i64"
        };
        self.write(&format!(
            "var {}: {} = {}; ",
            acc_name, acc_type, init_expr_str
        ));

        // For reduce, the for-loop captures the current element.
        // The first callback param (elem_param, e.g., "acc") aliases the accumulator.
        // The second callback param (idx_param, e.g., "x") is the current element.
        let loop_var = if !data.idx_param.is_empty() && data.idx_param != "_" {
            // Two-param callback: use idx_param as the loop variable (current element)
            data.idx_param.clone()
        } else {
            // Single-param callback or no second param: use elem_param as loop variable
            data.elem_param.clone()
        };

        self.write(&format!("for ({}.items) |{}| ", receiver, loop_var));
        self.write("{\n");
        self.indent_push();

        // Bind elem_param to the accumulator when it differs from the loop variable
        // (i.e., when the callback has two params and elem_param is "acc")
        if data.elem_param != "_" && data.elem_param != loop_var {
            self.writeln(&format!("const {} = {};", data.elem_param, acc_name));
        }

        let acc_name_clone = acc_name.clone();
        self.emit_callback_body(&data.body, |emitter, expr| {
            emitter.write(&format!("{} = ", acc_name_clone));
            emitter.emit_expr(expr);
            emitter.write(";");
        });
        self.indent_pop();
        self.writeln("");
        self.write("}");
        self.write(&format!(" break :{} {}; }})", blk, acc_name));
    }

    // ── sort (with compareFn) ──────────────────────────────
    //
    //  arr.sort((a, b) => a - b)  →  in-place sort with custom comparator
    //
    //  Note: JS compareFn(a, b) returns <0 if a < b, 0 if equal, >0 if a > b.
    //  Zig lessThan returns bool, so we convert: compareFn(a, b) < 0 → a < b.
    //
    pub(super) fn emit_sort_callback_inline(
        &mut self,
        data: &crate::zigir::types::IrArrayCallbackInline,
    ) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        let (elem_type_str, param_a, param_b) = self.resolve_sort_params(data);

        if let Some(b) = &binding {
            self.write("{ ");
            self.write(b);
        }

        let blk = self.next_label();
        self.write(&format!("({}: ", blk));

        self.emit_sort_less_than(
            &format!("{}.items", receiver),
            &elem_type_str,
            &param_a,
            &param_b,
            &data.body,
        );

        self.write(&format!(" break :{} {}; }})", blk, receiver));

        if binding.is_some() {
            self.write(" }");
        }
    }

    // ── toSorted (with compareFn) ───────────────────────────
    //
    //  arr.toSorted((a, b) => a - b)  →  sort returning a new array
    //
    pub(super) fn emit_to_sorted_callback_inline(
        &mut self,
        data: &crate::zigir::types::IrArrayCallbackInline,
    ) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        let (elem_type_str, param_a, param_b) = self.resolve_sort_params(data);

        let blk = self.begin_labeled_block(&binding);
        self.write(&format!(
            "var __sorted: std.ArrayList({}) = .empty; ",
            elem_type_str
        ));
        self.write(&format!(
            "__sorted.appendSlice(js_allocator.allocator(), {}.items) catch @panic(\"OOM: Array.toSorted appendSlice\"); ",
            receiver
        ));

        self.emit_sort_less_than(
            "__sorted.items",
            &elem_type_str,
            &param_a,
            &param_b,
            &data.body,
        );

        self.write(&format!(" break :{} __sorted; }})", blk));
    }

    /// Resolve shared sort/toSorted parameters: (elem_type_str, param_a, param_b).
    fn resolve_sort_params(
        &mut self,
        data: &crate::zigir::types::IrArrayCallbackInline,
    ) -> (String, String, String) {
        let elem_type_str = data.elem_type.to_zig_type();
        let param_a = data.elem_param.clone();
        let param_b = if !data.idx_param.is_empty() && data.idx_param != "_" {
            data.idx_param.clone()
        } else {
            "_".to_string()
        };
        (elem_type_str, param_a, param_b)
    }

    /// Shared helper: emit `std.mem.sort(ElemType, target, {}, struct { fn lessThan ... }`
    /// for both sort and toSorted callback inlining.
    fn emit_sort_less_than(
        &mut self,
        target_items: &str,
        elem_type_str: &str,
        param_a: &str,
        param_b: &str,
        body: &[crate::zigir::types::IrStmt],
    ) {
        self.write(&format!(
            "std.mem.sort({}, {}, {{}}, struct {{ fn lessThan(_: void, {}: {}, {}: {}) bool {{ ",
            elem_type_str, target_items, param_a, elem_type_str, param_b, elem_type_str
        ));

        self.emit_callback_body(body, |emitter, expr| {
            emitter.write("return (");
            emitter.emit_expr(expr);
            emitter.write(") < 0;");
        });

        self.write(" } }}.lessThan);");
    }

    // ── flatMap ───────────────────────────────────────
    //
    //  arr.flatMap(fn) → map + flatten(depth=1).
    //  Since our type system uses uniform element types (ArrayList(i64), etc.),
    //  the callback returns a scalar, so flatMap is semantically equivalent to map
    //  (flatten(1) on a flat array of scalars is a no-op).
    //  Delegates to emit_collect_inline with __fmap prefix.

    pub(super) fn emit_flat_map_inline(
        &mut self,
        data: &crate::zigir::types::IrArrayCallbackInline,
    ) {
        self.emit_collect_inline(data, "__fmap", "flatMap");
    }
}
