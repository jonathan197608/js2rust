// Closure, arrow function, and function expression codegen.

use super::Codegen;
use crate::native_builtins as builtins;
use crate::types::ZigType;
use oxc_ast::ast::*;

// ── Arrow function support ─────────────────────────────

impl Codegen {
    /// Emit an arrow function as a Zig function.
    /// Generates the function definition and returns the function name.
    /// Detect which variables are mutated (assigned to or updated) in a list of statements.
    /// Returns a set of variable names that are mutated.
    fn detect_mutated_vars_in_stmts(stmts: &[Statement]) -> std::collections::HashSet<String> {
        let mut mutated = std::collections::HashSet::new();
        for stmt in stmts {
            Self::detect_mutated_in_stmt(stmt, &mut mutated);
        }
        mutated
    }

    fn detect_mutated_in_stmt(stmt: &Statement, mutated: &mut std::collections::HashSet<String>) {
        match stmt {
            Statement::ExpressionStatement(es) => {
                Self::detect_mutated_in_expr(&es.expression, mutated);
            }
            Statement::ReturnStatement(rs) => {
                if let Some(expr) = &rs.argument {
                    Self::detect_mutated_in_expr(expr, mutated);
                }
            }
            Statement::BlockStatement(bs) => {
                for s in &bs.body {
                    Self::detect_mutated_in_stmt(s, mutated);
                }
            }
            Statement::IfStatement(is) => {
                Self::detect_mutated_in_expr(&is.test, mutated);
                Self::detect_mutated_in_stmt(&is.consequent, mutated);
                if let Some(alt) = &is.alternate {
                    Self::detect_mutated_in_stmt(alt, mutated);
                }
            }
            Statement::WhileStatement(ws) => {
                Self::detect_mutated_in_expr(&ws.test, mutated);
                Self::detect_mutated_in_stmt(&ws.body, mutated);
            }
            Statement::ForStatement(fs) => {
                if let Some(test) = &fs.test {
                    Self::detect_mutated_in_expr(test, mutated);
                }
                if let Some(update) = &fs.update {
                    Self::detect_mutated_in_expr(update, mutated);
                }
                Self::detect_mutated_in_stmt(&fs.body, mutated);
            }
            Statement::ForOfStatement(fos) => {
                Self::detect_mutated_in_expr(&fos.right, mutated);
                Self::detect_mutated_in_stmt(&fos.body, mutated);
            }
            Statement::SwitchStatement(ss) => {
                Self::detect_mutated_in_expr(&ss.discriminant, mutated);
                for case in &ss.cases {
                    for s in &case.consequent {
                        Self::detect_mutated_in_stmt(s, mutated);
                    }
                }
            }
            Statement::TryStatement(ts) => {
                for s in &ts.block.body {
                    Self::detect_mutated_in_stmt(s, mutated);
                }
                if let Some(handler) = &ts.handler {
                    for s in &handler.body.body {
                        Self::detect_mutated_in_stmt(s, mutated);
                    }
                }
                if let Some(finalizer) = &ts.finalizer {
                    for s in &finalizer.body {
                        Self::detect_mutated_in_stmt(s, mutated);
                    }
                }
            }
            _ => {}
        }
    }

    fn detect_mutated_in_expr(expr: &Expression, mutated: &mut std::collections::HashSet<String>) {
        match expr {
            Expression::AssignmentExpression(ae) => {
                // The assignment target is mutated
                if let AssignmentTarget::AssignmentTargetIdentifier(id) = &ae.left {
                    mutated.insert(id.name.to_string());
                }
                // Also check the right side (might contain mutations)
                Self::detect_mutated_in_expr(&ae.right, mutated);
            }
            Expression::UpdateExpression(ue) => {
                // x++ or ++x
                if let SimpleAssignmentTarget::AssignmentTargetIdentifier(id) = &ue.argument {
                    mutated.insert(id.name.to_string());
                }
            }
            Expression::BinaryExpression(be) => {
                Self::detect_mutated_in_expr(&be.left, mutated);
                Self::detect_mutated_in_expr(&be.right, mutated);
            }
            Expression::CallExpression(ce) => {
                Self::detect_mutated_in_expr(&ce.callee, mutated);
                for arg in &ce.arguments {
                    if let Some(e) = arg.as_expression() {
                        Self::detect_mutated_in_expr(e, mutated);
                    }
                }
            }
            Expression::LogicalExpression(le) => {
                Self::detect_mutated_in_expr(&le.left, mutated);
                Self::detect_mutated_in_expr(&le.right, mutated);
            }
            Expression::ConditionalExpression(ce) => {
                Self::detect_mutated_in_expr(&ce.test, mutated);
                Self::detect_mutated_in_expr(&ce.consequent, mutated);
                Self::detect_mutated_in_expr(&ce.alternate, mutated);
            }
            Expression::UnaryExpression(ue) => {
                Self::detect_mutated_in_expr(&ue.argument, mutated);
            }
            Expression::AwaitExpression(ae) => {
                Self::detect_mutated_in_expr(&ae.argument, mutated);
            }
            _ => {}
        }
    }

    /// Collect locally declared variable names from a list of statements.
    /// These variables (const/let/var in the function body) are NOT captures.
    fn collect_local_declarations(
        stmts: &oxc_allocator::Vec<'_, Statement>,
    ) -> std::collections::HashSet<String> {
        let mut names = std::collections::HashSet::new();
        for stmt in stmts.iter() {
            if let Statement::VariableDeclaration(var_decl) = stmt {
                for declarator in &var_decl.declarations {
                    if let Some(name) = crate::infer::binding_name(&declarator.id) {
                        names.insert(name.to_string());
                    }
                }
            }
        }
        names
    }

    /// Collect captured variables from an arrow function body.
    /// A variable is "captured" if it's referenced in the body but is not a parameter
    /// and not a locally declared variable.
    /// Correctly sets `is_mut` by detecting mutations in the arrow body.
    fn collect_captured_vars(
        &self,
        arrow: &ArrowFunctionExpression,
    ) -> Vec<(String, ZigType, bool)> {
        let mut captured = Vec::new();
        let mut seen = std::collections::HashSet::new();

        // Collect parameter names + locally declared variable names
        let mut local_names: std::collections::HashSet<String> = arrow
            .params
            .items
            .iter()
            .filter_map(|p| crate::infer::binding_name(&p.pattern))
            .map(|s| s.to_string())
            .collect();
        local_names.extend(Self::collect_local_declarations(&arrow.body.statements));

        // Walk the body statements to find Identifier references
        for stmt in &arrow.body.statements {
            Self::collect_idents_from_stmt(
                stmt,
                &mut captured,
                &mut seen,
                &local_names,
                &self.type_info,
            );
        }

        // Detect which captured variables are mutated in the arrow body
        let mutated = Self::detect_mutated_vars_in_stmts(&arrow.body.statements);
        // Update is_mut for each captured variable
        for (name, _ztype, is_mut) in &mut captured {
            *is_mut = mutated.contains(name);
        }

        captured
    }

    /// Detect variables captured by a nested function declaration.
    /// Returns list of (variable_name, ZigType, is_mutable) for variables from the
    /// enclosing scope that are referenced in the function body but are not parameters
    /// and not locally declared variables.
    pub(super) fn detect_fn_body_captures(&self, fd: &Function) -> Vec<(String, ZigType, bool)> {
        let mut captured = Vec::new();
        let mut seen = std::collections::HashSet::new();

        // Collect parameter names + locally declared variable names
        let mut local_names: std::collections::HashSet<String> = fd
            .params
            .items
            .iter()
            .filter_map(|p| crate::infer::binding_name(&p.pattern))
            .map(|s| s.to_string())
            .collect();

        // Walk the body statements to find Identifier references
        if let Some(body) = &fd.body {
            local_names.extend(Self::collect_local_declarations(&body.statements));
            for stmt in &body.statements {
                Self::collect_idents_from_stmt(
                    stmt,
                    &mut captured,
                    &mut seen,
                    &local_names,
                    &self.type_info,
                );
            }

            // Detect which captured variables are mutated in the body
            let mutated = Self::detect_mutated_vars_in_stmts(&body.statements);
            for (name, _ztype, is_mut) in &mut captured {
                *is_mut = mutated.contains(name);
            }
        }

        captured
    }

    /// Helper: collect identifiers from a statement
    fn collect_idents_from_stmt(
        stmt: &Statement,
        captured: &mut Vec<(String, ZigType, bool)>,
        seen: &mut std::collections::HashSet<String>,
        local_names: &std::collections::HashSet<String>,
        type_info: &crate::infer::TypeCheckResult,
    ) {
        match stmt {
            Statement::ExpressionStatement(es) => {
                Self::collect_idents_from_expr(
                    &es.expression,
                    captured,
                    seen,
                    local_names,
                    type_info,
                );
            }
            Statement::ReturnStatement(ret) => {
                if let Some(expr) = &ret.argument {
                    Self::collect_idents_from_expr(expr, captured, seen, local_names, type_info);
                }
            }
            Statement::VariableDeclaration(var_decl) => {
                // Process init expressions (right-hand side) — they may reference
                // outer variables that need to be captured. The binding names (left-hand
                // side) are local and already in `local_names`.
                for declarator in &var_decl.declarations {
                    if let Some(init) = &declarator.init {
                        Self::collect_idents_from_expr(
                            init,
                            captured,
                            seen,
                            local_names,
                            type_info,
                        );
                    }
                }
            }
            _ => {}
        }
    }

    /// Helper: collect identifiers from an expression
    fn collect_idents_from_expr(
        expr: &Expression,
        captured: &mut Vec<(String, ZigType, bool)>,
        seen: &mut std::collections::HashSet<String>,
        local_names: &std::collections::HashSet<String>,
        type_info: &crate::infer::TypeCheckResult,
    ) {
        use oxc_ast::ast::Expression;
        match expr {
            Expression::Identifier(id) => {
                let name = id.name.as_str();
                // Skip parameters and locally declared variables — they are not captures
                // Skip JS builtins (parseInt, decodeURIComponent, Math, console, etc.)
                if !local_names.contains(name)
                    && !seen.contains(name)
                    && !builtins::is_js_builtin_identifier(name)
                {
                    seen.insert(name.to_string());
                    let ztype = type_info
                        .var_types
                        .get(name)
                        .cloned()
                        .unwrap_or(ZigType::I64);
                    // TODO: properly detect if captured var is mutated in arrow body
                    let is_mut = false;
                    captured.push((name.to_string(), ztype, is_mut));
                }
            }
            Expression::BinaryExpression(be) => {
                Self::collect_idents_from_expr(&be.left, captured, seen, local_names, type_info);
                Self::collect_idents_from_expr(&be.right, captured, seen, local_names, type_info);
            }
            Expression::CallExpression(ce) => {
                for arg in &ce.arguments {
                    if let Some(expr) = arg.as_expression() {
                        Self::collect_idents_from_expr(
                            expr,
                            captured,
                            seen,
                            local_names,
                            type_info,
                        );
                    }
                }
                Self::collect_idents_from_expr(&ce.callee, captured, seen, local_names, type_info);
            }
            _ => {}
        }
    }

    /// Get the return type string for an arrow function.
    fn arrow_return_type_str(&self, arrow: &ArrowFunctionExpression) -> &'static str {
        let inferred = self.infer_arrow_return_type(arrow);
        match inferred {
            Some(ZigType::I64) => "i64",
            Some(ZigType::F64) => "f64",
            Some(ZigType::Bool) => "bool",
            Some(ZigType::Str) => "[]const u8",
            Some(ZigType::Void) => "void",
            Some(_) => "i64", // NamedStruct, ArrayList, etc. — use i64 fallback
            None => {
                // When type is indeterminate:
                // - Single-expression arrow: always returns a value → i64
                // - Block-body without return → void
                if arrow.body.statements.len() == 1
                    && matches!(arrow.body.statements[0], Statement::ExpressionStatement(_))
                {
                    "i64"
                } else {
                    // Check if any statement in the block is a return
                    let has_return = arrow
                        .body
                        .statements
                        .iter()
                        .any(|s| matches!(s, Statement::ReturnStatement(_)));
                    if has_return { "i64" } else { "void" }
                }
            }
        }
    }

    /// Infer the return type of an arrow function by examining its body.
    fn infer_arrow_return_type(&self, arrow: &ArrowFunctionExpression) -> Option<ZigType> {
        // Single-expression arrow: type is the expression's type
        if arrow.body.statements.len() == 1
            && let Statement::ExpressionStatement(es) = &arrow.body.statements[0]
        {
            return self.infer_arrow_expr_type(&es.expression);
        }
        // Block body: scan return statements
        for stmt in &arrow.body.statements {
            if let Statement::ReturnStatement(rs) = stmt {
                if let Some(ref arg) = rs.argument {
                    return self.infer_arrow_expr_type(arg);
                }
                return None; // bare `return;` means void
            }
        }
        None // no return → void
    }

    /// Best-effort type inference for arrow body expressions.
    fn infer_arrow_expr_type(&self, expr: &Expression) -> Option<ZigType> {
        match expr {
            Expression::NumericLiteral(nl) => {
                if let Some(raw) = &nl.raw {
                    let s = raw.as_str();
                    if s.contains('.') || s.contains('e') || s.contains('E') {
                        Some(ZigType::F64)
                    } else {
                        Some(ZigType::I64)
                    }
                } else {
                    Some(ZigType::I64)
                }
            }
            Expression::StringLiteral(_) => Some(ZigType::Str),
            Expression::BooleanLiteral(_) => Some(ZigType::Bool),
            Expression::Identifier(id) => self.type_info.var_types.get(id.name.as_str()).cloned(),
            Expression::BinaryExpression(be) => {
                // Heuristic: try left operand first (covers patterns like `x * 2`, `x > 0`)
                self.infer_arrow_expr_type(&be.left)
                    .or_else(|| self.infer_arrow_expr_type(&be.right))
            }
            Expression::UnaryExpression(ue) => self.infer_arrow_expr_type(&ue.argument),
            Expression::CallExpression(ce) => {
                // Look up callee in fn_return_types
                if let Expression::Identifier(id) = &ce.callee {
                    self.type_info
                        .fn_return_types
                        .get(id.name.as_str())
                        .cloned()
                } else {
                    None
                }
            }
            Expression::StaticMemberExpression(sme) => {
                // Handle patterns like obj.prop, arr.length etc.
                let field = sme.property.name.as_str();
                match field {
                    "length" | "len" => Some(ZigType::I64),
                    _ => None,
                }
            }
            Expression::ConditionalExpression(ce) => {
                // For ternary, prefer consequent type (they should match)
                self.infer_arrow_expr_type(&ce.consequent)
                    .or_else(|| self.infer_arrow_expr_type(&ce.alternate))
            }
            _ => None,
        }
    }
    /// Generates the struct definition (with fields and call method) and stores it in self.closures.closure_defs.
    /// Returns the struct name.
    fn emit_closure_struct(
        &mut self,
        arrow: &ArrowFunctionExpression,
        captured: Vec<(String, ZigType, bool)>,
    ) -> String {
        let struct_name = format!("Closure_{}", self.names.next_arrow());

        // Store closure info for assignment site (so emit_var_decl can generate instantiation)
        self.closures
            .closure_vars
            .insert(struct_name.clone(), captured.clone());

        // ── Temporarily redirect output to build the struct definition ──
        let old_output = std::mem::take(&mut self.output);
        let old_indent = self.indent;
        self.output = String::new();
        self.indent = 0;

        // ── Struct definition ──
        self.writeln(&format!("const {} = struct {{", struct_name));
        self.indent = 1;

        // Fields for captured variables
        // Value capture (is_mut=false):  T
        // Reference capture (is_mut=true): *T  (the closure holds a pointer)
        for (name, ztype, is_mut) in &captured {
            let tstr = match ztype {
                ZigType::I64 => "i64".to_string(),
                ZigType::F64 => "f64".to_string(),
                ZigType::Bool => "bool".to_string(),
                ZigType::Str => "[]const u8".to_string(),
                ZigType::Void => "void".to_string(),
                ZigType::NamedStruct(s) => s.clone(),
                ZigType::ArrayList(_) => "std.ArrayList(JsAny)".to_string(),
                _ => "i64".to_string(),
            };
            if *is_mut {
                // Reference capture: store a pointer so the closure can mutate the outer variable
                self.writeln(&format!("{}: *{},", name, tstr));
            } else {
                // Value capture: store a copy
                self.writeln(&format!("{}: {},", name, tstr));
            }
        }

        // ── call method (single-line signature) ──
        let mut sig = String::from("fn call(self: *@This()");
        for param in &arrow.params.items {
            sig.push_str(", ");
            if let Some(pname) = crate::infer::binding_name(&param.pattern) {
                sig.push_str(&format!("{}: anytype", pname));
            }
        }
        // Infer return type
        sig.push_str(&format!(") {} {{", self.arrow_return_type_str(arrow)));
        self.writeln(&sig);
        self.indent += 1;

        // ── Generate method body ──
        // Set current_captured so emit_expr rewrites identifiers to self.xxx
        let saved_captured = self.closures.take_captured();
        self.closures.current_captured = captured.clone();

        // Check if arrow function has expression body (single ExpressionStatement without return)
        // In JS: `(y) => x + y` → oxc parses as ExpressionStatement(x + y)
        // In Zig: need to add `return` prefix
        if arrow.body.statements.len() == 1 {
            if let Statement::ExpressionStatement(es) = &arrow.body.statements[0] {
                self.write_indent();
                self.write("return ");
                self.emit_expr(&es.expression);
                self.write(";\n");
            } else {
                // Block body with statements
                for stmt in &arrow.body.statements {
                    self.emit_fn_stmt(stmt);
                }
            }
        } else {
            // Multiple statements or empty: generate as-is
            for stmt in &arrow.body.statements {
                self.emit_fn_stmt(stmt);
            }
        }

        // Restore
        self.closures.restore_captured(saved_captured);

        self.indent = 1;
        self.writeln("}");

        self.indent = 0;
        self.writeln("};");

        // ── Get the generated struct definition and restore output ──
        let struct_def = std::mem::take(&mut self.output);
        self.output = old_output;
        self.indent = old_indent;

        // Store in closure_defs (will be prepended to output later)
        self.closures.closure_defs.push(struct_def);

        struct_name
    }

    pub(crate) fn emit_arrow_function(&mut self, arrow: &ArrowFunctionExpression) -> String {
        // Detect captured variables (closure check)
        let captured = self.collect_captured_vars(arrow);
        if !captured.is_empty() {
            // Generate closure struct (captures outer variables)
            let struct_name = self.emit_closure_struct(arrow, captured);
            return struct_name;
        }
        // No captured vars: generate struct with call method (Zig 0.16 does not
        // allow nested `fn` declarations with return statements inside function
        // bodies, so we use the same struct+call pattern as closures).
        let fn_name = format!("_arrow_fn_{}", self.names.next_arrow());
        self.nested_fn_names.insert(fn_name.clone());

        // Struct definition
        self.writeln(&format!("const {} = struct {{", fn_name));
        self.indent += 1;

        // call method signature
        let mut sig = String::from("pub fn call(");
        for (param_idx, param) in arrow.params.items.iter().enumerate() {
            if param_idx > 0 {
                sig.push_str(", ");
            }
            if let Some(pname) = crate::infer::binding_name(&param.pattern) {
                sig.push_str(&format!("{}: anytype", pname));
            }
        }
        sig.push_str(&format!(") {} {{", self.arrow_return_type_str(arrow)));
        self.writeln(&sig);

        // Generate function body
        self.indent += 1;

        // Handle body: for single-expression arrows, the body is a FunctionBody
        // with a single ExpressionStatement.
        // We need to generate "return expr;" for the expression.
        if arrow.body.statements.len() == 1 {
            if let Statement::ExpressionStatement(es) = &arrow.body.statements[0] {
                // Single-expression arrow: generate "return expr;"
                self.write_indent();
                self.write("return ");
                self.emit_expr(&es.expression);
                self.write(
                    ";
",
                );
            } else {
                // Block body with a single statement (not expression)
                for stmt in &arrow.body.statements {
                    self.emit_fn_stmt(stmt);
                }
            }
        } else {
            // Block body with multiple statements
            for stmt in &arrow.body.statements {
                self.emit_fn_stmt(stmt);
            }
        }

        self.indent -= 1;
        self.writeln("}");

        // Close struct
        self.indent -= 1;
        self.writeln("};");

        fn_name
    }
}

/// Emit a FunctionExpression as a struct+instance inline.
/// Returns the instance name for use as an expression value.
impl Codegen {
    pub(crate) fn emit_fn_expr(&mut self, func: &Function) -> String {
        // Determine name: use function's own id if present, else generate unique name
        let name = func
            .id
            .as_ref()
            .map(|id| id.name.to_string())
            .unwrap_or_else(|| {
                let n = format!("_fn_expr_{}", self.names.next_fn_expr());
                n
            });
        let safe_name = self.zig_safe_name(&name);

        // Detect captured variables from enclosing scope
        let captures = self.detect_fn_body_captures(func);

        // Save state
        let old_current_fn = self.current_fn.clone();
        let old_fn_has_throw = self.fn_has_throw;
        let old_seen_return = self.seen_return;
        let old_fn_return_type = self.current_fn_return_type.clone();
        let old_captured = self.closures.take_captured();

        self.current_fn = Some(name.clone());

        // Pre-scan for throw
        let has_throw = func
            .body
            .as_ref()
            .is_some_and(|b| Codegen::has_throw_in_body(b));
        self.fn_has_throw = has_throw;

        // Read pre-computed return type from type_info
        let ret_ty = self.type_info.fn_return_types.get(&name).cloned();
        self.current_fn_return_type = ret_ty.clone();

        if !captures.is_empty() {
            // Has captures: generate struct with capture fields + instance
            self.nested_fn_names.insert(name.clone());

            self.write_indent();
            self.writeln(&format!("const {} = struct {{", safe_name));
            self.indent += 1;

            // Add capture fields
            for (cap_name, cap_type, _is_mut) in &captures {
                let zig_type = cap_type.to_zig_type();
                self.write_indent();
                self.writeln(&format!("{}: {},", self.zig_safe_name(cap_name), zig_type));
            }

            self.closures.current_captured = captures.clone();

            // Generate call method
            let old_nested = self.current_nested_fn_name.take();
            self.current_nested_fn_name = Some(name.clone());
            self.emit_fn(func);
            self.current_nested_fn_name = old_nested;
            self.closures.current_captured.clear();

            self.indent -= 1;
            self.write_indent();

            // Create instance
            let mut init = String::from(".{{ ");
            for (i, (cap_name, _, _)) in captures.iter().enumerate() {
                if i > 0 {
                    init.push_str(", ");
                }
                let safe_cap = self.zig_safe_name(cap_name);
                init.push_str(&format!(".{} = {}", safe_cap, safe_cap));
            }
            init.push_str(" }};");
            self.writeln(&init);
        } else {
            // No captures: generate inline struct with static call method
            self.nested_fn_names.insert(name.clone());

            self.write_indent();
            self.writeln(&format!("const {} = struct {{", safe_name));
            self.indent += 1;

            let old_nested = self.current_nested_fn_name.take();
            self.current_nested_fn_name = Some(name.clone());
            self.emit_fn(func);
            self.current_nested_fn_name = old_nested;

            self.indent -= 1;
            self.write_indent();
            self.writeln("};");
        }

        // Restore state
        self.current_fn = old_current_fn;
        self.fn_has_throw = old_fn_has_throw;
        self.seen_return = old_seen_return;
        self.current_fn_return_type = old_fn_return_type;
        self.closures.restore_captured(old_captured);

        safe_name
    }
}
