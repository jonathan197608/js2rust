// zigir/lower/expr/member.rs
// Static/computed member expressions + type inference.

use oxc_ast::ast::*;

use crate::types::ZigType;
use crate::zigir::kinds::{ComputedKeyKind, FieldKind, IndexKind};

use super::Lowerer;

impl Lowerer {
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
                return IrExpr::FieldAccess {
                    object: Box::new(self.lower_expr(&mem.object)),
                    field: field_name.to_string(),
                    field_kind: FieldKind::MathConstant(field_name.to_string()),
                };
            }
            // ── Number constants: Number.MAX_VALUE, Number.NaN, etc. ──
            if id.name.as_str() == "Number" {
                return IrExpr::FieldAccess {
                    object: Box::new(self.lower_expr(&mem.object)),
                    field: field_name.to_string(),
                    field_kind: FieldKind::NumberConstant(field_name.to_string()),
                };
            }
            // ── Symbol well-known: Symbol.iterator, etc. ──
            if id.name.as_str() == "Symbol" {
                return IrExpr::FieldAccess {
                    object: Box::new(self.lower_expr(&mem.object)),
                    field: field_name.to_string(),
                    field_kind: FieldKind::SymbolWellKnown(field_name.to_string()),
                };
            }
            // ── TypedArray properties ──
            if let Some(zig_type) = self.type_info.var_types.get(id.name.as_str()) {
                if let ZigType::NamedStruct(name) = zig_type {
                    // ── TypedArray properties (buffer, byteLength, byteOffset) ──
                    if Self::is_typedarray_type(name)
                        && matches!(field_name, "buffer" | "byteLength" | "byteOffset")
                    {
                        let type_suffix = Self::typedarray_type_suffix(name).map(|s| s.to_string());
                        return IrExpr::FieldAccess {
                            object: Box::new(self.lower_expr(&mem.object)),
                            field: field_name.to_string(),
                            field_kind: FieldKind::TypedArrayProp {
                                prop: field_name.to_string(),
                                type_suffix,
                            },
                        };
                    }
                    // ── Map/Set .size ──
                    if matches!(name.as_str(), "Map" | "Set") && field_name == "size" {
                        return IrExpr::FieldAccess {
                            object: Box::new(self.lower_expr(&mem.object)),
                            field: field_name.to_string(),
                            field_kind: FieldKind::MapSetSize,
                        };
                    }
                }
                // ── ArrayList .length → .items.len ──
                if matches!(zig_type, ZigType::ArrayList(_)) && field_name == "length" {
                    return IrExpr::FieldAccess {
                        object: Box::new(self.lower_expr(&mem.object)),
                        field: field_name.to_string(),
                        field_kind: FieldKind::ArrayListLen,
                    };
                }
            }
        }

        // ── .length on all types (string, slice, call result, etc.) ──
        if field_name == "length" {
            return IrExpr::FieldAccess {
                object: Box::new(self.lower_expr(&mem.object)),
                field: field_name.to_string(),
                field_kind: FieldKind::StringLen,
            };
        }

        // ── Default: struct field access ──
        IrExpr::FieldAccess {
            object: Box::new(self.lower_expr(&mem.object)),
            field: field_name.to_string(),
            field_kind: FieldKind::StructField,
        }
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

        // ── Case 1: NumericLiteral key → IndexAccess ──
        if let Expression::NumericLiteral(nl) = &mem.expression {
            let is_arraylist = obj_type
                .as_ref()
                .map(|t| matches!(t, ZigType::ArrayList(_)))
                .unwrap_or(false);
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

    /// Infer the ZigType of an expression based on type_info and expression structure.
    /// Enhanced version that covers literal types, member access, calls, and more.
    pub(super) fn infer_expr_type(&self, expr: &Expression) -> Option<ZigType> {
        match expr {
            Expression::Identifier(id) => {
                // Special globals
                match id.name.as_str() {
                    "Infinity" | "NaN" => return Some(ZigType::F64),
                    "undefined" => return Some(ZigType::JsAny),
                    _ => {}
                }
                // Try exact match, then qualified, then suffix-based
                if let Some(ty) = self.type_info.var_types.get(id.name.as_str()) {
                    return Some(ty.clone());
                }
                if let Some(ctx) = self.fn_ctx.as_ref() {
                    let qualified = format!("{}::{}", ctx.name, id.name);
                    if let Some(ty) = self.type_info.var_types.get(&qualified) {
                        return Some(ty.clone());
                    }
                }
                let suffix = format!("::{}", id.name);
                for (k, v) in &self.type_info.var_types {
                    if k.ends_with(&suffix) {
                        return Some(v.clone());
                    }
                }
                None
            }
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

    /// Infer the type of an assignment target (left-hand side of `=` / `+=` etc.).
    /// Only handles the common cases: identifier and static member expression.
    pub(super) fn infer_assign_target_type(
        &self,
        target: &oxc_ast::ast::AssignmentTarget,
    ) -> Option<ZigType> {
        use oxc_ast::ast::AssignmentTarget;
        match target {
            AssignmentTarget::AssignmentTargetIdentifier(id) => {
                // Reuse the same logic as infer_expr_type for identifiers
                match id.name.as_str() {
                    "Infinity" | "NaN" => return Some(ZigType::F64),
                    "undefined" => return Some(ZigType::JsAny),
                    _ => {}
                }
                if let Some(ty) = self.type_info.var_types.get(id.name.as_str()) {
                    return Some(ty.clone());
                }
                if let Some(ctx) = self.fn_ctx.as_ref() {
                    let qualified = format!("{}::{}", ctx.name, id.name);
                    if let Some(ty) = self.type_info.var_types.get(&qualified) {
                        return Some(ty.clone());
                    }
                }
                let suffix = format!("::{}", id.name);
                for (k, v) in &self.type_info.var_types {
                    if k.ends_with(&suffix) {
                        return Some(v.clone());
                    }
                }
                None
            }
            AssignmentTarget::StaticMemberExpression(mem) => {
                // If the object is a known type, try to infer the field type
                self.infer_expr_type(&mem.object)
            }
            _ => None,
        }
    }
}
