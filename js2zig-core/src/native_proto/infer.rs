// native_proto/infer.rs
// Simplified type inference for native_proto mode.
// Follows the 8-rule simplification plan:
// 1. Literal expressions → definite type (use JSDoc if available)
// 2. Binary expressions → definite only if BOTH operands are literals
// 3. Other expressions → indeterminate (None)
// 4. const → no type annotation, let Zig infer
// 5. Local variables → check ALL assignments, at least one definite
// 6. Return types → check ALL return expressions, at least one definite
// 7. Non-export function params → indeterminate → anytype
// 8. Indeterminate → report compile error

use oxc_ast::ast::*;
use std::collections::{HashMap, HashSet};

use crate::native_proto::jsdoc;
use crate::native_proto::ZigType;

/// Result of type inference: either a definite type or indeterminate.
#[derive(Debug, Clone, PartialEq)]
pub enum InferResult {
    /// Definite type
    Definite(ZigType),
    /// Indeterminate (cannot infer from context)
    Indeterminate,
}

/// Simplified type inferrer for native_proto mode.
pub struct TypeInferrer {
    /// Variable types inferred from initializers
    pub var_types: HashMap<String, ZigType>,
    /// Array element types (for ArrayList push type checking)
    pub array_element_types: HashMap<String, ZigType>,
    /// Collected errors (reported during type checking)
    pub errors: Vec<String>,
    /// JSDoc data for type annotations
    pub jsdoc_data: Option<jsdoc::JSDocData>,
    /// Set of mutated variables (need `var` instead of `const`)
    pub mutated_vars: HashSet<String>,
}

impl TypeInferrer {
    pub fn new() -> Self {
        Self {
            var_types: HashMap::new(),
            array_element_types: HashMap::new(),
            errors: Vec::new(),
            jsdoc_data: None,
            mutated_vars: HashSet::new(),
        }
    }

    /// Set JSDoc data for type annotations
    pub fn set_jsdoc_data(&mut self, data: jsdoc::JSDocData) {
        self.jsdoc_data = Some(data);
    }

    // ============================================================
    // Rule 1: Literal expressions → definite type
    // ============================================================

    /// Infer the type of an expression.
    /// Returns `InferResult::Definite(type)` if the type can be determined,
    /// `InferResult::Indeterminate` otherwise.
    pub fn infer_expr_type(&mut self, expr: &Expression) -> InferResult {
        match expr {
            // Literals → definite type
            Expression::NumericLiteral(n) => {
                let s = n.value.to_string();
                if s.contains('.') || s.contains('e') || s.contains('E') {
                    InferResult::Definite(ZigType::F64)
                } else {
                    InferResult::Definite(ZigType::I64)
                }
            }
            Expression::StringLiteral(_) => InferResult::Definite(ZigType::Str),
            Expression::BooleanLiteral(_) => InferResult::Definite(ZigType::Bool),
            Expression::NullLiteral(_) => InferResult::Definite(ZigType::Null),

            // Rule 1: JSDoc @type annotation
            Expression::Identifier(id) => {
                if let Some(ref data) = self.jsdoc_data {
                    if let Some(ty) = data.type_annotations.get(id.name.as_str()) {
                        return InferResult::Definite(jsdoc::jsdoc_type_to_zig_type(ty, &data.typedefs));
                    }
                }
                // Look up variable type from var_types
                if let Some(ty) = self.var_types.get(id.name.as_str()) {
                    InferResult::Definite(ty.clone())
                } else {
                    InferResult::Indeterminate
                }
            }

            // Rule 2: Binary expression → definite only if BOTH operands are literals
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

            // Logical expression → Bool
            Expression::LogicalExpression(_) => InferResult::Definite(ZigType::Bool),

            // Unary expression
            Expression::UnaryExpression(ue) => {
                match ue.operator {
                    UnaryOperator::LogicalNot => InferResult::Definite(ZigType::Bool),
                    UnaryOperator::UnaryNegation | UnaryOperator::UnaryPlus => {
                        let operand = self.infer_expr_type(&ue.argument);
                        match operand {
                            InferResult::Definite(ty) => InferResult::Definite(ty),
                            InferResult::Indeterminate => InferResult::Indeterminate,
                        }
                    }
                    _ => InferResult::Indeterminate,
                }
            }

            // Array expression → definite if all elements have the same definite type
            Expression::ArrayExpression(ae) => {
                if ae.elements.is_empty() {
                    self.errors.push(
                        "Cannot infer element type for empty array. Use ArrayList with explicit type.".to_string()
                    );
                    return InferResult::Indeterminate;
                }
                // Check first element
                let first = ae.elements.first();
                if let Some(elem) = first {
                    if let Some(e) = elem.as_expression() {
                        let elem_ty = self.infer_expr_type(e);
                        match elem_ty {
                            InferResult::Definite(et) => {
                                // Check all elements have the same type
                                let mut all_same = true;
                                for elem in ae.elements.iter().skip(1) {
                                    if let Some(e) = elem.as_expression() {
                                        let ty = self.infer_expr_type(e);
                                        if ty != InferResult::Definite(et.clone()) {
                                            all_same = false;
                                            break;
                                        }
                                    }
                                }
                                if all_same {
                                    InferResult::Definite(ZigType::ArrayList(Box::new(et)))
                                } else {
                                    InferResult::Indeterminate
                                }
                            }
                            InferResult::Indeterminate => InferResult::Indeterminate,
                        }
                    } else {
                        InferResult::Indeterminate
                    }
                } else {
                    InferResult::Indeterminate
                }
            }

            // Object expression → definite as Struct
            Expression::ObjectExpression(oe) => {
                let mut fields: Vec<(String, ZigType)> = Vec::new();
                for prop in &oe.properties {
                    if let ObjectPropertyKind::ObjectProperty(p) = prop {
                        let field_name = match &p.key {
                            PropertyKey::StaticIdentifier(id) => id.name.to_string(),
                            PropertyKey::StringLiteral(s) => s.value.to_string(),
                            _ => continue,
                        };
                        let field_type = self.infer_expr_type(&p.value);
                        match field_type {
                            InferResult::Definite(ft) => fields.push((field_name, ft)),
                            InferResult::Indeterminate => return InferResult::Indeterminate,
                        }
                    }
                }
                InferResult::Definite(ZigType::Struct(fields))
            }

            // Member expression → may be definite (e.g., string.length → I64)
            Expression::StaticMemberExpression(mem) => {
                let obj_ty = self.infer_expr_type(&mem.object);
                match obj_ty {
                    InferResult::Definite(ZigType::Str) => {
                        match mem.property.name.as_str() {
                            "length" => InferResult::Definite(ZigType::I64),
                            _ => InferResult::Indeterminate,
                        }
                    }
                    _ => InferResult::Indeterminate,
                }
            }

            // Rule 3: Other cases → indeterminate
            _ => InferResult::Indeterminate,
        }
    }

    /// Infer binary expression result type
    fn infer_binary_type(op: BinaryOperator, left: ZigType, right: ZigType) -> ZigType {
        match op {
            // Arithmetic operators
            BinaryOperator::Addition | BinaryOperator::Subtraction |
            BinaryOperator::Multiplication | BinaryOperator::Division |
            BinaryOperator::Remainder | BinaryOperator::Exponential => {
                if left == ZigType::F64 || right == ZigType::F64 {
                    ZigType::F64
                } else {
                    ZigType::I64
                }
            }
            // Comparison operators → Bool
            BinaryOperator::Equality | BinaryOperator::Inequality |
            BinaryOperator::StrictEquality | BinaryOperator::StrictInequality |
            BinaryOperator::LessThan | BinaryOperator::LessEqualThan |
            BinaryOperator::GreaterThan | BinaryOperator::GreaterEqualThan => ZigType::Bool,
            // Logical operators
            BinaryOperator::LogicalAnd | BinaryOperator::LogicalOr => {
                if left == right { left } else { ZigType::Bool }
            }
            // Bitwise operators → I64
            BinaryOperator::BitwiseAnd | BinaryOperator::BitwiseOr |
            BinaryOperator::BitwiseXOr | BinaryOperator::ShiftLeft |
            BinaryOperator::ShiftRight | BinaryOperator::ShiftRightZeroFill => ZigType::I64,
            // Default
            _ => ZigType::I64,
        }
    }

    // ============================================================
    // Rule 5: Local variables → check ALL assignments
    // ============================================================

    /// Collect variable types from a variable declaration.
    /// Rule 5: Check ALL assignment expressions, at least one definite.
    pub fn collect_var_type(&mut self, vd: &VariableDeclaration) {
        for decl in &vd.declarations {
            if let Some(name) = Self::binding_name(&decl.id) {
                if let Some(init) = &decl.init {
                    let result = self.infer_expr_type(init);
                    match result {
                        InferResult::Definite(ty) => {
                            self.var_types.insert(name.to_string(), ty.clone());
                            // Track array element type
                            if let ZigType::ArrayList(elem_ty) = &ty {
                                self.array_element_types.insert(name.to_string(), (**elem_ty).clone());
                            }
                        }
                        InferResult::Indeterminate => {
                            // Rule 8: Report error
                            self.errors.push(format!(
                                "Cannot infer type of variable '{}'. Please add a type annotation.",
                                name
                            ));
                        }
                    }
                } else {
                    // No initializer → error
                    self.errors.push(format!(
                        "Variable '{}' must be initialized (Rule 8: indeterminate type).",
                        name
                    ));
                }
            }
        }
    }

    /// Check all assignments in function body
    pub fn check_assignments_in_fn(&mut self, body: &[Statement]) {
        for stmt in body {
            self.check_assignments_in_stmt(stmt);
        }
    }

    fn check_assignments_in_stmt(&mut self, stmt: &Statement) {
        match stmt {
            Statement::ExpressionStatement(es) => {
                if let Expression::AssignmentExpression(ae) = &es.expression {
                    if let Expression::Identifier(id) = &ae.left {
                        let result = self.infer_expr_type(&ae.right);
                        match result {
                            InferResult::Definite(ty) => {
                                self.var_types.insert(id.name.to_string(), ty);
                            }
                            InferResult::Indeterminate => {
                                self.errors.push(format!(
                                    "Cannot infer type of assignment to '{}'. Please add a type annotation.",
                                    id.name.as_str()
                                ));
                            }
                        }
                    }
                }
            }
            Statement::BlockStatement(bs) => {
                for s in &bs.body {
                    self.check_assignments_in_stmt(s);
                }
            }
            Statement::IfStatement(is) => {
                self.check_assignments_in_stmt(&is.consequent);
                if let Some(alt) = &is.alternate {
                    self.check_assignments_in_stmt(alt);
                }
            }
            _ => {}
        }
    }

    // ============================================================
    // Rule 6: Return types → check ALL return expressions
    // ============================================================

    /// Infer return type from function body.
    /// Rule 6: Check ALL return expressions, at least one definite.
    pub fn infer_return_type(&mut self, body: &[Statement]) -> InferResult {
        let mut return_types: Vec<ZigType> = Vec::new();
        self.collect_return_types(body, &mut return_types);

        if return_types.is_empty() {
            return InferResult::Definite(ZigType::Void);
        }

        // Check if all return types are definite
        let first = &return_types[0];
        if return_types.iter().all(|t| t == first) {
            return InferResult::Definite(first.clone());
        }

        // Heterogeneous return types → indeterminate
        self.errors.push(
            "Function has heterogeneous return types. Please add a return type annotation.".to_string()
        );
        InferResult::Indeterminate
    }

    fn collect_return_types(&mut self, stmts: &[Statement], out: &mut Vec<ZigType>) {
        for stmt in stmts {
            if let Statement::ReturnStatement(rs) = stmt {
                if let Some(arg) = &rs.argument {
                    let result = self.infer_expr_type(arg);
                    match result {
                        InferResult::Definite(ty) => out.push(ty),
                        InferResult::Indeterminate => {
                            self.errors.push(
                                "Cannot infer return type. Please add a return type annotation.".to_string()
                            );
                        }
                    }
                } else {
                    out.push(ZigType::Void);
                }
            } else if let Statement::BlockStatement(bs) = stmt {
                self.collect_return_types(&bs.body, out);
            }
        }
    }

    // ============================================================
    // Rule 7: Non-export function params → anytype
    // ============================================================

    /// Infer function parameter types.
    /// Rule 7: Non-export function params → indeterminate → anytype.
    pub fn infer_fn_params(&mut self, fd: &Function, is_export: bool) -> Vec<(String, InferResult)> {
        let mut params = Vec::new();
        for (i, param) in fd.params.items.iter().enumerate() {
            let name = Self::binding_name(&param.pattern);
            if let Some(pname) = name {
                if is_export {
                    // Export function: try to infer from JSDoc or default to I64
                    if let Some(ref data) = self.jsdoc_data {
                        if let Some(ty) = data.param_types.get(pname) {
                            params.push((pname.to_string(), InferResult::Definite(jsdoc::jsdoc_type_to_zig_type(ty, &data.typedefs))));
                            continue;
                        }
                    }
                    // Default to I64 for export functions
                    params.push((pname.to_string(), InferResult::Definite(ZigType::I64)));
                } else {
                    // Non-export function: indeterminate → anytype
                    params.push((pname.to_string(), InferResult::Indeterminate));
                }
            }
        }
        params
    }

    // ============================================================
    // Helpers
    // ============================================================

    /// Get binding name from pattern
    fn binding_name<'a>(pattern: &BindingPattern<'a>) -> Option<&'a str> {
        match pattern {
            BindingPattern::BindingIdentifier(id) => Some(id.name.as_str()),
            _ => None,
        }
    }

    /// Check if a variable has a JSDoc @type annotation
    pub fn has_jsdoc_type(&self, var_name: &str) -> bool {
        if let Some(ref data) = self.jsdoc_data {
            data.type_annotations.contains_key(var_name)
        } else {
            false
        }
    }

    /// Get JSDoc @type annotation for a variable
    pub fn get_jsdoc_type(&self, var_name: &str) -> Option<ZigType> {
        if let Some(ref data) = self.jsdoc_data {
            data.type_annotations.get(var_name).map(|ty| jsdoc::jsdoc_type_to_zig_type(ty, &data.typedefs))
        } else {
            None
        }
    }
}
