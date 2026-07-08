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
                        stmts.into_iter().next().unwrap()
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
                if let Some(fn_ctx) = self.fn_ctx_mut() {
                    fn_ctx.seen_return = true;
                }
                let value = rs.argument.as_ref().map(|expr| self.lower_expr(expr));
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
                    self.in_expr_stmt = true;
                    let expr = self.lower_expr(&es.expression);
                    self.in_expr_stmt = false;
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
                // Emit as VarDecl statement(s)
                let decl = self.lower_var_decl(&vd.declarations[0], vd.kind.is_const());
                match decl {
                    IrDecl::Var(v) => Box::new(crate::zigir::types::IrStmt::VarDecl(v)),
                    _ => Box::new(crate::zigir::types::IrStmt::Comment(
                        "// skipped init".to_string(),
                    )),
                }
            }
            _ => {
                // Expression init: lower as expression statement
                if let Some(expr) = init.as_expression() {
                    Box::new(crate::zigir::types::IrStmt::Expr(self.lower_expr(expr)))
                } else {
                    Box::new(crate::zigir::types::IrStmt::Comment(
                        "// skipped init".to_string(),
                    ))
                }
            }
        });

        let cond = fs.test.as_ref().map(|expr| self.lower_expr(expr));
        let update = fs
            .update
            .as_ref()
            .map(|expr| Box::new(crate::zigir::types::IrStmt::Expr(self.lower_expr(expr))));
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
        let (kind, iterable_is_arraylist) = self.detect_for_of_kind(&fos.right, &destructure_vars);

        if matches!(kind, IrForOfKind::AsyncUnsupported) {
            return crate::zigir::types::IrStmt::CompileError {
                span: SourceSpan::default(),
                msg: "for await...of is not supported".to_string(),
            };
        }

        let iterable = self.lower_expr(&fos.right);
        let body = self.lower_stmt_as_block(&fos.body, None);

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
        // "parameter used" detection. For HashMapIter, we need the actual
        // iterable expression at runtime.
        //
        // However, for unused-param detection, we still need to track that
        // the iterable expression references identifiers (e.g., the param `cfg`
        // in `for (const key in cfg)`), even though it's replaced by Null.
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
    /// Note: `destructure_vars` is not used for kind detection (it's stored
    /// in the ForOf node for the Emitter to use), but kept for future use
    /// (e.g. distinguishing single-var vs destructure patterns).
    #[allow(unused_variables)]
    pub(super) fn detect_for_of_kind(
        &self,
        right: &Expression,
        _destructure_vars: &[IrIdent],
    ) -> (IrForOfKind, bool) {
        match right {
            Expression::Identifier(id) => {
                if let Some(zig_type) = self.type_info.var_types.get(id.name.as_str()) {
                    // Map ¡ú iterator pattern
                    if let ZigType::NamedStruct(name) = zig_type {
                        if name == "Map" {
                            return (IrForOfKind::MapSetIter { is_map: true }, false);
                        }
                        if name == "Set" {
                            return (IrForOfKind::MapSetIter { is_map: false }, false);
                        }
                    }
                    // ArrayList ¡ú use .items
                    if matches!(zig_type, ZigType::ArrayList(_)) {
                        return (IrForOfKind::Array, true);
                    }
                }
                // Default: array iteration
                (IrForOfKind::Array, false)
            }
            _ => (IrForOfKind::Array, false),
        }
    }

    /// Detect for-in iteration kind based on the right-hand expression type.
    pub(super) fn detect_for_in_kind(&self, right: &Expression) -> IrForInKind {
        match right {
            Expression::Identifier(id) => {
                if let Some(zig_type) = self.type_info.var_types.get(id.name.as_str()) {
                    // HashMap/dynamic object ¡ú iterator-based
                    if matches!(zig_type, ZigType::Anytype) {
                        return IrForInKind::HashMapIter;
                    }
                    // Static struct with known fields ¡ú unroll
                    if let ZigType::Struct(fields) = zig_type
                        && !fields.is_empty()
                    {
                        return IrForInKind::StructUnroll {
                            fields: fields.iter().map(|(n, _)| n.clone()).collect(),
                        };
                    }
                    // Named struct (e.g., JSDoc @typedef) ¡ú resolve to StructUnroll
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
        let cases: Vec<crate::zigir::types::IrSwitchCase> = ss
            .cases
            .iter()
            .map(|case| {
                let test = case.test.as_ref().map(|e| self.lower_expr(e));
                // Filter out break statements (Zig switch doesn't need them)
                let body: Vec<crate::zigir::types::IrStmt> = case
                    .consequent
                    .iter()
                    .filter(|s| !matches!(s, Statement::BreakStatement(_)))
                    .map(|s| self.lower_stmt(s))
                    .collect();
                crate::zigir::types::IrSwitchCase { test, body }
            })
            .collect();

        crate::zigir::types::IrStmt::Switch { expr, cases }
    }

    /// Lower a try-catch statement.
    pub(super) fn lower_try(&mut self, ts: &TryStatement) -> crate::zigir::types::IrStmt {
        // Lower try block first, then inspect the resulting IR for throws.
        // This catches implicit throws like const-reassignment guards that
        // `stmt_has_throw_any` (AST-level) cannot detect.
        let try_block = {
            let stmts = ts.block.body.iter().map(|s| self.lower_stmt(s)).collect();
            IrBlock::new(stmts)
        };

        // AST-level throw detection (for nested try exclusion)
        let has_nested_try = ts
            .block
            .body
            .iter()
            .any(|s| matches!(s, Statement::TryStatement(_)));

        // IR-level throw detection: scan lowered try_block for any IrStmt::Throw.
        fn ir_block_has_throw(block: &IrBlock) -> bool {
            block.stmts.iter().any(|s| match s {
                crate::zigir::types::IrStmt::Throw { .. } => true,
                // Recurse into nested blocks (if, while, for, etc.)
                crate::zigir::types::IrStmt::If { then, else_, .. } => {
                    ir_block_has_throw(then) || else_.as_ref().is_some_and(ir_block_has_throw)
                }
                crate::zigir::types::IrStmt::While { body, .. }
                | crate::zigir::types::IrStmt::DoWhile { body, .. }
                | crate::zigir::types::IrStmt::For { body, .. }
                | crate::zigir::types::IrStmt::ForOf { body, .. }
                | crate::zigir::types::IrStmt::ForIn { body, .. } => ir_block_has_throw(body),
                crate::zigir::types::IrStmt::Block(b) => ir_block_has_throw(b),
                _ => false,
            })
        }
        let has_throw = ir_block_has_throw(&try_block);

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
            Statement::WhileStatement(ws) => Self::stmt_references_name(&ws.body, name),
            Statement::ForStatement(fs) => {
                Self::stmt_references_name(&fs.body, name)
                    || fs
                        .test
                        .as_ref()
                        .is_some_and(|e| Self::expr_references_name(e, name))
            }
            Statement::ForOfStatement(fos) => Self::stmt_references_name(&fos.body, name),
            Statement::ForInStatement(fis) => Self::stmt_references_name(&fis.body, name),
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
            Expression::CallExpression(ce) => {
                Self::expr_references_name(&ce.callee, name)
                    || ce
                        .arguments
                        .iter()
                        .any(|a| Self::arg_references_name(a, name))
            }
            Expression::StaticMemberExpression(sme) => {
                Self::expr_references_name(&sme.object, name)
            }
            Expression::UnaryExpression(ue) => Self::expr_references_name(&ue.argument, name),
            Expression::ConditionalExpression(ce) => {
                Self::expr_references_name(&ce.test, name)
                    || Self::expr_references_name(&ce.consequent, name)
                    || Self::expr_references_name(&ce.alternate, name)
            }
            Expression::AssignmentExpression(ae) => Self::expr_references_name(&ae.right, name),
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

    /// Check if a statement contains a `throw` (directly, not inside a
    /// nested try-catch — those throws are caught by the inner catch).
    #[allow(dead_code)]
    pub(super) fn stmt_has_throw_any(stmt: &Statement) -> bool {
        match stmt {
            Statement::ThrowStatement(_) => true,
            Statement::BlockStatement(bs) => bs.body.iter().any(|s| Self::stmt_has_throw_any(s)),
            Statement::IfStatement(is) => {
                Self::stmt_has_throw_any(&is.consequent)
                    || is
                        .alternate
                        .as_ref()
                        .is_some_and(|a| Self::stmt_has_throw_any(a))
            }
            Statement::WhileStatement(ws) => Self::stmt_has_throw_any(&ws.body),
            Statement::DoWhileStatement(dws) => Self::stmt_has_throw_any(&dws.body),
            Statement::ForStatement(fs) => Self::stmt_has_throw_any(&fs.body),
            Statement::ForOfStatement(fos) => Self::stmt_has_throw_any(&fos.body),
            Statement::ForInStatement(fis) => Self::stmt_has_throw_any(&fis.body),
            Statement::SwitchStatement(ss) => ss
                .cases
                .iter()
                .any(|c| c.consequent.iter().any(|s| Self::stmt_has_throw_any(s))),
            Statement::LabeledStatement(ls) => Self::stmt_has_throw_any(&ls.body),
            // Intentionally NOT recursing into TryStatement ¡ª
            // throws inside a nested try are caught by its own catch.
            _ => false,
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
}
