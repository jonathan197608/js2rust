// zigir/lower/mod.rs
// AST → ZigIR lowering: transforms JS AST + type info into structured IR.
//
// The Lowerer transforms JS AST + type-inference results into structured
// ZigIR. All semantic decisions (type resolution, name mangling, closure
// lowering) happen in sub-modules; the Emitter phase is pure formatting.
//
// Sub-module layout:
//   decl.rs    — variable / function / destructure declaration lowering
//   stmt.rs    — statement lowering (if/for/switch/try/loop/etc.)
//   class.rs   — class declaration + method lowering
//   expr.rs    — expression lowering (literals, operators, calls, etc.)
//   closure.rs — closure struct generation + capture analysis
//   cabi.rs    — C ABI export metadata + utility/query methods + free helpers
//   helpers.rs — FnContext struct + SetFlagGuard

pub mod cabi;
pub mod class;
pub mod closure;
pub mod decl;
pub mod expr;
pub mod helpers;
pub mod stmt;

use std::collections::{HashMap, HashSet};

use oxc_ast::ast::*;

use crate::infer::TypeCheckResult;
use crate::types::{ClosureManager, JSDocData};
use crate::zigir::ident::NameMangler;
use crate::zigir::source_span::{DiagnosticLevel, IrDiagnostic};
use crate::zigir::types::{IrDecl, IrModule, IrTypedef, IrTypedefField};

use helpers::FnContext;

// ═══════════════════════════════════════════════════════
//  Lowerer struct
// ═══════════════════════════════════════════════════════

/// AST → ZigIR lowering engine.
///
/// Composes 3 sub-structures:
/// - `name_mangler`: counter-per-prefix + shadow-scope stack
/// - `fn_ctx`:       per-function context
/// - `closure_mgr`:  closure struct collection
pub struct Lowerer {
    // ── Read-only inputs ──────────────────────────────
    /// Pre-computed type-inference results (read-only during lowering).
    pub(super) type_info: TypeCheckResult,
    /// JSDoc annotations (typedefs, type/return/param annotations).
    pub(super) jsdoc_data: JSDocData,
    /// Async host function names (for await lowering).
    pub(super) async_host_fns: HashSet<String>,
    /// Exported function names from pipeline.
    pub(super) exported_functions: Option<HashSet<String>>,
    /// Original JS source text (for diagnostics line:col).
    pub(super) source: String,
    /// Module name (derived from JS filename, e.g. "main" or "array_methods").
    pub(super) module_name: String,

    // ── Name management ───────────────────────────────
    pub(super) name_mangler: NameMangler,

    // ── Function context stack ────────────────────────
    pub(super) fn_ctx: Option<FnContext>,

    // ── Closure management ────────────────────────────
    pub(super) closure_mgr: ClosureManager,

    // ── Module-level tracking ─────────────────────────
    /// Known class names (used to route `new ClassName()` correctly).
    pub(super) class_names: HashSet<String>,

    /// Static field names per class: class_name → {field_name, ...}.
    /// Used to route `ClassName.field` access to `__ClassName_field` module-scope var.
    pub(super) class_static_fields: HashMap<String, HashSet<String>>,

    /// Class inheritance map: class_name → parent_class_name.
    /// Used for `instanceof` prototype chain traversal.
    pub(super) class_extends_map: HashMap<String, String>,

    // ── Deferred declarations ─────────────────────────
    /// Function definitions deferred from expression context (def-before-use).
    /// These are inserted before the statement that triggered them.
    pub(super) pending_expr_fns: Vec<IrDecl>,

    /// Arrow function / closure struct definitions collected during var decl lowering.
    /// These are moved to IrModule.closure_structs at the end of lowering.
    pub(super) pending_arrow_structs: Vec<crate::zigir::types::IrClosureStruct>,

    /// Pending loop label from a LabeledStatement parent.
    /// Consumed by the next loop statement (while/for/do-while/for-of/for-in).
    pub(super) pending_label: Option<String>,

    /// Currently inside a class (affects `this` lowering).
    pub(super) current_class: Option<String>,

    /// Currently inside a static block (affects `this` → ClassName rewriting).
    pub(super) in_static_block: bool,

    /// R8-C7: When set (during a constructor body), `this.field = value`
    /// statements are rewritten to `const field = value` so the Emitter can
    /// build the struct return. Propagates automatically into nested
    /// if/loop/switch/try bodies (they lower via `lower_stmt`).
    /// Reset to `None` inside nested function contexts (via `enter_fn`)
    /// because their `this` differs from the constructor's.
    pub(super) this_rewrite_fields: Option<Vec<String>>,

    /// C5: When true, `this` inside a closure/arrow function body should be
    /// lowered as `IrExpr::Ident("__self")` instead of `IrExpr::This`.
    /// Set during lowering of arrow functions / function expressions that
    /// capture `this` from their enclosing class method.
    pub(super) in_closure_with_this: bool,

    /// Whether we're lowering inside an ExpressionStatement.
    /// Affects UpdateExpression lowering (e.g., `i++` → `i += 1` vs block expr).
    pub(super) in_expr_stmt: bool,

    /// Counter for anonymous class expressions (to generate unique names).
    pub(super) anon_class_counter: u32,

    /// When lowering a class expression assigned to a variable (e.g. `const X = class { ... }`),
    /// this holds the variable name. The type inferrer stores field types under the variable
    /// name, but `lower_class_decl` generates `_AnonClass_N` for anonymous classes. This field
    /// allows field type lookups to fall back to the variable name.
    pub(super) class_expr_var_name: Option<String>,

    // ── Diagnostics ───────────────────────────────────
    pub(super) diagnostics: Vec<IrDiagnostic>,
}

// ═══════════════════════════════════════════════════════
//  Constructor
// ═══════════════════════════════════════════════════════

impl Lowerer {
    /// Create a new Lowerer.
    pub fn new(
        type_info: TypeCheckResult,
        jsdoc_data: JSDocData,
        exported_functions: Option<HashSet<String>>,
        async_host_fns: HashSet<String>,
        source: String,
        module_name: String,
    ) -> Self {
        Self {
            type_info,
            jsdoc_data,
            async_host_fns,
            exported_functions,
            source,
            module_name,
            name_mangler: NameMangler::new(),
            fn_ctx: None,
            closure_mgr: ClosureManager::new(),
            class_names: HashSet::new(),
            class_static_fields: HashMap::new(),
            class_extends_map: HashMap::new(),
            pending_expr_fns: Vec::new(),
            pending_arrow_structs: Vec::new(),
            pending_label: None,
            current_class: None,
            in_static_block: false,
            this_rewrite_fields: None,
            in_closure_with_this: false,
            in_expr_stmt: false,
            anon_class_counter: 0,
            class_expr_var_name: None,
            diagnostics: Vec::new(),
        }
    }
}

// ═══════════════════════════════════════════════════════
//  Main entry point
// ═══════════════════════════════════════════════════════

impl Lowerer {
    /// Lower a complete JS `Program` into an `IrModule`.
    pub fn lower(&mut self, program: &Program) -> IrModule {
        let module_name = self.module_name.clone();

        // 1. Lower JSDoc @typedef definitions
        let typedefs = self.lower_typedefs();

        // 2. Lower top-level declarations
        //    (Also collects class names, closure structs, etc.)
        let mut declarations = Vec::new();
        for stmt in &program.body {
            let mut stmt_decls = self.lower_toplevel(stmt);
            declarations.append(&mut stmt_decls);
        }

        // 3. Collect deferred closure structs
        let mut closure_structs = self.lower_closure_structs();
        // Also add arrow function struct definitions from var decl lowering
        closure_structs.append(&mut self.pending_arrow_structs);

        // 4. Build CABI exports metadata
        let cabi_exports = self.build_cabi_exports(&declarations);

        IrModule {
            name: module_name,
            typedefs,
            closure_structs,
            declarations,
            diagnostics: std::mem::take(&mut self.diagnostics),
            cabi_exports,
        }
    }
}

// ═══════════════════════════════════════════════════════
//  Typedef lowering
// ═══════════════════════════════════════════════════════

impl Lowerer {
    /// Lower JSDoc @typedef definitions into IrTypedef nodes.
    fn lower_typedefs(&self) -> Vec<IrTypedef> {
        let mut typedefs = Vec::new();
        for (name, td) in &self.jsdoc_data.typedefs {
            let fields: Vec<IrTypedefField> = td
                .fields
                .iter()
                .map(|f| {
                    let zig_type_str =
                        crate::jsdoc::jsdoc_type_to_zig(&f.ty, &self.jsdoc_data.typedefs);
                    IrTypedefField {
                        name: f.name.clone(),
                        zig_type: if f.optional {
                            format!("?{}", zig_type_str)
                        } else {
                            zig_type_str
                        },
                        optional: f.optional,
                    }
                })
                .collect();

            let is_opaque = fields.is_empty();
            typedefs.push(IrTypedef {
                name: name.clone(),
                fields,
                is_opaque,
                has_to_json: !is_opaque,
            });
        }
        typedefs
    }
}

// ═══════════════════════════════════════════════════════
//  Top-level dispatch
// ═══════════════════════════════════════════════════════

impl Lowerer {
    /// Lower a top-level statement into zero or more IrDecl nodes.
    fn lower_toplevel(&mut self, stmt: &Statement) -> Vec<IrDecl> {
        // Flush pending expression functions (def-before-use ordering)
        let mut decls = std::mem::take(&mut self.pending_expr_fns);

        match stmt {
            Statement::VariableDeclaration(vd) => {
                for decl in &vd.declarations {
                    decls.push(self.lower_var_decl(decl, vd.kind.is_const()));
                }
            }
            Statement::ClassDeclaration(cd) => {
                if cd.super_class.is_some() {
                    let span = self.span_to_source_span(cd.span);
                    self.add_error(
                        span.clone(),
                        "class extends is not supported: use composition instead",
                    );
                    decls.push(IrDecl::CompileError {
                        span,
                        msg: "class extends is not supported: use composition instead".to_string(),
                    });
                } else if let Some(ir_class) = self.lower_class_decl(cd) {
                    self.class_names.insert(ir_class.name.js_name.clone());
                    // Register extends relationship for instanceof chain traversal
                    if let Some(ref parent) = ir_class.extends {
                        self.class_extends_map
                            .insert(ir_class.name.js_name.clone(), parent.clone());
                    }
                    decls.push(IrDecl::Class(ir_class));
                }
            }
            Statement::FunctionDeclaration(fd) => {
                let is_export = self.is_export_fn(fd.id.as_ref().map(|id| id.name.as_str()));
                if let Some(ir_fn) = self.lower_fn_decl(fd, is_export) {
                    decls.push(IrDecl::Fn(ir_fn));
                }
            }
            Statement::ExportNamedDeclaration(export_decl) => {
                if let Some(decl) = &export_decl.declaration {
                    match decl {
                        Declaration::FunctionDeclaration(fd) => {
                            let is_export =
                                self.is_export_fn(fd.id.as_ref().map(|id| id.name.as_str()));
                            if let Some(ir_fn) = self.lower_fn_decl(fd, is_export) {
                                decls.push(IrDecl::Fn(ir_fn));
                            }
                        }
                        Declaration::VariableDeclaration(vd) => {
                            for decl in &vd.declarations {
                                decls.push(self.lower_var_decl(decl, vd.kind.is_const()));
                            }
                        }
                        _ => { /* skip unsupported */ }
                    }
                }
            }
            Statement::WithStatement(ws) => {
                let span = self.span_to_source_span(ws.span);
                self.diagnostics.push(IrDiagnostic {
                    level: DiagnosticLevel::Error,
                    span: Some(span),
                    message: "with statement is not supported and deprecated in strict mode. Use explicit property access instead.".to_string(),
                });
            }
            _ => { /* skip */ }
        }

        // Any new pending_expr_fns generated during this statement
        // should be inserted before the NEXT statement's output.
        let pending = std::mem::take(&mut self.pending_expr_fns);
        decls.extend(pending);

        decls
    }
}

// ═══════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{JSDocData, ZigType};
    use std::collections::{HashMap, HashSet};

    fn empty_type_info() -> TypeCheckResult {
        empty_type_info_base()
    }

    fn make_jsdoc_data() -> JSDocData {
        JSDocData {
            typedefs: HashMap::new(),
            type_annotations: HashMap::new(),
            return_types: HashMap::new(),
            param_types: HashMap::new(),
        }
    }

    fn empty_type_info_base() -> TypeCheckResult {
        TypeCheckResult {
            var_types: HashMap::new(),
            array_element_types: HashMap::new(),
            fn_return_types: HashMap::new(),
            fn_param_types: HashMap::new(),
            mutated_vars: HashSet::new(),
            reassigned_vars: HashSet::new(),
            used_names: HashSet::new(),
            has_json_parse_types: HashSet::new(),
            errors: Vec::new(),
            is_async: HashMap::new(),
            class_field_types: HashMap::new(),
            host_return_types: HashMap::new(),
            functions_needing_synthetic_rest: HashSet::new(),
        }
    }

    #[test]
    fn test_lowerer_new() {
        let type_info = empty_type_info();
        let lowerer = Lowerer::new(
            type_info,
            make_jsdoc_data(),
            None,
            HashSet::new(),
            "let x = 1;".to_string(),
            String::from("test"),
        );
        assert!(lowerer.fn_ctx.is_none());
        assert!(lowerer.class_names.is_empty());
        assert!(lowerer.diagnostics.is_empty());
    }

    #[test]
    fn test_lowerer_empty_program() {
        let type_info = empty_type_info();
        let mut lowerer = Lowerer::new(
            type_info,
            make_jsdoc_data(),
            None,
            HashSet::new(),
            String::new(),
            String::from("test"),
        );

        // Parse an empty program
        let js = "";
        let allocator = oxc_allocator::Allocator::default();
        let source_type = SourceType::default();
        let parser = oxc_parser::Parser::new(&allocator, js, source_type);
        let result = parser.parse();
        let module = lowerer.lower(&result.program);

        assert_eq!(module.name, "test");
        assert!(module.declarations.is_empty());
        assert!(module.closure_structs.is_empty());
        assert!(module.diagnostics.is_empty());
    }

    #[test]
    fn test_fn_context_enter_exit() {
        let type_info = empty_type_info();
        let mut lowerer = Lowerer::new(
            type_info,
            make_jsdoc_data(),
            None,
            HashSet::new(),
            String::new(),
            String::from("test"),
        );

        // Enter a function
        let saved = lowerer.enter_fn("foo", true, Some(ZigType::I64));
        assert!(saved.is_none()); // No previous context
        assert!(lowerer.fn_ctx.is_some());

        let ctx = lowerer.fn_ctx.as_ref().unwrap();
        assert_eq!(ctx.name, "foo");
        assert!(ctx.is_export);

        // Exit and check returned context
        let returned = lowerer.exit_fn(None);
        assert_eq!(returned.name, "foo");
        assert!(lowerer.fn_ctx.is_none());
    }

    #[test]
    fn test_fn_context_nesting() {
        let type_info = empty_type_info();
        let mut lowerer = Lowerer::new(
            type_info,
            make_jsdoc_data(),
            None,
            HashSet::new(),
            String::new(),
            String::from("test"),
        );

        // Enter outer function
        let saved_outer = lowerer.enter_fn("outer", true, Some(ZigType::Void));
        assert!(saved_outer.is_none());

        // Enter inner function (nested)
        let saved_inner = lowerer.enter_fn("inner", false, Some(ZigType::I64));
        assert!(saved_inner.is_some()); // Previous context saved
        assert_eq!(saved_inner.as_ref().unwrap().name, "outer");

        // Exit inner
        let inner_ctx = lowerer.exit_fn(saved_inner);
        assert_eq!(inner_ctx.name, "inner");

        // Outer context restored
        assert!(lowerer.fn_ctx.is_some());
        assert_eq!(lowerer.fn_ctx.as_ref().unwrap().name, "outer");

        // Exit outer
        let outer_ctx = lowerer.exit_fn(saved_outer);
        assert_eq!(outer_ctx.name, "outer");
        assert!(lowerer.fn_ctx.is_none());
    }

    #[test]
    fn test_is_export_fn() {
        let type_info = empty_type_info();
        let mut exported = HashSet::new();
        exported.insert("greet".to_string());

        let lowerer = Lowerer::new(
            type_info,
            make_jsdoc_data(),
            Some(exported),
            HashSet::new(),
            String::new(),
            String::from("test"),
        );

        assert!(lowerer.is_export_fn(Some("greet")));
        assert!(!lowerer.is_export_fn(Some("helper")));
        assert!(!lowerer.is_export_fn(None));
    }
}
