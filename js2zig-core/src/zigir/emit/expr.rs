// zigir/emit/expr.rs
// Expression-level Zig emission from IrExpr nodes.

use crate::zigir::emit::Emitter;
use crate::zigir::emit::helpers::{
    EmitterHelpers, assign_op_to_zig, bin_op_to_zig, escape_zig_string, format_param_with_rest,
    logical_op_to_zig, update_op_to_zig,
};
use crate::zigir::kinds::{
    CallKind, ComputedKeyKind, DateConstructorKind, FieldKind, IndexKind, NewConstructor,
    TypedArrayKind,
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
                if *op == crate::zigir::ops::AssignOp::Mod {
                    // Zig doesn't support % on signed integers; use x = @rem(x, y)
                    self.emit_assign_target_inner(target);
                    self.write(" = @rem(");
                    self.emit_assign_target_inner(target);
                    self.write(", ");
                    self.emit_expr(value);
                    self.write(")");
                } else if *op == crate::zigir::ops::AssignOp::Div {
                    // Zig signed integer division requires @divTrunc
                    self.emit_assign_target_inner(target);
                    self.write(" = @divTrunc(");
                    self.emit_assign_target_inner(target);
                    self.write(", ");
                    self.emit_expr(value);
                    self.write(")");
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
                // Simplified await emission
                self.write("(try ");
                if await_expr.is_host_async {
                    self.write("host.");
                    self.emit_expr(&await_expr.callee);
                    self.write("_async(");
                    // NOTE: await_expr.args contains the args, but for now we emit simplified
                    for (i, arg) in await_expr.args.iter().enumerate() {
                        if i > 0 {
                            self.write(", ");
                        }
                        self.emit_expr(arg);
                    }
                    self.write(")");
                } else {
                    self.emit_expr(&await_expr.callee);
                }
                self.write(")");
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
                    self.write(" else JsAny.fromNull())");
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

    fn emit_assign_target_inner(&mut self, target: &crate::zigir::types::IrAssignTarget) {
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

    fn emit_args(&mut self, args: &[crate::zigir::types::IrExpr]) {
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

    fn emit_call_expr(&mut self, call: &crate::zigir::types::IrCallExpr) {
        match &call.call_kind {
            CallKind::Direct => {
                self.emit_expr(&call.callee);
                self.emit_args(&call.args);
            }
            CallKind::Method { object_type: _ } => {
                self.emit_expr(&call.callee);
                self.emit_args(&call.args);
            }
            CallKind::Closure => {
                self.emit_expr(&call.callee);
                self.write(".call(");
                for (i, arg) in call.args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.emit_expr(arg);
                }
                self.write(")");
            }
        }
    }

    fn emit_field_access(
        &mut self,
        object: &crate::zigir::types::IrExpr,
        field: &str,
        kind: &FieldKind,
    ) {
        match kind {
            FieldKind::StructField => {
                self.emit_expr(object);
                self.write(&format!(".{}", field));
            }
            FieldKind::Namespace => {
                self.emit_expr(object);
                self.write(&format!(".{}", field));
            }
            FieldKind::ArrayListLen => {
                self.emit_expr(object);
                self.write(".items.len");
            }
            FieldKind::StringLen => {
                self.emit_expr(object);
                self.write(".len");
            }
            FieldKind::MapSetSize => {
                self.emit_expr(object);
                self.write(".size()");
            }
            FieldKind::MathConstant(val) => {
                self.write(&format!("std.math.{}", val));
            }
            FieldKind::NumberConstant(val) => {
                self.write(&format!("std.math.{}", val));
            }
            FieldKind::SymbolWellKnown(val) => {
                self.write(&format!("js_symbol.{}()", val));
            }
            FieldKind::TypedArrayProp(prop) => {
                self.emit_expr(object);
                self.write(&format!(".{}", prop));
            }
            FieldKind::Private => {
                self.emit_expr(object);
                self.write(&format!(".{}", field));
            }
            FieldKind::PointerDeref => {
                self.emit_expr(object);
                self.write(".*");
            }
        }
    }

    fn emit_index_access(
        &mut self,
        object: &crate::zigir::types::IrExpr,
        index: &crate::zigir::types::IrExpr,
        kind: &IndexKind,
    ) {
        match kind {
            IndexKind::ArrayListItem => {
                self.emit_expr(object);
                self.write(".items[");
                self.emit_expr(index);
                self.write("]");
            }
            IndexKind::SliceIndex => {
                self.emit_expr(object);
                self.write("[");
                self.emit_expr(index);
                self.write("]");
            }
        }
    }

    fn emit_computed_field(
        &mut self,
        object: &crate::zigir::types::IrExpr,
        key: &crate::zigir::types::IrExpr,
        kind: &ComputedKeyKind,
    ) {
        match kind {
            ComputedKeyKind::StructField => {
                self.write("@field(");
                self.emit_expr(object);
                self.write(", ");
                self.emit_expr(key);
                self.write(")");
            }
            ComputedKeyKind::MapGet => {
                self.emit_expr(object);
                self.write(".get(");
                self.emit_expr(key);
                self.write(")");
            }
            ComputedKeyKind::JsAnyGetByKey => {
                self.emit_expr(object);
                self.write(".getByKey(");
                self.emit_expr(key);
                self.write(", js_allocator.allocator())");
            }
            ComputedKeyKind::ArrayListItem => {
                self.emit_expr(object);
                self.write(".items[");
                self.emit_expr(key);
                self.write("]");
            }
            ComputedKeyKind::CompileError(msg) => {
                self.write(&format!("@compileError(\"{}\")", escape_zig_string(msg)));
            }
        }
    }

    fn emit_array_literal(&mut self, arr: &crate::zigir::types::IrArrayLiteral) {
        if arr.elements.is_empty() {
            self.write("std.ArrayList(JsAny).empty");
            return;
        }

        // Determine element type: if any spread is present, force JsAny (mixed types guaranteed).
        // Otherwise, infer from first element.
        let elem_type = if !arr.spread_indices.is_empty() {
            "JsAny"
        } else {
            arr.elements
                .first()
                .map(|e| match e {
                    crate::zigir::types::IrExpr::IntLiteral(_) => "i64",
                    crate::zigir::types::IrExpr::FloatLiteral(_) => "f64",
                    crate::zigir::types::IrExpr::StringLiteral(_) => "[]const u8",
                    crate::zigir::types::IrExpr::BoolLiteral(_) => "bool",
                    _ => "i64",
                })
                .unwrap_or("i64")
        };

        // Emit as labeled block with ArrayList builder, matching Codegen pattern:
        // (blk: { var __arr: std.ArrayList(Type) = .empty; append...; break :blk __arr; })
        let blk = self.next_label();
        self.write(&format!(
            "({}: {{ var __arr: std.ArrayList({}) = .empty; ",
            blk, elem_type
        ));
        for (i, elem) in arr.elements.iter().enumerate() {
            if arr.spread_indices.contains(&i) {
                // Spread element: use appendSlice
                if let crate::zigir::types::IrExpr::Spread(inner) = elem {
                    self.write("__arr.appendSlice(js_allocator.allocator(), ");
                    self.emit_expr(inner);
                    self.write(".items) catch @panic(\"OOM: Array.spread\"); ");
                }
            } else {
                self.write("__arr.append(js_allocator.allocator(), ");
                self.emit_expr(elem);
                self.write(") catch @panic(\"OOM: Array.push append\"); ");
            }
        }
        self.write(&format!("break :{} __arr; }})", blk));
    }

    fn emit_object_literal(&mut self, obj: &crate::zigir::types::IrObjectLiteral) {
        use crate::zigir::types::IrObjectItem;

        // Check if any spread items exist
        let has_spread = obj
            .items
            .iter()
            .any(|item| matches!(item, IrObjectItem::Spread(_)));

        if !has_spread {
            // Pure inline properties — emit directly as .{ ... }
            self.write(".{ ");
            let mut first = true;
            for item in &obj.items {
                if let IrObjectItem::Field(field) = item {
                    if !first {
                        self.write(", ");
                    }
                    first = false;
                    if field.is_computed {
                        self.write(&format!("@\"{}\" = ", field.key)); // simplified
                    } else {
                        self.write(&format!(".{} = ", field.key));
                    }
                    self.emit_expr(&field.value);
                }
            }
            self.write(" }");
            return;
        }

        // Has spread: build a left-fold spreadMerge(...) chain.
        // Strategy (same as Codegen):
        //   { ...a }                       → a
        //   { ...a, ...b }                 → js_runtime.spreadMerge(a, b)
        //   { ...a, b: 1 }                 → js_runtime.spreadMerge(a, .{ .b = 1 })
        //   { ...a, ...b, c: 1 }           → js_runtime.spreadMerge(spreadMerge(a, b), .{ .c = 1 })

        // Collect spread expression texts
        let mut spread_texts: Vec<String> = Vec::new();
        for item in &obj.items {
            if let IrObjectItem::Spread(expr) = item {
                spread_texts.push(self.expr_to_string(expr));
            }
        }

        // Collect inline fields as .{ .key = val } string
        let inline_fields: Vec<_> = obj
            .items
            .iter()
            .filter_map(|item| {
                if let IrObjectItem::Field(f) = item {
                    Some(f)
                } else {
                    None
                }
            })
            .collect();

        let inline_text = if inline_fields.is_empty() {
            None
        } else {
            let saved_output = std::mem::take(&mut self.output);
            self.write(".{ ");
            let mut first = true;
            for field in &inline_fields {
                if !first {
                    self.write(", ");
                }
                first = false;
                if field.is_computed {
                    self.write(&format!("@\"{}\" = ", field.key));
                } else {
                    self.write(&format!(".{} = ", field.key));
                }
                self.emit_expr(&field.value);
            }
            self.write(" }");
            let text = std::mem::take(&mut self.output);
            self.output = saved_output;
            Some(text)
        };

        match (spread_texts.len(), &inline_text) {
            (0, _) => unreachable!(), // has_spread is true, so spread_texts is non-empty
            (1, None) => {
                // Single spread, no inline → identity
                self.write(&spread_texts[0]);
            }
            _ => {
                // Multi-spread or spread + inline → build spreadMerge chain
                let mut result = spread_texts[0].clone();
                for text in &spread_texts[1..] {
                    result = format!("js_runtime.spreadMerge({}, {})", result, text);
                }
                if let Some(ref inline) = inline_text {
                    result = format!("js_runtime.spreadMerge({}, {})", result, inline);
                }
                self.write(&result);
            }
        }
    }

    fn emit_template_literal(
        &mut self,
        parts: &[String],
        exprs: &[crate::zigir::types::IrExpr],
        format_specs: &[String],
    ) {
        // Zig template literal → std.fmt.allocPrint
        if exprs.is_empty() {
            // No expressions: just a string literal
            self.write(&format!("\"{}\"", escape_zig_string(&parts[0])));
        } else {
            // Build the format string: parts[0] + {spec0} + parts[1] + {spec1} + ...
            let mut fmt = String::new();
            for (i, part) in parts.iter().enumerate() {
                if i > 0 && i - 1 < format_specs.len() {
                    fmt.push_str(&format_specs[i - 1]);
                }
                fmt.push_str(&escape_zig_string(part));
            }
            // Emit args as a separate pass to get their string representations
            let arg_strs: Vec<String> = exprs
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

    fn emit_new_expr(&mut self, new_expr: &crate::zigir::types::IrNewExpr) {
        match &new_expr.constructor {
            NewConstructor::Map => {
                self.write("js_collections.JsMap.init()");
            }
            NewConstructor::Set => {
                self.write("js_collections.JsSet.init()");
            }
            NewConstructor::Date(kind) => match kind {
                DateConstructorKind::Now => {
                    self.write("js_date.JsDate.init()");
                }
                DateConstructorKind::FromMillis => {
                    self.write("js_date.JsDate.fromMillis(");
                    if let Some(arg) = new_expr.args.first() {
                        self.emit_expr(arg);
                    }
                    self.write(")");
                }
                DateConstructorKind::FromString => {
                    self.write("js_date.JsDate.fromString(");
                    if let Some(arg) = new_expr.args.first() {
                        self.emit_expr(arg);
                    }
                    self.write(")");
                }
                DateConstructorKind::FromComponents => {
                    self.write("js_date.JsDate.fromComponents(");
                    for (i, arg) in new_expr.args.iter().enumerate() {
                        if i > 0 {
                            self.write(", ");
                        }
                        self.emit_expr(arg);
                    }
                    self.write(")");
                }
            },
            NewConstructor::RegExp => {
                self.write("js_regexp.JsRegExp.init(");
                for (i, arg) in new_expr.args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.emit_expr(arg);
                }
                self.write(")");
            }
            NewConstructor::TypedArray(kind) => {
                let (module, init_fn) = typed_array_init(kind);
                self.write(&format!("{}.{}(", module, init_fn));
                if let Some(arg) = new_expr.args.first() {
                    self.emit_expr(arg);
                }
                self.write(")");
            }
            NewConstructor::Class(class_name) => {
                self.write(&format!("{}.init(", class_name));
                for (i, arg) in new_expr.args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.emit_expr(arg);
                }
                self.write(")");
            }
            NewConstructor::Error(msg) => {
                self.write(&format!("JsAny.fromError(\"{}\")", escape_zig_string(msg)));
            }
            NewConstructor::Unsupported(name) => {
                self.write(&format!(
                    "@compileError(\"new {}() is not supported\")",
                    name
                ));
            }
        }
    }
}

/// Map TypedArrayKind to (module, init_fn) for construction.
fn typed_array_init(kind: &TypedArrayKind) -> (&'static str, &'static str) {
    match kind {
        TypedArrayKind::Int8Array => ("js_typedarray", "JsInt8Array.init"),
        TypedArrayKind::Uint8Array => ("js_typedarray", "JsUint8Array.init"),
        TypedArrayKind::Uint8ClampedArray => ("js_typedarray", "JsUint8ClampedArray.init"),
        TypedArrayKind::Int16Array => ("js_typedarray", "JsInt16Array.init"),
        TypedArrayKind::Uint16Array => ("js_typedarray", "JsUint16Array.init"),
        TypedArrayKind::Int32Array => ("js_typedarray", "JsInt32Array.init"),
        TypedArrayKind::Uint32Array => ("js_typedarray", "JsUint32Array.init"),
        TypedArrayKind::Float32Array => ("js_typedarray", "JsFloat32Array.init"),
        TypedArrayKind::Float64Array => ("js_typedarray", "JsFloat64Array.init"),
        TypedArrayKind::BigInt64Array => ("js_typedarray", "JsBigInt64Array.init"),
        TypedArrayKind::BigUint64Array => ("js_typedarray", "JsBigUint64Array.init"),
    }
}

fn arrow_name_placeholder() -> String {
    "_arrow_fn".to_string()
}

// ═══════════════════════════════════════════════════════
//  Float conversion helpers for PowExpr
// ═══════════════════════════════════════════════════════

impl Emitter {
    /// Emit a float conversion for a `PowExpr` operand.
    /// - F64: emit directly
    /// - I64/BigInt: wrap in `@as(f64, @floatFromInt(...))`
    /// - Other: wrap in `@as(f64, ...)` (comptime_int, bool, etc.)
    fn emit_float_conversion(
        &mut self,
        expr: &crate::zigir::types::IrExpr,
        ty: &crate::types::ZigType,
    ) {
        match ty {
            crate::types::ZigType::F64 => {
                self.emit_expr(expr);
            }
            crate::types::ZigType::I64 | crate::types::ZigType::BigInt => {
                self.write("@as(f64, @floatFromInt(");
                self.emit_expr(expr);
                self.write("))");
            }
            _ => {
                // comptime_int, bool, or unknown — @as(f64, expr) works for comptime_int
                self.write("@as(f64, ");
                self.emit_expr(expr);
                self.write(")");
            }
        }
    }

    /// Emit a BigInt binary operation.
    /// BigInt arithmetic uses method calls like `_a.add(&_b, alloc)`.
    fn emit_bigint_binary(
        &mut self,
        op: crate::zigir::ops::BinOp,
        left: &crate::zigir::types::IrExpr,
        right: &crate::zigir::types::IrExpr,
    ) {
        use crate::zigir::ops::BinOp;

        match op {
            // Arithmetic: _a.op(&_b, alloc) catch @panic("BigInt op OOM")
            BinOp::Add => {
                self.write("(");
                self.emit_expr(left);
                self.write(".add(&");
                self.emit_expr(right);
                self.write(", js_allocator.allocator()) catch @panic(\"BigInt add OOM\"))");
            }
            BinOp::Sub => {
                self.write("(");
                self.emit_expr(left);
                self.write(".sub(&");
                self.emit_expr(right);
                self.write(", js_allocator.allocator()) catch @panic(\"BigInt sub OOM\"))");
            }
            BinOp::Mul => {
                self.write("(");
                self.emit_expr(left);
                self.write(".mul(&");
                self.emit_expr(right);
                self.write(", js_allocator.allocator()) catch @panic(\"BigInt mul OOM\"))");
            }
            BinOp::Div => {
                self.write("(");
                self.emit_expr(left);
                self.write(".div(&");
                self.emit_expr(right);
                self.write(", js_allocator.allocator()) catch @panic(\"BigInt div OOM\"))");
            }
            BinOp::Mod => {
                self.write("(");
                self.emit_expr(left);
                self.write(".rem(&");
                self.emit_expr(right);
                self.write(", js_allocator.allocator()) catch @panic(\"BigInt rem OOM\"))");
            }
            BinOp::Pow => {
                self.write("(");
                self.emit_expr(left);
                self.write(".pow(&");
                self.emit_expr(right);
                self.write(", js_allocator.allocator()) catch @panic(\"BigInt pow OOM\"))");
            }
            BinOp::BitAnd => {
                self.write("(");
                self.emit_expr(left);
                self.write(".bitwiseAnd(&");
                self.emit_expr(right);
                self.write(", js_allocator.allocator()) catch @panic(\"BigInt and OOM\"))");
            }
            BinOp::BitOr => {
                self.write("(");
                self.emit_expr(left);
                self.write(".bitwiseOr(&");
                self.emit_expr(right);
                self.write(", js_allocator.allocator()) catch @panic(\"BigInt or OOM\"))");
            }
            BinOp::BitXor => {
                self.write("(");
                self.emit_expr(left);
                self.write(".bitwiseXor(&");
                self.emit_expr(right);
                self.write(", js_allocator.allocator()) catch @panic(\"BigInt xor OOM\"))");
            }
            BinOp::Shl => {
                self.write("(");
                self.emit_expr(left);
                self.write(".shiftLeft(");
                self.emit_expr(right);
                self.write(".toU64(), js_allocator.allocator()) catch @panic(\"BigInt shl OOM\"))");
            }
            BinOp::Shr => {
                self.write("(");
                self.emit_expr(left);
                self.write(".shiftRight(");
                self.emit_expr(right);
                self.write(".toU64(), js_allocator.allocator()) catch @panic(\"BigInt shr OOM\"))");
            }
            // Equality
            BinOp::Eq | BinOp::StrictEq => {
                self.emit_expr(left);
                self.write(".eq(&");
                self.emit_expr(right);
                self.write(")");
            }
            BinOp::Ne | BinOp::StrictNe => {
                self.write("!");
                self.emit_expr(left);
                self.write(".eq(&");
                self.emit_expr(right);
                self.write(")");
            }
            // Ordering
            BinOp::Lt => {
                self.write("(");
                self.emit_expr(left);
                self.write(".order(&");
                self.emit_expr(right);
                self.write(") == .lt)");
            }
            BinOp::Le => {
                self.write("(");
                self.emit_expr(left);
                self.write(".order(&");
                self.emit_expr(right);
                self.write(") != .gt)");
            }
            BinOp::Gt => {
                self.write("(");
                self.emit_expr(left);
                self.write(".order(&");
                self.emit_expr(right);
                self.write(") == .gt)");
            }
            BinOp::Ge => {
                self.write("(");
                self.emit_expr(left);
                self.write(".order(&");
                self.emit_expr(right);
                self.write(") != .lt)");
            }
            // >>> is not supported for BigInt (JS throws TypeError)
            BinOp::UrShr => {
                self.write("@compileError(\"BigInt does not support unsigned right shift\")");
            }
            _ => {
                // Fallback: try direct operator
                self.emit_expr(left);
                self.write(&format!(" {} ", bin_op_to_zig(op)));
                self.emit_expr(right);
            }
        }
    }

    /// Emit a string comparison.
    /// String equality uses std.mem.eql, ordering uses std.mem.order.
    fn emit_string_comparison(
        &mut self,
        op: crate::zigir::ops::BinOp,
        left: &crate::zigir::types::IrExpr,
        right: &crate::zigir::types::IrExpr,
    ) {
        use crate::zigir::ops::BinOp;

        match op {
            BinOp::Eq | BinOp::StrictEq => {
                self.write("std.mem.eql(u8, ");
                self.emit_expr(left);
                self.write(", ");
                self.emit_expr(right);
                self.write(")");
            }
            BinOp::Ne | BinOp::StrictNe => {
                self.write("(!std.mem.eql(u8, ");
                self.emit_expr(left);
                self.write(", ");
                self.emit_expr(right);
                self.write("))");
            }
            BinOp::Lt => {
                self.write("(std.mem.order(u8, ");
                self.emit_expr(left);
                self.write(", ");
                self.emit_expr(right);
                self.write(") == .lt)");
            }
            BinOp::Le => {
                self.write("(std.mem.order(u8, ");
                self.emit_expr(left);
                self.write(", ");
                self.emit_expr(right);
                self.write(") != .gt)");
            }
            BinOp::Gt => {
                self.write("(std.mem.order(u8, ");
                self.emit_expr(left);
                self.write(", ");
                self.emit_expr(right);
                self.write(") == .gt)");
            }
            BinOp::Ge => {
                self.write("(std.mem.order(u8, ");
                self.emit_expr(left);
                self.write(", ");
                self.emit_expr(right);
                self.write(") != .lt)");
            }
            _ => {
                // Other string operators (not expected)
                self.emit_expr(left);
                self.write(&format!(" {} ", bin_op_to_zig(op)));
                self.emit_expr(right);
            }
        }
    }

    /// Emit a JsAny comparison operation.
    /// JsAny equality uses .eq()/.strictEq(), ordering uses .asI64().
    fn emit_jsany_comparison(
        &mut self,
        op: crate::zigir::ops::BinOp,
        left: &crate::zigir::types::IrExpr,
        right: &crate::zigir::types::IrExpr,
        left_is_jsany: bool,
        right_is_jsany: bool,
    ) {
        use crate::zigir::ops::BinOp;

        // Wrap non-JsAny operand with JsAny.from() if needed
        let emit_left_as_jsany = |emitter: &mut Emitter| {
            if left_is_jsany {
                emitter.emit_expr(left);
            } else {
                emitter.write("JsAny.from(");
                emitter.emit_expr(left);
                emitter.write(")");
            }
        };
        let emit_right_as_jsany = |emitter: &mut Emitter| {
            if right_is_jsany {
                emitter.emit_expr(right);
            } else {
                emitter.write("JsAny.from(");
                emitter.emit_expr(right);
                emitter.write(")");
            }
        };

        match op {
            BinOp::Eq => {
                emit_left_as_jsany(self);
                self.write(".eq(");
                emit_right_as_jsany(self);
                self.write(")");
            }
            BinOp::StrictEq => {
                emit_left_as_jsany(self);
                self.write(".strictEq(");
                emit_right_as_jsany(self);
                self.write(")");
            }
            BinOp::Ne => {
                self.write("!(");
                emit_left_as_jsany(self);
                self.write(".eq(");
                emit_right_as_jsany(self);
                self.write("))");
            }
            BinOp::StrictNe => {
                self.write("!(");
                emit_left_as_jsany(self);
                self.write(".strictEq(");
                emit_right_as_jsany(self);
                self.write("))");
            }
            // Ordering: convert JsAny to i64 for numeric comparison
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                let zig_op = bin_op_to_zig(op);
                // Use .asI64() on JsAny sides for numeric comparison
                if left_is_jsany {
                    self.write("(");
                    self.emit_expr(left);
                    self.write(".asI64())");
                } else {
                    self.emit_expr(left);
                }
                self.write(&format!(" {} ", zig_op));
                if right_is_jsany {
                    self.write("(");
                    self.emit_expr(right);
                    self.write(".asI64())");
                } else {
                    self.emit_expr(right);
                }
            }
            _ => {
                // Fallback
                self.emit_expr(left);
                self.write(&format!(" {} ", bin_op_to_zig(op)));
                self.emit_expr(right);
            }
        }
    }

    /// Check if an IrExpr is definitely of type `bool`.
    fn ir_expr_is_bool(expr: &crate::zigir::types::IrExpr) -> bool {
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
    fn ir_expr_is_string(expr: &crate::zigir::types::IrExpr) -> bool {
        use crate::zigir::types::IrExpr;
        matches!(expr, IrExpr::StringLiteral(_) | IrExpr::TemplateLiteral { .. })
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

// ═══════════════════════════════════════════════════════
//  BigInt, JsAny, and String comparison helpers
// ═══════════════════════════════════════════════════════
