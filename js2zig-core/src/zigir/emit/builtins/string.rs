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

        // Method dispatch: JS name → Zig runtime name + allocator + fallible.
        let (zig_method, needs_allocator, is_fallible, min_args, max_args, opt_defaults): (
            &str,
            bool,
            bool,
            usize,
            usize,
            &[&str],
        ) = match method {
            // ── No allocator, 0 args, non-fallible ──
            "trim" => ("trim", false, false, 0, 0, &[]),
            "trimStart" => ("trimStart", false, false, 0, 0, &[]),
            "trimEnd" => ("trimEnd", false, false, 0, 0, &[]),
            // ── No allocator, 1 arg, non-fallible ──
            "indexOf" => ("indexOf", false, false, 1, 1, &[]),
            "includes" => ("includes", false, false, 1, 1, &[]),
            "startsWith" => ("startsWith", false, false, 1, 1, &[]),
            "endsWith" => ("endsWith", false, false, 1, 1, &[]),
            "lastIndexOf" => ("lastIndexOf", false, false, 1, 1, &[]),
            "charCodeAt" => ("charCodeAt", false, false, 1, 1, &[]),
            "codePointAt" => ("codePointAt", false, false, 1, 1, &[]),
            // ── No allocator, 1-2 args, non-fallible ──
            "slice" => ("slice", false, false, 1, 2, &["std.math.maxInt(i64)"]),
            "substring" => ("substring", false, false, 1, 2, &["std.math.maxInt(i64)"]),
            // ── No allocator, 0-1 arg, non-fallible ──
            "localeCompare" => ("localeCompare", false, false, 0, 1, &[]),
            // ── With allocator, 0 args, fallible ──
            "toUpperCase" => ("toUpper", true, true, 0, 0, &[]),
            "toLocaleUpperCase" => ("toLocaleUpper", true, true, 0, 0, &[]),
            "toLowerCase" => ("toLower", true, true, 0, 0, &[]),
            "toLocaleLowerCase" => ("toLocaleLower", true, true, 0, 0, &[]),
            // ── With allocator, 1 arg, fallible ──
            "charAt" => ("charAt", true, true, 1, 1, &[]),
            "at" => ("at", true, true, 1, 1, &[]),
            "concat" => ("concat", true, true, 1, 1, &[]),
            "repeat" => ("repeat", true, true, 1, 1, &[]),
            // ── With allocator, 1 arg, fallible (returns ![][]const u8) ──
            "split" => ("split", true, true, 1, 1, &[]),
            // ── With allocator, 2 args, fallible ──
            "padStart" => ("padStart", true, true, 2, 2, &[]),
            "padEnd" => ("padEnd", true, true, 2, 2, &[]),
            "replace" => ("replace", true, true, 2, 2, &[]),
            "replaceAll" => ("replaceAll", true, true, 2, 2, &[]),
            // ── With allocator, 0-1 arg, fallible ──
            "normalize" => ("normalize", true, true, 0, 1, &["\"NFC\""]),
            // ── Fallback ──
            _ => {
                // Unknown string method — naive emission
                self.emit_module_call("js_string", method, args);
                return;
            }
        };

        // Emit: [try ]js_string.zig_method([js_allocator.allocator(), ]obj[, arg1[, arg2...]])
        if is_fallible {
            self.write("try ");
        }
        self.write(&format!("js_string.{}(", zig_method));
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
        self.write(")");
    }

    /// Shared by match/matchAll: emit `js_string.{method}(js_allocator.allocator(), receiver, pattern_expr) catch @panic(...)`
    /// for the is_var_ref and literal branches. The fallback branch uses js_string.{method}(receiver).
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
                        "js_string.{}(js_allocator.allocator(), {}, {}.pattern) catch @panic(\"OOM: allocation\")",
                        method, receiver, var
                    ));
                }
            }
            Some(ri) => {
                if let Some(pattern) = &ri.pattern {
                    self.write(&format!(
                        "js_string.{}(js_allocator.allocator(), {}, \"{}\") catch @panic(\"OOM: allocation\")",
                        method, receiver, pattern
                    ));
                }
            }
            None => {
                self.write(&format!("js_string.{}({})", method, receiver));
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
                "js_string.matchStringGlobal(js_allocator.allocator(), {}, \"{}\") catch @panic(\"OOM: allocation\")",
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
                    self.write(&format!("host.regex_search({}.pattern, {})", var, receiver));
                }
            }
            Some(ri) => {
                if let Some(pattern) = &ri.pattern {
                    self.write(&format!("host.regex_search(\"{}\", {})", pattern, receiver));
                }
            }
            None => {
                // Non-regex argument: render the first arg as a string pattern
                if let Some(arg) = args.first() {
                    let pattern = Self::emit_expr_inline(arg);
                    self.write(&format!("host.regex_search({}, {})", pattern, receiver));
                } else {
                    self.write(&format!("host.regex_search(\"\", {})", receiver));
                }
            }
        }
    }
}
