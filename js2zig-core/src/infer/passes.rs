// native_proto/infer/passes.rs
// Analysis passes: Pass 0 (object analysis), Pass 1 (used names),
// Pass 2 (toplevel type walker) — excluding walk_fn_for_types.

use super::helpers::binding_name;
use super::{InferResult, TypeInferrer};
use crate::types::ZigType;
use oxc_ast::ast::*;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

impl TypeInferrer {
    // ============================================================
    // Pass 0: analyze objects (mutations, dynamic access errors)
    // ============================================================

    pub(crate) fn analyze_objects(&mut self, program: &Program) {
        for stmt in &program.body {
            self.walk_stmt_for_analysis(stmt);
        }
    }

    pub(crate) fn walk_stmt_for_analysis(&mut self, stmt: &Statement) {
        match stmt {
            Statement::VariableDeclaration(vd) => {
                for decl in &vd.declarations {
                    if let Some(init) = &decl.init {
                        self.walk_expr_for_analysis(init);
                    }
                }
            }
            Statement::FunctionDeclaration(fd) => {
                self.walk_fn_body_for_analysis(fd);
            }
            Statement::ExportNamedDeclaration(export_decl) => match &export_decl.declaration {
                Some(Declaration::FunctionDeclaration(fd)) => {
                    self.walk_fn_body_for_analysis(fd);
                }
                Some(Declaration::ClassDeclaration(cd)) => {
                    self.walk_class_body_for_analysis(cd);
                }
                _ => {}
            },
            Statement::ExportDefaultDeclaration(export_decl) => match &export_decl.declaration {
                ExportDefaultDeclarationKind::FunctionDeclaration(fd) => {
                    self.walk_fn_body_for_analysis(fd);
                }
                ExportDefaultDeclarationKind::ClassDeclaration(cd) => {
                    self.walk_class_body_for_analysis(cd);
                }
                _ => {}
            },
            Statement::ExpressionStatement(es) => {
                self.walk_expr_for_analysis(&es.expression);
            }
            Statement::IfStatement(is) => {
                self.walk_expr_for_analysis(&is.test);
                self.walk_stmt_for_analysis(&is.consequent);
                if let Some(alt) = &is.alternate {
                    self.walk_stmt_for_analysis(alt);
                }
            }
            Statement::TryStatement(ts) => {
                for s in &ts.block.body {
                    self.walk_stmt_for_analysis(s);
                }
                if let Some(handler) = &ts.handler {
                    for s in &handler.body.body {
                        self.walk_stmt_for_analysis(s);
                    }
                }
                if let Some(finalizer) = &ts.finalizer {
                    for s in &finalizer.body {
                        self.walk_stmt_for_analysis(s);
                    }
                }
            }
            Statement::WhileStatement(ws) => {
                self.walk_expr_for_analysis(&ws.test);
                self.walk_stmt_for_analysis(&ws.body);
            }
            Statement::DoWhileStatement(dws) => {
                self.walk_stmt_for_analysis(&dws.body);
                self.walk_expr_for_analysis(&dws.test);
            }
            Statement::ForStatement(fs) => {
                if let Some(init) = &fs.init {
                    if let ForStatementInit::VariableDeclaration(vd) = init {
                        self.walk_expr_for_stmt_init_vardecl(vd);
                    } else if let Some(expr) = init.as_expression() {
                        self.walk_expr_for_analysis(expr);
                    }
                }
                if let Some(test) = &fs.test {
                    self.walk_expr_for_analysis(test);
                }
                if let Some(update) = &fs.update {
                    self.walk_expr_for_analysis(update);
                }
                self.walk_stmt_for_analysis(&fs.body);
            }
            Statement::ForOfStatement(fos) => {
                if let ForStatementLeft::VariableDeclaration(vd) = &fos.left {
                    self.walk_expr_for_stmt_init_vardecl(vd);
                }
                self.walk_expr_for_analysis(&fos.right);
                self.walk_stmt_for_analysis(&fos.body);
            }
            Statement::ForInStatement(fis) => {
                // for-in loop variable is always string (property name) — no init to walk.
                self.walk_expr_for_analysis(&fis.right);
                self.walk_stmt_for_analysis(&fis.body);
            }
            Statement::SwitchStatement(ss) => {
                self.walk_expr_for_analysis(&ss.discriminant);
                for case in &ss.cases {
                    if let Some(test) = &case.test {
                        self.walk_expr_for_analysis(test);
                    }
                    for s in &case.consequent {
                        self.walk_stmt_for_analysis(s);
                    }
                }
            }
            Statement::BlockStatement(bs) => {
                for s in &bs.body {
                    self.walk_stmt_for_analysis(s);
                }
            }
            Statement::LabeledStatement(ls) => {
                // Recurse into labeled statement body (e.g. `outer: for(...) {...}`)
                self.walk_stmt_for_analysis(&ls.body);
            }
            Statement::ClassDeclaration(cd) => {
                // Walk class method bodies for analysis (mutations, dynamic access)
                self.walk_class_body_for_analysis(cd);
            }
            Statement::ThrowStatement(ts) => {
                self.walk_expr_for_analysis(&ts.argument);
            }
            Statement::ReturnStatement(rs) => {
                if let Some(arg) = &rs.argument {
                    self.walk_expr_for_analysis(arg);
                }
            }
            _ => {}
        }
    }

    /// Walk a class's method bodies for analysis (mutations, dynamic access).
    /// Used by ClassDeclaration, ExportNamedDeclaration, and ExportDefaultDeclaration.
    fn walk_class_body_for_analysis(&mut self, cd: &Class) {
        for elem in &cd.body.body {
            if let ClassElement::MethodDefinition(md) = elem
                && let Some(body) = &md.value.body
            {
                let saved_fn = std::mem::take(&mut self.current_fn);
                self.current_fn = md.key.name().map(|s| s.to_string());
                for s in &body.statements {
                    self.walk_stmt_for_analysis(s);
                }
                self.current_fn = saved_fn;
            }
        }
    }

    /// Walk a VariableDeclaration inside a for-statement init for analysis.
    fn walk_expr_for_stmt_init_vardecl(&mut self, vd: &VariableDeclaration) {
        for decl in &vd.declarations {
            if let Some(init) = &decl.init {
                self.walk_expr_for_analysis(init);
            }
        }
    }

    /// Walk a function declaration body, saving and restoring `current_fn`.
    fn walk_fn_body_for_analysis(&mut self, fd: &Function) {
        let saved_current_fn = std::mem::take(&mut self.current_fn);
        self.current_fn = fd.id.as_ref().map(|id| id.name.to_string());
        if let Some(body) = &fd.body {
            for s in &body.statements {
                self.walk_stmt_for_analysis(s);
            }
        }
        self.current_fn = saved_current_fn;
    }

    pub(crate) fn walk_expr_for_analysis(&mut self, expr: &Expression) {
        match expr {
            Expression::ComputedMemberExpression(mem) => {
                self.walk_expr_for_analysis(&mem.object);
                self.walk_expr_for_analysis(&mem.expression);
            }
            Expression::StaticMemberExpression(mem) => {
                self.walk_expr_for_analysis(&mem.object);
            }
            Expression::AssignmentExpression(ae) => {
                self.check_assignment_target(&ae.left);
                self.walk_expr_for_analysis(&ae.right);
            }
            Expression::BinaryExpression(be) => {
                self.walk_expr_for_analysis(&be.left);
                self.walk_expr_for_analysis(&be.right);
            }
            Expression::CallExpression(ce) => {
                self.walk_expr_for_analysis(&ce.callee);
                for arg in &ce.arguments {
                    if let Some(e) = arg.as_expression() {
                        self.walk_expr_for_analysis(e);
                    }
                }
            }
            Expression::ParenthesizedExpression(pe) => {
                self.walk_expr_for_analysis(&pe.expression);
            }
            Expression::ConditionalExpression(ce) => {
                self.walk_expr_for_analysis(&ce.test);
                self.walk_expr_for_analysis(&ce.consequent);
                self.walk_expr_for_analysis(&ce.alternate);
            }
            Expression::UnaryExpression(ue) => {
                self.walk_expr_for_analysis(&ue.argument);
            }
            Expression::LogicalExpression(le) => {
                self.walk_expr_for_analysis(&le.left);
                self.walk_expr_for_analysis(&le.right);
            }
            Expression::ArrayExpression(ae) => {
                for elem in &ae.elements {
                    if let Some(e) = elem.as_expression() {
                        self.walk_expr_for_analysis(e);
                    }
                }
            }
            Expression::ObjectExpression(oe) => {
                for prop in &oe.properties {
                    if let ObjectPropertyKind::ObjectProperty(p) = prop {
                        self.walk_expr_for_analysis(&p.value);
                    }
                }
            }
            Expression::ArrowFunctionExpression(arrow) => {
                // Walk arrow function body to detect mutations to outer variables.
                // Arrow functions capture from the outer scope, so mutations
                // inside the arrow function affect the outer scope's variables.
                for stmt in &arrow.body.statements {
                    self.walk_stmt_for_analysis(stmt);
                }
            }
            Expression::UpdateExpression(ue) => {
                // i++, i--, ++i, --i → mark the argument as mutated and reassigned
                let prefix = self
                    .current_fn
                    .as_deref()
                    .unwrap_or("__toplevel__")
                    .to_string();
                match &ue.argument {
                    SimpleAssignmentTarget::AssignmentTargetIdentifier(id) => {
                        self.mutated_vars.insert(format!("{}::{}", prefix, id.name));
                        self.reassigned_vars
                            .insert(format!("{}::{}", prefix, id.name));
                    }
                    SimpleAssignmentTarget::StaticMemberExpression(sme) => {
                        self.walk_expr_for_analysis(&sme.object);
                        if let Expression::Identifier(id) = &sme.object {
                            self.mutated_vars.insert(format!("{}::{}", prefix, id.name));
                        }
                    }
                    SimpleAssignmentTarget::ComputedMemberExpression(cme) => {
                        self.walk_expr_for_analysis(&cme.object);
                        self.walk_expr_for_analysis(&cme.expression);
                        if let Expression::Identifier(id) = &cme.object {
                            self.mutated_vars.insert(format!("{}::{}", prefix, id.name));
                        }
                    }
                    SimpleAssignmentTarget::PrivateFieldExpression(pfe) => {
                        self.walk_expr_for_analysis(&pfe.object);
                    }
                    _ => {}
                }
            }
            Expression::NewExpression(ne) => {
                self.walk_expr_for_analysis(&ne.callee);
                for arg in &ne.arguments {
                    if let Some(e) = arg.as_expression() {
                        self.walk_expr_for_analysis(e);
                    }
                }
            }
            Expression::SequenceExpression(se) => {
                for e in &se.expressions {
                    self.walk_expr_for_analysis(e);
                }
            }
            Expression::TemplateLiteral(tl) => {
                for e in &tl.expressions {
                    self.walk_expr_for_analysis(e);
                }
            }
            Expression::TaggedTemplateExpression(tte) => {
                self.walk_expr_for_analysis(&tte.tag);
                for e in &tte.quasi.expressions {
                    self.walk_expr_for_analysis(e);
                }
            }
            Expression::AwaitExpression(ae) => {
                self.walk_expr_for_analysis(&ae.argument);
            }
            Expression::ChainExpression(ce) => match &ce.expression {
                ChainElement::CallExpression(cce) => {
                    self.walk_expr_for_analysis(&cce.callee);
                    for arg in &cce.arguments {
                        if let Some(e) = arg.as_expression() {
                            self.walk_expr_for_analysis(e);
                        }
                    }
                }
                ChainElement::StaticMemberExpression(sme) => {
                    self.walk_expr_for_analysis(&sme.object);
                }
                ChainElement::ComputedMemberExpression(cme) => {
                    self.walk_expr_for_analysis(&cme.object);
                    self.walk_expr_for_analysis(&cme.expression);
                }
                _ => {}
            },
            Expression::PrivateFieldExpression(pfe) => {
                self.walk_expr_for_analysis(&pfe.object);
            }
            Expression::FunctionExpression(fe) => {
                // Walk function expression body to detect mutations to outer variables.
                if let Some(body) = &fe.body {
                    let saved_fn = std::mem::take(&mut self.current_fn);
                    self.current_fn = fe.id.as_ref().map(|id| id.name.to_string());
                    for stmt in &body.statements {
                        self.walk_stmt_for_analysis(stmt);
                    }
                    self.current_fn = saved_fn;
                }
            }
            _ => {}
        }
    }

    pub(crate) fn check_assignment_target(&mut self, target: &AssignmentTarget) {
        let prefix = self.current_fn.as_deref().unwrap_or("__toplevel__");
        match target {
            AssignmentTarget::AssignmentTargetIdentifier(id) => {
                self.mutated_vars.insert(format!("{}::{}", prefix, id.name));
                // Track direct variable reassignment (not property mutation)
                self.reassigned_vars
                    .insert(format!("{}::{}", prefix, id.name));
            }
            AssignmentTarget::StaticMemberExpression(mem) => {
                if let Expression::Identifier(id) = &mem.object {
                    self.mutated_vars.insert(format!("{}::{}", prefix, id.name));
                }
            }
            AssignmentTarget::ComputedMemberExpression(mem) => {
                if let Expression::Identifier(id) = &mem.object {
                    self.mutated_vars.insert(format!("{}::{}", prefix, id.name));
                }
            }
            _ => {}
        }
    }

    // ============================================================
    // Pass 1: collect used names (unused-constant elimination)
    // ============================================================

    pub(crate) fn collect_used_names(&mut self, program: &Program) {
        self.used_names.clear();
        let names = RefCell::new(HashSet::new());
        for stmt in &program.body {
            match stmt {
                Statement::FunctionDeclaration(fd) => {
                    Self::collect_idents_from_function(fd, &names);
                }
                Statement::ClassDeclaration(cd) => {
                    Self::collect_idents_from_class(cd, &names);
                }
                Statement::ExportNamedDeclaration(export_decl) => match &export_decl.declaration {
                    Some(Declaration::FunctionDeclaration(fd)) => {
                        Self::collect_idents_from_function(fd.as_ref(), &names);
                    }
                    Some(Declaration::ClassDeclaration(cd)) => {
                        Self::collect_idents_from_class(cd.as_ref(), &names);
                    }
                    _ => {}
                },
                Statement::ExportDefaultDeclaration(export_decl) => {
                    match &export_decl.declaration {
                        ExportDefaultDeclarationKind::FunctionDeclaration(fd) => {
                            Self::collect_idents_from_function(fd, &names);
                        }
                        ExportDefaultDeclarationKind::ClassDeclaration(cd) => {
                            Self::collect_idents_from_class(cd, &names);
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        self.used_names = names.into_inner();
    }

    /// Collect identifier references from class method bodies.
    /// Ensures top-level consts used only in class methods are not eliminated.
    fn collect_idents_from_class(class: &Class, names: &RefCell<HashSet<String>>) {
        for elem in &class.body.body {
            if let ClassElement::MethodDefinition(md) = elem
                && let Some(body) = &md.value.body
            {
                for stmt in &body.statements {
                    Self::collect_idents_from_stmt(stmt, names);
                }
            }
        }
    }

    pub(crate) fn collect_idents_from_function(fd: &Function, names: &RefCell<HashSet<String>>) {
        if let Some(body) = &fd.body {
            for stmt in &body.statements {
                Self::collect_idents_from_stmt(stmt, names);
            }
        }
    }

    pub(crate) fn collect_idents_from_stmt(stmt: &Statement, names: &RefCell<HashSet<String>>) {
        crate::infer::ast_walk::for_each_stmt_child(
            stmt,
            &mut |s| Self::collect_idents_from_stmt(s, names),
            &mut |e| Self::collect_idents_from_expr(e, names),
            &mut |vd| {
                crate::infer::ast_walk::for_each_var_decl_init(vd, &mut |init| {
                    Self::collect_idents_from_expr(init, names);
                });
            },
        );
    }

    pub(crate) fn collect_idents_from_expr(expr: &Expression, names: &RefCell<HashSet<String>>) {
        crate::infer::ast_walk::for_each_expr_child(
            expr,
            &mut |e| Self::collect_idents_from_expr(e, names),
            &mut |name| {
                names.borrow_mut().insert(name.to_string());
            },
            &mut |target| {
                // Assignment target identifiers count as "used"
                if let AssignmentTarget::AssignmentTargetIdentifier(id) = target {
                    names.borrow_mut().insert(id.name.to_string());
                }
            },
            &mut |_| {}, // on_simple_target: update targets handled via on_ident
            &mut |_, _| {
                // Function/arrow scope boundary — stop at function scope boundary
            },
        );
    }

    // ============================================================
    // Pass 2: collect variable types from ALL scopes
    // ============================================================

    pub(crate) fn walk_toplevel_for_types(&mut self, program: &Program) {
        for stmt in &program.body {
            self.walk_stmt_for_types(stmt);
        }
    }

    /// Walk a statement to collect variable types (no code generation).
    pub(crate) fn walk_stmt_for_types(&mut self, stmt: &Statement) {
        match stmt {
            // VariableDeclaration is handled after ClassDeclaration (see below)
            Statement::FunctionDeclaration(fd) => {
                self.walk_fn_for_types(
                    fd,
                    fd.id.as_ref().map(|id| id.name.as_str()).unwrap_or(""),
                    false,
                );
            }
            Statement::ExportNamedDeclaration(export_decl) => {
                // export function declarations are parsed as ExportNamedDeclaration
                // containing a Declaration::FunctionDeclaration
                if let Some(decl) = &export_decl.declaration {
                    match decl {
                        Declaration::FunctionDeclaration(fd) => {
                            let fn_name = fd.id.as_ref().map(|id| id.name.as_str()).unwrap_or("");
                            self.walk_fn_for_types(fd.as_ref(), fn_name, true);
                        }
                        Declaration::ClassDeclaration(cd) => {
                            if let Some(id) = &cd.id {
                                let class_name = id.name.to_string();
                                self.process_class_for_types(&class_name, cd.as_ref());
                            }
                        }
                        _ => {}
                    }
                }
            }
            Statement::ExportDefaultDeclaration(export_decl) => {
                // export default function/class declarations
                match &export_decl.declaration {
                    ExportDefaultDeclarationKind::FunctionDeclaration(fd) => {
                        let fn_name = fd
                            .id
                            .as_ref()
                            .map(|id| id.name.as_str())
                            .unwrap_or("default");
                        self.walk_fn_for_types(fd.as_ref(), fn_name, true);
                    }
                    ExportDefaultDeclarationKind::ClassDeclaration(cd) => {
                        if let Some(id) = &cd.id {
                            let class_name = id.name.to_string();
                            self.process_class_for_types(&class_name, cd.as_ref());
                        }
                    }
                    _ => {}
                }
            }
            Statement::IfStatement(is) => {
                self.walk_stmt_for_types(&is.consequent);
                if let Some(alt) = &is.alternate {
                    self.walk_stmt_for_types(alt);
                }
            }
            Statement::WhileStatement(ws) => {
                self.walk_stmt_for_types(&ws.body);
            }
            Statement::DoWhileStatement(dws) => {
                self.walk_stmt_for_types(&dws.body);
            }
            Statement::ForStatement(fs) => {
                self.walk_stmt_for_types(&fs.body);
                if let Some(ForStatementInit::VariableDeclaration(vd)) = &fs.init {
                    self.collect_var_types_from_decl(vd);
                }
            }
            Statement::ForOfStatement(fos) => {
                self.walk_stmt_for_types(&fos.body);
                // For-of loop variables have no initializer in the AST;
                // infer their type from the iterable expression.
                if let ForStatementLeft::VariableDeclaration(vd) = &fos.left {
                    // Try to get element type from the iterable
                    let elem_ty = match self.infer_expr_type(&fos.right) {
                        InferResult::Definite(ZigType::ArrayList(box_elem)) => *box_elem,
                        InferResult::Definite(ZigType::Str) => ZigType::Str,
                        _ => ZigType::JsAny,
                    };
                    for decl in &vd.declarations {
                        if let Some(name) = binding_name(&decl.id) {
                            self.var_types.insert(name.to_string(), elem_ty.clone());
                        }
                    }
                }
            }
            Statement::ForInStatement(fis) => {
                self.walk_stmt_for_types(&fis.body);
                // for-in loop variable is always a string (object property name).
                if let ForStatementLeft::VariableDeclaration(vd) = &fis.left {
                    for decl in &vd.declarations {
                        if let Some(name) = binding_name(&decl.id) {
                            self.var_types.insert(name.to_string(), ZigType::Str);
                        }
                    }
                }
            }
            Statement::TryStatement(ts) => {
                for s in &ts.block.body {
                    self.walk_stmt_for_types(s);
                }
                if let Some(handler) = &ts.handler {
                    // Register catch parameter in var_types as JsError
                    if let Some(param) = &handler.param {
                        if let BindingPattern::BindingIdentifier(id) = &param.pattern {
                            self.var_types.insert(id.name.to_string(), ZigType::JsError);
                        } else if let BindingPattern::AssignmentPattern(ap) = &param.pattern {
                            // catch(e = defaultValue) — extract name from left
                            if let BindingPattern::BindingIdentifier(id) = &ap.left {
                                self.var_types.insert(id.name.to_string(), ZigType::JsError);
                            }
                        }
                    }
                    for s in &handler.body.body {
                        self.walk_stmt_for_types(s);
                    }
                }
                if let Some(finalizer) = &ts.finalizer {
                    for s in &finalizer.body {
                        self.walk_stmt_for_types(s);
                    }
                }
            }
            Statement::SwitchStatement(ss) => {
                for case in &ss.cases {
                    for s in &case.consequent {
                        self.walk_stmt_for_types(s);
                    }
                }
            }
            Statement::BlockStatement(bs) => {
                for s in &bs.body {
                    self.walk_stmt_for_types(s);
                }
            }
            Statement::LabeledStatement(ls) => {
                // Recurse into labeled statement body (e.g. `outer: for(...) {...}`)
                self.walk_stmt_for_types(&ls.body);
            }
            Statement::ClassDeclaration(cd) => {
                // Register class name for type inference of `new ClassName()`
                if let Some(id) = &cd.id {
                    let class_name = id.name.to_string();
                    self.process_class_for_types(&class_name, cd.as_ref());
                }
            }
            Statement::VariableDeclaration(vd) => {
                // Handle class expressions: const X = class { ... }
                for decl in &vd.declarations {
                    if let Some(name) = binding_name(&decl.id)
                        && let Some(Expression::ClassExpression(ce)) = &decl.init
                    {
                        self.process_class_for_types(name, ce.as_ref());
                    }
                }
                self.collect_var_types_from_decl(vd);
            }
            _ => {}
        }
    }

    /// Collect type information for a class (declaration or expression).
    /// Registers class name, field types (from PropertyDefinition + constructor
    /// `this.x = expr` / `this.#x = expr`), and method return types.
    fn process_class_for_types(&mut self, class_name: &str, class: &Class) {
        self.class_names.insert(class_name.to_string());

        // Collect field types from PropertyDefinitions.
        // Use `PropertyKey::name()` instead of `static_name()` because
        // `static_name()` returns `None` for `PrivateIdentifier` (#field).
        let mut field_types: HashMap<String, ZigType> = HashMap::new();
        for elem in &class.body.body {
            if let ClassElement::PropertyDefinition(pd) = elem
                && let Some(field_name) = pd.key.name()
            {
                let field_ty = if let Some(init) = &pd.value {
                    match self.infer_expr_type(init) {
                        InferResult::Definite(ty) => ty,
                        InferResult::Indeterminate => ZigType::JsAny,
                    }
                } else {
                    ZigType::JsAny
                };
                field_types.insert(field_name.to_string(), field_ty);
            }
        }
        // Also scan constructor body for `this.x = expr` / `this.#x = expr`
        // assignments that implicitly declare or override fields.
        for elem in &class.body.body {
            if let ClassElement::MethodDefinition(md) = elem
                && md.key.name().as_deref() == Some("constructor")
                && let Some(body) = &md.value.body
            {
                self.collect_this_fields_from_body(&body.statements, &mut field_types);
                break;
            }
        }

        self.class_field_types
            .insert(class_name.to_string(), field_types);

        // Process class methods: infer return types
        let saved_class = self.current_class.clone();
        self.current_class = Some(class_name.to_string());
        for elem in &class.body.body {
            if let ClassElement::MethodDefinition(md) = elem {
                let method_name = md.key.name().map(|s| s.to_string()).unwrap_or_default();
                if method_name.is_empty() || method_name == "constructor" {
                    // constructor return type is always the class itself
                    if method_name == "constructor" {
                        self.fn_return_types.insert(
                            format!("{}.constructor", class_name),
                            ZigType::NamedStruct(class_name.to_string()),
                        );
                    }
                    // Still walk body for local type info
                    if let Some(body) = &md.value.body {
                        for s in &body.statements {
                            self.walk_stmt_for_types(s);
                        }
                    }
                    continue;
                }

                // Infer return type for regular class method
                let ret_ty = self.infer_class_method_return_type(md);
                match ret_ty {
                    InferResult::Definite(ty) => {
                        self.fn_return_types
                            .insert(format!("{}.{}", class_name, method_name), ty);
                    }
                    InferResult::Indeterminate => {
                        // Default to JsAny for methods that can't infer
                        self.fn_return_types
                            .insert(format!("{}.{}", class_name, method_name), ZigType::JsAny);
                    }
                }
            }
        }
        self.current_class = saved_class;
    }

    /// Recursively walk constructor body statements to find `this.x = expr`
    /// or `this.#x = expr` and collect field names + inferred types.
    fn collect_this_fields_from_body(
        &mut self,
        stmts: &[Statement],
        field_types: &mut HashMap<String, ZigType>,
    ) {
        for stmt in stmts {
            match stmt {
                Statement::ExpressionStatement(es) => {
                    if let Expression::AssignmentExpression(ae) = &es.expression {
                        let maybe_fname = match &ae.left {
                            // this.field = expr
                            AssignmentTarget::StaticMemberExpression(sme)
                                if matches!(&sme.object, Expression::ThisExpression(_)) =>
                            {
                                Some(sme.property.name.to_string())
                            }
                            // this.#field = expr
                            AssignmentTarget::PrivateFieldExpression(pfe)
                                if matches!(&pfe.object, Expression::ThisExpression(_)) =>
                            {
                                // PrivateIdentifier.name does NOT include the '#' prefix
                                Some(pfe.field.name.to_string())
                            }
                            _ => None,
                        };
                        if let Some(fname) = maybe_fname {
                            field_types.entry(fname).or_insert_with(|| {
                                match self.infer_expr_type(&ae.right) {
                                    InferResult::Definite(ty) => ty,
                                    InferResult::Indeterminate => ZigType::JsAny,
                                }
                            });
                        }
                    }
                }
                Statement::IfStatement(is) => {
                    self.collect_this_fields_from_body(
                        std::slice::from_ref(&is.consequent),
                        field_types,
                    );
                    if let Some(alt) = &is.alternate {
                        self.collect_this_fields_from_body(std::slice::from_ref(alt), field_types);
                    }
                }
                Statement::BlockStatement(bs) => {
                    self.collect_this_fields_from_body(&bs.body, field_types);
                }
                Statement::ForStatement(fs) => {
                    self.collect_this_fields_from_body(std::slice::from_ref(&fs.body), field_types);
                }
                Statement::ForOfStatement(fos) => {
                    self.collect_this_fields_from_body(
                        std::slice::from_ref(&fos.body),
                        field_types,
                    );
                }
                Statement::ForInStatement(fis) => {
                    self.collect_this_fields_from_body(
                        std::slice::from_ref(&fis.body),
                        field_types,
                    );
                }
                Statement::WhileStatement(ws) => {
                    self.collect_this_fields_from_body(std::slice::from_ref(&ws.body), field_types);
                }
                Statement::DoWhileStatement(dws) => {
                    self.collect_this_fields_from_body(
                        std::slice::from_ref(&dws.body),
                        field_types,
                    );
                }
                Statement::TryStatement(ts) => {
                    self.collect_this_fields_from_body(&ts.block.body, field_types);
                    if let Some(handler) = &ts.handler {
                        self.collect_this_fields_from_body(&handler.body.body, field_types);
                    }
                    if let Some(finalizer) = &ts.finalizer {
                        self.collect_this_fields_from_body(&finalizer.body, field_types);
                    }
                }
                Statement::SwitchStatement(ss) => {
                    for case in &ss.cases {
                        self.collect_this_fields_from_body(&case.consequent, field_types);
                    }
                }
                Statement::LabeledStatement(ls) => {
                    self.collect_this_fields_from_body(std::slice::from_ref(&ls.body), field_types);
                }
                _ => {}
            }
        }
    }

    pub(crate) fn collect_var_types_from_decl(&mut self, vd: &VariableDeclaration) {
        for decl in &vd.declarations {
            if let Some(name) = binding_name(&decl.id) {
                if let Some(init) = &decl.init {
                    // Skip class expressions — type info is collected in
                    // walk_stmt_for_types via process_class_for_types.
                    if matches!(init, Expression::ClassExpression(_)) {
                        continue;
                    }
                    // Check if this is JSON.parse(@type)
                    if let Some(type_name) = self.get_json_parse_type(name, init) {
                        self.has_json_parse_types.insert(name.to_string());
                        // Resolve the JSDoc type properly (handles arrays, named types, etc.)
                        let typedefs = self.jsdoc_data.as_ref().map(|d| &d.typedefs);
                        let zig_type = if let Some(td) = typedefs {
                            Self::jsdoc_str_to_zig_type(&type_name, td)
                        } else {
                            // No typedefs available — use direct mapping
                            if self.class_names.contains(&type_name)
                                || self.host_struct_fields.contains_key(&type_name)
                            {
                                ZigType::NamedStruct(type_name)
                            } else {
                                ZigType::JsAny
                            }
                        };
                        // Populate array_element_types for typed arrays (e.g., @type {User[]})
                        if let ZigType::ArrayList(elem_ty) = &zig_type {
                            self.array_element_types
                                .insert(name.to_string(), (**elem_ty).clone());
                        }
                        self.var_types.insert(name.to_string(), zig_type);
                        continue;
                    }

                    // Check JSDoc @type annotation for this variable
                    if let Some(ref jsdoc_data) = self.jsdoc_data
                        && let Some(ty_str) = jsdoc_data.type_annotations.get(name)
                    {
                        let zig_ty = Self::jsdoc_str_to_zig_type(ty_str, &jsdoc_data.typedefs);
                        // Populate array_element_types for typed arrays (e.g., @type {string[]})
                        if let ZigType::ArrayList(elem_ty) = &zig_ty {
                            self.array_element_types
                                .insert(name.to_string(), (**elem_ty).clone());
                        }
                        self.var_types.insert(name.to_string(), zig_ty);
                        continue;
                    }

                    let result = self.infer_expr_type(init);
                    match result {
                        InferResult::Definite(ty) => {
                            self.var_types.insert(name.to_string(), ty.clone());
                            if let ZigType::ArrayList(elem_ty) = &ty {
                                self.array_element_types
                                    .insert(name.to_string(), (**elem_ty).clone());
                            }
                        }
                        InferResult::Indeterminate => {
                            // Function expressions and arrow functions: assign JsAny
                            // (functions are objects in JS, callable via JsAny in Zig)
                            if matches!(
                                init,
                                Expression::FunctionExpression(_)
                                    | Expression::ArrowFunctionExpression(_)
                            ) {
                                self.var_types.insert(name.to_string(), ZigType::JsAny);
                            } else {
                                self.errors.push(format!(
                                    "Cannot infer type of variable '{}' (Rule 8). \
                                     Add a type annotation or initialize with a literal.",
                                    name
                                ));
                            }
                        }
                    }
                } else {
                    self.errors.push(format!(
                        "Variable '{}' must be initialized (strict type system)",
                        name
                    ));
                }
            }
        }
    }
}
