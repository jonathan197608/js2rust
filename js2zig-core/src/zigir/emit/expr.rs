// zigir/emit/expr.rs
// Expression-level Zig emission from IrExpr nodes.

use crate::zigir::emit::Emitter;
use crate::zigir::emit::helpers::{
    EmitterHelpers, assign_op_to_zig, bin_op_to_zig, escape_zig_string, format_param,
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
            crate::zigir::types::IrExpr::Binary { op, left, right } => {
                self.emit_expr(left);
                self.write(&format!(" {} ", bin_op_to_zig(*op)));
                self.emit_expr(right);
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
                // Zig `and`/`or` require bool operands. Codegen always wraps
                // logical expressions in parentheses.
                self.write("(");
                self.emit_expr(left);
                self.write(&format!(" {} ", logical_op_to_zig(*op)));
                self.emit_expr(right);
                self.write(")");
            }

            crate::zigir::types::IrExpr::Update {
                op,
                target,
                is_expr_stmt,
            } => {
                if *is_expr_stmt {
                    // Statement context: `i += 1`
                    self.emit_assign_target_inner(target);
                    self.write(&format!(" {} ", update_op_to_zig(*op)));
                } else {
                    // Expression context: `({blk}: { ... break :blk old_val })`
                    self.write("(");
                    self.emit_assign_target_inner(target);
                    self.write(&format!(" {} ", update_op_to_zig(*op)));
                    self.write(")");
                }
            }

            crate::zigir::types::IrExpr::Assign { op, target, value } => {
                self.emit_assign_target_inner(target);
                self.write(&format!(" {} ", assign_op_to_zig(*op)));
                self.emit_expr(value);
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
                    sig.push_str(&format_param(&param.name, &param.zig_type));
                }
                sig.push_str(&format!(") {} {{", ret));
                self.writeln(&sig);
                self.indent_push();
                // Body
                if arrow.is_concise {
                    self.write_indent();
                    self.write("return ");
                    if let Some(stmt) = arrow.body.stmts.first() {
                        if let crate::zigir::types::IrStmt::Expr(e) = stmt {
                            self.emit_expr(e);
                        }
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
                self.emit_expr(cond);
                self.write(") ");
                self.emit_expr(then);
                self.write(" else ");
                self.emit_expr(else_);
            }

            crate::zigir::types::IrExpr::TemplateLiteral { parts, exprs } => {
                self.emit_template_literal(parts, exprs);
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
                self.write("(\"undefined\")"); // simplified
                let _ = inner;
            }

            crate::zigir::types::IrExpr::Void(inner) => {
                self.write("void{}"); // simplified
                let _ = inner;
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

            crate::zigir::types::IrExpr::CompileError { span: _, msg } => {
                self.write(&format!("@compileError(\"{}\")", escape_zig_string(msg)));
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
            self.emit_expr(arg);
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
        if arr.spread_indices.is_empty() {
            // Simple array literal
            self.write(
                "&.{
",
            );
            for (i, elem) in arr.elements.iter().enumerate() {
                if i > 0 {
                    self.write(", ");
                }
                self.emit_expr(elem);
            }
            self.write("}");
        } else {
            // Array with spread: need ArrayList init + appendSlice
            // This is simplified — full impl would use ArrayList builder
            self.write(
                "&.{
",
            );
            for (i, elem) in arr.elements.iter().enumerate() {
                if i > 0 {
                    self.write(", ");
                }
                if arr.spread_indices.contains(&i) {
                    // Spread element
                } else {
                    self.emit_expr(elem);
                }
            }
            self.write("}");
        }
    }

    fn emit_object_literal(&mut self, obj: &crate::zigir::types::IrObjectLiteral) {
        self.write(
            ".{
",
        );
        for (i, field) in obj.fields.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            if field.is_computed {
                self.write(&format!("@\"{}\"", field.key)); // simplified
            } else {
                self.write(&format!(".{} = ", field.key));
            }
            self.emit_expr(&field.value);
        }
        if !obj.spreads.is_empty() {
            for spread in &obj.spreads {
                self.write(", ");
                self.emit_expr(spread);
            }
        }
        self.write("}");
    }

    fn emit_template_literal(&mut self, parts: &[String], exprs: &[crate::zigir::types::IrExpr]) {
        // Zig string concatenation or std.fmt.format
        if exprs.is_empty() {
            // No expressions: just a string literal
            self.write(&format!("\"{}\"", escape_zig_string(&parts[0])));
        } else {
            // Has expressions: use std.fmt.format
            self.write("std.fmt.format(\"");
            for part in parts {
                self.write(&escape_zig_string(part));
            }
            self.write(
                "\", .{
",
            );
            for (i, expr) in exprs.iter().enumerate() {
                if i > 0 {
                    self.write(", ");
                }
                self.emit_expr(expr);
            }
            self.write("})");
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
                    self.write("js_date.JsDate.now()");
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
