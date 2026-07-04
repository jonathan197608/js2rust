// zigir/lower/helpers.rs
// FnContext sub-structure and lowering utility methods.

use std::collections::{HashMap, HashSet};

use crate::types::ZigType;

// ═══════════════════════════════════════════════════════
//  FnContext — per-function lowering state
// ═══════════════════════════════════════════════════════

/// Per-function context for the Lowerer.
///
/// Groups the 6+ flags and per-function sets that were previously
/// scattered across the Codegen "god struct" as flat fields:
///
/// | Old Codegen fields            | FnContext field         |
/// |-------------------------------|-------------------------|
/// | current_fn                    | name                    |
/// | current_fn_is_export          | is_export               |
/// | current_fn_return_type        | return_type             |
/// | seen_return                   | seen_return             |
/// | fn_has_throw                  | fn_has_throw            |
/// | in_return_expr                | in_return_expr          |
/// | in_expr_stmt                  | in_expr_stmt            |
/// | call_generated_catch          | call_generated_catch    |
/// | inside_try_block              | inside_try_block        |
/// | current_class                 | current_class           |
/// | nested_fn_names               | nested_fn_names         |
/// | current_nested_fn_name        | current_nested_fn_name  |
/// | fn_scope_vars                 | fn_scope_vars           |
/// | typedarray_vars               | typedarray_vars         |
/// | regexp_vars                   | regexp_vars             |
pub struct FnContext {
    /// Current function name.
    pub name: String,
    /// Whether this is an export function.
    pub is_export: bool,
    /// Inferred return type (None = let Zig infer).
    pub return_type: Option<ZigType>,
    /// Whether a return/throw was seen in the body.
    pub seen_return: bool,
    /// Whether the body contains throw/try-catch.
    pub fn_has_throw: bool,
    /// Currently inside a return value expression.
    pub in_return_expr: bool,
    /// Currently at the top-level of an ExpressionStatement.
    pub in_expr_stmt: bool,
    /// Current call generated a catch block.
    pub call_generated_catch: bool,
    /// Inside a try block: label name for `break :label`.
    pub inside_try_block: Option<String>,
    /// Inside a class method: class name (for this.x → self.x).
    pub current_class: Option<String>,
    /// Nested function declaration names (rewrite to .call()).
    pub nested_fn_names: HashSet<String>,
    /// Currently generating a nested fn's body (override signature).
    pub current_nested_fn_name: Option<String>,
    /// Variable names in current function scope (shadow detection).
    pub fn_scope_vars: HashSet<String>,
    /// Variables holding TypedArray instances (name → element type suffix).
    pub typedarray_vars: HashMap<String, String>,
    /// Variables holding RegExp instances.
    pub regexp_vars: HashSet<String>,
    /// Identifiers referenced in expressions that were resolved at compile time
    /// (e.g., typeof x → "number" when x's type is known). These references
    /// must be tracked to avoid falsely marking parameters as unused.
    pub compile_time_referenced_idents: HashSet<String>,
}

impl FnContext {
    /// Create a new FnContext for a function with the given name.
    pub fn new(name: &str, is_export: bool, return_type: Option<ZigType>) -> Self {
        Self {
            name: name.to_string(),
            is_export,
            return_type,
            seen_return: false,
            fn_has_throw: false,
            in_return_expr: false,
            in_expr_stmt: false,
            call_generated_catch: false,
            inside_try_block: None,
            current_class: None,
            nested_fn_names: HashSet::new(),
            current_nested_fn_name: None,
            fn_scope_vars: HashSet::new(),
            typedarray_vars: HashMap::new(),
            regexp_vars: HashSet::new(),
            compile_time_referenced_idents: HashSet::new(),
        }
    }

    /// Register a variable in the current function scope.
    pub fn add_scope_var(&mut self, name: &str) {
        self.fn_scope_vars.insert(name.to_string());
    }

    /// Register a nested function name.
    pub fn add_nested_fn(&mut self, name: &str) {
        self.nested_fn_names.insert(name.to_string());
    }

    /// Check if a name is a nested function.
    pub fn is_nested_fn(&self, name: &str) -> bool {
        self.nested_fn_names.contains(name)
    }

    /// Register a TypedArray variable.
    pub fn add_typedarray_var(&mut self, name: &str, suffix: &str) {
        self.typedarray_vars
            .insert(name.to_string(), suffix.to_string());
    }

    /// Register a RegExp variable.
    pub fn add_regexp_var(&mut self, name: &str) {
        self.regexp_vars.insert(name.to_string());
    }
}

// ═══════════════════════════════════════════════════════
//  Lowering guard helpers
// ═══════════════════════════════════════════════════════

/// RAII guard for temporarily setting a FnContext flag.
///
/// Automatically restores the previous value on drop.
pub struct SetFlagGuard<'a> {
    field: &'a mut bool,
    old_value: bool,
}

impl<'a> SetFlagGuard<'a> {
    pub fn new(field: &'a mut bool, value: bool) -> Self {
        let old_value = *field;
        *field = value;
        Self { field, old_value }
    }
}

impl Drop for SetFlagGuard<'_> {
    fn drop(&mut self) {
        *self.field = self.old_value;
    }
}

// ═══════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fn_context_new() {
        let ctx = FnContext::new("add", true, Some(ZigType::I64));
        assert_eq!(ctx.name, "add");
        assert!(ctx.is_export);
        assert_eq!(ctx.return_type, Some(ZigType::I64));
        assert!(!ctx.seen_return);
        assert!(!ctx.fn_has_throw);
        assert!(ctx.nested_fn_names.is_empty());
    }

    #[test]
    fn test_fn_context_register_vars() {
        let mut ctx = FnContext::new("test", false, None);
        ctx.add_scope_var("x");
        ctx.add_scope_var("y");
        assert!(ctx.fn_scope_vars.contains("x"));
        assert!(ctx.fn_scope_vars.contains("y"));

        ctx.add_nested_fn("helper");
        assert!(ctx.is_nested_fn("helper"));
        assert!(!ctx.is_nested_fn("other"));

        ctx.add_typedarray_var("buf", "U8");
        assert_eq!(ctx.typedarray_vars.get("buf"), Some(&"U8".to_string()));

        ctx.add_regexp_var("re");
        assert!(ctx.regexp_vars.contains("re"));
    }

    #[test]
    fn test_set_flag_guard() {
        let mut ctx = FnContext::new("test", false, None);
        assert!(!ctx.in_return_expr);

        {
            let _guard = SetFlagGuard::new(&mut ctx.in_return_expr, true);
            // The flag is set while the guard is alive
            // (can't assert ctx.in_return_expr here due to borrow conflict,
            // but we verify the restored value after the guard drops)
        }

        assert!(!ctx.in_return_expr); // Restored on drop
    }
}
