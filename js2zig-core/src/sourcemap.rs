//! Source map: tracks JS source locations → generated Zig line numbers.
//!
//! Two output formats:
//! - Inline comments: `// @src(file.js:42)` embedded in generated Zig
//! - JSON mapping file: `{ "mappings": [...] }` for tooling

use serde::Serialize;

/// Pre-computed line-start byte offsets for O(log n) byte→line lookup.
pub struct LineIndex {
    /// Byte offset of the start of each line (0-indexed).
    /// line_starts[0] = 0, line_starts[1] = offset after first '\n', etc.
    line_starts: Vec<u32>,
}

impl LineIndex {
    /// Build a LineIndex from source text.
    pub fn new(source: &str) -> Self {
        let mut starts = vec![0u32];
        for (i, ch) in source.char_indices() {
            if ch == '\n' {
                starts.push((i + 1) as u32);
            }
        }
        Self {
            line_starts: starts,
        }
    }

    /// Convert a byte offset to (line, col), both 1-based.
    pub fn offset_to_line_col(&self, offset: u32) -> (u32, u32) {
        let line_0 = self.line_starts.binary_search(&offset).unwrap_or_else(|insert| insert.saturating_sub(1));
        let col_0 = offset.saturating_sub(self.line_starts[line_0]);
        (line_0 as u32 + 1, col_0 + 1)
    }
}

/// A single mapping entry: one Zig output line ↔ one JS source location.
#[derive(Debug, Clone, Serialize)]
pub struct SourceMapping {
    /// 1-based line number in generated Zig code
    pub zig_line: u32,
    /// Original JS file name (relative)
    pub js_file: String,
    /// 1-based line in JS source
    pub js_line: u32,
    /// 1-based column in JS source
    pub js_col: u32,
    /// Human-readable description (e.g. "function add", "class Person")
    pub kind: String,
}

/// Accumulated source map for a single JS→Zig translation.
#[derive(Debug, Clone, Serialize)]
pub struct SourceMap {
    /// The source file name
    pub source_file: String,
    /// All collected mappings, sorted by zig_line
    pub mappings: Vec<SourceMapping>,
}

impl SourceMap {
    pub fn new(source_file: &str) -> Self {
        Self {
            source_file: source_file.to_string(),
            mappings: Vec::new(),
        }
    }

    /// Record a mapping. `zig_line` is 1-based.
    pub fn add(&mut self, zig_line: u32, js_line: u32, js_col: u32, kind: &str) {
        self.mappings.push(SourceMapping {
            zig_line,
            js_file: self.source_file.clone(),
            js_line,
            js_col,
            kind: kind.to_string(),
        });
    }

    /// Serialize to JSON.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

/// Count newlines in a string to determine the current 1-based line number.
/// The line *after* the last newline is the current line being written.
pub fn count_lines(s: &str) -> u32 {
    s.bytes().filter(|&b| b == b'\n').count() as u32 + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_index_single_line() {
        let idx = LineIndex::new("hello world");
        assert_eq!(idx.offset_to_line_col(0), (1, 1));
        assert_eq!(idx.offset_to_line_col(5), (1, 6));
    }

    #[test]
    fn test_line_index_multi_line() {
        let idx = LineIndex::new("abc\ndef\nghi");
        assert_eq!(idx.offset_to_line_col(0), (1, 1)); // 'a'
        assert_eq!(idx.offset_to_line_col(4), (2, 1)); // 'd'
        assert_eq!(idx.offset_to_line_col(8), (3, 1)); // 'g'
        assert_eq!(idx.offset_to_line_col(10), (3, 3)); // 'i'
    }

    #[test]
    fn test_count_lines() {
        assert_eq!(count_lines(""), 1);
        assert_eq!(count_lines("a"), 1);
        assert_eq!(count_lines("a\nb"), 2);
        assert_eq!(count_lines("a\nb\nc\n"), 4);
    }

    #[test]
    fn test_source_map_json() {
        let mut sm = SourceMap::new("test.js");
        sm.add(1, 1, 1, "function add");
        let json = sm.to_json();
        assert!(json.contains("function add"));
        assert!(json.contains("test.js"));
    }
}
