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
                        self.emit_builtin_method_call(trans, obj_expr, &call.arguments);
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
        };
        tmp.emit_expr(receiver);
        all_args.push(tmp.output.clone());

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
            };
            tmp2.emit_arg(arg);
            all_args.push(tmp2.output.clone());
        }

        // Now apply template: {} = all_args joined, {0} = all_args[0], etc.
        let mut result = String::new();
        let mut chars = template.chars().peekable();
        let all_args_ref: Vec<&str> = all_args.iter().map(|s| s.as_str()).collect();
        while let Some(ch) = chars.next() {
            if ch == '{' {
                if let Some(&('0'..='9')) = chars.peek() {
                    let mut idx_str = String::new();
                    while let Some(&('0'..='9')) = chars.peek() {
                        idx_str.push(chars.next().unwrap());
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
                    result.push_str(&all_args_ref.join(", "));
                } else {
                    result.push(ch);
                }
            } else {
                result.push(ch);
            }
        }
        self.push(&result);
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
                // arr.push(val) → arr.append(js_allocator.g_alloc(), val) catch {};
                // Zig 0.16: do NOT return the new length (blk expression return value ignored error)
                self.push(&escaped);
                self.push(".append(js_allocator.g_alloc(), ");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { self.push(", "); }
                    self.emit_arg(arg);
                }
                self.push(") catch {}");
            }
            "pop" => {
                self.push(&escaped);
                self.push(".pop() orelse null");
            }
            "shift" => {
                self.push("(blk: { if (");
                self.push(&escaped);
                self.push(".items.len == 0) break :blk @as(?i64, null); break :blk ");
                self.push(&escaped);
                self.push(".orderedRemove(0); })");
            }
            "unshift" => {
                // arr.unshift(val) → arr.insert(js_allocator.g_alloc(), 0, val) catch {};
                // Zig 0.16: do NOT return the new length
                self.push(&escaped);
                self.push(".insert(js_allocator.g_alloc(), 0, ");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { self.push(", "); }
                    self.emit_arg(arg);
                }
                self.push(") catch {}");
            }
            "splice" | "sort" | "reverse" => {
                self.push("@compileError(\"");
                self.push(method);
                self.push(" not yet implemented for dynamic array\")");
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
                };
                tmp.emit_arg(arg);
                tmp.output
            })
            .collect();

        // Replace positional placeholders {0}, {1}, ...
        let mut result = String::new();
        let mut chars = template.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '{' {
                if let Some(&('0'..='9')) = chars.peek() {
                    let mut idx_str = String::new();
                    while let Some(&('0'..='9')) = chars.peek() {
                        idx_str.push(chars.next().unwrap());
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
                    // {} → all args comma-separated
                    chars.next();
                    result.push_str(&formatted_args.join(", "));
                } else {
                    result.push(ch);
                }
            } else {
                result.push(ch);
            }
        }

        // If no placeholders were found, result == template → just push template
        // Actually, some templates like `@abs({})` have `{}` → replace with first arg
        if result == *template {
            // Simple case: template has no positional args, use first arg
            if let Some(first) = formatted_args.first() {
                result = template.replace("{}", first);
            }
        }

        self.push(&result);
    }
}
