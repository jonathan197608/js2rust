use super::*;

impl<'a> ZigCodegen<'a> {
    pub(super) fn body_contains_await(body: &oxc_allocator::Box<'_, FunctionBody<'_>>) -> bool {
        for stmt in &body.statements {
            if Self::stmt_contains_await(stmt) {
                return true;
            }
        }
        false
    }

    /// Check if a statement contains any `AwaitExpression`.
    pub(super) fn stmt_contains_await(stmt: &Statement) -> bool {
        match stmt {
            Statement::ExpressionStatement(es) => Self::expr_contains_await(&es.expression),
            Statement::ReturnStatement(rs) => {
                rs.argument
                    .as_ref()
                    .map(|arg| Self::expr_contains_await(arg))
                    .unwrap_or(false)
            }
            Statement::VariableDeclaration(vd) => {
                for decl in &vd.declarations {
                    if let Some(init) = &decl.init
                        && Self::expr_contains_await(init)
                    {
                        return true;
                    }
                }
                false
            }
            Statement::IfStatement(if_stmt) => {
                Self::expr_contains_await(&if_stmt.test)
                    || Self::stmt_contains_await(&if_stmt.consequent)
                    || if_stmt
                        .alternate
                        .as_ref()
                        .map(|alt| Self::stmt_contains_await(alt))
                        .unwrap_or(false)
            }
            Statement::BlockStatement(block) => {
                block.body.iter().any(|s| Self::stmt_contains_await(s))
            }
            Statement::ForStatement(_)
            | Statement::WhileStatement(_)
            | Statement::DoWhileStatement(_)
            | Statement::ForInStatement(_)
            | Statement::ForOfStatement(_)
            | Statement::SwitchStatement(_)
            | Statement::TryStatement(_) => true, // conservatively assume await
            _ => false,
        }
    }

    /// Check if an expression contains any `AwaitExpression`.
    pub(super) fn expr_contains_await(expr: &Expression) -> bool {
        match expr {
            Expression::AwaitExpression(_) => true,
            Expression::CallExpression(call) => {
                Self::expr_contains_await(&call.callee)
                    || call.arguments.iter().any(|arg| match arg {
                        Argument::SpreadElement(s) => Self::expr_contains_await(&s.argument),
                        _ => arg
                            .as_expression()
                            .map(|e| Self::expr_contains_await(e))
                            .unwrap_or(false),
                    })
            }
            Expression::BinaryExpression(bin) => {
                Self::expr_contains_await(&bin.left) || Self::expr_contains_await(&bin.right)
            }
            Expression::UnaryExpression(unary) => Self::expr_contains_await(&unary.argument),
            Expression::LogicalExpression(logic) => {
                Self::expr_contains_await(&logic.left)
                    || Self::expr_contains_await(&logic.right)
            }
            Expression::ParenthesizedExpression(p) => Self::expr_contains_await(&p.expression),
            Expression::ConditionalExpression(cond) => {
                Self::expr_contains_await(&cond.test)
                    || Self::expr_contains_await(&cond.consequent)
                    || Self::expr_contains_await(&cond.alternate)
            }
            Expression::AssignmentExpression(assign) => {
                Self::expr_contains_await(&assign.right)
            }
            Expression::SequenceExpression(seq) => {
                seq.expressions.iter().any(|e| Self::expr_contains_await(e))
            }
            _ => false,
        }
    }

    // --- Function declarations ---

    pub(super) fn emit_fn_decl(&mut self, fd: &Function) {
        let raw_name = fd.id.as_ref().map(|id| id.name.as_str()).unwrap_or("anonymous");
        let name = Self::escape_keyword(raw_name);
        let is_async = fd.r#async;
        let is_export = self.exports.contains(raw_name);

        // Generate struct definitions for Object-typed parameters BEFORE the function signature.
        // This must run before any emit_params call so the structs are in scope.
        self.current_obj_structs.clear();
        let param_types = self.inferrer.get_fn_param_types(raw_name);
        let mut obj_defs: Vec<String> = Vec::new();
        for (i, ptype) in param_types.iter().enumerate() {
            // Generate struct definitions for Object-typed parameters.
            // Any fields are filtered out (same as top-level const objects).
            if let ZigType::Object { fields } = ptype {
                // Keep only fields with known (non-Any) types.
                let known: Vec<(String, ZigType)> = fields
                    .iter()
                    .filter(|(_, ty)| *ty != ZigType::JsValue && *ty != ZigType::JsAny)
                    .map(|(n, ty)| (n.clone(), ty.clone()))
                    .collect();
                if known.is_empty() {
                    continue;
                }
                let pname = if i < fd.params.items.len() {
                    self.binding_name(&fd.params.items[i].pattern).to_string()
                } else {
                    format!("arg{}", i)
                };
                let struct_name = format!(
                    "{}{}",
                    Self::capitalize_first(raw_name),
                    Self::capitalize_first(&pname),
                );
                let def = Self::gen_obj_struct_def(&struct_name, &known);
                obj_defs.push(def);
                if i >= self.current_obj_structs.len() {
                    self.current_obj_structs.resize_with(i + 1, || None);
                }
                self.current_obj_structs[i] = Some(struct_name);
            }
        }
        for def in &obj_defs {
            self.push(def);
        }

        // Build return type string
        let ret_type_str = if let Some(&span) = self.fn_closure_spans.get(raw_name)
            && let Some(ci) = self.closure_map.get(&span)
        {
            ci.struct_name.clone()
        } else {
            self.inferrer.get_fn_return_type(raw_name).to_zig_str()
        };

        // Async functions cannot use C ABI (error union return not C-compatible).
        // For async exports: keep as `pub fn` (Zig-only, no callconv).
        if is_async && is_export {
            self.emit_indent();
            self.push("pub fn ");
            self.push(&name);
            self.push("(");
            self.push("io: Io");
            if !fd.params.items.is_empty() {
                self.push(", ");
            }
            self.emit_params(&fd.params, Some(raw_name));
            self.push(") !");
            self.push(&ret_type_str);
            self.push(" ");
            self.emit_fn_body(fd, raw_name, true);
            return;
        }

        // Determine if this sync export needs a C ABI wrapper
        let needs_cabi_wrapper;
        let param_types: Vec<ZigType>;
        let ret_type: ZigType;
        if is_export && !is_async {
            param_types = self.inferrer.get_fn_param_types(raw_name);
            let has_string_param = param_types.contains(&ZigType::String);
            ret_type = self.inferrer.get_fn_return_type(raw_name);
            let returns_string = ret_type == ZigType::String;
            let returns_closure = self.fn_closure_spans.contains_key(raw_name);
            needs_cabi_wrapper = has_string_param || returns_string || returns_closure;
        } else {
            param_types = Vec::new();
            ret_type = ZigType::JsValue;
            needs_cabi_wrapper = false;
        };

        if needs_cabi_wrapper {
            // Emit internal impl function (pub so lib.zig can call it directly)
            self.emit_indent();
            self.push("pub fn ");
            self.push(&name);
            self.push("_impl(");
            self.emit_params(&fd.params, Some(raw_name));
            self.push(") ");
            self.push(&ret_type_str);
            self.push(" ");
            self.emit_fn_body(fd, raw_name, false);

            // Buffer C ABI wrapper
            let wrapper = self.generate_cabi_wrapper(raw_name, &name, fd, &ret_type_str);
            self.cabi_wrappers.push(wrapper);

            // Record C ABI export metadata
            let returns_string = ret_type == ZigType::String;
            let returns_closure = self.fn_closure_spans.contains_key(raw_name);
            let mut params: Vec<(String, ZigType)> = Vec::new();
            for (i, p) in fd.params.items.iter().enumerate() {
                let pname = Self::escape_keyword(self.binding_name(&p.pattern));
                let ptype = if i < param_types.len() {
                    param_types[i].clone()
                } else {
                    ZigType::JsValue
                };
                params.push((pname, ptype));
            }
            self.cabi_exports.push(CabiExport {
                name: name.clone(),
                params,
                ret_type: ret_type.clone(),
                has_free_func: returns_string || returns_closure,
            });
        } else if is_export {
            // Simple export: no string/closure types, use direct C ABI.
            // NOTE: `pub fn` not `pub export fn` — the orchestrator lib.zig
            // re-exports via `comptime { @export(...) }` which is required by
            // Zig's linking model: only root-module exports reach the .lib.
            // For JsValue/JsAny returns, skip callconv(.c) since unions
            // can't be passed over C ABI. The lib.zig wrapper handles .int extraction.
            let is_js_obj_ret = matches!(ret_type, ZigType::JsValue | ZigType::JsAny);
            self.emit_indent();
            self.push("pub fn ");
            self.push(&name);
            self.push("(");
            self.emit_params(&fd.params, Some(raw_name));
            self.push(") ");
            if !is_js_obj_ret {
                self.push("callconv(.c) ");
            }
            self.push(&ret_type_str);
            self.push(" ");
            self.emit_fn_body(fd, raw_name, false);

            // Record simple C ABI export metadata
            let mut params: Vec<(String, ZigType)> = Vec::new();
            for (i, p) in fd.params.items.iter().enumerate() {
                let pname = Self::escape_keyword(self.binding_name(&p.pattern));
                let ptype = if i < param_types.len() {
                    param_types[i].clone()
                } else {
                    ZigType::JsValue
                };
                params.push((pname, ptype));
            }
            self.cabi_exports.push(CabiExport {
                name: name.clone(),
                params,
                ret_type: ret_type.clone(),
                has_free_func: false,
            });
        } else {
            // Non-exported function — `pub` so orchestrator can re-export for tests.
            self.emit_indent();
            if is_async {
                self.push("pub fn ");
                self.push(&name);
                self.push("(");
                self.push("io: Io");
                if !fd.params.items.is_empty() {
                    self.push(", ");
                }
                self.emit_params(&fd.params, Some(raw_name));
                self.push(") !");
                self.push(&ret_type_str);
                self.push(" ");
                self.emit_fn_body(fd, raw_name, true);
            } else {
                self.push("pub fn ");
                self.push(&name);
                self.push("(");
                self.emit_params(&fd.params, Some(raw_name));
                self.push(") ");
                self.push(&ret_type_str);
                self.push(" ");
                self.emit_fn_body(fd, raw_name, false);
            }
        }
    }

    /// Emit function body block
    pub(super) fn emit_fn_body(&mut self, fd: &Function, raw_name: &str, is_async: bool) {
        if let Some(body) = &fd.body {
            self.push("{\n");
            self.indent += 1;

            // Emit destructured parameter prelude statements
            // e.g., for `function foo({a, b})`: `const a = _arg0.a; const b = _arg0.b;`
            for prelude in self.destructure_prelude.drain(..) {
                self.output.push_str(&prelude);
            }

            if is_async && !Self::body_contains_await(body) {
                self.emit_indent();
                self.push_line("_ = io;");
            }
            let prev = self.in_top_level;
            self.in_top_level = false;
            let prev_fn = self.current_fn.take();
            self.current_fn = Some(raw_name.to_string());
            // Also set inferrer.current_fn so get_var_type() can look up fn_local_types
            let prev_infer_fn = self.inferrer.current_fn.take();
            self.inferrer.current_fn = Some(raw_name.to_string());
            for stmt in &body.statements {
                self.emit_stmt(stmt);
            }
            self.inferrer.current_fn = prev_infer_fn;
            self.current_fn = prev_fn;
            self.in_top_level = prev;
            self.indent -= 1;
            self.push_line("}");
        } else {
            self.push("{};\n");
        }
        self.push("\n");
    }

    /// Generate a C ABI export wrapper for a sync function with string params/returns or closures.
    pub(super) fn generate_cabi_wrapper(
        &mut self,
        raw_name: &str,
        escaped_name: &str,
        fd: &Function,
        ret_type_str: &str,
    ) -> String {
        let param_types = self.inferrer.get_fn_param_types(raw_name);
        let ret_type = self.inferrer.get_fn_return_type(raw_name);
        let returns_string = ret_type == ZigType::String;
        let returns_closure = self.fn_closure_spans.contains_key(raw_name);
        let returns_js_obj = matches!(ret_type, ZigType::JsValue | ZigType::JsAny);

        let mut w = String::new();
        // NOTE: `pub fn` not `pub export fn` — orchestrator lib.zig handles export via @export
        w.push_str(&format!("pub fn {}(", escaped_name));

        // C ABI params (no async)
        let mut cabi_params: Vec<String> = Vec::new();
        for (i, param) in fd.params.items.iter().enumerate() {
            let pname = self.binding_name(&param.pattern);
            let safe_pname = Self::escape_keyword(pname);
            let ptype = if i < param_types.len() {
                param_types[i].clone()
            } else {
                ZigType::JsValue
            };
            if ptype == ZigType::String {
                cabi_params.push(format!("{}: [*:0]const u8", safe_pname));
            } else {
                cabi_params.push(format!("{}: {}", safe_pname, ptype.to_zig_str()));
            }
        }
        w.push_str(&cabi_params.join(", "));
        w.push_str(") callconv(.c) ");

        if returns_string {
            w.push_str("[*:0]const u8");
        } else if returns_closure {
            w.push_str("*anyopaque");
        } else if returns_js_obj {
            w.push_str("i64");
        } else {
            w.push_str(ret_type_str);
        }
        w.push_str(" {\n");

        // Body: convert C strings → Zig slices
        for (i, param) in fd.params.items.iter().enumerate() {
            let pname = self.binding_name(&param.pattern);
            let safe_pname = Self::escape_keyword(pname);
            let ptype = if i < param_types.len() {
                param_types[i].clone()
            } else {
                ZigType::JsValue
            };
            if ptype == ZigType::String {
                w.push_str(&format!(
                    "    const {}_slice: []const u8 = std.mem.span({});\n",
                    safe_pname, safe_pname
                ));
            }
        }

        // Call impl
        w.push_str("    ");
        if returns_string || returns_closure || returns_js_obj {
            w.push_str("const _result = ");
        } else if ret_type_str != "void" {
            w.push_str("return ");
        }
        w.push_str(&format!("{}_impl(", escaped_name));
        let mut call_args: Vec<String> = Vec::new();
        for (i, param) in fd.params.items.iter().enumerate() {
            let pname = self.binding_name(&param.pattern);
            let safe_pname = Self::escape_keyword(pname);
            let ptype = if i < param_types.len() {
                param_types[i].clone()
            } else {
                ZigType::JsValue
            };
            if ptype == ZigType::String {
                call_args.push(format!("{}_slice", safe_pname));
            } else {
                call_args.push(safe_pname);
            }
        }
        w.push_str(&call_args.join(", "));
        w.push_str(");\n");

        // Handle string return
        if returns_string {
            w.push_str("    return @ptrCast(_result.ptr);\n");
        }

        // Handle JsValue/JsAny return: extract .int field for C ABI compatibility
        if returns_js_obj {
            w.push_str("    return _result.int;\n");
        }

        // Handle closure return: allocate on heap, return opaque pointer
        if returns_closure {
            w.push_str("    const alloc = js_allocator.g_alloc();\n");
            w.push_str("    const ptr = alloc.create(@TypeOf(_result)) catch @panic(\"OOM\");\n");
            w.push_str("    ptr.* = _result;\n");
            w.push_str("    return @ptrCast(ptr);\n");
        }

        w.push_str("}\n\n");

        // Generate free_xxx for string returns
        if returns_string {
            w.push_str(&format!(
                "pub fn free_{}(ptr: [*:0]const u8) callconv(.c) void {{\n    _ = js_allocator.g_alloc().free(std.mem.span(ptr));\n}}\n\n",
                escaped_name
            ));
        }

        // Generate free_xxx for closure returns
        if returns_closure {
            w.push_str(&format!(
                "pub fn free_{}(ptr: *anyopaque) callconv(.c) void {{\n    const alloc = js_allocator.g_alloc();\n    const typed: *{} = @ptrCast(@alignCast(ptr));\n    alloc.destroy(typed);\n}}\n\n",
                escaped_name, ret_type_str
            ));
        }

        if returns_string || returns_closure {
            self.string_return_fns.insert(raw_name.to_string());
        }

        w
    }

    // ========== Class Support ==========

    pub(super) fn collect_class_fields(body: &FunctionBody) -> Vec<String> {
        let mut fields = Vec::new();
        for stmt in &body.statements {
            if let Statement::ExpressionStatement(es) = stmt
                && let Expression::AssignmentExpression(assign) = &es.expression
                && let AssignmentTarget::StaticMemberExpression(mem) = &assign.left
                && matches!(mem.object, Expression::ThisExpression(_))
            {
                let name = mem.property.name.to_string();
                if !fields.contains(&name) {
                    fields.push(name);
                }
            }
        }
        fields
    }

    pub(super) fn emit_class_decl(&mut self, cd: &Class) {
        let raw_name = cd
            .id
            .as_ref()
            .map(|id| id.name.as_str())
            .unwrap_or("Anonymous");
        let name = Self::escape_keyword(raw_name);
        let is_export = self.exports.contains(raw_name);

        // Collect fields from constructor AND property definitions
        let mut fields = Vec::new();
        let mut field_defaults: Vec<(String, Option<&Expression>)> = Vec::new();
        let mut methods: Vec<&MethodDefinition> = Vec::new();
        let mut static_methods: Vec<&MethodDefinition> = Vec::new();
        let mut static_fields: Vec<(&str, Option<&Expression>)> = Vec::new();
        let mut constructor: Option<&MethodDefinition> = None;
        // Track parent class for extends
        let super_class = cd.super_class.as_ref();

        for elem in &cd.body.body {
            match elem {
                ClassElement::MethodDefinition(md) => {
                    if md.r#static {
                        static_methods.push(md);
                    } else {
                        match &md.key {
                            PropertyKey::StaticIdentifier(id) if id.name.as_str() == "constructor" => {
                                constructor = Some(md);
                                if let Some(body) = &md.value.body {
                                    fields = Self::collect_class_fields(body);
                                }
                            }
                            _ => {
                                methods.push(md);
                            }
                        }
                    }
                }
                ClassElement::PropertyDefinition(pd) => {
                    if let PropertyKey::StaticIdentifier(id) = &pd.key {
                        if pd.r#static {
                            static_fields.push((id.name.as_str(), pd.value.as_ref()));
                        } else {
                            let fname = id.name.to_string();
                            if !fields.contains(&fname) {
                                fields.push(fname.clone());
                                field_defaults.push((fname, pd.value.as_ref()));
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // Emit struct definition
        let vis = if is_export { "pub const" } else { "const" };
        self.push(&format!("{} {} = struct {{\n", vis, name));
        self.indent += 1;

        // Embed parent struct for extends
        if let Some(super_expr) = super_class {
            self.emit_indent();
            self.push("base: ");
            self.emit_expr(super_expr);
            self.push(",\n");
        }

        // Fields
        if !fields.is_empty() {
            for (i, f) in fields.iter().enumerate() {
                self.emit_indent();
                self.push(f);
                self.push(": i64");
                if i < fields.len() - 1 || constructor.is_some() || !methods.is_empty() || !static_methods.is_empty() {
                    self.push(",");
                }
                self.push("\n");
            }
            if constructor.is_some() || !methods.is_empty() || !static_methods.is_empty() {
                self.push("\n");
            }
        }

        // Static fields → `pub const field = value;`
        for (sf_name, sf_val) in &static_fields {
            self.emit_indent();
            self.push("pub const ");
            self.push(sf_name);
            if let Some(val) = sf_val {
                self.push(" = ");
                self.emit_expr(val);
            } else {
                self.push(" = undefined");
            }
            self.push(";\n");
        }
        if !static_fields.is_empty() {
            self.push("\n");
        }

        // Emit constructor inside struct
        if let Some(cons) = constructor {
            self.emit_class_method(&name, cons, &fields, true, super_class.is_some());
        }

        // Emit methods inside struct (including getters/setters)
        for method in &methods {
            self.emit_class_method(&name, method, &fields, false, super_class.is_some());
        }

        // Emit static methods (no self parameter)
        for method in &static_methods {
            self.emit_static_class_method(method);
        }

        self.indent -= 1;
        self.emit_indent();
        self.push("};\n\n");
    }

    pub(super) fn emit_class_method(&mut self, struct_name: &str, md: &MethodDefinition, fields: &[String], is_constructor: bool, _has_super: bool) {
        let prev_class = self.current_class.take();
        self.current_class = Some((struct_name.to_string(), fields.to_vec()));

        let method_name = match &md.key {
            PropertyKey::StaticIdentifier(id) => id.name.as_str().to_string(),
            _ => "unknown".to_string(),
        };

        // Determine the emitted function name
        let escaped_name = if is_constructor {
            "init".to_string()
        } else {
            // Getter/setter: prefix with get_/set_ to avoid name collision
            match md.kind {
                MethodDefinitionKind::Get => format!("get_{}", Self::escape_keyword(&method_name)),
                MethodDefinitionKind::Set => format!("set_{}", Self::escape_keyword(&method_name)),
                _ => Self::escape_keyword(&method_name),
            }
        };

        let fd = &md.value;

        // Infer return type
        let ret_type = if is_constructor {
            struct_name.to_string()
        } else {
            // Try to infer return type from method body
            // If inference fails, Any.to_zig_str() returns "JsValue"
            // which is undefined in generated Zig code → compile error
            let body_ret = self.inferrer.infer_return_type_from_function_body(&fd.body);
            body_ret.to_zig_str()
        };

        self.emit_indent();
        self.push("pub fn ");
        self.push(&escaped_name);
        self.push("(");

        if is_constructor {
            // Constructor: no self param, creates instance from scratch
            // All class fields are i64, so constructor params assigned to
            // this.field also get i64 (no need for is_fallback defaults).
            self.emit_constructor_params(&fd.params);
        } else {
            // Regular method: self pointer as first param
            self.push("self: *const ");
            self.push(struct_name);
            if !fd.params.items.is_empty() {
                self.push(", ");
                self.emit_params(&fd.params, None);
            }
        }

        self.push(") ");
        self.push(&ret_type);
        self.push(" ");

        // Emit body
        if let Some(body) = &fd.body {
            self.push("{\n");
            self.indent += 1;

            if is_constructor {
                // Inject `var self: StructName = undefined;` so `this.x = val` → `self.x = val` works
                self.emit_indent();
                self.push(&format!("var self: {} = undefined;\n", struct_name));
            }

            let prev = self.in_top_level;
            self.in_top_level = false;
            for stmt in &body.statements {
                self.emit_stmt(stmt);
            }

            if is_constructor {
                // Ensure constructor returns the initialized instance
                self.emit_indent();
                self.push("return self;\n");
            }

            self.in_top_level = prev;
            self.indent -= 1;
            self.emit_indent();
            self.push("}\n\n");
        } else {
            self.push("{};\n\n");
        }

        self.current_class = prev_class;
    }

    /// Emit a static class method (no `self` parameter).
    pub(super) fn emit_static_class_method(&mut self, md: &MethodDefinition) {
        let method_name = match &md.key {
            PropertyKey::StaticIdentifier(id) => id.name.as_str().to_string(),
            _ => "unknown".to_string(),
        };
        let escaped_name = Self::escape_keyword(&method_name);
        let fd = &md.value;

        // Infer return type
        let body_ret = self.inferrer.infer_return_type_from_function_body(&fd.body);
        let ret_type = body_ret.to_zig_str();

        self.emit_indent();
        self.push("pub fn ");
        self.push(&escaped_name);
        self.push("(");
        // Static methods: no self parameter, just regular params
        self.emit_params(&fd.params, None);
        self.push(") ");
        self.push(&ret_type);
        self.push(" ");

        // Emit body
        if let Some(body) = &fd.body {
            self.push("{\n");
            self.indent += 1;
            let prev = self.in_top_level;
            self.in_top_level = false;
            for stmt in &body.statements {
                self.emit_stmt(stmt);
            }
            self.in_top_level = prev;
            self.indent -= 1;
            self.emit_indent();
            self.push("}\n\n");
        } else {
            self.push("{};\n\n");
        }
    }

    pub(super) fn emit_constructor_params(&mut self, params: &FormalParameters) {
        self.destructure_prelude.clear();

        for (i, param) in params.items.iter().enumerate() {
            if i > 0 {
                self.push(", ");
            }

            if !is_simple_binding(&param.pattern) {
                // Destructured pattern: keep Any (no field-based inference)
                let arg_name = format!("_arg{}", i);
                self.push(&arg_name);
                self.push(": ");
                self.push(&ZigType::JsValue.to_zig_str());

                let mut leaves = Vec::new();
                flatten_binding_pattern(&param.pattern, &arg_name, &mut leaves);
                let mut prelude = String::new();
                for leaf in &leaves {
                    let escaped = Self::escape_keyword(leaf.name);
                    let indent_str = self.get_indent_str(self.indent + 1);
                    prelude.push_str(&format!(
                        "{}const {} = {};\n",
                        indent_str, escaped, leaf.access
                    ));
                }
                if !prelude.is_empty() {
                    self.destructure_prelude.push(prelude);
                }

                if let Some(default) = &param.initializer {
                    self.push(" = ");
                    self.emit_expr(default);
                }
                continue;
            }

            let raw_name: String = self.binding_name(&param.pattern).to_owned();
            let name = Self::escape_keyword(&raw_name);
            self.push(&name);
            self.push(": ");

            // Constructor params default to i64 (matching class field types),
            // unless a default value provides a different type.
            let ty = if let Some(default) = &param.initializer {
                self.inferrer.infer_expr(default)
            } else {
                ZigType::I64
            };
            self.push(&ty.to_zig_str());

            if let Some(default) = &param.initializer {
                self.push(" = ");
                self.emit_expr(default);
            }
        }
    }

    pub(super) fn emit_params(&mut self, params: &FormalParameters, fn_name: Option<&str>) {
        // Clear any previous prelude
        self.destructure_prelude.clear();

        for (i, param) in params.items.iter().enumerate() {
            if i > 0 {
                self.push(", ");
            }

            // Check if this parameter has a destructured pattern (ObjectPattern/ArrayPattern)
            if !is_simple_binding(&param.pattern) {
                // Generate a synthetic parameter name: _arg0, _arg1, etc.
                let arg_name = format!("_arg{}", i);
                self.push(&arg_name);
                self.push(": ");
                self.push(&ZigType::JsValue.to_zig_str());

                // Generate body prelude: unpack destructured fields
                let mut leaves = Vec::new();
                flatten_binding_pattern(&param.pattern, &arg_name, &mut leaves);
                let mut prelude = String::new();
                for leaf in &leaves {
                    let escaped = Self::escape_keyword(leaf.name);
                    let indent = self.get_indent_str(self.indent + 1);
                    prelude.push_str(&format!(
                        "{}const {} = {};\n",
                        indent, escaped, leaf.access
                    ));
                }
                if !prelude.is_empty() {
                    self.destructure_prelude.push(prelude);
                }

                // Handle param default value
                if let Some(default) = &param.initializer {
                    self.push(" = ");
                    self.emit_expr(default);
                }
                continue;
            }

            let raw_name: String = self.binding_name(&param.pattern).to_owned();
            let name = Self::escape_keyword(&raw_name);
            self.push(&name);
            self.push(": ");

            let ty = if let Some(fn_name) = fn_name {
                let param_types = self.inferrer.get_fn_param_types(fn_name);
                if i < param_types.len() {
                    // Use inferred type even if it's Any (will become "JsValue" in output)
                    param_types[i].clone()
                } else if let Some(default) = &param.initializer {
                    self.inferrer.infer_expr(default)
                } else {
                    ZigType::JsValue // inference failed → Zig compile error
                }
            } else if let Some(default) = &param.initializer {
                self.inferrer.infer_expr(default)
            } else {
                ZigType::JsValue // inference failed → Zig compile error
            };

            let type_str = if i < self.current_obj_structs.len() {
                if let Some(Some(s)) = self.current_obj_structs.get(i) {
                    s.clone()
                } else {
                    ty.to_zig_str()
                }
            } else {
                ty.to_zig_str()
            };
            self.push(&type_str);

            if let Some(default) = &param.initializer {
                self.push(" = ");
                self.emit_expr(default);
            }
        }

        // Handle rest parameter: ...args → args: []const i64
        if let Some(rest) = &params.rest {
            if !params.items.is_empty() {
                self.push(", ");
            }
            let rest_name = self.binding_name(&rest.rest.argument);
            let escaped = Self::escape_keyword(rest_name);
            self.push(&escaped);
            self.push(": []const i64");
        }
    }
}
