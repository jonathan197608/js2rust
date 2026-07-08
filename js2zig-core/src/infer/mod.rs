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
// Phase A: All type inference runs BEFORE lowering/emission.
// TypeInferrer walks the full AST once and produces a TypeCheckResult
// that the Lowerer/Emitter reads (purely generative after that).

use oxc_ast::ast::*;
use std::collections::{HashMap, HashSet};

use crate::types::JSDocData;
use crate::types::ZigType;

pub mod expr;
pub mod fn_types;
pub mod helpers;
pub mod passes;

// Re-export public utilities for backward compatibility
// (the Lowerer uses crate::infer::binding_name)
pub use helpers::binding_name;

/// Result of type inference: either a definite type or indeterminate.
#[derive(Debug, Clone, PartialEq)]
pub enum InferResult {
    /// Definite type
    Definite(ZigType),
    /// Indeterminate (cannot infer from context)
    Indeterminate,
}

// ── TypeInferResult: read-only snapshot passed to the Lowerer ──

/// Complete type-checking result computed by TypeInferrer.
/// The Lowerer reads from this during lowering — no writes.
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
    /// Variable names that are directly reassigned (`x = ...`, not `x.y = ...`).
    /// Used to distinguish "variable reassignment" from "property mutation".
    pub reassigned_vars: HashSet<String>,
    /// Variable names that hold a Set (for MapKeys→SetKeys override)
    pub set_vars: HashSet<String>,
    /// Identifier names referenced anywhere (for unused-constant elimination)
    pub used_names: HashSet<String>,
    /// Variable names initialized with JSON.parse(@type)
    pub has_json_parse_types: HashSet<String>,
    /// Type-check errors (Rule 8 violations, etc.)
    pub errors: Vec<String>,
    /// Whether each function is async (needs io: anytype)
    pub is_async: HashMap<String, bool>,
    /// Class field types: class_name → (field_name → ZigType)
    /// Collected from PropertyDefinition initializers by the TypeInferrer.
    pub class_field_types: HashMap<String, HashMap<String, ZigType>>,
    /// Host function return types: fn_name → ZigType (full name, e.g. "host_add")
    pub host_return_types: HashMap<String, ZigType>,
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
    /// Set of directly reassigned variables (variable = ..., not variable.field = ...)
    pub(crate) reassigned_vars: HashSet<String>,
    /// Set variable names (for MapKeys→SetKeys override)
    pub(crate) set_vars: HashSet<String>,
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
    /// Host function return types: fn_name → ZigType
    pub(crate) host_return_types: HashMap<String, ZigType>,
    /// Host struct field types: struct_name → (field_name → ZigType)
    pub(crate) host_struct_fields: HashMap<String, HashMap<String, ZigType>>,
    /// Current function name being analyzed (for function-scoped mutated_vars)
    pub(crate) current_fn: Option<String>,
    /// Class names known at the module level.
    /// Used to infer return type of `new ClassName()` → NamedStruct.
    pub(crate) class_names: HashSet<String>,
    /// Class field types: class_name → (field_name → ZigType)
    /// Collected from PropertyDefinition initializers.
    pub(crate) class_field_types: HashMap<String, HashMap<String, ZigType>>,
    /// When inside a class method body, this holds the class name.
    /// Used to resolve `this.field` → field type for return-type inference.
    pub(crate) current_class: Option<String>,
}

impl TypeInferrer {
    pub fn new() -> Self {
        Self {
            var_types: HashMap::new(),
            array_element_types: HashMap::new(),
            fn_return_types: HashMap::new(),
            fn_param_types: HashMap::new(),
            mutated_vars: HashSet::new(),
            reassigned_vars: HashSet::new(),
            set_vars: HashSet::new(),
            used_names: HashSet::new(),
            has_json_parse_types: HashSet::new(),
            is_async: HashMap::new(),
            errors: Vec::new(),
            jsdoc_data: None,
            exported_functions: None,
            host_return_types: HashMap::new(),
            host_struct_fields: HashMap::new(),
            current_fn: None,
            class_names: HashSet::new(),
            class_field_types: HashMap::new(),
            current_class: None,
        }
    }

    /// Set JSDoc data for type annotations
    pub fn set_jsdoc_data(&mut self, data: JSDocData) {
        self.jsdoc_data = Some(data);
    }

    /// Pre-populate host function return types and struct field info.
    /// Called from pipeline after host_fns are registered.
    pub fn set_host_fn_types(&mut self, host_fns: &crate::host::HostFnRegistry) {
        for def in host_fns.iter() {
            self.host_return_types
                .insert(def.name.clone(), def.ret_type.clone());
            // Populate struct field types for async return structs
            if let ZigType::NamedStruct(ref struct_name) = def.ret_type
                && let Some(fields) = host_fns.struct_fields_map().get(struct_name)
            {
                let field_map: HashMap<String, ZigType> =
                    fields.iter().map(|(n, t)| (n.clone(), t.clone())).collect();
                self.host_struct_fields
                    .insert(struct_name.clone(), field_map);
            }
        }
    }

    // ============================================================
    // Main entry: run all passes
    // ============================================================

    /// Run all type-inference passes on a program and return the result.
    /// After this, the Lowerer can lower without doing any inference.
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
            reassigned_vars: std::mem::take(&mut self.reassigned_vars),
            set_vars: std::mem::take(&mut self.set_vars),
            used_names: std::mem::take(&mut self.used_names),
            has_json_parse_types: std::mem::take(&mut self.has_json_parse_types),
            errors: std::mem::take(&mut self.errors),
            is_async: std::mem::take(&mut self.is_async),
            class_field_types: std::mem::take(&mut self.class_field_types),
            host_return_types: std::mem::take(&mut self.host_return_types),
        }
    }
}
