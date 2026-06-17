/// JS module preprocessing: import/export resolution, naming conflict handling, merging.
///
/// Steps:
/// 1. Parse all .js files in in/ dir
/// 2. Extract imports, exports, and top-level declarations from each
/// 3. Build dependency graph, topological sort
/// 4. Resolve naming conflicts (internal decls get `_filename` suffix)
/// 5. Transform each file: strip import/export, rename internal decls + refs
/// 6. Merge all into single JS string
use std::cmp::Reverse;

use crate::codegen::collect_binding_names;
use crate::codegen::collect_binding_names_with_spans;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_span::GetSpan;

use crate::parser;

/// Convert a module name (from filename) to a valid Zig identifier suffix.
/// Non-ASCII characters are converted to Unicode codepoint format `_uXXXX`.
/// ASCII alphanumeric and underscore characters pass through unchanged.
pub fn sanitize_module_name(raw: &str) -> String {
    let mut result = String::new();
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            result.push(ch);
        } else {
            result.push_str(&format!("_u{:04x}", ch as u32));
        }
    }
    // Ensure non-empty
    if result.is_empty() {
        result.push_str("_unnamed");
    }
    // Ensure starts with ASCII letter or underscore
    if !result.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_') {
        result.insert(0, '_');
    }
    result
}

#[cfg(test)]
#[test]
fn test_sanitize_module_name() {
    assert_eq!(sanitize_module_name("utils"), "utils");
    assert_eq!(sanitize_module_name("测试"), "_u6d4b_u8bd5");
    assert_eq!(sanitize_module_name("hello_测试"), "hello__u6d4b_u8bd5");
    assert_eq!(sanitize_module_name("123"), "_123");
    assert_eq!(sanitize_module_name(""), "_unnamed");
    assert_eq!(sanitize_module_name("math"), "math");
}

/// Metadata about a single JS module
#[derive(Debug, Clone)]
pub struct ModuleMeta {
    /// Module name (filename without .js)
    pub name: String,
    /// Full source text
    pub source: String,
    /// Imported names: (imported_symbol, source_module_name)
    #[allow(dead_code)]
    pub imports: Vec<(String, String)>,
    /// Modules this file depends on (sanitized names from import ... from '...')
    pub deps: HashSet<String>,
    /// Export names that THIS module imports (original names from import specifiers).
    /// Used to determine which exports don't need `pub` in Zig (they're used internally).
    pub imported_names: HashSet<String>,
    /// Exported names
    pub exports: HashSet<String>,
    /// Internal top-level declarations (function/var names not exported)
    pub internal_decls: HashSet<String>,
    /// Internal top-level function declarations (not variables)
    pub internal_fns: HashSet<String>,
    /// Internal async function names (non-exported)
    #[allow(dead_code)]
    pub async_internal: HashSet<String>,
}

/// Result of merging all modules
pub struct PreprocessResult {
    /// Map from export name → export name (for marking pub fn in Zig).
    /// Only contains exports NOT consumed internally — these are the
    /// truly external exports for lib.zig re-exports.
    pub export_map: HashMap<String, String>,
    /// Per-file transformed JS: (module_name, transformed_source).
    /// Always populated (single-file inputs included).
    pub per_file: Vec<(String, String)>,
    /// Per-file export names: module_name → set of exported function/var names.
    /// For per-file codegen: ALL exports are `pub` in their source file,
    /// even if consumed internally by another file in the same group.
    pub per_file_exports: HashMap<String, HashSet<String>>,
    /// Diagnostics (warnings/errors)
    pub diagnostics: Vec<String>,
}

impl PreprocessResult {
    /// Reconstruct the merged JS source by joining per-file transformed sources.
    pub fn merged_js(&self) -> String {
        self.per_file
            .iter()
            .map(|(name, src)| format!("// --- from {}.js ---\n{}", name, src))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Collect edits to apply to source text. Each edit replaces a span range.
#[derive(Debug, Clone)]
struct Edit {
    start: usize,
    end: usize,
    replacement: String,
}

/// Preprocess all .js files in in_dir. Returns per-file transformed sources and export map.
pub fn preprocess(in_dir: &str) -> PreprocessResult {
    let in_path = Path::new(in_dir);
    let mut diagnostics = Vec::new();

    // 1. Read and parse all .js files
    let mut modules: Vec<ModuleMeta> = Vec::new();
    if let Ok(entries) = fs::read_dir(in_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "js") {
                let name = sanitize_module_name(
                    &path.file_stem().unwrap().to_string_lossy(),
                );
                let source = fs::read_to_string(&path).unwrap_or_default();
                let allocator = Allocator::default();
                let program = parser::parse(&allocator, &source);
                let meta = extract_module_meta(&name, &source, &program);
                modules.push(meta);
            }
        }
    }

    // Sort deterministically
    modules.sort_by(|a, b| a.name.cmp(&b.name));

    // 2. Detect export naming conflicts
    let mut all_exports: HashMap<String, Vec<String>> = HashMap::new();
    for m in &modules {
        for exp in &m.exports {
            all_exports.entry(exp.clone()).or_default().push(m.name.clone());
        }
    }
    for (name, mods) in &all_exports {
        if mods.len() > 1 {
            diagnostics.push(format!(
                "error: export name '{}' conflicts between modules: {:?}",
                name, mods
            ));
        }
    }

    // 3. Build rename map for internal declarations
    //    Format: (module_name, original) → original_filename
    let mut rename_map: HashMap<(String, String), String> = HashMap::new();
    for m in &modules {
        for decl in &m.internal_decls {
            rename_map.insert((m.name.clone(), decl.clone()), format!("{}_{}", decl, m.name));
        }
    }

    // 4. Build dependency graph and topological sort
    let sorted_modules = topological_sort(&modules, &mut diagnostics);
    if !modules.iter().any(|m| m.name == "main") {
        diagnostics.push("warning: no main.js found — export map may be incomplete".to_string());
    }

    // 4.5 Collect all exported names that are actually imported by some module.
    //     Exports that are imported → internal use only, no `pub` needed in Zig.
    let all_imported: HashSet<String> = modules
        .iter()
        .flat_map(|m| m.imported_names.iter().cloned())
        .collect();

    // 5. Transform each module in order.
    //    Always track per-file results (single-file inputs included).
    let mut per_file: Vec<(String, String)> = Vec::new();
    let mut per_file_exports: HashMap<String, HashSet<String>> = HashMap::new();
    let mut export_map: HashMap<String, String> = HashMap::new();

    for &idx in &sorted_modules {
        let m = &modules[idx];
        let transformed = transform_module(m, &rename_map);

        // Track per-file transformed source
        per_file.push((m.name.clone(), transformed));

        // Track per-file exports (all exports, including internally-consumed ones)
        per_file_exports.insert(m.name.clone(), m.exports.clone());

        // Export map: only mark as `pub` if NOT imported by any module.
        // Exports that are imported are purely internal after merging.
        for exp in &m.exports {
            if !all_imported.contains(exp) {
                export_map.insert(exp.clone(), exp.clone());
            }
        }
    }

    PreprocessResult {
        export_map,
        per_file,
        per_file_exports,
        diagnostics,
    }
}

/// Extract imports, exports, and internal declarations from a parsed module.
fn extract_module_meta(_name: &str, _source: &str, program: &Program) -> ModuleMeta {
    let mut imports = Vec::new();
    let mut imported_names = HashSet::new();
    let mut deps = HashSet::new();
    let mut exports = HashSet::new();
    let mut internal_decls = HashSet::new();
    let mut internal_fns = HashSet::new();
    let mut async_internal = HashSet::new();

    for stmt in &program.body {
        match stmt {
            Statement::ImportDeclaration(import) => {
                let from_raw = import
                    .source
                    .value
                    .to_string()
                    .trim_start_matches("./")
                    .trim_end_matches(".js")
                    .to_string();
                let from = sanitize_module_name(&from_raw);
                deps.insert(from.clone());
                if let Some(specifiers) = &import.specifiers {
                    for spec in specifiers {
                        // Track the original export name (before aliasing)
                        let original_name = match spec {
                            ImportDeclarationSpecifier::ImportSpecifier(s) => {
                                s.imported.name().to_string()
                            }
                            _ => spec.local().name.to_string(),
                        };
                        imported_names.insert(original_name);
                        let local = spec.local().name.to_string();
                        imports.push((local, from.clone()));
                    }
                }
            }
            Statement::ExportNamedDeclaration(export) => {
                if let Some(decl) = &export.declaration {
                    collect_exports_from_decl(decl, &mut exports);
                }
            }
            Statement::ExportDefaultDeclaration(export) => {
                if let ExportDefaultDeclarationKind::FunctionDeclaration(f) = &export.declaration
                    && let Some(ref id) = f.id
                {
                    exports.insert(id.name.to_string());
                }
            }
            Statement::FunctionDeclaration(fn_decl) => {
                if let Some(ref id) = fn_decl.id {
                    internal_decls.insert(id.name.to_string());
                    internal_fns.insert(id.name.to_string());
                    if fn_decl.r#async {
                        async_internal.insert(id.name.to_string());
                    }
                }
            }
            Statement::VariableDeclaration(var_decl) => {
                for d in &var_decl.declarations {
                    let mut names = Vec::new();
                    collect_binding_names(&d.id, &mut names);
                    for n in names {
                        internal_decls.insert(n);
                    }
                }
            }
            _ => {}
        }
    }

    ModuleMeta {
        name: _name.to_string(),
        source: _source.to_string(),
        imports,
        imported_names,
        deps,
        exports,
        internal_decls,
        internal_fns,
        async_internal,
    }
}

fn collect_exports_from_decl(
    decl: &Declaration,
    exports: &mut HashSet<String>,
) {
    match decl {
        Declaration::FunctionDeclaration(fn_decl) => {
            if let Some(ref id) = fn_decl.id {
                exports.insert(id.name.to_string());
            }
        }
        Declaration::VariableDeclaration(var_decl) => {
            for d in &var_decl.declarations {
                let mut names = Vec::new();
                collect_binding_names(&d.id, &mut names);
                for n in names {
                    exports.insert(n);
                }
            }
        }
        _ => {}
    }
}

/// Build dependency graph and return topologically sorted indices (Kahn's algorithm).
fn topological_sort(modules: &[ModuleMeta], diagnostics: &mut Vec<String>) -> Vec<usize> {
    let n = modules.len();

    // Map sanitized module name -> index
    let name_to_idx: HashMap<&str, usize> = modules
        .iter()
        .enumerate()
        .map(|(i, m)| (m.name.as_str(), i))
        .collect();

    // Reverse adjacency: dep_idx -> [dependent_indices]
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut in_degree: Vec<usize> = vec![0; n];

    for (i, m) in modules.iter().enumerate() {
        for dep_name in &m.deps {
            if let Some(&dep_idx) = name_to_idx.get(dep_name.as_str()) {
                // dep_idx must come before i
                adj[dep_idx].push(i);
                in_degree[i] += 1;
            }
            // else: imported from module not in our file set (external), skip silently
        }
    }

    // Kahn's algorithm
    let mut queue: Vec<usize> = (0..n).filter(|&i| in_degree[i] == 0).collect();
    let mut result = Vec::with_capacity(n);

    while let Some(u) = queue.pop() {
        result.push(u);
        for &v in &adj[u] {
            in_degree[v] -= 1;
            if in_degree[v] == 0 {
                queue.push(v);
            }
        }
    }

    // Check for cycles: remaining nodes have non-zero in-degree
    if result.len() < n {
        let remaining: Vec<&str> = (0..n)
            .filter(|i| in_degree[*i] > 0)
            .map(|i| modules[i].name.as_str())
            .collect();
        diagnostics.push(format!(
            "warning: circular import dependency involving: {:?}",
            remaining
        ));
        // Include cycle participants after sorted modules
        result.extend(
            in_degree
                .iter()
                .enumerate()
                .filter(|&(_, &d)| d > 0)
                .map(|(i, _)| i),
        );
    }

    result
}

/// Transform a module's source text:
/// - Strip import/export statements
/// - Rename internal declarations and their references
fn transform_module(
    m: &ModuleMeta,
    rename_map: &HashMap<(String, String), String>,
) -> String {
    let source = &m.source;
    let allocator = Allocator::default();
    let program = parser::parse(&allocator, source);

    let mut edits: Vec<Edit> = Vec::new();

    // Single walk: strip import/export + collect decl renames
    for stmt in &program.body {
        match stmt {
            // Skip import statements entirely
            Statement::ImportDeclaration(import) => {
                edits.push(Edit {
                    start: import.span.start as usize,
                    end: import.span.end as usize,
                    replacement: String::new(),
                });
            }
            // Export named: strip 'export' keyword, keep declaration
            Statement::ExportNamedDeclaration(export) => {
                let decl_start = if let Some(decl) = &export.declaration {
                    decl.span().start
                } else {
                    export.span.end
                };
                edits.push(Edit {
                    start: export.span.start as usize,
                    end: decl_start as usize,
                    replacement: String::new(),
                });
            }
            // Export default: strip 'export default ' prefix
            Statement::ExportDefaultDeclaration(export) => {
                let decl_start = match &export.declaration {
                    ExportDefaultDeclarationKind::FunctionDeclaration(f) => f.span.start,
                    _ => export.span.end,
                };
                edits.push(Edit {
                    start: export.span.start as usize,
                    end: decl_start as usize,
                    replacement: String::new(),
                });
            }
            // Function declarations: inline decl rename logic
            Statement::FunctionDeclaration(fn_decl) => {
                if let Some(ref id) = fn_decl.id {
                    let key = (m.name.clone(), id.name.to_string());
                    if let Some(new_name) = rename_map.get(&key) {
                        edits.push(Edit {
                            start: id.span.start as usize,
                            end: id.span.end as usize,
                            replacement: new_name.clone(),
                        });
                    }
                }
            }
            // Variable declarations: inline decl rename logic
            Statement::VariableDeclaration(var_decl) => {
                for d in &var_decl.declarations {
                    let mut bindings = Vec::new();
                    collect_binding_names_with_spans(&d.id, &mut bindings);
                    for (n, span) in bindings {
                        let key = (m.name.clone(), n.to_string());
                        if let Some(new_name) = rename_map.get(&key) {
                            edits.push(Edit {
                                start: span.start as usize,
                                end: span.end as usize,
                                replacement: new_name.clone(),
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Collect rename edits for references to renamed identifiers (with scope tracking)
    let mut scope = ScopeTracker::new();
    collect_ref_rename_edits(&program.body, &m.name, rename_map, &mut scope, &mut edits);

    // Apply edits (sort descending by start to preserve positions)
    edits.sort_by_key(|e| Reverse(e.start));
    let mut result = source.to_string();
    for edit in &edits {
        if edit.start < result.len() && edit.end <= result.len() {
            if edit.replacement.is_empty() {
                // Remove the span; extend end to include trailing newline/whitespace
                // but do NOT extend start backwards (preserve leading newline)
                let start = edit.start;
                let mut end = edit.end;
                while end < result.len() {
                    let ch = result.as_bytes()[end] as char;
                    if ch == '\n' || ch == '\r' {
                        end += 1;
                        break;
                    } else if ch == ' ' || ch == '\t' {
                        end += 1;
                    } else {
                        break;
                    }
                }
                result.replace_range(start..end, "");
            } else {
                result.replace_range(edit.start..edit.end, &edit.replacement);
            }
        }
    }

    // Clean up: collapse multiple blank lines
    while result.contains("\n\n\n") {
        result = result.replace("\n\n\n", "\n\n");
    }

    result.trim().to_string()
}

// ===== Scope tracking for reference renaming =====

/// Tracks locally declared identifiers across nested scopes
/// to avoid renaming local variables that shadow module-level names.
struct ScopeTracker {
    scopes: Vec<HashSet<String>>,
}

impl ScopeTracker {
    fn new() -> Self {
        Self { scopes: Vec::new() }
    }

    fn push(&mut self) {
        self.scopes.push(HashSet::new());
    }

    fn pop(&mut self) {
        self.scopes.pop();
    }

    fn declare(&mut self, name: &str) {
        if let Some(s) = self.scopes.last_mut() {
            s.insert(name.to_string());
        }
    }

    fn is_local(&self, name: &str) -> bool {
        self.scopes.iter().any(|s| s.contains(name))
    }
}

fn collect_refs_in_decl(
    decl: &Declaration,
    module_name: &str,
    rename_map: &HashMap<(String, String), String>,
    scope: &mut ScopeTracker,
    edits: &mut Vec<Edit>,
) {
    match decl {
        Declaration::FunctionDeclaration(fn_decl) => {
            if let Some(ref body) = fn_decl.body {
                scope.push();
                for param in &fn_decl.params.items {
                    let mut names = Vec::new();
                    collect_binding_names(&param.pattern, &mut names);
                    for n in &names { scope.declare(n); }
                }
                collect_refs_in_fn_body(body, module_name, rename_map, scope, edits);
                scope.pop();
            }
        }
        Declaration::VariableDeclaration(var_decl) => {
            for d in &var_decl.declarations {
                if let Some(ref init) = d.init {
                    collect_refs_in_expr(init, module_name, rename_map, scope, edits);
                }
            }
        }
        _ => {}
    }
}

/// Collect edits for identifier references in top-level/module statements.
fn collect_ref_rename_edits(
    stmts: &[Statement],
    module_name: &str,
    rename_map: &HashMap<(String, String), String>,
    scope: &mut ScopeTracker,
    edits: &mut Vec<Edit>,
) {
    for stmt in stmts {
        collect_refs_in_stmt(stmt, module_name, rename_map, scope, edits);
    }
}

/// Visit statement node to find identifier references needing renaming.
/// This is the main statement dispatcher — covers all JS statement types.
fn collect_refs_in_stmt(
    stmt: &Statement,
    module_name: &str,
    rename_map: &HashMap<(String, String), String>,
    scope: &mut ScopeTracker,
    edits: &mut Vec<Edit>,
) {
    match stmt {
        // === Standard statements ===
        Statement::BlockStatement(block) => {
            scope.push();
            for s in &block.body {
                collect_refs_in_stmt(s, module_name, rename_map, scope, edits);
            }
            scope.pop();
        }
        Statement::BreakStatement(_) | Statement::ContinueStatement(_) => {}
        Statement::DebuggerStatement(_) | Statement::EmptyStatement(_) => {}
        Statement::DoWhileStatement(dw) => {
            collect_refs_in_stmt(&dw.body, module_name, rename_map, scope, edits);
            collect_refs_in_expr(&dw.test, module_name, rename_map, scope, edits);
        }
        Statement::ExpressionStatement(expr_stmt) => {
            collect_refs_in_expr(&expr_stmt.expression, module_name, rename_map, scope, edits);
        }
        Statement::ForInStatement(fi) => {
            collect_refs_in_expr(&fi.right, module_name, rename_map, scope, edits);
            visit_for_left(&fi.left, module_name, rename_map, scope, edits);
            scope.push();
            declare_for_left(&fi.left, scope);
            collect_refs_in_stmt(&fi.body, module_name, rename_map, scope, edits);
            scope.pop();
        }
        Statement::ForOfStatement(fo) => {
            collect_refs_in_expr(&fo.right, module_name, rename_map, scope, edits);
            visit_for_left(&fo.left, module_name, rename_map, scope, edits);
            scope.push();
            declare_for_left(&fo.left, scope);
            collect_refs_in_stmt(&fo.body, module_name, rename_map, scope, edits);
            scope.pop();
        }
        Statement::ForStatement(fs) => {
            scope.push();
            if let Some(ref init) = fs.init {
                match init {
                    ForStatementInit::VariableDeclaration(vd) => {
                        for d in &vd.declarations {
                            let mut names = Vec::new();
                            collect_binding_names(&d.id, &mut names);
                            for n in &names { scope.declare(n); }
                            if let Some(ref init_expr) = d.init {
                                collect_refs_in_expr(
                                    init_expr, module_name, rename_map, scope, edits,
                                );
                            }
                        }
                    }
                    _ => {
                        // Expression variants (CallExpression, etc.)
                        if let Some(expr) = init.as_expression() {
                            collect_refs_in_expr(expr, module_name, rename_map, scope, edits);
                        }
                    }
                }
            }
            if let Some(ref test) = fs.test {
                collect_refs_in_expr(test, module_name, rename_map, scope, edits);
            }
            if let Some(ref update) = fs.update {
                collect_refs_in_expr(update, module_name, rename_map, scope, edits);
            }
            collect_refs_in_stmt(&fs.body, module_name, rename_map, scope, edits);
            scope.pop();
        }
        Statement::IfStatement(if_stmt) => {
            collect_refs_in_expr(&if_stmt.test, module_name, rename_map, scope, edits);
            collect_refs_in_stmt(&if_stmt.consequent, module_name, rename_map, scope, edits);
            if let Some(ref alt) = if_stmt.alternate {
                collect_refs_in_stmt(alt, module_name, rename_map, scope, edits);
            }
        }
        Statement::LabeledStatement(ls) => {
            collect_refs_in_stmt(&ls.body, module_name, rename_map, scope, edits);
        }
        Statement::ReturnStatement(ret) => {
            if let Some(ref arg) = ret.argument {
                collect_refs_in_expr(arg, module_name, rename_map, scope, edits);
            }
        }
        Statement::SwitchStatement(sw) => {
            collect_refs_in_expr(&sw.discriminant, module_name, rename_map, scope, edits);
            for case in &sw.cases {
                if let Some(ref test) = case.test {
                    collect_refs_in_expr(test, module_name, rename_map, scope, edits);
                }
                for s in &case.consequent {
                    collect_refs_in_stmt(s, module_name, rename_map, scope, edits);
                }
            }
        }
        Statement::ThrowStatement(th) => {
            collect_refs_in_expr(&th.argument, module_name, rename_map, scope, edits);
        }
        Statement::TryStatement(tr) => {
            for s in &tr.block.body {
                collect_refs_in_stmt(s, module_name, rename_map, scope, edits);
            }
            if let Some(ref handler) = tr.handler {
                scope.push();
                if let Some(ref param) = handler.param
                    && let BindingPattern::BindingIdentifier(id) = &param.pattern
                {
                    scope.declare(&id.name);
                }
                for s in &handler.body.body {
                    collect_refs_in_stmt(s, module_name, rename_map, scope, edits);
                }
                scope.pop();
            }
            if let Some(ref finalizer) = tr.finalizer {
                for s in &finalizer.body {
                    collect_refs_in_stmt(s, module_name, rename_map, scope, edits);
                }
            }
        }
        Statement::WhileStatement(wh) => {
            collect_refs_in_expr(&wh.test, module_name, rename_map, scope, edits);
            collect_refs_in_stmt(&wh.body, module_name, rename_map, scope, edits);
        }
        Statement::WithStatement(wi) => {
            collect_refs_in_expr(&wi.object, module_name, rename_map, scope, edits);
            collect_refs_in_stmt(&wi.body, module_name, rename_map, scope, edits);
        }

        // === Declarations (inherited into Statement via @inherit Declaration) ===
        Statement::FunctionDeclaration(fn_decl) => {
            scope.push();
            for param in &fn_decl.params.items {
                let mut names = Vec::new();
                collect_binding_names(&param.pattern, &mut names);
                for n in &names { scope.declare(n); }
            }
            if let Some(ref body) = fn_decl.body {
                collect_refs_in_fn_body(body, module_name, rename_map, scope, edits);
            }
            scope.pop();
        }
        Statement::VariableDeclaration(var_decl) => {
            for d in &var_decl.declarations {
                let mut names = Vec::new();
                collect_binding_names(&d.id, &mut names);
                for n in &names { scope.declare(n); }
                if let Some(ref init) = d.init {
                    collect_refs_in_expr(init, module_name, rename_map, scope, edits);
                }
            }
        }
        Statement::ClassDeclaration(_) | Statement::TSTypeAliasDeclaration(_)
        | Statement::TSInterfaceDeclaration(_) | Statement::TSEnumDeclaration(_)
        | Statement::TSModuleDeclaration(_) | Statement::TSGlobalDeclaration(_)
        | Statement::TSImportEqualsDeclaration(_) => {
            // TS declarations: no expressions to visit
        }

        // === Exports ===
        Statement::ExportNamedDeclaration(export) => {
            if let Some(decl) = &export.declaration {
                collect_refs_in_decl(decl, module_name, rename_map, scope, edits);
            }
        }
        Statement::ExportDefaultDeclaration(export) => {
            if let ExportDefaultDeclarationKind::FunctionDeclaration(f) = &export.declaration
                && let Some(ref body) = f.body
            {
                scope.push();
                for param in &f.params.items {
                    let mut names = Vec::new();
                    collect_binding_names(&param.pattern, &mut names);
                    for n in &names { scope.declare(n); }
                }
                collect_refs_in_fn_body(body, module_name, rename_map, scope, edits);
                scope.pop();
            }
        }

        // === Module declarations ===
        Statement::ImportDeclaration(_) => {
            // Import bindings are not references to rename
        }

        _ => {
            // Catch-all for future variants: no expressions to visit by default
        }
    }
}

/// Visit the left side of a for-in/for-of loop to find references
/// (member expression objects, init expressions).
fn visit_for_left(
    left: &ForStatementLeft,
    module_name: &str,
    rename_map: &HashMap<(String, String), String>,
    scope: &mut ScopeTracker,
    edits: &mut Vec<Edit>,
) {
    match left {
        ForStatementLeft::VariableDeclaration(vd) => {
            for d in &vd.declarations {
                if let Some(ref init) = d.init {
                    collect_refs_in_expr(init, module_name, rename_map, scope, edits);
                }
            }
        }
        ForStatementLeft::AssignmentTargetIdentifier(_) => {}
        ForStatementLeft::ComputedMemberExpression(cme) => {
            collect_refs_in_expr(&cme.object, module_name, rename_map, scope, edits);
            collect_refs_in_expr(&cme.expression, module_name, rename_map, scope, edits);
        }
        ForStatementLeft::StaticMemberExpression(sme) => {
            collect_refs_in_expr(&sme.object, module_name, rename_map, scope, edits);
        }
        ForStatementLeft::PrivateFieldExpression(_) => {}
        _ => {} // TS types, destructuring — skip
    }
}

/// Declare the names bound by a for-in/for-of left side as local variables.
fn declare_for_left(left: &ForStatementLeft, scope: &mut ScopeTracker) {
    match left {
        ForStatementLeft::VariableDeclaration(vd) => {
            for d in &vd.declarations {
                let mut names = Vec::new();
                collect_binding_names(&d.id, &mut names);
                for n in &names { scope.declare(n); }
            }
        }
        ForStatementLeft::AssignmentTargetIdentifier(id) => {
            scope.declare(&id.name);
        }
        _ => {} // Destructuring patterns — complex, skip scope tracking for now
    }
}

/// Visit an expression node to find identifier references needing renaming.
/// This is the main expression dispatcher — covers all JS expression types.
fn collect_refs_in_expr(
    expr: &Expression,
    module_name: &str,
    rename_map: &HashMap<(String, String), String>,
    scope: &mut ScopeTracker,
    edits: &mut Vec<Edit>,
) {
    match expr {
        // === Literals (no sub-expressions) ===
        Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::NumericLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::RegExpLiteral(_)
        | Expression::StringLiteral(_) => {}

        // === Template literal: visit interpolated expressions ===
        Expression::TemplateLiteral(tl) => {
            for e in &tl.expressions {
                collect_refs_in_expr(e, module_name, rename_map, scope, edits);
            }
        }

        // === Identifier reference: check for renaming ===
        Expression::Identifier(id_ref) => {
            // Skip if this identifier is a locally-declared name (shadowing)
            if scope.is_local(&id_ref.name) {
                return;
            }
            let key = (module_name.to_string(), id_ref.name.to_string());
            if let Some(new_name) = rename_map.get(&key) {
                edits.push(Edit {
                    start: id_ref.span.start as usize,
                    end: id_ref.span.end as usize,
                    replacement: new_name.clone(),
                });
            }
        }

        // === Unary / Update / Await / Yield / Spread: visit single operand ===
        Expression::UnaryExpression(unary) => {
            collect_refs_in_expr(&unary.argument, module_name, rename_map, scope, edits);
        }
        Expression::UpdateExpression(up) => {
            visit_update_target(&up.argument, module_name, rename_map, scope, edits);
        }
        Expression::AwaitExpression(await_expr) => {
            collect_refs_in_expr(&await_expr.argument, module_name, rename_map, scope, edits);
        }
        Expression::YieldExpression(yield_expr) => {
            if let Some(ref arg) = yield_expr.argument {
                collect_refs_in_expr(arg, module_name, rename_map, scope, edits);
            }
        }
        Expression::ParenthesizedExpression(paren) => {
            collect_refs_in_expr(&paren.expression, module_name, rename_map, scope, edits);
        }

        // === Binary / Logical: visit both sides ===
        Expression::BinaryExpression(bin) => {
            collect_refs_in_expr(&bin.left, module_name, rename_map, scope, edits);
            collect_refs_in_expr(&bin.right, module_name, rename_map, scope, edits);
        }
        Expression::LogicalExpression(log) => {
            collect_refs_in_expr(&log.left, module_name, rename_map, scope, edits);
            collect_refs_in_expr(&log.right, module_name, rename_map, scope, edits);
        }

        // === Conditional: visit test, consequent, alternate ===
        Expression::ConditionalExpression(cond) => {
            collect_refs_in_expr(&cond.test, module_name, rename_map, scope, edits);
            collect_refs_in_expr(&cond.consequent, module_name, rename_map, scope, edits);
            collect_refs_in_expr(&cond.alternate, module_name, rename_map, scope, edits);
        }

        // === Assignment: visit right side; visit left side only for member expr objects ===
        Expression::AssignmentExpression(assign) => {
            collect_refs_in_expr(&assign.right, module_name, rename_map, scope, edits);
            visit_assign_target(&assign.left, module_name, rename_map, scope, edits);
        }

        // === Call / New: visit callee + arguments ===
        Expression::CallExpression(call) => {
            collect_refs_in_expr(&call.callee, module_name, rename_map, scope, edits);
            for arg in &call.arguments {
                if let Some(e) = arg.as_expression() {
                    collect_refs_in_expr(e, module_name, rename_map, scope, edits);
                }
            }
        }
        Expression::NewExpression(ne) => {
            collect_refs_in_expr(&ne.callee, module_name, rename_map, scope, edits);
            for arg in &ne.arguments {
                if let Some(e) = arg.as_expression() {
                    collect_refs_in_expr(e, module_name, rename_map, scope, edits);
                }
            }
        }

        // === Array / Object: visit elements/properties ===
        Expression::ArrayExpression(arr) => {
            for elem in &arr.elements {
                visit_array_element(elem, module_name, rename_map, scope, edits);
            }
        }
        Expression::ObjectExpression(obj) => {
            for prop in &obj.properties {
                match prop {
                    ObjectPropertyKind::ObjectProperty(op) => {
                        if op.computed
                            && let Some(key_expr) = op.key.as_expression()
                        {
                            collect_refs_in_expr(
                                key_expr, module_name, rename_map, scope, edits,
                            );
                        }
                        collect_refs_in_expr(&op.value, module_name, rename_map, scope, edits);
                    }
                    ObjectPropertyKind::SpreadProperty(sp) => {
                        collect_refs_in_expr(
                            &sp.argument, module_name, rename_map, scope, edits,
                        );
                    }
                }
            }
        }

        // === Sequence: visit all expressions ===
        Expression::SequenceExpression(seq) => {
            for e in &seq.expressions {
                collect_refs_in_expr(e, module_name, rename_map, scope, edits);
            }
        }

        // === Tagged template: visit tag + template expressions ===
        Expression::TaggedTemplateExpression(tt) => {
            collect_refs_in_expr(&tt.tag, module_name, rename_map, scope, edits);
            for e in &tt.quasi.expressions {
                collect_refs_in_expr(e, module_name, rename_map, scope, edits);
            }
        }

        // === Chain expression (optional chaining) ===
        Expression::ChainExpression(chain) => {
            visit_chain_element(&chain.expression, module_name, rename_map, scope, edits);
        }

        // === Arrow function: new scope ===
        Expression::ArrowFunctionExpression(arrow) => {
            scope.push();
            for param in &arrow.params.items {
                let mut names = Vec::new();
                collect_binding_names(&param.pattern, &mut names);
                for n in &names { scope.declare(n); }
            }
            collect_refs_in_fn_body(&arrow.body, module_name, rename_map, scope, edits);
            scope.pop();
        }

        // === Function expression: new scope ===
        Expression::FunctionExpression(fe) => {
            scope.push();
            for param in &fe.params.items {
                let mut names = Vec::new();
                collect_binding_names(&param.pattern, &mut names);
                for n in &names { scope.declare(n); }
            }
            if let Some(ref body) = fe.body {
                collect_refs_in_fn_body(body, module_name, rename_map, scope, edits);
            }
            scope.pop();
        }

        // === Member expressions: visit object; computed case: visit expression too ===
        Expression::ComputedMemberExpression(cme) => {
            collect_refs_in_expr(&cme.object, module_name, rename_map, scope, edits);
            collect_refs_in_expr(&cme.expression, module_name, rename_map, scope, edits);
        }
        Expression::StaticMemberExpression(sme) => {
            collect_refs_in_expr(&sme.object, module_name, rename_map, scope, edits);
        }
        Expression::PrivateFieldExpression(_) => {}

        // === TS type expressions: visit inner expression ===
        Expression::TSAsExpression(ts) => {
            collect_refs_in_expr(&ts.expression, module_name, rename_map, scope, edits);
        }
        Expression::TSSatisfiesExpression(ts) => {
            collect_refs_in_expr(&ts.expression, module_name, rename_map, scope, edits);
        }
        Expression::TSTypeAssertion(ts) => {
            collect_refs_in_expr(&ts.expression, module_name, rename_map, scope, edits);
        }
        Expression::TSNonNullExpression(ts) => {
            collect_refs_in_expr(&ts.expression, module_name, rename_map, scope, edits);
        }

        // === JSX: not relevant for JS-to-Zig ===
        Expression::JSXElement(_) | Expression::JSXFragment(_) => {}

        // === Other expressions with no sub-expressions or not relevant ===
        Expression::MetaProperty(_) | Expression::Super(_) | Expression::ThisExpression(_) => {}
        Expression::ImportExpression(_) | Expression::ClassExpression(_) => {}
        Expression::PrivateInExpression(_) => {}

        _ => {} // Catch-all for future variants
    }
}

/// Visit the left side of an assignment expression for references.
/// Skips the assigned variable itself; only visits sub-expressions
/// (e.g., the object of a member expression like `obj.prop = ...`).
fn visit_assign_target(
    target: &AssignmentTarget,
    module_name: &str,
    rename_map: &HashMap<(String, String), String>,
    scope: &mut ScopeTracker,
    edits: &mut Vec<Edit>,
) {
    match target {
        // Simple targets: the variable being assigned — NOT a reference to rename
        AssignmentTarget::AssignmentTargetIdentifier(_) => {}
        AssignmentTarget::PrivateFieldExpression(_) => {}
        // Member expressions: visit the object (it's a reference)
        AssignmentTarget::ComputedMemberExpression(cme) => {
            collect_refs_in_expr(&cme.object, module_name, rename_map, scope, edits);
            collect_refs_in_expr(&cme.expression, module_name, rename_map, scope, edits);
        }
        AssignmentTarget::StaticMemberExpression(sme) => {
            collect_refs_in_expr(&sme.object, module_name, rename_map, scope, edits);
        }
        // TS wrappers: unwrap inner expression
        AssignmentTarget::TSAsExpression(ts) => {
            collect_refs_in_expr(&ts.expression, module_name, rename_map, scope, edits);
        }
        AssignmentTarget::TSSatisfiesExpression(ts) => {
            collect_refs_in_expr(&ts.expression, module_name, rename_map, scope, edits);
        }
        AssignmentTarget::TSNonNullExpression(ts) => {
            collect_refs_in_expr(&ts.expression, module_name, rename_map, scope, edits);
        }
        AssignmentTarget::TSTypeAssertion(ts) => {
            collect_refs_in_expr(&ts.expression, module_name, rename_map, scope, edits);
        }
        // Destructuring patterns: visit default value expressions
        AssignmentTarget::ArrayAssignmentTarget(arr) => {
            for elem in arr.elements.iter().flatten() {
                if let AssignmentTargetMaybeDefault::AssignmentTargetWithDefault(atd) = elem {
                    collect_refs_in_expr(
                        &atd.init, module_name, rename_map, scope, edits,
                    );
                }
            }
            // rest element: binding only, no expressions to visit
        }
        AssignmentTarget::ObjectAssignmentTarget(obj) => {
            for prop in &obj.properties {
                if let AssignmentTargetProperty::AssignmentTargetPropertyProperty(pp) = prop
                    && let AssignmentTargetMaybeDefault::AssignmentTargetWithDefault(atd) =
                        &pp.binding
                {
                    collect_refs_in_expr(
                        &atd.init, module_name, rename_map, scope, edits,
                    );
                }
            }
            // rest element: binding only, no expressions to visit
        }
    }
}

/// Visit a SimpleAssignmentTarget inside an UpdateExpression (++/--)
/// to find references in member expression objects.
fn visit_update_target(
    target: &SimpleAssignmentTarget,
    module_name: &str,
    rename_map: &HashMap<(String, String), String>,
    scope: &mut ScopeTracker,
    edits: &mut Vec<Edit>,
) {
    match target {
        SimpleAssignmentTarget::AssignmentTargetIdentifier(_) => {}
        SimpleAssignmentTarget::ComputedMemberExpression(cme) => {
            collect_refs_in_expr(&cme.object, module_name, rename_map, scope, edits);
            collect_refs_in_expr(&cme.expression, module_name, rename_map, scope, edits);
        }
        SimpleAssignmentTarget::StaticMemberExpression(sme) => {
            collect_refs_in_expr(&sme.object, module_name, rename_map, scope, edits);
        }
        _ => {} // PrivateField, TS variants
    }
}

/// Visit a ChainElement (optional chaining inner expression) for references.
fn visit_chain_element(
    element: &ChainElement,
    module_name: &str,
    rename_map: &HashMap<(String, String), String>,
    scope: &mut ScopeTracker,
    edits: &mut Vec<Edit>,
) {
    match element {
        ChainElement::CallExpression(call) => {
            collect_refs_in_expr(&call.callee, module_name, rename_map, scope, edits);
            for arg in &call.arguments {
                if let Some(e) = arg.as_expression() {
                    collect_refs_in_expr(e, module_name, rename_map, scope, edits);
                }
            }
        }
        ChainElement::ComputedMemberExpression(cme) => {
            collect_refs_in_expr(&cme.object, module_name, rename_map, scope, edits);
            collect_refs_in_expr(&cme.expression, module_name, rename_map, scope, edits);
        }
        ChainElement::StaticMemberExpression(sme) => {
            collect_refs_in_expr(&sme.object, module_name, rename_map, scope, edits);
        }
        ChainElement::PrivateFieldExpression(_) => {}
        _ => {} // TS variants
    }
}

/// Visit an ArrayExpressionElement to find references.
fn visit_array_element(
    elem: &ArrayExpressionElement,
    module_name: &str,
    rename_map: &HashMap<(String, String), String>,
    scope: &mut ScopeTracker,
    edits: &mut Vec<Edit>,
) {
    match elem {
        ArrayExpressionElement::SpreadElement(sp) => {
            collect_refs_in_expr(&sp.argument, module_name, rename_map, scope, edits);
        }
        _ => {
            // Expression variants (NumericLiteral, CallExpression, etc.)
            if let Some(e) = elem.as_expression() {
                collect_refs_in_expr(e, module_name, rename_map, scope, edits);
            }
        }
    }
}

fn collect_refs_in_fn_body(
    body: &FunctionBody,
    module_name: &str,
    rename_map: &HashMap<(String, String), String>,
    scope: &mut ScopeTracker,
    edits: &mut Vec<Edit>,
) {
    for stmt in &body.statements {
        collect_refs_in_stmt(stmt, module_name, rename_map, scope, edits);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn write_temp_files(files: &[(&str, &str)]) -> String {
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        let tmp = std::env::temp_dir().join(format!("js2rust_test_preprocess_{id}"));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        for (name, content) in files {
            fs::write(tmp.join(name), content).unwrap();
        }
        tmp.to_string_lossy().to_string()
    }

    #[test]
    fn preprocess_single_file() {
        let dir = write_temp_files(&[("main.js", "function add(a,b) { return a+b; }")]);
        let result = preprocess(&dir);
        assert!(result.diagnostics.is_empty());
        // Preprocess may prefix names to avoid conflicts.
        assert!(!result.merged_js().is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn preprocess_with_import() {
        let dir = write_temp_files(&[
            ("math.js", "export function add(a,b) { return a+b; }"),
            (
                "main.js",
                "import { add } from './math.js';\nfunction test() { return add(1,2); }",
            ),
        ]);
        let result = preprocess(&dir);
        let has_err = result.diagnostics.iter().any(|d| d.starts_with("error:"));
        assert!(!has_err, "unexpected errors: {:?}", result.diagnostics);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn preprocess_empty_dir() {
        let dir = write_temp_files(&[]);
        let result = preprocess(&dir);
        // Empty dir yields no JS files → merged_js is empty, but no hard error.
        assert!(result.merged_js().is_empty());
        let _ = fs::remove_dir_all(&dir);
    }
}
