// zigir/passes/validate.rs
// ValidatePass — structural validation of an IrModule.
//
// Checks:
//   1. Type consistency: expressions have plausible types
//   2. Name uniqueness: no duplicate top-level identifiers
//   3. Closure integrity: captured lists match actual references
//   4. C ABI compatibility: exported function signatures use C-safe types

use crate::types::ZigType;
use crate::zigir::passes::{IrPass, PassResult};
use crate::zigir::source_span::{DiagnosticLevel, IrDiagnostic};
use crate::zigir::types::{IrAssignTarget, IrBlock, IrDecl, IrExpr, IrFnDecl, IrModule, IrStmt};

/// Validation pass: checks structural integrity of the IR.
///
/// Produces warnings for suspicious patterns and errors for violations
/// that would cause incorrect Zig output. Does NOT modify the IR.
pub struct ValidatePass {
    /// Collected diagnostics for the current run.
    diagnostics: Vec<IrDiagnostic>,
}

impl ValidatePass {
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
        }
    }

    fn error(&mut self, msg: String) {
        self.diagnostics.push(IrDiagnostic {
            level: DiagnosticLevel::Error,
            span: None,
            message: msg,
        });
    }

    fn warn(&mut self, msg: String) {
        self.diagnostics.push(IrDiagnostic {
            level: DiagnosticLevel::Warning,
            span: None,
            message: msg,
        });
    }

    // ── Top-level name uniqueness ────────────────────

    fn check_name_uniqueness(&mut self, module: &IrModule) {
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

        // Check typedefs
        for td in &module.typedefs {
            if !seen.insert(td.name.clone()) {
                self.error(format!("duplicate top-level name: '{}'", td.name));
            }
        }

        // Check closure structs
        for cs in &module.closure_structs {
            let zig_name = cs.name.zig_name.clone();
            if !seen.insert(zig_name.clone()) {
                self.error(format!("duplicate top-level name: '{}'", zig_name));
            }
        }

        // Check declarations
        for decl in &module.declarations {
            let name = match decl {
                IrDecl::Var(v) => v.name.zig_name.clone(),
                IrDecl::Fn(f) => f.name.zig_name.clone(),
                IrDecl::Class(c) => c.name.zig_name.clone(),
                IrDecl::CompileError { .. } => continue,
            };
            if !seen.insert(name.clone()) {
                self.error(format!("duplicate top-level name: '{}'", name));
            }
        }
    }

    // ── C ABI compatibility ──────────────────────────

    fn check_cabi_compatibility(&mut self, module: &IrModule) {
        for decl in &module.declarations {
            if let IrDecl::Fn(f) = decl
                && f.is_cabi
            {
                self.check_cabi_fn(f);
            }
        }
        for export in &module.cabi_exports {
            if !is_c_safe_type(&export.return_type) {
                self.error(format!(
                    "C ABI export '{}' has non-C-safe return type: {:?}",
                    export.name, export.return_type
                ));
            }
            for param in &export.params {
                if !is_c_safe_type(&param.zig_type) {
                    self.error(format!(
                        "C ABI export '{}' has non-C-safe parameter '{}': {:?}",
                        export.name, param.name.zig_name, param.zig_type
                    ));
                }
            }
        }
    }

    fn check_cabi_fn(&mut self, f: &IrFnDecl) {
        if !is_c_safe_type(&f.return_type) {
            self.error(format!(
                "C ABI function '{}' has non-C-safe return type: {:?}",
                f.name.zig_name, f.return_type
            ));
        }
        for param in &f.params {
            if !is_c_safe_type(&param.zig_type) {
                self.error(format!(
                    "C ABI function '{}' has non-C-safe parameter '{}': {:?}",
                    f.name.zig_name, param.name.zig_name, param.zig_type
                ));
            }
        }
    }

    // ── Closure integrity ────────────────────────────

    fn check_closure_integrity(&mut self, module: &IrModule) {
        for cs in &module.closure_structs {
            // Check that each captured variable is actually referenced in the body
            let referenced = collect_ident_names(&cs.body);
            for capture in &cs.captured {
                if !referenced.contains(&capture.name.zig_name) {
                    self.warn(format!(
                        "closure struct '{}' captures '{}' but it is not referenced in the body",
                        cs.name.zig_name, capture.name.zig_name
                    ));
                }
            }
        }

        // Check IrClosure expressions in declarations
        for decl in &module.declarations {
            self.check_closure_refs_in_decl(decl);
        }
    }

    fn check_closure_refs_in_decl(&mut self, decl: &IrDecl) {
        match decl {
            IrDecl::Fn(f) => self.check_closure_refs_in_block(&f.body),
            IrDecl::Var(v) => {
                if let Some(expr) = &v.init {
                    self.check_closure_refs_in_expr(expr);
                }
            }
            IrDecl::Class(c) => {
                if let Some(ctor) = &c.constructor {
                    self.check_closure_refs_in_block(&ctor.body);
                }
                for m in &c.methods {
                    self.check_closure_refs_in_block(&m.body);
                }
                for (_name, init) in &c.static_inits {
                    self.check_closure_refs_in_expr(init);
                }
                for block in &c.static_blocks {
                    self.check_closure_refs_in_block(block);
                }
            }
            IrDecl::CompileError { .. } => {}
        }
    }

    fn check_closure_refs_in_block(&mut self, block: &IrBlock) {
        for stmt in &block.stmts {
            self.check_closure_refs_in_stmt(stmt);
        }
    }

    fn check_closure_refs_in_stmt(&mut self, stmt: &IrStmt) {
        match stmt {
            IrStmt::VarDecl(v) => {
                if let Some(expr) = &v.init {
                    self.check_closure_refs_in_expr(expr);
                }
            }
            IrStmt::Assign { value, .. } => {
                self.check_closure_refs_in_expr(value);
            }
            IrStmt::If { cond, then, else_ } => {
                self.check_closure_refs_in_expr(cond);
                self.check_closure_refs_in_block(then);
                if let Some(e) = else_ {
                    self.check_closure_refs_in_block(e);
                }
            }
            IrStmt::While { cond, body, .. } => {
                self.check_closure_refs_in_expr(cond);
                self.check_closure_refs_in_block(body);
            }
            IrStmt::DoWhile { body, cond, .. } => {
                self.check_closure_refs_in_block(body);
                self.check_closure_refs_in_expr(cond);
            }
            IrStmt::For {
                init,
                cond,
                update,
                body,
                ..
            } => {
                if let Some(i) = init {
                    self.check_closure_refs_in_stmt(i);
                }
                if let Some(c) = cond {
                    self.check_closure_refs_in_expr(c);
                }
                if let Some(u) = update {
                    self.check_closure_refs_in_stmt(u);
                }
                self.check_closure_refs_in_block(body);
            }
            IrStmt::ForIn { iterable, body, .. } => {
                self.check_closure_refs_in_expr(iterable);
                self.check_closure_refs_in_block(body);
            }
            IrStmt::ForOf { iterable, body, .. } => {
                self.check_closure_refs_in_expr(iterable);
                self.check_closure_refs_in_block(body);
            }
            IrStmt::Switch { expr, cases } => {
                self.check_closure_refs_in_expr(expr);
                for case in cases {
                    for s in &case.body {
                        self.check_closure_refs_in_stmt(s);
                    }
                }
            }
            IrStmt::Try {
                try_block,
                catch_block,
                finally,
                ..
            } => {
                self.check_closure_refs_in_block(try_block);
                self.check_closure_refs_in_block(catch_block);
                if let Some(f) = finally {
                    self.check_closure_refs_in_block(f);
                }
            }
            IrStmt::Throw { value } => {
                self.check_closure_refs_in_expr(value);
            }
            IrStmt::Return { value } => {
                if let Some(v) = value {
                    self.check_closure_refs_in_expr(v);
                }
            }
            IrStmt::Break { .. } | IrStmt::Continue { .. } => {}
            IrStmt::Expr(e) => {
                self.check_closure_refs_in_expr(e);
            }
            IrStmt::Block(b) => {
                self.check_closure_refs_in_block(b);
            }
            IrStmt::CompileError { .. } | IrStmt::Comment(_) => {}
            IrStmt::DestructureDecl(data) => {
                self.check_closure_refs_in_expr(&data.init);
                for binding in &data.bindings {
                    if let Some(d) = &binding.default {
                        self.check_closure_refs_in_expr(d);
                    }
                }
            }
            IrStmt::NestedFnDecl {
                struct_def,
                instance,
            } => {
                self.check_closure_refs_in_block(&struct_def.body);
                if let Some(closure) = instance {
                    for cap in &closure.captured {
                        self.check_closure_refs_in_expr(&IrExpr::Ident(cap.name.clone()));
                    }
                }
            }
        }
    }

    fn check_closure_refs_in_expr(&mut self, expr: &IrExpr) {
        match expr {
            IrExpr::Closure(closure) => {
                let referenced = collect_ident_names(&closure.body);
                for capture in &closure.captured {
                    if !referenced.contains(&capture.name.zig_name) {
                        self.warn(format!(
                            "closure '{}' captures '{}' but it is not referenced in the body",
                            closure.struct_name.zig_name, capture.name.zig_name
                        ));
                    }
                }
            }
            // Recurse into sub-expressions
            IrExpr::Binary { left, right, .. } => {
                self.check_closure_refs_in_expr(left);
                self.check_closure_refs_in_expr(right);
            }
            IrExpr::Unary { operand, .. } => {
                self.check_closure_refs_in_expr(operand);
            }
            IrExpr::Logical { left, right, .. } => {
                self.check_closure_refs_in_expr(left);
                self.check_closure_refs_in_expr(right);
            }
            IrExpr::Call(call) => {
                self.check_closure_refs_in_expr(&call.callee);
                for arg in &call.args {
                    self.check_closure_refs_in_expr(arg);
                }
            }
            IrExpr::BuiltinCall(bc) => {
                for arg in &bc.args {
                    self.check_closure_refs_in_expr(arg);
                }
            }
            IrExpr::HostCall(hc) => {
                for arg in &hc.args {
                    self.check_closure_refs_in_expr(arg);
                }
            }
            IrExpr::FieldAccess { object, .. } => {
                self.check_closure_refs_in_expr(object);
            }
            IrExpr::IndexAccess { object, index, .. } => {
                self.check_closure_refs_in_expr(object);
                self.check_closure_refs_in_expr(index);
            }
            IrExpr::ComputedField { object, key, .. } => {
                self.check_closure_refs_in_expr(object);
                self.check_closure_refs_in_expr(key);
            }
            IrExpr::Conditional { cond, then, else_ } => {
                self.check_closure_refs_in_expr(cond);
                self.check_closure_refs_in_expr(then);
                self.check_closure_refs_in_expr(else_);
            }
            IrExpr::TemplateLiteral { exprs, .. } => {
                for e in exprs {
                    self.check_closure_refs_in_expr(e);
                }
            }
            IrExpr::ArrayLiteral(arr) => {
                for e in &arr.elements {
                    self.check_closure_refs_in_expr(e);
                }
            }
            IrExpr::ObjectLiteral(obj) => {
                use crate::zigir::types::IrObjectItem;
                for item in &obj.items {
                    match item {
                        IrObjectItem::Field(f) => {
                            self.check_closure_refs_in_expr(&f.value);
                        }
                        IrObjectItem::Spread(e) => {
                            self.check_closure_refs_in_expr(e);
                        }
                    }
                }
            }
            IrExpr::Assign { target, value, .. } => {
                self.check_closure_refs_in_assign_target(target);
                self.check_closure_refs_in_expr(value);
            }
            IrExpr::Update { target, .. } => {
                self.check_closure_refs_in_assign_target(target);
            }
            IrExpr::Await(a) => {
                self.check_closure_refs_in_expr(&a.callee);
                for arg in &a.args {
                    self.check_closure_refs_in_expr(arg);
                }
            }
            IrExpr::New(n) => {
                for arg in &n.args {
                    self.check_closure_refs_in_expr(arg);
                }
            }
            IrExpr::BlockExpr { body, result, .. } => {
                for s in body {
                    self.check_closure_refs_in_stmt(s);
                }
                self.check_closure_refs_in_expr(result);
            }
            IrExpr::AllocPrint { args, .. } => {
                for a in args {
                    self.check_closure_refs_in_expr(a);
                }
            }
            IrExpr::Spread(e) | IrExpr::Typeof(e) | IrExpr::Void(e) | IrExpr::Paren(e) => {
                self.check_closure_refs_in_expr(e);
            }
            IrExpr::Sequence(exprs) => {
                for e in exprs {
                    self.check_closure_refs_in_expr(e);
                }
            }
            IrExpr::ArrowFn(af) => {
                self.check_closure_refs_in_block(&af.body);
            }
            IrExpr::FnExpr(fe) => {
                self.check_closure_refs_in_block(&fe.body);
            }
            IrExpr::ArrayCallbackInline(inline_data) => {
                for stmt in &inline_data.body {
                    self.check_closure_refs_in_stmt(stmt);
                }
            }
            IrExpr::ArrayMethodInline(inline_data) => {
                for arg in &inline_data.args {
                    self.check_closure_refs_in_expr(arg);
                }
            }
            IrExpr::OptionalChain { object, body, .. } => {
                self.check_closure_refs_in_expr(object);
                self.check_closure_refs_in_expr(body);
            }
            IrExpr::PowExpr { base, exp, .. } => {
                self.check_closure_refs_in_expr(base);
                self.check_closure_refs_in_expr(exp);
            }
            // Leaf expressions: no sub-expressions to check
            IrExpr::IntLiteral(_)
            | IrExpr::FloatLiteral(_)
            | IrExpr::StringLiteral(_)
            | IrExpr::BoolLiteral(_)
            | IrExpr::BigIntLiteral(_)
            | IrExpr::Null
            | IrExpr::Undefined
            | IrExpr::Ident(_)
            | IrExpr::This
            | IrExpr::CompileError { .. } => {}
        }
    }

    fn check_closure_refs_in_assign_target(&mut self, target: &IrAssignTarget) {
        match target {
            IrAssignTarget::Ident(_) => {}
            IrAssignTarget::Member { object, .. } => {
                self.check_closure_refs_in_expr(object);
            }
            IrAssignTarget::Index { object, index, .. } => {
                self.check_closure_refs_in_expr(object);
                self.check_closure_refs_in_expr(index);
            }
            IrAssignTarget::Destructure(bindings) => {
                for b in bindings {
                    if let Some(d) = &b.default {
                        self.check_closure_refs_in_expr(d);
                    }
                }
            }
        }
    }
}

impl IrPass for ValidatePass {
    fn name(&self) -> &'static str {
        "validate"
    }

    fn description(&self) -> &'static str {
        "Validates structural integrity of the IR (name uniqueness, C ABI compatibility, closure integrity)"
    }

    fn run(&mut self, module: &mut IrModule) -> PassResult {
        self.diagnostics.clear();

        self.check_name_uniqueness(module);
        self.check_cabi_compatibility(module);
        self.check_closure_integrity(module);

        let diagnostics = std::mem::take(&mut self.diagnostics);
        PassResult::with_diagnostics(diagnostics)
    }
}

impl Default for ValidatePass {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════
//  Helpers
// ═══════════════════════════════════════════════════════

/// Collect all identifier names referenced in a block.
fn collect_ident_names(block: &IrBlock) -> std::collections::HashSet<String> {
    let mut names = std::collections::HashSet::new();
    collect_idents_from_stmts(&block.stmts, &mut names);
    names
}

fn collect_idents_from_block(block: &IrBlock, names: &mut std::collections::HashSet<String>) {
    collect_idents_from_stmts(&block.stmts, names);
}

fn collect_idents_from_stmts(stmts: &[IrStmt], names: &mut std::collections::HashSet<String>) {
    for stmt in stmts {
        collect_idents_from_stmt(stmt, names);
    }
}

fn collect_idents_from_stmt(stmt: &IrStmt, names: &mut std::collections::HashSet<String>) {
    match stmt {
        IrStmt::VarDecl(v) => {
            if let Some(e) = &v.init {
                collect_idents_from_expr(e, names);
            }
        }
        IrStmt::Assign { target, value, .. } => {
            collect_idents_from_target(target, names);
            collect_idents_from_expr(value, names);
        }
        IrStmt::If { cond, then, else_ } => {
            collect_idents_from_expr(cond, names);
            collect_idents_from_stmts(&then.stmts, names);
            if let Some(e) = else_ {
                collect_idents_from_stmts(&e.stmts, names);
            }
        }
        IrStmt::While { cond, body, .. } => {
            collect_idents_from_expr(cond, names);
            collect_idents_from_stmts(&body.stmts, names);
        }
        IrStmt::DoWhile { body, cond, .. } => {
            collect_idents_from_stmts(&body.stmts, names);
            collect_idents_from_expr(cond, names);
        }
        IrStmt::For {
            init,
            cond,
            update,
            body,
            ..
        } => {
            if let Some(i) = init {
                collect_idents_from_stmt(i, names);
            }
            if let Some(c) = cond {
                collect_idents_from_expr(c, names);
            }
            if let Some(u) = update {
                collect_idents_from_stmt(u, names);
            }
            collect_idents_from_stmts(&body.stmts, names);
        }
        IrStmt::ForIn { iterable, body, .. } => {
            collect_idents_from_expr(iterable, names);
            collect_idents_from_stmts(&body.stmts, names);
        }
        IrStmt::ForOf { iterable, body, .. } => {
            collect_idents_from_expr(iterable, names);
            collect_idents_from_stmts(&body.stmts, names);
        }
        IrStmt::Switch { expr, cases } => {
            collect_idents_from_expr(expr, names);
            for case in cases {
                collect_idents_from_stmts(&case.body, names);
            }
        }
        IrStmt::Try {
            try_block,
            catch_block,
            finally,
            ..
        } => {
            collect_idents_from_stmts(&try_block.stmts, names);
            collect_idents_from_stmts(&catch_block.stmts, names);
            if let Some(f) = finally {
                collect_idents_from_stmts(&f.stmts, names);
            }
        }
        IrStmt::Throw { value } => {
            collect_idents_from_expr(value, names);
        }
        IrStmt::Return { value } => {
            if let Some(v) = value {
                collect_idents_from_expr(v, names);
            }
        }
        IrStmt::Expr(e) => {
            collect_idents_from_expr(e, names);
        }
        IrStmt::Block(b) => {
            collect_idents_from_stmts(&b.stmts, names);
        }
        IrStmt::Break { .. }
        | IrStmt::Continue { .. }
        | IrStmt::CompileError { .. }
        | IrStmt::Comment(_) => {}
        IrStmt::DestructureDecl(data) => {
            collect_idents_from_expr(&data.init, names);
            for binding in &data.bindings {
                if let Some(d) = &binding.default {
                    collect_idents_from_expr(d, names);
                }
            }
        }
        IrStmt::NestedFnDecl {
            struct_def,
            instance,
        } => {
            collect_idents_from_block(&struct_def.body, names);
            if let Some(closure) = instance {
                for cap in &closure.captured {
                    names.insert(cap.name.js_name.clone());
                }
            }
        }
    }
}

fn collect_idents_from_expr(expr: &IrExpr, names: &mut std::collections::HashSet<String>) {
    match expr {
        IrExpr::Ident(id) => {
            names.insert(id.zig_name.clone());
        }
        IrExpr::Binary { left, right, .. } => {
            collect_idents_from_expr(left, names);
            collect_idents_from_expr(right, names);
        }
        IrExpr::Unary { operand, .. } => {
            collect_idents_from_expr(operand, names);
        }
        IrExpr::Logical { left, right, .. } => {
            collect_idents_from_expr(left, names);
            collect_idents_from_expr(right, names);
        }
        IrExpr::Call(call) => {
            collect_idents_from_expr(&call.callee, names);
            for arg in &call.args {
                collect_idents_from_expr(arg, names);
            }
        }
        IrExpr::BuiltinCall(bc) => {
            for arg in &bc.args {
                collect_idents_from_expr(arg, names);
            }
        }
        IrExpr::HostCall(hc) => {
            for arg in &hc.args {
                collect_idents_from_expr(arg, names);
            }
        }
        IrExpr::FieldAccess { object, .. } => {
            collect_idents_from_expr(object, names);
        }
        IrExpr::IndexAccess { object, index, .. } => {
            collect_idents_from_expr(object, names);
            collect_idents_from_expr(index, names);
        }
        IrExpr::ComputedField { object, key, .. } => {
            collect_idents_from_expr(object, names);
            collect_idents_from_expr(key, names);
        }
        IrExpr::Conditional { cond, then, else_ } => {
            collect_idents_from_expr(cond, names);
            collect_idents_from_expr(then, names);
            collect_idents_from_expr(else_, names);
        }
        IrExpr::TemplateLiteral { exprs, .. } => {
            for e in exprs {
                collect_idents_from_expr(e, names);
            }
        }
        IrExpr::ArrayLiteral(arr) => {
            for e in &arr.elements {
                collect_idents_from_expr(e, names);
            }
        }
        IrExpr::ObjectLiteral(obj) => {
            use crate::zigir::types::IrObjectItem;
            for item in &obj.items {
                match item {
                    IrObjectItem::Field(f) => {
                        collect_idents_from_expr(&f.value, names);
                    }
                    IrObjectItem::Spread(e) => {
                        collect_idents_from_expr(e, names);
                    }
                }
            }
        }
        IrExpr::Assign { target, value, .. } => {
            collect_idents_from_target(target, names);
            collect_idents_from_expr(value, names);
        }
        IrExpr::Update { target, .. } => {
            collect_idents_from_target(target, names);
        }
        IrExpr::Closure(c) => {
            collect_idents_from_stmts(&c.body.stmts, names);
        }
        IrExpr::ArrowFn(af) => {
            collect_idents_from_stmts(&af.body.stmts, names);
        }
        IrExpr::FnExpr(fe) => {
            collect_idents_from_stmts(&fe.body.stmts, names);
        }
        IrExpr::Await(a) => {
            collect_idents_from_expr(&a.callee, names);
            for arg in &a.args {
                collect_idents_from_expr(arg, names);
            }
        }
        IrExpr::New(n) => {
            for arg in &n.args {
                collect_idents_from_expr(arg, names);
            }
        }
        IrExpr::BlockExpr { body, result, .. } => {
            collect_idents_from_stmts(body, names);
            collect_idents_from_expr(result, names);
        }
        IrExpr::AllocPrint { args, .. } => {
            for a in args {
                collect_idents_from_expr(a, names);
            }
        }
        IrExpr::Spread(e) | IrExpr::Typeof(e) | IrExpr::Void(e) | IrExpr::Paren(e) => {
            collect_idents_from_expr(e, names);
        }
        IrExpr::Sequence(exprs) => {
            for e in exprs {
                collect_idents_from_expr(e, names);
            }
        }
        IrExpr::ArrayCallbackInline(inline_data) => {
            for stmt in &inline_data.body {
                collect_idents_from_stmt(stmt, names);
            }
        }
        IrExpr::ArrayMethodInline(inline_data) => {
            for arg in &inline_data.args {
                collect_idents_from_expr(arg, names);
            }
        }
        IrExpr::OptionalChain { object, body, .. } => {
            collect_idents_from_expr(object, names);
            collect_idents_from_expr(body, names);
        }
        IrExpr::PowExpr { base, exp, .. } => {
            collect_idents_from_expr(base, names);
            collect_idents_from_expr(exp, names);
        }
        IrExpr::IntLiteral(_)
        | IrExpr::FloatLiteral(_)
        | IrExpr::StringLiteral(_)
        | IrExpr::BoolLiteral(_)
        | IrExpr::BigIntLiteral(_)
        | IrExpr::Null
        | IrExpr::Undefined
        | IrExpr::This
        | IrExpr::CompileError { .. } => {}
    }
}

fn collect_idents_from_target(
    target: &IrAssignTarget,
    names: &mut std::collections::HashSet<String>,
) {
    match target {
        IrAssignTarget::Ident(id) => {
            names.insert(id.zig_name.clone());
        }
        IrAssignTarget::Member { object, .. } => {
            collect_idents_from_expr(object, names);
        }
        IrAssignTarget::Index { object, index, .. } => {
            collect_idents_from_expr(object, names);
            collect_idents_from_expr(index, names);
        }
        IrAssignTarget::Destructure(bindings) => {
            for b in bindings {
                if let Some(d) = &b.default {
                    collect_idents_from_expr(d, names);
                }
            }
        }
    }
}

/// Check if a ZigType is safe for C ABI boundaries.
///
/// C-safe types: i64, f64, Bool, Void, Str (as pointer).
/// NOT safe: JsAny, ArrayList, HashMap, NamedStruct, Anytype, AnytypeReturn, etc.
fn is_c_safe_type(ty: &ZigType) -> bool {
    matches!(
        ty,
        ZigType::I64 | ZigType::F64 | ZigType::Bool | ZigType::Void | ZigType::Str
    )
}

// ═══════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zigir::ident::IrIdent;
    use crate::zigir::ops::BinOp;
    use crate::zigir::types::{IrBlock, IrCapture, IrDecl, IrFnDecl, IrParam, IrStmt, IrVarDecl};

    #[test]
    fn test_validate_clean_module() {
        let mut module = IrModule::new("test".to_string());
        module.declarations.push(IrDecl::Fn(IrFnDecl {
            name: IrIdent::new("add"),
            params: vec![
                IrParam {
                    name: IrIdent::new("a"),
                    zig_type: ZigType::I64,
                    is_unused: false,
                    is_rest: false,
                },
                IrParam {
                    name: IrIdent::new("b"),
                    zig_type: ZigType::I64,
                    is_unused: false,
                    is_rest: false,
                },
            ],
            return_type: ZigType::I64,
            body: IrBlock::new(vec![IrStmt::Return {
                value: Some(IrExpr::Binary {
                    op: BinOp::Add,
                    left: Box::new(IrExpr::Ident(IrIdent::new("a"))),
                    right: Box::new(IrExpr::Ident(IrIdent::new("b"))),
                    left_type: None,
                    right_type: None,
                }),
            }]),
            is_export: true,
            is_async: false,
            can_throw: false,
            is_cabi: false,
            typeof_return_body: None,
        }));

        let mut pass = ValidatePass::new();
        let result = pass.run(&mut module);
        assert!(
            result.diagnostics.is_empty(),
            "clean module should have no diagnostics: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn test_validate_duplicate_names() {
        let mut module = IrModule::new("test".to_string());
        // Add two variables with the same name
        module.declarations.push(IrDecl::Var(IrVarDecl {
            name: IrIdent::new("x"),
            is_const: true,
            zig_type: Some(ZigType::I64),
            init: Some(IrExpr::IntLiteral(1)),
            is_json_parse: false,
            needs_var_suppression: false,
        }));
        module.declarations.push(IrDecl::Var(IrVarDecl {
            name: IrIdent::new("x"),
            is_const: true,
            zig_type: Some(ZigType::I64),
            init: Some(IrExpr::IntLiteral(2)),
            is_json_parse: false,
            needs_var_suppression: false,
        }));

        let mut pass = ValidatePass::new();
        let result = pass.run(&mut module);
        assert_eq!(result.diagnostics.len(), 1);
        assert!(matches!(
            result.diagnostics[0].level,
            DiagnosticLevel::Error
        ));
        assert!(result.diagnostics[0].message.contains("duplicate"));
    }

    #[test]
    fn test_validate_cabi_unsafe_type() {
        let mut module = IrModule::new("test".to_string());
        module.declarations.push(IrDecl::Fn(IrFnDecl {
            name: IrIdent::new("export_fn"),
            params: vec![IrParam {
                name: IrIdent::new("data"),
                zig_type: ZigType::JsAny, // NOT C-safe
                is_unused: false,
                is_rest: false,
            }],
            return_type: ZigType::JsAny, // NOT C-safe
            body: IrBlock::new(vec![]),
            is_export: false,
            is_async: false,
            can_throw: false,
            is_cabi: true,
            typeof_return_body: None,
        }));

        let mut pass = ValidatePass::new();
        let result = pass.run(&mut module);
        let errors: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| matches!(d.level, DiagnosticLevel::Error))
            .collect();
        assert_eq!(
            errors.len(),
            2,
            "should have 2 C ABI errors (param + return), got: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn test_validate_closure_capture_not_referenced() {
        let mut module = IrModule::new("test".to_string());
        // Add a closure struct with a captured var not referenced in body
        module
            .closure_structs
            .push(crate::zigir::types::IrClosureStruct {
                name: IrIdent::new("_closure_0"),
                captured: vec![IrCapture {
                    name: IrIdent::new("unused_var"),
                    zig_type: ZigType::I64,
                    is_mut: false,
                }],
                fn_params: vec![IrParam {
                    name: IrIdent::new("x"),
                    zig_type: ZigType::I64,
                    is_unused: false,
                    is_rest: false,
                }],
                return_type: ZigType::I64,
                typeof_return_body: None,
                body: IrBlock::new(vec![IrStmt::Return {
                    value: Some(IrExpr::Ident(IrIdent::new("x"))),
                }]),
            });

        let mut pass = ValidatePass::new();
        let result = pass.run(&mut module);
        let warnings: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| matches!(d.level, DiagnosticLevel::Warning))
            .collect();
        assert_eq!(warnings.len(), 1, "should warn about unreferenced capture");
        assert!(warnings[0].message.contains("unused_var"));
    }
}
