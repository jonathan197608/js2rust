// zigir/emit/builtins/array_method.rs
// Array non-callback method inlining (includes, indexOf, join, slice, splice, etc.).

use crate::types::ZigType;
use crate::zigir::emit::helpers::EmitterHelpers;

use crate::zigir::emit::Emitter;

// ═══════════════════════════════════════════════════════
//  Array non-callback method inlining
// ═══════════════════════════════════════════════════════

impl Emitter {
    /// Emit the start-index and delete-count computation shared by splice/toSpliced.
    /// Writes: `const {start_var} = @as(usize, @intCast(@max(0, start_expr)));  const {cnt_var} = @as(usize, @intCast(@min(@max(0, count_expr), {receiver}.items.len -| {start_var})));  `
    fn emit_splice_start_count(
        &mut self,
        start_var: &str,
        cnt_var: &str,
        receiver: &str,
        args: &[crate::zigir::types::IrExpr],
    ) {
        self.write(&format!(
            "const {} = @as(usize, @intCast(@max(0, ",
            start_var
        ));
        if let Some(arg) = args.first() {
            self.emit_expr(arg);
        } else {
            self.write("0");
        }
        self.write("))); ");
        self.write(&format!(
            "const {} = @as(usize, @intCast(@min(@max(0, ",
            cnt_var
        ));
        if args.len() >= 2 {
            self.emit_expr(&args[1]);
        } else {
            self.write("0");
        }
        self.write(&format!("), {}.items.len -| {}))); ", receiver, start_var));
    }

    /// Emit an inlined array non-callback method as a Zig block expression or
    /// statement. This handles inline patterns for includes,
    /// indexOf, lastIndexOf, join, slice, splice, at, concat, copyWithin, fill.
    pub(crate) fn emit_array_method_inline(
        &mut self,
        data: &crate::zigir::types::IrArrayMethodInline,
    ) {
        use crate::zigir::types::ArrayMethodKind as K;

        match data.kind {
            K::Includes => self.emit_includes_inline(data),
            K::IndexOf => self.emit_index_of_inline(data),
            K::LastIndexOf => self.emit_last_index_of_inline(data),
            K::Join => self.emit_join_inline(data),
            K::Slice => self.emit_slice_inline(data),
            K::Splice => self.emit_splice_inline(data),
            K::At => self.emit_at_inline(data),
            K::Concat => self.emit_concat_inline(data),
            K::CopyWithin => self.emit_copy_within_inline(data),
            K::Fill => self.emit_fill_inline(data),
            K::With => self.emit_with_inline(data),
            K::ToReversed => self.emit_to_reversed_inline(data),
            K::ToSorted => self.emit_to_sorted_inline(data),
            K::ToSpliced => self.emit_to_spliced_inline(data),
        }
    }

    // ── includes ───────────────────────────────────────
    // For string arrays: (std.mem.indexOf(u8, obj, target) != null)
    // For i64 arrays: (blk: { for (obj.items) |item| { if (item == target) break :blk true; } break :blk false; })
    pub(super) fn emit_includes_inline(&mut self, data: &crate::zigir::types::IrArrayMethodInline) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        // If the array is a string type, use std.mem.indexOf
        if matches!(data.elem_type, ZigType::Str) {
            self.write("(std.mem.indexOf(u8, ");
            self.write(&receiver);
            self.write(", ");
            if let Some(arg) = data.args.first() {
                self.emit_expr(arg);
            }
            self.write(") != null)");
        } else {
            let blk = self.begin_labeled_block(&binding);
            self.write(&format!("for ({}.items) |item| ", receiver));
            self.write("{\n");
            self.indent_push();
            self.writeln("if (item == ");
            if let Some(arg) = data.args.first() {
                self.emit_expr(arg);
            }
            self.write(&format!(") break :{} true;", blk));
            self.indent_pop();
            self.writeln("");
            self.write("}");
            self.write(&format!(" break :{} false; }})", blk));
        }
    }

    // ── indexOf ────────────────────────────────────────
    // For string: (if (std.mem.indexOf(u8, obj, target)) |idx| @as(i64, @intCast(idx)) else @as(i64, -1))
    // For i64: (blk: { for (obj.items, 0..) |item, i| { if (item == target) break :blk @as(i64, @intCast(i)); } break :blk @as(i64, -1); })
    pub(super) fn emit_index_of_inline(&mut self, data: &crate::zigir::types::IrArrayMethodInline) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        if matches!(data.elem_type, ZigType::Str) {
            self.write("(if (std.mem.indexOf(u8, ");
            self.write(&receiver);
            self.write(", ");
            if let Some(arg) = data.args.first() {
                self.emit_expr(arg);
            }
            self.write(")) |idx| @as(i64, @intCast(idx)) else @as(i64, -1))");
        } else {
            let blk = self.begin_labeled_block(&binding);
            self.write(&format!("for ({}.items, 0..) |item, i| ", receiver));
            self.write("{\n");
            self.indent_push();
            self.writeln("if (item == ");
            if let Some(arg) = data.args.first() {
                self.emit_expr(arg);
            }
            self.write(&format!(") break :{} @as(i64, @intCast(i));", blk));
            self.indent_pop();
            self.writeln("");
            self.write("}");
            self.write(&format!(" break :{} @as(i64, -1); }})", blk));
        }
    }

    // ── lastIndexOf ────────────────────────────────────
    // (blk: { var __i: isize = @as(isize, @intCast(obj.items.len)) - 1; while (__i >= 0) : (__i -= 1) { if (obj.items[@as(usize, @intCast(__i))] == target) break :blk @as(i64, __i); } break :blk @as(i64, -1); })
    pub(super) fn emit_last_index_of_inline(
        &mut self,
        data: &crate::zigir::types::IrArrayMethodInline,
    ) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        let blk = self.begin_labeled_block(&binding);
        self.write(&format!(
            "var __i: isize = @as(isize, @intCast({}.items.len)) - 1; while (__i >= 0) : (__i -= 1) {{ if ({}.items[@as(usize, @intCast(__i))] == ",
            receiver, receiver
        ));
        if let Some(arg) = data.args.first() {
            self.emit_expr(arg);
        }
        self.write(&format!(
            ") break :{} @as(i64, __i); }} break :{} @as(i64, -1); }})",
            blk, blk
        ));
    }

    // ── join ───────────────────────────────────────────
    // (blk: { var __join_buf = std.io.Writer.Allocating.init(js_allocator.allocator());
    //   for (obj.items, 0..) |__item, __i| { if (__i > 0) __join_buf.writer().writeAll(sep) catch break :blk "";
    //     __join_buf.writer().print("{d}", .{__item}) catch break :blk ""; }
    //   break :blk __join_buf.toOwnedSlice() catch ""; })
    pub(super) fn emit_join_inline(&mut self, data: &crate::zigir::types::IrArrayMethodInline) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        // Format specifier based on element type:
        // I64/F64 → {d}, Bool → {}, Str → {s}, other → {any}
        let fmt_spec = match data.elem_type {
            ZigType::I64 => "{d}",
            ZigType::F64 => "{d:.15}",
            ZigType::Bool => "{}",
            ZigType::Str => "{s}",
            _ => "{any}",
        };
        let blk = self.begin_labeled_block(&binding);
        self.write("var __join_buf = std.io.Writer.Allocating.init(js_allocator.allocator()); ");
        self.write(&format!("for ({}.items, 0..) |__item, __i| ", receiver));
        self.write("{\n");
        self.indent_push();
        let sep = if let Some(arg) = data.args.first() {
            self.render_expr_to_string(arg)
        } else {
            ",".to_string()
        };
        self.writeln(&format!(
            "if (__i > 0) __join_buf.writer().writeAll(\"{}\") catch break :{} \"\";",
            sep.replace('\\', "\\\\").replace('"', "\\\""),
            blk
        ));
        self.writeln(&format!(
            "__join_buf.writer().print(\"{}\", .{{__item}}) catch break :{} \"\";",
            fmt_spec, blk
        ));
        self.indent_pop();
        self.writeln("");
        self.write("}");
        self.write(&format!(
            " break :{} __join_buf.toOwnedSlice() catch \"\"; }})",
            blk
        ));
    }

    // ── slice ──────────────────────────────────────────
    // (blk: { var __slice: std.ArrayList(elem_type) = .empty;
    //   __slice.appendSlice(js_allocator.allocator(), obj.items[start..end]) catch @panic("OOM");
    //   break :blk __slice; })
    pub(super) fn emit_slice_inline(&mut self, data: &crate::zigir::types::IrArrayMethodInline) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        let blk = self.begin_labeled_block(&binding);
        let elem_type_str = data.elem_type.to_zig_type();
        self.write(&format!(
            "var __slice: std.ArrayList({}) = .empty; ",
            elem_type_str
        ));

        // Build the slice expression: obj.items, obj.items[start..], or obj.items[start..end]
        self.write("__slice.appendSlice(js_allocator.allocator(), ");
        self.write(&receiver);
        self.write(".items");
        match data.args.len() {
            0 => {}
            1 => {
                self.write("[");
                self.emit_expr(&data.args[0]);
                self.write("..]");
            }
            _ => {
                self.write("[");
                self.emit_expr(&data.args[0]);
                self.write("..");
                self.emit_expr(&data.args[1]);
                self.write("]");
            }
        }
        self.write(") catch @panic(\"OOM: Array.slice appendSlice\"); ");
        self.write(&format!("break :{} __slice; }})", blk));
    }

    // ── splice ─────────────────────────────────────────
    // (blk: { var __spliced: std.ArrayList(elem_type) = .empty;
    //   const __start = @as(usize, @intCast(@max(0, start)));
    //   const __cnt = @as(usize, @intCast(@min(@max(0, count), obj.items.len -| __start)));
    //   var __i: usize = 0; while (__i < __cnt) : (__i += 1) { __spliced.append(allocator, obj.orderedRemove(__start)) catch @panic("OOM"); }
    //   [insert items if args > 2]
    //   break :blk __spliced; })
    pub(super) fn emit_splice_inline(&mut self, data: &crate::zigir::types::IrArrayMethodInline) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        let blk = self.begin_labeled_block(&binding);
        let elem_type_str = data.elem_type.to_zig_type();
        self.write(&format!(
            "var __spliced: std.ArrayList({}) = .empty; ",
            elem_type_str
        ));
        self.emit_splice_start_count("__start", "__cnt", &receiver, &data.args);
        self.write("var __i: usize = 0; while (__i < __cnt) : (__i += 1) { ");
        self.write(&format!(
            "__spliced.append(js_allocator.allocator(), {}.orderedRemove(__start)) catch @panic(\"OOM: Array.splice\"); }} ",
            receiver
        ));
        // Insert items if provided (args beyond start and count)
        // Use insertSlice for batch insertion
        if data.args.len() > 2 {
            let insert_items: Vec<String> = data.args[2..]
                .iter()
                .map(|arg| self.render_expr_to_string(arg))
                .collect();
            self.write(&format!(
                "{}.insertSlice(js_allocator.allocator(), __start, &[_]{}{{ {} }}) catch @panic(\"OOM: Array.splice insert\"); ",
                receiver, elem_type_str, insert_items.join(", ")
            ));
        }
        self.write(&format!("break :{} __spliced; }})", blk));
    }

    // ── at ─────────────────────────────────────────────
    // (blk: { const __idx = arg; const __at_idx = if (__idx < 0) @as(usize, @intCast(@as(isize, @intCast(obj.items.len)) + @as(isize, @intCast(__idx)))) else @as(usize, @intCast(__idx)); break :blk obj.items[__at_idx]; })
    pub(super) fn emit_at_inline(&mut self, data: &crate::zigir::types::IrArrayMethodInline) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        let blk = self.begin_labeled_block(&binding);
        self.write("const __idx = ");
        if let Some(arg) = data.args.first() {
            self.emit_expr(arg);
        } else {
            self.write("0");
        }
        self.write("; ");
        self.write(&format!(
            "const __at_idx = if (__idx < 0) @as(usize, @intCast(@as(isize, @intCast({}.items.len)) + @as(isize, @intCast(__idx)))) else @as(usize, @intCast(__idx)); ",
            receiver
        ));
        self.write(&format!("break :{} {}.items[__at_idx]; }})", blk, receiver));
    }

    // ── concat ─────────────────────────────────────────
    // (blk: { var __concat: std.ArrayList(elem_type) = .empty;
    //   __concat.appendSlice(allocator, obj.items) catch @panic("OOM");
    //   [for each arg:] __concat.appendSlice(allocator, arg.items) catch @panic("OOM");
    //   break :blk __concat; })
    pub(super) fn emit_concat_inline(&mut self, data: &crate::zigir::types::IrArrayMethodInline) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        let blk = self.begin_labeled_block(&binding);
        let elem_type_str = data.elem_type.to_zig_type();
        self.write(&format!(
            "var __concat: std.ArrayList({}) = .empty; ",
            elem_type_str
        ));
        self.write(&format!(
            "__concat.appendSlice(js_allocator.allocator(), {}.items) catch @panic(\"OOM: Array.concat appendSlice\"); ",
            receiver
        ));
        for arg in &data.args {
            self.write("__concat.appendSlice(js_allocator.allocator(), ");
            self.emit_expr(arg);
            self.write(".items) catch @panic(\"OOM: Array.concat appendSlice\"); ");
        }
        self.write(&format!("break :{} __concat; }})", blk));
    }

    // ── copyWithin ─────────────────────────────────────
    // Simplified: for (obj.items[@intCast(start)..@intCast(end)]) |*elem, i| { elem.* = obj.items[@intCast(target) + i]; }
    // Full version has reverse copy logic when target > start.
    // For now, emit a simpler forward-only version.
    pub(super) fn emit_copy_within_inline(
        &mut self,
        data: &crate::zigir::types::IrArrayMethodInline,
    ) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        let blk = self.begin_labeled_block(&binding);
        self.write("const __cpw_target = @as(isize, @intCast(");
        if let Some(arg) = data.args.first() {
            self.emit_expr(arg);
        } else {
            self.write("0");
        }
        self.write(")); ");
        self.write("const __cpw_start = @as(isize, @intCast(");
        if data.args.len() >= 2 {
            self.emit_expr(&data.args[1]);
        } else {
            self.write("0");
        }
        self.write(")); ");
        self.write("const __cpw_end = @as(isize, @intCast(");
        if data.args.len() >= 3 {
            self.emit_expr(&data.args[2]);
        } else {
            self.write(&format!("@as(i64, @intCast({}.items.len))", receiver));
        }
        self.write(")); ");
        self.write("const __cpw_cnt = __cpw_end - __cpw_start; ");
        self.write("if (__cpw_cnt > 0) { ");
        self.write("const __src = @as(usize, @intCast(__cpw_start)); const __dst = @as(usize, @intCast(__cpw_target)); ");
        self.write(&format!(
            "for (0..@as(usize, @intCast(__cpw_cnt))) |__j| {{ {}.items[__dst + __j] = {}.items[__src + __j]; }} }} ",
            receiver, receiver
        ));
        self.write(&format!("break :{} &{}; }})", blk, receiver));
    }

    // ── fill ───────────────────────────────────────────
    // for (obj.items[@intCast(start)..@intCast(end)]) |*elem| { elem.* = val; }
    // When chaining, wraps in a block so the const binding is scoped.
    pub(super) fn emit_fill_inline(&mut self, data: &crate::zigir::types::IrArrayMethodInline) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        // When chaining, wrap in a block so the const binding is scoped.
        if let Some(b) = &binding {
            self.write("{ ");
            self.write(b);
        }
        self.write(&format!("for ({}.items", receiver));
        match data.args.len() {
            0 => {
                // fill entire array
                self.write(") |*elem| { elem.* = ");
                self.write("undefined"); // no value arg
                self.write("; }");
            }
            1 => {
                // fill(value) — fill entire array
                self.write(") |*elem| { elem.* = ");
                self.emit_expr(&data.args[0]);
                self.write("; }");
            }
            2 => {
                // fill(value, start)
                self.write("[@intCast(");
                self.emit_expr(&data.args[1]);
                self.write(")..]) |*elem| { elem.* = ");
                self.emit_expr(&data.args[0]);
                self.write("; }");
            }
            _ => {
                // fill(value, start, end)
                self.write("[@intCast(");
                self.emit_expr(&data.args[1]);
                self.write(")..@intCast(");
                self.emit_expr(&data.args[2]);
                self.write(")]) |*elem| { elem.* = ");
                self.emit_expr(&data.args[0]);
                self.write("; }");
            }
        }
        if binding.is_some() {
            self.write(" }");
        }
    }

    // ── with ───────────────────────────────────────────
    // arr.with(index, value) → clone the array, replace element at index
    // (blk: { var __with: std.ArrayList(elem_type) = .empty;
    //   __with.appendSlice(allocator, obj.items) catch @panic("OOM");
    //   __with.items[@intCast(idx)] = value;
    //   break :blk __with; })
    pub(super) fn emit_with_inline(&mut self, data: &crate::zigir::types::IrArrayMethodInline) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        let blk = self.begin_labeled_block(&binding);
        let elem_type_str = data.elem_type.to_zig_type();
        self.write(&format!(
            "var __with: std.ArrayList({}) = .empty; ",
            elem_type_str
        ));
        self.write(&format!(
            "__with.appendSlice(js_allocator.allocator(), {}.items) catch @panic(\"OOM: Array.with appendSlice\"); ",
            receiver
        ));
        // Replace element at index
        if let Some(idx_arg) = data.args.first() {
            self.write("__with.items[@intCast(");
            self.emit_expr(idx_arg);
            self.write(")] = ");
        } else {
            self.write("__with.items[0] = ");
        }
        if data.args.len() >= 2 {
            self.emit_expr(&data.args[1]);
        } else {
            self.write("undefined");
        }
        self.write("; ");
        self.write(&format!("break :{} __with; }})", blk));
    }

    // ── toReversed ─────────────────────────────────────
    // arr.toReversed() → clone the array in reverse order
    // (blk: { var __rev: std.ArrayList(elem_type) = .empty;
    //   __rev.ensureTotalCapacity(allocator, obj.items.len) catch @panic("OOM");
    //   var __ri = obj.items.len; while (__ri > 0) { __ri -= 1; __rev.append(allocator, obj.items[__ri]) catch @panic("OOM"); }
    //   break :blk __rev; })
    pub(super) fn emit_to_reversed_inline(
        &mut self,
        data: &crate::zigir::types::IrArrayMethodInline,
    ) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        let blk = self.begin_labeled_block(&binding);
        let elem_type_str = data.elem_type.to_zig_type();
        self.write(&format!(
            "var __rev: std.ArrayList({}) = .empty; ",
            elem_type_str
        ));
        self.write(&format!(
            "__rev.ensureTotalCapacity(js_allocator.allocator(), {}.items.len) catch @panic(\"OOM: Array.toReversed capacity\"); ",
            receiver
        ));
        self.write(&format!(
            "var __ri: usize = {}.items.len; while (__ri > 0) {{ __ri -= 1; __rev.append(js_allocator.allocator(), {}.items[__ri]) catch @panic(\"OOM: Array.toReversed append\"); }} ",
            receiver, receiver
        ));
        self.write(&format!("break :{} __rev; }})", blk));
    }

    // ── toSorted ───────────────────────────────────────
    // arr.toSorted(compareFn?) → clone the array, sort the clone
    // No compareFn path: default ascending sort. With compareFn, dispatched via
    // ArrayCallbackKind::ToSorted in array_callback.rs instead.
    // (blk: { var __sorted: std.ArrayList(elem_type) = .empty;
    //   __sorted.appendSlice(allocator, obj.items) catch @panic("OOM");
    //   std.mem.sort(elem_type, __sorted.items, {}, comptime std.sort.asc(elem_type));
    //   break :blk __sorted; })
    pub(super) fn emit_to_sorted_inline(
        &mut self,
        data: &crate::zigir::types::IrArrayMethodInline,
    ) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        let blk = self.begin_labeled_block(&binding);
        let elem_type_str = data.elem_type.to_zig_type();
        self.write(&format!(
            "var __sorted: std.ArrayList({}) = .empty; ",
            elem_type_str
        ));
        self.write(&format!(
            "__sorted.appendSlice(js_allocator.allocator(), {}.items) catch @panic(\"OOM: Array.toSorted appendSlice\"); ",
            receiver
        ));
        // Sort — for JsAny elements use JsAny.lt(); for i64 use numeric sort
        if matches!(data.elem_type, ZigType::JsAny) {
            self.write("std.mem.sort(JsAny, __sorted.items, {}, struct { fn lessThan(_: void, a: JsAny, b: JsAny) bool { return a.lt(b); } }.lessThan); ");
        } else {
            self.write(&format!(
                "std.mem.sort({}, __sorted.items, {{}}, comptime std.sort.asc({})); ",
                elem_type_str, elem_type_str
            ));
        }
        self.write(&format!("break :{} __sorted; }})", blk));
    }

    // ── toSpliced ──────────────────────────────────────
    // arr.toSpliced(start, deleteCount?, ...items) → clone, then splice the clone
    // (blk: { var __sp: std.ArrayList(elem_type) = .empty;
    //   __sp.appendSlice(allocator, obj.items) catch @panic("OOM");
    //   const __sp_start = @as(usize, @intCast(@max(0, start)));
    //   const __sp_cnt = @as(usize, @intCast(@min(@max(0, count), __sp.items.len -| __sp_start)));
    //   var __j: usize = 0; while (__j < __sp_cnt) : (__j += 1) { _ = __sp.orderedRemove(__sp_start); }
    //   [insert items if args > 2]
    //   break :blk __sp; })
    pub(super) fn emit_to_spliced_inline(
        &mut self,
        data: &crate::zigir::types::IrArrayMethodInline,
    ) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        let blk = self.begin_labeled_block(&binding);
        let elem_type_str = data.elem_type.to_zig_type();
        self.write(&format!(
            "var __sp: std.ArrayList({}) = .empty; ",
            elem_type_str
        ));
        // Clone original array
        self.write(&format!(
            "__sp.appendSlice(js_allocator.allocator(), {}.items) catch @panic(\"OOM: Array.toSpliced appendSlice\"); ",
            receiver
        ));
        // Compute start index and delete count
        self.emit_splice_start_count("__sp_start", "__sp_cnt", &receiver, &data.args);
        // Remove elements from clone
        self.write("var __j: usize = 0; while (__j < __sp_cnt) : (__j += 1) { _ = __sp.orderedRemove(__sp_start); } ");
        // Insert items if provided (args beyond start and deleteCount)
        if data.args.len() > 2 {
            let insert_items: Vec<String> = data.args[2..]
                .iter()
                .map(|arg| self.render_expr_to_string(arg))
                .collect();
            self.write(&format!(
                "__sp.insertSlice(js_allocator.allocator(), __sp_start, &[_]{}{{ {} }}) catch @panic(\"OOM: Array.toSpliced insert\"); ",
                elem_type_str, insert_items.join(", ")
            ));
        }
        self.write(&format!("break :{} __sp; }})", blk));
    }
}
