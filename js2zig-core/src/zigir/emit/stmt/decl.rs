// zigir/emit/stmt/decl.rs
// Typedef, closure, variable, function, and class declaration emission.

use crate::types::ZigType;
use crate::zigir::emit::Emitter;
use crate::zigir::emit::helpers::{EmitterHelpers, format_param_with_rest, format_return_type};
use crate::zigir::types::{
    IrClassDecl, IrClassField, IrClassMethod, IrClosureStruct, IrFnDecl, IrTypedef, IrVarDecl,
};

impl Emitter {
    pub(crate) fn emit_typedef(&mut self, typedef: &IrTypedef) {
        self.writeln(&format!("const {} = struct {{", typedef.name));
        self.indent_push();
        for field in &typedef.fields {
            // Note: zig_type already includes the '?' prefix for optional fields
            // (set in lower/mod.rs), so we must NOT add another '?' here.
            self.writeln(&format!("{}: {},", field.name, field.zig_type));
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
            sig.push_str(&format_param_with_rest(
                &param.name,
                &param.zig_type,
                param.is_rest,
            ));
            need_comma = true;
        }
        // Return type: use @TypeOf(expr) for AnytypeReturn, else normal type
        let ret_type_str = if matches!(cs.return_type, ZigType::AnytypeReturn) {
            if let Some(ref body_expr) = cs.typeof_return_body {
                let inline = Self::emit_expr_inline(body_expr);
                format!("@TypeOf({})", inline.replace("try ", ""))
            } else {
                cs.return_type.to_zig_type()
            }
        } else {
            cs.return_type.to_zig_type()
        };
        sig.push_str(&format!(") {} {{", ret_type_str));
        self.writeln(&sig);

        self.indent_push();
        self.emit_block_stmts_unlabeled(&cs.body);
        self.indent_pop();

        self.writeln("}");
        self.indent_pop();
        self.writeln("};");
    }

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

    pub(crate) fn emit_fn_decl(&mut self, fd: &IrFnDecl) {
        let name = &fd.name.zig_name;

        // Function signature — all top-level functions are `pub fn` in Zig.
        // `is_export` only controls C ABI wrapper generation, not visibility.
        self.write(&format!("pub fn {}(", name));

        for (i, param) in fd.params.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            let type_str = if param.is_rest {
                "[]const JsAny".to_string()
            } else {
                param.zig_type.to_zig_type()
            };
            if param.is_unused {
                // Output `_name: Type` for unused params to suppress Zig's unused-variable warning
                self.write(&format!("_{}: {}", param.name.zig_name, type_str));
            } else {
                self.write(&format!("{}: {}", param.name.zig_name, type_str));
            }
        }

        let ret_type = if matches!(fd.return_type, ZigType::AnytypeReturn) {
            // Generate @TypeOf(first_return_expr) instead of literal "anytype"
            if let Some(ref body_expr) = fd.typeof_return_body {
                let captured = self.expr_to_string(body_expr);
                // Strip 'try ' prefixes — try is not valid in @TypeOf (comptime type expression)
                let stripped = captured.replace("try ", "");
                let base = format!("@TypeOf({})", stripped);
                if fd.is_async || fd.can_throw {
                    format!("!{}", base)
                } else {
                    base
                }
            } else {
                // Fallback: no return expression found, use void
                if fd.can_throw {
                    "!void".to_string()
                } else {
                    "void".to_string()
                }
            }
        } else {
            format_return_type(&fd.return_type, fd.is_async, fd.can_throw)
        };
        self.write(&format!(") {} {{\n", ret_type));

        // Emit `_ = _param;` for unused params at the start of the body
        self.indent_push();
        for param in &fd.params {
            if param.is_unused {
                self.writeln(&format!("_ = _{};", param.name.zig_name));
            }
        }

        // Function body
        self.emit_block_stmts_unlabeled(&fd.body);
        self.indent_pop();

        self.writeln("}");
        // NOTE: No trailing blank line.
        // Inter-declaration spacing is handled at the module level if needed.
    }

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

        // Static initialization blocks — emitted as top-level code after struct definition.
        // Each `static { ... }` block is wrapped in a const declaration so it's valid
        // Zig at module scope: `const _: void = blk: { ... break :blk {}; };`
        for block in &class.static_blocks {
            if !block.stmts.is_empty() {
                self.write("const _: void = blk: { ");
                for stmt in &block.stmts {
                    self.emit_stmt(stmt);
                }
                self.writeln("break :blk {}; };");
            }
        }
    }

    pub(super) fn emit_class_init(
        &mut self,
        class_name: &str,
        ctor: &IrClassMethod,
        fields: &[IrClassField],
    ) {
        let mut sig = "pub fn init(".to_string();
        for (i, param) in ctor.params.iter().enumerate() {
            if i > 0 {
                sig.push_str(", ");
            }
            sig.push_str(&format_param_with_rest(
                &param.name,
                &param.zig_type,
                param.is_rest,
            ));
        }
        sig.push_str(&format!(") {} {{", class_name));
        self.writeln(&sig);
        self.indent_push();

        // Constructor body
        self.emit_block_stmts_unlabeled(&ctor.body);

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

    pub(super) fn emit_default_init(&mut self, class_name: &str, fields: &[IrClassField]) {
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

    pub(super) fn emit_class_method(&mut self, _class_name: &str, method: &IrClassMethod) {
        let mut sig = if method.is_static {
            format!("pub fn {}(", method.name)
        } else {
            format!("pub fn {}(self: @This()", method.name)
        };

        for param in &method.params {
            sig.push_str(&format!(
                ", {}",
                format_param_with_rest(&param.name, &param.zig_type, param.is_rest)
            ));
        }
        sig.push_str(&format!(") {} {{", method.return_type.to_zig_type()));
        self.writeln(&sig);

        self.indent_push();
        self.emit_block_stmts_unlabeled(&method.body);
        self.indent_pop();

        self.writeln("}");
    }
}
