// zigir/lower/stmt.rs
// Statement lowering: control flow (if/for/while/switch/try/labeled), blocks.

use std::collections::HashSet;

use oxc_ast::ast::*;

use crate::types::ZigType;
use crate::zigir::ident::IrIdent;
use crate::zigir::source_span::SourceSpan;
use crate::zigir::types::{IrBlock, IrDecl, IrForInKind, IrForOfKind};

use super::Lowerer;

// ¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T
//  Declaration lowering (remaining stubs)
// ¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T

impl Lowerer {
    /// Lower a statement into an IrStmt.
    ///
    /// This is the main dispatch method for statement-level AST ¡ú IR
    /// transformation. Each branch extracts semantic information and
    /// defers formatting (indentation, `_ = ` discard prefixes, etc.)
    /// to the Emitter phase.
    pub(super) fn lower_stmt(&mut self, stmt: &Statement) -> crate::zigir::types::IrStmt {
        match stmt {
            // ©¤©¤ Variable declarations ©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤
            Statement::VariableDeclaration(vd) => {
                // Multi-declarator: emit as a block of statements
                if vd.declarations.len() == 1 {
                    let decl = &vd.declarations[0];
                    match &decl.id {
                        BindingPattern::ObjectPattern(_) | BindingPattern::ArrayPattern(_) => {
                            self.lower_destructure_decl(decl)
                        }
                        _ => {
                            let ir_decl = self.lower_var_decl(decl, vd.kind.is_const());
                            match ir_decl {
                                IrDecl::Var(v) => crate::zigir::types::IrStmt::VarDecl(v),
                                IrDecl::CompileError { span, msg } => {
                                    crate::zigir::types::IrStmt::CompileError { span, msg }
                                }
                                _ => crate::zigir::types::IrStmt::Comment(
                                    "// unexpected decl type in statement context".to_string(),
                                ),
                            }
                        }
                    }
                } else {
                    let stmts: Vec<crate::zigir::types::IrStmt> = vd
                        .declarations
                        .iter()
                        .flat_map(|decl| match &decl.id {
                            BindingPattern::ObjectPattern(_) | BindingPattern::ArrayPattern(_) => {
                                vec![self.lower_destructure_decl(decl)]
                            }
                            _ => {
                                let ir_decl = self.lower_var_decl(decl, vd.kind.is_const());
                                match ir_decl {
                                    IrDecl::Var(v) => vec![crate::zigir::types::IrStmt::VarDecl(v)],
                                    IrDecl::CompileError { span, msg } => {
                                        vec![crate::zigir::types::IrStmt::CompileError {
                                            span,
                                            msg,
                                        }]
                                    }
                                    _ => vec![],
                                }
                            }
                        })
                        .collect();
                    if stmts.len() == 1 {
                        stmts.into_iter().next().expect("stmts.len()==1 guarantees one element")
                    } else {
                        // Transparent block: emits flat without {} braces so that
                        // `const a = 1, b = 2;` doesn't create a new Zig scope.
                        crate::zigir::types::IrStmt::Block(IrBlock::new_transparent(stmts))
                    }
                }
            }

            // ©¤©¤ Control flow ©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤
            Statement::IfStatement(is) => self.lower_if(is),

            Statement::WhileStatement(ws) => {
                let label = self.current_loop_label();
                crate::zigir::types::IrStmt::While {
                    cond: self.lower_expr(&ws.test),
                    body: self.lower_stmt_as_block(&ws.body, None),
                    label,
                }
            }

            Statement::DoWhileStatement(dws) => {
                let label = self.current_loop_label();
                crate::zigir::types::IrStmt::DoWhile {
                    body: self.lower_stmt_as_block(&dws.body, None),
                    cond: self.lower_expr(&dws.test),
                    label,
                }
            }

            Statement::ForStatement(fs) => self.lower_for(fs),
            Statement::ForOfStatement(fos) => self.lower_for_of(fos),
            Statement::ForInStatement(fis) => self.lower_for_in(fis),

            Statement::SwitchStatement(ss) => self.lower_switch(ss),

            // ©¤©¤ Exception handling ©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤
            Statement::TryStatement(ts) => self.lower_try(ts),
            Statement::ThrowStatement(ts) => crate::zigir::types::IrStmt::Throw {
                value: self.lower_expr(&ts.argument),
                error_name: None,
            },

            // ©¤©¤ Function control ©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤
            Statement::ReturnStatement(rs) => {
                if let Some(fn_ctx) = self.fn_ctx.as_mut() {
                    fn_ctx.seen_return = true;
                }
                let value = rs.argument.as_ref().map(|expr| {
                    let mut val = self.lower_expr(expr);
                    // When the function return type is i64, RemExpr, DivExpr, and PowExpr need
                    // result_type = Some(I64) so the emitter wraps in @intFromFloat.
                    // Also, calls to JsAny-returning functions need .asI64() wrapping.
                    if let Some(ref fn_ctx) = self.fn_ctx
                        && fn_ctx.return_type == Some(ZigType::I64)
                    {
                        val = self.coerce_i64_result_type(val);
                    }
                    // When the function return type is f64, I64 expressions
                    // (IntLiteral, Ident, BuiltinCall, HostCall, Date methods)
                    // need wrapping in @floatFromInt so the return type matches.
                    if let Some(ref fn_ctx) = self.fn_ctx
                        && fn_ctx.return_type == Some(ZigType::F64)
                    {
                        val = self.coerce_f64_result_type(val);
                    }
                    // When the function return type is JsAny (e.g., P1-B7 default
                    // for non-export functions without JSDoc), wrap non-JsAny
                    // expressions in JsAny.from() so the types match.
                    if let Some(ref fn_ctx) = self.fn_ctx
                        && fn_ctx.return_type == Some(ZigType::JsAny)
                    {
                        val = self.coerce_jsany_result_type(val);
                    }
                    val
                });
                crate::zigir::types::IrStmt::Return { value }
            }
            Statement::BreakStatement(bs) => crate::zigir::types::IrStmt::Break {
                label: bs.label.as_ref().map(|l| l.name.to_string()),
            },
            Statement::ContinueStatement(cs) => crate::zigir::types::IrStmt::Continue {
                label: cs.label.as_ref().map(|l| l.name.to_string()),
            },

            // ©¤©¤ Block ©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤
            Statement::BlockStatement(bs) => {
                self.name_mangler.push_shadow_scope();
                let stmts: Vec<crate::zigir::types::IrStmt> =
                    bs.body.iter().map(|s| self.lower_stmt(s)).collect();
                self.name_mangler.pop_shadow_scope();
                crate::zigir::types::IrStmt::Block(IrBlock::new(stmts))
            }
            Statement::LabeledStatement(ls) => self.lower_labeled(ls),

            // ©¤©¤ Expression statement ©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤
            Statement::ExpressionStatement(es) => {
                // R8-C7: In a constructor body, `this.field = value` is
                // rewritten to a `const field = value` binding that the
                // Emitter uses to build the struct return. Because this arm
                // is the shared statement-lowering path, the rewrite reaches
                // this-field assignments nested inside if/loop/switch/try
                // bodies (which lower via lower_stmt_as_block → lower_stmt).
                // `this_rewrite_fields` is None outside constructors and is
                // cleared inside nested functions (enter_fn), so non-
                // constructor and nested-fn `this` are unaffected.
                // Clone the field list to release the immutable borrow on
                // `self` before the mutable `try_rewrite_*` call (the list is
                // small — a handful of class fields per constructor).
                if let Some(fields) = self.this_rewrite_fields.clone()
                    && let Some(rewritten) =
                        self.try_rewrite_this_field_assignment(&es.expression, &fields)
                {
                    return rewritten;
                }
                // Check if this is an assignment to a JS-const variable.
                // In JS, `const x = 1; x = 2` throws TypeError at runtime.
                // We detect this and emit a Throw with error.ConstReassignment
                // instead of performing the assignment.
                if let Some(throw_value) = self.make_const_reassign_throw(&es.expression) {
                    crate::zigir::types::IrStmt::Throw {
                        value: throw_value,
                        error_name: Some("ConstReassignment".to_string()),
                    }
                } else {
                    // Save/restore the old flag value rather than unconditionally
                    // resetting to false. If the lowered expression contains a
                    // nested function/closure, lowering that body may itself
                    // touch in_expr_stmt; restoring the saved value keeps the
                    // outer context consistent instead of clobbering it.
                    let prev_in_expr_stmt = self.in_expr_stmt;
                    self.in_expr_stmt = true;
                    let expr = self.lower_expr(&es.expression);
                    self.in_expr_stmt = prev_in_expr_stmt;
                    crate::zigir::types::IrStmt::Expr(expr)
                }
            }

            // ©¤©¤ Function declaration (nested) ©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤
            Statement::FunctionDeclaration(fd) => {
                self.lower_nested_fn_decl(fd)
            }

            // ©¤©¤ With statement (unsupported) ©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤
            Statement::WithStatement(ws) => {
                let span = self.span_to_source_span(ws.span);
                self.add_error(span, "with statement is not supported");
                crate::zigir::types::IrStmt::CompileError {
                    span: SourceSpan::default(),
                    msg: "with statement is not supported".to_string(),
                }
            }

            // ©¤©¤ Skippable ©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤
            Statement::EmptyStatement(_) => crate::zigir::types::IrStmt::Comment("".to_string()),
            Statement::DebuggerStatement(ds) => {
                crate::zigir::types::IrStmt::CompileError {
                    span: self.span_to_source_span(ds.span),
                    msg: "debugger statement is not supported (debugging is not available in compiled Zig)".to_string(),
                }
            }

            // ©¤©¤ Descriptive unsupported ©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤
            Statement::ClassDeclaration(cd) => {
                let name = cd
                    .id
                    .as_ref()
                    .map(|id| id.name.as_str())
                    .unwrap_or("<anonymous>");
                crate::zigir::types::IrStmt::CompileError {
                    span: self.span_to_source_span(oxc_span::GetSpan::span(stmt)),
                    msg: format!("nested class declaration '{}' is not supported", name),
                }
            }
            Statement::ExportDefaultDeclaration(_) => crate::zigir::types::IrStmt::CompileError {
                span: self.span_to_source_span(oxc_span::GetSpan::span(stmt)),
                msg: "export default is not supported".to_string(),
            },
            Statement::ImportDeclaration(_) => crate::zigir::types::IrStmt::CompileError {
                span: self.span_to_source_span(oxc_span::GetSpan::span(stmt)),
                msg: "import declaration is not supported".to_string(),
            },

            // ©¤©¤ Unsupported ©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤©¤
            _ => {
                let span = oxc_span::GetSpan::span(stmt);
                crate::zigir::types::IrStmt::CompileError {
                    span: self.span_to_source_span(span),
                    msg: "Unsupported statement type".to_string(),
                }
            }
        }
    }

    /// Lower a statement into an IrBlock (used for loop bodies, etc.).
    ///
    /// For `BlockStatement`, emits inner statements directly.
    /// For single statements, wraps in a single-element block.
    pub(super) fn lower_stmt_as_block(
        &mut self,
        stmt: &Statement,
        label: Option<String>,
    ) -> IrBlock {
        let stmts = match stmt {
            Statement::BlockStatement(bs) => {
                self.name_mangler.push_shadow_scope();
                let stmts: Vec<crate::zigir::types::IrStmt> =
                    bs.body.iter().map(|s| self.lower_stmt(s)).collect();
                self.name_mangler.pop_shadow_scope();
                stmts
            }
            _ => vec![self.lower_stmt(stmt)],
        };
        IrBlock {
            stmts,
            label,
            transparent: false,
        }
    }

    /// Lower an if statement (including else-if chains).
    pub(super) fn lower_if(&mut self, is: &IfStatement) -> crate::zigir::types::IrStmt {
        let cond = self.lower_expr(&is.test);
        let then = self.lower_stmt_as_block(&is.consequent, None);
        let else_ = is
            .alternate
            .as_ref()
            .map(|alt| self.lower_stmt_as_block(alt, None));
        crate::zigir::types::IrStmt::If { cond, then, else_ }
    }

    /// Lower a for statement.
    ///
    /// JS `for(init; test; update) { body }` maps to
    /// Zig `{ init; while (test) : (update) { body } }`.
    pub(super) fn lower_for(&mut self, fs: &ForStatement) -> crate::zigir::types::IrStmt {
        let label = self.current_loop_label();

        let init = fs.init.as_ref().map(|init| match init {
            ForStatementInit::VariableDeclaration(vd) => {
                // Lower ALL declarations (handle `for (let i = 0, j = 10; ...)`)
                let is_const = vd.kind.is_const();
                let stmts: Vec<crate::zigir::types::IrStmt> = vd
                    .declarations
                    .iter()
                    .filter_map(|d| {
                        let decl = self.lower_var_decl(d, is_const);
                        match decl {
                            IrDecl::Var(v) => Some(crate::zigir::types::IrStmt::VarDecl(v)),
                            _ => None,
                        }
                    })
                    .collect();
                if stmts.len() == 1 {
                    Box::new(
                        stmts
                            .into_iter()
                            .next()
                            .expect("stmts.len()==1 guarantees one element"),
                    )
                } else {
                    // Transparent block: emits flat without {} braces so that
                    // `for (let i = 0, j = 10; ...)` doesn't create a new Zig scope.
                    // A non-transparent block would hide i/j from the while condition.
                    Box::new(crate::zigir::types::IrStmt::Block(
                        IrBlock::new_transparent(stmts),
                    ))
                }
            }
            _ => {
                // Expression init: lower as expression statement.
                // In a constructor, `this.field = value` in the init
                // position is rewritten to a local assignment.
                if let Some(expr) = init.as_expression() {
                    let ctor_fields = self.this_rewrite_fields.clone();
                    if let Some(fields) = &ctor_fields
                        && let Some(rewritten) =
                            self.try_rewrite_this_field_assignment(expr, fields)
                    {
                        Box::new(rewritten)
                    } else {
                        Box::new(crate::zigir::types::IrStmt::Expr(self.lower_expr(expr)))
                    }
                } else {
                    Box::new(crate::zigir::types::IrStmt::Comment(
                        "// skipped init".to_string(),
                    ))
                }
            }
        });

        let cond = fs.test.as_ref().map(|expr| self.lower_expr(expr));
        let update = fs.update.as_ref().map(|expr| {
            // C9: In a constructor, `this.field++` / `this.field += val`
            // in the update position must be rewritten to local mutations.
            let ctor_fields = self.this_rewrite_fields.clone();
            if let Some(fields) = &ctor_fields
                && let Some(rewritten) = self.try_rewrite_this_field_assignment(expr, fields)
            {
                Box::new(rewritten)
            } else {
                Box::new(crate::zigir::types::IrStmt::Expr(self.lower_expr(expr)))
            }
        });
        let body = self.lower_stmt_as_block(&fs.body, None);

        crate::zigir::types::IrStmt::For {
            init,
            cond,
            update,
            body,
            label,
        }
    }

    /// Lower a for-of statement.
    ///
    /// JS `for (const x of iterable) { ... }`
    /// - Array/ArrayList: Zig `for (iterable) |x| { ... }` or `for (iterable.items) |x| { ... }`
    /// - Map/Set: Zig `var __it = obj.inner.iterator(); while (__it.next()) |__kv| { const x = __kv.key_ptr.*; ... }`
    pub(super) fn lower_for_of(&mut self, fos: &ForOfStatement) -> crate::zigir::types::IrStmt {
        let label = self.current_loop_label();

        // for await...of is not supported
        if fos.r#await {
            let span = self.span_to_source_span(fos.span);
            self.add_error(
                span,
                "for await...of is not supported. Use synchronous for...of instead.",
            );
            return crate::zigir::types::IrStmt::CompileError {
                span: SourceSpan::default(),
                msg: "for await...of is not supported".to_string(),
            };
        }

        // Extract loop variable name(s)
        let (var, destructure_vars) = self.extract_for_of_vars(&fos.left);

        // Determine iteration kind
        let (kind, iterable_is_arraylist) = self.detect_for_of_kind(&fos.right);

        if matches!(kind, IrForOfKind::AsyncUnsupported) {
            return crate::zigir::types::IrStmt::CompileError {
                span: SourceSpan::default(),
                msg: "for await...of is not supported".to_string(),
            };
        }

        // Save old var_types entries for loop-scoped variables to restore after body lowering.
        // Without this, Map/Set (JsAny) and String (I64) types leak to code after the loop.
        let loop_var_keys: Vec<String> = if matches!(kind, IrForOfKind::MapSetIter { .. }) {
            if destructure_vars.is_empty() {
                vec![var.js_name.clone()]
            } else {
                destructure_vars
                    .iter()
                    .map(|dv| dv.js_name.clone())
                    .collect()
            }
        } else if let IrForOfKind::Str { .. } = &kind {
            vec![var.js_name.clone()]
        } else {
            Vec::new()
        };
        let saved_types: Vec<(String, Option<ZigType>)> = loop_var_keys
            .iter()
            .map(|k| (k.clone(), self.type_info.var_types.get(k).cloned()))
            .collect();

        // For Map/Set iteration, destructure variables and the primary variable
        // are JsAny (from __kv.key_ptr.* / __kv.value_ptr.*). Set var_types so
        // the body's binary expressions have correct type info.
        if matches!(kind, IrForOfKind::MapSetIter { .. }) {
            if destructure_vars.is_empty() {
                self.type_info
                    .var_types
                    .insert(var.js_name.clone(), ZigType::JsAny);
            } else {
                for dv in &destructure_vars {
                    self.type_info
                        .var_types
                        .insert(dv.js_name.clone(), ZigType::JsAny);
                }
            }
        }

        // For String iteration, set var type to I64 so binary expressions work.
        // The emit layer casts the u8 capture to i64.
        if let IrForOfKind::Str { .. } = &kind {
            self.type_info
                .var_types
                .insert(var.js_name.clone(), ZigType::I64);
        }

        let iterable = self.lower_expr(&fos.right);
        let body = self.lower_stmt_as_block(&fos.body, None);

        // Restore var_types to pre-loop state (loop variables are not visible outside)
        for (key, old_type) in &saved_types {
            if let Some(t) = old_type {
                self.type_info.var_types.insert(key.clone(), t.clone());
            } else {
                self.type_info.var_types.remove(key);
            }
        }

        // For String for-of, determine if the loop variable is actually used
        // in the body. If not, emit |_| to avoid Zig 0.16 unused-capture error.
        let kind = if let IrForOfKind::Str { .. } = &kind {
            let var_used = match &fos.body {
                Statement::BlockStatement(b) => b
                    .body
                    .iter()
                    .any(|s| Self::ast_stmt_uses_ident(&var.js_name, s)),
                other => Self::ast_stmt_uses_ident(&var.js_name, other),
            };
            IrForOfKind::Str { var_used }
        } else {
            kind
        };

        crate::zigir::types::IrStmt::ForOf {
            var,
            destructure_vars,
            iterable,
            iterable_is_arraylist,
            body,
            kind,
            is_async: fos.r#await,
            label,
        }
    }

    /// Lower a for-in statement.
    ///
    /// JS `for (const key in obj) { ... }`
    /// - HashMap/dynamic: `var __it = obj.iterator(); while (__it.next()) |__kv| { const key = __kv.key_ptr.*; ... }`
    /// - Static struct: unrolled loop ¡ª one block per field with `const key = "fieldName"`
    pub(super) fn lower_for_in(&mut self, fis: &ForInStatement) -> crate::zigir::types::IrStmt {
        let label = self.current_loop_label();

        // Extract loop variable name
        let var = self.extract_for_in_var(&fis.left);

        // Determine iteration kind
        let kind = self.detect_for_in_kind(&fis.right);

        if matches!(kind, IrForInKind::Unsupported) {
            let obj_name = match &fis.right {
                Expression::Identifier(id) => id.name.to_string(),
                _ => "<expression>".to_string(),
            };
            let span = self.span_to_source_span(fis.span);
            self.add_error(
                span,
                format!("for-in: '{}' is not a dynamic object", obj_name),
            );
        }

        // For StructUnroll, the iterable is not used at runtime (fields are
        // hardcoded as string literals), so we use Null to avoid false
        // "parameter used" detection. For HashMapIter/MapIter, we need the actual
        // iterable expression at runtime.
        //
        // However, for unused-param detection, we still need to track that
        // the iterable expression references identifiers (e.g., the param `cfg`
        // in `for (const key in cfg)`), even though it's replaced by Null.

        // Register loop variable type so body expressions have correct type info.
        // - MapIter: key is JsAny (from std.HashMap(JsAny, JsAny, ...))
        // - HashMapIter: key is []const u8 (from JsObjectMap = StringArrayHashMap(JsAny))
        match &kind {
            IrForInKind::MapIter => {
                self.type_info
                    .var_types
                    .insert(var.js_name.clone(), ZigType::JsAny);
            }
            IrForInKind::HashMapIter => {
                self.type_info
                    .var_types
                    .insert(var.js_name.clone(), ZigType::Str);
            }
            _ => {}
        }

        let iterable = if matches!(kind, IrForInKind::StructUnroll { .. }) {
            // Track identifiers from the iterable for unused-param detection
            let mut idents = HashSet::new();
            Self::collect_ast_expr_idents(&fis.right, &mut idents);
            if let Some(ctx) = self.fn_ctx.as_mut() {
                ctx.compile_time_referenced_idents.extend(idents);
            }
            crate::zigir::types::IrExpr::Null
        } else {
            self.lower_expr(&fis.right)
        };
        let body = self.lower_stmt_as_block(&fis.body, None);

        crate::zigir::types::IrStmt::ForIn {
            var,
            iterable,
            body,
            kind,
            label,
        }
    }

    /// Extract variable name from for-of left side.
    /// Returns (primary_var, destructure_vars) where destructure_vars is
    /// non-empty for ArrayPattern destructure like `[key, val]`.
    pub(super) fn extract_for_of_vars(&self, left: &ForStatementLeft) -> (IrIdent, Vec<IrIdent>) {
        match left {
            ForStatementLeft::VariableDeclaration(vd) => {
                if let Some(decl) = vd.declarations.first() {
                    // Check for ArrayPattern destructure: [key, val]
                    if let BindingPattern::ArrayPattern(ap) = &decl.id {
                        let names: Vec<IrIdent> = ap
                            .elements
                            .iter()
                            .filter_map(|elem| {
                                elem.as_ref().and_then(|pat| {
                                    crate::infer::binding_name(pat).map(IrIdent::new)
                                })
                            })
                            .collect();
                        let primary = names
                            .first()
                            .cloned()
                            .unwrap_or_else(|| IrIdent::new("item"));
                        return (primary, names);
                    }
                    // Simple identifier
                    if let Some(name) = crate::infer::binding_name(&decl.id) {
                        return (IrIdent::new(name), vec![]);
                    }
                }
                (IrIdent::new("item"), vec![])
            }
            _ => (IrIdent::new("item"), vec![]),
        }
    }

    /// Extract variable name from for-in left side.
    pub(super) fn extract_for_in_var(&self, left: &ForStatementLeft) -> IrIdent {
        match left {
            ForStatementLeft::VariableDeclaration(vd) => vd
                .declarations
                .first()
                .and_then(|decl| crate::infer::binding_name(&decl.id))
                .map(IrIdent::new)
                .unwrap_or_else(|| IrIdent::new("key")),
            ForStatementLeft::AssignmentTargetIdentifier(id) => IrIdent::new(id.name.as_str()),
            _ => IrIdent::new("key"),
        }
    }

    /// Detect for-of iteration kind based on the right-hand expression type.
    pub(super) fn detect_for_of_kind(&self, right: &Expression) -> (IrForOfKind, bool) {
        match right {
            Expression::Identifier(id) => {
                if let Some(zig_type) = self.type_info.var_types.get(id.name.as_str()) {
                    // Map → iterator pattern
                    if let ZigType::NamedStruct(name) = zig_type {
                        if name == "Map" {
                            return (IrForOfKind::MapSetIter { is_map: true }, false);
                        }
                        if name == "Set" {
                            return (IrForOfKind::MapSetIter { is_map: false }, false);
                        }
                    }
                    // ArrayList → use .items
                    if matches!(zig_type, ZigType::ArrayList(_)) {
                        return (IrForOfKind::Array, true);
                    }
                    // String → byte iteration
                    if matches!(zig_type, ZigType::Str) {
                        // Check if the loop variable is used in the body
                        // (we can't check here since we don't have the body;
                        //  the var_used flag is set in lower_for_of via a re-check)
                        return (IrForOfKind::Str { var_used: true }, false);
                    }
                }
                // Default: array iteration
                (IrForOfKind::Array, false)
            }
            Expression::StringLiteral(_) => (IrForOfKind::Str { var_used: true }, false),
            _ => (IrForOfKind::Array, false),
        }
    }

    /// Detect for-in iteration kind based on the right-hand expression type.
    pub(super) fn detect_for_in_kind(&self, right: &Expression) -> IrForInKind {
        match right {
            Expression::Identifier(id) => {
                if let Some(zig_type) = self.type_info.var_types.get(id.name.as_str()) {
                    // Map (NamedStruct("Map")) → iterator via .inner.iterator()
                    if let ZigType::NamedStruct(name) = zig_type
                        && name == "Map"
                    {
                        return IrForInKind::MapIter;
                    }
                    // HashMap/dynamic object → iterator-based
                    if matches!(zig_type, ZigType::Anytype) {
                        return IrForInKind::HashMapIter;
                    }
                    // Static struct with known fields → unroll
                    if let ZigType::Struct(fields) = zig_type
                        && !fields.is_empty()
                    {
                        return IrForInKind::StructUnroll {
                            fields: fields.iter().map(|(n, _)| n.clone()).collect(),
                        };
                    }
                    // Named struct (e.g., JSDoc @typedef) → resolve to StructUnroll
                    if let ZigType::NamedStruct(name) = zig_type
                        && let Some(typedef) = self.jsdoc_data.typedefs.get(name)
                        && !typedef.fields.is_empty()
                    {
                        let fields: Vec<String> =
                            typedef.fields.iter().map(|f| f.name.clone()).collect();
                        return IrForInKind::StructUnroll { fields };
                    }
                }
                IrForInKind::Unsupported
            }
            _ => IrForInKind::Unsupported,
        }
    }

    /// Lower a switch statement.
    pub(super) fn lower_switch(&mut self, ss: &SwitchStatement) -> crate::zigir::types::IrStmt {
        let expr = self.lower_expr(&ss.discriminant);

        // Pre-compute fall-through ranges: for each case i, determine which
        // cases' consequents must be merged into case i's body.
        // A case falls through to the next if its consequent has no top-level
        // break statement.
        let n = ss.cases.len();
        let mut body_ranges: Vec<(usize, usize)> = Vec::with_capacity(n);
        for i in 0..n {
            let mut end = i + 1;
            while end <= n {
                // Only unlabeled `break;` terminates switch fall-through.
                // Labeled `break outerLabel;` must be preserved for outer loops.
                let has_break = ss.cases[end - 1]
                    .consequent
                    .iter()
                    .any(|s| matches!(s, Statement::BreakStatement(bs) if bs.label.is_none()));
                if has_break {
                    break;
                }
                if end < n {
                    end += 1;
                } else {
                    break;
                }
            }
            body_ranges.push((i, end));
        }

        // Lower each case: merge fall-through bodies, strip break statements.
        let mut cases: Vec<crate::zigir::types::IrSwitchCase> = Vec::with_capacity(n);
        for (i, case) in ss.cases.iter().enumerate() {
            let test = case.test.as_ref().map(|e| self.lower_expr(e));
            let (start, end) = body_ranges[i];
            let mut body: Vec<crate::zigir::types::IrStmt> = Vec::new();
            for j in start..end {
                for s in &ss.cases[j].consequent {
                    // Only strip unlabeled `break;` (switch break).
                    // Labeled `break outerLabel;` must be lowered normally.
                    if matches!(s, Statement::BreakStatement(bs) if bs.label.is_none()) {
                        break;
                    }
                    body.push(self.lower_stmt(s));
                }
            }
            cases.push(crate::zigir::types::IrSwitchCase { test, body });
        }

        crate::zigir::types::IrStmt::Switch { expr, cases }
    }

    /// Lower a try-catch statement.
    pub(super) fn lower_try(&mut self, ts: &TryStatement) -> crate::zigir::types::IrStmt {
        // Lower try block first, then inspect the resulting IR for throws.
        // This catches implicit throws like const-reassignment guards that
        // `stmt_has_throw_any` (AST-level) cannot detect.

        // Record catchable_error state before lowering try body.
        // If it changes from false to true, the try body contains operations
        // that emit `catch return error.JsThrow` (JSON.parse, BigInt div/mod, etc.)
        // — these need the labeled block pattern so the emit layer can use
        // `break :label` instead of `return error.JsThrow`.
        let catchable_before = self
            .fn_ctx
            .as_ref()
            .is_some_and(|ctx| ctx.has_catchable_error);

        let try_block = {
            let stmts = ts.block.body.iter().map(|s| self.lower_stmt(s)).collect();
            IrBlock::new(stmts)
        };

        let catchable_after = self
            .fn_ctx
            .as_ref()
            .is_some_and(|ctx| ctx.has_catchable_error);
        let has_catchable_error_in_try = catchable_after && !catchable_before;

        // Reset has_catchable_error to its pre-try state.
        // Without this reset, the flag is sticky: once any catchable operation
        // (JSON.parse, BigInt div/mod, BigInt **) sets it to true, it stays true
        // for all subsequent try blocks in the same function. Since lower_try
        // detects catchable errors via the DIFFERENCE between catchable_before
        // and catchable_after, a sticky true makes has_catchable_error_in_try =
        // true && !true = false for every try after the first — silently
        // skipping the labeled-block emit pattern and producing invalid Zig
        // (catch return error.JsThrow inside a non-error-returning function).
        // Catch/finally blocks below may set the flag again, which is correct:
        // those errors are outside the try body and need function-level can_throw.
        if let Some(ctx) = self.fn_ctx.as_mut() {
            ctx.has_catchable_error = catchable_before;
        }

        // AST-level throw detection (for nested try exclusion)
        let has_nested_try = ts
            .block
            .body
            .iter()
            .any(|s| matches!(s, Statement::TryStatement(_)));

        // IR-level throw detection: scan lowered try_block for any IrStmt::Throw.
        fn ir_stmt_has_throw(s: &crate::zigir::types::IrStmt) -> bool {
            match s {
                crate::zigir::types::IrStmt::Throw { .. } => true,
                // Recurse into nested blocks (if, while, for, switch, etc.)
                crate::zigir::types::IrStmt::If { then, else_, .. } => {
                    ir_block_has_throw(then) || else_.as_ref().is_some_and(ir_block_has_throw)
                }
                crate::zigir::types::IrStmt::While { body, .. }
                | crate::zigir::types::IrStmt::DoWhile { body, .. }
                | crate::zigir::types::IrStmt::For { body, .. }
                | crate::zigir::types::IrStmt::ForOf { body, .. }
                | crate::zigir::types::IrStmt::ForIn { body, .. } => ir_block_has_throw(body),
                crate::zigir::types::IrStmt::Switch { cases, .. } => {
                    cases.iter().any(|c| c.body.iter().any(ir_stmt_has_throw))
                }
                crate::zigir::types::IrStmt::Block(b) => ir_block_has_throw(b),
                // Recurse into nested try: throws in the try body, catch, or
                // finally all propagate to the outer try's has_throw flag
                // (conservative — even throws caught by the inner catch count,
                //  which only makes the outer emit use the more general path).
                crate::zigir::types::IrStmt::Try {
                    try_block,
                    catch_block,
                    finally,
                    ..
                } => {
                    ir_block_has_throw(try_block)
                        || ir_block_has_throw(catch_block)
                        || finally.as_ref().is_some_and(ir_block_has_throw)
                }
                _ => false,
            }
        }
        fn ir_block_has_throw(block: &IrBlock) -> bool {
            block.stmts.iter().any(ir_stmt_has_throw)
        }
        let has_throw = ir_block_has_throw(&try_block) || has_catchable_error_in_try;

        // Check if a finally block contains break/continue that would escape
        // the defer block (which is how finally is emitted). Break/continue
        // inside nested loops are valid — they target the loop. Only
        // break/continue in If/Block/Switch/Try (not inside a loop) would
        // target a loop outside the defer, producing invalid Zig.
        fn ir_finally_has_escaping_break_or_continue(
            stmts: &[crate::zigir::types::IrStmt],
        ) -> bool {
            fn check(s: &crate::zigir::types::IrStmt) -> bool {
                use crate::zigir::types::IrStmt;
                match s {
                    IrStmt::Break { .. } | IrStmt::Continue { .. } => true,
                    IrStmt::If { then, else_, .. } => {
                        check_block(&then.stmts)
                            || else_.as_ref().is_some_and(|e| check_block(&e.stmts))
                    }
                    IrStmt::Block(b) => check_block(&b.stmts),
                    IrStmt::Switch { cases, .. } => cases.iter().any(|c| c.body.iter().any(check)),
                    IrStmt::Try {
                        try_block,
                        catch_block,
                        finally,
                        ..
                    } => {
                        check_block(&try_block.stmts)
                            || check_block(&catch_block.stmts)
                            || finally.as_ref().is_some_and(|f| check_block(&f.stmts))
                    }
                    // Loops: break/continue inside these target the loop — valid
                    IrStmt::While { .. }
                    | IrStmt::DoWhile { .. }
                    | IrStmt::For { .. }
                    | IrStmt::ForOf { .. }
                    | IrStmt::ForIn { .. } => false,
                    _ => false,
                }
            }
            fn check_block(stmts: &[crate::zigir::types::IrStmt]) -> bool {
                stmts.iter().any(check)
            }
            check_block(stmts)
        }

        let (catch_var, catch_var_referenced, catch_block) = if let Some(handler) = &ts.handler {
            let var = handler
                .param
                .as_ref()
                .and_then(|p| crate::infer::binding_name(&p.pattern))
                .map(|name| self.make_ident(name));
            // Register catch variable type as JsError so member access
            // (e.name, e.message, e.stack) works correctly.
            if let Some(ref v) = var {
                self.type_info
                    .var_types
                    .insert(v.zig_name.clone(), ZigType::JsError);
            }
            let stmts = handler
                .body
                .body
                .iter()
                .map(|s| self.lower_stmt(s))
                .collect();
            // Check if catch variable is referenced in the catch body
            let catch_var_referenced = if let Some(ref cv) = var {
                let js_name = &cv.js_name;
                handler
                    .body
                    .body
                    .iter()
                    .any(|s| Self::stmt_references_name(s, js_name))
            } else {
                false
            };
            (var, catch_var_referenced, IrBlock::new(stmts))
        } else {
            (None, false, IrBlock::new(vec![]))
        };

        let finally = ts.finalizer.as_ref().map(|f| {
            let stmts = f.body.iter().map(|s| self.lower_stmt(s)).collect();
            IrBlock::new(stmts)
        });

        // Detect break/continue in finally that would escape the defer.
        // The emit layer already catches return/throw via block_has_return_or_throw,
        // but break/continue are not checked there — producing invalid Zig that
        // fails at compile time with an unhelpful error. Catch it here instead.
        // Also call add_error so the message surfaces in TranspileResult.errors
        // (not just compile_errors / emitted @compileError).
        if let Some(ref fin) = finally
            && ir_finally_has_escaping_break_or_continue(&fin.stmts)
        {
            let span = self.span_to_source_span(ts.span);
            let msg = "break/continue in finally block is not supported";
            self.add_error(span.clone(), msg);
            return crate::zigir::types::IrStmt::CompileError {
                span,
                msg: msg.to_string(),
            };
        }

        crate::zigir::types::IrStmt::Try {
            try_block,
            catch_var,
            catch_var_referenced,
            catch_block,
            finally,
            has_throw,
            has_nested_try,
        }
    }

    /// Check if a statement references a given identifier name.
    /// Used to detect whether a catch variable is actually used in the catch body.
    pub(super) fn stmt_references_name(stmt: &Statement, name: &str) -> bool {
        match stmt {
            Statement::ExpressionStatement(es) => Self::expr_references_name(&es.expression, name),
            Statement::ReturnStatement(rs) => rs
                .argument
                .as_ref()
                .is_some_and(|a| Self::expr_references_name(a, name)),
            Statement::VariableDeclaration(vd) => vd.declarations.iter().any(|d| {
                d.init
                    .as_ref()
                    .is_some_and(|init| Self::expr_references_name(init, name))
            }),
            Statement::BlockStatement(bs) => {
                bs.body.iter().any(|s| Self::stmt_references_name(s, name))
            }
            Statement::ThrowStatement(ts) => Self::expr_references_name(&ts.argument, name),
            Statement::IfStatement(ifs) => {
                Self::stmt_references_name(&ifs.consequent, name)
                    || ifs
                        .alternate
                        .as_ref()
                        .is_some_and(|a| Self::stmt_references_name(a, name))
            }
            Statement::WhileStatement(ws) => {
                Self::expr_references_name(&ws.test, name)
                    || Self::stmt_references_name(&ws.body, name)
            }
            Statement::DoWhileStatement(dws) => {
                Self::stmt_references_name(&dws.body, name)
                    || Self::expr_references_name(&dws.test, name)
            }
            Statement::ForStatement(fs) => {
                Self::stmt_references_name(&fs.body, name)
                    || fs
                        .test
                        .as_ref()
                        .is_some_and(|e| Self::expr_references_name(e, name))
                    || fs
                        .update
                        .as_ref()
                        .is_some_and(|e| Self::expr_references_name(e, name))
            }
            Statement::ForOfStatement(fos) => {
                Self::expr_references_name(&fos.right, name)
                    || Self::stmt_references_name(&fos.body, name)
            }
            Statement::ForInStatement(fis) => {
                Self::expr_references_name(&fis.right, name)
                    || Self::stmt_references_name(&fis.body, name)
            }
            Statement::LabeledStatement(ls) => Self::stmt_references_name(&ls.body, name),
            Statement::SwitchStatement(ss) => ss.cases.iter().any(|c| {
                c.consequent
                    .iter()
                    .any(|s| Self::stmt_references_name(s, name))
            }),
            Statement::TryStatement(ts) => {
                ts.block
                    .body
                    .iter()
                    .any(|s| Self::stmt_references_name(s, name))
                    || ts.handler.as_ref().is_some_and(|h| {
                        h.body
                            .body
                            .iter()
                            .any(|s| Self::stmt_references_name(s, name))
                    })
                    || ts
                        .finalizer
                        .as_ref()
                        .is_some_and(|f| f.body.iter().any(|s| Self::stmt_references_name(s, name)))
            }
            _ => false,
        }
    }

    pub(super) fn expr_references_name(expr: &Expression, name: &str) -> bool {
        match expr {
            Expression::Identifier(id) => id.name.as_str() == name,
            Expression::BinaryExpression(be) => {
                Self::expr_references_name(&be.left, name)
                    || Self::expr_references_name(&be.right, name)
            }
            Expression::LogicalExpression(le) => {
                Self::expr_references_name(&le.left, name)
                    || Self::expr_references_name(&le.right, name)
            }
            Expression::UnaryExpression(ue) => Self::expr_references_name(&ue.argument, name),
            Expression::UpdateExpression(ue) => {
                Self::simple_assign_target_references_name(&ue.argument, name)
            }
            Expression::AssignmentExpression(ae) => {
                Self::assign_target_references_name(&ae.left, name)
                    || Self::expr_references_name(&ae.right, name)
            }
            Expression::ConditionalExpression(ce) => {
                Self::expr_references_name(&ce.test, name)
                    || Self::expr_references_name(&ce.consequent, name)
                    || Self::expr_references_name(&ce.alternate, name)
            }
            Expression::CallExpression(ce) => {
                Self::expr_references_name(&ce.callee, name)
                    || ce
                        .arguments
                        .iter()
                        .any(|a| Self::arg_references_name(a, name))
            }
            Expression::NewExpression(ne) => {
                Self::expr_references_name(&ne.callee, name)
                    || ne
                        .arguments
                        .iter()
                        .any(|a| Self::arg_references_name(a, name))
            }
            Expression::StaticMemberExpression(sme) => {
                Self::expr_references_name(&sme.object, name)
            }
            Expression::ComputedMemberExpression(cme) => {
                Self::expr_references_name(&cme.object, name)
                    || Self::expr_references_name(&cme.expression, name)
            }
            Expression::ArrayExpression(ae) => ae.elements.iter().any(|e| match e {
                ArrayExpressionElement::SpreadElement(se) => {
                    Self::expr_references_name(&se.argument, name)
                }
                other => other
                    .as_expression()
                    .is_some_and(|e| Self::expr_references_name(e, name)),
            }),
            Expression::ObjectExpression(oe) => oe.properties.iter().any(|p| match p {
                ObjectPropertyKind::ObjectProperty(op) => {
                    Self::expr_references_name(&op.value, name)
                }
                ObjectPropertyKind::SpreadProperty(sp) => {
                    Self::expr_references_name(&sp.argument, name)
                }
            }),
            Expression::SequenceExpression(se) => se
                .expressions
                .iter()
                .any(|e| Self::expr_references_name(e, name)),
            Expression::TemplateLiteral(tl) => tl
                .expressions
                .iter()
                .any(|e| Self::expr_references_name(e, name)),
            Expression::ChainExpression(ce) => {
                Self::chain_element_references_name(&ce.expression, name)
            }
            Expression::AwaitExpression(ae) => Self::expr_references_name(&ae.argument, name),
            Expression::TaggedTemplateExpression(tte) => {
                Self::expr_references_name(&tte.tag, name)
                    || tte
                        .quasi
                        .expressions
                        .iter()
                        .any(|e| Self::expr_references_name(e, name))
            }
            _ => false,
        }
    }

    /// Recursively check whether an assignment target references the given name.
    fn assign_target_references_name(target: &AssignmentTarget, name: &str) -> bool {
        match target {
            AssignmentTarget::AssignmentTargetIdentifier(id) => id.name.as_str() == name,
            // Destructuring / member targets: identifiers inside the
            // destructure pattern still count as references to `name`.
            AssignmentTarget::ArrayAssignmentTarget(_)
            | AssignmentTarget::ObjectAssignmentTarget(_) => {
                // Conservative: assume destructuring patterns may reference
                // the name rather than missing it. Catch-variable usage is
                // intentionally over-reported here to avoid silently
                // dropping the variable binding.
                true
            }
            _ => false,
        }
    }

    /// Recursively check whether a simple assignment target references the given name.
    fn simple_assign_target_references_name(target: &SimpleAssignmentTarget, name: &str) -> bool {
        match target {
            SimpleAssignmentTarget::AssignmentTargetIdentifier(id) => id.name.as_str() == name,
            _ => false,
        }
    }

    /// Check whether an optional chain element references the given name.
    fn chain_element_references_name(elem: &ChainElement, name: &str) -> bool {
        match elem {
            ChainElement::StaticMemberExpression(sme) => {
                Self::expr_references_name(&sme.object, name)
            }
            ChainElement::ComputedMemberExpression(cme) => {
                Self::expr_references_name(&cme.object, name)
                    || Self::expr_references_name(&cme.expression, name)
            }
            ChainElement::CallExpression(ce) => {
                Self::expr_references_name(&ce.callee, name)
                    || ce
                        .arguments
                        .iter()
                        .any(|a| Self::arg_references_name(a, name))
            }
            _ => false,
        }
    }

    /// Check if a call argument references a given identifier name.
    fn arg_references_name(arg: &Argument, name: &str) -> bool {
        match arg {
            Argument::SpreadElement(se) => Self::expr_references_name(&se.argument, name),
            // All other variants are inherited from Expression — use as_expression()
            other => {
                if let Some(expr) = other.as_expression() {
                    Self::expr_references_name(expr, name)
                } else {
                    false
                }
            }
        }
    }

    /// Lower a labeled statement.
    ///
    /// For loops, the label is attached to the loop body.
    /// For blocks, the label is attached only if a `break :label` exists
    /// inside the body.
    pub(super) fn lower_labeled(&mut self, ls: &LabeledStatement) -> crate::zigir::types::IrStmt {
        let label_str = ls.label.name.to_string();

        match &ls.body {
            // Loops: label attaches to the loop (handled by lower_stmt_for_loop)
            Statement::WhileStatement(_)
            | Statement::DoWhileStatement(_)
            | Statement::ForStatement(_)
            | Statement::ForOfStatement(_)
            | Statement::ForInStatement(_) => {
                // Set pending label for the loop to pick up
                self.pending_label = Some(label_str);
                self.lower_stmt(&ls.body)
            }
            // Other statements: only add label if body contains `break :label`
            _ => {
                let ir_stmt = self.lower_stmt(&ls.body);
                // Check if body contains break to this label
                let has_break = Self::stmt_has_break_to_label(&ls.body, &label_str);
                if has_break {
                    // Wrap in a labeled block
                    crate::zigir::types::IrStmt::Block(IrBlock::with_label(
                        if let crate::zigir::types::IrStmt::Block(b) = ir_stmt {
                            b.stmts
                        } else {
                            vec![ir_stmt]
                        },
                        label_str,
                    ))
                } else {
                    ir_stmt
                }
            }
        }
    }

    /// Pre-scan: check if a statement tree contains `break :label_name`.
    pub(super) fn stmt_has_break_to_label(stmt: &Statement, label_name: &str) -> bool {
        match stmt {
            Statement::BreakStatement(bs) => bs
                .label
                .as_ref()
                .is_some_and(|l| l.name.as_str() == label_name),
            Statement::BlockStatement(bs) => bs
                .body
                .iter()
                .any(|s| Self::stmt_has_break_to_label(s, label_name)),
            Statement::IfStatement(is) => {
                Self::stmt_has_break_to_label(&is.consequent, label_name)
                    || is
                        .alternate
                        .as_ref()
                        .is_some_and(|a| Self::stmt_has_break_to_label(a, label_name))
            }
            Statement::LabeledStatement(ls) => Self::stmt_has_break_to_label(&ls.body, label_name),
            Statement::TryStatement(ts) => {
                ts.block
                    .body
                    .iter()
                    .any(|s| Self::stmt_has_break_to_label(s, label_name))
                    || ts.handler.as_ref().is_some_and(|h| {
                        h.body
                            .body
                            .iter()
                            .any(|s| Self::stmt_has_break_to_label(s, label_name))
                    })
                    || ts.finalizer.as_ref().is_some_and(|f| {
                        f.body
                            .iter()
                            .any(|s| Self::stmt_has_break_to_label(s, label_name))
                    })
            }
            Statement::SwitchStatement(ss) => ss.cases.iter().any(|c| {
                c.consequent
                    .iter()
                    .any(|s| Self::stmt_has_break_to_label(s, label_name))
            }),
            Statement::ForStatement(fs) => Self::stmt_has_break_to_label(&fs.body, label_name),
            Statement::ForOfStatement(fos) => Self::stmt_has_break_to_label(&fos.body, label_name),
            Statement::ForInStatement(fis) => Self::stmt_has_break_to_label(&fis.body, label_name),
            Statement::WhileStatement(ws) => Self::stmt_has_break_to_label(&ws.body, label_name),
            Statement::DoWhileStatement(dws) => {
                Self::stmt_has_break_to_label(&dws.body, label_name)
            }
            _ => false,
        }
    }

    /// If the expression is an assignment to a JS-const variable, return Some(value)
    /// where `value` is the lowered right-hand side of the assignment. This ensures
    /// that variables referenced in the RHS (e.g., `y` in `x %= y`) are still
    /// considered "used" by the Zig compiler, avoiding "unused local constant" errors.
    /// Returns None if the expression is not a const-reassignment.
    fn make_const_reassign_throw(
        &mut self,
        expr: &Expression,
    ) -> Option<crate::zigir::types::IrExpr> {
        if let Expression::AssignmentExpression(ae) = expr {
            // All assignment operators (simple `=` and compound `+=`, `-=`, etc.)
            if let AssignmentTarget::AssignmentTargetIdentifier(id) = &ae.left {
                // Check if the identifier is in js_const_reassigned
                let is_const_reassign = self
                    .fn_ctx
                    .as_ref()
                    .is_some_and(|ctx| ctx.js_const_reassigned.contains(id.name.as_str()));
                if is_const_reassign {
                    // Lower the RHS only (not the full assignment, since Zig
                    // assignment is a statement, not an expression).
                    // This preserves references to variables in the RHS (e.g., `y`
                    // in `x %= y`) so they are not marked as unused.
                    return Some(self.lower_expr(&ae.right));
                }
            }
        }
        None
    }

    /// Get and consume the pending loop label (set by lower_labeled).
    pub(super) fn current_loop_label(&mut self) -> Option<String> {
        self.pending_label.take()
    }

    /// Lower a block of statements.
    pub(super) fn lower_block(&mut self, stmts: &[Statement]) -> IrBlock {
        let ir_stmts: Vec<crate::zigir::types::IrStmt> =
            stmts.iter().map(|s| self.lower_stmt(s)).collect();
        IrBlock::new(ir_stmts)
    }

    /// When an i64-returning function contains a `return expr` where `expr`
    /// is a RemExpr, DivExpr, PowExpr, or a BuiltinCall returning f64, ensure
    /// the emitter wraps the f64 result in `@as(i64, @intFromFloat(...))`.
    ///
    /// For RemExpr/DivExpr/PowExpr: set `result_type` to `Some(I64)`.
    /// For BuiltinCall with F64 return type (e.g., Math.sqrt, parseFloat):
    /// wrap in a DivExpr with divisor 1 and `result_type: Some(I64)`.
    /// `x / 1.0` is the IEEE 754 identity for all f64 values (including NaN,
    /// Infinity, -0), so Zig optimizes the division away at comptime.
    ///
    /// Also recurses into Conditional and Sequence expressions to find
    /// nested expressions in value-producing positions
    /// (e.g., `return cond ? a % b : 0`).
    fn coerce_i64_result_type(
        &self,
        expr: crate::zigir::types::IrExpr,
    ) -> crate::zigir::types::IrExpr {
        use crate::zigir::types::IrExpr;
        match expr {
            // Direct matches: coerce result_type to I64
            IrExpr::RemExpr {
                left,
                right,
                left_type,
                right_type,
                result_type: None,
            } => IrExpr::RemExpr {
                left,
                right,
                left_type,
                right_type,
                result_type: Some(ZigType::I64),
            },
            IrExpr::DivExpr {
                left,
                right,
                left_type,
                right_type,
                result_type: None,
            } => IrExpr::DivExpr {
                left,
                right,
                left_type,
                right_type,
                result_type: Some(ZigType::I64),
            },
            IrExpr::PowExpr {
                base,
                exp,
                base_type,
                exp_type,
                result_type: None,
            } => IrExpr::PowExpr {
                base,
                exp,
                base_type,
                exp_type,
                result_type: Some(ZigType::I64),
            },
            // BuiltinCall returning JsAny (e.g., arr.pop(), arr.shift()):
            // wrap in .asI64() to extract the i64 value.
            IrExpr::BuiltinCall(bc) if bc.return_type == ZigType::JsAny => {
                Self::wrap_jsany_to_i64(IrExpr::BuiltinCall(bc))
            }
            // BuiltinCall returning F64 (e.g., Math.sqrt, parseFloat):
            // wrap in DivExpr with divisor 1 and result_type I64 to trigger
            // @as(i64, @intFromFloat(...)) in the emitter.
            // `x / 1.0` is the identity for all IEEE 754 f64 values.
            IrExpr::BuiltinCall(bc) if bc.return_type == ZigType::F64 => IrExpr::DivExpr {
                left: Box::new(IrExpr::BuiltinCall(bc)),
                right: Box::new(IrExpr::IntLiteral(1)),
                left_type: ZigType::F64,
                right_type: ZigType::I64,
                result_type: Some(ZigType::I64),
            },
            // HostCall returning F64: wrap in DivExpr, same as BuiltinCall above.
            IrExpr::HostCall(hc) if hc.return_type == ZigType::F64 => IrExpr::DivExpr {
                left: Box::new(IrExpr::HostCall(hc)),
                right: Box::new(IrExpr::IntLiteral(1)),
                left_type: ZigType::F64,
                right_type: ZigType::I64,
                result_type: Some(ZigType::I64),
            },
            // HostCall returning JsAny: wrap in .asI64() to extract the i64
            // value from the JsAny union, same as Call with JsAny return.
            IrExpr::HostCall(hc) if hc.return_type == ZigType::JsAny => {
                Self::wrap_jsany_to_i64(IrExpr::HostCall(hc))
            }
            // Call to a JsAny-returning function: wrap in .asI64() to
            // extract the i64 value from the JsAny union.
            IrExpr::Call(call) => {
                let returns_jsany = match &*call.callee {
                    IrExpr::Ident(ident) | IrExpr::TypedIdent { ident, .. } => {
                        self.type_info.fn_return_types.get(&ident.js_name) == Some(&ZigType::JsAny)
                    }
                    _ => false,
                };
                if returns_jsany {
                    Self::wrap_jsany_to_i64(IrExpr::Call(call))
                } else {
                    IrExpr::Call(call)
                }
            }
            // Conditional: recurse into both branches (value-producing).
            IrExpr::Conditional { cond, then, else_ } => IrExpr::Conditional {
                cond,
                then: Box::new(self.coerce_i64_result_type(*then)),
                else_: Box::new(self.coerce_i64_result_type(*else_)),
            },
            // Sequence: only the last element is the return value
            IrExpr::Sequence(mut exprs) => {
                if let Some(last) = exprs.pop() {
                    exprs.push(self.coerce_i64_result_type(last));
                }
                IrExpr::Sequence(exprs)
            }
            // Binary with f64 result → wrap in @intFromFloat
            IrExpr::Binary {
                ref left_type,
                ref right_type,
                ..
            } => {
                let is_f64 = left_type.as_ref() == Some(&ZigType::F64)
                    || right_type.as_ref() == Some(&ZigType::F64);
                if is_f64 {
                    Self::wrap_f64_to_i64(expr)
                } else {
                    expr
                }
            }
            // Unary with f64 operand → wrap
            IrExpr::Unary {
                ref operand_type, ..
            } => {
                if operand_type.as_ref() == Some(&ZigType::F64) {
                    Self::wrap_f64_to_i64(expr)
                } else {
                    expr
                }
            }
            // Logical with f64 same-type → wrap
            IrExpr::Logical {
                ref left_type,
                ref right_type,
                ..
            } => {
                let same_type =
                    left_type.is_some() && right_type.is_some() && left_type == right_type;
                if same_type && left_type.as_ref() == Some(&ZigType::F64) {
                    Self::wrap_f64_to_i64(expr)
                } else {
                    expr
                }
            }
            // Already-coerced or other expressions: pass through unchanged
            // TypedIdent with F64 → wrap to i64
            IrExpr::TypedIdent {
                ident,
                ty: ZigType::F64,
            } => Self::wrap_f64_to_i64(IrExpr::TypedIdent {
                ident,
                ty: ZigType::F64,
            }),
            // TypedIdent with JsAny → wrap via .asI64()
            IrExpr::TypedIdent {
                ident,
                ty: ZigType::JsAny,
            } => Self::wrap_jsany_to_i64(IrExpr::TypedIdent {
                ident,
                ty: ZigType::JsAny,
            }),
            other => other,
        }
    }

    /// Coerce an I64 expression to f64 when the function's return type is f64.
    /// This is the inverse of `coerce_i64_result_type`: it wraps I64-producing
    /// expressions in a DivExpr with divisor 1, which the emitter renders as
    /// `(@as(f64, @floatFromInt(expr)) / @as(f64, @floatFromInt(1)))`.
    /// Zig optimizes `/ 1.0` at comptime, leaving just the `@floatFromInt` cast.
    fn coerce_f64_result_type(
        &self,
        expr: crate::zigir::types::IrExpr,
    ) -> crate::zigir::types::IrExpr {
        use crate::zigir::kinds::{CallKind, MethodObjectKind};
        use crate::zigir::types::IrExpr;

        // I64-returning Date methods (from infer layer)
        const I64_DATE_METHODS: &[&str] = &[
            "getTime",
            "getFullYear",
            "getMonth",
            "getDate",
            "getDay",
            "getHours",
            "getMinutes",
            "getSeconds",
            "getMilliseconds",
            "getTimezoneOffset",
            "valueOf",
            "getUTCFullYear",
            "getUTCMonth",
            "getUTCDate",
            "getUTCDay",
            "getUTCHours",
            "getUTCMinutes",
            "getUTCSeconds",
            "getUTCMilliseconds",
            "setFullYear",
            "setMonth",
            "setDate",
            "setHours",
            "setMinutes",
            "setSeconds",
            "setMilliseconds",
            "setUTCFullYear",
            "setUTCMonth",
            "setUTCDate",
            "setUTCHours",
            "setUTCMinutes",
            "setUTCSeconds",
            "setUTCMilliseconds",
            "setTime",
        ];

        match expr {
            // Already F64 — pass through
            IrExpr::FloatLiteral(_) => expr,
            IrExpr::DivExpr { .. } | IrExpr::RemExpr { .. } | IrExpr::PowExpr { .. } => expr,
            IrExpr::BuiltinCall(bc) if bc.return_type == ZigType::F64 => IrExpr::BuiltinCall(bc),
            IrExpr::HostCall(hc) if hc.return_type == ZigType::F64 => IrExpr::HostCall(hc),

            // I64 BuiltinCall — wrap
            IrExpr::BuiltinCall(bc) if bc.return_type == ZigType::I64 => {
                Self::wrap_i64_to_f64(IrExpr::BuiltinCall(bc))
            }
            // I64 HostCall — wrap
            IrExpr::HostCall(hc) if hc.return_type == ZigType::I64 => {
                Self::wrap_i64_to_f64(IrExpr::HostCall(hc))
            }

            // Date method call returning I64 — wrap
            IrExpr::Call(call) => {
                let is_date_method = matches!(
                    call.call_kind,
                    CallKind::Method {
                        object_type: MethodObjectKind::Date
                    }
                );
                let method_name = match &*call.callee {
                    IrExpr::FieldAccess { field, .. } => Some(field.as_str()),
                    _ => None,
                };
                let needs_wrap = is_date_method
                    && method_name.is_some_and(|name| I64_DATE_METHODS.contains(&name));

                if needs_wrap {
                    Self::wrap_i64_to_f64(IrExpr::Call(call))
                } else {
                    IrExpr::Call(call)
                }
            }

            // Ident — check variable type in type_info
            IrExpr::Ident(ident) => {
                let needs_wrap =
                    self.type_info.var_types.get(&ident.js_name) == Some(&ZigType::I64);
                if needs_wrap {
                    Self::wrap_i64_to_f64(IrExpr::Ident(ident))
                } else {
                    IrExpr::Ident(ident)
                }
            }

            // TypedIdent — use embedded ty field directly (no lookup needed)
            IrExpr::TypedIdent { ident, ty } => {
                if ty == ZigType::I64 {
                    Self::wrap_i64_to_f64(IrExpr::TypedIdent { ident, ty })
                } else {
                    IrExpr::TypedIdent { ident, ty }
                }
            }

            // IntLiteral — always I64, wrap
            IrExpr::IntLiteral(_) => Self::wrap_i64_to_f64(expr),

            // Conditional — recurse into both branches
            IrExpr::Conditional { cond, then, else_ } => IrExpr::Conditional {
                cond,
                then: Box::new(self.coerce_f64_result_type(*then)),
                else_: Box::new(self.coerce_f64_result_type(*else_)),
            },

            // Sequence — only the last element is the return value
            IrExpr::Sequence(mut exprs) => {
                if let Some(last) = exprs.pop() {
                    exprs.push(self.coerce_f64_result_type(last));
                }
                IrExpr::Sequence(exprs)
            }

            // Binary: wrap only if definitely I64 (known numeric type, not F64, not Anytype).
            // Unknown/Anytype types → pass through (Zig handles coercion).
            IrExpr::Binary {
                ref left_type,
                ref right_type,
                ..
            } => {
                let is_f64 = left_type.as_ref() == Some(&ZigType::F64)
                    || right_type.as_ref() == Some(&ZigType::F64);
                let is_known_i64 = left_type.as_ref() == Some(&ZigType::I64)
                    || right_type.as_ref() == Some(&ZigType::I64);
                if is_f64 || !is_known_i64 {
                    expr
                } else {
                    Self::wrap_i64_to_f64(expr)
                }
            }
            // Unary: wrap only if operand type is known I64
            IrExpr::Unary {
                ref operand_type, ..
            } => {
                if operand_type.as_ref() == Some(&ZigType::I64) {
                    Self::wrap_i64_to_f64(expr)
                } else {
                    expr
                }
            }
            // Logical: wrap only if known I64 same-type
            IrExpr::Logical {
                ref left_type,
                ref right_type,
                ..
            } => {
                let same_type =
                    left_type.is_some() && right_type.is_some() && left_type == right_type;
                if same_type && left_type.as_ref() == Some(&ZigType::I64) {
                    Self::wrap_i64_to_f64(expr)
                } else {
                    expr
                }
            }

            // Other expressions — pass through unchanged
            other => other,
        }
    }

    /// Wrap an I64 expression in a DivExpr that converts it to f64.
    /// The emitter generates `(@as(f64, @floatFromInt(expr)) / @as(f64, @floatFromInt(1)))`.
    fn wrap_i64_to_f64(expr: crate::zigir::types::IrExpr) -> crate::zigir::types::IrExpr {
        crate::zigir::types::IrExpr::DivExpr {
            left: Box::new(expr),
            right: Box::new(crate::zigir::types::IrExpr::IntLiteral(1)),
            left_type: ZigType::I64,
            right_type: ZigType::I64,
            result_type: None,
        }
    }

    /// Wrap an f64-producing expression so the result is `i64`.
    /// The emitter generates `@as(i64, @intFromFloat((expr / @as(f64, @floatFromInt(1)))))`.
    /// The `/ 1.0` is a no-op for IEEE 754 floats; Zig optimizes it away at comptime.
    fn wrap_f64_to_i64(expr: crate::zigir::types::IrExpr) -> crate::zigir::types::IrExpr {
        crate::zigir::types::IrExpr::DivExpr {
            left: Box::new(expr),
            right: Box::new(crate::zigir::types::IrExpr::IntLiteral(1)),
            left_type: ZigType::F64,
            right_type: ZigType::I64,
            result_type: Some(ZigType::I64),
        }
    }

    /// Wrap a JsAny-producing expression in `.asI64()` so the result is `i64`.
    /// The emitter renders `Call(FieldAccess(object, "asI64"), [])` as
    /// `object.asI64()`.
    fn wrap_jsany_to_i64(expr: crate::zigir::types::IrExpr) -> crate::zigir::types::IrExpr {
        use crate::zigir::kinds::CallKind;
        use crate::zigir::types::{IrCallExpr, IrExpr};
        IrExpr::Call(IrCallExpr {
            callee: Box::new(IrExpr::FieldAccess {
                object: Box::new(expr),
                field: "asI64".to_string(),
                field_kind: crate::zigir::kinds::FieldKind::StructField,
            }),
            args: vec![],
            call_kind: CallKind::Direct,
        })
    }

    /// Wrap an expression in `JsAny.from(...)` so the result is `JsAny`.
    /// The emitter renders `Call(FieldAccess(Ident("JsAny"), "from"), [expr])`
    /// as `JsAny.from(expr)`.
    fn wrap_in_jsany_from(expr: crate::zigir::types::IrExpr) -> crate::zigir::types::IrExpr {
        use crate::zigir::kinds::{CallKind, FieldKind};
        use crate::zigir::types::{IrCallExpr, IrExpr};
        IrExpr::Call(IrCallExpr {
            callee: Box::new(IrExpr::FieldAccess {
                object: Box::new(IrExpr::Ident(crate::zigir::ident::IrIdent::new("JsAny"))),
                field: "from".to_string(),
                field_kind: FieldKind::StructField,
            }),
            args: vec![expr],
            call_kind: CallKind::Direct,
        })
    }

    /// Coerce a return expression to `JsAny` when the function's return type
    /// is `JsAny` (e.g., P1-B7 default for non-export functions without JSDoc).
    /// Wraps non-JsAny expressions in `JsAny.from(...)`.
    fn coerce_jsany_result_type(
        &self,
        expr: crate::zigir::types::IrExpr,
    ) -> crate::zigir::types::IrExpr {
        use crate::zigir::types::IrExpr;

        match expr {
            // Already JsAny — pass through
            IrExpr::Call(call) => {
                let returns_jsany = match &*call.callee {
                    IrExpr::Ident(ident) | IrExpr::TypedIdent { ident, .. } => {
                        self.type_info.fn_return_types.get(&ident.js_name) == Some(&ZigType::JsAny)
                    }
                    _ => false,
                };
                if returns_jsany {
                    IrExpr::Call(call)
                } else {
                    // Non-JsAny-returning call — wrap
                    Self::wrap_in_jsany_from(IrExpr::Call(call))
                }
            }
            IrExpr::BuiltinCall(bc) if bc.return_type == ZigType::JsAny => IrExpr::BuiltinCall(bc),
            IrExpr::HostCall(hc) if hc.return_type == ZigType::JsAny => IrExpr::HostCall(hc),

            // Ident — check variable type in type_info
            IrExpr::Ident(ident) => {
                let is_jsany =
                    self.type_info.var_types.get(&ident.js_name) == Some(&ZigType::JsAny);
                if is_jsany {
                    IrExpr::Ident(ident)
                } else {
                    Self::wrap_in_jsany_from(IrExpr::Ident(ident))
                }
            }

            // TypedIdent — use embedded ty field directly
            IrExpr::TypedIdent { ident, ty } => {
                if ty == ZigType::JsAny {
                    IrExpr::TypedIdent { ident, ty }
                } else {
                    Self::wrap_in_jsany_from(IrExpr::TypedIdent { ident, ty })
                }
            }

            // Primitives and typed expressions — wrap in JsAny.from()
            IrExpr::IntLiteral(_)
            | IrExpr::FloatLiteral(_)
            | IrExpr::BoolLiteral(_)
            | IrExpr::StringLiteral(_) => Self::wrap_in_jsany_from(expr),
            IrExpr::BuiltinCall(bc)
                if matches!(
                    bc.return_type,
                    ZigType::I64 | ZigType::F64 | ZigType::Bool | ZigType::Str
                ) =>
            {
                Self::wrap_in_jsany_from(IrExpr::BuiltinCall(bc))
            }
            IrExpr::HostCall(hc)
                if matches!(
                    hc.return_type,
                    ZigType::I64 | ZigType::F64 | ZigType::Bool | ZigType::Str
                ) =>
            {
                Self::wrap_in_jsany_from(IrExpr::HostCall(hc))
            }
            IrExpr::Binary { .. } | IrExpr::Unary { .. } => Self::wrap_in_jsany_from(expr),
            IrExpr::FieldAccess { .. } => Self::wrap_in_jsany_from(expr),
            IrExpr::IndexAccess { .. } => Self::wrap_in_jsany_from(expr),
            // Arithmetic expressions producing f64 or i64 — wrap
            IrExpr::RemExpr { .. } | IrExpr::DivExpr { .. } | IrExpr::PowExpr { .. } => {
                Self::wrap_in_jsany_from(expr)
            }
            // Logical expressions (same_type path produces operand type)
            IrExpr::Logical { .. } => Self::wrap_in_jsany_from(expr),
            // Update (++, --) and assignment expressions produce i64/f64
            IrExpr::Update { .. } | IrExpr::Assign { .. } => Self::wrap_in_jsany_from(expr),
            // Literals — wrap
            IrExpr::ArrayLiteral(_) | IrExpr::ObjectLiteral(_) => Self::wrap_in_jsany_from(expr),
            IrExpr::TemplateLiteral { .. } | IrExpr::AllocPrint { .. } => {
                Self::wrap_in_jsany_from(expr)
            }
            IrExpr::BigIntLiteral(_) => Self::wrap_in_jsany_from(expr),
            IrExpr::New(_) => Self::wrap_in_jsany_from(expr),
            IrExpr::BlockExpr { .. } => Self::wrap_in_jsany_from(expr),
            // Null/Undefined already emit as JsAny — pass through
            IrExpr::Null | IrExpr::Undefined => expr,

            // Conditional — recurse into both branches
            IrExpr::Conditional { cond, then, else_ } => IrExpr::Conditional {
                cond,
                then: Box::new(self.coerce_jsany_result_type(*then)),
                else_: Box::new(self.coerce_jsany_result_type(*else_)),
            },

            // Sequence — only the last element is the return value
            IrExpr::Sequence(mut exprs) => {
                if let Some(last) = exprs.pop() {
                    exprs.push(self.coerce_jsany_result_type(last));
                }
                IrExpr::Sequence(exprs)
            }

            // Other expressions — pass through unchanged
            other => other,
        }
    }
}
