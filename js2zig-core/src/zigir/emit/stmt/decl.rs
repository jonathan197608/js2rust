// zigir/emit/stmt/decl.rs
// Typedef, closure, variable, function, and class declaration emission.

use crate::types::ZigType;
use crate::zigir::emit::Emitter;
use crate::zigir::emit::helpers::{EmitterHelpers, format_param_with_rest, format_return_type};
use crate::zigir::types::{
    IrClassDecl, IrClassField, IrClassMethod, IrClosureStruct, IrFnDecl, IrStmt, IrTypedef,
    IrVarDecl,
};

impl Emitter {
    /// Resolve the return type string for a function/closure that may use `AnytypeReturn`.
    ///
    /// When the return type is `AnytypeReturn`, generates `@TypeOf(first_return_expr)`
    /// (stripping `try` prefixes which are invalid in comptime type expressions).
    /// For non-AnytypeReturn types, delegates to `format_return_type`.
    pub(super) fn resolve_anytype_return(
        &mut self,
        return_type: &ZigType,
        typeof_return_body: &Option<Box<crate::zigir::types::IrExpr>>,
        is_async: bool,
        can_throw: bool,
    ) -> String {
        if matches!(return_type, ZigType::AnytypeReturn) {
            if let Some(body_expr) = typeof_return_body {
                let captured = self.expr_to_string(body_expr);
                let stripped = captured.replace("try ", "");
                let base = format!("@TypeOf({})", stripped);
                if is_async || can_throw {
                    format!("!{}", base)
                } else {
                    base
                }
            } else {
                // Fallback: no return expression found
                if can_throw {
                    "!void".to_string()
                } else {
                    "void".to_string()
                }
            }
        } else {
            format_return_type(return_type, is_async, can_throw)
        }
    }

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

        // When the body is only @compileError, prefix params with `_` to suppress
        // Zig's "unused function parameter" error — the function will never run.
        let body_is_compile_error = cs
            .body
            .stmts
            .iter()
            .all(|s| matches!(s, IrStmt::CompileError { .. }));

        // Call method signature
        // Arrow fn (no captures): pub fn call(x: i64) i64 {
        // Closure (has captures): pub fn call(self: *@This(), x: i64) i64 {
        let mut sig = String::from("pub fn call(");
        let has_self = !cs.captured.is_empty();
        let mut need_comma = false;
        if has_self {
            if body_is_compile_error {
                sig.push_str("_: *@This()");
            } else {
                sig.push_str("self: *@This()");
            }
            need_comma = true;
        }
        for param in &cs.fn_params {
            if need_comma {
                sig.push_str(", ");
            }
            if body_is_compile_error {
                // Use bare `_` to suppress "unused parameter" error in Zig
                let type_str = if param.is_rest {
                    "[]const JsAny".to_string()
                } else {
                    param.zig_type.to_zig_type().into_owned()
                };
                sig.push_str(&format!("_: {}", type_str));
            } else {
                sig.push_str(&format_param_with_rest(
                    &param.name,
                    &param.zig_type,
                    param.is_rest,
                ));
            }
            need_comma = true;
        }
        // Return type: use @TypeOf(expr) for AnytypeReturn, else normal type
        let ret_type_str =
            self.resolve_anytype_return(&cs.return_type, &cs.typeof_return_body, false, false);
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
                    other => other.to_zig_type().into_owned(),
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

        // Var usage suppression for ArrayList/Map/Set variables
        // whose value may not be read after compilation transforms.
        if vd.needs_var_suppression {
            self.write_indent();
            self.write(&format!("_ = &{}; // var usage\n", vd.name.zig_name));
        }

        // Auto-cleanup: defer deinit for Map/Set/BigInt variables and class instances
        // that contain Map/Set/ArrayList/BigInt fields, to prevent memory leaks.
        if vd.needs_deinit
            || matches!(&vd.zig_type, Some(ZigType::NamedStruct(name))
                if name != "Map" && name != "Set" && name != "JsDate"
                    && name != "JsBigInt" && name != "JsRegExp"
                    && name != "Error" && name != "JsError"
                    && self.class_needs_deinit.contains(name))
            || matches!(&vd.zig_type, Some(ZigType::BigInt))
        {
            self.write_indent();
            self.write(&format!(
                "defer {}.deinit(js_allocator.allocator()); // auto-cleanup\n",
                vd.name.zig_name
            ));
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
                param.zig_type.to_zig_type().into_owned()
            };
            if param.is_unused {
                // Output `_name: Type` for unused params to suppress Zig's unused-variable warning
                self.write(&format!("_{}: {}", param.name.zig_name, type_str));
            } else {
                self.write(&format!("{}: {}", param.name.zig_name, type_str));
            }
        }

        let ret_type = self.resolve_anytype_return(
            &fd.return_type,
            &fd.typeof_return_body,
            fd.is_async,
            fd.can_throw,
        );
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

        // Register class in needs_deinit set if it contains Map/Set/ArrayList fields.
        if class.needs_deinit {
            self.class_needs_deinit.insert(class_name.clone());
        }

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

        // deinit() method — generated when any field is Map, Set, or ArrayList.
        if class.needs_deinit {
            self.writeln("");
            self.writeln("pub fn deinit(self: *@This(), alloc: std.mem.Allocator) void {");
            self.indent_push();
            for field in &class.fields {
                let needs_field_deinit = matches!(
                    &field.zig_type,
                ZigType::NamedStruct(n) if n == "Map" || n == "Set"
                ) || matches!(&field.zig_type, ZigType::ArrayList(_))
                    || matches!(&field.zig_type, ZigType::BigInt);
                if needs_field_deinit {
                    self.writeln(&format!("self.{}.deinit(alloc);", field.name));
                }
            }
            self.indent_pop();
            self.writeln("}");
        }

        self.indent_pop();
        self.writeln("};");
        self.writeln("");

        // Static field initializers — emitted as module-scope `var` declarations.
        // e.g. `static x = 1` → `var __ClassName_x: i64 = 1;`
        for (field_name, init_expr, field_ty) in &class.static_inits {
            let var_name = format!("__{}_{}", class_name, field_name);
            self.write(&format!("var {}: ", var_name));
            self.write(&format!("{} = ", field_ty.to_zig_type()));
            self.emit_expr(init_expr);
            self.writeln(";");

            // Register static Map/Set/ArrayList/BigInt fields for deinit in deinit_js2rust()
            let needs_static_deinit = matches!(
                field_ty,
                ZigType::NamedStruct(n) if n == "Map" || n == "Set"
            ) || matches!(field_ty, ZigType::ArrayList(_))
                || matches!(field_ty, ZigType::BigInt);
            if needs_static_deinit {
                self.static_deinit_buffer.push_str(&format!(
                    "    {}.deinit(js_allocator.allocator());\n",
                    var_name
                ));
            }
        }

        // Static initialization blocks — collected into `static_init_buffer`.
        // When any class has static blocks, the module-level `init_js2rust()`
        // function is generated, which the orchestrator's root `init_js2rust()`
        // auto-discovers and calls. This ensures static blocks execute at runtime
        // (Zig's lazy analysis would otherwise skip unreferenced top-level `const`).
        for block in &class.static_blocks {
            if !block.stmts.is_empty() {
                // Temporarily swap output to static_init_buffer so statements go there
                let saved = std::mem::take(&mut self.output);
                self.output = std::mem::take(&mut self.static_init_buffer);
                for stmt in &block.stmts {
                    self.emit_stmt(stmt);
                }
                self.static_init_buffer = std::mem::take(&mut self.output);
                self.output = saved;
            }
        }
    }

    /// Emit a struct literal return: `return .{ .field0 = val0, .field1 = val1, ... };`
    /// Pairs are provided as `(field_name, value_string)` slices.
    fn emit_struct_literal_return(&mut self, pairs: &[(&str, String)]) {
        self.write_indent();
        self.write("return .{ ");
        for (i, (name, val)) in pairs.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            self.write(&format!(".{} = {}", name, val));
        }
        self.write(" };\n");
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

        // Return struct literal (from fields assigned in body — values are the local vars)
        let pairs: Vec<(&str, String)> = fields
            .iter()
            .map(|f| (f.name.as_str(), f.name.clone()))
            .collect();
        self.emit_struct_literal_return(&pairs);

        self.indent_pop();
        self.writeln("}");
    }

    pub(super) fn emit_default_init(&mut self, class_name: &str, fields: &[IrClassField]) {
        self.writeln(&format!("pub fn init() {} {{", class_name));
        self.indent_push();
        if fields.is_empty() {
            self.writeln("return .{};");
        } else {
            let pairs: Vec<(&str, String)> = fields
                .iter()
                .map(|f| {
                    (
                        f.name.as_str(),
                        match &f.default {
                            Some(expr) => self.expr_to_string(expr),
                            None => "0".to_string(),
                        },
                    )
                })
                .collect();
            self.emit_struct_literal_return(&pairs);
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
