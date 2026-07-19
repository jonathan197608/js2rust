// zigir/emit/builtins/string.rs
// String builtin method emission.

use crate::zigir::emit::helpers::EmitterHelpers;

use crate::zigir::emit::Emitter;

impl Emitter {
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
                self.emit_string_match(obj, regex_info);
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
                    self.emit_expr(arg);
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
                    self.emit_expr(arg);
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
            "replace" => ("replace", true, true, 2, 2, &[], "js_string"),
            "replaceAll" => ("replaceAll", true, true, 2, 2, &[], "js_string"),
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
        self.emit_string_match_like(obj, regex_info, "matchString");
    }

    // ── String.matchAll() ───────────────────────────────
    pub(super) fn emit_string_match_all(
        &mut self,
        obj: Option<&str>,
        regex_info: Option<&crate::zigir::types::IrRegexInfo>,
    ) {
        self.emit_string_match_like(obj, regex_info, "matchAllString");
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
                    let pattern = Self::emit_expr_inline(arg);
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
}
