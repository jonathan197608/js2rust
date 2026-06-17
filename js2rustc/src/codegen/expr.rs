use super::*;

impl<'a> ZigCodegen<'a> {
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
                self.push(&lit.value);
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
                    _ => {}
                }
                self.push(&Self::escape_keyword(id.name.as_str()));
            }

            Expression::ThisExpression(_) => {
                self.push("self");
            },

            Expression::BinaryExpression(bin) => {
                // Handle special operators that don't map 1:1 to Zig
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
                }
                self.emit_expr(&bin.left);
                self.push(" ");
                self.push(self.map_binary_op(&bin.operator));
                self.push(" ");
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
                        self.push("@TypeOf(");
                        self.emit_expr(&unary.argument);
                        self.push(")");
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
                let op = match update.operator {
                    UpdateOperator::Increment => "+=",
                    UpdateOperator::Decrement => "-=",
                };
                self.emit_assign_target_from_simple(&update.argument);
                self.push(" ");
                self.push(op);
                self.push(" 1");
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
                // Check for built-in constructors (Map, Set, etc.)
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
                
                // Map JS .length to Zig .len for arrays and strings
                if mem.property.name.as_str() == "length" {
                    // Dynamic arrays (ArrayList): use .items.len
                    if let Expression::Identifier(id) = &mem.object
                        && self.inferrer.is_dynamic_array(id.name.as_str())
                    {
                        self.emit_expr(&mem.object);
                        self.push(".items.len");
                        return;
                    }
                    let obj_ty = self.inferrer.infer_expr(&mem.object);
                    if obj_ty == ZigType::String || matches!(obj_ty, ZigType::Array(_)) {
                        self.emit_expr(&mem.object);
                        self.push(".len");
                        return;
                    }
                }

                // Check builtin static properties (e.g., Math.PI → std.math.pi)
                if let Expression::Identifier(id) = &mem.object
                    && let Some(zig_expr) = self.builtins.lookup_property(id.name.as_str(), mem.property.name.as_str()) {
                        self.push(zig_expr);
                        return;
                    }

                self.emit_expr(&mem.object);
                self.push(".");
                self.push(mem.property.name.as_str());
            }

            Expression::ComputedMemberExpression(mem) => {
                // Check if object is a dynamic array (ArrayList)
                // Distinguish: function params with slice type use direct indexing;
                // locally-declared dynamic arrays use .items[...]
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
                    self.emit_expr(&mem.object);
                    self.push(".");
                    self.push(s.value.as_str());
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
                self.push("// TODO: private field");
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

                self.emit_assign_target(&assign.left);
                self.push(" ");
                self.push(self.map_assign_op(&assign.operator));
                self.push(" ");
                self.emit_expr(&assign.right);
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
                    }).unwrap_or(ZigType::Any)  // inference failed → Zig compile error
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
                            self.push(cooked.as_ref());
                            self.push("\"");
                            return;
                        }

                // Template literal with expressions: use std.fmt.allocPrint
                // e.g. `hello ${name}, you are ${age}` →
                //   std.fmt.allocPrint(js_allocator.g_alloc(), "hello {}{}!", .{ name, age }) catch @panic("OOM")
                self.push("std.fmt.allocPrint(js_allocator.g_alloc(), \"");
                // Build format string
                for (i, quasi) in tl.quasis.iter().enumerate() {
                    if let Some(cooked) = &quasi.value.cooked {
                        self.push(cooked.as_ref());
                    }
                    if i < tl.expressions.len() {
                        self.push("{}");
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
                self.push("// TODO: tagged template");
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

                // emit: (blk: { var _tN = io.async(fn, .{io, args...}); defer _tN.cancel(io) catch {}; break :blk try _tN.await(io); })
                self.push("(blk: {\n");
                self.indent += 1;
                self.emit_indent();
                self.push("var ");
                self.push(&task_var);
                self.push(" = io.async(");

                // Extract the function and arguments from the await argument
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
                        // await non-call expression: treat as io.async(expr, .{io})
                        self.emit_expr(&ae.argument);
                        self.push(", .{ io });\n");
                    }
                }

                self.emit_indent();
                self.push("defer ");
                self.push(&task_var);
                self.push(".cancel(io) catch {};\n");

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
                        self.emit_expr(&mem.object);
                        self.push(".");
                        self.push(mem.property.name.as_str());
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
                self.push("// TODO: meta property");
            }

            Expression::ImportExpression(_) => {
                self.push("@compileError(\"dynamic import (import()) is not supported — use static import instead\")");
            }

            Expression::Super(_) => {
                self.push("super");
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
                self.push("// TODO: yield");
            }

            Expression::V8IntrinsicExpression(_) => {
                self.push("// TODO: V8 intrinsic");
            }

            Expression::PrivateInExpression(_) => {
                self.push("// TODO: private in");
            }

            Expression::JSXElement(_) | Expression::JSXFragment(_) => {
                self.push("// TODO: JSX");
            }
        }
    }
}
