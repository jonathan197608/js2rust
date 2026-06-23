// native_proto/codegen/mod.rs
// Core Codegen struct, constructor, and entry point.

use crate::native_proto::Codegen;
use oxc_ast::ast::*;

pub mod expr;
pub mod helpers;
pub mod stmt;

// ── Constructor ─────────────────────────────────────

impl Codegen {
    pub fn new(
        type_info: crate::native_proto::TypeCheckResult,
        jsdoc_data: crate::native_proto::JSDocData,
        exported_functions: Option<std::collections::HashSet<String>>,
    ) -> Self {
        Self {
            output: String::new(),
            indent: 0,
            errors: Vec::new(),
            warnings: Vec::new(),
            type_info,
            jsdoc_data: Some(jsdoc_data),
            current_fn_is_export: false,
            current_fn_return_type: None,
            exported_fns: Vec::new(),
            cabi_exports: Vec::new(),
            task_counter: 0,
            exported_functions,
            seen_return: false,
            fn_has_throw: false,
            in_return_expr: false,
            try_label_counter: 0,
            arrow_counter: 0,
            pending_arrow_fns: Vec::new(),
            inside_try_block: None,
            current_fn: None,
            current_captured: Vec::new(),
            closure_vars: std::collections::HashMap::new(),
            closure_instances: std::collections::HashSet::new(),
            closure_defs: Vec::new(),
            oc_counter: 0,
        }
    }
}

// ── Entry point ─────────────────────────────────────

impl Codegen {
    /// Emit all @typedef struct definitions at the top of the generated file.
    fn emit_typedefs(&mut self) {
        // Clone typedefs to avoid borrow checker issues
        let typedefs = match &self.jsdoc_data {
            Some(data) => data.typedefs.clone(),
            None => return,
        };
        if typedefs.is_empty() {
            return;
        }
        for (name, td) in &typedefs {
            self.writeln(&format!("const {} = struct {{", name));
            self.indent += 1;
            for field in &td.fields {
                let zig_ty = crate::native_proto::jsdoc::jsdoc_type_to_zig(&field.ty, &typedefs);
                // Optional field: prepend ? to the type
                let zig_ty = if field.optional {
                    format!("?{}", zig_ty)
                } else {
                    zig_ty
                };
                self.writeln(&format!("{}: {},", field.name, zig_ty));
            }
            // Generate toJson() method for serialization using std.json.fmt()
            // Use the global arena allocator (js_allocator.getAllocator()).
            self.writeln("");
            self.writeln("pub fn toJson(self: *const @This()) ![]u8 {");
            self.indent += 1;
            // Use std.io.Writer.Allocating + std.json.fmt() for serialization
            self.writeln(
                "var string = std.io.Writer.Allocating.init(js_allocator.getAllocator());",
            );
            self.writeln("errdefer string.deinit();");
            self.writeln("try string.writer().print(\"{f}\", .{std.json.fmt(self.*, .{})});");
            self.writeln("return string.toOwnedSlice();");
            self.indent -= 1;
            self.writeln("}");
            self.indent -= 1;
            self.writeln("};");
            self.writeln("");
        }
    }

    pub fn generate(&mut self, program: &Program) {
        // Phase A: analyze_objects, collect_used_names, walk_stmt_for_types
        // are all handled by TypeInferrer::infer_all() before codegen starts.

        // Emit struct typedefs (from JSDoc @typedef).
        self.emit_typedefs();

        // Emit code, skipping unused toplevel constants.
        for stmt in &program.body {
            self.emit_toplevel(stmt);
        }

        // After generating all statements, prepend closure struct definitions
        // so they appear at module level (before all functions).
        if !self.closure_defs.is_empty() {
            let mut prefix = String::new();
            for def in self.closure_defs.iter() {
                prefix.push_str(def);
                prefix.push('\n');
            }
            self.output = prefix + &self.output;
        }
    }

    fn emit_toplevel(&mut self, stmt: &Statement) {
        match stmt {
            Statement::VariableDeclaration(vd) => self.emit_var_decl(vd),
            Statement::FunctionDeclaration(fd) => {
                // Determine if this function is an export function.
                // Priority:
                // 1. If `exported_functions` is provided (from pipeline), use it.
                // 2. Otherwise, check if the function is inside `export {...}` (not supported yet).
                // 3. Default: non-export (pub fn, not C ABI).
                let fn_name = fd.id.as_ref().map(|id| id.name.as_str());
                let is_export = if let Some(ref exported) = self.exported_functions {
                    // Use exported_functions set from pipeline
                    fn_name.is_some_and(|name| exported.contains(name))
                } else {
                    // No export info: default to non-export.
                    // NOTE: `function foo() {}` (without `export`) is non-export.
                    false
                };

                let old_export = self.current_fn_is_export;
                self.current_fn_is_export = is_export;
                self.emit_fn(fd);
                self.current_fn_is_export = old_export;
            }
            Statement::ExportNamedDeclaration(export_decl) => {
                // `export function foo() {}` or `export const foo = ...`
                // These are ALWAYS export functions.
                match &export_decl.declaration {
                    Some(decl) => {
                        match decl {
                            Declaration::FunctionDeclaration(fd) => {
                                // is_export determined by exported_functions set,
                                // NOT always-true — dependency files only
                                // export names that the core file re-exports.
                                let fn_name = fd.id.as_ref().map(|id| id.name.as_str());
                                let is_export = self
                                    .exported_functions
                                    .as_ref()
                                    .is_some_and(|ex| fn_name.is_some_and(|n| ex.contains(n)));
                                let old_export = self.current_fn_is_export;
                                self.current_fn_is_export = is_export;
                                self.emit_fn(fd);
                                self.current_fn_is_export = old_export;
                            }
                            Declaration::VariableDeclaration(vd) => {
                                self.emit_var_decl(vd);
                            }
                            _ => { /* skip unsupported */ }
                        }
                    }
                    None => { /* skip (e.g., export {{ ... }} */ }
                }
            }
            _ => { /* skip */ }
        }
    }
}
