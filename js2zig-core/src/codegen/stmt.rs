// native_proto/codegen/stmt.rs
// Statement-level code generation: toplevel, var_decl, fn, if, while, for, switch.

use super::Codegen;
use crate::native_proto::builtins;
use crate::native_proto::{ExportedFunction, ZigType};
use oxc_ast::ast::*;
use oxc_span::GetSpan;
use std::collections::HashSet;

// ── Variable declarations ────────────────────────────

/// Check if an identifier name is referenced in a list of statements.
/// Uses a simplified AST walk — covers common patterns in catch bodies
/// (console.log(e), return e.message, etc.).
fn stmt_list_references_name(stmts: &[Statement], name: &str) -> bool {
    stmts.iter().any(|s| stmt_references_name(s, name))
}

fn stmt_references_name(stmt: &Statement, name: &str) -> bool {
    match stmt {
        Statement::ExpressionStatement(es) => expr_references_name(&es.expression, name),
        Statement::ReturnStatement(rs) => rs
            .argument
            .as_ref()
            .is_some_and(|a| expr_references_name(a, name)),
        Statement::VariableDeclaration(vd) => vd.declarations.iter().any(|d| {
            d.init
                .as_ref()
                .is_some_and(|init| expr_references_name(init, name))
        }),
        Statement::BlockStatement(bs) => stmt_list_references_name(&bs.body, name),
        Statement::ThrowStatement(ts) => expr_references_name(&ts.argument, name),
        // IfStatement with else branch can have catch param refs
        Statement::IfStatement(ifs) => {
            stmt_references_name(&ifs.consequent, name)
                || ifs
                    .alternate
                    .as_ref()
                    .is_some_and(|a| stmt_references_name(a, name))
        }
        _ => false,
    }
}

fn expr_references_name(expr: &Expression, name: &str) -> bool {
    match expr {
        Expression::Identifier(id) => id.name.as_str() == name,
        Expression::BinaryExpression(be) => {
            expr_references_name(&be.left, name) || expr_references_name(&be.right, name)
        }
        Expression::CallExpression(ce) => {
            // Callee is Expression, so check it directly
            expr_references_name(&ce.callee, name)
        }
        Expression::StaticMemberExpression(sme) => expr_references_name(&sme.object, name),
        _ => false,
    }
}

impl Codegen {
    /// Emit a variable declaration. Toplevel: only `const` allowed.
    /// Inside functions: `var` with type inference + undefined init.
    pub(crate) fn emit_var_decl(&mut self, vd: &VariableDeclaration) {
        for decl in &vd.declarations {
            if let Some(name) = crate::native_proto::infer::binding_name(&decl.id) {
                let zig_name = self.zig_safe_name(name);

                // Use Zig 'const' when the variable is never mutated (regardless of JS const/var/let).
                // Only use Zig 'var' when the variable is actually reassigned.
                let fn_prefix = self.current_fn.as_deref().unwrap_or("__toplevel__");
                let is_const = !self
                    .type_info
                    .mutated_vars
                    .contains(&format!("{}::{}", fn_prefix, name));

                // Skip unused toplevel constants to avoid Zig unused errors.
                // NOTE: only toplevel — local variables may have side-effectful initializers.
                let has_type_annotation = self
                    .jsdoc_data
                    .as_ref()
                    .is_some_and(|d| d.type_annotations.contains_key(name));
                if self.indent == 0
                    && is_const
                    && !self.type_info.used_names.contains(name)
                    && !has_type_annotation
                {
                    continue;
                }
                // Rule: toplevel var/let → error. Only allow const.
                if self.indent == 0 && !is_const {
                    self.write_indent();
                    self.write(&format!(
                        "// error: toplevel only allows 'const', not '{}'",
                        zig_name
                    ));
                    self.writeln("");
                    continue;
                }

                match &decl.init {
                    Some(Expression::ArrowFunctionExpression(arrow)) => {
                        // Generate arrow function or closure
                        let fn_name = self.emit_arrow_function(arrow);
                        // Check if this is a closure (struct name in closure_vars)
                        if self.closure_vars.contains_key(&fn_name) {
                            // Closure: generate instantiation code
                            // Clone the captured vars to avoid borrow conflict
                            let captured = self.closure_vars.get(&fn_name).cloned();
                            if let Some(captured) = captured {
                                self.write_indent();
                                self.write(&format!("const {} = {} {{ ", zig_name, fn_name));
                                // Generate field initializers
                                for (i, (cap_name, _, is_mut)) in captured.iter().enumerate() {
                                    if i > 0 {
                                        self.write(", ");
                                    }
                                    let safe_cap = self.zig_safe_name(cap_name);
                                    if *is_mut {
                                        self.write(&format!(".{} = &{}", safe_cap, safe_cap));
                                    } else {
                                        self.write(&format!(".{} = {}", safe_cap, safe_cap));
                                    }
                                }
                                self.write(" };\n");
                            }
                            // Mark this variable as a closure instance
                            self.closure_instances.insert(name.to_string());
                        } else {
                            // Plain arrow function: assign function to variable
                            self.write_indent();
                            self.write(&format!(
                                "const {} = {};
",
                                zig_name, fn_name
                            ));
                        }
                    }
                    Some(init) => {
                        self.write_indent();
                        // Force 'var' for Map/Set/ArrayList types (mutated via methods).
                        let is_const = if let Some(inferred_ty) = self.type_info.var_types.get(name)
                        {
                            match inferred_ty {
                                ZigType::ArrayList(_) => false,
                                ZigType::NamedStruct(n) if n == "Map" || n == "Set" => false,
                                _ => is_const,
                            }
                        } else {
                            is_const
                        };
                        let kw = if is_const { "const" } else { "var" };

                        let is_json_parse = self.type_info.has_json_parse_types.contains(name);

                        if is_json_parse {
                            // JSDoc-annotated JSON.parse: type is NamedStruct
                            let type_name = self
                                .type_info
                                .var_types
                                .get(name)
                                .and_then(|t| match t {
                                    ZigType::NamedStruct(n) => Some(n.as_str()),
                                    _ => None,
                                })
                                .unwrap_or("i64");
                            self.write(&format!(
                                "{} {}: {} = std.json.parse({}, ",
                                kw, zig_name, type_name, type_name
                            ));
                            if let Expression::CallExpression(ce) = init
                                && let Some(first_arg) = ce.arguments.first()
                            {
                                self.emit_expr_arg(first_arg);
                            }
                            self.write(") catch @panic(\"OOM: JSON.parse alloc\");\n");
                        } else if let Some(inferred_ty) = self.type_info.var_types.get(name) {
                            let inferred_ty = inferred_ty.clone();
                            // Definite type from pre-computed type info.
                            // Skip type annotation for NamedStruct and ArrayList — Zig infers from init.
                            let skip_type_annotation = matches!(
                                inferred_ty,
                                ZigType::NamedStruct(_) | ZigType::ArrayList(_)
                            );
                            if is_const || skip_type_annotation {
                                self.write(&format!("{} {} = ", kw, zig_name));
                            } else {
                                self.write(&format!(
                                    "{} {}: {} = ",
                                    kw,
                                    zig_name,
                                    inferred_ty.to_zig_type()
                                ));
                            }
                            self.emit_expr(init);
                            self.write(";\n");
                            if let Some(ta_type) = typedarray_init_type(init) {
                                self.typedarray_vars
                                    .insert(name.to_string(), ta_type.to_string());
                            }
                            if is_regexp_new(init) {
                                self.regexp_vars.insert(name.to_string());
                            }
                            // Zig 0.16.0: 'var' for ArrayList/Map/Set needs &var suppression
                            // (method calls like reverse/sort go through .items, not &arr)
                            if !is_const {
                                match inferred_ty {
                                    ZigType::ArrayList(_) => {
                                        self.write_indent();
                                        self.write(&format!("_ = &{}; // var usage\n", zig_name));
                                    }
                                    ZigType::NamedStruct(n) if n == "Map" || n == "Set" => {
                                        self.write_indent();
                                        self.write(&format!("_ = &{}; // var usage\n", zig_name));
                                    }
                                    _ => {}
                                }
                            }
                        } else {
                            // Indeterminate type (Rule 8 error already in type_info.errors)
                            self.write(&format!("{} {} = ", kw, zig_name));
                            self.emit_expr(init);
                            self.write(";\n");
                            // HACK 1: track JsMap.init() return type as NamedStruct("Map")
                            if let Expression::CallExpression(ce) = init
                                && let Expression::StaticMemberExpression(sme) = &ce.callee
                                && sme.property.name.as_str() == "init"
                            {
                                // obj.init(...) — check if obj type is JsMap/JsSet
                                if let Expression::Identifier(obj_id) = &sme.object {
                                    let obj_name = obj_id.name.as_str();
                                    if obj_name == "js_collections" {
                                        // js_collections.JsMap.init() → Map
                                        // Heuristic: check property chain
                                        // sme.object is "js_collections", sme.property is "JsMap"
                                        // This is a hack — JsMap → Map
                                        let prop = sme.property.name.as_str();
                                        if prop == "JsMap" {
                                            self.type_info.var_types.insert(
                                                name.to_string(),
                                                ZigType::NamedStruct("Map".to_string()),
                                            );
                                        } else if prop == "JsSet" {
                                            self.type_info.var_types.insert(
                                                name.to_string(),
                                                ZigType::NamedStruct("Set".to_string()),
                                            );
                                        }
                                    }
                                }
                            }
                            // HACK 2: directly check if init is Map.get() and track v as JsAny.
                            // This avoids relying on builtins module (which grep can't find
                            // but Rust compilation confirms it exists).
                            // Handle both ComputedMemberExpression (obj[key]()) and
                            // StaticMemberExpression (obj.method(...)) callees.
                            if let Expression::CallExpression(ce) = init {
                                // Helper: check if callee is obj.get/set/has and obj is a Map/Set
                                let mut check_callee = |mem_obj: &Expression| {
                                    if let Expression::Identifier(obj_id) = mem_obj
                                        && let Some(ZigType::NamedStruct(n)) =
                                            self.type_info.var_types.get(obj_id.name.as_str())
                                        && n == "Map"
                                    {
                                        self.type_info
                                            .var_types
                                            .insert(name.to_string(), ZigType::JsAny);
                                    }
                                };
                                // ComputedMemberExpression: obj["get"](key) — rare, kept for completeness
                                if let Expression::ComputedMemberExpression(mem) = &ce.callee {
                                    check_callee(&mem.object);
                                }
                                // StaticMemberExpression: obj.get(key) — the common case
                                if let Expression::StaticMemberExpression(mem) = &ce.callee {
                                    // Only track as JsAny if method is "get" (returns JsAny)
                                    if mem.property.name.as_str() == "get" {
                                        check_callee(&mem.object);
                                    }
                                }
                            }
                            if let Some(ta_type) = typedarray_init_type(init) {
                                self.typedarray_vars
                                    .insert(name.to_string(), ta_type.to_string());
                            }
                            if is_regexp_new(init) {
                                self.regexp_vars.insert(name.to_string());
                            }
                        }
                    }
                    None => {
                        // No initializer → error.
                        self.write_indent();
                        self.write(&format!(
                            "// error: variable '{}' must be initialized",
                            name
                        ));
                        self.writeln("");
                    }
                }
            } else {
                // ── Destructuring patterns ────────────────────────────
                match &decl.id {
                    BindingPattern::ObjectPattern(op) => {
                        self.emit_object_destructure(op, &decl.init);
                    }
                    BindingPattern::ArrayPattern(ap) => {
                        self.emit_array_destructure(ap, &decl.init);
                    }
                    _ => {
                        // Unknown pattern — skip silently
                    }
                }
            }
        }
    }
}

/// Check if init expression should be emitted to a temp variable (has side effects)
/// to avoid evaluating it multiple times in destructuring.
fn init_may_have_side_effects(init: &Expression) -> bool {
    matches!(
        init,
        Expression::CallExpression(_)
            | Expression::NewExpression(_)
            | Expression::AssignmentExpression(_)
            | Expression::UpdateExpression(_)
    )
}

// ── Destructuring codegen ──────────────────────────────

impl Codegen {
    /// Get the ZigType of an expression if it's a simple identifier reference.
    fn expr_var_type(&self, expr: &Expression) -> Option<&crate::native_proto::ZigType> {
        if let Expression::Identifier(id) = expr {
            self.type_info.var_types.get(id.name.as_str())
        } else {
            None
        }
    }

    /// Generate code for object destructuring:
    /// `const {a, b: c, d = default} = expr`
    /// Handles struct (field access), HashMap (.get("key")), and fallback (.field orelse).
    fn emit_object_destructure(&mut self, op: &ObjectPattern, init: &Option<Expression>) {
        let Some(init_expr) = init else {
            self.writeln("// error: destructuring requires an initializer");
            return;
        };

        // Generate init expression as a temp variable (to avoid evaluating twice)
        let init_str = self.emit_expr_to_string(init_expr);
        let needs_temp = init_may_have_side_effects(init_expr);
        let temp_name = if needs_temp || op.properties.len() > 1 {
            let n = self.destructure_counter;
            self.destructure_counter += 1;
            let name = format!("_js_dest_{}", n);
            self.write_indent();
            self.write(&format!("const {} = {};\n", name, init_str));
            name
        } else {
            // Single property with pure init — inline (clone to keep init_str alive)
            init_str.clone()
        };
        let is_temp = needs_temp || op.properties.len() > 1;

        // Determine the type of the source object for correct field access.
        // Clone field names to release the immutable borrow on self before the loop.
        let init_type = self.expr_var_type(init_expr);
        let struct_field_names: Option<std::collections::HashSet<String>> = match init_type {
            Some(crate::native_proto::ZigType::Struct(fields)) => {
                if fields.is_empty() {
                    None // empty struct → treat as HashMap
                } else {
                    Some(fields.iter().map(|(name, _)| name.clone()).collect())
                }
            }
            _ => None,
        };
        let is_hashmap = struct_field_names.is_none(); // non-struct or empty struct → HashMap access
        let _ = init_type;

        let source = if is_temp { &temp_name } else { &init_str };

        for prop in &op.properties {
            let key_name = match self.property_key_name(&prop.key) {
                Some(k) => k,
                None => {
                    self.writeln(
                        "// error: computed property key in destructuring not yet supported",
                    );
                    continue;
                }
            };

            let Some((bind_name, default_expr)) = self.destructure_binding(&prop.value) else {
                self.writeln(&format!(
                    "// error: unsupported destructure binding for '{}'",
                    key_name
                ));
                continue;
            };
            let zig_bind_name = self.zig_safe_name(bind_name);

            // Check is_const for individual binding
            let fn_prefix = self.current_fn.as_deref().unwrap_or("__toplevel__");
            let is_const = !self
                .type_info
                .mutated_vars
                .contains(&format!("{}::{}", fn_prefix, bind_name));
            // Skip unused toplevel constants
            if self.indent == 0 && is_const && !self.type_info.used_names.contains(bind_name) {
                continue;
            }
            // Toplevel only allows const
            if self.indent == 0 && !is_const {
                self.write_indent();
                self.write(&format!(
                    "// error: toplevel only allows 'const', not '{}'",
                    zig_bind_name
                ));
                self.writeln("");
                continue;
            }

            let kw = if is_const { "const" } else { "var" };

            self.write_indent();
            self.write(&format!("{} {} = ", kw, zig_bind_name));

            if let Some(ref field_names) = struct_field_names {
                // ── Struct: use direct field access ──
                let field_exists = field_names.contains(&key_name);
                if field_exists {
                    // Field exists: no orelse needed (struct fields always present)
                    self.write(&format!("{}.{}", source, key_name));
                } else if let Some(default) = &default_expr {
                    // Field doesn't exist in struct → use default directly
                    self.write(default);
                } else {
                    // Field missing and no default → compile error
                    self.writeln(&format!(
                        "// error: destructure key '{}' not found in struct and no default value",
                        key_name
                    ));
                    continue;
                }
            } else if is_hashmap {
                // ── HashMap / unknown type: use .get("key") with type-aware conversion ──
                if let Some(default) = &default_expr {
                    // Map ?JsAny → concrete type via if/else (orelse doesn't work: JsAny ≠ i64).
                    let conv = if default == "true" || default == "false" {
                        ".asBool()"
                    } else if default.starts_with('"') {
                        ".value.string"
                    } else {
                        ".asI64()"
                    };
                    self.write(&format!(
                        "if ({source}.get(\"{key}\")) |v| v{conv} else {default}",
                        source = source,
                        key = key_name,
                        conv = conv,
                        default = default
                    ));
                } else {
                    // No default: emit raw .get() (returns ?JsAny, caller must handle)
                    self.write(&format!("{}.get(\"{}\")", source, key_name));
                }
            } else {
                // Should not reach here (is_hashmap is true for non-struct)
                self.write(&format!("{}.{}", source, key_name));
                if let Some(default) = &default_expr {
                    self.write(&format!(" orelse {}", default));
                }
            }

            self.write(";\n");
        }
    }

    /// Generate code for array destructuring:
    /// `const [a, , b = default] = expr`
    /// For ArrayList: `const a = if (arr.items.len > 0) arr.items[0] else default;`
    /// For slices/arrays: `const a = arr[0];` (no orelse — not optional)
    fn emit_array_destructure(&mut self, ap: &ArrayPattern, init: &Option<Expression>) {
        let Some(init_expr) = init else {
            self.writeln("// error: destructuring requires an initializer");
            return;
        };

        let init_str = self.emit_expr_to_string(init_expr);
        // Determine the type for correct indexing
        let init_type = self.expr_var_type(init_expr);
        let is_arraylist = matches!(init_type, Some(crate::native_proto::ZigType::ArrayList(_)));
        // Count non-None elements to decide if we need a temp
        let element_count = ap.elements.iter().filter(|e| e.is_some()).count();
        let needs_temp = init_may_have_side_effects(init_expr) || element_count > 1;
        let temp_name = if needs_temp {
            let n = self.destructure_counter;
            self.destructure_counter += 1;
            let name = format!("_js_dest_{}", n);
            self.write_indent();
            self.write(&format!("const {} = {};\n", name, init_str));
            name
        } else {
            init_str.clone()
        };
        let is_temp = needs_temp;

        let source = if is_temp { &temp_name } else { &init_str };

        for (i, elem) in ap.elements.iter().enumerate() {
            let Some(pattern) = elem else {
                continue; // skip holes like `const [a, , b] = arr`
            };

            let Some((bind_name, default_expr)) = self.destructure_binding(pattern) else {
                self.writeln(&format!(
                    "// error: unsupported destructure binding for element {}",
                    i
                ));
                continue;
            };
            let zig_bind_name = self.zig_safe_name(bind_name);

            // Check is_const for individual binding
            let fn_prefix = self.current_fn.as_deref().unwrap_or("__toplevel__");
            let is_const = !self
                .type_info
                .mutated_vars
                .contains(&format!("{}::{}", fn_prefix, bind_name));
            if self.indent == 0 && is_const && !self.type_info.used_names.contains(bind_name) {
                continue;
            }
            if self.indent == 0 && !is_const {
                self.write_indent();
                self.write(&format!(
                    "// error: toplevel only allows 'const', not '{}'",
                    zig_bind_name
                ));
                self.writeln("");
                continue;
            }

            let kw = if is_const { "const" } else { "var" };

            self.write_indent();
            self.write(&format!("{} {} = ", kw, zig_bind_name));

            if is_arraylist {
                // ── ArrayList: bounds-safe .items[i] access ──
                if let Some(default) = &default_expr {
                    self.write(&format!(
                        "if ({}.items.len > {}) {}.items[{}] else {}",
                        source, i, source, i, default
                    ));
                } else {
                    self.write(&format!("{}.items[{}]", source, i));
                }
            } else {
                // ── Slice/array or unknown: direct [i] ──
                self.write(&format!("{}[{}]", source, i));
                if let Some(default) = &default_expr {
                    self.write(&format!(" orelse {}", default));
                }
            }

            self.write(";\n");
        }
    }
}

// ── Function declarations ──────────────────────────────

impl Codegen {
    pub(crate) fn has_throw_in_body(body: &FunctionBody) -> bool {
        fn stmt_has_throw(stmt: &Statement) -> bool {
            match stmt {
                Statement::ThrowStatement(_) | Statement::TryStatement(_) => true,
                Statement::LabeledStatement(s) => stmt_or_expr_has_throw(&s.body),
                Statement::IfStatement(s) => {
                    stmt_or_expr_has_throw(&s.consequent)
                        || s.alternate
                            .as_ref()
                            .is_some_and(|a| stmt_or_expr_has_throw(a))
                }
                Statement::WhileStatement(s) => stmt_or_expr_has_throw(&s.body),
                Statement::DoWhileStatement(s) => stmt_or_expr_has_throw(&s.body),
                Statement::ForStatement(s) => stmt_or_expr_has_throw(&s.body),
                Statement::ForOfStatement(s) => stmt_or_expr_has_throw(&s.body),
                Statement::ForInStatement(s) => stmt_or_expr_has_throw(&s.body),
                Statement::BlockStatement(s) => s.body.iter().any(stmt_has_throw),
                Statement::SwitchStatement(s) => s
                    .cases
                    .iter()
                    .any(|c| c.consequent.iter().any(stmt_has_throw)),
                _ => false,
            }
        }

        fn stmt_or_expr_has_throw(stmt: &Statement) -> bool {
            match stmt {
                Statement::BlockStatement(s) => s.body.iter().any(stmt_has_throw),
                other => stmt_has_throw(other),
            }
        }

        body.statements.iter().any(stmt_has_throw)
    }

    /// Check if a single statement contains a throw (does NOT count try-catch blocks).
    /// Used for pre-scanning try blocks to decide whether to generate catch handler.
    pub(crate) fn stmt_has_throw_any(stmt: &Statement) -> bool {
        match stmt {
            Statement::ThrowStatement(_) => true,
            Statement::LabeledStatement(s) => {
                crate::native_proto::codegen::stmt::Codegen::stmt_has_throw_any_alt(&s.body)
            }
            Statement::IfStatement(s) => {
                crate::native_proto::codegen::stmt::Codegen::stmt_has_throw_any_alt(&s.consequent)
                    || s.alternate.as_ref().is_some_and(|a| {
                        crate::native_proto::codegen::stmt::Codegen::stmt_has_throw_any_alt(a)
                    })
            }
            Statement::WhileStatement(s) => {
                crate::native_proto::codegen::stmt::Codegen::stmt_has_throw_any_alt(&s.body)
            }
            Statement::DoWhileStatement(s) => {
                crate::native_proto::codegen::stmt::Codegen::stmt_has_throw_any_alt(&s.body)
            }
            Statement::ForStatement(s) => {
                crate::native_proto::codegen::stmt::Codegen::stmt_has_throw_any_alt(&s.body)
            }
            Statement::ForOfStatement(s) => {
                crate::native_proto::codegen::stmt::Codegen::stmt_has_throw_any_alt(&s.body)
            }
            Statement::ForInStatement(s) => {
                crate::native_proto::codegen::stmt::Codegen::stmt_has_throw_any_alt(&s.body)
            }
            Statement::BlockStatement(s) => s.body.iter().any(|s1| Self::stmt_has_throw_any(s1)),
            Statement::SwitchStatement(s) => s
                .cases
                .iter()
                .any(|c| c.consequent.iter().any(|s1| Self::stmt_has_throw_any(s1))),
            _ => false,
        }
    }

    fn stmt_has_throw_any_alt(stmt: &Statement) -> bool {
        match stmt {
            Statement::BlockStatement(s) => s.body.iter().any(|s1| Self::stmt_has_throw_any(s1)),
            other => Self::stmt_has_throw_any(other),
        }
    }

    /// Enter a try block. Sets `inside_try_block` so that `throw` statements
    /// inside the block generate `break :label error.JsThrow` instead of
    /// `return error.JsThrow`.
    pub(crate) fn start_try_block(&mut self, label: &str) {
        self.inside_try_block = Some(label.to_string());
    }

    /// Exit a try block. Clears `inside_try_block`.
    pub(crate) fn end_try_block(&mut self) {
        self.inside_try_block = None;
    }

    pub(crate) fn emit_fn(&mut self, fd: &Function) {
        let name = fd
            .id
            .as_ref()
            .map(|id| id.name.as_str())
            .unwrap_or("anonymous");
        // For nested function declarations, use "call" as the generated
        // function name (the inline struct pattern requires .call() method).
        let emit_name = if self.current_nested_fn_name.is_some() {
            "call"
        } else {
            name
        };
        let saved_current_fn = std::mem::take(&mut self.current_fn);
        self.current_fn = Some(emit_name.to_string());

        // Check if function contains await (from pre-computed type_info)
        let is_async = self.type_info.is_async.get(name).copied().unwrap_or(false);

        // Pre-scan: check if function contains throw or try-catch.
        // This must happen BEFORE generating the return signature (need !T for throw).
        let has_throw = fd
            .body
            .as_ref()
            .is_some_and(|b| Codegen::has_throw_in_body(b));
        self.fn_has_throw = has_throw;

        // Read pre-computed return type from TypeInferResult.
        let ret_ty = self.type_info.fn_return_types.get(name).cloned();
        self.current_fn_return_type = ret_ty.clone();

        // Build set of identifiers used in THIS function body (per-function).
        let mut fn_used_names = HashSet::new();
        if let Some(body) = &fd.body {
            for s in &body.statements {
                Self::collect_stmt_idents(s, &mut fn_used_names);
            }
        }

        // Generate function signature.
        let has_captures = !self.current_captured.is_empty();
        let safe_emit_name = self.zig_safe_name(emit_name);
        if is_async {
            if has_captures {
                self.write(&format!(
                    "pub fn {}(self: @This(), io: anytype",
                    safe_emit_name
                ));
            } else {
                self.write(&format!("pub fn {}(io: anytype", safe_emit_name));
            }
        } else {
            if has_captures {
                self.write(&format!("pub fn {}(self: @This(), ", safe_emit_name));
            } else {
                self.write(&format!("pub fn {}(", safe_emit_name));
            }
        }

        // Generate parameter list (read param types from type_info).
        let param_list = self.type_info.fn_param_types.get(name).cloned();
        if let Some(params) = param_list {
            let mut param_idx = 0;
            for (pname, ptype) in &params {
                if is_async && pname == "io" {
                    continue;
                }
                if param_idx > 0 || is_async {
                    self.write(", ");
                }
                // Zig 0.16.0: unused params are compile errors. Prefix with _ if unused
                // in THIS function body (per-function tracking).
                let safe_pname = self.zig_safe_name(pname);
                let zig_pname = if fn_used_names.contains(pname) {
                    safe_pname.as_str()
                } else {
                    self.write("_");
                    safe_pname.as_str()
                };
                // Export function params: always typed; non-export: use anytype
                if self.current_fn_is_export {
                    self.write(&format!("{}: {}", zig_pname, ptype.to_zig_type()));
                } else {
                    // Non-export params are inferred as anytype in the type info
                    self.write(&format!("{}: {}", zig_pname, ptype.to_zig_type()));
                }
                param_idx += 1;
            }
            // Handle rest parameter (...args) from type_info or AST
            if let Some(rest_name) = fd
                .params
                .rest
                .as_ref()
                .map(|r| crate::native_proto::infer::binding_name(&r.rest.argument))
                && let Some(rname) = rest_name
            {
                if param_idx > 0 || is_async {
                    self.write(", ");
                }
                let safe_rname = self.zig_safe_name(rname);
                let zig_pname = if fn_used_names.contains(rname) {
                    safe_rname.as_str()
                } else {
                    self.write("_");
                    safe_rname.as_str()
                };
                // Rest parameter: accepts []const JsAny
                self.write(&format!("{}: []const JsAny", zig_pname));
            }
        } else {
            // Fallback: generate params from AST with anytype
            let mut param_idx = 0;
            for param in &fd.params.items {
                if let Some(pname) = crate::native_proto::infer::binding_name(&param.pattern) {
                    if is_async && pname == "io" {
                        continue;
                    }
                    if param_idx > 0 || is_async {
                        self.write(", ");
                    }
                    // Zig 0.16.0: unused params are compile errors.
                    let safe_pname = self.zig_safe_name(pname);
                    let zig_pname = if fn_used_names.contains(pname) {
                        safe_pname.as_str()
                    } else {
                        self.write("_");
                        safe_pname.as_str()
                    };
                    self.write(&format!("{}: anytype", zig_pname));
                    param_idx += 1;
                }
            }
            // Handle rest parameter (...args) in fallback mode
            if let Some(rest_name) = fd
                .params
                .rest
                .as_ref()
                .map(|r| crate::native_proto::infer::binding_name(&r.rest.argument))
                && let Some(rname) = rest_name
            {
                if param_idx > 0 || is_async {
                    self.write(", ");
                }
                let safe_rname = self.zig_safe_name(rname);
                let zig_pname = if fn_used_names.contains(rname) {
                    safe_rname.as_str()
                } else {
                    self.write("_");
                    safe_rname.as_str()
                };
                self.write(&format!("{}: []const JsAny", zig_pname));
            }
        }

        // Return type — async + throw functions return error unions
        let ret_zig_type = match &self.current_fn_return_type {
            Some(ZigType::I64) => "i64".to_string(),
            Some(ZigType::F64) => "f64".to_string(),
            Some(ZigType::Bool) => "bool".to_string(),
            Some(ZigType::Str) => "[]const u8".to_string(),
            Some(ZigType::Void) => "void".to_string(),
            Some(ZigType::AnytypeReturn) => {
                if let Some(first_ret) =
                    crate::native_proto::infer::helpers::find_first_return_expr(fd)
                {
                    let captured = self.capture_expr(first_ret);
                    format!("@TypeOf({})", captured)
                } else {
                    "void".to_string()
                }
            }
            None => "void".to_string(),
            Some(other) => other.to_zig_type(),
        };
        let ret_zig_type = if (is_async || self.fn_has_throw) && ret_zig_type != "void" {
            format!("!{}", ret_zig_type)
        } else if self.fn_has_throw && ret_zig_type == "void" {
            "!void".to_string()
        } else {
            ret_zig_type
        };
        self.writeln(&format!(") {} {{", ret_zig_type));

        self.indent += 1;
        self.seen_return = false;

        // Suppress "unused parameter" errors for params not used in body
        // (fetch params again to determine which are unused)
        let unused_params: Vec<String> =
            if let Some(param_list) = self.type_info.fn_param_types.get(name) {
                param_list
                    .iter()
                    .filter(|(pn, _)| !fn_used_names.contains(pn.as_str()))
                    .map(|(pn, _)| pn.clone())
                    .collect()
            } else {
                fd.params
                    .items
                    .iter()
                    .filter_map(|p| crate::native_proto::infer::binding_name(&p.pattern))
                    .filter(|pn| !fn_used_names.contains(*pn))
                    .map(|pn| pn.to_string())
                    .collect()
            };
        for pname in &unused_params {
            self.write_indent();
            self.write(&format!("_ = _{};\n", pname));
        }

        if let Some(body) = &fd.body {
            for stmt in &body.statements {
                self.emit_fn_stmt(stmt);
            }
        }
        // If function has non-void return type but no explicit return,
        // add a default return 0 to avoid Zig compile error.
        if !self.seen_return && ret_zig_type != "void" {
            self.write_indent();
            self.write("return 0;\n");
        }
        self.indent -= 1;
        self.writeln("}");

        // If this is an export function, add to exported_fns for C ABI wrapper generation.
        if self.current_fn_is_export {
            let func_name = name.to_string();
            let return_type = self.current_fn_return_type.clone().unwrap_or(ZigType::I64);

            // Get parameter types from type_info.
            let params: Vec<ZigType> = self
                .type_info
                .fn_param_types
                .get(name)
                .map(|p| {
                    p.iter()
                        .filter(|(n, _)| !is_async || n != "io")
                        .map(|(_, t)| t.clone())
                        .collect()
                })
                .unwrap_or_default();

            self.exported_fns.push(ExportedFunction {
                name: func_name,
                params,
                return_type,
                can_throw: self.fn_has_throw,
            });
        }
        self.current_fn = saved_current_fn;
    }
}

// ── Function body statements ─────────────────────────

impl Codegen {
    pub(crate) fn emit_fn_stmt(&mut self, stmt: &Statement) {
        // Flush any function definitions that were deferred from a previous statement.
        if !self.pending_expr_fns.is_empty() {
            let pending = std::mem::take(&mut self.pending_expr_fns);
            for def in pending {
                self.write(&def);
            }
        }

        // Snapshot: if this statement generates new deferred function definitions,
        // they must be inserted BEFORE the statement output (Zig requires def-before-use).
        let snapshot = self.output.len();

        match stmt {
            Statement::VariableDeclaration(vd) => {
                self.emit_var_decl(vd);
            }
            Statement::ReturnStatement(rs) => {
                self.write_indent();
                if let Some(arg) = &rs.argument {
                    self.write("return ");
                    self.in_return_expr = true;
                    self.emit_expr(arg);
                    self.in_return_expr = false;
                    self.write(";\n");
                } else {
                    self.write("return;\n");
                }
                self.seen_return = true;
            }
            Statement::ExpressionStatement(es) => {
                // Special handling for forEach/some/every: they generate 'for' loops (statements), not expressions.
                // If we add ';' after a 'for' loop, Zig will report a syntax error.
                let mut need_semi = true;
                if let Expression::CallExpression(ce) = &es.expression
                    && let Some(builtin) = builtins::detect_builtin_call(ce)
                {
                    match builtin {
                        builtins::BuiltinCall::ArrayForEach
                        | builtins::BuiltinCall::ArraySome
                        | builtins::BuiltinCall::ArrayEvery
                        | builtins::BuiltinCall::ArrayFill => {
                            // These generate 'for' loops (statements), no ';' needed
                            need_semi = false;
                        }
                        _ => {}
                    }
                }

                self.write_indent();
                self.in_expr_stmt = true;
                self.emit_expr(&es.expression);
                self.in_expr_stmt = false;
                if need_semi {
                    self.write(";\n");
                } else {
                    self.write("\n");
                }
            }
            Statement::IfStatement(is) => {
                self.emit_if(is);
            }
            Statement::WhileStatement(ws) => {
                self.emit_while(ws);
            }
            Statement::DoWhileStatement(dws) => {
                self.emit_do_while(dws);
            }
            Statement::ForStatement(fs) => {
                self.emit_for(fs);
            }
            Statement::ForOfStatement(fos) => {
                self.emit_for_of(fos);
            }
            Statement::ForInStatement(fis) => {
                self.emit_for_in(fis);
            }
            Statement::SwitchStatement(ss) => {
                self.emit_switch(ss);
            }
            Statement::BreakStatement(bs) => {
                self.write_indent();
                if let Some(ref label) = bs.label {
                    self.write(&format!(
                        "break :{};\n",
                        self.zig_safe_name(label.name.as_str())
                    ));
                } else {
                    self.write("break;\n");
                }
            }
            Statement::ContinueStatement(cs) => {
                self.write_indent();
                if let Some(ref label) = cs.label {
                    self.write(&format!(
                        "continue :{};\n",
                        self.zig_safe_name(label.name.as_str())
                    ));
                } else {
                    self.write("continue;\n");
                }
            }
            Statement::LabeledStatement(ls) => {
                // labeled statement → Zig labeled block or loop
                let label_name = self.zig_safe_name(ls.label.name.as_str());
                match &ls.body {
                    // For loops: label attaches directly to the loop syntax
                    Statement::WhileStatement(_) => {
                        self.write_indent();
                        self.write(&format!("{}: ", label_name));
                        self.emit_while_labeled(match &ls.body {
                            Statement::WhileStatement(ws) => ws,
                            _ => unreachable!(),
                        });
                    }
                    Statement::ForStatement(_) => {
                        self.write_indent();
                        self.write(&format!("{}: ", label_name));
                        self.emit_for_labeled(match &ls.body {
                            Statement::ForStatement(fs) => fs,
                            _ => unreachable!(),
                        });
                    }
                    Statement::ForOfStatement(_) => {
                        self.write_indent();
                        self.write(&format!("{}: ", label_name));
                        self.emit_for_of_labeled(match &ls.body {
                            Statement::ForOfStatement(fos) => fos,
                            _ => unreachable!(),
                        });
                    }
                    Statement::ForInStatement(_) => {
                        self.write_indent();
                        self.write(&format!("{}: ", label_name));
                        self.emit_for_in_labeled(match &ls.body {
                            Statement::ForInStatement(fis) => fis,
                            _ => unreachable!(),
                        });
                    }
                    Statement::DoWhileStatement(_) => {
                        self.write_indent();
                        self.write(&format!("{}: ", label_name));
                        self.emit_do_while_labeled(match &ls.body {
                            Statement::DoWhileStatement(dws) => dws,
                            _ => unreachable!(),
                        });
                    }
                    _ => {
                        // Generic labeled block (for if/switch/block etc)
                        self.write_indent();
                        self.writeln(&format!("{}: {{", label_name));
                        self.indent += 1;
                        self.emit_fn_stmt(&ls.body);
                        self.indent -= 1;
                        self.write_indent();
                        self.writeln("}");
                    }
                }
            }
            Statement::BlockStatement(bs) => {
                self.emit_block(bs);
            }
            Statement::ThrowStatement(ts) => {
                // JS throw expr → Zig: evaluate expr for side effects, then propagate error.
                // If inside a try block, use `break :label error.JsThrow` so the catch handler
                // catches it. Otherwise, `return error.JsThrow`.
                self.write_indent();
                self.write("_ = ");
                self.in_return_expr = true;
                self.emit_expr(&ts.argument);
                self.in_return_expr = false;
                self.write(";\n");

                if let Some(ref label) = self.inside_try_block.clone() {
                    // Inside try-catch: break the try block with error
                    self.write_indent();
                    self.writeln(&format!(
                        "break :{} @as(anyerror!void, error.JsThrow);",
                        label
                    ));
                } else {
                    // Bare throw: propagate to function return
                    self.write_indent();
                    self.writeln("return error.JsThrow;");
                }
                self.seen_return = true;
            }
            Statement::TryStatement(ts) => {
                // JS try { ... } catch(e) { ... } finally { ... }
                //
                // Two code paths:
                //   A) Try body has throw → labeled block + catch handler (real semantics)
                //   B) Try body has no throw → emit body inline, skip catch (unreachable)

                // Pre-scan: does the try body contain throw statements,
                // or is there a catch handler (need to generate catch code path)?
                // Also check for nested try-catch (which might re-throw).
                let has_throw = ts.block.body.iter().any(|s| Self::stmt_has_throw_any(s));
                let needs_catch = ts.handler.is_some();
                let has_nested_try = ts
                    .block
                    .body
                    .iter()
                    .any(|s| matches!(s, Statement::TryStatement(_)));

                if !has_throw && !needs_catch && !has_nested_try {
                    // Case B1 (finally only, no throw, no catch):
                    // emit body, then emit finally inline after (not defer).
                    for stmt in &ts.block.body {
                        self.emit_fn_stmt(stmt);
                    }
                    if let Some(ref finalizer) = ts.finalizer {
                        for stmt in &finalizer.body {
                            self.emit_fn_stmt(stmt);
                        }
                    }
                    return; // caller continues
                }

                if !has_throw && !has_nested_try {
                    // Case B2 (catch handler present but no throw, no nested try):
                    // Catch is unreachable. Emit body + finally inline, skip handler.
                    for stmt in &ts.block.body {
                        self.emit_fn_stmt(stmt);
                    }
                    if let Some(ref finalizer) = ts.finalizer {
                        for stmt in &finalizer.body {
                            self.emit_fn_stmt(stmt);
                        }
                    }
                    return; // caller continues
                }

                // Case A: try body has throw, or has nested try-catch
                // → always generate full labeled block + if-else catch handler

                let label_id = self.try_label_counter;
                self.try_label_counter += 1;
                let blk_label = format!("_js_try_blk_{}", label_id);
                let result_var = format!("_js_try_{}", label_id);

                // ── Try body + catch handler inside single labeled block ──
                // Uses if-else instead of `catch |err| {}` so that `break :label`
                // from catch body (re-throw) stays in scope.
                //
                // Capture parent's inside_try_block BEFORE start_try_block
                // overwrites it. Used later to propagate re-throw errors upward.
                let saved_inside = self.inside_try_block.clone();
                self.start_try_block(&blk_label);
                self.write_indent();
                self.writeln(&format!(
                    "const {}: anyerror!void = {blk}: {{",
                    result_var,
                    blk = blk_label,
                ));
                self.indent += 1;

                // ── Finally as defer (always runs, inside labeled block) ──
                if let Some(ref finalizer) = ts.finalizer {
                    self.write_indent();
                    self.writeln("defer {");
                    self.indent += 1;
                    for stmt in &finalizer.body {
                        self.emit_fn_stmt(stmt);
                    }
                    self.indent -= 1;
                    self.write_indent();
                    self.writeln("}");
                }

                // ── Try body as const with explicit anyerror!void type ──
                // Using a standalone const ensures the labeled block has
                // the correct error union type regardless of whether the
                // body throws or not.
                let body_label = format!("_js_try_body_{}", label_id);
                let body_blk_label = format!("_js_try_body_blk_{}", label_id);
                self.write_indent();
                self.writeln(&format!(
                    "const {}: anyerror!void = {}: {{",
                    body_label, body_blk_label,
                ));
                self.indent += 1;

                // Set inside_try_block so throw → break :body_blk_label
                self.inside_try_block = Some(body_blk_label.clone());

                let seen_before = self.seen_return;
                self.seen_return = false;
                for stmt in &ts.block.body {
                    self.emit_fn_stmt(stmt);
                }
                let body_exited = self.seen_return;
                self.seen_return = seen_before;

                if !body_exited {
                    self.write_indent();
                    self.writeln(&format!("break :{} {{}};", body_blk_label));
                }

                self.indent -= 1;
                self.write_indent();
                self.writeln("};");

                // ── Catch handler as if-else (in scope of blk_label) ──
                self.write_indent();
                self.writeln(&format!("if ({}) |_| {{", body_label));
                self.indent += 1;
                // Success: no error, fall through
                self.indent -= 1;
                self.write_indent();
                self.writeln("} else |err| {");
                self.indent += 1;

                if let Some(ref handler) = ts.handler {
                    // Bind catch parameter
                    if let Some(ref param) = handler.param
                        && let BindingPattern::BindingIdentifier(ref id) = param.pattern
                    {
                        let name = id.name.as_str();
                        let is_referenced = stmt_list_references_name(&handler.body.body, name);
                        self.write_indent();
                        if is_referenced {
                            self.writeln(&format!(
                                "const {} = @errorName(err);",
                                self.zig_safe_name(name)
                            ));
                        } else {
                            self.writeln("_ = @errorName(err);");
                        }
                    }

                    // Set inside_try_block to blk_label so that `throw` in catch
                    // body generates `break :blk_label error.JsThrow` (re-throw),
                    // NOT `return error.JsThrow` (which would skip outer catch).
                    self.inside_try_block = Some(blk_label.clone());
                    let cb_before = self.seen_return;
                    self.seen_return = false;
                    for stmt in &handler.body.body {
                        self.emit_fn_stmt(stmt);
                    }
                    let catch_threw = self.seen_return;
                    self.seen_return = cb_before;

                    // If catch body didn't re-throw, fall through normally
                    if !catch_threw {
                        // Nothing; falls through to break after if-else
                    }
                } else {
                    // No handler: discard the error (try-finally with throw)
                    self.write_indent();
                    self.writeln("_ = err;");
                }

                self.indent -= 1;
                self.write_indent();
                self.writeln("}");

                // ── Normal completion (no re-throw from catch) ──
                self.write_indent();
                self.writeln(&format!("break :{blk} {{}};", blk = blk_label));

                self.indent -= 1;
                self.write_indent();
                self.writeln("};");

                // ── Propagate unhandled error from re-throw ──
                // If catch body re-threw (break :blk_label error.JsThrow),
                // the const becomes error.JsThrow.
                // When inside a parent try body, break to parent (outer catch
                // intercepts it). Otherwise, return from the function.
                if ts.handler.is_some() {
                    self.write_indent();
                    if let Some(ref parent_body_label) = saved_inside {
                        self.writeln(&format!(
                            "if ({0}) |_| {{}} else |_| break :{1} @as(anyerror!void, error.JsThrow);",
                            result_var, parent_body_label
                        ));
                    } else {
                        self.writeln(&format!(
                            "if ({}) |_| {{}} else |_| return error.JsThrow;",
                            result_var
                        ));
                    }
                } else {
                    self.write_indent();
                    self.writeln(&format!("_ = {};", result_var));
                }

                // Restore inside_try_block
                self.inside_try_block = saved_inside;

                self.end_try_block();
            }
            Statement::FunctionDeclaration(fd) => {
                // Nested function declaration: hoist as inline struct with .call() method.
                let fn_name = fd.id.as_ref().map(|id| id.name.as_str());
                let Some(fn_name) = fn_name else {
                    self.write_indent();
                    self.writeln("// error: nested function must have a name");
                    return;
                };
                let safe_fn_name = self.zig_safe_name(fn_name);

                // Detect captured variables from enclosing scope
                let captures = self.detect_fn_body_captures(fd);

                if !captures.is_empty() {
                    // Has captures: generate struct with capture fields + instance
                    self.nested_fn_names.insert(fn_name.to_string());

                    // Zig 0.16 does not support `struct { .. }.{ .. }` inline syntax.
                    // Use a separate type declaration to avoid `} .{` on same line.
                    let type_name = format!("_{safe_fn_name}_type");
                    self.write_indent();
                    self.writeln(&format!("const {type_name} = struct {{"));
                    self.indent += 1;

                    // Add capture fields to struct
                    for (cap_name, cap_type, _is_mut) in &captures {
                        let zig_type = cap_type.to_zig_type();
                        self.write_indent();
                        self.writeln(&format!("{}: {},", self.zig_safe_name(cap_name), zig_type));
                    }

                    // Set current_captured so variable references are rewritten to self.xxx
                    let saved_captured = std::mem::take(&mut self.current_captured);
                    self.current_captured = captures.clone();

                    // Generate function body
                    let old_current_fn = self.current_fn.clone();
                    self.current_fn = Some(fn_name.to_string());

                    let old_fn_has_throw = self.fn_has_throw;
                    let old_seen_return = self.seen_return;
                    self.fn_has_throw = false;
                    self.seen_return = false;

                    let old_nested = self.current_nested_fn_name.take();
                    self.current_nested_fn_name = Some(fn_name.to_string());

                    self.emit_fn(fd);

                    self.current_nested_fn_name = old_nested;
                    self.fn_has_throw = old_fn_has_throw;
                    self.seen_return = old_seen_return;
                    self.current_fn = old_current_fn;

                    // Restore current_captured
                    self.current_captured = saved_captured;

                    self.indent -= 1;
                    self.write_indent();
                    self.writeln("};");

                    // Create instance with captured values (named type syntax)
                    let mut init_fields = String::new();
                    for (i, (cap_name, _, _)) in captures.iter().enumerate() {
                        if i > 0 {
                            init_fields.push_str(", ");
                        }
                        let safe_cap = self.zig_safe_name(cap_name);
                        init_fields.push_str(&format!(".{} = {}", safe_cap, safe_cap));
                    }
                    self.write_indent();
                    self.writeln(&format!(
                        "const {safe_fn_name} = {type_name}{{ {init_fields} }};"
                    ));
                } else {
                    // No captures: generate inline struct with static call method
                    self.nested_fn_names.insert(fn_name.to_string());

                    self.write_indent();
                    self.writeln(&format!("const {} = struct {{", safe_fn_name));
                    self.indent += 1;

                    let old_current_fn = self.current_fn.clone();
                    self.current_fn = Some(fn_name.to_string());

                    let old_fn_has_throw = self.fn_has_throw;
                    let old_seen_return = self.seen_return;
                    self.fn_has_throw = false;
                    self.seen_return = false;

                    let old_nested = self.current_nested_fn_name.take();
                    self.current_nested_fn_name = Some(fn_name.to_string());

                    self.emit_fn(fd);

                    self.current_nested_fn_name = old_nested;
                    self.fn_has_throw = old_fn_has_throw;
                    self.seen_return = old_seen_return;
                    self.current_fn = old_current_fn;

                    self.indent -= 1;
                    self.write_indent();
                    self.writeln("};");
                }
            }
            Statement::WithStatement(_ws) => {
                // 🔘 with statement: not supported (deprecated in strict mode)
                self.compile_error_stmt(
                    GetSpan::span(stmt),
                    "with statement is not supported and deprecated in strict mode. Use explicit property access instead.",
                );
            }
            _ => {
                // Unsupported statement type: generate @compileError
                self.write_indent();
                self.compile_error_stmt(GetSpan::span(stmt), "Unsupported statement type");
            }
        }

        // If this statement generated deferred function definitions (e.g., arrow functions
        // or function expressions used as values), insert them BEFORE the statement output.
        // Zig requires definitions to appear before use.
        if !self.pending_expr_fns.is_empty() {
            let pending = std::mem::take(&mut self.pending_expr_fns);
            // Extract the post-snapshot portion (the statement we just emitted)
            let statement_output = self.output[snapshot..].to_string();
            self.output.truncate(snapshot);
            // Emit deferred definitions first, then the statement
            for def in pending {
                self.write(&def);
            }
            self.write(&statement_output);
        }
    }
}

// ── If / Else ──────────────────────────────────────

impl Codegen {
    pub(crate) fn emit_if(&mut self, is: &IfStatement) {
        self.write_indent();
        self.write("if (");
        self.emit_expr(&is.test);
        self.write(") {\n");

        self.indent += 1;
        self.emit_stmt_or_block(&is.consequent);
        self.indent -= 1;

        if let Some(alt) = &is.alternate {
            let inner: &Statement = alt;
            match inner {
                Statement::IfStatement(else_if) => {
                    self.write_indent();
                    self.write("} else ");
                    self.emit_if(else_if);
                    return;
                }
                other => {
                    self.writeln("} else {");
                    self.indent += 1;
                    self.emit_stmt_or_block(other);
                    self.indent -= 1;
                }
            }
        }
        self.writeln("}");
    }

    fn emit_stmt_or_block(&mut self, stmt: &Statement) {
        match stmt {
            Statement::BlockStatement(bs) => {
                for s in &bs.body {
                    self.emit_fn_stmt(s);
                }
            }
            _ => self.emit_fn_stmt(stmt),
        }
    }

    fn emit_block(&mut self, bs: &BlockStatement) {
        self.writeln("{");
        self.indent += 1;
        for stmt in &bs.body {
            self.emit_fn_stmt(stmt);
        }
        self.indent -= 1;
        self.writeln("}");
    }
}

// ── While / Do-While / For-Of / Switch ───────────

impl Codegen {
    pub(crate) fn emit_while(&mut self, ws: &WhileStatement) {
        self.write_indent();
        self.write("while (");
        self.emit_expr(&ws.test);
        self.write(") {\n");
        self.indent += 1;
        self.emit_stmt_or_block(&ws.body);
        self.indent -= 1;
        self.writeln("}");
    }

    fn emit_while_labeled(&mut self, ws: &WhileStatement) {
        // Label already written by LabeledStatement handler, no indent needed
        self.write("while (");
        self.emit_expr(&ws.test);
        self.write(") {\n");
        self.indent += 1;
        self.emit_stmt_or_block(&ws.body);
        self.indent -= 1;
        self.writeln("}");
    }

    // JS:  do { ... } while (cond);
    // Zig: while (true) { ...; if (cond) {} else { break; } }
    fn emit_do_while(&mut self, dws: &DoWhileStatement) {
        self.write_indent();
        self.writeln("while (true) {");

        self.indent += 1;
        self.emit_stmt_or_block(&dws.body);
        self.write_indent();
        self.write("if (");
        self.emit_expr(&dws.test);
        self.write(") {} else { break; }\n");

        self.indent -= 1;

        self.writeln("}");
    }

    fn emit_do_while_labeled(&mut self, dws: &DoWhileStatement) {
        self.writeln("while (true) {");
        self.indent += 1;
        self.emit_stmt_or_block(&dws.body);
        self.write_indent();
        self.write("if (");
        self.emit_expr(&dws.test);
        self.write(") {} else { break; }\n");
        self.indent -= 1;
        self.writeln("}");
    }

    // JS:  for (init; test; update) { ... }
    // Zig: { init; while (test) : (update) { ... } }
    fn emit_for(&mut self, fs: &ForStatement) {
        self.write_indent();
        self.emit_for_body(fs);
    }

    fn emit_for_labeled(&mut self, fs: &ForStatement) {
        self.emit_for_body(fs);
    }

    fn emit_for_body(&mut self, fs: &ForStatement) {
        self.write("{\n");
        self.indent += 1;

        // init
        if let Some(init) = &fs.init {
            if let ForStatementInit::VariableDeclaration(vd) = init {
                for decl in &vd.declarations {
                    if let Some(name) = crate::native_proto::infer::binding_name(&decl.id) {
                        self.write_indent();
                        let safe_name = self.zig_safe_name(name);
                        if let Some(init_expr) = &decl.init {
                            // Emit the actual initializer value (e.g. `let i = 1` → `var i: i64 = 1`)
                            let init_text = self.capture_expr(init_expr);
                            self.write(&format!(
                                "var {}: i64 = {};\n",
                                safe_name,
                                init_text.trim()
                            ));
                        } else {
                            self.write(&format!("var {}: i64 = 0;\n", safe_name));
                        }
                    }
                }
            } else {
                // Expression init: emit as statement
                if let Some(expr) = init.as_expression() {
                    self.write_indent();
                    self.emit_expr(expr);
                    self.write(";\n");
                }
            }
        }

        // test — generate a while loop
        self.write_indent();
        self.write("while (");
        if let Some(test) = &fs.test {
            self.emit_expr(test);
        } else {
            self.write("true");
        }
        self.write(")");

        // update — must be a void expression in Zig
        if let Some(update) = &fs.update {
            self.write(" : ({ ");
            let saved = self.in_expr_stmt;
            self.in_expr_stmt = true;
            self.emit_expr(update);
            self.in_expr_stmt = saved;
            self.write("; })");
        }

        self.write(" {\n");
        self.indent += 1;
        self.emit_stmt_or_block(&fs.body);
        self.indent -= 1;

        self.write_indent();
        self.write("}\n");

        // Close the block
        self.indent -= 1;
        self.write_indent();
        self.write("}\n");
    }

    // JS:  for (const x of iterable) { ... }
    // Zig: for (iterable) |x| { ... }
    //  Map/Set: var __it = obj.inner.iterator(); while (__it.next()) |__kv| { const x = __kv.key_ptr.*; ... }
    fn emit_for_of(&mut self, fos: &ForOfStatement) {
        // 🔘 for await...of: not supported
        if fos.r#await {
            self.compile_error_stmt(
                GetSpan::span(fos),
                "for await...of is not supported. Use synchronous for...of instead.",
            );
            return;
        }
        // Map / Set → HashMap iterator pattern
        if self.detect_map_set_iter(&fos.right, fos) {
            return;
        }

        let var_name = match &fos.left {
            ForStatementLeft::VariableDeclaration(vd) => vd
                .declarations
                .first()
                .and_then(|decl| self.binding_name(&decl.id))
                .map(|n| self.zig_safe_name(n))
                .unwrap_or_else(|| "item".to_string()),
            _ => "item".to_string(),
        };

        // Check if the iterable is an ArrayList variable
        let iterable_is_arraylist = match &fos.right {
            Expression::Identifier(id) => self
                .type_info
                .var_types
                .get(id.name.as_str())
                .map(|t| matches!(t, ZigType::ArrayList(_)))
                .unwrap_or(false),
            _ => false,
        };

        self.write_indent();
        self.write("for (");
        self.emit_expr(&fos.right);
        if iterable_is_arraylist {
            self.write(".items");
        }
        self.write(&format!(") |{}| {{\n", var_name));

        self.indent += 1;
        self.emit_stmt_or_block(&fos.body);
        self.indent -= 1;

        self.writeln("}");
    }

    fn emit_for_of_labeled(&mut self, fos: &ForOfStatement) {
        // Map / Set → HashMap iterator pattern
        if self.detect_map_set_iter(&fos.right, fos) {
            return;
        }

        let var_name = match &fos.left {
            ForStatementLeft::VariableDeclaration(vd) => vd
                .declarations
                .first()
                .and_then(|decl| self.binding_name(&decl.id))
                .map(|n| self.zig_safe_name(n))
                .unwrap_or_else(|| "item".to_string()),
            _ => "item".to_string(),
        };
        let iterable_is_arraylist = match &fos.right {
            Expression::Identifier(id) => self
                .type_info
                .var_types
                .get(id.name.as_str())
                .map(|t| matches!(t, ZigType::ArrayList(_)))
                .unwrap_or(false),
            _ => false,
        };
        self.write("for (");
        self.emit_expr(&fos.right);
        if iterable_is_arraylist {
            self.write(".items");
        }
        self.write(&format!(") |{}| {{\n", var_name));
        self.indent += 1;
        self.emit_stmt_or_block(&fos.body);
        self.indent -= 1;
        self.writeln("}");
    }

    /// Detect if the for-of iterable is a Map or Set, and if so emit the
    /// HashMap iterator pattern. Returns true if the pattern was matched.
    ///
    /// Generated pattern:
    /// ```zig
    /// var __it = obj.inner.iterator();
    /// while (__it.next()) |__kv| {
    ///     const {var} = __kv.key_ptr.*;          // single-var Map / Set
    ///     // or for destructured Map:
    ///     const key = __kv.key_ptr.*;
    ///     const val = __kv.value_ptr.*;
    ///     ...
    /// }
    /// ```
    fn detect_map_set_iter(&mut self, right: &Expression, fos: &ForOfStatement) -> bool {
        let (obj_name, is_map) = match right {
            Expression::Identifier(id) => match self.type_info.var_types.get(id.name.as_str()) {
                Some(ZigType::NamedStruct(name)) if name == "Map" => (id.name.to_string(), true),
                Some(ZigType::NamedStruct(name)) if name == "Set" => (id.name.to_string(), false),
                _ => return false,
            },
            _ => return false,
        };

        // Check if left side is an ArrayPattern destructure ([key, val])
        let is_destructure = match &fos.left {
            ForStatementLeft::VariableDeclaration(vd) => vd
                .declarations
                .first()
                .map(|decl| matches!(&decl.id, BindingPattern::ArrayPattern(_)))
                .unwrap_or(false),
            _ => false,
        };

        // Infer the var name(s)
        let var_decls: Vec<String> = if is_destructure && is_map {
            self.extract_destructure_names(&fos.left)
        } else {
            let name = match &fos.left {
                ForStatementLeft::VariableDeclaration(vd) => vd
                    .declarations
                    .first()
                    .and_then(|decl| self.binding_name(&decl.id))
                    .unwrap_or("item")
                    .to_string(),
                _ => "item".to_string(),
            };
            vec![name]
        };

        // Emit: var __it = obj.inner.iterator();
        self.write_indent();
        self.write(&format!(
            "var __it = {obj}.inner.iterator();\n",
            obj = obj_name
        ));

        // Emit: while (__it.next()) |__kv| {
        self.write_indent();
        self.write("while (__it.next()) |__kv| {\n");
        self.indent += 1;

        if is_destructure && is_map && var_decls.len() >= 2 {
            // Destructure: const key = __kv.key_ptr.*;  const val = __kv.value_ptr.*;
            self.write_indent();
            self.write(&format!(
                "const {key} = __kv.key_ptr.*;\n",
                key = var_decls[0]
            ));
            self.write_indent();
            self.write(&format!(
                "const {val} = __kv.value_ptr.*;\n",
                val = var_decls[1]
            ));
        } else {
            // Single var: const x = __kv.key_ptr.*;
            self.write_indent();
            self.write(&format!(
                "const {var} = __kv.key_ptr.*;\n",
                var = var_decls[0]
            ));
        }

        // Emit the loop body
        self.emit_stmt_or_block(&fos.body);

        self.indent -= 1;
        self.write_indent();
        self.write("}\n"); // while

        true
    }

    /// Extract binding names from a destructuring ArrayPattern like [key, val].
    fn extract_destructure_names(&self, left: &ForStatementLeft) -> Vec<String> {
        if let ForStatementLeft::VariableDeclaration(vd) = left
            && let Some(decl) = vd.declarations.first()
            && let BindingPattern::ArrayPattern(ap) = &decl.id
        {
            return ap
                .elements
                .iter()
                .filter_map(|elem| elem.as_ref().and_then(|pat| self.binding_name(pat)))
                .map(|s| s.to_string())
                .collect();
        }
        Vec::new()
    }

    /// JS: for (var key in obj) { ... }
    /// Zig:
    ///   - HashMap: var it = obj.iterator(); while (it.next()) |kv| { const key = kv.key_ptr.*; ... }
    ///   - Static struct: unroll loop — one block per field with const key = "fieldName"
    fn emit_for_in(&mut self, fis: &ForInStatement) {
        let var_name = match &fis.left {
            ForStatementLeft::VariableDeclaration(vd) => vd
                .declarations
                .first()
                .and_then(|decl| self.binding_name(&decl.id))
                .map(|n| self.zig_safe_name(n))
                .unwrap_or_else(|| "key".to_string()),
            ForStatementLeft::AssignmentTargetIdentifier(id) => {
                // for (key in obj) — key is an existing variable
                self.zig_safe_name(id.name.as_str())
            }
            _ => "key".to_string(),
        };

        // Get the object name from the right side (must be a simple identifier)
        let obj_name = match &fis.right {
            Expression::Identifier(id) => id.name.to_string(),
            _ => {
                // Non-identifier expressions not supported for for-in
                self.write_indent();
                self.compile_error_stmt(fis.span, "for-in only supported with identifier objects");
                return;
            }
        };

        // Check if the object is a dynamic object (HashMap-like).
        let obj_type = self.type_info.var_types.get(&obj_name);

        // Case 1: HashMap → iterator-based for-in
        let is_dynamic = obj_type
            .map(|t| matches!(t, ZigType::Anytype))
            .unwrap_or(false);

        if is_dynamic {
            self.write_indent();
            self.write(&format!(
                "{{ var __it = {obj}.iterator(); while (__it.next()) |__kv| {{ const {var} = __kv.key_ptr.*;\n",
                obj = obj_name,
                var = var_name
            ));
            self.indent += 1;
            self.emit_stmt_or_block(&fis.body);
            self.indent -= 1;
            self.write_indent();
            self.writeln(&format!("}}}} // for-in {}", obj_name));
            return;
        }

        // Case 2: Static struct with known fields → unroll loop
        if let Some(ZigType::Struct(fields)) = obj_type
            && !fields.is_empty()
        {
            let fields: Vec<_> = fields.iter().map(|(n, t)| (n.clone(), t.clone())).collect();
            for (field_name, _) in &fields {
                self.write_indent();
                self.writeln("{");
                self.indent += 1;
                self.write_indent();
                self.writeln(&format!("const {} = \"{}\";", var_name, field_name));
                self.emit_stmt_or_block(&fis.body);
                self.indent -= 1;
                self.write_indent();
                self.writeln("}");
            }
            return;
        }

        // Case 3: Unknown type → compile error
        self.write_indent();
        self.compile_error_stmt_fmt(
            fis.span,
            format!("for-in: '{}' is not a dynamic object", obj_name),
        );
    }

    fn emit_for_in_labeled(&mut self, fis: &ForInStatement) {
        let var_name = match &fis.left {
            ForStatementLeft::VariableDeclaration(vd) => vd
                .declarations
                .first()
                .and_then(|decl| self.binding_name(&decl.id))
                .map(|n| self.zig_safe_name(n))
                .unwrap_or_else(|| "key".to_string()),
            ForStatementLeft::AssignmentTargetIdentifier(id) => {
                self.zig_safe_name(id.name.as_str())
            }
            _ => "key".to_string(),
        };
        let obj_name = match &fis.right {
            Expression::Identifier(id) => id.name.to_string(),
            _ => {
                self.compile_error_stmt(fis.span, "for-in only supported with identifier objects");
                return;
            }
        };
        let obj_type = self.type_info.var_types.get(&obj_name);
        let is_dynamic = obj_type
            .map(|t| matches!(t, ZigType::Anytype))
            .unwrap_or(false);
        if is_dynamic {
            self.write(&format!(
                "{{ var __it = {obj}.iterator(); while (__it.next()) |__kv| {{ const {var} = __kv.key_ptr.*;\n",
                obj = obj_name, var = var_name
            ));
            self.indent += 1;
            self.emit_stmt_or_block(&fis.body);
            self.indent -= 1;
            self.write_indent();
            self.writeln(&format!("}}}} // for-in {}", obj_name));
            return;
        }
        if let Some(ZigType::Struct(fields)) = obj_type
            && !fields.is_empty()
        {
            let fields: Vec<_> = fields.iter().map(|(n, t)| (n.clone(), t.clone())).collect();
            for (field_name, _) in &fields {
                self.write_indent();
                self.writeln("{");
                self.indent += 1;
                self.write_indent();
                self.writeln(&format!("const {} = \"{}\";", var_name, field_name));
                self.emit_stmt_or_block(&fis.body);
                self.indent -= 1;
                self.write_indent();
                self.writeln("}");
            }
            return;
        }
        self.compile_error_stmt_fmt(
            fis.span,
            format!("for-in: '{}' is not a dynamic object", obj_name),
        );
    }

    // JS:  switch (expr) { case v: ...; break; default: ... }
    // Zig: switch (expr) { v => { ... }, else => { ... }, }
    fn emit_switch(&mut self, ss: &SwitchStatement) {
        self.write_indent();

        self.write("switch (");
        self.emit_expr(&ss.discriminant);
        self.write(") {\n");

        self.indent += 1;
        let mut has_default = false;

        for case in ss.cases.iter() {
            self.write_indent();
            if let Some(test) = &case.test {
                self.emit_expr(test);
            } else {
                has_default = true;
                self.write("else");
            }
            self.write(" => {\n");

            self.indent += 1;
            for stmt in &case.consequent {
                // Skip break statements (not needed in Zig switch)
                if let Statement::BreakStatement(_) = stmt {
                    continue;
                }
                self.emit_fn_stmt(stmt);
            }
            self.indent -= 1;

            self.write_indent();
            self.write("},\n");
        }

        // Zig switch must be exhaustive; add empty else if no default
        if !has_default {
            self.write_indent();
            self.writeln("else => {},");
        }

        self.indent -= 1;

        self.writeln("}");
    }

    /// Recursively collect all identifier names from a statement and its children.
    fn collect_stmt_idents(stmt: &Statement, names: &mut HashSet<String>) {
        match stmt {
            Statement::ExpressionStatement(es) => {
                Self::collect_expr_idents(&es.expression, names);
            }
            Statement::ReturnStatement(rs) => {
                if let Some(arg) = &rs.argument {
                    Self::collect_expr_idents(arg, names);
                }
            }
            Statement::IfStatement(is) => {
                Self::collect_expr_idents(&is.test, names);
                Self::collect_stmt_idents(&is.consequent, names);
                if let Some(alt) = &is.alternate {
                    Self::collect_stmt_idents(alt, names);
                }
            }
            Statement::WhileStatement(ws) => {
                Self::collect_expr_idents(&ws.test, names);
                Self::collect_stmt_idents(&ws.body, names);
            }
            Statement::DoWhileStatement(dws) => {
                Self::collect_stmt_idents(&dws.body, names);
                Self::collect_expr_idents(&dws.test, names);
            }
            Statement::ForStatement(fs) => {
                if let Some(test) = &fs.test {
                    Self::collect_expr_idents(test, names);
                }
                if let Some(update) = &fs.update {
                    Self::collect_expr_idents(update, names);
                }
                Self::collect_stmt_idents(&fs.body, names);
            }
            Statement::ForOfStatement(fos) => {
                Self::collect_expr_idents(&fos.right, names);
                Self::collect_stmt_idents(&fos.body, names);
            }
            Statement::VariableDeclaration(vd) => {
                for decl in &vd.declarations {
                    if let Some(init) = &decl.init {
                        Self::collect_expr_idents(init, names);
                    }
                }
            }
            Statement::BlockStatement(bs) => {
                for s in &bs.body {
                    Self::collect_stmt_idents(s, names);
                }
            }
            Statement::TryStatement(ts) => {
                for s in &ts.block.body {
                    Self::collect_stmt_idents(s, names);
                }
                if let Some(handler) = &ts.handler {
                    for s in &handler.body.body {
                        Self::collect_stmt_idents(s, names);
                    }
                }
            }
            Statement::SwitchStatement(ss) => {
                Self::collect_expr_idents(&ss.discriminant, names);
                for case in &ss.cases {
                    if let Some(test) = &case.test {
                        Self::collect_expr_idents(test, names);
                    }
                    for s in &case.consequent {
                        Self::collect_stmt_idents(s, names);
                    }
                }
            }
            _ => {}
        }
    }

    /// Recursively collect all identifier names from an expression.
    fn collect_expr_idents(expr: &Expression, names: &mut HashSet<String>) {
        match expr {
            Expression::Identifier(id) => {
                names.insert(id.name.to_string());
            }
            Expression::BinaryExpression(be) => {
                Self::collect_expr_idents(&be.left, names);
                Self::collect_expr_idents(&be.right, names);
            }
            Expression::CallExpression(ce) => {
                Self::collect_expr_idents(&ce.callee, names);
                for arg in &ce.arguments {
                    if let Some(e) = arg.as_expression() {
                        Self::collect_expr_idents(e, names);
                    }
                }
            }
            Expression::AssignmentExpression(ae) => {
                if let AssignmentTarget::AssignmentTargetIdentifier(id) = &ae.left {
                    names.insert(id.name.to_string());
                }
                Self::collect_expr_idents(&ae.right, names);
            }
            Expression::UnaryExpression(ue) => {
                Self::collect_expr_idents(&ue.argument, names);
            }
            Expression::AwaitExpression(ae) => {
                Self::collect_expr_idents(&ae.argument, names);
            }
            Expression::UpdateExpression(ue) => {
                if let SimpleAssignmentTarget::AssignmentTargetIdentifier(id) = &ue.argument {
                    names.insert(id.name.to_string());
                }
            }
            Expression::LogicalExpression(le) => {
                Self::collect_expr_idents(&le.left, names);
                Self::collect_expr_idents(&le.right, names);
            }
            Expression::ConditionalExpression(ce) => {
                Self::collect_expr_idents(&ce.test, names);
                Self::collect_expr_idents(&ce.consequent, names);
                Self::collect_expr_idents(&ce.alternate, names);
            }
            Expression::ArrayExpression(ae) => {
                for elem in &ae.elements {
                    if let Some(e) = elem.as_expression() {
                        Self::collect_expr_idents(e, names);
                    }
                }
            }
            Expression::StaticMemberExpression(mem) => {
                Self::collect_expr_idents(&mem.object, names);
            }
            Expression::ComputedMemberExpression(mem) => {
                Self::collect_expr_idents(&mem.object, names);
                Self::collect_expr_idents(&mem.expression, names);
            }
            Expression::NewExpression(ne) => {
                Self::collect_expr_idents(&ne.callee, names);
                for arg in &ne.arguments {
                    if let Some(e) = arg.as_expression() {
                        Self::collect_expr_idents(e, names);
                    }
                }
            }
            Expression::ParenthesizedExpression(pe) => {
                Self::collect_expr_idents(&pe.expression, names);
            }
            Expression::TemplateLiteral(tl) => {
                for e in &tl.expressions {
                    Self::collect_expr_idents(e, names);
                }
            }
            _ => {}
        }
    }
}

// ── Arrow function support ─────────────────────────────

impl Codegen {
    /// Emit an arrow function as a Zig function.
    /// Generates the function definition and returns the function name.
    /// Detect which variables are mutated (assigned to or updated) in a list of statements.
    /// Returns a set of variable names that are mutated.
    fn detect_mutated_vars_in_stmts(stmts: &[Statement]) -> std::collections::HashSet<String> {
        let mut mutated = std::collections::HashSet::new();
        for stmt in stmts {
            Self::detect_mutated_in_stmt(stmt, &mut mutated);
        }
        mutated
    }

    fn detect_mutated_in_stmt(stmt: &Statement, mutated: &mut std::collections::HashSet<String>) {
        match stmt {
            Statement::ExpressionStatement(es) => {
                Self::detect_mutated_in_expr(&es.expression, mutated);
            }
            Statement::ReturnStatement(rs) => {
                if let Some(expr) = &rs.argument {
                    Self::detect_mutated_in_expr(expr, mutated);
                }
            }
            Statement::BlockStatement(bs) => {
                for s in &bs.body {
                    Self::detect_mutated_in_stmt(s, mutated);
                }
            }
            Statement::IfStatement(is) => {
                Self::detect_mutated_in_expr(&is.test, mutated);
                Self::detect_mutated_in_stmt(&is.consequent, mutated);
                if let Some(alt) = &is.alternate {
                    Self::detect_mutated_in_stmt(alt, mutated);
                }
            }
            Statement::WhileStatement(ws) => {
                Self::detect_mutated_in_expr(&ws.test, mutated);
                Self::detect_mutated_in_stmt(&ws.body, mutated);
            }
            Statement::ForStatement(fs) => {
                if let Some(test) = &fs.test {
                    Self::detect_mutated_in_expr(test, mutated);
                }
                if let Some(update) = &fs.update {
                    Self::detect_mutated_in_expr(update, mutated);
                }
                Self::detect_mutated_in_stmt(&fs.body, mutated);
            }
            Statement::ForOfStatement(fos) => {
                Self::detect_mutated_in_expr(&fos.right, mutated);
                Self::detect_mutated_in_stmt(&fos.body, mutated);
            }
            Statement::SwitchStatement(ss) => {
                Self::detect_mutated_in_expr(&ss.discriminant, mutated);
                for case in &ss.cases {
                    for s in &case.consequent {
                        Self::detect_mutated_in_stmt(s, mutated);
                    }
                }
            }
            Statement::TryStatement(ts) => {
                for s in &ts.block.body {
                    Self::detect_mutated_in_stmt(s, mutated);
                }
                if let Some(handler) = &ts.handler {
                    for s in &handler.body.body {
                        Self::detect_mutated_in_stmt(s, mutated);
                    }
                }
                if let Some(finalizer) = &ts.finalizer {
                    for s in &finalizer.body {
                        Self::detect_mutated_in_stmt(s, mutated);
                    }
                }
            }
            _ => {}
        }
    }

    fn detect_mutated_in_expr(expr: &Expression, mutated: &mut std::collections::HashSet<String>) {
        match expr {
            Expression::AssignmentExpression(ae) => {
                // The assignment target is mutated
                if let AssignmentTarget::AssignmentTargetIdentifier(id) = &ae.left {
                    mutated.insert(id.name.to_string());
                }
                // Also check the right side (might contain mutations)
                Self::detect_mutated_in_expr(&ae.right, mutated);
            }
            Expression::UpdateExpression(ue) => {
                // x++ or ++x
                if let SimpleAssignmentTarget::AssignmentTargetIdentifier(id) = &ue.argument {
                    mutated.insert(id.name.to_string());
                }
            }
            Expression::BinaryExpression(be) => {
                Self::detect_mutated_in_expr(&be.left, mutated);
                Self::detect_mutated_in_expr(&be.right, mutated);
            }
            Expression::CallExpression(ce) => {
                Self::detect_mutated_in_expr(&ce.callee, mutated);
                for arg in &ce.arguments {
                    if let Some(e) = arg.as_expression() {
                        Self::detect_mutated_in_expr(e, mutated);
                    }
                }
            }
            Expression::LogicalExpression(le) => {
                Self::detect_mutated_in_expr(&le.left, mutated);
                Self::detect_mutated_in_expr(&le.right, mutated);
            }
            Expression::ConditionalExpression(ce) => {
                Self::detect_mutated_in_expr(&ce.test, mutated);
                Self::detect_mutated_in_expr(&ce.consequent, mutated);
                Self::detect_mutated_in_expr(&ce.alternate, mutated);
            }
            Expression::UnaryExpression(ue) => {
                Self::detect_mutated_in_expr(&ue.argument, mutated);
            }
            Expression::AwaitExpression(ae) => {
                Self::detect_mutated_in_expr(&ae.argument, mutated);
            }
            _ => {}
        }
    }

    /// Collect locally declared variable names from a list of statements.
    /// These variables (const/let/var in the function body) are NOT captures.
    fn collect_local_declarations(
        stmts: &oxc_allocator::Vec<'_, Statement>,
    ) -> std::collections::HashSet<String> {
        let mut names = std::collections::HashSet::new();
        for stmt in stmts.iter() {
            if let Statement::VariableDeclaration(var_decl) = stmt {
                for declarator in &var_decl.declarations {
                    if let Some(name) = crate::native_proto::infer::binding_name(&declarator.id) {
                        names.insert(name.to_string());
                    }
                }
            }
        }
        names
    }

    /// Collect captured variables from an arrow function body.
    /// A variable is "captured" if it's referenced in the body but is not a parameter
    /// and not a locally declared variable.
    /// Correctly sets `is_mut` by detecting mutations in the arrow body.
    fn collect_captured_vars(
        &self,
        arrow: &ArrowFunctionExpression,
    ) -> Vec<(String, ZigType, bool)> {
        let mut captured = Vec::new();
        let mut seen = std::collections::HashSet::new();

        // Collect parameter names + locally declared variable names
        let mut local_names: std::collections::HashSet<String> = arrow
            .params
            .items
            .iter()
            .filter_map(|p| crate::native_proto::infer::binding_name(&p.pattern))
            .map(|s| s.to_string())
            .collect();
        local_names.extend(Self::collect_local_declarations(&arrow.body.statements));

        // Walk the body statements to find Identifier references
        for stmt in &arrow.body.statements {
            Self::collect_idents_from_stmt(
                stmt,
                &mut captured,
                &mut seen,
                &local_names,
                &self.type_info,
            );
        }

        // Detect which captured variables are mutated in the arrow body
        let mutated = Self::detect_mutated_vars_in_stmts(&arrow.body.statements);
        // Update is_mut for each captured variable
        for (name, _ztype, is_mut) in &mut captured {
            *is_mut = mutated.contains(name);
        }

        captured
    }

    /// Detect variables captured by a nested function declaration.
    /// Returns list of (variable_name, ZigType, is_mutable) for variables from the
    /// enclosing scope that are referenced in the function body but are not parameters
    /// and not locally declared variables.
    fn detect_fn_body_captures(&self, fd: &Function) -> Vec<(String, ZigType, bool)> {
        let mut captured = Vec::new();
        let mut seen = std::collections::HashSet::new();

        // Collect parameter names + locally declared variable names
        let mut local_names: std::collections::HashSet<String> = fd
            .params
            .items
            .iter()
            .filter_map(|p| crate::native_proto::infer::binding_name(&p.pattern))
            .map(|s| s.to_string())
            .collect();

        // Walk the body statements to find Identifier references
        if let Some(body) = &fd.body {
            local_names.extend(Self::collect_local_declarations(&body.statements));
            for stmt in &body.statements {
                Self::collect_idents_from_stmt(
                    stmt,
                    &mut captured,
                    &mut seen,
                    &local_names,
                    &self.type_info,
                );
            }

            // Detect which captured variables are mutated in the body
            let mutated = Self::detect_mutated_vars_in_stmts(&body.statements);
            for (name, _ztype, is_mut) in &mut captured {
                *is_mut = mutated.contains(name);
            }
        }

        captured
    }

    /// Helper: collect identifiers from a statement
    fn collect_idents_from_stmt(
        stmt: &Statement,
        captured: &mut Vec<(String, ZigType, bool)>,
        seen: &mut std::collections::HashSet<String>,
        local_names: &std::collections::HashSet<String>,
        type_info: &crate::native_proto::TypeCheckResult,
    ) {
        match stmt {
            Statement::ExpressionStatement(es) => {
                Self::collect_idents_from_expr(
                    &es.expression,
                    captured,
                    seen,
                    local_names,
                    type_info,
                );
            }
            Statement::ReturnStatement(ret) => {
                if let Some(expr) = &ret.argument {
                    Self::collect_idents_from_expr(expr, captured, seen, local_names, type_info);
                }
            }
            Statement::VariableDeclaration(var_decl) => {
                // Process init expressions (right-hand side) — they may reference
                // outer variables that need to be captured. The binding names (left-hand
                // side) are local and already in `local_names`.
                for declarator in &var_decl.declarations {
                    if let Some(init) = &declarator.init {
                        Self::collect_idents_from_expr(
                            init,
                            captured,
                            seen,
                            local_names,
                            type_info,
                        );
                    }
                }
            }
            _ => {}
        }
    }

    /// Helper: collect identifiers from an expression
    fn collect_idents_from_expr(
        expr: &Expression,
        captured: &mut Vec<(String, ZigType, bool)>,
        seen: &mut std::collections::HashSet<String>,
        local_names: &std::collections::HashSet<String>,
        type_info: &crate::native_proto::TypeCheckResult,
    ) {
        use oxc_ast::ast::Expression;
        match expr {
            Expression::Identifier(id) => {
                let name = id.name.as_str();
                // Skip parameters and locally declared variables — they are not captures
                if !local_names.contains(name) && !seen.contains(name) {
                    seen.insert(name.to_string());
                    let ztype = type_info
                        .var_types
                        .get(name)
                        .cloned()
                        .unwrap_or(ZigType::I64);
                    // TODO: properly detect if captured var is mutated in arrow body
                    let is_mut = false;
                    captured.push((name.to_string(), ztype, is_mut));
                }
            }
            Expression::BinaryExpression(be) => {
                Self::collect_idents_from_expr(&be.left, captured, seen, local_names, type_info);
                Self::collect_idents_from_expr(&be.right, captured, seen, local_names, type_info);
            }
            Expression::CallExpression(ce) => {
                for arg in &ce.arguments {
                    if let Some(expr) = arg.as_expression() {
                        Self::collect_idents_from_expr(
                            expr,
                            captured,
                            seen,
                            local_names,
                            type_info,
                        );
                    }
                }
                Self::collect_idents_from_expr(&ce.callee, captured, seen, local_names, type_info);
            }
            _ => {}
        }
    }

    /// Get the return type string for an arrow function.
    fn arrow_return_type_str(&self, arrow: &ArrowFunctionExpression) -> &'static str {
        let inferred = self.infer_arrow_return_type(arrow);
        match inferred {
            Some(ZigType::I64) => "i64",
            Some(ZigType::F64) => "f64",
            Some(ZigType::Bool) => "bool",
            Some(ZigType::Str) => "[]const u8",
            Some(ZigType::Void) => "void",
            Some(_) => "i64", // NamedStruct, ArrayList, etc. — use i64 fallback
            None => {
                // When type is indeterminate:
                // - Single-expression arrow: always returns a value → i64
                // - Block-body without return → void
                if arrow.body.statements.len() == 1
                    && matches!(arrow.body.statements[0], Statement::ExpressionStatement(_))
                {
                    "i64"
                } else {
                    // Check if any statement in the block is a return
                    let has_return = arrow
                        .body
                        .statements
                        .iter()
                        .any(|s| matches!(s, Statement::ReturnStatement(_)));
                    if has_return { "i64" } else { "void" }
                }
            }
        }
    }

    /// Infer the return type of an arrow function by examining its body.
    fn infer_arrow_return_type(&self, arrow: &ArrowFunctionExpression) -> Option<ZigType> {
        // Single-expression arrow: type is the expression's type
        if arrow.body.statements.len() == 1
            && let Statement::ExpressionStatement(es) = &arrow.body.statements[0]
        {
            return self.infer_arrow_expr_type(&es.expression);
        }
        // Block body: scan return statements
        for stmt in &arrow.body.statements {
            if let Statement::ReturnStatement(rs) = stmt {
                if let Some(ref arg) = rs.argument {
                    return self.infer_arrow_expr_type(arg);
                }
                return None; // bare `return;` means void
            }
        }
        None // no return → void
    }

    /// Best-effort type inference for arrow body expressions.
    fn infer_arrow_expr_type(&self, expr: &Expression) -> Option<ZigType> {
        match expr {
            Expression::NumericLiteral(nl) => {
                if let Some(raw) = &nl.raw {
                    let s = raw.as_str();
                    if s.contains('.') || s.contains('e') || s.contains('E') {
                        Some(ZigType::F64)
                    } else {
                        Some(ZigType::I64)
                    }
                } else {
                    Some(ZigType::I64)
                }
            }
            Expression::StringLiteral(_) => Some(ZigType::Str),
            Expression::BooleanLiteral(_) => Some(ZigType::Bool),
            Expression::Identifier(id) => self.type_info.var_types.get(id.name.as_str()).cloned(),
            Expression::BinaryExpression(be) => {
                // Heuristic: try left operand first (covers patterns like `x * 2`, `x > 0`)
                self.infer_arrow_expr_type(&be.left)
                    .or_else(|| self.infer_arrow_expr_type(&be.right))
            }
            Expression::UnaryExpression(ue) => self.infer_arrow_expr_type(&ue.argument),
            Expression::CallExpression(ce) => {
                // Look up callee in fn_return_types
                if let Expression::Identifier(id) = &ce.callee {
                    self.type_info
                        .fn_return_types
                        .get(id.name.as_str())
                        .cloned()
                } else {
                    None
                }
            }
            Expression::StaticMemberExpression(sme) => {
                // Handle patterns like obj.prop, arr.length etc.
                let field = sme.property.name.as_str();
                match field {
                    "length" | "len" => Some(ZigType::I64),
                    _ => None,
                }
            }
            Expression::ConditionalExpression(ce) => {
                // For ternary, prefer consequent type (they should match)
                self.infer_arrow_expr_type(&ce.consequent)
                    .or_else(|| self.infer_arrow_expr_type(&ce.alternate))
            }
            _ => None,
        }
    }
    /// Generates the struct definition (with fields and call method) and stores it in self.closure_defs.
    /// Returns the struct name.
    fn emit_closure_struct(
        &mut self,
        arrow: &ArrowFunctionExpression,
        captured: Vec<(String, ZigType, bool)>,
    ) -> String {
        let struct_name = format!("Closure_{}", self.arrow_counter);
        self.arrow_counter += 1;

        // Store closure info for assignment site (so emit_var_decl can generate instantiation)
        self.closure_vars
            .insert(struct_name.clone(), captured.clone());

        // ── Temporarily redirect output to build the struct definition ──
        let old_output = std::mem::take(&mut self.output);
        let old_indent = self.indent;
        self.output = String::new();
        self.indent = 0;

        // ── Struct definition ──
        self.writeln(&format!("const {} = struct {{", struct_name));
        self.indent = 1;

        // Fields for captured variables
        // Value capture (is_mut=false):  T
        // Reference capture (is_mut=true): *T  (the closure holds a pointer)
        for (name, ztype, is_mut) in &captured {
            let tstr = match ztype {
                ZigType::I64 => "i64".to_string(),
                ZigType::F64 => "f64".to_string(),
                ZigType::Bool => "bool".to_string(),
                ZigType::Str => "[]const u8".to_string(),
                ZigType::Void => "void".to_string(),
                ZigType::NamedStruct(s) => s.clone(),
                ZigType::ArrayList(_) => "std.ArrayList(JsAny)".to_string(),
                _ => "i64".to_string(),
            };
            if *is_mut {
                // Reference capture: store a pointer so the closure can mutate the outer variable
                self.writeln(&format!("{}: *{},", name, tstr));
            } else {
                // Value capture: store a copy
                self.writeln(&format!("{}: {},", name, tstr));
            }
        }

        // ── call method (single-line signature) ──
        let mut sig = String::from("fn call(self: *@This()");
        for param in &arrow.params.items {
            sig.push_str(", ");
            if let Some(pname) = crate::native_proto::infer::binding_name(&param.pattern) {
                sig.push_str(&format!("{}: anytype", pname));
            }
        }
        // Infer return type
        sig.push_str(&format!(") {} {{", self.arrow_return_type_str(arrow)));
        self.writeln(&sig);
        self.indent += 1;

        // ── Generate method body ──
        // Set current_captured so emit_expr rewrites identifiers to self.xxx
        let saved_captured = std::mem::take(&mut self.current_captured);
        self.current_captured = captured.clone();

        // Check if arrow function has expression body (single ExpressionStatement without return)
        // In JS: `(y) => x + y` → oxc parses as ExpressionStatement(x + y)
        // In Zig: need to add `return` prefix
        if arrow.body.statements.len() == 1 {
            if let Statement::ExpressionStatement(es) = &arrow.body.statements[0] {
                self.write_indent();
                self.write("return ");
                self.emit_expr(&es.expression);
                self.write(";\n");
            } else {
                // Block body with statements
                for stmt in &arrow.body.statements {
                    self.emit_fn_stmt(stmt);
                }
            }
        } else {
            // Multiple statements or empty: generate as-is
            for stmt in &arrow.body.statements {
                self.emit_fn_stmt(stmt);
            }
        }

        // Restore
        self.current_captured = saved_captured;

        self.indent = 1;
        self.writeln("}");

        self.indent = 0;
        self.writeln("};");

        // ── Get the generated struct definition and restore output ──
        let struct_def = std::mem::take(&mut self.output);
        self.output = old_output;
        self.indent = old_indent;

        // Store in closure_defs (will be prepended to output later)
        self.closure_defs.push(struct_def);

        struct_name
    }

    pub(crate) fn emit_arrow_function(&mut self, arrow: &ArrowFunctionExpression) -> String {
        // Detect captured variables (closure check)
        let captured = self.collect_captured_vars(arrow);
        if !captured.is_empty() {
            // Generate closure struct (captures outer variables)
            let struct_name = self.emit_closure_struct(arrow, captured);
            return struct_name;
        }
        // No captured vars: generate struct with call method (Zig 0.16 does not
        // allow nested `fn` declarations with return statements inside function
        // bodies, so we use the same struct+call pattern as closures).
        let fn_name = format!("_arrow_fn_{}", self.arrow_counter);
        self.arrow_counter += 1;
        self.nested_fn_names.insert(fn_name.clone());

        // Struct definition
        self.write_indent();
        self.writeln(&format!("const {} = struct {{", fn_name));
        self.indent += 1;

        // call method signature
        let mut sig = String::from("pub fn call(");
        for (param_idx, param) in arrow.params.items.iter().enumerate() {
            if param_idx > 0 {
                sig.push_str(", ");
            }
            if let Some(pname) = crate::native_proto::infer::binding_name(&param.pattern) {
                sig.push_str(&format!("{}: anytype", pname));
            }
        }
        sig.push_str(&format!(") {} {{", self.arrow_return_type_str(arrow)));
        self.write_indent();
        self.writeln(&sig);

        // Generate function body
        self.indent += 1;

        // Handle body: for single-expression arrows, the body is a FunctionBody
        // with a single ExpressionStatement.
        // We need to generate "return expr;" for the expression.
        if arrow.body.statements.len() == 1 {
            if let Statement::ExpressionStatement(es) = &arrow.body.statements[0] {
                // Single-expression arrow: generate "return expr;"
                self.write_indent();
                self.write("return ");
                self.emit_expr(&es.expression);
                self.write(
                    ";
",
                );
            } else {
                // Block body with a single statement (not expression)
                for stmt in &arrow.body.statements {
                    self.emit_fn_stmt(stmt);
                }
            }
        } else {
            // Block body with multiple statements
            for stmt in &arrow.body.statements {
                self.emit_fn_stmt(stmt);
            }
        }

        self.indent -= 1;
        self.write_indent();
        self.writeln("}");

        // Close struct
        self.indent -= 1;
        self.write_indent();
        self.writeln("};");

        fn_name
    }
}

/// Emit a FunctionExpression as a struct+instance inline.
/// Returns the instance name for use as an expression value.
impl Codegen {
    pub(crate) fn emit_fn_expr(&mut self, func: &Function) -> String {
        // Determine name: use function's own id if present, else generate unique name
        let name = func
            .id
            .as_ref()
            .map(|id| id.name.to_string())
            .unwrap_or_else(|| {
                let n = format!("_fn_expr_{}", self.fn_expr_counter);
                self.fn_expr_counter += 1;
                n
            });
        let safe_name = self.zig_safe_name(&name);

        // Detect captured variables from enclosing scope
        let captures = self.detect_fn_body_captures(func);

        // Save state
        let old_current_fn = self.current_fn.clone();
        let old_fn_has_throw = self.fn_has_throw;
        let old_seen_return = self.seen_return;
        let old_captured = std::mem::take(&mut self.current_captured);

        self.current_fn = Some(name.clone());

        // Pre-scan for throw
        let has_throw = func
            .body
            .as_ref()
            .is_some_and(|b| Codegen::has_throw_in_body(b));
        self.fn_has_throw = has_throw;

        // Read pre-computed return type from type_info
        let ret_ty = self.type_info.fn_return_types.get(&name).cloned();
        self.current_fn_return_type = ret_ty.clone();

        if !captures.is_empty() {
            // Has captures: generate struct with capture fields + instance
            self.nested_fn_names.insert(name.clone());

            self.write_indent();
            self.writeln(&format!("const {} = struct {{", safe_name));
            self.indent += 1;

            // Add capture fields
            for (cap_name, cap_type, _is_mut) in &captures {
                let zig_type = cap_type.to_zig_type();
                self.write_indent();
                self.writeln(&format!("{}: {},", self.zig_safe_name(cap_name), zig_type));
            }

            self.current_captured = captures.clone();

            // Generate call method
            let old_nested = self.current_nested_fn_name.take();
            self.current_nested_fn_name = Some(name.clone());
            self.emit_fn(func);
            self.current_nested_fn_name = old_nested;
            self.current_captured.clear();

            self.indent -= 1;
            self.write_indent();

            // Create instance
            let mut init = String::from(".{{ ");
            for (i, (cap_name, _, _)) in captures.iter().enumerate() {
                if i > 0 {
                    init.push_str(", ");
                }
                let safe_cap = self.zig_safe_name(cap_name);
                init.push_str(&format!(".{} = {}", safe_cap, safe_cap));
            }
            init.push_str(" }};");
            self.writeln(&init);
        } else {
            // No captures: generate inline struct with static call method
            self.nested_fn_names.insert(name.clone());

            self.write_indent();
            self.writeln(&format!("const {} = struct {{", safe_name));
            self.indent += 1;

            let old_nested = self.current_nested_fn_name.take();
            self.current_nested_fn_name = Some(name.clone());
            self.emit_fn(func);
            self.current_nested_fn_name = old_nested;

            self.indent -= 1;
            self.write_indent();
            self.writeln("};");
        }

        // Restore state
        self.current_fn = old_current_fn;
        self.fn_has_throw = old_fn_has_throw;
        self.seen_return = old_seen_return;
        self.current_captured = old_captured;
        self.current_fn_return_type = ret_ty;

        safe_name
    }
}
/// (new Int32Array(...), new Uint8Array(...), new Float64Array(...)).
/// Returns the Zig type suffix (e.g., "I32", "U8", "F64") if it is.
pub(crate) fn typedarray_init_type(expr: &Expression) -> Option<&'static str> {
    match expr {
        Expression::NewExpression(ne) => {
            if let Expression::Identifier(id) = &ne.callee {
                match id.name.as_str() {
                    "Int32Array" => Some("I32"),
                    "Uint8Array" => Some("U8"),
                    "Float64Array" => Some("F64"),
                    _ => None,
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Check if an expression is `new RegExp(...)` — a dynamic RegExp constructor.
/// Also returns true for RegExp literals (`/pattern/`).
/// Returns true if it matches,  the codegen can track the variable as a RegExp object.
pub(crate) fn is_regexp_new(expr: &Expression) -> bool {
    match expr {
        Expression::NewExpression(ne) => {
            if let Expression::Identifier(id) = &ne.callee {
                id.name.as_str() == "RegExp"
            } else {
                false
            }
        }
        Expression::RegExpLiteral(_) => true, // RegExp literal /pattern/
        _ => false,
    }
}

/// Extract the string name from a PropertyKey (IdentifierName or StringLiteral).
pub(crate) fn property_key_name(key: &PropertyKey) -> Option<String> {
    match key {
        PropertyKey::StaticIdentifier(id) => Some(id.name.to_string()),
        PropertyKey::StringLiteral(sl) => Some(sl.value.to_string()),
        PropertyKey::PrivateIdentifier(id) => Some(id.name.to_string()),
        _ => None,
    }
}

/// Check if a MethodDefinition is `constructor()`.
fn is_constructor_method(md: &MethodDefinition) -> bool {
    if let Some(name) = property_key_name(&md.key) {
        name == "constructor"
    } else {
        false
    }
}

// ── Class declaration helpers ────────────────────────

/// Convert a simple expression to a Zig default value string.
/// Used for class field default values (e.g., `#secret = 42` → "42").
fn expr_to_default_str(expr: &Expression) -> String {
    match expr {
        Expression::NumericLiteral(n) => format!("{}", n.value),
        Expression::StringLiteral(s) => format!("\"{}\"", s.value),
        Expression::BooleanLiteral(b) => format!("{}", b.value),
        Expression::NullLiteral(_) => "null".to_string(),
        _ => "0".to_string(), // fallback: use zero
    }
}

// ── Class declaration codegen ────────────────────────

impl Codegen {
    /// Emit a class declaration as a Zig struct with methods.
    pub(crate) fn emit_class(&mut self, cd: &Class) {
        let class_name = cd
            .id
            .as_ref()
            .map(|id| id.name.as_str())
            .unwrap_or("AnonymousClass");
        let class_name_s = class_name.to_string();
        let safe_class_name = self.zig_safe_name(class_name);

        // Collect fields and methods from the class body
        let mut field_names: Vec<String> = Vec::new();
        let mut field_types: Vec<ZigType> = Vec::new();
        let mut field_defaults: Vec<Option<String>> = Vec::new();
        let mut static_field_names: Vec<String> = Vec::new();
        let mut has_constructor = false;

        // First pass: collect field names from property definitions
        for elem in &cd.body.body {
            match elem {
                ClassElement::PropertyDefinition(pd) => {
                    let is_static = pd.r#static;
                    let is_computed = pd.computed;
                    if is_computed {
                        continue; // skip computed properties
                    }
                    if let Some(name) = property_key_name(&pd.key) {
                        if is_static {
                            if !static_field_names.contains(&name) {
                                static_field_names.push(name);
                            }
                        } else if !field_names.contains(&name) {
                            // Look up field type from TypeInferrer first (before name is moved)
                            let field_ty = self
                                .type_info
                                .class_field_types
                                .get(&class_name_s)
                                .and_then(|fields| fields.get(&name))
                                .cloned()
                                .unwrap_or(ZigType::I64);
                            // Extract default value from initializer (e.g., `secret = 42` → "42")
                            let default_val = pd.value.as_ref().map(expr_to_default_str);
                            field_names.push(name);
                            field_types.push(field_ty);
                            field_defaults.push(default_val);
                        }
                    }
                }
                ClassElement::MethodDefinition(md) if is_constructor_method(md) => {
                    has_constructor = true;
                }
                _ => {}
            }
        }

        // Second pass: scan constructor body for `this.x = expr` implicit fields
        {
            let mut constructor_stmts: Option<&[Statement]> = None;
            for elem in &cd.body.body {
                if let ClassElement::MethodDefinition(md) = elem
                    && is_constructor_method(md)
                    && let Some(body) = &md.value.body
                {
                    constructor_stmts = Some(&body.statements);
                    break;
                }
            }
            if let Some(body_stmts) = constructor_stmts {
                self.collect_implicit_class_fields(
                    body_stmts,
                    &class_name_s,
                    &mut field_names,
                    &mut field_types,
                    &mut field_defaults,
                );
            }
        }

        self.class_names.insert(class_name_s.clone());

        // ── Generate struct definition ──
        self.writeln(&format!("const {} = struct {{", safe_class_name));

        // Emit struct fields
        self.indent += 1;
        for (i, fname) in field_names.iter().enumerate() {
            let ftype = &field_types[i];
            self.writeln(&format!("{}: {},", fname, ftype.to_zig_type()));
        }
        self.writeln("");

        // Save current state
        let saved_class = self.current_class.take();

        // Emit methods (constructor → init, regular methods → pub fn methodName)
        for elem in &cd.body.body {
            match elem {
                ClassElement::MethodDefinition(md) => {
                    self.emit_class_method(class_name, &field_names, md);
                }
                ClassElement::PropertyDefinition(pd)
                    // Static property initializers
                    if pd.r#static && !pd.computed => {
                        self.emit_static_field_init(pd);
                    }
                // 🔘 static {} blocks: not supported — generate @compileError
                ClassElement::StaticBlock(sb) => {
                    self.compile_error_stmt(
                        sb.span,
                        "static {} blocks are not supported. Use static field initializers instead.",
                    );
                }
                _ => {}
            }
        }

        // If no explicit constructor, generate a default init()
        if !has_constructor {
            self.writeln("");
            self.writeln(&format!("pub fn init() {} {{", safe_class_name));
            self.indent += 1;
            if field_names.is_empty() {
                self.writeln("return .{};");
            } else {
                let indent = "    ".repeat(self.indent);
                self.write(&format!("{}return .{{ ", indent));
                for (i, fname) in field_names.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    let default_val = field_defaults
                        .get(i)
                        .and_then(|d| d.as_deref())
                        .unwrap_or("0");
                    self.write(&format!(".{} = {}", fname, default_val));
                }
                self.write(" };\n");
            }
            self.indent -= 1;
            self.writeln("}");
        }

        // Restore state
        self.current_class = saved_class;

        self.indent -= 1;
        self.writeln("};");
        self.writeln("");
    }

    /// Recursively scan constructor body for `this.x = expr` assignments
    /// and add discovered fields to field_names/field_types (if not already present).
    fn collect_implicit_class_fields(
        &self,
        stmts: &[Statement],
        class_name: &str,
        field_names: &mut Vec<String>,
        field_types: &mut Vec<ZigType>,
        field_defaults: &mut Vec<Option<String>>,
    ) {
        for stmt in stmts {
            match stmt {
                Statement::ExpressionStatement(es) => {
                    if let Expression::AssignmentExpression(ae) = &es.expression
                        && let AssignmentTarget::StaticMemberExpression(sme) = &ae.left
                        && matches!(&sme.object, Expression::ThisExpression(_))
                    {
                        let fname = sme.property.name.to_string();
                        if !field_names.contains(&fname) {
                            let ftype = self
                                .type_info
                                .class_field_types
                                .get(class_name)
                                .and_then(|fields| fields.get(&fname))
                                .cloned()
                                .unwrap_or(ZigType::I64);
                            field_names.push(fname);
                            field_types.push(ftype);
                            field_defaults.push(None); // set by constructor, no static default
                        }
                    }
                }
                Statement::IfStatement(is) => {
                    self.collect_implicit_class_fields(
                        std::slice::from_ref(&is.consequent),
                        class_name,
                        field_names,
                        field_types,
                        field_defaults,
                    );
                    if let Some(alt) = &is.alternate {
                        self.collect_implicit_class_fields(
                            std::slice::from_ref(alt),
                            class_name,
                            field_names,
                            field_types,
                            field_defaults,
                        );
                    }
                }
                Statement::BlockStatement(bs) => {
                    self.collect_implicit_class_fields(
                        &bs.body,
                        class_name,
                        field_names,
                        field_types,
                        field_defaults,
                    );
                }
                _ => {}
            }
        }
    }

    /// Emit a class method (or constructor) as part of a struct.
    fn emit_class_method(
        &mut self,
        class_name: &str,
        field_names: &[String],
        md: &MethodDefinition,
    ) {
        let method_name = property_key_name(&md.key).unwrap_or_else(|| "anonymous".to_string());

        // Set current_class so `this.x` → `self.x` rewriting activates
        self.current_class = Some(class_name.to_string());

        if is_constructor_method(md) {
            // constructor → pub fn init(...)
            // The function body needs `return .{ .field1 = val1, ... };` if no explicit return
            self.emit_class_constructor(class_name, field_names, &md.value);
        } else {
            // Regular method → pub fn methodName(self: @This(), ...)
            self.emit_class_regular_method(class_name, &method_name, &md.value);
        }

        self.writeln("");
        self.current_class = None;
    }

    /// Emit the class constructor as `pub fn init(...) ClassName { ... }`.
    fn emit_class_constructor(
        &mut self,
        class_name: &str,
        field_names: &[String],
        func: &oxc_allocator::Box<'_, Function>,
    ) {
        // Check if constructor has a body
        let body_stmts = match &func.body {
            Some(body) => &body.statements,
            None => &[] as &[_],
        };

        // Check if body has explicit return
        let has_return = body_stmts
            .iter()
            .any(|s| matches!(s, Statement::ReturnStatement(_)));

        let saved_fn = self.current_fn.take();
        self.current_fn = Some("init".to_string());

        // Generate function signature
        // pub fn init(...) ClassName {
        self.writeln("");
        self.write("pub fn init(");

        // Parameters
        let mut param_idx = 0;
        for param in &func.params.items {
            if let Some(pname) = crate::native_proto::infer::binding_name(&param.pattern) {
                if param_idx > 0 {
                    self.write(", ");
                }
                // Read param type from type_info if available
                let ptype = self
                    .type_info
                    .fn_param_types
                    .get("init")
                    .and_then(|params| {
                        params
                            .iter()
                            .find(|(n, _)| n == pname)
                            .map(|(_, t)| t.clone())
                    })
                    .unwrap_or(ZigType::Anytype);
                self.write(&format!("{}: {}", pname, ptype.to_zig_type()));
                param_idx += 1;
            }
        }

        let safe_class_name = self.zig_safe_name(class_name);
        self.writeln(&format!(") {} {{", safe_class_name));

        // Emit body
        self.indent += 1;

        // If no explicit return, generate the struct return at the end
        // (fields are initialized in the body via `this.x = val` or `return .{...}`)
        if !has_return {
            // First emit the body statements (which may set local variables)
            for stmt in body_stmts {
                self.emit_stmt_with_this_rewrite(stmt, field_names);
            }

            // Then generate the struct return using field values
            // We assume variables with same names as fields exist
            self.write("return .{ ");
            for (i, fname) in field_names.iter().enumerate() {
                if i > 0 {
                    self.write(", ");
                }
                self.write(&format!(".{} = {}", fname, fname));
            }
            self.writeln(" };");
        } else {
            // Has explicit return — emit body as-is with this→self rewriting
            for stmt in body_stmts {
                self.emit_stmt_with_this_rewrite(stmt, field_names);
            }
        }

        self.indent -= 1;
        self.writeln("}");

        self.current_fn = saved_fn;
    }

    /// Emit a regular class method as `pub fn methodName(self: @This(), ...) RetTy { ... }`.
    fn emit_class_regular_method(
        &mut self,
        class_name: &str,
        method_name: &str,
        func: &oxc_allocator::Box<'_, Function>,
    ) {
        let saved_fn = self.current_fn.take();
        self.current_fn = Some(method_name.to_string());

        // Build fully-qualified key for TypeInferrer lookups
        let fq_method = format!("{}.{}", class_name, method_name);

        // Resolve return type:
        // 1. Check JSDoc @returns annotation
        // 2. Check TypeInferrer return types (fully-qualified key)
        // 3. Quick body scan for return statements
        // 4. Default to void
        let ret_ty = self
            .jsdoc_data
            .as_ref()
            .and_then(|d| d.return_types.get(method_name))
            .cloned()
            .or_else(|| {
                self.type_info
                    .fn_return_types
                    .get(&fq_method)
                    .map(|t| t.to_zig_type())
            })
            .or_else(|| {
                // Quick body scan: find return statement and infer type
                func.body.as_ref().and_then(|body| {
                    body.statements.iter().find_map(|s| {
                        if let Statement::ReturnStatement(rs) = s
                            && let Some(ret_expr) = &rs.argument
                        {
                            scan_ret_expr_type(ret_expr)
                        } else {
                            None
                        }
                    })
                })
            })
            .unwrap_or_else(|| "void".to_string());

        // Generate signature
        self.write(&format!("pub fn {}(self: @This()", method_name));

        // Parameters (skip self)
        // Look up with fully-qualified key first, fall back to plain method name
        let param_list = self
            .type_info
            .fn_param_types
            .get(&fq_method)
            .or_else(|| self.type_info.fn_param_types.get(method_name))
            .cloned();
        if let Some(params) = param_list {
            for (pname, ptype) in &params {
                self.write(", ");
                self.write(&format!("{}: {}", pname, ptype.to_zig_type()));
            }
        } else {
            // Fallback: generate from AST
            for param in &func.params.items {
                if let Some(pname) = crate::native_proto::infer::binding_name(&param.pattern) {
                    self.write(&format!(", {}: anytype", pname));
                }
            }
        }

        // Return type
        self.writeln(&format!(") {} {{", ret_ty));

        // Emit body with this→self rewriting
        self.indent += 1;

        if let Some(body) = &func.body {
            for stmt in &body.statements {
                self.emit_fn_stmt(stmt);
            }
        }

        self.indent -= 1;
        self.writeln("}");

        self.current_fn = saved_fn;
    }

    /// Emit a statement with `this.x` → `self.x` rewriting.
    /// This is used in constructor bodies where JS code assigns `this.field = value`.
    fn emit_stmt_with_this_rewrite(&mut self, stmt: &Statement, field_names: &[String]) {
        match stmt {
            Statement::ExpressionStatement(es) => {
                // Check if this is `this.field = value`
                if let Expression::AssignmentExpression(ae) = &es.expression
                    && let AssignmentTarget::StaticMemberExpression(sme) = &ae.left
                    && matches!(&sme.object, Expression::ThisExpression(_))
                {
                    // this.field = value → const field = value;
                    let fname = sme.property.name.to_string();
                    if field_names.contains(&fname) {
                        self.write_indent();
                        self.write("const ");
                        self.write(&fname);
                        self.write(" = ");
                        self.in_return_expr = true;
                        self.emit_expr(&ae.right);
                        self.in_return_expr = false;
                        self.writeln(";");
                        return;
                    }
                }
                // Fallback: emit as normal
                self.write_indent();
                self.in_expr_stmt = true;
                self.emit_expr(&es.expression);
                self.in_expr_stmt = false;
                self.writeln(";");
            }
            Statement::VariableDeclaration(vd) => {
                self.emit_var_decl(vd);
            }
            Statement::IfStatement(is) => {
                self.write_indent();
                self.write("if (");
                self.emit_expr(&is.test);
                self.writeln(") {");
                self.indent += 1;
                self.emit_stmt_with_this_rewrite(&is.consequent, field_names);
                self.indent -= 1;
                if let Some(alt) = &is.alternate {
                    self.writeln("} else {");
                    self.indent += 1;
                    self.emit_stmt_with_this_rewrite(alt, field_names);
                    self.indent -= 1;
                }
                self.writeln("}");
            }
            Statement::ReturnStatement(rs) => {
                self.write_indent();
                self.write("return ");
                if let Some(arg) = &rs.argument {
                    self.emit_expr(arg);
                }
                self.writeln(";");
            }
            Statement::BlockStatement(bs) => {
                self.writeln("{");
                self.indent += 1;
                for s in &bs.body {
                    self.emit_stmt_with_this_rewrite(s, field_names);
                }
                self.indent -= 1;
                self.writeln("}");
            }
            _ => {
                // For other statements, emit normally
                self.emit_fn_stmt(stmt);
            }
        }
    }

    /// Emit a static field initializer.
    fn emit_static_field_init(&mut self, pd: &PropertyDefinition) {
        if let Some(name) = property_key_name(&pd.key)
            && let Some(value) = &pd.value
        {
            self.writeln(&format!("pub const {} = ", name));
            self.indent += 1;
            self.emit_expr(value);
            self.writeln(";");
            self.indent -= 1;
        }
    }
}

/// Quick expression type scanner for return statements in class methods.
/// Returns a Zig type string ("i64", "f64", "[]const u8", "bool") or None.
fn scan_ret_expr_type(expr: &Expression) -> Option<String> {
    match expr {
        Expression::NumericLiteral(n) => {
            if n.value.fract() == 0.0 {
                Some("i64".to_string())
            } else {
                Some("f64".to_string())
            }
        }
        Expression::StringLiteral(_) => Some("[]const u8".to_string()),
        Expression::BooleanLiteral(_) => Some("bool".to_string()),
        Expression::BinaryExpression(be) => {
            let left = scan_ret_expr_type(&be.left);
            let right = scan_ret_expr_type(&be.right);
            match (left, right) {
                (Some(ref l), Some(ref r)) if l == "[]const u8" || r == "[]const u8" => {
                    Some("[]const u8".to_string())
                }
                (Some(ref l), _) => Some(l.clone()),
                (_, Some(ref r)) => Some(r.clone()),
                _ => Some("i64".to_string()),
            }
        }
        Expression::CallExpression(_) => None,
        Expression::StaticMemberExpression(sme) => {
            scan_ret_expr_type(&sme.object).or(Some("i64".to_string()))
        }
        Expression::Identifier(_) => None,
        Expression::ParenthesizedExpression(pe) => scan_ret_expr_type(&pe.expression),
        _ => Some("i64".to_string()),
    }
}
