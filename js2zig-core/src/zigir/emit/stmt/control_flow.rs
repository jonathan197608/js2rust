// zigir/emit/stmt/control_flow.rs
// If, while, do-while, for, for-in, for-of, switch, and try statement emission.

use crate::zigir::emit::Emitter;
use crate::zigir::emit::helpers::EmitterHelpers;
use crate::zigir::ident::IrIdent;
use crate::zigir::types::{IrBlock, IrExpr, IrForInKind, IrForOfKind, IrStmt, IrSwitchCase};

/// Parameters for `emit_for_of_stmt`, bundled to avoid too-many-arguments.
pub(super) struct ForOfInfo<'a> {
    pub var: &'a IrIdent,
    pub destructure_vars: &'a [IrIdent],
    pub iterable: &'a IrExpr,
    pub iterable_is_arraylist: bool,
    pub body: &'a IrBlock,
    pub kind: &'a IrForOfKind,
    pub is_async: bool,
    pub label: &'a Option<String>,
}

/// Parameters for `emit_try_stmt`, bundled to avoid too-many-arguments.
pub(super) struct TryInfo<'a> {
    pub try_block: &'a IrBlock,
    pub catch_var: &'a Option<IrIdent>,
    pub catch_var_referenced: bool,
    pub finally: &'a Option<IrBlock>,
    pub has_throw: bool,
    pub has_nested_try: bool,
    pub catch_block: &'a IrBlock,
}

impl Emitter {
    /// Emit label prefix if present: `lbl: ` (used by while/do-while/for/for-in/for-of).
    fn emit_label_prefix(&mut self, label: &Option<String>) {
        if let Some(lbl) = label {
            self.write(&format!("{}: ", lbl));
        }
    }

    /// Check if a block always exits (return/throw/break/continue).
    fn block_always_exits(block: &IrBlock) -> bool {
        block.stmts.last().is_some_and(|s| {
            matches!(
                s,
                IrStmt::Return { .. }
                    | IrStmt::Throw { .. }
                    | IrStmt::Break { .. }
                    | IrStmt::Continue { .. }
            )
        })
    }

    pub(super) fn emit_if_stmt(
        &mut self,
        cond: &IrExpr,
        then: &IrBlock,
        else_: &Option<IrBlock>,
    ) {
        self.write_indent();
        self.write("if (");
        self.emit_expr_as_bool(cond);
        self.write(") {\n");
        self.indent_push();
        self.emit_block_stmts_unlabeled(then);
        self.indent_pop();
        self.emit_else_chain(else_);
    }

    /// Recursively emit else / else-if chains.
    pub(super) fn emit_else_chain(&mut self, else_: &Option<IrBlock>) {
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
                self.emit_expr_as_bool(inner_cond);
                self.write(") {\n");
                self.indent_push();
                self.emit_block_stmts_unlabeled(inner_then);
                self.indent_pop();
                self.emit_else_chain(inner_else);
                return;
            }
            self.writeln("} else {");
            self.indent_push();
            self.emit_block_stmts_unlabeled(else_block);
            self.indent_pop();
        }
        self.writeln("}");
    }

    pub(super) fn emit_while_stmt(
        &mut self,
        cond: &IrExpr,
        body: &IrBlock,
        label: &Option<String>,
    ) {
        self.write_indent();
        self.emit_label_prefix(label);
        self.write("while (");
        self.emit_expr_as_bool(cond);
        self.write(") {\n");
        self.indent_push();
        self.emit_block_stmts_unlabeled(body);
        self.indent_pop();
        self.writeln("}");
    }

    pub(super) fn emit_do_while_stmt(
        &mut self,
        body: &IrBlock,
        cond: &IrExpr,
        label: &Option<String>,
    ) {
        self.write_indent();
        self.emit_label_prefix(label);
        // Zig doesn't have do-while; use `while (true)` with break at end
        self.write("while (true) {\n");
        self.indent_push();
        self.emit_block_stmts_unlabeled(body);
        self.write_indent();
        self.write("if (");
        self.emit_expr_as_bool(cond);
        self.write(") {} else { break; }\n");
        self.indent_pop();
        self.writeln("}");
    }

    pub(super) fn emit_for_stmt(
        &mut self,
        init: &Option<Box<IrStmt>>,
        cond: &Option<IrExpr>,
        update: &Option<Box<IrStmt>>,
        body: &IrBlock,
        label: &Option<String>,
    ) {
        // Wrap the entire for loop in a block scope: { init; while (cond) : ({ update; }) { body } }
        self.write_indent();
        self.emit_label_prefix(label);
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
            self.emit_expr_as_bool(c);
        } else {
            self.write("true");
        }
        self.write(")");

        // Zig while continuation syntax for the update expression.
        // We emit the update inline inside a block expression: `: ({ update; })`
        // NOTE: must NOT wrap in extra parentheses — `+=` is a statement in Zig,
        // and `(i += 1)` is invalid. Use statement-level emission (no parens).
        if let Some(update_stmt) = update {
            self.write(" : ({ ");
            match update_stmt.as_ref() {
                IrStmt::Expr(expr) => {
                    // For Update expressions (i++, i--), force statement-level
                    // output (no wrapping parentheses).
                    if let IrExpr::Update { op, target, .. } = expr {
                        self.emit_assign_target_inner(target);
                        self.write(&format!(
                            " {}",
                            crate::zigir::emit::helpers::update_op_to_zig(*op)
                        ));
                    } else {
                        self.emit_expr(expr);
                    }
                }
                IrStmt::Assign { target, op, value } => {
                    // Use emit_assign_stmt logic but without indent/newline
                    self.emit_assign_inline(target, *op, value);
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
        self.emit_block_stmts_unlabeled(body);
        self.indent_pop();

        self.write_indent();
        self.write("}\n");

        // Close the enclosing block scope
        self.indent_pop();
        self.writeln("}");
    }

    pub(super) fn emit_for_in_stmt(
        &mut self,
        var: &IrIdent,
        iterable: &IrExpr,
        body: &IrBlock,
        kind: &IrForInKind,
        label: &Option<String>,
    ) {
        match kind {
            IrForInKind::HashMapIter => {
                // `var __it = obj.iterator(); while (__it.next()) |__kv| { const var = __kv.key_ptr.*; ... }`
                self.write_indent();
                self.emit_label_prefix(label);
                self.write("var ");
                let it_name = "__it";
                self.write(it_name);
                self.write(" = ");
                self.emit_expr(iterable);
                self.write(".iterator();\n");
                // The while loop is NOT labeled separately — the label is on
                // the outer `var __it` statement for HashMapIter.
                // The label wraps `{ var __it = ... while ... }` as a single block.
                // We match that by emitting the while at the same level.
                self.write_indent();
                self.write("while (");
                self.write(it_name);
                self.write(".next()) |__kv| {\n");
                self.indent_push();
                self.writeln(&format!("const {} = __kv.key_ptr.*;", var.zig_name));
                self.emit_block_stmts_unlabeled(body);
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
                    self.emit_block_stmts_unlabeled(body);
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

    pub(super) fn emit_for_of_stmt(&mut self, info: &ForOfInfo<'_>) {
        let var = info.var;
        let destructure_vars = info.destructure_vars;
        let iterable = info.iterable;
        let iterable_is_arraylist = info.iterable_is_arraylist;
        let body = info.body;
        let kind = info.kind;
        let is_async = info.is_async;
        let label = info.label;

        if is_async {
            self.writeln("@compileError(\"for-await-of is not supported\");");
            return;
        }

        match kind {
            IrForOfKind::Array => {
                self.write_indent();
                self.emit_label_prefix(label);
                self.write("for (");
                self.emit_expr(iterable);
                if iterable_is_arraylist {
                    self.write(".items");
                }
                self.write(") |");
                self.write(&var.zig_name);
                self.write("| {\n");
                self.indent_push();
                self.emit_block_stmts_unlabeled(body);
                self.indent_pop();
                self.writeln("}");
            }
            IrForOfKind::MapSetIter { is_map } => {
                self.write_indent();
                self.emit_label_prefix(label);
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
                self.emit_block_stmts_unlabeled(body);
                self.indent_pop();
                self.writeln("}");
            }
            IrForOfKind::AsyncUnsupported => {
                self.writeln("@compileError(\"for-await-of is not supported\");");
            }
        }
    }

    pub(super) fn emit_switch_stmt(
        &mut self,
        expr: &IrExpr,
        cases: &[IrSwitchCase],
    ) {
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

    pub(super) fn emit_try_stmt(&mut self, info: &TryInfo<'_>) {
        let try_block = info.try_block;
        let catch_var = info.catch_var;
        let catch_var_referenced = info.catch_var_referenced;
        let finally = info.finally;
        let has_throw = info.has_throw;
        let has_nested_try = info.has_nested_try;
        let catch_block = info.catch_block;

        let needs_catch = catch_var.is_some();

        // ── B1: No throw, no catch, no nested try → inline body + finally ──
        // Also applies when: no throw AND body always exits (catch is unreachable)
        if !has_throw && (!needs_catch || Self::block_always_exits(try_block)) && !has_nested_try {
            self.emit_block_stmts_unlabeled(try_block);
            if let Some(finally_block) = finally {
                self.emit_block_stmts_unlabeled(finally_block);
            }
            return;
        }

        // ── Case A: Has catch (and catch may be reached), throw, or nested try
        // → full labeled block pattern ──
        // Double labeled-block pattern:
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
            self.emit_block_stmts_unlabeled(finally_block);
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

        // Set inside_try_block so that throw/break statements emit
        // break :body_blk_label or break :blk_label
        self.inside_try_block = Some(body_blk_label.clone());

        self.emit_block_stmts_unlabeled(try_block);

        // Normal completion of try body (no throw).
        // Skip if the try body ends with a return/break/continue/throw —
        // execution never reaches this point.
        let body_always_exits = Self::block_always_exits(try_block);
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
            if catch_var_referenced {
                self.writeln("} else |__catch_err| {");
            } else {
                self.writeln("} else |_| {");
            }
            self.indent_push();
            if let Some(var) = catch_var
                && catch_var_referenced
            {
                self.writeln(&format!(
                    "const {} = js_error.JsError.fromError(__catch_err, js_allocator.allocator()) catch @panic(\"OOM: JsError\");",
                    var.zig_name
                ));
            }
            // When !catch_var_referenced: catch capture is |_| (discarded).
            // Set inside_try_block to outer label so re-throw in catch body
            // produces break :blk_label error.JsThrow
            self.inside_try_block = Some(blk_label.clone());
            self.emit_block_stmts_unlabeled(catch_block);
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
