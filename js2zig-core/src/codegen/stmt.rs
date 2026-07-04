// native_proto/codegen/stmt.rs
// Statement-level code generation: toplevel, var_decl, fn, if, while, for, switch.

use super::Codegen;
use super::helpers::zig_safe_name;
use crate::native_builtins as builtins;
use crate::types::{ExportedFunction, ZigType};
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

/// Recursively scan a statement tree for a `BreakStatement` targeting a specific label.
/// Used to determine whether a JS labeled statement actually needs a Zig labeled block.
fn has_break_to_label(stmt: &Statement, label_name: &str) -> bool {
    match stmt {
        Statement::BreakStatement(bs) => bs
            .label
            .as_ref()
            .is_some_and(|l| l.name.as_str() == label_name),
        Statement::BlockStatement(bs) => bs.body.iter().any(|s| has_break_to_label(s, label_name)),
        Statement::IfStatement(is) => {
            has_break_to_label(&is.consequent, label_name)
                || is
                    .alternate
                    .as_ref()
                    .is_some_and(|alt| has_break_to_label(alt, label_name))
        }
        Statement::LabeledStatement(ls) => {
            // Only scan inner body, not checking self
            has_break_to_label(&ls.body, label_name)
        }
        Statement::TryStatement(ts) => {
            ts.block
                .body
                .iter()
                .any(|s| has_break_to_label(s, label_name))
                || ts.handler.as_ref().is_some_and(|h| {
                    h.body
                        .body
                        .iter()
                        .any(|s| has_break_to_label(s, label_name))
                })
                || ts
                    .finalizer
                    .as_ref()
                    .is_some_and(|f| f.body.iter().any(|s| has_break_to_label(s, label_name)))
        }
        Statement::SwitchStatement(ss) => ss.cases.iter().any(|c| {
            c.consequent
                .iter()
                .any(|s| has_break_to_label(s, label_name))
        }),
        Statement::ForStatement(fs) => has_break_to_label(&fs.body, label_name),
        Statement::ForOfStatement(fos) => has_break_to_label(&fos.body, label_name),
        Statement::ForInStatement(fis) => has_break_to_label(&fis.body, label_name),
        Statement::WhileStatement(ws) => has_break_to_label(&ws.body, label_name),
        Statement::DoWhileStatement(dws) => has_break_to_label(&dws.body, label_name),
        _ => false,
    }
}

impl Codegen {
    /// Emit a variable declaration. Toplevel: only `const` allowed.
    /// Inside functions: `var` with type inference + undefined init.
    pub(crate) fn emit_var_decl(&mut self, vd: &VariableDeclaration) {
        for decl in &vd.declarations {
            if let Some(name) = crate::infer::binding_name(&decl.id) {
                let zig_name = self.zig_safe_name(name);

                // #946: Track variable names in function scope and detect shadowing
                // in nested blocks. Zig 0.16.0 forbids local variable shadowing of
                // outer scope declarations. When a var/let/const in a nested block
                // shadows a name already in fn_scope_vars, rename it and register the
                // mapping in shadow_renames so all references inside this block are
                // rewritten to the new name.
                let zig_name = if self.fn_scope_vars.contains(&zig_name) {
                    let shadow_id = self.names.next_shadow();
                    let renamed = format!("{}_shadow_{}", name, shadow_id);
                    if let Some(top_scope) = self.shadow_renames.last_mut() {
                        top_scope.insert(name.to_string(), renamed.clone());
                    }
                    self.fn_scope_vars.insert(renamed.clone());
                    renamed
                } else {
                    self.fn_scope_vars.insert(zig_name.clone());
                    zig_name
                };

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
                        if self.closures.closure_vars.contains_key(&fn_name) {
                            // Closure: generate instantiation code
                            // Clone the captured vars to avoid borrow conflict
                            let captured = self.closures.closure_vars.get(&fn_name).cloned();
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
                            self.closures.closure_instances.insert(name.to_string());
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
    fn expr_var_type(&self, expr: &Expression) -> Option<&crate::types::ZigType> {
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
            let n = self.names.next_destructure();
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
            Some(crate::types::ZigType::Struct(fields)) => {
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
            let key_name = match super::class::property_key_name(&prop.key) {
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
        let is_arraylist = matches!(init_type, Some(crate::types::ZigType::ArrayList(_)));
        // Count non-None elements to decide if we need a temp
        let element_count = ap.elements.iter().filter(|e| e.is_some()).count();
        let needs_temp = init_may_have_side_effects(init_expr) || element_count > 1;
        let temp_name = if needs_temp {
            let n = self.names.next_destructure();
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
                crate::codegen::stmt::Codegen::stmt_has_throw_any_alt(&s.body)
            }
            Statement::IfStatement(s) => {
                crate::codegen::stmt::Codegen::stmt_has_throw_any_alt(&s.consequent)
                    || s.alternate
                        .as_ref()
                        .is_some_and(|a| crate::codegen::stmt::Codegen::stmt_has_throw_any_alt(a))
            }
            Statement::WhileStatement(s) => {
                crate::codegen::stmt::Codegen::stmt_has_throw_any_alt(&s.body)
            }
            Statement::DoWhileStatement(s) => {
                crate::codegen::stmt::Codegen::stmt_has_throw_any_alt(&s.body)
            }
            Statement::ForStatement(s) => {
                crate::codegen::stmt::Codegen::stmt_has_throw_any_alt(&s.body)
            }
            Statement::ForOfStatement(s) => {
                crate::codegen::stmt::Codegen::stmt_has_throw_any_alt(&s.body)
            }
            Statement::ForInStatement(s) => {
                crate::codegen::stmt::Codegen::stmt_has_throw_any_alt(&s.body)
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

        // Save outer function's scope vars; this function gets its own fresh set.
        // Collisions with outer scope variable names must be detected so we can
        // rename the parameter (Zig 0.16.0: function parameter must not shadow
        // outer declarations).
        let saved_fn_scope_vars = std::mem::take(&mut self.fn_scope_vars);
        // Push a shadow scope for this function's body — param renames live here.
        self.push_shadow_scope();

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
        let has_captures = !self.closures.current_captured.is_empty();
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
                // #947: Detect parameter shadowing of outer scope variables.
                let base_pname = zig_safe_name(pname);
                let effective_pname = if saved_fn_scope_vars.contains(&base_pname) {
                    self.names.next_shadow(); // consume counter for uniqueness (value not used in `_param` suffix)
                    let renamed = format!("{}_param", pname);
                    if let Some(top_scope) = self.shadow_renames.last_mut() {
                        top_scope.insert(pname.to_string(), renamed.clone());
                    }
                    self.fn_scope_vars.insert(renamed.clone());
                    renamed
                } else {
                    self.fn_scope_vars.insert(base_pname.clone());
                    base_pname
                };
                let zig_pname = if fn_used_names.contains(pname) {
                    effective_pname.as_str()
                } else {
                    self.write("_");
                    effective_pname.as_str()
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
                .map(|r| crate::infer::binding_name(&r.rest.argument))
                && let Some(rname) = rest_name
            {
                if param_idx > 0 || is_async {
                    self.write(", ");
                }
                let base_rname = zig_safe_name(rname);
                let effective_rname = if saved_fn_scope_vars.contains(&base_rname) {
                    self.names.next_shadow();
                    let renamed = format!("{}_param", rname);
                    if let Some(top_scope) = self.shadow_renames.last_mut() {
                        top_scope.insert(rname.to_string(), renamed.clone());
                    }
                    self.fn_scope_vars.insert(renamed.clone());
                    renamed
                } else {
                    self.fn_scope_vars.insert(base_rname.clone());
                    base_rname
                };
                let zig_pname = if fn_used_names.contains(rname) {
                    effective_rname.as_str()
                } else {
                    self.write("_");
                    effective_rname.as_str()
                };
                // Rest parameter: accepts []const JsAny
                self.write(&format!("{}: []const JsAny", zig_pname));
            }
        } else {
            // Fallback: generate params from AST with anytype
            let mut param_idx = 0;
            for param in &fd.params.items {
                if let Some(pname) = crate::infer::binding_name(&param.pattern) {
                    if is_async && pname == "io" {
                        continue;
                    }
                    if param_idx > 0 || is_async {
                        self.write(", ");
                    }
                    // Zig 0.16.0: unused params are compile errors.
                    // #947: Detect parameter shadowing of outer scope variables.
                    let base_pname = zig_safe_name(pname);
                    let effective_pname = if saved_fn_scope_vars.contains(&base_pname) {
                        self.names.next_shadow();
                        let renamed = format!("{}_param", pname);
                        if let Some(top_scope) = self.shadow_renames.last_mut() {
                            top_scope.insert(pname.to_string(), renamed.clone());
                        }
                        self.fn_scope_vars.insert(renamed.clone());
                        renamed
                    } else {
                        self.fn_scope_vars.insert(base_pname.clone());
                        base_pname
                    };
                    let zig_pname = if fn_used_names.contains(pname) {
                        effective_pname.as_str()
                    } else {
                        self.write("_");
                        effective_pname.as_str()
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
                .map(|r| crate::infer::binding_name(&r.rest.argument))
                && let Some(rname) = rest_name
            {
                if param_idx > 0 || is_async {
                    self.write(", ");
                }
                let base_rname = zig_safe_name(rname);
                let effective_rname = if saved_fn_scope_vars.contains(&base_rname) {
                    self.names.next_shadow();
                    let renamed = format!("{}_param", rname);
                    if let Some(top_scope) = self.shadow_renames.last_mut() {
                        top_scope.insert(rname.to_string(), renamed.clone());
                    }
                    self.fn_scope_vars.insert(renamed.clone());
                    renamed
                } else {
                    self.fn_scope_vars.insert(base_rname.clone());
                    base_rname
                };
                let zig_pname = if fn_used_names.contains(rname) {
                    effective_rname.as_str()
                } else {
                    self.write("_");
                    effective_rname.as_str()
                };
                self.write(&format!("{}: []const JsAny", zig_pname));
            }
        }

        // Return type — async + throw functions return error unions
        let base_ret_type = match &self.current_fn_return_type {
            Some(ZigType::I64) => "i64".to_string(),
            Some(ZigType::F64) => "f64".to_string(),
            Some(ZigType::Bool) => "bool".to_string(),
            Some(ZigType::Str) => "[]const u8".to_string(),
            Some(ZigType::Void) => "void".to_string(),
            Some(ZigType::AnytypeReturn) => {
                if let Some(first_ret) = crate::infer::helpers::find_first_return_expr(fd) {
                    let mut captured = self.capture_expr(first_ret);
                    // Strip 'try ' prefixes — try is not valid in @TypeOf (comptime type expression).
                    // We remove all occurrences since the expression may contain nested try:
                    //   try js_string.replace(..., try js_regexp.JsRegExp.init(...), ...)
                    captured = captured.replace("try ", "");
                    format!("@TypeOf({})", captured)
                } else {
                    "void".to_string()
                }
            }
            None => "void".to_string(),
            Some(other) => {
                // Async host functions return structs defined in host.zig,
                // so NamedStruct return types need the "host." prefix.
                if is_async && matches!(other, ZigType::NamedStruct(_)) {
                    format!("host.{}", other.to_zig_type())
                } else {
                    other.to_zig_type()
                }
            }
        };
        let base_ztype = base_ret_type.clone();
        let ret_zig_type = if (is_async || self.fn_has_throw) && base_ret_type != "void" {
            format!("!{}", base_ret_type)
        } else if self.fn_has_throw && base_ret_type == "void" {
            "!void".to_string()
        } else {
            base_ret_type
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
                    .filter_map(|p| crate::infer::binding_name(&p.pattern))
                    .filter(|pn| !fn_used_names.contains(*pn))
                    .map(|pn| pn.to_string())
                    .collect()
            };
        for pname in &unused_params {
            self.write_indent();
            let zig_name = self.zig_safe_name(pname);
            self.write(&format!("_ = _{};\n", zig_name));
        }

        if let Some(body) = &fd.body {
            for stmt in &body.statements {
                self.emit_fn_stmt(stmt);
            }
        }
        // If function has non-void return type but no explicit return,
        // add a default return 0 to avoid Zig compile error.
        if !self.seen_return && base_ztype != "void" {
            self.write_indent();
            self.write("return 0;\n");
        }
        self.indent -= 1;
        self.writeln("}");

        // If this is an export function, add to exported_fns for C ABI wrapper generation.
        if self.current_fn_is_export {
            let func_name = name.to_string();
            let return_type = self.current_fn_return_type.clone().unwrap_or(ZigType::Void);

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
        self.pop_shadow_scope();
        self.fn_scope_vars = saved_fn_scope_vars;
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
                // By default, ALL expression statements need `_ = ` prefix to discard their return
                // value. Zig requires this for non-void expressions (string/float/bool literals,
                // call expressions, subscript, etc.).
                //
                // Special cases (generate complete code including `;` or no `;`):
                //   - ArrayForEach/ArrayFill: full for-loop, no `_ = ` and no `;` needed.
                //   - ArraySome/ArrayEvery: block expression + `;`, need `_ = ` but no extra `;`.
                //   - ArrayPop/ArrayShift: `arr.pop();`, need `_ = ` and `;` (already complete).
                //   - AssignmentExpression/UpdateExpression: already valid Zig statement (void return).
                let mut need_semi = true;
                let mut needs_discard_prefix = true;
                if matches!(
                    &es.expression,
                    Expression::AssignmentExpression(_) | Expression::UpdateExpression(_)
                ) {
                    // Assignment (`a = b`) and update (`i++`) are already valid Zig statements
                    // (they return void). `_ = a = b;` is invalid Zig syntax.
                    needs_discard_prefix = false;
                } else if let Expression::CallExpression(ce) = &es.expression
                    && let Some(builtin) = builtins::detect_builtin_call(ce)
                {
                    match builtin {
                        // These generate complete for-loop code (void) — no `_ = ` and no `;`.
                        builtins::BuiltinCall::ArrayForEach | builtins::BuiltinCall::ArrayFill => {
                            needs_discard_prefix = false;
                            need_semi = false;
                        }
                        // ArraySome/ArrayEvery: block expression returns bool; `_ = ` needed, no `;`.
                        builtins::BuiltinCall::ArraySome | builtins::BuiltinCall::ArrayEvery => {
                            // needs_discard_prefix = true (default), need_semi = false
                        }
                        // ArrayPop/ArrayShift: full expression + `;`; need `_ = ` (default).
                        _ => {}
                    }
                }
                // Non-builtin calls: already handled by default (needs_discard_prefix = true)
                self.write_indent();
                self.in_expr_stmt = true;
                if needs_discard_prefix {
                    self.write("_ = ");
                }
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
                self.emit_while(ws, None);
            }
            Statement::DoWhileStatement(dws) => {
                self.emit_do_while(dws, None);
            }
            Statement::ForStatement(fs) => {
                self.emit_for(fs, None);
            }
            Statement::ForOfStatement(fos) => {
                self.emit_for_of(fos, None);
            }
            Statement::ForInStatement(fis) => {
                self.emit_for_in(fis, None);
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
                        self.emit_while(
                            match &ls.body {
                                Statement::WhileStatement(ws) => ws,
                                _ => unreachable!(),
                            },
                            Some(label_name.as_str()),
                        );
                    }
                    Statement::ForStatement(_) => {
                        self.emit_for(
                            match &ls.body {
                                Statement::ForStatement(fs) => fs,
                                _ => unreachable!(),
                            },
                            Some(label_name.as_str()),
                        );
                    }
                    Statement::ForOfStatement(_) => {
                        self.emit_for_of(
                            match &ls.body {
                                Statement::ForOfStatement(fos) => fos,
                                _ => unreachable!(),
                            },
                            Some(label_name.as_str()),
                        );
                    }
                    Statement::ForInStatement(_) => {
                        self.emit_for_in(
                            match &ls.body {
                                Statement::ForInStatement(fis) => fis,
                                _ => unreachable!(),
                            },
                            Some(label_name.as_str()),
                        );
                    }
                    Statement::DoWhileStatement(_) => {
                        self.emit_do_while(
                            match &ls.body {
                                Statement::DoWhileStatement(dws) => dws,
                                _ => unreachable!(),
                            },
                            Some(label_name.as_str()),
                        );
                    }
                    _ => {
                        // Generic labeled block (for if/switch/block etc).
                        // Skip the label if no break statement targets it (e.g. MDN
                        // documentation labels like "Before: <expr>").
                        if has_break_to_label(&ls.body, ls.label.name.as_str()) {
                            self.write_indent();
                            self.writeln(&format!("{}: {{", label_name));
                            self.indent += 1;
                            self.emit_fn_stmt(&ls.body);
                            self.indent -= 1;
                            self.write_indent();
                            self.writeln("}");
                        } else {
                            self.emit_fn_stmt(&ls.body);
                        }
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
                    self.writeln(&format!(
                        "break :{} @as(anyerror!void, error.JsThrow);",
                        label
                    ));
                } else {
                    // Bare throw: propagate to function return
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

                let label_id = self.names.next_try_label();
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
                self.writeln(&format!(
                    "const {}: anyerror!void = {blk}: {{",
                    result_var,
                    blk = blk_label,
                ));
                self.indent += 1;

                // ── Finally as defer (always runs, inside labeled block) ──
                if let Some(ref finalizer) = ts.finalizer {
                    self.writeln("defer {");
                    self.indent += 1;
                    for stmt in &finalizer.body {
                        self.emit_fn_stmt(stmt);
                    }
                    self.indent -= 1;
                    self.writeln("}");
                }

                // ── Try body as const with explicit anyerror!void type ──
                // Using a standalone const ensures the labeled block has
                // the correct error union type regardless of whether the
                // body throws or not.
                let body_label = format!("_js_try_body_{}", label_id);
                let body_blk_label = format!("_js_try_body_blk_{}", label_id);
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
                    self.writeln(&format!("break :{} {{}};", body_blk_label));
                }

                self.indent -= 1;
                self.writeln("};");

                // ── Catch handler as if-else (in scope of blk_label) ──
                self.writeln(&format!("if ({}) |_| {{", body_label));
                self.indent += 1;
                // Success: no error, fall through
                self.indent -= 1;
                self.writeln("} else |err| {");
                self.indent += 1;

                // Push a shadow scope for the catch handler body.
                // The catch parameter creates a block scope in JS; any variable
                // shadowing within the catch handler must not leak outside.
                self.push_shadow_scope();

                if let Some(ref handler) = ts.handler {
                    // Bind catch parameter
                    if let Some(ref param) = handler.param
                        && let BindingPattern::BindingIdentifier(ref id) = param.pattern
                    {
                        let name = id.name.as_str();
                        let is_referenced = stmt_list_references_name(&handler.body.body, name);
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
                    self.writeln("_ = err;");
                }

                self.pop_shadow_scope();

                self.indent -= 1;
                self.writeln("}");

                // ── Normal completion (no re-throw from catch) ──
                self.writeln(&format!("break :{blk} {{}};", blk = blk_label));

                self.indent -= 1;
                self.writeln("};");

                // ── Propagate unhandled error from re-throw ──
                // If catch body re-threw (break :blk_label error.JsThrow),
                // the const becomes error.JsThrow.
                // When inside a parent try body, break to parent (outer catch
                // intercepts it). Otherwise, return from the function.
                if ts.handler.is_some() {
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
                    let saved_captured = self.closures.take_captured();
                    self.closures.current_captured = captures.clone();

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
                    self.closures.restore_captured(saved_captured);

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
            Statement::EmptyStatement(_) => {
                // Empty statement (`;`) — generates no output
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
    /// Emit a condition expression for use inside `if (...)` / `while (...)`.
    /// In Zig every condition must be `bool`; in JS any truthy value works.
    /// When the expression is not statically known to be bool, we wrap it
    /// with `(!= 0)` truthiness coercion.
    pub(crate) fn emit_condition(&mut self, expr: &oxc_ast::ast::Expression) {
        if self.expr_is_definitely_bool(expr) {
            self.emit_expr(expr);
        } else {
            self.emit_expr_as_bool(expr);
        }
    }

    pub(crate) fn emit_if(&mut self, is: &IfStatement) {
        self.emit_if_impl(is, false);
    }

    /// Inner implementation with `skip_indent` flag.
    /// When `skip_indent` is true, the leading `write_indent()` is omitted —
    /// used for else-if chains where `} else ` was already written on the line.
    fn emit_if_impl(&mut self, is: &IfStatement, skip_indent: bool) {
        if !skip_indent {
            self.write_indent();
        }
        self.write("if (");
        self.emit_condition(&is.test);
        self.write(") {\n");

        self.indent += 1;
        self.emit_stmt_or_block(&is.consequent);
        self.indent -= 1;

        if let Some(alt) = &is.alternate {
            let inner: &Statement = alt;
            match inner {
                Statement::IfStatement(else_if) => {
                    // Write `} else if (...)` on one line. The `}` closes the
                    // consequent block at the current indent level.
                    self.write_indent();
                    self.write("} else ");
                    // Recursive call skips indent since we're mid-line after `} else `.
                    // The recursive call handles ALL closing braces for the chain.
                    self.emit_if_impl(else_if, true);
                    return;
                }
                other => {
                    self.writeln("} else {");
                    self.indent += 1;
                    self.emit_stmt_or_block(other);
                    self.indent -= 1;
                    self.writeln("}");
                    return;
                }
            }
        }
        self.writeln("}");
    }

    /// Emit a statement that forms the body of a control-flow construct
    /// (if/while/for/for-of/for-in/switch). The caller has already written
    /// the opening `{` and will write the closing `}`.
    /// For BlockStatement, emits the body statements directly (with shadow scope)
    /// WITHOUT writing extra braces — avoiding double `{}`.
    fn emit_stmt_or_block(&mut self, stmt: &Statement) {
        match stmt {
            Statement::BlockStatement(bs) => {
                self.push_shadow_scope();
                for s in &bs.body {
                    self.emit_fn_stmt(s);
                }
                self.pop_shadow_scope();
            }
            _ => self.emit_fn_stmt(stmt),
        }
    }

    fn emit_block(&mut self, bs: &BlockStatement) {
        self.writeln("{");
        self.indent += 1;
        self.push_shadow_scope();
        for stmt in &bs.body {
            self.emit_fn_stmt(stmt);
        }
        self.pop_shadow_scope();
        self.indent -= 1;
        self.writeln("}");
    }
}

// ── While / Do-While / For-Of / Switch ───────────

impl Codegen {
    pub(crate) fn emit_while(&mut self, ws: &WhileStatement, label: Option<&str>) {
        self.write_indent();
        if let Some(lbl) = label {
            self.write(&format!("{}: ", lbl));
        }
        self.write("while (");
        self.emit_condition(&ws.test);
        self.write(") {\n");
        self.indent += 1;
        self.emit_stmt_or_block(&ws.body);
        self.indent -= 1;
        self.writeln("}");
    }

    // JS:  do { ... } while (cond);
    // Zig: while (true) { ...; if (cond) {} else { break; } }
    fn emit_do_while(&mut self, dws: &DoWhileStatement, label: Option<&str>) {
        self.write_indent();
        if let Some(lbl) = label {
            self.write(&format!("{}: ", lbl));
        }
        self.write("while (true) {\n");

        self.indent += 1;
        self.emit_stmt_or_block(&dws.body);
        self.write_indent();
        self.write("if (");
        self.emit_condition(&dws.test);
        self.write(") {} else { break; }\n");

        self.indent -= 1;

        self.writeln("}");
    }

    // JS:  for (init; test; update) { ... }
    // Zig: { init; while (test) : (update) { ... } }
    fn emit_for(&mut self, fs: &ForStatement, label: Option<&str>) {
        self.write_indent();
        if let Some(lbl) = label {
            self.write(&format!("{}: ", lbl));
        }
        self.emit_for_body(fs);
    }

    fn emit_for_body(&mut self, fs: &ForStatement) {
        self.write("{\n");
        self.indent += 1;

        // init
        if let Some(init) = &fs.init {
            if let ForStatementInit::VariableDeclaration(vd) = init {
                for decl in &vd.declarations {
                    if let Some(name) = crate::infer::binding_name(&decl.id) {
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
            self.emit_condition(test);
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
    fn emit_for_of(&mut self, fos: &ForOfStatement, label: Option<&str>) {
        // 🔘 for await...of: not supported
        if fos.r#await {
            self.compile_error_stmt(
                GetSpan::span(fos),
                "for await...of is not supported. Use synchronous for...of instead.",
            );
            return;
        }
        // Map / Set → HashMap iterator pattern
        if self.detect_map_set_iter(&fos.right, fos, label) {
            return;
        }

        let var_name = match &fos.left {
            ForStatementLeft::VariableDeclaration(vd) => vd
                .declarations
                .first()
                .and_then(|decl| crate::infer::helpers::binding_name(&decl.id))
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
        if let Some(lbl) = label {
            self.write(&format!("{}: ", lbl));
        }
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
    fn detect_map_set_iter(
        &mut self,
        right: &Expression,
        fos: &ForOfStatement,
        label: Option<&str>,
    ) -> bool {
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
                    .and_then(|decl| crate::infer::helpers::binding_name(&decl.id))
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
        if let Some(lbl) = label {
            self.write(&format!("{}: ", lbl));
        }
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
                .filter_map(|elem| {
                    elem.as_ref()
                        .and_then(|pat| crate::infer::helpers::binding_name(pat))
                })
                .map(|s| s.to_string())
                .collect();
        }
        Vec::new()
    }

    /// JS: for (var key in obj) { ... }
    /// Zig:
    ///   - HashMap: var it = obj.iterator(); while (it.next()) |kv| { const key = kv.key_ptr.*; ... }
    ///   - Static struct: unroll loop — one block per field with const key = "fieldName"
    fn emit_for_in(&mut self, fis: &ForInStatement, label: Option<&str>) {
        let var_name = match &fis.left {
            ForStatementLeft::VariableDeclaration(vd) => vd
                .declarations
                .first()
                .and_then(|decl| crate::infer::helpers::binding_name(&decl.id))
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
                if let Some(lbl) = label {
                    self.write(&format!("{}: ", lbl));
                }
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
            if let Some(lbl) = label {
                self.write(&format!("{}: ", lbl));
            }
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
            for (i, (field_name, _)) in fields.iter().enumerate() {
                self.write_indent();
                if i == 0
                    && let Some(lbl) = label
                {
                    self.write(&format!("{}: ", lbl));
                }
                self.write("{\n");
                self.indent += 1;
                self.write_indent();
                self.write(&format!("const {} = \"{}\";\n", var_name, field_name));
                self.emit_stmt_or_block(&fis.body);
                self.indent -= 1;
                self.write_indent();
                self.write("}\n");
            }
            return;
        }

        // Case 2b: Named struct (@typedef) → resolve fields and unroll
        if let Some(ZigType::NamedStruct(name)) = obj_type
            && let Some(ref jsdoc) = self.jsdoc_data
            && let Some(typedef) = jsdoc.typedefs.get(name)
            && !typedef.fields.is_empty()
        {
            let fields: Vec<_> = typedef.fields.iter().map(|f| f.name.clone()).collect();
            for (i, field_name) in fields.iter().enumerate() {
                self.write_indent();
                if i == 0
                    && let Some(lbl) = label
                {
                    self.write(&format!("{}: ", lbl));
                }
                self.write("{\n");
                self.indent += 1;
                self.write_indent();
                self.write(&format!("const {} = \"{}\";\n", var_name, field_name));
                self.emit_stmt_or_block(&fis.body);
                self.indent -= 1;
                self.write_indent();
                self.write("}\n");
            }
            return;
        }

        // Case 3: Unknown type → compile error
        self.write_indent();
        if let Some(lbl) = label {
            self.write(&format!("{}: ", lbl));
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
                if let Some(init) = &fs.init {
                    match init {
                        ForStatementInit::VariableDeclaration(vd) => {
                            for decl in &vd.declarations {
                                if let Some(init_expr) = &decl.init {
                                    Self::collect_expr_idents(init_expr, names);
                                }
                            }
                        }
                        other => {
                            if let Some(expr) = other.as_expression() {
                                Self::collect_expr_idents(expr, names);
                            }
                        }
                    }
                }
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
            Statement::LabeledStatement(ls) => {
                Self::collect_stmt_idents(&ls.body, names);
            }
            Statement::ForInStatement(fis) => {
                Self::collect_expr_idents(&fis.right, names);
                Self::collect_stmt_idents(&fis.body, names);
            }
            Statement::ThrowStatement(ts) => {
                Self::collect_expr_idents(&ts.argument, names);
            }
            Statement::ContinueStatement(_) | Statement::BreakStatement(_) => {}
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
            Expression::FunctionExpression(func) => {
                // Scan function body for identifiers (captured vars from enclosing scope)
                if let Some(body) = &func.body {
                    for s in &body.statements {
                        Self::collect_stmt_idents(s, names);
                    }
                }
            }
            Expression::ArrowFunctionExpression(arrow) => {
                // Scan arrow body for identifiers (captured vars from enclosing scope)
                for s in &arrow.body.statements {
                    Self::collect_stmt_idents(s, names);
                }
            }
            _ => {}
        }
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
