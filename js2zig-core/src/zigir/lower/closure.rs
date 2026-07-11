// zigir/lower/closure.rs
// Closure struct lowering and capture analysis.

use oxc_ast::ast::*;

use crate::types::ZigType;
use crate::zigir::types::{IrBlock, IrParam};

use super::Lowerer;

// ¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T
//  Closure struct lowering
// ¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T

impl Lowerer {
    /// Convert collected closure definitions from ClosureManager
    /// into IrClosureStruct nodes.
    ///
    /// In ZigIR these are structured IrClosureStruct nodes.
    ///
    /// After lowering, `closure_mgr.closure_vars` contains the mapping from
    /// struct name ¡ú captured vars that was built during `lower_arrow_fn` /
    /// `lower_fn_expr`.  We produce one `IrClosureStruct` per entry.
    pub(super) fn lower_closure_structs(&self) -> Vec<crate::zigir::types::IrClosureStruct> {
        self.closure_mgr
            .closure_vars
            .iter()
            .map(|(struct_name, captured)| {
                let ir_captures = self.make_ir_captures(
                    captured
                        .iter()
                        .map(|(name, zig_type, is_mut)| (name.clone(), zig_type.clone(), *is_mut))
                        .collect(),
                );
                crate::zigir::types::IrClosureStruct {
                    name: self.make_ident(struct_name),
                    captured: ir_captures,
                    fn_params: vec![], // Will be filled by the Emitter from the IrClosure
                    return_type: ZigType::Void,
                    typeof_return_body: None,
                    body: IrBlock::new(vec![]),
                }
            })
            .collect()
    }
}

// ¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T
//  Closure capture analysis
// ¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T

impl Lowerer {
    /// Collect captured variables from an arrow function body.
    ///
    /// A variable is "captured" if it's referenced in the body but is not a
    /// parameter and not a locally declared variable.
    pub(super) fn collect_arrow_captures(
        &self,
        arrow: &ArrowFunctionExpression,
    ) -> Vec<(String, ZigType, bool)> {
        let param_names = Self::collect_param_names(&arrow.params.items);
        self.collect_captures_from_body(&param_names, &arrow.body.statements, true)
    }

    /// Detect variables captured by a nested function (declaration or expression).
    ///
    /// Returns list of (variable_name, ZigType, is_mutable) for variables from
    /// the enclosing scope that are referenced in the function body.
    pub(super) fn detect_fn_body_captures(&self, fd: &Function) -> Vec<(String, ZigType, bool)> {
        let param_names = Self::collect_param_names(&fd.params.items);
        fd.body
            .as_ref()
            .map(|body| self.collect_captures_from_body(&param_names, &body.statements, true))
            .unwrap_or_default()
    }

    /// Extract parameter names from a parameter list.
    fn collect_param_names(
        params: &oxc_allocator::Vec<'_, FormalParameter>,
    ) -> std::collections::HashSet<String> {
        params
            .iter()
            .filter_map(|p| crate::infer::binding_name(&p.pattern))
            .map(|s| s.to_string())
            .collect()
    }

    /// Core capture-collection logic shared by arrow and regular functions.
    fn collect_captures_from_body(
        &self,
        param_names: &std::collections::HashSet<String>,
        stmts: &oxc_allocator::Vec<'_, Statement>,
        include_local_decls: bool,
    ) -> Vec<(String, ZigType, bool)> {
        let mut captured = Vec::new();
        let mut seen = std::collections::HashSet::new();

        let mut local_names = param_names.clone();
        if include_local_decls {
            local_names.extend(Self::collect_local_declarations(stmts));
        }

        for stmt in stmts {
            Self::collect_idents_from_stmt(
                stmt,
                &mut captured,
                &mut seen,
                &local_names,
                &self.type_info,
            );
        }

        let mutated = Self::detect_mutated_vars_in_stmts(stmts);
        for (name, _ztype, is_mut) in &mut captured {
            *is_mut = mutated.contains(name);
        }

        captured
    }

    /// Collect locally declared variable names from a list of statements.
    /// These variables (const/let/var in the function body) are NOT captures.
    pub(super) fn collect_local_declarations(
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

    /// Detect which variables are mutated (assigned to or updated) in a list of statements.
    pub(super) fn detect_mutated_vars_in_stmts(
        stmts: &[Statement],
    ) -> std::collections::HashSet<String> {
        let mut mutated = std::collections::HashSet::new();
        for stmt in stmts {
            Self::detect_mutated_in_stmt(stmt, &mut mutated);
        }
        mutated
    }

    pub(super) fn detect_mutated_in_stmt(
        stmt: &Statement,
        mutated: &mut std::collections::HashSet<String>,
    ) {
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
            Statement::ForInStatement(fis) => {
                Self::detect_mutated_in_expr(&fis.right, mutated);
                Self::detect_mutated_in_stmt(&fis.body, mutated);
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

    pub(super) fn detect_mutated_in_expr(
        expr: &Expression,
        mutated: &mut std::collections::HashSet<String>,
    ) {
        match expr {
            Expression::AssignmentExpression(ae) => {
                if let AssignmentTarget::AssignmentTargetIdentifier(id) = &ae.left {
                    mutated.insert(id.name.to_string());
                }
                Self::detect_mutated_in_expr(&ae.right, mutated);
            }
            Expression::UpdateExpression(ue) => {
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

    /// Helper: collect identifiers from a statement that reference variables
    /// in an enclosing scope (possible captures).
    pub(super) fn collect_idents_from_stmt(
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
                // Process init expressions (right-hand side) ¡ª they may reference
                // outer variables that need to be captured.
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

    /// Helper: collect identifiers from an expression that reference variables
    /// in an enclosing scope.
    pub(super) fn collect_idents_from_expr(
        expr: &Expression,
        captured: &mut Vec<(String, ZigType, bool)>,
        seen: &mut std::collections::HashSet<String>,
        local_names: &std::collections::HashSet<String>,
        type_info: &crate::infer::TypeCheckResult,
    ) {
        match expr {
            Expression::Identifier(id) => {
                let name = id.name.as_str();
                if !local_names.contains(name)
                    && !seen.contains(name)
                    && !crate::native_builtins::is_js_builtin_identifier(name)
                {
                    seen.insert(name.to_string());
                    let ztype = type_info
                        .var_types
                        .get(name)
                        .cloned()
                        .unwrap_or(ZigType::I64);
                    captured.push((name.to_string(), ztype, false));
                }
            }
            Expression::BinaryExpression(be) => {
                Self::collect_idents_from_expr(&be.left, captured, seen, local_names, type_info);
                Self::collect_idents_from_expr(&be.right, captured, seen, local_names, type_info);
            }
            Expression::CallExpression(ce) => {
                for arg in &ce.arguments {
                    if let Some(e) = arg.as_expression() {
                        Self::collect_idents_from_expr(e, captured, seen, local_names, type_info);
                    }
                }
                Self::collect_idents_from_expr(&ce.callee, captured, seen, local_names, type_info);
            }
            // Function/arrow expressions: recurse into the body to find
            // identifiers that the inner function references from the
            // enclosing scope (transitive capture).
            Expression::FunctionExpression(fe) => {
                if let Some(body) = &fe.body {
                    // Add this function's parameter names to local_names
                    let mut inner_locals = local_names.clone();
                    for param in &fe.params.items {
                        if let Some(pname) = crate::infer::binding_name(&param.pattern) {
                            inner_locals.insert(pname.to_string());
                        }
                    }
                    for stmt in &body.statements {
                        Self::collect_idents_from_stmt(
                            stmt,
                            captured,
                            seen,
                            &inner_locals,
                            type_info,
                        );
                    }
                }
            }
            Expression::ArrowFunctionExpression(af) => {
                let mut inner_locals = local_names.clone();
                for param in &af.params.items {
                    if let Some(pname) = crate::infer::binding_name(&param.pattern) {
                        inner_locals.insert(pname.to_string());
                    }
                }
                for stmt in &af.body.statements {
                    Self::collect_idents_from_stmt(stmt, captured, seen, &inner_locals, type_info);
                }
            }
            _ => {}
        }
    }

    /// Lower arrow function parameters into IrParam list.
    pub(super) fn lower_arrow_params(&mut self, arrow: &ArrowFunctionExpression) -> Vec<IrParam> {
        let mut params = Vec::new();
        for param in &arrow.params.items {
            if let Some(pname) = crate::infer::binding_name(&param.pattern) {
                let ptype = self
                    .type_info
                    .var_types
                    .get(pname)
                    .cloned()
                    .unwrap_or(ZigType::Anytype);
                params.push(IrParam {
                    name: self.make_ident(pname),
                    zig_type: ptype,
                    is_unused: false,
                    is_rest: false,
                });
            }
        }
        params
    }

    /// Find the first `IrExpr` returned from a block of IR statements.
    /// Used for `@TypeOf(return_expr)` when the return type is `AnytypeReturn`.
    pub(super) fn find_first_return_expr_in_block(
        block: &IrBlock,
    ) -> Option<&crate::zigir::types::IrExpr> {
        for stmt in &block.stmts {
            if let Some(expr) = Self::find_first_return_expr_in_stmt(stmt) {
                return Some(expr);
            }
        }
        None
    }

    pub(super) fn find_first_return_expr_in_stmt(
        stmt: &crate::zigir::types::IrStmt,
    ) -> Option<&crate::zigir::types::IrExpr> {
        match stmt {
            crate::zigir::types::IrStmt::Return { value, .. } => value.as_ref(),
            crate::zigir::types::IrStmt::If { then, else_, .. } => {
                Self::find_first_return_expr_in_block(then).or_else(|| {
                    else_
                        .as_ref()
                        .and_then(Self::find_first_return_expr_in_block)
                })
            }
            crate::zigir::types::IrStmt::Block(b) => Self::find_first_return_expr_in_block(b),
            _ => None,
        }
    }

    /// Scan statements for the first ReturnStatement and infer its type.
    fn scan_return_type_from_stmts(
        &self,
        stmts: &[Statement],
        captured: &[(String, ZigType, bool)],
        default_type: ZigType,
    ) -> ZigType {
        for stmt in stmts {
            if let Statement::ReturnStatement(rs) = stmt {
                if let Some(ref arg) = rs.argument {
                    return self
                        .infer_arrow_expr_type_with_captures(arg, captured)
                        .unwrap_or(default_type);
                }
                return ZigType::Void;
            }
        }
        ZigType::Void
    }

    /// Infer the return type of an arrow function.
    pub(super) fn infer_arrow_return_type(
        &self,
        arrow: &ArrowFunctionExpression,
        captured: &[(String, ZigType, bool)],
    ) -> ZigType {
        if arrow.body.statements.len() == 1
            && let Statement::ExpressionStatement(es) = &arrow.body.statements[0]
        {
            return self
                .infer_arrow_expr_type_with_captures(&es.expression, captured)
                .unwrap_or(ZigType::I64);
        }
        self.scan_return_type_from_stmts(&arrow.body.statements, captured, ZigType::I64)
    }

    /// Infer the return type of a function expression by scanning return statements.
    pub(super) fn infer_fn_expr_return_type(
        &self,
        fe: &Function,
        captured: &[(String, ZigType, bool)],
    ) -> ZigType {
        fe.body
            .as_ref()
            .map(|body| self.scan_return_type_from_stmts(&body.statements, captured, ZigType::Void))
            .unwrap_or(ZigType::Void)
    }

    /// Best-effort type inference with captured variable fallback.
    /// When a captured variable's type isn't in `var_types` (e.g., the variable
    /// derives from an `anytype` parameter), we can look it up from the capture
    /// list which was populated by `detect_fn_body_captures`.
    pub(super) fn infer_arrow_expr_type_with_captures(
        &self,
        expr: &Expression,
        captured: &[(String, ZigType, bool)],
    ) -> Option<ZigType> {
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
            Expression::Identifier(id) => {
                // Steps 1-3: delegate to infer_ident_type (exact, qualified, suffix)
                if let Some(ty) = self.infer_ident_type(id.name.as_str()) {
                    return Some(ty);
                }
                // Step 4: captured variable fallback (handles anytype-derived vars)
                for (name, ty, _is_mut) in captured {
                    if name == id.name.as_str() {
                        return Some(ty.clone());
                    }
                }
                None
            }
            Expression::BinaryExpression(be) => self
                .infer_arrow_expr_type_with_captures(&be.left, captured)
                .or_else(|| self.infer_arrow_expr_type_with_captures(&be.right, captured)),
            Expression::UnaryExpression(ue) => {
                self.infer_arrow_expr_type_with_captures(&ue.argument, captured)
            }
            Expression::CallExpression(ce) => {
                match &ce.callee {
                    Expression::Identifier(id) => self
                        .type_info
                        .fn_return_types
                        .get(id.name.as_str())
                        .cloned(),
                    // IIFE: function(c){...}(1) — infer from the function body
                    Expression::FunctionExpression(fe) => fe.body.as_ref().map(|body| {
                        self.scan_return_type_from_stmts(&body.statements, captured, ZigType::Void)
                    }),
                    Expression::ArrowFunctionExpression(af) => {
                        Some(self.infer_arrow_return_type(af, captured))
                    }
                    _ => None,
                }
            }
            Expression::StaticMemberExpression(sme) => {
                let field = sme.property.name.as_str();
                match field {
                    "length" | "len" => Some(ZigType::I64),
                    _ => None,
                }
            }
            Expression::ConditionalExpression(ce) => self
                .infer_arrow_expr_type_with_captures(&ce.consequent, captured)
                .or_else(|| self.infer_arrow_expr_type_with_captures(&ce.alternate, captured)),
            _ => None,
        }
    }

    /// Check whether a list of statements contains a `throw`.
    pub(super) fn has_throw_in_stmts(stmts: &oxc_allocator::Vec<'_, Statement>) -> bool {
        use super::helpers::{ThrowWalkMode, stmt_has_throw};
        stmts
            .iter()
            .any(|s| stmt_has_throw(s, ThrowWalkMode::TryBlockOnly))
    }
}
