use super::*;

impl<'a> ZigCodegen<'a> {
    pub(super) fn pre_scan_closures(&mut self, program: &Program) {
        for stmt in &program.body {
            if let Statement::FunctionDeclaration(fd) = stmt {
                let fn_name = fd.id.as_ref().map(|id| id.name.as_str()).unwrap_or("");
                if fn_name.is_empty() {
                    continue;
                }
                if let Some(body) = &fd.body {
                    self.scan_fn_body_for_closures(fn_name, &fd.params, body);
                }
            }
        }
    }

    /// Scan a function body for ArrowFunctionExpressions in ALL positions
    pub(super) fn scan_fn_body_for_closures(
        &mut self,
        fn_name: &str,
        fn_params: &FormalParameters,
        body: &FunctionBody,
    ) {
        for stmt in &body.statements {
            match stmt {
                Statement::ReturnStatement(rs) => {
                    if let Some(arg) = &rs.argument {
                        self.scan_expr_for_closures(fn_name, fn_params, arg, true);
                    }
                }
                Statement::VariableDeclaration(vd) => {
                    for decl in &vd.declarations {
                        if let Some(init) = &decl.init {
                            let var_name = self.binding_name(&decl.id);
                            self.scan_expr_for_closures(var_name, fn_params, init, false);
                        }
                    }
                }
                Statement::ExpressionStatement(es) => {
                    self.scan_expr_for_closures(fn_name, fn_params, &es.expression, false);
                }
                // Recursively scan nested statements for nested closures
                _ => self.scan_stmt_for_closures(fn_name, fn_params, stmt),
            }
        }
    }

    /// Recursively scan an expression tree for ArrowFunctionExpressions
    pub(super) fn scan_expr_for_closures(
        &mut self,
        context_name: &str,
        fn_params: &FormalParameters,
        expr: &Expression,
        is_return_closure: bool,
    ) {
        match expr {
            Expression::ArrowFunctionExpression(arrow) => {
                self.record_closure(context_name, fn_params, arrow, is_return_closure);
                // Also scan the arrow body for nested closures
                let ctx_name = format!("{}_inner", context_name);
                for s in &arrow.body.statements {
                    self.scan_stmt_for_closures(&ctx_name, fn_params, s);
                }
            }
            Expression::CallExpression(call) => {
                self.scan_expr_for_closures(context_name, fn_params, &call.callee, false);
                for arg in &call.arguments {
                    if let Some(expr) = arg.as_expression() {
                        self.scan_expr_for_closures(context_name, fn_params, expr, false);
                    }
                }
            }
            Expression::BinaryExpression(bin) => {
                self.scan_expr_for_closures(context_name, fn_params, &bin.left, false);
                self.scan_expr_for_closures(context_name, fn_params, &bin.right, false);
            }
            Expression::UnaryExpression(un) => {
                self.scan_expr_for_closures(context_name, fn_params, &un.argument, false);
            }
            Expression::ConditionalExpression(cond) => {
                self.scan_expr_for_closures(context_name, fn_params, &cond.test, false);
                self.scan_expr_for_closures(context_name, fn_params, &cond.consequent, false);
                self.scan_expr_for_closures(context_name, fn_params, &cond.alternate, false);
            }
            Expression::AssignmentExpression(ass) => {
                self.scan_expr_for_closures(context_name, fn_params, &ass.right, false);
            }
            Expression::ArrayExpression(arr) => {
                for elem in &arr.elements {
                    if let Some(e) = elem.as_expression() {
                        self.scan_expr_for_closures(context_name, fn_params, e, false);
                    }
                }
            }
            Expression::NewExpression(new) => {
                for arg in &new.arguments {
                    if let Some(expr) = arg.as_expression() {
                        self.scan_expr_for_closures(context_name, fn_params, expr, false);
                    }
                }
            }
            _ => {}
        }
    }

    /// Recursively scan a statement for nested ArrowFunctionExpressions
    pub(super) fn scan_stmt_for_closures(
        &mut self,
        context_name: &str,
        fn_params: &FormalParameters,
        stmt: &Statement,
    ) {
        match stmt {
            Statement::IfStatement(if_stmt) => {
                self.scan_expr_for_closures(context_name, fn_params, &if_stmt.test, false);
                self.scan_stmt_for_closures(context_name, fn_params, &if_stmt.consequent);
                if let Some(alt) = &if_stmt.alternate {
                    self.scan_stmt_for_closures(context_name, fn_params, alt);
                }
            }
            Statement::ForStatement(fs) => {
                if let Some(init) = &fs.init {
                    match init {
                        ForStatementInit::VariableDeclaration(vd) => {
                            for decl in &vd.declarations {
                                if let Some(init_expr) = &decl.init {
                                    self.scan_expr_for_closures(context_name, fn_params, init_expr, false);
                                }
                            }
                        }
                        _ => {
                            if let Some(e) = init.as_expression() {
                                self.scan_expr_for_closures(context_name, fn_params, e, false);
                            }
                        }
                    }
                }
                if let Some(test) = &fs.test {
                    self.scan_expr_for_closures(context_name, fn_params, test, false);
                }
                if let Some(update) = &fs.update {
                    self.scan_expr_for_closures(context_name, fn_params, update, false);
                }
                self.scan_stmt_for_closures(context_name, fn_params, &fs.body);
            }
            Statement::WhileStatement(ws) => {
                self.scan_expr_for_closures(context_name, fn_params, &ws.test, false);
                self.scan_stmt_for_closures(context_name, fn_params, &ws.body);
            }
            Statement::DoWhileStatement(dw) => {
                self.scan_stmt_for_closures(context_name, fn_params, &dw.body);
                self.scan_expr_for_closures(context_name, fn_params, &dw.test, false);
            }
            Statement::BlockStatement(block) => {
                for s in &block.body {
                    self.scan_stmt_for_closures(context_name, fn_params, s);
                }
            }
            Statement::ReturnStatement(rs) => {
                if let Some(arg) = &rs.argument {
                    self.scan_expr_for_closures(context_name, fn_params, arg, false);
                }
            }
            Statement::VariableDeclaration(vd) => {
                for decl in &vd.declarations {
                    if let Some(init) = &decl.init {
                        let var_name = self.binding_name(&decl.id);
                        self.scan_expr_for_closures(var_name, fn_params, init, false);
                    }
                }
            }
            Statement::ExpressionStatement(es) => {
                self.scan_expr_for_closures(context_name, fn_params, &es.expression, false);
            }
            Statement::SwitchStatement(ss) => {
                self.scan_expr_for_closures(context_name, fn_params, &ss.discriminant, false);
                for case in &ss.cases {
                    for stmt in &case.consequent {
                        self.scan_stmt_for_closures(context_name, fn_params, stmt);
                    }
                }
            }
            Statement::TryStatement(ts) => {
                for stmt in &ts.block.body {
                    self.scan_stmt_for_closures(context_name, fn_params, stmt);
                }
                if let Some(handler) = &ts.handler {
                    for stmt in &handler.body.body {
                        self.scan_stmt_for_closures(context_name, fn_params, stmt);
                    }
                }
            }
            Statement::ForInStatement(fis) => {
                self.scan_expr_for_closures(context_name, fn_params, &fis.right, false);
                self.scan_stmt_for_closures(context_name, fn_params, &fis.body);
            }
            Statement::ForOfStatement(fos) => {
                self.scan_expr_for_closures(context_name, fn_params, &fos.right, false);
                self.scan_stmt_for_closures(context_name, fn_params, &fos.body);
            }
            _ => {}
        }
    }

    /// Record closure info for an arrow function found at any position.
    /// `fn_name` is the enclosing function name (for fn_closure_spans lookup).
    /// `struct_context` determines the struct name:
    ///   - Return closures: use `fn_name`
    ///   - Var init closures: use the variable name
    ///   - Callback closures: use synthetic name like `{fn}_cb{N}`
    pub(super) fn record_closure(
        &mut self,
        fn_name: &str,
        _fn_params: &FormalParameters,
        arrow: &ArrowFunctionExpression,
        is_return_closure: bool,
    ) {
        let span_key = arrow.span.start;
        if self.closure_map.contains_key(&span_key) {
            return; // already recorded
        }

        let struct_name = if is_return_closure {
            closure_name(fn_name)
        } else {
            // For variable assignments and callbacks, use a synthetic name
            self.closure_counter += 1;
            closure_name(&format!("{}_cb{}", fn_name, self.closure_counter))
        };

        if is_return_closure {
            self.fn_closure_spans.insert(fn_name.to_string(), span_key);
        }

        // Collect arrow function parameter info
        let mut params = Vec::new();
        let mut arrow_param_types: Vec<(String, crate::infer::ZigType)> = Vec::new();
        for p in &arrow.params.items {
            let pname = self.binding_name(&p.pattern).to_owned();
            let ptype = if let Some(default) = &p.initializer {
                self.inferrer.infer_expr(default)
            } else {
                self.inferrer.infer_arrow_param_type(&pname, &arrow.body)
            };
            arrow_param_types.push((pname.clone(), ptype.clone()));
            params.push((pname, ptype.to_zig_str()));
        }

        // Collect captured (free) variables from the arrow body.
        // "Captured" = identifiers in the arrow body that are NOT arrow params or local decls.
        // Outer function params referenced in the arrow ARE captured variables.

        // Collect arrow's own parameter names to exclude from captured set
        let arrow_param_set: HashSet<&str> = arrow
            .params
            .items
            .iter()
            .map(|p| self.binding_name(&p.pattern))
            .collect();

        let mut local_decls = HashSet::new();
        let mut free_vars = HashSet::new();
        if !arrow.expression {
            // Block body: collect locally declared variables first
            for s in &arrow.body.statements {
                if let Statement::VariableDeclaration(vd) = s {
                    for decl in &vd.declarations {
                        local_decls.insert(self.binding_name(&decl.id).to_owned());
                    }
                }
            }
        }
        for s in &arrow.body.statements {
            Self::collect_identifiers_in_stmt(s, &mut free_vars);
        }

        // Keep only identifiers that are NOT arrow params and NOT locally declared.
        // These are the captured (free) variables from the outer scope.
        let mut captured: Vec<(String, String)> = free_vars
            .into_iter()
            .filter(|name| !arrow_param_set.contains(name.as_str()) && !local_decls.contains(name))
            .map(|name| {
                let ty = self.inferrer.get_var_type(&name).to_zig_str();
                (name, ty)
            })
            .collect();
        captured.sort_by(|a, b| a.0.cmp(&b.0));

        // Infer return type of the arrow body, with arrow params registered
        let ret_ty = self.inferrer.infer_return_type_from_arrow_with_params(arrow, &arrow_param_types);

        let mut info = ClosureInfo {
            struct_name,
            captured,
            params,
            return_type: ret_ty.to_zig_str(),
            struct_def: String::new(),
        };

        // Generate struct definition string immediately (avoids storing AST references)
        let struct_def = self.generate_closure_struct_def(&info, arrow);
        info.struct_def = struct_def.clone();
        self.closure_struct_defs.insert(span_key, struct_def);
        self.closure_map.insert(span_key, info);
    }

    /// Recursively collect all identifier names used in an expression
    pub(super) fn collect_identifiers_in_expr(expr: &Expression, set: &mut HashSet<String>) {
        match expr {
            Expression::Identifier(id) => {
                set.insert(id.name.to_string());
            }
            Expression::BinaryExpression(bin) => {
                Self::collect_identifiers_in_expr(&bin.left, set);
                Self::collect_identifiers_in_expr(&bin.right, set);
            }
            Expression::UnaryExpression(un) => {
                Self::collect_identifiers_in_expr(&un.argument, set);
            }
            Expression::CallExpression(call) => {
                Self::collect_identifiers_in_expr(&call.callee, set);
                for arg in &call.arguments {
                    if let Some(e) = arg.as_expression() {
                        Self::collect_identifiers_in_expr(e, set);
                    }
                }
            }
            Expression::StaticMemberExpression(mem) => {
                Self::collect_identifiers_in_expr(&mem.object, set);
            }
            Expression::ComputedMemberExpression(mem) => {
                Self::collect_identifiers_in_expr(&mem.object, set);
                Self::collect_identifiers_in_expr(&mem.expression, set);
            }
            Expression::AssignmentExpression(assign) => {
                // For identifier collection, only traverse the right side
                Self::collect_identifiers_in_expr(&assign.right, set);
            }
            Expression::ConditionalExpression(cond) => {
                Self::collect_identifiers_in_expr(&cond.test, set);
                Self::collect_identifiers_in_expr(&cond.consequent, set);
                Self::collect_identifiers_in_expr(&cond.alternate, set);
            }
            Expression::LogicalExpression(log) => {
                Self::collect_identifiers_in_expr(&log.left, set);
                Self::collect_identifiers_in_expr(&log.right, set);
            }
            Expression::ArrayExpression(arr) => {
                for elem in &arr.elements {
                    if let Some(e) = elem.as_expression() {
                        Self::collect_identifiers_in_expr(e, set);
                    }
                }
            }
            Expression::ObjectExpression(obj) => {
                for prop in &obj.properties {
                    if let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(p) = prop {
                        Self::collect_identifiers_in_expr(&p.value, set);
                    }
                }
            }
            Expression::ParenthesizedExpression(p) => {
                Self::collect_identifiers_in_expr(&p.expression, set);
            }
            Expression::SequenceExpression(seq) => {
                for e in &seq.expressions {
                    Self::collect_identifiers_in_expr(e, set);
                }
            }
            _ => {}
        }
    }

    /// Collect identifiers from a statement (simplified: only recurse into return/expression)
    pub(super) fn collect_identifiers_in_stmt(stmt: &Statement, set: &mut HashSet<String>) {
        match stmt {
            Statement::ExpressionStatement(es) => {
                Self::collect_identifiers_in_expr(&es.expression, set);
            }
            Statement::ReturnStatement(rs) => {
                if let Some(arg) = &rs.argument {
                    Self::collect_identifiers_in_expr(arg, set);
                }
            }
            Statement::VariableDeclaration(vd) => {
                for decl in &vd.declarations {
                    if let Some(init) = &decl.init {
                        Self::collect_identifiers_in_expr(init, set);
                    }
                }
            }
            Statement::IfStatement(if_stmt) => {
                Self::collect_identifiers_in_expr(&if_stmt.test, set);
                Self::collect_identifiers_in_stmt(&if_stmt.consequent, set);
                if let Some(alt) = &if_stmt.alternate {
                    Self::collect_identifiers_in_stmt(alt, set);
                }
            }
            Statement::BlockStatement(block) => {
                for s in &block.body {
                    Self::collect_identifiers_in_stmt(s, set);
                }
            }
            Statement::ForInStatement(fi) => {
                Self::collect_identifiers_in_stmt(&fi.body, set);
            }
            Statement::ForOfStatement(fo) => {
                Self::collect_identifiers_in_stmt(&fo.body, set);
            }
            Statement::WhileStatement(ws) => {
                Self::collect_identifiers_in_stmt(&ws.body, set);
            }
            Statement::DoWhileStatement(dw) => {
                Self::collect_identifiers_in_stmt(&dw.body, set);
            }
            _ => {}
        }
    }

    pub(super) fn generate_closure_struct_def(&self, ci: &ClosureInfo, arrow: &ArrowFunctionExpression) -> String {
        let mut def = String::new();
        def.push_str(&format!("const {} = struct {{\n", ci.struct_name));

        // Emit captured fields
        for (cap_name, cap_type) in &ci.captured {
            let safe_name = Self::escape_keyword(cap_name);
            def.push_str(&format!("    {}: {},\n", safe_name, cap_type));
        }
        if !ci.captured.is_empty() {
            def.push('\n');
        }

        // Emit call method signature
        def.push_str("    pub fn call(self: @This()");
        for (pname, ptype) in &ci.params {
            let safe_pname = Self::escape_keyword(pname);
            def.push_str(&format!(", {}: {}", safe_pname, ptype));
        }
        def.push_str(") ");
        def.push_str(&ci.return_type);
        def.push_str(" {\n");

        // Emit the arrow body
        if arrow.expression {
            def.push_str("        return ");
            if let Some(first) = arrow.body.statements.first() {
                match first {
                    Statement::ExpressionStatement(es) => {
                        // Emit expression, replacing captured vars with self. references
                        let expr_code = self.emit_closure_expr(&es.expression, ci);
                        def.push_str(&expr_code);
                    }
                    Statement::ReturnStatement(rs) => {
                        if let Some(arg) = &rs.argument {
                            let expr_code = self.emit_closure_expr(arg, ci);
                            def.push_str(&expr_code);
                        }
                    }
                    _ => {
                        def.push_str("/* unsupported expression */");
                    }
                }
            }
            def.push_str(";\n");
        } else {
            // Block body — just emit a placeholder for now
            def.push_str("        // multi-statement closure body\n");
            for s in &arrow.body.statements {
                let stmt_code = self.emit_closure_stmt(s, ci);
                def.push_str(&format!("        {}\n", stmt_code));
            }
        }

        def.push_str("    }\n");
        def.push_str("};\n\n");
        def
    }

    // ========== Object type helpers ==========

    pub(super) fn emit_closure_expr(&self, expr: &Expression, ci: &ClosureInfo) -> String {
        let captured_names: HashSet<&str> = ci.captured.iter().map(|(n, _)| n.as_str()).collect();
        self.emit_expr_with_capture(expr, &captured_names)
    }

    /// Emit a statement in a closure context, replacing captured vars with `self.` prefix
    pub(super) fn emit_closure_stmt(&self, stmt: &Statement, ci: &ClosureInfo) -> String {
        let captured_names: HashSet<&str> = ci.captured.iter().map(|(n, _)| n.as_str()).collect();
        self.emit_stmt_with_capture(stmt, &captured_names)
    }

    /// Emit an expression, replacing captured variable names with `self.` prefix
    pub(super) fn emit_expr_with_capture(&self, expr: &Expression, captured: &HashSet<&str>) -> String {
        let mut buf = String::new();
        match expr {
            Expression::Identifier(id) => {
                let name = id.name.as_str();
                if captured.contains(name) {
                    buf.push_str(&format!("self.{}", Self::escape_keyword(name)));
                } else {
                    buf.push_str(&Self::escape_keyword(name));
                }
            }
            Expression::BinaryExpression(bin) => {
                buf.push_str(&self.emit_expr_with_capture(&bin.left, captured));
                buf.push(' ');
                buf.push_str(bin.operator.as_str());
                buf.push(' ');
                buf.push_str(&self.emit_expr_with_capture(&bin.right, captured));
            }
            Expression::UnaryExpression(un) => {
                buf.push_str(un.operator.as_str());
                buf.push(' ');
                buf.push_str(&self.emit_expr_with_capture(&un.argument, captured));
            }
            Expression::CallExpression(call) => {
                buf.push_str(&self.emit_expr_with_capture(&call.callee, captured));
                buf.push('(');
                for (i, arg) in call.arguments.iter().enumerate() {
                    if i > 0 {
                        buf.push_str(", ");
                    }
                    if let Some(e) = arg.as_expression() {
                        buf.push_str(&self.emit_expr_with_capture(e, captured));
                    }
                }
                buf.push(')');
            }
            Expression::ParenthesizedExpression(p) => {
                buf.push('(');
                buf.push_str(&self.emit_expr_with_capture(&p.expression, captured));
                buf.push(')');
            }
            Expression::ConditionalExpression(cond) => {
                buf.push_str(&self.emit_expr_with_capture(&cond.test, captured));
                buf.push_str(" ? ");
                buf.push_str(&self.emit_expr_with_capture(&cond.consequent, captured));
                buf.push_str(" : ");
                buf.push_str(&self.emit_expr_with_capture(&cond.alternate, captured));
            }
            Expression::NumericLiteral(n) => {
                // oxc raw is Option<Str>, fall back to formatting the value
                if let Some(raw) = &n.raw {
                    buf.push_str(raw.as_str());
                } else {
                    // Format as integer if it's a whole number
                    if n.value.fract() == 0.0 {
                        buf.push_str(&format!("{}", n.value as i64));
                    } else {
                        buf.push_str(&format!("{}", n.value));
                    }
                }
            }
            Expression::StringLiteral(s) => {
                buf.push('"');
                buf.push_str(&s.value);
                buf.push('"');
            }
            Expression::BooleanLiteral(b) => {
                buf.push_str(if b.value { "true" } else { "false" });
            }
            Expression::NullLiteral(_) => {
                buf.push_str("null");
            }
            _ => {
                // Unsupported expression type — emit placeholder
                buf.push_str("<unsupported_expr>");
            }
        }
        buf
    }

    /// Emit a statement in closure context
    pub(super) fn emit_stmt_with_capture(&self, stmt: &Statement, captured: &HashSet<&str>) -> String {
        match stmt {
            Statement::ReturnStatement(rs) => {
                let mut s = String::from("return");
                if let Some(arg) = &rs.argument {
                    s.push(' ');
                    s.push_str(&self.emit_expr_with_capture(arg, captured));
                }
                s.push(';');
                s
            }
            Statement::ExpressionStatement(es) => {
                let s = self.emit_expr_with_capture(&es.expression, captured);
                format!("{};", s)
            }
            Statement::IfStatement(if_stmt) => {
                let test = self.emit_expr_with_capture(&if_stmt.test, captured);
                let cons = self.emit_stmt_with_capture(&if_stmt.consequent, captured);
                let mut s = format!("if ({}) {{ {} }}", test, cons);
                if let Some(alt) = &if_stmt.alternate {
                    let alt_code = self.emit_stmt_with_capture(alt, captured);
                    s.push_str(&format!(" else {{ {} }}", alt_code));
                }
                s
            }
            _ => format!("// TODO: {:?} in closure", std::mem::discriminant(stmt)),
        }
    }

    /// Emit a closure struct literal assignment: `const __cl_name = StructName{ .captured = value };`
    /// The `__cl_` prefix avoids Zig 0.16 "shadows declaration" errors.
    /// Also tracks `__cl_name` as a closure variable for call translation.
    pub(super) fn emit_closure_var_init(&mut self, name: &str, ci: &ClosureInfo) {
        let safe_name = Self::escape_keyword(name);
        let cl_name = format!("__cl_{}", safe_name);
        self.emit_indent();
        self.push("const ");
        self.push(&cl_name);
        self.push(" = ");
        self.push(&ci.struct_name);
        self.push("{ ");
        for (cap_name, _cap_type) in &ci.captured {
            self.push(".");
            self.push(cap_name);
            self.push(" = ");
            self.push(cap_name);
            self.push(", ");
        }
        self.push("};\n");

        self.closure_vars.insert(cl_name);
    }

    pub(super) fn emit_arrow_fn(&mut self, raw_name: &str, arrow: &ArrowFunctionExpression) {
        let name = Self::escape_keyword(raw_name);
        let is_async = arrow.r#async;

        self.emit_indent();
        self.push("pub fn ");
        self.push(&name);
        self.push("(");

        if is_async {
            self.push("io: Io");
            if !arrow.params.items.is_empty() {
                self.push(", ");
            }
        }
        self.emit_params(&arrow.params, Some(raw_name));
        self.push(") ");

        let ret_type = self.inferrer.get_fn_return_type(raw_name);
        // If inference fails, Any.to_zig_str() returns "JsValue"
        // which is undefined → Zig compile error
        let ret_type_str = ret_type.to_zig_str();
        if is_async {
            self.push("!");
        }
        self.push(&ret_type_str);
        self.push(" {\n");
        self.indent += 1;

        // Emit destructured parameter prelude
        for prelude in self.destructure_prelude.drain(..) {
            self.output.push_str(&prelude);
        }

        // Suppress "unused parameter" for async `io` param unless the body uses await
        if is_async
            && !arrow
                .body
                .statements
                .iter()
                .any(|s| Self::stmt_contains_await(s))
        {
            self.emit_indent();
            self.push_line("_ = io;");
        }

        let prev = self.in_top_level;
        self.in_top_level = false;

        if arrow.expression {
            self.emit_indent();
            self.push("return ");
            if let Some(first) = arrow.body.statements.first() {
                match first {
                    Statement::ExpressionStatement(es) => self.emit_expr(&es.expression),
                    Statement::ReturnStatement(rs) => {
                        if let Some(arg) = &rs.argument {
                            self.emit_expr(arg);
                        }
                    }
                    _ => self.push("/* complex expression */"),
                }
            }
            self.push(";\n");
        } else {
            for stmt in &arrow.body.statements {
                self.emit_stmt(stmt);
            }
        }

        self.in_top_level = prev;
        self.indent -= 1;
        self.push_line("}");
        self.push("\n");
    }

    pub(super) fn emit_fn_from_func_expr(&mut self, name: &str, fe: &Function) {
        let escaped_name = Self::escape_keyword(name);
        let is_async = fe.r#async;

        self.emit_indent();
        self.push("pub fn ");
        self.push(&escaped_name);
        self.push("(");

        if is_async {
            self.push("io: Io");
            if !fe.params.items.is_empty() {
                self.push(", ");
            }
        }
        self.emit_params(&fe.params, Some(name));
        self.push(") ");

        let ret_type = self.inferrer.get_fn_return_type(name);
        // If inference fails, Any.to_zig_str() returns "JsValue"
        // which is undefined → Zig compile error.
        let ret_type_str = ret_type.to_zig_str();
        if is_async {
            self.push("!");
        }
        self.push(&ret_type_str);
        self.push(" ");

        if let Some(body) = &fe.body {
            self.push("{\n");
            self.indent += 1;

            // Emit destructured parameter prelude
            for prelude in self.destructure_prelude.drain(..) {
                self.output.push_str(&prelude);
            }

            // Suppress "unused parameter" for async `io` param unless the body uses await
            if is_async && !Self::body_contains_await(body) {
                self.emit_indent();
                self.push_line("_ = io;");
            }
            let prev = self.in_top_level;
            self.in_top_level = false;
            for stmt in &body.statements {
                self.emit_stmt(stmt);
            }
            self.in_top_level = prev;
            self.indent -= 1;
            self.push_line("}");
        } else {
            self.push("{};\n");
        }
        self.push("\n");
    }
}
