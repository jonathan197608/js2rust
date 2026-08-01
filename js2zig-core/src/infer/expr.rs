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
                // Value-based detection via shared function (P2-2: deduplicated
                // to ensure consistency with lower/expr/mod.rs and member.rs).
                InferResult::Definite(crate::types::numeric_literal_type(n.value))
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
                // `arguments` is a synthetic ArrayList(JsAny) capturing all
                // call arguments. Matches the lowerer (member.rs:310-316).
                if id.name.as_str() == "arguments" {
                    return InferResult::Definite(ZigType::ArrayList(Box::new(ZigType::JsAny)));
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
                        | BinaryOperator::Exponential
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
                        | BinaryOperator::In
                        | BinaryOperator::Instanceof
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
                    // Numeric promotion: if one operand is F64, result is F64.
                    // Exception: Addition (+) with an indeterminate operand could
                    // be string concatenation (e.g., 3.14 + x where x is anytype),
                    // so we cannot be certain the result is F64. Non-addition
                    // numeric ops always return number regardless of operand types.
                    _ if is_numeric_op && has_f64 && !is_addition => {
                        InferResult::Definite(ZigType::F64)
                    }
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
            // For ??, if left is a definite non-nullish type (i64, f64, bool,
            // Str, BigInt), the result is always left's type — matching the
            // lowerer's short-circuit (mod.rs: nullish check on raw_left_type).
            // For && and ||, if both operands infer to the same type, the
            // result is that type; if types differ, the result is JsAny.
            Expression::LogicalExpression(le) => {
                let left_ty = self.infer_expr_type(&le.left);
                // ?? on non-JsAny types is a no-op: the left value can never
                // be null/undefined in our type system (i64, f64, bool, Str,
                // BigInt). Matches lowerer's ?? short-circuit (mod.rs).
                if le.operator == LogicalOperator::Coalesce {
                    match &left_ty {
                        InferResult::Definite(ZigType::JsAny) => {
                            // left could be nullish → result could be right
                            let right_ty = self.infer_expr_type(&le.right);
                            match right_ty {
                                InferResult::Definite(_) => InferResult::Definite(ZigType::JsAny),
                                InferResult::Indeterminate => InferResult::Definite(ZigType::JsAny),
                            }
                        }
                        InferResult::Definite(_) => {
                            // left is non-nullish → result is left's type
                            left_ty
                        }
                        InferResult::Indeterminate => {
                            // unknown left type → fallback to right or JsAny
                            self.infer_expr_type(&le.right)
                        }
                    }
                } else {
                    // && or ||
                    let right_ty = self.infer_expr_type(&le.right);
                    match (left_ty, right_ty) {
                        (InferResult::Definite(l), InferResult::Definite(r)) => {
                            if l == r {
                                InferResult::Definite(l)
                            } else {
                                InferResult::Definite(ZigType::JsAny)
                            }
                        }
                        // Both operands Indeterminate → Indeterminate so the
                        // var_types fallback in decl.rs can pick up JSDoc @type
                        // annotations (Bug A pattern). Matches lowerer's
                        // (None, None) => None mapping (member.rs:649). Mixed
                        // Definite/Indeterminate returns Definite(JsAny), matching
                        // the lowerer's (Some(_), None) | (None, Some(_)) => JsAny.
                        // (R24-INF-4)
                        (InferResult::Indeterminate, InferResult::Indeterminate) => {
                            InferResult::Indeterminate
                        }
                        _ => InferResult::Definite(ZigType::JsAny),
                    }
                }
            }

            Expression::UnaryExpression(ue) => {
                #[allow(unreachable_patterns)] // defensive: oxc may add new variants
                match ue.operator {
                    UnaryOperator::LogicalNot => InferResult::Definite(ZigType::Bool),
                    UnaryOperator::UnaryNegation | UnaryOperator::UnaryPlus => {
                        // Unary `+` performs ToNumber: bool/str/etc → number.
                        // Unary `-` also performs ToNumber then negates.
                        match self.infer_expr_type(&ue.argument) {
                            InferResult::Definite(ty) => match ty {
                                // Numbers pass through unchanged.
                                ZigType::I64 | ZigType::F64 => InferResult::Definite(ty),
                                // Bool/Str/JsAny → number (I64 for bool, F64 for
                                // str/others, matching the lowerer's emission).
                                ZigType::Bool => InferResult::Definite(ZigType::I64),
                                ZigType::Str | ZigType::JsAny => {
                                    InferResult::Definite(ZigType::F64)
                                }
                                // BigInt stays BigInt (unary -); unary + on BigInt
                                // is a JS TypeError but we let it pass through.
                                _ => InferResult::Definite(ty),
                            },
                            InferResult::Indeterminate => InferResult::Indeterminate,
                        }
                    }
                    UnaryOperator::Void => InferResult::Definite(ZigType::JsAny),
                    UnaryOperator::Delete => InferResult::Definite(ZigType::Bool),
                    UnaryOperator::Typeof => InferResult::Definite(ZigType::Str),
                    // Bitwise NOT returns Int32 (I64) for number operands,
                    // but BigInt stays BigInt (~n).
                    UnaryOperator::BitwiseNot => match self.infer_expr_type(&ue.argument) {
                        InferResult::Definite(ZigType::BigInt) => {
                            InferResult::Definite(ZigType::BigInt)
                        }
                        _ => InferResult::Definite(ZigType::I64),
                    },
                    _ => InferResult::Indeterminate,
                }
            }

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
                    } else if matches!(
                        name,
                        "Error" | "TypeError" | "RangeError" | "SyntaxError" | "ReferenceError"
                    ) {
                        InferResult::Definite(ZigType::JsError)
                    } else if name == "BigInt" {
                        InferResult::Definite(ZigType::BigInt)
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
                                let result = self
                                    .infer_array_method_return(mem.property.name.as_str(), elem_ty);
                                // Only return on Definite; Indeterminate falls
                                // through to detect_builtin_call for a second
                                // chance at type inference (same pattern as
                                // var_types below).
                                if let InferResult::Definite(ty) = result {
                                    return InferResult::Definite(ty);
                                }
                            }
                            // Map/Set/Date/Str/BigInt/ArrayList methods
                            if let Some(var_ty) = self.var_types.get(&obj_name) {
                                let result = self
                                    .infer_named_method_return(var_ty, mem.property.name.as_str());
                                // Only return on Definite; Indeterminate falls
                                // through to detect_builtin_call for a second
                                // chance at type inference.
                                if let InferResult::Definite(ty) = result {
                                    return InferResult::Definite(ty);
                                }
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
                    // Host struct / class field access
                    // INF-4: Also check class_field_types (lowerer checks both).
                    InferResult::Definite(ZigType::NamedStruct(ref struct_name))
                        if self.host_struct_fields.contains_key(struct_name.as_str())
                            || self.class_field_types.contains_key(struct_name.as_str()) =>
                    {
                        let field_name = mem.property.name.as_str();
                        if let Some(fields) = self.host_struct_fields.get(struct_name.as_str())
                            && let Some(field_ty) = fields.get(field_name)
                        {
                            return InferResult::Definite(field_ty.clone());
                        }
                        if let Some(fields) = self.class_field_types.get(struct_name.as_str())
                            && let Some(field_ty) = fields.get(field_name)
                        {
                            return InferResult::Definite(field_ty.clone());
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
                    // TypedArray properties — matches lower_static_member's
                    // FieldKind::TypedArrayProp routing. Runtime returns
                    // i64 for byteLength/byteOffset, []const u8 for buffer
                    // (js_typedarray.zig).
                    InferResult::Definite(ZigType::NamedStruct(ref name))
                        if matches!(
                            name.as_str(),
                            "Int8Array"
                                | "Uint8Array"
                                | "Uint8ClampedArray"
                                | "Int16Array"
                                | "Uint16Array"
                                | "Int32Array"
                                | "Uint32Array"
                                | "Float32Array"
                                | "Float64Array"
                                | "BigInt64Array"
                                | "BigUint64Array"
                        ) =>
                    {
                        match mem.property.name.as_str() {
                            "byteLength" | "byteOffset" => InferResult::Definite(ZigType::I64),
                            "buffer" => InferResult::Definite(ZigType::Str),
                            _ => InferResult::Indeterminate,
                        }
                    }
                    // INF-3: RegExp .source/.flags → Str
                    InferResult::Definite(ZigType::NamedStruct(ref name)) if name == "RegExp" => {
                        match mem.property.name.as_str() {
                            "source" | "flags" => InferResult::Definite(ZigType::Str),
                            _ => InferResult::Indeterminate,
                        }
                    }
                    // INF-2: JsError .name/.message/.stack → Str
                    InferResult::Definite(ZigType::JsError) => match mem.property.name.as_str() {
                        "name" | "message" | "stack" => InferResult::Definite(ZigType::Str),
                        _ => InferResult::Indeterminate,
                    },
                    // JsSymbol property access
                    InferResult::Definite(ZigType::JsSymbol) => {
                        match mem.property.name.as_str() {
                            // description is ?[]const u8 — return Str (callers handle optionality)
                            "description" => InferResult::Definite(ZigType::Str),
                            _ => InferResult::Indeterminate,
                        }
                    }
                    // ArrayList property access: .length
                    InferResult::Definite(ZigType::ArrayList(_)) => {
                        match mem.property.name.as_str() {
                            "length" => InferResult::Definite(ZigType::I64),
                            _ => InferResult::Indeterminate,
                        }
                    }
                    // JsAny property access is dynamic: the actual type is
                    // only known at runtime. Return Indeterminate (rather
                    // than Definite(JsAny)) so the var_types fallback in
                    // decl.rs (infer_expr_type().or_else(var_types)) can pick
                    // up JSDoc @type annotations. Returning Definite(JsAny)
                    // would block the JSDoc fallback (Bug A pattern). The
                    // lowerer's StaticMemberExpression arm already returns
                    // None for non-NamedStruct objects, so this brings the
                    // analysis pass in line with the lowerer. (R24-INF-7)
                    InferResult::Definite(ZigType::JsAny) => InferResult::Indeterminate,
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
                        // str[idx] → single-character substring (JS charAt)
                        // Returns Str ([]const u8) for both literal and variable index.
                        InferResult::Definite(ZigType::Str)
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
                        // INF-1: Check both host_struct_fields and class_field_types
                        if let Expression::StringLiteral(s) = &mem.expression {
                            let key = s.value.as_str();
                            if let Some(host_fields) = self.host_struct_fields.get(name.as_str())
                                && let Some(field_ty) = host_fields.get(key)
                            {
                                return InferResult::Definite(field_ty.clone());
                            }
                            if let Some(class_fields) = self.class_field_types.get(name.as_str())
                                && let Some(field_ty) = class_fields.get(key)
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
            // LHS type for &&=/||= (conditional assignment returns LHS type).
            Expression::AssignmentExpression(ae) => match ae.operator {
                AssignmentOperator::Exponential => InferResult::Definite(ZigType::F64),
                AssignmentOperator::Addition => {
                    // INF-5: str += x → Str (string concatenation).
                    // When the LHS is a string variable or the RHS is a
                    // string expression, the result is always Str per
                    // ECMA-262 ToString coercion.
                    let lhs_is_str = matches!(
                        &ae.left,
                        oxc_ast::ast::AssignmentTarget::AssignmentTargetIdentifier(id)
                            if self.var_types.get(id.name.as_str()) == Some(&ZigType::Str)
                    );
                    if lhs_is_str || self.expr_is_string(&ae.right) {
                        InferResult::Definite(ZigType::Str)
                    } else {
                        self.infer_expr_type(&ae.right)
                    }
                }
                AssignmentOperator::LogicalAnd
                | AssignmentOperator::LogicalOr
                | AssignmentOperator::LogicalNullish => {
                    // For logical assignments (x &&= y, x ||= y),
                    // result is either the original LHS (short-circuit) or the RHS.
                    // Return the LHS variable's type as the most likely result.
                    if let oxc_ast::ast::AssignmentTarget::AssignmentTargetIdentifier(id) = &ae.left
                        && let Some(t) = self.var_types.get(id.name.as_str())
                    {
                        return InferResult::Definite(t.clone());
                    }
                    // For member-access targets (obj.x &&= y), fall through
                    // to RHS type since we can't easily look up the member type.
                    self.infer_expr_type(&ae.right)
                }
                _ => self.infer_expr_type(&ae.right),
            },

            // ParenthesizedExpression: unwrap and recurse
            Expression::ParenthesizedExpression(pe) => self.infer_expr_type(&pe.expression),

            // Private field access: this.#field inside a class method → look up field type
            Expression::PrivateFieldExpression(pfe) => {
                if matches!(&pfe.object, Expression::ThisExpression(_))
                    && let Some(class_name) = &self.current_class
                {
                    if let Some(field_types) = self.class_field_types.get(class_name.as_str()) {
                        // PrivateIdentifier.name does NOT include the '#' prefix
                        let field_name = pfe.field.name.to_string();
                        if let Some(field_ty) = field_types.get(&field_name) {
                            return InferResult::Definite(field_ty.clone());
                        }
                    }
                    return InferResult::Indeterminate;
                }
                InferResult::Indeterminate
            }

            Expression::UpdateExpression(ue) => {
                // ++x / x-- always returns a number.
                // Return the variable's type only if it's numeric (I64/F64);
                // otherwise default to F64 (JS numbers are always float64).
                if let SimpleAssignmentTarget::AssignmentTargetIdentifier(id) = &ue.argument {
                    if let Some(t) = self.var_types.get(id.name.as_str()) {
                        match t {
                            ZigType::I64 => InferResult::Definite(ZigType::I64),
                            ZigType::F64 => InferResult::Definite(ZigType::F64),
                            // Non-numeric variable: ++ still produces a number (f64)
                            _ => InferResult::Definite(ZigType::F64),
                        }
                    } else {
                        // Unknown variable: default to f64 (JS number)
                        InferResult::Definite(ZigType::F64)
                    }
                } else {
                    InferResult::Definite(ZigType::F64)
                }
            }

            Expression::SequenceExpression(se) => {
                if let Some(last) = se.expressions.last() {
                    self.infer_expr_type(last)
                } else {
                    InferResult::Indeterminate
                }
            }

            // this expression inside a class method → NamedStruct(className)
            Expression::ThisExpression(_) => {
                if let Some(class_name) = &self.current_class {
                    InferResult::Definite(ZigType::NamedStruct(class_name.clone()))
                } else {
                    InferResult::Indeterminate
                }
            }

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
            // LogicalExpression (||, &&, ??): result is string if either branch
            // is string — but ?? only returns right when left is null/undefined.
            // So for ??, if left is a definite non-nullish type that is NOT
            // string, the result is NOT string (matching lowerer's short-circuit).
            Expression::LogicalExpression(le) => {
                if le.operator == LogicalOperator::Coalesce {
                    // ?? returns left if not nullish; only string if left is
                    // string or left is nullish and right is string.
                    let left_is_string = self.expr_is_string(&le.left);
                    if left_is_string {
                        return true;
                    }
                    // In our type system, only JsAny can be null/undefined.
                    // Check if left is a JsAny identifier — if so, result could
                    // be right; otherwise left is non-nullish and not string.
                    if let Expression::Identifier(id) = &le.left
                        && self.var_types.get(id.name.as_str()) == Some(&ZigType::JsAny)
                    {
                        return self.expr_is_string(&le.right);
                    }
                    // For non-identifier left (or non-JsAny identifier),
                    // we can't determine nullish-ness without infer_expr_type.
                    // Fall back to checking right (conservative — may over-report
                    // string, but never under-reports).
                    self.expr_is_string(&le.right)
                } else {
                    // || or &&: result is string if either branch is string
                    self.expr_is_string(&le.left) || self.expr_is_string(&le.right)
                }
            }
            // INF-4: AwaitExpression — unwrap and check inner expression
            Expression::AwaitExpression(ae) => self.expr_is_string(&ae.argument),
            // ParenthesizedExpression: unwrap and recurse
            Expression::ParenthesizedExpression(pe) => self.expr_is_string(&pe.expression),
            // CallExpression: check if the function/method returns a string
            Expression::CallExpression(ce) => match &ce.callee {
                Expression::Identifier(id) => {
                    // Check user-defined and host function return types
                    if let Some(ret_ty) = self.fn_return_types.get(id.name.as_str()) {
                        return *ret_ty == ZigType::Str;
                    }
                    if let Some(ret_ty) = self.host_return_types.get(id.name.as_str()) {
                        return *ret_ty == ZigType::Str;
                    }
                    // Built-in functions known to return strings
                    matches!(
                        id.name.as_str(),
                        "String"
                            | "decodeURI"
                            | "decodeURIComponent"
                            | "encodeURI"
                            | "encodeURIComponent"
                    )
                }
                // Method calls: obj.method() — check method return type
                Expression::StaticMemberExpression(mem) => {
                    if let Some(obj_name) =
                        super::helpers::extract_expr_identifier_name(&mem.object)
                    {
                        // Array methods that return strings
                        if let Some(elem_ty) = self.array_element_types.get(&obj_name) {
                            let result =
                                self.infer_array_method_return(mem.property.name.as_str(), elem_ty);
                            if let InferResult::Definite(ZigType::Str) = result {
                                return true;
                            }
                        }
                        // Map/Set/Date/Str/etc. methods
                        if let Some(var_ty) = self.var_types.get(&obj_name) {
                            let result =
                                self.infer_named_method_return(var_ty, mem.property.name.as_str());
                            if let InferResult::Definite(ZigType::Str) = result {
                                return true;
                            }
                        }
                    }
                    // Built-in method calls (String.xxx, Math.xxx, etc.)
                    if let Some(builtin) = builtins::detect_builtin_call(ce)
                        && let Some(ret_ty) = builtins::builtin_return_type(&builtin)
                    {
                        return ret_ty == ZigType::Str;
                    }
                    false
                }
                // TODO: Function return type tracking for complex callees (e.g.,
                // chained calls, computed member calls) requires full expression
                // type inference which is not available in this &self context.
                _ => false,
            },
            // StaticMemberExpression: check known string properties by object type
            Expression::StaticMemberExpression(mem) => {
                if let Some(obj_name) = super::helpers::extract_expr_identifier_name(&mem.object)
                    && let Some(var_ty) = self.var_types.get(&obj_name)
                {
                    return match var_ty {
                        ZigType::JsError => {
                            matches!(mem.property.name.as_str(), "name" | "message" | "stack")
                        }
                        ZigType::JsSymbol => mem.property.name.as_str() == "description",
                        // INF-9: RegExp.source and RegExp.flags return strings
                        ZigType::NamedStruct(name) if name == "RegExp" => {
                            matches!(mem.property.name.as_str(), "source" | "flags")
                        }
                        _ => false,
                    };
                }
                // Also check RegExp literals: /pattern/.source, /pattern/g.flags
                if let Expression::RegExpLiteral(_) = &mem.object {
                    return matches!(mem.property.name.as_str(), "source" | "flags");
                }
                false
            }
            _ => false,
        }
    }

    pub(crate) fn infer_array_type(&mut self, ae: &ArrayExpression) -> InferResult {
        if ae.elements.is_empty() {
            // Empty array: default to ArrayList(JsAny) — JS allows any type in [].
            return InferResult::Definite(ZigType::ArrayList(Box::new(ZigType::JsAny)));
        }
        // Spread elements force ArrayList(JsAny) at the emit layer
        // (emit/expr/container.rs:16 checks `arr.spread_indices.is_empty()`).
        // Without this check, `[1, 2, ...x]` would infer `ArrayList(I64)`
        // since spreads are silently skipped by `as_expression()` below,
        // mismatching emit's `ArrayList(JsAny)` and risking downstream
        // type-annotation mismatches. (R24-INF-9)
        if ae
            .elements
            .iter()
            .any(|e| matches!(e, ArrayExpressionElement::SpreadElement(_)))
        {
            return InferResult::Indeterminate;
        }
        // R39-INF-3: Array holes (Elision) are replaced with JsAny by the
        // lowerer, so the inferred type must account for them.
        if ae
            .elements
            .iter()
            .any(|e| matches!(e, ArrayExpressionElement::Elision(_)))
        {
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
                    // R39-INF-6: Mixed-type arrays should degrade to
                    // ArrayList(JsAny) instead of Indeterminate (Rule 8 error).
                    _ => {
                        return InferResult::Definite(ZigType::ArrayList(Box::new(ZigType::JsAny)));
                    }
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
        // JsAny operand propagation: when either operand is JsAny (indeterminate
        // runtime type), arithmetic and bitwise operators return JsAny to avoid
        // incorrect type narrowing (e.g. JsAny + I64 must NOT infer I64).
        // Comparison operators always return Bool regardless of operand types.
        // Exception: Addition with a Str operand → Str (string concatenation
        // takes priority per ECMA-262 ToString coercion).
        let is_comparison = matches!(
            op,
            BinaryOperator::Equality
                | BinaryOperator::Inequality
                | BinaryOperator::StrictEquality
                | BinaryOperator::StrictInequality
                | BinaryOperator::LessThan
                | BinaryOperator::LessEqualThan
                | BinaryOperator::GreaterThan
                | BinaryOperator::GreaterEqualThan
                | BinaryOperator::In
                | BinaryOperator::Instanceof
        );
        if !is_comparison && (left == ZigType::JsAny || right == ZigType::JsAny) {
            if op == BinaryOperator::Addition && (left == ZigType::Str || right == ZigType::Str) {
                return ZigType::Str;
            }
            return ZigType::JsAny;
        }

        #[allow(unreachable_patterns)] // defensive: oxc may add new variants
        match op {
            BinaryOperator::Addition => {
                // JS `+`: if either operand is String, the result is a String
                // (implicit toString coercion per ECMA). This must be checked
                // BEFORE BigInt / F64 paths, otherwise `"a" + "b"` (Str + Str)
                // would be misclassified as I64. Mirrors the lowerer's
                // `infer_binary_result_type` Addition arm.
                if left == ZigType::Str || right == ZigType::Str {
                    return ZigType::Str;
                }
                // BigInt + BigInt → BigInt
                if left == ZigType::BigInt && right == ZigType::BigInt {
                    return ZigType::BigInt;
                }
                // String + BigInt → String (JS spec: implicit toString)
                // BigInt + String → String
                // (Redundant after the Str check above, kept for clarity and
                // to document the JS spec rule.)
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
            BinaryOperator::Subtraction | BinaryOperator::Multiplication => {
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
            // Division: JS `/` always returns float (5/2 === 2.5),
            // even when both operands are integers.
            // The Emitter generates DivExpr which always computes f64.
            BinaryOperator::Division => {
                if left == ZigType::BigInt && right == ZigType::BigInt {
                    return ZigType::BigInt;
                }
                ZigType::F64
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
            | BinaryOperator::GreaterEqualThan
            | BinaryOperator::In
            | BinaryOperator::Instanceof => ZigType::Bool,
            BinaryOperator::BitwiseAnd | BinaryOperator::BitwiseOR | BinaryOperator::BitwiseXOR => {
                // BigInt bitwise ops preserve BigInt (lowerer generates .bitAnd() etc.)
                if left == ZigType::BigInt && right == ZigType::BigInt {
                    return ZigType::BigInt;
                }
                ZigType::I64
            }
            BinaryOperator::ShiftLeft
            | BinaryOperator::ShiftRight
            | BinaryOperator::ShiftRightZeroFill => {
                // BigInt shifts preserve BigInt (lowerer generates .shl() etc.)
                if left == ZigType::BigInt && right == ZigType::BigInt {
                    return ZigType::BigInt;
                }
                ZigType::I64
            }
            _ => ZigType::JsAny,
        }
    }

    // ============================================================
    // Array / named method return type inference
    // ============================================================

    /// Infer the return type of array method calls like arr.slice(), arr.map(), etc.
    pub(crate) fn infer_array_method_return(
        &self,
        method: &str,
        _elem_ty: &ZigType,
    ) -> InferResult {
        match method {
            // Methods that return a new array — always ArrayList(JsAny) to match
            // builtin_return_type and infer_named_method_return. The emitter never
            // preserves the source element type (all builtin array methods produce
            // ArrayList(JsAny)), so preserving elem_ty here would cause type
            // annotation mismatches (e.g. var_types says ArrayList(I64) but the
            // generated code produces ArrayList(JsAny)).
            "slice" | "filter" | "concat" | "flat" | "flatMap" | "toReversed" | "toSorted"
            | "toSpliced" | "map" | "with" => {
                InferResult::Definite(ZigType::ArrayList(Box::new(ZigType::JsAny)))
            }
            // Methods that return a boolean
            "some" | "every" | "includes" => InferResult::Definite(ZigType::Bool),
            // Methods returning index or length
            "indexOf" | "lastIndexOf" | "findIndex" => InferResult::Definite(ZigType::I64),
            // reduce: returns accumulator type (default JsAny since accumulator can be any type)
            "reduce" | "reduceRight" => InferResult::Definite(ZigType::JsAny),
            // pop/shift/find/findLast/at: return element or undefined → JsAny
            // (matches native_builtins which all return JsAny for these methods)
            "pop" | "shift" | "find" | "findLast" | "at" => InferResult::Definite(ZigType::JsAny),
            // join: returns string
            "join" => InferResult::Definite(ZigType::Str),
            // push/unshift: return new length (i64)
            "push" | "unshift" => InferResult::Definite(ZigType::I64),
            // findLastIndex: returns index or -1
            "findLastIndex" => InferResult::Definite(ZigType::I64),
            // Iterator methods: return ArrayList(JsAny) — matches builtin_return_type
            "keys" | "values" | "entries" => {
                InferResult::Definite(ZigType::ArrayList(Box::new(ZigType::JsAny)))
            }
            // Mutation methods that return the receiver as JsAny — matches builtin_return_type
            "reverse" | "sort" | "copyWithin" | "fill" => InferResult::Definite(ZigType::JsAny),
            // splice: returns deleted elements as ArrayList(JsAny) — matches builtin_return_type
            "splice" => InferResult::Definite(ZigType::ArrayList(Box::new(ZigType::JsAny))),
            // forEach: returns undefined
            "forEach" => InferResult::Definite(ZigType::Void),
            // toString/toLocaleString: return comma-separated string
            "toString" | "toLocaleString" => InferResult::Definite(ZigType::Str),
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
                        "clear" | "forEach" => InferResult::Definite(ZigType::Void),
                        "keys" => {
                            InferResult::Definite(ZigType::ArrayList(Box::new(ZigType::JsAny)))
                        }
                        "values" => {
                            InferResult::Definite(ZigType::ArrayList(Box::new(ZigType::JsAny)))
                        }
                        "entries" => InferResult::Definite(ZigType::ArrayList(Box::new(
                            ZigType::ArrayList(Box::new(ZigType::JsAny)),
                        ))),
                        _ => InferResult::Indeterminate,
                    },
                    "Set" => match method {
                        "add" => InferResult::Definite(ZigType::NamedStruct("Set".into())),
                        "has" | "delete" => InferResult::Definite(ZigType::Bool),
                        "clear" | "forEach" => InferResult::Definite(ZigType::Void),
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
                        | "getHours" | "getMinutes" | "getSeconds" | "getMilliseconds"
                        | "valueOf"
                        // UTC getters
                        | "getUTCFullYear" | "getUTCMonth" | "getUTCDate" | "getUTCDay"
                        | "getUTCHours" | "getUTCMinutes" | "getUTCSeconds"
                        | "getUTCMilliseconds" | "getTimezoneOffset"
                        // Setters return the new timestamp (i64, same as getTime)
                        | "setFullYear" | "setMonth" | "setDate" | "setHours" | "setMinutes"
                        | "setSeconds" | "setMilliseconds" | "setTime"
                        | "setUTCFullYear" | "setUTCMonth" | "setUTCDate" | "setUTCHours"
                        | "setUTCMinutes" | "setUTCSeconds" | "setUTCMilliseconds" => {
                            InferResult::Definite(ZigType::I64)
                        }
                        "toString" | "toISOString" | "toDateString" | "toTimeString"
                        | "toLocaleString" | "toLocaleDateString" | "toLocaleTimeString"
                        | "toUTCString" | "toJSON" => InferResult::Definite(ZigType::Str),
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
                "indexOf" | "lastIndexOf" | "search" => InferResult::Definite(ZigType::I64),
                "charCodeAt" => InferResult::Definite(ZigType::F64), // Number (0-65535 or NaN)
                "codePointAt" => InferResult::Definite(ZigType::I64), // Number (0-0x10FFFF or 0 for out-of-bounds)
                "localeCompare" => InferResult::Definite(ZigType::I64),
                "includes" | "startsWith" | "endsWith" => InferResult::Definite(ZigType::Bool),
                "trim" | "trimStart" | "trimEnd" | "padStart" | "padEnd" | "charAt" | "at"
                | "toUpperCase" | "toLowerCase" | "slice" | "substring" | "replace"
                | "replaceAll" | "concat" | "repeat" | "normalize" | "toLocaleUpperCase"
                | "toLocaleLowerCase" => InferResult::Definite(ZigType::Str),
                // split() returns an array of strings, not a single string
                "split" => InferResult::Definite(ZigType::ArrayList(Box::new(ZigType::Str))),
                "match" | "matchAll" => InferResult::Definite(ZigType::JsAny),
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
            // Number methods (R8-NumberToString) — `.toString()` on an F64 or
            // I64 variable returns Str. Other Number prototype methods
            // (toFixed/toExponential/toPrecision) are statically dispatched
            // at detection time and never reach this fallback resolver.
            ZigType::F64 | ZigType::I64 => match method {
                "toString" | "toFixed" | "toExponential" | "toPrecision" => {
                    InferResult::Definite(ZigType::Str)
                }
                "valueOf" => InferResult::Definite(var_ty.clone()),
                _ => InferResult::Indeterminate,
            },
            // ArrayList methods — fallback for when array methods aren't
            // statically detected as BuiltinCalls. Types match builtin_return_type.
            ZigType::ArrayList(_) => match method {
                "push" | "unshift" => InferResult::Definite(ZigType::I64),
                "pop" | "shift" | "find" | "findLast" | "at" => {
                    InferResult::Definite(ZigType::JsAny)
                }
                "indexOf" | "lastIndexOf" | "findIndex" | "findLastIndex" => {
                    InferResult::Definite(ZigType::I64)
                }
                "includes" | "some" | "every" => InferResult::Definite(ZigType::Bool),
                "join" => InferResult::Definite(ZigType::Str),
                "forEach" => InferResult::Definite(ZigType::Void),
                "map" | "filter" | "slice" | "concat" | "flat" | "flatMap" | "with"
                | "toReversed" | "toSorted" | "toSpliced" => {
                    InferResult::Definite(ZigType::ArrayList(Box::new(ZigType::JsAny)))
                }
                "reverse" | "sort" | "copyWithin" | "fill" => InferResult::Definite(ZigType::JsAny),
                "splice" => InferResult::Definite(ZigType::ArrayList(Box::new(ZigType::JsAny))),
                "keys" | "values" | "entries" => {
                    InferResult::Definite(ZigType::ArrayList(Box::new(ZigType::JsAny)))
                }
                "reduce" | "reduceRight" => InferResult::Definite(ZigType::JsAny),
                "toString" | "toLocaleString" => InferResult::Definite(ZigType::Str),
                _ => InferResult::Indeterminate,
            },
            // JsError methods: name, message, stack are strings; toString returns string
            ZigType::JsError => match method {
                "name" | "message" | "stack" | "toString" => InferResult::Definite(ZigType::Str),
                _ => InferResult::Indeterminate,
            },
            _ => InferResult::Indeterminate,
        }
    }
}
