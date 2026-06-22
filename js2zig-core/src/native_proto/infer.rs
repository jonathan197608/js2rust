// native_proto/infer.rs
// Type inference for native_proto mode.
// Follows the 8-rule simplification plan:
// 1. Literal expressions → definite type (use JSDoc if available)
// 2. Binary expressions → definite only if BOTH operands are literals
// 3. Other expressions → indeterminate (None)
// 4. const → no type annotation, let Zig infer
// 5. Local variables → check ALL assignments, at least one definite
// 6. Return types → check ALL return expressions, at least one definite
// 7. Non-export function params → indeterminate → anytype
// 8. Indeterminate → report compile error
//
// Phase A: All type inference runs BEFORE codegen.
// TypeInferrer walks the full AST once and produces a TypeCheckResult
// that Codegen reads (purely generative after that).

use oxc_ast::ast::*;
use std::collections::{HashMap, HashSet};

use crate::native_proto::JSDocData;
use crate::native_proto::ZigType;
use crate::native_proto::jsdoc;

/// Result of type inference: either a definite type or indeterminate.
#[derive(Debug, Clone, PartialEq)]
pub enum InferResult {
    /// Definite type
    Definite(ZigType),
    /// Indeterminate (cannot infer from context)
    Indeterminate,
}

// ── TypeInferResult: read-only snapshot passed to Codegen ──

/// Complete type-checking result computed by TypeInferrer.
/// Codegen reads from this during the code-generation pass — no writes.
#[derive(Debug, Clone)]
pub struct TypeCheckResult {
    /// Variable → inferred type (toplevel + function-local, keyed by name only)
    pub var_types: HashMap<String, ZigType>,
    /// Array variable → element type
    pub array_element_types: HashMap<String, ZigType>,
    /// Function name → return type
    pub fn_return_types: HashMap<String, ZigType>,
    /// Function name → [(param_name, param_type)]
    pub fn_param_types: HashMap<String, Vec<(String, ZigType)>>,
    /// Variable names that must use `var` (member-assignment target)
    pub mutated_vars: HashSet<String>,
    /// Identifier names referenced anywhere (for unused-constant elimination)
    pub used_names: HashSet<String>,
    /// Variable names initialized with JSON.parse(@type)
    pub has_json_parse_types: HashSet<String>,
    /// Type-check errors (Rule 8 violations, etc.)
    pub errors: Vec<String>,
    /// Whether each function is async (needs io: anytype)
    pub is_async: HashMap<String, bool>,
}

// ── TypeInferrer ────────────────────────────────────

/// Simplified type inferrer for native_proto mode.
pub struct TypeInferrer {
    /// Variable types inferred from initializers
    var_types: HashMap<String, ZigType>,
    /// Array element types (for ArrayList push type checking)
    array_element_types: HashMap<String, ZigType>,
    /// Function name → return type
    fn_return_types: HashMap<String, ZigType>,
    /// Function name → [(param_name, param_type)]
    fn_param_types: HashMap<String, Vec<(String, ZigType)>>,
    /// Set of mutated variables (need `var` instead of `const`)
    mutated_vars: HashSet<String>,
    /// Identifier names referenced anywhere
    used_names: HashSet<String>,
    /// Variable names initialized with JSON.parse(@type)
    has_json_parse_types: HashSet<String>,
    /// Whether each function is async
    is_async: HashMap<String, bool>,
    /// Collected errors (reported during type checking)
    pub errors: Vec<String>,
    /// JSDoc data for type annotations
    jsdoc_data: Option<JSDocData>,
    /// Exported function names (from pipeline)
    exported_functions: Option<HashSet<String>>,
}

impl TypeInferrer {
    pub fn new() -> Self {
        Self {
            var_types: HashMap::new(),
            array_element_types: HashMap::new(),
            fn_return_types: HashMap::new(),
            fn_param_types: HashMap::new(),
            mutated_vars: HashSet::new(),
            used_names: HashSet::new(),
            has_json_parse_types: HashSet::new(),
            is_async: HashMap::new(),
            errors: Vec::new(),
            jsdoc_data: None,
            exported_functions: None,
        }
    }

    /// Set JSDoc data for type annotations
    pub fn set_jsdoc_data(&mut self, data: JSDocData) {
        self.jsdoc_data = Some(data);
    }

    // ============================================================
    // Main entry: run all passes
    // ============================================================

    /// Run all type-inference passes on a program and return the result.
    /// After this, Codegen can generate code without doing any inference.
    pub fn infer_all(
        &mut self,
        program: &Program,
        exported_functions: Option<HashSet<String>>,
    ) -> TypeCheckResult {
        self.exported_functions = exported_functions;

        // Pass 0: Analyze objects — detect mutations and dynamic access errors.
        self.analyze_objects(program);

        // Pass 1: Collect referenced names (for unused-constant elimination).
        self.collect_used_names(program);

        // Pass 2: Walk ALL scopes (top-level + function bodies) to collect types.
        self.walk_toplevel_for_types(program);

        // Produce snapshot.
        TypeCheckResult {
            var_types: std::mem::take(&mut self.var_types),
            array_element_types: std::mem::take(&mut self.array_element_types),
            fn_return_types: std::mem::take(&mut self.fn_return_types),
            fn_param_types: std::mem::take(&mut self.fn_param_types),
            mutated_vars: std::mem::take(&mut self.mutated_vars),
            used_names: std::mem::take(&mut self.used_names),
            has_json_parse_types: std::mem::take(&mut self.has_json_parse_types),
            errors: std::mem::take(&mut self.errors),
            is_async: std::mem::take(&mut self.is_async),
        }
    }

    // ============================================================
    // Rule 1: Literal expressions → definite type
    // ============================================================

    /// Infer the type of an expression.
    pub fn infer_expr_type(&mut self, expr: &Expression) -> InferResult {
        match expr {
            Expression::NumericLiteral(n) => {
                let s = n.value.to_string();
                if s.contains('.') || s.contains('e') || s.contains('E') {
                    InferResult::Definite(ZigType::F64)
                } else {
                    InferResult::Definite(ZigType::I64)
                }
            }
            Expression::StringLiteral(_) => InferResult::Definite(ZigType::Str),
            Expression::TemplateLiteral(_) => InferResult::Definite(ZigType::Str),
            Expression::BooleanLiteral(_) => InferResult::Definite(ZigType::Bool),
            Expression::NullLiteral(_) => InferResult::Indeterminate,

            // Identifier: look up from var_types
            Expression::Identifier(id) => {
                // JSDoc @type annotation takes priority
                if let Some(ref data) = self.jsdoc_data
                    && let Some(ty_str) = data.type_annotations.get(id.name.as_str())
                {
                    return InferResult::Definite(Self::jsdoc_str_to_zig_type(
                        ty_str,
                        &data.typedefs,
                    ));
                }
                // Then var_types
                if let Some(ty) = self.var_types.get(id.name.as_str()) {
                    // Anytype params are indeterminate for type inference
                    if ty == &ZigType::Anytype {
                        InferResult::Indeterminate
                    } else {
                        InferResult::Definite(ty.clone())
                    }
                } else {
                    InferResult::Indeterminate
                }
            }

            // Binary expression → definite only if BOTH operands are literals
            Expression::BinaryExpression(be) => {
                let left = self.infer_expr_type(&be.left);
                let right = self.infer_expr_type(&be.right);
                match (left, right) {
                    (InferResult::Definite(l), InferResult::Definite(r)) => {
                        InferResult::Definite(Self::infer_binary_type(be.operator, l, r))
                    }
                    _ => InferResult::Indeterminate,
                }
            }

            Expression::LogicalExpression(_) => InferResult::Definite(ZigType::Bool),

            Expression::UnaryExpression(ue) => match ue.operator {
                UnaryOperator::LogicalNot => InferResult::Definite(ZigType::Bool),
                UnaryOperator::UnaryNegation | UnaryOperator::UnaryPlus => {
                    match self.infer_expr_type(&ue.argument) {
                        InferResult::Definite(ty) => InferResult::Definite(ty),
                        InferResult::Indeterminate => InferResult::Indeterminate,
                    }
                }
                _ => InferResult::Indeterminate,
            },

            // Array: definite if all elements have same definite type
            Expression::ArrayExpression(ae) => self.infer_array_type(ae),

            // Object: definite as Struct
            Expression::ObjectExpression(oe) => self.infer_object_type(oe),

            // NewExpression: new Map(), new Set()
            Expression::NewExpression(ne) => {
                if let Expression::Identifier(id) = &ne.callee {
                    match id.name.as_str() {
                        "Map" => InferResult::Definite(ZigType::NamedStruct("Map".to_string())),
                        "Set" => InferResult::Definite(ZigType::NamedStruct("Set".to_string())),
                        _ => InferResult::Indeterminate,
                    }
                } else {
                    InferResult::Indeterminate
                }
            }

            // CallExpression: look up from fn_return_types cache
            Expression::CallExpression(ce) => {
                match &ce.callee {
                    Expression::Identifier(id) => {
                        if let Some(ret_ty) = self.fn_return_types.get(id.name.as_str()) {
                            return InferResult::Definite(ret_ty.clone());
                        }
                    }
                    // Method calls: arr.slice(), arr.map(), arr.filter(), etc.
                    Expression::StaticMemberExpression(mem) => {
                        if let Some(obj_name) = extract_expr_identifier_name(&mem.object) {
                            // Array methods
                            if let Some(elem_ty) = self.array_element_types.get(&obj_name) {
                                return self.infer_array_method_return(
                                    mem.property.name.as_str(),
                                    elem_ty,
                                );
                            }
                            // Map/Set methods
                            if let Some(var_ty) = self.var_types.get(&obj_name) {
                                return self
                                    .infer_named_method_return(var_ty, mem.property.name.as_str());
                            }
                        }
                    }
                    _ => {}
                }
                InferResult::Indeterminate
            }

            // Static member access
            Expression::StaticMemberExpression(mem) => {
                match self.infer_expr_type(&mem.object) {
                    InferResult::Definite(ZigType::Str) => match mem.property.name.as_str() {
                        "length" => InferResult::Definite(ZigType::I64),
                        _ => InferResult::Indeterminate,
                    },
                    InferResult::Definite(ZigType::Struct(fields)) => {
                        let field_name = mem.property.name.as_str();
                        for (name, ty) in &fields {
                            if name == field_name {
                                return InferResult::Definite(ty.clone());
                            }
                        }
                        InferResult::Indeterminate
                    }
                    // Map/Set property access: .size
                    InferResult::Definite(ZigType::NamedStruct(ref name))
                        if name == "Map" || name == "Set" =>
                    {
                        match mem.property.name.as_str() {
                            "size" => InferResult::Definite(ZigType::I64),
                            _ => InferResult::Indeterminate,
                        }
                    }
                    _ => InferResult::Indeterminate,
                }
            }

            // Everything else → indeterminate
            _ => InferResult::Indeterminate,
        }
    }

    fn infer_array_type(&mut self, ae: &ArrayExpression) -> InferResult {
        if ae.elements.is_empty() {
            self.errors.push(
                "Cannot infer element type for empty array. Use ArrayList with explicit type."
                    .to_string(),
            );
            return InferResult::Indeterminate;
        }
        let first = match ae.elements.first() {
            Some(e) => e,
            None => return InferResult::Indeterminate,
        };
        let first_expr = match first.as_expression() {
            Some(e) => e,
            None => return InferResult::Indeterminate,
        };
        let elem_ty = match self.infer_expr_type(first_expr) {
            InferResult::Definite(et) => et,
            InferResult::Indeterminate => return InferResult::Indeterminate,
        };
        for elem in ae.elements.iter().skip(1) {
            if let Some(e) = elem.as_expression() {
                match self.infer_expr_type(e) {
                    InferResult::Definite(t) if t == elem_ty => {}
                    _ => return InferResult::Indeterminate,
                }
            }
        }
        InferResult::Definite(ZigType::ArrayList(Box::new(elem_ty)))
    }

    fn infer_object_type(&mut self, oe: &ObjectExpression) -> InferResult {
        let mut fields: Vec<(String, ZigType)> = Vec::new();
        for prop in &oe.properties {
            let p = match prop {
                ObjectPropertyKind::ObjectProperty(p) => p,
                _ => return InferResult::Indeterminate,
            };
            let field_name = match &p.key {
                PropertyKey::StaticIdentifier(id) => id.name.to_string(),
                PropertyKey::StringLiteral(s) => s.value.to_string(),
                _ => return InferResult::Indeterminate,
            };
            match self.infer_expr_type(&p.value) {
                InferResult::Definite(ft) => fields.push((field_name, ft)),
                InferResult::Indeterminate => return InferResult::Indeterminate,
            }
        }
        InferResult::Definite(ZigType::Struct(fields))
    }

    fn infer_binary_type(op: BinaryOperator, left: ZigType, right: ZigType) -> ZigType {
        match op {
            BinaryOperator::Addition
            | BinaryOperator::Subtraction
            | BinaryOperator::Multiplication
            | BinaryOperator::Division
            | BinaryOperator::Remainder
            | BinaryOperator::Exponential => {
                if left == ZigType::F64 || right == ZigType::F64 {
                    ZigType::F64
                } else {
                    ZigType::I64
                }
            }
            BinaryOperator::Equality
            | BinaryOperator::Inequality
            | BinaryOperator::StrictEquality
            | BinaryOperator::StrictInequality
            | BinaryOperator::LessThan
            | BinaryOperator::LessEqualThan
            | BinaryOperator::GreaterThan
            | BinaryOperator::GreaterEqualThan => ZigType::Bool,
            BinaryOperator::BitwiseAnd | BinaryOperator::BitwiseOR | BinaryOperator::BitwiseXOR => {
                ZigType::I64
            }
            BinaryOperator::ShiftLeft
            | BinaryOperator::ShiftRight
            | BinaryOperator::ShiftRightZeroFill => ZigType::I64,
            _ => ZigType::I64,
        }
    }

    // ============================================================
    // Pass 0: analyze objects (mutations, dynamic access errors)
    // ============================================================

    fn analyze_objects(&mut self, program: &Program) {
        for stmt in &program.body {
            self.walk_stmt_for_analysis(stmt);
        }
    }

    fn walk_stmt_for_analysis(&mut self, stmt: &Statement) {
        match stmt {
            Statement::VariableDeclaration(vd) => {
                for decl in &vd.declarations {
                    if let Some(init) = &decl.init {
                        self.walk_expr_for_analysis(init);
                    }
                }
            }
            Statement::FunctionDeclaration(fd) => {
                if let Some(body) = &fd.body {
                    for s in &body.statements {
                        self.walk_stmt_for_analysis(s);
                    }
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
            Statement::WhileStatement(ws) => {
                self.walk_expr_for_analysis(&ws.test);
                self.walk_stmt_for_analysis(&ws.body);
            }
            Statement::BlockStatement(bs) => {
                for s in &bs.body {
                    self.walk_stmt_for_analysis(s);
                }
            }
            _ => {}
        }
    }

    fn walk_expr_for_analysis(&mut self, expr: &Expression) {
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
            _ => {}
        }
    }

    fn check_assignment_target(&mut self, target: &AssignmentTarget) {
        match target {
            AssignmentTarget::AssignmentTargetIdentifier(id) => {
                // Simple variable assignment: e.g., `total = total + 1`
                self.mutated_vars.insert(id.name.to_string());
            }
            AssignmentTarget::StaticMemberExpression(mem) => {
                if let Expression::Identifier(id) = &mem.object {
                    self.mutated_vars.insert(id.name.to_string());
                }
            }
            AssignmentTarget::ComputedMemberExpression(mem) => {
                if let Expression::Identifier(id) = &mem.object {
                    self.mutated_vars.insert(id.name.to_string());
                }
            }
            _ => {}
        }
    }

    // ============================================================
    // Pass 1: collect used names (unused-constant elimination)
    // ============================================================

    fn collect_used_names(&mut self, program: &Program) {
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

    fn collect_idents_from_function(fd: &Function, names: &mut HashSet<String>) {
        if let Some(body) = &fd.body {
            for stmt in &body.statements {
                Self::collect_idents_from_stmt(stmt, names);
            }
        }
    }

    fn collect_idents_from_stmt(stmt: &Statement, names: &mut HashSet<String>) {
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

    fn collect_idents_from_expr(expr: &Expression, names: &mut HashSet<String>) {
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

    fn walk_toplevel_for_types(&mut self, program: &Program) {
        for stmt in &program.body {
            self.walk_stmt_for_types(stmt);
        }
    }

    /// Walk a statement to collect variable types (no code generation).
    fn walk_stmt_for_types(&mut self, stmt: &Statement) {
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
            Statement::BlockStatement(bs) => {
                for s in &bs.body {
                    self.walk_stmt_for_types(s);
                }
            }
            _ => {}
        }
    }

    /// Helper: process a function for type inference (params, return type, body).
    /// Extracted so both `Statement::FunctionDeclaration` and
    /// `Statement::ExportNamedDeclaration` paths can share the same logic.
    fn walk_fn_for_types(&mut self, fd: &Function, fn_name: &str, from_export_decl: bool) {
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

    fn collect_var_types_from_decl(&mut self, vd: &VariableDeclaration) {
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

    // ============================================================
    // Function return type inference
    // ============================================================

    fn infer_fn_return_type(
        &mut self,
        fd: &Function,
        fn_name: &str,
        is_export: bool,
    ) -> InferResult {
        // Export function: require @returns annotation
        if is_export {
            if let Some(ty) = self.lookup_jsdoc_return_type(fn_name) {
                return InferResult::Definite(ty);
            }
            self.errors.push(format!(
                "Export function '{}' must have @returns annotation",
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

    fn collect_return_exprs<'a>(fd: &'a Function<'a>) -> Vec<&'a Expression<'a>> {
        let mut exprs = Vec::new();
        if let Some(body) = &fd.body {
            for stmt in &body.statements {
                Self::collect_returns(stmt, &mut exprs);
            }
        }
        exprs
    }

    fn collect_returns<'a>(stmt: &'a Statement<'a>, exprs: &mut Vec<&'a Expression<'a>>) {
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
    // Function parameters (Rule 7)
    // ============================================================

    /// Infer function parameter types.
    /// Rule 7: Non-export function params → indeterminate → anytype.
    pub fn infer_fn_params(
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

    fn fn_contains_await(fd: &Function) -> bool {
        if let Some(body) = &fd.body {
            body.statements.iter().any(|s| Self::stmt_contains_await(s))
        } else {
            false
        }
    }

    fn stmt_contains_await(stmt: &Statement) -> bool {
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

    fn expr_contains_await(expr: &Expression) -> bool {
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
    // Helpers
    // ============================================================

    fn is_fn_export(&self, fn_name: &str) -> bool {
        self.exported_functions
            .as_ref()
            .is_some_and(|set| set.contains(fn_name))
    }

    fn binding_name<'a>(pattern: &BindingPattern<'a>) -> Option<&'a str> {
        match pattern {
            BindingPattern::BindingIdentifier(id) => Some(id.name.as_str()),
            _ => None,
        }
    }

    /// Check if an initializer is JSON.parse() and return the @type annotation.
    fn get_json_parse_type(&self, var_name: &str, init: &Expression) -> Option<String> {
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
    fn jsdoc_str_to_zig_type(s: &str, typedefs: &HashMap<String, jsdoc::TypedefDef>) -> ZigType {
        let zig_str = jsdoc::jsdoc_type_to_zig(s, typedefs);
        Self::zig_str_to_type(&zig_str)
    }

    /// Look up JSDoc @returns annotation for a function and convert to ZigType.
    fn lookup_jsdoc_return_type(&self, fn_name: &str) -> Option<ZigType> {
        let jsdoc_data = self.jsdoc_data.as_ref()?;
        let ret_type_name = jsdoc_data.return_types.get(fn_name)?;
        let zig_ty = jsdoc::jsdoc_type_to_zig(ret_type_name, &jsdoc_data.typedefs);
        Some(Self::zig_str_to_type(&zig_ty))
    }

    fn zig_str_to_type(s: &str) -> ZigType {
        match s {
            "i64" => ZigType::I64,
            "f64" => ZigType::F64,
            "bool" => ZigType::Bool,
            "[]const u8" => ZigType::Str,
            "void" => ZigType::Void,
            _ => ZigType::I64, // default
        }
    }

    /// Infer the return type of array method calls like arr.slice(), arr.map(), etc.
    fn infer_array_method_return(&self, method: &str, elem_ty: &ZigType) -> InferResult {
        match method {
            // Methods that return a new array (same element type)
            "slice" | "map" | "filter" | "concat" | "reverse" | "sort" | "splice" | "flat" => {
                InferResult::Definite(ZigType::ArrayList(Box::new(elem_ty.clone())))
            }
            // Methods that return a boolean
            "some" | "every" | "includes" => InferResult::Definite(ZigType::Bool),
            // Methods returning index or length
            "indexOf" | "lastIndexOf" | "findIndex" => InferResult::Definite(ZigType::I64),
            // reduce: returns accumulator type (default i64)
            "reduce" | "reduceRight" => InferResult::Definite(ZigType::I64),
            // pop/shift: return element type
            "pop" | "shift" | "find" => InferResult::Definite(elem_ty.clone()),
            // join: returns string
            "join" => InferResult::Definite(ZigType::Str),
            // push/unshift: return new length (i64)
            "push" | "unshift" => InferResult::Definite(ZigType::I64),
            _ => InferResult::Indeterminate,
        }
    }

    /// Infer the return type of method calls on Map/Set/NamedStruct objects.
    fn infer_named_method_return(&self, var_ty: &ZigType, method: &str) -> InferResult {
        match var_ty {
            ZigType::NamedStruct(name) => {
                match name.as_str() {
                    "Map" => match method {
                        "set" => InferResult::Indeterminate,          // void/mutating
                        "get" => InferResult::Definite(ZigType::I64), // default value type
                        "has" | "delete" => InferResult::Definite(ZigType::Bool),
                        _ => InferResult::Indeterminate,
                    },
                    "Set" => match method {
                        "add" => InferResult::Indeterminate, // void/mutating
                        "has" | "delete" => InferResult::Definite(ZigType::Bool),
                        _ => InferResult::Indeterminate,
                    },
                    _ => InferResult::Indeterminate,
                }
            }
            _ => InferResult::Indeterminate,
        }
    }
}

// ── Public utilities (used by Codegen) ──────────────

/// Extract variable name from a binding pattern.
pub fn binding_name<'a>(pattern: &BindingPattern<'a>) -> Option<&'a str> {
    match pattern {
        BindingPattern::BindingIdentifier(id) => Some(id.name.as_str()),
        _ => None,
    }
}

/// Extract the identifier name from an Expression if it is an Identifier.
pub fn extract_expr_identifier_name(expr: &Expression) -> Option<String> {
    match expr {
        Expression::Identifier(id) => Some(id.name.to_string()),
        _ => None,
    }
}
