// zigir/emit/builtins/array_callback.rs
// Array callback method inlining (forEach, some, every, filter, find, map, reduce, etc.).

use crate::types::ZigType;
use crate::zigir::emit::helpers::EmitterHelpers;

use crate::zigir::emit::Emitter;

// ═══════════════════════════════════════════════════════
//  Array callback inlining
// ═══════════════════════════════════════════════════════

impl Emitter {
    /// Return the items access suffix for a receiver: ".items" for ArrayList,
    /// empty string for rest-param slices ([]const JsAny).
    pub(super) fn items_path(&self, receiver: &str) -> &str {
        if self.rest_param_names.contains(receiver) {
            ""
        } else {
            ".items"
        }
    }

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
            K::ReduceRight => self.emit_reduce_right_inline(data),
            K::Sort => self.emit_sort_callback_inline(data),
            K::ToSorted => self.emit_to_sorted_callback_inline(data),
            K::FlatMap => self.emit_flat_map_inline(data),
        }
    }

    // ── forEach ────────────────────────────────────────
    //
    //  Array: for (obj.items) |elem| { <body stmts> }
    //  Map:   var iter = m.inner.iterator(); while (iter.next()) |entry| { ... }
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
                let items = self.items_path(&receiver).to_string();
                self.emit_for_each_simple_loop(
                    &binding,
                    &receiver,
                    &items,
                    &data.elem_param,
                    data.has_idx_param,
                    &data.idx_param,
                    data.has_arr_param,
                    &data.arr_param,
                    &data.body,
                );
            }
            CollectionKind::Map => {
                self.emit_for_each_map_loop(&binding, &receiver, data, &data.body);
            }
            CollectionKind::Set => {
                self.emit_for_each_set_loop(&binding, &receiver, data, &data.body);
            }
        }
    }

    /// Emit a simple for-loop forEach (Array/Set): binding + for (recv.items_path) |elem| { body }
    #[allow(clippy::too_many_arguments)]
    fn emit_for_each_simple_loop(
        &mut self,
        binding: &Option<String>,
        receiver: &str,
        items_path: &str,
        elem_param: &str,
        has_idx_param: bool,
        idx_param: &str,
        has_arr_param: bool,
        arr_param: &str,
        body: &[crate::zigir::types::IrStmt],
    ) {
        if let Some(b) = binding {
            self.write("{ ");
            self.write(b);
        }
        if has_idx_param && !idx_param.is_empty() && idx_param != "_" {
            self.write(&format!(
                "for ({}{}, 0..) |{}, {}| ",
                receiver, items_path, elem_param, idx_param
            ));
        } else {
            self.write(&format!(
                "for ({}{}) |{}| ",
                receiver, items_path, elem_param
            ));
        }
        self.write("{\n");
        self.indent_push();
        // Bind arr_param (third param: array reference) if present
        if has_arr_param && !arr_param.is_empty() && arr_param != "_" {
            self.writeln(&format!("const {} = {};", arr_param, receiver));
        }
        for stmt in body {
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

    /// Emit Map.forEach: while-iterator over inner HashMap with key/value binding.
    fn emit_for_each_map_loop(
        &mut self,
        binding: &Option<String>,
        receiver: &str,
        data: &crate::zigir::types::IrArrayCallbackInline,
        body: &[crate::zigir::types::IrStmt],
    ) {
        if let Some(b) = binding {
            self.write("{ ");
            self.write(b);
        }
        self.writeln(&format!("var iter = {}.iterator();", receiver));
        self.writeln("while (iter.next()) |entry| {");
        self.indent_push();
        if data.elem_param != "_" {
            self.writeln(&format!("const {} = entry.value_ptr.*;", data.elem_param));
        }
        if !data.idx_param.is_empty() && data.idx_param != "_" {
            self.writeln(&format!("const {} = entry.key_ptr.*;", data.idx_param));
        }
        // Bind arr_param (third param: Map reference) if present
        if data.has_arr_param && !data.arr_param.is_empty() && data.arr_param != "_" {
            self.writeln(&format!("const {} = {};", data.arr_param, receiver));
        }
        for stmt in body {
            self.emit_stmt(stmt);
        }
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

    /// Emit Set.forEach: while-iterator over inner HashMap with key binding.
    /// Set stores values as keys (value type is void), so we use key_ptr.*.
    fn emit_for_each_set_loop(
        &mut self,
        binding: &Option<String>,
        receiver: &str,
        data: &crate::zigir::types::IrArrayCallbackInline,
        body: &[crate::zigir::types::IrStmt],
    ) {
        if let Some(b) = binding {
            self.write("{ ");
            self.write(b);
        }
        self.writeln(&format!("var iter = {}.iterator();", receiver));
        self.writeln("while (iter.next()) |entry| {");
        self.indent_push();
        if data.elem_param != "_" {
            self.writeln(&format!("const {} = entry.key_ptr.*;", data.elem_param));
        }
        // Bind arr_param (third param: Set reference) if present
        if data.has_arr_param && !data.arr_param.is_empty() && data.arr_param != "_" {
            self.writeln(&format!("const {} = {};", data.arr_param, receiver));
        }
        for stmt in body {
            self.emit_stmt(stmt);
        }
        if data.elem_param != "_" {
            self.writeln(&format!("_ = &{};", data.elem_param));
        }
        self.indent_pop();
        self.write("}");
        if binding.is_some() {
            self.write(" }");
        }
    }
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
        let items = self.items_path(&receiver);
        if data.has_idx_param {
            self.write(&format!(
                "for ({}{}, 0..) |{}, {}| ",
                receiver, items, data.elem_param, data.idx_param
            ));
        } else {
            self.write(&format!(
                "for ({}{}) |{}| ",
                receiver, items, data.elem_param
            ));
        }
        self.write("{\n");
        self.indent_push();
        // Bind arr_param (third param: array reference) if present
        if data.has_arr_param && !data.arr_param.is_empty() && data.arr_param != "_" {
            self.writeln(&format!("const {} = {};", data.arr_param, receiver));
        }
        let blk_clone = blk.clone();
        let match_val = match_value.to_string();
        self.emit_callback_body(&data.body, |emitter, expr| {
            emitter.emit_if_break_pred(expr, &blk_clone, &match_val, negate);
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
        let var = format!("_f_{}", blk);
        let loop_elem_fallback = format!("_fe_{}", blk);
        let elem_type_str = data.elem_type.to_zig_type();
        let loop_elem = Self::resolve_loop_elem(&data.elem_param, &loop_elem_fallback);
        self.write(&format!(
            "var {}: std.ArrayList({}) = .empty; ",
            var, elem_type_str
        ));
        self.write(&format!(
            "for ({}{}) |{}| ",
            receiver,
            self.items_path(&receiver),
            loop_elem
        ));
        self.write("{\n");
        self.indent_push();
        // Bind arr_param (third param: array reference) if present
        if data.has_arr_param && !data.arr_param.is_empty() && data.arr_param != "_" {
            self.writeln(&format!("const {} = {};", data.arr_param, receiver));
        }
        let loop_elem_clone = loop_elem.clone();
        let var_clone = var.clone();
        self.emit_callback_body(&data.body, move |emitter, expr| {
            emitter.write("if (js_runtime.isTruthy(");
            emitter.emit_expr(expr);
            emitter.write(&format!(
                ")) {{ {}.append(js_allocator.allocator(), {}) catch @panic(\"OOM: Array.filter append\"); }}",
                var_clone, loop_elem_clone
            ));
        });
        self.indent_pop();
        self.writeln("");
        self.write("}");
        self.write(&format!(" break :{} {}; }})", blk, var));
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
            let loop_var_name = format!("_i_{}", blk);
            self.emit_reverse_loop_header(&receiver, &data.elem_param, "", &loop_var_name);
        } else {
            self.write(&format!(
                "for ({}{}) |{}| ",
                receiver,
                self.items_path(&receiver),
                data.elem_param
            ));
            self.write("{\n");
        }
        self.indent_push();
        // Bind arr_param (third param: array reference) if present
        if data.has_arr_param && !data.arr_param.is_empty() && data.arr_param != "_" {
            self.writeln(&format!("const {} = {};", data.arr_param, receiver));
        }
        self.emit_callback_body(&data.body, |emitter, expr| {
            emitter.emit_if_break_pred(expr, &blk_clone, &elem_param, false);
        });
        self.indent_pop();
        self.writeln("");

        // For JsAny arrays, use JsAny.fromUndefined() instead of `undefined`
        // to properly initialize the union type tag (P1-EM-6).
        // For Struct types, generate a zero-valued struct literal so the
        // labeled block return type matches the element type.
        let not_found: String = match &data.elem_type {
            ZigType::JsAny => "JsAny.fromUndefined()".to_string(),
            ZigType::F64 => "0.0".to_string(),
            ZigType::Bool => "false".to_string(),
            ZigType::Str => "\"\"".to_string(),
            ZigType::Struct(fields) => {
                let mut s = String::from(".{ ");
                for (i, (name, ty)) in fields.iter().enumerate() {
                    if i > 0 {
                        s.push_str(", ");
                    }
                    let default = match ty {
                        ZigType::Str => "\"\"",
                        ZigType::F64 => "0.0",
                        ZigType::Bool => "false",
                        _ => "0",
                    };
                    s.push_str(&format!(".{} = {}", name, default));
                }
                s.push_str(" }");
                s
            }
            _ => "0".to_string(),
        };
        if reverse {
            self.write(&format!("}} break :{} {}; }})", blk, not_found));
        } else {
            self.write("}");
            self.write(&format!(" break :{} {}; }})", blk, not_found));
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
        let idx_name = format!("_idx_{}", blk);
        let idx_name_clone = idx_name.clone();
        let blk_clone = blk.clone();

        if reverse {
            let loop_var_name = format!("_i_{}", blk);
            let extra = format!("const {}: i64 = @intCast({}); ", idx_name, loop_var_name);
            self.emit_reverse_loop_header(&receiver, &data.elem_param, &extra, &loop_var_name);
        } else {
            let index_name = format!("_li_{}", blk);
            self.write(&format!(
                "for ({}{}, 0..) |{}, {}| ",
                receiver,
                self.items_path(&receiver),
                data.elem_param,
                index_name
            ));
            self.write("{\n");
            self.writeln(&format!(
                "const {}: i64 = @intCast({});",
                idx_name, index_name
            ));
        }
        self.indent_push();
        // Bind arr_param (third param: array reference) if present
        if data.has_arr_param && !data.arr_param.is_empty() && data.arr_param != "_" {
            self.writeln(&format!("const {} = {};", data.arr_param, receiver));
        }
        self.emit_callback_body(&data.body, |emitter, expr| {
            emitter.emit_if_break_pred(expr, &blk_clone, &idx_name_clone, false);
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
        let var = format!("_{}_{}", var_prefix.trim_start_matches('_'), blk);
        let loop_elem_fallback = format!("_me_{}", blk);
        let elem_type_str = data.elem_type.to_zig_type();
        let loop_elem = Self::resolve_loop_elem(&data.elem_param, &loop_elem_fallback);
        let items_suffix = self.items_path(&receiver).to_string();
        let len_expr = if items_suffix.is_empty() {
            format!("{}.len", receiver)
        } else {
            format!("{}{}.len", receiver, items_suffix)
        };
        self.write(&format!(
            "var {}: std.ArrayList({}) = .empty; ",
            var, elem_type_str
        ));
        self.write(&format!(
            "{}.ensureTotalCapacity(js_allocator.allocator(), {}) catch @panic(\"OOM: Array.{} capacity\"); ",
            var, len_expr, method_name
        ));
        self.write(&format!(
            "for ({}{}) |{}| ",
            receiver, items_suffix, loop_elem
        ));
        self.write("{\n");
        self.indent_push();
        // Bind arr_param (third param: array reference) if present
        if data.has_arr_param && !data.arr_param.is_empty() && data.arr_param != "_" {
            self.writeln(&format!("const {} = {};", data.arr_param, receiver));
        }
        let var_clone = var.clone();
        let method_clone = method_name.to_string();
        let needs_jsany_wrap = matches!(data.elem_type, crate::types::ZigType::JsAny);
        self.emit_callback_body(&data.body, move |emitter, expr| {
            emitter.write(&format!("{}.append(js_allocator.allocator(), ", var_clone));
            if needs_jsany_wrap {
                emitter.write("JsAny.from(");
            }
            emitter.emit_expr(expr);
            if needs_jsany_wrap {
                emitter.write(")");
            }
            emitter.write(&format!(
                ") catch @panic(\"OOM: Array.{} append\");",
                method_clone
            ));
        });
        self.indent_pop();
        self.writeln("");
        self.write("}");
        self.write(&format!(" break :{} {}; }})", blk, var));
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
        let has_init = data.reduce_init.is_some();
        // Determine init value and accumulator type using type-aware detection
        let (init_expr_str, acc_type) = match &data.reduce_init {
            Some(expr) => {
                let ty = if super::math::expr_is_float(expr) {
                    "f64".to_string()
                } else if matches!(expr, crate::zigir::types::IrExpr::BigIntLiteral(_))
                    || matches!(
                        expr,
                        crate::zigir::types::IrExpr::TypedIdent {
                            ty: ZigType::BigInt,
                            ..
                        }
                    )
                {
                    "js_bigint.JsBigInt".to_string()
                } else if matches!(expr, crate::zigir::types::IrExpr::StringLiteral(_))
                    || matches!(
                        expr,
                        crate::zigir::types::IrExpr::TypedIdent {
                            ty: ZigType::Str,
                            ..
                        }
                    )
                {
                    "[]const u8".to_string()
                } else if data.elem_type == ZigType::F64 {
                    // P2-EM-5: Init is integer but array elements are f64 —
                    // callback produces f64 results, accumulator must be f64.
                    "f64".to_string()
                } else if matches!(data.elem_type, ZigType::JsAny) {
                    "JsAny".to_string()
                } else if data.elem_type == ZigType::Bool
                    || matches!(expr, crate::zigir::types::IrExpr::BoolLiteral(_))
                {
                    "bool".to_string()
                } else {
                    "i64".to_string()
                };
                (self.render_expr_to_string(expr), ty)
            }
            None => {
                // JS spec: no initial value → use arr[0] as accumulator, iterate from index 1
                // P0-R17: Guard against empty array (TypeError: Reduce of empty array with no initial value)
                self.write(&format!(
                    "if ({}{}.len == 0) @panic(\"TypeError: Reduce of empty array with no initial value\"); ",
                    receiver, self.items_path(&receiver)
                ));
                let ty = data.elem_type.to_zig_type().into_owned();
                (format!("{}{}[0]", receiver, self.items_path(&receiver)), ty)
            }
        };
        let init_val = if acc_type == "JsAny" && has_init {
            format!("JsAny.from({})", init_expr_str)
        } else {
            init_expr_str
        };
        self.write(&format!("var {}: {} = {}; ", acc_name, acc_type, init_val));

        // For reduce, the for-loop captures the current element.
        // The first callback param (elem_param, e.g., "acc") aliases the accumulator.
        // The second callback param (idx_param, e.g., "x") is the current element.
        // When the callback has only one param, elem_param IS the accumulator name
        // and must be bound to acc_name — we use a synthetic loop variable for the
        // element to avoid the name collision.
        let needs_index = data.reduce_idx_param.is_some();
        let (loop_var, needs_acc_bind) = if !data.idx_param.is_empty() && data.idx_param != "_" {
            // Two-param callback: use idx_param as the loop variable (current element)
            (data.idx_param.clone(), true)
        } else if data.elem_param != "_" {
            // Single-param callback: elem_param is the accumulator — use a
            // synthetic name for the element loop variable and bind elem_param
            // to the accumulator.
            (format!("_elem_{}", blk), true)
        } else {
            // No named params (both "_") — just use a throwaway loop var
            (format!("_elem_{}", blk), false)
        };

        // When the callback has a third param (index), we need the loop index.
        // Use a synthetic index var and bind reduce_idx_param to it.
        let synth_idx_var = format!("_ri_{}", blk);
        let items_path = self.items_path(&receiver).to_string();
        if has_init {
            if needs_index {
                self.write(&format!(
                    "for ({}{}, 0..) |{}, {}| ",
                    receiver, items_path, loop_var, synth_idx_var
                ));
            } else {
                self.write(&format!("for ({}{}) |{}| ", receiver, items_path, loop_var));
            }
        } else {
            // Skip index 0 (used as initial accumulator value)
            if needs_index {
                // Start from index 1, so offset the synthetic index by 1
                self.write(&format!(
                    "for ({}{}[1..], 1..) |{}, {}| ",
                    receiver, items_path, loop_var, synth_idx_var
                ));
            } else {
                self.write(&format!(
                    "for ({}{}[1..]) |{}| ",
                    receiver, items_path, loop_var
                ));
            }
        }
        self.write("{\n");
        self.indent_push();

        // Bind elem_param to the accumulator when it differs from the loop variable
        // (i.e., two-param callback where elem_param is "acc", or single-param
        // where elem_param is the accumulator and loop_var is synthetic)
        if needs_acc_bind && data.elem_param != "_" && data.elem_param != loop_var {
            self.writeln(&format!("const {} = {};", data.elem_param, acc_name));
        }

        // Bind reduce_idx_param (third param: currentIndex) if present
        if let Some(ref ri_name) = data.reduce_idx_param {
            self.writeln(&format!(
                "const {}: i64 = @intCast({});",
                ri_name, synth_idx_var
            ));
        }

        // Bind arr_param (fourth param: array reference) if present
        if data.has_arr_param && !data.arr_param.is_empty() && data.arr_param != "_" {
            self.writeln(&format!("const {} = {};", data.arr_param, receiver));
        }

        // When using a synthetic loop variable (element not named by the callback),
        // suppress Zig's "unused capture" error.
        let is_synthetic_loop_var = loop_var.starts_with("_elem_");
        if is_synthetic_loop_var {
            self.writeln(&format!("_ = &{};", loop_var));
        }

        let acc_name_clone = acc_name.clone();
        let acc_is_jsany = acc_type == "JsAny";
        self.emit_callback_body(&data.body, |emitter, expr| {
            if acc_is_jsany {
                emitter.write(&format!("{} = ", acc_name_clone));
                emitter.write("JsAny.from(");
                emitter.emit_expr(expr);
                emitter.write(");");
            } else {
                emitter.write(&format!("{} = ", acc_name_clone));
                emitter.emit_expr(expr);
                emitter.write(";");
            }
        });
        self.indent_pop();
        self.writeln("");
        self.write("}");
        self.write(&format!(" break :{} {}; }})", blk, acc_name));
    }

    // ── reduceRight ────────────────────────────────────
    //
    //  arr.reduceRight((acc, x) => acc + x, 0)
    //
    //  Same as reduce but iterates from right-to-left using the reverse loop pattern.
    //
    //  (blk_N: {
    //      var _acc_N: i64 = 0;           // accumulator + init
    //      var __i: usize = arr.items.len;
    //      while (__i > 0) {
    //          __i -= 1;
    //          const x = arr.items[__i];
    //          acc = <callback_body>;
    //      }
    //      break :blk_N acc;
    //  })
    //
    pub(super) fn emit_reduce_right_inline(
        &mut self,
        data: &crate::zigir::types::IrArrayCallbackInline,
    ) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        let blk = self.begin_labeled_block(&binding);
        let acc_name = format!("_acc_{}", self.peek_label_id());
        let has_init = data.reduce_init.is_some();
        // Determine init value and accumulator type using type-aware detection
        let (init_expr_str, acc_type) = match &data.reduce_init {
            Some(expr) => {
                let ty = if super::math::expr_is_float(expr) {
                    "f64".to_string()
                } else if matches!(expr, crate::zigir::types::IrExpr::BigIntLiteral(_))
                    || matches!(
                        expr,
                        crate::zigir::types::IrExpr::TypedIdent {
                            ty: ZigType::BigInt,
                            ..
                        }
                    )
                {
                    "js_bigint.JsBigInt".to_string()
                } else if matches!(expr, crate::zigir::types::IrExpr::StringLiteral(_))
                    || matches!(
                        expr,
                        crate::zigir::types::IrExpr::TypedIdent {
                            ty: ZigType::Str,
                            ..
                        }
                    )
                {
                    "[]const u8".to_string()
                } else if data.elem_type == ZigType::F64 {
                    // P2-EM-5: Init is integer but array elements are f64 —
                    // callback produces f64 results, accumulator must be f64.
                    "f64".to_string()
                } else if matches!(data.elem_type, ZigType::JsAny) {
                    "JsAny".to_string()
                } else if data.elem_type == ZigType::Bool
                    || matches!(expr, crate::zigir::types::IrExpr::BoolLiteral(_))
                {
                    "bool".to_string()
                } else {
                    "i64".to_string()
                };
                (self.render_expr_to_string(expr), ty)
            }
            None => {
                // JS spec: no initial value → use arr[len-1] as accumulator, iterate from len-2
                // P0-R17: Guard against empty array (TypeError: Reduce of empty array with no initial value)
                self.write(&format!(
                    "if ({}{}.len == 0) @panic(\"TypeError: Reduce of empty array with no initial value\"); ",
                    receiver, self.items_path(&receiver)
                ));
                let ty = data.elem_type.to_zig_type().into_owned();
                (
                    format!(
                        "{}{}[{}{}.len - 1]",
                        receiver,
                        self.items_path(&receiver),
                        receiver,
                        self.items_path(&receiver)
                    ),
                    ty,
                )
            }
        };
        let init_val = if acc_type == "JsAny" && has_init {
            format!("JsAny.from({})", init_expr_str)
        } else {
            init_expr_str
        };
        self.write(&format!("var {}: {} = {}; ", acc_name, acc_type, init_val));

        // Same single/duo param logic as reduce: for single-param callbacks,
        // elem_param is the accumulator — use a synthetic element loop var.
        let blk_for_var = blk.clone();
        let (loop_var, needs_acc_bind) = if !data.idx_param.is_empty() && data.idx_param != "_" {
            (data.idx_param.clone(), true)
        } else if data.elem_param != "_" {
            (format!("_elem_{}", blk_for_var), true)
        } else {
            (format!("_elem_{}", blk_for_var), false)
        };
        // Reverse loop header: var __i: usize = receiver.items.len; while (__i > 0) { __i -= 1; const loop_var = receiver.items[__i];
        // When no initial value: start from len-2 (last element is the initial accumulator)
        let loop_var_name = format!("_i_{}", blk);
        if has_init {
            self.emit_reverse_loop_header(&receiver, &loop_var, "", &loop_var_name);
        } else {
            self.write(&format!(
                "var {}: usize = {}{}.len - 1; while ({} > 0) {{ {} -= 1; const {} = {}{}[{}]; ",
                loop_var_name,
                receiver,
                self.items_path(&receiver),
                loop_var_name,
                loop_var_name,
                loop_var,
                receiver,
                self.items_path(&receiver),
                loop_var_name
            ));
        }
        self.indent_push();

        // Bind elem_param to the accumulator when it differs from the loop variable
        if needs_acc_bind && data.elem_param != "_" && data.elem_param != loop_var {
            self.writeln(&format!("const {} = {};", data.elem_param, acc_name));
        }

        // Bind reduce_idx_param (third param: currentIndex) if present.
        // For reduceRight, the loop variable (_i_N) is the current index.
        if let Some(ref ri_name) = data.reduce_idx_param {
            self.writeln(&format!(
                "const {}: i64 = @intCast({});",
                ri_name, loop_var_name
            ));
        }

        // Bind arr_param (fourth param: array reference) if present
        if data.has_arr_param && !data.arr_param.is_empty() && data.arr_param != "_" {
            self.writeln(&format!("const {} = {};", data.arr_param, receiver));
        }

        // When using a synthetic loop variable (element not named by the callback),
        // suppress Zig's "unused capture" error.
        let is_synthetic_loop_var = loop_var.starts_with("_elem_");
        if is_synthetic_loop_var {
            self.writeln(&format!("_ = &{};", loop_var));
        }

        let acc_name_clone = acc_name.clone();
        let acc_is_jsany = acc_type == "JsAny";
        self.emit_callback_body(&data.body, |emitter, expr| {
            if acc_is_jsany {
                emitter.write(&format!("{} = ", acc_name_clone));
                emitter.write("JsAny.from(");
                emitter.emit_expr(expr);
                emitter.write(");");
            } else {
                emitter.write(&format!("{} = ", acc_name_clone));
                emitter.emit_expr(expr);
                emitter.write(";");
            }
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
            // R33: When binding (chained receiver), emit sort in a labeled block
            // without nesting another labeled block. The pattern:
            //   blk: { const __chain_N = <expr>; std.mem.sort(...); break :blk __chain_N; }
            let items_target = format!("{}{}", receiver, self.items_path(&receiver));
            let blk = self.next_label();
            self.write(&format!("{}: {{ ", blk));
            self.write(b);
            self.emit_sort_less_than(
                &items_target,
                &elem_type_str,
                &param_a,
                &param_b,
                &data.body,
            );
            self.write(&format!(" break :{} {}; }}", blk, receiver));
        } else {
            let items_target = format!("{}{}", receiver, self.items_path(&receiver));
            let blk = self.next_label();
            self.write(&format!("({}: {{ ", blk));
            self.emit_sort_less_than(
                &items_target,
                &elem_type_str,
                &param_a,
                &param_b,
                &data.body,
            );
            self.write(&format!(" break :{} {}; }})", blk, receiver));
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
        let var = format!("_sorted_{}", blk);
        let items_ref = format!("{}.items", var);
        let recv_items = format!("{}{}", receiver, self.items_path(&receiver));
        self.write(&format!(
            "var {}: std.ArrayList({}) = .empty; ",
            var, elem_type_str
        ));
        self.write(&format!(
            "{}.appendSlice(js_allocator.allocator(), {}) catch @panic(\"OOM: Array.toSorted appendSlice\"); ",
            var, recv_items
        ));

        self.emit_sort_less_than(&items_ref, &elem_type_str, &param_a, &param_b, &data.body);

        self.write(&format!(" break :{} {}; }})", blk, var));
    }

    /// Emit the header for a reverse iteration loop:
    ///   `var <lv>: usize = <receiver>.items.len; while (<lv> > 0) { <lv> -= 1; const <elem> = <receiver>.items[<lv>]; <extra>`
    /// Used by findLast and findLastIndex. The `loop_var` parameter must be a
    /// unique name (e.g., derived from `blk` via `format!("_i_{}", blk)`) to
    /// avoid shadowing errors when nested (R29-EMIT-2).
    fn emit_reverse_loop_header(
        &mut self,
        receiver: &str,
        elem_param: &str,
        extra: &str,
        loop_var: &str,
    ) {
        let items = self.items_path(receiver);
        let len_expr = if items.is_empty() {
            format!("{}.len", receiver)
        } else {
            format!("{}.items.len", receiver)
        };
        let idx_expr = if items.is_empty() {
            format!("{}[{}]", receiver, loop_var)
        } else {
            format!("{}.items[{}]", receiver, loop_var)
        };
        self.write(&format!(
            "var {}: usize = {}; while ({} > 0) {{ {} -= 1; const {} = {}; {}",
            loop_var, len_expr, loop_var, loop_var, elem_param, idx_expr, extra
        ));
    }

    /// When `elem_param` is `"_"` (discard), substitute `fallback` as a real Zig identifier.
    /// Zig's `_` is a discard, not an identifier, so we can't use it in `.append(_, _)` calls.
    fn resolve_loop_elem(elem_param: &str, fallback: &str) -> String {
        if elem_param == "_" {
            fallback.to_string()
        } else {
            elem_param.to_string()
        }
    }

    /// Resolve shared sort/toSorted parameters: (elem_type_str, param_a, param_b).
    fn resolve_sort_params(
        &mut self,
        data: &crate::zigir::types::IrArrayCallbackInline,
    ) -> (String, String, String) {
        let elem_type_str = data.elem_type.to_zig_type().into_owned();
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

        self.write(" } }.lessThan);");
    }

    // ── flatMap ───────────────────────────────────────
    //
    //  arr.flatMap(fn) → map + flatten(depth=1).
    //  Known limitation: since our type system uses uniform element types
    //  (ArrayList(i64), etc.), we cannot distinguish at compile time whether
    //  the callback returns an array (which should be flattened) or a scalar
    //  (which should be appended as-is). A proper fix would require runtime
    //  type checking on each callback result. For now, flatMap delegates to
    //  emit_collect_inline (same as map), which appends each result as a
    //  single element — meaning array results are NOT flattened.
    //  Delegates to emit_collect_inline with __fmap prefix.

    pub(super) fn emit_flat_map_inline(
        &mut self,
        data: &crate::zigir::types::IrArrayCallbackInline,
    ) {
        self.emit_collect_inline(data, "__fmap", "flatMap");
    }
}
