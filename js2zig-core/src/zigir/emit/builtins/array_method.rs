// zigir/emit/builtins/array_method.rs
// Array non-callback method inlining (includes, indexOf, join, slice, splice, etc.).

use crate::types::ZigType;
use crate::zigir::emit::helpers::EmitterHelpers;

use crate::zigir::emit::Emitter;

// ═══════════════════════════════════════════════════════
//  Array non-callback method inlining
// ═══════════════════════════════════════════════════════

impl Emitter {
    /// Emit insert-items block shared by splice/toSpliced: if args.len() > 2,
    /// collect args[2..] as rendered strings and emit `target.insertSlice(allocator, start_var, &[_]elem_type{ items }) catch @panic(...)`.
    fn emit_splice_insert(
        &mut self,
        target: &str,
        start_var: &str,
        elem_type_str: &str,
        args: &[crate::zigir::types::IrExpr],
        method_name: &str,
    ) {
        if args.len() > 2 {
            let insert_items: Vec<String> = args[2..]
                .iter()
                .map(|arg| self.render_expr_to_string(arg))
                .collect();
            self.write(&format!(
                "{}.insertSlice(js_allocator.allocator(), {}, &[_]{}{{ {} }}) catch @panic(\"OOM: Array.{} insert\"); ",
                target, start_var, elem_type_str, insert_items.join(", "), method_name
            ));
        }
    }

    /// Emit the start-index and delete-count computation shared by splice/toSpliced.
    /// R16: Negative start uses JS from-end conversion instead of @max(0, start).
    /// Writes: `const {start_var} = if (start < 0) max(0, len+start) else min(start, len); const {cnt_var} = min(max(0, count), len - start);  `
    fn emit_splice_start_count(
        &mut self,
        start_var: &str,
        cnt_var: &str,
        receiver: &str,
        args: &[crate::zigir::types::IrExpr],
    ) {
        // Emit start as isize const first to avoid double evaluation
        self.write(&format!("const {}_raw: isize = @intCast(", start_var));
        if let Some(arg) = args.first() {
            self.emit_i64_coerced(arg);
        } else {
            self.write("0");
        }
        self.write(&format!(
            "); const __spl_len = {}.items.len; const {}: usize = @intCast(if ({}_raw < 0) @max(0, @as(isize, @intCast(__spl_len)) + {}_raw) else @min(@as(usize, @intCast({}_raw)), __spl_len)); ",
            receiver, start_var, start_var, start_var, start_var
        ));
        self.write(&format!(
            "const {}: usize = @intCast(@min(@max(0, ",
            cnt_var
        ));
        if args.len() >= 2 {
            self.emit_i64_coerced(&args[1]);
        } else {
            self.write("0");
        }
        self.write(&format!("), {}.items.len -| {})); ", receiver, start_var));
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
    // For string: delegate to js_string.includes (UTF-16 aware) or inline byte search.
    // For i64 arrays: (blk: { for (obj.items) |item| { if (item == target) break :blk true; } break :blk false; })
    // R16-P2-EM #1: support fromIndex parameter (2nd arg).
    // R16-P2-EM #2: Str path uses begin_labeled_block to emit chain binding.
    pub(super) fn emit_includes_inline(&mut self, data: &crate::zigir::types::IrArrayMethodInline) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);
        let has_from = data.args.len() >= 2;

        // If the array is a string type, use std.mem.indexOf or runtime
        if matches!(data.elem_type, ZigType::Str) {
            let blk = self.begin_labeled_block(&binding);
            if has_from {
                // With fromIndex: delegate to runtime for UTF-16 semantics
                self.write(&format!("break :{} js_string.includes(", blk));
                self.write(&receiver);
                self.write(", ");
                if let Some(arg) = data.args.first() {
                    self.emit_expr(arg);
                }
                self.write(", ");
                self.emit_expr(&data.args[1]);
                self.write("); })");
            } else {
                // No fromIndex: fast inline byte search
                self.write(&format!("break :{} (std.mem.indexOf(u8, ", blk));
                self.write(&receiver);
                self.write(", ");
                if let Some(arg) = data.args.first() {
                    self.emit_expr(arg);
                }
                self.write(") != null); })");
            }
        } else if has_from {
            // Array path with fromIndex: clamp to [0, len] and iterate from start
            let blk = self.begin_labeled_block(&binding);
            self.write("const __from: isize = @intCast(");
            self.emit_i64_coerced(&data.args[1]);
            self.write(&format!(
                "); const __len = {}.items.len; const __start: usize = @intCast(if (__from < 0) 0 else if (@as(usize, @intCast(__from)) > __len) __len else @as(usize, @intCast(__from))); var __i: usize = __start; while (__i < __len) : (__i += 1) {{ if ({}.items[__i] == ",
                receiver, receiver
            ));
            if let Some(arg) = data.args.first() {
                self.emit_expr(arg);
            }
            self.write(&format!(
                ") break :{} true; }} break :{} false; }})",
                blk, blk
            ));
        } else {
            // Array path without fromIndex: original for-loop behavior
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
    // For string: js_string.indexOf (UTF-16 aware) or inline byte search.
    // For i64: (blk: { for (obj.items, 0..) |item, i| { if (item == target) break :blk @as(i64, @intCast(i)); } break :blk @as(i64, -1); })
    // R16-P2-EM #1: support fromIndex parameter (2nd arg).
    // R16-P2-EM #2: Str path uses begin_labeled_block to emit chain binding.
    pub(super) fn emit_index_of_inline(&mut self, data: &crate::zigir::types::IrArrayMethodInline) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);
        let has_from = data.args.len() >= 2;

        if matches!(data.elem_type, ZigType::Str) {
            let blk = self.begin_labeled_block(&binding);
            if has_from {
                // With fromIndex: delegate to runtime for UTF-16 semantics
                self.write(&format!("break :{} js_string.indexOf(", blk));
                self.write(&receiver);
                self.write(", ");
                if let Some(arg) = data.args.first() {
                    self.emit_expr(arg);
                }
                self.write(", ");
                self.emit_expr(&data.args[1]);
                self.write("); })");
            } else {
                // No fromIndex: fast inline byte search
                self.write(&format!("break :{} (if (std.mem.indexOf(u8, ", blk));
                self.write(&receiver);
                self.write(", ");
                if let Some(arg) = data.args.first() {
                    self.emit_expr(arg);
                }
                self.write(")) |idx| @as(i64, @intCast(idx)) else @as(i64, -1)); })");
            }
        } else if has_from {
            // Array path with fromIndex: clamp to [0, len] and iterate from start
            let blk = self.begin_labeled_block(&binding);
            self.write("const __from: isize = @intCast(");
            self.emit_i64_coerced(&data.args[1]);
            self.write(&format!(
                "); const __len = {}.items.len; const __start: usize = @intCast(if (__from < 0) 0 else if (@as(usize, @intCast(__from)) > __len) __len else @as(usize, @intCast(__from))); var __i: usize = __start; while (__i < __len) : (__i += 1) {{ if ({}.items[__i] == ",
                receiver, receiver
            ));
            if let Some(arg) = data.args.first() {
                self.emit_expr(arg);
            }
            self.write(&format!(
                ") break :{} @as(i64, @intCast(__i)); }} break :{} @as(i64, -1); }})",
                blk, blk
            ));
        } else {
            // Array path without fromIndex: original for-loop behavior
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
    // (blk: { var __i: isize = ...; while (__i >= 0) : (__i -= 1) { if (obj.items[__i] == target) break :blk @as(i64, __i); } break :blk @as(i64, -1); })
    // R16-P2-EM #1: support fromIndex parameter (2nd arg).
    //   - fromIndex >= len → search from len-1 (entire array)
    //   - fromIndex < 0    → search from len+fromIndex (clamps to -1 → not found)
    pub(super) fn emit_last_index_of_inline(
        &mut self,
        data: &crate::zigir::types::IrArrayMethodInline,
    ) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        let blk = self.begin_labeled_block(&binding);
        if data.args.len() >= 2 {
            // With fromIndex: compute start position per JS spec
            self.write("const __from: isize = @intCast(");
            self.emit_i64_coerced(&data.args[1]);
            self.write(&format!(
                "); const __len = {}.items.len; var __i: isize = if (__from < 0) @as(isize, @intCast(__len)) + __from else @min(__from, @as(isize, @intCast(__len)) - 1); while (__i >= 0) : (__i -= 1) {{ if ({}.items[@as(usize, @intCast(__i))] == ",
                receiver, receiver
            ));
        } else {
            // Default: search entire array from end
            self.write(&format!(
                "var __i: isize = @as(isize, @intCast({}.items.len)) - 1; while (__i >= 0) : (__i -= 1) {{ if ({}.items[@as(usize, @intCast(__i))] == ",
                receiver, receiver
            ));
        }
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
        // I64→{d}, F64→{} (shortest round-trip; R8-E2), Bool→{}, Str→{s}, other→{any}
        let fmt_spec = match data.elem_type {
            ZigType::I64 => "{d}",
            ZigType::F64 => "{}",
            ZigType::Bool => "{}",
            ZigType::Str => "{s}",
            _ => "{any}",
        };
        let blk = self.begin_labeled_block(&binding);
        self.write("var __join_buf: std.ArrayList(u8) = .empty; ");
        self.write(&format!("for ({}.items, 0..) |__item, __i| ", receiver));
        self.write("{\n");
        self.indent_push();
        // Emit separator as a Zig expression directly — not embedded inside a
        // string literal. This avoids double-quoting for StringLiteral args
        // (e.g. join("-") should appendSlice("-") not appendSlice("\"-\"")) and
        // correctly handles variable separators (join(sep) → appendSlice(sep)).
        self.write("if (__i > 0) __join_buf.appendSlice(js_allocator.allocator(), ");
        if let Some(arg) = data.args.first() {
            self.emit_expr(arg);
        } else {
            self.write("\",\"");
        }
        self.writeln(&format!(") catch break :{} \"\";", blk));
        if matches!(data.elem_type, ZigType::F64) {
            self.writeln(&format!(
                "__join_buf.appendSlice(js_allocator.allocator(), js_number.toString(js_allocator.allocator(), __item, 10) catch break :{} \"\") catch break :{} \"\";",
                blk, blk
            ));
        } else if matches!(data.elem_type, ZigType::Str) {
            self.writeln(&format!(
                "__join_buf.appendSlice(js_allocator.allocator(), __item) catch break :{} \"\";",
                blk
            ));
        } else {
            self.writeln(&format!(
                "{{ const __s = std.fmt.allocPrint(js_allocator.allocator(), \"{}\", .{{__item}}) catch break :{} \"\"; __join_buf.appendSlice(js_allocator.allocator(), __s) catch break :{} \"\"; js_allocator.allocator().free(__s); }}",
                fmt_spec, blk, blk
            ));
        }
        self.indent_pop();
        self.writeln("");
        self.write("}");
        self.write(&format!(
            " break :{} __join_buf.toOwnedSlice(js_allocator.allocator()) catch \"\"; }})",
            blk
        ));
    }

    // ── slice ──────────────────────────────────────────
    // (blk: { var __slice: std.ArrayList(elem_type) = .empty;
    //   const __len = obj.items.len; const __s = if (start < 0) max(0, len+start) else min(start, len);
    //   const __e = if (end < 0) max(0, len+end) else min(end, len);
    //   __slice.appendSlice(allocator, obj.items[__s..__e]) catch @panic("OOM");
    //   break :blk __slice; })
    pub(super) fn emit_slice_inline(&mut self, data: &crate::zigir::types::IrArrayMethodInline) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        let blk = self.begin_labeled_block(&binding);
        let elem_type_str = data.elem_type.to_zig_type();
        self.write(&format!(
            "var __slice: std.ArrayList({}) = .empty; ",
            elem_type_str
        ));

        // R16: Handle negative indices via JS from-end conversion.
        match data.args.len() {
            0 => {
                self.write(&format!(
                    "__slice.appendSlice(js_allocator.allocator(), {}.items) catch @panic(\"OOM: Array.slice appendSlice\"); ",
                    receiver
                ));
            }
            1 => {
                // slice(start): store start in a const, compute from-end
                self.write("const __slice_start: isize = @intCast(");
                self.emit_i64_coerced(&data.args[0]);
                self.write(&format!(
                    "); const __len = {}.items.len; const __s: usize = @intCast(if (__slice_start < 0) @max(0, @as(isize, @intCast(__len)) + __slice_start) else @min(@as(usize, @intCast(__slice_start)), __len)); ",
                    receiver
                ));
                self.write(&format!(
                    "__slice.appendSlice(js_allocator.allocator(), {}.items[__s..]) catch @panic(\"OOM: Array.slice appendSlice\"); ",
                    receiver
                ));
            }
            _ => {
                // slice(start, end): store both, compute from-end
                self.write("const __slice_start: isize = @intCast(");
                self.emit_i64_coerced(&data.args[0]);
                self.write("); const __slice_end: isize = @intCast(");
                self.emit_i64_coerced(&data.args[1]);
                self.write(&format!(
                    "); const __len = {}.items.len; const __s: usize = @intCast(if (__slice_start < 0) @max(0, @as(isize, @intCast(__len)) + __slice_start) else @min(@as(usize, @intCast(__slice_start)), __len)); const __e: usize = @intCast(if (__slice_end < 0) @max(0, @as(isize, @intCast(__len)) + __slice_end) else @min(@as(usize, @intCast(__slice_end)), __len)); ",
                    receiver
                ));
                // Clamp end to start: when end < start, slice returns empty array (JS spec).
                self.write(&format!(
                    "__slice.appendSlice(js_allocator.allocator(), {}.items[__s..@max(__s, __e)]) catch @panic(\"OOM: Array.slice appendSlice\"); ",
                    receiver
                ));
            }
        }
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
        self.emit_splice_insert(&receiver, "__start", &elem_type_str, &data.args, "splice");
        self.write(&format!("break :{} __spliced; }})", blk));
    }

    // ── at ─────────────────────────────────────────────
    // Returns undefined for out-of-range indices (per JS spec).
    pub(super) fn emit_at_inline(&mut self, data: &crate::zigir::types::IrArrayMethodInline) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        let blk = self.begin_labeled_block(&binding);
        self.write("const __idx: isize = @intCast(");
        if let Some(arg) = data.args.first() {
            self.emit_i64_coerced(arg);
        } else {
            self.write("0");
        }
        self.write("); ");
        self.write(&format!(
            "const __at_idx = if (__idx < 0) @as(usize, @intCast(@as(isize, @intCast({}.items.len)) + @as(isize, @intCast(__idx)))) else @as(usize, @intCast(__idx)); ",
            receiver
        ));
        // Bounds check: return undefined if out of range (P0-R13-1 fix).
        // Previously returned items[0] which is wrong and panics on empty arrays.
        // For JsAny arrays, use JsAny.fromUndefined() to properly initialize the
        // union; `undefined` leaves the union's type tag uninitialized (P1-EM-6).
        let not_found = match data.elem_type {
            ZigType::JsAny => "JsAny.fromUndefined()",
            ZigType::F64 => "0.0",
            ZigType::Bool => "false",
            ZigType::Str => "\"\"",
            _ => "0",
        };
        self.write(&format!(
            "break :{} if (__at_idx >= {}.items.len) {} else {}.items[__at_idx]; }})",
            blk, receiver, not_found, receiver
        ));
    }

    // ── concat ─────────────────────────────────────────
    // For each arg, checks if it has .items (is an array); if so, appendSlice,
    // otherwise append as a single element (P0-5 fix: handle non-array args).
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
            // Use comptime type equality to distinguish array args from scalars.
            // @hasField fails (compile error) on primitive types like i64/f64/bool.
            self.write("{ const __ca = ");
            self.emit_expr(arg);
            self.write(&format!(
                "; if (@TypeOf(__ca) == std.ArrayList({})) {{ ",
                elem_type_str
            ));
            self.write("__concat.appendSlice(js_allocator.allocator(), __ca.items) catch @panic(\"OOM: Array.concat\"); ");
            self.write("} else { ");
            if matches!(data.elem_type, ZigType::JsAny) {
                self.write("__concat.append(js_allocator.allocator(), JsAny.from(__ca)) catch @panic(\"OOM: Array.concat\"); ");
            } else {
                self.write("__concat.append(js_allocator.allocator(), __ca) catch @panic(\"OOM: Array.concat\"); ");
            }
            self.write("} } ");
        }
        self.write(&format!("break :{} __concat; }})", blk));
    }

    // ── copyWithin ─────────────────────────────────────
    // Copies a sequence of elements within the array. Uses reverse copy
    // when target > start to handle overlapping regions correctly (P0-4 fix).
    // R16: Negative target/start/end use JS from-end conversion.
    pub(super) fn emit_copy_within_inline(
        &mut self,
        data: &crate::zigir::types::IrArrayMethodInline,
    ) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        let blk = self.begin_labeled_block(&binding);
        // Emit target, start, end as isize consts for from-end conversion
        self.write("const __cpw_target_raw: isize = @intCast(");
        if let Some(arg) = data.args.first() {
            self.emit_i64_coerced(arg);
        } else {
            self.write("0");
        }
        self.write("); const __cpw_start_raw: isize = @intCast(");
        if data.args.len() >= 2 {
            self.emit_i64_coerced(&data.args[1]);
        } else {
            self.write("0");
        }
        self.write("); const __cpw_end_raw: isize = @intCast(");
        if data.args.len() >= 3 {
            self.emit_i64_coerced(&data.args[2]);
        } else {
            self.write(&format!("@as(i64, @intCast({}.items.len))", receiver));
        }
        // Convert negative indices via from-end
        self.write(&format!(
            "); const __len = {}.items.len; const __cpw_target: usize = @intCast(if (__cpw_target_raw < 0) @max(0, @as(isize, @intCast(__len)) + __cpw_target_raw) else @min(@as(usize, @intCast(__cpw_target_raw)), __len)); const __cpw_start: usize = @intCast(if (__cpw_start_raw < 0) @max(0, @as(isize, @intCast(__len)) + __cpw_start_raw) else @min(@as(usize, @intCast(__cpw_start_raw)), __len)); const __cpw_end: usize = @intCast(if (__cpw_end_raw < 0) @max(0, @as(isize, @intCast(__len)) + __cpw_end_raw) else @min(@as(usize, @intCast(__cpw_end_raw)), __len)); ",
            receiver
        ));
        // Use existing copyWithin logic with forward/backward copy based on overlap.
        // Saturating subtraction: when end < start, count = 0 (no-op per JS spec).
        self.write("const __cpw_cnt = __cpw_end -| __cpw_start; ");
        self.write("if (__cpw_cnt > 0) { ");
        // Use reverse copy when target > start to avoid overwriting source
        self.write(&format!(
            "if (__cpw_target > __cpw_start) {{ var __j: usize = @as(usize, @intCast(__cpw_cnt)); while (__j > 0) {{ __j -= 1; {}.items[__cpw_target + __j] = {}.items[__cpw_start + __j]; }} }} else {{ for (0..@as(usize, @intCast(__cpw_cnt))) |__j| {{ {}.items[__cpw_target + __j] = {}.items[__cpw_start + __j]; }} }} }} ",
            receiver, receiver, receiver, receiver
        ));
        self.write(&format!("break :{} {}; }})", blk, receiver));
    }

    // ── fill ───────────────────────────────────────────
    // for (obj.items[@intCast(start)..@intCast(end)]) |*elem| { elem.* = val; }
    // Returns receiver for JS chaining semantics (arr.fill(v) === arr).
    // R16: Negative start/end use JS from-end conversion.
    // Wraps in labeled block so the const binding is scoped and receiver is returned.
    pub(super) fn emit_fill_inline(&mut self, data: &crate::zigir::types::IrArrayMethodInline) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        let blk = self.begin_labeled_block(&binding);
        match data.args.len() {
            0 => {
                // fill entire array — no value arg, use type-appropriate default.
                // JS fill(undefined) sets all elements to undefined.
                let default_val = match data.elem_type {
                    ZigType::F64 => "0.0",
                    ZigType::Bool => "false",
                    ZigType::Str => "\"\"",
                    ZigType::JsAny => "JsAny.fromUndefined()",
                    _ => "0",
                };
                self.write(&format!(
                    "for ({}.items) |*elem| {{ elem.* = {}; }}",
                    receiver, default_val
                ));
            }
            1 => {
                // fill(value) — fill entire array
                self.write(&format!("for ({}.items) |*elem| {{ elem.* = ", receiver));
                self.emit_expr(&data.args[0]);
                self.write("; }");
            }
            2 => {
                // fill(value, start) — with negative index support
                self.write("const __fill_start: isize = @intCast(");
                self.emit_i64_coerced(&data.args[1]);
                self.write(&format!(
                    "); const __len = {}.items.len; const __fs: usize = @intCast(if (__fill_start < 0) @max(0, @as(isize, @intCast(__len)) + __fill_start) else @min(@as(usize, @intCast(__fill_start)), __len)); ",
                    receiver
                ));
                self.write(&format!(
                    "for ({}.items[__fs..]) |*elem| {{ elem.* = ",
                    receiver
                ));
                self.emit_expr(&data.args[0]);
                self.write("; }");
            }
            _ => {
                // fill(value, start, end) — with negative index support
                self.write("const __fill_start: isize = @intCast(");
                self.emit_i64_coerced(&data.args[1]);
                self.write("); const __fill_end: isize = @intCast(");
                self.emit_i64_coerced(&data.args[2]);
                self.write(&format!(
                    "); const __len = {}.items.len; const __fs: usize = @intCast(if (__fill_start < 0) @max(0, @as(isize, @intCast(__len)) + __fill_start) else @min(@as(usize, @intCast(__fill_start)), __len)); const __fe: usize = @intCast(if (__fill_end < 0) @max(0, @as(isize, @intCast(__len)) + __fill_end) else @min(@as(usize, @intCast(__fill_end)), __len)); ",
                    receiver
                ));
                // Guard: when end < start, fill is a no-op per JS spec.
                self.write(&format!(
                    "if (__fe > __fs) {{ for ({}.items[__fs..__fe]) |*elem| {{ elem.* = ",
                    receiver
                ));
                self.emit_expr(&data.args[0]);
                self.write("; } }");
            }
        }
        self.write(&format!(" break :{} {}; }})", blk, receiver));
    }

    // ── with ───────────────────────────────────────────
    // arr.with(index, value) → clone the array, replace element at index
    // R16: Negative index uses JS from-end conversion.
    // (blk: { var __with: std.ArrayList(elem_type) = .empty;
    //   __with.appendSlice(allocator, obj.items) catch @panic("OOM");
    //   const __with_idx = if (idx < 0) max(0, len+idx) else min(idx, len);
    //   __with.items[__with_idx] = value;
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
        // Compute index with from-end conversion for negative indices
        self.write("const __with_raw: isize = @intCast(");
        if let Some(idx_arg) = data.args.first() {
            self.emit_i64_coerced(idx_arg);
        } else {
            self.write("0");
        }
        self.write(
            "); const __with_len = __with.items.len; const __with_idx: usize = @intCast(if (__with_raw < 0) @max(0, @as(isize, @intCast(__with_len)) + __with_raw) else @min(@as(usize, @intCast(__with_raw)), __with_len)); "
        );
        // JS spec: with() throws RangeError for out-of-range index.
        self.write("if (__with_idx >= __with.items.len) @panic(\"RangeError: Invalid array index for Array.with()\"); ");
        self.write("__with.items[__with_idx] = ");
        if data.args.len() >= 2 {
            self.emit_expr(&data.args[1]);
        } else {
            match data.elem_type {
                ZigType::JsAny => self.write("JsAny.fromUndefined()"),
                ZigType::F64 => self.write("0.0"),
                ZigType::Bool => self.write("false"),
                ZigType::Str => self.write("\"\""),
                _ => self.write("0"),
            }
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
        // Sort — for JsAny elements use JsAny.lt(); for i64/f64 use ECMA-262
        // string comparison (format as strings, compare lexicographically);
        // other primitive types fall back to numeric std.sort.asc.
        if matches!(data.elem_type, ZigType::JsAny) {
            self.write("std.mem.sort(JsAny, __sorted.items, {}, struct { fn lessThan(_: void, a: JsAny, b: JsAny) bool { return a.lt(b); } }.lessThan); ");
        } else if matches!(data.elem_type, ZigType::I64 | ZigType::F64) {
            self.write(&format!(
                "std.mem.sort({}, __sorted.items, {{}}, struct {{ fn lessThan(_: void, a: {}, b: {}) bool {{",
                elem_type_str, elem_type_str, elem_type_str
            ));
            self.write(" var __sa: [64]u8 = undefined; var __sb: [64]u8 = undefined;");
            self.write(
                " const __stra = std.fmt.bufPrint(&__sa, \"{d}\", .{a}) catch return a < b;",
            );
            self.write(
                " const __strb = std.fmt.bufPrint(&__sb, \"{d}\", .{b}) catch return a < b;",
            );
            self.write(" return std.mem.order(u8, __stra, __strb) == .lt; } }.lessThan); ");
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
        self.emit_splice_insert(
            "__sp",
            "__sp_start",
            &elem_type_str,
            &data.args,
            "toSpliced",
        );
        self.write(&format!("break :{} __sp; }})", blk));
    }
}
