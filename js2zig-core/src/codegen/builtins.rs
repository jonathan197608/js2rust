use super::*;

impl<'a> ZigCodegen<'a> {
    pub(super) fn try_emit_builtin_call(&mut self, call: &CallExpression) -> bool {
        // Case 1: obj.method(args) — StaticMemberExpression callee
        if let Expression::StaticMemberExpression(mem) = &call.callee {
            let obj_expr = &mem.object;
            let method_name = mem.property.name.as_str();

            // Dynamic array methods: use ArrayList directly (before any lookup)
            if let Expression::Identifier(id) = obj_expr
                && self.inferrer.is_dynamic_array(id.name.as_str()) {
                    self.emit_dynamic_array_method(id.name.as_str(), method_name, &call.arguments);
                    return true;
                }

            // ── Namespace lookup (Math.abs, console.log, Object.keys, …) ──
            // Use the object's identifier name (e.g. "Math", "console", "Object")
            if let Expression::Identifier(id) = obj_expr
                && let Some(trans) = self.builtins.lookup_method(id.name.as_str(), method_name) {
                    // Namespace call: template already bakes in the receiver.
                    // e.g. template "js_console.log({})" → just pass call arguments.
                    self.apply_builtin_template(trans, &call.arguments);
                    return true;
                }

                // ── Type-based lookup (arr.indexOf, str.toUpperCase, …) ──
                // e.g.  arr.indexOf(42) → key "array", template "js_array.indexOf({}, {})"
                let obj_ty = self.inferrer.infer_expr(obj_expr);
                if let Some(type_key) = Self::type_to_builtin_key(&obj_ty)
                    && let Some(trans) = self.builtins.lookup_method(type_key, method_name) {
                        // Type-based call: template expects receiver as {0}.
                        self.emit_builtin_method_call(trans, obj_expr, &call.arguments, &obj_ty);
                        return true;
                    }

                // ── Regexp dispatch (re.test(str), re.exec(str)) ──
                // Simplified: regexp literals are emitted as pattern strings,
                // so re.test(str) → js_regexp.test_(str, re)
                if method_name == "test" {
                    self.push("js_regexp.test_(");
                    if let Some(arg0) = call.arguments.first() {
                        self.emit_arg(arg0);
                    }
                    self.push(", ");
                    self.emit_expr(obj_expr);
                    self.push(")");
                    return true;
                }
                if method_name == "exec" {
                    self.push("(js_regexp.exec(js_allocator.g_alloc(), ");
                    if let Some(arg0) = call.arguments.first() {
                        self.emit_arg(arg0);
                    }
                    self.push(", ");
                    self.emit_expr(obj_expr);
                    self.push(") catch null)");
                    // Also emit a follow-up if-block for the caller
                    // This is handled by the caller's if-clause in JS
                    return true;
                }
            }

        // Case 2: globalFunc(args) — Identifier callee
        if let Expression::Identifier(id) = &call.callee
            && let Some(trans) = self.builtins.lookup_global(id.name.as_str())
        {
            self.apply_builtin_template(trans, &call.arguments);
            return true;
        }

        false
    }

    /// Map a ZigType to a builtin lookup key ("array", "string", etc.)
    pub(super) fn type_to_builtin_key(ty: &ZigType) -> Option<&'static str> {
        match ty {
            ZigType::String => Some("string"),
            ZigType::Array(_) => Some("array"),
            ZigType::Object { .. } => Some("object"),
            ZigType::Struct(s) => {
                match s.as_str() {
                    "Map" => Some("map"),
                    "Set" => Some("set"),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Emit a builtin method call, handling the receiver object.
    /// The template may use {} (all args) or {0}, {1} (positional).
    /// For type-dispatched calls, the receiver is implicitly arg 0.
    pub(super) fn emit_builtin_method_call(
        &mut self,
        trans: &crate::builtins::BuiltinTranslation,
        receiver: &Expression,
        args: &oxc_allocator::Vec<'_, Argument>,
        receiver_type: &ZigType,
    ) {
        let template = &trans.template;

        // Check if template starts with a runtime function that needs allocator
        // e.g., "js_array.indexOf({}, {})" — receiver goes into {}
        // We need to replace {} with "receiver, arg0, arg1..."
        // and {0}, {1} with positional args

        // Collect all arg strings (receiver + call args)
        let mut all_args: Vec<String> = Vec::new();
        let empty_exports = std::collections::HashSet::new();
        let mut tmp = ZigCodegen {
            output: String::new(),
            indent: self.indent,
            inferrer: TypeInferrer::new(),
            diagnostics: &mut Vec::new(),
            in_top_level: self.in_top_level,
            task_counter: self.task_counter,
            builtins: self.builtins,
            closure_map: std::collections::HashMap::new(),
            closure_struct_defs: std::collections::HashMap::new(),
            fn_closure_spans: std::collections::HashMap::new(),
            closure_counter: 0,
            closure_structs: Vec::new(),
            cabi_wrappers: Vec::new(),
            cabi_exports: Vec::new(),
            string_return_fns: std::collections::HashSet::new(),
            closure_vars: std::collections::HashSet::new(),
            current_fn: None,
            exports: empty_exports,
            try_label: None,
            catch_label: None,
            try_counter: self.try_counter,
            temp_counter: 0,
            destructure_prelude: Vec::new(),
            current_class: None,
            object_type_defs: Vec::new(),
            current_obj_structs: Vec::new(),
            init_globals_code: Vec::new(),
            source_map: crate::sourcemap::SourceMap::new(""),
            line_index: crate::sourcemap::LineIndex::new(""),
            source_file: String::new(),
        };
        tmp.emit_expr(receiver);
        // Static arrays need & to coerce to []const T for runtime functions
        if matches!(receiver_type, ZigType::Array(_)) {
            all_args.push(format!("&{}", tmp.output));
        } else {
            all_args.push(tmp.output.clone());
        }

        for arg in args.iter() {
            let mut tmp2 = ZigCodegen {
                output: String::new(),
                indent: self.indent,
                inferrer: TypeInferrer::new(),
                diagnostics: &mut Vec::new(),
                in_top_level: self.in_top_level,
                task_counter: self.task_counter,
                builtins: self.builtins,
                closure_map: std::collections::HashMap::new(),
                closure_struct_defs: std::collections::HashMap::new(),
                fn_closure_spans: std::collections::HashMap::new(),
                closure_counter: 0,
                closure_structs: Vec::new(),
                cabi_wrappers: Vec::new(),
                cabi_exports: Vec::new(),
                string_return_fns: std::collections::HashSet::new(),
                closure_vars: std::collections::HashSet::new(),
                current_fn: None,
                exports: std::collections::HashSet::new(),
                try_label: None,
                catch_label: None,
                try_counter: self.try_counter,
                temp_counter: 0,
                destructure_prelude: Vec::new(),
                current_class: None,
                object_type_defs: Vec::new(),
                current_obj_structs: Vec::new(),
                init_globals_code: Vec::new(),
            source_map: crate::sourcemap::SourceMap::new(""),
            line_index: crate::sourcemap::LineIndex::new(""),
            source_file: String::new(),
            };
            tmp2.emit_arg(arg);
            all_args.push(tmp2.output.clone());
        }

        // Now apply template: {} = sequential arg, {0} = all_args[0], etc.
        let mut result = String::new();
        let mut chars = template.chars().peekable();
        let all_args_ref: Vec<&str> = all_args.iter().map(|s| s.as_str()).collect();
        let mut seq_idx: usize = 0; // sequential argument index for {} placeholders
        while let Some(ch) = chars.next() {
            if ch == '{' {
                if let Some(&('0'..='9')) = chars.peek() {
                    let mut idx_str = String::new();
                    while let Some(&('0'..='9')) = chars.peek() {
                        idx_str.push(chars.next().expect("peek() guaranteed a digit"));
                    }
                    if chars.peek() == Some(&'}') {
                        chars.next();
                    }
                    if let Ok(idx) = idx_str.parse::<usize>()
                        && let Some(arg) = all_args_ref.get(idx) {
                            result.push_str(arg);
                        }
                } else if chars.peek() == Some(&'}') {
                    chars.next();
                    if let Some(arg) = all_args_ref.get(seq_idx) {
                        result.push_str(arg);
                    }
                    seq_idx += 1;
                } else {
                    result.push(ch);
                }
            } else {
                result.push(ch);
            }
        }
        self.push(&result);
    }

    /// Infer the element type of a dynamic array variable.
    /// Returns the Zig type string for the element (e.g., "i64", "f64").
    pub(super) fn infer_dynamic_array_elem_type(&self, var_name: &str) -> String {
        let var_type = self.inferrer.get_var_type(var_name);
        match var_type {
            ZigType::Array(elem) | ZigType::Slice(elem) => elem.to_zig_str(),
            ZigType::JsAny => "JsAny".to_string(),
            _ => var_type.to_zig_str(),
        }
    }

    /// Emit direct ArrayList method calls for dynamic arrays
    /// (instead of going through js_array runtime functions).
    pub(super) fn emit_dynamic_array_method(
        &mut self,
        obj_name: &str,
        method: &str,
        args: &oxc_allocator::Vec<'_, Argument>,
    ) {
        let escaped = Self::escape_keyword(obj_name);

        match method {
            "push" => {
                // arr.push(val) → arr.append(js_allocator.g_alloc(), JsAny.fromXxx(val)) catch {};
                self.push(&escaped);
                self.push(".append(js_allocator.g_alloc(), ");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { self.push(", "); }
                    self.emit_jsany_arg(arg);
                }
                self.push(") catch {}");
            }
            "pop" => {
                self.push(&escaped);
                self.push(".pop() orelse JsAny.fromNull()");
            }
            "shift" => {
                self.push("(blk: { if (");
                self.push(&escaped);
                self.push(".items.len == 0) break :blk JsAny.fromNull(); break :blk ");
                self.push(&escaped);
                self.push(".orderedRemove(0); })");
            }
            "unshift" => {
                // arr.unshift(val) → arr.insert(js_allocator.g_alloc(), 0, JsAny.fromXxx(val)) catch {};
                self.push(&escaped);
                self.push(".insert(js_allocator.g_alloc(), 0, ");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { self.push(", "); }
                    self.emit_jsany_arg(arg);
                }
                self.push(") catch {}");
            }
            "reverse" => {
                // arr.reverse() → std.mem.reverse(JsAny, arr.items);
                self.push("std.mem.reverse(JsAny, ");
                self.push(&escaped);
                self.push(".items)");
            }
            "sort" => {
                // arr.sort() → std.mem.sort with JsAny comparator using .lt()
                self.push(&format!(
                    "std.mem.sort(JsAny, {}.items, {{}}, (struct {{ fn lessThan(_: void, a: JsAny, b: JsAny) bool {{ return a.lt(b); }} }}).lessThan)",
                    escaped
                ));
            }
            "splice" => {
                // arr.splice(start, deleteCount, ...items)
                // Only delete-removal (0–2 args) is fully supported; insertion is a compileError.
                if args.len() > 2 {
                    self.push("@compileError(\"splice with insert items not yet supported — use manual insert operations\")");
                } else {
                    self.push("(blk: {\n");
                    self.indent += 1;

                    let elem_ty = self.infer_dynamic_array_elem_type(obj_name);
                    self.emit_indent();
                    self.push("const _start: usize = @intCast(@max(0, ");
                    if let Some(arg0) = args.first() {
                        self.emit_arg(arg0);
                    } else {
                        self.push("0");
                    }
                    self.push("));\n");

                    self.emit_indent();
                    self.push("var _n: usize = @intCast(@max(0, ");
                    if args.len() >= 2 {
                        if let Some(arg1) = args.get(1) {
                            self.emit_arg(arg1);
                        } else {
                            self.push("0");
                        }
                    } else {
                        // splice(start) → deleteCount = all remaining
                        self.push(&format!("{}.items.len -| _start", escaped));
                    }
                    self.push("));\n");

                    self.emit_indent();
                    self.push(&format!("var _result = std.ArrayList({}).init(js_allocator.g_alloc());\n", elem_ty));
                    self.emit_indent();
                    self.push("defer _result.deinit();\n");

                    self.emit_indent();
                    self.push("while (_n > 0 and _start < ");
                    self.push(&escaped);
                    self.push(".items.len) : (_n -= 1) {\n");
                    self.indent += 1;

                    self.emit_indent();
                    self.push("_result.append(");
                    self.push(&escaped);
                    self.push(".orderedRemove(_start)) catch @panic(\"OOM\");\n");

                    self.indent -= 1;
                    self.emit_indent();
                    self.push("}\n");

                    self.emit_indent();
                    self.push("break :blk _result;\n");

                    self.indent -= 1;
                    self.emit_indent();
                    self.push("})");
                }
            }
            _ => {
                self.push("@compileError(\"unknown array method: ");
                self.push(method);
                self.push("\")");
            }
        }
    }

    /// Apply a builtin template by splitting on "{}" placeholders.
    pub(super) fn apply_builtin_template(
        &mut self,
        trans: &crate::builtins::BuiltinTranslation,
        args: &oxc_allocator::Vec<'_, Argument>,
    ) {
        let template = &trans.template;
        // Collect formatted arguments
        let formatted_args: Vec<String> = args
            .iter()
            .map(|arg| {
                // Use a temp codegen to format the arg
                let empty_exports = HashSet::new();
                let mut tmp = ZigCodegen {
                    output: String::new(),
                    indent: self.indent,
                    inferrer: TypeInferrer::new(), // dummy, not used for emit_arg
                    diagnostics: &mut Vec::new(),
                    in_top_level: self.in_top_level,
                    task_counter: self.task_counter,
                    builtins: self.builtins,
                    closure_map: HashMap::new(),
                    closure_struct_defs: HashMap::new(),
                    fn_closure_spans: HashMap::new(),
                    closure_counter: 0,
                    closure_structs: Vec::new(),
                    cabi_wrappers: Vec::new(),
                    cabi_exports: Vec::new(),
                    string_return_fns: HashSet::new(),
                    closure_vars: HashSet::new(),
                    current_fn: None,
                    exports: empty_exports,
                    try_label: None,
                    catch_label: None,
                    try_counter: self.try_counter,
                    temp_counter: 0,
                    destructure_prelude: Vec::new(),
                    current_class: None,
                    object_type_defs: Vec::new(),
                    current_obj_structs: Vec::new(),
                    init_globals_code: Vec::new(),
            source_map: crate::sourcemap::SourceMap::new(""),
            line_index: crate::sourcemap::LineIndex::new(""),
            source_file: String::new(),
                };
                tmp.emit_arg(arg);
                tmp.output
            })
            .collect();

        // Replace positional placeholders {0}, {1}, ... and sequential {}
        let mut result = String::new();
        let mut chars = template.chars().peekable();
        let mut seq_idx: usize = 0; // sequential argument index for {} placeholders
        while let Some(ch) = chars.next() {
            if ch == '{' {
                if let Some(&('0'..='9')) = chars.peek() {
                    let mut idx_str = String::new();
                    while let Some(&('0'..='9')) = chars.peek() {
                        idx_str.push(chars.next().expect("peek() guaranteed a digit"));
                    }
                    // Skip closing }
                    if chars.peek() == Some(&'}') {
                        chars.next();
                    }
                    if let Ok(idx) = idx_str.parse::<usize>()
                        && let Some(arg) = formatted_args.get(idx)
                    {
                        result.push_str(arg);
                    }
                } else if chars.peek() == Some(&'}') {
                    // {} → next sequential argument
                    chars.next();
                    if let Some(arg) = formatted_args.get(seq_idx) {
                        result.push_str(arg);
                    }
                    seq_idx += 1;
                } else {
                    result.push(ch);
                }
            } else {
                result.push(ch);
            }
        }

        self.push(&result);
    }
}
