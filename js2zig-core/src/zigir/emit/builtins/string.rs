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
                self.emit_string_search(obj, regex_info);
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
                self.write(&format!("js_string.{}(", method));
                self.emit_inline_args(args);
                self.write(")");
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

    // ── String.match() ──────────────────────────────────
    pub(super) fn emit_string_match(
        &mut self,
        obj: Option<&str>,
        regex_info: Option<&crate::zigir::types::IrRegexInfo>,
    ) {
        let receiver = obj.unwrap_or("\"\"");
        match regex_info {
            Some(ri) if ri.is_var_ref => {
                if let Some(var) = &ri.var_name {
                    self.write(&format!(
                        "js_string.matchString(js_allocator.allocator(), {}, {}.pattern) catch @panic(\"OOM: allocation\")",
                        receiver, var
                    ));
                }
            }
            Some(ri) => {
                if let Some(pattern) = &ri.pattern {
                    if ri.has_global {
                        self.write(&format!(
                            "js_string.matchStringGlobal(js_allocator.allocator(), {}, \"{}\") catch @panic(\"OOM: allocation\")",
                            receiver, pattern
                        ));
                    } else {
                        self.write(&format!(
                            "js_string.matchString(js_allocator.allocator(), {}, \"{}\") catch @panic(\"OOM: allocation\")",
                            receiver, pattern
                        ));
                    }
                }
            }
            None => {
                // Fallback: no regex info available — emit as js_string.match()
                self.write(&format!("js_string.match({})", receiver));
            }
        }
    }

    // ── String.matchAll() ───────────────────────────────
    pub(super) fn emit_string_match_all(
        &mut self,
        obj: Option<&str>,
        regex_info: Option<&crate::zigir::types::IrRegexInfo>,
    ) {
        let receiver = obj.unwrap_or("\"\"");
        match regex_info {
            Some(ri) if ri.is_var_ref => {
                if let Some(var) = &ri.var_name {
                    self.write(&format!(
                        "js_string.matchAllString(js_allocator.allocator(), {}, {}.pattern) catch @panic(\"OOM: allocation\")",
                        receiver, var
                    ));
                }
            }
            Some(ri) => {
                if let Some(pattern) = &ri.pattern {
                    self.write(&format!(
                        "js_string.matchAllString(js_allocator.allocator(), {}, \"{}\") catch @panic(\"OOM: allocation\")",
                        receiver, pattern
                    ));
                }
            }
            None => {
                // Fallback
                self.write(&format!("js_string.matchAll({})", receiver));
            }
        }
    }

    // ── String.search() ─────────────────────────────────
    pub(super) fn emit_string_search(
        &mut self,
        obj: Option<&str>,
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
                self.write(&format!("host.regex_search(, {})", receiver));
            }
        }
    }
}
