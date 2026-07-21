// zigir/lower/class.rs
// Class declaration lowering: fields, methods, constructor, this-rewrite.

use std::collections::HashSet;

use oxc_ast::ast::*;

use crate::types::ZigType;
use crate::zigir::ops::{AssignOp, BinOp, UpdateOp};
use crate::zigir::types::{IrAssignTarget, IrBlock, IrExpr, IrParam, IrStmt};

use super::Lowerer;
use super::cabi::property_key_name;

// ¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T
//  Remaining stubs
// ¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T¨T

impl Lowerer {
    /// Generate a unique name for an anonymous class expression.
    fn next_anon_class_name(&mut self) -> String {
        let n = self.anon_class_counter;
        self.anon_class_counter += 1;
        format!("_AnonClass_{}", n)
    }

    /// Lower a class declaration into IrClassDecl.
    ///
    /// Extracts fields (from PropertyDefinition and implicit constructor `this.x = ...`),
    /// constructor ¡ú IrClassMethod, regular methods ¡ú IrClassMethod, and static inits.
    pub(super) fn lower_class_decl(
        &mut self,
        cd: &Class,
    ) -> Option<crate::zigir::types::IrClassDecl> {
        use crate::zigir::types::{IrClassDecl, IrClassField, IrClassMethod};

        let class_name = cd
            .id
            .as_ref()
            .map(|id| id.name.to_string())
            .unwrap_or_else(|| {
                // Anonymous class expression needs a unique name to avoid collisions
                self.next_anon_class_name()
            });

        // Save/restore current_class — set early so that static blocks
        // and field initializers can reference `this` correctly.
        let saved_class = self.current_class.take();
        self.current_class = Some(class_name.clone());

        // Pre-scan: collect static field names so `this.field` in static blocks
        // can be routed correctly via class_static_fields during lowering.
        let mut pre_scan_static_names: HashSet<String> = HashSet::new();
        for elem in &cd.body.body {
            if let ClassElement::PropertyDefinition(pd) = elem
                && pd.r#static
                && !pd.computed
                && let Some(name) = property_key_name(&pd.key)
            {
                pre_scan_static_names.insert(name);
            }
        }
        if !pre_scan_static_names.is_empty() {
            self.class_static_fields
                .insert(class_name.clone(), pre_scan_static_names);
        }

        // ── First pass: collect explicit fields from PropertyDefinition ──
        let mut field_names: Vec<String> = Vec::new();
        let mut fields: Vec<IrClassField> = Vec::new();
        let mut static_inits: Vec<(String, crate::zigir::types::IrExpr, ZigType)> = Vec::new();
        let mut static_blocks: Vec<IrBlock> = Vec::new();
        let mut has_constructor = false;
        let mut constructor_func: Option<&Function> = None;

        for elem in &cd.body.body {
            match elem {
                ClassElement::PropertyDefinition(pd) => {
                    if pd.computed {
                        continue;
                    }
                    let is_static = pd.r#static;
                    if let Some(name) = property_key_name(&pd.key) {
                        if is_static {
                            if let Some(value) = &pd.value {
                                let field_ty = self
                                    .type_info
                                    .class_field_types
                                    .get(&class_name)
                                    .and_then(|m| m.get(&name))
                                    .cloned()
                                    .or_else(|| {
                                        // Fallback: for anonymous class expressions,
                                        // field types are stored under the variable name
                                        self.class_expr_var_name.as_ref().and_then(|vn| {
                                            self.type_info
                                                .class_field_types
                                                .get(vn)
                                                .and_then(|m| m.get(&name))
                                                .cloned()
                                        })
                                    })
                                    .unwrap_or(ZigType::JsAny);
                                // Register static field type in var_types for Member target type inference
                                let var_key = format!("__{}_{}", class_name, name);
                                self.type_info.var_types.insert(var_key, field_ty.clone());
                                static_inits.push((name.clone(), self.lower_expr(value), field_ty));
                            }
                        } else if !field_names.contains(&name) {
                            let field_ty = self
                                .type_info
                                .class_field_types
                                .get(&class_name)
                                .and_then(|m| m.get(&name))
                                .cloned()
                                .or_else(|| {
                                    // Fallback: for anonymous class expressions,
                                    // field types are stored under the variable name
                                    self.class_expr_var_name.as_ref().and_then(|vn| {
                                        self.type_info
                                            .class_field_types
                                            .get(vn)
                                            .and_then(|m| m.get(&name))
                                            .cloned()
                                    })
                                })
                                .unwrap_or(ZigType::JsAny);
                            let default = pd.value.as_ref().map(|v| self.lower_expr(v));
                            field_names.push(name.clone());
                            fields.push(IrClassField {
                                name,
                                zig_type: field_ty,
                                default,
                            });
                        }
                    }
                }
                ClassElement::MethodDefinition(md) if Self::is_constructor_method(md) => {
                    has_constructor = true;
                    constructor_func = Some(&md.value);
                }
                ClassElement::StaticBlock(sb) => {
                    // Lower static block — set in_static_block so `this` → ClassName
                    let saved_static_block = self.in_static_block;
                    self.in_static_block = true;
                    let block = self.lower_block(&sb.body);
                    self.in_static_block = saved_static_block;
                    static_blocks.push(block);
                }
                _ => {}
            }
        }

        // ── Second pass: scan constructor body for implicit `this.x = ...` fields ──
        if let Some(func) = constructor_func
            && let Some(body) = &func.body
        {
            self.collect_implicit_class_fields(
                &body.statements,
                &class_name,
                &mut field_names,
                &mut fields,
            );
        }

        // ── Lower constructor ──
        let constructor = if has_constructor {
            constructor_func
                .map(|func| self.lower_class_method(&class_name, &field_names, "init", func, false))
        } else {
            None
        };

        // ©¤©¤ Lower methods ©¤©¤
        let mut methods: Vec<IrClassMethod> = Vec::new();
        for elem in &cd.body.body {
            if let ClassElement::MethodDefinition(md) = elem
                && !Self::is_constructor_method(md)
            {
                let method_name =
                    property_key_name(&md.key).unwrap_or_else(|| "anonymous".to_string());
                let is_static = md.r#static;
                let method = self.lower_class_method(
                    &class_name,
                    &field_names,
                    &method_name,
                    &md.value,
                    is_static,
                );
                methods.push(method);
            }
        }

        // ©¤©¤ Restore ©¤©¤
        self.current_class = saved_class;

        // ©¤©¤ extends ©¤©¤
        let extends = cd.super_class.as_ref().and_then(|sc| {
            if let Expression::Identifier(id) = sc {
                Some(id.name.to_string())
            } else {
                None
            }
        });

        // static field names already registered in class_static_fields during pre-scan

        // Compute needs_deinit: true if any field is Map, Set, ArrayList, BigInt,
        // or a NamedStruct that itself needs deinit (nested class).
        let needs_deinit = fields.iter().any(|f| {
            matches!(
                f.zig_type,
                ZigType::NamedStruct(ref n) if n == "Map" || n == "Set"
            ) || matches!(f.zig_type, ZigType::ArrayList(_))
                || matches!(f.zig_type, ZigType::BigInt)
        });

        Some(IrClassDecl {
            name: self.make_ident(&class_name),
            fields,
            constructor,
            methods,
            static_inits,
            static_blocks,
            extends,
            needs_deinit,
        })
    }

    /// Check if a MethodDefinition is `constructor()`.
    pub(super) fn is_constructor_method(md: &MethodDefinition) -> bool {
        property_key_name(&md.key).is_some_and(|name| name == "constructor")
    }

    /// Scan constructor body for `this.x = ...` assignments and add
    /// discovered fields if not already present.
    pub(super) fn collect_implicit_class_fields(
        &self,
        stmts: &[Statement],
        class_name: &str,
        field_names: &mut Vec<String>,
        fields: &mut Vec<crate::zigir::types::IrClassField>,
    ) {
        for stmt in stmts {
            match stmt {
                Statement::ExpressionStatement(es) => {
                    if let Expression::AssignmentExpression(ae) = &es.expression {
                        let maybe_fname = match &ae.left {
                            AssignmentTarget::StaticMemberExpression(sme)
                                if matches!(&sme.object, Expression::ThisExpression(_)) =>
                            {
                                Some(sme.property.name.to_string())
                            }
                            AssignmentTarget::PrivateFieldExpression(pfe)
                                if matches!(&pfe.object, Expression::ThisExpression(_)) =>
                            {
                                Some(pfe.field.name.to_string())
                            }
                            _ => None,
                        };
                        if let Some(fname) = maybe_fname
                            && !field_names.contains(&fname)
                        {
                            let ftype = self
                                .type_info
                                .class_field_types
                                .get(class_name)
                                .and_then(|m| m.get(&fname))
                                .cloned()
                                .or_else(|| {
                                    // Fallback: for anonymous class expressions,
                                    // field types are stored under the variable name
                                    self.class_expr_var_name.as_ref().and_then(|vn| {
                                        self.type_info
                                            .class_field_types
                                            .get(vn)
                                            .and_then(|m| m.get(&fname))
                                            .cloned()
                                    })
                                })
                                .unwrap_or(ZigType::JsAny);
                            field_names.push(fname.clone());
                            fields.push(crate::zigir::types::IrClassField {
                                name: fname,
                                zig_type: ftype,
                                default: None,
                            });
                        }
                    }
                }
                Statement::IfStatement(is) => {
                    self.collect_implicit_class_fields(
                        std::slice::from_ref(&is.consequent),
                        class_name,
                        field_names,
                        fields,
                    );
                    if let Some(alt) = &is.alternate {
                        self.collect_implicit_class_fields(
                            std::slice::from_ref(alt),
                            class_name,
                            field_names,
                            fields,
                        );
                    }
                }
                Statement::BlockStatement(bs) => {
                    self.collect_implicit_class_fields(&bs.body, class_name, field_names, fields);
                }
                // R8-C7: Recurse into loop/switch/try bodies so that
                // `this.field = value` nested inside them is discovered as a
                // class field (added to field_names) and later rewritten by
                // try_rewrite_this_field_assignment during body lowering.
                // Without this, such fields were silently dropped.
                Statement::WhileStatement(ws) => {
                    self.collect_implicit_class_fields(
                        std::slice::from_ref(&ws.body),
                        class_name,
                        field_names,
                        fields,
                    );
                }
                Statement::DoWhileStatement(dws) => {
                    self.collect_implicit_class_fields(
                        std::slice::from_ref(&dws.body),
                        class_name,
                        field_names,
                        fields,
                    );
                }
                Statement::ForStatement(fs) => {
                    self.collect_implicit_class_fields(
                        std::slice::from_ref(&fs.body),
                        class_name,
                        field_names,
                        fields,
                    );
                }
                Statement::ForOfStatement(fos) => {
                    self.collect_implicit_class_fields(
                        std::slice::from_ref(&fos.body),
                        class_name,
                        field_names,
                        fields,
                    );
                }
                Statement::ForInStatement(fis) => {
                    self.collect_implicit_class_fields(
                        std::slice::from_ref(&fis.body),
                        class_name,
                        field_names,
                        fields,
                    );
                }
                Statement::LabeledStatement(ls) => {
                    self.collect_implicit_class_fields(
                        std::slice::from_ref(&ls.body),
                        class_name,
                        field_names,
                        fields,
                    );
                }
                Statement::SwitchStatement(ss) => {
                    for case in &ss.cases {
                        self.collect_implicit_class_fields(
                            &case.consequent,
                            class_name,
                            field_names,
                            fields,
                        );
                    }
                }
                Statement::TryStatement(ts) => {
                    self.collect_implicit_class_fields(
                        &ts.block.body,
                        class_name,
                        field_names,
                        fields,
                    );
                    if let Some(handler) = &ts.handler {
                        self.collect_implicit_class_fields(
                            &handler.body.body,
                            class_name,
                            field_names,
                            fields,
                        );
                    }
                    if let Some(fin) = &ts.finalizer {
                        self.collect_implicit_class_fields(
                            &fin.body,
                            class_name,
                            field_names,
                            fields,
                        );
                    }
                }
                _ => {}
            }
        }
    }

    /// Lower a class method (constructor or regular) into IrClassMethod.
    pub(super) fn lower_class_method(
        &mut self,
        class_name: &str,
        field_names: &[String],
        method_name: &str,
        func: &Function,
        is_static: bool,
    ) -> crate::zigir::types::IrClassMethod {
        // For fully-qualified key lookups
        let fq_method = format!("{}.{}", class_name, method_name);

        let return_type = self
            .type_info
            .fn_return_types
            .get(&fq_method)
            .or_else(|| self.type_info.fn_return_types.get(method_name))
            .or_else(|| {
                // Fallback: for anonymous class expressions, return types are
                // stored under the variable name (e.g. "Point.sum" not "_AnonClass_0.sum")
                self.class_expr_var_name.as_ref().and_then(|vn| {
                    let var_fq = format!("{}.{}", vn, method_name);
                    self.type_info.fn_return_types.get(&var_fq)
                })
            })
            .cloned()
            .unwrap_or(if method_name == "init" {
                ZigType::NamedStruct(class_name.to_string())
            } else {
                ZigType::Void
            });

        // Parameters
        let params = if method_name == "init" {
            self.lower_fn_params(func, "init")
        } else {
            let param_types = self
                .type_info
                .fn_param_types
                .get(&fq_method)
                .or_else(|| self.type_info.fn_param_types.get(method_name))
                .or_else(|| {
                    // Fallback: for anonymous class expressions
                    self.class_expr_var_name.as_ref().and_then(|vn| {
                        let var_fq = format!("{}.{}", vn, method_name);
                        self.type_info.fn_param_types.get(&var_fq)
                    })
                })
                .cloned();
            if let Some(ptypes) = param_types {
                let mut params = Vec::new();
                for (pname, ptype) in &ptypes {
                    params.push(IrParam {
                        name: self.make_ident(pname),
                        zig_type: ptype.clone(),
                        is_unused: false,
                        is_rest: false,
                    });
                }
                params
            } else {
                self.lower_fn_params(func, method_name)
            }
        };

        // Enter function context
        let saved_fn = self.enter_fn(method_name, false, Some(return_type.clone()));

        // For static methods, set in_static_block so `this` → ClassName
        // (static methods don't have a `self` parameter in Zig).
        let saved_static_block = self.in_static_block;
        if is_static && method_name != "init" {
            self.in_static_block = true;
        }

        // Lower body
        let body = func
            .body
            .as_ref()
            .map(|b| {
                if method_name == "init" {
                    // Constructor: set the this-rewrite flag so that
                    // `this.field = value` statements (directly, or nested
                    // inside if/loop/switch/try bodies) are rewritten to
                    // `const field = value` for the Emitter's struct return.
                    // `enter_fn` above already saved+cleared the flag; we set
                    // it here and `exit_fn` will restore the (cleared) saved
                    // value, scoping the flag to just this constructor body.
                    self.this_rewrite_fields = Some(field_names.to_vec());
                    self.lower_block(&b.statements)
                } else {
                    self.lower_block(&b.statements)
                }
            })
            .unwrap_or_else(|| IrBlock::new(vec![]));

        self.in_static_block = saved_static_block;
        self.exit_fn(saved_fn);

        crate::zigir::types::IrClassMethod {
            name: method_name.to_string(),
            params,
            return_type,
            body,
            is_static,
        }
    }

    /// R8-C7/C9: Rewrite `this.field` mutations into local `field` operations
    /// that write to a pre-declared `var` in the constructor, so the Emitter
    /// can collect field values for the struct return.
    ///
    /// Handles three categories:
    /// 1. `this.field = value` → `Assign { target: Ident(field), op: Assign }`
    /// 2. `this.field += value` (and all compound ops) → `Assign { target: Ident(field), op: <compound> }`
    ///    - BigInt compound: expanded to `field = field <op> value` (no Zig += for BigInt)
    /// 3. `this.field++` / `this.field--` → `Expr(Update { target: Ident(field), is_expr_stmt: true })`
    ///    - BigInt ++/--: expanded to `field = field.add/sub(BigInt(1))`
    ///
    /// `**=` and `>>>=` are NOT handled (they expand to BlockExpr in
    /// lower_assignment) — these rare cases fall through to normal lowering.
    ///
    /// Returns `Some(IrStmt)` when the expression is a rewriteable this-field
    /// mutation whose name is in `field_names`; otherwise `None`.
    ///
    /// Invoked from `lower_stmt`'s ExpressionStatement arm and from
    /// `lower_for_statement`'s init/update. The active field list lives on
    /// `self.this_rewrite_fields` and is cleared inside nested function
    /// contexts (via `enter_fn`) so that `this` in a nested function is not
    /// confused with the constructor's `this`.
    pub(super) fn try_rewrite_this_field_assignment(
        &mut self,
        expr: &Expression,
        field_names: &[String],
    ) -> Option<IrStmt> {
        match expr {
            // this.field = value / this.field += value / etc.
            Expression::AssignmentExpression(ae) => {
                let fname = self.extract_this_field_name_from_assign_target(&ae.left)?;
                if !field_names.contains(&fname) {
                    return None;
                }
                // **= and >>>= are expanded by lower_assignment into
                // BlockExpr — let them fall through to normal lowering.
                if matches!(
                    ae.operator,
                    AssignmentOperator::Exponential | AssignmentOperator::ShiftRightZeroFill
                ) {
                    return None;
                }
                // BigInt compound assignments need expansion to
                // field = field <op> value (no Zig += for BigInt).
                if ae.operator != AssignmentOperator::Assign
                    && self.infer_assign_target_type(&ae.left) == Some(ZigType::BigInt)
                {
                    let bin_op = match ae.operator {
                        AssignmentOperator::Addition => BinOp::Add,
                        AssignmentOperator::Subtraction => BinOp::Sub,
                        AssignmentOperator::Multiplication => BinOp::Mul,
                        AssignmentOperator::Division => BinOp::Div,
                        AssignmentOperator::Remainder => BinOp::Mod,
                        AssignmentOperator::BitwiseAnd => BinOp::BitAnd,
                        AssignmentOperator::BitwiseOR => BinOp::BitOr,
                        AssignmentOperator::BitwiseXOR => BinOp::BitXor,
                        AssignmentOperator::ShiftLeft => BinOp::Shl,
                        AssignmentOperator::ShiftRight => BinOp::Shr,
                        _ => return None,
                    };
                    let value_ir = self.lower_expr(&ae.right);
                    let read_expr = IrExpr::Ident(self.make_ident(&fname));
                    return Some(IrStmt::Assign {
                        target: IrAssignTarget::Ident(self.make_ident(&fname)),
                        op: AssignOp::Assign,
                        value: IrExpr::Binary {
                            op: bin_op,
                            left: Box::new(read_expr),
                            right: Box::new(value_ir),
                            left_type: Some(ZigType::BigInt),
                            right_type: Some(ZigType::BigInt),
                        },
                    });
                }
                // Non-BigInt compound or plain =: map operator directly.
                let op = match ae.operator {
                    AssignmentOperator::Assign => AssignOp::Assign,
                    AssignmentOperator::Addition => AssignOp::Add,
                    AssignmentOperator::Subtraction => AssignOp::Sub,
                    AssignmentOperator::Multiplication => AssignOp::Mul,
                    AssignmentOperator::Division => AssignOp::Div,
                    AssignmentOperator::Remainder => AssignOp::Mod,
                    AssignmentOperator::ShiftLeft => AssignOp::Shl,
                    AssignmentOperator::ShiftRight => AssignOp::Shr,
                    AssignmentOperator::BitwiseAnd => AssignOp::BitAnd,
                    AssignmentOperator::BitwiseOR => AssignOp::BitOr,
                    AssignmentOperator::BitwiseXOR => AssignOp::BitXor,
                    AssignmentOperator::LogicalAnd => AssignOp::LogicAnd,
                    AssignmentOperator::LogicalOr => AssignOp::LogicOr,
                    AssignmentOperator::LogicalNullish => AssignOp::Nullish,
                    _ => return None,
                };
                let value_ir = self.lower_expr(&ae.right);
                Some(IrStmt::Assign {
                    target: IrAssignTarget::Ident(self.make_ident(&fname)),
                    op,
                    value: value_ir,
                })
            }
            // this.field++ / this.field-- / ++this.field / --this.field
            Expression::UpdateExpression(ue) => {
                let fname = self.extract_this_field_name_from_simple_target(&ue.argument)?;
                if !field_names.contains(&fname) {
                    return None;
                }
                // BigInt ++/-- needs expansion to field = field.add/sub(BigInt(1)).
                // In statement context (which is the only context this function is
                // called from), the return value is discarded, so prefix/postfix
                // are equivalent — both just mutate the field.
                if self.infer_simple_assign_target_type(&ue.argument) == Some(ZigType::BigInt) {
                    let bin_op = if ue.operator == UpdateOperator::Increment {
                        BinOp::Add
                    } else {
                        BinOp::Sub
                    };
                    let read_expr = IrExpr::Ident(self.make_ident(&fname));
                    return Some(IrStmt::Assign {
                        target: IrAssignTarget::Ident(self.make_ident(&fname)),
                        op: AssignOp::Assign,
                        value: IrExpr::Binary {
                            op: bin_op,
                            left: Box::new(read_expr),
                            right: Box::new(IrExpr::BigIntLiteral("1".to_string())),
                            left_type: Some(ZigType::BigInt),
                            right_type: Some(ZigType::BigInt),
                        },
                    });
                }
                // Non-BigInt: emit field++ / field-- as a statement.
                let op = if ue.operator == UpdateOperator::Increment {
                    UpdateOp::Increment
                } else {
                    UpdateOp::Decrement
                };
                Some(IrStmt::Expr(IrExpr::Update {
                    op,
                    target: Box::new(IrAssignTarget::Ident(self.make_ident(&fname))),
                    is_expr_stmt: true,
                    prefix: ue.prefix,
                }))
            }
            _ => None,
        }
    }

    /// Extract field name from `this.field` in an AssignmentTarget (for
    /// `this.field = ...` or `this.field += ...`).
    fn extract_this_field_name_from_assign_target(
        &self,
        target: &AssignmentTarget,
    ) -> Option<String> {
        match target {
            AssignmentTarget::StaticMemberExpression(sme)
                if matches!(&sme.object, Expression::ThisExpression(_)) =>
            {
                Some(sme.property.name.to_string())
            }
            AssignmentTarget::PrivateFieldExpression(pfe)
                if matches!(&pfe.object, Expression::ThisExpression(_)) =>
            {
                // PrivateIdentifier.name does NOT include the '#' prefix
                Some(pfe.field.name.to_string())
            }
            _ => None,
        }
    }

    /// Extract field name from `this.field` in a SimpleAssignmentTarget (for
    /// `this.field++` / `++this.field`).
    fn extract_this_field_name_from_simple_target(
        &self,
        target: &SimpleAssignmentTarget,
    ) -> Option<String> {
        match target {
            SimpleAssignmentTarget::StaticMemberExpression(sme)
                if matches!(&sme.object, Expression::ThisExpression(_)) =>
            {
                Some(sme.property.name.to_string())
            }
            SimpleAssignmentTarget::PrivateFieldExpression(pfe)
                if matches!(&pfe.object, Expression::ThisExpression(_)) =>
            {
                Some(pfe.field.name.to_string())
            }
            _ => None,
        }
    }
}
