use super::*;

impl<'a> ZigCodegen<'a> {
    /// Check if a ZigType is a JS object type (JsValue or JsAny) that needs method-call operators.
    fn is_js_obj_type(ty: &ZigType) -> bool {
        matches!(ty, ZigType::JsValue | ZigType::JsAny)
    }

    /// Get the type prefix for method calls ("JsValue" or "JsAny").
    fn js_type_prefix(ty: &ZigType) -> &'static str {
        match ty {
            ZigType::JsAny => "JsAny",
            _ => "JsValue",
        }
    }

    /// Escape a cooked template quasi value for embedding in a Zig string literal.
    /// Handles newlines, tabs, backslashes, and double quotes.
    fn escape_quasi_for_zig(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        for c in s.chars() {
            match c {
                '\\' => out.push_str("\\\\"),
                '"' => out.push_str("\\\""),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                _ => out.push(c),
            }
        }
        out
    }

    /// Wrap an expression in the appropriate JsValue/JsAny constructor based on its type.
    fn emit_wrap_js_value(&mut self, expr: &Expression, target_prefix: &str) {
        let expr_ty = self.inferrer.infer_expr(expr);
        match &expr_ty {
            ZigType::I64 | ZigType::I32 | ZigType::Usize => {
                self.push(target_prefix);
                self.push(".fromI64(@intCast(");
                self.emit_expr(expr);
                self.push("))");
            }
            ZigType::F64 | ZigType::F32 => {
                self.push(target_prefix);
                self.push(".fromF64(@floatCast(");
                self.emit_expr(expr);
                self.push("))");
            }
            ZigType::Bool => {
                self.push(target_prefix);
                self.push(".fromBool(");
                self.emit_expr(expr);
                self.push(")");
            }
            ZigType::String => {
                self.push(target_prefix);
                self.push(".fromString(");
                self.emit_expr(expr);
                self.push(")");
            }
            ZigType::Null => {
                self.push(target_prefix);
                self.push(".fromNull()");
            }
            ZigType::JsValue | ZigType::JsAny => {
                // Already a JS type, emit as-is
                self.emit_expr(expr);
            }
            _ => {
                // Unknown type, wrap as i64
                self.push(target_prefix);
                self.push(".fromI64(@intCast(");
                self.emit_expr(expr);
                self.push("))");
            }
        }
    }

    /// Unwrap a JsAny/JsValue expression to the specified primitive type.
    pub fn emit_unwrap_js_any(&mut self, expr: &Expression, target_ty: &ZigType) {
        match target_ty {
            ZigType::I64 | ZigType::I32 | ZigType::Usize => {
                self.emit_expr(expr);
                self.push(".asI64()");
            }
            ZigType::F64 | ZigType::F32 => {
                self.emit_expr(expr);
                self.push(".asF64()");
            }
            ZigType::Bool => {
                self.emit_expr(expr);
                self.push(".asBool()");
            }
            ZigType::String => {
                self.emit_expr(expr);
                self.push(".asString()");
            }
            _ => {
                // Fallback: emit as-is (may cause Zig type error if mismatched)
                self.emit_expr(expr);
            }
        }
    }

    /// Infer the type of a simple assignment target.
    fn infer_simple_target_type(&self, target: &SimpleAssignmentTarget) -> ZigType {
        match target {
            SimpleAssignmentTarget::AssignmentTargetIdentifier(id) => {
                self.inferrer.get_var_type(id.name.as_str())
            }
            SimpleAssignmentTarget::StaticMemberExpression(mem) => {
                // Check if it's a class field access (this.field or self.field)
                let is_this = matches!(&mem.object, Expression::ThisExpression(_))
                    || matches!(&mem.object, Expression::Identifier(id) if id.name.as_str() == "self");
                if is_this {
                    let field_name = mem.property.name.as_str();
                    if let Some((_, ref fields)) = self.current_class
                        && fields.iter().any(|f| f == field_name)
                    {
                        return ZigType::I64; // class fields are i64
                    }
                }
                ZigType::JsValue
            }
            _ => ZigType::JsValue,
        }
    }

    /// Infer the type of an assignment target expression.
    fn infer_assign_target_type(&self, target: &AssignmentTarget) -> ZigType {
        if let Some(simple) = target.as_simple_assignment_target() {
            match simple {
                SimpleAssignmentTarget::AssignmentTargetIdentifier(id) => {
                    self.inferrer.get_var_type(id.name.as_str())
                }
                SimpleAssignmentTarget::StaticMemberExpression(mem) => {
                    // Check if it's a class field access (this.field or self.field)
                    let is_this = matches!(&mem.object, Expression::ThisExpression(_))
                        || matches!(&mem.object, Expression::Identifier(id) if id.name.as_str() == "self");
                    if is_this {
                        let field_name = mem.property.name.as_str();
                        if let Some((_, ref fields)) = self.current_class
                            && fields.iter().any(|f| f == field_name)
                        {
                            return ZigType::I64; // class fields are i64
                        }
                    }
                    ZigType::JsValue
                }
                _ => ZigType::JsValue,
            }
        } else {
            ZigType::JsValue
        }
    }

    /// Emit a compound assignment (e.g., +=, -=) expanded to a method call for JsValue/JsAny.
    /// sum += i  →  sum = sum.add(JsValue.fromI64(i), alloc)
    fn emit_js_compound_assign(
        &mut self,
        left: &AssignmentTarget,
        right: &Expression,
        op: &AssignmentOperator,
        lhs_type: &ZigType,
    ) {
        let prefix = Self::js_type_prefix(lhs_type);
        // Emit: left = left.method(right_wrapped)
        self.emit_assign_target(left);
        self.push(" = ");
        self.emit_assign_target(left);
        match op {
            AssignmentOperator::Addition => {
                self.push(".add(");
                self.emit_wrap_js_value(right, prefix);
                self.push(", js_allocator.g_alloc())");
            }
            AssignmentOperator::Subtraction => {
                self.push(".sub(");
                self.emit_wrap_js_value(right, prefix);
                self.push(")");
            }
            AssignmentOperator::Multiplication => {
                self.push(".mul(");
                self.emit_wrap_js_value(right, prefix);
                self.push(")");
            }
            AssignmentOperator::Division => {
                self.push(".div(");
                self.emit_wrap_js_value(right, prefix);
                self.push(")");
            }
            AssignmentOperator::Remainder => {
                self.push(".rem(");
                self.emit_wrap_js_value(right, prefix);
                self.push(")");
            }
            _ => {
                // Fallback: raw compound assignment (bitwise ops)
                self.push(" ");
                self.push(self.map_assign_op(op));
                self.push(" ");
                self.emit_expr(right);
            }
        }
    }

    /// Emit a JS-style binary operation as a method call.
    /// For JsValue/JsAny typed operands, uses .add()/.sub()/.mul() etc.
    /// If left operand is NOT JsAny/JsValue, emit standard Zig operator instead.
    fn emit_js_binary_op(
        &mut self,
        left: &Expression,
        right: &Expression,
        op: &BinaryOperator,
        left_ty: &ZigType,
    ) {
        // If left operand is NOT JsAny/JsValue, emit standard Zig operator
        if !Self::is_js_obj_type(left_ty) {
            self.emit_expr(left);
            self.push(self.map_binary_op(op));
            let right_ty = self.inferrer.infer_expr(right);
            if Self::is_js_obj_type(&right_ty) {
                // Unwrap JsAny to the left type
                self.emit_unwrap_js_any(right, left_ty);
            } else {
                self.emit_expr(right);
            }
            return;
        }
        // Determine the wider type for method calls
        let right_ty = self.inferrer.infer_expr(right);
        let prefix = if self.current_callback_method.is_some() || matches!(left_ty, ZigType::JsAny) || matches!(&right_ty, ZigType::JsAny) {
            "JsAny"
        } else {
            "JsValue"
        };

        match op {
            // Arithmetic
            BinaryOperator::Addition => {
                self.emit_expr(left);
                self.push(".add(");
                self.emit_wrap_js_value(right, prefix);
                self.push(", js_allocator.g_alloc())");
            }
            BinaryOperator::Subtraction => {
                self.emit_expr(left);
                self.push(".sub(");
                self.emit_wrap_js_value(right, prefix);
                self.push(")");
            }
            BinaryOperator::Multiplication => {
                self.emit_expr(left);
                self.push(".mul(");
                self.emit_wrap_js_value(right, prefix);
                self.push(")");
            }
            BinaryOperator::Division => {
                self.emit_expr(left);
                self.push(".div(");
                self.emit_wrap_js_value(right, prefix);
                self.push(")");
            }
            BinaryOperator::Remainder => {
                self.emit_expr(left);
                self.push(".rem(");
                self.emit_wrap_js_value(right, prefix);
                self.push(")");
            }
            // Comparison — wrap right operand in same JS type
            // NOTE: JsAny.gt/lt/ge/le return bool, so in callback contexts (where return type is JsAny),
            // we need to wrap the result as JsAny.fromBool(...)
            BinaryOperator::LessThan => {
                if self.current_callback_method.is_some() {
                    self.push("JsAny.fromBool(");
                    self.emit_expr(left);
                    self.push(".lt(");
                    self.emit_wrap_js_value(right, prefix);
                    self.push("))");
                } else {
                    self.emit_expr(left);
                    self.push(".lt(");
                    self.emit_wrap_js_value(right, prefix);
                    self.push(")");
                }
            }
            BinaryOperator::LessEqualThan => {
                if self.current_callback_method.is_some() {
                    self.push("JsAny.fromBool(");
                    self.emit_expr(left);
                    self.push(".le(");
                    self.emit_wrap_js_value(right, prefix);
                    self.push("))");
                } else {
                    self.emit_expr(left);
                    self.push(".le(");
                    self.emit_wrap_js_value(right, prefix);
                    self.push(")");
                }
            }
            BinaryOperator::GreaterThan => {
                if self.current_callback_method.is_some() {
                    self.push("JsAny.fromBool(");
                    self.emit_expr(left);
                    self.push(".gt(");
                    self.emit_wrap_js_value(right, prefix);
                    self.push("))");
                } else {
                    self.emit_expr(left);
                    self.push(".gt(");
                    self.emit_wrap_js_value(right, prefix);
                    self.push(")");
                }
            }
            BinaryOperator::GreaterEqualThan => {
                if self.current_callback_method.is_some() {
                    self.push("JsAny.fromBool(");
                    self.emit_expr(left);
                    self.push(".ge(");
                    self.emit_wrap_js_value(right, prefix);
                    self.push("))");
                } else {
                    self.emit_expr(left);
                    self.push(".ge(");
                    self.emit_wrap_js_value(right, prefix);
                    self.push(")");
                }
            }
            BinaryOperator::Equality | BinaryOperator::StrictEquality => {
                let left_ty = self.inferrer.infer_expr(left);
                let right_ty = self.inferrer.infer_expr(right);
                // If both operands are numeric, use == instead of .eq()
                if left_ty.is_numeric() && right_ty.is_numeric() {
                    self.emit_expr(left);
                    self.push(" == ");
                    self.emit_expr(right);
                } else {
                    self.emit_expr(left);
                    self.push(".eq(");
                    self.emit_wrap_js_value(right, prefix);
                    self.push(")");
                }
            }
            BinaryOperator::Inequality | BinaryOperator::StrictInequality => {
                let left_ty = self.inferrer.infer_expr(left);
                let right_ty = self.inferrer.infer_expr(right);
                // If both operands are numeric, use != instead of .neq()
                if left_ty.is_numeric() && right_ty.is_numeric() {
                    self.emit_expr(left);
                    self.push(" != ");
                    self.emit_expr(right);
                } else {
                    self.emit_expr(left);
                    self.push(".neq(");
                    self.emit_wrap_js_value(right, prefix);
                    self.push(")");
                }
            }
            // Fallback: emit raw operator (bitwise, shift, instanceof, in)
            _ => {
                self.push("/* unsupported JS binary op for ");
                self.push(prefix);
                self.push(" */ ");
                self.emit_expr(left);
                self.push(" ");
                self.push(self.map_binary_op(op));
                self.push(" ");
                self.emit_expr(right);
            }
        }
    }
    pub(super) fn emit_expr(&mut self, expr: &Expression) {
        match expr {
            Expression::NumericLiteral(lit) => {
                // Use raw source text when available (preserves hex/base), fallback to value
                if let Some(raw) = &lit.raw {
                    self.push(raw);
                } else if lit.value.fract() == 0.0 {
                    self.push(&format!("{}", lit.value as i64));
                } else {
                    self.push(&format!("{}", lit.value));
                }
            }

            Expression::StringLiteral(lit) => {
                self.push("\"");
                // Escape the string value for Zig: \ -> \\, " -> \"
                let escaped = lit.value.replace('\\', "\\\\").replace('"', "\\\"");
                self.push(&escaped);
                self.push("\"");
            }

            Expression::BooleanLiteral(lit) => {
                self.push(if lit.value { "true" } else { "false" });
            }

            Expression::NullLiteral(_) => {
                self.push("null");
            }

            Expression::BigIntLiteral(lit) => {
                if let Some(raw) = &lit.raw {
                    self.push(raw);
                } else {
                    self.push(&lit.value);
                }
            }

            Expression::Identifier(id) => {
                // Built-in global constants
                match id.name.as_str() {
                    "NaN" => { self.push("std.math.nan(f64)"); return; }
                    "Infinity" => { self.push("std.math.inf(f64)"); return; }
                    "undefined" => { self.push("JsAny.undefined"); return; }
                    _ => {}
                }
                self.push(&Self::escape_keyword(id.name.as_str()));
            }

            Expression::ThisExpression(_) => {
                self.push("self");
            },

            Expression::BinaryExpression(bin) => {
                // Handle special operators that don't map 1:1 to Zig
                if bin.operator == BinaryOperator::Instanceof {
                    // JS `x instanceof Y` → Zig `@TypeOf(x) == Y`
                    self.push("(@TypeOf(");
                    self.emit_expr(&bin.left);
                    self.push(") == ");
                    self.emit_expr(&bin.right);
                    self.push(")");
                    return;
                }
                if bin.operator == BinaryOperator::In {
                    // JS `"key" in obj` → Zig `@hasField(@TypeOf(obj), key)`
                    // For dynamic access (HashMap): `obj.contains(key)`
                    let is_dynamic = if let Expression::Identifier(id) = &bin.right {
                        self.inferrer.get_dynamic_access_vars().contains(id.name.as_str())
                    } else {
                        false
                    };
                    if is_dynamic {
                        self.emit_expr(&bin.right);
                        self.push(".contains(");
                        self.emit_expr(&bin.left);
                        self.push(")");
                    } else {
                        self.push("@hasField(@TypeOf(");
                        self.emit_expr(&bin.right);
                        self.push("), ");
                        self.emit_expr(&bin.left);
                        self.push(")");
                    }
                    return;
                }
                if bin.operator == BinaryOperator::Exponential {
                    // JS `**` is exponentiation; Zig's `**` is array repetition
                    self.push("std.math.pow(f64, @floatFromInt(");
                    self.emit_expr(&bin.left);
                    self.push("), @floatFromInt(");
                    self.emit_expr(&bin.right);
                    self.push("))");
                    return;
                }
                if bin.operator == BinaryOperator::Addition {
                    let left_ty = self.inferrer.infer_expr(&bin.left);
                    let right_ty = self.inferrer.infer_expr(&bin.right);
                    if left_ty == ZigType::String || right_ty == ZigType::String {
                        // Check if both operands are string literals (comptime concat)
                        let left_is_lit = Self::is_string_literal_expr(&bin.left);
                        let right_is_lit = Self::is_string_literal_expr(&bin.right);
                        if left_is_lit && right_is_lit {
                            self.emit_expr(&bin.left);
                            self.push(" ++ ");
                            self.emit_expr(&bin.right);
                        } else {
                            // Runtime string concat: use allocPrint with page_allocator
                            // Produces: std.fmt.allocPrint(js_allocator.g_alloc(), "{s}{s}", .{a, b}) catch @panic("OOM")
                            self.push("std.fmt.allocPrint(js_allocator.g_alloc(), \"{s}{s}\", .{ ");
                            self.emit_expr(&bin.left);
                            self.push(", ");
                            self.emit_expr(&bin.right);
                            self.push(" }) catch @panic(\"OOM\")");
                        }
                        return;
                    }
                    // Check JsValue/JsAny after String check
                    // Only use method-call syntax if LEFT operand is JsAny/JsValue
                    if Self::is_js_obj_type(&left_ty) {
                        self.emit_js_binary_op(&bin.left, &bin.right, &bin.operator, &left_ty);
                        return;
                    }
                    // If left is primitive and right is JsAny, fall through to standard operator
                }
                // For non-Addition binary ops with JsValue/JsAny operands
                if bin.operator != BinaryOperator::Addition {
                    let left_ty = self.inferrer.infer_expr(&bin.left);
                    // Only use method-call syntax if LEFT operand is JsAny/JsValue
                    if Self::is_js_obj_type(&left_ty) {
                        self.emit_js_binary_op(&bin.left, &bin.right, &bin.operator, &left_ty);
                        return;
                    }
                }
                // Integer division and modulo require Zig builtins
                if bin.operator == BinaryOperator::Division || bin.operator == BinaryOperator::Remainder {
                    let left_ty = self.inferrer.infer_expr(&bin.left);
                    let right_ty = self.inferrer.infer_expr(&bin.right);
                    let left_is_int = matches!(left_ty, ZigType::I64 | ZigType::I32 | ZigType::Usize);
                    let right_is_int = matches!(right_ty, ZigType::I64 | ZigType::I32 | ZigType::Usize);
                    if left_is_int || right_is_int {
                        let builtin = if bin.operator == BinaryOperator::Division {
                            "@divTrunc"
                        } else {
                            "@rem"
                        };
                        self.push(builtin);
                        self.push("(");
                        self.emit_expr(&bin.left);
                        self.push(", ");
                        self.emit_expr(&bin.right);
                        self.push(")");
                        return;
                    }
                }
                // Standard operator emission (both operands are primitive, or mixed)
                let left_ty = self.inferrer.infer_expr(&bin.left);
                let right_ty = self.inferrer.infer_expr(&bin.right);
                self.emit_expr(&bin.left);
                self.push(" ");
                self.push(self.map_binary_op(&bin.operator));
                self.push(" ");
                // Handle JsAny unwrapping for binary ops with mixed types
                // e.g., sum (i64) + x (JsAny) → sum + x.asI64()
                if Self::is_js_obj_type(&right_ty) && !Self::is_js_obj_type(&left_ty) {
                    // Right is JsAny, left is primitive: unwrap right to left's type
                    self.emit_unwrap_js_any(&bin.right, &left_ty);
                } else {
                    // Zig shift amount must be unsigned: i64 << n requires n: u6
                    if bin.operator == BinaryOperator::ShiftLeft
                        || bin.operator == BinaryOperator::ShiftRight
                        || bin.operator == BinaryOperator::ShiftRightZeroFill
                    {
                        self.push("@as(u6, @intCast(");
                        self.emit_expr(&bin.right);
                        self.push("))");
                    } else {
                        self.emit_expr(&bin.right);
                    }
                }
            }

            Expression::LogicalExpression(logic) => {
                self.emit_expr(&logic.left);
                self.push(" ");
                self.push(self.map_logical_op(&logic.operator));
                self.push(" ");
                self.emit_expr(&logic.right);
            },

            Expression::UnaryExpression(unary) => {
                match unary.operator {
                    UnaryOperator::Typeof => {
                        // JS typeof returns a string; map inferred type to the JS convention
                        let arg_ty = self.inferrer.infer_expr(&unary.argument);
                        let type_str = match &arg_ty {
                            ZigType::I64 | ZigType::I32 | ZigType::Usize
                            | ZigType::F64 | ZigType::F32 => "\"number\"",
                            ZigType::String => "\"string\"",
                            ZigType::Bool => "\"boolean\"",
                            ZigType::Void => "\"undefined\"",
                            ZigType::Null => "\"object\"",
                            _ => "\"object\"",
                        };
                        self.push(type_str);
                    }
                    UnaryOperator::Void | UnaryOperator::Delete => {
                        self.emit_expr(&unary.argument);
                    }
                    UnaryOperator::UnaryPlus => {
                        // Zig has no unary plus — emit argument as-is
                        self.emit_expr(&unary.argument);
                    }
                    UnaryOperator::BitwiseNot => {
                        // ~ on comptime_int needs explicit type cast
                        self.push("~@as(i64, ");
                        self.emit_expr(&unary.argument);
                        self.push(")");
                    }
                    _ => {
                        self.push(self.map_unary_op(&unary.operator));
                        self.push(" ");
                        self.emit_expr(&unary.argument);
                    }
                }
            }

            Expression::UpdateExpression(update) => {
                // Check if argument is a JsValue/JsAny variable — expand to method call
                let arg_type = self.infer_simple_target_type(&update.argument);
                if Self::is_js_obj_type(&arg_type) {
                    let prefix = Self::js_type_prefix(&arg_type);
                    let method = match update.operator {
                        UpdateOperator::Increment => "add",
                        UpdateOperator::Decrement => "sub",
                    };
                    self.emit_assign_target_from_simple(&update.argument);
                    self.push(" = ");
                    self.emit_assign_target_from_simple(&update.argument);
                    if update.operator == UpdateOperator::Increment {
                        self.push(&format!(".{}({}.fromI64(1), js_allocator.g_alloc())", method, prefix));
                    } else {
                        self.push(&format!(".{}({}.fromI64(1))", method, prefix));
                    }
                } else {
                    let op = match update.operator {
                        UpdateOperator::Increment => "+=",
                        UpdateOperator::Decrement => "-=",
                    };
                    self.emit_assign_target_from_simple(&update.argument);
                    self.push(" ");
                    self.push(op);
                    self.push(" 1");
                }
            }

            Expression::CallExpression(call) => {
                // Check builtin registry for known method/global calls
                if self.try_emit_builtin_call(call) {
                    // handled by builtin registry
                } else {
                    // Check if callee is a closure variable → emit __cl_callee.call(args)
                    let maybe_cl_name =
                        if let Expression::Identifier(callee_id) = &call.callee {
                            let cl_name = format!("__cl_{}", callee_id.name.as_str());
                            if self.closure_vars.contains(&cl_name) {
                                Some(cl_name)
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                    if let Some(cl_name) = maybe_cl_name {
                        self.push(&cl_name);
                        self.push(".call(");
                    } else {
                        // Default: emit callee(args)
                        self.emit_expr(&call.callee);
                        self.push("(");
                    }
                    for (i, arg) in call.arguments.iter().enumerate() {
                        if i > 0 {
                            self.push(", ");
                        }
                        self.emit_arg(arg);
                    }
                    self.push(")");
                }
            }

            Expression::NewExpression(ne) => {
                // Check for built-in constructors (Map, Set, Error, etc.)
                if let Expression::Identifier(id) = &ne.callee {
                    match id.name.as_str() {
                        "Map" => {
                            self.push("js_map.JsMap.init(js_allocator.g_alloc())");
                            return;
                        }
                        "Set" => {
                            self.push("js_set.JsSet.init(js_allocator.g_alloc())");
                            return;
                        }
                        "Error" | "TypeError" | "RangeError" | "ReferenceError" | "SyntaxError" => {
                            // new Error(msg) → js_error.JsError.init(js_allocator.g_alloc(), msg) catch @panic("OOM")
                            self.push("js_error.JsError.init(js_allocator.g_alloc(), ");
                            if let Some(arg) = ne.arguments.first() {
                                self.emit_arg(arg);
                            } else {
                                self.push("\"\"");
                            }
                            self.push(") catch @panic(\"OOM\")");
                            return;
                        }
                        // ── TypedArray constructors ──
                        // new Int32Array([1,2,3]) → js_runtime.js_typedarray.fromI32(js_allocator.g_alloc(), &[_]i32{1,2,3}) catch &[_]i32{}
                        "Int8Array" => {
                            self.push("js_runtime.js_typedarray.fromI64AsI8(js_allocator.g_alloc(), ");
                            if let Some(arg) = ne.arguments.first() {
                                self.emit_arg(arg);
                            } else {
                                self.push("&[_]i8{}");
                            }
                            self.push(") catch js_runtime.js_typedarray.emptyI8()");
                            return;
                        }
                        "Uint8Array" | "Uint8ClampedArray" => {
                            self.push("js_runtime.js_typedarray.fromI64AsU8(js_allocator.g_alloc(), ");
                            if let Some(arg) = ne.arguments.first() {
                                self.emit_arg(arg);
                            } else {
                                self.push("&[_]u8{}");
                            }
                            self.push(") catch js_runtime.js_typedarray.emptyU8()");
                            return;
                        }
                        "Int16Array" => {
                            self.push("js_runtime.js_typedarray.fromI64AsI16(js_allocator.g_alloc(), ");
                            if let Some(arg) = ne.arguments.first() {
                                self.emit_arg(arg);
                            } else {
                                self.push("&[_]i16{}");
                            }
                            self.push(") catch js_runtime.js_typedarray.emptyI16()");
                            return;
                        }
                        "Uint16Array" => {
                            self.push("js_runtime.js_typedarray.fromI64AsU16(js_allocator.g_alloc(), ");
                            if let Some(arg) = ne.arguments.first() {
                                self.emit_arg(arg);
                            } else {
                                self.push("&[_]u16{}");
                            }
                            self.push(") catch js_runtime.js_typedarray.emptyU16()");
                            return;
                        }
                        "Int32Array" => {
                            self.push("js_runtime.js_typedarray.fromI64AsI32(js_allocator.g_alloc(), ");
                            if let Some(arg) = ne.arguments.first() {
                                self.emit_arg(arg);
                            } else {
                                self.push("&[_]i32{}");
                            }
                            self.push(") catch js_runtime.js_typedarray.emptyI32()");
                            return;
                        }
                        "Uint32Array" => {
                            self.push("js_runtime.js_typedarray.fromI64AsU32(js_allocator.g_alloc(), ");
                            if let Some(arg) = ne.arguments.first() {
                                self.emit_arg(arg);
                            } else {
                                self.push("&[_]u32{}");
                            }
                            self.push(") catch js_runtime.js_typedarray.emptyU32()");
                            return;
                        }
                        "Float32Array" => {
                            self.push("js_runtime.js_typedarray.fromF64AsF32(js_allocator.g_alloc(), ");
                            if let Some(arg) = ne.arguments.first() {
                                self.emit_arg(arg);
                            } else {
                                self.push("&[_]f32{}");
                            }
                            self.push(") catch js_runtime.js_typedarray.emptyF32()");
                            return;
                        }
                        "Float64Array" => {
                            self.push("js_runtime.js_typedarray.fromF64(js_allocator.g_alloc(), ");
                            if let Some(arg) = ne.arguments.first() {
                                self.emit_arg(arg);
                            } else {
                                self.push("&[_]f64{}");
                            }
                            self.push(") catch js_runtime.js_typedarray.emptyF64()");
                            return;
                        }
                        _ => {}
                    }
                }
                
                // Default: new ClassName(args) → ClassName.init(args)
                self.emit_expr(&ne.callee);
                self.push(".init(");
                for (i, arg) in ne.arguments.iter().enumerate() {
                    if i > 0 {
                        self.push(", ");
                    }
                    self.emit_arg(arg);
                }
                self.push(")");
            }

            Expression::StaticMemberExpression(mem) => {
                // Check if object is a dynamic access variable (uses HashMap)
                let is_dynamic = if let Expression::Identifier(id) = &mem.object {
                    self.inferrer.get_dynamic_access_vars().contains(id.name.as_str())
                } else {
                    false
                };

                if is_dynamic {
                    // Look up field type from the original object type to pick
                    // the correct JsValue variant accessor.
                    self.emit_expr(&mem.object);
                    self.push(".get(\"");
                    self.push(mem.property.name.as_str());
                    self.push("\").?");
                    let accessor = self.dynamic_field_accessor(&mem.object, mem.property.name.as_str());
                    self.push(&accessor);
                    return;
                }
                
                // Map/Set .size property → .size() method call
                if mem.property.name.as_str() == "size" {
                    let obj_ty = self.inferrer.infer_expr(&mem.object);
                    if let ZigType::Struct(ref s) = obj_ty
                        && (s == "Map" || s == "Set")
                    {
                        self.emit_expr(&mem.object);
                        self.push(".size()");
                        return;
                    }
                }

                // Map JS .length to Zig .len / .items.len for arrays and strings
                if mem.property.name.as_str() == "length" {
                    let obj_ty = self.inferrer.infer_expr(&mem.object);

                    // Slice or String: use .len (Zig slice / []const u8)
                    if matches!(obj_ty, ZigType::Slice(_) | ZigType::String) {
                        self.push("@as(i64, @intCast(");
                        self.emit_expr(&mem.object);
                        self.push(".len))");
                        return;
                    }

                    // ArrayList (JsAny or Array + dynamic_array): use .items.len
                    if matches!(obj_ty, ZigType::JsAny | ZigType::Array(_))
                        && let Expression::Identifier(id) = &mem.object
                        && self.inferrer.is_dynamic_array(id.name.as_str())
                    {
                        self.push("@as(i64, @intCast(");
                        self.emit_expr(&mem.object);
                        self.push(".items.len))");
                        return;
                    }

                    // Fixed-size array or other: use .len as fallback
                    self.push("@as(i64, @intCast(");
                    self.emit_expr(&mem.object);
                    self.push(".len))");
                    return;
                }

                // Check builtin static properties (e.g., Math.PI → std.math.pi)
                if let Expression::Identifier(id) = &mem.object
                    && let Some(zig_expr) = self.builtins.lookup_property(id.name.as_str(), mem.property.name.as_str()) {
                        self.push(zig_expr);
                        return;
                    }

                self.emit_expr(&mem.object);
                self.push(".");
                let prop = mem.property.name.as_str();
                if prop == "catch" || prop == "async" || prop == "await" {
                    self.push("@\"");
                    self.push(prop);
                    self.push("\"");
                } else {
                    self.push(prop);
                }
            }

            Expression::ComputedMemberExpression(mem) => {
                // Check if object is a dynamic array (ArrayList)
                // Distinguish: function params with slice type use direct indexing;
                // locally-declared dynamic arrays use .items[...]
                // dynamic_arrays is file-global, so a TypedArray parameter named "arr"
                // could be incorrectly flagged as dynamic — check is_fn_param_of.
                if let Expression::Identifier(id) = &mem.object
                    && self.inferrer.is_dynamic_array(id.name.as_str())
                {
                    // Check if this variable is a parameter of the CURRENT function.
                    // If yes, it's a slice — use direct indexing.
                    // If no, it's a locally-declared ArrayList — use .items[...].
                    let is_current_fn_param = self.current_fn.as_ref()
                        .map(|fn_name| self.inferrer.is_fn_param_of(fn_name, id.name.as_str()))
                        .unwrap_or(false);
                    if is_current_fn_param {
                        self.emit_expr(&mem.object);
                        self.push("[");
                        self.emit_expr(&mem.expression);
                        self.push("]");
                        return;
                    }
                    // Locally-declared ArrayList - use .items[...]
                    self.emit_expr(&mem.object);
                    self.push(".items[");
                    self.emit_expr(&mem.expression);
                    self.push("]");
                    return;
                }

                // Check if object is a dynamic access variable (uses HashMap)
                let is_dynamic = if let Expression::Identifier(id) = &mem.object {
                    self.inferrer.get_dynamic_access_vars().contains(id.name.as_str())
                } else {
                    false
                };

                if is_dynamic {
                    // Generate: object.get(key).?
                    self.emit_expr(&mem.object);
                    self.push(".get(");
                    self.emit_expr(&mem.expression);
                    self.push(").?");
                    return;
                }

                // Check if object is a struct type and key is a string literal
                let obj_type = self.inferrer.infer_expr(&mem.object);
                if matches!(&obj_type, ZigType::Object { .. })
                    && let Expression::StringLiteral(s) = &mem.expression
                {
                    // String literal key → direct field access
                    let field = s.value.to_string();  // String -> String (owned)
                    let field_str: &str = &field;
                    self.emit_expr(&mem.object);
                    self.push(".");
                    if field_str == "catch" || field_str == "async" || field_str == "await" {
                        self.push("@\"");
                        self.push(field_str);
                        self.push("\"");
                    } else {
                        self.push(field_str);
                    }
                    return;
                }

                // Check if object is an array type (ZigType::Array) - use direct indexing for slices
                if matches!(&obj_type, ZigType::Array(_)) {
                    self.emit_expr(&mem.object);
                    self.push("[");
                    self.emit_expr(&mem.expression);
                    self.push("]");
                    return;
                }

                // Fall through: string indexing or other cases
                self.emit_expr(&mem.object);
                self.push("[");
                self.emit_expr(&mem.expression);
                self.push("]");
            }

            Expression::PrivateFieldExpression(_) => {
                self.push("@compileError(\"private field access is not supported\")");
            }

            Expression::AssignmentExpression(assign) => {
                // Check if assigning to a field of a dynamic access object (HashMap)
                if let AssignmentTarget::StaticMemberExpression(mem) = &assign.left
                    && let Expression::Identifier(obj_id) = &mem.object
                {
                    let obj_name = obj_id.name.as_str();
                    let dyn_vars = self.inferrer.get_dynamic_access_vars();
                    if dyn_vars.contains(obj_name) {
                        // Generate: obj.put("field", JsValue{...}) catch @panic("OOM");
                        self.emit_expr(&mem.object);
                        self.push(".put(\"");
                        self.push(mem.property.name.as_str());
                        self.push("\", ");
                        self.emit_js_value_construction(&assign.right);
                        self.push(") catch @panic(\"OOM\")");
                        return;
                    }
                }

                // For JsValue/JsAny targets, expand compound assignments to method calls
                let lhs_type = self.infer_assign_target_type(&assign.left);
                if Self::is_js_obj_type(&lhs_type) && assign.operator != AssignmentOperator::Assign {
                    self.emit_js_compound_assign(&assign.left, &assign.right, &assign.operator, &lhs_type);
                    return;
                }

                self.emit_assign_target(&assign.left);
                self.push(" ");
                self.push(self.map_assign_op(&assign.operator));
                self.push(" ");
                // For simple = with JsValue/JsAny target, wrap RHS
                if Self::is_js_obj_type(&lhs_type) && assign.operator == AssignmentOperator::Assign {
                    let prefix = Self::js_type_prefix(&lhs_type);
                    self.emit_wrap_js_value(&assign.right, prefix);
                } else {
                    self.emit_expr(&assign.right);
                }
            }

            Expression::ConditionalExpression(cond) => {
                self.push("if (");
                self.emit_expr(&cond.test);
                self.push(") ");
                self.emit_expr(&cond.consequent);
                self.push(" else ");
                self.emit_expr(&cond.alternate);
            }

            Expression::ArrayExpression(arr) => {
                let elem_type = if arr.elements.is_empty() {
                    ZigType::I64  // empty array → default to i64
                } else {
                    arr.elements.iter().find_map(|elem| match elem {
                        ArrayExpressionElement::SpreadElement(_) => None,
                        ArrayExpressionElement::Elision(_) => None,
                        _ => elem.as_expression().map(|e| self.inferrer.infer_expr(e)),
                    }).unwrap_or(ZigType::JsValue)  // inference failed → Zig compile error
                };
                // If inference failed, elem_type = Any → "JsValue" → Zig compile error
                self.push(&format!("[_]{}{{", elem_type.to_zig_str()));
                for (i, elem) in arr.elements.iter().enumerate() {
                    if i > 0 {
                        self.push(", ");
                    }
                    self.emit_array_element(elem);
                }
                self.push("}");
            },

            Expression::ObjectExpression(obj) => {
                // Collect properties into categories for spread handling
                let mut normal_props: Vec<(&ObjectProperty, String, &Expression)> = Vec::new();
                let mut spread_props: Vec<&SpreadElement> = Vec::new();

                for prop in &obj.properties {
                    match prop {
                        ObjectPropertyKind::ObjectProperty(p) => {
                            if matches!(p.value, Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_)) {
                                continue;
                            }
                            let key_str = property_key_name(&p.key);
                            normal_props.push((p, key_str, &p.value));
                        }
                        ObjectPropertyKind::SpreadProperty(spread) => {
                            spread_props.push(spread);
                        }
                    }
                }

                // Case 1: Pure spread { ...expr } — just pass through the expression
                if normal_props.is_empty() && spread_props.len() == 1 {
                    self.emit_expr(&spread_props[0].argument);
                    return;
                }

                // Case 2: Spread with property overrides — copy then mutate
                // { a: 1, ...base, c: 3 }  →  var _tmp = base; _tmp.a = 1; _tmp.c = 3; _tmp
                if spread_props.len() == 1 {
                    self.push("(blk: {\n");
                    self.indent += 1;

                    // Always use @TypeOf(base) to get the correct type, whether it's a
                    // named struct (e.g., Base_objects) or an anonymous struct.
                    self.emit_indent();
                    self.push("var _tmp: @TypeOf(");
                    self.emit_expr(&spread_props[0].argument);
                    self.push(") = ");
                    self.emit_expr(&spread_props[0].argument);
                    self.push(";\n");
                    for (_p, key_str, val_expr) in &normal_props {
                        self.emit_indent();
                        self.push("_tmp.");
                        self.push(key_str);
                        self.push(" = ");
                        self.emit_expr(val_expr);
                        self.push(";\n");
                    }
                    self.emit_indent();
                    self.push("break :blk _tmp;\n");
                    self.indent -= 1;
                    self.emit_indent();
                    self.push("})");
                    return;
                }

                // Case 3: Multiple spreads — not supported in Zig (no dynamic struct merging)
                if spread_props.len() > 1 {
                    self.push("@compileError(\"object spread with multiple sources is not supported — use field-by-field assignment instead\")");
                    return;
                }

                // Case 0: No spreads — normal struct literal emission
                self.push(".{ ");
                let mut first = true;
                for (_, key_str, val_expr) in &normal_props {
                    if !first {
                        self.push(", ");
                    }
                    first = false;
                    self.push(".");
                    self.push(key_str);
                    self.push(" = ");
                    self.emit_expr(val_expr);
                }
                self.push(" }");
            },

            Expression::TemplateLiteral(tl) => {
                // Emit template literals as string literals when possible
                if tl.expressions.is_empty()
                    && let Some(quasi) = tl.quasis.first()
                        && let Some(cooked) = &quasi.value.cooked {
                            self.push("\"");
                            self.push(&Self::escape_quasi_for_zig(cooked.as_ref()));
                            self.push("\"");
                            return;
                        }

                // Template literal with expressions: use std.fmt.allocPrint
                // e.g. `hello ${name}, you are ${age}` →
                //   std.fmt.allocPrint(js_allocator.g_alloc(), "hello {}{}!", .{ name, age }) catch @panic("OOM")
                //
                // Zig 0.16.0 format dispatch: {} uses default formatter (prints union internals),
                // {f} calls the custom .format(writer) method. JsAny/JsValue unions need {f}.
                let inferrer = &self.inferrer;
                let fmt_specs: Vec<&str> = tl.expressions.iter().map(|expr| {
                    match inferrer.infer_expr(expr) {
                        ZigType::JsAny | ZigType::JsValue => "{f}",
                        _ => "{}",
                    }
                }).collect();

                self.push("std.fmt.allocPrint(js_allocator.g_alloc(), \"");
                // Build format string with type-aware specifiers
                for (i, quasi) in tl.quasis.iter().enumerate() {
                    if let Some(cooked) = &quasi.value.cooked {
                        self.push(&Self::escape_quasi_for_zig(cooked.as_ref()));
                    }
                    if i < tl.expressions.len() {
                        self.push(fmt_specs[i]);
                    }
                }
                self.push("\", .{ ");
                for (i, expr) in tl.expressions.iter().enumerate() {
                    if i > 0 {
                        self.push(", ");
                    }
                    self.emit_expr(expr);
                }
                self.push(" }) catch @panic(\"OOM\")");
            }

            Expression::TaggedTemplateExpression(_) => {
                self.push("@compileError(\"tagged template expression is not supported\")");
            }

            Expression::ArrowFunctionExpression(arrow) => {
                // Look up by span start — covers return closures, callbacks, and assignments.
                // Struct definitions are already pre-generated during pre_scan_closures
                // and buffered in closure_structs for output after all functions.
                if let Some(ci) = self.closure_map.get(&arrow.span.start).cloned() {
                    // Emit struct literal only: ClosureName{ .cap1 = cap1, .cap2 = cap2 }
                    self.push(&ci.struct_name);
                    self.push("{ ");
                    for (i, (cap_name, _)) in ci.captured.iter().enumerate() {
                        if i > 0 {
                            self.push(", ");
                        }
                        self.push(".");
                        self.push(cap_name);
                        self.push(" = ");
                        self.push(cap_name);
                    }
                    self.push(" }");
                    return;
                }
                self.push("(@compileError(\"inline arrow function not yet supported - rewrite JS to use named functions\"))");
            }

            Expression::FunctionExpression(_) => {
                self.push("(@compileError(\"inline function not yet supported - rewrite JS to use named functions\"))");
            }

            Expression::AwaitExpression(ae) => {
                let task_var = format!("_t{}", self.task_counter);
                self.task_counter += 1;

                // emit: (blk: { var _tN = io.async(fn_async, .{io, args...}); defer _ = _tN.cancel(io) catch undefined; break :blk try _tN.await(io); })
                self.push("(blk: {\n");
                self.indent += 1;
                self.emit_indent();
                self.push("var ");
                self.push(&task_var);
                self.push(" = io.async(");

                match &ae.argument {
                    Expression::CallExpression(call) => {
                        self.emit_expr(&call.callee);
                        self.push(", .{ io");
                        for arg in &call.arguments {
                            self.push(", ");
                            if let Some(expr) = arg.as_expression() {
                                self.emit_expr(expr);
                            } else {
                                self.push("undefined");
                            }
                        }
                        self.push(" });\n");
                    }
                    _ => {
                        self.emit_expr(&ae.argument);
                        self.push(", .{ io });\n");
                    }
                }

                self.emit_indent();
                self.push("defer _ = ");
                self.push(&task_var);
                self.push(".cancel(io) catch undefined;\n");
                self.emit_indent();
                self.push("break :blk try ");
                self.push(&task_var);
                self.push(".await(io);\n");

                self.indent -= 1;
                self.emit_indent();
                self.push("})");
            }

            Expression::ParenthesizedExpression(parens) => {
                self.push("(");
                self.emit_expr(&parens.expression);
                self.push(")");
            }

            Expression::SequenceExpression(seq) => {
                for (i, expr) in seq.expressions.iter().enumerate() {
                    if i > 0 {
                        self.push(", ");
                    }
                    self.emit_expr(expr);
                }
            }

            Expression::ChainExpression(chain) => {
                match &chain.expression {
                    ChainElement::CallExpression(call) => {
                        self.emit_expr(&call.callee);
                        self.push("(");
                        for (i, arg) in call.arguments.iter().enumerate() {
                            if i > 0 { self.push(", "); }
                            self.emit_arg(arg);
                        }
                        self.push(")");
                    }
                    ChainElement::StaticMemberExpression(mem) => {
                        let prop = mem.property.name.as_str();
                        self.emit_expr(&mem.object);
                        self.push(".");
                        if prop == "catch" || prop == "async" || prop == "await" {
                            self.push("@\"");
                            self.push(prop);
                            self.push("\"");
                        } else {
                            self.push(prop);
                        }
                    }
                    ChainElement::ComputedMemberExpression(mem) => {
                        let obj_type = self.inferrer.infer_expr(&mem.object);
                        if matches!(&obj_type, ZigType::Object { .. })
                            && let Expression::StringLiteral(s) = &mem.expression
                        {
                            self.emit_expr(&mem.object);
                            self.push(".");
                            self.push(s.value.as_str());
                        } else {
                            self.emit_expr(&mem.object);
                            self.push("[");
                            self.emit_expr(&mem.expression);
                            self.push("]");
                        }
                    }
                    _ => {
                        self.push("/* chain element */");
                    }
                }
            }

            Expression::ClassExpression(_) => {
                self.push("@compileError(\"class expression (const X = class { ... }) is not yet implemented\")");
            }

            Expression::MetaProperty(_) => {
                self.push("@compileError(\"meta property (new.target) is not supported\")");
            }

            Expression::ImportExpression(_) => {
                self.push("@compileError(\"dynamic import (import()) is not supported — use static import instead\")");
            }

            Expression::Super(_) => {
                // In extends class: super.method() → self.base.method()
                self.push("self.base");
            }

            Expression::RegExpLiteral(lit) => {
                // Extract pattern from raw source (e.g., "/world/" → "world", "/zig/g" → "zig")
                let pattern = if let Some(ref raw) = lit.raw {
                    let raw_str = raw.as_str();
                    if let Some(inner) = raw_str.strip_prefix('/') {
                        if let Some(end) = inner.rfind('/') {
                            &inner[..end]
                        } else {
                            inner
                        }
                    } else {
                        raw_str
                    }
                } else {
                    ""
                };
                self.push("\"");
                self.push(pattern);
                self.push("\"");
            }

            Expression::TSAsExpression(ts) => {
                let ty = self.inferrer.infer_expr(&ts.expression);
                self.push("@as(");
                self.push(&ty.to_zig_str());
                self.push(", ");
                self.emit_expr(&ts.expression);
                self.push(")");
            }

            Expression::TSTypeAssertion(ts) => {
                let ty = self.inferrer.infer_expr(&ts.expression);
                self.push("@as(");
                self.push(&ty.to_zig_str());
                self.push(", ");
                self.emit_expr(&ts.expression);
                self.push(")");
            }

            Expression::TSNonNullExpression(ts) => {
                self.emit_expr(&ts.expression);
                self.push(".?");
            }

            Expression::TSSatisfiesExpression(ts) => {
                self.emit_expr(&ts.expression);
            }

            Expression::TSInstantiationExpression(ts) => {
                self.emit_expr(&ts.expression);
            }

            Expression::YieldExpression(_) => {
                self.push("@compileError(\"yield expression is not supported — generators are not yet implemented\")");
            }

            Expression::V8IntrinsicExpression(_) => {
                self.push("@compileError(\"V8 intrinsic expression is not supported\")");
            }

            Expression::PrivateInExpression(_) => {
                self.push("@compileError(\"private in expression is not supported\")");
            }

            Expression::JSXElement(_) | Expression::JSXFragment(_) => {
                self.push("@compileError(\"JSX is not supported — use createElement() calls instead\")");
            }
        }
    }
}
