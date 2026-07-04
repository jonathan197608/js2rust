// zigir/lower/mod.rs
// AST → ZigIR lowering: transforms JS AST + type info into structured IR.
//
// The Lowerer replaces the old Codegen's string-concat approach with
// structured IR construction.  All semantic decisions (type resolution,
// name mangling, closure lowering) happen here; the later Emitter phase
// is pure formatting.

pub mod helpers;

use std::collections::HashSet;

use oxc_ast::ast::*;

use crate::infer::TypeCheckResult;
use crate::types::{ClosureManager, JSDocData, ZigType};
use crate::zigir::builtins::BuiltinModule;
use crate::zigir::ident::{IrIdent, NameMangler};
use crate::zigir::kinds::{CallKind, ComputedKeyKind, FieldKind, IndexKind};
use crate::zigir::ops::{AssignOp, BinOp, LogicalOp, UnaOp, UpdateOp};
use crate::zigir::source_span::{DiagnosticLevel, IrDiagnostic, SourceSpan};
use crate::zigir::types::{
    IrBlock, IrCabiExport, IrDecl, IrForInKind, IrForOfKind, IrImport, IrModule, IrParam,
    IrTypedef, IrTypedefField, IrVarDecl,
};

use helpers::FnContext;

// ═══════════════════════════════════════════════════════
//  Lowerer struct
// ═══════════════════════════════════════════════════════

/// AST → ZigIR lowering engine.
///
/// Composes 3 sub-structures instead of the old Codegen's 30+ flat fields:
/// - `name_mangler`: counter-per-prefix + shadow-scope stack  (replaces 9 counters + shadow_renames)
/// - `fn_ctx`:       per-function context                      (replaces 6+ flags + per-fn sets)
/// - `closure_mgr`:  closure struct collection                 (replaces 4 closure fields)
pub struct Lowerer {
    // ── Read-only inputs ──────────────────────────────
    /// Pre-computed type-inference results (read-only during lowering).
    type_info: TypeCheckResult,
    /// JSDoc annotations (typedefs, type/return/param annotations).
    jsdoc_data: JSDocData,
    /// Async host function names (for await lowering).
    async_host_fns: HashSet<String>,
    /// Exported function names from pipeline.
    exported_functions: Option<HashSet<String>>,
    /// Original JS source text (for diagnostics line:col).
    source: String,

    // ── Name management ───────────────────────────────
    name_mangler: NameMangler,

    // ── Function context stack ────────────────────────
    fn_ctx: Option<FnContext>,

    // ── Closure management ────────────────────────────
    closure_mgr: ClosureManager,

    // ── Module-level tracking ─────────────────────────
    /// Known class names (used to route `new ClassName()` correctly).
    class_names: HashSet<String>,

    // ── Deferred declarations ─────────────────────────
    /// Function definitions deferred from expression context (def-before-use).
    /// These are inserted before the statement that triggered them.
    pending_expr_fns: Vec<IrDecl>,

    /// Arrow function / closure struct definitions collected during var decl lowering.
    /// These are moved to IrModule.closure_structs at the end of lowering.
    pending_arrow_structs: Vec<crate::zigir::types::IrClosureStruct>,

    /// Pending loop label from a LabeledStatement parent.
    /// Consumed by the next loop statement (while/for/do-while/for-of/for-in).
    pending_label: Option<String>,

    /// Currently inside a class (affects `this` lowering).
    current_class: Option<String>,

    /// Whether we're lowering inside an ExpressionStatement.
    /// Affects UpdateExpression lowering (e.g., `i++` → `i += 1` vs block expr).
    in_expr_stmt: bool,

    // ── Diagnostics ───────────────────────────────────
    diagnostics: Vec<IrDiagnostic>,
}

// ═══════════════════════════════════════════════════════
//  Constructor
// ═══════════════════════════════════════════════════════

impl Lowerer {
    /// Create a new Lowerer with the same inputs as the old Codegen.
    ///
    /// Mirrors `Codegen::new()` so the dual-track comparison can supply
    /// identical inputs to both paths.
    pub fn new(
        type_info: TypeCheckResult,
        jsdoc_data: JSDocData,
        exported_functions: Option<HashSet<String>>,
        async_host_fns: HashSet<String>,
        source: String,
    ) -> Self {
        Self {
            type_info,
            jsdoc_data,
            async_host_fns,
            exported_functions,
            source,
            name_mangler: NameMangler::new(),
            fn_ctx: None,
            closure_mgr: ClosureManager::new(),
            class_names: HashSet::new(),
            pending_expr_fns: Vec::new(),
            pending_arrow_structs: Vec::new(),
            pending_label: None,
            current_class: None,
            in_expr_stmt: false,
            diagnostics: Vec::new(),
        }
    }
}

// ═══════════════════════════════════════════════════════
//  Main entry point
// ═══════════════════════════════════════════════════════

impl Lowerer {
    /// Lower a complete JS `Program` into an `IrModule`.
    ///
    /// This is the Lowerer's primary API, corresponding to
    /// `Codegen::generate(program)`.
    pub fn lower(&mut self, program: &Program) -> IrModule {
        let module_name = String::from("main"); // TODO: derive from filename

        // 1. Infer required imports from type info
        let imports = self.infer_imports();

        // 2. Lower JSDoc @typedef definitions
        let typedefs = self.lower_typedefs();

        // 3. Lower top-level declarations
        //    (Also collects class names, closure structs, etc.)
        let mut declarations = Vec::new();
        for stmt in &program.body {
            let mut stmt_decls = self.lower_toplevel(stmt);
            declarations.append(&mut stmt_decls);
        }

        // 4. Collect deferred closure structs
        let mut closure_structs = self.lower_closure_structs();
        // Also add arrow function struct definitions from var decl lowering
        closure_structs.append(&mut self.pending_arrow_structs);

        // 5. Build CABI exports metadata
        let cabi_exports = self.build_cabi_exports(&declarations);

        IrModule {
            name: module_name,
            imports,
            typedefs,
            closure_structs,
            declarations,
            diagnostics: std::mem::take(&mut self.diagnostics),
            cabi_exports,
        }
    }
}

// ═══════════════════════════════════════════════════════
//  Import inference
// ═══════════════════════════════════════════════════════

impl Lowerer {
    /// Infer required Zig imports from type-info and source analysis.
    ///
    /// Replaces the ad-hoc `self.write("const ... = @import(...)")` calls
    /// scattered across the old Codegen (which doesn't write imports at all —
    /// they're added by `project.rs` templates unconditionally). The IR approach
    /// is more precise: only the actually-needed runtime modules are imported.
    ///
    /// Strategy:
    /// 1. Walk all `ZigType` values in `type_info` to detect which runtime
    ///    modules are needed (JsAny → jsany, JsSymbol → js_symbol, etc.)
    /// 2. Check structural heuristics (typedefs → std + js_allocator,
    ///    has_json_parse_types → std.json + js_json, async → host)
    /// 3. Collect into `IrImport` nodes grouped by module.
    fn infer_imports(&self) -> Vec<IrImport> {
        let mut imports = Vec::new();

        // ── Track which runtime modules are needed ─────────
        let mut needs_std = false;
        let mut needs_js_allocator = false;
        let mut needs_js_runtime = false;
        let mut needs_jsany = false;
        let mut runtime_modules: std::collections::BTreeSet<BuiltinModule> =
            std::collections::BTreeSet::new();

        // ── Heuristic: typedefs always need std (toJson) + js_allocator ──
        if !self.jsdoc_data.typedefs.is_empty() {
            needs_std = true;
            needs_js_allocator = true;
        }

        // ── Heuristic: has_json_parse_types → std.json + js_json ────────
        if !self.type_info.has_json_parse_types.is_empty() {
            needs_std = true;
            runtime_modules.insert(BuiltinModule::JsJson);
        }

        // ── Heuristic: async host functions → host import ───────────────
        // (host import is added by the pipeline layer, not here — but we
        //  note the need for js_allocator for async patterns)
        if !self.async_host_fns.is_empty() {
            needs_js_allocator = true;
        }

        // ── Walk all ZigType values to detect runtime needs ─────────────
        // Collect from: var_types, fn_return_types, fn_param_types, class_field_types,
        // array_element_types
        let mut all_types: Vec<&ZigType> = Vec::new();

        for ty in self.type_info.var_types.values() {
            all_types.push(ty);
        }
        for ty in self.type_info.fn_return_types.values() {
            all_types.push(ty);
        }
        for params in self.type_info.fn_param_types.values() {
            for (_name, ty) in params {
                all_types.push(ty);
            }
        }
        for fields in self.type_info.class_field_types.values() {
            for ty in fields.values() {
                all_types.push(ty);
            }
        }
        for inner in self.type_info.array_element_types.values() {
            all_types.push(inner);
        }

        for ty in &all_types {
            self.zigtype_needs_imports(
                ty,
                &mut needs_std,
                &mut needs_js_allocator,
                &mut needs_js_runtime,
                &mut needs_jsany,
                &mut runtime_modules,
            );
        }

        // ── Construct IrImport nodes ────────────────────────────────────

        // 1. std import
        if needs_std {
            let mut std_items: Vec<(String, String)> = Vec::new();

            // Always include ArrayList if any ArrayList type exists
            if self
                .type_info
                .var_types
                .values()
                .any(|t| matches!(t, ZigType::ArrayList(_)))
            {
                std_items.push(("ArrayList".to_string(), "ArrayList".to_string()));
            }

            // std.json needed for JSON.parse with typed struct or typedef toJson
            if !self.type_info.has_json_parse_types.is_empty()
                || !self.jsdoc_data.typedefs.is_empty()
            {
                // std.json is accessed as a nested module, not a named import
                // The Emitter will emit `const std = @import("std");` and
                // use `std.json.fmt()` / `std.json.parse()` directly.
            }

            imports.push(IrImport {
                module_name: "std".to_string(),
                items: std_items,
            });
        }

        // 2. js_allocator (global allocator for all runtime allocations)
        if needs_js_allocator {
            imports.push(IrImport {
                module_name: "js_runtime/js_allocator.zig".to_string(),
                items: vec![("js_allocator".to_string(), "js_allocator".to_string())],
            });
        }

        // 3. js_runtime umbrella (for jsTypeof, spreadMerge, js_typedarray, etc.)
        if needs_js_runtime {
            imports.push(IrImport {
                module_name: "js_runtime/js_runtime.zig".to_string(),
                items: vec![("js_runtime".to_string(), "js_runtime".to_string())],
            });
        }

        // 4. Per-module runtime imports
        for module in &runtime_modules {
            let module_path = module.module_path().to_string();

            // Determine the items to import from this module
            match module {
                BuiltinModule::JsMath => {
                    // std.math is accessed via `std` import, not a separate import
                    // Already handled by needs_std above
                }
                BuiltinModule::JsConsole => {
                    imports.push(IrImport {
                        module_name: format!("js_runtime/{}.zig", module_path),
                        items: vec![(module_path.clone(), module_path.clone())],
                    });
                }
                BuiltinModule::JsSymbol => {
                    // js_symbol module + JsSymbol type export
                    imports.push(IrImport {
                        module_name: format!("js_runtime/{}.zig", module_path),
                        items: vec![
                            (module_path.clone(), module_path.clone()),
                            ("JsSymbol".to_string(), "JsSymbol".to_string()),
                        ],
                    });
                }
                BuiltinModule::JsBigInt => {
                    imports.push(IrImport {
                        module_name: format!("js_runtime/{}.zig", module_path),
                        items: vec![(module_path.clone(), module_path.clone())],
                    });
                }
                BuiltinModule::JsTypedArray => {
                    // TypedArray is accessed via js_runtime.js_typedarray
                    // Ensure js_runtime is also imported
                    imports.push(IrImport {
                        module_name: "js_runtime/js_runtime.zig".to_string(),
                        items: vec![("js_runtime".to_string(), "js_runtime".to_string())],
                    });
                }
                _ => {
                    imports.push(IrImport {
                        module_name: format!("js_runtime/{}.zig", module_path),
                        items: vec![(module_path.clone(), module_path.clone())],
                    });
                }
            }
        }

        // 5. Special type imports that are needed based on ZigType analysis
        //    but not tied to a specific BuiltinModule
        // (JsAny import is tracked via needs_jsany flag in zigtype_needs_imports)

        if needs_jsany {
            imports.push(IrImport {
                module_name: "js_runtime/jsany.zig".to_string(),
                items: vec![("JsAny".to_string(), "JsAny".to_string())],
            });
        }

        // ── Deduplicate ─────────────────────────────────────────────────
        // Multiple triggers may request the same module_path (e.g. js_runtime).
        // Merge items by module_name, deduplicating items.
        imports = Self::deduplicate_imports(imports);

        imports
    }

    /// Merge imports with the same module_name, deduplicating their items.
    fn deduplicate_imports(imports: Vec<IrImport>) -> Vec<IrImport> {
        let mut map: std::collections::BTreeMap<String, Vec<(String, String)>> =
            std::collections::BTreeMap::new();
        for imp in imports {
            let entry = map.entry(imp.module_name).or_default();
            for item in imp.items {
                if !entry.contains(&item) {
                    entry.push(item);
                }
            }
        }
        map.into_iter()
            .map(|(module_name, items)| IrImport { module_name, items })
            .collect()
    }

    /// Recursively inspect a ZigType to determine which runtime imports it needs.
    ///
    /// Sets flags and inserts into `runtime_modules` as side effects.
    fn zigtype_needs_imports(
        &self,
        ty: &ZigType,
        needs_std: &mut bool,
        needs_js_allocator: &mut bool,
        needs_js_runtime: &mut bool,
        needs_jsany: &mut bool,
        runtime_modules: &mut std::collections::BTreeSet<BuiltinModule>,
    ) {
        match ty {
            ZigType::ArrayList(_) => {
                *needs_std = true;
            }
            ZigType::JsAny => {
                *needs_jsany = true;
                // JsAny comparisons involve js_runtime helpers
                *needs_js_runtime = true;
            }
            ZigType::JsSymbol => {
                runtime_modules.insert(BuiltinModule::JsSymbol);
            }
            ZigType::BigInt => {
                runtime_modules.insert(BuiltinModule::JsBigInt);
                *needs_js_allocator = true;
            }
            ZigType::NamedStruct(name) => {
                match name.as_str() {
                    "Map" | "Set" => {
                        runtime_modules.insert(BuiltinModule::JsCollections);
                        *needs_js_allocator = true;
                    }
                    "Date" | "JsDate" => {
                        runtime_modules.insert(BuiltinModule::JsDate);
                        *needs_js_allocator = true;
                    }
                    _ => {
                        // User-defined class or host struct — no additional import
                        // Host structs use `host.{StructName}`, added by pipeline.
                    }
                }
            }
            ZigType::Struct(fields) => {
                for (_, field_ty) in fields {
                    self.zigtype_needs_imports(
                        field_ty,
                        needs_std,
                        needs_js_allocator,
                        needs_js_runtime,
                        needs_jsany,
                        runtime_modules,
                    );
                }
            }
            // Primitive types and anytype don't need runtime imports
            ZigType::Void
            | ZigType::I64
            | ZigType::F64
            | ZigType::Bool
            | ZigType::Str
            | ZigType::Anytype
            | ZigType::AnytypeReturn => {}
        }
    }
}

// ═══════════════════════════════════════════════════════
//  Typedef lowering
// ═══════════════════════════════════════════════════════

impl Lowerer {
    /// Lower JSDoc @typedef definitions into IrTypedef nodes.
    ///
    /// All non-opaque typedefs get `has_to_json = true`, matching the old
    /// Codegen which unconditionally generates a `toJson()` method for
    /// each typedef struct.
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
    ///
    /// Corresponds to `Codegen::emit_toplevel()`.
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
                if let Some(ir_class) = self.lower_class_decl(cd) {
                    self.class_names.insert(ir_class.name.js_name.clone());
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
        // For IR we simply append them after the current decls
        // since IR declarations can be reordered later.
        let pending = std::mem::take(&mut self.pending_expr_fns);
        decls.extend(pending);

        decls
    }
}

// ═══════════════════════════════════════════════════════
//  Declaration lowering (stubs — filled in Tasks 1.2-1.3)
// ═══════════════════════════════════════════════════════

impl Lowerer {
    /// Lower a variable declaration.
    ///
    /// Translates JS `const`/`var`/`let` into `IrDecl::Var`. The Lowerer
    /// resolves semantic information (const vs var, type annotation, JSON.parse
    /// special case) and defers all formatting to the Emitter.
    ///
    /// Shadow renaming is NOT done here — it's a scope-level concern handled
    /// by the NameMangler during identifier resolution.
    fn lower_var_decl(&mut self, decl: &VariableDeclarator, _vd_is_const: bool) -> IrDecl {
        let name = match crate::infer::binding_name(&decl.id) {
            Some(n) => n,
            None => {
                return IrDecl::CompileError {
                    span: SourceSpan::default(),
                    msg: "unsupported binding pattern in variable declaration".to_string(),
                };
            }
        };

        let ident = self.make_ident(name);

        // Determine const vs var based on mutation analysis (not JS keyword).
        // Zig 'const' for never-mutated, 'var' for actually reassigned.
        let fn_prefix = self
            .fn_ctx
            .as_ref()
            .map(|ctx| ctx.name.as_str())
            .unwrap_or("__toplevel__");
        let is_const = !self
            .type_info
            .mutated_vars
            .contains(&format!("{}::{}", fn_prefix, name));

        // Skip unused toplevel constants (same logic as Codegen)
        let has_type_annotation = self.jsdoc_data.type_annotations.contains_key(name);
        if self.fn_ctx.is_none()
            && is_const
            && !self.type_info.used_names.contains(name)
            && !has_type_annotation
        {
            return IrDecl::CompileError {
                span: SourceSpan::default(),
                msg: format!("skipped unused toplevel const: {}", name),
            };
        }

        // Toplevel var/let → error (only const allowed at module level)
        if self.fn_ctx.is_none() && !is_const {
            return IrDecl::CompileError {
                span: SourceSpan::default(),
                msg: format!("toplevel only allows 'const', not '{}'", name),
            };
        }

        // Force 'var' for Map/Set/ArrayList types (mutated via methods)
        let is_const = if let Some(inferred_ty) = self.type_info.var_types.get(name) {
            match inferred_ty {
                ZigType::ArrayList(_) => false,
                ZigType::NamedStruct(n) if n == "Map" || n == "Set" => false,
                _ => is_const,
            }
        } else {
            is_const
        };

        // Type from inference
        let zig_type = self.type_info.var_types.get(name).cloned();

        // JSON.parse special case
        let is_json_parse = self.type_info.has_json_parse_types.contains(name);

        // Needs var suppression (ArrayList/Map/Set method calls need `_= &var;`)
        let needs_var_suppression = !is_const
            && matches!(
                zig_type,
                Some(ZigType::ArrayList(_)) | Some(ZigType::NamedStruct(_))
            );

        // Lower initializer expression
        let init = match decl.init.as_ref() {
            Some(expr) => {
                // Special case: arrow function / closure initializer.
                // Instead of returning IrExpr::ArrowFn as init, we:
                // 1. Register the struct definition in module.closure_structs
                // 2. Return IrExpr::Ident pointing to the struct name
                // This matches Codegen's output pattern:
                //   const _arrow_fn_0 = struct { ... };
                //   const double = _arrow_fn_0;
                let ir = self.lower_expr(expr);
                match ir {
                    crate::zigir::types::IrExpr::ArrowFn(ref arrow) => {
                        // Generate a unique struct name like _arrow_fn_0, _arrow_fn_1, ...
                        // Use name_mangler counter to get sequential numbering.
                        let idx = self.name_mangler.peek_count("arrow_fn");
                        self.name_mangler.next_name("arrow_fn"); // advance counter
                        let struct_name = format!("_arrow_fn_{}", idx);
                        let struct_ident = IrIdent::new(&struct_name);
                        // Register as closure struct (with empty captures)
                        self.pending_arrow_structs
                            .push(crate::zigir::types::IrClosureStruct {
                                name: struct_ident.clone(),
                                captured: vec![],
                                fn_params: arrow.params.clone(),
                                return_type: arrow.return_type.clone(),
                                body: arrow.body.clone(),
                            });
                        Some(crate::zigir::types::IrExpr::Ident(struct_ident))
                    }
                    crate::zigir::types::IrExpr::Closure(ref closure) => {
                        // Struct already registered by lower_fn_expr / lower_arrow_fn.
                        // Closure instance: StructName { .captured = val, ... }
                        Some(crate::zigir::types::IrExpr::Closure(closure.clone()))
                    }
                    _ => Some(ir),
                }
            }
            None => None,
        };

        IrDecl::Var(IrVarDecl {
            name: ident,
            is_const,
            zig_type,
            init,
            is_json_parse,
            needs_var_suppression,
        })
    }

    /// Lower a function declaration.
    ///
    /// Translates JS `function foo(a, b) { ... }` into `IrDecl::Fn`.
    /// Handles:
    /// - export / C ABI determination
    /// - async detection (from type_info.is_async)
    /// - throw/catch detection (pre-scan body)
    /// - parameter type resolution
    /// - return type resolution (including AnytypeReturn → @TypeOf)
    /// - shadow renaming for parameters
    fn lower_fn_decl(
        &mut self,
        fd: &Function,
        is_export: bool,
    ) -> Option<crate::zigir::types::IrFnDecl> {
        let name = fd.id.as_ref().map(|id| id.name.as_str())?;

        // Pre-scan: check if function contains throw or try-catch
        let has_throw = fd.body.as_ref().is_some_and(|b| Self::has_throw_in_body(b));

        // Check async from type_info
        let is_async = self.type_info.is_async.get(name).copied().unwrap_or(false);

        // Return type from inference
        let return_type = self
            .type_info
            .fn_return_types
            .get(name)
            .cloned()
            .unwrap_or(ZigType::Void);

        // Enter function context
        let saved = self.enter_fn(name, is_export, Some(return_type.clone()));

        // Lower parameters
        let mut params = self.lower_fn_params(fd, name);

        // Lower function body
        let body = fd
            .body
            .as_ref()
            .map(|b| self.lower_block(&b.statements))
            .unwrap_or_else(|| IrBlock::new(vec![]));

        // Mark unused parameters: collect all identifier references in the body,
        // then check which params don't appear. Also include identifiers from
        // compile-time-resolved expressions (e.g., typeof x → "number") that
        // were optimized away but still semantically reference the parameter.
        let mut used_idents = Self::collect_ir_idents_in_block(&body);
        if let Some(ctx) = self.fn_ctx.as_ref() {
            used_idents.extend(ctx.compile_time_referenced_idents.iter().cloned());
        }
        for param in &mut params {
            if !used_idents.contains(&param.name.js_name) {
                param.is_unused = true;
            }
        }

        // Exit function context
        let _fn_ctx = self.exit_fn(saved);

        // Determine C ABI: export functions use `export fn` calling convention
        let is_cabi = is_export;

        Some(crate::zigir::types::IrFnDecl {
            name: self.make_ident(name),
            params,
            return_type,
            body,
            is_export,
            is_async,
            can_throw: has_throw,
            is_cabi,
        })
    }

    /// Lower function parameters into IrParam list.
    ///
    /// Reads parameter types from type_info when available, falls back to
    /// anytype for untyped parameters.
    fn lower_fn_params(&mut self, fd: &Function, fn_name: &str) -> Vec<IrParam> {
        let mut params = Vec::new();

        // Try to get param types from type_info
        let param_types = self.type_info.fn_param_types.get(fn_name).cloned();

        if let Some(ptypes) = param_types {
            for (pname, ptype) in &ptypes {
                // Skip 'io' param for async functions (it's injected by the runtime)
                if self
                    .type_info
                    .is_async
                    .get(fn_name)
                    .copied()
                    .unwrap_or(false)
                    && pname == "io"
                {
                    continue;
                }
                params.push(IrParam {
                    name: self.make_ident(pname),
                    zig_type: ptype.clone(),
                    is_unused: false, // set later in lower_fn_decl
                });
            }
        } else {
            // Fallback: generate params from AST with anytype
            for param in &fd.params.items {
                if let Some(pname) = crate::infer::binding_name(&param.pattern) {
                    params.push(IrParam {
                        name: self.make_ident(pname),
                        zig_type: ZigType::Anytype,
                        is_unused: false, // set later in lower_fn_decl
                    });
                }
            }
        }

        // Handle rest parameter (...args) → []const JsAny
        if let Some(rname) = fd
            .params
            .rest
            .as_ref()
            .and_then(|r| crate::infer::binding_name(&r.rest.argument))
        {
            params.push(IrParam {
                name: self.make_ident(rname),
                zig_type: ZigType::Anytype, // Will be rendered as []const JsAny by Emitter
                is_unused: false,           // set later in lower_fn_decl
            });
        }

        params
    }

    /// Pre-scan: check if a function body contains `throw` or `try-catch`.
    ///
    /// This is needed to determine whether the return type should be an
    /// error union (`!T` vs `T`). Ported from Codegen's `has_throw_in_body()`.
    fn has_throw_in_body(body: &FunctionBody) -> bool {
        body.statements.iter().any(|s| Self::stmt_has_throw(s))
    }

    fn stmt_has_throw(stmt: &Statement) -> bool {
        match stmt {
            Statement::ThrowStatement(_) => true,
            Statement::BlockStatement(bs) => bs.body.iter().any(|s| Self::stmt_has_throw(s)),
            Statement::IfStatement(is) => {
                Self::stmt_has_throw(&is.consequent)
                    || is
                        .alternate
                        .as_ref()
                        .is_some_and(|a| Self::stmt_has_throw(a))
            }
            Statement::WhileStatement(ws) => Self::stmt_has_throw(&ws.body),
            Statement::DoWhileStatement(dws) => Self::stmt_has_throw(&dws.body),
            Statement::ForStatement(fs) => Self::stmt_has_throw(&fs.body),
            Statement::ForOfStatement(fos) => Self::stmt_has_throw(&fos.body),
            Statement::ForInStatement(fis) => Self::stmt_has_throw(&fis.body),
            Statement::TryStatement(_) => true, // try-catch implies potential throw
            Statement::SwitchStatement(ss) => ss
                .cases
                .iter()
                .any(|c| c.consequent.iter().any(|s| Self::stmt_has_throw(s))),
            _ => false,
        }
    }
}

// ═══════════════════════════════════════════════════════
//  Declaration lowering (remaining stubs)
// ═══════════════════════════════════════════════════════

impl Lowerer {
    /// Lower a statement into an IrStmt.
    ///
    /// This is the main dispatch method for statement-level AST → IR
    /// transformation. Each branch extracts semantic information and
    /// defers formatting (indentation, `_ = ` discard prefixes, etc.)
    /// to the Emitter phase.
    fn lower_stmt(&mut self, stmt: &Statement) -> crate::zigir::types::IrStmt {
        match stmt {
            // ── Variable declarations ──────────────────────
            Statement::VariableDeclaration(vd) => {
                // Multi-declarator: emit as a block of VarDecl statements
                if vd.declarations.len() == 1 {
                    let decl = &vd.declarations[0];
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
                } else {
                    let stmts: Vec<crate::zigir::types::IrStmt> = vd
                        .declarations
                        .iter()
                        .filter_map(|decl| {
                            let ir_decl = self.lower_var_decl(decl, vd.kind.is_const());
                            match ir_decl {
                                IrDecl::Var(v) => Some(crate::zigir::types::IrStmt::VarDecl(v)),
                                _ => None, // skip unused/error decls
                            }
                        })
                        .collect();
                    if stmts.len() == 1 {
                        stmts.into_iter().next().unwrap()
                    } else {
                        crate::zigir::types::IrStmt::Block(IrBlock::new(stmts))
                    }
                }
            }

            // ── Control flow ───────────────────────────────
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

            // ── Exception handling ─────────────────────────
            Statement::TryStatement(ts) => self.lower_try(ts),
            Statement::ThrowStatement(ts) => crate::zigir::types::IrStmt::Throw {
                value: self.lower_expr(&ts.argument),
            },

            // ── Function control ────────────────────────────
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

            // ── Block ──────────────────────────────────────
            Statement::BlockStatement(bs) => {
                let stmts: Vec<crate::zigir::types::IrStmt> =
                    bs.body.iter().map(|s| self.lower_stmt(s)).collect();
                crate::zigir::types::IrStmt::Block(IrBlock::new(stmts))
            }
            Statement::LabeledStatement(ls) => self.lower_labeled(ls),

            // ── Expression statement ───────────────────────
            Statement::ExpressionStatement(es) => {
                self.in_expr_stmt = true;
                let expr = self.lower_expr(&es.expression);
                self.in_expr_stmt = false;
                crate::zigir::types::IrStmt::Expr(expr)
            }

            // ── Function declaration (nested) ──────────────
            Statement::FunctionDeclaration(fd) => {
                // Nested function: generate as a pending_expr_fn
                let fn_name = fd.id.as_ref().map(|id| id.name.as_str());
                let is_export = self.is_export_fn(fn_name);
                if let Some(ir_fn) = self.lower_fn_decl(fd, is_export) {
                    self.pending_expr_fns.push(IrDecl::Fn(ir_fn));
                }
                crate::zigir::types::IrStmt::Comment(
                    "// nested function declaration hoisted".to_string(),
                )
            }

            // ── With statement (unsupported) ───────────────
            Statement::WithStatement(ws) => {
                let span = self.span_to_source_span(ws.span);
                self.add_error(span, "with statement is not supported");
                crate::zigir::types::IrStmt::CompileError {
                    span: SourceSpan::default(),
                    msg: "with statement is not supported".to_string(),
                }
            }

            // ── Unsupported / skippable ────────────────────
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
    fn lower_stmt_as_block(&mut self, stmt: &Statement, label: Option<String>) -> IrBlock {
        let stmts = match stmt {
            Statement::BlockStatement(bs) => bs.body.iter().map(|s| self.lower_stmt(s)).collect(),
            _ => vec![self.lower_stmt(stmt)],
        };
        IrBlock { stmts, label }
    }

    /// Lower an if statement (including else-if chains).
    fn lower_if(&mut self, is: &IfStatement) -> crate::zigir::types::IrStmt {
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
    fn lower_for(&mut self, fs: &ForStatement) -> crate::zigir::types::IrStmt {
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
    fn lower_for_of(&mut self, fos: &ForOfStatement) -> crate::zigir::types::IrStmt {
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
    /// - Static struct: unrolled loop — one block per field with `const key = "fieldName"`
    fn lower_for_in(&mut self, fis: &ForInStatement) -> crate::zigir::types::IrStmt {
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
    fn extract_for_of_vars(&self, left: &ForStatementLeft) -> (IrIdent, Vec<IrIdent>) {
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
    fn extract_for_in_var(&self, left: &ForStatementLeft) -> IrIdent {
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
    fn detect_for_of_kind(
        &self,
        right: &Expression,
        _destructure_vars: &[IrIdent],
    ) -> (IrForOfKind, bool) {
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
                }
                // Default: array iteration
                (IrForOfKind::Array, false)
            }
            _ => (IrForOfKind::Array, false),
        }
    }

    /// Detect for-in iteration kind based on the right-hand expression type.
    fn detect_for_in_kind(&self, right: &Expression) -> IrForInKind {
        match right {
            Expression::Identifier(id) => {
                if let Some(zig_type) = self.type_info.var_types.get(id.name.as_str()) {
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
    fn lower_switch(&mut self, ss: &SwitchStatement) -> crate::zigir::types::IrStmt {
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
    fn lower_try(&mut self, ts: &TryStatement) -> crate::zigir::types::IrStmt {
        let has_throw = ts.block.body.iter().any(|s| Self::stmt_has_throw_any(s));
        let has_nested_try = ts
            .block
            .body
            .iter()
            .any(|s| matches!(s, Statement::TryStatement(_)));

        let try_block = {
            let stmts = ts.block.body.iter().map(|s| self.lower_stmt(s)).collect();
            IrBlock::new(stmts)
        };

        let (catch_var, catch_var_referenced, catch_block) = if let Some(handler) = &ts.handler {
            let var = handler
                .param
                .as_ref()
                .and_then(|p| crate::infer::binding_name(&p.pattern))
                .map(|name| self.make_ident(name));
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
    fn stmt_references_name(stmt: &Statement, name: &str) -> bool {
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

    fn expr_references_name(expr: &Expression, name: &str) -> bool {
        match expr {
            Expression::Identifier(id) => id.name.as_str() == name,
            Expression::BinaryExpression(be) => {
                Self::expr_references_name(&be.left, name)
                    || Self::expr_references_name(&be.right, name)
            }
            Expression::CallExpression(ce) => Self::expr_references_name(&ce.callee, name),
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

    /// Check if a statement contains a `throw` (directly, not inside a
    /// nested try-catch — those throws are caught by the inner catch).
    /// Mirrors Codegen's `stmt_has_throw_any`.
    fn stmt_has_throw_any(stmt: &Statement) -> bool {
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
            // Intentionally NOT recursing into TryStatement —
            // throws inside a nested try are caught by its own catch.
            _ => false,
        }
    }

    /// Lower a labeled statement.
    ///
    /// For loops, the label is attached to the loop body.
    /// For blocks, the label is attached only if a `break :label` exists
    /// inside the body.
    fn lower_labeled(&mut self, ls: &LabeledStatement) -> crate::zigir::types::IrStmt {
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
    fn stmt_has_break_to_label(stmt: &Statement, label_name: &str) -> bool {
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

    /// Get and consume the pending loop label (set by lower_labeled).
    fn current_loop_label(&mut self) -> Option<String> {
        self.pending_label.take()
    }

    /// Lower a block of statements.
    fn lower_block(&mut self, stmts: &[Statement]) -> IrBlock {
        let ir_stmts: Vec<crate::zigir::types::IrStmt> =
            stmts.iter().map(|s| self.lower_stmt(s)).collect();
        IrBlock::new(ir_stmts)
    }
}

// ═══════════════════════════════════════════════════════
//  Remaining stubs
// ═══════════════════════════════════════════════════════

impl Lowerer {
    /// Lower a class declaration into IrClassDecl.
    ///
    /// Extracts fields (from PropertyDefinition and implicit constructor `this.x = ...`),
    /// constructor → IrClassMethod, regular methods → IrClassMethod, and static inits.
    fn lower_class_decl(&mut self, cd: &Class) -> Option<crate::zigir::types::IrClassDecl> {
        use crate::zigir::types::{IrClassDecl, IrClassField, IrClassMethod};

        let class_name = cd
            .id
            .as_ref()
            .map(|id| id.name.to_string())
            .unwrap_or_else(|| "AnonymousClass".to_string());

        // ── First pass: collect explicit fields from PropertyDefinition ──
        let mut field_names: Vec<String> = Vec::new();
        let mut fields: Vec<IrClassField> = Vec::new();
        let mut static_inits: Vec<crate::zigir::types::IrExpr> = Vec::new();
        let mut has_constructor = false;
        let mut constructor_func: Option<&Function> = None;

        for elem in &cd.body.body {
            match elem {
                ClassElement::PropertyDefinition(pd) => {
                    if pd.computed {
                        continue;
                    }
                    let is_static = pd.r#static;
                    if let Some(name) = Self::property_key_name(&pd.key) {
                        if is_static {
                            if let Some(value) = &pd.value {
                                static_inits.push(self.lower_expr(value));
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

        // ── Save/restore current_class ──
        let saved_class = self.current_class.take();
        self.current_class = Some(class_name.clone());

        // ── Lower constructor ──
        let constructor = if has_constructor {
            constructor_func
                .map(|func| self.lower_class_method(&class_name, &field_names, "init", func, false))
        } else {
            None
        };

        // ── Lower methods ──
        let mut methods: Vec<IrClassMethod> = Vec::new();
        for elem in &cd.body.body {
            if let ClassElement::MethodDefinition(md) = elem
                && !Self::is_constructor_method(md)
            {
                let method_name =
                    Self::property_key_name(&md.key).unwrap_or_else(|| "anonymous".to_string());
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

        // ── Restore ──
        self.current_class = saved_class;

        // ── extends ──
        let extends = cd.super_class.as_ref().and_then(|sc| {
            if let Expression::Identifier(id) = sc {
                Some(id.name.to_string())
            } else {
                None
            }
        });

        Some(IrClassDecl {
            name: self.make_ident(&class_name),
            fields,
            constructor,
            methods,
            static_inits,
            extends,
        })
    }

    /// Extract the string name from a PropertyKey.
    fn property_key_name(key: &PropertyKey) -> Option<String> {
        match key {
            PropertyKey::StaticIdentifier(id) => Some(id.name.to_string()),
            PropertyKey::StringLiteral(sl) => Some(sl.value.to_string()),
            PropertyKey::PrivateIdentifier(id) => Some(id.name.to_string()),
            _ => None,
        }
    }

    /// Check if a MethodDefinition is `constructor()`.
    fn is_constructor_method(md: &MethodDefinition) -> bool {
        Self::property_key_name(&md.key).is_some_and(|name| name == "constructor")
    }

    /// Scan constructor body for `this.x = ...` assignments and add
    /// discovered fields if not already present.
    fn collect_implicit_class_fields(
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
    fn lower_class_method(
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
    fn lower_block_with_this_rewrite(
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
                            // this.field = value → const field = value
                            let value_ir = self.lower_expr(&ae.right);
                            ir_stmts.push(crate::zigir::types::IrStmt::VarDecl(
                                crate::zigir::types::IrVarDecl {
                                    name: self.make_ident(&fname),
                                    is_const: true,
                                    zig_type: None,
                                    init: Some(value_ir),
                                    is_json_parse: false,
                                    needs_var_suppression: false,
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

    /// Lower an expression.
    fn lower_expr(&mut self, expr: &Expression) -> crate::zigir::types::IrExpr {
        use crate::zigir::types::IrExpr;

        match expr {
            // ── Literals ────────────────────────────────
            Expression::NumericLiteral(n) => {
                // Zig considers `-0` ambiguous; emit `-0.0` explicitly for negative zero
                if n.value == -0.0 && n.value.is_sign_negative() {
                    IrExpr::FloatLiteral(-0.0)
                } else if n.value.fract() == 0.0 && n.value.abs() < i64::MAX as f64 {
                    IrExpr::IntLiteral(n.value as i64)
                } else {
                    IrExpr::FloatLiteral(n.value)
                }
            }
            Expression::StringLiteral(s) => IrExpr::StringLiteral(s.value.to_string()),
            Expression::BooleanLiteral(b) => IrExpr::BoolLiteral(b.value),
            Expression::NullLiteral(_) => IrExpr::Null,
            Expression::RegExpLiteral(rl) => {
                let span = self.span_to_source_span(rl.span);
                self.add_error(span, "RegExp literals are not directly supported");
                IrExpr::CompileError {
                    span: SourceSpan::default(),
                    msg: "RegExp literal not supported".to_string(),
                }
            }
            Expression::BigIntLiteral(bi) => {
                // BigInt is emitted as a string in Zig; store as StringLiteral
                let raw = bi.raw.as_ref().map(|s| s.to_string()).unwrap_or_default();
                IrExpr::StringLiteral(raw)
            }

            // ── Identifier ──────────────────────────────
            Expression::Identifier(id) => self.lower_ident_expr(id),

            // ── This ───────────────────────────────────
            Expression::ThisExpression(te) => {
                if self.current_class.is_some() {
                    IrExpr::This
                } else {
                    let span = self.span_to_source_span(te.span);
                    self.add_error(span, "`this` used outside of a class method");
                    IrExpr::CompileError {
                        span: SourceSpan::default(),
                        msg: "`this` used outside of a class method".to_string(),
                    }
                }
            }

            // ── Binary expression ──────────────────────
            Expression::BinaryExpression(be) => self.lower_binary(be),

            // ── Logical expression ─────────────────────
            Expression::LogicalExpression(le) => {
                let op = match le.operator {
                    LogicalOperator::And => LogicalOp::And,
                    LogicalOperator::Or => LogicalOp::Or,
                    LogicalOperator::Coalesce => LogicalOp::Nullish,
                };
                IrExpr::Logical {
                    op,
                    left: Box::new(self.lower_expr(&le.left)),
                    right: Box::new(self.lower_expr(&le.right)),
                }
            }

            // ── Unary expression ───────────────────────
            Expression::UnaryExpression(ue) => self.lower_unary(ue),

            // ── Update expression ──────────────────────
            Expression::UpdateExpression(ue) => self.lower_update(ue),

            // ── Assignment expression ──────────────────
            Expression::AssignmentExpression(ae) => self.lower_assignment(ae),

            // ── Parenthesized ──────────────────────────
            Expression::ParenthesizedExpression(pe) => {
                IrExpr::Paren(Box::new(self.lower_expr(&pe.expression)))
            }

            // ── Conditional ────────────────────────────
            Expression::ConditionalExpression(ce) => IrExpr::Conditional {
                cond: Box::new(self.lower_expr(&ce.test)),
                then: Box::new(self.lower_expr(&ce.consequent)),
                else_: Box::new(self.lower_expr(&ce.alternate)),
            },

            // ── Sequence expression ────────────────────
            Expression::SequenceExpression(se) => {
                let exprs: Vec<IrExpr> =
                    se.expressions.iter().map(|e| self.lower_expr(e)).collect();
                IrExpr::Sequence(exprs)
            }

            // ── Calls ──────────────────────────────────
            Expression::CallExpression(ce) => self.lower_call(ce),
            Expression::NewExpression(ne) => self.lower_new(ne),

            // ── Member access ──────────────────────────
            Expression::StaticMemberExpression(mem) => self.lower_static_member(mem),
            Expression::ComputedMemberExpression(mem) => self.lower_computed_member(mem),

            // ── Array / Object literals ────────────────
            Expression::ArrayExpression(ae) => self.lower_array_expr(ae),
            Expression::ObjectExpression(oe) => self.lower_object_expr(oe),

            // ── Function expressions ───────────────────
            Expression::ArrowFunctionExpression(af) => self.lower_arrow_fn(af),
            Expression::FunctionExpression(fe) => self.lower_fn_expr(fe),

            // ── Template literal ───────────────────────
            Expression::TemplateLiteral(tl) => self.lower_template_literal(tl),

            // ── Tagged template ────────────────────────
            Expression::TaggedTemplateExpression(tte) => {
                let span = self.span_to_source_span(tte.span);
                self.add_error(span, "Tagged template literals are not supported");
                IrExpr::CompileError {
                    span: SourceSpan::default(),
                    msg: "Tagged template literals not supported".to_string(),
                }
            }

            // ── Await ──────────────────────────────────
            Expression::AwaitExpression(ae) => self.lower_await(ae),

            // ── typeof / void / delete ─────────────────
            // (handled via UnaryExpression, but also here as fallback)

            // ── Yield ──────────────────────────────────
            Expression::YieldExpression(ye) => {
                let span = self.span_to_source_span(ye.span);
                self.add_error(span, "Yield expressions are not supported");
                IrExpr::CompileError {
                    span: SourceSpan::default(),
                    msg: "yield not supported".to_string(),
                }
            }

            // ── MetaProperty (import.meta, new.target) ──
            Expression::MetaProperty(mp) => {
                let span = self.span_to_source_span(mp.span);
                self.add_error(
                    span,
                    "MetaProperty (import.meta/new.target) is not supported",
                );
                IrExpr::CompileError {
                    span: SourceSpan::default(),
                    msg: "MetaProperty not supported".to_string(),
                }
            }

            // ── Super ──────────────────────────────────
            Expression::Super(sup) => {
                let span = self.span_to_source_span(sup.span);
                self.add_error(span, "super is not supported");
                IrExpr::CompileError {
                    span: SourceSpan::default(),
                    msg: "super not supported".to_string(),
                }
            }

            // ── Import ─────────────────────────────────
            Expression::ImportExpression(ie) => {
                let span = self.span_to_source_span(ie.span);
                self.add_error(span, "dynamic import() is not supported");
                IrExpr::CompileError {
                    span: SourceSpan::default(),
                    msg: "dynamic import() not supported".to_string(),
                }
            }

            // ── PrivateFieldAccess ─────────────────────
            Expression::PrivateFieldExpression(pfe) => {
                // Private fields are lowered like normal member access
                // with a field_kind marker
                let object = Box::new(self.lower_expr(&pfe.object));
                let field = pfe.field.name.to_string();
                IrExpr::FieldAccess {
                    object,
                    field,
                    field_kind: FieldKind::Private,
                }
            }

            // ── Fallback ───────────────────────────────
            _ => IrExpr::CompileError {
                span: SourceSpan::default(),
                msg: format!("unsupported expression type: {}", expr_type_name(expr)),
            },
        }
    }

    /// Lower an identifier expression with special handling for
    /// built-in globals (NaN, Infinity, undefined, arguments)
    /// and captured closure variables.
    fn lower_ident_expr(&mut self, id: &IdentifierReference) -> crate::zigir::types::IrExpr {
        use crate::zigir::types::IrExpr;

        let var_name = id.name.as_str();

        // arguments object: not supported
        if var_name == "arguments" {
            let span = self.span_to_source_span(id.span);
            self.add_error(
                span,
                "arguments object is not supported. Use rest parameters (...args) instead.",
            );
            return IrExpr::CompileError {
                span: SourceSpan::default(),
                msg: "arguments not supported".to_string(),
            };
        }

        // JS global constants
        if var_name == "NaN" {
            return IrExpr::FieldAccess {
                object: Box::new(IrExpr::Ident(IrIdent::new("std"))),
                field: "nan".to_string(),
                field_kind: FieldKind::Namespace,
            };
        }
        if var_name == "Infinity" {
            return IrExpr::FieldAccess {
                object: Box::new(IrExpr::Ident(IrIdent::new("std"))),
                field: "inf".to_string(),
                field_kind: FieldKind::Namespace,
            };
        }
        // undefined → JsAny.fromUndefined()
        // (Stored as Ident with special name; Emitter will handle)
        if var_name == "undefined" {
            return IrExpr::Undefined;
        }

        // Check if this identifier is a captured closure variable.
        // If so, rewrite to self.var_name (value capture) or self.var_name.* (ref capture).
        if let Some((_, _, is_mut)) = self
            .closure_mgr
            .current_captured
            .iter()
            .find(|(n, _, _)| n == var_name)
        {
            let field_name = self.make_ident(var_name);
            let self_access = IrExpr::FieldAccess {
                object: Box::new(IrExpr::Ident(IrIdent::new("self"))),
                field: field_name.zig_name.clone(),
                field_kind: FieldKind::StructField,
            };
            if *is_mut {
                // Reference capture: dereference the pointer
                return IrExpr::FieldAccess {
                    object: Box::new(self_access),
                    field: "*".to_string(),
                    field_kind: FieldKind::PointerDeref,
                };
            } else {
                return self_access;
            }
        }

        IrExpr::Ident(IrIdent::new(var_name))
    }

    /// Lower a binary expression.
    fn lower_binary(&mut self, be: &BinaryExpression) -> crate::zigir::types::IrExpr {
        use crate::zigir::types::IrExpr;

        // String concatenation detection: if `+` has any string operand,
        // flatten the chain and produce IrExpr::AllocPrint instead of
        // a binary tree of IrExpr::Binary nodes.
        if be.operator == BinaryOperator::Addition {
            let left_is_str = self.expr_is_string(&be.left);
            let right_is_str = self.expr_is_string(&be.right);
            if left_is_str || right_is_str {
                return self.lower_string_concat(be);
            }
        }

        let op = match be.operator {
            BinaryOperator::Addition => BinOp::Add,
            BinaryOperator::Subtraction => BinOp::Sub,
            BinaryOperator::Multiplication => BinOp::Mul,
            BinaryOperator::Division => BinOp::Div,
            BinaryOperator::Remainder => BinOp::Mod,
            BinaryOperator::Exponential => BinOp::Pow,
            BinaryOperator::LessThan => BinOp::Lt,
            BinaryOperator::GreaterThan => BinOp::Gt,
            BinaryOperator::LessEqualThan => BinOp::Le,
            BinaryOperator::GreaterEqualThan => BinOp::Ge,
            BinaryOperator::Equality => BinOp::Eq,
            BinaryOperator::Inequality => BinOp::Ne,
            BinaryOperator::StrictEquality => BinOp::StrictEq,
            BinaryOperator::StrictInequality => BinOp::StrictNe,
            BinaryOperator::BitwiseAnd => BinOp::BitAnd,
            BinaryOperator::BitwiseOR => BinOp::BitOr,
            BinaryOperator::BitwiseXOR => BinOp::BitXor,
            BinaryOperator::ShiftLeft => BinOp::Shl,
            BinaryOperator::ShiftRight => BinOp::Shr,
            BinaryOperator::ShiftRightZeroFill => BinOp::UrShr,
            BinaryOperator::In => BinOp::In,
            BinaryOperator::Instanceof => BinOp::InstanceOf,
        };

        IrExpr::Binary {
            op,
            left: Box::new(self.lower_expr(&be.left)),
            right: Box::new(self.lower_expr(&be.right)),
        }
    }

    /// Lower a unary expression.
    fn lower_unary(&mut self, ue: &UnaryExpression) -> crate::zigir::types::IrExpr {
        use crate::zigir::types::IrExpr;

        match ue.operator {
            UnaryOperator::UnaryNegation => IrExpr::Unary {
                op: UnaOp::Neg,
                operand: Box::new(self.lower_expr(&ue.argument)),
            },
            UnaryOperator::UnaryPlus => {
                // Unary plus is a no-op in terms of IR; just lower the argument
                self.lower_expr(&ue.argument)
            }
            UnaryOperator::LogicalNot => IrExpr::Unary {
                op: UnaOp::Not,
                operand: Box::new(self.lower_expr(&ue.argument)),
            },
            UnaryOperator::BitwiseNot => IrExpr::Unary {
                op: UnaOp::BitNot,
                operand: Box::new(self.lower_expr(&ue.argument)),
            },
            UnaryOperator::Typeof => {
                // Use inferred Zig type to emit the JS typeof string at compile time.
                // For dynamic types (JsAny/Anytype), call the runtime jsTypeof() helper.
                if let Some(ty) = self.infer_expr_type(&ue.argument) {
                    if let Some(js_typeof) = ty.to_js_typeof() {
                        // Compile-time resolution: the argument is not included in the IR.
                        // Track its identifiers so unused-param detection doesn't
                        // falsely mark them as unused.
                        let mut idents = HashSet::new();
                        Self::collect_ast_expr_idents(&ue.argument, &mut idents);
                        if let Some(ctx) = self.fn_ctx.as_mut() {
                            ctx.compile_time_referenced_idents.extend(idents);
                        }
                        IrExpr::StringLiteral(js_typeof.to_string())
                    } else {
                        IrExpr::BuiltinCall(crate::zigir::types::IrBuiltinCall {
                            module: crate::zigir::builtins::BuiltinModule::JsRuntime,
                            method: "jsTypeof".to_string(),
                            obj_name: None,
                            args: vec![self.lower_expr(&ue.argument)],
                            return_type: crate::types::ZigType::Str,
                        })
                    }
                } else {
                    IrExpr::BuiltinCall(crate::zigir::types::IrBuiltinCall {
                        module: crate::zigir::builtins::BuiltinModule::JsRuntime,
                        method: "jsTypeof".to_string(),
                        obj_name: None,
                        args: vec![self.lower_expr(&ue.argument)],
                        return_type: crate::types::ZigType::Str,
                    })
                }
            }
            UnaryOperator::Void => IrExpr::Void(Box::new(self.lower_expr(&ue.argument))),
            UnaryOperator::Delete => IrExpr::Unary {
                op: UnaOp::Delete,
                operand: Box::new(self.lower_expr(&ue.argument)),
            },
        }
    }

    /// Lower an update expression (++/--).
    fn lower_update(&mut self, ue: &UpdateExpression) -> crate::zigir::types::IrExpr {
        let op = if ue.operator == UpdateOperator::Increment {
            UpdateOp::Increment
        } else {
            UpdateOp::Decrement
        };
        let target = Box::new(self.lower_simple_assign_target(&ue.argument));
        crate::zigir::types::IrExpr::Update {
            op,
            target,
            is_expr_stmt: self.in_expr_stmt,
        }
    }

    /// Lower an assignment expression.
    fn lower_assignment(&mut self, ae: &AssignmentExpression) -> crate::zigir::types::IrExpr {
        let op = match ae.operator {
            AssignmentOperator::Assign => AssignOp::Assign,
            AssignmentOperator::Addition => AssignOp::Add,
            AssignmentOperator::Subtraction => AssignOp::Sub,
            AssignmentOperator::Multiplication => AssignOp::Mul,
            AssignmentOperator::Division => AssignOp::Div,
            AssignmentOperator::Remainder => AssignOp::Mod,
            AssignmentOperator::Exponential => {
                // **= is not a direct AssignOp; emit as pow call
                // For now, store as AssignOp::Assign and note in a comment
                AssignOp::Assign
            }
            AssignmentOperator::ShiftLeft => AssignOp::Shl,
            AssignmentOperator::ShiftRight => AssignOp::Shr,
            AssignmentOperator::ShiftRightZeroFill => AssignOp::Shr,
            AssignmentOperator::BitwiseAnd => AssignOp::BitAnd,
            AssignmentOperator::BitwiseOR => AssignOp::BitOr,
            AssignmentOperator::BitwiseXOR => AssignOp::BitXor,
            AssignmentOperator::LogicalAnd => AssignOp::LogicAnd,
            AssignmentOperator::LogicalOr => AssignOp::LogicOr,
            AssignmentOperator::LogicalNullish => AssignOp::Nullish,
        };
        let target = Box::new(self.lower_assign_target(&ae.left));
        let value = Box::new(self.lower_expr(&ae.right));
        crate::zigir::types::IrExpr::Assign { op, target, value }
    }

    /// Lower a simple assignment target (from UpdateExpression).
    /// SimpleAssignmentTarget can be an identifier or member expression.
    fn lower_simple_assign_target(
        &mut self,
        target: &SimpleAssignmentTarget,
    ) -> crate::zigir::types::IrAssignTarget {
        match target {
            SimpleAssignmentTarget::AssignmentTargetIdentifier(id) => {
                let var_name = id.name.as_str();

                // Check if this identifier is a captured closure variable.
                if let Some((_, _, is_mut)) = self
                    .closure_mgr
                    .current_captured
                    .iter()
                    .find(|(n, _, _)| n == var_name)
                {
                    let field_name = self.make_ident(var_name).zig_name;
                    return crate::zigir::types::IrAssignTarget::Member {
                        object: Box::new(crate::zigir::types::IrExpr::Ident(IrIdent::new("self"))),
                        field: field_name,
                        is_pointer: *is_mut,
                    };
                }

                crate::zigir::types::IrAssignTarget::Ident(IrIdent::new(var_name))
            }
            SimpleAssignmentTarget::StaticMemberExpression(mem) => {
                crate::zigir::types::IrAssignTarget::Member {
                    object: Box::new(self.lower_expr(&mem.object)),
                    field: mem.property.name.to_string(),
                    is_pointer: false,
                }
            }
            SimpleAssignmentTarget::ComputedMemberExpression(mem) => {
                crate::zigir::types::IrAssignTarget::Index {
                    object: Box::new(self.lower_expr(&mem.object)),
                    index: Box::new(self.lower_expr(&mem.expression)),
                }
            }
            _ => crate::zigir::types::IrAssignTarget::Ident(IrIdent::new("__unsupported_target")),
        }
    }

    /// Extract (pattern, default) from an AssignmentTargetMaybeDefault.
    fn lower_maybe_default(
        &mut self,
        target: &AssignmentTargetMaybeDefault,
    ) -> (IrIdent, Option<crate::zigir::types::IrExpr>) {
        match target {
            AssignmentTargetMaybeDefault::AssignmentTargetWithDefault(atwd) => {
                let pattern = self.extract_target_ident(&atwd.binding);
                let default = Some(self.lower_expr(&atwd.init));
                (pattern, default)
            }
            AssignmentTargetMaybeDefault::AssignmentTargetIdentifier(id) => {
                (IrIdent::new(id.name.as_str()), None)
            }
            _ => (IrIdent::new("_"), None),
        }
    }

    /// Extract an identifier name from an AssignmentTarget (best-effort).
    fn extract_target_ident(&self, target: &AssignmentTarget) -> IrIdent {
        match target {
            AssignmentTarget::AssignmentTargetIdentifier(id) => IrIdent::new(id.name.as_str()),
            _ => IrIdent::new("_"),
        }
    }

    /// Lower an assignment target (lhs).
    fn lower_assign_target(
        &mut self,
        target: &AssignmentTarget,
    ) -> crate::zigir::types::IrAssignTarget {
        match target {
            AssignmentTarget::AssignmentTargetIdentifier(id) => {
                let var_name = id.name.as_str();

                // Check if this identifier is a captured closure variable.
                // If so, rewrite the assignment target to self.xxx (value) or self.xxx.* (ref).
                if let Some((_, _, is_mut)) = self
                    .closure_mgr
                    .current_captured
                    .iter()
                    .find(|(n, _, _)| n == var_name)
                {
                    let field_name = self.make_ident(var_name).zig_name;
                    return crate::zigir::types::IrAssignTarget::Member {
                        object: Box::new(crate::zigir::types::IrExpr::Ident(IrIdent::new("self"))),
                        field: field_name,
                        is_pointer: *is_mut,
                    };
                }

                crate::zigir::types::IrAssignTarget::Ident(IrIdent::new(var_name))
            }
            AssignmentTarget::StaticMemberExpression(mem) => {
                crate::zigir::types::IrAssignTarget::Member {
                    object: Box::new(self.lower_expr(&mem.object)),
                    field: mem.property.name.to_string(),
                    is_pointer: false,
                }
            }
            AssignmentTarget::ComputedMemberExpression(mem) => {
                crate::zigir::types::IrAssignTarget::Index {
                    object: Box::new(self.lower_expr(&mem.object)),
                    index: Box::new(self.lower_expr(&mem.expression)),
                }
            }
            AssignmentTarget::ObjectAssignmentTarget(ot) => {
                let bindings: Vec<crate::zigir::types::IrDestructureBinding> = ot
                    .properties
                    .iter()
                    .map(|prop| {
                        match prop {
                            AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(ap) => {
                                let pattern = IrIdent::new(ap.binding.name.as_str());
                                let default = ap.init.as_ref().map(|e| self.lower_expr(e));
                                crate::zigir::types::IrDestructureBinding { pattern, default }
                            }
                            AssignmentTargetProperty::AssignmentTargetPropertyProperty(ap) => {
                                // e.g. { name: alias } — extract binding from value
                                let (pattern, default) = self.lower_maybe_default(&ap.binding);
                                crate::zigir::types::IrDestructureBinding { pattern, default }
                            }
                        }
                    })
                    .collect();
                crate::zigir::types::IrAssignTarget::Destructure(bindings)
            }
            AssignmentTarget::ArrayAssignmentTarget(at) => {
                let bindings: Vec<crate::zigir::types::IrDestructureBinding> = at
                    .elements
                    .iter()
                    .filter_map(|elem| {
                        elem.as_ref().map(|target| {
                            let (pattern, default) = self.lower_maybe_default(target);
                            crate::zigir::types::IrDestructureBinding { pattern, default }
                        })
                    })
                    .collect();
                crate::zigir::types::IrAssignTarget::Destructure(bindings)
            }
            _ => crate::zigir::types::IrAssignTarget::Ident(IrIdent::new("__unsupported_target")),
        }
    }

    /// Lower a call expression.
    ///
    /// Routing priority (mirrors Codegen's `emit_call`):
    /// 1. Builtin detection → `IrBuiltinCall`
    /// 2. Closure / nested function call → `IrCall { call_kind: Closure }`
    /// 3. Host function call → `IrHostCall`
    /// 4. Direct user function → `IrCall { call_kind: Direct }`
    /// 5. Method call → `IrCall { call_kind: Method { .. } }`
    /// 6. IIFE / expression callee → `IrCall { call_kind: Closure }`
    fn lower_call(&mut self, ce: &CallExpression) -> crate::zigir::types::IrExpr {
        use crate::zigir::types::IrExpr;

        let args: Vec<IrExpr> = ce
            .arguments
            .iter()
            .map(|arg| {
                match arg {
                    Argument::SpreadElement(se) => {
                        IrExpr::Spread(Box::new(self.lower_expr(&se.argument)))
                    }
                    // Argument inherits all Expression variants
                    _ => {
                        // All Expression variants are directly accessible
                        let expr = arg.as_expression().unwrap();
                        self.lower_expr(expr)
                    }
                }
            })
            .collect();

        // ── Step 1: Builtin detection ──
        if let Some(builtin) = crate::native_builtins::detect_builtin_call(ce) {
            // ── Step 1a: Array callback inlining ──
            if let Some(inlined) = self.try_inline_array_callback(ce, &builtin) {
                return inlined;
            }

            // ── Step 1b: Array non-callback method inlining ──
            if let Some(inlined) = self.try_inline_array_method(ce, &builtin, &args) {
                return inlined;
            }

            let (module, method, return_type) = builtin_call_to_ir(&builtin);
            let obj_name = Self::extract_callee_object_name_static(&ce.callee);
            return IrExpr::BuiltinCall(crate::zigir::types::IrBuiltinCall {
                module,
                method,
                obj_name,
                args,
                return_type,
            });
        }

        // ── Step 2: Identify callee pattern ──
        match &ce.callee {
            // Identifier callee: direct function call, host call, or closure
            Expression::Identifier(id) => {
                let name = id.name.as_str();

                // Host function call: starts with "host_"
                if let Some(host_name) = name.strip_prefix("host_") {
                    let is_async = self.async_host_fns.contains(name);
                    let return_type = self.infer_host_return_type(host_name);
                    return IrExpr::HostCall(crate::zigir::types::IrHostCall {
                        name: host_name.to_string(),
                        args,
                        return_type,
                        is_async,
                    });
                }

                // Closure / nested function call
                if self
                    .closure_mgr
                    .current_captured
                    .iter()
                    .any(|(n, _, _)| n.as_str() == name)
                    || self.is_closure_instance(name)
                {
                    return IrExpr::Call(crate::zigir::types::IrCallExpr {
                        callee: Box::new(IrExpr::Ident(IrIdent::new(name))),
                        args,
                        call_kind: CallKind::Closure,
                    });
                }

                // Direct user function call
                IrExpr::Call(crate::zigir::types::IrCallExpr {
                    callee: Box::new(IrExpr::Ident(IrIdent::new(name))),
                    args,
                    call_kind: CallKind::Direct,
                })
            }

            // Static member expression callee: obj.method()
            Expression::StaticMemberExpression(mem) => {
                let method_name = mem.property.name.as_str();
                let obj_expr = self.lower_expr(&mem.object);

                // Determine method object type for CallKind::Method
                let object_type = self.infer_method_object_kind(&mem.object);

                IrExpr::Call(crate::zigir::types::IrCallExpr {
                    callee: Box::new(IrExpr::FieldAccess {
                        object: Box::new(obj_expr),
                        field: method_name.to_string(),
                        field_kind: FieldKind::StructField,
                    }),
                    args,
                    call_kind: CallKind::Method { object_type },
                })
            }

            // Function expression / arrow function callee (IIFE)
            Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_) => {
                // IIFE: emit the function then call it
                let callee = self.lower_expr(&ce.callee);
                IrExpr::Call(crate::zigir::types::IrCallExpr {
                    callee: Box::new(callee),
                    args,
                    call_kind: CallKind::Closure,
                })
            }

            // Parenthesized expression containing function
            Expression::ParenthesizedExpression(_) => {
                let callee = self.lower_expr(&ce.callee);
                IrExpr::Call(crate::zigir::types::IrCallExpr {
                    callee: Box::new(callee),
                    args,
                    call_kind: CallKind::Closure,
                })
            }

            // Any other callee type (computed member, etc.)
            _ => {
                let callee = self.lower_expr(&ce.callee);
                IrExpr::Call(crate::zigir::types::IrCallExpr {
                    callee: Box::new(callee),
                    args,
                    call_kind: CallKind::Direct,
                })
            }
        }
    }

    /// Determine the MethodObjectKind for a method call's receiver object.
    fn infer_method_object_kind(&self, obj: &Expression) -> crate::zigir::kinds::MethodObjectKind {
        use crate::zigir::kinds::MethodObjectKind;

        match obj {
            Expression::Identifier(id) => {
                if let Some(zig_type) = self.type_info.var_types.get(id.name.as_str()) {
                    match zig_type {
                        ZigType::ArrayList(_) => MethodObjectKind::ArrayList,
                        ZigType::Str => MethodObjectKind::String,
                        ZigType::NamedStruct(name) => match name.as_str() {
                            "Map" => MethodObjectKind::Map,
                            "Set" => MethodObjectKind::Set,
                            "Date" | "JsDate" => MethodObjectKind::Date,
                            other => {
                                if self.class_names.contains(other) {
                                    MethodObjectKind::Class(other.to_string())
                                } else {
                                    MethodObjectKind::Unknown
                                }
                            }
                        },
                        ZigType::JsAny | ZigType::Anytype => MethodObjectKind::JsAny,
                        _ => MethodObjectKind::Unknown,
                    }
                } else {
                    MethodObjectKind::Unknown
                }
            }
            _ => MethodObjectKind::Unknown,
        }
    }

    /// Check if a name refers to a closure instance.
    fn is_closure_instance(&self, name: &str) -> bool {
        self.closure_mgr.closure_instances.contains(name)
    }

    /// Infer the return type of a host function.
    fn infer_host_return_type(&self, _host_name: &str) -> ZigType {
        // TODO: look up host function return type from type_info
        ZigType::JsAny
    }

    /// Lower a static member expression (`obj.field`).
    ///
    /// Determines the FieldKind based on:
    /// - Math constants → `MathConstant`
    /// - Number constants → `NumberConstant`
    /// - Symbol well-known → `SymbolWellKnown`
    /// - TypedArray properties → `TypedArrayProp`
    /// - Map/Set `.size` → `MapSetSize`
    /// - ArrayList `.length` → `ArrayListLen`
    /// - Other `.length` → `StringLen`
    /// - Default → `StructField`
    fn lower_static_member(&mut self, mem: &StaticMemberExpression) -> crate::zigir::types::IrExpr {
        use crate::zigir::types::IrExpr;

        let field_name = mem.property.name.as_str();

        // ── Math constants: Math.PI, Math.E, etc. ──
        if let Expression::Identifier(id) = &mem.object {
            if id.name.as_str() == "Math" {
                return IrExpr::FieldAccess {
                    object: Box::new(self.lower_expr(&mem.object)),
                    field: field_name.to_string(),
                    field_kind: FieldKind::MathConstant(field_name.to_string()),
                };
            }
            // ── Number constants: Number.MAX_VALUE, Number.NaN, etc. ──
            if id.name.as_str() == "Number" {
                return IrExpr::FieldAccess {
                    object: Box::new(self.lower_expr(&mem.object)),
                    field: field_name.to_string(),
                    field_kind: FieldKind::NumberConstant(field_name.to_string()),
                };
            }
            // ── Symbol well-known: Symbol.iterator, etc. ──
            if id.name.as_str() == "Symbol" {
                return IrExpr::FieldAccess {
                    object: Box::new(self.lower_expr(&mem.object)),
                    field: field_name.to_string(),
                    field_kind: FieldKind::SymbolWellKnown(field_name.to_string()),
                };
            }
            // ── TypedArray properties ──
            if let Some(zig_type) = self.type_info.var_types.get(id.name.as_str()) {
                if matches!(zig_type, ZigType::NamedStruct(name) if name == "TypedArray")
                    && matches!(field_name, "buffer" | "byteLength" | "byteOffset")
                {
                    return IrExpr::FieldAccess {
                        object: Box::new(self.lower_expr(&mem.object)),
                        field: field_name.to_string(),
                        field_kind: FieldKind::TypedArrayProp(field_name.to_string()),
                    };
                }
                // ── Map/Set .size ──
                if matches!(zig_type, ZigType::NamedStruct(name) if name == "Map" || name == "Set")
                    && field_name == "size"
                {
                    return IrExpr::FieldAccess {
                        object: Box::new(self.lower_expr(&mem.object)),
                        field: field_name.to_string(),
                        field_kind: FieldKind::MapSetSize,
                    };
                }
                // ── ArrayList .length → .items.len ──
                if matches!(zig_type, ZigType::ArrayList(_)) && field_name == "length" {
                    return IrExpr::FieldAccess {
                        object: Box::new(self.lower_expr(&mem.object)),
                        field: field_name.to_string(),
                        field_kind: FieldKind::ArrayListLen,
                    };
                }
            }
            // ── .length on other types (string, slice, etc.) ──
            if field_name == "length" {
                return IrExpr::FieldAccess {
                    object: Box::new(self.lower_expr(&mem.object)),
                    field: field_name.to_string(),
                    field_kind: FieldKind::StringLen,
                };
            }
        }

        // ── Default: struct field access ──
        IrExpr::FieldAccess {
            object: Box::new(self.lower_expr(&mem.object)),
            field: field_name.to_string(),
            field_kind: FieldKind::StructField,
        }
    }

    /// Lower a computed member expression (`obj[key]`).
    ///
    /// Three sub-cases:
    /// - NumericLiteral key → IndexAccess (ArrayListItem or SliceIndex)
    /// - StringLiteral key → ComputedField (StructField, MapGet, JsAnyGetByKey)
    /// - Dynamic expression key → ComputedField (varies by object type)
    fn lower_computed_member(
        &mut self,
        mem: &ComputedMemberExpression,
    ) -> crate::zigir::types::IrExpr {
        use crate::zigir::types::IrExpr;

        let object = Box::new(self.lower_expr(&mem.object));

        // Determine the ZigType of the object (for routing)
        let obj_type = self.infer_expr_type(&mem.object);

        // ── Case 1: NumericLiteral key → IndexAccess ──
        if let Expression::NumericLiteral(nl) = &mem.expression {
            let is_arraylist = obj_type
                .as_ref()
                .map(|t| matches!(t, ZigType::ArrayList(_)))
                .unwrap_or(false);
            return IrExpr::IndexAccess {
                object,
                index: Box::new(IrExpr::IntLiteral(nl.value as i64)),
                index_kind: if is_arraylist {
                    IndexKind::ArrayListItem
                } else {
                    IndexKind::SliceIndex
                },
            };
        }

        // ── Case 2: StringLiteral key → ComputedField ──
        if let Expression::StringLiteral(sl) = &mem.expression {
            let key_kind = match &obj_type {
                Some(ZigType::Struct(_)) => ComputedKeyKind::StructField,
                Some(ZigType::NamedStruct(name)) if name == "Map" => ComputedKeyKind::MapGet,
                Some(ZigType::NamedStruct(_)) => ComputedKeyKind::StructField,
                Some(ZigType::Anytype) | Some(ZigType::JsAny) => ComputedKeyKind::JsAnyGetByKey,
                _ => ComputedKeyKind::JsAnyGetByKey,
            };
            return IrExpr::ComputedField {
                object,
                key: Box::new(IrExpr::StringLiteral(sl.value.to_string())),
                key_kind,
            };
        }

        // ── Case 3: Dynamic expression key → ComputedField ──
        let key = Box::new(self.lower_expr(&mem.expression));
        let key_kind = match &obj_type {
            Some(ZigType::Anytype) | Some(ZigType::JsAny) => ComputedKeyKind::JsAnyGetByKey,
            Some(ZigType::NamedStruct(name)) if name == "Map" => ComputedKeyKind::MapGet,
            Some(ZigType::ArrayList(_)) => ComputedKeyKind::ArrayListItem,
            Some(ZigType::Struct(_)) | Some(ZigType::NamedStruct(_)) => {
                ComputedKeyKind::StructField
            }
            None => ComputedKeyKind::JsAnyGetByKey,
            _ => ComputedKeyKind::CompileError(format!(
                "computed access on unsupported type: {:?}",
                obj_type
            )),
        };
        IrExpr::ComputedField {
            object,
            key,
            key_kind,
        }
    }

    /// Infer the ZigType of an expression based on type_info and expression structure.
    /// This is a simplified version compared to Codegen's full inference.
    fn infer_expr_type(&self, expr: &Expression) -> Option<ZigType> {
        match expr {
            Expression::Identifier(id) => {
                // Try exact match, then qualified, then suffix-based (same as infer_arrow_expr_type)
                if let Some(ty) = self.type_info.var_types.get(id.name.as_str()) {
                    return Some(ty.clone());
                }
                if let Some(ctx) = self.fn_ctx.as_ref() {
                    let qualified = format!("{}::{}", ctx.name, id.name);
                    if let Some(ty) = self.type_info.var_types.get(&qualified) {
                        return Some(ty.clone());
                    }
                }
                let suffix = format!("::{}", id.name);
                for (k, v) in &self.type_info.var_types {
                    if k.ends_with(&suffix) {
                        return Some(v.clone());
                    }
                }
                None
            }
            Expression::NumericLiteral(_) => Some(ZigType::F64),
            Expression::StringLiteral(_) => Some(ZigType::Str),
            Expression::BooleanLiteral(_) => Some(ZigType::Bool),
            Expression::NullLiteral(_) => Some(ZigType::JsAny),
            // Could add more patterns here from Codegen's infer_expr_type
            _ => None,
        }
    }

    /// Check if an expression is a string type (for string concatenation detection).
    fn expr_is_string(&self, expr: &Expression) -> bool {
        match expr {
            Expression::StringLiteral(_) => true,
            Expression::TemplateLiteral(_) => true,
            Expression::Identifier(_id) => {
                // Use infer_expr_type which handles qualified name lookup
                self.infer_expr_type(expr) == Some(ZigType::Str)
            }
            Expression::BinaryExpression(be) if be.operator == BinaryOperator::Addition => {
                self.expr_is_string(&be.left) || self.expr_is_string(&be.right)
            }
            Expression::ParenthesizedExpression(pe) => self.expr_is_string(&pe.expression),
            _ => self.infer_expr_type(expr) == Some(ZigType::Str),
        }
    }

    /// Lower a string concatenation chain into IrExpr::AllocPrint.
    /// Flattens nested `a + b + c` into a single format string + args list.
    fn lower_string_concat(&mut self, be: &BinaryExpression) -> crate::zigir::types::IrExpr {
        use crate::zigir::types::IrExpr;

        let mut operands: Vec<&Expression> = Vec::new();
        Self::collect_concat_from_be(be, &mut operands);

        let mut fmt = String::new();
        let mut args: Vec<IrExpr> = Vec::new();

        for op in &operands {
            match op {
                Expression::StringLiteral(sl) => {
                    // Escape for Zig format string literal
                    for ch in sl.value.chars() {
                        match ch {
                            '\\' => fmt.push_str("\\\\"),
                            '"' => fmt.push_str("\\\""),
                            '\n' => fmt.push_str("\\n"),
                            '\r' => fmt.push_str("\\r"),
                            '\t' => fmt.push_str("\\t"),
                            '{' => fmt.push_str("{{"),
                            '}' => fmt.push_str("}}"),
                            c => fmt.push(c),
                        }
                    }
                }
                _ => {
                    // Pick placeholder based on inferred type
                    let placeholder = if self.expr_is_string(op) {
                        "{s}"
                    } else {
                        match self.infer_expr_type(op) {
                            Some(ZigType::Str) => "{s}",
                            Some(ZigType::I64) | Some(ZigType::F64) => "{d}",
                            Some(ZigType::Bool) => "{}",
                            _ => "{}",
                        }
                    };
                    fmt.push_str(placeholder);
                    // Unwrap parentheses before lowering
                    let lowered = match op {
                        Expression::ParenthesizedExpression(pe) => self.lower_expr(&pe.expression),
                        _ => self.lower_expr(op),
                    };
                    args.push(lowered);
                }
            }
        }

        IrExpr::AllocPrint { fmt, args }
    }

    /// Recursively collect all operands in a string concatenation chain.
    /// Only recurses into BinaryExpression(+); other nodes become leaves.
    fn collect_concat_from_be<'a>(be: &'a BinaryExpression<'a>, out: &mut Vec<&'a Expression<'a>>) {
        // Left side
        if let Expression::BinaryExpression(ref left_be) = be.left {
            if left_be.operator == BinaryOperator::Addition {
                Self::collect_concat_from_be(left_be, out);
            } else {
                out.push(&be.left);
            }
        } else {
            out.push(&be.left);
        }

        // Right side
        if let Expression::BinaryExpression(ref right_be) = be.right {
            if right_be.operator == BinaryOperator::Addition {
                Self::collect_concat_from_be(right_be, out);
            } else {
                out.push(&be.right);
            }
        } else {
            out.push(&be.right);
        }
    }

    /// Lower an array expression.
    fn lower_array_expr(&mut self, ae: &ArrayExpression) -> crate::zigir::types::IrExpr {
        let mut elements = Vec::new();
        let mut spread_indices = Vec::new();

        for (i, elem) in ae.elements.iter().enumerate() {
            match elem {
                ArrayExpressionElement::SpreadElement(se) => {
                    spread_indices.push(i);
                    elements.push(crate::zigir::types::IrExpr::Spread(Box::new(
                        self.lower_expr(&se.argument),
                    )));
                }
                ArrayExpressionElement::Elision(_) => {
                    elements.push(crate::zigir::types::IrExpr::Null);
                }
                _ => {
                    if let Some(expr) = elem.as_expression() {
                        elements.push(self.lower_expr(expr));
                    }
                }
            }
        }

        crate::zigir::types::IrExpr::ArrayLiteral(crate::zigir::types::IrArrayLiteral {
            elements,
            spread_indices,
        })
    }

    /// Lower an object expression.
    fn lower_object_expr(&mut self, oe: &ObjectExpression) -> crate::zigir::types::IrExpr {
        let mut fields = Vec::new();
        let mut spreads = Vec::new();

        for prop in oe.properties.iter() {
            match prop {
                ObjectPropertyKind::ObjectProperty(op) => {
                    let (key, is_computed) = match &op.key {
                        PropertyKey::StaticIdentifier(id) => (id.name.to_string(), false),
                        PropertyKey::StringLiteral(sl) => (sl.value.to_string(), false),
                        PropertyKey::NumericLiteral(nl) => (nl.value.to_string(), false),
                        _ => ("__computed__".to_string(), true),
                    };
                    let value = self.lower_expr(&op.value);
                    fields.push(crate::zigir::types::IrObjectField {
                        key,
                        value,
                        is_computed,
                    });
                }
                ObjectPropertyKind::SpreadProperty(sp) => {
                    spreads.push(self.lower_expr(&sp.argument));
                }
            }
        }

        crate::zigir::types::IrExpr::ObjectLiteral(crate::zigir::types::IrObjectLiteral {
            fields,
            spreads,
        })
    }

    /// Lower an arrow function expression.
    ///
    /// If the arrow captures variables from the enclosing scope, we produce
    /// an `IrClosure` (struct + instance).  Otherwise we produce a plain
    /// `IrArrowFn` (struct + static call — Zig 0.16 doesn't allow nested
    /// fn declarations with return statements).
    fn lower_arrow_fn(&mut self, af: &ArrowFunctionExpression) -> crate::zigir::types::IrExpr {
        use crate::zigir::types::{IrArrowFn, IrCapture, IrClosure, IrExpr};

        let captured = self.collect_arrow_captures(af);
        let is_concise = af.body.statements.len() == 1
            && matches!(af.body.statements[0], Statement::ExpressionStatement(_));
        let return_type = self.infer_arrow_return_type(af, &captured);
        let params = self.lower_arrow_params(af);

        // Enter a temporary fn context so that lower_stmt / lower_expr can
        // see captured-variable state.
        let arrow_fn_label = format!("_arrow_{}", self.name_mangler.next_name("arrow"));
        let saved_fn = self.enter_fn(&arrow_fn_label, false, Some(return_type.clone()));

        // Set closure_mgr.current_captured so that lower_ident_expr can
        // rewrite captured identifiers to self.xxx.
        let saved_captured = self.closure_mgr.take_captured();
        self.closure_mgr.current_captured = captured
            .iter()
            .map(|(n, t, m)| (n.clone(), t.clone(), *m))
            .collect();

        // Lower the body
        let body = if is_concise {
            if let Statement::ExpressionStatement(es) = &af.body.statements[0] {
                let expr_ir = self.lower_expr(&es.expression);
                IrBlock::new(vec![crate::zigir::types::IrStmt::Return {
                    value: Some(expr_ir),
                }])
            } else {
                self.lower_block(&af.body.statements)
            }
        } else {
            self.lower_block(&af.body.statements)
        };

        // Restore closure state
        self.closure_mgr.restore_captured(saved_captured);
        self.exit_fn(saved_fn);

        if !captured.is_empty() {
            // Has captures → IrClosure
            let idx = self.name_mangler.peek_count("closure");
            let struct_name = IrIdent::new(&format!("Closure_{}", idx));
            let instance_name = IrIdent::new(&format!("_cl_{}", idx));
            self.name_mangler.next_name("closure"); // advance counter

            let ir_captures: Vec<IrCapture> = captured
                .into_iter()
                .map(|(name, zig_type, is_mut)| IrCapture {
                    name: self.make_ident(&name),
                    zig_type,
                    is_mut,
                })
                .collect();

            // Register this as a closure instance
            self.closure_mgr
                .closure_instances
                .insert(instance_name.zig_name.clone());

            // Register the closure struct definition so the Emitter can emit it
            // at module scope.
            self.pending_arrow_structs
                .push(crate::zigir::types::IrClosureStruct {
                    name: struct_name.clone(),
                    captured: ir_captures.clone(),
                    fn_params: params.clone(),
                    return_type: return_type.clone(),
                    body: body.clone(),
                });

            IrExpr::Closure(IrClosure {
                struct_name,
                captured: ir_captures,
                fn_params: params,
                return_type,
                body,
                instance_name,
            })
        } else {
            // No captures → IrArrowFn
            IrExpr::ArrowFn(IrArrowFn {
                params,
                return_type,
                body,
                is_concise,
            })
        }
    }

    /// Lower a function expression.
    ///
    /// Like arrow functions, if the function captures variables we produce
    /// an `IrClosure`; otherwise a plain `IrFnExpr`.
    fn lower_fn_expr(&mut self, fe: &Function) -> crate::zigir::types::IrExpr {
        use crate::zigir::types::{IrCapture, IrClosure, IrExpr, IrFnExpr};

        let name = fe
            .id
            .as_ref()
            .map(|id| id.name.to_string())
            .unwrap_or_else(|| {
                let idx = self.name_mangler.peek_count("_fn_expr");
                self.name_mangler.next_name("_fn_expr"); // advance counter
                format!("_fn_expr_{}", idx)
            });

        let captured = self.detect_fn_body_captures(fe);
        let return_type = self
            .type_info
            .fn_return_types
            .get(&name)
            .cloned()
            .unwrap_or_else(|| self.infer_fn_expr_return_type(fe, &captured));

        // Enter function context
        let _has_throw = fe
            .body
            .as_ref()
            .is_some_and(|b| Self::has_throw_in_stmts(&b.statements));
        let saved_fn = self.enter_fn(&name, false, Some(return_type.clone()));

        // Set captured variables for identifier rewriting
        let saved_captured = self.closure_mgr.take_captured();
        self.closure_mgr.current_captured = captured
            .iter()
            .map(|(n, t, m)| (n.clone(), t.clone(), *m))
            .collect();

        // Lower params
        let params = self.lower_fn_params(fe, &name);

        // Lower body
        let body = fe
            .body
            .as_ref()
            .map(|b| self.lower_block(&b.statements))
            .unwrap_or_else(|| IrBlock::new(vec![]));

        // Restore
        self.closure_mgr.restore_captured(saved_captured);
        self.exit_fn(saved_fn);

        if !captured.is_empty() {
            // Has captures → IrClosure
            let struct_name = self.make_ident(&name);
            let instance_name = IrIdent::new(&format!("_{}_inst", name));

            let ir_captures: Vec<IrCapture> = captured
                .into_iter()
                .map(|(n, zig_type, is_mut)| IrCapture {
                    name: self.make_ident(&n),
                    zig_type,
                    is_mut,
                })
                .collect();

            self.closure_mgr
                .closure_instances
                .insert(instance_name.zig_name.clone());

            // Register the closure struct definition so the Emitter can emit it
            // at module scope.
            self.pending_arrow_structs
                .push(crate::zigir::types::IrClosureStruct {
                    name: struct_name.clone(),
                    captured: ir_captures.clone(),
                    fn_params: params.clone(),
                    return_type: return_type.clone(),
                    body: body.clone(),
                });

            IrExpr::Closure(IrClosure {
                struct_name,
                captured: ir_captures,
                fn_params: params,
                return_type,
                body,
                instance_name,
            })
        } else {
            // No captures → IrFnExpr
            IrExpr::FnExpr(IrFnExpr {
                name: Some(self.make_ident(&name)),
                params,
                return_type,
                body,
            })
        }
    }

    /// Collect all identifier names (js_name) referenced in an IR block.
    /// Used to determine which function parameters are unused.
    fn collect_ir_idents_in_block(block: &IrBlock) -> std::collections::HashSet<String> {
        let mut idents = std::collections::HashSet::new();
        for stmt in &block.stmts {
            Self::collect_ir_idents_in_stmt(stmt, &mut idents);
        }
        idents
    }

    fn collect_ir_idents_in_stmt(
        stmt: &crate::zigir::types::IrStmt,
        idents: &mut std::collections::HashSet<String>,
    ) {
        use crate::zigir::types::IrStmt;
        match stmt {
            IrStmt::VarDecl(vd) => {
                if let Some(init) = &vd.init {
                    Self::collect_ir_idents_in_expr(init, idents);
                }
            }
            IrStmt::Assign { target, value, .. } => {
                Self::collect_ir_idents_in_assign_target(target, idents);
                Self::collect_ir_idents_in_expr(value, idents);
            }
            IrStmt::If { cond, then, else_ } => {
                Self::collect_ir_idents_in_expr(cond, idents);
                for s in &then.stmts {
                    Self::collect_ir_idents_in_stmt(s, idents);
                }
                if let Some(e) = else_ {
                    for s in &e.stmts {
                        Self::collect_ir_idents_in_stmt(s, idents);
                    }
                }
            }
            IrStmt::While { cond, body, .. } | IrStmt::DoWhile { cond, body, .. } => {
                Self::collect_ir_idents_in_expr(cond, idents);
                for s in &body.stmts {
                    Self::collect_ir_idents_in_stmt(s, idents);
                }
            }
            IrStmt::For {
                init,
                cond,
                update,
                body,
                ..
            } => {
                if let Some(s) = init {
                    Self::collect_ir_idents_in_stmt(s, idents);
                }
                if let Some(e) = cond {
                    Self::collect_ir_idents_in_expr(e, idents);
                }
                if let Some(s) = update {
                    Self::collect_ir_idents_in_stmt(s, idents);
                }
                for s in &body.stmts {
                    Self::collect_ir_idents_in_stmt(s, idents);
                }
            }
            IrStmt::ForIn { iterable, body, .. } | IrStmt::ForOf { iterable, body, .. } => {
                Self::collect_ir_idents_in_expr(iterable, idents);
                for s in &body.stmts {
                    Self::collect_ir_idents_in_stmt(s, idents);
                }
            }
            IrStmt::Switch { expr, cases } => {
                Self::collect_ir_idents_in_expr(expr, idents);
                for c in cases {
                    if let Some(e) = &c.test {
                        Self::collect_ir_idents_in_expr(e, idents);
                    }
                    for s in &c.body {
                        Self::collect_ir_idents_in_stmt(s, idents);
                    }
                }
            }
            IrStmt::Try {
                try_block,
                catch_block,
                finally,
                ..
            } => {
                for s in &try_block.stmts {
                    Self::collect_ir_idents_in_stmt(s, idents);
                }
                for s in &catch_block.stmts {
                    Self::collect_ir_idents_in_stmt(s, idents);
                }
                if let Some(f) = finally {
                    for s in &f.stmts {
                        Self::collect_ir_idents_in_stmt(s, idents);
                    }
                }
            }
            IrStmt::Throw { value } => {
                Self::collect_ir_idents_in_expr(value, idents);
            }
            IrStmt::Return { value } => {
                if let Some(e) = value {
                    Self::collect_ir_idents_in_expr(e, idents);
                }
            }
            IrStmt::Break { .. } | IrStmt::Continue { .. } => {}
            IrStmt::Expr(e) => {
                Self::collect_ir_idents_in_expr(e, idents);
            }
            IrStmt::Block(b) => {
                for s in &b.stmts {
                    Self::collect_ir_idents_in_stmt(s, idents);
                }
            }
            IrStmt::CompileError { .. } | IrStmt::Comment(_) => {}
        }
    }

    fn collect_ir_idents_in_assign_target(
        target: &crate::zigir::types::IrAssignTarget,
        idents: &mut std::collections::HashSet<String>,
    ) {
        use crate::zigir::types::IrAssignTarget;
        match target {
            IrAssignTarget::Ident(name) => {
                idents.insert(name.js_name.clone());
            }
            IrAssignTarget::Member { object, .. } => {
                Self::collect_ir_idents_in_expr(object, idents);
            }
            IrAssignTarget::Index { object, index } => {
                Self::collect_ir_idents_in_expr(object, idents);
                Self::collect_ir_idents_in_expr(index, idents);
            }
            IrAssignTarget::Destructure(bindings) => {
                for b in bindings {
                    if let Some(d) = &b.default {
                        Self::collect_ir_idents_in_expr(d, idents);
                    }
                }
            }
        }
    }

    /// Collect identifier names from an AST expression (used for tracking
    /// references that are optimized away at compile time, e.g. typeof).
    fn collect_ast_expr_idents(expr: &oxc_ast::ast::Expression, idents: &mut HashSet<String>) {
        use oxc_ast::ast::Expression;
        match expr {
            Expression::Identifier(id) => {
                idents.insert(id.name.to_string());
            }
            Expression::BinaryExpression(be) => {
                Self::collect_ast_expr_idents(&be.left, idents);
                Self::collect_ast_expr_idents(&be.right, idents);
            }
            Expression::UnaryExpression(ue) => {
                Self::collect_ast_expr_idents(&ue.argument, idents);
            }
            Expression::CallExpression(ce) => {
                Self::collect_ast_expr_idents(&ce.callee, idents);
            }
            Expression::StaticMemberExpression(me) => {
                Self::collect_ast_expr_idents(&me.object, idents);
            }
            Expression::ComputedMemberExpression(me) => {
                Self::collect_ast_expr_idents(&me.object, idents);
            }
            Expression::ParenthesizedExpression(pe) => {
                Self::collect_ast_expr_idents(&pe.expression, idents);
            }
            _ => {}
        }
    }

    fn collect_ir_idents_in_expr(
        expr: &crate::zigir::types::IrExpr,
        idents: &mut std::collections::HashSet<String>,
    ) {
        use crate::zigir::types::IrExpr;
        match expr {
            IrExpr::Ident(name) => {
                idents.insert(name.js_name.clone());
            }
            IrExpr::Binary { left, right, .. } | IrExpr::Logical { left, right, .. } => {
                Self::collect_ir_idents_in_expr(left, idents);
                Self::collect_ir_idents_in_expr(right, idents);
            }
            IrExpr::Unary { operand, .. }
            | IrExpr::Typeof(operand)
            | IrExpr::Void(operand)
            | IrExpr::Paren(operand)
            | IrExpr::Spread(operand) => {
                Self::collect_ir_idents_in_expr(operand, idents);
            }
            IrExpr::Update { target, .. } => {
                Self::collect_ir_idents_in_assign_target(target, idents);
            }
            IrExpr::Assign { target, value, .. } => {
                Self::collect_ir_idents_in_assign_target(target, idents);
                Self::collect_ir_idents_in_expr(value, idents);
            }
            IrExpr::Call(call) => {
                Self::collect_ir_idents_in_expr(&call.callee, idents);
                for a in &call.args {
                    Self::collect_ir_idents_in_expr(a, idents);
                }
            }
            IrExpr::BuiltinCall(bc) => {
                for a in &bc.args {
                    Self::collect_ir_idents_in_expr(a, idents);
                }
            }
            IrExpr::HostCall(hc) => {
                for a in &hc.args {
                    Self::collect_ir_idents_in_expr(a, idents);
                }
            }
            IrExpr::FieldAccess { object, .. }
            | IrExpr::IndexAccess { object, .. }
            | IrExpr::ComputedField { object, .. } => {
                Self::collect_ir_idents_in_expr(object, idents);
                if let IrExpr::IndexAccess { index, .. } = expr {
                    Self::collect_ir_idents_in_expr(index, idents);
                }
                if let IrExpr::ComputedField { key, .. } = expr {
                    Self::collect_ir_idents_in_expr(key, idents);
                }
            }
            IrExpr::Conditional { cond, then, else_ } => {
                Self::collect_ir_idents_in_expr(cond, idents);
                Self::collect_ir_idents_in_expr(then, idents);
                Self::collect_ir_idents_in_expr(else_, idents);
            }
            IrExpr::Closure(c) => {
                for cap in &c.captured {
                    idents.insert(cap.name.js_name.clone());
                }
                for s in &c.body.stmts {
                    Self::collect_ir_idents_in_stmt(s, idents);
                }
            }
            IrExpr::ArrowFn(a) => {
                for s in &a.body.stmts {
                    Self::collect_ir_idents_in_stmt(s, idents);
                }
            }
            IrExpr::FnExpr(f) => {
                for s in &f.body.stmts {
                    Self::collect_ir_idents_in_stmt(s, idents);
                }
            }
            IrExpr::ArrayLiteral(al) => {
                for e in &al.elements {
                    Self::collect_ir_idents_in_expr(e, idents);
                }
            }
            IrExpr::ObjectLiteral(ol) => {
                for f in &ol.fields {
                    Self::collect_ir_idents_in_expr(&f.value, idents);
                }
            }
            IrExpr::New(ne) => {
                for a in &ne.args {
                    Self::collect_ir_idents_in_expr(a, idents);
                }
            }
            IrExpr::TemplateLiteral { exprs, .. } => {
                for e in exprs {
                    Self::collect_ir_idents_in_expr(e, idents);
                }
            }
            IrExpr::AllocPrint { args, .. } => {
                for a in args {
                    Self::collect_ir_idents_in_expr(a, idents);
                }
            }
            IrExpr::BlockExpr { body, result, .. } => {
                for s in body {
                    Self::collect_ir_idents_in_stmt(s, idents);
                }
                Self::collect_ir_idents_in_expr(result, idents);
            }
            IrExpr::Sequence(exprs) => {
                for e in exprs {
                    Self::collect_ir_idents_in_expr(e, idents);
                }
            }
            IrExpr::Await(ae) => {
                Self::collect_ir_idents_in_expr(&ae.callee, idents);
                for a in &ae.args {
                    Self::collect_ir_idents_in_expr(a, idents);
                }
            }
            IrExpr::ArrayCallbackInline(inline_data) => {
                for s in &inline_data.body {
                    Self::collect_ir_idents_in_stmt(s, idents);
                }
                if let Some(ref init) = inline_data.reduce_init {
                    Self::collect_ir_idents_in_expr(init, idents);
                }
            }
            IrExpr::ArrayMethodInline(inline_data) => {
                for arg in &inline_data.args {
                    Self::collect_ir_idents_in_expr(arg, idents);
                }
            }
            IrExpr::IntLiteral(_)
            | IrExpr::FloatLiteral(_)
            | IrExpr::StringLiteral(_)
            | IrExpr::BoolLiteral(_)
            | IrExpr::Null
            | IrExpr::Undefined
            | IrExpr::This
            | IrExpr::CompileError { .. } => {}
        }
    }

    /// Lower a template literal.
    fn lower_template_literal(&mut self, tl: &TemplateLiteral) -> crate::zigir::types::IrExpr {
        let parts: Vec<String> = tl.quasis.iter().map(|q| q.value.raw.to_string()).collect();
        let exprs: Vec<crate::zigir::types::IrExpr> =
            tl.expressions.iter().map(|e| self.lower_expr(e)).collect();

        // Determine the Zig format specifier for each interpolation expression.
        // This must match Codegen's logic:
        //   Str→{s}, I64/F64→{d}, Bool→{}, other→expr_is_string?{s}:{}
        let format_specs: Vec<String> = tl
            .expressions
            .iter()
            .map(|expr| match self.infer_expr_type(expr) {
                Some(ZigType::Str) => "{s}".to_string(),
                Some(ZigType::I64) | Some(ZigType::F64) => "{d}".to_string(),
                Some(ZigType::Bool) => "{}".to_string(),
                _ => {
                    if self.expr_is_string(expr) {
                        "{s}".to_string()
                    } else {
                        "{}".to_string()
                    }
                }
            })
            .collect();

        crate::zigir::types::IrExpr::TemplateLiteral {
            parts,
            exprs,
            format_specs,
        }
    }

    /// Lower an await expression.
    fn lower_await(&mut self, ae: &AwaitExpression) -> crate::zigir::types::IrExpr {
        // Simplified: wrap in IrAwaitExpr
        // Full implementation (Task 1.14) needs async frame + block label
        let argument = self.lower_expr(&ae.argument);
        crate::zigir::types::IrExpr::Await(crate::zigir::types::IrAwaitExpr {
            task_var: IrIdent::new("__task"),
            callee: Box::new(argument),
            args: vec![],
            is_host_async: false,
            block_label: "__await_blk".to_string(),
        })
    }

    /// Lower a new expression.
    fn lower_new(&mut self, ne: &NewExpression) -> crate::zigir::types::IrExpr {
        use crate::zigir::kinds::NewConstructor;

        // Determine constructor kind from callee
        let constructor = match &ne.callee {
            Expression::Identifier(id) => match id.name.as_str() {
                "Map" => NewConstructor::Map,
                "Set" => NewConstructor::Set,
                "Date" => NewConstructor::Date(crate::zigir::kinds::DateConstructorKind::Now),
                "RegExp" => NewConstructor::RegExp,
                "Int8Array" | "Uint8Array" | "Uint8ClampedArray" | "Int16Array" | "Uint16Array"
                | "Int32Array" | "Uint32Array" | "Float32Array" | "Float64Array" => {
                    let kind = match id.name.as_str() {
                        "Int8Array" => crate::zigir::kinds::TypedArrayKind::Int8Array,
                        "Uint8Array" => crate::zigir::kinds::TypedArrayKind::Uint8Array,
                        "Uint8ClampedArray" => {
                            crate::zigir::kinds::TypedArrayKind::Uint8ClampedArray
                        }
                        "Int16Array" => crate::zigir::kinds::TypedArrayKind::Int16Array,
                        "Uint16Array" => crate::zigir::kinds::TypedArrayKind::Uint16Array,
                        "Int32Array" => crate::zigir::kinds::TypedArrayKind::Int32Array,
                        "Uint32Array" => crate::zigir::kinds::TypedArrayKind::Uint32Array,
                        "Float32Array" => crate::zigir::kinds::TypedArrayKind::Float32Array,
                        "Float64Array" => crate::zigir::kinds::TypedArrayKind::Float64Array,
                        _ => crate::zigir::kinds::TypedArrayKind::Float64Array,
                    };
                    NewConstructor::TypedArray(kind)
                }
                "Error" => NewConstructor::Error("Error".to_string()),
                "TypeError" => NewConstructor::Error("TypeError".to_string()),
                "RangeError" => NewConstructor::Error("RangeError".to_string()),
                name if self.class_names.contains(name) => NewConstructor::Class(name.to_string()),
                _ => {
                    let span = oxc_span::GetSpan::span(ne);
                    return crate::zigir::types::IrExpr::CompileError {
                        span: self.span_to_source_span(span),
                        msg: "Unsupported NewExpression".to_string(),
                    };
                }
            },
            _ => {
                let span = oxc_span::GetSpan::span(ne);
                return crate::zigir::types::IrExpr::CompileError {
                    span: self.span_to_source_span(span),
                    msg: "Unsupported NewExpression".to_string(),
                };
            }
        };

        let args: Vec<crate::zigir::types::IrExpr> = ne
            .arguments
            .iter()
            .map(|arg| match arg {
                Argument::SpreadElement(se) => {
                    crate::zigir::types::IrExpr::Spread(Box::new(self.lower_expr(&se.argument)))
                }
                _ => {
                    let expr = arg.as_expression().unwrap();
                    self.lower_expr(expr)
                }
            })
            .collect();

        let result_type = match &constructor {
            NewConstructor::Map => ZigType::NamedStruct("Map".to_string()),
            NewConstructor::Set => ZigType::NamedStruct("Set".to_string()),
            NewConstructor::Date(_) => ZigType::NamedStruct("JsDate".to_string()),
            NewConstructor::RegExp => ZigType::JsAny,
            NewConstructor::TypedArray(_) => ZigType::NamedStruct("TypedArray".to_string()),
            NewConstructor::Class(name) => ZigType::NamedStruct(name.clone()),
            NewConstructor::Error(_) => ZigType::JsAny,
            _ => ZigType::JsAny,
        };

        crate::zigir::types::IrExpr::New(crate::zigir::types::IrNewExpr {
            constructor,
            args,
            result_type,
        })
    }
}

// ═══════════════════════════════════════════════════════
//  Closure struct lowering
// ═══════════════════════════════════════════════════════

impl Lowerer {
    /// Convert collected closure definitions from ClosureManager
    /// into IrClosureStruct nodes.
    ///
    /// In the old Codegen these were string snippets prepended to output.
    /// In ZigIR they become structured IrClosureStruct nodes.
    ///
    /// After lowering, `closure_mgr.closure_vars` contains the mapping from
    /// struct name → captured vars that was built during `lower_arrow_fn` /
    /// `lower_fn_expr`.  We produce one `IrClosureStruct` per entry.
    fn lower_closure_structs(&self) -> Vec<crate::zigir::types::IrClosureStruct> {
        self.closure_mgr
            .closure_vars
            .iter()
            .map(|(struct_name, captured)| {
                let ir_captures: Vec<crate::zigir::types::IrCapture> = captured
                    .iter()
                    .map(|(name, zig_type, is_mut)| crate::zigir::types::IrCapture {
                        name: self.make_ident(name),
                        zig_type: zig_type.clone(),
                        is_mut: *is_mut,
                    })
                    .collect();
                crate::zigir::types::IrClosureStruct {
                    name: self.make_ident(struct_name),
                    captured: ir_captures,
                    fn_params: vec![], // Will be filled by the Emitter from the IrClosure
                    return_type: ZigType::Void,
                    body: IrBlock::new(vec![]),
                }
            })
            .collect()
    }
}

// ═══════════════════════════════════════════════════════
//  Closure capture analysis
// ═══════════════════════════════════════════════════════

impl Lowerer {
    /// Collect captured variables from an arrow function body.
    ///
    /// A variable is "captured" if it's referenced in the body but is not a
    /// parameter and not a locally declared variable.
    fn collect_arrow_captures(
        &self,
        arrow: &ArrowFunctionExpression,
    ) -> Vec<(String, ZigType, bool)> {
        let mut captured = Vec::new();
        let mut seen = std::collections::HashSet::new();

        // Collect parameter names + locally declared variable names
        let mut local_names: std::collections::HashSet<String> = arrow
            .params
            .items
            .iter()
            .filter_map(|p| crate::infer::binding_name(&p.pattern))
            .map(|s| s.to_string())
            .collect();
        local_names.extend(Self::collect_local_declarations(&arrow.body.statements));

        // Walk the body statements to find Identifier references
        for stmt in &arrow.body.statements {
            Self::collect_idents_from_stmt(
                stmt,
                &mut captured,
                &mut seen,
                &local_names,
                &self.type_info,
            );
        }

        // Detect which captured variables are mutated in the arrow body
        let mutated = Self::detect_mutated_vars_in_stmts(&arrow.body.statements);
        for (name, _ztype, is_mut) in &mut captured {
            *is_mut = mutated.contains(name);
        }

        captured
    }

    /// Detect variables captured by a nested function (declaration or expression).
    ///
    /// Returns list of (variable_name, ZigType, is_mutable) for variables from
    /// the enclosing scope that are referenced in the function body.
    fn detect_fn_body_captures(&self, fd: &Function) -> Vec<(String, ZigType, bool)> {
        let mut captured = Vec::new();
        let mut seen = std::collections::HashSet::new();

        let mut local_names: std::collections::HashSet<String> = fd
            .params
            .items
            .iter()
            .filter_map(|p| crate::infer::binding_name(&p.pattern))
            .map(|s| s.to_string())
            .collect();

        if let Some(body) = &fd.body {
            local_names.extend(Self::collect_local_declarations(&body.statements));
            for stmt in &body.statements {
                Self::collect_idents_from_stmt(
                    stmt,
                    &mut captured,
                    &mut seen,
                    &local_names,
                    &self.type_info,
                );
            }
            let mutated = Self::detect_mutated_vars_in_stmts(&body.statements);
            for (name, _ztype, is_mut) in &mut captured {
                *is_mut = mutated.contains(name);
            }
        }

        captured
    }

    /// Collect locally declared variable names from a list of statements.
    /// These variables (const/let/var in the function body) are NOT captures.
    fn collect_local_declarations(
        stmts: &oxc_allocator::Vec<'_, Statement>,
    ) -> std::collections::HashSet<String> {
        let mut names = std::collections::HashSet::new();
        for stmt in stmts.iter() {
            if let Statement::VariableDeclaration(var_decl) = stmt {
                for declarator in &var_decl.declarations {
                    if let Some(name) = crate::infer::binding_name(&declarator.id) {
                        names.insert(name.to_string());
                    }
                }
            }
        }
        names
    }

    /// Detect which variables are mutated (assigned to or updated) in a list of statements.
    fn detect_mutated_vars_in_stmts(stmts: &[Statement]) -> std::collections::HashSet<String> {
        let mut mutated = std::collections::HashSet::new();
        for stmt in stmts {
            Self::detect_mutated_in_stmt(stmt, &mut mutated);
        }
        mutated
    }

    fn detect_mutated_in_stmt(stmt: &Statement, mutated: &mut std::collections::HashSet<String>) {
        match stmt {
            Statement::ExpressionStatement(es) => {
                Self::detect_mutated_in_expr(&es.expression, mutated);
            }
            Statement::ReturnStatement(rs) => {
                if let Some(expr) = &rs.argument {
                    Self::detect_mutated_in_expr(expr, mutated);
                }
            }
            Statement::BlockStatement(bs) => {
                for s in &bs.body {
                    Self::detect_mutated_in_stmt(s, mutated);
                }
            }
            Statement::IfStatement(is) => {
                Self::detect_mutated_in_expr(&is.test, mutated);
                Self::detect_mutated_in_stmt(&is.consequent, mutated);
                if let Some(alt) = &is.alternate {
                    Self::detect_mutated_in_stmt(alt, mutated);
                }
            }
            Statement::WhileStatement(ws) => {
                Self::detect_mutated_in_expr(&ws.test, mutated);
                Self::detect_mutated_in_stmt(&ws.body, mutated);
            }
            Statement::ForStatement(fs) => {
                if let Some(test) = &fs.test {
                    Self::detect_mutated_in_expr(test, mutated);
                }
                if let Some(update) = &fs.update {
                    Self::detect_mutated_in_expr(update, mutated);
                }
                Self::detect_mutated_in_stmt(&fs.body, mutated);
            }
            Statement::ForOfStatement(fos) => {
                Self::detect_mutated_in_expr(&fos.right, mutated);
                Self::detect_mutated_in_stmt(&fos.body, mutated);
            }
            Statement::ForInStatement(fis) => {
                Self::detect_mutated_in_expr(&fis.right, mutated);
                Self::detect_mutated_in_stmt(&fis.body, mutated);
            }
            Statement::SwitchStatement(ss) => {
                Self::detect_mutated_in_expr(&ss.discriminant, mutated);
                for case in &ss.cases {
                    for s in &case.consequent {
                        Self::detect_mutated_in_stmt(s, mutated);
                    }
                }
            }
            Statement::TryStatement(ts) => {
                for s in &ts.block.body {
                    Self::detect_mutated_in_stmt(s, mutated);
                }
                if let Some(handler) = &ts.handler {
                    for s in &handler.body.body {
                        Self::detect_mutated_in_stmt(s, mutated);
                    }
                }
                if let Some(finalizer) = &ts.finalizer {
                    for s in &finalizer.body {
                        Self::detect_mutated_in_stmt(s, mutated);
                    }
                }
            }
            _ => {}
        }
    }

    fn detect_mutated_in_expr(expr: &Expression, mutated: &mut std::collections::HashSet<String>) {
        match expr {
            Expression::AssignmentExpression(ae) => {
                if let AssignmentTarget::AssignmentTargetIdentifier(id) = &ae.left {
                    mutated.insert(id.name.to_string());
                }
                Self::detect_mutated_in_expr(&ae.right, mutated);
            }
            Expression::UpdateExpression(ue) => {
                if let SimpleAssignmentTarget::AssignmentTargetIdentifier(id) = &ue.argument {
                    mutated.insert(id.name.to_string());
                }
            }
            Expression::BinaryExpression(be) => {
                Self::detect_mutated_in_expr(&be.left, mutated);
                Self::detect_mutated_in_expr(&be.right, mutated);
            }
            Expression::CallExpression(ce) => {
                Self::detect_mutated_in_expr(&ce.callee, mutated);
                for arg in &ce.arguments {
                    if let Some(e) = arg.as_expression() {
                        Self::detect_mutated_in_expr(e, mutated);
                    }
                }
            }
            Expression::LogicalExpression(le) => {
                Self::detect_mutated_in_expr(&le.left, mutated);
                Self::detect_mutated_in_expr(&le.right, mutated);
            }
            Expression::ConditionalExpression(ce) => {
                Self::detect_mutated_in_expr(&ce.test, mutated);
                Self::detect_mutated_in_expr(&ce.consequent, mutated);
                Self::detect_mutated_in_expr(&ce.alternate, mutated);
            }
            Expression::UnaryExpression(ue) => {
                Self::detect_mutated_in_expr(&ue.argument, mutated);
            }
            Expression::AwaitExpression(ae) => {
                Self::detect_mutated_in_expr(&ae.argument, mutated);
            }
            _ => {}
        }
    }

    /// Helper: collect identifiers from a statement that reference variables
    /// in an enclosing scope (possible captures).
    fn collect_idents_from_stmt(
        stmt: &Statement,
        captured: &mut Vec<(String, ZigType, bool)>,
        seen: &mut std::collections::HashSet<String>,
        local_names: &std::collections::HashSet<String>,
        type_info: &crate::infer::TypeCheckResult,
    ) {
        match stmt {
            Statement::ExpressionStatement(es) => {
                Self::collect_idents_from_expr(
                    &es.expression,
                    captured,
                    seen,
                    local_names,
                    type_info,
                );
            }
            Statement::ReturnStatement(ret) => {
                if let Some(expr) = &ret.argument {
                    Self::collect_idents_from_expr(expr, captured, seen, local_names, type_info);
                }
            }
            Statement::VariableDeclaration(var_decl) => {
                // Process init expressions (right-hand side) — they may reference
                // outer variables that need to be captured.
                for declarator in &var_decl.declarations {
                    if let Some(init) = &declarator.init {
                        Self::collect_idents_from_expr(
                            init,
                            captured,
                            seen,
                            local_names,
                            type_info,
                        );
                    }
                }
            }
            _ => {}
        }
    }

    /// Helper: collect identifiers from an expression that reference variables
    /// in an enclosing scope.
    fn collect_idents_from_expr(
        expr: &Expression,
        captured: &mut Vec<(String, ZigType, bool)>,
        seen: &mut std::collections::HashSet<String>,
        local_names: &std::collections::HashSet<String>,
        type_info: &crate::infer::TypeCheckResult,
    ) {
        match expr {
            Expression::Identifier(id) => {
                let name = id.name.as_str();
                if !local_names.contains(name)
                    && !seen.contains(name)
                    && !crate::native_builtins::is_js_builtin_identifier(name)
                {
                    seen.insert(name.to_string());
                    let ztype = type_info
                        .var_types
                        .get(name)
                        .cloned()
                        .unwrap_or(ZigType::I64);
                    captured.push((name.to_string(), ztype, false));
                }
            }
            Expression::BinaryExpression(be) => {
                Self::collect_idents_from_expr(&be.left, captured, seen, local_names, type_info);
                Self::collect_idents_from_expr(&be.right, captured, seen, local_names, type_info);
            }
            Expression::CallExpression(ce) => {
                for arg in &ce.arguments {
                    if let Some(e) = arg.as_expression() {
                        Self::collect_idents_from_expr(e, captured, seen, local_names, type_info);
                    }
                }
                Self::collect_idents_from_expr(&ce.callee, captured, seen, local_names, type_info);
            }
            _ => {}
        }
    }

    /// Lower arrow function parameters into IrParam list.
    fn lower_arrow_params(&mut self, arrow: &ArrowFunctionExpression) -> Vec<IrParam> {
        let mut params = Vec::new();
        for param in &arrow.params.items {
            if let Some(pname) = crate::infer::binding_name(&param.pattern) {
                let ptype = self
                    .type_info
                    .var_types
                    .get(pname)
                    .cloned()
                    .unwrap_or(ZigType::Anytype);
                params.push(IrParam {
                    name: self.make_ident(pname),
                    zig_type: ptype,
                    is_unused: false,
                });
            }
        }
        params
    }

    /// Infer the return type of an arrow function.
    fn infer_arrow_return_type(
        &self,
        arrow: &ArrowFunctionExpression,
        captured: &[(String, ZigType, bool)],
    ) -> ZigType {
        // Single-expression arrow: type is the expression's type
        if arrow.body.statements.len() == 1
            && let Statement::ExpressionStatement(es) = &arrow.body.statements[0]
        {
            return self
                .infer_arrow_expr_type_with_captures(&es.expression, captured)
                .unwrap_or(ZigType::I64);
        }
        // Block body: scan return statements
        for stmt in &arrow.body.statements {
            if let Statement::ReturnStatement(rs) = stmt {
                if let Some(ref arg) = rs.argument {
                    return self
                        .infer_arrow_expr_type_with_captures(arg, captured)
                        .unwrap_or(ZigType::I64);
                }
                return ZigType::Void; // bare `return;` means void
            }
        }
        ZigType::Void // no return → void
    }

    /// Infer the return type of a function expression by scanning return statements.
    /// `captured` provides a fallback type lookup for captured variables that might
    /// not be in `var_types` (e.g., when the variable's type is `anytype`-derived).
    fn infer_fn_expr_return_type(
        &self,
        fe: &Function,
        captured: &[(String, ZigType, bool)],
    ) -> ZigType {
        if let Some(body) = &fe.body {
            for stmt in &body.statements {
                if let Statement::ReturnStatement(rs) = stmt {
                    if let Some(ref arg) = rs.argument {
                        return self
                            .infer_arrow_expr_type_with_captures(arg, captured)
                            .unwrap_or(ZigType::Void);
                    }
                    return ZigType::Void;
                }
            }
        }
        ZigType::Void
    }

    /// Best-effort type inference for arrow body expressions.
    /// Delegates to `infer_arrow_expr_type_with_captures` with an empty capture list.
    #[allow(dead_code)]
    fn infer_arrow_expr_type(&self, expr: &Expression) -> Option<ZigType> {
        self.infer_arrow_expr_type_with_captures(expr, &[])
    }

    /// Best-effort type inference with captured variable fallback.
    /// When a captured variable's type isn't in `var_types` (e.g., the variable
    /// derives from an `anytype` parameter), we can look it up from the capture
    /// list which was populated by `detect_fn_body_captures`.
    fn infer_arrow_expr_type_with_captures(
        &self,
        expr: &Expression,
        captured: &[(String, ZigType, bool)],
    ) -> Option<ZigType> {
        match expr {
            Expression::NumericLiteral(nl) => {
                if let Some(raw) = &nl.raw {
                    let s = raw.as_str();
                    if s.contains('.') || s.contains('e') || s.contains('E') {
                        Some(ZigType::F64)
                    } else {
                        Some(ZigType::I64)
                    }
                } else {
                    Some(ZigType::I64)
                }
            }
            Expression::StringLiteral(_) => Some(ZigType::Str),
            Expression::BooleanLiteral(_) => Some(ZigType::Bool),
            Expression::Identifier(id) => {
                // Try exact match first (simple name like "count")
                if let Some(ty) = self.type_info.var_types.get(id.name.as_str()) {
                    return Some(ty.clone());
                }
                // Try qualified name with current function prefix
                if let Some(ctx) = self.fn_ctx.as_ref() {
                    let qualified = format!("{}::{}", ctx.name, id.name);
                    if let Some(ty) = self.type_info.var_types.get(&qualified) {
                        return Some(ty.clone());
                    }
                }
                // Suffix-based fallback (covers nested scopes)
                let suffix = format!("::{}", id.name);
                for (k, v) in &self.type_info.var_types {
                    if k.ends_with(&suffix) {
                        return Some(v.clone());
                    }
                }
                // Fallback: check captured variable list (handles anytype-derived vars)
                for (name, ty, _is_mut) in captured {
                    if name == id.name.as_str() {
                        return Some(ty.clone());
                    }
                }
                None
            }
            Expression::BinaryExpression(be) => self
                .infer_arrow_expr_type_with_captures(&be.left, captured)
                .or_else(|| self.infer_arrow_expr_type_with_captures(&be.right, captured)),
            Expression::UnaryExpression(ue) => {
                self.infer_arrow_expr_type_with_captures(&ue.argument, captured)
            }
            Expression::CallExpression(ce) => {
                if let Expression::Identifier(id) = &ce.callee {
                    self.type_info
                        .fn_return_types
                        .get(id.name.as_str())
                        .cloned()
                } else {
                    None
                }
            }
            Expression::StaticMemberExpression(sme) => {
                let field = sme.property.name.as_str();
                match field {
                    "length" | "len" => Some(ZigType::I64),
                    _ => None,
                }
            }
            Expression::ConditionalExpression(ce) => self
                .infer_arrow_expr_type_with_captures(&ce.consequent, captured)
                .or_else(|| self.infer_arrow_expr_type_with_captures(&ce.alternate, captured)),
            _ => None,
        }
    }

    /// Check whether a list of statements contains a `throw`.
    fn has_throw_in_stmts(stmts: &oxc_allocator::Vec<'_, Statement>) -> bool {
        for stmt in stmts {
            if Self::has_throw_in_stmt(stmt) {
                return true;
            }
        }
        false
    }

    fn has_throw_in_stmt(stmt: &Statement) -> bool {
        match stmt {
            Statement::ThrowStatement(_) => true,
            Statement::BlockStatement(bs) => Self::has_throw_in_stmts(&bs.body),
            Statement::IfStatement(is) => {
                Self::has_throw_in_stmt(&is.consequent)
                    || is
                        .alternate
                        .as_ref()
                        .is_some_and(|a| Self::has_throw_in_stmt(a))
            }
            Statement::SwitchStatement(ss) => ss
                .cases
                .iter()
                .any(|c| c.consequent.iter().any(|s| Self::has_throw_in_stmt(s))),
            Statement::TryStatement(ts) => Self::has_throw_in_stmts(&ts.block.body),
            Statement::WhileStatement(ws) => Self::has_throw_in_stmt(&ws.body),
            Statement::ForStatement(fs) => Self::has_throw_in_stmt(&fs.body),
            _ => false,
        }
    }
}

// ═══════════════════════════════════════════════════════
//  CABI export metadata
// ═══════════════════════════════════════════════════════

impl Lowerer {
    /// Build C ABI export metadata from the lowered declarations.
    ///
    /// Corresponds to the cabi_exports construction in old
    /// `native_proto::transpile_js_inner()`.
    fn build_cabi_exports(&self, declarations: &[IrDecl]) -> Vec<IrCabiExport> {
        let mut exports = Vec::new();
        for decl in declarations {
            if let IrDecl::Fn(f) = decl
                && f.is_export
            {
                let params: Vec<IrParam> = f
                    .params
                    .iter()
                    .map(|p| IrParam {
                        name: p.name.clone(),
                        zig_type: p.zig_type.clone(),
                        is_unused: p.is_unused,
                    })
                    .collect();
                let ret_struct_name =
                    if let crate::types::ZigType::NamedStruct(ref s) = f.return_type {
                        Some(s.clone())
                    } else {
                        None
                    };
                exports.push(IrCabiExport {
                    name: f.name.zig_name.clone(),
                    params,
                    return_type: f.return_type.clone(),
                    is_async: f.is_async,
                    can_throw: f.can_throw,
                    ret_struct_name,
                });
            }
        }
        exports
    }
}

// ═══════════════════════════════════════════════════════
//  Utility methods
// ═══════════════════════════════════════════════════════

impl Lowerer {
    /// Check whether a function name is in the exported set.
    fn is_export_fn(&self, fn_name: Option<&str>) -> bool {
        if let Some(ref exported) = self.exported_functions {
            fn_name.is_some_and(|name| exported.contains(name))
        } else {
            false
        }
    }

    /// Convert a `Span` to an `IrDiagnostic` with source location.
    fn span_to_source_span(&self, span: oxc_span::Span) -> SourceSpan {
        let offset = span.start as usize;
        let mut line: usize = 1;
        let mut col: usize = 1;
        for (i, ch) in self.source.char_indices() {
            if i >= offset {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
        SourceSpan {
            js_line: line,
            js_col: col,
            js_file: String::new(),
        }
    }

    /// Add an error diagnostic.
    fn add_error(&mut self, span: SourceSpan, msg: impl Into<String>) {
        self.diagnostics.push(IrDiagnostic {
            level: DiagnosticLevel::Error,
            span: Some(span),
            message: msg.into(),
        });
    }

    /// Add a warning diagnostic.
    #[allow(dead_code)]
    fn add_warning(&mut self, span: SourceSpan, msg: impl Into<String>) {
        self.diagnostics.push(IrDiagnostic {
            level: DiagnosticLevel::Warning,
            span: Some(span),
            message: msg.into(),
        });
    }

    /// Resolve a JS identifier name through the NameMangler (keyword escaping + shadow).
    #[allow(dead_code)]
    fn resolve_name(&self, name: &str) -> String {
        self.name_mangler.resolve_name(name)
    }

    /// Create an IrIdent for the given JS name, applying shadow renaming.
    fn make_ident(&self, js_name: &str) -> IrIdent {
        self.name_mangler.make_ident(js_name)
    }

    /// Try to inline an array non-callback method (includes, indexOf, lastIndexOf,
    /// join, slice, splice, at, concat, copyWithin, fill) when we have the
    /// object variable name. Returns `IrExpr::ArrayMethodInline` if inlinable.
    fn try_inline_array_method(
        &self,
        ce: &CallExpression,
        builtin: &crate::native_builtins::BuiltinCall,
        args: &[crate::zigir::types::IrExpr],
    ) -> Option<crate::zigir::types::IrExpr> {
        use crate::native_builtins::BuiltinCall as BC;
        use crate::zigir::types::{ArrayMethodKind, IrArrayMethodInline, IrExpr};

        let kind = match builtin {
            BC::ArrayIncludes => ArrayMethodKind::Includes,
            BC::ArrayIndexOf => ArrayMethodKind::IndexOf,
            BC::ArrayLastIndexOf => ArrayMethodKind::LastIndexOf,
            BC::ArrayJoin => ArrayMethodKind::Join,
            BC::ArraySlice => ArrayMethodKind::Slice,
            BC::ArraySplice => ArrayMethodKind::Splice,
            BC::ArrayAt => ArrayMethodKind::At,
            BC::ArrayConcat => ArrayMethodKind::Concat,
            BC::ArrayCopyWithin => ArrayMethodKind::CopyWithin,
            BC::ArrayFill => ArrayMethodKind::Fill,
            _ => return None,
        };

        let obj_name = Self::extract_callee_object_name_static(&ce.callee)?;

        let elem_type = self
            .type_info
            .array_element_types
            .get(obj_name.as_str())
            .cloned()
            .unwrap_or(ZigType::I64);

        Some(IrExpr::ArrayMethodInline(Box::new(IrArrayMethodInline {
            kind,
            obj_name,
            elem_type,
            args: args.to_vec(),
        })))
    }

    /// Try to inline an array callback method (forEach, some, every, filter, find,
    /// findIndex, findLast, findLastIndex, map, reduce) when the first argument
    /// is an ArrowFunctionExpression or FunctionExpression.
    ///
    /// Returns `IrExpr::ArrayCallbackInline` if inlinable, `None` otherwise.
    fn try_inline_array_callback(
        &mut self,
        ce: &CallExpression,
        builtin: &crate::native_builtins::BuiltinCall,
    ) -> Option<crate::zigir::types::IrExpr> {
        use crate::native_builtins::BuiltinCall as BC;
        use crate::zigir::types::{ArrayCallbackKind, IrArrayCallbackInline, IrExpr};

        let kind = match builtin {
            BC::ArrayForEach => ArrayCallbackKind::ForEach,
            BC::ArraySome => ArrayCallbackKind::Some,
            BC::ArrayEvery => ArrayCallbackKind::Every,
            BC::ArrayFilter => ArrayCallbackKind::Filter,
            BC::ArrayFind => ArrayCallbackKind::Find,
            BC::ArrayFindIndex => ArrayCallbackKind::FindIndex,
            BC::ArrayFindLast => ArrayCallbackKind::FindLast,
            BC::ArrayFindLastIndex => ArrayCallbackKind::FindLastIndex,
            BC::ArrayMap => ArrayCallbackKind::Map,
            BC::ArrayReduce => ArrayCallbackKind::Reduce,
            _ => return None,
        };

        let first_arg = ce.arguments.first()?.as_expression()?;

        let (params, body) = match first_arg {
            Expression::ArrowFunctionExpression(arrow) => (&arrow.params, &arrow.body),
            Expression::FunctionExpression(f) => match &f.body {
                Some(b) => (&f.params, b),
                None => return None,
            },
            _ => return None,
        };

        let elem_param_raw = params
            .items
            .first()
            .and_then(|p| crate::infer::binding_name(&p.pattern))
            .unwrap_or("_")
            .to_string();
        let idx_param_raw = params
            .items
            .get(1)
            .and_then(|p| crate::infer::binding_name(&p.pattern));
        let has_idx_param = idx_param_raw.is_some();

        // Check if parameters are actually used in the callback body.
        // We use the same simple AST walk as Codegen's arrow_body_uses_ident().
        let elem_used = body
            .statements
            .iter()
            .any(|s| Self::ast_stmt_uses_ident(&elem_param_raw, s));
        let elem_param = if elem_used {
            elem_param_raw
        } else {
            "_".to_string()
        };

        let idx_param = if let Some(idx_name) = idx_param_raw {
            if idx_name != "_"
                && body
                    .statements
                    .iter()
                    .any(|s| Self::ast_stmt_uses_ident(idx_name, s))
            {
                idx_name.to_string()
            } else {
                "_".to_string()
            }
        } else {
            String::new()
        };

        // Lower the callback body
        let ir_body: Vec<crate::zigir::types::IrStmt> =
            body.statements.iter().map(|s| self.lower_stmt(s)).collect();

        // Extract object name
        let obj_name = self.extract_callee_object_name(ce)?;

        // Get element type
        let elem_type = self
            .type_info
            .array_element_types
            .get(obj_name.as_str())
            .cloned()
            .unwrap_or(ZigType::I64);

        // Reduce init value
        let reduce_init = if kind == ArrayCallbackKind::Reduce && ce.arguments.len() >= 2 {
            ce.arguments
                .get(1)
                .and_then(|a| a.as_expression())
                .map(|e| self.lower_expr(e))
        } else {
            None
        };

        Some(IrExpr::ArrayCallbackInline(Box::new(
            IrArrayCallbackInline {
                kind,
                obj_name,
                elem_type,
                elem_param,
                has_idx_param,
                idx_param,
                body: ir_body,
                reduce_init,
            },
        )))
    }

    /// Extract the object variable name from a CallExpression's callee.
    fn extract_callee_object_name(&self, ce: &CallExpression) -> Option<String> {
        Self::extract_callee_object_name_static(&ce.callee)
    }

    /// Extract the object variable name from a callee Expression.
    fn extract_callee_object_name_static(callee: &Expression) -> Option<String> {
        match callee {
            Expression::StaticMemberExpression(mem) => match &mem.object {
                Expression::Identifier(id) => Some(id.name.as_str().to_string()),
                _ => None,
            },
            _ => None,
        }
    }

    // ── AST ident-usage helpers ─────────────────────
    // These mirror Codegen's stmt_uses_ident / expr_uses_ident,
    // used to check whether a callback parameter is actually referenced.

    fn ast_stmt_uses_ident(ident: &str, stmt: &Statement) -> bool {
        match stmt {
            Statement::ReturnStatement(r) => r
                .argument
                .as_ref()
                .is_some_and(|e| Self::ast_expr_uses_ident(ident, e)),
            Statement::ExpressionStatement(e) => Self::ast_expr_uses_ident(ident, &e.expression),
            Statement::BlockStatement(b) => {
                b.body.iter().any(|s| Self::ast_stmt_uses_ident(ident, s))
            }
            _ => false,
        }
    }

    fn ast_expr_uses_ident(ident: &str, expr: &Expression) -> bool {
        match expr {
            Expression::Identifier(id) => id.name.as_str() == ident,
            Expression::BinaryExpression(b) => {
                Self::ast_expr_uses_ident(ident, &b.left)
                    || Self::ast_expr_uses_ident(ident, &b.right)
            }
            Expression::UnaryExpression(u) => Self::ast_expr_uses_ident(ident, &u.argument),
            Expression::StaticMemberExpression(m) => Self::ast_expr_uses_ident(ident, &m.object),
            Expression::ComputedMemberExpression(m) => {
                Self::ast_expr_uses_ident(ident, &m.object)
                    || Self::ast_expr_uses_ident(ident, &m.expression)
            }
            Expression::CallExpression(c) => {
                Self::ast_expr_uses_ident(ident, &c.callee)
                    || c.arguments.iter().any(|a| match a.as_expression() {
                        Some(e) => Self::ast_expr_uses_ident(ident, e),
                        None => false,
                    })
            }
            Expression::ParenthesizedExpression(p) => {
                Self::ast_expr_uses_ident(ident, &p.expression)
            }
            Expression::ConditionalExpression(c) => {
                Self::ast_expr_uses_ident(ident, &c.test)
                    || Self::ast_expr_uses_ident(ident, &c.consequent)
                    || Self::ast_expr_uses_ident(ident, &c.alternate)
            }
            Expression::NumericLiteral(_)
            | Expression::StringLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
            | Expression::BigIntLiteral(_)
            | Expression::RegExpLiteral(_) => false,
            // Conservative: assume identifier MAY appear in unhandled variants
            _ => true,
        }
    }
}

// ═══════════════════════════════════════════════════════
//  FnContext management
// ═══════════════════════════════════════════════════════

impl Lowerer {
    /// Enter a function context. Saves any existing context (nesting support).
    fn enter_fn(
        &mut self,
        name: &str,
        is_export: bool,
        return_type: Option<ZigType>,
    ) -> Option<FnContext> {
        let old = self.fn_ctx.take();
        self.fn_ctx = Some(FnContext::new(name, is_export, return_type));
        old
    }

    /// Exit the current function context. Restores the previous one if any.
    fn exit_fn(&mut self, saved: Option<FnContext>) -> FnContext {
        let ctx = self.fn_ctx.take().expect("exit_fn called without enter_fn");
        self.fn_ctx = saved;
        ctx
    }

    /// Get a reference to the current function context.
    #[allow(dead_code)]
    fn fn_ctx(&self) -> Option<&FnContext> {
        self.fn_ctx.as_ref()
    }

    /// Get a mutable reference to the current function context.
    fn fn_ctx_mut(&mut self) -> Option<&mut FnContext> {
        self.fn_ctx.as_mut()
    }
}

// ═══════════════════════════════════════════════════════
//  Utility: statement type name (for diagnostics)
// ═══════════════════════════════════════════════════════

/// Map a BuiltinCall to (BuiltinModule, method_name, return_type).
fn builtin_call_to_ir(
    bc: &crate::native_builtins::BuiltinCall,
) -> (BuiltinModule, String, ZigType) {
    use crate::native_builtins::BuiltinCall;

    match bc {
        // Math
        BuiltinCall::MathAbs => (BuiltinModule::JsMath, "abs".into(), ZigType::F64),
        BuiltinCall::MathFloor => (BuiltinModule::JsMath, "floor".into(), ZigType::F64),
        BuiltinCall::MathCeil => (BuiltinModule::JsMath, "ceil".into(), ZigType::F64),
        BuiltinCall::MathRound => (BuiltinModule::JsMath, "round".into(), ZigType::F64),
        BuiltinCall::MathSqrt => (BuiltinModule::JsMath, "sqrt".into(), ZigType::F64),
        BuiltinCall::MathRandom => (BuiltinModule::JsMath, "random".into(), ZigType::F64),
        BuiltinCall::MathPow => (BuiltinModule::JsMath, "pow".into(), ZigType::F64),
        BuiltinCall::MathMax => (BuiltinModule::JsMath, "max".into(), ZigType::F64),
        BuiltinCall::MathMin => (BuiltinModule::JsMath, "min".into(), ZigType::F64),
        BuiltinCall::MathHypot => (BuiltinModule::JsMath, "hypot".into(), ZigType::F64),
        BuiltinCall::MathSin => (BuiltinModule::JsMath, "sin".into(), ZigType::F64),
        BuiltinCall::MathCos => (BuiltinModule::JsMath, "cos".into(), ZigType::F64),
        BuiltinCall::MathTan => (BuiltinModule::JsMath, "tan".into(), ZigType::F64),
        BuiltinCall::MathAsin => (BuiltinModule::JsMath, "asin".into(), ZigType::F64),
        BuiltinCall::MathAcos => (BuiltinModule::JsMath, "acos".into(), ZigType::F64),
        BuiltinCall::MathAtan => (BuiltinModule::JsMath, "atan".into(), ZigType::F64),
        BuiltinCall::MathAtan2 => (BuiltinModule::JsMath, "atan2".into(), ZigType::F64),
        BuiltinCall::MathLog => (BuiltinModule::JsMath, "log".into(), ZigType::F64),
        BuiltinCall::MathLog10 => (BuiltinModule::JsMath, "log10".into(), ZigType::F64),
        BuiltinCall::MathLog2 => (BuiltinModule::JsMath, "log2".into(), ZigType::F64),
        BuiltinCall::MathExp => (BuiltinModule::JsMath, "exp".into(), ZigType::F64),
        BuiltinCall::MathSign => (BuiltinModule::JsMath, "sign".into(), ZigType::F64),
        BuiltinCall::MathTrunc => (BuiltinModule::JsMath, "trunc".into(), ZigType::F64),
        BuiltinCall::MathCbrt => (BuiltinModule::JsMath, "cbrt".into(), ZigType::F64),
        BuiltinCall::MathExpm1 => (BuiltinModule::JsMath, "expm1".into(), ZigType::F64),
        BuiltinCall::MathSinh => (BuiltinModule::JsMath, "sinh".into(), ZigType::F64),
        BuiltinCall::MathCosh => (BuiltinModule::JsMath, "cosh".into(), ZigType::F64),
        BuiltinCall::MathTanh => (BuiltinModule::JsMath, "tanh".into(), ZigType::F64),
        BuiltinCall::MathAsinh => (BuiltinModule::JsMath, "asinh".into(), ZigType::F64),
        BuiltinCall::MathAcosh => (BuiltinModule::JsMath, "acosh".into(), ZigType::F64),
        BuiltinCall::MathAtanh => (BuiltinModule::JsMath, "atanh".into(), ZigType::F64),
        BuiltinCall::MathClz32 => (BuiltinModule::JsMath, "clz32".into(), ZigType::I64),
        BuiltinCall::MathFround => (BuiltinModule::JsMath, "fround".into(), ZigType::F64),
        BuiltinCall::MathImul => (BuiltinModule::JsMath, "imul".into(), ZigType::I64),
        BuiltinCall::MathLog1p => (BuiltinModule::JsMath, "log1p".into(), ZigType::F64),

        // Console
        BuiltinCall::ConsoleLog => (BuiltinModule::JsConsole, "log".into(), ZigType::Void),
        BuiltinCall::ConsoleError => (BuiltinModule::JsConsole, "error".into(), ZigType::Void),
        BuiltinCall::ConsoleWarn => (BuiltinModule::JsConsole, "warn".into(), ZigType::Void),

        // JSON
        BuiltinCall::JsonStringify => (BuiltinModule::JsJson, "stringify".into(), ZigType::Str),
        BuiltinCall::JsonParse => (BuiltinModule::JsJson, "parse".into(), ZigType::JsAny),

        // Global functions
        BuiltinCall::ParseInt => (BuiltinModule::JsUri, "parseInt".into(), ZigType::I64),
        BuiltinCall::ParseFloat => (BuiltinModule::JsUri, "parseFloat".into(), ZigType::F64),
        BuiltinCall::IsNaN => (BuiltinModule::JsUri, "isNaN".into(), ZigType::Bool),
        BuiltinCall::IsFinite => (BuiltinModule::JsUri, "isFinite".into(), ZigType::Bool),
        BuiltinCall::EncodeURIComponent => (
            BuiltinModule::JsUri,
            "encodeURIComponent".into(),
            ZigType::Str,
        ),
        BuiltinCall::DecodeURIComponent => (
            BuiltinModule::JsUri,
            "decodeURIComponent".into(),
            ZigType::Str,
        ),
        BuiltinCall::EncodeURI => (BuiltinModule::JsUri, "encodeURI".into(), ZigType::Str),
        BuiltinCall::DecodeURI => (BuiltinModule::JsUri, "decodeURI".into(), ZigType::Str),
        BuiltinCall::Eval => (BuiltinModule::JsUri, "eval".into(), ZigType::Void),

        // Constructors
        BuiltinCall::NumberConstructor => {
            (BuiltinModule::JsNumber, "constructor".into(), ZigType::F64)
        }
        BuiltinCall::StringConstructor => {
            (BuiltinModule::JsString, "constructor".into(), ZigType::Str)
        }
        BuiltinCall::BooleanConstructor => (
            BuiltinModule::JsNumber,
            "booleanConstructor".into(),
            ZigType::Bool,
        ),
        BuiltinCall::BigIntConstructor => (
            BuiltinModule::JsNumber,
            "bigIntConstructor".into(),
            ZigType::BigInt,
        ),
        BuiltinCall::ObjectConstructor => (
            BuiltinModule::JsObject,
            "constructor".into(),
            ZigType::JsAny,
        ),
        BuiltinCall::SymbolConstructor => (
            BuiltinModule::JsSymbol,
            "constructor".into(),
            ZigType::JsSymbol,
        ),

        // Number static methods
        BuiltinCall::NumberIsNaN => (BuiltinModule::JsNumber, "isNaN".into(), ZigType::Bool),
        BuiltinCall::NumberIsFinite => (BuiltinModule::JsNumber, "isFinite".into(), ZigType::Bool),
        BuiltinCall::NumberIsInteger => {
            (BuiltinModule::JsNumber, "isInteger".into(), ZigType::Bool)
        }
        BuiltinCall::NumberIsSafeInteger => (
            BuiltinModule::JsNumber,
            "isSafeInteger".into(),
            ZigType::Bool,
        ),
        BuiltinCall::NumberParseInt => (BuiltinModule::JsNumber, "parseInt".into(), ZigType::I64),
        BuiltinCall::NumberParseFloat => {
            (BuiltinModule::JsNumber, "parseFloat".into(), ZigType::F64)
        }

        // Number instance methods
        BuiltinCall::NumberToFixed => (BuiltinModule::JsNumber, "toFixed".into(), ZigType::Str),
        BuiltinCall::NumberToExponential => (
            BuiltinModule::JsNumber,
            "toExponential".into(),
            ZigType::Str,
        ),
        BuiltinCall::NumberToPrecision => {
            (BuiltinModule::JsNumber, "toPrecision".into(), ZigType::Str)
        }

        // String instance methods
        BuiltinCall::StringIndexOf => (BuiltinModule::JsString, "indexOf".into(), ZigType::I64),
        BuiltinCall::StringIncludes => (BuiltinModule::JsString, "includes".into(), ZigType::Bool),
        BuiltinCall::StringStartsWith => {
            (BuiltinModule::JsString, "startsWith".into(), ZigType::Bool)
        }
        BuiltinCall::StringEndsWith => (BuiltinModule::JsString, "endsWith".into(), ZigType::Bool),
        BuiltinCall::StringLastIndexOf => {
            (BuiltinModule::JsString, "lastIndexOf".into(), ZigType::I64)
        }
        BuiltinCall::StringTrim => (BuiltinModule::JsString, "trim".into(), ZigType::Str),
        BuiltinCall::StringSplit => (
            BuiltinModule::JsString,
            "split".into(),
            ZigType::ArrayList(Box::new(ZigType::Str)),
        ),
        BuiltinCall::StringPadStart => (BuiltinModule::JsString, "padStart".into(), ZigType::Str),
        BuiltinCall::StringPadEnd => (BuiltinModule::JsString, "padEnd".into(), ZigType::Str),
        BuiltinCall::StringTrimStart => (BuiltinModule::JsString, "trimStart".into(), ZigType::Str),
        BuiltinCall::StringTrimEnd => (BuiltinModule::JsString, "trimEnd".into(), ZigType::Str),
        BuiltinCall::StringToUpperCase => {
            (BuiltinModule::JsString, "toUpperCase".into(), ZigType::Str)
        }
        BuiltinCall::StringToLowerCase => {
            (BuiltinModule::JsString, "toLowerCase".into(), ZigType::Str)
        }
        BuiltinCall::StringCharAt => (BuiltinModule::JsString, "charAt".into(), ZigType::Str),
        BuiltinCall::StringCharCodeAt => {
            (BuiltinModule::JsString, "charCodeAt".into(), ZigType::I64)
        }
        BuiltinCall::StringCodePointAt => {
            (BuiltinModule::JsString, "codePointAt".into(), ZigType::I64)
        }
        BuiltinCall::StringConcat => (BuiltinModule::JsString, "concat".into(), ZigType::Str),
        BuiltinCall::StringSlice => (BuiltinModule::JsString, "slice".into(), ZigType::Str),
        BuiltinCall::StringReplace => (BuiltinModule::JsString, "replace".into(), ZigType::Str),
        BuiltinCall::StringReplaceAll => {
            (BuiltinModule::JsString, "replaceAll".into(), ZigType::Str)
        }
        BuiltinCall::StringRepeat => (BuiltinModule::JsString, "repeat".into(), ZigType::Str),
        BuiltinCall::StringSubstring => (BuiltinModule::JsString, "substring".into(), ZigType::Str),
        BuiltinCall::StringAt => (BuiltinModule::JsString, "at".into(), ZigType::Str),
        BuiltinCall::StringMatch => (BuiltinModule::JsString, "match".into(), ZigType::JsAny),
        BuiltinCall::StringSearch => (BuiltinModule::JsString, "search".into(), ZigType::I64),
        BuiltinCall::StringFromCharCode => {
            (BuiltinModule::JsString, "fromCharCode".into(), ZigType::Str)
        }
        BuiltinCall::StringFromCodePoint => (
            BuiltinModule::JsString,
            "fromCodePoint".into(),
            ZigType::Str,
        ),
        BuiltinCall::StringMatchAll => (BuiltinModule::JsString, "matchAll".into(), ZigType::JsAny),
        BuiltinCall::StringLocaleCompare => (
            BuiltinModule::JsString,
            "localeCompare".into(),
            ZigType::I64,
        ),
        BuiltinCall::StringNormalize => (BuiltinModule::JsString, "normalize".into(), ZigType::Str),
        BuiltinCall::StringToLocaleUpperCase => (
            BuiltinModule::JsString,
            "toLocaleUpperCase".into(),
            ZigType::Str,
        ),
        BuiltinCall::StringToLocaleLowerCase => (
            BuiltinModule::JsString,
            "toLocaleLowerCase".into(),
            ZigType::Str,
        ),

        // Array methods
        BuiltinCall::ArrayPush => (BuiltinModule::JsArray, "push".into(), ZigType::I64),
        BuiltinCall::ArrayPop => (BuiltinModule::JsArray, "pop".into(), ZigType::JsAny),
        BuiltinCall::ArrayShift => (BuiltinModule::JsArray, "shift".into(), ZigType::JsAny),
        BuiltinCall::ArrayUnshift => (BuiltinModule::JsArray, "unshift".into(), ZigType::I64),
        BuiltinCall::ArrayReverse => (BuiltinModule::JsArray, "reverse".into(), ZigType::Void),
        BuiltinCall::ArraySort => (BuiltinModule::JsArray, "sort".into(), ZigType::Void),
        BuiltinCall::ArrayIndexOf => (BuiltinModule::JsArray, "indexOf".into(), ZigType::I64),
        BuiltinCall::ArrayIncludes => (BuiltinModule::JsArray, "includes".into(), ZigType::Bool),
        BuiltinCall::ArrayJoin => (BuiltinModule::JsArray, "join".into(), ZigType::Str),
        BuiltinCall::ArraySlice => (
            BuiltinModule::JsArray,
            "slice".into(),
            ZigType::ArrayList(Box::new(ZigType::JsAny)),
        ),
        BuiltinCall::ArraySplice => (
            BuiltinModule::JsArray,
            "splice".into(),
            ZigType::ArrayList(Box::new(ZigType::JsAny)),
        ),
        BuiltinCall::ArrayConcat => (
            BuiltinModule::JsArray,
            "concat".into(),
            ZigType::ArrayList(Box::new(ZigType::JsAny)),
        ),
        BuiltinCall::ArrayAt => (BuiltinModule::JsArray, "at".into(), ZigType::JsAny),
        BuiltinCall::ArrayLastIndexOf => {
            (BuiltinModule::JsArray, "lastIndexOf".into(), ZigType::I64)
        }
        BuiltinCall::ArrayCopyWithin => {
            (BuiltinModule::JsArray, "copyWithin".into(), ZigType::Void)
        }
        BuiltinCall::ArrayForEach => (BuiltinModule::JsArray, "forEach".into(), ZigType::Void),
        BuiltinCall::ArrayMap => (
            BuiltinModule::JsArray,
            "map".into(),
            ZigType::ArrayList(Box::new(ZigType::JsAny)),
        ),
        BuiltinCall::ArrayFilter => (
            BuiltinModule::JsArray,
            "filter".into(),
            ZigType::ArrayList(Box::new(ZigType::JsAny)),
        ),
        BuiltinCall::ArrayReduce => (BuiltinModule::JsArray, "reduce".into(), ZigType::JsAny),
        BuiltinCall::ArraySome => (BuiltinModule::JsArray, "some".into(), ZigType::Bool),
        BuiltinCall::ArrayEvery => (BuiltinModule::JsArray, "every".into(), ZigType::Bool),
        BuiltinCall::ArrayFlat => (
            BuiltinModule::JsArray,
            "flat".into(),
            ZigType::ArrayList(Box::new(ZigType::JsAny)),
        ),
        BuiltinCall::ArrayFlatMap => (
            BuiltinModule::JsArray,
            "flatMap".into(),
            ZigType::ArrayList(Box::new(ZigType::JsAny)),
        ),
        BuiltinCall::ArrayFind => (BuiltinModule::JsArray, "find".into(), ZigType::JsAny),
        BuiltinCall::ArrayFindIndex => (BuiltinModule::JsArray, "findIndex".into(), ZigType::I64),
        BuiltinCall::ArrayFindLast => (BuiltinModule::JsArray, "findLast".into(), ZigType::JsAny),
        BuiltinCall::ArrayFindLastIndex => {
            (BuiltinModule::JsArray, "findLastIndex".into(), ZigType::I64)
        }
        BuiltinCall::ArrayReduceRight => {
            (BuiltinModule::JsArray, "reduceRight".into(), ZigType::JsAny)
        }
        BuiltinCall::ArrayFill => (BuiltinModule::JsArray, "fill".into(), ZigType::Void),
        BuiltinCall::ArrayKeys => (
            BuiltinModule::JsArray,
            "keys".into(),
            ZigType::ArrayList(Box::new(ZigType::I64)),
        ),
        BuiltinCall::ArrayValues => (
            BuiltinModule::JsArray,
            "values".into(),
            ZigType::ArrayList(Box::new(ZigType::JsAny)),
        ),
        BuiltinCall::ArrayEntries => (
            BuiltinModule::JsArray,
            "entries".into(),
            ZigType::ArrayList(Box::new(ZigType::JsAny)),
        ),
        BuiltinCall::ArrayFrom => (
            BuiltinModule::JsArray,
            "from".into(),
            ZigType::ArrayList(Box::new(ZigType::JsAny)),
        ),
        BuiltinCall::ArrayOf => (
            BuiltinModule::JsArray,
            "of".into(),
            ZigType::ArrayList(Box::new(ZigType::JsAny)),
        ),
        BuiltinCall::ArrayIsArray => (BuiltinModule::JsArray, "isArray".into(), ZigType::Bool),
        BuiltinCall::TypedArraySubarray => (
            BuiltinModule::JsTypedArray,
            "subarray".into(),
            ZigType::JsAny,
        ),

        // Map/Set
        BuiltinCall::MapSet => (BuiltinModule::JsCollections, "set".into(), ZigType::Void),
        BuiltinCall::MapGet => (BuiltinModule::JsCollections, "get".into(), ZigType::JsAny),
        BuiltinCall::MapHas => (BuiltinModule::JsCollections, "has".into(), ZigType::Bool),
        BuiltinCall::MapDelete => (BuiltinModule::JsCollections, "delete".into(), ZigType::Bool),
        BuiltinCall::MapKeys => (
            BuiltinModule::JsCollections,
            "keys".into(),
            ZigType::ArrayList(Box::new(ZigType::Str)),
        ),
        BuiltinCall::MapValues => (
            BuiltinModule::JsCollections,
            "values".into(),
            ZigType::ArrayList(Box::new(ZigType::JsAny)),
        ),
        BuiltinCall::MapEntries => (
            BuiltinModule::JsCollections,
            "entries".into(),
            ZigType::ArrayList(Box::new(ZigType::JsAny)),
        ),
        BuiltinCall::MapClear => (BuiltinModule::JsCollections, "clear".into(), ZigType::Void),
        BuiltinCall::SetAdd => (BuiltinModule::JsCollections, "add".into(), ZigType::Void),
        BuiltinCall::SetForEach => (
            BuiltinModule::JsCollections,
            "forEach".into(),
            ZigType::Void,
        ),
        BuiltinCall::SetKeys => (
            BuiltinModule::JsCollections,
            "keys".into(),
            ZigType::ArrayList(Box::new(ZigType::JsAny)),
        ),
        BuiltinCall::SetValues => (
            BuiltinModule::JsCollections,
            "values".into(),
            ZigType::ArrayList(Box::new(ZigType::JsAny)),
        ),
        BuiltinCall::SetEntries => (
            BuiltinModule::JsCollections,
            "entries".into(),
            ZigType::ArrayList(Box::new(ZigType::JsAny)),
        ),

        // Date static
        BuiltinCall::DateNow => (BuiltinModule::JsDate, "now".into(), ZigType::I64),
        BuiltinCall::DateParse => (BuiltinModule::JsDate, "parse".into(), ZigType::I64),
        BuiltinCall::DateUTC => (BuiltinModule::JsDate, "utc".into(), ZigType::I64),

        // Date instance
        BuiltinCall::DateGetTime => (BuiltinModule::JsDate, "getTime".into(), ZigType::I64),
        BuiltinCall::DateGetFullYear => (BuiltinModule::JsDate, "getFullYear".into(), ZigType::I64),
        BuiltinCall::DateGetMonth => (BuiltinModule::JsDate, "getMonth".into(), ZigType::I64),
        BuiltinCall::DateGetDate => (BuiltinModule::JsDate, "getDate".into(), ZigType::I64),
        BuiltinCall::DateGetDay => (BuiltinModule::JsDate, "getDay".into(), ZigType::I64),
        BuiltinCall::DateGetHours => (BuiltinModule::JsDate, "getHours".into(), ZigType::I64),
        BuiltinCall::DateGetMinutes => (BuiltinModule::JsDate, "getMinutes".into(), ZigType::I64),
        BuiltinCall::DateGetSeconds => (BuiltinModule::JsDate, "getSeconds".into(), ZigType::I64),
        BuiltinCall::DateGetMilliseconds => (
            BuiltinModule::JsDate,
            "getMilliseconds".into(),
            ZigType::I64,
        ),
        BuiltinCall::DateGetTimezoneOffset => (
            BuiltinModule::JsDate,
            "getTimezoneOffset".into(),
            ZigType::I64,
        ),
        BuiltinCall::DateToISOString => (BuiltinModule::JsDate, "toISOString".into(), ZigType::Str),
        BuiltinCall::DateToString => (BuiltinModule::JsDate, "toString".into(), ZigType::Str),
        BuiltinCall::DateToDateString => {
            (BuiltinModule::JsDate, "toDateString".into(), ZigType::Str)
        }
        BuiltinCall::DateToTimeString => {
            (BuiltinModule::JsDate, "toTimeString".into(), ZigType::Str)
        }
        BuiltinCall::DateToLocaleString => {
            (BuiltinModule::JsDate, "toLocaleString".into(), ZigType::Str)
        }
        BuiltinCall::DateGetUTCFullYear => {
            (BuiltinModule::JsDate, "getUTCFullYear".into(), ZigType::I64)
        }
        BuiltinCall::DateGetUTCMonth => (BuiltinModule::JsDate, "getUTCMonth".into(), ZigType::I64),
        BuiltinCall::DateGetUTCDate => (BuiltinModule::JsDate, "getUTCDate".into(), ZigType::I64),
        BuiltinCall::DateGetUTCDay => (BuiltinModule::JsDate, "getUTCDay".into(), ZigType::I64),
        BuiltinCall::DateGetUTCHours => (BuiltinModule::JsDate, "getUTCHours".into(), ZigType::I64),
        BuiltinCall::DateGetUTCMinutes => {
            (BuiltinModule::JsDate, "getUTCMinutes".into(), ZigType::I64)
        }
        BuiltinCall::DateGetUTCSeconds => {
            (BuiltinModule::JsDate, "getUTCSeconds".into(), ZigType::I64)
        }
        BuiltinCall::DateGetUTCMilliseconds => (
            BuiltinModule::JsDate,
            "getUTCMilliseconds".into(),
            ZigType::I64,
        ),
        BuiltinCall::DateToJSON => (BuiltinModule::JsDate, "toJSON".into(), ZigType::Str),
        BuiltinCall::DateValueOf => (BuiltinModule::JsDate, "valueOf".into(), ZigType::I64),
        BuiltinCall::DateSetFullYear => (BuiltinModule::JsDate, "setFullYear".into(), ZigType::I64),
        BuiltinCall::DateSetMonth => (BuiltinModule::JsDate, "setMonth".into(), ZigType::I64),
        BuiltinCall::DateSetDate => (BuiltinModule::JsDate, "setDate".into(), ZigType::I64),
        BuiltinCall::DateSetHours => (BuiltinModule::JsDate, "setHours".into(), ZigType::I64),
        BuiltinCall::DateSetMinutes => (BuiltinModule::JsDate, "setMinutes".into(), ZigType::I64),
        BuiltinCall::DateSetSeconds => (BuiltinModule::JsDate, "setSeconds".into(), ZigType::I64),
        BuiltinCall::DateSetMilliseconds => (
            BuiltinModule::JsDate,
            "setMilliseconds".into(),
            ZigType::I64,
        ),
        BuiltinCall::DateSetUTCFullYear => {
            (BuiltinModule::JsDate, "setUTCFullYear".into(), ZigType::I64)
        }
        BuiltinCall::DateSetUTCMonth => (BuiltinModule::JsDate, "setUTCMonth".into(), ZigType::I64),
        BuiltinCall::DateSetUTCDate => (BuiltinModule::JsDate, "setUTCDate".into(), ZigType::I64),
        BuiltinCall::DateSetUTCHours => (BuiltinModule::JsDate, "setUTCHours".into(), ZigType::I64),
        BuiltinCall::DateSetUTCMinutes => {
            (BuiltinModule::JsDate, "setUTCMinutes".into(), ZigType::I64)
        }
        BuiltinCall::DateSetUTCSeconds => {
            (BuiltinModule::JsDate, "setUTCSeconds".into(), ZigType::I64)
        }
        BuiltinCall::DateSetUTCMilliseconds => (
            BuiltinModule::JsDate,
            "setUTCMilliseconds".into(),
            ZigType::I64,
        ),

        // Object static
        BuiltinCall::ObjectKeys => (
            BuiltinModule::JsObject,
            "keys".into(),
            ZigType::ArrayList(Box::new(ZigType::Str)),
        ),
        BuiltinCall::ObjectValues => (
            BuiltinModule::JsObject,
            "values".into(),
            ZigType::ArrayList(Box::new(ZigType::JsAny)),
        ),
        BuiltinCall::ObjectEntries => (
            BuiltinModule::JsObject,
            "entries".into(),
            ZigType::ArrayList(Box::new(ZigType::JsAny)),
        ),
        BuiltinCall::ObjectFromEntries => (
            BuiltinModule::JsObject,
            "fromEntries".into(),
            ZigType::JsAny,
        ),
        BuiltinCall::ObjectAssign => (BuiltinModule::JsObject, "assign".into(), ZigType::Void),
        BuiltinCall::ObjectFreeze => (BuiltinModule::JsObject, "freeze".into(), ZigType::Void),
        BuiltinCall::ObjectSeal => (BuiltinModule::JsObject, "seal".into(), ZigType::Void),
        BuiltinCall::ObjectPreventExtensions => (
            BuiltinModule::JsObject,
            "preventExtensions".into(),
            ZigType::Void,
        ),
        BuiltinCall::ObjectHasOwn => (BuiltinModule::JsObject, "hasOwn".into(), ZigType::Bool),
        BuiltinCall::ObjectIs => (BuiltinModule::JsObject, "is".into(), ZigType::Bool),
        BuiltinCall::ObjectGetOwnPropertyNames => (
            BuiltinModule::JsObject,
            "getOwnPropertyNames".into(),
            ZigType::ArrayList(Box::new(ZigType::Str)),
        ),
        BuiltinCall::ObjectCreate => (BuiltinModule::JsObject, "create".into(), ZigType::JsAny),
        BuiltinCall::ObjectDefineProperty => (
            BuiltinModule::JsObject,
            "defineProperty".into(),
            ZigType::Void,
        ),
        BuiltinCall::ObjectGetPrototypeOf => (
            BuiltinModule::JsObject,
            "getPrototypeOf".into(),
            ZigType::JsAny,
        ),
        BuiltinCall::ObjectDefineProperties => (
            BuiltinModule::JsObject,
            "defineProperties".into(),
            ZigType::Void,
        ),
        BuiltinCall::ObjectGetOwnPropertyDescriptor => (
            BuiltinModule::JsObject,
            "getOwnPropertyDescriptor".into(),
            ZigType::JsAny,
        ),
        BuiltinCall::ObjectSetPrototypeOf => (
            BuiltinModule::JsObject,
            "setPrototypeOf".into(),
            ZigType::Void,
        ),
        BuiltinCall::ObjectIsSealed => (BuiltinModule::JsObject, "isSealed".into(), ZigType::Bool),
        BuiltinCall::ObjectIsFrozen => (BuiltinModule::JsObject, "isFrozen".into(), ZigType::Bool),
        BuiltinCall::ObjectIsExtensible => (
            BuiltinModule::JsObject,
            "isExtensible".into(),
            ZigType::Bool,
        ),

        // Symbol
        BuiltinCall::SymbolFor => (BuiltinModule::JsSymbol, "for".into(), ZigType::JsSymbol),
        BuiltinCall::SymbolKeyFor => (BuiltinModule::JsSymbol, "keyFor".into(), ZigType::Str),

        // RegExp
        BuiltinCall::RegExpTest => (BuiltinModule::JsRegExp, "test".into(), ZigType::Bool),
        BuiltinCall::RegExpExec => (BuiltinModule::JsRegExp, "exec".into(), ZigType::JsAny),
    }
}

#[allow(dead_code)]
fn stmt_type_name(stmt: &Statement) -> &'static str {
    match stmt {
        Statement::BlockStatement(_) => "BlockStatement",
        Statement::BreakStatement(_) => "BreakStatement",
        Statement::ContinueStatement(_) => "ContinueStatement",
        Statement::DebuggerStatement(_) => "DebuggerStatement",
        Statement::DoWhileStatement(_) => "DoWhileStatement",
        Statement::EmptyStatement(_) => "EmptyStatement",
        Statement::ExpressionStatement(_) => "ExpressionStatement",
        Statement::ForInStatement(_) => "ForInStatement",
        Statement::ForOfStatement(_) => "ForOfStatement",
        Statement::ForStatement(_) => "ForStatement",
        Statement::FunctionDeclaration(_) => "FunctionDeclaration",
        Statement::IfStatement(_) => "IfStatement",
        Statement::LabeledStatement(_) => "LabeledStatement",
        Statement::ReturnStatement(_) => "ReturnStatement",
        Statement::SwitchStatement(_) => "SwitchStatement",
        Statement::ThrowStatement(_) => "ThrowStatement",
        Statement::TryStatement(_) => "TryStatement",
        Statement::VariableDeclaration(_) => "VariableDeclaration",
        Statement::WhileStatement(_) => "WhileStatement",
        Statement::WithStatement(_) => "WithStatement",
        Statement::ClassDeclaration(_) => "ClassDeclaration",
        Statement::ExportNamedDeclaration(_) => "ExportNamedDeclaration",
        Statement::ExportDefaultDeclaration(_) => "ExportDefaultDeclaration",
        Statement::ImportDeclaration(_) => "ImportDeclaration",
        _ => "Unknown",
    }
}

fn expr_type_name(expr: &Expression) -> &'static str {
    match expr {
        Expression::NumericLiteral(_) => "NumericLiteral",
        Expression::StringLiteral(_) => "StringLiteral",
        Expression::BooleanLiteral(_) => "BooleanLiteral",
        Expression::NullLiteral(_) => "NullLiteral",
        Expression::RegExpLiteral(_) => "RegExpLiteral",
        Expression::BigIntLiteral(_) => "BigIntLiteral",
        Expression::Identifier(_) => "Identifier",
        Expression::ThisExpression(_) => "ThisExpression",
        Expression::BinaryExpression(_) => "BinaryExpression",
        Expression::LogicalExpression(_) => "LogicalExpression",
        Expression::UnaryExpression(_) => "UnaryExpression",
        Expression::UpdateExpression(_) => "UpdateExpression",
        Expression::AssignmentExpression(_) => "AssignmentExpression",
        Expression::CallExpression(_) => "CallExpression",
        Expression::NewExpression(_) => "NewExpression",
        Expression::StaticMemberExpression(_) => "StaticMemberExpression",
        Expression::ComputedMemberExpression(_) => "ComputedMemberExpression",
        Expression::ArrayExpression(_) => "ArrayExpression",
        Expression::ObjectExpression(_) => "ObjectExpression",
        Expression::ArrowFunctionExpression(_) => "ArrowFunctionExpression",
        Expression::FunctionExpression(_) => "FunctionExpression",
        Expression::TemplateLiteral(_) => "TemplateLiteral",
        Expression::ParenthesizedExpression(_) => "ParenthesizedExpression",
        Expression::ConditionalExpression(_) => "ConditionalExpression",
        Expression::SequenceExpression(_) => "SequenceExpression",
        Expression::AwaitExpression(_) => "AwaitExpression",
        Expression::PrivateFieldExpression(_) => "PrivateFieldExpression",
        _ => "Unknown",
    }
}

// ═══════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infer::TypeCheckResult;
    use crate::types::JSDocData;
    use oxc_parser::Parser;
    use oxc_span::SourceType;
    use std::collections::HashMap;

    fn empty_type_info() -> TypeCheckResult {
        TypeCheckResult {
            var_types: HashMap::new(),
            array_element_types: HashMap::new(),
            fn_return_types: HashMap::new(),
            fn_param_types: HashMap::new(),
            mutated_vars: HashSet::new(),
            set_vars: HashSet::new(),
            used_names: HashSet::new(),
            has_json_parse_types: HashSet::new(),
            errors: Vec::new(),
            is_async: HashMap::new(),
            class_field_types: HashMap::new(),
        }
    }

    #[test]
    fn test_lowerer_new() {
        let type_info = empty_type_info();
        let jsdoc_data = JSDocData {
            typedefs: HashMap::new(),
            type_annotations: HashMap::new(),
            return_types: HashMap::new(),
            param_types: HashMap::new(),
        };
        let lowerer = Lowerer::new(
            type_info,
            jsdoc_data,
            None,
            HashSet::new(),
            "let x = 1;".to_string(),
        );
        assert!(lowerer.fn_ctx.is_none());
        assert!(lowerer.class_names.is_empty());
        assert!(lowerer.diagnostics.is_empty());
    }

    #[test]
    fn test_lowerer_empty_program() {
        let type_info = empty_type_info();
        let jsdoc_data = JSDocData {
            typedefs: HashMap::new(),
            type_annotations: HashMap::new(),
            return_types: HashMap::new(),
            param_types: HashMap::new(),
        };
        let mut lowerer = Lowerer::new(type_info, jsdoc_data, None, HashSet::new(), String::new());

        // Parse an empty program
        let js = "";
        let allocator = oxc_allocator::Allocator::default();
        let source_type = SourceType::default();
        let parser = Parser::new(&allocator, js, source_type);
        let result = parser.parse();
        let module = lowerer.lower(&result.program);

        assert_eq!(module.name, "main");
        assert!(module.declarations.is_empty());
        assert!(module.closure_structs.is_empty());
        assert!(module.diagnostics.is_empty());
    }

    #[test]
    fn test_fn_context_enter_exit() {
        let type_info = empty_type_info();
        let jsdoc_data = JSDocData {
            typedefs: HashMap::new(),
            type_annotations: HashMap::new(),
            return_types: HashMap::new(),
            param_types: HashMap::new(),
        };
        let mut lowerer = Lowerer::new(type_info, jsdoc_data, None, HashSet::new(), String::new());

        // Enter a function
        let saved = lowerer.enter_fn("foo", true, Some(ZigType::I64));
        assert!(saved.is_none()); // No previous context
        assert!(lowerer.fn_ctx.is_some());

        let ctx = lowerer.fn_ctx().unwrap();
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
        let jsdoc_data = JSDocData {
            typedefs: HashMap::new(),
            type_annotations: HashMap::new(),
            return_types: HashMap::new(),
            param_types: HashMap::new(),
        };
        let mut lowerer = Lowerer::new(type_info, jsdoc_data, None, HashSet::new(), String::new());

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
        assert_eq!(lowerer.fn_ctx().unwrap().name, "outer");

        // Exit outer
        let outer_ctx = lowerer.exit_fn(saved_outer);
        assert_eq!(outer_ctx.name, "outer");
        assert!(lowerer.fn_ctx.is_none());
    }

    #[test]
    fn test_is_export_fn() {
        let type_info = empty_type_info();
        let jsdoc_data = JSDocData {
            typedefs: HashMap::new(),
            type_annotations: HashMap::new(),
            return_types: HashMap::new(),
            param_types: HashMap::new(),
        };
        let mut exported = HashSet::new();
        exported.insert("greet".to_string());

        let lowerer = Lowerer::new(
            type_info,
            jsdoc_data,
            Some(exported),
            HashSet::new(),
            String::new(),
        );

        assert!(lowerer.is_export_fn(Some("greet")));
        assert!(!lowerer.is_export_fn(Some("helper")));
        assert!(!lowerer.is_export_fn(None));
    }

    // ── infer_imports tests ─────────────────────────────────

    fn make_jsdoc_data() -> JSDocData {
        JSDocData {
            typedefs: HashMap::new(),
            type_annotations: HashMap::new(),
            return_types: HashMap::new(),
            param_types: HashMap::new(),
        }
    }

    #[test]
    fn test_infer_imports_empty() {
        let lowerer = Lowerer::new(
            empty_type_info(),
            make_jsdoc_data(),
            None,
            HashSet::new(),
            String::new(),
        );
        let imports = lowerer.infer_imports();
        assert!(
            imports.is_empty(),
            "empty program should have no imports, got {:?}",
            imports
        );
    }

    #[test]
    fn test_infer_imports_typedef() {
        // When typedefs exist, should import std + js_allocator
        let mut td = HashMap::new();
        td.insert(
            "User".to_string(),
            crate::jsdoc::TypedefDef {
                name: "User".to_string(),
                fields: vec![crate::jsdoc::TypedefField {
                    name: "name".to_string(),
                    ty: "string".to_string(),
                    optional: false,
                }],
            },
        );
        let jsdoc_data = JSDocData {
            typedefs: td,
            type_annotations: HashMap::new(),
            return_types: HashMap::new(),
            param_types: HashMap::new(),
        };
        let lowerer = Lowerer::new(
            empty_type_info(),
            jsdoc_data,
            None,
            HashSet::new(),
            String::new(),
        );
        let imports = lowerer.infer_imports();
        let module_names: Vec<&str> = imports.iter().map(|i| i.module_name.as_str()).collect();
        assert!(
            module_names.contains(&"std"),
            "typedef should require std import, got {:?}",
            module_names
        );
        assert!(
            module_names.contains(&"js_runtime/js_allocator.zig"),
            "typedef should require js_allocator, got {:?}",
            module_names
        );
    }

    #[test]
    fn test_infer_imports_jsany() {
        // When a variable has type JsAny, should import jsany.zig + js_runtime
        let mut var_types = HashMap::new();
        var_types.insert("data".to_string(), ZigType::JsAny);
        let type_info = TypeCheckResult {
            var_types,
            array_element_types: HashMap::new(),
            fn_return_types: HashMap::new(),
            fn_param_types: HashMap::new(),
            mutated_vars: HashSet::new(),
            set_vars: HashSet::new(),
            used_names: HashSet::new(),
            has_json_parse_types: HashSet::new(),
            errors: Vec::new(),
            is_async: HashMap::new(),
            class_field_types: HashMap::new(),
        };
        let lowerer = Lowerer::new(
            type_info,
            make_jsdoc_data(),
            None,
            HashSet::new(),
            String::new(),
        );
        let imports = lowerer.infer_imports();
        let module_names: Vec<&str> = imports.iter().map(|i| i.module_name.as_str()).collect();
        assert!(
            module_names.contains(&"js_runtime/jsany.zig"),
            "JsAny should require jsany.zig, got {:?}",
            module_names
        );
        assert!(
            module_names.contains(&"js_runtime/js_runtime.zig"),
            "JsAny should require js_runtime, got {:?}",
            module_names
        );
    }

    #[test]
    fn test_infer_imports_date_and_map() {
        // NamedStruct("Date") → js_date, NamedStruct("Map") → js_collections
        let mut var_types = HashMap::new();
        var_types.insert("d".to_string(), ZigType::NamedStruct("Date".to_string()));
        var_types.insert("m".to_string(), ZigType::NamedStruct("Map".to_string()));
        let type_info = TypeCheckResult {
            var_types,
            array_element_types: HashMap::new(),
            fn_return_types: HashMap::new(),
            fn_param_types: HashMap::new(),
            mutated_vars: HashSet::new(),
            set_vars: HashSet::new(),
            used_names: HashSet::new(),
            has_json_parse_types: HashSet::new(),
            errors: Vec::new(),
            is_async: HashMap::new(),
            class_field_types: HashMap::new(),
        };
        let lowerer = Lowerer::new(
            type_info,
            make_jsdoc_data(),
            None,
            HashSet::new(),
            String::new(),
        );
        let imports = lowerer.infer_imports();
        let module_names: Vec<&str> = imports.iter().map(|i| i.module_name.as_str()).collect();
        assert!(
            module_names.contains(&"js_runtime/js_date.zig"),
            "Date should require js_date, got {:?}",
            module_names
        );
        assert!(
            module_names.contains(&"js_runtime/js_collections.zig"),
            "Map should require js_collections, got {:?}",
            module_names
        );
        assert!(
            module_names.contains(&"js_runtime/js_allocator.zig"),
            "Date/Map should require js_allocator, got {:?}",
            module_names
        );
    }

    #[test]
    fn test_infer_imports_deduplication() {
        // Multiple triggers for js_runtime should produce only one import
        let mut var_types = HashMap::new();
        var_types.insert("a".to_string(), ZigType::JsAny);
        var_types.insert("b".to_string(), ZigType::JsSymbol);
        let type_info = TypeCheckResult {
            var_types,
            array_element_types: HashMap::new(),
            fn_return_types: HashMap::new(),
            fn_param_types: HashMap::new(),
            mutated_vars: HashSet::new(),
            set_vars: HashSet::new(),
            used_names: HashSet::new(),
            has_json_parse_types: HashSet::new(),
            errors: Vec::new(),
            is_async: HashMap::new(),
            class_field_types: HashMap::new(),
        };
        let lowerer = Lowerer::new(
            type_info,
            make_jsdoc_data(),
            None,
            HashSet::new(),
            String::new(),
        );
        let imports = lowerer.infer_imports();
        let js_runtime_count = imports
            .iter()
            .filter(|i| i.module_name == "js_runtime/js_runtime.zig")
            .count();
        assert_eq!(
            js_runtime_count, 1,
            "js_runtime should appear exactly once, got {} times",
            js_runtime_count
        );
    }
}
