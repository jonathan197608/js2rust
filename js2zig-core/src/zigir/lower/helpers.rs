// zigir/lower/helpers.rs
// FnContext sub-structure and lowering utility methods.

use std::collections::{HashMap, HashSet};

use crate::types::ZigType;

// ═══════════════════════════════════════════════════════
//  FnContext — per-function lowering state
// ═══════════════════════════════════════════════════════

/// Per-function context for the Lowerer.
///
/// Groups the 6+ flags and per-function sets:
///
/// | Field                         | Purpose                |
/// |-------------------------------|------------------------|
/// | name                          | current fn name        |
/// | is_export                     | C ABI export flag      |
/// | return_type                   | fn return type         |
/// | seen_return                   | has return expr        |
/// | fn_has_throw                  | contains throw/try     |
/// | in_return_expr                | inside return expr     |
/// | in_expr_stmt                  | inside expr statement  |
/// | call_generated_catch          | catch dispatch needed  |
/// | inside_try_block              | inside try block       |
/// | current_class                 | current class name     |
/// | nested_fn_names               | nested fn name set     |
/// | current_nested_fn_name        | current nested fn      |
/// | fn_scope_vars                 | fn-local var types     |
/// | typedarray_vars               | typedarray var set     |
/// | regexp_vars                   | regexp var set         |
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
    /// Whether the body contains BigInt division/modulo (can throw RangeError).
    pub has_bigint_div: bool,
    /// JS-const variables that are reassigned (emit runtime TypeError guard).
    /// In JS, `const x = 1; x = 2` throws TypeError at runtime, but Zig uses
    /// `var` for reassigned variables so the assignment succeeds. We track these
    /// to insert a throw before the actual assignment.
    pub js_const_reassigned: HashSet<String>,
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
    /// Per-function local variable types (takes priority over global var_types).
    /// Populated during lowering when VarDecl is processed.
    pub fn_local_types: HashMap<String, ZigType>,
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
            has_bigint_div: false,
            js_const_reassigned: HashSet::new(),
            in_return_expr: false,
            in_expr_stmt: false,
            call_generated_catch: false,
            inside_try_block: None,
            current_class: None,
            nested_fn_names: HashSet::new(),
            current_nested_fn_name: None,
            fn_scope_vars: HashSet::new(),
            fn_local_types: HashMap::new(),
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

// ═══════════════════════════════════════════════════════
//  Throw detection — unified AST walker
// ═══════════════════════════════════════════════════════

/// Strategy for walking `TryStatement` inside [`stmt_has_throw`].
#[derive(Clone, Copy)]
pub(crate) enum ThrowWalkMode {
    /// `TryStatement` counts as containing throw (conservative — any try
    /// implies potential throw). Used by `decl.rs::has_throw_in_body`.
    TryImpliesThrow,
    /// Recurse into the try-block only (catch/finally ignored). Used by
    /// `closure.rs::has_throw_in_stmt`.
    TryBlockOnly,
}

/// Check whether a statement contains a `throw`, recursing into
/// blocks / if / loops / switch according to `mode`.
pub(crate) fn stmt_has_throw(stmt: &oxc_ast::ast::Statement, mode: ThrowWalkMode) -> bool {
    use oxc_ast::ast::Statement;
    match stmt {
        Statement::ThrowStatement(_) => true,
        Statement::BlockStatement(bs) => bs.body.iter().any(|s| stmt_has_throw(s, mode)),
        Statement::IfStatement(is) => {
            stmt_has_throw(&is.consequent, mode)
                || is
                    .alternate
                    .as_ref()
                    .is_some_and(|a| stmt_has_throw(a, mode))
        }
        Statement::WhileStatement(ws) => stmt_has_throw(&ws.body, mode),
        Statement::DoWhileStatement(dws) => stmt_has_throw(&dws.body, mode),
        Statement::ForStatement(fs) => stmt_has_throw(&fs.body, mode),
        Statement::ForOfStatement(fos) => stmt_has_throw(&fos.body, mode),
        Statement::ForInStatement(fis) => stmt_has_throw(&fis.body, mode),
        Statement::LabeledStatement(ls) => stmt_has_throw(&ls.body, mode),
        Statement::SwitchStatement(ss) => ss
            .cases
            .iter()
            .any(|c| c.consequent.iter().any(|s| stmt_has_throw(s, mode))),
        Statement::TryStatement(ts) => match mode {
            ThrowWalkMode::TryImpliesThrow => true,
            ThrowWalkMode::TryBlockOnly => ts.block.body.iter().any(|s| stmt_has_throw(s, mode)),
        },
        _ => false,
    }
}

/// Determine the Zig format specifier for a given type.
/// Used by string concatenation and template literal lowering.
/// Note: `{any}` is used as the catch-all because Zig 0.16 does not
/// allow `{}` for slice types (`[]const u8`); `{any}` works for all types.
pub(crate) fn format_specifier_for_type(ty: &ZigType) -> &'static str {
    match ty {
        ZigType::Str => "{s}",
        ZigType::I64 => "{d}",
        ZigType::F64 => "{d:.15}",
        ZigType::Bool => "{}",
        _ => "{any}",
    }
}
