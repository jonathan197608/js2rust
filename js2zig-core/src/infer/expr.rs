// native_proto/infer/expr.rs
// Expression type inference.
// Rule 1: Literal expressions → definite type.
// Rule 2: Binary expressions → definite only if BOTH operands are literals.

use super::{InferResult, TypeInferrer};
use crate::native_builtins as builtins;
use crate::types::ZigType;
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
            Expression::NullLiteral(_) => InferResult::Definite(ZigType::JsAny),
            // RegExp literal (/pattern/) → NamedStruct("RegExp")
            Expression::RegExpLiteral(_) => {
                InferResult::Definite(ZigType::NamedStruct("RegExp".to_string()))
            }
            Expression::BigIntLiteral(_) => InferResult::Definite(ZigType::BigInt),

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
                // Built-in global constants
                if matches!(id.name.as_str(), "NaN" | "Infinity") {
                    return InferResult::Definite(ZigType::F64);
                }
                if id.name.as_str() == "undefined" {
                    return InferResult::Definite(ZigType::JsAny);
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
            // Special case: Addition (+) with a string operand → result is Str (string concatenation)
            Expression::BinaryExpression(be) => {
                let left = self.infer_expr_type(&be.left);
                let right = self.infer_expr_type(&be.right);

                // Pre-compute flags used in multiple match arms (before left/right are moved)
                let is_numeric_op = matches!(
                    be.operator,
                    BinaryOperator::Addition
                        | BinaryOperator::Subtraction
                        | BinaryOperator::Multiplication
                        | BinaryOperator::Division
                        | BinaryOperator::Remainder
                );
                let has_f64 = matches!(left, InferResult::Definite(ZigType::F64))
                    || matches!(right, InferResult::Definite(ZigType::F64));
                let is_compare_op = matches!(
                    be.operator,
                    BinaryOperator::Equality
                        | BinaryOperator::Inequality
                        | BinaryOperator::StrictEquality
                        | BinaryOperator::StrictInequality
                        | BinaryOperator::LessThan
                        | BinaryOperator::LessEqualThan
                        | BinaryOperator::GreaterThan
                        | BinaryOperator::GreaterEqualThan
                );
                let is_addition = be.operator == BinaryOperator::Addition;
                let is_string_concat = is_addition
                    && (self.expr_is_string(&be.left) || self.expr_is_string(&be.right));

                match (left, right) {
                    (InferResult::Definite(l), InferResult::Definite(r)) => {
                        InferResult::Definite(Self::infer_binary_type(be.operator, l, r))
                    }
                    // Comparison operators always return Bool
                    _ if is_compare_op => InferResult::Definite(ZigType::Bool),
                    // String concatenation
                    _ if is_string_concat => InferResult::Definite(ZigType::Str),
                    // Numeric promotion: if one operand is F64, result is F64
                    _ if is_numeric_op && has_f64 => InferResult::Definite(ZigType::F64),
                    _ => InferResult::Indeterminate,
                }
            }

            // LogicalExpression (&&, ||, ??): value-returning semantics.
            //
            // In JS, logical operators return one of their operands, not a bool:
            //   - `a && b`: returns a if falsy, else returns b
            //   - `a || b`: returns a if truthy, else returns b
            //   - `a ?? b`: returns a if not null/undefined, else returns b
            //
            // If both operands infer to the same type, the result is that type.
            // If types differ or are indeterminate, the result is JsAny.
            Expression::LogicalExpression(le) => {
                let left_ty = self.infer_expr_type(&le.left);
                let right_ty = self.infer_expr_type(&le.right);
                match (left_ty, right_ty) {
                    (InferResult::Definite(l), InferResult::Definite(r)) => {
                        if l == r {
                            InferResult::Definite(l)
                        } else {
                            InferResult::Definite(ZigType::JsAny)
                        }
                    }
                    _ => InferResult::Definite(ZigType::JsAny),
                }
            }

            Expression::UnaryExpression(ue) => match ue.operator {
                UnaryOperator::LogicalNot => InferResult::Definite(ZigType::Bool),
                UnaryOperator::UnaryNegation | UnaryOperator::UnaryPlus => {
                    match self.infer_expr_type(&ue.argument) {
                        InferResult::Definite(ty) => InferResult::Definite(ty),
                        InferResult::Indeterminate => InferResult::Indeterminate,
                    }
                }
                UnaryOperator::Void => InferResult::Definite(ZigType::JsAny),
                UnaryOperator::Delete => InferResult::Definite(ZigType::Bool),
                UnaryOperator::Typeof => InferResult::Definite(ZigType::Str),
                _ => InferResult::Indeterminate,
            },

            // Array: definite if all elements have same definite type
            Expression::ArrayExpression(ae) => self.infer_array_type(ae),

            // Object: definite as Struct
            Expression::ObjectExpression(oe) => self.infer_object_type(oe),

            // NewExpression: new Map(), new Set(), new Date()
            Expression::NewExpression(ne) => {
                if let Expression::Identifier(id) = &ne.callee {
                    let name = id.name.as_str();
                    // TypedArray / builtin NamedStruct constructors
                    if matches!(
                        name,
                        "Map"
                            | "Set"
                            | "Date"
                            | "DataView"
                            | "ArrayBuffer"
                            | "Uint8Array"
                            | "Uint8ClampedArray"
                            | "Uint16Array"
                            | "Uint32Array"
                            | "Int8Array"
                            | "Int16Array"
                            | "Int32Array"
                            | "Float32Array"
                            | "Float64Array"
                            | "BigInt64Array"
                            | "BigUint64Array"
                            | "RegExp"
                    ) {
                        InferResult::Definite(ZigType::NamedStruct(name.to_string()))
                    } else if name == "Error" {
                        InferResult::Definite(ZigType::JsError)
                    } else if name == "Number" {
                        InferResult::Definite(ZigType::F64)
                    } else if name == "Boolean" {
                        InferResult::Definite(ZigType::Bool)
                    } else if name == "String" {
                        InferResult::Definite(ZigType::Str)
                    } else if self.class_names.contains(name) {
                        InferResult::Definite(ZigType::NamedStruct(name.to_string()))
                    } else {
                        InferResult::Indeterminate
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
                            // AnytypeReturn cannot be propagated through function calls:
                            // - Nested functions are not visible at the return-type position
                            // - @TypeOf(call_expr) may reference undeclared names
                            if *ret_ty == ZigType::AnytypeReturn {
                                return InferResult::Indeterminate;
                            }
                            return InferResult::Definite(ret_ty.clone());
                        }
                        if let Some(ret_ty) = self.host_return_types.get(id.name.as_str()) {
                            return InferResult::Definite(ret_ty.clone());
                        }
                        // Global built-in functions (e.g., parseInt)
                        if let Some(builtin) = builtins::detect_builtin_call(ce) {
                            // Object(x) → passthrough (runtime returns @TypeOf(value)).
                            // In our simplified model, Object() doesn't create wrapper objects,
                            // so the return type matches the input type.
                            if builtin == builtins::BuiltinCall::ObjectConstructor
                                && ce.arguments.len() == 1
                                && let Some(arg) = ce.arguments.first()
                                && let Some(e) = arg.as_expression()
                                && let InferResult::Definite(arg_ty) = self.infer_expr_type(e)
                            {
                                return InferResult::Definite(arg_ty);
                            }
                            if let Some(ret_ty) = builtins::builtin_return_type(&builtin) {
                                return InferResult::Definite(ret_ty);
                            }
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

                // Symbol well-known symbols: Symbol.iterator, Symbol.asyncIterator, etc.
                if let Expression::Identifier(id) = &mem.object
                    && id.name.as_str() == "Symbol"
                {
                    match mem.property.name.as_str() {
                        "iterator" | "asyncIterator" | "hasInstance" | "isConcatSpreadable"
                        | "species" | "toPrimitive" | "toStringTag" | "unscopables" | "match"
                        | "matchAll" | "replace" | "search" | "split" | "dispose" => {
                            return InferResult::Definite(ZigType::JsSymbol);
                        }
                        _ => {}
                    }
                }

                // Number static properties: Number.MAX_VALUE, Number.EPSILON, etc.
                if let Expression::Identifier(id) = &mem.object
                    && id.name.as_str() == "Number"
                {
                    match mem.property.name.as_str() {
                        "MAX_VALUE" | "MIN_VALUE" | "MAX_SAFE_INTEGER" | "MIN_SAFE_INTEGER"
                        | "EPSILON" | "NaN" | "POSITIVE_INFINITY" | "NEGATIVE_INFINITY" => {
                            return InferResult::Definite(ZigType::F64);
                        }
                        _ => {}
                    }
                }

                // Math static properties: Math.PI, Math.E, Math.LN2, etc.
                if let Expression::Identifier(id) = &mem.object
                    && id.name.as_str() == "Math"
                {
                    match mem.property.name.as_str() {
                        "PI" | "E" | "LN10" | "LN2" | "LOG10E" | "LOG2E" | "SQRT1_2" | "SQRT2" => {
                            return InferResult::Definite(ZigType::F64);
                        }
                        _ => {}
                    }
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
                    // JsSymbol property access
                    InferResult::Definite(ZigType::JsSymbol) => {
                        match mem.property.name.as_str() {
                            // description is ?[]const u8 — return Str (callers handle optionality)
                            "description" => InferResult::Definite(ZigType::Str),
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

            // ComputedMemberExpression: obj[key] → infer from obj type and key
            Expression::ComputedMemberExpression(mem) => {
                let obj_ty = self.infer_expr_type(&mem.object);
                match obj_ty {
                    InferResult::Definite(ZigType::JsAny) => {
                        // obj[key] on JsAny → getByKey/get returns JsAny
                        InferResult::Definite(ZigType::JsAny)
                    }
                    InferResult::Definite(ZigType::NamedStruct(ref name)) if name == "Map" => {
                        // Map.get(key) returns ?JsAny → JsAny (orelse .undefined_value)
                        InferResult::Definite(ZigType::JsAny)
                    }
                    InferResult::Definite(ZigType::Str) => {
                        // str[idx] → u8 character (promoted to i64 in Zig context)
                        // Both literal and variable index return I64
                        InferResult::Definite(ZigType::I64)
                    }
                    InferResult::Definite(ZigType::ArrayList(ref elem_ty)) => {
                        // arr[idx] → element type (both literal and variable index)
                        InferResult::Definite(*elem_ty.clone())
                    }
                    InferResult::Definite(ZigType::Struct(ref fields)) => {
                        // obj["key"] on anonymous struct → field type
                        if let Expression::StringLiteral(s) = &mem.expression {
                            let key = s.value.as_str();
                            for (name, ty) in fields {
                                if name == key {
                                    return InferResult::Definite(ty.clone());
                                }
                            }
                        }
                        InferResult::Indeterminate
                    }
                    InferResult::Definite(ZigType::NamedStruct(ref name)) => {
                        // obj["key"] on named struct → treat like struct for field lookup
                        // Try host struct fields first
                        if let Expression::StringLiteral(s) = &mem.expression {
                            let key = s.value.as_str();
                            if let Some(host_fields) = self.host_struct_fields.get(name.as_str())
                                && let Some(field_ty) = host_fields.get(key)
                            {
                                return InferResult::Definite(field_ty.clone());
                            }
                        }
                        InferResult::Indeterminate
                    }
                    _ => InferResult::Indeterminate,
                }
            }

            // AwaitExpression: strip the await, infer inner expression type
            Expression::AwaitExpression(ae) => self.infer_expr_type(&ae.argument),

            // ConditionalExpression (ternary: a ? b : c):
            // return type = common type of both branches.
            // If both branches have the same definite type, return that.
            // If one is I64 and the other F64, return F64 (JS numeric coercion).
            // Otherwise Indeterminate.
            Expression::ConditionalExpression(ce) => {
                let cons_ty = self.infer_expr_type(&ce.consequent);
                let alt_ty = self.infer_expr_type(&ce.alternate);
                match (cons_ty, alt_ty) {
                    (InferResult::Definite(t1), InferResult::Definite(t2)) => {
                        if t1 == t2 {
                            InferResult::Definite(t1)
                        } else {
                            // Numeric coercion: I64 + F64 → F64
                            match (t1, t2) {
                                (ZigType::I64, ZigType::F64) => InferResult::Definite(ZigType::F64),
                                (ZigType::F64, ZigType::I64) => InferResult::Definite(ZigType::F64),
                                _ => InferResult::Indeterminate,
                            }
                        }
                    }
                    _ => InferResult::Indeterminate,
                }
            }

            // ChainExpression (?. ): result is nullable → Indeterminate.
            // The Zig compiler will infer optional type from 'if (obj) |v| v.prop else null'.
            Expression::ChainExpression(_chain) => InferResult::Indeterminate,

            // AssignmentExpression: result type = RHS type for simple, F64 for **=,
            // LHS type for &&=/||=/??= (conditional assignment returns LHS type).
            Expression::AssignmentExpression(ae) => match ae.operator {
                AssignmentOperator::Exponential => InferResult::Definite(ZigType::F64),
                _ => self.infer_expr_type(&ae.right),
            },

            // ParenthesizedExpression: unwrap and recurse
            Expression::ParenthesizedExpression(pe) => self.infer_expr_type(&pe.expression),

            // Everything else → indeterminate
            _ => InferResult::Indeterminate,
        }
    }

    /// Check if an expression definitely evaluates to a string type.
    /// Used for string concatenation type inference.
    fn expr_is_string(&self, expr: &Expression) -> bool {
        match expr {
            Expression::StringLiteral(_) => true,
            Expression::TemplateLiteral(_) => true,
            Expression::Identifier(id) => {
                self.var_types.get(id.name.as_str()) == Some(&ZigType::Str)
            }
            // Handle nested binary expressions: if it's string concatenation, result is string
            Expression::BinaryExpression(be) if be.operator == BinaryOperator::Addition => {
                self.expr_is_string(&be.left) || self.expr_is_string(&be.right)
            }
            // ConditionalExpression (ternary): result is string if both branches are strings
            Expression::ConditionalExpression(ce) => {
                self.expr_is_string(&ce.consequent) && self.expr_is_string(&ce.alternate)
            }
            // ParenthesizedExpression: unwrap and recurse
            Expression::ParenthesizedExpression(pe) => self.expr_is_string(&pe.expression),
            _ => false,
        }
    }

    pub(crate) fn infer_array_type(&mut self, ae: &ArrayExpression) -> InferResult {
        if ae.elements.is_empty() {
            // Empty array: default to ArrayList(JsAny) — JS allows any type in [].
            return InferResult::Definite(ZigType::ArrayList(Box::new(ZigType::JsAny)));
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
    fn extract_return_expr<'a>(body: &'a FunctionBody<'a>) -> Option<&'a Expression<'a>> {
        if body.statements.len() == 1
            && let Statement::ReturnStatement(ret) = &body.statements[0]
        {
            return ret.argument.as_ref();
        }
        None
    }

    pub(crate) fn infer_binary_type(op: BinaryOperator, left: ZigType, right: ZigType) -> ZigType {
        match op {
            BinaryOperator::Addition => {
                // BigInt + BigInt → BigInt
                if left == ZigType::BigInt && right == ZigType::BigInt {
                    return ZigType::BigInt;
                }
                // String + BigInt → String (JS spec: implicit toString)
                // BigInt + String → String
                if (left == ZigType::Str && right == ZigType::BigInt)
                    || (left == ZigType::BigInt && right == ZigType::Str)
                {
                    return ZigType::Str;
                }
                if left == ZigType::F64 || right == ZigType::F64 {
                    ZigType::F64
                } else {
                    ZigType::I64
                }
            }
            BinaryOperator::Subtraction
            | BinaryOperator::Multiplication
            | BinaryOperator::Division => {
                // BigInt arithmetic preserves BigInt type
                if left == ZigType::BigInt && right == ZigType::BigInt {
                    return ZigType::BigInt;
                }
                if left == ZigType::F64 || right == ZigType::F64 {
                    ZigType::F64
                } else {
                    ZigType::I64
                }
            }
            // Remainder: JS % always uses f64 semantics (to preserve -0).
            // The Emitter generates js_runtime.jsRem() for integer operands,
            // which returns f64, so the inferred type must be F64.
            BinaryOperator::Remainder => {
                if left == ZigType::BigInt && right == ZigType::BigInt {
                    return ZigType::BigInt;
                }
                ZigType::F64
            }
            // Exponential: JS `**` always returns number (f64).
            // The Emitter generates std.math.pow(f64, ...) for all non-BigInt cases.
            BinaryOperator::Exponential => {
                if left == ZigType::BigInt && right == ZigType::BigInt {
                    return ZigType::BigInt;
                }
                ZigType::F64
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
            _ => ZigType::JsAny,
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
            | "flatMap" | "toReversed" | "toSorted" | "toSpliced" => {
                InferResult::Definite(ZigType::ArrayList(Box::new(elem_ty.clone())))
            }
            // Array.prototype.with(index, value) returns same element type array
            "with" => InferResult::Definite(ZigType::ArrayList(Box::new(elem_ty.clone()))),
            // Methods that return a boolean
            "some" | "every" | "includes" => InferResult::Definite(ZigType::Bool),
            // Methods returning index or length
            "indexOf" | "lastIndexOf" | "findIndex" => InferResult::Definite(ZigType::I64),
            // reduce: returns accumulator type (default JsAny since accumulator can be any type)
            "reduce" | "reduceRight" => InferResult::Definite(ZigType::JsAny),
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
                        "set" => InferResult::Definite(ZigType::NamedStruct("Map".into())),
                        "get" => InferResult::Definite(ZigType::JsAny), // Map.get() returns JsAny
                        "has" | "delete" => InferResult::Definite(ZigType::Bool),
                        _ => InferResult::Indeterminate,
                    },
                    "Set" => match method {
                        "add" => InferResult::Definite(ZigType::NamedStruct("Set".into())),
                        "has" | "delete" => InferResult::Definite(ZigType::Bool),
                        "keys" | "values" => {
                            InferResult::Definite(ZigType::ArrayList(Box::new(ZigType::JsAny)))
                        }
                        "entries" => InferResult::Definite(ZigType::ArrayList(Box::new(
                            ZigType::ArrayList(Box::new(ZigType::JsAny)),
                        ))),
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
                "indexOf" | "lastIndexOf" => InferResult::Definite(ZigType::I64),
                "includes" | "startsWith" | "endsWith" => InferResult::Definite(ZigType::Bool),
                "trim" | "trimStart" | "trimEnd" | "split" | "padStart" | "padEnd" | "charAt"
                | "at" | "toUpperCase" | "toLowerCase" | "slice" | "substring" | "replace"
                | "replaceAll" | "concat" | "repeat" => InferResult::Definite(ZigType::Str),
                _ => InferResult::Indeterminate,
            },
            // JsSymbol methods
            ZigType::JsSymbol => match method {
                // sym.toString() → "Symbol(description)" or "Symbol()"
                "toString" => InferResult::Definite(ZigType::Str),
                _ => InferResult::Indeterminate,
            },
            // BigInt methods
            ZigType::BigInt => match method {
                "toString" | "toLocaleString" => InferResult::Definite(ZigType::Str),
                "valueOf" => InferResult::Definite(ZigType::BigInt),
                _ => InferResult::Indeterminate,
            },
            _ => InferResult::Indeterminate,
        }
    }
}
