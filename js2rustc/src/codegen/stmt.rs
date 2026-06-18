use super::*;

impl<'a> ZigCodegen<'a> {
    pub(super) fn emit_stmt(&mut self, stmt: &Statement) {
        // Top-level: only VariableDeclaration and FunctionDeclaration (and ClassDeclaration) are allowed
        if self.in_top_level {
            match stmt {
                Statement::VariableDeclaration(vd) => {
                    // Record source mapping for top-level variable declarations
                    if let Some(first_decl) = vd.declarations.first() {
                        let name = self.binding_name(&first_decl.id);
                        if !name.starts_with("test_") {
                            self.record_src(vd.span.start, &format!("const {}", name));
                        }
                    }
                    self.emit_var_decl(vd);
                }
                Statement::FunctionDeclaration(fd) => {
                    let fn_name = fd.id.as_ref()
                        .map(|id| id.name.as_str())
                        .unwrap_or("anonymous");
                    if !fn_name.starts_with("test_") {
                        self.record_src(fd.span.start, &format!("fn {}", fn_name));
                    }
                    self.emit_fn_decl(fd);
                }
                Statement::ClassDeclaration(cd) => {
                    let class_name = cd.id.as_ref()
                        .map(|id| id.name.as_str())
                        .unwrap_or("anonymous");
                    self.record_src(cd.span.start, &format!("class {}", class_name));
                    self.emit_class_decl(cd);
                }
                Statement::ExpressionStatement(es) => {
                    self.diagnostics.push(
                        crate::infer::Diagnostic::new(
                            crate::infer::DiagnosticKind::Error,
                            "top-level expression statements are not allowed; \
                                  use a variable declaration or function declaration instead"
                                .to_string(),
                        )
                        .with_span(es.span.start as usize, es.span.end as usize),
                    );
                }
                _ => {
                    self.diagnostics.push(crate::infer::Diagnostic::new(
                        crate::infer::DiagnosticKind::Error,
                        format!(
                            "only variable declarations and function declarations are allowed \
                             at top level, found: {:?}",
                            std::mem::discriminant(stmt)
                        ),
                    ));
                }
            }
            return;
        }

        // Inside function body: reject nested FunctionDeclaration
        if matches!(stmt, Statement::FunctionDeclaration(_)) {
            self.push_line("// ERROR: nested function declarations are not allowed");
            self.diagnostics.push(crate::infer::Diagnostic::new(
                crate::infer::DiagnosticKind::Error,
                "nested function declarations are not allowed".to_string(),
            ));
            return;
        }

        match stmt {
            Statement::VariableDeclaration(vd) => self.emit_var_decl(vd),
            Statement::FunctionDeclaration(fd) => self.emit_fn_decl(fd),
            Statement::ExpressionStatement(es) => {
                self.emit_indent();
                // Zig 0.16: do NOT use `_ = expr;` for expression statements.
                // This causes "error set is discarded" error.
                self.emit_expr(&es.expression);
                self.push(";\n");
            }
            Statement::ReturnStatement(rs) => {
                self.emit_indent();
                if let Some(ref label) = self.catch_label {
                    // Inside a catch block: break to catch label (provides default value)
                    self.push(&format!("break :{} ", label));
                } else if let Some(ref label) = self.try_label {
                    // Inside a try block: break to the try label
                    self.push(&format!("break :{} ", label));
                } else {
                    self.push("return ");
                }
                if let Some(arg) = &rs.argument {
                    self.emit_expr(arg);
                }
                self.push(";\n");
            }
            Statement::IfStatement(if_stmt) => self.emit_if_stmt(if_stmt),
            Statement::BlockStatement(block) => {
                self.push_line("{");
                self.indent += 1;
                for s in &block.body {
                    self.emit_stmt(s);
                }
                self.indent -= 1;
                self.push_line("}");
            }
            Statement::ForStatement(fs) => self.emit_for_stmt(fs),
            Statement::ForInStatement(fis) => self.emit_for_in_stmt(fis),
            Statement::ForOfStatement(fos) => self.emit_for_of_stmt(fos),
            Statement::WhileStatement(ws) => self.emit_while_stmt(ws),
            Statement::DoWhileStatement(dw) => self.emit_do_while_stmt(dw),
            Statement::EmptyStatement(_) => {}
            Statement::BreakStatement(bs) => {
                self.emit_indent();
                if let Some(label) = &bs.label {
                    self.push(&format!("break :{}", Self::escape_keyword(label.name.as_str())));
                } else {
                    self.push("break");
                }
                self.push(";\n");
            }
            Statement::ContinueStatement(cs) => {
                self.emit_indent();
                if let Some(label) = &cs.label {
                    self.push(&format!("continue :{}", Self::escape_keyword(label.name.as_str())));
                } else {
                    self.push("continue");
                }
                self.push(";\n");
            }
            Statement::SwitchStatement(sw) => self.emit_switch_stmt(sw),
            Statement::ThrowStatement(_throw_stmt) => {
                self.emit_indent();
                if let Some(ref label) = self.try_label {
                    // Inside a try block: break to the try label with an error
                    self.push(&format!("break :{} error.Unexpected", label));
                } else {
                    // Outside try block: return error (will be caught by caller)
                    self.push("return error.Unexpected");
                }
                self.push(";\n");
            }
            Statement::TryStatement(ts) => self.emit_try_stmt(ts),
            Statement::LabeledStatement(ls) => {
                // JS `label: stmt` → Zig labeled block/loop
                let label = Self::escape_keyword(ls.label.name.as_str());
                match &ls.body {
                    // Loop statements: apply label directly to the Zig loop
                    Statement::ForStatement(_)
                    | Statement::ForInStatement(_)
                    | Statement::ForOfStatement(_)
                    | Statement::WhileStatement(_)
                    | Statement::DoWhileStatement(_) => {
                        // Push label prefix, then emit the loop body
                        self.emit_indent();
                        self.push(&format!("{}: ", label));
                        // Remove indent from the loop emission since we already emitted it
                        // Actually, emit the labeled body directly
                        self.emit_labeled_loop_body(&ls.body, &label);
                    }
                    // Non-loop: wrap in a labeled block
                    _ => {
                        self.emit_indent();
                        self.push(&format!("{}: {{\n", label));
                        self.indent += 1;
                        self.emit_stmt(&ls.body);
                        self.indent -= 1;
                        self.emit_indent();
                        self.push("}\n");
                    }
                }
            }
            _ => {
                self.push_line("@compileError(\"unsupported statement type\");");
            }
        }
    }

    pub(super) fn emit_var_decl(&mut self, vd: &VariableDeclaration) {
        for decl in &vd.declarations {
            // Handle destructured patterns: flatten into individual variable declarations
            if !is_simple_binding(&decl.id) {
                self.emit_destructured_var_decl(decl, vd.kind);
                continue;
            }
            let name = self.binding_name(&decl.id);
            // Skip test_ variables — test generation helpers, stripped from output
            if name.starts_with("test_") {
                continue;
            }

            if let Some(init) = &decl.init {
                match init {
                    Expression::ArrowFunctionExpression(arrow) => {
                        // Check if this arrow is a closure (has captured variables)
                        let maybe_ci = self.closure_map.get(&arrow.span.start).cloned();
                        if let Some(ci) = maybe_ci
                            && !ci.captured.is_empty()
                        {
                            self.emit_closure_var_init(name, &ci);
                            continue;
                        }
                        self.emit_arrow_fn(name, arrow);
                        continue;
                    }
                    Expression::FunctionExpression(fe) => {
                        self.emit_fn_from_func_expr(name, fe);
                        continue;
                    }
                    Expression::ObjectExpression(obj) if self.in_top_level => {
                        // Check if this variable needs dynamic access (HashMap instead of struct)
                        if self.inferrer.get_dynamic_access_vars().contains(name) {
                            self.emit_dynamic_access_var_decl(name);
                            self.emit_dynamic_access_var_init_code(name, obj);
                            continue;
                        }
                        let obj_type = self.inferrer.infer_expr(init);
                        if let ZigType::Object { ref fields } = obj_type
                            && !fields.is_empty()
                            && fields.iter().all(|(_, ty)| *ty != ZigType::JsValue && *ty != ZigType::JsAny)
                        {
                            let kw = match vd.kind {
                                VariableDeclarationKind::Const => "const",
                                _ => "var",
                            };
                            let escaped_name = Self::escape_keyword(name);
                            let struct_name = Self::capitalize_first(name);
                            let def = Self::gen_obj_struct_def(&struct_name, fields);
                            self.object_type_defs.push(def);

                            self.emit_indent();
                            self.push(kw);
                            self.push(" ");
                            self.push(&escaped_name);
                            self.push(": ");
                            self.push(&struct_name);
                            self.push(" = .{ ");
                            let mut first = true;
                            for prop in &obj.properties {
                                if let ObjectPropertyKind::ObjectProperty(p) = prop {
                                    if !first { self.push(", "); }
                                    first = false;
                                    self.push(".");
                                    let key_str = property_key_name(&p.key);
                                    self.push(&key_str);
                                    self.push(" = ");
                                    self.emit_expr(&p.value);
                                }
                            }
                            self.push(" };\n");
                            continue;
                        }
                        // Fall through to generic anonymous .{} emission
                        // (occurs when fields include functions or other unresolvable types)
                    }
                    _ => {}
                }
            }

            // Dynamic array: use std.ArrayList(JsAny) for heterogeneous element support
            if self.inferrer.is_dynamic_array(name)
                && let Some(init) = &decl.init
            {
                let et = "JsAny".to_string();
                let escaped = Self::escape_keyword(name);

                // var name = std.ArrayList(JsAny).empty; // Zig 0.16 correct initialization
                self.emit_indent();
                self.push("var ");
                self.push(&escaped);
                self.push(" = std.ArrayList(");
                self.push(&et);
                self.push(").empty; ");

                // Append initial elements if array literal
                if let Expression::ArrayExpression(arr) = init
                    && !arr.elements.is_empty()
                {
                    self.emit_indent();
                    self.push(&escaped);
                    self.push(".appendSlice(js_allocator.g_alloc(), &[_]");
                    self.push(&et);
                    self.push("{ ");
                    for (i, elem) in arr.elements.iter().enumerate() {
                        if i > 0 { self.push(", "); }
                        self.emit_jsany_array_element(elem);
                    }
                    self.push(" }) catch unreachable;\n");
                }
                continue;
            }

            let keyword = match vd.kind {
                VariableDeclarationKind::Const => "const",
                VariableDeclarationKind::Let | VariableDeclarationKind::Var => "var",
                _ => "var",
            };

            let name: String = Self::escape_keyword(name);
            self.emit_indent();
            self.push(keyword);
            self.push(" ");
            self.push(&name);

            // Zig 0.16: add type annotation for var declarations
            if keyword == "var" {
                let var_type = self.inferrer.get_var_type(&name);
                let type_str = var_type.to_zig_str();
                self.push(": ");
                self.push(&type_str);
            }

            if let Some(init) = &decl.init {
                self.push(" = ");
                let var_type = self.inferrer.get_var_type(&name);
                self.emit_typed_init(init, &var_type);
            } else {
                self.push(" = undefined");
            }

            self.push(";\n");

            // Zig 0.16: do NOT emit `_ = name;` — causes "pointless discard" error.
            // Unused variable warnings are now handled by the Zig compiler differently.
            // (Previously: suppress "unused local constant" for trivial literals)
            }
    }

    ///   const _tmp_0 = expr;
    ///   const a = _tmp_0.a;
    ///   const b = _tmp_0.b;
    pub(super) fn emit_destructured_var_decl(
        &mut self,
        decl: &VariableDeclarator,
        kind: VariableDeclarationKind,
    ) {
        let mut leaves = Vec::new();
        // Start with empty prefix — will be replaced with temp name after init is emitted
        flatten_binding_pattern(&decl.id, "", &mut leaves);

        // Skip if all leaves are test_ helpers
        if leaves.iter().all(|l| l.name.starts_with("test_")) {
            return;
        }

        let keyword = match kind {
            VariableDeclarationKind::Const => "const",
            _ => "var",
        };

        if let Some(init) = &decl.init {
            let temp_name = format!("_tmp_{}", self.temp_counter);
            self.temp_counter += 1;

            // Check if the init expression is a dynamic array
            let is_init_dynamic_array = if let Expression::Identifier(id) = init {
                self.inferrer.is_dynamic_array(id.name.as_str())
            } else {
                false
            };

            self.emit_indent();
            self.push(keyword);
            self.push(" ");
            self.push(&temp_name);
            self.push(" = ");
            self.emit_expr(init);
            self.push(";\n");

            for leaf in &leaves {
                if leaf.name.starts_with("test_") {
                    continue;
                }
                let escaped = Self::escape_keyword(leaf.name);
                self.emit_indent();
                self.push(keyword);
                self.push(" ");
                self.push(&escaped);
                if !leaf.access.is_empty() {
                    self.push(" = ");
                    self.push(&temp_name);
                    // For dynamic arrays, use .items[...] instead of [...]
                    if is_init_dynamic_array && leaf.access.starts_with('[') {
                        self.push(".items");
                    }
                    self.push(&leaf.access);
                } else {
                    // No access path (shouldn't happen for destructured patterns)
                    self.push(" = undefined");
                }
                self.push(";\n");
            }
        } else {
            // No initializer — declare with undefined
            for leaf in &leaves {
                if leaf.name.starts_with("test_") {
                    continue;
                }
                let escaped = Self::escape_keyword(leaf.name);
                self.emit_indent();
                self.push(keyword);
                self.push(" ");
                self.push(&escaped);
                self.push(" = undefined;\n");
            }
        }
    }

    pub(super) fn emit_if_stmt(&mut self, if_stmt: &IfStatement) {
        self.emit_indent();
        self.push("if (");
        let cond = &if_stmt.test;
        let cond_ty = self.inferrer.infer_expr(cond);
        // Zig 0.16: `if (optional)` is not allowed; use `if (cond != null)` for optionals
        if matches!(cond_ty, ZigType::Optional(_)) {
            self.emit_expr(cond);
            self.push(" != null");
        } else {
            self.emit_expr(cond);
        }
        self.push(") {\n");
        self.indent += 1;
        self.emit_stmts_inside(&if_stmt.consequent);
        self.indent -= 1;

        if let Some(alt) = &if_stmt.alternate {
            self.emit_indent();
            self.push("} else ");
            self.emit_else_body(alt);
        } else {
            self.emit_indent();
            self.push("}\n");
        }
    }

    pub(super) fn emit_else_body(&mut self, alt: &Statement) {
        match alt {
            Statement::IfStatement(inner) => {
                self.push("if (");
                self.emit_expr(&inner.test);
                self.push(") {\n");
                self.indent += 1;
                self.emit_stmts_inside(&inner.consequent);
                self.indent -= 1;
                if let Some(nested_alt) = &inner.alternate {
                    self.emit_indent();
                    self.push("} else ");
                    self.emit_else_body(nested_alt);
                } else {
                    self.emit_indent();
                    self.push("}\n");
                }
            }
            Statement::BlockStatement(block) => {
                self.push("{\n");
                self.indent += 1;
                for s in &block.body {
                    self.emit_stmt(s);
                }
                self.indent -= 1;
                self.emit_indent();
                self.push("}\n");
            }
            _ => {
                self.push("{\n");
                self.indent += 1;
                self.emit_stmt(alt);
                self.indent -= 1;
                self.emit_indent();
                self.push("}\n");
            }
        }
    }

    pub(super) fn emit_stmts_inside(&mut self, stmt: &Statement) {
        match stmt {
            Statement::BlockStatement(block) => {
                for s in &block.body {
                    self.emit_stmt(s);
                }
            }
            _ => {
                self.emit_stmt(stmt);
            }
        }
    }

    /// Check whether an identifier `name` is referenced anywhere in the statement tree.
    /// Used to decide whether a for-loop capture needs a `_ = name;` discard.
    pub(super) fn capture_used_in_body(name: &str, stmt: &Statement) -> bool {
        let mut set = HashSet::new();
        Self::collect_identifiers_in_stmt(stmt, &mut set);
        set.contains(name)
    }

    pub(super) fn emit_for_stmt(&mut self, fs: &ForStatement) {
        // Translate JS `for (init; test; update) { body }` to Zig:
        //   { init; while (test) : (update) { body } }
        self.push_line("{");
        self.indent += 1;

        // Emit init before the while loop
        if let Some(init) = &fs.init {
            match init {
                ForStatementInit::VariableDeclaration(vd) => {
                    let keyword = match vd.kind {
                        VariableDeclarationKind::Const => "const",
                        _ => "var",
                    };
                    // Handle both simple and destructured declarations
                    let any_destructured = vd.declarations.iter().any(|d| !is_simple_binding(&d.id));
                    if any_destructured {
                        // Emit as individual declarations (cannot use Zig comma-separated init)
                        let saved_indent = self.indent;
                        self.indent += 1; // We're inside the { } block
                        for decl in &vd.declarations {
                            if !is_simple_binding(&decl.id) {
                                self.emit_destructured_var_decl(decl, vd.kind);
                            } else {
                                self.emit_indent();
                                self.push(keyword);
                                self.push(" ");
                            self.push(&Self::escape_keyword(self.binding_name(&decl.id)));
                            if keyword == "var" {
                                let var_type = self.inferrer.get_var_type(&Self::escape_keyword(self.binding_name(&decl.id)));
                                let type_str = var_type.to_zig_str();
                                self.push(": ");
                                    self.push(&type_str);
                                }
                                if let Some(init_expr) = &decl.init {
                                    self.push(" = ");
                                    let vt = self.inferrer.get_var_type(&Self::escape_keyword(self.binding_name(&decl.id)));
                                    self.emit_typed_init(init_expr, &vt);
                                }
                                self.push(";\n");
                            }
                        }
                        self.indent = saved_indent;
                    } else {
                        self.emit_indent();
                        self.push(keyword);
                        self.push(" ");
                        for (i, decl) in vd.declarations.iter().enumerate() {
                            if i > 0 {
                                self.push(", ");
                            }
                            self.push(&Self::escape_keyword(self.binding_name(&decl.id)));
                            let var_type = if keyword == "var" {
                                let vt = self.inferrer.get_var_type(&Self::escape_keyword(self.binding_name(&decl.id)));
                                let type_str = vt.to_zig_str();
                                self.push(": ");
                                self.push(&type_str);
                                vt
                            } else {
                                ZigType::JsValue // dummy, won't be used for init
                            };
                            if let Some(init_expr) = &decl.init {
                                self.push(" = ");
                                if keyword == "var" {
                                    self.emit_typed_init(init_expr, &var_type);
                                } else {
                                    self.emit_expr(init_expr);
                                }
                            }
                        }
                        self.push(";\n");
                    }
                }
                _ => {
                    if let Some(expr) = init.as_expression() {
                        self.emit_indent();
                        self.push("_ = ");
                        self.emit_expr(expr);
                        self.push(";\n");
                    }
                }
            }
        }

        // Emit while (test) : (update)
        self.emit_indent();
        self.push("while (");
        if let Some(test) = &fs.test {
            self.emit_expr(test);
        } else {
            self.push("true");
        }
        self.push(")");

        if let Some(update) = &fs.update {
            self.push(" : (");
            self.emit_expr(update);
            self.push(")");
        }

        self.push(" {\n");
        self.indent += 1;
        if let Statement::BlockStatement(_) = &fs.body {
            self.emit_stmts_inside(&fs.body);
        } else {
            self.emit_stmt(&fs.body);
        }
        self.indent -= 1;
        self.push_line("}");

        // Close the outer block
        self.indent -= 1;
        self.push_line("}");
    }

    pub(super) fn emit_for_in_stmt(&mut self, fis: &ForInStatement) {
        // JS for-in iterates over enumerable keys of an object.
        // Only supported for HashMap-based dynamic access objects.
        //
        // Zig output:
        //   {
        //       const _obj = <expr>;
        //       var _iter = _obj.iterator();
        //       while (_iter.next()) |_entry| {
        //           const key = _entry.key_ptr.*;
        //           // body
        //       }
        //   }

        let is_dynamic = if let Expression::Identifier(id) = &fis.right {
            self.inferrer.get_dynamic_access_vars().contains(id.name.as_str())
        } else {
            false
        };

        if !is_dynamic {
            // T10: for-in on non-dynamic object — not supported for struct-based objects
            self.emit_indent();
            self.push("@compileError(\"for-in requires a dynamic access object (HashMap). Declare with: const obj = { ... } (dynamic access)\")");
            return;
        }

        let key_name: Option<String>;
        let is_var_assign: bool;

        match &fis.left {
            ForStatementLeft::VariableDeclaration(vd) => {
                let first_decl = vd.declarations.first();
                match first_decl {
                    Some(decl) if is_simple_binding(&decl.id) => {
                        let raw = self.binding_name(&decl.id);
                        if raw.starts_with("test_") {
                            return;
                        }
                        key_name = Some(Self::escape_keyword(raw));
                        is_var_assign = false;
                    }
                    Some(_) => {
                        // T11: for-in with destructuring — JS iterates over keys (strings),
                        // destructuring a string doesn't make sense. Generate error.
                        self.emit_indent();
                        self.push("@compileError(\"for-in with destructuring is not supported (iterates over string keys)\")");
                        return;
                    }
                    None => {
                        // T12: for-in with empty declaration — JS syntax error
                        self.emit_indent();
                        self.push("@compileError(\"for-in with empty declaration is invalid JS\")");
                        return;
                    }
                }
            }
            ForStatementLeft::AssignmentTargetIdentifier(id) => {
                key_name = Some(Self::escape_keyword(&id.name));
                is_var_assign = true;
            }
            _ => {
                        // T13: for-in with member expression (AssignmentTarget)
                        // JS: for (obj[key] in iterable) { ... }
                        // Zig: not directly supported — generate error
                        self.emit_indent();
                        self.push("@compileError(\"for-in with member expression is not yet implemented\")");
                        return;
                    }
        }

        let key = key_name.unwrap();

        // Check whether the key variable is referenced in the body
        let used = Self::capture_used_in_body(&key, &fis.body);

        // Emit: { const _obj = expr; var _iter = _obj.iterator(); while (_iter.next()) |_entry| { const key = ...; body } }
        self.emit_indent();
        self.push("{\n");
        self.indent += 1;

        self.emit_indent();
        self.push("const _obj = ");
        self.emit_expr(&fis.right);
        self.push(";\n");

        self.emit_indent();
        self.push("var _iter = _obj.iterator();\n");

        self.emit_indent();
        self.push("while (_iter.next()) |_entry| {\n");
        self.indent += 1;

        self.emit_indent();
        if is_var_assign {
            self.push(&key);
            self.push(" = _entry.key_ptr.*;\n");
        } else {
            self.push("const ");
            self.push(&key);
            self.push(" = _entry.key_ptr.*;\n");
        }

        if !used {
            self.emit_indent();
            self.push("_ = ");
            self.push(&key);
            self.push(";\n");
        }

        self.emit_stmts_inside(&fis.body);

        self.indent -= 1;
        self.emit_indent();
        self.push("}\n");

        self.indent -= 1;
        self.emit_indent();
        self.push("}\n");
    }

    pub(super) fn emit_for_of_stmt(&mut self, fos: &ForOfStatement) {
        // JS: for (const x of iterable) { body }
        // Zig: for (iterable) |x| { body }
        //
        // JS: for (x of iterable) { body }  (existing var)
        // Zig: for (iterable) |_item| { x = _item; body }
        //
        // JS: for await (const x of asyncIter) { body }
        // Zig: // TODO — not directly translatable
        //
        // Note: Zig 0.16 requires all for-loop captures to be used.
        // If the captured variable is not referenced in the body,
        // we emit `_ = name;` to suppress the "unused capture" error.

        if fos.r#await {
            self.emit_indent();
            self.push_line("@compileError(\"for-await-of is not supported — async iteration requires runtime support\");");
            return;
        }

        match &fos.left {
            ForStatementLeft::VariableDeclaration(vd) => {
                let first_decl = vd.declarations.first();
                match first_decl {
                    Some(decl) if !is_simple_binding(&decl.id) => {
                        // Destructured for-of: `for (const {a, b} of arr)`
                        // → `for (arr) |_item| { const a = _item.a; const b = _item.b; ... }`
                        self.emit_indent();
                        self.push("for (");
                        self.emit_expr(&fos.right);
                        self.push(") |_item| {\n");
                        self.indent += 1;

                        // Unpack destructured bindings from _item
                        let mut leaves = Vec::new();
                        flatten_binding_pattern(&decl.id, "_item", &mut leaves);
                        for leaf in &leaves {
                            if leaf.name.starts_with("test_") {
                                continue;
                            }
                            let escaped = Self::escape_keyword(leaf.name);
                            self.emit_indent();
                            self.push("const ");
                            self.push(&escaped);
                            self.push(" = ");
                            self.push(&leaf.access);
                            self.push(";\n");
                        }

                        self.emit_stmts_inside(&fos.body);
                        self.indent -= 1;
                        self.emit_indent();
                        self.push_line("}");
                    }
                    Some(decl) => {
                        // Simple identifier
                        let name_str = self.binding_name(&decl.id).to_string();
                        let cap_name = Self::escape_keyword(&name_str);
                        let used = Self::capture_used_in_body(&name_str, &fos.body);
                        self.emit_indent();
                        self.push("for (");
                        self.emit_expr(&fos.right);
                        self.push(") |");
                        self.push(&cap_name);
                        self.push("| {\n");
                        self.indent += 1;
                        if !used {
                            self.emit_indent();
                            self.push("_ = ");
                            self.push(&cap_name);
                            self.push(";\n");
                        }
                        self.emit_stmts_inside(&fos.body);
                        self.indent -= 1;
                        self.emit_indent();
                        self.push_line("}");
                    }
                    None => {
                        // T15: for-of with empty declaration — JS syntax error
                        self.emit_indent();
                        self.push("@compileError(\"for-of requires a variable declaration or identifier\")");
                    }
                }
            }
            ForStatementLeft::AssignmentTargetIdentifier(id) => {
                let cap_name = Self::escape_keyword(&id.name);
                self.emit_indent();
                self.push("for (");
                self.emit_expr(&fos.right);
                self.push(") |_item| {\n");
                self.indent += 1;
                self.emit_indent();
                self.push(&cap_name);
                self.push(" = _item;\n");
                self.emit_stmts_inside(&fos.body);
                self.indent -= 1;
                self.emit_indent();
                self.push_line("}");
            }
            _ => {
                        // T16: for-of with member expression (AssignmentTarget)
                        // JS: for (obj[key] of arr) { ... }
                        // Zig: for (arr) |_item| { obj[key] = _item; ... }
                        self.emit_indent();
                        self.push("@compileError(\"for-of with member expression is not yet implemented\")");
                    }
        }
    }

    pub(super) fn emit_while_stmt(&mut self, ws: &WhileStatement) {
        self.emit_indent();
        self.push("while (");
        self.emit_expr(&ws.test);
        self.push(") {\n");
        self.indent += 1;
        self.emit_stmt(&ws.body);
        self.indent -= 1;
        self.push_line("}");
    }

    pub(super) fn emit_do_while_stmt(&mut self, dw: &DoWhileStatement) {
        self.push_line("while (true) {");
        self.indent += 1;
        self.emit_stmt(&dw.body);
        self.emit_indent();
        self.push("if (!(");
        self.emit_expr(&dw.test);
        self.push(")) break;\n");
        self.indent -= 1;
        self.push_line("}");
    }

    pub(super) fn emit_switch_stmt(&mut self, sw: &SwitchStatement) {
        self.emit_indent();
        self.push("_ = switch (");
        self.emit_expr(&sw.discriminant);
        self.push(") {\n");
        self.indent += 1;
        for case in &sw.cases {
            match &case.test {
                Some(test) => {
                    self.emit_indent();
                    // Numeric literals don't use `.` prefix in Zig switch
                    if matches!(test, Expression::NumericLiteral(_)) {
                        self.emit_expr(test);
                    } else {
                        self.push(".");
                        self.emit_expr(test);
                    }
                    self.push(" => {\n");
                }
                None => {
                    self.emit_indent();
                    self.push("else => {\n");
                }
            }
            self.indent += 1;
            for s in &case.consequent {
                // Skip `break` inside switch cases (Zig cases implicitly break)
                if matches!(s, Statement::BreakStatement(_)) {
                    continue;
                }
                self.emit_stmt(s);
            }
            self.indent -= 1;
            self.emit_indent();
            self.push("},\n");
        }
        self.indent -= 1;
        self.emit_indent();
        self.push("};\n");
    }

    pub(super) fn emit_try_stmt(&mut self, ts: &TryStatement) {
        // JS try-catch-finally → Zig error union + defer block.
        //
        // JS:  try { body } catch (e) { catch_body } finally { finally_body }
        // Zig: defer { finally_body }
        //      const _try_result = _tryN: { body } catch |e| _catchN: { _ = e; catch_body };
        //      _ = _try_result;
        //
        // Zig's `defer` runs when the enclosing scope exits, whether by normal completion,
        // error propagation, or return. This matches JS finally semantics.
        //
        // throw inside try  → break :_tryN error.Unexpected
        // return inside try → break :_tryN value
        // return inside catch → break :_catchN value
        //
        // Limitation: return/throw inside the finally block are not intercepted,
        // as Zig defer bodies cannot alter control flow of the deferred scope.
        let try_label = format!("_try{}", self.try_counter);
        let catch_label = format!("_catch{}", self.try_counter);
        self.try_counter += 1;

        // Emit defer with finally body (before try-catch so it runs after both try and catch)
        if let Some(ref finalizer) = ts.finalizer {
            self.emit_indent();
            self.push("defer {\n");
            self.indent += 1;
            for s in &finalizer.body {
                self.emit_stmt(s);
            }
            self.indent -= 1;
            self.emit_indent();
            self.push("}\n");
        }

        // Enter try block
        self.try_label = Some(try_label.clone());

        self.emit_indent();
        // If the enclosing function has a non-void return type AND there is no
        // finalizer (finally), emit `return` so the try-catch value becomes the
        // function's return value. With a finalizer, defer {} captures cleanup
        // and there may be trailing statements after the try-catch-finally.
        let use_return = ts.finalizer.is_none()
            && self.current_fn.as_ref().map(|fn_name| {
                let ret = self.inferrer.get_fn_return_type(fn_name);
                ret != ZigType::Void
            }).unwrap_or(false);
        if use_return {
            self.push("return ");
        } else {
            self.push("_ = ");
        }
        self.push(&format!("{}: {{", try_label));
        self.push("\n");
        self.indent += 1;

        for s in &ts.block.body {
            self.emit_stmt(s);
        }

        self.try_label = None;
        self.indent -= 1;

        // Enter catch block
        self.emit_indent();
        self.push(&format!("\n}} catch {}: {{", catch_label));
        self.push("\n");
        self.indent += 1;

        self.catch_label = Some(catch_label.clone());
        if let Some(handler) = &ts.handler {
            for s in &handler.body.body {
                self.emit_stmt(s);
            }
        }
        self.catch_label = None;

        self.indent -= 1;
        self.emit_indent();
        self.push("};\n");

    }

    /// Emit a loop statement body for labeled statements.
    /// The label prefix (e.g. "outer: ") is already emitted by the caller.
    /// This emits the while/for loop WITHOUT the leading indent (since label takes that line).
    pub(super) fn emit_labeled_loop_body(&mut self, stmt: &Statement, _label: &str) {
        match stmt {
            Statement::WhileStatement(ws) => {
                self.push("while (");
                self.emit_expr(&ws.test);
                self.push(") {\n");
                self.indent += 1;
                self.emit_stmt(&ws.body);
                self.indent -= 1;
                self.push_line("}");
            }
            Statement::DoWhileStatement(dw) => {
                self.push("while (true) {\n");
                self.indent += 1;
                self.emit_stmt(&dw.body);
                self.emit_indent();
                self.push("if (!(");
                self.emit_expr(&dw.test);
                self.push(")) break;\n");
                self.indent -= 1;
                self.push_line("}");
            }
            Statement::ForStatement(fs) => {
                // For labeled for-loops, emit as: label: { init; label_inner: while (...) ... }
                // Actually simpler: just wrap in a block and delegate
                self.push("{\n");
                self.indent += 1;
                self.emit_for_stmt(fs);
                self.indent -= 1;
                self.emit_indent();
                self.push("}\n");
            }
            Statement::ForInStatement(fis) => {
                self.push("{\n");
                self.indent += 1;
                self.emit_for_in_stmt(fis);
                self.indent -= 1;
                self.emit_indent();
                self.push("}\n");
            }
            Statement::ForOfStatement(fos) => {
                self.push("{\n");
                self.indent += 1;
                self.emit_for_of_stmt(fos);
                self.indent -= 1;
                self.emit_indent();
                self.push("}\n");
            }
            _ => {
                // Fallback: emit as block
                self.push("{\n");
                self.indent += 1;
                self.emit_stmt(stmt);
                self.indent -= 1;
                self.emit_indent();
                self.push("}\n");
            }
        }
    }
}
