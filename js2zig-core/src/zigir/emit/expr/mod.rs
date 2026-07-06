// zigir/emit/expr/mod.rs
// Expression-level Zig emission from IrExpr nodes.

pub mod binary;
pub mod call_member;
pub mod container;
pub mod template_new;

use crate::zigir::emit::Emitter;
use crate::zigir::emit::helpers::{
    EmitterHelpers, assign_op_to_zig, bin_op_to_zig, escape_zig_string, format_param_with_rest,
    logical_op_to_zig, update_op_to_zig,
};

// ═══════════════════════════════════════════════════════
//  Expression dispatch
// ═══════════════════════════════════════════════════════

impl Emitter {
    pub(crate) fn emit_expr(&mut self, expr: &crate::zigir::types::IrExpr) {
        match expr {
            crate::zigir::types::IrExpr::IntLiteral(n) => {
                self.write(&n.to_string());
            }

            crate::zigir::types::IrExpr::FloatLiteral(n) => {
                if *n == -0.0 && n.is_sign_negative() {
                    self.write("-0.0");
                } else {
                    self.write(&n.to_string());
                }
            }

            crate::zigir::types::IrExpr::StringLiteral(s) => {
                self.write(&format!("\"{}\"", escape_zig_string(s)));
            }

            crate::zigir::types::IrExpr::BoolLiteral(b) => {
                self.write(if *b { "true" } else { "false" });
            }

            crate::zigir::types::IrExpr::BigIntLiteral(s) => {
                // Wrap in parentheses: `catch` has lower precedence than `+`/`-`,
                // so `BigInt.init(...) catch @panic(...) + x` would parse incorrectly.
                self.write(&format!(
                    "(js_bigint.JsBigInt.init(js_allocator.allocator(), \"{}\") catch @panic(\"OOM: BigInt init\"))",
                    s
                ));
            }

            crate::zigir::types::IrExpr::Null => {
                self.write("JsAny.fromNull()");
            }

            crate::zigir::types::IrExpr::Undefined => {
                self.write("JsAny.fromUndefined()");
            }

            crate::zigir::types::IrExpr::Ident(ident) => {
                self.write(&ident.zig_name);
            }

            crate::zigir::types::IrExpr::This => {
                self.write("self");
            }

            // ── Arithmetic / comparison ─────────────
            crate::zigir::types::IrExpr::Binary {
                op,
                left,
                right,
                left_type,
                right_type,
            } => {
                use crate::types::ZigType;
                use crate::zigir::ops::BinOp;

                // Determine if operands are BigInt, JsAny, String, or other
                let lt = left_type.as_ref();
                let rt = right_type.as_ref();
                let left_is_bigint = lt == Some(&ZigType::BigInt);
                let right_is_bigint = rt == Some(&ZigType::BigInt);
                let left_is_str = lt == Some(&ZigType::Str);
                let right_is_str = rt == Some(&ZigType::Str);
                let left_is_jsany = lt == Some(&ZigType::JsAny);
                let right_is_jsany = rt == Some(&ZigType::JsAny);
                let left_is_float = lt == Some(&ZigType::F64);
                let right_is_float = rt == Some(&ZigType::F64);

                // ── BigInt arithmetic/comparison ──
                if left_is_bigint && right_is_bigint {
                    self.emit_bigint_binary(*op, left, right);
                }
                // ── String equality/comparison ──
                else if left_is_str && right_is_str {
                    self.emit_string_comparison(*op, left, right);
                }
                // ── Cross-type comparison (e.g. String vs Number, Bool vs I64) ──
                // When both types are known but different, Zig's == / != / < etc.
                // are type-mismatch errors. Route through JsAny comparison instead.
                else if lt.is_some()
                    && rt.is_some()
                    && lt != rt
                    && matches!(
                        op,
                        BinOp::Eq
                            | BinOp::Ne
                            | BinOp::StrictEq
                            | BinOp::StrictNe
                            | BinOp::Lt
                            | BinOp::Le
                            | BinOp::Gt
                            | BinOp::Ge
                    )
                {
                    // Neither side is JsAny here (that branch is above),
                    // but emit_jsany_comparison will wrap both in JsAny.from().
                    self.emit_jsany_comparison(*op, left, right, false, false);
                }
                // ── JsAny comparison ──
                else if (left_is_jsany || right_is_jsany)
                    && matches!(
                        op,
                        BinOp::Eq
                            | BinOp::Ne
                            | BinOp::StrictEq
                            | BinOp::StrictNe
                            | BinOp::Lt
                            | BinOp::Le
                            | BinOp::Gt
                            | BinOp::Ge
                    )
                {
                    self.emit_jsany_comparison(*op, left, right, left_is_jsany, right_is_jsany);
                }
                // ── Division ──
                else if *op == BinOp::Div {
                    if left_is_float || right_is_float {
                        self.write("(");
                        self.emit_expr(left);
                        self.write(" / ");
                        self.emit_expr(right);
                        self.write(")");
                    } else {
                        self.write("@divTrunc(");
                        self.emit_expr(left);
                        self.write(", ");
                        self.emit_expr(right);
                        self.write(")");
                    }
                }
                // ── Remainder ──
                else if *op == BinOp::Mod {
                    if left_is_float || right_is_float {
                        self.write("(");
                        self.emit_expr(left);
                        self.write(" % ");
                        self.emit_expr(right);
                        self.write(")");
                    } else {
                        self.write("@rem(");
                        self.emit_expr(left);
                        self.write(", ");
                        self.emit_expr(right);
                        self.write(")");
                    }
                }
                // ── Unsigned right shift ──
                else if *op == BinOp::UrShr {
                    self.write("@as(i64, @intCast(@as(u32, @bitCast(@as(i32, @truncate(");
                    self.emit_expr(left);
                    self.write(")))) >> @intCast(");
                    self.emit_expr(right);
                    self.write(" & 31)))");
                }
                // ── `in` operator: key in obj → obj.contains(key) ──
                else if *op == BinOp::In {
                    // Right side is the object, left side is the key (operands swapped for .contains())
                    self.emit_expr(right);
                    self.write(".contains(");
                    self.emit_expr(left);
                    self.write(")");
                }
                // ── Default: direct operator ──
                else {
                    self.emit_expr(left);
                    self.write(&format!(" {} ", bin_op_to_zig(*op)));
                    self.emit_expr(right);
                }
            }

            crate::zigir::types::IrExpr::PowExpr {
                base,
                exp,
                base_type,
                exp_type,
            } => {
                // JS `**` always returns f64. Use std.math.pow(f64, ...) with
                // temporary f64 variables in a labeled block.
                let pow_id = self.peek_label_id();
                let blk = self.next_label();
                self.write(&format!("({blk}: {{ "));
                self.write(&format!("const _base_f64_{pow_id}: f64 = "));
                self.emit_float_conversion(base, base_type);
                self.write(&format!("; const _exp_f64_{pow_id}: f64 = "));
                self.emit_float_conversion(exp, exp_type);
                self.write(&format!(
                    "; break :{blk} std.math.pow(f64, _base_f64_{pow_id}, _exp_f64_{pow_id}); }})",
                ));
            }

            crate::zigir::types::IrExpr::Unary { op, operand } => {
                match op {
                    crate::zigir::ops::UnaOp::Neg => {
                        self.write("-");
                        self.emit_expr(operand);
                    }
                    crate::zigir::ops::UnaOp::Not => {
                        self.write("!");
                        self.emit_expr(operand);
                    }
                    crate::zigir::ops::UnaOp::BitNot => {
                        self.write("~");
                        self.emit_expr(operand);
                    }
                    crate::zigir::ops::UnaOp::TypeOf => {
                        // typeof expr → simplified; real impl depends on type
                        self.write("\"undefined\""); // placeholder
                        let _ = operand; // suppress warning
                    }
                    crate::zigir::ops::UnaOp::Void => {
                        // void expr → evaluate and discard
                        self.emit_expr(operand);
                    }
                    crate::zigir::ops::UnaOp::Delete => {
                        // delete is a no-op in this context
                        let _ = operand;
                    }
                }
            }

            crate::zigir::types::IrExpr::Logical { op, left, right } => {
                // Zig `and`/`or` require bool operands. Coerce operands with
                // truthiness if they are not already bool.
                self.write("(");
                self.emit_expr_as_bool(left);
                self.write(&format!(" {} ", logical_op_to_zig(*op)));
                self.emit_expr_as_bool(right);
                self.write(")");
            }

            crate::zigir::types::IrExpr::Update {
                op,
                target,
                is_expr_stmt,
            } => {
                if *is_expr_stmt {
                    // Statement context: `i += 1` (no parens, no _ = prefix)
                    self.emit_assign_target_inner(target);
                    self.write(&format!(" {}", update_op_to_zig(*op)));
                } else {
                    // Expression context: `({blk}: { ... break :blk old_val })`
                    self.write("(");
                    self.emit_assign_target_inner(target);
                    self.write(&format!(" {}", update_op_to_zig(*op)));
                    self.write(")");
                }
            }

            crate::zigir::types::IrExpr::Assign { op, target, value } => {
                use crate::zigir::ops::AssignOp;
                if *op == AssignOp::Mod {
                    // Zig doesn't support % on signed integers; use x = @rem(x, y)
                    self.emit_assign_target_inner(target);
                    self.write(" = @rem(");
                    self.emit_assign_target_inner(target);
                    self.write(", ");
                    self.emit_expr(value);
                    self.write(")");
                } else if *op == AssignOp::Div {
                    // Zig signed integer division requires @divTrunc
                    self.emit_assign_target_inner(target);
                    self.write(" = @divTrunc(");
                    self.emit_assign_target_inner(target);
                    self.write(", ");
                    self.emit_expr(value);
                    self.write(")");
                } else if *op == AssignOp::LogicAnd {
                    // a &&= b → a = if (a.toBool()) b else a
                    self.emit_assign_target_inner(target);
                    self.write(" = if (");
                    self.emit_assign_target_inner(target);
                    self.write(".toBool()) ");
                    self.emit_expr(value);
                    self.write(" else ");
                    self.emit_assign_target_inner(target);
                } else if *op == AssignOp::LogicOr {
                    // a ||= b → a = if (!a.toBool()) b else a
                    self.emit_assign_target_inner(target);
                    self.write(" = if (!");
                    self.emit_assign_target_inner(target);
                    self.write(".toBool()) ");
                    self.emit_expr(value);
                    self.write(" else ");
                    self.emit_assign_target_inner(target);
                } else if *op == AssignOp::Nullish {
                    // a ??= b → a = if (a.isNullish()) b else a
                    self.emit_assign_target_inner(target);
                    self.write(" = if (");
                    self.emit_assign_target_inner(target);
                    self.write(".isNullish()) ");
                    self.emit_expr(value);
                    self.write(" else ");
                    self.emit_assign_target_inner(target);
                } else {
                    self.emit_assign_target_inner(target);
                    self.write(&format!(" {} ", assign_op_to_zig(*op)));
                    self.emit_expr(value);
                }
            }

            // ── Calls ───────────────────────────────
            crate::zigir::types::IrExpr::Call(call) => {
                self.emit_call_expr(call);
            }

            crate::zigir::types::IrExpr::BuiltinCall(bc) => {
                self.emit_builtin_call(bc);
            }

            crate::zigir::types::IrExpr::HostCall(hc) => {
                self.write(&format!("host.{}", hc.name));
                self.emit_args(&hc.args);
            }

            // ── Member access ───────────────────────
            crate::zigir::types::IrExpr::FieldAccess {
                object,
                field,
                field_kind,
            } => {
                self.emit_field_access(object, field, field_kind);
            }

            crate::zigir::types::IrExpr::IndexAccess {
                object,
                index,
                index_kind,
            } => {
                self.emit_index_access(object, index, index_kind);
            }

            crate::zigir::types::IrExpr::ComputedField {
                object,
                key,
                key_kind,
            } => {
                self.emit_computed_field(object, key, key_kind);
            }

            // ── Object / Array ──────────────────────
            crate::zigir::types::IrExpr::ArrayLiteral(arr) => {
                self.emit_array_literal(arr);
            }

            crate::zigir::types::IrExpr::ObjectLiteral(obj) => {
                self.emit_object_literal(obj);
            }

            // ── Function expressions ────────────────
            // NOTE: Dead code path — Lowerer always converts ArrowFn to IrExpr::Ident
            // + pending_arrow_structs. Kept as defensive fallback.
            crate::zigir::types::IrExpr::ArrowFn(arrow) => {
                // Arrow fn at expression level: emit inline struct
                self.writeln(&format!("const {} = struct {{", arrow_name_placeholder()));
                self.indent_push();
                let ret = arrow.return_type.to_zig_type();
                let mut sig = String::from("pub fn call(");
                for (i, param) in arrow.params.iter().enumerate() {
                    if i > 0 {
                        sig.push_str(", ");
                    }
                    sig.push_str(&format_param_with_rest(
                        &param.name,
                        &param.zig_type,
                        param.is_rest,
                    ));
                }
                sig.push_str(&format!(") {} {{", ret));
                self.writeln(&sig);
                self.indent_push();
                // Body
                if arrow.is_concise {
                    self.write_indent();
                    self.write("return ");
                    if let Some(stmt) = arrow.body.stmts.first()
                        && let crate::zigir::types::IrStmt::Expr(e) = stmt
                    {
                        self.emit_expr(e);
                    }
                    self.write(";\n");
                } else {
                    for stmt in &arrow.body.stmts {
                        self.emit_stmt(stmt);
                    }
                }
                self.indent_pop();
                self.writeln("}");
                self.indent_pop();
                self.writeln("};");
            }

            crate::zigir::types::IrExpr::Closure(closure) => {
                // Closure instance: `StructName { .captured = val, ... }`
                self.write(&closure.struct_name.zig_name);
                self.write("{ ");
                for (i, cap) in closure.captured.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    if cap.is_mut {
                        self.write(&format!(".{} = &{}", cap.name.zig_name, cap.name.zig_name));
                    } else {
                        self.write(&format!(".{} = {}", cap.name.zig_name, cap.name.zig_name));
                    }
                }
                self.write(" }");
            }

            crate::zigir::types::IrExpr::FnExpr(fn_expr) => {
                // Function expression: emit as struct with call method
                let name = fn_expr
                    .name
                    .as_ref()
                    .map(|n| n.zig_name.clone())
                    .unwrap_or_else(|| "_fn_expr".to_string());
                self.write(&name);
            }

            // ── Conditional / template ───────────────
            crate::zigir::types::IrExpr::Conditional { cond, then, else_ } => {
                self.write("if (");
                self.emit_expr_as_bool(cond);
                self.write(") ");
                self.emit_expr(then);
                self.write(" else ");
                self.emit_expr(else_);
            }

            crate::zigir::types::IrExpr::TemplateLiteral {
                parts,
                exprs,
                format_specs,
            } => {
                self.emit_template_literal(parts, exprs, format_specs);
            }

            // ── Async ───────────────────────────────
            crate::zigir::types::IrExpr::Await(await_expr) => {
                // Emit: (blk: { var _tN = io.async(callee, .{ io, args... }); defer _ = _tN.cancel(io) catch undefined; break :blk try _tN.await(io); })
                self.write(&format!("({}: {{\n", await_expr.block_label));
                self.indent_push();
                self.write_indent();
                self.write(&format!("var {} = io.async(", await_expr.task_var.zig_name));

                if await_expr.is_host_async {
                    // For host async, callee is an Ident holding the host fn name
                    if let crate::zigir::types::IrExpr::Ident(id) = &*await_expr.callee {
                        self.write(&format!("host.{}_async", id.zig_name));
                    } else {
                        // Fallback
                        self.emit_expr(&await_expr.callee);
                    }
                } else {
                    self.emit_expr(&await_expr.callee);
                }

                self.write(", .{ io");
                for arg in &await_expr.args {
                    self.write(", ");
                    self.emit_expr(arg);
                }
                self.write(" });\n");

                self.write_indent();
                self.write(&format!(
                    "defer _ = {}.cancel(io) catch undefined;\n",
                    await_expr.task_var.zig_name
                ));

                self.write_indent();
                self.write(&format!(
                    "break :{} try {}.await(io);\n",
                    await_expr.block_label, await_expr.task_var.zig_name
                ));

                self.indent_pop();
                self.write_indent();
                self.write("})");
            }

            // ── Construction ────────────────────────
            crate::zigir::types::IrExpr::New(new_expr) => {
                self.emit_new_expr(new_expr);
            }

            // ── String formatting ──────────────────────
            crate::zigir::types::IrExpr::AllocPrint { fmt, args } => {
                if args.is_empty() {
                    // Pure-text → plain string literal (no allocation)
                    let unescaped = fmt.replace("{{", "{").replace("}}", "}");
                    self.write(&format!("\"{}\"", unescaped));
                } else {
                    // Emit args by capturing each as a string
                    let arg_strs: Vec<String> = args
                        .iter()
                        .map(|arg| {
                            let saved = std::mem::take(self.output_mut());
                            self.emit_expr(arg);
                            let rendered = std::mem::take(self.output_mut());
                            *self.output_mut() = saved;
                            rendered
                        })
                        .collect();
                    let args_str = format!(".{{{}}}", arg_strs.join(", "));
                    self.write(&format!(
                        "std.fmt.allocPrint(js_allocator.allocator(), \"{}\", {}) catch @panic(\"OOM: template literal allocPrint\")",
                        fmt, args_str
                    ));
                }
            }

            // ── Block expression ────────────────────
            crate::zigir::types::IrExpr::BlockExpr {
                label,
                body,
                result,
            } => {
                self.write(&format!("({}: {{", label));
                for stmt in body {
                    self.emit_stmt(stmt);
                }
                self.write("break :");
                self.write(label);
                self.write(" ");
                self.emit_expr(result);
                self.write("}})");
            }

            // ── Special ─────────────────────────────
            crate::zigir::types::IrExpr::Spread(inner) => {
                self.emit_expr(inner);
            }

            crate::zigir::types::IrExpr::Typeof(inner) => {
                // typeof on identifiers with known types is resolved at lower time
                // to StringLiteral. This branch handles any remaining typeof
                // expressions (shouldn't occur with current lowerer, but as fallback).
                self.write("js_runtime.jsTypeof(");
                self.emit_expr(inner);
                self.write(")");
            }

            crate::zigir::types::IrExpr::Void(inner) => {
                // void expr: evaluate expr for side effects, return undefined.
                // Output: blk_N: { _ = expr; break :blk_N JsAny.fromUndefined(); }
                let label = self.next_label();
                self.write(&format!("{}: {{ _ = ", label));
                self.emit_expr(inner);
                self.write(&format!("; break :{} JsAny.fromUndefined(); }}", label));
            }

            crate::zigir::types::IrExpr::Paren(inner) => {
                self.write("(");
                self.emit_expr(inner);
                self.write(")");
            }

            crate::zigir::types::IrExpr::Sequence(exprs) => {
                for (i, e) in exprs.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.emit_expr(e);
                }
            }

            crate::zigir::types::IrExpr::ArrayCallbackInline(inline_data) => {
                self.emit_array_callback_inline(inline_data);
            }

            crate::zigir::types::IrExpr::ArrayMethodInline(inline_data) => {
                self.emit_array_method_inline(inline_data);
            }

            crate::zigir::types::IrExpr::OptionalChain {
                object,
                capture_var,
                body,
                needs_null_check,
            } => {
                if *needs_null_check {
                    self.write("(if (");
                    self.emit_expr(object);
                    self.write(") |");
                    self.write(capture_var);
                    self.write("| ");
                    self.emit_expr(body);
                    self.write(" else null)");
                } else {
                    self.emit_expr(body);
                }
            }

            crate::zigir::types::IrExpr::CompileError { span, msg } => {
                let loc = format!("{}:{}", span.js_line, span.js_col);
                self.write(&format!(
                    "@compileError(\"{} (at {})\")",
                    escape_zig_string(msg),
                    loc
                ));
            }
        }
    }

    /// Convert an expression to a string without writing to the output buffer.
    /// Used for contexts where we need the expression text as a value.
    pub(crate) fn expr_to_string(&mut self, expr: &crate::zigir::types::IrExpr) -> String {
        let saved_output = std::mem::take(&mut self.output);
        self.emit_expr(expr);
        let result = std::mem::take(&mut self.output);
        self.output = saved_output;
        result
    }

    // ── Internal helpers ───────────────────────────

    pub(super) fn emit_assign_target_inner(
        &mut self,
        target: &crate::zigir::types::IrAssignTarget,
    ) {
        match target {
            crate::zigir::types::IrAssignTarget::Ident(ident) => {
                self.write(&ident.zig_name);
            }
            crate::zigir::types::IrAssignTarget::Member {
                object,
                field,
                is_pointer,
            } => {
                self.emit_expr(object);
                if *is_pointer {
                    self.write(&format!(".{}.*", field));
                } else {
                    self.write(&format!(".{}", field));
                }
            }
            crate::zigir::types::IrAssignTarget::Index { object, index } => {
                self.emit_expr(object);
                self.write("[");
                self.emit_expr(index);
                self.write("]");
            }
            crate::zigir::types::IrAssignTarget::Destructure(bindings) => {
                for (i, binding) in bindings.iter().enumerate() {
                    if i > 0 {
                        self.write("; ");
                    }
                    self.write(&binding.pattern.zig_name);
                    if let Some(default) = &binding.default {
                        self.write(" orelse ");
                        self.emit_expr(default);
                    }
                }
            }
        }
    }

    pub(super) fn emit_args(&mut self, args: &[crate::zigir::types::IrExpr]) {
        self.write("(");
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            // foo(...args) → foo(args.items)
            if let crate::zigir::types::IrExpr::Spread(inner) = arg {
                self.emit_expr(inner);
                self.write(".items");
            } else {
                self.emit_expr(arg);
            }
        }
        self.write(")");
    }

    /// Check if an IrExpr is definitely of type `bool`.
    pub(super) fn ir_expr_is_bool(expr: &crate::zigir::types::IrExpr) -> bool {
        use crate::zigir::ops::BinOp;
        use crate::zigir::types::IrExpr;

        match expr {
            IrExpr::BoolLiteral(_) => true,
            IrExpr::Logical { .. } => true,
            IrExpr::Unary {
                op: crate::zigir::ops::UnaOp::Not | crate::zigir::ops::UnaOp::Delete,
                ..
            } => true,
            IrExpr::Binary { op, .. } => matches!(
                op,
                BinOp::Eq
                    | BinOp::Ne
                    | BinOp::StrictEq
                    | BinOp::StrictNe
                    | BinOp::Lt
                    | BinOp::Le
                    | BinOp::Gt
                    | BinOp::Ge
                    | BinOp::In
                    | BinOp::InstanceOf
            ),
            _ => false,
        }
    }

    /// Check if an IrExpr produces a string type.
    pub(super) fn ir_expr_is_string(expr: &crate::zigir::types::IrExpr) -> bool {
        use crate::zigir::types::IrExpr;
        matches!(
            expr,
            IrExpr::StringLiteral(_) | IrExpr::TemplateLiteral { .. }
        )
    }

    /// Emit an expression with truthiness coercion for Zig `bool` context.
    ///
    /// When a non-bool expression appears in a position that Zig requires `bool`
    /// (e.g. `if` condition, `while` condition), we coerce it via JS truthiness:
    /// - `bool` expressions → emitted directly (no coercion needed)
    /// - `Str` expressions → `.len != 0` (empty string is falsy in JS)
    /// - numeric/other → `((expr) != 0)` (0 is falsy in JS)
    pub(crate) fn emit_expr_as_bool(&mut self, expr: &crate::zigir::types::IrExpr) {
        if Self::ir_expr_is_bool(expr) {
            self.emit_expr(expr);
        } else if Self::ir_expr_is_string(expr) {
            // String truthiness: non-empty → true, empty → false
            self.write("(");
            self.emit_expr(expr);
            self.write(".len != 0)");
        } else {
            // Default numeric truthiness: 0 → false, non-zero → true
            self.write("((");
            self.emit_expr(expr);
            self.write(") != 0)");
        }
    }
}

/// Map TypedArrayKind to (module, init_fn) for construction.
/// Uses fromI64AsI8/fromI64AsU8/.../fromF64 series instead of direct .init().
pub(super) fn typed_array_init(
    kind: &crate::zigir::kinds::TypedArrayKind,
) -> (&'static str, &'static str) {
    use crate::zigir::kinds::TypedArrayKind;
    match kind {
        TypedArrayKind::Int8Array => ("js_runtime.js_typedarray", "fromI64AsI8"),
        TypedArrayKind::Uint8Array => ("js_runtime.js_typedarray", "fromI64AsU8"),
        TypedArrayKind::Uint8ClampedArray => ("js_runtime.js_typedarray", "fromI64AsU8"),
        TypedArrayKind::Int16Array => ("js_runtime.js_typedarray", "fromI64AsI16"),
        TypedArrayKind::Uint16Array => ("js_runtime.js_typedarray", "fromI64AsU16"),
        TypedArrayKind::Int32Array => ("js_runtime.js_typedarray", "fromI64AsI32"),
        TypedArrayKind::Uint32Array => ("js_runtime.js_typedarray", "fromI64AsU32"),
        TypedArrayKind::Float32Array => ("js_runtime.js_typedarray", "fromF64AsF32"),
        TypedArrayKind::Float64Array => ("js_runtime.js_typedarray", "fromF64"),
        TypedArrayKind::BigInt64Array => ("js_runtime.js_typedarray", "fromI64AsI64"),
        TypedArrayKind::BigUint64Array => ("js_runtime.js_typedarray", "fromI64AsU64"),
    }
}

fn arrow_name_placeholder() -> String {
    "_arrow_fn".to_string()
}
