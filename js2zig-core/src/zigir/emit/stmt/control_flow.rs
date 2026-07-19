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

    /// Recursively check if a block contains `return` or `throw` statements.
    /// Used to detect finally blocks that can't be emitted as `defer { }`
    /// (Zig doesn't allow return/throw inside defer).
    fn block_has_return_or_throw(stmts: &[IrStmt]) -> bool {
        fn ir_stmt_has_return_or_throw(stmt: &IrStmt) -> bool {
            match stmt {
                IrStmt::Return { .. } | IrStmt::Throw { .. } => true,
                IrStmt::If { then, else_, .. } => {
                    Emitter::block_has_return_or_throw(&then.stmts)
                        || else_
                            .as_ref()
                            .is_some_and(|e| Emitter::block_has_return_or_throw(&e.stmts))
                }
                IrStmt::While { body, .. }
                | IrStmt::DoWhile { body, .. }
                | IrStmt::For { body, .. }
                | IrStmt::ForOf { body, .. }
                | IrStmt::ForIn { body, .. } => Emitter::block_has_return_or_throw(&body.stmts),
                IrStmt::Block(b) => Emitter::block_has_return_or_throw(&b.stmts),
                IrStmt::Switch { cases, .. } => cases
                    .iter()
                    .any(|c| Emitter::block_has_return_or_throw(&c.body)),
                IrStmt::Try {
                    try_block,
                    catch_block,
                    finally,
                    ..
                } => {
                    Emitter::block_has_return_or_throw(&try_block.stmts)
                        || Emitter::block_has_return_or_throw(&catch_block.stmts)
                        || finally
                            .as_ref()
                            .is_some_and(|f| Emitter::block_has_return_or_throw(&f.stmts))
                }
                _ => false,
            }
        }
        stmts.iter().any(ir_stmt_has_return_or_throw)
    }

    pub(super) fn emit_if_stmt(&mut self, cond: &IrExpr, then: &IrBlock, else_: &Option<IrBlock>) {
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
        // JS do-while: body always runs at least once, then cond is checked.
        // `continue` must re-evaluate cond (not jump to loop top).
        // Use a first-iteration flag as the while condition so that
        // continue → continue expr (flag=false) → re-check cond.
        //
        // P1-10: Use a unique per-iteration flag name (`__dw_first_N`) instead
        // of a hardcoded `__dw_first`. Zig 0.16 forbids local-variable
        // shadowing across nesting scopes, so an inner do-while nested in
        // the body of an outer do-while would otherwise fail ast-check
        // ("local variable '__dw_first' shadows local variable from outer
        // scope").
        let flag = self.next_do_while_flag();
        self.writeln("{");
        self.indent_push();
        self.writeln(&format!("var {flag}: bool = true;", flag = flag));
        self.write_indent();
        self.emit_label_prefix(label);
        self.write("while (");
        self.write(&format!("{flag} or (", flag = flag));
        self.emit_expr_as_bool(cond);
        self.write(&format!(")) : ({flag} = false) {{\n", flag = flag));
        self.indent_push();
        self.emit_block_stmts_unlabeled(body);
        self.indent_pop();
        self.writeln("}");
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
        // Wrap the entire for loop in a block scope:
        //   { init; label: while (cond) : ({ update; }) { body } }
        // The label MUST be on the `while`, not on the enclosing block —
        // Zig `continue :label` only targets a labeled loop, and a label on
        // a plain block labels that block (which `continue` cannot target).
        self.write_indent();
        self.write("{\n");
        self.indent_push();

        // Emit init statement
        if let Some(init_stmt) = init {
            self.emit_stmt(init_stmt);
        }

        // While loop with optional update continuation
        self.write_indent();
        // Place the label directly before `while` so `continue :label`
        // (and `break :label`) target this loop rather than the wrapping block.
        self.emit_label_prefix(label);
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
                // `var __it = obj.iterator(); label: while (__it.next()) |__kv| { ... }`
                // Label goes directly on while (not a wrapping block) so that
                // `continue :label` targets the loop, not a block.
                self.write_indent();
                let it_name = "__it";
                self.write("var ");
                self.write(it_name);
                self.write(" = ");
                self.emit_expr(iterable);
                self.write(".iterator();\n");
                self.write_indent();
                if let Some(lbl) = label {
                    self.write(&format!("{}: ", lbl));
                }
                self.write("while (");
                self.write(it_name);
                self.write(".next()) |__kv| {\n");
                self.indent_push();
                self.writeln(&format!("const {} = __kv.key_ptr.*;", var.zig_name));
                self.emit_block_stmts_unlabeled(body);
                self.indent_pop();
                self.writeln("}");
            }
            IrForInKind::MapIter => {
                // `var __it = obj.inner.iterator(); label: while (__it.next()) |__kv| { ... }`
                self.write_indent();
                let it_name = "__it";
                self.write("var ");
                self.write(it_name);
                self.write(" = ");
                self.emit_expr(iterable);
                self.write(".inner.iterator();\n");
                self.write_indent();
                if let Some(lbl) = label {
                    self.write(&format!("{}: ", lbl));
                }
                self.write("while (");
                self.write(it_name);
                self.write(".next()) |__kv| {\n");
                self.indent_push();
                self.writeln(&format!("const {} = __kv.key_ptr.*;", var.zig_name));
                self.emit_block_stmts_unlabeled(body);
                // Suppress unused-variable error when key is not referenced in body
                self.writeln(&format!("_ = &{};", var.zig_name));
                self.indent_pop();
                self.writeln("}");
            }
            IrForInKind::StructUnroll { fields } => {
                // Unrolled: one iteration per field. When a label is present
                // we wrap ALL iterations in a single labeled block so that
                // `break :label` exits the entire for-in (previously the label
                // was attached only to the first iteration's block, so a break
                // would skip just that iteration and silently fall through to
                // the remaining ones).
                if let Some(lbl) = label {
                    self.write_indent();
                    self.write(lbl);
                    self.write(": {\n");
                    self.indent_push();
                }
                for field_name in fields.iter() {
                    self.write_indent();
                    self.write("{\n");
                    self.indent_push();
                    self.writeln(&format!("const {} = \"{}\";", var.zig_name, field_name));
                    self.emit_block_stmts_unlabeled(body);
                    self.indent_pop();
                    self.writeln("}");
                }
                if label.is_some() {
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
                // `var __it = obj.inner.iterator(); label: while (__it.next()) |__kv| { ... }`
                // Label goes directly on while (not a wrapping block) so that
                // `continue :label` targets the loop, not a block.
                self.write_indent();
                let it_name = "__it";
                self.write("var ");
                self.write(it_name);
                self.write(" = ");
                self.emit_expr(iterable);
                self.write(".inner.iterator();\n");
                self.write_indent();
                if let Some(lbl) = label {
                    self.write(&format!("{}: ", lbl));
                }
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
                // Suppress unused-variable errors for destructured vars that
                // the body doesn't reference (e.g., `for (const [k, v] of m)`
                // where only v is used). Taking the address marks it as "used".
                if *is_map && !destructure_vars.is_empty() {
                    for dv in destructure_vars {
                        self.writeln(&format!("_ = &{};", dv.zig_name));
                    }
                } else {
                    self.writeln(&format!("_ = &{};", var.zig_name));
                }
                self.indent_pop();
                self.writeln("}");
            }
            IrForOfKind::Str { var_used } => {
                // JS `for (x of str)` iterates over Unicode code points (UTF-8
                // scalar values), not raw bytes. Zig's `for (str)` iterates u8
                // bytes, so we emit a std.unicode.Utf8View while-loop instead.
                self.write_indent();
                self.write("var __iter = std.unicode.Utf8View.initUnchecked(");
                self.emit_expr(iterable);
                self.writeln(").iterator();");
                self.write_indent();
                self.emit_label_prefix(label);
                self.write("while (__iter.nextCodepoint()) ");
                if *var_used {
                    // Bind to a temp var, then cast u21 → i64 for the user-facing name.
                    self.write("|__cp| {\n");
                    self.indent_push();
                    self.writeln(&format!("const {} = @as(i64, __cp);", var.zig_name));
                } else {
                    // Unused capture: use |_| to avoid Zig 0.16 unused-capture error.
                    self.write("|_| {\n");
                    self.indent_push();
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

    pub(super) fn emit_switch_stmt(&mut self, expr: &IrExpr, cases: &[IrSwitchCase]) {
        // Detect a string-keyed switch: Zig's `switch` cannot operate on
        // []const u8 (only integers/enums/bools/union tags). If any case's
        // test lowers to `IrExpr::StringLiteral`, lower the whole switch as
        // an `if`/`else if` chain using `std.mem.eql(u8, ...)` (R6-2).
        let is_string_switch = cases.iter().any(|c| {
            c.test
                .as_ref()
                .is_some_and(|t| matches!(t, crate::zigir::types::IrExpr::StringLiteral(_)))
        });
        if is_string_switch {
            self.emit_string_switch(expr, cases);
            return;
        }

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

    /// Emit a string-keyed switch as an if/else-if chain using std.mem.eql.
    /// Used when any case test is a `StringLiteral` — Zig's `switch` cannot
    /// operate on `[]const u8`.
    fn emit_string_switch(&mut self, expr: &IrExpr, cases: &[IrSwitchCase]) {
        // Bind the discriminant to a temp so each mem.eql call doesn't
        // re-evaluate it (especially if it has side effects).
        // We don't have access to a local scope here at the statement level;
        // emit a labeled-block inline that captures the value. Simpler: emit
        // each test as `std.mem.eql(u8, <expr>, "literal")` and chain with
        // `else if`. The expr is emitted N times — but for string switches
        // the discriminant is virtually always a variable or a cheap literal
        // (the AST lowerer already lowered it once before handing to us).
        let mut written_first = false;
        let mut has_default = false;
        for case in cases {
            match &case.test {
                Some(crate::zigir::types::IrExpr::StringLiteral(_)) => {
                    self.write_indent();
                    if !written_first {
                        self.write("if (");
                        written_first = true;
                    } else {
                        self.write("} else if (");
                    }
                    self.write("std.mem.eql(u8, ");
                    self.emit_expr(expr);
                    self.write(", ");
                    self.emit_expr(case.test.as_ref().unwrap());
                    self.write(")) {\n");
                    // Body must be emitted INSIDE the string-literal arm —
                    // otherwise the default case body leaks into the previous
                    // case's scope (R6-2 follow-up: unreachable-code ast error).
                    self.indent_push();
                    for stmt in &case.body {
                        self.emit_stmt(stmt);
                    }
                    self.indent_pop();
                }
                _ => {
                    // default (test == None) — collect for the trailing else.
                    // Body is emitted later from the `)} else {` block below.
                    has_default = true;
                }
            }
        }
        if written_first {
            self.write_indent();
            if has_default {
                self.write("} else {\n");
                self.indent_push();
                // find default body
                for case in cases {
                    if case.test.is_none() {
                        for stmt in &case.body {
                            self.emit_stmt(stmt);
                        }
                    }
                }
                self.indent_pop();
                self.write_indent();
                self.write("}\n");
            } else {
                self.write("}\n");
            }
        } else if has_default {
            // All cases were default (no string tests) — just emit the default body inline.
            for case in cases {
                if case.test.is_none() {
                    for stmt in &case.body {
                        self.emit_stmt(stmt);
                    }
                }
            }
        }
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
            // Emit finally as defer BEFORE the body so it always runs,
            // even if the body contains return/break/continue.
            if let Some(finally_block) = finally {
                if Self::block_has_return_or_throw(&finally_block.stmts) {
                    self.writeln(
                        "@compileError(\"return/throw in finally block is not yet supported\");",
                    );
                } else {
                    self.writeln("defer {");
                    self.indent_push();
                    self.emit_block_stmts_unlabeled(finally_block);
                    self.indent_pop();
                    self.writeln("}");
                }
            }
            self.emit_block_stmts_unlabeled(try_block);
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
            if Self::block_has_return_or_throw(&finally_block.stmts) {
                self.writeln(
                    "@compileError(\"return/throw in finally block is not yet supported\");",
                );
            } else {
                self.writeln("defer {");
                self.indent_push();
                self.emit_block_stmts_unlabeled(finally_block);
                self.indent_pop();
                self.writeln("}");
            }
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
        // When `needs_catch == false` but `has_throw == true` (try-finally with
        // no catch handler that can throw), propagate the inner body's error
        // result to the outer block. Without this, a thrown error in the body
        // is silently swallowed (R6-1).
        self.write_indent();
        if !needs_catch && has_throw {
            self.write(&format!(
                "break :{} if ({}) |_| @as(anyerror!void, {{}}) else |_| @as(anyerror!void, error.JsThrow);\n",
                blk_label, body_result_var,
            ));
        } else {
            self.write(&format!("break :{} {{}};\n", blk_label));
        }

        self.indent_pop();
        self.writeln("};");

        // ── Propagate unhandled re-throw ──
        // If the result is an error (re-throw from catch, or a throw that
        // escaped the body when there is no catch handler), propagate it.
        // Pre-fix: guarded by `if needs_catch`, which meant a try-finally
        // with NO catch handler silently dropped any throw from the body
        // (body_result_var was error.JsThrow but never inspected).
        // Post-fix: propagate whenever `needs_catch || has_throw`.
        if needs_catch || has_throw {
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
