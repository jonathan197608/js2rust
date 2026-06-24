// native_proto/infer/fn_types.rs
// Function-level type inference: parameters, return types, async detection.
// Also contains private helper methods for TypeInferrer.

use super::{InferResult, TypeInferrer};
use crate::native_proto::ZigType;
use crate::native_proto::jsdoc;
use oxc_ast::ast::*;
use std::collections::HashMap;

impl TypeInferrer {
    // ============================================================
    // walk_fn_for_types — shared by FunctionDeclaration and
    // ExportNamedDeclaration paths
    // ============================================================

    /// Helper: process a function for type inference (params, return type, body).
    /// Extracted so both `Statement::FunctionDeclaration` and
    /// `Statement::ExportNamedDeclaration` paths can share the same logic.
    pub(crate) fn walk_fn_for_types(
        &mut self,
        fd: &Function,
        fn_name: &str,
        from_export_decl: bool,
    ) {
        // Async detection
        if Self::fn_contains_await(fd) {
            self.is_async.insert(fn_name.to_string(), true);
        }

        // Param types
        let is_export = from_export_decl || self.is_fn_export(fn_name);
        let params = self.infer_fn_params(fd, is_export);
        for (pname, result) in &params {
            match result {
                InferResult::Definite(ty) => {
                    self.var_types.insert(pname.clone(), ty.clone());
                }
                InferResult::Indeterminate => {
                    self.var_types.insert(pname.clone(), ZigType::Anytype);
                }
            }
        }
        self.fn_param_types.insert(
            fn_name.to_string(),
            params
                .iter()
                .map(|(n, r)| match r {
                    InferResult::Definite(ty) => (n.clone(), ty.clone()),
                    InferResult::Indeterminate => (n.clone(), ZigType::Anytype),
                })
                .collect(),
        );

        // Walk body for local var types FIRST,
        // so return-type inference can reference them.
        if let Some(body) = &fd.body {
            for s in &body.statements {
                self.walk_stmt_for_types(s);
            }
        }

        // Return type (after body walk so local vars are known)
        let ret_ty = self.infer_fn_return_type(fd, fn_name, is_export);
        match &ret_ty {
            InferResult::Definite(ty) => {
                // anytype is not valid as a Zig return type; default to I64
                if *ty == ZigType::Anytype {
                    self.fn_return_types
                        .insert(fn_name.to_string(), ZigType::I64);
                } else {
                    self.fn_return_types.insert(fn_name.to_string(), ty.clone());
                }
            }
            InferResult::Indeterminate => {
                self.fn_return_types
                    .insert(fn_name.to_string(), ZigType::I64); // default
            }
        }
    }

    // ============================================================
    // Function return type inference
    // ============================================================

    pub(crate) fn infer_fn_return_type(
        &mut self,
        fd: &Function,
        fn_name: &str,
        is_export: bool,
    ) -> InferResult {
        // Export function: try @returns annotation FIRST
        if is_export {
            if let Some(ty) = self.lookup_jsdoc_return_type(fn_name) {
                return InferResult::Definite(ty);
            }
            // No @returns — try infer from return expressions (handles async host functions)
            let return_exprs = Self::collect_return_exprs(fd);
            if !return_exprs.is_empty() {
                let mut ty: Option<ZigType> = None;
                for expr in &return_exprs {
                    let expr_ty = self.infer_expr_type(expr);
                    match (&ty, &expr_ty) {
                        (None, InferResult::Definite(et)) => ty = Some(et.clone()),
                        (Some(t), InferResult::Definite(et)) if *t != *et => {
                            self.errors.push(format!(
                                "Return type mismatch in '{}': expected {:?}, found {:?}",
                                fn_name, t, et
                            ));
                            return InferResult::Indeterminate;
                        }
                        _ => {}
                    }
                }
                if let Some(definite_ty) = ty {
                    return InferResult::Definite(definite_ty);
                }
            }
            // Still can't infer — report error and default
            self.errors.push(format!(
                "Export function '{}' must have @returns annotation (or return a value that can be inferred)",
                fn_name
            ));
            return InferResult::Definite(ZigType::Str); // default for export
        }

        // Non-export: infer from return expressions
        let return_exprs = Self::collect_return_exprs(fd);
        if return_exprs.is_empty() {
            return InferResult::Definite(ZigType::Void);
        }

        let mut ty: Option<ZigType> = None;
        for expr in &return_exprs {
            let expr_ty = self.infer_expr_type(expr);
            match (&ty, &expr_ty) {
                (None, InferResult::Definite(et)) => ty = Some(et.clone()),
                (Some(t), InferResult::Definite(et)) if *t != *et => {
                    self.errors.push(format!(
                        "Return type mismatch in '{}': expected {:?}, found {:?}",
                        fn_name, t, et
                    ));
                    return InferResult::Indeterminate;
                }
                _ => {}
            }
        }

        match ty {
            Some(definite_ty) => InferResult::Definite(definite_ty),
            None => {
                // No definite return type from expressions — check JSDoc
                if let Some(ty) = self.lookup_jsdoc_return_type(fn_name) {
                    return InferResult::Definite(ty);
                }
                // Default to i64
                self.errors.push(format!(
                    "Cannot infer return type of '{}' (Rule 8). Defaulting to i64.",
                    fn_name
                ));
                InferResult::Definite(ZigType::I64)
            }
        }
    }

    pub(crate) fn collect_return_exprs<'a>(fd: &'a Function<'a>) -> Vec<&'a Expression<'a>> {
        let mut exprs = Vec::new();
        if let Some(body) = &fd.body {
            for stmt in &body.statements {
                Self::collect_returns(stmt, &mut exprs);
            }
        }
        exprs
    }

    pub(crate) fn collect_returns<'a>(
        stmt: &'a Statement<'a>,
        exprs: &mut Vec<&'a Expression<'a>>,
    ) {
        match stmt {
            Statement::ReturnStatement(rs) => {
                if let Some(ref arg) = rs.argument {
                    exprs.push(arg);
                }
            }
            Statement::IfStatement(is) => {
                Self::collect_returns(&is.consequent, exprs);
                if let Some(alt) = &is.alternate {
                    Self::collect_returns(alt, exprs);
                }
            }
            Statement::BlockStatement(bs) => {
                for s in &bs.body {
                    Self::collect_returns(s, exprs);
                }
            }
            Statement::WhileStatement(ws) => {
                Self::collect_returns(&ws.body, exprs);
            }
            _ => {}
        }
    }

    // ============================================================
    // Class method return type inference
    // ============================================================

    /// Infer return type of a class method by walking its body.
    /// Uses `self.current_class` and `class_field_types` to resolve `this.field`.
    pub(crate) fn infer_class_method_return_type(&mut self, md: &MethodDefinition) -> InferResult {
        let body = if let Some(body) = &md.value.body {
            body
        } else {
            return InferResult::Definite(ZigType::Void);
        };

        // Collect return expressions
        let mut return_exprs = Vec::new();
        for stmt in &body.statements {
            Self::collect_returns(stmt, &mut return_exprs);
        }
        if return_exprs.is_empty() {
            return InferResult::Definite(ZigType::Void);
        }

        // Infer each return expression type
        let mut ty: Option<ZigType> = None;
        for expr in &return_exprs {
            let expr_ty = self.infer_expr_type(expr);
            match (&ty, &expr_ty) {
                (None, InferResult::Definite(et)) => ty = Some(et.clone()),
                (Some(t), InferResult::Definite(et)) if *t != *et => {
                    return InferResult::Indeterminate; // mismatched types
                }
                _ => {}
            }
        }
        match ty {
            Some(definite_ty) => InferResult::Definite(definite_ty),
            None => InferResult::Indeterminate,
        }
    }

    // ============================================================
    // Function parameters (Rule 7)
    // ============================================================

    /// Infer function parameter types.
    /// Rule 7: Non-export function params → indeterminate → anytype.
    pub(crate) fn infer_fn_params(
        &mut self,
        fd: &Function,
        is_export: bool,
    ) -> Vec<(String, InferResult)> {
        let fn_name = fd.id.as_ref().map(|id| id.name.as_str()).unwrap_or("");
        let mut params = Vec::new();

        for param in &fd.params.items {
            if let Some(pname) = Self::binding_name(&param.pattern) {
                if is_export {
                    // Export: check JSDoc @param
                    let mut found_jsdoc = false;
                    if let Some(ref data) = self.jsdoc_data
                        && let Some(param_list) = data.param_types.get(fn_name)
                    {
                        for (annot_name, type_name) in param_list {
                            if annot_name == pname {
                                let zig_ty = jsdoc::jsdoc_type_to_zig(type_name, &data.typedefs);
                                params.push((
                                    pname.to_string(),
                                    InferResult::Definite(Self::zig_str_to_type(&zig_ty)),
                                ));
                                found_jsdoc = true;
                                break; // exit inner for, skip default
                            }
                        }
                    }
                    if !found_jsdoc {
                        // Default export param: I64
                        params.push((pname.to_string(), InferResult::Definite(ZigType::I64)));
                    }
                } else {
                    // Non-export: anytype
                    params.push((pname.to_string(), InferResult::Indeterminate));
                }
            }
        }
        params
    }

    // ============================================================
    // Async detection
    // ============================================================

    pub(crate) fn fn_contains_await(fd: &Function) -> bool {
        if let Some(body) = &fd.body {
            body.statements.iter().any(|s| Self::stmt_contains_await(s))
        } else {
            false
        }
    }

    pub(crate) fn stmt_contains_await(stmt: &Statement) -> bool {
        match stmt {
            Statement::ExpressionStatement(es) => Self::expr_contains_await(&es.expression),
            Statement::ReturnStatement(rs) => rs
                .argument
                .as_ref()
                .is_some_and(|e| Self::expr_contains_await(e)),
            Statement::VariableDeclaration(vd) => vd.declarations.iter().any(|d| {
                d.init
                    .as_ref()
                    .is_some_and(|e| Self::expr_contains_await(e))
            }),
            Statement::IfStatement(is) => {
                Self::expr_contains_await(&is.test)
                    || Self::stmt_contains_await(&is.consequent)
                    || is
                        .alternate
                        .as_ref()
                        .is_some_and(|a| Self::stmt_contains_await(a))
            }
            Statement::WhileStatement(ws) => {
                Self::expr_contains_await(&ws.test) || Self::stmt_contains_await(&ws.body)
            }
            Statement::DoWhileStatement(dws) => Self::stmt_contains_await(&dws.body),
            Statement::ForOfStatement(fos) => Self::stmt_contains_await(&fos.body),
            Statement::SwitchStatement(ss) => ss
                .cases
                .iter()
                .any(|c| c.consequent.iter().any(|s| Self::stmt_contains_await(s))),
            Statement::BlockStatement(bs) => bs.body.iter().any(|s| Self::stmt_contains_await(s)),
            _ => false,
        }
    }

    pub(crate) fn expr_contains_await(expr: &Expression) -> bool {
        match expr {
            Expression::AwaitExpression(_) => true,
            Expression::ParenthesizedExpression(pe) => Self::expr_contains_await(&pe.expression),
            Expression::BinaryExpression(be) => {
                Self::expr_contains_await(&be.left) || Self::expr_contains_await(&be.right)
            }
            Expression::LogicalExpression(le) => {
                Self::expr_contains_await(&le.left) || Self::expr_contains_await(&le.right)
            }
            Expression::ConditionalExpression(ce) => {
                Self::expr_contains_await(&ce.test)
                    || Self::expr_contains_await(&ce.consequent)
                    || Self::expr_contains_await(&ce.alternate)
            }
            Expression::CallExpression(ce) => {
                Self::expr_contains_await(&ce.callee)
                    || ce.arguments.iter().any(|a| {
                        a.as_expression()
                            .is_some_and(|e| Self::expr_contains_await(e))
                    })
            }
            Expression::UnaryExpression(ue) => Self::expr_contains_await(&ue.argument),
            Expression::ArrayExpression(ae) => ae.elements.iter().any(|e| {
                e.as_expression()
                    .is_some_and(|e| Self::expr_contains_await(e))
            }),
            _ => false,
        }
    }

    // ============================================================
    // Helpers (private to TypeInferrer)
    // ============================================================

    pub(crate) fn is_fn_export(&self, fn_name: &str) -> bool {
        self.exported_functions
            .as_ref()
            .is_some_and(|set| set.contains(fn_name))
    }

    pub(crate) fn binding_name<'a>(pattern: &BindingPattern<'a>) -> Option<&'a str> {
        match pattern {
            BindingPattern::BindingIdentifier(id) => Some(id.name.as_str()),
            _ => None,
        }
    }

    /// Check if an initializer is JSON.parse() and return the @type annotation.
    pub(crate) fn get_json_parse_type(&self, var_name: &str, init: &Expression) -> Option<String> {
        let ce = match init {
            Expression::CallExpression(ce) => ce,
            _ => return None,
        };
        let is_json_parse = match &ce.callee {
            Expression::StaticMemberExpression(mem) => {
                if let Expression::Identifier(obj_id) = &mem.object {
                    obj_id.name.as_str() == "JSON" && mem.property.name.as_str() == "parse"
                } else {
                    false
                }
            }
            _ => false,
        };
        if !is_json_parse {
            return None;
        }
        self.jsdoc_data
            .as_ref()
            .and_then(|d| d.type_annotations.get(var_name))
            .cloned()
    }

    /// Convert a JSDoc type string to ZigType.
    pub(crate) fn jsdoc_str_to_zig_type(
        s: &str,
        typedefs: &HashMap<String, jsdoc::TypedefDef>,
    ) -> ZigType {
        let zig_str = jsdoc::jsdoc_type_to_zig(s, typedefs);
        Self::zig_str_to_type(&zig_str)
    }

    /// Look up JSDoc @returns annotation for a function and convert to ZigType.
    pub(crate) fn lookup_jsdoc_return_type(&self, fn_name: &str) -> Option<ZigType> {
        let jsdoc_data = self.jsdoc_data.as_ref()?;
        let ret_type_name = jsdoc_data.return_types.get(fn_name)?;
        let zig_ty = jsdoc::jsdoc_type_to_zig(ret_type_name, &jsdoc_data.typedefs);
        Some(Self::zig_str_to_type(&zig_ty))
    }

    pub(crate) fn zig_str_to_type(s: &str) -> ZigType {
        match s {
            "i64" => ZigType::I64,
            "f64" => ZigType::F64,
            "bool" => ZigType::Bool,
            "[]const u8" => ZigType::Str,
            "void" => ZigType::Void,
            _ => ZigType::I64, // default
        }
    }
}
