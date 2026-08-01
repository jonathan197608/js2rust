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

        // R8-C7: Inside a constructor body, reads of `this.<field>` must also
        // be rewritten to the pre-declared local `var <field>`. The ctor's
        // `init` function has no `self` parameter (it returns a fresh struct
        // by value), so the default lowering — `IrExpr::This` + FieldAccess —
        // would emit `self.field`, which Zig rejects as an undeclared
        // identifier. The `this_rewrite_fields` flag is set inside
        // `lower_class_method` for constructors exactly so that BOTH reads
        // and writes of `this.field` get rewritten consistently.
        //
        // This branch must come before every other `this`-based special case
        // (static block, default StructField) because those assume a `self`
        // receiver exists, which is false inside `init`.
        if let Some(ref fields) = self.this_rewrite_fields
            && matches!(&mem.object, Expression::ThisExpression(_))
            && fields.contains(&field_name.to_string())
        {
            return IrExpr::Ident(IrIdent::new(field_name));
        }

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
            if let Some(zig_type) = self.get_var_type(id.name.as_str()) {
                if let ZigType::NamedStruct(ref name) = zig_type {
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
                // -- ArrayList .length -> .items.len --
                // (arguments/__arguments are []const JsAny slices, NOT ArrayList -
                //  handled by special case below)
                if matches!(zig_type, ZigType::ArrayList(_))
                    && field_name == "length"
                    && id.name.as_str() != "__arguments"
                    && id.name.as_str() != "arguments"
                {
                    return self.make_field_access(mem, FieldKind::ArrayListLen);
                }
            }
        }

        // ── .length — type-aware dispatch ──
        if field_name == "length" {
            // Special case: __arguments is a []const JsAny slice
            if let Expression::Identifier(id) = &mem.object
                && (id.name.as_str() == "__arguments" || id.name.as_str() == "arguments")
            {
                return self.make_field_access(mem, FieldKind::ArgumentsLen);
            }
            // Check type info for the object to determine the right FieldKind
            if let Expression::Identifier(id) = &mem.object
                && let Some(zig_type) = self.get_var_type(id.name.as_str())
            {
                if matches!(zig_type, ZigType::Str) {
                    return self.make_field_access(mem, FieldKind::StringLen);
                }
                if matches!(zig_type, ZigType::ArrayList(_)) {
                    return self.make_field_access(mem, FieldKind::ArrayListLen);
                }
                // Rest params are []const JsAny slices, NOT JsAny unions.
                // Skip JsAnyLen for them — use SliceLen (.len on a slice).
                let is_rest_param = self
                    .fn_ctx
                    .as_ref()
                    .and_then(|ctx| ctx.rest_param_name.as_deref())
                    .is_some_and(|name| name == id.name.as_str());
                if !is_rest_param && matches!(zig_type, ZigType::JsAny) {
                    return self.make_field_access(mem, FieldKind::JsAnyLen);
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
                    if matches!(inferred, ZigType::JsAny) {
                        return self.make_field_access(mem, FieldKind::JsAnyLen);
                    }
                    return self.make_field_access(mem, FieldKind::SliceLen);
                }
                // No type info at all — default to SliceLen
                return self.make_field_access(mem, FieldKind::SliceLen);
            }
            // Identifier with no type info: default to StringLen
            return self.make_field_access(mem, FieldKind::StringLen);
        }

        // ── RegExp properties: .source, .flags, .global, .ignoreCase ──
        if let Expression::Identifier(id) = &mem.object {
            let var_name = id.name.as_str();
            if let Some(ctx) = &self.fn_ctx
                && ctx.regexp_vars.contains(var_name)
                && matches!(
                    field_name,
                    "source" | "flags" | "global" | "ignoreCase" | "lastIndex"
                )
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

        // Empty struct (JsObjectMap): route to map get
        if let Some(ZigType::Struct(ref f)) = self.infer_expr_type(&mem.object)
            && f.is_empty()
        {
            return IrExpr::ComputedField {
                object: Box::new(self.lower_expr(&mem.object)),
                key: Box::new(IrExpr::StringLiteral(field_name.to_string())),
                key_kind: ComputedKeyKind::ObjectMapGet,
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

        // Special case: arguments/__arguments → slice indexing
        let is_arguments = matches!(&mem.object, Expression::Identifier(id)
            if id.name.as_str() == "arguments" || id.name.as_str() == "__arguments");

        // ── Case 1: NumericLiteral key → IndexAccess or StringCharAt ──
        if let Expression::NumericLiteral(nl) = &mem.expression {
            if is_arguments {
                return IrExpr::IndexAccess {
                    object,
                    index: Box::new(IrExpr::IntLiteral(nl.value as i64)),
                    index_kind: IndexKind::SliceIndex,
                };
            }
            let is_arraylist = obj_type
                .as_ref()
                .map(|t| matches!(t, ZigType::ArrayList(_)))
                .unwrap_or(false);
            let is_jsany = obj_type
                .as_ref()
                .map(|t| matches!(t, ZigType::JsAny))
                .unwrap_or(false);
            let is_string = obj_type
                .as_ref()
                .map(|t| matches!(t, ZigType::Str))
                .unwrap_or(false);
            // str[0] → StringCharAt (JS charAt semantics, returns Str)
            if is_string {
                return IrExpr::ComputedField {
                    object,
                    key: Box::new(IrExpr::IntLiteral(nl.value as i64)),
                    key_kind: ComputedKeyKind::StringCharAt,
                };
            }
            return IrExpr::IndexAccess {
                object,
                index: Box::new(IrExpr::IntLiteral(nl.value as i64)),
                index_kind: if is_arraylist {
                    IndexKind::ArrayListItem
                } else if is_jsany {
                    IndexKind::JsAnyIndex
                } else {
                    IndexKind::SliceIndex
                },
            };
        }

        // ── Case 2: StringLiteral key → ComputedField ──
        if let Expression::StringLiteral(sl) = &mem.expression {
            let key_kind = match &obj_type {
                Some(ZigType::Struct(f)) if f.is_empty() => ComputedKeyKind::ObjectMapGet,
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
        // Special case: arguments/__arguments → IndexAccess with SliceIndex
        if is_arguments {
            return IrExpr::IndexAccess {
                object,
                index: key,
                index_kind: IndexKind::SliceIndex,
            };
        }
        let key_kind = match &obj_type {
            Some(ZigType::Anytype) | Some(ZigType::JsAny) => ComputedKeyKind::JsAnyGetByKey,
            Some(ZigType::NamedStruct(name)) if name == "Map" => ComputedKeyKind::MapGet,
            Some(ZigType::ArrayList(_)) => ComputedKeyKind::ArrayListItem,
            Some(ZigType::Str) => ComputedKeyKind::StringCharAt,
            Some(ZigType::Struct(f)) if f.is_empty() => ComputedKeyKind::ObjectMapGet,
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
    ///
    /// R24-INF-2: When the resolution lands on `ZigType::Anytype` (i.e., the
    /// identifier is an untyped function parameter), the result is converted
    /// to `None` to mirror the analysis pass (infer/expr.rs:60), which treats
    /// Anytype as Indeterminate. This prevents Anytype from leaking into
    /// array literals' element types, function return-type inference, and
    /// downstream annotations — all of which would produce invalid Zig
    /// (anytype is not a valid ArrayList element type or a property type).
    /// Returning None also lets decl.rs's `.or_else(var_types.get(name))`
    /// fallback surface a more specific JSDoc-annotated type when available.
    pub(crate) fn infer_ident_type(&self, name: &str) -> Option<ZigType> {
        // Special globals
        match name {
            "Infinity" | "NaN" => return Some(ZigType::F64),
            "undefined" => return Some(ZigType::JsAny),
            // arguments is lowered to __arguments: []const JsAny
            // Use ArrayList(JsAny) as the closest ZigType approximation for inference.
            "arguments" => return Some(ZigType::ArrayList(Box::new(ZigType::JsAny))),
            _ => {}
        }
        // Check fn_local_types first (takes priority over global var_types).
        // This prevents cross-function name collisions where a local variable
        // in one function shadows a global variable of the same name but
        // different type.
        // (Anytype params are already excluded from fn_local_types at insertion
        // time — see class.rs:475 and decl.rs:190 — so no extra filter needed.)
        if let Some(ty) = self
            .fn_ctx
            .as_ref()
            .and_then(|ctx| ctx.fn_local_types.get(name))
        {
            return Some(ty.clone());
        }
        // Collect candidate from var_types — exact, qualified.
        // Each step is allowed to override an earlier Anytype result so a
        // more specific qualified entry (e.g., "MyClass::x") can still win.
        let mut candidate: Option<ZigType> = None;
        // Exact match
        if let Some(ty) = self.type_info.var_types.get(name) {
            candidate = Some(ty.clone());
        }
        // Qualified match (fn_name::var_name)
        if matches!(candidate.as_ref(), None | Some(ZigType::Anytype))
            && let Some(ctx) = self.fn_ctx.as_ref()
        {
            let qualified = format!("{}::{}", ctx.name, name);
            if let Some(ty) = self.type_info.var_types.get(&qualified) {
                candidate = Some(ty.clone());
            }
        }
        // Filter: Anytype is indeterminate — return None to surface the
        // var_types.get(name) fallback in decl.rs's `.or_else` chain.
        match candidate {
            Some(ZigType::Anytype) => None,
            Some(other) => Some(other),
            None => None,
        }
    }

    /// Infer the ZigType of an expression based on type_info and expression structure.
    /// Enhanced version that covers literal types, member access, calls, and more.
    pub(crate) fn infer_expr_type(&self, expr: &Expression) -> Option<ZigType> {
        match expr {
            Expression::Identifier(id) => self.infer_ident_type(id.name.as_str()),
            Expression::NumericLiteral(nl) => {
                // Shared logic (P2-2): value-based detection via
                // crate::types::numeric_literal_type.
                Some(crate::types::numeric_literal_type(nl.value))
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
                UnaryOperator::UnaryNegation => {
                    // -0.0 must stay F64 to preserve IEEE 754 signed zero:
                    // Zig's @as(i64, @intFromFloat(-0.0)) produces 0, losing
                    // the sign. Match on the underlying NumericLiteral value
                    // (covers both `0.0` and `-0.0` since they compare equal).
                    if let Expression::NumericLiteral(nl) = &ue.argument
                        && nl.value == 0.0
                    {
                        return Some(ZigType::F64);
                    }
                    // INF-5: Mirror UnaryPlus for ToNumber conversion
                    // (bool→I64, str→F64). Previously returned arg type
                    // unchanged, causing type mismatches with the analysis pass.
                    match self.infer_expr_type(&ue.argument) {
                        Some(ZigType::Bool) => Some(ZigType::I64),
                        Some(ZigType::Str) | Some(ZigType::JsAny) => Some(ZigType::F64),
                        other => other,
                    }
                }
                UnaryOperator::UnaryPlus => match self.infer_expr_type(&ue.argument) {
                    // ToNumber conversion: bool → i64, str → f64, numbers pass through.
                    // R24-INF-1: JsAny also converts to F64 to match the analysis
                    // pass (infer/expr.rs). Without this case, `+jsanyVar` would
                    // fall through to `other => other` and return Some(JsAny),
                    // blocking the var_types fallback in decl.rs (Bug A pattern)
                    // and losing JSDoc-annotated F64/I64 types.
                    Some(ZigType::Bool) => Some(ZigType::I64),
                    Some(ZigType::Str) | Some(ZigType::JsAny) => Some(ZigType::F64),
                    other => other,
                },
                UnaryOperator::BitwiseNot => {
                    // JS: ~x converts to Int32 then bit-negates; we store as
                    // I64. A BigInt operand stays BigInt.
                    match self.infer_expr_type(&ue.argument) {
                        Some(ZigType::BigInt) => Some(ZigType::BigInt),
                        _ => Some(ZigType::I64),
                    }
                }
                UnaryOperator::Delete => Some(ZigType::Bool),
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
                    // Both branches agree → that type
                    (Some(a), Some(b)) if a == b => Some(a),
                    // Numeric promotion only: I64 ↔ F64 → F64. Mirrors
                    // infer_binary_result_type's I64+F64 rule. Other mixed
                    // combinations (e.g. Str ? F64) must NOT promote:
                    // the Str branch cannot be coerced to F64, so returning
                    // F64 here would cause downstream emit to wrap the
                    // string-branch value in @floatFromInt, which is invalid
                    // Zig. Fall through to None (JsAny fallback).
                    (Some(ZigType::F64), Some(ZigType::I64))
                    | (Some(ZigType::I64), Some(ZigType::F64)) => Some(ZigType::F64),
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
                        "Symbol" => {
                            // INF-7: Well-known symbols (Symbol.iterator, etc.)
                            // Mirrors analysis pass (infer/expr.rs:332-344).
                            // Without this, Symbol.xxx fell through to _ => {},
                            // returning None and potentially blocking the
                            // var_types fallback for symbol-keyed computed
                            // property access like obj[Symbol.iterator].
                            return match mem.property.name.as_str() {
                                "iterator" | "asyncIterator" | "hasInstance"
                                | "isConcatSpreadable" | "species" | "toPrimitive"
                                | "toStringTag" | "unscopables" | "match" | "matchAll"
                                | "replace" | "search" | "split" | "dispose" => {
                                    Some(ZigType::JsSymbol)
                                }
                                _ => None,
                            };
                        }
                        _ => {}
                    }
                }
                // Try struct field inference
                let obj_ty = self.infer_expr_type(&mem.object);
                match obj_ty {
                    Some(ZigType::Str) => {
                        if mem.property.name.as_str() == "length" {
                            return Some(ZigType::I64);
                        }
                        None
                    }
                    Some(ZigType::JsError) => match mem.property.name.as_str() {
                        "name" | "message" | "stack" => Some(ZigType::Str),
                        _ => None,
                    },
                    Some(ZigType::JsSymbol) => {
                        if mem.property.name.as_str() == "description" {
                            Some(ZigType::Str)
                        } else {
                            None
                        }
                    }
                    Some(ZigType::ArrayList(_)) => {
                        if mem.property.name.as_str() == "length" {
                            return Some(ZigType::I64);
                        }
                        None
                    }
                    Some(ZigType::Struct(fields)) => {
                        // Anonymous struct literal field access:
                        // {name: "x", age: 42}.name → Str
                        let field_name = mem.property.name.as_str();
                        for (name, ty) in &fields {
                            if name == field_name {
                                return Some(ty.clone());
                            }
                        }
                        None
                    }
                    Some(ZigType::NamedStruct(name)) => {
                        // TypedArray properties — must match lower_static_member's
                        // FieldKind::TypedArrayProp routing. Runtime returns
                        // i64 for byteLength/byteOffset, []const u8 for buffer
                        // (js_typedarray.zig).
                        if Self::is_typedarray_type(&name) {
                            match mem.property.name.as_str() {
                                "byteLength" | "byteOffset" => return Some(ZigType::I64),
                                "buffer" => return Some(ZigType::Str),
                                _ => {}
                            }
                        }
                        // Map/Set .size
                        if (name == "Map" || name == "Set") && mem.property.name.as_str() == "size"
                        {
                            return Some(ZigType::I64);
                        }
                        // RegExp .source/.flags
                        if name == "RegExp"
                            && matches!(mem.property.name.as_str(), "source" | "flags")
                        {
                            return Some(ZigType::Str);
                        }
                        // Host struct fields
                        if let Some(host_fields) = self.type_info.host_struct_fields.get(&name)
                            && let Some(ty) = host_fields.get(mem.property.name.as_str())
                        {
                            return Some(ty.clone());
                        }
                        // Class field types
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
                    // Number.* methods: let the builtin detection path
                    // (detect_builtin_call + builtin_return_type) handle
                    // them correctly. Previously "Number" returned I64 for
                    // ALL methods including isInteger/isNaN/isFinite (P0-8 fix).
                    // Method return type from object type
                    let obj_ty = self.infer_expr_type(&mem.object);
                    if let Some(ZigType::NamedStruct(name)) = &obj_ty {
                        match name.as_str() {
                            "Map" => match mem.property.name.as_str() {
                                "get" => return Some(ZigType::JsAny),
                                // `map.set(k, v)` returns the Map (receiver) for
                                // chaining, matching infer_named_method_return
                                // (infer/expr.rs:982). Without this, lowerer
                                // returned None, blocking the var_types fallback
                                // and emitting no type annotation for chained
                                // `map.set().set()` calls. (R24-INF-11)
                                "set" => return Some(ZigType::NamedStruct("Map".into())),
                                "has" | "delete" => return Some(ZigType::Bool),
                                _ => {}
                            },
                            "Set" => match mem.property.name.as_str() {
                                // `set.add(v)` returns the Set (receiver) for
                                // chaining, matching infer_named_method_return
                                // (infer/expr.rs:998). (R24-INF-11)
                                "add" => return Some(ZigType::NamedStruct("Set".into())),
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
                            | "padEnd" | "concat" | "at" => return Some(ZigType::Str),
                            // charCodeAt can return NaN → F64.
                            // codePointAt returns i64 (0 for out-of-bounds,
                            // matching runtime js_string.codePointAt).
                            "charCodeAt" => return Some(ZigType::F64),
                            "codePointAt" => return Some(ZigType::I64),
                            "indexOf" | "lastIndexOf" => return Some(ZigType::I64),
                            "includes" | "startsWith" | "endsWith" => return Some(ZigType::Bool),
                            // match/matchAll return JsAny (match array or null/iterator).
                            // Without this, .length on the result falls through to
                            // SliceLen, which emits .len on JsAny (compile error).
                            "match" | "matchAll" => return Some(ZigType::JsAny),
                            _ => {}
                        }
                    }
                    // Fallback: match/matchAll return JsAny even when the object
                    // type is unknown (e.g., untyped function parameter used as
                    // a string). Without this, .length on the result falls through
                    // to SliceLen, emitting .len on JsAny (compile error).
                    if matches!(mem.property.name.as_str(), "match" | "matchAll") {
                        return Some(ZigType::JsAny);
                    }
                }
                // Try function return type lookup
                if let Expression::Identifier(id) = &ce.callee
                    && let Some(ty) = self.type_info.fn_return_types.get(id.name.as_str())
                {
                    // Filter AnytypeReturn → None (mirrors analysis pass at
                    // expr.rs:255). AnytypeReturn cannot be propagated through
                    // call boundaries; returning None allows decl.rs var_types
                    // fallback to JSDoc-annotated types.
                    if *ty != ZigType::AnytypeReturn {
                        return Some(ty.clone());
                    }
                }
                // INF-4: Try qualified return type for class methods.
                // fn_return_types is keyed by "ClassName.methodName" for
                // class methods (passes.rs:715). Mirrors the analysis pass
                // qualified lookup. Without this, `myClass.method()` calls
                // returned None, blocking the var_types fallback.
                if let Expression::StaticMemberExpression(mem) = &ce.callee
                    && let Expression::Identifier(id) = &mem.object
                {
                    let qualified_key = format!("{}.{}", id.name, mem.property.name);
                    if let Some(ty) = self.type_info.fn_return_types.get(qualified_key.as_str())
                        && *ty != ZigType::AnytypeReturn
                    {
                        return Some(ty.clone());
                    }
                }
                // Try host function return type (mirrors analysis pass at expr.rs:260)
                if let Expression::Identifier(id) = &ce.callee
                    && let Some(ty) = self.type_info.host_return_types.get(id.name.as_str())
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
            // Object literal → infer as Struct with field types.
            // PropertyKind handling mirrors the analysis pass
            // (infer/expr.rs:758-787) and the lowerer's lower_object_expr
            // (container.rs:62-124). (R24-INF-13)
            Expression::ObjectExpression(oe) => {
                // R29-INF-2: Handle SpreadProperty by merging struct fields
                // from spread sources. Non-Struct spread sources yield None
                // (unknown fields) so decl.rs can fall back to var_types.
                // Mirrors the analysis pass (infer/expr.rs:794-851).
                // Without this, spreads were silently skipped, producing
                // partial struct types that omitted spread-contributed fields.
                let mut fields: Vec<(String, ZigType)> = Vec::new();
                for prop in &oe.properties {
                    match prop {
                        ObjectPropertyKind::SpreadProperty(sp) => {
                            match self.infer_expr_type(&sp.argument) {
                                Some(ZigType::Struct(spread_fields)) => {
                                    for (name, ty) in spread_fields {
                                        fields.retain(|(n, _)| n != &name);
                                        fields.push((name, ty));
                                    }
                                }
                                _ => return None,
                            }
                        }
                        ObjectPropertyKind::ObjectProperty(op) => {
                            let field_name = match &op.key {
                                PropertyKey::StaticIdentifier(id) => id.name.as_str().to_string(),
                                PropertyKey::StringLiteral(s) => s.value.to_string(),
                                _ => continue,
                            };
                            match op.kind {
                                PropertyKind::Init => {
                                    let field_ty =
                                        self.infer_expr_type(&op.value).unwrap_or(ZigType::JsAny);
                                    // Inline property overrides any spread field with same name
                                    fields.retain(|(n, _)| n != &field_name);
                                    fields.push((field_name, field_ty));
                                }
                                PropertyKind::Get => {
                                    // Getter: try to infer the return type from the
                                    // single-return function body (mirrors
                                    // container.rs:71-91 which inlines getters whose
                                    // body is a single eturn ...;). Complex
                                    // getters fall back to JsAny; the IR layer
                                    // emits @compileError for those so the runtime
                                    // type is irrelevant.
                                    let field_ty = if let Expression::FunctionExpression(func) =
                                        &op.value
                                        && let Some(body) = &func.body
                                        && body.statements.len() == 1
                                        && let Statement::ReturnStatement(ret) = &body.statements[0]
                                        && let Some(return_expr) = &ret.argument
                                    {
                                        self.infer_expr_type(return_expr).unwrap_or(ZigType::JsAny)
                                    } else {
                                        ZigType::JsAny
                                    };
                                    fields.retain(|(n, _)| n != &field_name);
                                    fields.push((field_name, field_ty));
                                }
                                PropertyKind::Set => {
                                    // Setter: doesn't contribute a field (mirrors
                                    // the analysis pass → infer/expr.rs:843-845).
                                }
                            }
                        }
                    }
                }
                Some(ZigType::Struct(fields))
            }
            // Array literal → ArrayList of element type.
            // Walk ALL elements and unify their types.
            //
            // IMPORTANT: this must agree with `emit_array_literal`'s element
            // type decision. The emitter uses a conservative `all_same` check
            // — any mismatch (e.g. IntLiteral + FloatLiteral) forces the
            // whole array to JsAny. So we mirror that here: any mismatch
            // degrades to JsAny. Previously this branch used `find_map` which
            // returned the FIRST element's type — so for `[1, 2.5]` the
            // lowerer inferred `ArrayList(I64)` while emit produced
            // `ArrayList(JsAny)`, an inconsistency that risked downstream
            // type-annotation mismatches.
            //
            // Spread elements (`...arr`) are skipped here for the element-walking
            // loop, but detected up-front and forced to JsAny to match
            // emit_array_literal (emit/expr/container.rs:16 checks
            // `arr.spread_indices.is_empty()`). Without the up-front check
            // `[1, 2, ...x]` would infer `ArrayList(I64)`, mismatching emit's
            // `ArrayList(JsAny)`. (R24-INF-9)
            Expression::ArrayExpression(ae) => {
                if ae
                    .elements
                    .iter()
                    .any(|e| matches!(e, ArrayExpressionElement::SpreadElement(_)))
                {
                    return Some(ZigType::ArrayList(Box::new(ZigType::JsAny)));
                }
                let mut elem_ty: Option<ZigType> = None;
                for el in &ae.elements {
                    let Some(e) = el.as_expression() else {
                        continue;
                    };
                    let t = self.infer_expr_type(e).unwrap_or(ZigType::JsAny);
                    elem_ty = Some(match elem_ty {
                        None => t,
                        Some(a) if a == t => a,
                        // Any mismatch degrades to JsAny — matches emit
                        // (all_same=false → JsAny). No numeric promotion
                        // here, to stay strictly aligned with the emitter.
                        Some(_) => ZigType::JsAny,
                    });
                    if elem_ty == Some(ZigType::JsAny) {
                        break;
                    }
                }
                Some(ZigType::ArrayList(Box::new(
                    elem_ty.unwrap_or(ZigType::JsAny),
                )))
            }
            // Computed member access: obj[key]
            // Mirrors the analysis pass (infer/expr.rs:442-489). Without the
            // JsAny / Map / Str / Struct / NamedStruct cases, those forms
            // returned None, blocking the var_types fallback in decl.rs (Bug A
            // pattern). (R24-INF-5)
            Expression::ComputedMemberExpression(cme) => {
                // arguments[i] → JsAny (since __arguments is []const JsAny)
                if let Expression::Identifier(id) = &cme.object
                    && (id.name.as_str() == "arguments" || id.name.as_str() == "__arguments")
                {
                    return Some(ZigType::JsAny);
                }
                match self.infer_expr_type(&cme.object) {
                    // ArrayList(T)[i] → T
                    Some(ZigType::ArrayList(elem)) => Some(*elem),
                    // JsAny[idx] → JsAny (dynamic, runtime-decided)
                    Some(ZigType::JsAny) => Some(ZigType::JsAny),
                    // Str[idx] → Str (single-character substring, like charAt)
                    Some(ZigType::Str) => Some(ZigType::Str),
                    // Struct["key"] → field type (anonymous struct literal)
                    Some(ZigType::Struct(fields)) => {
                        if let Expression::StringLiteral(s) = &cme.expression {
                            for (name, ty) in fields {
                                if name == s.value.as_str() {
                                    return Some(ty.clone());
                                }
                            }
                        }
                        None
                    }
                    // NamedStruct("Map")[key] → JsAny (computed access on Map
                    // behaves like map.get(key)). Other NamedStruct["key"]
                    // tries the host struct fields table.
                    Some(ZigType::NamedStruct(name)) => {
                        if name.as_str() == "Map" {
                            return Some(ZigType::JsAny);
                        }
                        if let Expression::StringLiteral(s) = &cme.expression {
                            // INF-1: Check both class_field_types and host_struct_fields
                            if let Some(class_fields) =
                                self.type_info.class_field_types.get(name.as_str())
                                && let Some(field_ty) = class_fields.get(s.value.as_str())
                            {
                                return Some(field_ty.clone());
                            }
                            if let Some(host_fields) =
                                self.type_info.host_struct_fields.get(name.as_str())
                                && let Some(field_ty) = host_fields.get(s.value.as_str())
                            {
                                return Some(field_ty.clone());
                            }
                        }
                        None
                    }
                    _ => None,
                }
            }
            Expression::LogicalExpression(le) => {
                // || and &&: result type depends on both operands.
                // If both sides have the same type, that's the result type.
                // If they differ, return JsAny (runtime decides which value
                // is returned). This matches the analysis pass behavior in
                // infer/expr.rs.
                let left_ty = self.infer_expr_type(&le.left);
                let right_ty = self.infer_expr_type(&le.right);
                match (left_ty, right_ty) {
                    (Some(l), Some(r)) if l == r => Some(l),
                    (Some(_), Some(_)) => Some(ZigType::JsAny),
                    (Some(_), None) | (None, Some(_)) => Some(ZigType::JsAny),
                    (None, None) => None,
                }
            }
            Expression::AssignmentExpression(ae) => {
                // Assignment result type depends on the operator.
                match ae.operator {
                    AssignmentOperator::Exponential => Some(ZigType::F64),
                    AssignmentOperator::Addition => {
                        // INF-5: str += x → Str (string concatenation).
                        // When the LHS is a string or the RHS is a string,
                        // the result is always Str per ECMA-262.
                        let lhs_is_str =
                            self.infer_assign_target_type(&ae.left) == Some(ZigType::Str);
                        let rhs_ty = self.infer_expr_type(&ae.right);
                        if lhs_is_str || rhs_ty == Some(ZigType::Str) {
                            Some(ZigType::Str)
                        } else {
                            rhs_ty
                        }
                    }
                    AssignmentOperator::LogicalAnd
                    | AssignmentOperator::LogicalOr
                    | AssignmentOperator::LogicalNullish => {
                        // For logical assignments (x &&= y, x ||= y, x ??= y),
                        // result is either the original LHS (short-circuit)
                        // or the RHS. Return LHS type as primary.
                        self.infer_assign_target_type(&ae.left)
                            .or_else(|| self.infer_expr_type(&ae.right))
                    }
                    _ => self.infer_expr_type(&ae.right),
                }
            }
            Expression::UpdateExpression(ue) => {
                // ++x / x-- returns the variable's type if it's I64,
                // otherwise defaults to F64 (matching analysis pass).
                match &ue.argument {
                    SimpleAssignmentTarget::AssignmentTargetIdentifier(id) => {
                        match self.get_var_type(id.name.as_str()) {
                            Some(ZigType::I64) => Some(ZigType::I64),
                            _ => Some(ZigType::F64),
                        }
                    }
                    _ => Some(ZigType::F64),
                }
            }
            Expression::SequenceExpression(se) => {
                // Type is that of the last expression.
                se.expressions.last().and_then(|e| self.infer_expr_type(e))
            }
            Expression::NewExpression(ne) => {
                // Infer constructor type for known built-in constructors.
                match &ne.callee {
                    Expression::Identifier(id) => match id.name.as_str() {
                        "Map" => Some(ZigType::NamedStruct("Map".to_string())),
                        "Set" => Some(ZigType::NamedStruct("Set".to_string())),
                        "Date" => Some(ZigType::NamedStruct("Date".to_string())),
                        "RegExp" => Some(ZigType::NamedStruct("RegExp".to_string())),
                        "DataView" | "ArrayBuffer" => {
                            Some(ZigType::NamedStruct(id.name.to_string()))
                        }
                        "Error" | "TypeError" | "RangeError" | "SyntaxError" | "ReferenceError" => {
                            Some(ZigType::JsError)
                        }
                        "BigInt" => Some(ZigType::BigInt),
                        // TypedArray constructors → NamedStruct with constructor
                        // name.  Matches the analysis pass (infer/expr.rs) and
                        // is_typedarray_type, so that decl.rs can register the
                        // correct per-function type in fn_local_types.
                        n if Self::is_typedarray_type(n) => {
                            Some(ZigType::NamedStruct(n.to_string()))
                        }
                        // Wrapper constructors: new Number(x), new String(x),
                        // new Boolean(x) always coerce to the wrapper's primitive
                        // type. Matches the analysis pass (infer/expr.rs) and
                        // builtin_return_type. The argument is coerced: e.g.
                        // new Number("hello") → NaN → F64 (NOT Str).
                        "String" => Some(ZigType::Str),
                        "Number" => Some(ZigType::F64),
                        "Boolean" => Some(ZigType::Bool),
                        // User-defined class names: return NamedStruct so that
                        // decl.rs's is_const check (class_names.contains)
                        // forces 'var' for method-call mutation + deinit.
                        // Mirrors the analysis pass (infer/expr.rs:217).
                        // Without this, the lowerer returns JsAny which
                        // (being Some) blocks the var_types fallback in
                        // decl.rs Bug A's init-expression-first path.
                        n if self.class_names.contains(n) => {
                            Some(ZigType::NamedStruct(n.to_string()))
                        }
                        // Unknown constructor: return None so that decl.rs's
                        // Bug A init-expression-first path falls through to
                        // var_types (JSDoc @type annotations). Returning
                        // Some(JsAny) would block that fallback.
                        _ => None,
                    },
                    _ => None,
                }
            }
            Expression::AwaitExpression(ae) => self.infer_expr_type(&ae.argument),
            // INF-6: Private field access (this.#field) - look up field
            // type from class_field_types. Mirrors the analysis pass
            // (infer/expr.rs:585-598). Without this, private field reads
            // fell through to _ => None, blocking the var_types fallback.
            Expression::PrivateFieldExpression(pfe) => self.infer_private_field_type(pfe),
            // RegExp literal (/pattern/) → NamedStruct("RegExp")
            // Matches analysis pass (infer/expr.rs:29). Enables
            // /pattern/.source and /pattern/.flags to infer Str via the
            // StaticMemberExpression NamedStruct("RegExp") arm.
            Expression::RegExpLiteral(_) => Some(ZigType::NamedStruct("RegExp".to_string())),
            Expression::ThisExpression(_) => self
                .current_class
                .as_ref()
                .map(|name| ZigType::NamedStruct(name.clone())),
            _ => None,
        }
    }

    /// Infer the result type of a binary operation from operand types.
    ///
    /// Delegates to `TypeInferrer::infer_binary_type` for the core type-mapping
    /// logic when both operand types are known (P2-2: eliminates ~70 lines of
    /// duplicated operator→type match arms). Falls back to conservative defaults
    /// when one or both types are unknown.
    pub(super) fn infer_binary_result_type(
        op: &BinaryOperator,
        left_ty: Option<ZigType>,
        right_ty: Option<ZigType>,
    ) -> Option<ZigType> {
        // Comparison operators (including `in` and `instanceof`) always
        // produce bool, even when operand types are unknown.
        if matches!(
            op,
            BinaryOperator::Equality
                | BinaryOperator::Inequality
                | BinaryOperator::StrictEquality
                | BinaryOperator::StrictInequality
                | BinaryOperator::LessThan
                | BinaryOperator::GreaterThan
                | BinaryOperator::LessEqualThan
                | BinaryOperator::GreaterEqualThan
                | BinaryOperator::In
                | BinaryOperator::Instanceof
        ) {
            return Some(ZigType::Bool);
        }

        // When both operand types are known, delegate to the shared core logic.
        // `infer_binary_type` must not take `&self` (it's a pure function of
        // operator + types), so we call it as an associated function.
        if let (Some(l), Some(r)) = (&left_ty, &right_ty) {
            let result = crate::infer::TypeInferrer::infer_binary_type(*op, l.clone(), r.clone());
            // JsAny is the catch-all "unknown" return from infer_binary_type
            // (for operators not explicitly handled); convert to None for the
            // Option-based API used by the lowerer.
            return (result != ZigType::JsAny).then_some(result);
        }

        // Partial knowledge: produce conservative defaults for operators with
        // predictable results regardless of the missing operand type.
        let both_bigint = left_ty == Some(ZigType::BigInt) && right_ty == Some(ZigType::BigInt);
        let either_bigint = left_ty == Some(ZigType::BigInt) || right_ty == Some(ZigType::BigInt);
        let either_str = left_ty == Some(ZigType::Str) || right_ty == Some(ZigType::Str);
        match op {
            // Addition: Str if either operand is Str (string concatenation).
            // If neither is Str but one operand's numeric type is known, use
            // it as the result (matching the old OR-pattern heuristic for
            // arrow functions: `x + 1` infers I64 when x is anytype).
            BinaryOperator::Addition => {
                if either_str {
                    Some(ZigType::Str)
                } else if either_bigint {
                    // Mixed BigInt + non-BigInt is a TypeError in JS; can't infer.
                    None
                } else if left_ty == Some(ZigType::F64) || right_ty == Some(ZigType::F64) {
                    Some(ZigType::F64)
                } else if left_ty == Some(ZigType::I64) || right_ty == Some(ZigType::I64) {
                    Some(ZigType::I64)
                } else {
                    None
                }
            }
            // Subtraction/Multiplication: BigInt-aware, partial knowledge.
            // INF-7: Previously fell through to `_ => None`, losing type info
            // for `var x = a - 1` where a is anytype (should infer I64).
            BinaryOperator::Subtraction | BinaryOperator::Multiplication => {
                if both_bigint {
                    Some(ZigType::BigInt)
                } else if either_bigint {
                    None
                } else if left_ty == Some(ZigType::F64) || right_ty == Some(ZigType::F64) {
                    Some(ZigType::F64)
                } else if left_ty == Some(ZigType::I64) || right_ty == Some(ZigType::I64) {
                    Some(ZigType::I64)
                } else {
                    None
                }
            }
            // Remainder/Division/Exponential: BigInt if both BigInt, F64 if
            // neither is BigInt. If exactly one is BigInt, the result depends
            // on the unknown operand — return None (conservative).
            BinaryOperator::Remainder | BinaryOperator::Division | BinaryOperator::Exponential => {
                if both_bigint {
                    Some(ZigType::BigInt)
                } else if either_bigint {
                    None
                } else {
                    Some(ZigType::F64)
                }
            }
            // Bitwise/Shift: BigInt if both BigInt, I64 if neither is BigInt.
            // If exactly one is BigInt, the result depends on the unknown
            // operand — return None (conservative).
            BinaryOperator::BitwiseAnd
            | BinaryOperator::BitwiseOR
            | BinaryOperator::BitwiseXOR
            | BinaryOperator::ShiftLeft
            | BinaryOperator::ShiftRight
            | BinaryOperator::ShiftRightZeroFill => {
                if both_bigint {
                    Some(ZigType::BigInt)
                } else if either_bigint {
                    None
                } else {
                    Some(ZigType::I64)
                }
            }
            _ => None,
        }
    }

    /// Infer the type of a simple assignment target (left-hand side of `++`/`--` etc.).
    /// Handles the same cases as `infer_assign_target_type` but for `SimpleAssignmentTarget`.
    pub(in crate::zigir::lower) fn infer_simple_assign_target_type(
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
            // R39-LEX-5: Handle ComputedMember and PrivateField so that
            // arr[i]++ on BigInt arrays correctly detects BigInt type.
            SimpleAssignmentTarget::ComputedMemberExpression(mem) => {
                self.infer_computed_member_type(mem)
            }
            SimpleAssignmentTarget::PrivateFieldExpression(pfe) => {
                self.infer_private_field_type(pfe)
            }
            _ => None,
        }
    }

    /// Infer the type of an assignment target (left-hand side of `=` / `+=` etc.).
    /// Only handles the common cases: identifier and static member expression.
    pub(in crate::zigir::lower) fn infer_assign_target_type(
        &self,
        target: &AssignmentTarget,
    ) -> Option<ZigType> {
        use oxc_ast::ast::AssignmentTarget;
        match target {
            AssignmentTarget::AssignmentTargetIdentifier(id) => {
                self.infer_ident_type(id.name.as_str())
            }
            AssignmentTarget::StaticMemberExpression(mem) => self.infer_static_member_type(mem),
            AssignmentTarget::ComputedMemberExpression(mem) => self.infer_computed_member_type(mem),
            AssignmentTarget::PrivateFieldExpression(pfe) => self.infer_private_field_type(pfe),
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
        // LOW-4: Check class_field_types before falling back to object type.
        // Without this, instance.jsanyField ??= value infers the instance
        // type (NamedStruct) instead of the field type (JsAny), causing ??=
        // no-op detection to incorrectly skip the isNullish check.
        let field_name = mem.property.name.as_str();
        let obj_type = self.infer_expr_type(&mem.object);
        if let Some(ZigType::NamedStruct(class_name)) = &obj_type
            && let Some(fields) = self.type_info.class_field_types.get(class_name)
            && let Some(ty) = fields.get(field_name)
        {
            return Some(ty.clone());
        }
        // Empty struct (JsObjectMap) properties are JsAny
        if let Some(ZigType::Struct(f)) = &obj_type
            && f.is_empty()
        {
            return Some(ZigType::JsAny);
        }
        obj_type
    }

    /// Infer the type of a computed member expression used as an assignment
    /// target (`obj[idx] = ...` or `obj[idx] += ...`).
    fn infer_computed_member_type(&self, mem: &ComputedMemberExpression) -> Option<ZigType> {
        // arguments[i] → JsAny
        if let Expression::Identifier(id) = &mem.object
            && (id.name.as_str() == "arguments" || id.name.as_str() == "__arguments")
        {
            return Some(ZigType::JsAny);
        }
        // ArrayList(T)[i] → T
        if let Some(ZigType::ArrayList(elem)) = self.infer_expr_type(&mem.object) {
            return Some(*elem);
        }
        // Empty struct (JsObjectMap) values are JsAny
        if let Some(ZigType::Struct(ref f)) = self.infer_expr_type(&mem.object)
            && f.is_empty()
        {
            return Some(ZigType::JsAny);
        }
        None
    }

    /// Infer the type of a private field expression used as an assignment
    /// target (`this.#field = ...` or `this.#field += ...`).
    fn infer_private_field_type(&self, pfe: &PrivateFieldExpression) -> Option<ZigType> {
        // Look up private field type in class_field_types (mirrors analysis pass expr.rs:585-598)
        if matches!(&pfe.object, Expression::ThisExpression(_))
            && let Some(ref class_name) = self.current_class
            && let Some(fields) = self.type_info.class_field_types.get(class_name)
            && let Some(ty) = fields.get(pfe.field.name.as_str())
        {
            return Some(ty.clone());
        }
        self.infer_expr_type(&pfe.object)
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
