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

    // ── some ───────────────────────────────────────────
    //
    //  (blk_N: {
    //      for (obj.items[, 0..]) |elem[, idx]| {
    //          if (<pred>) break :blk_N true;
    //      }
    //      break :blk_N false;
    //  })
    //
    pub(super) fn emit_some_inline(&mut self, data: &crate::zigir::types::IrArrayCallbackInline) {
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
        self.emit_callback_body(&data.body, |emitter, expr| {
            emitter.write("if (");
            emitter.emit_expr(expr);
            emitter.write(&format!(") break :{} true;", blk_clone));
        });
        self.indent_pop();
        self.writeln("");
        self.write("}");
        self.write(&format!(" break :{} false; }})", blk));
    }

    // ── every ──────────────────────────────────────────
    //
    //  (blk_N: {
    //      for (obj.items[, 0..]) |elem[, idx]| {
    //          if (!(<pred>)) break :blk_N false;
    //      }
    //      break :blk_N true;
    //  })
    //
    pub(super) fn emit_every_inline(&mut self, data: &crate::zigir::types::IrArrayCallbackInline) {
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
        self.emit_callback_body(&data.body, |emitter, expr| {
            emitter.write("if (!(");
            emitter.emit_expr(expr);
            emitter.write(&format!(")) break :{} false;", blk_clone));
        });
        self.indent_pop();
        self.writeln("");
        self.write("}");
        self.write(&format!(" break :{} true; }})", blk));
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

    // ── find ───────────────────────────────────────────
    //
    //  (blk_N: {
    //      for (obj.items) |elem| {
    //          if (<pred>) break :blk_N elem;
    //      }
    //      break :blk_N undefined;
    //  })
    //
    pub(super) fn emit_find_inline(&mut self, data: &crate::zigir::types::IrArrayCallbackInline) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        let blk = self.begin_labeled_block(&binding);
        self.write(&format!("for ({}.items) |{}| ", receiver, data.elem_param));
        self.write("{\n");
        self.indent_push();
        let elem_param = data.elem_param.clone();
        let blk_clone = blk.clone();
        self.emit_callback_body(&data.body, |emitter, expr| {
            emitter.write("if (");
            emitter.emit_expr(expr);
            emitter.write(&format!(") break :{} {};", blk_clone, elem_param));
        });
        self.indent_pop();
        self.writeln("");
        self.write("}");
        self.write(&format!(" break :{} undefined; }})", blk));
    }

    // ── findIndex ──────────────────────────────────────
    //
    //  (blk_N: {
    //      for (obj.items, 0..) |elem, __i| {
    //          const __idx: i64 = @intCast(__i);
    //          if (<pred>) break :blk_N __idx;
    //      }
    //      break :blk_N -1;
    //  })
    //
    pub(super) fn emit_find_index_inline(
        &mut self,
        data: &crate::zigir::types::IrArrayCallbackInline,
    ) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        let blk = self.begin_labeled_block(&binding);
        let index_name = format!("__{}_i", data.elem_param);
        let idx_name = format!("__{}_idx", data.elem_param);
        self.write(&format!(
            "for ({}.items, 0..) |{}, {}| ",
            receiver, data.elem_param, index_name
        ));
        self.write("{\n");
        self.indent_push();
        self.writeln(&format!(
            "const {}: i64 = @intCast({});",
            idx_name, index_name
        ));
        let idx_name_clone = idx_name.clone();
        let blk_clone = blk.clone();
        self.emit_callback_body(&data.body, |emitter, expr| {
            emitter.write("if (");
            emitter.emit_expr(expr);
            emitter.write(&format!(") break :{} {};", blk_clone, idx_name_clone));
        });
        self.indent_pop();
        self.writeln("");
        self.write("}");
        self.write(&format!(" break :{} -1; }})", blk));
    }

    // ── findLast ───────────────────────────────────────
    //
    //  (blk_N: {
    //      var __i: usize = obj.items.len;
    //      while (__i > 0) {
    //          __i -= 1;
    //          const elem = obj.items[__i];
    //          if (<pred>) break :blk_N elem;
    //      }
    //      break :blk_N undefined;
    //  })
    //
    pub(super) fn emit_find_last_inline(
        &mut self,
        data: &crate::zigir::types::IrArrayCallbackInline,
    ) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        let blk = self.begin_labeled_block(&binding);
        self.write(&format!(
            "var __i: usize = {}.items.len; while (__i > 0) {{ __i -= 1; const {} = {}.items[__i]; ",
            receiver, data.elem_param, receiver
        ));
        self.indent_push();
        let elem_param = data.elem_param.clone();
        let blk_clone = blk.clone();
        self.emit_callback_body(&data.body, |emitter, expr| {
            emitter.write("if (");
            emitter.emit_expr(expr);
            emitter.write(&format!(") break :{} {};", blk_clone, elem_param));
        });
        self.indent_pop();
        self.writeln("");
        self.write(&format!("}} break :{} undefined; }})", blk));
    }

    // ── findLastIndex ──────────────────────────────────
    //
    //  (blk_N: {
    //      var __i: usize = obj.items.len;
    //      while (__i > 0) {
    //          __i -= 1;
    //          const elem = obj.items[__i];
    //          const __idx: i64 = @intCast(__i);
    //          if (<pred>) break :blk_N __idx;
    //      }
    //      break :blk_N -1;
    //  })
    //
    pub(super) fn emit_find_last_index_inline(
        &mut self,
        data: &crate::zigir::types::IrArrayCallbackInline,
    ) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        let blk = self.begin_labeled_block(&binding);
        let idx_name = format!("__{}_idx", data.elem_param);
        self.write(&format!(
            "var __i: usize = {}.items.len; while (__i > 0) {{ __i -= 1; const {} = {}.items[__i]; const {}: i64 = @intCast(__i); ",
            receiver, data.elem_param, receiver, idx_name
        ));
        self.indent_push();
        let idx_name_clone = idx_name.clone();
        let blk_clone = blk.clone();
        self.emit_callback_body(&data.body, |emitter, expr| {
            emitter.write("if (");
            emitter.emit_expr(expr);
            emitter.write(&format!(") break :{} {};", blk_clone, idx_name_clone));
        });
        self.indent_pop();
        self.writeln("");
        self.write(&format!("}} break :{} -1; }})", blk));
    }

    // ── map ────────────────────────────────────────────
    //
    //  (blk_N: {
    //      var __map: std.ArrayList(elem_type) = .empty;
    //      __map.ensureTotalCapacity(allocator, obj.items.len) catch @panic("OOM");
    //      for (obj.items) |elem| {
    //          __map.append(allocator, <transform>) catch @panic("OOM: Array.map append");
    //      }
    //      break :blk_N __map;
    //  })
    //
    //  The <transform> is the callback return expression.
    //  For concise arrow bodies, body has a single Expr(Return { value }) or Expr(expr).
    //
    pub(super) fn emit_map_inline(&mut self, data: &crate::zigir::types::IrArrayCallbackInline) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        let blk = self.begin_labeled_block(&binding);
        let elem_type_str = data.elem_type.to_zig_type();
        // When elem_param is "_" (unused in body), we still need a real variable name
        // for the for-loop capture — Zig's "_" is a discard, not an identifier.
        let loop_elem = if data.elem_param == "_" {
            "__melem".to_string()
        } else {
            data.elem_param.clone()
        };
        self.write(&format!(
            "var __map: std.ArrayList({}) = .empty; ",
            elem_type_str
        ));
        self.write(&format!(
            "__map.ensureTotalCapacity(js_allocator.allocator(), {}.items.len) catch @panic(\"OOM: Array.map capacity\"); ",
            receiver
        ));
        self.write(&format!("for ({}.items) |{}| ", receiver, loop_elem));
        self.write("{\n");
        self.indent_push();
        // Handle index parameter if present
        if data.has_idx_param && !data.idx_param.is_empty() && data.idx_param != "_" {
            // Map doesn't provide index in the for-loop easily,
            // but the lowerer handles it by capturing the loop variable.
            // For now, the idx_param is available as a separate counter if needed.
        }
        self.emit_callback_body(&data.body, |emitter, expr| {
            emitter.write("__map.append(js_allocator.allocator(), ");
            emitter.emit_expr(expr);
            emitter.write(") catch @panic(\"OOM: Array.map append\");");
        });
        self.indent_pop();
        self.writeln("");
        self.write("}");
        self.write(&format!(" break :{} __map; }})", blk));
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
}
