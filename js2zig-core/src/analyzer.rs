//! Dependency analyzer — scan all JS files, build import graph,
//! and partition files into groups (one core file + its transitive deps).
//!
//! A "core file" is a file that is NOT imported by any other file.
//! Each core file becomes the root of a group. Non-core files can
//! belong to multiple groups.

use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Convert a module name (from filename) to a valid Zig identifier suffix.
/// Non-ASCII characters are converted to Unicode codepoint format `_uXXXX`.
pub fn sanitize_module_name(raw: &str) -> String {
    let mut result = String::new();
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            result.push(ch);
        } else {
            result.push_str(&format!("_u{:04x}", ch as u32));
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

/// Strip `import` and `export` statements from JS source,
/// and collect all exported function/var/class names.
///
/// Returns `(cleaned_source, exports_set)`.
pub fn strip_imports_extract_exports(src: &str) -> (String, HashSet<String>) {
    let mut result = String::with_capacity(src.len());
    let mut exports = HashSet::new();

    for line in src.lines() {
        let trimmed = line.trim();

        // Skip import statements entirely
        if trimmed.starts_with("import ") {
            continue;
        }

        // export function foo() → keep "function foo()", record "foo"
        if let Some(rest) = trimmed.strip_prefix("export ") {
            if let Some(after) = rest.strip_prefix("function ") {
                if let Some(paren) = after.find('(') {
                    exports.insert(after[..paren].trim().to_string());
                }
                // Keep the line without "export " prefix
                result.push_str(rest);
            } else if let Some(after) = rest.strip_prefix("async function ") {
                if let Some(paren) = after.find('(') {
                    exports.insert(after[..paren].trim().to_string());
                }
                result.push_str(rest);
            } else if rest.starts_with("const ") || rest.starts_with("let ") || rest.starts_with("var ") {
                let kw_len = if rest.starts_with("const ") { 6 } else { 4 };
                if let Some(eq) = rest[kw_len..].find('=') {
                    let name = rest[kw_len..kw_len + eq].trim();
                    exports.insert(name.to_string());
                }
                result.push_str(rest);
            } else if let Some(after_class) = rest.strip_prefix("class ") {
                if let Some(br) = after_class.find([' ', '{']) {
                    exports.insert(after_class[..br].trim().to_string());
                }
                result.push_str(rest);
            } else if let Some(after_default) = rest.strip_prefix("default ") {
                // export default function/class → unwrap, extract name
                if let Some(after_fn) = after_default.strip_prefix("function ") {
                    if let Some(paren) = after_fn.find('(') {
                        let name = after_fn[..paren].trim();
                        if !name.is_empty() {
                            exports.insert(name.to_string());
                        }
                    }
                } else if let Some(after_class) = after_default.strip_prefix("class ")
                    && let Some(br) = after_class.find([' ', '{'])
                {
                    exports.insert(after_class[..br].trim().to_string());
                }
                result.push_str(after_default);
            } else if rest.starts_with('{') {
                // export { name1, name2, ... } — extract names, skip the whole line
                if let Some(braces) = rest.strip_prefix('{')
                    .and_then(|s| s.strip_suffix('}'))
                    .or_else(|| rest.strip_prefix('{').and_then(|s| {
                        // Multi-line export { ... } — try to find closing brace
                        s.find('}').map(|i| &s[..i])
                    }))
                {
                    for name in braces.split(',') {
                        let name = name.trim();
                        // Also handle `foo as bar` → export "bar" (the exported name)
                        if let Some(alias) = name.strip_prefix("as ") {
                            let alias = alias.trim();
                            if !alias.is_empty() {
                                exports.insert(alias.to_string());
                            }
                        } else if !name.is_empty() && !name.starts_with("as ") {
                            exports.insert(name.to_string());
                        }
                    }
                }
                continue;
            } else {
                // export * or other unknown syntax — skip the whole line
                continue;
            }
            result.push('\n');
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }

    (result, exports)
}

/// Strip .js extension then sanitize for Zig identifier.
fn sanitize_name(filename: &str) -> String {
    let stem = filename.strip_suffix(".js").unwrap_or(filename);
    sanitize_module_name(stem)
}

/// A file group: one core file + all files transitively imported by it.
#[derive(Debug, Clone)]
pub struct FileGroup {
    /// Sanitized core file name (used as Zig project name).
    pub core_name: String,
    /// Original .js filename of the core file (e.g. "main.js").
    pub core_file: String,
    /// All .js filenames in this group (including core, in topological order).
    pub members: Vec<String>,
    /// Map: original filename → sanitzed Zig module name.
    pub name_map: HashMap<String, String>,
    /// Map: original filename → Vec<(imported_name, source_filename)>.
    /// e.g. "main.js" → [("add","math.js"), ("multiply","math.js"), ("greet","string_utils.js")]
    pub imported_names: HashMap<String, Vec<(String, String)>>,
}

/// Analyze a single core JS file and its transitive dependencies.
///
/// Only processes the specified core file and the files it imports,
/// rather than scanning an entire directory.
///
/// # Returns
/// - `groups`: Vec containing a single FileGroup.
/// - `groups_json`: JSON-serializable summary for `out/groups.json`.
pub fn analyze_single_group(in_dir: &str, core_file: &str) -> (Vec<FileGroup>, String) {
    let in_path = Path::new(in_dir);

    // 1. BFS/DFS from core_file to collect all transitively imported files.
    let mut visited: HashSet<String> = HashSet::new();
    let mut stack: Vec<String> = vec![core_file.to_string()];
    let mut js_files: Vec<String> = Vec::new();

    while let Some(cur) = stack.pop() {
        if !visited.insert(cur.clone()) {
            continue;
        }
        js_files.push(cur.clone());

        let src = std::fs::read_to_string(in_path.join(&cur))
            .unwrap_or_else(|e| panic!("Cannot read '{}': {}", cur, e));
        let (file_imports, _exports, _decls, _file_imported_names) = extract_imports(&src, &cur);
        for dep in &file_imports {
            if !visited.contains(dep.as_str()) {
                stack.push(dep.clone());
            }
        }
    }

    // 2. Parse each discovered file for imports + imported names.
    let mut imports: HashMap<String, Vec<String>> = HashMap::new();
    let mut imported_names: HashMap<String, Vec<(String, String)>> = HashMap::new();
    let mut sanitzed: HashMap<String, String> = HashMap::new();

    for file in &js_files {
        let src = std::fs::read_to_string(in_path.join(file))
            .unwrap_or_else(|e| panic!("Cannot read '{}': {}", file, e));
        let (file_imports, _exports, _decls, file_imported_names) = extract_imports(&src, file);
        imports.insert(file.clone(), file_imports);
        imported_names.insert(file.clone(), file_imported_names);
        sanitzed.insert(file.clone(), sanitize_name(file));
    }

    // 3. Build the single group.
    let members = transitive_deps(core_file, &imports);
    let core_name = sanitzed.get(core_file).cloned().unwrap_or_else(|| {
        sanitize_name(core_file)
    });

    let group = FileGroup {
        core_name: core_name.clone(),
        core_file: core_file.to_string(),
        members,
        name_map: sanitzed,
        imported_names,
    };

    let groups = vec![group];
    let groups_json = serde_json::to_string_pretty(&groups_to_json(&groups))
        .expect("Failed to serialize groups.json");

    (groups, groups_json)
}

/// Compute transitive dependencies of `file` (including itself).
fn transitive_deps(
    file: &str,
    imports: &HashMap<String, Vec<String>>,
) -> Vec<String> {
    let mut visited: HashSet<String> = HashSet::new();
    let mut stack: Vec<String> = vec![file.to_string()];
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

/// Return type for extract_imports.
type ImportMeta = (Vec<String>, Vec<String>, Vec<String>, Vec<(String, String)>);

/// Extract `import { ... } from '...'` statements from JS source.
///
/// Returns (imported_files, exported_names, top_level_declarations, imported_names).
///
/// `imported_names`: Vec<(imported_name, source_filename)> — e.g. [("add","math.js"), ("multiply","math.js")]
fn extract_imports(
    src: &str,
    _filename: &str,
) -> ImportMeta {
    // Use a simple regex-free approach: scan lines for "import" keyword.
    let mut imported_files: Vec<String> = Vec::new();
    let mut exported_names: Vec<String> = Vec::new();
    let mut declarations: Vec<String> = Vec::new();
    let mut imported_names: Vec<(String, String)> = Vec::new();

    for line in src.lines() {
        let trimmed = line.trim();

        // import { foo, bar } from './math.js';
        if trimmed.starts_with("import ")
            && let Some(from_idx) = trimmed.find(" from ") {
                // Extract the path
                let specifier = &trimmed[from_idx + 6..];
                let specifier = specifier.trim();
                let path_str = if let Some(stripped) = specifier.strip_prefix('\'') {
                    stripped.find('\'').map(|end| &stripped[..end])
                } else if let Some(stripped) = specifier.strip_prefix('"') {
                    stripped.find('"').map(|end| &stripped[..end])
                } else {
                    None
                };

                let source_filename = path_str
                    .and_then(path_to_filename);

                if let Some(ref sf) = source_filename {
                    imported_files.push(sf.clone());
                }

                // Extract imported names from between `import ` and ` from `
                let import_clause = &trimmed["import ".len()..from_idx].trim();
                if let Some(brace_open) = import_clause.find('{')
                    && let Some(brace_close) = import_clause[brace_open..].find('}')
                {
                    let names_str = &import_clause[brace_open + 1..brace_open + brace_close];
                    for name in names_str.split(',') {
                        let name = name.trim();
                        if !name.is_empty()
                            && let Some(ref sf) = source_filename
                        {
                            imported_names.push((name.to_string(), sf.clone()));
                        }
                    }
                }
            }

        // export function foo() {}  or  export const foo = ...
        if let Some(rest) = trimmed.strip_prefix("export ") {
            if let Some(after) = rest.strip_prefix("function ") {
                if let Some(name_end) = after.find('(') {
                    exported_names.push(after[..name_end].trim().to_string());
                }
            } else if rest.starts_with("const ") || rest.starts_with("let ") || rest.starts_with("var ") {
                // export const foo = ... → extract "foo"
                let after = &rest[6..];
                if let Some(eq) = after.find('=') {
                    let name = after[..eq].trim();
                    exported_names.push(name.to_string());
                }
            } else if let Some(after) = rest.strip_prefix("class ") {
                if let Some(name_end) = after.find([' ', '{']) {
                    exported_names.push(after[..name_end].trim().to_string());
                }
            } else if rest.starts_with("async function ")
                && let Some(name_end) = rest[16..].find('(') {
                    exported_names.push(rest[16..16 + name_end].trim().to_string());
                }
        }

        // Top-level declarations (for conflict detection).
        if trimmed.starts_with("function ") && !trimmed.starts_with("//")
            && let Some(name_end) = trimmed[9..].find('(') {
                declarations.push(trimmed[9..9 + name_end].trim().to_string());
            }
        if (trimmed.starts_with("const ") || trimmed.starts_with("let ") || trimmed.starts_with("var "))
            && !trimmed.contains("export ")
        {
            let prefix = if trimmed.starts_with("const ") { 6 } else { 4 };
            let after = &trimmed[prefix..];
            if let Some(eq) = after.find('=') {
                let name = after[..eq].trim();
                declarations.push(name.to_string());
            }
        }
    }

    // Deduplicate.
    imported_files.sort();
    imported_files.dedup();
    imported_names.sort();
    imported_names.dedup();

    (imported_files, exported_names, declarations, imported_names)
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

/// Convert groups to JSON-serializable map.
fn groups_to_json(groups: &[FileGroup]) -> serde_json::Value {
    let entries: Vec<serde_json::Value> = groups
        .iter()
        .map(|g| {
            serde_json::json!({
                "name": g.core_name,
                "core_file": g.core_file,
                "member_count": g.members.len(),
                "members": g.members,
            })
        })
        .collect();
    serde_json::Value::Array(entries)
}
