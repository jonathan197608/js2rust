//! Dependency analyzer — scan all JS files, build import graph,
//! and collect analysis results for the transpiler pipeline.
//!
//! The entry JS file and its transitive dependencies form a single
//! project. Non-entry files can be imported by multiple dependents.

use std::collections::{HashMap, HashSet};
use std::fmt::{self, Write};
use std::path::Path;

use oxc_allocator::Allocator;
use oxc_ast::ast::*;

/// Convert a module name (from filename) to a valid Zig identifier suffix.
/// Non-ASCII characters are converted to Unicode codepoint format `_uXXXX`.
#[must_use]
pub fn sanitize_module_name(raw: &str) -> String {
    let mut result = String::new();
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            result.push(ch);
        } else {
            let _ = write!(result, "_u{:04x}", ch as u32);
        }
    }
    if result.is_empty() {
        result.push_str("_unnamed");
    }
    if !result.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_') {
        result.insert(0, '_');
    }
    // Prevent collision with the Zig orchestrator file `lib.zig`
    if result == "lib" {
        result.push_str("_mod");
    }
    result
}

/// Strip .js extension then sanitize for Zig identifier.
fn sanitize_name(filename: &str) -> String {
    let stem = filename.strip_suffix(".js").unwrap_or(filename);
    sanitize_module_name(stem)
}

/// Analysis result: the entry file + all its transitive dependencies,
/// with parsed ASTs and import/export metadata.
pub struct AnalysisResult {
    /// Sanitized entry file name (used as Zig project name).
    pub core_name: String,
    /// Original .js filename of the entry file (e.g. "main.js").
    pub core_file: String,
    /// All .js filenames in this project (including entry, in topological order).
    pub members: Vec<String>,
    /// Map: original filename → sanitized Zig module name.
    pub name_map: HashMap<String, String>,
    /// Map: original filename → Vec<(imported_name, source_filename)>.
    /// e.g. "main.js" → [("add","math.js"), ("multiply","math.js"), ("greet","string_utils.js")]
    pub imported_names: HashMap<String, Vec<(String, String)>>,
    /// Map: original filename → exported function/var/class names (from AST).
    pub exported_names: HashMap<String, HashSet<String>>,
    /// Map: original filename → ALL toplevel function names (from AST, for test projects).
    pub all_fn_names: HashMap<String, HashSet<String>>,
    /// Cached source text per file (eliminates repeated I/O in the transpiler pipeline).
    pub file_sources: HashMap<String, String>,
    /// Parsed AST programs (one per file).  Allocators are leaked via Box::leak
    /// so programs carry 'static lifetime — safe for a CLI transpiler where the
    /// process exits shortly after.  Stored here so the Lowerer can reuse the AST
    /// without re-parsing the source text.
    pub parsed_programs: HashMap<String, Program<'static>>,
}

// Manual Debug: skip parsed_programs (AST is huge, not useful for debug output).
impl fmt::Debug for AnalysisResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AnalysisResult")
            .field("core_name", &self.core_name)
            .field("core_file", &self.core_file)
            .field("members", &self.members)
            .field("name_map", &self.name_map)
            .field("imported_names", &self.imported_names)
            .field("exported_names", &self.exported_names)
            .field("all_fn_names", &self.all_fn_names)
            .field(
                "file_sources",
                &format_args!("{} files", self.file_sources.len()),
            )
            .field(
                "parsed_programs",
                &format_args!("{} programs", self.parsed_programs.len()),
            )
            .finish()
    }
}

/// Analyze a JS entry file (and optional additional roots) and their
/// transitive dependencies, returning all data needed for transpilation.
///
/// Only processes the specified entry file(s) and the files they import,
/// rather than scanning an entire directory.
pub fn analyze_project(
    in_dir: &str,
    core_file: &str,
    additional_core_files: &[String],
) -> AnalysisResult {
    let in_path = Path::new(in_dir);

    // Single DFS pass: read + parse each file ONCE, extract import/export
    // metadata straight from the AST, and cache both the source text and the
    // parsed Program for Lowerer reuse (eliminates double-parsing).
    let mut visited: HashSet<String> = HashSet::new();
    let mut stack: Vec<String> = vec![core_file.to_string()];
    for addl_file in additional_core_files {
        stack.push(addl_file.clone());
    }
    // Reverse so the primary core_file is popped first (we parse it first).
    stack.reverse();

    let mut imports: HashMap<String, Vec<String>> = HashMap::new();
    let mut imported_names: HashMap<String, Vec<(String, String)>> = HashMap::new();
    let mut exported_names: HashMap<String, HashSet<String>> = HashMap::new();
    let mut all_fn_names: HashMap<String, HashSet<String>> = HashMap::new();
    let mut file_sources: HashMap<String, String> = HashMap::new();
    let mut parsed_programs: HashMap<String, Program<'static>> = HashMap::new();
    let mut sanitized: HashMap<String, String> = HashMap::new();

    while let Some(cur) = stack.pop() {
        if !visited.insert(cur.clone()) {
            continue;
        }

        let src = std::fs::read_to_string(in_path.join(&cur))
            .unwrap_or_else(|e| panic!("Cannot read '{}': {}", cur, e));

        // Parse ONCE: leak source for 'static lifetime, so the Program can be
        // stored in the HashMap and reused by the Lowerer later.
        // The oxc Allocator is shared across all files in this session (O(1) leak
        // instead of O(n) — bumpalo arena is safe for concurrent AST storage,
        // and the parse phase is single-threaded so the benign race on first
        // init cannot actually occur in practice).
        let allocator: &'static Allocator = {
            use std::sync::atomic::{AtomicPtr, Ordering};
            static ALLOC_PTR: AtomicPtr<Allocator> = AtomicPtr::new(std::ptr::null_mut());
            let ptr = ALLOC_PTR.load(Ordering::Acquire);
            if !ptr.is_null() {
                // SAFETY: ptr points to a leaked Box<Allocator> that lives for 'static
                unsafe { &*ptr }
            } else {
                let leaked: &'static mut Allocator = Box::leak(Box::new(Allocator::default()));
                ALLOC_PTR.store(leaked as *mut Allocator, Ordering::Release);
                leaked
            }
        };
        let src_static: &'static str = Box::leak(src.clone().into_boxed_str());
        let program: Program<'static> = crate::parser::parse_with_name(allocator, src_static, &cur);

        let info = analyze_module_ast(&program);

        for dep in &info.imported_files {
            if !visited.contains(dep.as_str()) {
                stack.push(dep.clone());
            }
        }

        sanitized.insert(cur.clone(), sanitize_name(&cur));
        imports.insert(cur.clone(), info.imported_files);
        imported_names.insert(cur.clone(), info.imported_names);
        exported_names.insert(cur.clone(), info.exported_names);
        all_fn_names.insert(cur.clone(), info.all_toplevel_fn_names);
        file_sources.insert(cur.clone(), src);
        parsed_programs.insert(cur, program);
    }

    // Build the result — all files from all roots merged.
    let all_roots: Vec<String> = std::iter::once(core_file.to_string())
        .chain(additional_core_files.iter().cloned())
        .collect();
    let members = transitive_deps_multi(&all_roots, &imports);
    let core_name = sanitized
        .get(core_file)
        .cloned()
        .unwrap_or_else(|| sanitize_name(core_file));

    AnalysisResult {
        core_name,
        core_file: core_file.to_string(),
        members,
        name_map: sanitized,
        imported_names,
        exported_names,
        all_fn_names,
        file_sources,
        parsed_programs,
    }
}

/// Compute transitive dependencies starting from multiple root files.
/// All roots and their transitive deps are merged into one topological list.
fn transitive_deps_multi(roots: &[String], imports: &HashMap<String, Vec<String>>) -> Vec<String> {
    let mut visited: HashSet<String> = HashSet::new();
    let mut stack: Vec<String> = roots.to_vec();
    let mut result: Vec<String> = Vec::new();

    // DFS produces core-first order; reverse for topological (deps-first).
    while let Some(cur) = stack.pop() {
        if !visited.insert(cur.clone()) {
            continue;
        }
        result.push(cur.clone());
        if let Some(deps) = imports.get(&cur) {
            for dep in deps {
                if !visited.contains(dep.as_str()) {
                    stack.push(dep.clone());
                }
            }
        }
    }
    result.reverse();
    result
}

/// Convert an import path like './math.js' or './math' to a filename.
fn path_to_filename(path: &str) -> Option<String> {
    let fname = Path::new(path).file_name()?.to_string_lossy().to_string();
    if fname.ends_with(".js") {
        Some(fname)
    } else {
        Some(format!("{}.js", fname))
    }
}

/// AST-extracted metadata for a single JS module file.
struct ModuleInfo {
    imported_files: Vec<String>,
    imported_names: Vec<(String, String)>,
    exported_names: HashSet<String>,
    all_toplevel_fn_names: HashSet<String>,
}

/// Extract import/export metadata from an oxc `Program` AST.
/// Replaces the old line-scanning `extract_imports` with accurate AST traversal.
fn analyze_module_ast(program: &Program) -> ModuleInfo {
    let mut imported_files = Vec::new();
    let mut imported_names = Vec::new();
    let mut exported_names = HashSet::new();
    let mut all_toplevel_fn_names = HashSet::new();

    for stmt in &program.body {
        match stmt {
            Statement::ImportDeclaration(import) => {
                let source_str = import.source.value.as_str();
                if let Some(sf) = path_to_filename(source_str) {
                    imported_files.push(sf.clone());
                    if let Some(specifiers) = &import.specifiers {
                        for spec in specifiers {
                            let name = match spec {
                                ImportDeclarationSpecifier::ImportSpecifier(s) => {
                                    s.local.name.as_str().to_string()
                                }
                                ImportDeclarationSpecifier::ImportDefaultSpecifier(s) => {
                                    s.local.name.as_str().to_string()
                                }
                                ImportDeclarationSpecifier::ImportNamespaceSpecifier(s) => {
                                    s.local.name.as_str().to_string()
                                }
                            };
                            imported_names.push((name, sf.clone()));
                        }
                    }
                }
            }
            Statement::ExportNamedDeclaration(e) => {
                // `export function foo() {}` / `export const foo = ...` / `export class Foo {}`
                if let Some(decl) = &e.declaration {
                    match decl {
                        Declaration::FunctionDeclaration(fd) => {
                            if let Some(id) = &fd.id {
                                let name = id.name.as_str().to_string();
                                exported_names.insert(name.clone());
                                all_toplevel_fn_names.insert(name);
                            }
                        }
                        Declaration::VariableDeclaration(vd) => {
                            for d in &vd.declarations {
                                if let BindingPattern::BindingIdentifier(id) = &d.id {
                                    exported_names.insert(id.name.as_str().to_string());
                                }
                            }
                        }
                        Declaration::ClassDeclaration(cd) => {
                            if let Some(id) = &cd.id {
                                exported_names.insert(id.name.as_str().to_string());
                            }
                        }
                        _ => {}
                    }
                }
                // `export { foo, bar as baz }`
                for spec in &e.specifiers {
                    if let ModuleExportName::IdentifierName(id) = &spec.exported {
                        exported_names.insert(id.name.as_str().to_string());
                    }
                }
            }
            Statement::ExportDefaultDeclaration(e) => {
                // `export default function foo() {}` → "foo"
                // `export default class Foo {}` → "Foo"
                if let Some(name) = match &e.declaration {
                    ExportDefaultDeclarationKind::FunctionDeclaration(fd) => {
                        fd.id.as_ref().map(|id| id.name.as_str().to_string())
                    }
                    ExportDefaultDeclarationKind::ClassDeclaration(cd) => {
                        cd.id.as_ref().map(|id| id.name.as_str().to_string())
                    }
                    _ => None,
                } {
                    exported_names.insert(name.clone());
                    all_toplevel_fn_names.insert(name);
                }
            }
            Statement::FunctionDeclaration(fd) => {
                if let Some(id) = &fd.id {
                    all_toplevel_fn_names.insert(id.name.as_str().to_string());
                }
            }
            Statement::ExpressionStatement(es) => {
                // Detect `module.exports = { foo, bar, ... }` (CommonJS style)
                if let Some(names) = extract_module_exports(es) {
                    for name in names {
                        exported_names.insert(name);
                    }
                }
            }
            _ => {}
        }
    }

    imported_files.sort();
    imported_files.dedup();
    imported_names.sort();
    imported_names.dedup();

    ModuleInfo {
        imported_files,
        imported_names,
        exported_names,
        all_toplevel_fn_names,
    }
}

/// Extract exported names from `module.exports = { ... }` expression statement.
/// Returns `Some(names)` if it's a CommonJS export pattern, `None` otherwise.
fn extract_module_exports(es: &ExpressionStatement) -> Option<Vec<String>> {
    use oxc_ast::ast::AssignmentTarget;

    let Expression::AssignmentExpression(assign) = &es.expression else {
        return None;
    };

    // Check left side: `module.exports`
    let AssignmentTarget::StaticMemberExpression(lhs) = &assign.left else {
        return None;
    };

    let Expression::Identifier(obj) = &lhs.object else {
        return None;
    };
    if obj.name.as_str() != "module" {
        return None;
    }
    if lhs.property.name.as_str() != "exports" {
        return None;
    }

    // Check right side: `{ ... }`
    match &assign.right {
        Expression::ObjectExpression(obj) => {
            let names: Vec<String> = obj
                .properties
                .iter()
                .filter_map(|p| match p {
                    ObjectPropertyKind::ObjectProperty(op) => match &op.key {
                        PropertyKey::StaticIdentifier(id) => Some(id.name.as_str().to_string()),
                        _ => None,
                    },
                    _ => None,
                })
                .collect();
            if names.is_empty() { None } else { Some(names) }
        }
        _ => None,
    }
}
