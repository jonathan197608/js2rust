// zigir/emit/stmt/decl.rs
// Typedef, closure, variable, function, and class declaration emission.

use crate::types::ZigType;
use crate::zigir::emit::Emitter;
use crate::zigir::emit::helpers::{EmitterHelpers, format_param_with_rest, format_return_type};
use crate::zigir::types::{
    IrAssignTarget, IrBlock, IrClassDecl, IrClassField, IrClassMethod, IrClosureStruct, IrExpr,
    IrFnDecl, IrStmt, IrTypedef, IrVarDecl,
};
use std::collections::HashSet;

/// Pick a type-appropriate zero value for a pre-declared constructor `var`
/// when the field has no explicit `default` (R8-C7 + R8-E4/C6).
///
/// Zig requires every `var` to have an initialiser. We use the most natural
/// "empty" value per ZigType; complex/owning types (Map, Set, ArrayList,
/// BigInt, JsError, NamedStruct) fall back to `undefined` because they have
/// no const-evaluable zero state. JS semantics for an uninitialised field is
/// `undefined`, which lines up with `undefined` here for the dynamic types.
/// For the common scalar types we pick a real zero so the value is safe to
/// read even if the constructor never assigns the field.
fn field_zero_value(ty: &ZigType) -> &'static str {
    match ty {
        ZigType::I64 | ZigType::Anytype | ZigType::AnytypeReturn => "0",
        ZigType::F64 => "0.0",
        ZigType::Bool => "false",
        ZigType::Str => "\"\"",
        // ArrayList has a const-evaluable empty state.
        ZigType::ArrayList(_) => ".empty",
        // JsAny / JsSymbol / BigInt / JsError / NamedStruct / Struct / AsyncIo
        // — no const-zero; `undefined` is used intentionally as a bit pattern
        // that will be overwritten before any read (the constructor assigns
        // these fields before they're accessed). Using `undefined` here is
        // safe because these are owning types stored by reference/pointer;
        // the undefined value is never dereferenced or deinitialized.
        _ => "undefined",
    }
}

/// R8-E4/C6: Walk the constructor body and collect every identifier that is
/// the target of an `IrStmt::Assign` anywhere in it (recursing through
/// if/while/for/for-of/for-in/switch/try/block bodies AND destructuring
/// targets). Zig 0.16.0 ast-check rejects `var` declarations that are never
/// mutated with `"local variable is never mutated; consider using 'const'"`,
/// so the Emitter must use `const` for pre-declared ctor fields that the
/// constructor body never reassigns.
///
/// Destructure targets are included because `const { a, b } = obj` followed
/// by a use of `a` near the struct return still has the binding declared as
/// `const` by the Emitter's destructure path — but a *destructuring
/// assignment* (`{ a, b } = obj;` — no `const`/`var` keyword) lowers to an
/// `IrStmt::Assign { target: Destructure(...) }` that does mutate each
/// binding, so those bindings count as "assigned" too and should be
/// pre-declared `var` here to keep the rewrite legal at any scope.
fn collect_assigned_idents_in_block(block: &IrBlock) -> HashSet<String> {
    let mut out = HashSet::new();
    collect_assigned_idents(&block.stmts, &mut out);
    out
}

fn collect_assigned_idents(stmts: &[IrStmt], out: &mut HashSet<String>) {
    for stmt in stmts {
        match stmt {
            IrStmt::Assign { target, .. } => match target {
                IrAssignTarget::Ident(id) => {
                    out.insert(id.js_name.clone());
                }
                IrAssignTarget::Destructure(bindings) => {
                    for b in bindings {
                        out.insert(b.pattern.js_name.clone());
                    }
                }
                _ => {}
            },
            // C9: IrStmt::Expr can contain IrExpr::Update (field++) or
            // IrExpr::Assign (field += val) with Ident targets produced by
            // try_rewrite_this_field_assignment. Also covers BlockExpr
            // (BigInt postfix expansion).
            IrStmt::Expr(expr) => collect_assigned_idents_in_expr(expr, out),
            IrStmt::If { then, else_, .. } => {
                collect_assigned_idents(&then.stmts, out);
                if let Some(eb) = else_ {
                    collect_assigned_idents(&eb.stmts, out);
                }
            }
            IrStmt::While { body, .. } | IrStmt::DoWhile { body, .. } => {
                collect_assigned_idents(&body.stmts, out);
            }
            // C9: also scan init and update (e.g., for (this.x = 0; ...; this.x++) {})
            IrStmt::For {
                init, update, body, ..
            } => {
                if let Some(i) = init {
                    collect_assigned_idents(std::slice::from_ref(i.as_ref()), out);
                }
                if let Some(u) = update {
                    collect_assigned_idents(std::slice::from_ref(u.as_ref()), out);
                }
                collect_assigned_idents(&body.stmts, out);
            }
            IrStmt::ForIn { body, .. } | IrStmt::ForOf { body, .. } => {
                collect_assigned_idents(&body.stmts, out);
            }
            IrStmt::Switch { cases, .. } => {
                for case in cases {
                    collect_assigned_idents(&case.body, out);
                }
            }
            IrStmt::Try {
                try_block,
                catch_block,
                finally,
                ..
            } => {
                collect_assigned_idents(&try_block.stmts, out);
                collect_assigned_idents(&catch_block.stmts, out);
                if let Some(fb) = finally {
                    collect_assigned_idents(&fb.stmts, out);
                }
            }
            IrStmt::Block(b) => collect_assigned_idents(&b.stmts, out),
            _ => {}
        }
    }
}

/// C9: Scan an IrExpr for Ident-targeted mutations (Update / Assign).
/// Handles compound assignments (`field += val`) and increment/decrement
/// (`field++`) that are lowered as `IrStmt::Expr(IrExpr::Assign/Update{...})`
/// rather than `IrStmt::Assign{...}`. Also recurses into BlockExpr (BigInt
/// postfix expansion).
fn collect_assigned_idents_in_expr(expr: &IrExpr, out: &mut HashSet<String>) {
    match expr {
        IrExpr::Update { target, .. } | IrExpr::Assign { target, .. } => {
            if let IrAssignTarget::Ident(id) = target.as_ref() {
                out.insert(id.js_name.clone());
            }
        }
        IrExpr::BlockExpr { body, .. } => {
            collect_assigned_idents(body, out);
        }
        _ => {}
    }
}

/// R8-E5/C1: Check whether a class method body mutates `self` — i.e.,
/// contains any `this.field = ...` assignment (an assignment whose target
/// is an `IrAssignTarget::Member` on `IrExpr::This`).
///
/// If so, the method signature must use `self: *@This()` (mutable pointer)
/// because Zig rejects assignment to a by-value parameter. Non-mutating
/// methods keep the cheaper `self: @This()` (by-value) form.
///
/// In the IR, `this.x = v` inside an ExpressionStatement is lowered as
/// `IrStmt::Expr(IrExpr::Assign { target: Member { object: This, .. }, .. })`,
/// NOT as `IrStmt::Assign`. Similarly `++this.x` becomes
/// `IrStmt::Expr(IrExpr::Update { target: Member { object: This, .. }, .. })`
/// (non-BigInt) or `IrStmt::Expr(IrExpr::BlockExpr { body: [..., Expr(Assign { .. })], .. })`
/// (BigInt). All three forms must be detected.
fn method_mutates_self(body: &IrBlock) -> bool {
    /// Returns true if the assignment target is `this.field` (a Member on This).
    fn target_is_self_member(target: &IrAssignTarget) -> bool {
        matches!(
            target,
            IrAssignTarget::Member { object, .. }
                if matches!(object.as_ref(), IrExpr::This)
        )
    }

    /// Returns true if an expression mutates `self` — i.e., it is an
    /// assignment or update expression (possibly wrapped in a BlockExpr)
    /// whose target is `this.field`.
    fn expr_mutates_self(expr: &IrExpr) -> bool {
        match expr {
            IrExpr::Assign { target, .. } => target_is_self_member(target.as_ref()),
            IrExpr::Update { target, .. } => target_is_self_member(target.as_ref()),
            IrExpr::BlockExpr { body, result, .. } => {
                stmts_mutate_self(body) || expr_mutates_self(result.as_ref())
            }
            _ => false,
        }
    }

    fn stmts_mutate_self(stmts: &[IrStmt]) -> bool {
        stmts.iter().any(stmt_mutates_self)
    }

    fn stmt_mutates_self(stmt: &IrStmt) -> bool {
        match stmt {
            IrStmt::Assign { target, .. } => target_is_self_member(target),
            IrStmt::Expr(expr) => expr_mutates_self(expr),
            IrStmt::If { then, else_, .. } => {
                stmts_mutate_self(&then.stmts)
                    || else_
                        .as_ref()
                        .is_some_and(|eb| stmts_mutate_self(&eb.stmts))
            }
            IrStmt::While { body, .. } | IrStmt::DoWhile { body, .. } => {
                stmts_mutate_self(&body.stmts)
            }
            // C9: also check init and update for `this.field++` / `this.field += val`
            IrStmt::For {
                init, update, body, ..
            } => {
                init.as_ref().is_some_and(|i| stmt_mutates_self(i))
                    || update.as_ref().is_some_and(|u| stmt_mutates_self(u))
                    || stmts_mutate_self(&body.stmts)
            }
            IrStmt::ForIn { body, .. } | IrStmt::ForOf { body, .. } => {
                stmts_mutate_self(&body.stmts)
            }
            IrStmt::Switch { cases, .. } => cases.iter().any(|c| stmts_mutate_self(&c.body)),
            IrStmt::Try {
                try_block,
                catch_block,
                finally,
                ..
            } => {
                stmts_mutate_self(&try_block.stmts)
                    || stmts_mutate_self(&catch_block.stmts)
                    || finally
                        .as_ref()
                        .is_some_and(|fb| stmts_mutate_self(&fb.stmts))
            }
            IrStmt::Block(b) => stmts_mutate_self(&b.stmts),
            _ => false,
        }
    }

    stmts_mutate_self(&body.stmts)
}

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
        // AnytypeReturn and Struct (anonymous object) both need @TypeOf
        // because `.{ .field = T, ... }` is a struct literal, not a valid type.
        if matches!(return_type, ZigType::AnytypeReturn | ZigType::Struct(_)) {
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
            if let Some(label) = &self.inside_try_block {
                self.write(&format!(
                    ") catch |err| break :{} @as(anyerror!void, err)",
                    label
                ));
            } else if self.in_function {
                // NOTE: `return error.JsThrow` requires the enclosing function
                // to return an error union (`!T`). The lowerer only generates
                // JSON.parse inside functions that return error unions, so this
                // is safe in practice. If a future change allows JSON.parse in
                // non-error-union functions, this path would produce a Zig
                // compile error and would need revisiting.
                self.write(") catch return error.JsThrow");
            } else {
                self.write(") catch @panic(\"JSON.parse failed\")");
            }
        } else if let Some(init) = &vd.init {
            // Has initializer
            // Special case: __arguments variable — emit as const slice with explicit type
            // annotation so that even empty arrays (&.{} ) are typed as []const JsAny (slice,
            // which supports indexing) rather than *const [0]JsAny (zero-length array, which
            // does not support indexing).
            if vd.name.zig_name == "__arguments" {
                self.write(&format!("{} {}: []const JsAny = ", kw, vd.name.zig_name));
                if let crate::zigir::types::IrExpr::ArrayLiteral(arr) = init {
                    self.write("&[_]JsAny{ ");
                    for (i, elem) in arr.elements.iter().enumerate() {
                        if i > 0 {
                            self.write(", ");
                        }
                        self.emit_expr(elem);
                    }
                    self.write(" }");
                } else {
                    self.emit_expr(init);
                }
            } else {
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
            }
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
                if name != "Map" && name != "Set" && name != "Date"
                    && name != "JsBigInt" && name != "JsRegExp" && name != "RegExp"
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
        self.in_function = true;
        for param in &fd.params {
            if param.is_unused {
                self.writeln(&format!("_ = _{};", param.name.zig_name));
            }
        }

        // Function body
        self.emit_block_stmts_unlabeled(&fd.body);
        self.in_function = false;
        self.indent_pop();

        self.writeln("}");
        // NOTE: No trailing blank line.
        // Inter-declaration spacing is handled at the module level if needed.
    }

    pub(crate) fn emit_class_decl(&mut self, class: &IrClassDecl) {
        let class_name = &class.name.zig_name;

        // Register class in needs_deinit set (redundant with pre-scan, but safe).
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

        // deinit() method — generated when any field needs deinit (Map, Set,
        // ArrayList, BigInt, or a NamedStruct class that itself needs deinit).
        // Uses the propagated class_needs_deinit set from the pre-scan step.
        if self.class_needs_deinit.contains(class_name.as_str()) {
            self.writeln("");
            self.writeln("pub fn deinit(self: *@This(), alloc: std.mem.Allocator) void {");
            self.indent_push();
            for field in &class.fields {
                let needs_field_deinit = matches!(
                    &field.zig_type,
                    ZigType::NamedStruct(n)
                        if n == "Map" || n == "Set"
                            || self.class_needs_deinit.contains(n.as_str())
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

            // Register static fields needing deinit in deinit_js2rust():
            // Map, Set, ArrayList, BigInt, or NamedStruct class that needs deinit.
            let needs_static_deinit = matches!(
                field_ty,
                ZigType::NamedStruct(n)
                    if n == "Map" || n == "Set"
                        || self.class_needs_deinit.contains(n.as_str())
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

        // R8-C7 + R8-E4/C6: Pre-declare each class field at the top of the
        // constructor body, so the Lowerer's rewritten `field = value` Assigns
        // have a target to write to (no matter how deeply nested).
        //
        // The initialiser value comes from the field's `default` (its class
        // body `x = ...`) when present, otherwise a type-appropriate zero.
        // This means a constructor that *never* touches a field still yields
        // the field's declared default instead of an undefined slot — which is
        // the R8-E4/C6 fix.
        //
        // Whether each pre-declaration is `var` or `const` depends on whether
        // the constructor body actually reassigns the field: Zig 0.16.0
        // ast-check rejects a `var` that is never mutated, so a field that
        // only carries its default through to the struct return must be
        // declared `const`. We compute the set of assigned-identifier names
        // up front (recursing into every nested container statement) and
        // route each field accordingly.
        let assigned = collect_assigned_idents_in_block(&ctor.body);
        for f in fields {
            let default_str = match &f.default {
                Some(expr) => self.expr_to_string(expr),
                None => field_zero_value(&f.zig_type).to_string(),
            };
            let kw = if assigned.contains(&f.name) {
                "var"
            } else {
                "const"
            };
            self.writeln(&format!(
                "{} {}: {} = {};",
                kw,
                f.name,
                f.zig_type.to_zig_type(),
                default_str
            ));
        }

        // Constructor body — rewritten assignments target the vars above.
        self.emit_block_stmts_unlabeled(&ctor.body);

        // R8-C2: Skip the appended `return .{...}` if the body's last
        // top-level statement is already a `Return`. Otherwise Zig would
        // reject the appended return as unreachable code.
        //
        // Limitation: this only detects an explicit top-level trailing return.
        // A return nested inside an if/switch branch still lets the appended
        // return stand; that path will hit the appended struct return with
        // whatever the branch left in the field vars, which matches JS
        // "early return from ctor returns the partial instance".
        let body_ends_in_return = ctor
            .body
            .stmts
            .last()
            .is_some_and(|last| matches!(last, IrStmt::Return { .. }));

        if !body_ends_in_return {
            // Return struct literal (from fields assigned in body — values are the local vars)
            let pairs: Vec<(&str, String)> = fields
                .iter()
                .map(|f| (f.name.as_str(), f.name.clone()))
                .collect();
            self.emit_struct_literal_return(&pairs);
        }

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
                            None => field_zero_value(&f.zig_type).to_string(),
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
        // R8-E5/C1: Methods that mutate `self` (assign to `self.field`) need
        // `self: *@This()` so Zig allows the assignment. Non-mutating methods
        // keep `self: @This()` (by-value) which is cheaper and works on both
        // const and var instances.
        let mut sig = if method.is_static {
            format!("pub fn {}(", method.name)
        } else if method_mutates_self(&method.body) {
            format!("pub fn {}(self: *@This()", method.name)
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
