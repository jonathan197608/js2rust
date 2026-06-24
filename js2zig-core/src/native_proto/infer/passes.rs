// native_proto/infer/passes.rs
// Analysis passes: Pass 0 (object analysis), Pass 1 (used names),
// Pass 2 (toplevel type walker) — excluding walk_fn_for_types.

use super::{InferResult, TypeInferrer};
use crate::native_proto::ZigType;
use oxc_ast::ast::*;
use std::collections::HashSet;

impl TypeInferrer {
    // ============================================================
    // Pass 0: analyze objects (mutations, dynamic access errors)
    // ============================================================

    pub(crate) fn analyze_objects(&mut self, program: &Program) {
        for stmt in &program.body {
            self.walk_stmt_for_analysis(stmt);
        }
    }

    pub(crate) fn walk_stmt_for_analysis(&mut self, stmt: &Statement) {
        match stmt {
            Statement::VariableDeclaration(vd) => {
                for decl in &vd.declarations {
                    if let Some(init) = &decl.init {
                        self.walk_expr_for_analysis(init);
                    }
                }
            }
            Statement::FunctionDeclaration(fd) => {
                let saved_current_fn = std::mem::take(&mut self.current_fn);
                self.current_fn = fd.id.as_ref().map(|id| id.name.to_string());
                if let Some(body) = &fd.body {
                    for s in &body.statements {
                        self.walk_stmt_for_analysis(s);
                    }
                }
                self.current_fn = saved_current_fn;
            }
            Statement::ExportNamedDeclaration(export_decl) => {
                if let Some(Declaration::FunctionDeclaration(fd)) = &export_decl.declaration {
                    let saved_current_fn = std::mem::take(&mut self.current_fn);
                    self.current_fn = fd.id.as_ref().map(|id| id.name.to_string());
                    if let Some(body) = &fd.body {
                        for s in &body.statements {
                            self.walk_stmt_for_analysis(s);
                        }
                    }
                    self.current_fn = saved_current_fn;
                }
            }
            Statement::ExportDefaultDeclaration(export_decl) => {
                if let ExportDefaultDeclarationKind::FunctionDeclaration(fd) =
                    &export_decl.declaration
                {
                    let saved_current_fn = std::mem::take(&mut self.current_fn);
                    self.current_fn = fd.id.as_ref().map(|id| id.name.to_string());
                    if let Some(body) = &fd.body {
                        for s in &body.statements {
                            self.walk_stmt_for_analysis(s);
                        }
                    }
                    self.current_fn = saved_current_fn;
                }
            }
            Statement::ExpressionStatement(es) => {
                self.walk_expr_for_analysis(&es.expression);
            }
            Statement::IfStatement(is) => {
                self.walk_expr_for_analysis(&is.test);
                self.walk_stmt_for_analysis(&is.consequent);
                if let Some(alt) = &is.alternate {
                    self.walk_stmt_for_analysis(alt);
                }
            }
            Statement::TryStatement(ts) => {
                for s in &ts.block.body {
                    self.walk_stmt_for_analysis(s);
                }
                if let Some(handler) = &ts.handler {
                    for s in &handler.body.body {
                        self.walk_stmt_for_analysis(s);
                    }
                }
                if let Some(finalizer) = &ts.finalizer {
                    for s in &finalizer.body {
                        self.walk_stmt_for_analysis(s);
                    }
                }
            }
            Statement::WhileStatement(ws) => {
                self.walk_expr_for_analysis(&ws.test);
                self.walk_stmt_for_analysis(&ws.body);
            }
            Statement::DoWhileStatement(dws) => {
                self.walk_stmt_for_analysis(&dws.body);
                self.walk_expr_for_analysis(&dws.test);
            }
            Statement::ForStatement(fs) => {
                if let Some(init) = &fs.init {
                    if let ForStatementInit::VariableDeclaration(vd) = init {
                        self.walk_expr_for_stmt_init_vardecl(vd);
                    } else if let Some(expr) = init.as_expression() {
                        self.walk_expr_for_analysis(expr);
                    }
                }
                if let Some(test) = &fs.test {
                    self.walk_expr_for_analysis(test);
                }
                if let Some(update) = &fs.update {
                    self.walk_expr_for_analysis(update);
                }
                self.walk_stmt_for_analysis(&fs.body);
            }
            Statement::ForOfStatement(fos) => {
                if let ForStatementLeft::VariableDeclaration(vd) = &fos.left {
                    self.walk_expr_for_stmt_init_vardecl(vd);
                }
                self.walk_expr_for_analysis(&fos.right);
                self.walk_stmt_for_analysis(&fos.body);
            }
            Statement::BlockStatement(bs) => {
                for s in &bs.body {
                    self.walk_stmt_for_analysis(s);
                }
            }
            _ => {}
        }
    }

    /// Walk a VariableDeclaration inside a for-statement init for analysis.
    fn walk_expr_for_stmt_init_vardecl(&mut self, vd: &VariableDeclaration) {
        for decl in &vd.declarations {
            if let Some(init) = &decl.init {
                self.walk_expr_for_analysis(init);
            }
        }
    }

    pub(crate) fn walk_expr_for_analysis(&mut self, expr: &Expression) {
        match expr {
            Expression::ComputedMemberExpression(mem) => match &mem.expression {
                Expression::NumericLiteral(_) => {
                    self.walk_expr_for_analysis(&mem.object);
                }
                _ => {
                    self.errors.push(
                        "Dynamic property access (obj[key]) is not allowed. \
                             Use static property access (obj.prop)."
                            .to_string(),
                    );
                    self.walk_expr_for_analysis(&mem.object);
                    self.walk_expr_for_analysis(&mem.expression);
                }
            },
            Expression::StaticMemberExpression(mem) => {
                self.walk_expr_for_analysis(&mem.object);
            }
            Expression::AssignmentExpression(ae) => {
                self.check_assignment_target(&ae.left);
                self.walk_expr_for_analysis(&ae.right);
            }
            Expression::BinaryExpression(be) => {
                self.walk_expr_for_analysis(&be.left);
                self.walk_expr_for_analysis(&be.right);
            }
            Expression::CallExpression(ce) => {
                self.walk_expr_for_analysis(&ce.callee);
                for arg in &ce.arguments {
                    if let Some(e) = arg.as_expression() {
                        self.walk_expr_for_analysis(e);
                    }
                }
            }
            Expression::ParenthesizedExpression(pe) => {
                self.walk_expr_for_analysis(&pe.expression);
            }
            Expression::ConditionalExpression(ce) => {
                self.walk_expr_for_analysis(&ce.test);
                self.walk_expr_for_analysis(&ce.consequent);
                self.walk_expr_for_analysis(&ce.alternate);
            }
            Expression::UnaryExpression(ue) => {
                self.walk_expr_for_analysis(&ue.argument);
            }
            Expression::LogicalExpression(le) => {
                self.walk_expr_for_analysis(&le.left);
                self.walk_expr_for_analysis(&le.right);
            }
            Expression::ArrayExpression(ae) => {
                for elem in &ae.elements {
                    if let Some(e) = elem.as_expression() {
                        self.walk_expr_for_analysis(e);
                    }
                }
            }
            Expression::ObjectExpression(oe) => {
                for prop in &oe.properties {
                    if let ObjectPropertyKind::ObjectProperty(p) = prop {
                        self.walk_expr_for_analysis(&p.value);
                    }
                }
            }
            Expression::ArrowFunctionExpression(arrow) => {
                // Walk arrow function body to detect mutations to outer variables.
                // Arrow functions capture from the outer scope, so mutations
                // inside the arrow function affect the outer scope's variables.
                for stmt in &arrow.body.statements {
                    self.walk_stmt_for_analysis(stmt);
                }
            }
            _ => {}
        }
    }

    pub(crate) fn check_assignment_target(&mut self, target: &AssignmentTarget) {
        let prefix = self.current_fn.as_deref().unwrap_or("__toplevel__");
        match target {
            AssignmentTarget::AssignmentTargetIdentifier(id) => {
                self.mutated_vars.insert(format!("{}::{}", prefix, id.name));
            }
            AssignmentTarget::StaticMemberExpression(mem) => {
                if let Expression::Identifier(id) = &mem.object {
                    self.mutated_vars.insert(format!("{}::{}", prefix, id.name));
                }
            }
            AssignmentTarget::ComputedMemberExpression(mem) => {
                if let Expression::Identifier(id) = &mem.object {
                    self.mutated_vars.insert(format!("{}::{}", prefix, id.name));
                }
            }
            _ => {}
        }
    }

    // ============================================================
    // Pass 1: collect used names (unused-constant elimination)
    // ============================================================

    pub(crate) fn collect_used_names(&mut self, program: &Program) {
        self.used_names.clear();
        for stmt in &program.body {
            match stmt {
                Statement::FunctionDeclaration(fd) => {
                    Self::collect_idents_from_function(fd, &mut self.used_names);
                }
                Statement::ExportNamedDeclaration(export_decl) => {
                    if let Some(Declaration::FunctionDeclaration(fd)) = &export_decl.declaration {
                        Self::collect_idents_from_function(fd.as_ref(), &mut self.used_names);
                    }
                }
                _ => {}
            }
        }
    }

    pub(crate) fn collect_idents_from_function(fd: &Function, names: &mut HashSet<String>) {
        if let Some(body) = &fd.body {
            for stmt in &body.statements {
                Self::collect_idents_from_stmt(stmt, names);
            }
        }
    }

    pub(crate) fn collect_idents_from_vardecl(
        vd: &VariableDeclaration,
        names: &mut HashSet<String>,
    ) {
        for decl in &vd.declarations {
            if let Some(init) = &decl.init {
                Self::collect_idents_from_expr(init, names);
            }
        }
    }

    pub(crate) fn collect_idents_from_stmt(stmt: &Statement, names: &mut HashSet<String>) {
        match stmt {
            Statement::ExpressionStatement(es) => {
                Self::collect_idents_from_expr(&es.expression, names);
            }
            Statement::ReturnStatement(rs) => {
                if let Some(arg) = &rs.argument {
                    Self::collect_idents_from_expr(arg, names);
                }
            }
            Statement::IfStatement(is) => {
                Self::collect_idents_from_expr(&is.test, names);
                Self::collect_idents_from_stmt(&is.consequent, names);
                if let Some(alt) = &is.alternate {
                    Self::collect_idents_from_stmt(alt, names);
                }
            }
            Statement::WhileStatement(ws) => {
                Self::collect_idents_from_expr(&ws.test, names);
                Self::collect_idents_from_stmt(&ws.body, names);
            }
            Statement::DoWhileStatement(dws) => {
                Self::collect_idents_from_stmt(&dws.body, names);
                Self::collect_idents_from_expr(&dws.test, names);
            }
            Statement::ForStatement(fs) => {
                if let Some(init) = &fs.init {
                    if let ForStatementInit::VariableDeclaration(vd) = init {
                        Self::collect_idents_from_vardecl(vd, names);
                    } else if let Some(expr) = init.as_expression() {
                        Self::collect_idents_from_expr(expr, names);
                    }
                }
                if let Some(test) = &fs.test {
                    Self::collect_idents_from_expr(test, names);
                }
                if let Some(update) = &fs.update {
                    Self::collect_idents_from_expr(update, names);
                }
                Self::collect_idents_from_stmt(&fs.body, names);
            }
            Statement::ForOfStatement(fos) => {
                if let ForStatementLeft::VariableDeclaration(vd) = &fos.left {
                    Self::collect_idents_from_vardecl(vd, names);
                }
                Self::collect_idents_from_expr(&fos.right, names);
                Self::collect_idents_from_stmt(&fos.body, names);
            }
            Statement::BlockStatement(bs) => {
                for s in &bs.body {
                    Self::collect_idents_from_stmt(s, names);
                }
            }
            Statement::VariableDeclaration(vd) => {
                for decl in &vd.declarations {
                    if let Some(init) = &decl.init {
                        Self::collect_idents_from_expr(init, names);
                    }
                }
            }
            _ => {}
        }
    }

    pub(crate) fn collect_idents_from_expr(expr: &Expression, names: &mut HashSet<String>) {
        match expr {
            Expression::Identifier(id) => {
                names.insert(id.name.to_string());
            }
            Expression::BinaryExpression(be) => {
                Self::collect_idents_from_expr(&be.left, names);
                Self::collect_idents_from_expr(&be.right, names);
            }
            Expression::CallExpression(ce) => {
                Self::collect_idents_from_expr(&ce.callee, names);
                for arg in &ce.arguments {
                    if let Some(e) = arg.as_expression() {
                        Self::collect_idents_from_expr(e, names);
                    }
                }
            }
            Expression::AssignmentExpression(ae) => {
                if let AssignmentTarget::AssignmentTargetIdentifier(id) = &ae.left {
                    names.insert(id.name.to_string());
                }
                Self::collect_idents_from_expr(&ae.right, names);
            }
            Expression::UnaryExpression(ue) => {
                Self::collect_idents_from_expr(&ue.argument, names);
            }
            Expression::LogicalExpression(le) => {
                Self::collect_idents_from_expr(&le.left, names);
                Self::collect_idents_from_expr(&le.right, names);
            }
            Expression::ParenthesizedExpression(pe) => {
                Self::collect_idents_from_expr(&pe.expression, names);
            }
            Expression::ConditionalExpression(ce) => {
                Self::collect_idents_from_expr(&ce.test, names);
                Self::collect_idents_from_expr(&ce.consequent, names);
                Self::collect_idents_from_expr(&ce.alternate, names);
            }
            _ => {}
        }
    }

    // ============================================================
    // Pass 2: collect variable types from ALL scopes
    // ============================================================

    pub(crate) fn walk_toplevel_for_types(&mut self, program: &Program) {
        for stmt in &program.body {
            self.walk_stmt_for_types(stmt);
        }
    }

    /// Walk a statement to collect variable types (no code generation).
    pub(crate) fn walk_stmt_for_types(&mut self, stmt: &Statement) {
        match stmt {
            Statement::VariableDeclaration(vd) => self.collect_var_types_from_decl(vd),
            Statement::FunctionDeclaration(fd) => {
                self.walk_fn_for_types(
                    fd,
                    fd.id.as_ref().map(|id| id.name.as_str()).unwrap_or(""),
                    false,
                );
            }
            Statement::ExportNamedDeclaration(export_decl) => {
                // export function declarations are parsed as ExportNamedDeclaration
                // containing a Declaration::FunctionDeclaration
                if let Some(Declaration::FunctionDeclaration(fd)) = &export_decl.declaration {
                    let fn_name = fd.id.as_ref().map(|id| id.name.as_str()).unwrap_or("");
                    self.walk_fn_for_types(fd.as_ref(), fn_name, true);
                }
            }
            Statement::IfStatement(is) => {
                self.walk_stmt_for_types(&is.consequent);
                if let Some(alt) = &is.alternate {
                    self.walk_stmt_for_types(alt);
                }
            }
            Statement::WhileStatement(ws) => {
                self.walk_stmt_for_types(&ws.body);
            }
            Statement::DoWhileStatement(dws) => {
                self.walk_stmt_for_types(&dws.body);
            }
            Statement::ForStatement(fs) => {
                self.walk_stmt_for_types(&fs.body);
                if let Some(ForStatementInit::VariableDeclaration(vd)) = &fs.init {
                    self.collect_var_types_from_decl(vd);
                }
            }
            Statement::ForOfStatement(fos) => {
                self.walk_stmt_for_types(&fos.body);
                // For-of loop variables have no initializer in the AST;
                // infer their type from the iterable expression.
                if let ForStatementLeft::VariableDeclaration(vd) = &fos.left {
                    // Try to get element type from the iterable
                    let elem_ty = match self.infer_expr_type(&fos.right) {
                        InferResult::Definite(ZigType::ArrayList(box_elem)) => *box_elem,
                        InferResult::Definite(ZigType::Str) => ZigType::Str,
                        _ => ZigType::I64,
                    };
                    for decl in &vd.declarations {
                        if let Some(name) = Self::binding_name(&decl.id) {
                            self.var_types.insert(name.to_string(), elem_ty.clone());
                        }
                    }
                }
            }
            Statement::BlockStatement(bs) => {
                for s in &bs.body {
                    self.walk_stmt_for_types(s);
                }
            }
            _ => {}
        }
    }

    pub(crate) fn collect_var_types_from_decl(&mut self, vd: &VariableDeclaration) {
        for decl in &vd.declarations {
            if let Some(name) = Self::binding_name(&decl.id) {
                if let Some(init) = &decl.init {
                    // Check if this is JSON.parse(@type)
                    if let Some(type_name) = self.get_json_parse_type(name, init) {
                        self.has_json_parse_types.insert(name.to_string());
                        self.var_types
                            .insert(name.to_string(), ZigType::NamedStruct(type_name));
                        continue;
                    }

                    // Check JSDoc @type annotation for this variable
                    if let Some(ref jsdoc_data) = self.jsdoc_data
                        && let Some(ty_str) = jsdoc_data.type_annotations.get(name)
                    {
                        let zig_ty = Self::jsdoc_str_to_zig_type(ty_str, &jsdoc_data.typedefs);
                        self.var_types.insert(name.to_string(), zig_ty);
                        continue;
                    }

                    let result = self.infer_expr_type(init);
                    match result {
                        InferResult::Definite(ty) => {
                            self.var_types.insert(name.to_string(), ty.clone());
                            if let ZigType::ArrayList(elem_ty) = &ty {
                                self.array_element_types
                                    .insert(name.to_string(), (**elem_ty).clone());
                            }
                        }
                        InferResult::Indeterminate => {
                            self.errors.push(format!(
                                "Cannot infer type of variable '{}' (Rule 8). \
                                 Add a type annotation or initialize with a literal.",
                                name
                            ));
                        }
                    }
                } else {
                    self.errors.push(format!(
                        "Variable '{}' must be initialized (strict type system)",
                        name
                    ));
                }
            }
        }
    }
}
