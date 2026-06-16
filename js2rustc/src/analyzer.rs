//! Dependency analyzer — scan all JS files, build import graph,
//! and partition files into groups (one core file + its transitive deps).
//!
//! A "core file" is a file that is NOT imported by any other file.
//! Each core file becomes the root of a group. Non-core files can
//! belong to multiple groups.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use crate::preprocess::sanitize_module_name;

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
}

/// Analyze all JS files in `in_dir` and return file groups.
///
/// # Returns
/// - `groups`: Vec of FileGroup, one per core file.
/// - `groups_json`: JSON-serializable summary for `out/groups.json`.
pub fn analyze_groups(in_dir: &str) -> (Vec<FileGroup>, String) {
    let in_path = Path::new(in_dir);

    // 1. Discover all .js files.
    let js_files: Vec<String> = std::fs::read_dir(in_path)
        .unwrap_or_else(|e| panic!("Cannot read in_dir '{}': {}", in_dir, e))
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension()? == "js" {
                Some(path.file_name()?.to_string_lossy().to_string())
            } else {
                None
            }
        })
        .collect();

    // 2. Parse each file → extract imports.
    let mut imports: HashMap<String, Vec<String>> = HashMap::new();
    let mut sanitzed: HashMap<String, String> = HashMap::new();

    for file in &js_files {
        let src = std::fs::read_to_string(in_path.join(file))
            .unwrap_or_else(|e| panic!("Cannot read '{}': {}", file, e));
        let (file_imports, _exports, _decls) = extract_imports(&src, file);
        imports.insert(file.clone(), file_imports);
        sanitzed.insert(file.clone(), sanitize_name(file));
    }

    // 3. Build reverse dependency map: who imports F?
    let mut imported_by: HashMap<String, Vec<String>> = HashMap::new();
    for (importer, deps) in &imports {
        for dep in deps {
            imported_by
                .entry(dep.clone())
                .or_default()
                .push(importer.clone());
        }
    }

    // 4. Core files = files NOT imported by any other file.
    let core_files: Vec<String> = js_files
        .iter()
        .filter(|f| {
            imported_by
                .get(*f)
                .is_none_or(|importers| importers.is_empty())
        })
        .cloned()
        .collect();

    // 5. For each core file, compute transitive closure.
    let mut groups: Vec<FileGroup> = Vec::new();
    for core in &core_files {
        let members = transitive_deps(core, &imports);
        let core_name = sanitzed.get(core).cloned().unwrap_or_else(|| {
            sanitize_name(core)
        });
        let name_map = sanitzed.clone();

        groups.push(FileGroup {
            core_name: core_name.clone(),
            core_file: core.clone(),
            members,
            name_map,
        });
    }

    // 6. Serialize groups.json.
    let groups_json = serde_json::to_string_pretty(&groups_to_json(&groups)).unwrap();

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
    result
}

/// Extract `import { ... } from '...'` statements from JS source.
///
/// Returns (imported_files, exported_names, top_level_declarations).
fn extract_imports(
    src: &str,
    _filename: &str,
) -> (Vec<String>, Vec<String>, Vec<String>) {
    // Use a simple regex-free approach: scan lines for "import" keyword.
    // This is a simplified version of preprocess.rs's extract_module_meta().
    //
    // For full accuracy we should use OXC, but for the grouping step
    // we only need import paths — a line scan is sufficient.
    let mut imported_files: Vec<String> = Vec::new();
    let mut exported_names: Vec<String> = Vec::new();
    let mut declarations: Vec<String> = Vec::new();

    for line in src.lines() {
        let trimmed = line.trim();

        // import { foo, bar } from './math.js';
        if trimmed.starts_with("import ")
            && let Some(from_idx) = trimmed.find(" from ") {
                let specifier = &trimmed[from_idx + 6..];
                let specifier = specifier.trim();
                if let Some(stripped) = specifier.strip_prefix('\'') {
                    if let Some(end) = stripped.find('\'') {
                        let path = &stripped[..end];
                        if let Some(filename) = path_to_filename(path) {
                            imported_files.push(filename);
                        }
                    }
                } else if let Some(stripped) = specifier.strip_prefix('"')
                    && let Some(end) = stripped.find('"') {
                        let path = &stripped[..end];
                        if let Some(filename) = path_to_filename(path) {
                            imported_files.push(filename);
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

    (imported_files, exported_names, declarations)
}

/// Convert an import path like './math.js' or '../utils.js' to a filename.
fn path_to_filename(path: &str) -> Option<String> {
    let path = Path::new(path);
    path.file_name()?.to_string_lossy().to_string();
    Some(path.file_name()?.to_string_lossy().to_string())
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
