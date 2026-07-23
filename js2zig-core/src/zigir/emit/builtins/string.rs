// zigir/emit/builtins/string.rs
// String builtin method emission.

use crate::zigir::emit::builtins::math::expr_is_float;
use crate::zigir::emit::helpers::EmitterHelpers;

use crate::zigir::emit::Emitter;

impl Emitter {
    /// Emit an argument coerced to i64, handling both int and float inputs.
    /// Used by fromCharCode/fromCodePoint which take `[]const i64`.
    /// - Float expressions: `@as(i64, @intFromFloat(expr))`
    /// - Int expressions/literals: emit directly (already i64)
    fn emit_i64_coerced(&mut self, arg: &crate::zigir::types::IrExpr) {
        if expr_is_float(arg) {
            self.write("@as(i64, @intFromFloat(");
            self.emit_expr(arg);
            self.write("))");
        } else {
            self.emit_expr(arg);
        }
    }

    pub(super) fn emit_string_builtin(
        &mut self,
        method: &str,
        obj: Option<&str>,
        args: &[crate::zigir::types::IrExpr],
        regex_info: Option<&crate::zigir::types::IrRegexInfo>,
    ) {
        // ── Regex-dependent methods: match / matchAll / search ──
        match method {
            "match" => {
                self.emit_string_match(obj, args, regex_info);
                return;
            }
            "matchAll" => {
                self.emit_string_match_all(obj, regex_info);
                return;
            }
            "search" => {
                self.emit_string_search(obj, args, regex_info);
                return;
            }
            // R8-P1-23: replace/replaceAll branch on regex_info for RegExp routing
            "replace" => {
                self.emit_string_replace(obj, args, regex_info);
                return;
            }
            "replaceAll" => {
                self.emit_string_replace_all(obj, args, regex_info);
                return;
            }
            _ => {}
        }

        // ── Static variadic calls: String.fromCharCode / String.fromCodePoint ──
        // These have signature `fn(Allocator, []const i64) ![]const u8`, so the
        // emitter must prepend the allocator and pack all JS arguments into an
        // `&[_]i64{ ... }` slice literal. (obj is None for these — they are
        // static methods on the `String` constructor, not instance methods.)
        match method {
            "fromCharCode" => {
                self.write("js_string.fromCharCode(js_allocator.allocator(), &[_]i64{");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.emit_i64_coerced(arg);
                }
                self.write("}) catch @panic(\"OOM: string method\")");
                return;
            }
            "fromCodePoint" => {
                // R8-P1-18: invalid code points (cp < 0 or cp > 0x10FFFF)
                // produce `error.RangeError` from the runtime. For now we
                // route it to a hard panic; full JS-throw routing (marking
                // the enclosing fn `can_throw` and returning `error.JsThrow`)
                // is tracked as a future improvement.
                self.write("js_string.fromCodePoint(js_allocator.allocator(), &[_]i64{");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.emit_i64_coerced(arg);
                }
                self.write(
                    "}) catch |err| switch (err) { \
                     error.RangeError => @panic(\"fromCodePoint: invalid code point\"), \
                     else => @panic(\"OOM: string method\") }",
                );
                return;
            }
            _ => {}
        }

        // Method dispatch: JS name → Zig runtime name + allocator + fallible.
        // ICU-dependent methods (localeCompare, normalize, toLocaleUpperCase,
        // toLocaleLowerCase) are routed through js_string_icu instead of
        // js_string. This module provides simplified implementations by default,
        // and ICU4X-based implementations when needs_icu=true.
        let (zig_method, needs_allocator, is_fallible, min_args, max_args, opt_defaults, module): (
            &str,
            bool,
            bool,
            usize,
            usize,
            &[&str],
            &str, // "js_string" or "js_string_icu"
        ) = match method {
            // ── No allocator, 0 args, non-fallible ──
            "trim" => ("trim", false, false, 0, 0, &[], "js_string"),
            "trimStart" => ("trimStart", false, false, 0, 0, &[], "js_string"),
            "trimEnd" => ("trimEnd", false, false, 0, 0, &[], "js_string"),
            // ── No allocator, 1 arg, non-fallible ──
            "includes" => ("includes", false, false, 1, 1, &[], "js_string"),
            "startsWith" => ("startsWith", false, false, 1, 1, &[], "js_string"),
            "endsWith" => ("endsWith", false, false, 1, 1, &[], "js_string"),
            "charCodeAt" => ("charCodeAt", false, false, 1, 1, &[], "js_string"),
            "codePointAt" => ("codePointAt", false, false, 1, 1, &[], "js_string"),
            // ── R8-P1-19: indexOf/lastIndexOf support optional fromIndex ──
            // JS `indexOf(searchString, fromIndex)` defaults fromIndex=0
            // (search from the start). JS `lastIndexOf(searchString, fromIndex)`
            // defaults fromIndex=+∞ (search from the end); the runtime clamps
            // any value ≥ len to len. We use `std.math.maxInt(i64)` as a Zig
            // sentinel guaranteed ≥ any valid string length. Sign convention
            // matches `slice`/`substring`.
            "indexOf" => ("indexOf", false, false, 1, 2, &["0"], "js_string"),
            "lastIndexOf" => (
                "lastIndexOf",
                false,
                false,
                1,
                2,
                &["std.math.maxInt(i64)"],
                "js_string",
            ),
            // ── No allocator, 1-2 args, non-fallible ──
            "slice" => (
                "slice",
                false,
                false,
                1,
                2,
                &["std.math.maxInt(i64)"],
                "js_string",
            ),
            "substring" => (
                "substring",
                false,
                false,
                1,
                2,
                &["std.math.maxInt(i64)"],
                "js_string",
            ),
            // ── ICU-dependent: No allocator, 0-1 arg, non-fallible ──
            "localeCompare" => ("localeCompare", false, false, 0, 1, &[], "js_string_icu"),
            // ── With allocator, 0 args, fallible ──
            "toUpperCase" => ("toUpper", true, true, 0, 0, &[], "js_string"),
            // ── ICU-dependent: With allocator, 0 args, fallible ──
            "toLocaleUpperCase" => ("toLocaleUpper", true, true, 0, 0, &[], "js_string_icu"),
            "toLowerCase" => ("toLower", true, true, 0, 0, &[], "js_string"),
            "toLocaleLowerCase" => ("toLocaleLower", true, true, 0, 0, &[], "js_string_icu"),
            // ── With allocator, 1 arg, fallible ──
            "charAt" => ("charAt", true, true, 1, 1, &[], "js_string"),
            "at" => ("at", true, true, 1, 1, &[], "js_string"),
            "concat" => ("concat", true, true, 1, 1, &[], "js_string"),
            "repeat" => ("repeat", true, true, 1, 1, &[], "js_string"),
            // ── With allocator, 1 arg, fallible (returns ![][]const u8) ──
            "split" => ("split", true, true, 1, 1, &[], "js_string"),
            // ── With allocator, 2 args, fallible ──
            "padStart" => ("padStart", true, true, 2, 2, &[], "js_string"),
            "padEnd" => ("padEnd", true, true, 2, 2, &[], "js_string"),
            // ── ICU-dependent: With allocator, 0-1 arg, fallible ──
            "normalize" => ("normalize", true, true, 0, 1, &["\"NFC\""], "js_string_icu"),
            // ── Fallback ──
            _ => {
                // Unknown string method — naive emission
                self.emit_module_call("js_string", method, args);
                return;
            }
        };

        // Emit: module.zig_method([js_allocator.allocator(), ]obj[, arg1[, arg2...]])[ catch @panic("OOM: string method")]
        // Fallible string methods use catch @panic instead of try, consistent with
        // regex methods (match/matchAll/search), BigInt init, and Map/Set operations.
        // try would require the enclosing function to return an error union, which
        // the transpiler does not currently propagate from builtin calls.
        self.write(&format!("{}.{}(", module, zig_method));
        if needs_allocator {
            self.write("js_allocator.allocator(), ");
        }
        // Receiver object
        if let Some(name) = obj {
            self.write(name);
        }
        // Arguments (fill to max_args with opt_defaults for missing slots)
        let n_args = args.len();
        let total_slots = max_args;
        for slot in 0..total_slots {
            if slot < n_args {
                self.write(", ");
                self.emit_expr(&args[slot]);
            } else {
                let opt_idx = slot - min_args;
                if let Some(default) = opt_defaults.get(opt_idx)
                    && !default.is_empty()
                {
                    self.write(&format!(", {}", default));
                }
            }
        }
        if is_fallible {
            self.write(") catch @panic(\"OOM: string method\")");
        } else {
            self.write(")");
        }
    }

    /// Shared by match/matchAll: emit `js_string_regex.{method}(js_allocator.allocator(), receiver, pattern_expr) catch @panic(...)`
    /// for the is_var_ref and literal branches. The fallback branch uses js_string_regex.{method}(receiver).
    fn emit_string_match_like(
        &mut self,
        obj: Option<&str>,
        regex_info: Option<&crate::zigir::types::IrRegexInfo>,
        method: &str,
    ) {
        let receiver = obj.unwrap_or("\"\"");
        match regex_info {
            Some(ri) if ri.is_var_ref => {
                if let Some(var) = &ri.var_name {
                    self.write(&format!(
                        "js_string_regex.{}(js_allocator.allocator(), {}, {}.pattern) catch @panic(\"OOM: allocation\")",
                        method, receiver, var
                    ));
                }
            }
            Some(ri) => {
                if let Some(pattern) = &ri.pattern {
                    self.write(&format!(
                        "js_string_regex.{}(js_allocator.allocator(), {}, \"{}\") catch @panic(\"OOM: allocation\")",
                        method, receiver, pattern
                    ));
                }
            }
            None => {
                self.write(&format!("js_string_regex.{}({})", method, receiver));
            }
        }
    }

    // ── String.match() ──────────────────────────────────
    pub(super) fn emit_string_match(
        &mut self,
        obj: Option<&str>,
        args: &[crate::zigir::types::IrExpr],
        regex_info: Option<&crate::zigir::types::IrRegexInfo>,
    ) {
        // match has an extra global-flag branch → matchStringGlobal
        if let Some(ri) = regex_info
            && !ri.is_var_ref
            && ri.has_global
            && let Some(pattern) = &ri.pattern
        {
            let receiver = obj.unwrap_or("\"\"");
            self.write(&format!(
                "js_string_regex.matchStringGlobal(js_allocator.allocator(), {}, \"{}\") catch @panic(\"OOM: allocation\")",
                receiver, pattern
            ));
            return;
        }
        // Variable regex: runtime branch on .global to select matchStringGlobal vs matchString
        if let Some(ri) = regex_info
            && ri.is_var_ref
            && let Some(var) = &ri.var_name
        {
            let receiver = obj.unwrap_or("\"\"");
            self.write(&format!(
                "(if ({}.global) js_string_regex.matchStringGlobal(js_allocator.allocator(), {}, {}.pattern) catch @panic(\"OOM: allocation\") else js_string_regex.matchString(js_allocator.allocator(), {}, {}.pattern) catch @panic(\"OOM: allocation\"))",
                var, receiver, var, receiver, var
            ));
            return;
        }
        // Non-RegExp argument (string literal, expression) or no regex_info.
        // Only reach here when regex_info is None (non-RegExp argument).
        // Use the first argument as the pattern string, or empty string if no args.
        // JS spec: str.match(x) converts x to RegExp via new RegExp(x), which
        // uses String(x) as the pattern. We pass the arg directly as the pattern.
        if regex_info.is_none() {
            let receiver = obj.unwrap_or("\"\"");
            let pattern = match args.first() {
                Some(arg) => {
                    // Render the arg expression to a string for inline embedding
                    let mut buf = String::new();
                    std::mem::swap(&mut self.output, &mut buf);
                    self.emit_expr(arg);
                    std::mem::swap(&mut self.output, &mut buf);
                    buf.trim().to_string()
                }
                None => "\"\"".to_string(),
            };
            self.write(&format!(
                "js_string_regex.matchString(js_allocator.allocator(), {}, {}) catch @panic(\"OOM: allocation\")",
                receiver, pattern
            ));
            return;
        }
        // Literal RegExp without global flag — delegate to shared helper
        self.emit_string_match_like(obj, regex_info, "matchString");
    }

    // ── String.matchAll() ───────────────────────────────
    /// R8-P1-25: JS spec requires matchAll to throw TypeError if the RegExp
    /// does not have the /g flag. Literal RegExp without /g is caught at
    /// lower time (IrExpr::CompileError). Variable RegExp needs a runtime guard.
    pub(super) fn emit_string_match_all(
        &mut self,
        obj: Option<&str>,
        regex_info: Option<&crate::zigir::types::IrRegexInfo>,
    ) {
        let receiver = obj.unwrap_or("\"\"");
        match regex_info {
            // No regex info — no RegExp argument, skip validation
            None => {
                self.write(&format!("js_string_regex.matchAllString({})", receiver));
            }
            // Literal RegExp with /g — valid, emit matchAllString
            // (Literal without /g is caught at lower time — won't reach here)
            Some(ri) if !ri.is_var_ref => {
                if let Some(pattern) = &ri.pattern {
                    self.write(&format!(
                        "js_string_regex.matchAllString(js_allocator.allocator(), {}, \"{}\") catch @panic(\"OOM: allocation\")",
                        receiver, pattern
                    ));
                }
            }
            // Variable RegExp — runtime guard on .global field
            Some(ri) if ri.is_var_ref => {
                if let Some(var) = &ri.var_name {
                    self.write(&format!(
                        "(if (!{}.global) @panic(\"TypeError: String.prototype.matchAll called with a non-global RegExp argument\") else js_string_regex.matchAllString(js_allocator.allocator(), {}, {}.pattern) catch @panic(\"OOM: allocation\"))",
                        var, receiver, var
                    ));
                }
            }
            // Fallback (shouldn't reach)
            Some(_) => {
                self.emit_string_match_like(obj, regex_info, "matchAllString");
            }
        }
    }

    // ── String.search() ─────────────────────────────────
    pub(super) fn emit_string_search(
        &mut self,
        obj: Option<&str>,
        args: &[crate::zigir::types::IrExpr],
        regex_info: Option<&crate::zigir::types::IrRegexInfo>,
    ) {
        let receiver = obj.unwrap_or("\"\"");
        match regex_info {
            Some(ri) if ri.is_var_ref => {
                if let Some(var) = &ri.var_name {
                    self.write(&format!(
                        "host_regex.regex_search({}.pattern, {})",
                        var, receiver
                    ));
                }
            }
            Some(ri) => {
                if let Some(pattern) = &ri.pattern {
                    self.write(&format!(
                        "host_regex.regex_search(\"{}\", {})",
                        pattern, receiver
                    ));
                }
            }
            None => {
                // Non-regex argument: render the first arg as a string pattern
                if let Some(arg) = args.first() {
                    let (pattern, new_counter) =
                        Self::emit_expr_inline_with_label_offset(arg, self.label_counter);
                    self.label_counter = new_counter;
                    self.write(&format!(
                        "host_regex.regex_search({}, {})",
                        pattern, receiver
                    ));
                } else {
                    self.write(&format!("host_regex.regex_search(\"\", {})", receiver));
                }
            }
        }
    }

    // ── String.replace() ──────────────────────────────────
    /// R8-P1-23: Branch on regex_info for RegExp vs plain-string routing.
    /// - None → js_string.replace(alloc, s, arg1, arg2)  (plain-string, first-only)
    /// - Some(ri) literal → js_string_regex.replaceRegex(alloc, s, "pattern", arg2)
    /// - Some(ri) variable → js_string_regex.replaceRegex(alloc, s, var.pattern, arg2)
    pub(super) fn emit_string_replace(
        &mut self,
        obj: Option<&str>,
        args: &[crate::zigir::types::IrExpr],
        regex_info: Option<&crate::zigir::types::IrRegexInfo>,
    ) {
        let receiver = obj.unwrap_or("\"\"");
        match regex_info {
            // Variable RegExp → runtime regex replace with var.pattern
            Some(ri) if ri.is_var_ref => {
                if let Some(var) = &ri.var_name {
                    self.write(&format!(
                        "js_string_regex.replaceRegex(js_allocator.allocator(), {}, {}.pattern, ",
                        receiver, var
                    ));
                    if let Some(arg) = args.get(1) {
                        self.emit_expr(arg);
                    } else {
                        self.write("\"\"");
                    }
                    self.write(") catch @panic(\"OOM: string method\")");
                }
            }
            // Literal RegExp → compile-time known pattern
            Some(ri) => {
                if let Some(pattern) = &ri.pattern {
                    self.write(&format!(
                        "js_string_regex.replaceRegex(js_allocator.allocator(), {}, \"{}\", ",
                        receiver, pattern
                    ));
                    if let Some(arg) = args.get(1) {
                        self.emit_expr(arg);
                    } else {
                        self.write("\"\"");
                    }
                    self.write(") catch @panic(\"OOM: string method\")");
                }
            }
            // No regex info → plain-string replace (first occurrence only)
            None => {
                self.write("js_string.replace(js_allocator.allocator(), ");
                self.write(receiver);
                self.write(", ");
                if let Some(arg) = args.first() {
                    self.emit_expr(arg);
                } else {
                    self.write("\"\"");
                }
                self.write(", ");
                if let Some(arg) = args.get(1) {
                    self.emit_expr(arg);
                } else {
                    self.write("\"\"");
                }
                self.write(") catch @panic(\"OOM: string method\")");
            }
        }
    }

    // ── String.replaceAll() ───────────────────────────────
    /// R8-P1-23: Branch on regex_info for RegExp vs plain-string routing.
    ///
    /// - None → js_string.replaceAll(alloc, s, arg1, arg2)  (plain-string)
    /// - Some(ri) literal with /g → js_string_regex.replaceAllRegex(alloc, s, "pattern", arg2)
    /// - Some(ri) variable with /g → js_string_regex.replaceAllRegex(alloc, s, var.pattern, arg2)
    ///   (runtime guard on .global for variable RegExp)
    ///
    /// R8-P1-25: Literal RegExp without /g is caught at lower time (CompileError).
    pub(super) fn emit_string_replace_all(
        &mut self,
        obj: Option<&str>,
        args: &[crate::zigir::types::IrExpr],
        regex_info: Option<&crate::zigir::types::IrRegexInfo>,
    ) {
        let receiver = obj.unwrap_or("\"\"");
        match regex_info {
            // Variable RegExp → runtime guard on .global + regex replaceAll
            Some(ri) if ri.is_var_ref => {
                if let Some(var) = &ri.var_name {
                    self.write(&format!(
                        "(if (!{}.global) @panic(\"TypeError: String.prototype.replaceAll called with a non-global RegExp argument\") else js_string_regex.replaceAllRegex(js_allocator.allocator(), {}, {}.pattern, ",
                        var, receiver, var
                    ));
                    if let Some(arg) = args.get(1) {
                        self.emit_expr(arg);
                    } else {
                        self.write("\"\"");
                    }
                    self.write(") catch @panic(\"OOM: string method\"))");
                }
            }
            // Literal RegExp with /g → compile-time known pattern
            // (Literal without /g is caught at lower time — won't reach here)
            Some(ri) => {
                if let Some(pattern) = &ri.pattern {
                    self.write(&format!(
                        "js_string_regex.replaceAllRegex(js_allocator.allocator(), {}, \"{}\", ",
                        receiver, pattern
                    ));
                    if let Some(arg) = args.get(1) {
                        self.emit_expr(arg);
                    } else {
                        self.write("\"\"");
                    }
                    self.write(") catch @panic(\"OOM: string method\")");
                }
            }
            // No regex info → plain-string replaceAll
            None => {
                self.write("js_string.replaceAll(js_allocator.allocator(), ");
                self.write(receiver);
                self.write(", ");
                if let Some(arg) = args.first() {
                    self.emit_expr(arg);
                } else {
                    self.write("\"\"");
                }
                self.write(", ");
                if let Some(arg) = args.get(1) {
                    self.emit_expr(arg);
                } else {
                    self.write("\"\"");
                }
                self.write(") catch @panic(\"OOM: string method\")");
            }
        }
    }
}
