// zigir/lower/class.rs
// Class declaration lowering: fields, methods, constructor, this-rewrite.

use std::collections::HashSet;

use oxc_ast::ast::*;

use crate::types::ZigType;
use crate::zigir::types::{IrBlock, IrParam};

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
        let mut static_inits: Vec<(String, crate::zigir::types::IrExpr, crate::types::ZigType)> =
            Vec::new();
        let mut static_blocks: Vec<crate::zigir::types::IrBlock> = Vec::new();
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
                                    .unwrap_or(ZigType::I64);
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
                                .unwrap_or(ZigType::I64);
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

        // Compute needs_deinit: true if any field is Map, Set, ArrayList,
        // or a NamedStruct that itself needs deinit (nested class).
        let needs_deinit = fields.iter().any(|f| {
            matches!(
                f.zig_type,
                ZigType::NamedStruct(ref n) if n == "Map" || n == "Set"
            ) || matches!(f.zig_type, ZigType::ArrayList(_))
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
                                .and_then(|m| m.get(&fname))
                                .cloned()
                                .unwrap_or(ZigType::I64);
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

        // Lower body
        let body = func
            .body
            .as_ref()
            .map(|b| {
                if method_name == "init" {
                    // Constructor: use this-rewrite lowering
                    self.lower_block_with_this_rewrite(&b.statements, field_names)
                } else {
                    self.lower_block(&b.statements)
                }
            })
            .unwrap_or_else(|| IrBlock::new(vec![]));

        self.exit_fn(saved_fn);

        crate::zigir::types::IrClassMethod {
            name: method_name.to_string(),
            params,
            return_type,
            body,
            is_static,
        }
    }

    /// Lower a block of statements with `this.x = value` rewriting.
    ///
    /// In constructors, `this.field = value` is rewritten as a local const binding
    /// that the Emitter will use to build the struct return.
    pub(super) fn lower_block_with_this_rewrite(
        &mut self,
        stmts: &[Statement],
        field_names: &[String],
    ) -> IrBlock {
        let mut ir_stmts = Vec::new();
        for stmt in stmts {
            match stmt {
                Statement::ExpressionStatement(es) => {
                    // Check if this is `this.field = value`
                    if let Expression::AssignmentExpression(ae) = &es.expression
                        && let AssignmentTarget::StaticMemberExpression(sme) = &ae.left
                        && matches!(&sme.object, Expression::ThisExpression(_))
                    {
                        let fname = sme.property.name.to_string();
                        if field_names.contains(&fname) {
                            // this.field = value ¡ú const field = value
                            let value_ir = self.lower_expr(&ae.right);
                            ir_stmts.push(crate::zigir::types::IrStmt::VarDecl(
                                crate::zigir::types::IrVarDecl {
                                    name: self.make_ident(&fname),
                                    is_const: true,
                                    zig_type: None,
                                    init: Some(value_ir),
                                    is_json_parse: false,
                                    needs_var_suppression: false,
                                    needs_const_suppression: false,
                                    needs_deinit: false,
                                },
                            ));
                            continue;
                        }
                    }
                    // Fallback: lower as normal expression statement
                    ir_stmts.push(self.lower_stmt(stmt));
                }
                Statement::IfStatement(is) => {
                    // Recurse with this-rewrite for if branches
                    let test_ir = self.lower_expr(&is.test);
                    let consequent = self.lower_block_with_this_rewrite(
                        std::slice::from_ref(&is.consequent),
                        field_names,
                    );
                    let alternate = is.alternate.as_ref().map(|alt| {
                        self.lower_block_with_this_rewrite(std::slice::from_ref(alt), field_names)
                    });
                    ir_stmts.push(crate::zigir::types::IrStmt::If {
                        cond: test_ir,
                        then: consequent,
                        else_: alternate,
                    });
                }
                Statement::BlockStatement(bs) => {
                    let block = self.lower_block_with_this_rewrite(&bs.body, field_names);
                    ir_stmts.push(crate::zigir::types::IrStmt::Block(block));
                }
                _ => {
                    ir_stmts.push(self.lower_stmt(stmt));
                }
            }
        }
        IrBlock::new(ir_stmts)
    }
}
