// zigir/lower/closure.rs
// Closure struct lowering and capture analysis.

use oxc_ast::ast::*;
use std::cell::{Cell, RefCell};
use std::collections::HashSet;

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
        let mut param_names = Self::collect_param_names(&arrow.params.items);
        // Include rest parameter name: it is a parameter, not a capture.
        if let Some(rest) = &arrow.params.rest
            && let Some(name) = crate::infer::binding_name(&rest.rest.argument)
        {
            param_names.insert(name.to_string());
        }
        self.collect_captures_from_body(&param_names, &arrow.body.statements, true)
    }

    /// Detect variables captured by a nested function (declaration or expression).
    ///
    /// Returns list of (variable_name, ZigType, is_mutable) for variables from
    /// the enclosing scope that are referenced in the function body.
    pub(super) fn detect_fn_body_captures(&self, fd: &Function) -> Vec<(String, ZigType, bool)> {
        let mut param_names = Self::collect_param_names(&fd.params.items);
        // Include rest parameter name: it is a parameter, not a capture.
        if let Some(rest) = &fd.params.rest
            && let Some(name) = crate::infer::binding_name(&rest.rest.argument)
        {
            param_names.insert(name.to_string());
        }
        fd.body
            .as_ref()
            .map(|body| self.collect_captures_from_body(&param_names, &body.statements, true))
            .unwrap_or_default()
    }

    /// Extract parameter names from a parameter list.
    fn collect_param_names(params: &oxc_allocator::Vec<'_, FormalParameter>) -> HashSet<String> {
        params
            .iter()
            .filter_map(|p| crate::infer::binding_name(&p.pattern))
            .map(|s| s.to_string())
            .collect()
    }

    /// Core capture-collection logic shared by arrow and regular functions.
    fn collect_captures_from_body(
        &self,
        param_names: &HashSet<String>,
        stmts: &oxc_allocator::Vec<'_, Statement>,
        include_local_decls: bool,
    ) -> Vec<(String, ZigType, bool)> {
        let captured = RefCell::new(Vec::new());
        let seen = RefCell::new(HashSet::new());

        let mut local_names = param_names.clone();
        if include_local_decls {
            local_names.extend(Self::collect_local_declarations(stmts));
        }

        for stmt in stmts {
            Self::collect_idents_from_stmt(stmt, &captured, &seen, &local_names, &self.type_info);
        }

        let mut captured = captured.into_inner();
        let mutated = Self::detect_mutated_vars_in_stmts(stmts);
        for (name, _ztype, is_mut) in &mut captured {
            *is_mut = mutated.contains(name);
        }

        // LOW-12: Re-resolve captured types via get_var_type, which checks
        // fn_local_types (per-function scope) before falling back to var_types.
        // check_and_add_capture only has access to var_types, missing per-function
        // type information.
        for (name, ztype, _) in &mut captured {
            if let Some(ty) = self.get_var_type(name) {
                *ztype = ty;
            }
        }

        captured
    }

    /// Collect locally declared variable names from a list of statements.
    /// These variables (const/let/var in the function body) are NOT captures.
    /// Recurses into nested control flow (if/for/while/block/try/switch, etc.)
    /// but does NOT recurse into FunctionDeclaration bodies (separate scope).
    pub(super) fn collect_local_declarations(
        stmts: &oxc_allocator::Vec<'_, Statement>,
    ) -> HashSet<String> {
        let names = RefCell::new(HashSet::new());
        for stmt in stmts {
            Self::collect_local_decls_from_stmt(stmt, &names);
        }
        names.into_inner()
    }

    fn collect_local_decls_from_stmt(stmt: &Statement, names: &RefCell<HashSet<String>>) {
        // Special case: TryStatement catch parameter is a local variable
        // (scoped to the catch block), not a capture from outer scope.
        // for_each_stmt_child does not pass catch params to any callback,
        // so we must extract them here before recursing.
        if let Statement::TryStatement(ts) = stmt
            && let Some(handler) = &ts.handler
            && let Some(param) = &handler.param
            && let Some(name) = crate::infer::binding_name(&param.pattern)
        {
            names.borrow_mut().insert(name.to_string());
        }
        // R38-LOW-4: FunctionDeclaration name is a local binding in the
        // enclosing scope. for_each_stmt_child falls through to `_ => {}`
        // for FunctionDeclaration, so we must extract the name here.
        if let Statement::FunctionDeclaration(fd) = stmt
            && let Some(id) = &fd.id
        {
            names.borrow_mut().insert(id.name.to_string());
        }
        crate::infer::ast_walk::for_each_stmt_child(
            stmt,
            &mut |s| Self::collect_local_decls_from_stmt(s, names),
            &mut |_| {}, // on_expr: not needed for local declarations
            &mut |vd| {
                for decl in &vd.declarations {
                    if let Some(name) = crate::infer::binding_name(&decl.id) {
                        names.borrow_mut().insert(name.to_string());
                    }
                }
            },
        );
    }

    /// Detect which variables are mutated (assigned to or updated) in a list of statements.
    pub(super) fn detect_mutated_vars_in_stmts(stmts: &[Statement]) -> HashSet<String> {
        let mutated = RefCell::new(HashSet::new());
        for stmt in stmts {
            Self::detect_mutated_in_stmt(stmt, &mutated);
        }
        mutated.into_inner()
    }

    pub(super) fn detect_mutated_in_stmt(stmt: &Statement, mutated: &RefCell<HashSet<String>>) {
        crate::infer::ast_walk::for_each_stmt_child(
            stmt,
            &mut |s| Self::detect_mutated_in_stmt(s, mutated),
            &mut |e| Self::detect_mutated_in_expr(e, mutated),
            &mut |_vd| {
                // VariableDeclaration inits are not mutation targets; skip.
            },
        );
    }

    pub(super) fn detect_mutated_in_expr(expr: &Expression, mutated: &RefCell<HashSet<String>>) {
        crate::infer::ast_walk::for_each_expr_child(
            expr,
            &mut |e| Self::detect_mutated_in_expr(e, mutated),
            &mut |_| {}, // on_ident: identifiers are not mutations
            &mut |target| {
                // R28-LOW-4: Also handle destructuring assignment targets
                // (ArrayAssignmentTarget, ObjectAssignmentTarget) so that
                // captured variables reassigned via destructuring are
                // correctly detected as mutated (by-reference capture).
                Self::collect_mutation_idents_from_target(target, mutated);
            },
            &mut |simple_target| {
                // UpdateExpression target (i++, ++i, etc.)
                if let SimpleAssignmentTarget::AssignmentTargetIdentifier(id) = simple_target {
                    mutated.borrow_mut().insert(id.name.to_string());
                }
            },
            &mut |_params, stmts| {
                // R38-LOW-1: Recurse into nested function bodies to detect
                // mutations to captured variables. collect_idents_from_fn_body
                // correctly recurses into nested functions for capture
                // detection, but this on_fn_scope callback was previously
                // empty, causing is_mut to stay false for variables mutated
                // only inside nested functions — leading to by-value capture
                // instead of by-reference. Mutations to the nested function's
                // own locals are also tracked but harmlessly ignored since
                // captured variables are by definition not local.
                for stmt in stmts {
                    Self::detect_mutated_in_stmt(stmt, mutated);
                }
            },
        );
    }

    /// R28-LOW-4: Recursively collect identifier names from a destructuring
    /// assignment target, including array and object patterns. Used by
    /// detect_mutated_in_expr to correctly mark captured variables that are
    /// reassigned via destructuring (e.g., [x] = arr, {a} = obj) as mutated.
    fn collect_mutation_idents_from_target(
        target: &AssignmentTarget,
        mutated: &RefCell<HashSet<String>>,
    ) {
        match target {
            AssignmentTarget::AssignmentTargetIdentifier(id) => {
                mutated.borrow_mut().insert(id.name.to_string());
            }
            AssignmentTarget::ArrayAssignmentTarget(at) => {
                for inner in at.elements.iter().flatten() {
                    Self::collect_mutation_idents_from_maybe_default(inner, mutated);
                }
            }
            AssignmentTarget::ObjectAssignmentTarget(ot) => {
                for prop in &ot.properties {
                    match prop {
                        AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(ap) => {
                            mutated.borrow_mut().insert(ap.binding.name.to_string());
                        }
                        AssignmentTargetProperty::AssignmentTargetPropertyProperty(ap) => {
                            Self::collect_mutation_idents_from_maybe_default(&ap.binding, mutated);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// R28-LOW-4: Collect mutation identifiers from an AssignmentTargetMaybeDefault
    /// (element of an array destructuring pattern).
    fn collect_mutation_idents_from_maybe_default(
        target: &AssignmentTargetMaybeDefault,
        mutated: &RefCell<HashSet<String>>,
    ) {
        match target {
            AssignmentTargetMaybeDefault::AssignmentTargetIdentifier(id) => {
                mutated.borrow_mut().insert(id.name.to_string());
            }
            AssignmentTargetMaybeDefault::AssignmentTargetWithDefault(atwd) => {
                Self::collect_mutation_idents_from_target(&atwd.binding, mutated);
            }
            AssignmentTargetMaybeDefault::ArrayAssignmentTarget(at) => {
                for inner in at.elements.iter().flatten() {
                    Self::collect_mutation_idents_from_maybe_default(inner, mutated);
                }
            }
            AssignmentTargetMaybeDefault::ObjectAssignmentTarget(ot) => {
                for prop in &ot.properties {
                    match prop {
                        AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(ap) => {
                            mutated.borrow_mut().insert(ap.binding.name.to_string());
                        }
                        AssignmentTargetProperty::AssignmentTargetPropertyProperty(ap) => {
                            Self::collect_mutation_idents_from_maybe_default(&ap.binding, mutated);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    /// Helper: collect identifiers from a function body (params + statements).
    /// Shared by FunctionExpression and ArrowFunctionExpression to avoid duplication.
    /// Also collects local declarations from the nested function body to avoid
    /// misclassifying them as captures.
    fn collect_idents_from_fn_body(
        params: &[FormalParameter],
        stmts: &oxc_allocator::Vec<'_, Statement>,
        captured: &RefCell<Vec<(String, ZigType, bool)>>,
        seen: &RefCell<HashSet<String>>,
        local_names: &HashSet<String>,
        type_info: &crate::infer::TypeCheckResult,
    ) {
        let mut inner_locals = local_names.clone();
        for param in params {
            if let Some(pname) = crate::infer::binding_name(&param.pattern) {
                inner_locals.insert(pname.to_string());
            }
        }
        // Also collect the nested function's own local declarations so they
        // are not falsely treated as captures from the enclosing scope.
        inner_locals.extend(Self::collect_local_declarations(stmts));

        for stmt in stmts {
            Self::collect_idents_from_stmt(stmt, captured, seen, &inner_locals, type_info);
        }
    }

    /// Helper: collect identifiers from a statement that reference variables
    /// in an enclosing scope (possible captures).
    ///
    /// Recurses into ALL statement types (if/while/for/try/block/switch/etc.)
    /// using `for_each_stmt_child` to ensure identifiers inside control flow
    /// are properly detected as captures.
    pub(super) fn collect_idents_from_stmt(
        stmt: &Statement,
        captured: &RefCell<Vec<(String, ZigType, bool)>>,
        seen: &RefCell<HashSet<String>>,
        local_names: &HashSet<String>,
        type_info: &crate::infer::TypeCheckResult,
    ) {
        crate::infer::ast_walk::for_each_stmt_child(
            stmt,
            &mut |s| Self::collect_idents_from_stmt(s, captured, seen, local_names, type_info),
            &mut |e| Self::collect_idents_from_expr(e, captured, seen, local_names, type_info),
            &mut |vd| {
                crate::infer::ast_walk::for_each_var_decl_init(vd, &mut |init| {
                    Self::collect_idents_from_expr(init, captured, seen, local_names, type_info);
                });
            },
        );
    }

    /// Helper: collect identifiers from an expression that reference variables
    /// in an enclosing scope.
    ///
    /// Recurses into ALL expression types using `for_each_expr_child` to ensure
    /// identifiers inside member accesses, assignments, conditionals, arrays,
    /// objects, optional chains, etc. are properly detected as captures.
    pub(super) fn collect_idents_from_expr(
        expr: &Expression,
        captured: &RefCell<Vec<(String, ZigType, bool)>>,
        seen: &RefCell<HashSet<String>>,
        local_names: &HashSet<String>,
        type_info: &crate::infer::TypeCheckResult,
    ) {
        crate::infer::ast_walk::for_each_expr_child(
            expr,
            &mut |e| Self::collect_idents_from_expr(e, captured, seen, local_names, type_info),
            &mut |name| {
                Self::check_and_add_capture(name, captured, seen, local_names, type_info);
            },
            &mut |target| {
                Self::collect_idents_from_assignment_target(
                    target,
                    captured,
                    seen,
                    local_names,
                    type_info,
                );
            },
            &mut |simple_target| match simple_target {
                SimpleAssignmentTarget::AssignmentTargetIdentifier(id) => {
                    Self::check_and_add_capture(
                        id.name.as_str(),
                        captured,
                        seen,
                        local_names,
                        type_info,
                    );
                }
                SimpleAssignmentTarget::StaticMemberExpression(mem) => {
                    Self::collect_idents_from_expr(
                        &mem.object,
                        captured,
                        seen,
                        local_names,
                        type_info,
                    );
                }
                SimpleAssignmentTarget::ComputedMemberExpression(mem) => {
                    Self::collect_idents_from_expr(
                        &mem.object,
                        captured,
                        seen,
                        local_names,
                        type_info,
                    );
                    Self::collect_idents_from_expr(
                        &mem.expression,
                        captured,
                        seen,
                        local_names,
                        type_info,
                    );
                }
                SimpleAssignmentTarget::PrivateFieldExpression(pfe) => {
                    Self::collect_idents_from_expr(
                        &pfe.object,
                        captured,
                        seen,
                        local_names,
                        type_info,
                    );
                }
                _ => {}
            },
            &mut |params, stmts| {
                Self::collect_idents_from_fn_body(
                    params,
                    stmts,
                    captured,
                    seen,
                    local_names,
                    type_info,
                );
            },
        );
    }

    /// Check if a name should be added to the capture list.
    /// A name is captured if it is NOT a local variable, NOT already seen,
    /// and NOT a JavaScript built-in identifier.
    fn check_and_add_capture(
        name: &str,
        captured: &RefCell<Vec<(String, ZigType, bool)>>,
        seen: &RefCell<HashSet<String>>,
        local_names: &HashSet<String>,
        type_info: &crate::infer::TypeCheckResult,
    ) {
        if !local_names.contains(name)
            && !seen.borrow().contains(name)
            && !crate::native_builtins::is_js_builtin_identifier(name)
        {
            seen.borrow_mut().insert(name.to_string());
            let ztype = type_info
                .var_types
                .get(name)
                .cloned()
                .unwrap_or(ZigType::JsAny);
            captured.borrow_mut().push((name.to_string(), ztype, false));
        }
    }

    /// P2-14: Recursively collect identifiers from an assignment target,
    /// including destructuring patterns (array and object).
    fn collect_idents_from_assignment_target(
        target: &AssignmentTarget,
        captured: &RefCell<Vec<(String, ZigType, bool)>>,
        seen: &RefCell<HashSet<String>>,
        local_names: &HashSet<String>,
        type_info: &crate::infer::TypeCheckResult,
    ) {
        match target {
            AssignmentTarget::AssignmentTargetIdentifier(id) => {
                Self::check_and_add_capture(
                    id.name.as_str(),
                    captured,
                    seen,
                    local_names,
                    type_info,
                );
            }
            AssignmentTarget::StaticMemberExpression(mem) => {
                Self::collect_idents_from_expr(&mem.object, captured, seen, local_names, type_info);
            }
            AssignmentTarget::ComputedMemberExpression(mem) => {
                Self::collect_idents_from_expr(&mem.object, captured, seen, local_names, type_info);
                Self::collect_idents_from_expr(
                    &mem.expression,
                    captured,
                    seen,
                    local_names,
                    type_info,
                );
            }
            AssignmentTarget::PrivateFieldExpression(pfe) => {
                Self::collect_idents_from_expr(&pfe.object, captured, seen, local_names, type_info);
            }
            AssignmentTarget::ArrayAssignmentTarget(at) => {
                for inner in at.elements.iter().flatten() {
                    Self::collect_idents_from_maybe_default(
                        inner,
                        captured,
                        seen,
                        local_names,
                        type_info,
                    );
                }
            }
            AssignmentTarget::ObjectAssignmentTarget(ot) => {
                for prop in &ot.properties {
                    match prop {
                        AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(ap) => {
                            Self::check_and_add_capture(
                                ap.binding.name.as_str(),
                                captured,
                                seen,
                                local_names,
                                type_info,
                            );
                            if let Some(init) = &ap.init {
                                Self::collect_idents_from_expr(
                                    init,
                                    captured,
                                    seen,
                                    local_names,
                                    type_info,
                                );
                            }
                        }
                        AssignmentTargetProperty::AssignmentTargetPropertyProperty(ap) => {
                            Self::collect_idents_from_maybe_default(
                                &ap.binding,
                                captured,
                                seen,
                                local_names,
                                type_info,
                            );
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// P2-14: Collect identifiers from an `AssignmentTargetMaybeDefault`
    /// (element of an array destructuring pattern).
    fn collect_idents_from_maybe_default(
        target: &AssignmentTargetMaybeDefault,
        captured: &RefCell<Vec<(String, ZigType, bool)>>,
        seen: &RefCell<HashSet<String>>,
        local_names: &HashSet<String>,
        type_info: &crate::infer::TypeCheckResult,
    ) {
        match target {
            AssignmentTargetMaybeDefault::AssignmentTargetIdentifier(id) => {
                Self::check_and_add_capture(
                    id.name.as_str(),
                    captured,
                    seen,
                    local_names,
                    type_info,
                );
            }
            AssignmentTargetMaybeDefault::AssignmentTargetWithDefault(atwd) => {
                Self::collect_idents_from_expr(&atwd.init, captured, seen, local_names, type_info);
                // Recurse into the inner binding target (handles nested patterns)
                Self::collect_idents_from_assignment_target(
                    &atwd.binding,
                    captured,
                    seen,
                    local_names,
                    type_info,
                );
            }
            // Nested array/object destructuring patterns appearing directly as
            // MaybeDefault elements (e.g. [[x], y] = arr). Delegate to
            // collect_idents_from_assignment_target which handles all variants.
            // as_assignment_target() always returns Some for these two variants.
            AssignmentTargetMaybeDefault::ArrayAssignmentTarget(_)
            | AssignmentTargetMaybeDefault::ObjectAssignmentTarget(_) => {
                if let Some(at) = target.as_assignment_target() {
                    Self::collect_idents_from_assignment_target(
                        at,
                        captured,
                        seen,
                        local_names,
                        type_info,
                    );
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
        // Handle rest parameter (...args) → []const JsAny
        if let Some(rname) = arrow
            .params
            .rest
            .as_ref()
            .and_then(|r| crate::infer::binding_name(&r.rest.argument))
        {
            params.push(IrParam {
                name: self.make_ident(rname),
                zig_type: ZigType::Anytype,
                is_unused: false,
                is_rest: true,
            });
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
            // P2-15: Also recurse into loops, switch, and try blocks
            crate::zigir::types::IrStmt::While { body, .. }
            | crate::zigir::types::IrStmt::DoWhile { body, .. }
            | crate::zigir::types::IrStmt::For { body, .. }
            | crate::zigir::types::IrStmt::ForOf { body, .. }
            | crate::zigir::types::IrStmt::ForIn { body, .. } => {
                Self::find_first_return_expr_in_block(body)
            }
            crate::zigir::types::IrStmt::Switch { cases, .. } => {
                for case in cases {
                    for s in &case.body {
                        if let Some(expr) = Self::find_first_return_expr_in_stmt(s) {
                            return Some(expr);
                        }
                    }
                }
                None
            }
            crate::zigir::types::IrStmt::Try {
                try_block,
                catch_block,
                finally,
                ..
            } => Self::find_first_return_expr_in_block(try_block)
                .or_else(|| Self::find_first_return_expr_in_block(catch_block))
                .or_else(|| {
                    finally
                        .as_ref()
                        .and_then(Self::find_first_return_expr_in_block)
                }),
            _ => None,
        }
    }

    /// Scan statements for the first ReturnStatement and infer its type.
    /// P2-EM-3: Now recurses into compound statements (If, For, While, Try,
    /// Switch, Block, Labeled) to find returns nested inside control flow.
    fn scan_return_type_from_stmts(
        &self,
        stmts: &[Statement],
        captured: &[(String, ZigType, bool)],
        default_type: ZigType,
    ) -> ZigType {
        self.scan_return_type_from_stmts_inner(stmts, captured, &default_type)
            .unwrap_or(ZigType::Void)
    }

    /// Inner version returning Option<ZigType> (None = no return found).
    fn scan_return_type_from_stmts_inner(
        &self,
        stmts: &[Statement],
        captured: &[(String, ZigType, bool)],
        default_type: &ZigType,
    ) -> Option<ZigType> {
        for stmt in stmts {
            if let Some(ty) = self.scan_return_type_from_stmt(stmt, captured, default_type) {
                return Some(ty);
            }
        }
        None
    }

    /// Scan a single statement for a return type, recursing into compound
    /// statements (If, For, While, Try, Switch, Block, Labeled, etc.).
    fn scan_return_type_from_stmt(
        &self,
        stmt: &Statement,
        captured: &[(String, ZigType, bool)],
        default_type: &ZigType,
    ) -> Option<ZigType> {
        match stmt {
            Statement::ReturnStatement(rs) => {
                if let Some(ref arg) = rs.argument {
                    Some(
                        self.infer_arrow_expr_type_with_captures(arg, captured)
                            .unwrap_or_else(|| default_type.clone()),
                    )
                } else {
                    Some(ZigType::Void)
                }
            }
            Statement::BlockStatement(bs) => {
                self.scan_return_type_from_stmts_inner(&bs.body, captured, default_type)
            }
            Statement::IfStatement(is) => self
                .scan_return_type_from_stmt(&is.consequent, captured, default_type)
                .or_else(|| {
                    is.alternate.as_ref().and_then(|alt| {
                        self.scan_return_type_from_stmt(alt, captured, default_type)
                    })
                }),
            Statement::ForStatement(fs) => {
                self.scan_return_type_from_stmt(&fs.body, captured, default_type)
            }
            Statement::ForOfStatement(fs) => {
                self.scan_return_type_from_stmt(&fs.body, captured, default_type)
            }
            Statement::ForInStatement(fs) => {
                self.scan_return_type_from_stmt(&fs.body, captured, default_type)
            }
            Statement::WhileStatement(ws) => {
                self.scan_return_type_from_stmt(&ws.body, captured, default_type)
            }
            Statement::DoWhileStatement(ds) => {
                self.scan_return_type_from_stmt(&ds.body, captured, default_type)
            }
            Statement::SwitchStatement(ss) => {
                for case in &ss.cases {
                    for s in &case.consequent {
                        if let Some(ty) = self.scan_return_type_from_stmt(s, captured, default_type)
                        {
                            return Some(ty);
                        }
                    }
                }
                None
            }
            Statement::TryStatement(ts) => {
                if let Some(ty) =
                    self.scan_return_type_from_stmts_inner(&ts.block.body, captured, default_type)
                {
                    return Some(ty);
                }
                if let Some(ref handler) = ts.handler
                    && let Some(ty) = self.scan_return_type_from_stmts_inner(
                        &handler.body.body,
                        captured,
                        default_type,
                    )
                {
                    return Some(ty);
                }
                // finally block: a return inside finally overrides any
                // try/catch return, so it must be scanned too. (LOW-19)
                if let Some(ref finalizer) = ts.finalizer
                    && let Some(ty) = self.scan_return_type_from_stmts_inner(
                        &finalizer.body,
                        captured,
                        default_type,
                    )
                {
                    return Some(ty);
                }
                None
            }
            Statement::LabeledStatement(ls) => {
                self.scan_return_type_from_stmt(&ls.body, captured, default_type)
            }
            _ => None,
        }
    }

    /// Infer the return type of an arrow function.
    pub(super) fn infer_arrow_return_type(
        &self,
        arrow: &ArrowFunctionExpression,
        captured: &[(String, ZigType, bool)],
    ) -> ZigType {
        // Only expression-body arrows (`() => expr`) return the expression value.
        // Block-body arrows (`() => { expr; }`) return void unless they have an
        // explicit `return` statement.
        if arrow.expression
            && arrow.body.statements.len() == 1
            && let Statement::ExpressionStatement(es) = &arrow.body.statements[0]
        {
            return self
                .infer_arrow_expr_type_with_captures(&es.expression, captured)
                .unwrap_or(ZigType::JsAny);
        }
        self.scan_return_type_from_stmts(&arrow.body.statements, captured, ZigType::JsAny)
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
            // For all other expression types, delegate to the full
            // infer_expr_type which has correct operator-aware logic
            // (BinaryExpression, UnaryExpression, ConditionalExpression,
            // CallExpression, NewExpression, UpdateExpression, etc.).
            // This avoids duplicating type inference logic and the bugs that
            // come with it (LOW-13 through LOW-18).
            _ => self.infer_expr_type(expr),
        }
    }

    /// C5: Detect whether a function body references `this`.
    ///
    /// Arrow functions inside class methods must capture the class instance
    /// (`self` in Zig) as a synthetic `__self` capture variable, because
    /// `IrExpr::This` (which emits `"self"`) would refer to the closure
    /// struct itself, not the class instance.
    ///
    /// Recursively scans statement and expression children, but stops at
    /// function/arrow scope boundaries (their `this` is their own).
    pub(super) fn detect_this_in_body(stmts: &[Statement]) -> bool {
        let found = Cell::new(false);
        for stmt in stmts {
            Self::scan_stmt_for_this(stmt, &found);
            if found.get() {
                return true;
            }
        }
        false
    }

    fn scan_stmt_for_this(stmt: &Statement, found: &Cell<bool>) {
        if found.get() {
            return;
        }
        crate::infer::ast_walk::for_each_stmt_child(
            stmt,
            &mut |s| Self::scan_stmt_for_this(s, found),
            &mut |e| Self::scan_expr_for_this(e, found),
            &mut |vd| {
                crate::infer::ast_walk::for_each_var_decl_init(vd, &mut |init| {
                    Self::scan_expr_for_this(init, found);
                });
            },
        );
    }

    fn scan_expr_for_this(expr: &Expression, found: &Cell<bool>) {
        if found.get() {
            return;
        }
        if matches!(expr, Expression::ThisExpression(_)) {
            found.set(true);
            return;
        }
        // Arrow functions do NOT bind their own `this` — they inherit it
        // from the enclosing scope. So we must recurse into the arrow body
        // to detect `this` references. for_each_expr_child routes both
        // FunctionExpression and ArrowFunctionExpression to on_fn_scope,
        // which stops recursion. We handle arrow functions here first so
        // their body is scanned; FunctionExpression correctly stops (it
        // binds its own this).
        if let Expression::ArrowFunctionExpression(af) = expr {
            for stmt in &af.body.statements {
                Self::scan_stmt_for_this(stmt, found);
                if found.get() {
                    return;
                }
            }
            return;
        }
        crate::infer::ast_walk::for_each_expr_child(
            expr,
            &mut |e| Self::scan_expr_for_this(e, found),
            &mut |_| {},    // on_ident: not relevant
            &mut |_| {},    // on_target: not relevant
            &mut |_| {},    // on_simple_target: not relevant
            &mut |_, _| {}, // on_fn_scope: stop — nested fn has its own this
        );
    }
}
