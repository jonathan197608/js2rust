// native_proto/infer/expr.rs
// Expression type inference.
// Rule 1: Literal expressions → definite type.
// Rule 2: Binary expressions → definite only if BOTH operands are literals.

use super::{InferResult, TypeInferrer};
use crate::native_proto::ZigType;
use crate::native_proto::builtins;
use oxc_ast::ast::*;

impl TypeInferrer {
    // ============================================================
    // Rule 1: Literal expressions → definite type
    // ============================================================

    /// Infer the type of an expression.
    pub(crate) fn infer_expr_type(&mut self, expr: &Expression) -> InferResult {
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

            // NewExpression: new Map(), new Set(), new Date()
            Expression::NewExpression(ne) => {
                if let Expression::Identifier(id) = &ne.callee {
                    match id.name.as_str() {
                        "Map" => InferResult::Definite(ZigType::NamedStruct("Map".to_string())),
                        "Set" => InferResult::Definite(ZigType::NamedStruct("Set".to_string())),
                        "Date" => InferResult::Definite(ZigType::NamedStruct("Date".to_string())),
                        name if self.class_names.contains(name) => {
                            InferResult::Definite(ZigType::NamedStruct(name.to_string()))
                        }
                        _ => InferResult::Indeterminate,
                    }
                } else {
                    InferResult::Indeterminate
                }
            }

            // CallExpression: look up from fn_return_types cache, then host_return_types
            Expression::CallExpression(ce) => {
                match &ce.callee {
                    Expression::Identifier(id) => {
                        if let Some(ret_ty) = self.fn_return_types.get(id.name.as_str()) {
                            return InferResult::Definite(ret_ty.clone());
                        }
                        if let Some(ret_ty) = self.host_return_types.get(id.name.as_str()) {
                            return InferResult::Definite(ret_ty.clone());
                        }
                        // Global built-in functions (e.g., parseInt)
                        if let Some(builtin) = builtins::detect_builtin_call(ce)
                            && let Some(ret_ty) = builtins::builtin_return_type(&builtin)
                        {
                            return InferResult::Definite(ret_ty);
                        }
                    }
                    // Method calls: arr.slice(), arr.map(), arr.filter(), etc.
                    Expression::StaticMemberExpression(mem) => {
                        if let Some(obj_name) =
                            super::helpers::extract_expr_identifier_name(&mem.object)
                        {
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
                        // Built-in method calls (String, Math, Date, etc.)
                        if let Some(builtin) = builtins::detect_builtin_call(ce)
                            && let Some(ret_ty) = builtins::builtin_return_type(&builtin)
                        {
                            return InferResult::Definite(ret_ty);
                        }
                    }
                    _ => {}
                }
                InferResult::Indeterminate
            }

            // Static member access
            Expression::StaticMemberExpression(mem) => {
                // Special case: this.field inside a class method → look up field type
                if matches!(&mem.object, Expression::ThisExpression(_))
                    && let Some(class_name) = &self.current_class
                {
                    if let Some(field_types) = self.class_field_types.get(class_name.as_str()) {
                        let field_name = mem.property.name.as_str();
                        if let Some(field_ty) = field_types.get(field_name) {
                            return InferResult::Definite(field_ty.clone());
                        }
                    }
                    return InferResult::Indeterminate;
                }

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
                    // Host struct field access (e.g. fetch_user().name)
                    InferResult::Definite(ZigType::NamedStruct(ref struct_name))
                        if self.host_struct_fields.contains_key(struct_name.as_str()) =>
                    {
                        if let Some(fields) = self.host_struct_fields.get(struct_name.as_str()) {
                            let field_name = mem.property.name.as_str();
                            if let Some(field_ty) = fields.get(field_name) {
                                return InferResult::Definite(field_ty.clone());
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
                    // JsAny property access: dynamic, returns JsAny
                    InferResult::Definite(ZigType::JsAny) => {
                        // Property access on JsAny returns JsAny
                        // (the actual type is only known at runtime)
                        InferResult::Definite(ZigType::JsAny)
                    }
                    _ => InferResult::Indeterminate,
                }
            }

            // AwaitExpression: strip the await, infer inner expression type
            Expression::AwaitExpression(ae) => self.infer_expr_type(&ae.argument),

            // ChainExpression (?. ): result is nullable → Indeterminate.
            // The Zig compiler will infer optional type from 'if (obj) |v| v.prop else null'.
            Expression::ChainExpression(_chain) => InferResult::Indeterminate,

            // Everything else → indeterminate
            _ => InferResult::Indeterminate,
        }
    }

    pub(crate) fn infer_array_type(&mut self, ae: &ArrayExpression) -> InferResult {
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

    pub(crate) fn infer_object_type(&mut self, oe: &ObjectExpression) -> InferResult {
        let mut fields: Vec<(String, ZigType)> = Vec::new();
        for prop in &oe.properties {
            match prop {
                ObjectPropertyKind::SpreadProperty(s) => {
                    // Merge the spread source's struct fields into the result.
                    // Later spreads and inline props override earlier ones on key conflict.
                    match self.infer_expr_type(&s.argument) {
                        InferResult::Definite(ZigType::Struct(spread_fields)) => {
                            for (name, ty) in spread_fields {
                                fields.retain(|(n, _)| n != &name);
                                fields.push((name, ty));
                            }
                        }
                        _ => return InferResult::Indeterminate,
                    }
                }
                ObjectPropertyKind::ObjectProperty(p) => {
                    let field_name = match &p.key {
                        PropertyKey::StaticIdentifier(id) => id.name.to_string(),
                        PropertyKey::StringLiteral(s) => s.value.to_string(),
                        _ => return InferResult::Indeterminate,
                    };
                    match p.kind {
                        PropertyKind::Init => match self.infer_expr_type(&p.value) {
                            InferResult::Definite(ft) => {
                                // Inline property overrides any spread field with same name
                                fields.retain(|(n, _)| n != &field_name);
                                fields.push((field_name, ft));
                            }
                            InferResult::Indeterminate => return InferResult::Indeterminate,
                        },
                        PropertyKind::Get => {
                            // Getter: infer from return expression in function body
                            if let Expression::FunctionExpression(func) = &p.value
                                && let Some(body) = &func.body
                                && let Some(return_expr) = Self::extract_return_expr(body)
                            {
                                match self.infer_expr_type(return_expr) {
                                    InferResult::Definite(ft) => {
                                        fields.retain(|(n, _)| n != &field_name);
                                        fields.push((field_name, ft));
                                    }
                                    InferResult::Indeterminate => {
                                        return InferResult::Indeterminate;
                                    }
                                }
                            }
                        }
                        PropertyKind::Set => {
                            // Setter: skip, doesn't contribute a field
                        }
                    }
                }
            }
        }
        InferResult::Definite(ZigType::Struct(fields))
    }

    /// Extract the return expression from a function body with a single return statement.
    fn extract_return_expr<'a>(
        body: &'a oxc_ast::ast::FunctionBody<'a>,
    ) -> Option<&'a Expression<'a>> {
        if body.statements.len() == 1
            && let Statement::ReturnStatement(ret) = &body.statements[0]
        {
            return ret.argument.as_ref();
        }
        None
    }

    pub(crate) fn infer_binary_type(op: BinaryOperator, left: ZigType, right: ZigType) -> ZigType {
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
    // Array / named method return type inference
    // ============================================================

    /// Infer the return type of array method calls like arr.slice(), arr.map(), etc.
    pub(crate) fn infer_array_method_return(&self, method: &str, elem_ty: &ZigType) -> InferResult {
        match method {
            // Methods that return a new array (same element type)
            "slice" | "map" | "filter" | "concat" | "reverse" | "sort" | "splice" | "flat"
            | "flatMap" => InferResult::Definite(ZigType::ArrayList(Box::new(elem_ty.clone()))),
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

    /// Infer the return type of method calls on Map/Set/Date/NamedStruct objects.
    pub(crate) fn infer_named_method_return(&self, var_ty: &ZigType, method: &str) -> InferResult {
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
                    "Date" => match method {
                        "getTime" | "getFullYear" | "getMonth" | "getDate" | "getDay"
                        | "getHours" | "getMinutes" | "getSeconds" | "valueOf" => {
                            InferResult::Definite(ZigType::I64)
                        }
                        _ => InferResult::Indeterminate,
                    },
                    // User-defined class: look up "ClassName.methodName" in fn_return_types
                    _ => {
                        let key = format!("{}.{}", name, method);
                        if let Some(ret_ty) = self.fn_return_types.get(&key) {
                            InferResult::Definite(ret_ty.clone())
                        } else {
                            InferResult::Indeterminate
                        }
                    }
                }
            }
            // String methods called on a str-typed variable
            ZigType::Str => match method {
                "indexOf" => InferResult::Definite(ZigType::I64),
                "includes" | "startsWith" | "endsWith" => InferResult::Definite(ZigType::Bool),
                "trim" | "split" | "padStart" | "padEnd" => InferResult::Definite(ZigType::Str),
                _ => InferResult::Indeterminate,
            },
            _ => InferResult::Indeterminate,
        }
    }
}
