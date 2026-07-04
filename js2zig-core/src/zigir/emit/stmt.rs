// zigir/emit/stmt.rs
// Statement-level Zig emission from IrStmt nodes.

use crate::types::ZigType;
use crate::zigir::emit::Emitter;
use crate::zigir::emit::helpers::{
    EmitterHelpers, escape_zig_string, format_param, format_return_type,
};
use crate::zigir::ident::IrIdent;
use crate::zigir::ops::AssignOp;
use crate::zigir::types::{
    IrAssignTarget, IrBlock, IrClassDecl, IrClassField, IrClassMethod, IrClosureStruct, IrFnDecl,
    IrForInKind, IrForOfKind, IrStmt, IrSwitchCase, IrTypedef, IrVarDecl,
};

// ═══════════════════════════════════════════════════════
//  Typedef emission
// ═══════════════════════════════════════════════════════

impl Emitter {
    pub(crate) fn emit_typedef(&mut self, typedef: &IrTypedef) {
        self.writeln(&format!("const {} = struct {{", typedef.name));
        self.indent_push();
        for field in &typedef.fields {
            let ty = if field.optional {
                format!("?{}", field.zig_type)
            } else {
                field.zig_type.clone()
            };
            self.writeln(&format!("{}: {},", field.name, ty));
        }
        // Generate toJson() method for non-opaque typedefs
        if typedef.has_to_json {
            self.writeln("");
            self.writeln("pub fn toJson(self: *const @This()) ![]u8 {");
            self.indent_push();
            self.writeln("var string = std.io.Writer.Allocating.init(js_allocator.allocator());");
            self.writeln("errdefer string.deinit();");
            self.writeln("try string.writer().print(\"{f}\", .{std.json.fmt(self.*, .{})});");
            self.writeln("return string.toOwnedSlice();");
            self.indent_pop();
            self.writeln("}");
        }
        self.indent_pop();
        self.writeln("};");
        self.writeln("");
    }
}

// ═══════════════════════════════════════════════════════
//  Closure struct emission
// ═══════════════════════════════════════════════════════

impl Emitter {
    pub(crate) fn emit_closure_struct(&mut self, cs: &IrClosureStruct) {
        self.writeln(&format!("const {} = struct {{", cs.name.zig_name));
        self.indent_push();

        // Captured variable fields
        for cap in &cs.captured {
            let ty_str = cap.zig_type.to_zig_type();
            if cap.is_mut {
                self.writeln(&format!("{}: *{},", cap.name.zig_name, ty_str));
            } else {
                self.writeln(&format!("{}: {},", cap.name.zig_name, ty_str));
            }
        }

        // Call method signature
        // Arrow fn (no captures): pub fn call(x: i64) i64 {
        // Closure (has captures): pub fn call(self: *@This(), x: i64) i64 {
        let mut sig = String::from("pub fn call(");
        let has_self = !cs.captured.is_empty();
        let mut need_comma = false;
        if has_self {
            sig.push_str("self: *@This()");
            need_comma = true;
        }
        for param in &cs.fn_params {
            if need_comma {
                sig.push_str(", ");
            }
            sig.push_str(&format_param(&param.name, &param.zig_type));
            need_comma = true;
        }
        sig.push_str(&format!(") {} {{", cs.return_type.to_zig_type()));
        self.writeln(&sig);

        self.indent_push();
        self.emit_block_stmts(&cs.body);
        self.indent_pop();

        self.writeln("}");
        self.indent_pop();
        self.writeln("};");
    }
}

// ═══════════════════════════════════════════════════════
//  Variable declaration emission
// ═══════════════════════════════════════════════════════

impl Emitter {
    pub(crate) fn emit_var_decl(&mut self, vd: &IrVarDecl) {
        self.write_indent();

        let kw = if vd.is_const { "const" } else { "var" };

        if vd.is_json_parse {
            // JSON.parse variable: `const name: Type = std.json.parse(Type, expr) catch @panic(...)`
            let type_name = vd
                .zig_type
                .as_ref()
                .map(|t| match t {
                    ZigType::NamedStruct(n) => n.clone(),
                    other => other.to_zig_type(),
                })
                .unwrap_or_else(|| "i64".to_string());
            self.write(&format!(
                "{} {}: {} = std.json.parse({}, ",
                kw, vd.name.zig_name, type_name, type_name
            ));
            if let Some(init) = &vd.init {
                self.emit_expr(init);
            }
            self.write(") catch @panic(\"OOM: JSON.parse alloc\")");
        } else if let Some(init) = &vd.init {
            // Has initializer
            let skip_type = vd
                .zig_type
                .as_ref()
                .is_some_and(|t| matches!(t, ZigType::NamedStruct(_) | ZigType::ArrayList(_)));
            if vd.is_const || skip_type {
                self.write(&format!("{} {} = ", kw, vd.name.zig_name));
            } else if let Some(ty) = &vd.zig_type {
                self.write(&format!(
                    "{} {}: {} = ",
                    kw,
                    vd.name.zig_name,
                    ty.to_zig_type()
                ));
            } else {
                self.write(&format!("{} {} = ", kw, vd.name.zig_name));
            }
            self.emit_expr(init);
        } else {
            // No initializer
            self.write(&format!("{} {}", kw, vd.name.zig_name));
        }

        self.write(";\n");

        // Var usage suppression for ArrayList/Map/Set
        if vd.needs_var_suppression {
            self.write_indent();
            self.write(&format!("_ = &{}; // var usage\n", vd.name.zig_name));
        }
    }
}

// ═══════════════════════════════════════════════════════
//  Function declaration emission
// ═══════════════════════════════════════════════════════

impl Emitter {
    pub(crate) fn emit_fn_decl(&mut self, fd: &IrFnDecl) {
        let name = &fd.name.zig_name;

        // Function signature — all top-level functions are `pub fn` in Zig.
        // `is_export` only controls C ABI wrapper generation, not visibility.
        self.write(&format!("pub fn {}(", name));

        for (i, param) in fd.params.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            if param.is_unused {
                // Output `_name: Type` for unused params to suppress Zig's unused-variable warning
                self.write(&format!(
                    "_{}: {}",
                    param.name.zig_name,
                    param.zig_type.to_zig_type()
                ));
            } else {
                self.write(&format_param(&param.name, &param.zig_type));
            }
        }

        let ret_type = format_return_type(&fd.return_type, fd.is_async, fd.can_throw);
        self.write(&format!(") {} {{\n", ret_type));

        // Emit `_ = _param;` for unused params at the start of the body
        self.indent_push();
        for param in &fd.params {
            if param.is_unused {
                self.writeln(&format!("_ = _{};", param.name.zig_name));
            }
        }

        // Function body
        self.emit_block_stmts(&fd.body);
        self.indent_pop();

        self.writeln("}");
        // NOTE: No trailing blank line — Codegen's emit_fn doesn't add one either.
        // Inter-declaration spacing is handled at the module level if needed.
    }
}

// ═══════════════════════════════════════════════════════
//  Class declaration emission
// ═══════════════════════════════════════════════════════

impl Emitter {
    pub(crate) fn emit_class_decl(&mut self, class: &IrClassDecl) {
        let class_name = &class.name.zig_name;

        self.writeln(&format!("const {} = struct {{", class_name));
        self.indent_push();

        // Struct fields
        for field in &class.fields {
            self.writeln(&format!(
                "{}: {},",
                field.name,
                field.zig_type.to_zig_type()
            ));
        }
        self.writeln("");

        // Constructor → init()
        if let Some(ctor) = &class.constructor {
            self.emit_class_init(class_name, ctor, &class.fields);
        } else {
            // Default init()
            self.emit_default_init(class_name, &class.fields);
        }

        // Methods
        for method in &class.methods {
            self.emit_class_method(class_name, method);
        }

        self.indent_pop();
        self.writeln("};");
        self.writeln("");
    }

    fn emit_class_init(&mut self, class_name: &str, ctor: &IrClassMethod, fields: &[IrClassField]) {
        let mut sig = "pub fn init(".to_string();
        for (i, param) in ctor.params.iter().enumerate() {
            if i > 0 {
                sig.push_str(", ");
            }
            sig.push_str(&format_param(&param.name, &param.zig_type));
        }
        sig.push_str(&format!(") {} {{", class_name));
        self.writeln(&sig);
        self.indent_push();

        // Constructor body
        self.emit_block_stmts(&ctor.body);

        // Return struct literal (from fields assigned in body)
        self.write_indent();
        self.write("return .{ ");
        for (i, field) in fields.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            self.write(&format!(".{} = {}", field.name, field.name));
        }
        self.write(" };\n");

        self.indent_pop();
        self.writeln("}");
    }

    fn emit_default_init(&mut self, class_name: &str, fields: &[IrClassField]) {
        self.writeln(&format!("pub fn init() {} {{", class_name));
        self.indent_push();
        if fields.is_empty() {
            self.writeln("return .{};");
        } else {
            self.write_indent();
            self.write("return .{ ");
            for (i, field) in fields.iter().enumerate() {
                if i > 0 {
                    self.write(", ");
                }
                let default_val = match &field.default {
                    Some(expr) => self.expr_to_string(expr),
                    None => "0".to_string(),
                };
                self.write(&format!(".{} = {}", field.name, default_val));
            }
            self.write(" };\n");
        }
        self.indent_pop();
        self.writeln("}");
    }

    fn emit_class_method(&mut self, _class_name: &str, method: &IrClassMethod) {
        let mut sig = if method.is_static {
            format!("pub fn {}(", method.name)
        } else {
            format!("pub fn {}(self: @This()", method.name)
        };

        for param in &method.params {
            sig.push_str(&format!(", {}", format_param(&param.name, &param.zig_type)));
        }
        sig.push_str(&format!(") {} {{", method.return_type.to_zig_type()));
        self.writeln(&sig);

        self.indent_push();
        self.emit_block_stmts(&method.body);
        self.indent_pop();

        self.writeln("}");
    }
}

// ═══════════════════════════════════════════════════════
//  Statement dispatch and individual statement emitters
// ═══════════════════════════════════════════════════════

impl Emitter {
    /// Emit all statements in a block.
    fn emit_block_stmts(&mut self, block: &IrBlock) {
        // Emit optional label
        if let Some(label) = &block.label {
            self.write_indent();
            self.write(&format!("{}: ", label));
        }
        for stmt in &block.stmts {
            self.emit_stmt(stmt);
        }
    }

    /// Emit a single statement.
    pub(crate) fn emit_stmt(&mut self, stmt: &IrStmt) {
        match stmt {
            IrStmt::VarDecl(var_decl) => self.emit_var_decl(var_decl),

            IrStmt::Assign { target, op, value } => {
                self.emit_assign_stmt(target, *op, value);
            }

            IrStmt::If { cond, then, else_ } => {
                self.emit_if_stmt(cond, then, else_);
            }

            IrStmt::While { cond, body, label } => {
                self.emit_while_stmt(cond, body, label);
            }

            IrStmt::DoWhile { body, cond, label } => {
                self.emit_do_while_stmt(body, cond, label);
            }

            IrStmt::For {
                init,
                cond,
                update,
                body,
                label,
            } => {
                self.emit_for_stmt(init, cond, update, body, label);
            }

            IrStmt::ForIn {
                var,
                iterable,
                body,
                kind,
                label,
            } => {
                self.emit_for_in_stmt(var, iterable, body, kind, label);
            }

            IrStmt::ForOf {
                var,
                destructure_vars,
                iterable,
                iterable_is_arraylist,
                body,
                kind,
                is_async,
                label,
            } => {
                self.emit_for_of_stmt(
                    var,
                    destructure_vars,
                    iterable,
                    *iterable_is_arraylist,
                    body,
                    kind,
                    *is_async,
                    label,
                );
            }

            IrStmt::Switch { expr, cases } => {
                self.emit_switch_stmt(expr, cases);
            }

            IrStmt::Try {
                try_block,
                catch_var,
                catch_var_referenced,
                catch_block,
                finally,
                has_throw,
                has_nested_try,
            } => {
                self.emit_try_stmt(
                    try_block,
                    catch_var,
                    *catch_var_referenced,
                    finally,
                    *has_throw,
                    *has_nested_try,
                    catch_block,
                );
            }

            IrStmt::Throw { value } => {
                // Evaluate throw value for side effects, then break/return error.
                self.write_indent();
                self.write("_ = ");
                self.emit_expr(value);
                self.write(";\n");

                let try_label = self.inside_try_block.clone();
                if let Some(label) = try_label {
                    // Inside try block: break to the labeled block with error
                    self.writeln(&format!(
                        "break :{} @as(anyerror!void, error.JsThrow);",
                        label,
                    ));
                } else {
                    // Outside try block: return error from function
                    self.writeln("return error.JsThrow;");
                }
            }

            IrStmt::Return { value } => {
                self.write_indent();
                if let Some(val) = value {
                    self.write("return ");
                    self.emit_expr(val);
                    self.write(";\n");
                } else {
                    self.write("return;\n");
                }
            }

            IrStmt::Break { label } => {
                self.write_indent();
                if let Some(l) = label {
                    self.write(&format!("break :{};\n", l));
                } else {
                    self.write("break;\n");
                }
            }

            IrStmt::Continue { label } => {
                self.write_indent();
                if let Some(l) = label {
                    self.write(&format!("continue :{};\n", l));
                } else {
                    self.write("continue;\n");
                }
            }

            IrStmt::Expr(expr) => {
                self.write_indent();
                // Assignment and Update (is_expr_stmt=true) don't need `_ = ` prefix
                // — they are statements in Zig. Other expressions need `_ = ` to
                // discard non-void return values.
                let needs_discard = !matches!(
                    expr,
                    crate::zigir::types::IrExpr::Assign { .. }
                        | crate::zigir::types::IrExpr::Update {
                            is_expr_stmt: true,
                            ..
                        }
                );
                if needs_discard {
                    self.write("_ = ");
                }
                self.emit_expr(expr);
                self.write(";\n");
            }

            IrStmt::Block(block) => {
                self.writeln("{");
                self.indent_push();
                self.emit_block_stmts(block);
                self.indent_pop();
                self.writeln("}");
            }

            IrStmt::CompileError { span, msg } => {
                let loc = format!("{}:{}", span.js_line, span.js_col);
                self.writeln(&format!(
                    "@compileError(\"{} (at {})\");",
                    escape_zig_string(msg),
                    loc
                ));
            }

            IrStmt::Comment(text) => {
                self.writeln(&format!("// {}", text));
            }
        }
    }

    // ── Individual statement emitters ────────────────

    fn emit_assign_stmt(
        &mut self,
        target: &IrAssignTarget,
        op: AssignOp,
        value: &crate::zigir::types::IrExpr,
    ) {
        self.write_indent();
        if op == AssignOp::Mod {
            // Zig doesn't support % on signed integers; use x = @rem(x, y)
            self.emit_assign_target(target);
            self.write(" = @rem(");
            self.emit_assign_target(target);
            self.write(", ");
            self.emit_expr(value);
            self.write(")");
        } else if op == AssignOp::Div {
            // Zig division on signed integers requires @divTrunc
            self.emit_assign_target(target);
            self.write(" = @divTrunc(");
            self.emit_assign_target(target);
            self.write(", ");
            self.emit_expr(value);
            self.write(")");
        } else {
            self.emit_assign_target(target);
            self.write(&format!(" {} ", op.to_zig_str()));
            self.emit_expr(value);
        }
        self.write(";\n");
    }

    fn emit_assign_target(&mut self, target: &IrAssignTarget) {
        match target {
            IrAssignTarget::Ident(ident) => {
                self.write(&ident.zig_name);
            }
            IrAssignTarget::Member {
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
            IrAssignTarget::Index { object, index } => {
                self.emit_expr(object);
                self.write("[");
                self.emit_expr(index);
                self.write("]");
            }
            IrAssignTarget::Destructure(bindings) => {
                // Destructuring assignment: emit as multiple statements
                // This case is handled differently — the parent should emit each binding separately
                for (i, binding) in bindings.iter().enumerate() {
                    if i > 0 {
                        self.write_indent();
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

    fn emit_if_stmt(
        &mut self,
        cond: &crate::zigir::types::IrExpr,
        then: &IrBlock,
        else_: &Option<IrBlock>,
    ) {
        self.write_indent();
        self.write("if (");
        self.emit_expr(cond);
        self.write(") {\n");
        self.indent_push();
        self.emit_block_stmts(then);
        self.indent_pop();
        if let Some(else_block) = else_ {
            // Check if the else block is a single if statement (else-if chain)
            if else_block.stmts.len() == 1
                && let IrStmt::If {
                    cond: inner_cond,
                    then: inner_then,
                    else_: inner_else,
                } = &else_block.stmts[0]
            {
                self.write_indent();
                self.write("} else if (");
                self.emit_expr(inner_cond);
                self.write(") {\n");
                self.indent_push();
                self.emit_block_stmts(inner_then);
                self.indent_pop();
                // Recurse for the inner else
                self.emit_else_chain(inner_else);
                return;
            }
            self.writeln("} else {");
            self.indent_push();
            self.emit_block_stmts(else_block);
            self.indent_pop();
        }
        self.writeln("}");
    }

    /// Recursively emit else / else-if chains.
    fn emit_else_chain(&mut self, else_: &Option<IrBlock>) {
        if let Some(else_block) = else_ {
            // Check for else-if chain
            if else_block.stmts.len() == 1
                && let IrStmt::If {
                    cond: inner_cond,
                    then: inner_then,
                    else_: inner_else,
                } = &else_block.stmts[0]
            {
                self.write_indent();
                self.write("} else if (");
                self.emit_expr(inner_cond);
                self.write(") {\n");
                self.indent_push();
                self.emit_block_stmts(inner_then);
                self.indent_pop();
                self.emit_else_chain(inner_else);
                return;
            }
            self.writeln("} else {");
            self.indent_push();
            self.emit_block_stmts(else_block);
            self.indent_pop();
        }
        self.writeln("}");
    }

    fn emit_while_stmt(
        &mut self,
        cond: &crate::zigir::types::IrExpr,
        body: &IrBlock,
        label: &Option<String>,
    ) {
        self.write_indent();
        if let Some(lbl) = label {
            self.write(&format!("{}: ", lbl));
        }
        self.write("while (");
        self.emit_expr(cond);
        self.write(") {\n");
        self.indent_push();
        self.emit_block_stmts(body);
        self.indent_pop();
        self.writeln("}");
    }

    fn emit_do_while_stmt(
        &mut self,
        body: &IrBlock,
        cond: &crate::zigir::types::IrExpr,
        label: &Option<String>,
    ) {
        self.write_indent();
        if let Some(lbl) = label {
            self.write(&format!("{}: ", lbl));
        }
        // Zig doesn't have do-while; use `while (true)` with break at end
        self.write("while (true) {\n");
        self.indent_push();
        self.emit_block_stmts(body);
        self.write_indent();
        self.write("if (");
        self.emit_expr(cond);
        self.write(") {} else { break; }\n");
        self.indent_pop();
        self.writeln("}");
    }

    fn emit_for_stmt(
        &mut self,
        init: &Option<Box<IrStmt>>,
        cond: &Option<crate::zigir::types::IrExpr>,
        update: &Option<Box<IrStmt>>,
        body: &IrBlock,
        label: &Option<String>,
    ) {
        // Codegen wraps the entire for loop in a block scope: { init; while (cond) : ({ update; }) { body } }
        self.write_indent();
        if let Some(lbl) = label {
            self.write(&format!("{}: ", lbl));
        }
        self.write("{\n");
        self.indent_push();

        // Emit init statement
        if let Some(init_stmt) = init {
            self.emit_stmt(init_stmt);
        }

        // While loop with optional update continuation
        self.write_indent();
        self.write("while (");
        if let Some(c) = cond {
            self.emit_expr(c);
        } else {
            self.write("true");
        }
        self.write(")");

        // Zig while continuation syntax for the update expression
        if let Some(update_stmt) = update {
            self.write(" : ({ ");
            // Emit the update as an expression-like statement inside the continuation
            match update_stmt.as_ref() {
                IrStmt::Expr(expr) => {
                    self.emit_expr(expr);
                }
                IrStmt::Assign { target, op, value } => {
                    self.emit_assign_target(target);
                    self.write(&format!(" {} ", op.to_zig_str()));
                    self.emit_expr(value);
                }
                _ => {
                    // Fallback: emit the full statement (rare)
                    self.emit_stmt(update_stmt);
                }
            }
            self.write("; })");
        }

        self.write(" {\n");
        self.indent_push();
        self.emit_block_stmts(body);
        self.indent_pop();

        self.write_indent();
        self.write("}\n");

        // Close the enclosing block scope
        self.indent_pop();
        self.writeln("}");
    }

    fn emit_for_in_stmt(
        &mut self,
        var: &IrIdent,
        iterable: &crate::zigir::types::IrExpr,
        body: &IrBlock,
        kind: &IrForInKind,
        label: &Option<String>,
    ) {
        match kind {
            IrForInKind::HashMapIter => {
                // `var __it = obj.iterator(); while (__it.next()) |__kv| { const var = __kv.key_ptr.*; ... }`
                self.write_indent();
                if let Some(lbl) = label {
                    self.write(&format!("{}: ", lbl));
                }
                self.write("var ");
                let it_name = "__it";
                self.write(it_name);
                self.write(" = ");
                self.emit_expr(iterable);
                self.write(".iterator();\n");
                // The while loop is NOT labeled separately — the label is on
                // the outer `var __it` statement for HashMapIter. In Codegen,
                // the label wraps `{ var __it = ... while ... }` as a single block.
                // We match that by emitting the while at the same level.
                self.write_indent();
                self.write("while (");
                self.write(it_name);
                self.write(".next()) |__kv| {\n");
                self.indent_push();
                self.writeln(&format!("const {} = __kv.key_ptr.*;", var.zig_name));
                self.emit_block_stmts(body);
                self.indent_pop();
                self.writeln("}");
            }
            IrForInKind::StructUnroll { fields } => {
                // Unrolled: one iteration per field; label on first iteration only
                for (i, field_name) in fields.iter().enumerate() {
                    self.write_indent();
                    if i == 0
                        && let Some(lbl) = label
                    {
                        self.write(&format!("{}: ", lbl));
                    }
                    self.write("{\n");
                    self.indent_push();
                    self.writeln(&format!("const {} = \"{}\";", var.zig_name, field_name));
                    self.emit_block_stmts(body);
                    self.indent_pop();
                    self.writeln("}");
                }
            }
            IrForInKind::Unsupported => {
                if let Some(lbl) = label {
                    self.write_indent();
                    self.write(&format!(
                        "{}: @compileError(\"for-in on this type is not supported\");\n",
                        lbl
                    ));
                } else {
                    self.writeln("@compileError(\"for-in on this type is not supported\");");
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_for_of_stmt(
        &mut self,
        var: &IrIdent,
        destructure_vars: &[IrIdent],
        iterable: &crate::zigir::types::IrExpr,
        iterable_is_arraylist: bool,
        body: &IrBlock,
        kind: &IrForOfKind,
        is_async: bool,
        label: &Option<String>,
    ) {
        if is_async {
            self.writeln("@compileError(\"for-await-of is not supported\");");
            return;
        }

        match kind {
            IrForOfKind::Array => {
                self.write_indent();
                if let Some(lbl) = label {
                    self.write(&format!("{}: ", lbl));
                }
                self.write("for (");
                self.emit_expr(iterable);
                if iterable_is_arraylist {
                    self.write(".items");
                }
                self.write(") |");
                self.write(&var.zig_name);
                self.write("| {\n");
                self.indent_push();
                self.emit_block_stmts(body);
                self.indent_pop();
                self.writeln("}");
            }
            IrForOfKind::MapSetIter { is_map } => {
                self.write_indent();
                if let Some(lbl) = label {
                    self.write(&format!("{}: ", lbl));
                }
                let it_name = "__it";
                self.write("var ");
                self.write(it_name);
                self.write(" = ");
                self.emit_expr(iterable);
                self.write(".inner.iterator();\n");
                self.write_indent();
                self.write("while (");
                self.write(it_name);
                self.write(".next()) |__kv| {\n");
                self.indent_push();
                if *is_map && !destructure_vars.is_empty() {
                    // Map destructure: const [key, val] = ...
                    self.writeln(&format!(
                        "const {} = __kv.key_ptr.*;",
                        destructure_vars[0].zig_name
                    ));
                    if destructure_vars.len() > 1 {
                        self.writeln(&format!(
                            "const {} = __kv.value_ptr.*;",
                            destructure_vars[1].zig_name
                        ));
                    }
                } else {
                    self.writeln(&format!("const {} = __kv.key_ptr.*;", var.zig_name));
                }
                self.emit_block_stmts(body);
                self.indent_pop();
                self.writeln("}");
            }
            IrForOfKind::AsyncUnsupported => {
                self.writeln("@compileError(\"for-await-of is not supported\");");
            }
        }
    }

    fn emit_switch_stmt(&mut self, expr: &crate::zigir::types::IrExpr, cases: &[IrSwitchCase]) {
        self.write_indent();
        self.write("switch (");
        self.emit_expr(expr);
        self.write(") {\n");
        self.indent_push();
        let mut has_default = false;
        for case in cases {
            self.write_indent();
            if let Some(test) = &case.test {
                self.emit_expr(test);
            } else {
                has_default = true;
                self.write("else");
            }
            self.write(" => {\n");
            self.indent_push();
            for stmt in &case.body {
                self.emit_stmt(stmt);
            }
            self.indent_pop();
            self.write_indent();
            self.write("},\n");
        }
        // Zig switch must be exhaustive; add empty else if no default
        if !has_default {
            self.writeln("else => {},");
        }
        self.indent_pop();
        self.writeln("}");
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_try_stmt(
        &mut self,
        try_block: &IrBlock,
        catch_var: &Option<IrIdent>,
        catch_var_referenced: bool,
        finally: &Option<IrBlock>,
        has_throw: bool,
        has_nested_try: bool,
        catch_block: &IrBlock,
    ) {
        let needs_catch = catch_var.is_some();

        // ── B1: No throw, no catch, no nested try → inline body + finally ──
        if !has_throw && !needs_catch && !has_nested_try {
            self.emit_block_stmts(try_block);
            if let Some(finally_block) = finally {
                self.emit_block_stmts(finally_block);
            }
            return;
        }

        // ── B2: Catch present but no throw, no nested try → inline body + finally, drop catch ──
        if !has_throw && !has_nested_try {
            self.emit_block_stmts(try_block);
            if let Some(finally_block) = finally {
                self.emit_block_stmts(finally_block);
            }
            return;
        }

        // ── Case A: Has throw or nested try → full labeled block pattern ──
        // Mirrors Codegen's double labeled-block pattern:
        //   outer blk (_js_try_blk_N): scope for defer (finally) + catch dispatch
        //   inner body blk (_js_try_body_blk_N): scope for throw → break to catch
        let label_id = self.next_try_label();
        let blk_label = format!("_js_try_blk_{}", label_id);
        let result_var = format!("_js_try_{}", label_id);
        let body_blk_label = format!("_js_try_body_blk_{}", label_id);
        let body_result_var = format!("_js_try_body_{}", label_id);

        // Save current inside_try_block so we can restore it later
        let saved_inside = self.inside_try_block.clone();

        // ── Outer labeled block ──
        self.write_indent();
        self.write(&format!(
            "const {}: anyerror!void = {}: {{\n",
            result_var, blk_label,
        ));
        self.indent_push();

        // ── Finally as defer (always runs, inside labeled block) ──
        if let Some(finally_block) = finally {
            self.writeln("defer {");
            self.indent_push();
            self.emit_block_stmts(finally_block);
            self.indent_pop();
            self.writeln("}");
        }

        // ── Inner body labeled block ──
        self.write_indent();
        self.write(&format!(
            "const {}: anyerror!void = {}: {{\n",
            body_result_var, body_blk_label,
        ));
        self.indent_push();

        // Set inside_try_block so that throw statements emit
        // break :body_blk_label error.JsThrow
        self.inside_try_block = Some(body_blk_label.clone());

        self.emit_block_stmts(try_block);

        // Normal completion of try body (no throw).
        // Skip if the try body ends with a return/break/continue/throw —
        // execution never reaches this point.
        let body_always_exits = try_block.stmts.last().is_some_and(|s| {
            matches!(
                s,
                IrStmt::Return { .. }
                    | IrStmt::Throw { .. }
                    | IrStmt::Break { .. }
                    | IrStmt::Continue { .. }
            )
        });
        if !body_always_exits {
            self.write_indent();
            self.write(&format!("break :{} {{}};\n", body_blk_label));
        }

        self.indent_pop();
        self.writeln("};");

        // ── Catch dispatch ──
        if needs_catch {
            self.writeln(&format!("if ({}) |_| {{", body_result_var));
            // success path: nothing to do
            self.writeln("} else |err| {");
            self.indent_push();
            if let Some(var) = catch_var {
                if catch_var_referenced {
                    self.writeln(&format!("const {} = @errorName(err);", var.zig_name));
                } else {
                    self.writeln("_ = @errorName(err);");
                }
            }
            // Set inside_try_block to outer label so re-throw in catch body
            // produces break :blk_label error.JsThrow
            self.inside_try_block = Some(blk_label.clone());
            self.emit_block_stmts(catch_block);
            self.indent_pop();
            self.writeln("}");
        }

        // Normal completion of outer block
        self.write_indent();
        self.write(&format!("break :{} {{}};\n", blk_label));

        self.indent_pop();
        self.writeln("};");

        // ── Propagate unhandled re-throw ──
        // If the result is an error (re-throw from catch), propagate it.
        if needs_catch {
            if let Some(ref parent_label) = saved_inside {
                // Inside another try block → break to parent
                self.writeln(&format!(
                    "if ({0}) |_| {{}} else |_| break :{1} @as(anyerror!void, error.JsThrow);",
                    result_var, parent_label,
                ));
            } else {
                // Top-level → return error
                self.writeln(&format!(
                    "if ({}) |_| {{}} else |_| return error.JsThrow;",
                    result_var,
                ));
            }
        }

        // Restore saved inside_try_block
        self.inside_try_block = saved_inside;
    }
}
