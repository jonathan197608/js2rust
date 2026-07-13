// native_proto/infer/fn_types.rs
// Function-level type inference: parameters, return types, async detection.
// Also contains private helper methods for TypeInferrer.

use super::helpers::{binding_name, expr_depends_on_anytype};
use super::{InferResult, TypeInferrer};
use crate::jsdoc;
use crate::types::ZigType;
use oxc_ast::ast::*;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

/// Split a string by commas that are at the top level (not inside nested braces).
/// This allows correct parsing of nested anonymous object types like:
///   "a: string, b: {c: number, d: boolean}"
/// which should split into ["a: string", "b: {c: number, d: boolean}"]
/// rather than ["a: string", "b: {c: number", "d: boolean}"].
fn split_at_top_level_commas(s: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0;
    let mut start = 0;
    for (i, c) in s.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    if start < s.len() {
        parts.push(&s[start..]);
    }
    parts
}

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

        // Handle rest parameter (...args) — register in var_types as JsAny
        // so that .length dispatch uses SliceLen (→ .len) instead of StringLen (→ utf16Len).
        // Do NOT add to fn_param_types since the lowerer handles rest param emission separately.
        if let Some(rest) = &fd.params.rest
            && let Some(rname) = binding_name(&rest.rest.argument)
        {
            self.var_types.insert(rname.to_string(), ZigType::JsAny);
        }

        // Walk body for local var types FIRST,
        // so return-type inference can reference them.
        if let Some(body) = &fd.body {
            for s in &body.statements {
                self.walk_stmt_for_types(s);
            }
        }

        // Refine parameter types from usage: if a parameter of Anytype
        // (non-export only) is used as the target of a string-specific method
        // call or property, refine it to Str. This reduces the need for
        // JSDoc @param {string}. We do NOT refine export function params
        // (which default to I64) because that would break C ABI signatures.
        if !is_export && let Some(body) = &fd.body {
            let param_names: Vec<String> = params
                .iter()
                .filter(|(_, r)| matches!(r, InferResult::Indeterminate))
                .map(|(n, _)| n.clone())
                .collect();
            if !param_names.is_empty() {
                let refined = RefCell::new(HashSet::new());
                for s in &body.statements {
                    Self::detect_string_param_usage(s, &param_names, &refined);
                }
                for pname in refined.borrow().iter() {
                    self.var_types.insert(pname.clone(), ZigType::Str);
                    if let Some(param_list) = self.fn_param_types.get_mut(fn_name) {
                        for (n, t) in param_list.iter_mut() {
                            if n == pname {
                                *t = ZigType::Str;
                            }
                        }
                    }
                }
            }
        }

        // Return type (after body walk so local vars are known)
        let ret_ty = self.infer_fn_return_type(fd, fn_name, is_export);
        match &ret_ty {
            InferResult::Definite(ty) => {
                // anytype is not valid as a Zig return type; default to JsAny
                if *ty == ZigType::Anytype {
                    self.fn_return_types
                        .insert(fn_name.to_string(), ZigType::JsAny);
                } else {
                    self.fn_return_types.insert(fn_name.to_string(), ty.clone());
                }
            }
            InferResult::Indeterminate => {
                self.fn_return_types
                    .insert(fn_name.to_string(), ZigType::JsAny);
            }
        }
    }

    // ============================================================
    // Function return type inference
    // ============================================================

    /// Unify types of all return expressions in a function.
    /// Returns Some(ZigType) if all return exprs agree, None on mismatch or no inferrable type.
    fn unify_return_expr_types(
        &mut self,
        return_exprs: &[&Expression],
        fn_name: &str,
    ) -> Option<ZigType> {
        let mut ty: Option<ZigType> = None;
        for expr in return_exprs {
            let expr_ty = self.infer_expr_type(expr);
            match (&ty, &expr_ty) {
                (None, InferResult::Definite(et)) => ty = Some(et.clone()),
                (Some(t), InferResult::Definite(et)) if *t != *et => {
                    self.errors.push(format!(
                        "Return type mismatch in '{}': expected {:?}, found {:?}",
                        fn_name, t, et
                    ));
                    return None;
                }
                _ => {}
            }
        }
        ty
    }

    pub(crate) fn infer_fn_return_type(
        &mut self,
        fd: &Function,
        fn_name: &str,
        is_export: bool,
    ) -> InferResult {
        // Export function: try @returns annotation FIRST
        if is_export && let Some(ty) = self.lookup_jsdoc_return_type(fn_name) {
            return InferResult::Definite(ty);
        }

        // Collect return expressions (shared by export & non-export)
        let return_exprs = Self::collect_return_exprs(fd);
        if return_exprs.is_empty() {
            return InferResult::Definite(ZigType::Void);
        }

        let ty = self.unify_return_expr_types(&return_exprs, fn_name);

        // Export: type mismatch or uninferrable → error
        if is_export {
            if let Some(definite_ty) = ty {
                return InferResult::Definite(definite_ty);
            }
            // Has return expressions but can't infer — report error and default
            self.errors.push(format!(
                "Export function '{}' must have @returns annotation (or return a value that can be inferred)",
                fn_name
            ));
            return InferResult::Definite(ZigType::Str);
        }

        // Non-export: resolve anytype/indeterminate
        match ty {
            Some(definite_ty) => {
                // When the return type is Anytype and all return expressions depend on
                // anytype parameters, use AnytypeReturn so the Emitter emits @TypeOf(...).
                // This lets Zig resolve the concrete type at the call site.
                if definite_ty == ZigType::Anytype
                    && Self::all_return_exprs_depend_on_anytype_params(
                        fn_name,
                        &return_exprs,
                        &self.fn_param_types,
                    )
                {
                    return InferResult::Definite(ZigType::AnytypeReturn);
                }
                InferResult::Definite(definite_ty)
            }
            None => {
                // No definite return type from expressions — check JSDoc
                if let Some(ty) = self.lookup_jsdoc_return_type(fn_name) {
                    return InferResult::Definite(ty);
                }
                // Check if all return exprs depend on anytype params.
                if Self::all_return_exprs_depend_on_anytype_params(
                    fn_name,
                    &return_exprs,
                    &self.fn_param_types,
                ) {
                    return InferResult::Definite(ZigType::AnytypeReturn);
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

    /// Check if all return expressions depend on anytype parameters.
    /// Returns true if:
    ///   1. There are return expressions
    ///   2. The function has at least one anytype parameter
    ///   3. ALL return expressions exclusively reference anytype parameters
    fn all_return_exprs_depend_on_anytype_params(
        fn_name: &str,
        return_exprs: &[&Expression],
        fn_param_types: &HashMap<String, Vec<(String, ZigType)>>,
    ) -> bool {
        if return_exprs.is_empty() {
            return false;
        }
        let anytype_params: HashSet<String> = fn_param_types
            .get(fn_name)
            .map(|params| {
                params
                    .iter()
                    .filter_map(|(name, ty)| {
                        if *ty == ZigType::Anytype {
                            Some(name.clone())
                        } else {
                            None
                        }
                    })
                    .collect::<HashSet<String>>()
            })
            .unwrap_or_default();
        if anytype_params.is_empty() {
            return false;
        }
        return_exprs
            .iter()
            .all(|expr| expr_depends_on_anytype(expr, &anytype_params))
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
            if let Some(pname) = binding_name(&param.pattern) {
                // Try JSDoc @param first (shared by export & non-export)
                if let Some(ty) = self.lookup_jsdoc_param_type(fn_name, pname) {
                    params.push((pname.to_string(), InferResult::Definite(ty)));
                } else if is_export {
                    // Export without JSDoc: default to I64
                    params.push((pname.to_string(), InferResult::Definite(ZigType::I64)));
                } else {
                    // Non-export without JSDoc: indeterminate → anytype
                    params.push((pname.to_string(), InferResult::Indeterminate));
                }
            }
        }

        params
    }

    /// Look up JSDoc @param type for a function parameter and convert to ZigType.
    fn lookup_jsdoc_param_type(&self, fn_name: &str, param_name: &str) -> Option<ZigType> {
        let data = self.jsdoc_data.as_ref()?;
        let param_list = data.param_types.get(fn_name)?;
        for (annot_name, type_name) in param_list {
            if annot_name == param_name {
                let zig_ty = jsdoc::jsdoc_type_to_zig(type_name, &data.typedefs);
                return Some(Self::zig_str_to_type(&zig_ty));
            }
        }
        None
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
    /// Supports:
    /// - Basic types: "string" → Str, "number" → I64, "boolean" → Bool
    /// - Named types: "User" → NamedStruct (if in typedefs)
    /// - Array types: "string[]" → ArrayList(Str), "User[]" → ArrayList(NamedStruct)
    /// - Anonymous object types: "{name: string, age: number}" → Struct
    pub(crate) fn jsdoc_str_to_zig_type(
        s: &str,
        typedefs: &HashMap<String, jsdoc::TypedefDef>,
    ) -> ZigType {
        let trimmed = s.trim();

        // Check for anonymous object type: {name: string, age: number}
        if trimmed.starts_with('{') && trimmed.ends_with('}') {
            return Self::parse_anonymous_object_type(trimmed, typedefs);
        }

        // Convert JSDoc type to Zig type string
        let zig_str = jsdoc::jsdoc_type_to_zig(trimmed, typedefs);

        // Check if it's a named type (in typedefs)
        if typedefs.contains_key(trimmed) {
            return ZigType::NamedStruct(trimmed.to_string());
        }

        // Check if the original JSDoc type is an array type (e.g., "string[]", "number[]")
        if let Some(base_jsdoc_type) = trimmed.strip_suffix("[]") {
            let base_zig_type = Self::jsdoc_str_to_zig_type(base_jsdoc_type, typedefs);
            return ZigType::ArrayList(Box::new(base_zig_type));
        }

        // Basic types
        Self::zig_str_to_type(&zig_str)
    }

    /// Parse anonymous object type: "{name: string, age: number}" → Struct
    fn parse_anonymous_object_type(
        s: &str,
        typedefs: &HashMap<String, jsdoc::TypedefDef>,
    ) -> ZigType {
        let inner = &s[1..s.len() - 1]; // Remove surrounding braces
        let mut fields = Vec::new();

        // Split by comma, respecting nested braces so that
        // {a: string, b: {c: number, d: boolean}} splits correctly.
        for part in split_at_top_level_commas(inner) {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }

            // Parse "name: type" or "name?: type" (optional)
            let colon_pos = part.find(':');
            if let Some(pos) = colon_pos {
                let field_name = part[..pos].trim().trim_end_matches('?').to_string();
                let field_type_str = part[pos + 1..].trim();

                // Recursively parse field type (supports nested objects)
                let field_type = Self::jsdoc_str_to_zig_type(field_type_str, typedefs);
                fields.push((field_name, field_type));
            }
        }

        ZigType::Struct(fields)
    }

    /// Look up JSDoc @returns annotation for a function and convert to ZigType.
    pub(crate) fn lookup_jsdoc_return_type(&self, fn_name: &str) -> Option<ZigType> {
        let jsdoc_data = self.jsdoc_data.as_ref()?;
        let ret_type_name = jsdoc_data.return_types.get(fn_name)?;
        Some(Self::jsdoc_str_to_zig_type(
            ret_type_name,
            &jsdoc_data.typedefs,
        ))
    }

    pub(crate) fn zig_str_to_type(s: &str) -> ZigType {
        ZigType::from_zig_str(s)
    }

    // ============================================================
    // Parameter type refinement from usage
    // ============================================================

    /// String-specific methods that unambiguously indicate the receiver is a string.
    /// Only includes methods that are (1) string-only (not on Array) and (2)
    /// have straightforward type implications when the parameter is refined.
    /// Excluded: match/matchAll/search (regex interaction), split/slice/
    /// substring/concat/at (also available on Array).
    const STRING_ONLY_METHODS: &'static [&'static str] = &[
        "charAt",
        "charCodeAt",
        "codePointAt",
        "toUpperCase",
        "toLowerCase",
        "toLocaleUpperCase",
        "toLocaleLowerCase",
        "trim",
        "trimStart",
        "trimEnd",
        "padStart",
        "padEnd",
        "repeat",
        "replace",
        "replaceAll",
        "normalize",
        "localeCompare",
    ];

    /// Detect parameters that are used as the target of string-specific
    /// method calls. Walks the statement recursively.
    fn detect_string_param_usage(
        stmt: &Statement,
        param_names: &[String],
        refined: &RefCell<HashSet<String>>,
    ) {
        crate::infer::ast_walk::for_each_stmt_child(
            stmt,
            &mut |s| Self::detect_string_param_usage(s, param_names, refined),
            &mut |e| Self::detect_string_param_usage_in_expr(e, param_names, refined),
            &mut |vd| {
                crate::infer::ast_walk::for_each_var_decl_init(vd, &mut |init| {
                    Self::detect_string_param_usage_in_expr(init, param_names, refined);
                });
            },
        );
    }

    /// Walk expression tree looking for string-only method calls on parameters.
    fn detect_string_param_usage_in_expr(
        expr: &Expression,
        param_names: &[String],
        refined: &RefCell<HashSet<String>>,
    ) {
        // Check for param.stringOnlyMethod(...) pattern before recursing.
        if let Expression::CallExpression(ce) = expr
            && let Expression::StaticMemberExpression(sme) = &ce.callee
            && let Some(name) = super::helpers::extract_expr_identifier_name(&sme.object)
            && param_names.contains(&name)
            && Self::STRING_ONLY_METHODS.contains(&sme.property.name.as_str())
        {
            refined.borrow_mut().insert(name);
        }

        crate::infer::ast_walk::for_each_expr_child(
            expr,
            &mut |e| Self::detect_string_param_usage_in_expr(e, param_names, refined),
            &mut |_| {}, // on_ident: not needed
            &mut |_| {}, // on_target: not needed
            &mut |_| {}, // on_simple_target: not needed
            &mut |_, _| {
                // Function/arrow scope boundary — stop recursing.
                // String parameter usage inside nested functions is an
                // inner-scope concern, not the outer function's.
            },
        );
    }
}
