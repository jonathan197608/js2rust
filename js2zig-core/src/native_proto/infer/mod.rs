// native_proto/infer/mod.rs
// Type inference for native_proto mode.
// Follows the 8-rule simplification plan:
// 1. Literal expressions → definite type (use JSDoc if available)
// 2. Binary expressions → definite only if BOTH operands are literals
// 3. Other expressions → indeterminate (None)
// 4. const → no type annotation, let Zig infer
// 5. Local variables → check ALL assignments, at least one definite
// 6. Return types → check ALL return expressions, at least one definite
// 7. Non-export function params → indeterminate → anytype
// 8. Indeterminate → report compile error
//
// Phase A: All type inference runs BEFORE codegen.
// TypeInferrer walks the full AST once and produces a TypeCheckResult
// that Codegen reads (purely generative after that).

use oxc_ast::ast::*;
use std::collections::{HashMap, HashSet};

use crate::native_proto::JSDocData;
use crate::native_proto::ZigType;

pub mod expr;
pub mod passes;
pub mod fn_types;
pub mod helpers;

// Re-export public utilities for backward compatibility
// (codegen uses crate::native_proto::infer::binding_name)
pub use helpers::binding_name;

/// Result of type inference: either a definite type or indeterminate.
#[derive(Debug, Clone, PartialEq)]
pub enum InferResult {
    /// Definite type
    Definite(ZigType),
    /// Indeterminate (cannot infer from context)
    Indeterminate,
}

// ── TypeInferResult: read-only snapshot passed to Codegen ──

/// Complete type-checking result computed by TypeInferrer.
/// Codegen reads from this during the code-generation pass — no writes.
#[derive(Debug, Clone)]
pub struct TypeCheckResult {
    /// Variable → inferred type (toplevel + function-local, keyed by name only)
    pub var_types: HashMap<String, ZigType>,
    /// Array variable → element type
    pub array_element_types: HashMap<String, ZigType>,
    /// Function name → return type
    pub fn_return_types: HashMap<String, ZigType>,
    /// Function name → [(param_name, param_type)]
    pub fn_param_types: HashMap<String, Vec<(String, ZigType)>>,
    /// Variable names that must use `var` (member-assignment target)
    pub mutated_vars: HashSet<String>,
    /// Identifier names referenced anywhere (for unused-constant elimination)
    pub used_names: HashSet<String>,
    /// Variable names initialized with JSON.parse(@type)
    pub has_json_parse_types: HashSet<String>,
    /// Type-check errors (Rule 8 violations, etc.)
    pub errors: Vec<String>,
    /// Whether each function is async (needs io: anytype)
    pub is_async: HashMap<String, bool>,
}

// ── TypeInferrer ────────────────────────────────────

/// Simplified type inferrer for native_proto mode.
pub struct TypeInferrer {
    /// Variable types inferred from initializers
    pub(crate) var_types: HashMap<String, ZigType>,
    /// Array element types (for ArrayList push type checking)
    pub(crate) array_element_types: HashMap<String, ZigType>,
    /// Function name → return type
    pub(crate) fn_return_types: HashMap<String, ZigType>,
    /// Function name → [(param_name, param_type)]
    pub(crate) fn_param_types: HashMap<String, Vec<(String, ZigType)>>,
    /// Set of mutated variables (need `var` instead of `const`)
    pub(crate) mutated_vars: HashSet<String>,
    /// Identifier names referenced anywhere
    pub(crate) used_names: HashSet<String>,
    /// Variable names initialized with JSON.parse(@type)
    pub(crate) has_json_parse_types: HashSet<String>,
    /// Whether each function is async
    pub(crate) is_async: HashMap<String, bool>,
    /// Collected errors (reported during type checking)
    pub errors: Vec<String>,
    /// JSDoc data for type annotations
    pub(crate) jsdoc_data: Option<JSDocData>,
    /// Exported function names (from pipeline)
    pub(crate) exported_functions: Option<HashSet<String>>,
}

impl TypeInferrer {
    pub fn new() -> Self {
        Self {
            var_types: HashMap::new(),
            array_element_types: HashMap::new(),
            fn_return_types: HashMap::new(),
            fn_param_types: HashMap::new(),
            mutated_vars: HashSet::new(),
            used_names: HashSet::new(),
            has_json_parse_types: HashSet::new(),
            is_async: HashMap::new(),
            errors: Vec::new(),
            jsdoc_data: None,
            exported_functions: None,
        }
    }

    /// Set JSDoc data for type annotations
    pub fn set_jsdoc_data(&mut self, data: JSDocData) {
        self.jsdoc_data = Some(data);
    }

    // ============================================================
    // Main entry: run all passes
    // ============================================================

    /// Run all type-inference passes on a program and return the result.
    /// After this, Codegen can generate code without doing any inference.
    pub fn infer_all(
        &mut self,
        program: &Program,
        exported_functions: Option<HashSet<String>>,
    ) -> TypeCheckResult {
        self.exported_functions = exported_functions;

        // Pass 0: Analyze objects — detect mutations and dynamic access errors.
        self.analyze_objects(program);

        // Pass 1: Collect referenced names (for unused-constant elimination).
        self.collect_used_names(program);

        // Pass 2: Walk ALL scopes (top-level + function bodies) to collect types.
        self.walk_toplevel_for_types(program);

        // Produce snapshot.
        TypeCheckResult {
            var_types: std::mem::take(&mut self.var_types),
            array_element_types: std::mem::take(&mut self.array_element_types),
            fn_return_types: std::mem::take(&mut self.fn_return_types),
            fn_param_types: std::mem::take(&mut self.fn_param_types),
            mutated_vars: std::mem::take(&mut self.mutated_vars),
            used_names: std::mem::take(&mut self.used_names),
            has_json_parse_types: std::mem::take(&mut self.has_json_parse_types),
            errors: std::mem::take(&mut self.errors),
            is_async: std::mem::take(&mut self.is_async),
        }
    }
}
