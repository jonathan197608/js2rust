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
    pub(super) fn emit_for_each_inline(
        &mut self,
        data: &crate::zigir::types::IrArrayCallbackInline,
    ) {
        use crate::zigir::types::CollectionKind;

        match data.collection_kind {
            CollectionKind::Array => {
                self.write(&format!(
                    "for ({}.items) |{}| ",
                    data.obj_name, data.elem_param
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
            }
            CollectionKind::Map => {
                // Map.forEach → while-iterator over inner HashMap
                self.writeln(&format!("var iter = {}.inner.iterator();", data.obj_name));
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
            }
            CollectionKind::Set => {
                // Set.forEach → for-loop over set.items.items
                self.write(&format!(
                    "for ({}.items.items) |{}| ",
                    data.obj_name, data.elem_param
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
        let blk = self.next_label();
        self.write(&format!("({}: {{ ", blk));
        if data.has_idx_param {
            self.write(&format!(
                "for ({}.items, 0..) |{}, {}| ",
                data.obj_name, data.elem_param, data.idx_param
            ));
        } else {
            self.write(&format!(
                "for ({}.items) |{}| ",
                data.obj_name, data.elem_param
            ));
        }
        self.write("{\n");
        self.indent_push();
        for stmt in &data.body {
            self.writeln("");
            match stmt {
                crate::zigir::types::IrStmt::Return { value: Some(expr) } => {
                    self.write("if (");
                    self.emit_expr(expr);
                    self.write(&format!(") break :{} true;", blk));
                }
                crate::zigir::types::IrStmt::Expr(expr) => {
                    self.write("if (");
                    self.emit_expr(expr);
                    self.write(&format!(") break :{} true;", blk));
                }
                _ => self.emit_stmt(stmt),
            }
        }
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
        let blk = self.next_label();
        self.write(&format!("({}: {{ ", blk));
        if data.has_idx_param {
            self.write(&format!(
                "for ({}.items, 0..) |{}, {}| ",
                data.obj_name, data.elem_param, data.idx_param
            ));
        } else {
            self.write(&format!(
                "for ({}.items) |{}| ",
                data.obj_name, data.elem_param
            ));
        }
        self.write("{\n");
        self.indent_push();
        for stmt in &data.body {
            self.writeln("");
            match stmt {
                crate::zigir::types::IrStmt::Return { value: Some(expr) } => {
                    self.write("if (!(");
                    self.emit_expr(expr);
                    self.write(&format!(")) break :{} false;", blk));
                }
                crate::zigir::types::IrStmt::Expr(expr) => {
                    self.write("if (!(");
                    self.emit_expr(expr);
                    self.write(&format!(")) break :{} false;", blk));
                }
                _ => self.emit_stmt(stmt),
            }
        }
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
        let blk = self.next_label();
        let elem_type_str = data.elem_type.to_zig_type();
        self.write(&format!("({}: {{ ", blk));
        self.write(&format!(
            "var __filter: std.ArrayList({}) = .empty; ",
            elem_type_str
        ));
        self.write(&format!(
            "for ({}.items) |{}| ",
            data.obj_name, data.elem_param
        ));
        self.write("{\n");
        self.indent_push();
        for stmt in &data.body {
            self.writeln("");
            match stmt {
                crate::zigir::types::IrStmt::Return { value: Some(expr) } => {
                    self.write("if (");
                    self.emit_expr(expr);
                    self.write(&format!(
                        ") {{ __filter.append(js_allocator.allocator(), {}) catch @panic(\"OOM: Array.filter append\"); }}",
                        data.elem_param
                    ));
                }
                crate::zigir::types::IrStmt::Expr(expr) => {
                    self.write("if (");
                    self.emit_expr(expr);
                    self.write(&format!(
                        ") {{ __filter.append(js_allocator.allocator(), {}) catch @panic(\"OOM: Array.filter append\"); }}",
                        data.elem_param
                    ));
                }
                _ => self.emit_stmt(stmt),
            }
        }
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
        let blk = self.next_label();
        self.write(&format!("({}: {{ ", blk));
        self.write(&format!(
            "for ({}.items) |{}| ",
            data.obj_name, data.elem_param
        ));
        self.write("{\n");
        self.indent_push();
        for stmt in &data.body {
            self.writeln("");
            match stmt {
                crate::zigir::types::IrStmt::Return { value: Some(expr) } => {
                    self.write("if (");
                    self.emit_expr(expr);
                    self.write(&format!(") break :{} {};", blk, data.elem_param));
                }
                crate::zigir::types::IrStmt::Expr(expr) => {
                    self.write("if (");
                    self.emit_expr(expr);
                    self.write(&format!(") break :{} {};", blk, data.elem_param));
                }
                _ => self.emit_stmt(stmt),
            }
        }
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
        let blk = self.next_label();
        let index_name = format!("__{}_i", data.elem_param);
        let idx_name = format!("__{}_idx", data.elem_param);
        self.write(&format!("({}: {{ ", blk));
        self.write(&format!(
            "for ({}.items, 0..) |{}, {}| ",
            data.obj_name, data.elem_param, index_name
        ));
        self.write("{\n");
        self.indent_push();
        self.writeln(&format!(
            "const {}: i64 = @intCast({});",
            idx_name, index_name
        ));
        for stmt in &data.body {
            self.writeln("");
            match stmt {
                crate::zigir::types::IrStmt::Return { value: Some(expr) } => {
                    self.write("if (");
                    self.emit_expr(expr);
                    self.write(&format!(") break :{} {};", blk, idx_name));
                }
                crate::zigir::types::IrStmt::Expr(expr) => {
                    self.write("if (");
                    self.emit_expr(expr);
                    self.write(&format!(") break :{} {};", blk, idx_name));
                }
                _ => self.emit_stmt(stmt),
            }
        }
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
        let blk = self.next_label();
        self.write(&format!(
            "({}: {{ var __i: usize = {}.items.len; while (__i > 0) {{ __i -= 1; const {} = {}.items[__i]; ",
            blk, data.obj_name, data.elem_param, data.obj_name
        ));
        self.indent_push();
        for stmt in &data.body {
            self.writeln("");
            match stmt {
                crate::zigir::types::IrStmt::Return { value: Some(expr) } => {
                    self.write("if (");
                    self.emit_expr(expr);
                    self.write(&format!(") break :{} {};", blk, data.elem_param));
                }
                crate::zigir::types::IrStmt::Expr(expr) => {
                    self.write("if (");
                    self.emit_expr(expr);
                    self.write(&format!(") break :{} {};", blk, data.elem_param));
                }
                _ => self.emit_stmt(stmt),
            }
        }
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
        let blk = self.next_label();
        let idx_name = format!("__{}_idx", data.elem_param);
        self.write(&format!(
            "({}: {{ var __i: usize = {}.items.len; while (__i > 0) {{ __i -= 1; const {} = {}.items[__i]; const {}: i64 = @intCast(__i); ",
            blk, data.obj_name, data.elem_param, data.obj_name, idx_name
        ));
        self.indent_push();
        for stmt in &data.body {
            self.writeln("");
            match stmt {
                crate::zigir::types::IrStmt::Return { value: Some(expr) } => {
                    self.write("if (");
                    self.emit_expr(expr);
                    self.write(&format!(") break :{} {};", blk, idx_name));
                }
                crate::zigir::types::IrStmt::Expr(expr) => {
                    self.write("if (");
                    self.emit_expr(expr);
                    self.write(&format!(") break :{} {};", blk, idx_name));
                }
                _ => self.emit_stmt(stmt),
            }
        }
        self.indent_pop();
        self.writeln("");
        self.write(&format!("}} break :{} -1; }})", blk));
    }

    // ── map (identity stub) ────────────────────────────
    //
    //  The Emitter just returns the object name — map is not fully implemented.
    //
    pub(super) fn emit_map_inline(&mut self, data: &crate::zigir::types::IrArrayCallbackInline) {
        self.write(&data.obj_name);
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
        let blk = self.next_label();
        let acc_name = format!("_acc_{}", self.peek_label_id());
        // Determine init value and accumulator type
        let init_expr_str = match &data.reduce_init {
            Some(expr) => {
                let saved = std::mem::take(self.output_mut());
                self.emit_expr(expr);
                let rendered = std::mem::take(self.output_mut());
                *self.output_mut() = saved;
                rendered
            }
            None => "0".to_string(),
        };
        let acc_type = if init_expr_str.contains('.') {
            "f64"
        } else {
            "i64"
        };
        self.write(&format!("({}: {{ ", blk));
        self.write(&format!(
            "var {}: {} = {}; ",
            acc_name, acc_type, init_expr_str
        ));
        self.write(&format!(
            "for ({}.items) |{}| ",
            data.obj_name, data.elem_param
        ));
        self.write("{\n");
        self.indent_push();
        for stmt in &data.body {
            self.writeln("");
            match stmt {
                crate::zigir::types::IrStmt::Return { value: Some(expr) } => {
                    self.write(&format!("{} = ", acc_name));
                    self.emit_expr(expr);
                    self.write(";");
                }
                crate::zigir::types::IrStmt::Expr(expr) => {
                    self.write(&format!("{} = ", acc_name));
                    self.emit_expr(expr);
                    self.write(";");
                }
                _ => self.emit_stmt(stmt),
            }
        }
        self.indent_pop();
        self.writeln("");
        self.write("}");
        self.write(&format!(" break :{} {}; }})", blk, acc_name));
    }
}
