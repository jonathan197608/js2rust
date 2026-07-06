// zigir/ident.rs
// IR identifier + name mangling.

/// Zig reserved keywords that JS identifiers might collide with.
/// Pre-pends `_` to avoid collisions.
const ZIG_RESERVED_KEYWORDS: &[&str] = &[
    "addrspace",
    "align",
    "allowzero",
    "and",
    "anyframe",
    "anytype",
    "asm",
    "async",
    "await",
    "break",
    "callconv",
    "catch",
    "comptime",
    "const",
    "continue",
    "defer",
    "else",
    "enum",
    "errdefer",
    "error",
    "export",
    "extern",
    "fn",
    "for",
    "if",
    "inline",
    "linksection",
    "noalias",
    "noinline",
    "nosuspend",
    "opaque",
    "or",
    "orelse",
    "packed",
    "pub",
    "resume",
    "return",
    "struct",
    "suspend",
    "switch",
    "test",
    "threadlocal",
    "try",
    "union",
    "unreachable",
    "usingnamespace",
    "var",
    "volatile",
    "while",
];

/// Convert a JS identifier to a Zig-safe identifier.
/// Pre-pends `_` to avoid collisions with Zig reserved keywords.
/// Also escapes `_` (Zig discard identifier) to `__js_underscore`.
pub fn zig_safe_name(name: &str) -> String {
    if name == "_" {
        "__js_underscore".to_string()
    } else if ZIG_RESERVED_KEYWORDS.contains(&name) {
        format!("_{}", name)
    } else {
        name.to_string()
    }
}

/// IR identifier — stores both the original JS name and the escaped Zig name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IrIdent {
    /// Original JS name (for diagnostics / source mapping).
    pub js_name: String,
    /// Escaped Zig name (keyword-safe, shadow-renamed).
    pub zig_name: String,
}

impl IrIdent {
    /// Create a new IrIdent with automatic keyword escaping (no shadow rename).
    pub fn new(js_name: &str) -> Self {
        Self {
            js_name: js_name.to_string(),
            zig_name: zig_safe_name(js_name),
        }
    }

    /// Create an IrIdent with an explicit Zig name (already escaped/renamed).
    pub fn with_zig_name(js_name: &str, zig_name: String) -> Self {
        Self {
            js_name: js_name.to_string(),
            zig_name,
        }
    }
}

impl std::fmt::Display for IrIdent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.zig_name)
    }
}

/// Name mangling counter + shadow rename manager.
/// Generates unique identifiers for closures, labels, blocks, etc.
#[derive(Debug, Clone)]
pub struct NameMangler {
    /// Counter per prefix: "closure" → 0, "arrow" → 0, "blk" → 0, etc.
    counters: std::collections::HashMap<String, usize>,
    /// Shadow renaming stack: each scope level has a map JS-name → renamed Zig-name.
    shadow_scopes: Vec<std::collections::HashMap<String, String>>,
}

impl NameMangler {
    pub fn new() -> Self {
        Self {
            counters: std::collections::HashMap::new(),
            shadow_scopes: Vec::new(),
        }
    }

    /// Generate the next unique name for a given prefix.
    /// Always includes a counter: `"{prefix}_0"`, `"{prefix}_1"`, `"{prefix}_2"`, etc.
    /// This uses the standard naming convention (e.g., `_js_dest_0`, `_js_dest_1`).
    pub fn next_name(&mut self, prefix: &str) -> String {
        let count = self.counters.entry(prefix.to_string()).or_insert(0);
        let name = format!("{}_{}", prefix, count);
        *count += 1;
        name
    }

    /// Peek at the current counter value for a prefix (does not increment).
    pub fn peek_count(&self, prefix: &str) -> usize {
        self.counters.get(prefix).copied().unwrap_or(0)
    }

    /// Push a new shadowing scope onto the rename stack.
    pub fn push_shadow_scope(&mut self) {
        self.shadow_scopes.push(std::collections::HashMap::new());
    }

    /// Pop the innermost shadowing scope from the rename stack.
    pub fn pop_shadow_scope(&mut self) {
        self.shadow_scopes.pop();
    }

    /// Record a shadow rename in the current scope.
    pub fn record_shadow(&mut self, js_name: &str, zig_name: String) {
        if let Some(scope) = self.shadow_scopes.last_mut() {
            scope.insert(js_name.to_string(), zig_name);
        }
    }

    /// Resolve a JS identifier through the shadow scope stack.
    /// Returns the Zig-safe name, checking shadow renames from innermost to outermost.
    pub fn resolve_name(&self, js_name: &str) -> String {
        for scope in self.shadow_scopes.iter().rev() {
            if let Some(renamed) = scope.get(js_name) {
                return renamed.clone();
            }
        }
        zig_safe_name(js_name)
    }

    /// Create an IrIdent for the given JS name, applying shadow renaming.
    pub fn make_ident(&self, js_name: &str) -> IrIdent {
        let zig_name = self.resolve_name(js_name);
        IrIdent::with_zig_name(js_name, zig_name)
    }
}

impl Default for NameMangler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zig_safe_name_normal() {
        assert_eq!(zig_safe_name("foo"), "foo");
    }

    #[test]
    fn test_zig_safe_name_keyword() {
        assert_eq!(zig_safe_name("return"), "_return");
        assert_eq!(zig_safe_name("fn"), "_fn");
        assert_eq!(zig_safe_name("try"), "_try");
    }

    #[test]
    fn test_ir_ident_new() {
        let id = IrIdent::new("myVar");
        assert_eq!(id.js_name, "myVar");
        assert_eq!(id.zig_name, "myVar");
    }

    #[test]
    fn test_ir_ident_keyword() {
        let id = IrIdent::new("error");
        assert_eq!(id.js_name, "error");
        assert_eq!(id.zig_name, "_error");
    }

    #[test]
    fn test_zig_safe_name_underscore() {
        assert_eq!(zig_safe_name("_"), "__js_underscore");
        let id = IrIdent::new("_");
        assert_eq!(id.js_name, "_");
        assert_eq!(id.zig_name, "__js_underscore");
    }

    #[test]
    fn test_name_mangler_sequence() {
        let mut m = NameMangler::new();
        assert_eq!(m.next_name("closure"), "closure_0");
        assert_eq!(m.next_name("closure"), "closure_1");
        assert_eq!(m.next_name("closure"), "closure_2");
        // Different prefix starts fresh
        assert_eq!(m.next_name("blk"), "blk_0");
    }

    #[test]
    fn test_name_mangler_shadow() {
        let mut m = NameMangler::new();
        m.push_shadow_scope();
        m.record_shadow("x", "x_1".to_string());
        assert_eq!(m.resolve_name("x"), "x_1");
        assert_eq!(m.resolve_name("y"), "y"); // not shadowed
        m.pop_shadow_scope();
        assert_eq!(m.resolve_name("x"), "x"); // scope gone
    }

    #[test]
    fn test_name_mangler_nested_shadow() {
        let mut m = NameMangler::new();
        m.push_shadow_scope();
        m.record_shadow("x", "x_1".to_string());
        m.push_shadow_scope();
        m.record_shadow("x", "x_2".to_string());
        assert_eq!(m.resolve_name("x"), "x_2"); // innermost wins
        m.pop_shadow_scope();
        assert_eq!(m.resolve_name("x"), "x_1"); // back to outer
        m.pop_shadow_scope();
        assert_eq!(m.resolve_name("x"), "x"); // original
    }

    #[test]
    fn test_make_ident_with_shadow() {
        let mut m = NameMangler::new();
        m.push_shadow_scope();
        m.record_shadow("type", "type_1".to_string());
        let id = m.make_ident("type");
        assert_eq!(id.js_name, "type");
        assert_eq!(id.zig_name, "type_1"); // shadow overrides keyword escaping
    }
}
