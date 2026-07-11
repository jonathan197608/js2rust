// zigir/lower/expr/member.rs
// Static/computed member expressions + type inference.

use oxc_ast::ast::*;

use crate::types::ZigType;
use crate::zigir::ident::IrIdent;
use crate::zigir::kinds::{ComputedKeyKind, FieldKind, IndexKind};

use super::Lowerer;

impl Lowerer {
    /// Shorthand to construct `IrExpr::FieldAccess { object, field, field_kind }`
    /// with the standard object-lowering and field-name conversion.
    fn make_field_access(
        &mut self,
        mem: &StaticMemberExpression,
        field_kind: FieldKind,
    ) -> crate::zigir::types::IrExpr {
        use crate::zigir::types::IrExpr;
        IrExpr::FieldAccess {
            object: Box::new(self.lower_expr(&mem.object)),
            field: mem.property.name.as_str().to_string(),
            field_kind,
        }
    }

    /// Lower a static member expression (`obj.field`).
    ///
    /// Determines the FieldKind based on:
    /// - Math constants → `MathConstant`
    /// - Number constants → `NumberConstant`
    /// - Symbol well-known → `SymbolWellKnown`
    /// - TypedArray properties → `TypedArrayProp`
    /// - Map/Set `.size` → `MapSetSize`
    /// - ArrayList `.length` → `ArrayListLen`
    /// - Other `.length` → `StringLen`
    /// - Default → `StructField`
    pub(super) fn lower_static_member(
        &mut self,
        mem: &StaticMemberExpression,
    ) -> crate::zigir::types::IrExpr {
        use crate::zigir::types::IrExpr;

        let field_name = mem.property.name.as_str();

        // ── Math constants: Math.PI, Math.E, etc. ──
        if let Expression::Identifier(id) = &mem.object {
            if id.name.as_str() == "Math" {
                return self
                    .make_field_access(mem, FieldKind::MathConstant(field_name.to_string()));
            }
            // ── Number constants: Number.MAX_VALUE, Number.NaN, etc. ──
            if id.name.as_str() == "Number" {
                return self
                    .make_field_access(mem, FieldKind::NumberConstant(field_name.to_string()));
            }
            // ── Symbol well-known: Symbol.iterator, etc. ──
            if id.name.as_str() == "Symbol" {
                return self
                    .make_field_access(mem, FieldKind::SymbolWellKnown(field_name.to_string()));
            }
            // ── TypedArray properties ──
            if let Some(zig_type) = self.type_info.var_types.get(id.name.as_str()) {
                if let ZigType::NamedStruct(name) = zig_type {
                    // ── TypedArray properties (buffer, byteLength, byteOffset) ──
                    if Self::is_typedarray_type(name)
                        && matches!(field_name, "buffer" | "byteLength" | "byteOffset")
                    {
                        let type_suffix = Self::typedarray_type_suffix(name).map(|s| s.to_string());
                        return self.make_field_access(
                            mem,
                            FieldKind::TypedArrayProp {
                                prop: field_name.to_string(),
                                type_suffix,
                            },
                        );
                    }
                    // ── Map/Set .size ──
                    if matches!(name.as_str(), "Map" | "Set") && field_name == "size" {
                        return self.make_field_access(mem, FieldKind::MapSetSize);
                    }
                }
                // ── ArrayList .length → .items.len ──
                if matches!(zig_type, ZigType::ArrayList(_)) && field_name == "length" {
                    return self.make_field_access(mem, FieldKind::ArrayListLen);
                }
            }
        }

        // ── .length — type-aware dispatch ──
        if field_name == "length" {
            // Check type info for the object to determine the right FieldKind
            if let Expression::Identifier(id) = &mem.object
                && let Some(zig_type) = self.type_info.var_types.get(id.name.as_str())
            {
                if matches!(zig_type, ZigType::Str) {
                    return self.make_field_access(mem, FieldKind::StringLen);
                }
                if matches!(zig_type, ZigType::ArrayList(_)) {
                    return self.make_field_access(mem, FieldKind::ArrayListLen);
                }
                // NamedStruct (TypedArray, Map, Set, etc.) or other types → slice .len
                return self.make_field_access(mem, FieldKind::SliceLen);
            }
            // No var_types entry: try infer_expr_type for non-Identifier objects
            if !matches!(&mem.object, Expression::Identifier(_)) {
                if let Some(inferred) = self.infer_expr_type(&mem.object) {
                    if matches!(inferred, ZigType::Str) {
                        return self.make_field_access(mem, FieldKind::StringLen);
                    }
                    return self.make_field_access(mem, FieldKind::SliceLen);
                }
                // No type info at all — default to SliceLen
                return self.make_field_access(mem, FieldKind::SliceLen);
            }
            // Identifier with no type info: default to StringLen
            return self.make_field_access(mem, FieldKind::StringLen);
        }

        // ── RegExp properties: .source, .flags, .global ──
        if let Expression::Identifier(id) = &mem.object {
            let var_name = id.name.as_str();
            if let Some(ctx) = &self.fn_ctx
                && ctx.regexp_vars.contains(var_name)
                && matches!(field_name, "source" | "flags" | "global")
            {
                return self.make_field_access(
                    mem,
                    FieldKind::RegExpProp {
                        prop: field_name.to_string(),
                    },
                );
            }
        }

        // ── Static class field: ClassName.field → StaticField kind ──
        if let Expression::Identifier(id) = &mem.object {
            let obj_name = id.name.as_str();
            if let Some(static_fields) = self.class_static_fields.get(obj_name)
                && static_fields.contains(field_name)
            {
                return self.make_field_access(
                    mem,
                    FieldKind::StaticField {
                        class_name: obj_name.to_string(),
                    },
                );
            }
        }

        // ── Static block: this.field → StaticField kind (same as ClassName.field) ──
        // Note: this uses a different object (class_name instead of lowered this), so
        // we can't use make_field_access here.
        if matches!(&mem.object, Expression::ThisExpression(_))
            && self.in_static_block
            && let Some(ref class_name) = self.current_class
            && let Some(static_fields) = self.class_static_fields.get(class_name)
            && static_fields.contains(field_name)
        {
            return IrExpr::FieldAccess {
                object: Box::new(IrExpr::Ident(IrIdent::new(class_name))),
                field: field_name.to_string(),
                field_kind: FieldKind::StaticField {
                    class_name: class_name.clone(),
                },
            };
        }

        // ── Default: struct field access ──
        self.make_field_access(mem, FieldKind::StructField)
    }

    /// Lower a computed member expression (`obj[key]`).
    ///
    /// Three sub-cases:
    /// - NumericLiteral key → IndexAccess (ArrayListItem or SliceIndex)
    /// - StringLiteral key → ComputedField (StructField, MapGet, JsAnyGetByKey)
    /// - Dynamic expression key → ComputedField (varies by object type)
    pub(super) fn lower_computed_member(
        &mut self,
        mem: &ComputedMemberExpression,
    ) -> crate::zigir::types::IrExpr {
        use crate::zigir::types::IrExpr;

        let object = Box::new(self.lower_expr(&mem.object));

        // Determine the ZigType of the object (for routing)
        let obj_type = self.infer_expr_type(&mem.object);

        // ── Case 1: NumericLiteral key → IndexAccess or StringChar ──
        if let Expression::NumericLiteral(nl) = &mem.expression {
            let is_arraylist = obj_type
                .as_ref()
                .map(|t| matches!(t, ZigType::ArrayList(_)))
                .unwrap_or(false);
            let is_string = obj_type
                .as_ref()
                .map(|t| matches!(t, ZigType::Str))
                .unwrap_or(false);
            // str[0] → StringChar (JS charCodeAt semantics, returns i64)
            if is_string {
                return IrExpr::ComputedField {
                    object,
                    key: Box::new(IrExpr::IntLiteral(nl.value as i64)),
                    key_kind: ComputedKeyKind::StringChar,
                };
            }
            return IrExpr::IndexAccess {
                object,
                index: Box::new(IrExpr::IntLiteral(nl.value as i64)),
                index_kind: if is_arraylist {
                    IndexKind::ArrayListItem
                } else {
                    IndexKind::SliceIndex
                },
            };
        }

        // ── Case 2: StringLiteral key → ComputedField ──
        if let Expression::StringLiteral(sl) = &mem.expression {
            let key_kind = match &obj_type {
                Some(ZigType::Struct(_)) => ComputedKeyKind::StructField,
                Some(ZigType::NamedStruct(name)) if name == "Map" => ComputedKeyKind::MapGet,
                Some(ZigType::NamedStruct(_)) => ComputedKeyKind::StructField,
                Some(ZigType::Anytype) | Some(ZigType::JsAny) => ComputedKeyKind::JsAnyGetByKey,
                _ => ComputedKeyKind::JsAnyGetByKey,
            };
            return IrExpr::ComputedField {
                object,
                key: Box::new(IrExpr::StringLiteral(sl.value.to_string())),
                key_kind,
            };
        }

        // ── Case 3: Dynamic expression key → ComputedField ──
        let key = Box::new(self.lower_expr(&mem.expression));
        let key_kind = match &obj_type {
            Some(ZigType::Anytype) | Some(ZigType::JsAny) => ComputedKeyKind::JsAnyGetByKey,
            Some(ZigType::NamedStruct(name)) if name == "Map" => ComputedKeyKind::MapGet,
            Some(ZigType::ArrayList(_)) => ComputedKeyKind::ArrayListItem,
            Some(ZigType::Str) => ComputedKeyKind::StringChar,
            Some(ZigType::Struct(_)) | Some(ZigType::NamedStruct(_)) => {
                ComputedKeyKind::StructField
            }
            None => ComputedKeyKind::JsAnyGetByKey,
            _ => ComputedKeyKind::CompileError(format!(
                "computed access on unsupported type: {:?}",
                obj_type
            )),
        };
        IrExpr::ComputedField {
            object,
            key,
            key_kind,
        }
    }

    /// Look up the ZigType of an identifier by name.
    /// Checks special globals, then var_types (exact, qualified, suffix-based).
    pub(crate) fn infer_ident_type(&self, name: &str) -> Option<ZigType> {
        // Special globals
        match name {
            "Infinity" | "NaN" => return Some(ZigType::F64),
            "undefined" => return Some(ZigType::JsAny),
            _ => {}
        }
        // Exact match
        if let Some(ty) = self.type_info.var_types.get(name) {
            return Some(ty.clone());
        }
        // Qualified match (fn_name::var_name)
        if let Some(ctx) = self.fn_ctx.as_ref() {
            let qualified = format!("{}::{}", ctx.name, name);
            if let Some(ty) = self.type_info.var_types.get(&qualified) {
                return Some(ty.clone());
            }
        }
        // Suffix match (any_key::var_name)
        let suffix = format!("::{}", name);
        for (k, v) in &self.type_info.var_types {
            if k.ends_with(&suffix) {
                return Some(v.clone());
            }
        }
        None
    }

    /// Infer the ZigType of an expression based on type_info and expression structure.
    /// Enhanced version that covers literal types, member access, calls, and more.
    pub(crate) fn infer_expr_type(&self, expr: &Expression) -> Option<ZigType> {
        match expr {
            Expression::Identifier(id) => self.infer_ident_type(id.name.as_str()),
            Expression::NumericLiteral(nl) => {
                // Distinguish I64 vs F64 based on presence of decimal point / exponent
                let s = nl.value.to_string();
                if s.contains('.') || s.contains('e') || s.contains('E') {
                    Some(ZigType::F64)
                } else {
                    Some(ZigType::I64)
                }
            }
            Expression::StringLiteral(_) => Some(ZigType::Str),
            Expression::TemplateLiteral(_) => Some(ZigType::Str),
            Expression::BooleanLiteral(_) => Some(ZigType::Bool),
            Expression::BigIntLiteral(_) => Some(ZigType::BigInt),
            Expression::NullLiteral(_) => Some(ZigType::JsAny),
            Expression::UnaryExpression(ue) => match ue.operator {
                UnaryOperator::LogicalNot => Some(ZigType::Bool),
                UnaryOperator::Void => Some(ZigType::JsAny),
                UnaryOperator::Typeof => Some(ZigType::Str),
                UnaryOperator::UnaryNegation | UnaryOperator::UnaryPlus => {
                    self.infer_expr_type(&ue.argument)
                }
                _ => None,
            },
            Expression::BinaryExpression(be) => {
                let left_ty = self.infer_expr_type(&be.left);
                let right_ty = self.infer_expr_type(&be.right);
                Self::infer_binary_result_type(&be.operator, left_ty, right_ty)
            }
            Expression::ConditionalExpression(ce) => {
                let then_ty = self.infer_expr_type(&ce.consequent);
                let else_ty = self.infer_expr_type(&ce.alternate);
                match (then_ty, else_ty) {
                    (Some(a), Some(b)) if a == b => Some(a),
                    (Some(ZigType::F64), _) | (_, Some(ZigType::F64)) => Some(ZigType::F64),
                    _ => None,
                }
            }
            Expression::ParenthesizedExpression(pe) => self.infer_expr_type(&pe.expression),
            Expression::StaticMemberExpression(mem) => {
                // Known constants
                if let Expression::Identifier(id) = &mem.object {
                    match id.name.as_str() {
                        "Math" => {
                            return match mem.property.name.as_str() {
                                "PI" | "E" | "LN2" | "LN10" | "LOG2E" | "LOG10E" | "SQRT1_2"
                                | "SQRT2" => Some(ZigType::F64),
                                _ => None,
                            };
                        }
                        "Number" => {
                            return match mem.property.name.as_str() {
                                "MAX_SAFE_INTEGER" | "MIN_SAFE_INTEGER" | "MAX_VALUE"
                                | "MIN_VALUE" => Some(ZigType::F64),
                                "POSITIVE_INFINITY" | "NEGATIVE_INFINITY" | "NaN" => {
                                    Some(ZigType::F64)
                                }
                                "EPSILON" => Some(ZigType::F64),
                                _ => None,
                            };
                        }
                        _ => {}
                    }
                }
                // Try struct field inference
                let obj_ty = self.infer_expr_type(&mem.object);
                match obj_ty {
                    Some(ZigType::NamedStruct(name)) => {
                        if let Some(fields) = self.type_info.class_field_types.get(&name)
                            && let Some(ty) = fields.get(mem.property.name.as_str())
                        {
                            return Some(ty.clone());
                        }
                        None
                    }
                    _ => None,
                }
            }
            Expression::CallExpression(ce) => {
                // Try to infer from known method calls
                if let Expression::StaticMemberExpression(mem) = &ce.callee {
                    if let Expression::Identifier(id) = &mem.object {
                        match id.name.as_str() {
                            "parseInt" | "Number" => return Some(ZigType::I64),
                            "parseFloat" => return Some(ZigType::F64),
                            _ => {}
                        }
                    }
                    // Method return type from object type
                    let obj_ty = self.infer_expr_type(&mem.object);
                    if let Some(ZigType::NamedStruct(name)) = &obj_ty {
                        match name.as_str() {
                            "Map" => match mem.property.name.as_str() {
                                "get" => return Some(ZigType::JsAny),
                                "has" | "delete" => return Some(ZigType::Bool),
                                _ => {}
                            },
                            "Set" => match mem.property.name.as_str() {
                                "has" | "delete" => return Some(ZigType::Bool),
                                _ => {}
                            },
                            _ => {}
                        }
                    }
                    // String method returns
                    if obj_ty == Some(ZigType::Str) {
                        match mem.property.name.as_str() {
                            "charAt" | "substring" | "slice" | "toLowerCase" | "toUpperCase"
                            | "trim" | "repeat" | "replace" | "replaceAll" | "padStart"
                            | "padEnd" => return Some(ZigType::Str),
                            "indexOf" | "lastIndexOf" | "charCodeAt" | "codePointAt" => {
                                return Some(ZigType::I64);
                            }
                            "includes" | "startsWith" | "endsWith" => return Some(ZigType::Bool),
                            _ => {}
                        }
                    }
                }
                // Try function return type lookup
                if let Expression::Identifier(id) = &ce.callee
                    && let Some(ty) = self.type_info.fn_return_types.get(id.name.as_str())
                {
                    return Some(ty.clone());
                }
                // Try built-in constructor / function calls
                if let Some(builtin) = crate::native_builtins::detect_builtin_call(ce) {
                    // Object(x) → passthrough: inherits argument type
                    if builtin == crate::native_builtins::BuiltinCall::ObjectConstructor
                        && ce.arguments.len() == 1
                        && let Some(arg) = ce.arguments.first()
                        && let Some(e) = arg.as_expression()
                        && let Some(arg_ty) = self.infer_expr_type(e)
                    {
                        return Some(arg_ty);
                    }
                    if let Some(ret_ty) = crate::native_builtins::builtin_return_type(&builtin) {
                        return Some(ret_ty);
                    }
                }
                None
            }
            // Object literal → infer as Struct with field types
            Expression::ObjectExpression(oe) => {
                let mut fields = Vec::new();
                for prop in &oe.properties {
                    if let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(op) = prop {
                        let field_name = match &op.key {
                            oxc_ast::ast::PropertyKey::StaticIdentifier(id) => {
                                id.name.as_str().to_string()
                            }
                            oxc_ast::ast::PropertyKey::StringLiteral(s) => s.value.to_string(),
                            _ => continue,
                        };
                        let field_ty = self.infer_expr_type(&op.value).unwrap_or(ZigType::I64);
                        fields.push((field_name, field_ty));
                    }
                }
                Some(ZigType::Struct(fields))
            }
            // Array literal → ArrayList of element type
            Expression::ArrayExpression(ae) => {
                let elem_ty = ae
                    .elements
                    .iter()
                    .find_map(|el| el.as_expression().and_then(|e| self.infer_expr_type(e)))
                    .unwrap_or(ZigType::I64);
                Some(ZigType::ArrayList(Box::new(elem_ty)))
            }
            // Could add more patterns here
            _ => None,
        }
    }

    /// Infer the result type of a binary operation from operand types.
    pub(super) fn infer_binary_result_type(
        op: &BinaryOperator,
        left_ty: Option<ZigType>,
        right_ty: Option<ZigType>,
    ) -> Option<ZigType> {
        match op {
            // Comparison operators always produce bool
            BinaryOperator::Equality
            | BinaryOperator::Inequality
            | BinaryOperator::StrictEquality
            | BinaryOperator::StrictInequality
            | BinaryOperator::LessThan
            | BinaryOperator::GreaterThan
            | BinaryOperator::LessEqualThan
            | BinaryOperator::GreaterEqualThan
            | BinaryOperator::In => Some(ZigType::Bool),

            // Addition: string if either operand is string, otherwise numeric
            BinaryOperator::Addition => match (left_ty.as_ref(), right_ty.as_ref()) {
                (Some(ZigType::Str), _) | (_, Some(ZigType::Str)) => Some(ZigType::Str),
                (Some(ZigType::F64), _) | (_, Some(ZigType::F64)) => Some(ZigType::F64),
                (Some(ZigType::I64), Some(ZigType::I64)) => Some(ZigType::I64),
                _ => None,
            },

            // Arithmetic: F64 if either F64, else I64
            BinaryOperator::Subtraction
            | BinaryOperator::Multiplication
            | BinaryOperator::Division
            | BinaryOperator::Remainder => match (left_ty.as_ref(), right_ty.as_ref()) {
                (Some(ZigType::F64), _) | (_, Some(ZigType::F64)) => Some(ZigType::F64),
                (Some(ZigType::I64), Some(ZigType::I64)) => Some(ZigType::I64),
                _ => None,
            },

            // Exponential: BigInt if both operands BigInt, f64 otherwise
            BinaryOperator::Exponential => match (left_ty.as_ref(), right_ty.as_ref()) {
                (Some(ZigType::BigInt), _) | (_, Some(ZigType::BigInt)) => Some(ZigType::BigInt),
                _ => Some(ZigType::F64),
            },

            // Bitwise: always I64
            BinaryOperator::BitwiseAnd
            | BinaryOperator::BitwiseOR
            | BinaryOperator::BitwiseXOR
            | BinaryOperator::ShiftLeft
            | BinaryOperator::ShiftRight
            | BinaryOperator::ShiftRightZeroFill => Some(ZigType::I64),

            _ => None,
        }
    }

    /// Infer the type of a simple assignment target (left-hand side of `++`/`--` etc.).
    /// Handles the same cases as `infer_assign_target_type` but for `SimpleAssignmentTarget`.
    pub(super) fn infer_simple_assign_target_type(
        &self,
        target: &SimpleAssignmentTarget,
    ) -> Option<ZigType> {
        use oxc_ast::ast::SimpleAssignmentTarget;
        match target {
            SimpleAssignmentTarget::AssignmentTargetIdentifier(id) => {
                self.infer_ident_type(id.name.as_str())
            }
            SimpleAssignmentTarget::StaticMemberExpression(mem) => {
                self.infer_static_member_type(mem)
            }
            _ => None,
        }
    }

    /// Infer the type of an assignment target (left-hand side of `=` / `+=` etc.).
    /// Only handles the common cases: identifier and static member expression.
    pub(super) fn infer_assign_target_type(&self, target: &AssignmentTarget) -> Option<ZigType> {
        use oxc_ast::ast::AssignmentTarget;
        match target {
            AssignmentTarget::AssignmentTargetIdentifier(id) => {
                self.infer_ident_type(id.name.as_str())
            }
            AssignmentTarget::StaticMemberExpression(mem) => self.infer_static_member_type(mem),
            _ => None,
        }
    }

    /// Shared logic for inferring the type of a static member expression
    /// used as an assignment target (`obj.field = ...` or `obj.field++`).
    /// Checks static class field type, then falls back to object type.
    fn infer_static_member_type(&self, mem: &StaticMemberExpression) -> Option<ZigType> {
        if let Expression::Identifier(id) = &mem.object {
            let obj_name = id.name.as_str();
            let field_name = mem.property.name.as_str();
            if let Some(static_fields) = self.class_static_fields.get(obj_name)
                && static_fields.contains(field_name)
            {
                let var_key = format!("__{}_{}", obj_name, field_name);
                if let Some(ty) = self.type_info.var_types.get(&var_key) {
                    return Some(ty.clone());
                }
            }
        }
        self.infer_expr_type(&mem.object)
    }

    /// Determine FieldKind for a member assignment target (`obj.field = ...`).
    ///
    /// Checks whether `object_expr.field_name` refers to a static class field,
    /// and returns `FieldKind::StaticField` if so. Otherwise returns `FieldKind::StructField`.
    pub(super) fn infer_member_field_kind(
        &self,
        object_expr: &Expression,
        field_name: &str,
    ) -> FieldKind {
        if let Expression::Identifier(id) = object_expr {
            let obj_name = id.name.as_str();
            if let Some(static_fields) = self.class_static_fields.get(obj_name)
                && static_fields.contains(field_name)
            {
                return FieldKind::StaticField {
                    class_name: obj_name.to_string(),
                };
            }
        }
        // In static blocks, `this.field` is equivalent to `ClassName.field`
        if matches!(object_expr, Expression::ThisExpression(_))
            && self.in_static_block
            && let Some(ref class_name) = self.current_class
            && let Some(static_fields) = self.class_static_fields.get(class_name)
            && static_fields.contains(field_name)
        {
            return FieldKind::StaticField {
                class_name: class_name.clone(),
            };
        }
        FieldKind::StructField
    }
}
