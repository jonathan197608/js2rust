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
        let len_var = format!("{}_len", start_var);
        self.write(&format!(
            "); const {} = {}.items.len; const {}: usize = @intCast(if ({}_raw < 0) @max(0, @as(isize, @intCast({})) + {}_raw) else @min(@as(usize, @intCast({}_raw)), {})); ",
            len_var, receiver, start_var, start_var, len_var, start_var, start_var, len_var
        ));
        self.write(&format!(
            "const {}: usize = @intCast(@min(@max(0, ",
            cnt_var
        ));
        if args.len() >= 2 {
            self.emit_i64_coerced(&args[1]);
        } else {
            // ECMA-262: splice(start) with no deleteCount -> len - start
            self.write(&format!("{}.items.len -| {}", receiver, start_var));
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
            let (_from, _len, _start, _i) = (
                format!("_fr_{}", blk),
                format!("_ln_{}", blk),
                format!("_st_{}", blk),
                format!("_i_{}", blk),
            );
            self.write(&format!("const {}: isize = @intCast(", _from));
            self.emit_i64_coerced(&data.args[1]);
            self.write(&format!(
                "); const {} = {}.items.len; const {}: usize = @intCast(if ({} < 0) @max(0, @as(isize, @intCast({})) + {}) else @min(@as(usize, @intCast({})), {})); var {}: usize = {}; while ({} < {}) : ({} += 1) {{ if ({}.items[{}] == ",
                _len, receiver, _start, _from, _len, _from, _from, _len, _i, _start, _i, _len, _i, receiver, _i
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
    pub(super) fn emit_index_of_inline(&mut self, data: &crate::zigir::types::IrArrayMethodInline) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);
        let has_from = data.args.len() >= 2;

        if matches!(data.elem_type, ZigType::Str) {
            let blk = self.begin_labeled_block(&binding);
            if has_from {
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
                self.write(&format!("break :{} (if (std.mem.indexOf(u8, ", blk));
                self.write(&receiver);
                self.write(", ");
                if let Some(arg) = data.args.first() {
                    self.emit_expr(arg);
                }
                self.write(")) |idx| @as(i64, @intCast(idx)) else @as(i64, -1)); })");
            }
        } else if has_from {
            let blk = self.begin_labeled_block(&binding);
            let (_from, _len, _start, _i) = (
                format!("_fr_{}", blk),
                format!("_ln_{}", blk),
                format!("_st_{}", blk),
                format!("_i_{}", blk),
            );
            self.write(&format!("const {}: isize = @intCast(", _from));
            self.emit_i64_coerced(&data.args[1]);
            self.write(&format!(
                "); const {} = {}.items.len; const {}: usize = @intCast(if ({} < 0) @max(0, @as(isize, @intCast({})) + {}) else @min(@as(usize, @intCast({})), {})); var {}: usize = {}; while ({} < {}) : ({} += 1) {{ if ({}.items[{}] == ",
                _len, receiver, _start, _from, _len, _from, _from, _len, _i, _start, _i, _len, _i, receiver, _i
            ));
            if let Some(arg) = data.args.first() {
                self.emit_expr(arg);
            }
            self.write(&format!(
                ") break :{} @as(i64, @intCast({})); }} break :{} @as(i64, -1); }})",
                blk, _i, blk
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
    pub(super) fn emit_last_index_of_inline(
        &mut self,
        data: &crate::zigir::types::IrArrayMethodInline,
    ) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        let blk = self.begin_labeled_block(&binding);
        if data.args.len() >= 2 {
            // With fromIndex: compute start position per JS spec
            let (_from, _len, _i) = (
                format!("_fr_{}", blk),
                format!("_ln_{}", blk),
                format!("_i_{}", blk),
            );
            self.write(&format!("const {}: isize = @intCast(", _from));
            self.emit_i64_coerced(&data.args[1]);
            self.write(&format!(
                "); const {} = {}.items.len; var {}: isize = if ({} < 0) @as(isize, @intCast({})) + {} else @min({}, @as(isize, @intCast({})) - 1); while ({} >= 0) : ({} -= 1) {{ if ({}.items[@as(usize, @intCast({}))] == ",
                _len, receiver, _i, _from, _len, _from, _from, _len, _i, _i, receiver, _i
            ));
        } else {
            // Default: search entire array from end
            let _i = format!("_i_{}", blk);
            self.write(&format!(
                "var {}: isize = @as(isize, @intCast({}.items.len)) - 1; while ({} >= 0) : ({} -= 1) {{ if ({}.items[@as(usize, @intCast({}))] == ",
                _i, receiver, _i, _i, receiver, _i
            ));
        }
        if let Some(arg) = data.args.first() {
            self.emit_expr(arg);
        }
        let _li = format!("_i_{}", blk);
        self.write(&format!(
            ") break :{} @as(i64, {}); }} break :{} @as(i64, -1); }})",
            blk, _li, blk
        ));
    }

    // ── join ───────────────────────────────────────────
    pub(super) fn emit_join_inline(&mut self, data: &crate::zigir::types::IrArrayMethodInline) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        // Format specifier based on element type:
        // I64->{d}, F64->{} (shortest round-trip; R8-E2), Bool->{}, Str->{s},
        // JsAny->{f} (only {f} dispatches to format() in Zig 0.16), other->{any}
        let fmt_spec = match data.elem_type {
            ZigType::I64 => "{d}",
            ZigType::F64 => "{}",
            ZigType::Bool => "{}",
            ZigType::Str => "{s}",
            ZigType::JsAny => "{f}",
            _ => "{any}",
        };
        let blk = self.begin_labeled_block(&binding);
        let (_jb, _je, _ji, _js) = (
            format!("_jb_{}", blk),
            format!("_je_{}", blk),
            format!("_ji_{}", blk),
            format!("_js_{}", blk),
        );
        self.write(&format!(
            "var {}: std.ArrayList(u8) = .empty; defer {}.deinit(js_allocator.allocator()); ",
            _jb, _jb
        ));
        // R32-4: Use receiver directly for slice receivers (from split()),
        // or receiver.items for ArrayList receivers.
        let items_access = if data.receiver_is_slice {
            receiver.clone()
        } else {
            format!("{}.items", receiver)
        };
        self.write(&format!("for ({}, 0..) |{}, {}| ", items_access, _je, _ji));
        self.write("{\n");
        self.indent_push();
        self.write(&format!(
            "if ({} > 0) {}.appendSlice(js_allocator.allocator(), ",
            _ji, _jb
        ));
        if let Some(arg) = data.args.first() {
            self.emit_expr(arg);
        } else {
            self.write("\",\"");
        }
        self.writeln(&format!(") catch break :{} \"\";", blk));
        if matches!(data.elem_type, ZigType::F64) {
            self.writeln(&format!(
                "{}.appendSlice(js_allocator.allocator(), js_number.toString(js_allocator.allocator(), {}, 10) catch break :{} \"\") catch break :{} \"\";",
                _jb, _je, blk, blk
            ));
        } else if matches!(data.elem_type, ZigType::Str) {
            self.writeln(&format!(
                "{}.appendSlice(js_allocator.allocator(), {}) catch break :{} \"\";",
                _jb, _je, blk
            ));
        } else {
            self.writeln(&format!(
                "{{ const {} = std.fmt.allocPrint(js_allocator.allocator(), \"{}\", .{{{}}}) catch break :{} \"\"; {}.appendSlice(js_allocator.allocator(), {}) catch break :{} \"\"; js_allocator.allocator().free({}); }}",
                _js, fmt_spec, _je, blk, _jb, _js, blk, _js
            ));
        }
        self.indent_pop();
        self.writeln("");
        self.write("}");
        self.write(&format!(
            " break :{} {}.toOwnedSlice(js_allocator.allocator()) catch \"\"; }})",
            blk, _jb
        ));
    }

    // ── slice ──────────────────────────────────────────
    pub(super) fn emit_slice_inline(&mut self, data: &crate::zigir::types::IrArrayMethodInline) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        let blk = self.begin_labeled_block(&binding);
        let elem_type_str = data.elem_type.to_zig_type();
        let (_sl, _ss, _se, _ln, _s, _e) = (
            format!("_sl_{}", blk),
            format!("_ss_{}", blk),
            format!("_se_{}", blk),
            format!("_ln_{}", blk),
            format!("_s_{}", blk),
            format!("_e_{}", blk),
        );
        self.write(&format!(
            "var {}: std.ArrayList({}) = .empty; ",
            _sl, elem_type_str
        ));

        // R16: Handle negative indices via JS from-end conversion.
        match data.args.len() {
            0 => {
                self.write(&format!(
                    "{}.appendSlice(js_allocator.allocator(), {}.items) catch @panic(\"OOM: Array.slice appendSlice\"); ",
                    _sl, receiver
                ));
            }
            1 => {
                // slice(start): store start in a const, compute from-end
                self.write(&format!("const {}: isize = @intCast(", _ss));
                self.emit_i64_coerced(&data.args[0]);
                self.write(&format!(
                    "); const {} = {}.items.len; const {}: usize = @intCast(if ({} < 0) @max(0, @as(isize, @intCast({})) + {}) else @min(@as(usize, @intCast({})), {})); ",
                    _ln, receiver, _s, _ss, _ln, _ss, _ss, _ln
                ));
                self.write(&format!(
                    "{}.appendSlice(js_allocator.allocator(), {}.items[{}..]) catch @panic(\"OOM: Array.slice appendSlice\"); ",
                    _sl, receiver, _s
                ));
            }
            _ => {
                // slice(start, end): store both, compute from-end
                self.write(&format!("const {}: isize = @intCast(", _ss));
                self.emit_i64_coerced(&data.args[0]);
                self.write(&format!("); const {}: isize = @intCast(", _se));
                self.emit_i64_coerced(&data.args[1]);
                self.write(&format!(
                    "); const {} = {}.items.len; const {}: usize = @intCast(if ({} < 0) @max(0, @as(isize, @intCast({})) + {}) else @min(@as(usize, @intCast({})), {})); const {}: usize = @intCast(if ({} < 0) @max(0, @as(isize, @intCast({})) + {}) else @min(@as(usize, @intCast({})), {})); ",
                    _ln, receiver, _s, _ss, _ln, _ss, _ss, _ln, _e, _se, _ln, _se, _se, _ln
                ));
                // Clamp end to start: when end < start, slice returns empty array (JS spec).
                self.write(&format!(
                    "{}.appendSlice(js_allocator.allocator(), {}.items[{}..@max({}, {})]) catch @panic(\"OOM: Array.slice appendSlice\"); ",
                    _sl, receiver, _s, _s, _e
                ));
            }
        }
        self.write(&format!("break :{} {}; }})", blk, _sl));
    }

    // ── splice ─────────────────────────────────────────
    pub(super) fn emit_splice_inline(&mut self, data: &crate::zigir::types::IrArrayMethodInline) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        let blk = self.begin_labeled_block(&binding);
        let elem_type_str = data.elem_type.to_zig_type();
        let (_sp, _st, _cnt, _i) = (
            format!("_sp_{}", blk),
            format!("_st_{}", blk),
            format!("_cnt_{}", blk),
            format!("_si_{}", blk),
        );
        self.write(&format!(
            "var {}: std.ArrayList({}) = .empty; ",
            _sp, elem_type_str
        ));
        self.emit_splice_start_count(&_st, &_cnt, &receiver, &data.args);
        self.write(&format!(
            "var {}: usize = 0; while ({} < {}) : ({} += 1) {{ ",
            _i, _i, _cnt, _i
        ));
        self.write(&format!(
            "{}.append(js_allocator.allocator(), {}.orderedRemove({})) catch @panic(\"OOM: Array.splice\"); }} ",
            _sp, receiver, _st
        ));
        // Insert items if provided (args beyond start and count)
        self.emit_splice_insert(&receiver, &_st, &elem_type_str, &data.args, "splice");
        self.write(&format!("break :{} {}; }})", blk, _sp));
    }
    // ── at ─────────────────────────────────────────────
    // Returns undefined for out-of-range indices (per JS spec).
    pub(super) fn emit_at_inline(&mut self, data: &crate::zigir::types::IrArrayMethodInline) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        let blk = self.begin_labeled_block(&binding);
        let inner_blk = self.next_label();
        let (_idx, _ai, _w) = (
            format!("_idx_{}", blk),
            format!("_ai_{}", blk),
            format!("_aw_{}", blk),
        );
        self.write(&format!("const {}: isize = @intCast(", _idx));
        if let Some(arg) = data.args.first() {
            self.emit_i64_coerced(arg);
        } else {
            self.write("0");
        }
        self.write("); ");
        self.write(&format!(
            "const {} = if ({} < 0) {}: {{ const {} = @as(isize, @intCast({}.items.len)) + {}; break :{} if ({} < 0) {}.items.len else @as(usize, @intCast({})); }} else @as(usize, @intCast({})); ",
            _ai, _idx, inner_blk, _w, receiver, _idx, inner_blk, _w, receiver, _w, _idx
        ));
        // Bounds check: return undefined if out of range (per JS spec).
        // at() can always return undefined for OOB, so we wrap the result
        // in JsAny for all element types. This ensures console.log prints
        // "undefined" instead of a type-specific default (0, 0.0, false, "").
        let (not_found, elem_access): (&str, String) = match data.elem_type {
            ZigType::JsAny => (
                "JsAny.fromUndefined()",
                format!("{}.items[{}]", receiver, _ai),
            ),
            ZigType::F64 => (
                "JsAny.fromUndefined()",
                format!("JsAny.fromF64({}.items[{}])", receiver, _ai),
            ),
            ZigType::Bool => (
                "JsAny.fromUndefined()",
                format!("JsAny.fromBool({}.items[{}])", receiver, _ai),
            ),
            ZigType::Str => (
                "JsAny.fromUndefined()",
                format!("JsAny.fromString({}.items[{}])", receiver, _ai),
            ),
            _ => (
                "JsAny.fromUndefined()",
                format!("JsAny.fromI64({}.items[{}])", receiver, _ai),
            ),
        };
        self.write(&format!(
            "break :{} if ({} >= {}.items.len) {} else {}; }})",
            blk, _ai, receiver, not_found, elem_access
        ));
    }

    // ── concat ─────────────────────────────────────────
    pub(super) fn emit_concat_inline(&mut self, data: &crate::zigir::types::IrArrayMethodInline) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        let blk = self.begin_labeled_block(&binding);
        let elem_type_str = data.elem_type.to_zig_type();
        let (_cc, _ca) = (format!("_cc_{}", blk), format!("_ca_{}", blk));
        self.write(&format!(
            "var {}: std.ArrayList({}) = .empty; ",
            _cc, elem_type_str
        ));
        self.write(&format!(
            "{}.appendSlice(js_allocator.allocator(), {}.items) catch @panic(\"OOM: Array.concat appendSlice\"); ",
            _cc, receiver
        ));
        for arg in &data.args {
            self.write(&format!("{{ const {} = ", _ca));
            self.emit_expr(arg);
            self.write(&format!(
                "; if (@TypeOf({}) == std.ArrayList({})) {{ ",
                _ca, elem_type_str
            ));
            self.write(&format!("{}.appendSlice(js_allocator.allocator(), {}.items) catch @panic(\"OOM: Array.concat\"); ", _cc, _ca));
            self.write("} else { ");
            if matches!(data.elem_type, ZigType::JsAny) {
                self.write(&format!("{}.append(js_allocator.allocator(), JsAny.from({})) catch @panic(\"OOM: Array.concat\"); ", _cc, _ca));
            } else {
                // elem_type is a scalar (i64/f64/bool/str). The arg may be
                // a JsAny (e.g. from a variable or function return) or a
                // compatible scalar. If arg is JsAny, extract the scalar
                // via the appropriate asXxx() method; otherwise use @as to
                // coerce compatible types.
                let extract = match &data.elem_type {
                    ZigType::I64 => "asI64()",
                    ZigType::F64 => "asF64()",
                    ZigType::Bool => "asBool()",
                    ZigType::Str => "asString(js_allocator.allocator())",
                    _ => "asI64()",
                };
                self.write(&format!(
                    "{cc}.append(js_allocator.allocator(), if (@TypeOf({ca}) == JsAny) {ca}.{ex} else @as({et}, {ca})) catch @panic(\"OOM: Array.concat\"); ",
                    cc = _cc, ca = _ca, ex = extract, et = elem_type_str
                ));
            }
            self.write("} } ");
        }
        self.write(&format!("break :{} {}; }})", blk, _cc));
    }

    // ── copyWithin ─────────────────────────────────────
    pub(super) fn emit_copy_within_inline(
        &mut self,
        data: &crate::zigir::types::IrArrayMethodInline,
    ) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        let blk = self.begin_labeled_block(&binding);
        let (_tr, _sr, _er, _ln, _tg, _cs, _ce, _cc, _j) = (
            format!("_tr_{}", blk),
            format!("_sr_{}", blk),
            format!("_er_{}", blk),
            format!("_ln_{}", blk),
            format!("_tg_{}", blk),
            format!("_cs_{}", blk),
            format!("_ce_{}", blk),
            format!("_cc_{}", blk),
            format!("_j_{}", blk),
        );
        // Emit target, start, end as isize consts for from-end conversion
        self.write(&format!("const {}: isize = @intCast(", _tr));
        if let Some(arg) = data.args.first() {
            self.emit_i64_coerced(arg);
        } else {
            self.write("0");
        }
        self.write(&format!("); const {}: isize = @intCast(", _sr));
        if data.args.len() >= 2 {
            self.emit_i64_coerced(&data.args[1]);
        } else {
            self.write("0");
        }
        self.write(&format!("); const {}: isize = @intCast(", _er));
        if data.args.len() >= 3 {
            self.emit_i64_coerced(&data.args[2]);
        } else {
            self.write(&format!("@as(i64, @intCast({}.items.len))", receiver));
        }
        // Convert negative indices via from-end
        self.write(&format!(
            "); const {} = {}.items.len; const {}: usize = @intCast(if ({} < 0) @max(0, @as(isize, @intCast({})) + {}) else @min(@as(usize, @intCast({})), {})); const {}: usize = @intCast(if ({} < 0) @max(0, @as(isize, @intCast({})) + {}) else @min(@as(usize, @intCast({})), {})); const {}: usize = @intCast(if ({} < 0) @max(0, @as(isize, @intCast({})) + {}) else @min(@as(usize, @intCast({})), {})); ",
            _ln, receiver, _tg, _tr, _ln, _tr, _tr, _ln,
            _cs, _sr, _ln, _sr, _sr, _ln,
            _ce, _er, _ln, _er, _er, _ln
        ));
        // Use existing copyWithin logic with forward/backward copy based on overlap.
        self.write(&format!("const {} = {} -| {}; ", _cc, _ce, _cs));
        self.write(&format!("if ({} > 0) {{ ", _cc));
        // Use reverse copy when target > start to avoid overwriting source
        self.write(&format!(
            "if ({} > {}) {{ var {}: usize = @as(usize, @intCast({})); while ({} > 0) {{ {} -= 1; {}.items[{} + {}] = {}.items[{} + {}]; }} }} else {{ for (0..@as(usize, @intCast({}))) |{}| {{ {}.items[{} + {}] = {}.items[{} + {}]; }} }} }} ",
            _tg, _cs, _j, _cc, _j, _j, receiver, _tg, _j, receiver, _cs, _j, _cc, _j, receiver, _tg, _j, receiver, _cs, _j
        ));
        self.write(&format!("break :{} {}; }})", blk, receiver));
    }
    // ── fill ───────────────────────────────────────────
    pub(super) fn emit_fill_inline(&mut self, data: &crate::zigir::types::IrArrayMethodInline) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        let blk = self.begin_labeled_block(&binding);
        match data.args.len() {
            0 => {
                // fill entire array — no value arg, use type-appropriate default.
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
                let (_fsr, _ln, _fs) = (
                    format!("_fsr_{}", blk),
                    format!("_ln_{}", blk),
                    format!("_fs_{}", blk),
                );
                self.write(&format!("const {}: isize = @intCast(", _fsr));
                self.emit_i64_coerced(&data.args[1]);
                self.write(&format!(
                    "); const {} = {}.items.len; const {}: usize = @intCast(if ({} < 0) @max(0, @as(isize, @intCast({})) + {}) else @min(@as(usize, @intCast({})), {})); ",
                    _ln, receiver, _fs, _fsr, _ln, _fsr, _fsr, _ln
                ));
                self.write(&format!(
                    "for ({}.items[{}..]) |*elem| {{ elem.* = ",
                    receiver, _fs
                ));
                self.emit_expr(&data.args[0]);
                self.write("; }");
            }
            _ => {
                // fill(value, start, end) — with negative index support
                let (_fsr, _fer, _ln, _fs, _fe) = (
                    format!("_fsr_{}", blk),
                    format!("_fer_{}", blk),
                    format!("_ln_{}", blk),
                    format!("_fs_{}", blk),
                    format!("_fe_{}", blk),
                );
                self.write(&format!("const {}: isize = @intCast(", _fsr));
                self.emit_i64_coerced(&data.args[1]);
                self.write(&format!("); const {}: isize = @intCast(", _fer));
                self.emit_i64_coerced(&data.args[2]);
                self.write(&format!(
                    "); const {} = {}.items.len; const {}: usize = @intCast(if ({} < 0) @max(0, @as(isize, @intCast({})) + {}) else @min(@as(usize, @intCast({})), {})); const {}: usize = @intCast(if ({} < 0) @max(0, @as(isize, @intCast({})) + {}) else @min(@as(usize, @intCast({})), {})); ",
                    _ln, receiver, _fs, _fsr, _ln, _fsr, _fsr, _ln,
                    _fe, _fer, _ln, _fer, _fer, _ln
                ));
                // Guard: when end < start, fill is a no-op per JS spec.
                self.write(&format!(
                    "if ({} > {}) {{ for ({}.items[{}..{}]) |*elem| {{ elem.* = ",
                    _fe, _fs, receiver, _fs, _fe
                ));
                self.emit_expr(&data.args[0]);
                self.write("; } }");
            }
        }
        self.write(&format!(" break :{} {}; }})", blk, receiver));
    }

    // ── with ───────────────────────────────────────────
    pub(super) fn emit_with_inline(&mut self, data: &crate::zigir::types::IrArrayMethodInline) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        let blk = self.begin_labeled_block(&binding);
        let elem_type_str = data.elem_type.to_zig_type();
        let (_w, _wr, _wl, _wi) = (
            format!("_w_{}", blk),
            format!("_wr_{}", blk),
            format!("_wl_{}", blk),
            format!("_wi_{}", blk),
        );
        self.write(&format!(
            "var {}: std.ArrayList({}) = .empty; ",
            _w, elem_type_str
        ));
        self.write(&format!(
            "{}.appendSlice(js_allocator.allocator(), {}.items) catch @panic(\"OOM: Array.with appendSlice\"); ",
            _w, receiver
        ));
        // Compute index with from-end conversion for negative indices
        self.write(&format!("const {}: isize = @intCast(", _wr));
        if let Some(idx_arg) = data.args.first() {
            self.emit_i64_coerced(idx_arg);
        } else {
            self.write("0");
        }
        self.write(&format!(
            "); const {} = {}.items.len; const {}: usize = @intCast(if ({} < 0) @max(0, @as(isize, @intCast({})) + {}) else @min(@as(usize, @intCast({})), {})); ",
            _wl, _w, _wi, _wr, _wl, _wr, _wr, _wl
        ));
        // JS spec: with() throws RangeError for out-of-range index.
        self.write(&format!("if ({} >= {}.items.len) @panic(\"RangeError: Invalid array index for Array.with()\"); ", _wi, _w));
        self.write(&format!("{}.items[{}] = ", _w, _wi));
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
        self.write(&format!("break :{} {}; }})", blk, _w));
    }

    // ── toReversed ─────────────────────────────────────
    pub(super) fn emit_to_reversed_inline(
        &mut self,
        data: &crate::zigir::types::IrArrayMethodInline,
    ) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        let blk = self.begin_labeled_block(&binding);
        let elem_type_str = data.elem_type.to_zig_type();
        let (_rv, _ri) = (format!("_rv_{}", blk), format!("_ri_{}", blk));
        self.write(&format!(
            "var {}: std.ArrayList({}) = .empty; ",
            _rv, elem_type_str
        ));
        self.write(&format!(
            "{}.ensureTotalCapacity(js_allocator.allocator(), {}.items.len) catch @panic(\"OOM: Array.toReversed capacity\"); ",
            _rv, receiver
        ));
        self.write(&format!(
            "var {}: usize = {}.items.len; while ({} > 0) {{ {} -= 1; {}.append(js_allocator.allocator(), {}.items[{}]) catch @panic(\"OOM: Array.toReversed append\"); }} ",
            _ri, receiver, _ri, _ri, _rv, receiver, _ri
        ));
        self.write(&format!("break :{} {}; }})", blk, _rv));
    }

    // ── toSorted ───────────────────────────────────────
    pub(super) fn emit_to_sorted_inline(
        &mut self,
        data: &crate::zigir::types::IrArrayMethodInline,
    ) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        let blk = self.begin_labeled_block(&binding);
        let elem_type_str = data.elem_type.to_zig_type();
        let _so = format!("_so_{}", blk);
        self.write(&format!(
            "var {}: std.ArrayList({}) = .empty; ",
            _so, elem_type_str
        ));
        self.write(&format!(
            "{}.appendSlice(js_allocator.allocator(), {}.items) catch @panic(\"OOM: Array.toSorted appendSlice\"); ",
            _so, receiver
        ));
        // Sort — for JsAny elements use JsAny.lt(); for i64/f64 use ECMA-262
        // string comparison (format as strings, compare lexicographically);
        // other primitive types fall back to numeric std.sort.asc.
        if matches!(data.elem_type, ZigType::JsAny) {
            self.write(&format!("std.mem.sort(JsAny, {}.items, {{}}, struct {{ fn lessThan(_: void, a: JsAny, b: JsAny) bool {{ return a.lt(b); }} }}.lessThan); ", _so));
        } else if matches!(data.elem_type, ZigType::I64) {
            let (_sa, _sb, _xa, _xb) = (
                format!("_sa_{}", blk),
                format!("_sb_{}", blk),
                format!("_ra_{}", blk),
                format!("_rb_{}", blk),
            );
            self.write(&format!(
                "std.mem.sort({}, {}.items, {{}}, struct {{ fn lessThan(_: void, a: {}, b: {}) bool {{ var {}: [32]u8 = undefined; var {}: [32]u8 = undefined; const {} = std.fmt.bufPrint(&{}, \"{{d}}\", .{{a}}) catch return a < b; const {} = std.fmt.bufPrint(&{}, \"{{d}}\", .{{b}}) catch return a < b; return std.mem.order(u8, {}, {}) == .lt; }} }}.lessThan); ",
                elem_type_str, _so, elem_type_str, elem_type_str, _sa, _sb, _xa, _sa, _xb, _sb, _xa, _xb
            ));
        } else if matches!(data.elem_type, ZigType::F64) {
            let (_xa, _xb, _ord) = (
                format!("_ra_{}", blk),
                format!("_rb_{}", blk),
                format!("_ord_{}", blk),
            );
            self.write(&format!("std.mem.sort(f64, {}.items, {{}}, struct {{ fn lessThan(_: void, a: f64, b: f64) bool {{ const {} = js_number.toString(js_allocator.allocator(), a, 10) catch return a < b; const {} = js_number.toString(js_allocator.allocator(), b, 10) catch {{ js_allocator.allocator().free({}); return a < b; }}; const {} = std.mem.order(u8, {}, {}); js_allocator.allocator().free({}); js_allocator.allocator().free({}); return {} == .lt; }} }}.lessThan); ", _so, _xa, _xb, _xa, _ord, _xa, _xb, _xa, _xb, _ord));
        } else if matches!(data.elem_type, ZigType::Str) {
            self.write(&format!("std.mem.sort([]const u8, {}.items, {{}}, struct {{ fn lessThan(_: void, a: []const u8, b: []const u8) bool {{ return std.mem.order(u8, a, b) == .lt; }} }}.lessThan); ", _so));
        } else {
            self.write(&format!(
                "std.mem.sort({}, {}.items, {{}}, comptime std.sort.asc({})); ",
                elem_type_str, _so, elem_type_str
            ));
        }
        self.write(&format!("break :{} {}; }})", blk, _so));
    }

    // ── toSpliced ──────────────────────────────────────
    pub(super) fn emit_to_spliced_inline(
        &mut self,
        data: &crate::zigir::types::IrArrayMethodInline,
    ) {
        let (receiver, binding) = self.resolve_receiver(&data.obj_expr, &data.obj_name);

        let blk = self.begin_labeled_block(&binding);
        let elem_type_str = data.elem_type.to_zig_type();
        let (_tsp, _tss, _tsc, _j) = (
            format!("_tsp_{}", blk),
            format!("_tss_{}", blk),
            format!("_tsc_{}", blk),
            format!("_j_{}", blk),
        );
        self.write(&format!(
            "var {}: std.ArrayList({}) = .empty; ",
            _tsp, elem_type_str
        ));
        // Clone original array
        self.write(&format!(
            "{}.appendSlice(js_allocator.allocator(), {}.items) catch @panic(\"OOM: Array.toSpliced appendSlice\"); ",
            _tsp, receiver
        ));
        // Compute start index and delete count
        self.emit_splice_start_count(&_tss, &_tsc, &receiver, &data.args);
        // Remove elements from clone
        self.write(&format!(
            "var {}: usize = 0; while ({} < {}) : ({} += 1) {{ _ = {}.orderedRemove({}); }} ",
            _j, _j, _tsc, _j, _tsp, _tss
        ));
        // Insert items if provided (args beyond start and deleteCount)
        self.emit_splice_insert(&_tsp, &_tss, &elem_type_str, &data.args, "toSpliced");
        self.write(&format!("break :{} {}; }})", blk, _tsp));
    }
}
