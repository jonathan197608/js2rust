use std::collections::{HashMap, HashSet};

use oxc_ast::ast::*;

use crate::builtins::BuiltinRegistry;
use crate::infer::{TypeInferrer, ZigType};

/// Information about a closure generated from an inline arrow function
#[derive(Debug, Clone)]
struct ClosureInfo {
    /// Struct name, e.g. "_Closure_makeAdder"
    struct_name: String,
    /// Captured (free) variable names and their Zig types
    captured: Vec<(String, String)>,
    /// Arrow function parameter names and types
    params: Vec<(String, String)>,
    /// Return type as Zig string
    return_type: String,
    /// Pre-generated struct definition string (filled during record_closure)
    struct_def: String,
}

/// Generate a unique closure struct name from a function name
fn closure_name(fn_name: &str) -> String {
    format!("_Closure_{}", fn_name)
}

/// A single leaf binding extracted from a destructuring pattern.
/// For `const { a: { b, c } } = obj`, we get two leaf bindings:
///   LeafBinding { name: "b", access: "_tmp.a.b", default: None }
///   LeafBinding { name: "c", access: "_tmp.a.c", default: None }
struct LeafBinding<'a> {
    name: &'a str,
    /// Full access path from the temp variable (e.g. "_tmp.a.b" or "_tmp[0].x")
    access: String,
    #[allow(dead_code)]
    default_value: Option<&'a Expression<'a>>,
}

/// Recursively flatten a BindingPattern into a list of leaf bindings.
/// `prefix` is the access path from the temp variable (e.g. "_tmp").
fn flatten_binding_pattern<'a>(
    pattern: &'a BindingPattern<'a>,
    prefix: &str,
    result: &mut Vec<LeafBinding<'a>>,
) {
    match pattern {
        BindingPattern::BindingIdentifier(id) => {
            result.push(LeafBinding {
                name: id.name.as_str(),
                access: prefix.to_string(),
                default_value: None,
            });
        }
        BindingPattern::ObjectPattern(obj) => {
            for prop in &obj.properties {
                let key_str = property_key_name(&prop.key);
                let new_prefix = format!("{}.{}", prefix, key_str);
                flatten_binding_pattern(&prop.value, &new_prefix, result);
            }
            // rest: ...rest — skip for now (requires runtime support)
        }
        BindingPattern::ArrayPattern(arr) => {
            for (i, elem) in arr.elements.iter().enumerate().filter_map(|(i, opt)| opt.as_ref().map(|e| (i, e))) {
                let new_prefix = format!("{}[{}]", prefix, i);
                flatten_binding_pattern(elem, &new_prefix, result);
            }
            // rest: ...rest — skip for now
        }
        BindingPattern::AssignmentPattern(assign) => {
            // Default value: for simplicity, skip the default in the first pass
            // The leaf binding gets the access path; default is not emitted.
            flatten_binding_pattern(&assign.left, prefix, result);
        }
    }
}

/// Extract a string representation of a PropertyKey for use in access paths.
/// E.g., `{ a: x }` → "a"
fn property_key_name(key: &PropertyKey) -> String {
    match key {
        PropertyKey::StaticIdentifier(id) => id.name.to_string(),
        _ => "_computed_key".to_string(),
    }
}

/// Collect all leaf identifier names from a BindingPattern.
/// Used by preprocess and infer to register variable names without access paths.
pub fn collect_binding_names(pattern: &BindingPattern, names: &mut Vec<String>) {
    match pattern {
        BindingPattern::BindingIdentifier(id) => {
            names.push(id.name.to_string());
        }
        BindingPattern::ObjectPattern(obj) => {
            for prop in &obj.properties {
                collect_binding_names(&prop.value, names);
            }
            if let Some(rest) = &obj.rest {
                collect_binding_names(&rest.argument, names);
            }
        }
        BindingPattern::ArrayPattern(arr) => {
            for elem in arr.elements.iter().flatten() {
                collect_binding_names(elem, names);
            }
            if let Some(rest) = &arr.rest {
                collect_binding_names(&rest.argument, names);
            }
        }
        BindingPattern::AssignmentPattern(assign) => {
            collect_binding_names(&assign.left, names);
        }
    }
}

/// Collect all leaf identifier names and their spans from a BindingPattern.
/// Used by preprocess for rename edits.
pub fn collect_binding_names_with_spans(
    pattern: &BindingPattern,
    names: &mut Vec<(String, oxc_span::Span)>,
) {
    match pattern {
        BindingPattern::BindingIdentifier(id) => {
            names.push((id.name.to_string(), id.span));
        }
        BindingPattern::ObjectPattern(obj) => {
            for prop in &obj.properties {
                collect_binding_names_with_spans(&prop.value, names);
            }
            if let Some(rest) = &obj.rest {
                collect_binding_names_with_spans(&rest.argument, names);
            }
        }
        BindingPattern::ArrayPattern(arr) => {
            for elem in arr.elements.iter().flatten() {
                collect_binding_names_with_spans(elem, names);
            }
            if let Some(rest) = &arr.rest {
                collect_binding_names_with_spans(&rest.argument, names);
            }
        }
        BindingPattern::AssignmentPattern(assign) => {
            collect_binding_names_with_spans(&assign.left, names);
        }
    }
}

/// Check whether a BindingPattern is just a simple identifier (no destructuring).
pub fn is_simple_binding(pattern: &BindingPattern) -> bool {
    matches!(pattern, BindingPattern::BindingIdentifier(_))
}

/// Return type for codegen::generate: (zig_code, diagnostics, closure_fns, fn_return_types, cabi_exports)
pub type CodegenResult = (
    String,
    Vec<crate::infer::Diagnostic>,
    std::collections::HashSet<String>,
    std::collections::HashMap<String, ZigType>,
    Vec<CabiExport>,
);

pub fn generate(
    program: &Program,
    builtins: &BuiltinRegistry,
    exports: &HashSet<String>,
) -> CodegenResult {
    let mut inferrer = TypeInferrer::new();
    inferrer.infer_program(program);

    // Extract fn_return_types before inferrer is consumed by ZigCodegen
    let fn_return_types = inferrer.all_fn_return_types();

    let mut diagnostics = inferrer.diagnostics().to_vec();

    let mut cg = ZigCodegen::new(inferrer, &mut diagnostics, builtins, exports.clone());
    // Header is added by project.rs/generate_lib_zig() — do NOT emit here.

    // Pre-scan: find functions that return inline arrow functions
    cg.pre_scan_closures(program);

    // Push all pre-generated closure struct defs to the output buffer
    for def in cg.closure_struct_defs.values() {
        cg.closure_structs.push(def.clone());
    }

    // Top-level: only VariableDeclaration and FunctionDeclaration
    // Rejected statements produce Error diagnostics inside emit_stmt()
    cg.in_top_level = true;
    for stmt in &program.body {
        cg.emit_stmt(stmt);
    }

    // Emit buffered closure struct definitions
    for def in &cg.closure_structs {
        cg.output.push_str(def);
    }

    // Emit buffered object type struct definitions
    for def in &cg.object_type_defs {
        cg.output.push_str(def);
    }

    // Emit C ABI export wrappers (after all functions, before tests)
    for wrapper in &cg.cabi_wrappers {
        cg.output.push_str(wrapper);
    }

    // Emit init_js2rust() function if there are dynamic access variables
    cg.emit_init_js2rust();
    // Emit deinit_js2rust() function for cleanup
    cg.emit_deinit_js2rust();

    let closure_fns: HashSet<String> = cg.fn_closure_spans.keys().cloned().collect();

    let cabi_exports = cg.cabi_exports;

    (cg.output, diagnostics, closure_fns, fn_return_types, cabi_exports)
}

/// Metadata for a C ABI exported function, used by sys crate to generate Rust FFI bindings.
#[derive(Debug, Clone)]
pub struct CabiExport {
    /// C ABI exported function name (e.g. "chineseAdd")
    pub name: String,
    /// (param_name, ZigType) pairs
    pub params: Vec<(String, ZigType)>,
    /// Return type (ZigType)
    pub ret_type: ZigType,
    /// Whether a corresponding free_xxx function exists
    pub has_free_func: bool,
}

struct ZigCodegen<'a> {
    output: String,
    indent: usize,
    inferrer: TypeInferrer,
    diagnostics: &'a mut Vec<crate::infer::Diagnostic>,
    in_top_level: bool,
    task_counter: usize,
    builtins: &'a BuiltinRegistry,
    /// arrow.span.start → ClosureInfo for ALL detected inline arrow functions
    closure_map: HashMap<u32, ClosureInfo>,
    /// arrow.span.start → pre-generated struct definition string
    closure_struct_defs: HashMap<u32, String>,
    /// fn_name → arrow.span.start (for functions that return a closure)
    fn_closure_spans: HashMap<String, u32>,
    /// Counter for generating unique synthetic closure names
    closure_counter: usize,
    /// Buffered closure struct definitions to emit after all functions
    closure_structs: Vec<String>,
    /// Buffered C ABI export wrappers to emit after all functions
    cabi_wrappers: Vec<String>,
    /// C ABI export metadata for sys crate FFI bindings
    cabi_exports: Vec<CabiExport>,
    /// Set of function names that need free_xxx functions (return strings)
    string_return_fns: HashSet<String>,
    /// Set of variable names that hold closure struct instances
    closure_vars: HashSet<String>,
    /// Current function name for closure lookup
    current_fn: Option<String>,
    /// Set of exported function/var names (from preprocess)
    exports: HashSet<String>,
    /// Current try-block label (for translating `throw` to `break :label error.X`)
    try_label: Option<String>,
    /// Current catch-block label (for translating `return` in catch to `break :label expr`)
    catch_label: Option<String>,
    /// Counter for generating unique try-block labels
    try_counter: usize,
    /// Counter for generating unique temp variable names (destructuring)
    temp_counter: usize,
    /// Pending body prelude statements for destructured function parameters
    /// Cleared after each function body is emitted.
    destructure_prelude: Vec<String>,
    /// Current class context: (struct_name, field_names) for `self` tracking
    current_class: Option<(String, Vec<String>)>,
    /// Buffered struct type definitions for top-level object literals
    object_type_defs: Vec<String>,
    /// Per-function: struct name for each Object-typed parameter (indexed by param position).
    /// Populated at the start of emit_fn_decl, consumed by emit_params, cleared after the function.
    current_obj_structs: Vec<Option<String>>,
    /// Buffered initialization code for init_globals() function.
    /// Each string is a line of code to be inserted into init_globals().
    init_globals_code: Vec<String>,
}

impl<'a> ZigCodegen<'a> {
    fn new(
        inferrer: TypeInferrer,
        diagnostics: &'a mut Vec<crate::infer::Diagnostic>,
        builtins: &'a BuiltinRegistry,
        exports: HashSet<String>,
    ) -> Self {
        Self {
            output: String::new(),
            indent: 0,
            inferrer,
            diagnostics,
            in_top_level: false,
            task_counter: 0,
            builtins,
            closure_map: HashMap::new(),
            closure_struct_defs: HashMap::new(),
            fn_closure_spans: HashMap::new(),
            closure_counter: 0,
            closure_structs: Vec::new(),
            cabi_wrappers: Vec::new(),
            cabi_exports: Vec::new(),
            string_return_fns: HashSet::new(),
            closure_vars: HashSet::new(),
            current_fn: None,
            exports,
            try_label: None,
            catch_label: None,
            try_counter: 0,
            temp_counter: 0,
            destructure_prelude: Vec::new(),
            current_class: None,
            object_type_defs: Vec::new(),
            current_obj_structs: Vec::new(),
            init_globals_code: Vec::new(),
        }
    }
    
    // ========== Closure pre-scan ==========

    /// Pre-scan the AST to find ALL ArrowFunctionExpressions and record closure info.
    /// Covers:
    ///   - return (x) => x + n           (return closure → key: fn_name)
    ///   - const fn = (x) => x + 1       (var assignment → key: var_name)
    ///   - arr.map((x) => x * 2)         (callback → synthetic key)
    fn pre_scan_closures(&mut self, program: &Program) {
        for stmt in &program.body {
            if let Statement::FunctionDeclaration(fd) = stmt {
                let fn_name = fd.id.as_ref().map(|id| id.name.as_str()).unwrap_or("");
                if fn_name.is_empty() {
                    continue;
                }
                if let Some(body) = &fd.body {
                    self.scan_fn_body_for_closures(fn_name, &fd.params, body);
                }
            }
        }
    }

    /// Scan a function body for ArrowFunctionExpressions in ALL positions
    fn scan_fn_body_for_closures(
        &mut self,
        fn_name: &str,
        fn_params: &FormalParameters,
        body: &FunctionBody,
    ) {
        for stmt in &body.statements {
            match stmt {
                Statement::ReturnStatement(rs) => {
                    if let Some(arg) = &rs.argument {
                        self.scan_expr_for_closures(fn_name, fn_params, arg, true);
                    }
                }
                Statement::VariableDeclaration(vd) => {
                    for decl in &vd.declarations {
                        if let Some(init) = &decl.init {
                            let var_name = self.binding_name(&decl.id);
                            self.scan_expr_for_closures(var_name, fn_params, init, false);
                        }
                    }
                }
                Statement::ExpressionStatement(es) => {
                    self.scan_expr_for_closures(fn_name, fn_params, &es.expression, false);
                }
                // Recursively scan nested statements for nested closures
                _ => self.scan_stmt_for_closures(fn_name, fn_params, stmt),
            }
        }
    }

    /// Recursively scan an expression tree for ArrowFunctionExpressions
    fn scan_expr_for_closures(
        &mut self,
        context_name: &str,
        fn_params: &FormalParameters,
        expr: &Expression,
        is_return_closure: bool,
    ) {
        match expr {
            Expression::ArrowFunctionExpression(arrow) => {
                self.record_closure(context_name, fn_params, arrow, is_return_closure);
                // Also scan the arrow body for nested closures
                let ctx_name = format!("{}_inner", context_name);
                for s in &arrow.body.statements {
                    self.scan_stmt_for_closures(&ctx_name, fn_params, s);
                }
            }
            Expression::CallExpression(call) => {
                self.scan_expr_for_closures(context_name, fn_params, &call.callee, false);
                for arg in &call.arguments {
                    if let Some(expr) = arg.as_expression() {
                        self.scan_expr_for_closures(context_name, fn_params, expr, false);
                    }
                }
            }
            Expression::BinaryExpression(bin) => {
                self.scan_expr_for_closures(context_name, fn_params, &bin.left, false);
                self.scan_expr_for_closures(context_name, fn_params, &bin.right, false);
            }
            Expression::UnaryExpression(un) => {
                self.scan_expr_for_closures(context_name, fn_params, &un.argument, false);
            }
            Expression::ConditionalExpression(cond) => {
                self.scan_expr_for_closures(context_name, fn_params, &cond.test, false);
                self.scan_expr_for_closures(context_name, fn_params, &cond.consequent, false);
                self.scan_expr_for_closures(context_name, fn_params, &cond.alternate, false);
            }
            Expression::AssignmentExpression(ass) => {
                self.scan_expr_for_closures(context_name, fn_params, &ass.right, false);
            }
            Expression::ArrayExpression(arr) => {
                for elem in &arr.elements {
                    if let Some(e) = elem.as_expression() {
                        self.scan_expr_for_closures(context_name, fn_params, e, false);
                    }
                }
            }
            Expression::NewExpression(new) => {
                for arg in &new.arguments {
                    if let Some(expr) = arg.as_expression() {
                        self.scan_expr_for_closures(context_name, fn_params, expr, false);
                    }
                }
            }
            _ => {}
        }
    }

    /// Recursively scan a statement for nested ArrowFunctionExpressions
    fn scan_stmt_for_closures(
        &mut self,
        context_name: &str,
        fn_params: &FormalParameters,
        stmt: &Statement,
    ) {
        match stmt {
            Statement::IfStatement(if_stmt) => {
                self.scan_expr_for_closures(context_name, fn_params, &if_stmt.test, false);
                self.scan_stmt_for_closures(context_name, fn_params, &if_stmt.consequent);
                if let Some(alt) = &if_stmt.alternate {
                    self.scan_stmt_for_closures(context_name, fn_params, alt);
                }
            }
            Statement::ForStatement(fs) => {
                if let Some(init) = &fs.init {
                    match init {
                        ForStatementInit::VariableDeclaration(vd) => {
                            for decl in &vd.declarations {
                                if let Some(init_expr) = &decl.init {
                                    self.scan_expr_for_closures(context_name, fn_params, init_expr, false);
                                }
                            }
                        }
                        _ => {
                            if let Some(e) = init.as_expression() {
                                self.scan_expr_for_closures(context_name, fn_params, e, false);
                            }
                        }
                    }
                }
                if let Some(test) = &fs.test {
                    self.scan_expr_for_closures(context_name, fn_params, test, false);
                }
                if let Some(update) = &fs.update {
                    self.scan_expr_for_closures(context_name, fn_params, update, false);
                }
                self.scan_stmt_for_closures(context_name, fn_params, &fs.body);
            }
            Statement::WhileStatement(ws) => {
                self.scan_expr_for_closures(context_name, fn_params, &ws.test, false);
                self.scan_stmt_for_closures(context_name, fn_params, &ws.body);
            }
            Statement::DoWhileStatement(dw) => {
                self.scan_stmt_for_closures(context_name, fn_params, &dw.body);
                self.scan_expr_for_closures(context_name, fn_params, &dw.test, false);
            }
            Statement::BlockStatement(block) => {
                for s in &block.body {
                    self.scan_stmt_for_closures(context_name, fn_params, s);
                }
            }
            Statement::ReturnStatement(rs) => {
                if let Some(arg) = &rs.argument {
                    self.scan_expr_for_closures(context_name, fn_params, arg, false);
                }
            }
            Statement::VariableDeclaration(vd) => {
                for decl in &vd.declarations {
                    if let Some(init) = &decl.init {
                        let var_name = self.binding_name(&decl.id);
                        self.scan_expr_for_closures(var_name, fn_params, init, false);
                    }
                }
            }
            Statement::ExpressionStatement(es) => {
                self.scan_expr_for_closures(context_name, fn_params, &es.expression, false);
            }
            Statement::SwitchStatement(ss) => {
                self.scan_expr_for_closures(context_name, fn_params, &ss.discriminant, false);
                for case in &ss.cases {
                    for stmt in &case.consequent {
                        self.scan_stmt_for_closures(context_name, fn_params, stmt);
                    }
                }
            }
            Statement::TryStatement(ts) => {
                for stmt in &ts.block.body {
                    self.scan_stmt_for_closures(context_name, fn_params, stmt);
                }
                if let Some(handler) = &ts.handler {
                    for stmt in &handler.body.body {
                        self.scan_stmt_for_closures(context_name, fn_params, stmt);
                    }
                }
            }
            Statement::ForInStatement(fis) => {
                self.scan_expr_for_closures(context_name, fn_params, &fis.right, false);
                self.scan_stmt_for_closures(context_name, fn_params, &fis.body);
            }
            Statement::ForOfStatement(fos) => {
                self.scan_expr_for_closures(context_name, fn_params, &fos.right, false);
                self.scan_stmt_for_closures(context_name, fn_params, &fos.body);
            }
            _ => {}
        }
    }

    /// Record closure info for an arrow function found at any position.
    /// `fn_name` is the enclosing function name (for fn_closure_spans lookup).
    /// `struct_context` determines the struct name:
    ///   - Return closures: use `fn_name`
    ///   - Var init closures: use the variable name
    ///   - Callback closures: use synthetic name like `{fn}_cb{N}`
    fn record_closure(
        &mut self,
        fn_name: &str,
        _fn_params: &FormalParameters,
        arrow: &ArrowFunctionExpression,
        is_return_closure: bool,
    ) {
        let span_key = arrow.span.start;
        if self.closure_map.contains_key(&span_key) {
            return; // already recorded
        }

        let struct_name = if is_return_closure {
            closure_name(fn_name)
        } else {
            // For variable assignments and callbacks, use a synthetic name
            self.closure_counter += 1;
            closure_name(&format!("{}_cb{}", fn_name, self.closure_counter))
        };

        if is_return_closure {
            self.fn_closure_spans.insert(fn_name.to_string(), span_key);
        }

        // Collect arrow function parameter info
        let mut params = Vec::new();
        let mut arrow_param_types: Vec<(String, crate::infer::ZigType)> = Vec::new();
        for p in &arrow.params.items {
            let pname = self.binding_name(&p.pattern).to_owned();
            let ptype = if let Some(default) = &p.initializer {
                self.inferrer.infer_expr(default)
            } else {
                self.inferrer.infer_arrow_param_type(&pname, &arrow.body)
            };
            arrow_param_types.push((pname.clone(), ptype.clone()));
            params.push((pname, ptype.to_zig_str()));
        }

        // Collect captured (free) variables from the arrow body.
        // "Captured" = identifiers in the arrow body that are NOT arrow params or local decls.
        // Outer function params referenced in the arrow ARE captured variables.

        // Collect arrow's own parameter names to exclude from captured set
        let arrow_param_set: HashSet<&str> = arrow
            .params
            .items
            .iter()
            .map(|p| self.binding_name(&p.pattern))
            .collect();

        let mut local_decls = HashSet::new();
        let mut free_vars = HashSet::new();
        if !arrow.expression {
            // Block body: collect locally declared variables first
            for s in &arrow.body.statements {
                if let Statement::VariableDeclaration(vd) = s {
                    for decl in &vd.declarations {
                        local_decls.insert(self.binding_name(&decl.id).to_owned());
                    }
                }
            }
        }
        for s in &arrow.body.statements {
            Self::collect_identifiers_in_stmt(s, &mut free_vars);
        }

        // Keep only identifiers that are NOT arrow params and NOT locally declared.
        // These are the captured (free) variables from the outer scope.
        let mut captured: Vec<(String, String)> = free_vars
            .into_iter()
            .filter(|name| !arrow_param_set.contains(name.as_str()) && !local_decls.contains(name))
            .map(|name| {
                let ty = self.inferrer.get_var_type(&name).to_zig_str();
                (name, ty)
            })
            .collect();
        captured.sort_by(|a, b| a.0.cmp(&b.0));

        // Infer return type of the arrow body, with arrow params registered
        let ret_ty = self.inferrer.infer_return_type_from_arrow_with_params(arrow, &arrow_param_types);

        let mut info = ClosureInfo {
            struct_name,
            captured,
            params,
            return_type: ret_ty.to_zig_str(),
            struct_def: String::new(),
        };

        // Generate struct definition string immediately (avoids storing AST references)
        let struct_def = self.generate_closure_struct_def(&info, arrow);
        info.struct_def = struct_def.clone();
        self.closure_struct_defs.insert(span_key, struct_def);
        self.closure_map.insert(span_key, info);
    }

    /// Recursively collect all identifier names used in an expression
    fn collect_identifiers_in_expr(expr: &Expression, set: &mut HashSet<String>) {
        match expr {
            Expression::Identifier(id) => {
                set.insert(id.name.to_string());
            }
            Expression::BinaryExpression(bin) => {
                Self::collect_identifiers_in_expr(&bin.left, set);
                Self::collect_identifiers_in_expr(&bin.right, set);
            }
            Expression::UnaryExpression(un) => {
                Self::collect_identifiers_in_expr(&un.argument, set);
            }
            Expression::CallExpression(call) => {
                Self::collect_identifiers_in_expr(&call.callee, set);
                for arg in &call.arguments {
                    if let Some(e) = arg.as_expression() {
                        Self::collect_identifiers_in_expr(e, set);
                    }
                }
            }
            Expression::StaticMemberExpression(mem) => {
                Self::collect_identifiers_in_expr(&mem.object, set);
            }
            Expression::ComputedMemberExpression(mem) => {
                Self::collect_identifiers_in_expr(&mem.object, set);
                Self::collect_identifiers_in_expr(&mem.expression, set);
            }
            Expression::AssignmentExpression(assign) => {
                // For identifier collection, only traverse the right side
                Self::collect_identifiers_in_expr(&assign.right, set);
            }
            Expression::ConditionalExpression(cond) => {
                Self::collect_identifiers_in_expr(&cond.test, set);
                Self::collect_identifiers_in_expr(&cond.consequent, set);
                Self::collect_identifiers_in_expr(&cond.alternate, set);
            }
            Expression::LogicalExpression(log) => {
                Self::collect_identifiers_in_expr(&log.left, set);
                Self::collect_identifiers_in_expr(&log.right, set);
            }
            Expression::ArrayExpression(arr) => {
                for elem in &arr.elements {
                    if let Some(e) = elem.as_expression() {
                        Self::collect_identifiers_in_expr(e, set);
                    }
                }
            }
            Expression::ObjectExpression(obj) => {
                for prop in &obj.properties {
                    if let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(p) = prop {
                        Self::collect_identifiers_in_expr(&p.value, set);
                    }
                }
            }
            Expression::ParenthesizedExpression(p) => {
                Self::collect_identifiers_in_expr(&p.expression, set);
            }
            Expression::SequenceExpression(seq) => {
                for e in &seq.expressions {
                    Self::collect_identifiers_in_expr(e, set);
                }
            }
            _ => {}
        }
    }

    /// Collect identifiers from a statement (simplified: only recurse into return/expression)
    fn collect_identifiers_in_stmt(stmt: &Statement, set: &mut HashSet<String>) {
        match stmt {
            Statement::ExpressionStatement(es) => {
                Self::collect_identifiers_in_expr(&es.expression, set);
            }
            Statement::ReturnStatement(rs) => {
                if let Some(arg) = &rs.argument {
                    Self::collect_identifiers_in_expr(arg, set);
                }
            }
            Statement::VariableDeclaration(vd) => {
                for decl in &vd.declarations {
                    if let Some(init) = &decl.init {
                        Self::collect_identifiers_in_expr(init, set);
                    }
                }
            }
            Statement::IfStatement(if_stmt) => {
                Self::collect_identifiers_in_expr(&if_stmt.test, set);
                Self::collect_identifiers_in_stmt(&if_stmt.consequent, set);
                if let Some(alt) = &if_stmt.alternate {
                    Self::collect_identifiers_in_stmt(alt, set);
                }
            }
            Statement::BlockStatement(block) => {
                for s in &block.body {
                    Self::collect_identifiers_in_stmt(s, set);
                }
            }
            Statement::ForInStatement(fi) => {
                Self::collect_identifiers_in_stmt(&fi.body, set);
            }
            Statement::ForOfStatement(fo) => {
                Self::collect_identifiers_in_stmt(&fo.body, set);
            }
            Statement::WhileStatement(ws) => {
                Self::collect_identifiers_in_stmt(&ws.body, set);
            }
            Statement::DoWhileStatement(dw) => {
                Self::collect_identifiers_in_stmt(&dw.body, set);
            }
            _ => {}
        }
    }

    // ========== Helpers ==========

    fn emit_indent(&mut self) {
        for _ in 0..self.indent {
            self.output.push_str("    ");
        }
    }

    fn get_indent_str(&self, level: usize) -> String {
        "    ".repeat(level)
    }

    fn push(&mut self, s: &str) {
        self.output.push_str(s);
    }

    fn push_line(&mut self, s: &str) {
        self.emit_indent();
        self.push(s);
        self.push("\n");
    }

    fn binding_name<'b>(&self, pattern: &BindingPattern<'b>) -> &'b str {
        match pattern {
            BindingPattern::BindingIdentifier(id) => id.name.as_str(),
            _ => "_unsupported_pattern",
        }
    }

    /// Escape Zig reserved keywords using @"name" syntax
    fn escape_keyword(name: &str) -> String {
        let keywords: &[&str] = &[
            "addrspace", "align", "and", "anyframe", "anytype", "asm", "async",
            "await", "break", "callconv", "catch", "comptime", "const", "continue",
            "defer", "else", "enum", "errdefer", "error", "export", "extern",
            "false", "fn", "for", "if", "inline", "linksection", "noalias",
            "noinline", "nosuspend", "null", "opaque", "or", "orelse", "packed",
            "pub", "resume", "return", "struct", "suspend", "switch", "test",
            "threadlocal", "true", "try", "type", "union", "unreachable",
            "usingnamespace", "var", "volatile", "while",
        ];
        if keywords.contains(&name) {
            format!("@\"{}\"", name)
        } else {
            name.to_string()
        }
    }

    // ========== Statements ==========

    fn emit_stmt(&mut self, stmt: &Statement) {
        // Top-level: only VariableDeclaration and FunctionDeclaration (and ClassDeclaration) are allowed
        if self.in_top_level {
            match stmt {
                Statement::VariableDeclaration(vd) => self.emit_var_decl(vd),
                Statement::FunctionDeclaration(fd) => self.emit_fn_decl(fd),
                Statement::ClassDeclaration(cd) => self.emit_class_decl(cd),
                Statement::ExpressionStatement(_) => {
                    self.diagnostics.push(crate::infer::Diagnostic {
                        kind: crate::infer::DiagnosticKind::Error,
                        message: "top-level expression statements are not allowed; \
                                  use a variable declaration or function declaration instead"
                            .to_string(),
                    });
                }
                _ => {
                    self.diagnostics.push(crate::infer::Diagnostic {
                        kind: crate::infer::DiagnosticKind::Error,
                        message: format!(
                            "only variable declarations and function declarations are allowed \
                             at top level, found: {:?}",
                            std::mem::discriminant(stmt)
                        ),
                    });
                }
            }
            return;
        }

        // Inside function body: reject nested FunctionDeclaration
        if matches!(stmt, Statement::FunctionDeclaration(_)) {
            self.push_line("// ERROR: nested function declarations are not allowed");
            self.diagnostics.push(crate::infer::Diagnostic {
                kind: crate::infer::DiagnosticKind::Error,
                message: "nested function declarations are not allowed".to_string(),
            });
            return;
        }

        match stmt {
            Statement::VariableDeclaration(vd) => self.emit_var_decl(vd),
            Statement::FunctionDeclaration(fd) => self.emit_fn_decl(fd),
            Statement::ExpressionStatement(es) => {
                self.emit_indent();
                // Zig 0.16: do NOT use `_ = expr;` for expression statements.
                // This causes "error set is discarded" error.
                self.emit_expr(&es.expression);
                self.push(";\n");
            }
            Statement::ReturnStatement(rs) => {
                self.emit_indent();
                if let Some(ref label) = self.catch_label {
                    // Inside a catch block: break to catch label (provides default value)
                    self.push(&format!("break :{} ", label));
                } else if let Some(ref label) = self.try_label {
                    // Inside a try block: break to the try label
                    self.push(&format!("break :{} ", label));
                } else {
                    self.push("return ");
                }
                if let Some(arg) = &rs.argument {
                    self.emit_expr(arg);
                }
                self.push(";\n");
            }
            Statement::IfStatement(if_stmt) => self.emit_if_stmt(if_stmt),
            Statement::BlockStatement(block) => {
                self.push_line("{");
                self.indent += 1;
                for s in &block.body {
                    self.emit_stmt(s);
                }
                self.indent -= 1;
                self.push_line("}");
            }
            Statement::ForStatement(fs) => self.emit_for_stmt(fs),
            Statement::ForInStatement(fis) => self.emit_for_in_stmt(fis),
            Statement::ForOfStatement(fos) => self.emit_for_of_stmt(fos),
            Statement::WhileStatement(ws) => self.emit_while_stmt(ws),
            Statement::DoWhileStatement(dw) => self.emit_do_while_stmt(dw),
            Statement::EmptyStatement(_) => {}
            Statement::BreakStatement(_) => self.push_line("break;"),
            Statement::ContinueStatement(_) => self.push_line("continue;"),
            Statement::SwitchStatement(sw) => self.emit_switch_stmt(sw),
            Statement::ThrowStatement(_throw_stmt) => {
                self.emit_indent();
                if let Some(ref label) = self.try_label {
                    // Inside a try block: break to the try label with an error
                    self.push(&format!("break :{} error.Unexpected", label));
                } else {
                    // Outside try block: return error (will be caught by caller)
                    self.push("return error.Unexpected");
                }
                self.push(";\n");
            }
            Statement::TryStatement(ts) => self.emit_try_stmt(ts),
            _ => {
                self.push_line("// TODO: unsupported statement");
            }
        }
    }

    /// Emit declaration for variables that need dynamic property access.
    /// Generates:
    ///   var name: std.StringHashMap(JsValue) = undefined;
    /// The actual initialization is done in init_js2rust() function.
    fn emit_dynamic_access_var_decl(&mut self, name: &str) {
        self.emit_indent();
        self.push("var ");
        self.push(name);
        self.push(": std.StringHashMap(JsValue) = undefined;\n");
    }

    /// Generate initialization code for a dynamic access variable.
    /// The code is buffered in init_globals_code and emitted in init_js2rust().
    fn emit_dynamic_access_var_init_code(&mut self, name: &str, obj: &ObjectExpression) {
        // Add initialization code: name = std.StringHashMap(JsValue).init(allocator);
        self.init_globals_code.push(format!(
            "    {} = std.StringHashMap(JsValue).init(allocator);\n",
            name
        ));

        // Add put() calls for each property
        for prop in &obj.properties {
            if let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(p) = prop {
                let field_name = match &p.key {
                    oxc_ast::ast::PropertyKey::StaticIdentifier(id) => id.name.to_string(),
                    oxc_ast::ast::PropertyKey::StringLiteral(s) => s.value.to_string(),
                    _ => continue,
                };
                
                // Generate JsValue literal based on expression type
                let js_value_lit = self.emit_js_value_literal(&p.value);
                self.init_globals_code.push(format!(
                    "    {}.put(\"{}\", {}) catch @panic(\"OOM\");\n",
                    name, field_name, js_value_lit
                ));
            }
        }
    }

    /// Emit JsValue literal for an expression (used in HashMap initialization).
    fn emit_js_value_literal(&self, expr: &Expression) -> String {
        match expr {
            Expression::NumericLiteral(lit) => {
                let val_str = lit.value.to_string();
                if lit.value.fract() != 0.0 {
                    format!(".{{ .float = {} }}", val_str)
                } else {
                    format!(".{{ .int = {} }}", val_str)
                }
            }
            Expression::StringLiteral(lit) => {
                format!(".{{ .string = \"{}\" }}", lit.value)
            }
            Expression::BooleanLiteral(lit) => {
                format!(".{{ .bool = {} }}", if lit.value { "true" } else { "false" })
            }
            Expression::NullLiteral(_) => {
                ".{ .null = {} }".to_string()
            }
            _ => {
                // Unsupported - store as int 0
                ".{ .int = 0 }".to_string()
            }
        }
    }

    /// Emit JsValue construction code for an expression (used in HashMap.put() calls).
    /// For literals, emit inline JsValue literal.
    /// For complex expressions, emit a simple wrap (assumes i64).
    fn emit_js_value_construction(&mut self, expr: &Expression) {
        match expr {
            Expression::NumericLiteral(lit) => {
                let val_str = lit.value.to_string();
                if lit.value.fract() != 0.0 {
                    self.push(&format!("JsValue{{ .float = {} }}", val_str));
                } else {
                    self.push(&format!("JsValue{{ .int = {} }}", val_str));
                }
            }
            Expression::StringLiteral(lit) => {
                self.push(&format!("JsValue{{ .string = \"{}\" }}", lit.value));
            }
            Expression::BooleanLiteral(lit) => {
                let b = if lit.value { "true" } else { "false" };
                self.push(&format!("JsValue{{ .bool = {} }}", b));
            }
            Expression::NullLiteral(_) => {
                self.push("JsValue{ .null = {} }");
            }
            _ => {
                // TODO: use runtime helper for proper type conversion
                self.push("JsValue{ .int = ");
                self.emit_expr(expr);
                self.push(" }");
            }
        }
    }

    /// Return the correct JsValue variant field accessor for a HashMap field lookup.
    /// Look up the field type from the original object type stored in the type inferrer.
    fn dynamic_field_accessor(&self, obj_expr: &Expression, prop: &str) -> String {
        if let Expression::Identifier(id) = obj_expr {
            let obj_type = self.inferrer.get_var_type(id.name.as_str());
            if let ZigType::Object { fields } = obj_type
                && let Some((_, field_type)) = fields.iter().find(|(n, _)| n == prop)
            {
                return match field_type {
                    ZigType::String => ".string".to_string(),
                    ZigType::F64 | ZigType::F32 => ".float".to_string(),
                    ZigType::Bool => ".bool".to_string(),
                    ZigType::Null => unreachable!(), // put() stores null variant
                    _ => ".asI64()".to_string(),
                };
            }
        }
        ".asI64()".to_string()
    }

    /// Emit the init_js2rust() function that initializes all global HashMaps.
    fn emit_init_js2rust(&mut self) {
        // Always generate init_js2rust() — empty if no dynamic access vars
        self.push("\n");
        self.push("/// Initialize global allocator and all objects that use dynamic property access.\n");
        self.push("pub fn init_js2rust(allocator: std.mem.Allocator) void {\n");
        self.push("    js_allocator.setGlobalAllocator(allocator);\n");
        // Collect init code to avoid borrowing self twice
        let init_code: String = self.init_globals_code.iter().cloned().collect();
        if !init_code.is_empty() {
            self.push(&init_code);
        }
        self.push("}\n");
    }

    /// Emit deinit_js2rust() that frees all global HashMaps.
    fn emit_deinit_js2rust(&mut self) {
        // Extract HashMap variable names from init code (format: "    varname = std.StringHashMap...")
        let mut hashmap_names: Vec<String> = Vec::new();
        for line in &self.init_globals_code {
            if line.contains(".init(")
                && let Some(name) = line.trim().split(" =").next()
            {
                hashmap_names.push(name.to_string());
            }
        }
        // Always generate deinit — noop if no HashMaps, but needed for defer
        self.push("\n");
        self.push("/// Deinitialize all global objects created by init_js2rust().\n");
        self.push("pub fn deinit_js2rust() void {\n");
        for name in &hashmap_names {
            self.push(&format!("    {}.deinit();\n", name));
        }
        self.push("}\n");
    }



    fn emit_var_decl(&mut self, vd: &VariableDeclaration) {
        for decl in &vd.declarations {
            // Handle destructured patterns: flatten into individual variable declarations
            if !is_simple_binding(&decl.id) {
                self.emit_destructured_var_decl(decl, vd.kind);
                continue;
            }
            let name = self.binding_name(&decl.id);
            // Skip test_ variables — test generation helpers, stripped from output
            if name.starts_with("test_") {
                continue;
            }

            if let Some(init) = &decl.init {
                match init {
                    Expression::ArrowFunctionExpression(arrow) => {
                        // Check if this arrow is a closure (has captured variables)
                        let maybe_ci = self.closure_map.get(&arrow.span.start).cloned();
                        if let Some(ci) = maybe_ci
                            && !ci.captured.is_empty()
                        {
                            self.emit_closure_var_init(name, &ci);
                            continue;
                        }
                        self.emit_arrow_fn(name, arrow);
                        continue;
                    }
                    Expression::FunctionExpression(fe) => {
                        self.emit_fn_from_func_expr(name, fe);
                        continue;
                    }
                    Expression::ObjectExpression(obj) if self.in_top_level => {
                        // Check if this variable needs dynamic access (HashMap instead of struct)
                        if self.inferrer.get_dynamic_access_vars().contains(name) {
                            self.emit_dynamic_access_var_decl(name);
                            self.emit_dynamic_access_var_init_code(name, obj);
                            continue;
                        }
                        let obj_type = self.inferrer.infer_expr(init);
                        if let ZigType::Object { ref fields } = obj_type
                            && !fields.is_empty()
                            && fields.iter().all(|(_, ty)| *ty != ZigType::Any)
                        {
                            let kw = match vd.kind {
                                VariableDeclarationKind::Const => "const",
                                _ => "var",
                            };
                            let escaped_name = Self::escape_keyword(name);
                            let struct_name = Self::capitalize_first(name);
                            let def = Self::gen_obj_struct_def(&struct_name, fields);
                            self.object_type_defs.push(def);

                            self.emit_indent();
                            self.push(kw);
                            self.push(" ");
                            self.push(&escaped_name);
                            self.push(": ");
                            self.push(&struct_name);
                            self.push(" = .{ ");
                            let mut first = true;
                            for prop in &obj.properties {
                                if let ObjectPropertyKind::ObjectProperty(p) = prop {
                                    if !first { self.push(", "); }
                                    first = false;
                                    self.push(".");
                                    let key_str = property_key_name(&p.key);
                                    self.push(&key_str);
                                    self.push(" = ");
                                    self.emit_expr(&p.value);
                                }
                            }
                            self.push(" };\n");
                            continue;
                        }
                        // Fall through to generic anonymous .{} emission
                        // (occurs when fields include functions or other unresolvable types)
                    }
                    _ => {}
                }
            }

            // Dynamic array: use std.ArrayList instead of fixed-size [_]T
            if self.inferrer.is_dynamic_array(name)
                && let Some(init) = &decl.init
            {
                let elem_type = match init {
                    Expression::ArrayExpression(arr) => {
                        if arr.elements.is_empty() {
                            ZigType::I64
                        } else {
                            arr.elements.iter().find_map(|elem| match elem {
                                ArrayExpressionElement::SpreadElement(_) => None,
                                ArrayExpressionElement::Elision(_) => None,
                                _ => elem.as_expression().map(|e| self.inferrer.infer_expr(e)),
                            }).unwrap_or(ZigType::I64)
                        }
                    }
                    _ => self.inferrer.infer_expr(init),
                };
                let et = elem_type.to_zig_str();
                let escaped = Self::escape_keyword(name);

                // var name = std.ArrayList(T).empty; // Zig 0.16 correct initialization
                self.emit_indent();
                self.push("var ");
                self.push(&escaped);
                self.push(" = std.ArrayList(");
                self.push(&et);
                self.push(").empty; ");

                // Append initial elements if array literal
                if let Expression::ArrayExpression(arr) = init
                    && !arr.elements.is_empty()
                {
                    self.emit_indent();
                    self.push(&escaped);
                    self.push(".appendSlice(js_allocator.g_alloc(), &[_]");
                    self.push(&et);
                    self.push("{ ");
                    for (i, elem) in arr.elements.iter().enumerate() {
                        if i > 0 { self.push(", "); }
                        self.emit_array_element(elem);
                    }
                    self.push(" }) catch unreachable;\n");
                }
                continue;
            }

            let keyword = match vd.kind {
                VariableDeclarationKind::Const => "const",
                VariableDeclarationKind::Let | VariableDeclarationKind::Var => "var",
                _ => "var",
            };

            let name: String = Self::escape_keyword(name);
            self.emit_indent();
            self.push(keyword);
            self.push(" ");
            self.push(&name);

            // Zig 0.16: add type annotation for var declarations
            if keyword == "var" {
                let var_type = self.inferrer.get_var_type(&name);
                let type_str = match &var_type {
                    ZigType::Any => "i64".to_string(),
                    _ => var_type.to_zig_str(),
                };
                self.push(": ");
                self.push(&type_str);
            }

            if let Some(init) = &decl.init {
                self.push(" = ");
                self.emit_expr(init);
            } else {
                self.push(" = undefined");
            }

            self.push(";\n");

            // Zig 0.16: do NOT emit `_ = name;` — causes "pointless discard" error.
            // Unused variable warnings are now handled by the Zig compiler differently.
            // (Previously: suppress "unused local constant" for trivial literals)
            }
    }

    ///   const _tmp_0 = expr;
    ///   const a = _tmp_0.a;
    ///   const b = _tmp_0.b;
    fn emit_destructured_var_decl(
        &mut self,
        decl: &VariableDeclarator,
        kind: VariableDeclarationKind,
    ) {
        let mut leaves = Vec::new();
        // Start with empty prefix — will be replaced with temp name after init is emitted
        flatten_binding_pattern(&decl.id, "", &mut leaves);

        // Skip if all leaves are test_ helpers
        if leaves.iter().all(|l| l.name.starts_with("test_")) {
            return;
        }

        let keyword = match kind {
            VariableDeclarationKind::Const => "const",
            _ => "var",
        };

        if let Some(init) = &decl.init {
            let temp_name = format!("_tmp_{}", self.temp_counter);
            self.temp_counter += 1;

            // Check if the init expression is a dynamic array
            let is_init_dynamic_array = if let Expression::Identifier(id) = init {
                self.inferrer.is_dynamic_array(id.name.as_str())
            } else {
                false
            };

            self.emit_indent();
            self.push(keyword);
            self.push(" ");
            self.push(&temp_name);
            self.push(" = ");
            self.emit_expr(init);
            self.push(";\n");

            for leaf in &leaves {
                if leaf.name.starts_with("test_") {
                    continue;
                }
                let escaped = Self::escape_keyword(leaf.name);
                self.emit_indent();
                self.push(keyword);
                self.push(" ");
                self.push(&escaped);
                if !leaf.access.is_empty() {
                    self.push(" = ");
                    self.push(&temp_name);
                    // For dynamic arrays, use .items[...] instead of [...]
                    if is_init_dynamic_array && leaf.access.starts_with('[') {
                        self.push(".items");
                    }
                    self.push(&leaf.access);
                } else {
                    // No access path (shouldn't happen for destructured patterns)
                    self.push(" = undefined");
                }
                self.push(";\n");
            }
        } else {
            // No initializer — declare with undefined
            for leaf in &leaves {
                if leaf.name.starts_with("test_") {
                    continue;
                }
                let escaped = Self::escape_keyword(leaf.name);
                self.emit_indent();
                self.push(keyword);
                self.push(" ");
                self.push(&escaped);
                self.push(" = undefined;\n");
            }
        }
    }

    /// Check if a function body contains any `AwaitExpression` (which codegen will
    /// translate into `io.async(...)` usage).
    fn body_contains_await(body: &oxc_allocator::Box<'_, FunctionBody<'_>>) -> bool {
        for stmt in &body.statements {
            if Self::stmt_contains_await(stmt) {
                return true;
            }
        }
        false
    }

    /// Check if a statement contains any `AwaitExpression`.
    fn stmt_contains_await(stmt: &Statement) -> bool {
        match stmt {
            Statement::ExpressionStatement(es) => Self::expr_contains_await(&es.expression),
            Statement::ReturnStatement(rs) => {
                rs.argument
                    .as_ref()
                    .map(|arg| Self::expr_contains_await(arg))
                    .unwrap_or(false)
            }
            Statement::VariableDeclaration(vd) => {
                for decl in &vd.declarations {
                    if let Some(init) = &decl.init
                        && Self::expr_contains_await(init)
                    {
                        return true;
                    }
                }
                false
            }
            Statement::IfStatement(if_stmt) => {
                Self::expr_contains_await(&if_stmt.test)
                    || Self::stmt_contains_await(&if_stmt.consequent)
                    || if_stmt
                        .alternate
                        .as_ref()
                        .map(|alt| Self::stmt_contains_await(alt))
                        .unwrap_or(false)
            }
            Statement::BlockStatement(block) => {
                block.body.iter().any(|s| Self::stmt_contains_await(s))
            }
            Statement::ForStatement(_)
            | Statement::WhileStatement(_)
            | Statement::DoWhileStatement(_)
            | Statement::ForInStatement(_)
            | Statement::ForOfStatement(_)
            | Statement::SwitchStatement(_)
            | Statement::TryStatement(_) => true, // conservatively assume await
            _ => false,
        }
    }

    /// Check if an expression contains any `AwaitExpression`.
    fn expr_contains_await(expr: &Expression) -> bool {
        match expr {
            Expression::AwaitExpression(_) => true,
            Expression::CallExpression(call) => {
                Self::expr_contains_await(&call.callee)
                    || call.arguments.iter().any(|arg| match arg {
                        Argument::SpreadElement(s) => Self::expr_contains_await(&s.argument),
                        _ => arg
                            .as_expression()
                            .map(|e| Self::expr_contains_await(e))
                            .unwrap_or(false),
                    })
            }
            Expression::BinaryExpression(bin) => {
                Self::expr_contains_await(&bin.left) || Self::expr_contains_await(&bin.right)
            }
            Expression::UnaryExpression(unary) => Self::expr_contains_await(&unary.argument),
            Expression::LogicalExpression(logic) => {
                Self::expr_contains_await(&logic.left)
                    || Self::expr_contains_await(&logic.right)
            }
            Expression::ParenthesizedExpression(p) => Self::expr_contains_await(&p.expression),
            Expression::ConditionalExpression(cond) => {
                Self::expr_contains_await(&cond.test)
                    || Self::expr_contains_await(&cond.consequent)
                    || Self::expr_contains_await(&cond.alternate)
            }
            Expression::AssignmentExpression(assign) => {
                Self::expr_contains_await(&assign.right)
            }
            Expression::SequenceExpression(seq) => {
                seq.expressions.iter().any(|e| Self::expr_contains_await(e))
            }
            _ => false,
        }
    }

    // --- Function declarations ---

    fn emit_fn_decl(&mut self, fd: &Function) {
        let raw_name = fd.id.as_ref().map(|id| id.name.as_str()).unwrap_or("anonymous");
        let name = Self::escape_keyword(raw_name);
        let is_async = fd.r#async;
        let is_export = self.exports.contains(raw_name);

        // Generate struct definitions for Object-typed parameters BEFORE the function signature.
        // This must run before any emit_params call so the structs are in scope.
        self.current_obj_structs.clear();
        let param_types = self.inferrer.get_fn_param_types(raw_name);
        let mut obj_defs: Vec<String> = Vec::new();
        for (i, ptype) in param_types.iter().enumerate() {
            // Generate struct definitions for Object-typed parameters.
            // Any fields are filtered out (same as top-level const objects).
            if let ZigType::Object { fields } = ptype {
                // Keep only fields with known (non-Any) types.
                let known: Vec<(String, ZigType)> = fields
                    .iter()
                    .filter(|(_, ty)| *ty != ZigType::Any)
                    .map(|(n, ty)| (n.clone(), ty.clone()))
                    .collect();
                if known.is_empty() {
                    continue;
                }
                let pname = if i < fd.params.items.len() {
                    self.binding_name(&fd.params.items[i].pattern).to_string()
                } else {
                    format!("arg{}", i)
                };
                let struct_name = format!(
                    "{}{}",
                    Self::capitalize_first(raw_name),
                    Self::capitalize_first(&pname),
                );
                let def = Self::gen_obj_struct_def(&struct_name, &known);
                obj_defs.push(def);
                if i >= self.current_obj_structs.len() {
                    self.current_obj_structs.resize_with(i + 1, || None);
                }
                self.current_obj_structs[i] = Some(struct_name);
            }
        }
        for def in &obj_defs {
            self.push(def);
        }

        // Build return type string
        let ret_type_str = if let Some(&span) = self.fn_closure_spans.get(raw_name)
            && let Some(ci) = self.closure_map.get(&span)
        {
            ci.struct_name.clone()
        } else {
            self.inferrer.get_fn_return_type(raw_name).to_zig_str()
        };

        // Async functions cannot use C ABI (error union return not C-compatible).
        // For async exports: keep as `pub fn` (Zig-only, no callconv).
        if is_async && is_export {
            self.emit_indent();
            self.push("pub fn ");
            self.push(&name);
            self.push("(");
            self.push("io: Io");
            if !fd.params.items.is_empty() {
                self.push(", ");
            }
            self.emit_params(&fd.params, Some(raw_name));
            self.push(") !");
            self.push(&ret_type_str);
            self.push(" ");
            self.emit_fn_body(fd, raw_name, true);
            return;
        }

        // Determine if this sync export needs a C ABI wrapper
        let needs_cabi_wrapper;
        let param_types: Vec<ZigType>;
        let ret_type: ZigType;
        if is_export && !is_async {
            param_types = self.inferrer.get_fn_param_types(raw_name);
            let has_string_param = param_types.contains(&ZigType::String);
            ret_type = self.inferrer.get_fn_return_type(raw_name);
            let returns_string = ret_type == ZigType::String;
            let returns_closure = self.fn_closure_spans.contains_key(raw_name);
            needs_cabi_wrapper = has_string_param || returns_string || returns_closure;
        } else {
            param_types = Vec::new();
            ret_type = ZigType::Any;
            needs_cabi_wrapper = false;
        };

        if needs_cabi_wrapper {
            // Emit internal impl function
            self.emit_indent();
            self.push("fn ");
            self.push(&name);
            self.push("_impl(");
            self.emit_params(&fd.params, Some(raw_name));
            self.push(") ");
            self.push(&ret_type_str);
            self.push(" ");
            self.emit_fn_body(fd, raw_name, false);

            // Buffer C ABI wrapper
            let wrapper = self.generate_cabi_wrapper(raw_name, &name, fd, &ret_type_str);
            self.cabi_wrappers.push(wrapper);

            // Record C ABI export metadata
            let returns_string = ret_type == ZigType::String;
            let returns_closure = self.fn_closure_spans.contains_key(raw_name);
            let mut params: Vec<(String, ZigType)> = Vec::new();
            for (i, p) in fd.params.items.iter().enumerate() {
                let pname = Self::escape_keyword(self.binding_name(&p.pattern));
                let ptype = if i < param_types.len() {
                    param_types[i].clone()
                } else {
                    ZigType::Any
                };
                params.push((pname, ptype));
            }
            self.cabi_exports.push(CabiExport {
                name: name.clone(),
                params,
                ret_type: ret_type.clone(),
                has_free_func: returns_string || returns_closure,
            });
        } else if is_export {
            // Simple export: no string/closure types, use direct C ABI
            self.emit_indent();
            self.push("export fn ");
            self.push(&name);
            self.push("(");
            self.emit_params(&fd.params, Some(raw_name));
            self.push(") callconv(.c) ");
            self.push(&ret_type_str);
            self.push(" ");
            self.emit_fn_body(fd, raw_name, false);

            // Record simple C ABI export metadata
            let mut params: Vec<(String, ZigType)> = Vec::new();
            for (i, p) in fd.params.items.iter().enumerate() {
                let pname = Self::escape_keyword(self.binding_name(&p.pattern));
                let ptype = if i < param_types.len() {
                    param_types[i].clone()
                } else {
                    ZigType::Any
                };
                params.push((pname, ptype));
            }
            self.cabi_exports.push(CabiExport {
                name: name.clone(),
                params,
                ret_type: ret_type.clone(),
                has_free_func: false,
            });
        } else {
            // Non-exported function
            self.emit_indent();
            if is_async {
                self.push("fn ");
                self.push(&name);
                self.push("(");
                self.push("io: Io");
                if !fd.params.items.is_empty() {
                    self.push(", ");
                }
                self.emit_params(&fd.params, Some(raw_name));
                self.push(") !");
                self.push(&ret_type_str);
                self.push(" ");
                self.emit_fn_body(fd, raw_name, true);
            } else {
                self.push("fn ");
                self.push(&name);
                self.push("(");
                self.emit_params(&fd.params, Some(raw_name));
                self.push(") ");
                self.push(&ret_type_str);
                self.push(" ");
                self.emit_fn_body(fd, raw_name, false);
            }
        }
    }

    /// Emit function body block
    fn emit_fn_body(&mut self, fd: &Function, raw_name: &str, is_async: bool) {
        if let Some(body) = &fd.body {
            self.push("{\n");
            self.indent += 1;

            // Emit destructured parameter prelude statements
            // e.g., for `function foo({a, b})`: `const a = _arg0.a; const b = _arg0.b;`
            for prelude in self.destructure_prelude.drain(..) {
                self.output.push_str(&prelude);
            }

            if is_async && !Self::body_contains_await(body) {
                self.emit_indent();
                self.push_line("_ = io;");
            }
            let prev = self.in_top_level;
            self.in_top_level = false;
            let prev_fn = self.current_fn.take();
            self.current_fn = Some(raw_name.to_string());
            // Also set inferrer.current_fn so get_var_type() can look up fn_local_types
            let prev_infer_fn = self.inferrer.current_fn.take();
            self.inferrer.current_fn = Some(raw_name.to_string());
            for stmt in &body.statements {
                self.emit_stmt(stmt);
            }
            self.inferrer.current_fn = prev_infer_fn;
            self.current_fn = prev_fn;
            self.in_top_level = prev;
            self.indent -= 1;
            self.push_line("}");
        } else {
            self.push("{};\n");
        }
        self.push("\n");
    }

    /// Generate a C ABI export wrapper for a sync function with string params/returns or closures.
    fn generate_cabi_wrapper(
        &mut self,
        raw_name: &str,
        escaped_name: &str,
        fd: &Function,
        ret_type_str: &str,
    ) -> String {
        let param_types = self.inferrer.get_fn_param_types(raw_name);
        let ret_type = self.inferrer.get_fn_return_type(raw_name);
        let returns_string = ret_type == ZigType::String;
        let returns_closure = self.fn_closure_spans.contains_key(raw_name);

        let mut w = String::new();
        w.push_str(&format!("export fn {}(", escaped_name));

        // C ABI params (no async)
        let mut cabi_params: Vec<String> = Vec::new();
        for (i, param) in fd.params.items.iter().enumerate() {
            let pname = self.binding_name(&param.pattern);
            let safe_pname = Self::escape_keyword(pname);
            let ptype = if i < param_types.len() {
                param_types[i].clone()
            } else {
                ZigType::Any
            };
            if ptype == ZigType::String {
                cabi_params.push(format!("{}: [*:0]const u8", safe_pname));
            } else {
                cabi_params.push(format!("{}: {}", safe_pname, ptype.to_zig_str()));
            }
        }
        w.push_str(&cabi_params.join(", "));
        w.push_str(") callconv(.c) ");

        if returns_string {
            w.push_str("[*:0]const u8");
        } else if returns_closure {
            w.push_str("*anyopaque");
        } else {
            w.push_str(ret_type_str);
        }
        w.push_str(" {\n");

        // Body: convert C strings → Zig slices
        for (i, param) in fd.params.items.iter().enumerate() {
            let pname = self.binding_name(&param.pattern);
            let safe_pname = Self::escape_keyword(pname);
            let ptype = if i < param_types.len() {
                param_types[i].clone()
            } else {
                ZigType::Any
            };
            if ptype == ZigType::String {
                w.push_str(&format!(
                    "    const {}_slice: []const u8 = std.mem.span({});\n",
                    safe_pname, safe_pname
                ));
            }
        }

        // Call impl
        w.push_str("    ");
        if returns_string || returns_closure {
            w.push_str("const _result = ");
        } else if ret_type_str != "void" {
            w.push_str("return ");
        }
        w.push_str(&format!("{}_impl(", escaped_name));
        let mut call_args: Vec<String> = Vec::new();
        for (i, param) in fd.params.items.iter().enumerate() {
            let pname = self.binding_name(&param.pattern);
            let safe_pname = Self::escape_keyword(pname);
            let ptype = if i < param_types.len() {
                param_types[i].clone()
            } else {
                ZigType::Any
            };
            if ptype == ZigType::String {
                call_args.push(format!("{}_slice", safe_pname));
            } else {
                call_args.push(safe_pname);
            }
        }
        w.push_str(&call_args.join(", "));
        w.push_str(");\n");

        // Handle string return
        if returns_string {
            w.push_str("    return @ptrCast(_result.ptr);\n");
        }

        // Handle closure return: allocate on heap, return opaque pointer
        if returns_closure {
            w.push_str("    const alloc = js_allocator.g_alloc();\n");
            w.push_str("    const ptr = alloc.create(@TypeOf(_result)) catch @panic(\"OOM\");\n");
            w.push_str("    ptr.* = _result;\n");
            w.push_str("    return @ptrCast(ptr);\n");
        }

        w.push_str("}\n\n");

        // Generate free_xxx for string returns
        if returns_string {
            w.push_str(&format!(
                "export fn free_{}(ptr: [*:0]const u8) callconv(.c) void {{\n    _ = js_allocator.g_alloc().free(std.mem.span(ptr));\n}}\n\n",
                escaped_name
            ));
        }

        // Generate free_xxx for closure returns
        if returns_closure {
            w.push_str(&format!(
                "export fn free_{}(ptr: *anyopaque) callconv(.c) void {{\n    const alloc = js_allocator.g_alloc();\n    const typed: *{} = @ptrCast(@alignCast(ptr));\n    alloc.destroy(typed);\n}}\n\n",
                escaped_name, ret_type_str
            ));
        }

        if returns_string || returns_closure {
            self.string_return_fns.insert(raw_name.to_string());
        }

        w
    }

    // ========== Class Support ==========

    /// Collect field names from `this.x = val` assignments in the constructor body.
    fn collect_class_fields(body: &FunctionBody) -> Vec<String> {
        let mut fields = Vec::new();
        for stmt in &body.statements {
            if let Statement::ExpressionStatement(es) = stmt
                && let Expression::AssignmentExpression(assign) = &es.expression
                && let AssignmentTarget::StaticMemberExpression(mem) = &assign.left
                && matches!(mem.object, Expression::ThisExpression(_))
            {
                let name = mem.property.name.to_string();
                if !fields.contains(&name) {
                    fields.push(name);
                }
            }
        }
        fields
    }

    fn emit_class_decl(&mut self, cd: &Class) {
        let raw_name = cd
            .id
            .as_ref()
            .map(|id| id.name.as_str())
            .unwrap_or("Anonymous");
        let name = Self::escape_keyword(raw_name);
        let is_export = self.exports.contains(raw_name);

        // Collect fields from constructor
        let mut fields = Vec::new();
        let mut methods: Vec<&MethodDefinition> = Vec::new();
        let mut constructor: Option<&MethodDefinition> = None;

        for elem in &cd.body.body {
            if let ClassElement::MethodDefinition(md) = elem {
                match &md.key {
                    PropertyKey::StaticIdentifier(id) if id.name.as_str() == "constructor" => {
                        constructor = Some(md);
                        if let Some(body) = &md.value.body {
                            fields = Self::collect_class_fields(body);
                        }
                    }
                    _ => {
                        methods.push(md);
                    }
                }
            }
        }

        // Emit struct definition
        let vis = if is_export { "pub const" } else { "const" };
        self.push(&format!("{} {} = struct {{\n", vis, name));
        self.indent += 1;

        // Fields
        if !fields.is_empty() {
            for (i, f) in fields.iter().enumerate() {
                self.emit_indent();
                self.push(f);
                self.push(": i64");
                if i < fields.len() - 1 || constructor.is_some() || !methods.is_empty() {
                    self.push(",");
                }
                self.push("\n");
            }
            if constructor.is_some() || !methods.is_empty() {
                self.push("\n");
            }
        }

        // Emit constructor inside struct
        if let Some(cons) = constructor {
            self.emit_class_method(&name, cons, &fields, true);
        }

        // Emit methods inside struct
        for method in &methods {
            self.emit_class_method(&name, method, &fields, false);
        }

        self.indent -= 1;
        self.emit_indent();
        self.push("};\n\n");
    }

    fn emit_class_method(&mut self, struct_name: &str, md: &MethodDefinition, fields: &[String], is_constructor: bool) {
        let prev_class = self.current_class.take();
        self.current_class = Some((struct_name.to_string(), fields.to_vec()));

        let method_name = match &md.key {
            PropertyKey::StaticIdentifier(id) => id.name.as_str().to_string(),
            _ => "unknown".to_string(),
        };
        let escaped_name = if is_constructor {
            "init".to_string()
        } else {
            Self::escape_keyword(&method_name)
        };

        let fd = &md.value;

        // Infer return type
        let ret_type = if is_constructor {
            struct_name.to_string()
        } else {
            // Try to infer return type from method body
            // If inference fails, Any.to_zig_str() returns "JsValue"
            // which is undefined in generated Zig code → compile error
            let body_ret = self.inferrer.infer_return_type_from_function_body(&fd.body);
            body_ret.to_zig_str()
        };

        self.emit_indent();
        self.push("pub fn ");
        self.push(&escaped_name);
        self.push("(");

        if is_constructor {
            // Constructor: no self param, creates instance from scratch
            // All class fields are i64, so constructor params assigned to
            // this.field also get i64 (no need for is_fallback defaults).
            self.emit_constructor_params(&fd.params);
        } else {
            // Regular method: self pointer as first param
            self.push("self: *const ");
            self.push(struct_name);
            if !fd.params.items.is_empty() {
                self.push(", ");
                self.emit_params(&fd.params, None);
            }
        }

        self.push(") ");
        self.push(&ret_type);
        self.push(" ");

        // Emit body
        if let Some(body) = &fd.body {
            self.push("{\n");
            self.indent += 1;

            if is_constructor {
                // Inject `var self: StructName = undefined;` so `this.x = val` → `self.x = val` works
                self.emit_indent();
                self.push(&format!("var self: {} = undefined;\n", struct_name));
            }

            let prev = self.in_top_level;
            self.in_top_level = false;
            for stmt in &body.statements {
                self.emit_stmt(stmt);
            }

            if is_constructor {
                // Ensure constructor returns the initialized instance
                self.emit_indent();
                self.push("return self;\n");
            }

            self.in_top_level = prev;
            self.indent -= 1;
            self.emit_indent();
            self.push("}\n\n");
        } else {
            self.push("{};\n\n");
        }

        self.current_class = prev_class;
    }

    /// Generate a Zig struct definition for a closure and return it as a string.
    fn generate_closure_struct_def(&self, ci: &ClosureInfo, arrow: &ArrowFunctionExpression) -> String {
        let mut def = String::new();
        def.push_str(&format!("const {} = struct {{\n", ci.struct_name));

        // Emit captured fields
        for (cap_name, cap_type) in &ci.captured {
            let safe_name = Self::escape_keyword(cap_name);
            def.push_str(&format!("    {}: {},\n", safe_name, cap_type));
        }
        if !ci.captured.is_empty() {
            def.push('\n');
        }

        // Emit call method signature
        def.push_str("    pub fn call(self: @This()");
        for (pname, ptype) in &ci.params {
            let safe_pname = Self::escape_keyword(pname);
            def.push_str(&format!(", {}: {}", safe_pname, ptype));
        }
        def.push_str(") ");
        def.push_str(&ci.return_type);
        def.push_str(" {\n");

        // Emit the arrow body
        if arrow.expression {
            def.push_str("        return ");
            if let Some(first) = arrow.body.statements.first() {
                match first {
                    Statement::ExpressionStatement(es) => {
                        // Emit expression, replacing captured vars with self. references
                        let expr_code = self.emit_closure_expr(&es.expression, ci);
                        def.push_str(&expr_code);
                    }
                    Statement::ReturnStatement(rs) => {
                        if let Some(arg) = &rs.argument {
                            let expr_code = self.emit_closure_expr(arg, ci);
                            def.push_str(&expr_code);
                        }
                    }
                    _ => {
                        def.push_str("/* unsupported expression */");
                    }
                }
            }
            def.push_str(";\n");
        } else {
            // Block body — just emit a placeholder for now
            def.push_str("        // multi-statement closure body\n");
            for s in &arrow.body.statements {
                let stmt_code = self.emit_closure_stmt(s, ci);
                def.push_str(&format!("        {}\n", stmt_code));
            }
        }

        def.push_str("    }\n");
        def.push_str("};\n\n");
        def
    }

    // ========== Object type helpers ==========

    /// Generate a Zig struct type definition for an anonymous object type.
    fn gen_obj_struct_def(struct_name: &str, fields: &[(String, ZigType)]) -> String {
        let mut s = format!("const {} = struct {{\n", struct_name);
        for (fname, ftype) in fields {
            s.push_str(&format!("    {}: {},\n", fname, ftype.to_zig_str()));
        }
        s.push_str("};\n\n");
        s
    }

    /// Capitalize the first letter for struct naming (e.g., "person" → "Person")
    fn capitalize_first(s: &str) -> String {
        let mut chars = s.chars();
        match chars.next() {
            None => String::new(),
            Some(c) => {
                let upper = c.to_uppercase().collect::<String>();
                upper + chars.as_str()
            }
        }
    }

    /// Emit an expression in a closure context, replacing captured vars with `self.` prefix
    fn emit_closure_expr(&self, expr: &Expression, ci: &ClosureInfo) -> String {
        let captured_names: HashSet<&str> = ci.captured.iter().map(|(n, _)| n.as_str()).collect();
        self.emit_expr_with_capture(expr, &captured_names)
    }

    /// Emit a statement in a closure context, replacing captured vars with `self.` prefix
    fn emit_closure_stmt(&self, stmt: &Statement, ci: &ClosureInfo) -> String {
        let captured_names: HashSet<&str> = ci.captured.iter().map(|(n, _)| n.as_str()).collect();
        self.emit_stmt_with_capture(stmt, &captured_names)
    }

    /// Emit an expression, replacing captured variable names with `self.` prefix
    fn emit_expr_with_capture(&self, expr: &Expression, captured: &HashSet<&str>) -> String {
        let mut buf = String::new();
        match expr {
            Expression::Identifier(id) => {
                let name = id.name.as_str();
                if captured.contains(name) {
                    buf.push_str(&format!("self.{}", Self::escape_keyword(name)));
                } else {
                    buf.push_str(&Self::escape_keyword(name));
                }
            }
            Expression::BinaryExpression(bin) => {
                buf.push_str(&self.emit_expr_with_capture(&bin.left, captured));
                buf.push(' ');
                buf.push_str(bin.operator.as_str());
                buf.push(' ');
                buf.push_str(&self.emit_expr_with_capture(&bin.right, captured));
            }
            Expression::UnaryExpression(un) => {
                buf.push_str(un.operator.as_str());
                buf.push(' ');
                buf.push_str(&self.emit_expr_with_capture(&un.argument, captured));
            }
            Expression::CallExpression(call) => {
                buf.push_str(&self.emit_expr_with_capture(&call.callee, captured));
                buf.push('(');
                for (i, arg) in call.arguments.iter().enumerate() {
                    if i > 0 {
                        buf.push_str(", ");
                    }
                    if let Some(e) = arg.as_expression() {
                        buf.push_str(&self.emit_expr_with_capture(e, captured));
                    }
                }
                buf.push(')');
            }
            Expression::ParenthesizedExpression(p) => {
                buf.push('(');
                buf.push_str(&self.emit_expr_with_capture(&p.expression, captured));
                buf.push(')');
            }
            Expression::ConditionalExpression(cond) => {
                buf.push_str(&self.emit_expr_with_capture(&cond.test, captured));
                buf.push_str(" ? ");
                buf.push_str(&self.emit_expr_with_capture(&cond.consequent, captured));
                buf.push_str(" : ");
                buf.push_str(&self.emit_expr_with_capture(&cond.alternate, captured));
            }
            Expression::NumericLiteral(n) => {
                // oxc raw is Option<Str>, fall back to formatting the value
                if let Some(raw) = &n.raw {
                    buf.push_str(raw.as_str());
                } else {
                    // Format as integer if it's a whole number
                    if n.value.fract() == 0.0 {
                        buf.push_str(&format!("{}", n.value as i64));
                    } else {
                        buf.push_str(&format!("{}", n.value));
                    }
                }
            }
            Expression::StringLiteral(s) => {
                buf.push('"');
                buf.push_str(&s.value);
                buf.push('"');
            }
            Expression::BooleanLiteral(b) => {
                buf.push_str(if b.value { "true" } else { "false" });
            }
            Expression::NullLiteral(_) => {
                buf.push_str("null");
            }
            _ => {
                // Unsupported expression type — emit placeholder
                buf.push_str("<unsupported_expr>");
            }
        }
        buf
    }

    /// Emit a statement in closure context
    fn emit_stmt_with_capture(&self, stmt: &Statement, captured: &HashSet<&str>) -> String {
        match stmt {
            Statement::ReturnStatement(rs) => {
                let mut s = String::from("return");
                if let Some(arg) = &rs.argument {
                    s.push(' ');
                    s.push_str(&self.emit_expr_with_capture(arg, captured));
                }
                s.push(';');
                s
            }
            Statement::ExpressionStatement(es) => {
                let s = self.emit_expr_with_capture(&es.expression, captured);
                format!("{};", s)
            }
            Statement::IfStatement(if_stmt) => {
                let test = self.emit_expr_with_capture(&if_stmt.test, captured);
                let cons = self.emit_stmt_with_capture(&if_stmt.consequent, captured);
                let mut s = format!("if ({}) {{ {} }}", test, cons);
                if let Some(alt) = &if_stmt.alternate {
                    let alt_code = self.emit_stmt_with_capture(alt, captured);
                    s.push_str(&format!(" else {{ {} }}", alt_code));
                }
                s
            }
            _ => format!("// TODO: {:?} in closure", std::mem::discriminant(stmt)),
        }
    }

    /// Emit a closure struct literal assignment: `const __cl_name = StructName{ .captured = value };`
    /// The `__cl_` prefix avoids Zig 0.16 "shadows declaration" errors.
    /// Also tracks `__cl_name` as a closure variable for call translation.
    fn emit_closure_var_init(&mut self, name: &str, ci: &ClosureInfo) {
        let safe_name = Self::escape_keyword(name);
        let cl_name = format!("__cl_{}", safe_name);
        self.emit_indent();
        self.push("const ");
        self.push(&cl_name);
        self.push(" = ");
        self.push(&ci.struct_name);
        self.push("{ ");
        for (cap_name, _cap_type) in &ci.captured {
            self.push(".");
            self.push(cap_name);
            self.push(" = ");
            self.push(cap_name);
            self.push(", ");
        }
        self.push("};\n");

        self.closure_vars.insert(cl_name);
    }

    fn emit_arrow_fn(&mut self, raw_name: &str, arrow: &ArrowFunctionExpression) {
        let name = Self::escape_keyword(raw_name);
        let is_async = arrow.r#async;

        self.emit_indent();
        self.push("pub fn ");
        self.push(&name);
        self.push("(");

        if is_async {
            self.push("io: Io");
            if !arrow.params.items.is_empty() {
                self.push(", ");
            }
        }
        self.emit_params(&arrow.params, Some(raw_name));
        self.push(") ");

        let ret_type = self.inferrer.get_fn_return_type(raw_name);
        // If inference fails, Any.to_zig_str() returns "JsValue"
        // which is undefined → Zig compile error
        let ret_type_str = ret_type.to_zig_str();
        if is_async {
            self.push("!");
        }
        self.push(&ret_type_str);
        self.push(" {\n");
        self.indent += 1;

        // Emit destructured parameter prelude
        for prelude in self.destructure_prelude.drain(..) {
            self.output.push_str(&prelude);
        }

        // Suppress "unused parameter" for async `io` param unless the body uses await
        if is_async
            && !arrow
                .body
                .statements
                .iter()
                .any(|s| Self::stmt_contains_await(s))
        {
            self.emit_indent();
            self.push_line("_ = io;");
        }

        let prev = self.in_top_level;
        self.in_top_level = false;

        if arrow.expression {
            self.emit_indent();
            self.push("return ");
            if let Some(first) = arrow.body.statements.first() {
                match first {
                    Statement::ExpressionStatement(es) => self.emit_expr(&es.expression),
                    Statement::ReturnStatement(rs) => {
                        if let Some(arg) = &rs.argument {
                            self.emit_expr(arg);
                        }
                    }
                    _ => self.push("/* complex expression */"),
                }
            }
            self.push(";\n");
        } else {
            for stmt in &arrow.body.statements {
                self.emit_stmt(stmt);
            }
        }

        self.in_top_level = prev;
        self.indent -= 1;
        self.push_line("}");
        self.push("\n");
    }

    fn emit_fn_from_func_expr(&mut self, name: &str, fe: &Function) {
        let escaped_name = Self::escape_keyword(name);
        let is_async = fe.r#async;

        self.emit_indent();
        self.push("pub fn ");
        self.push(&escaped_name);
        self.push("(");

        if is_async {
            self.push("io: Io");
            if !fe.params.items.is_empty() {
                self.push(", ");
            }
        }
        self.emit_params(&fe.params, Some(name));
        self.push(") ");

        let ret_type = self.inferrer.get_fn_return_type(name);
        // If inference fails, Any.to_zig_str() returns "JsValue"
        // which is undefined → Zig compile error.
        let ret_type_str = ret_type.to_zig_str();
        if is_async {
            self.push("!");
        }
        self.push(&ret_type_str);
        self.push(" ");

        if let Some(body) = &fe.body {
            self.push("{\n");
            self.indent += 1;

            // Emit destructured parameter prelude
            for prelude in self.destructure_prelude.drain(..) {
                self.output.push_str(&prelude);
            }

            // Suppress "unused parameter" for async `io` param unless the body uses await
            if is_async && !Self::body_contains_await(body) {
                self.emit_indent();
                self.push_line("_ = io;");
            }
            let prev = self.in_top_level;
            self.in_top_level = false;
            for stmt in &body.statements {
                self.emit_stmt(stmt);
            }
            self.in_top_level = prev;
            self.indent -= 1;
            self.push_line("}");
        } else {
            self.push("{};\n");
        }
        self.push("\n");
    }

    /// Emit constructor params with i64 types (class fields are always i64).
    fn emit_constructor_params(&mut self, params: &FormalParameters) {
        self.destructure_prelude.clear();

        for (i, param) in params.items.iter().enumerate() {
            if i > 0 {
                self.push(", ");
            }

            if !is_simple_binding(&param.pattern) {
                // Destructured pattern: keep Any (no field-based inference)
                let arg_name = format!("_arg{}", i);
                self.push(&arg_name);
                self.push(": ");
                self.push(&ZigType::Any.to_zig_str());

                let mut leaves = Vec::new();
                flatten_binding_pattern(&param.pattern, &arg_name, &mut leaves);
                let mut prelude = String::new();
                for leaf in &leaves {
                    let escaped = Self::escape_keyword(leaf.name);
                    let indent_str = self.get_indent_str(self.indent + 1);
                    prelude.push_str(&format!(
                        "{}const {} = {};\n",
                        indent_str, escaped, leaf.access
                    ));
                }
                if !prelude.is_empty() {
                    self.destructure_prelude.push(prelude);
                }

                if let Some(default) = &param.initializer {
                    self.push(" = ");
                    self.emit_expr(default);
                }
                continue;
            }

            let raw_name: String = self.binding_name(&param.pattern).to_owned();
            let name = Self::escape_keyword(&raw_name);
            self.push(&name);
            self.push(": ");

            // Constructor params default to i64 (matching class field types),
            // unless a default value provides a different type.
            let ty = if let Some(default) = &param.initializer {
                self.inferrer.infer_expr(default)
            } else {
                ZigType::I64
            };
            self.push(&ty.to_zig_str());

            if let Some(default) = &param.initializer {
                self.push(" = ");
                self.emit_expr(default);
            }
        }
    }

    fn emit_params(&mut self, params: &FormalParameters, fn_name: Option<&str>) {
        // Clear any previous prelude
        self.destructure_prelude.clear();

        for (i, param) in params.items.iter().enumerate() {
            if i > 0 {
                self.push(", ");
            }

            // Check if this parameter has a destructured pattern (ObjectPattern/ArrayPattern)
            if !is_simple_binding(&param.pattern) {
                // Generate a synthetic parameter name: _arg0, _arg1, etc.
                let arg_name = format!("_arg{}", i);
                self.push(&arg_name);
                self.push(": ");
                self.push(&ZigType::Any.to_zig_str());

                // Generate body prelude: unpack destructured fields
                let mut leaves = Vec::new();
                flatten_binding_pattern(&param.pattern, &arg_name, &mut leaves);
                let mut prelude = String::new();
                for leaf in &leaves {
                    let escaped = Self::escape_keyword(leaf.name);
                    let indent = self.get_indent_str(self.indent + 1);
                    prelude.push_str(&format!(
                        "{}const {} = {};\n",
                        indent, escaped, leaf.access
                    ));
                }
                if !prelude.is_empty() {
                    self.destructure_prelude.push(prelude);
                }

                // Handle param default value
                if let Some(default) = &param.initializer {
                    self.push(" = ");
                    self.emit_expr(default);
                }
                continue;
            }

            let raw_name: String = self.binding_name(&param.pattern).to_owned();
            let name = Self::escape_keyword(&raw_name);
            self.push(&name);
            self.push(": ");

            let ty = if let Some(fn_name) = fn_name {
                let param_types = self.inferrer.get_fn_param_types(fn_name);
                if i < param_types.len() {
                    // Use inferred type even if it's Any (will become "JsValue" in output)
                    param_types[i].clone()
                } else if let Some(default) = &param.initializer {
                    self.inferrer.infer_expr(default)
                } else {
                    ZigType::Any // inference failed → Zig compile error
                }
            } else if let Some(default) = &param.initializer {
                self.inferrer.infer_expr(default)
            } else {
                ZigType::Any // inference failed → Zig compile error
            };

            let type_str = if i < self.current_obj_structs.len() {
                if let Some(Some(s)) = self.current_obj_structs.get(i) {
                    s.clone()
                } else {
                    ty.to_zig_str()
                }
            } else {
                ty.to_zig_str()
            };
            self.push(&type_str);

            if let Some(default) = &param.initializer {
                self.push(" = ");
                self.emit_expr(default);
            }
        }
    }

    // --- Control flow ---

    fn emit_if_stmt(&mut self, if_stmt: &IfStatement) {
        self.emit_indent();
        self.push("if (");
        let cond = &if_stmt.test;
        let cond_ty = self.inferrer.infer_expr(cond);
        // Zig 0.16: `if (optional)` is not allowed; use `if (cond != null)` for optionals
        if matches!(cond_ty, ZigType::Optional(_)) {
            self.emit_expr(cond);
            self.push(" != null");
        } else {
            self.emit_expr(cond);
        }
        self.push(") {\n");
        self.indent += 1;
        self.emit_stmts_inside(&if_stmt.consequent);
        self.indent -= 1;

        if let Some(alt) = &if_stmt.alternate {
            self.emit_indent();
            self.push("} else ");
            self.emit_else_body(alt);
        } else {
            self.emit_indent();
            self.push("}\n");
        }
    }

    fn emit_else_body(&mut self, alt: &Statement) {
        match alt {
            Statement::IfStatement(inner) => {
                self.push("if (");
                self.emit_expr(&inner.test);
                self.push(") {\n");
                self.indent += 1;
                self.emit_stmts_inside(&inner.consequent);
                self.indent -= 1;
                if let Some(nested_alt) = &inner.alternate {
                    self.emit_indent();
                    self.push("} else ");
                    self.emit_else_body(nested_alt);
                } else {
                    self.emit_indent();
                    self.push("}\n");
                }
            }
            Statement::BlockStatement(block) => {
                self.push("{\n");
                self.indent += 1;
                for s in &block.body {
                    self.emit_stmt(s);
                }
                self.indent -= 1;
                self.emit_indent();
                self.push("}\n");
            }
            _ => {
                self.push("{\n");
                self.indent += 1;
                self.emit_stmt(alt);
                self.indent -= 1;
                self.emit_indent();
                self.push("}\n");
            }
        }
    }

    fn emit_stmts_inside(&mut self, stmt: &Statement) {
        match stmt {
            Statement::BlockStatement(block) => {
                for s in &block.body {
                    self.emit_stmt(s);
                }
            }
            _ => {
                self.emit_stmt(stmt);
            }
        }
    }

    /// Check whether an identifier `name` is referenced anywhere in the statement tree.
    /// Used to decide whether a for-loop capture needs a `_ = name;` discard.
    fn capture_used_in_body(name: &str, stmt: &Statement) -> bool {
        let mut set = HashSet::new();
        Self::collect_identifiers_in_stmt(stmt, &mut set);
        set.contains(name)
    }

    fn emit_for_stmt(&mut self, fs: &ForStatement) {
        // Translate JS `for (init; test; update) { body }` to Zig:
        //   { init; while (test) : (update) { body } }
        self.push_line("{");
        self.indent += 1;

        // Emit init before the while loop
        if let Some(init) = &fs.init {
            match init {
                ForStatementInit::VariableDeclaration(vd) => {
                    let keyword = match vd.kind {
                        VariableDeclarationKind::Const => "const",
                        _ => "var",
                    };
                    // Handle both simple and destructured declarations
                    let any_destructured = vd.declarations.iter().any(|d| !is_simple_binding(&d.id));
                    if any_destructured {
                        // Emit as individual declarations (cannot use Zig comma-separated init)
                        let saved_indent = self.indent;
                        self.indent += 1; // We're inside the { } block
                        for decl in &vd.declarations {
                            if !is_simple_binding(&decl.id) {
                                self.emit_destructured_var_decl(decl, vd.kind);
                            } else {
                                self.emit_indent();
                                self.push(keyword);
                                self.push(" ");
                                self.push(&Self::escape_keyword(self.binding_name(&decl.id)));
                                if let Some(init_expr) = &decl.init {
                                    self.push(" = ");
                                    self.emit_expr(init_expr);
                                }
                                self.push(";\n");
                            }
                        }
                        self.indent = saved_indent;
                    } else {
                        self.emit_indent();
                        self.push(keyword);
                        self.push(" ");
                        for (i, decl) in vd.declarations.iter().enumerate() {
                            if i > 0 {
                                self.push(", ");
                            }
                            self.push(&Self::escape_keyword(self.binding_name(&decl.id)));
                            if let Some(init_expr) = &decl.init {
                                self.push(" = ");
                                self.emit_expr(init_expr);
                            }
                        }
                        self.push(";\n");
                    }
                }
                _ => {
                    if let Some(expr) = init.as_expression() {
                        self.emit_indent();
                        self.push("_ = ");
                        self.emit_expr(expr);
                        self.push(";\n");
                    }
                }
            }
        }

        // Emit while (test) : (update)
        self.emit_indent();
        self.push("while (");
        if let Some(test) = &fs.test {
            self.emit_expr(test);
        } else {
            self.push("true");
        }
        self.push(")");

        if let Some(update) = &fs.update {
            self.push(" : (");
            self.emit_expr(update);
            self.push(")");
        }

        self.push(" {\n");
        self.indent += 1;
        if let Statement::BlockStatement(_) = &fs.body {
            self.emit_stmts_inside(&fs.body);
        } else {
            self.emit_stmt(&fs.body);
        }
        self.indent -= 1;
        self.push_line("}");

        // Close the outer block
        self.indent -= 1;
        self.push_line("}");
    }

    fn emit_for_in_stmt(&mut self, fis: &ForInStatement) {
        // JS for-in iterates over enumerable keys (string) of an object.
        // Zig has no direct equivalent without runtime library support
        // for key enumeration and prototype chain traversal.
        //
        // TODO: When Tier 3 runtime (js_string, js_object) is ready,
        // emit: for (jsObjectKeys(obj)) |key| { body }
        self.emit_indent();
        self.push("// TODO: for-in loop over ");
        self.emit_expr(&fis.right);
        self.push(" — requires object key enumeration runtime\n");
    }

    fn emit_for_of_stmt(&mut self, fos: &ForOfStatement) {
        // JS: for (const x of iterable) { body }
        // Zig: for (iterable) |x| { body }
        //
        // JS: for (x of iterable) { body }  (existing var)
        // Zig: for (iterable) |_item| { x = _item; body }
        //
        // JS: for await (const x of asyncIter) { body }
        // Zig: // TODO — not directly translatable
        //
        // Note: Zig 0.16 requires all for-loop captures to be used.
        // If the captured variable is not referenced in the body,
        // we emit `_ = name;` to suppress the "unused capture" error.

        if fos.r#await {
            self.emit_indent();
            self.push_line("// TODO: for-await-of — use io.async loop");
            return;
        }

        match &fos.left {
            ForStatementLeft::VariableDeclaration(vd) => {
                let first_decl = vd.declarations.first();
                match first_decl {
                    Some(decl) if !is_simple_binding(&decl.id) => {
                        // Destructured for-of: `for (const {a, b} of arr)`
                        // → `for (arr) |_item| { const a = _item.a; const b = _item.b; ... }`
                        self.emit_indent();
                        self.push("for (");
                        self.emit_expr(&fos.right);
                        self.push(") |_item| {\n");
                        self.indent += 1;

                        // Unpack destructured bindings from _item
                        let mut leaves = Vec::new();
                        flatten_binding_pattern(&decl.id, "_item", &mut leaves);
                        for leaf in &leaves {
                            if leaf.name.starts_with("test_") {
                                continue;
                            }
                            let escaped = Self::escape_keyword(leaf.name);
                            self.emit_indent();
                            self.push("const ");
                            self.push(&escaped);
                            self.push(" = ");
                            self.push(&leaf.access);
                            self.push(";\n");
                        }

                        self.emit_stmts_inside(&fos.body);
                        self.indent -= 1;
                        self.emit_indent();
                        self.push_line("}");
                    }
                    Some(decl) => {
                        // Simple identifier
                        let name_str = self.binding_name(&decl.id).to_string();
                        let cap_name = Self::escape_keyword(&name_str);
                        let used = Self::capture_used_in_body(&name_str, &fos.body);
                        self.emit_indent();
                        self.push("for (");
                        self.emit_expr(&fos.right);
                        self.push(") |");
                        self.push(&cap_name);
                        self.push("| {\n");
                        self.indent += 1;
                        if !used {
                            self.emit_indent();
                            self.push("_ = ");
                            self.push(&cap_name);
                            self.push(";\n");
                        }
                        self.emit_stmts_inside(&fos.body);
                        self.indent -= 1;
                        self.emit_indent();
                        self.push_line("}");
                    }
                    None => {
                        self.emit_indent();
                        self.push_line("// TODO: for-of with empty declaration");
                    }
                }
            }
            ForStatementLeft::AssignmentTargetIdentifier(id) => {
                let cap_name = Self::escape_keyword(&id.name);
                self.emit_indent();
                self.push("for (");
                self.emit_expr(&fos.right);
                self.push(") |_item| {\n");
                self.indent += 1;
                self.emit_indent();
                self.push(&cap_name);
                self.push(" = _item;\n");
                self.emit_stmts_inside(&fos.body);
                self.indent -= 1;
                self.emit_indent();
                self.push_line("}");
            }
            _ => {
                self.emit_indent();
                self.push_line("// TODO: for-of with member expression / destructuring");
            }
        }
    }

    fn emit_while_stmt(&mut self, ws: &WhileStatement) {
        self.emit_indent();
        self.push("while (");
        self.emit_expr(&ws.test);
        self.push(") {\n");
        self.indent += 1;
        self.emit_stmt(&ws.body);
        self.indent -= 1;
        self.push_line("}");
    }

    fn emit_do_while_stmt(&mut self, dw: &DoWhileStatement) {
        self.push_line("while (true) {");
        self.indent += 1;
        self.emit_stmt(&dw.body);
        self.emit_indent();
        self.push("if (!(");
        self.emit_expr(&dw.test);
        self.push(")) break;\n");
        self.indent -= 1;
        self.push_line("}");
    }

    fn emit_switch_stmt(&mut self, sw: &SwitchStatement) {
        self.emit_indent();
        self.push("_ = switch (");
        self.emit_expr(&sw.discriminant);
        self.push(") {\n");
        self.indent += 1;
        for case in &sw.cases {
            match &case.test {
                Some(test) => {
                    self.emit_indent();
                    // Numeric literals don't use `.` prefix in Zig switch
                    if matches!(test, Expression::NumericLiteral(_)) {
                        self.emit_expr(test);
                    } else {
                        self.push(".");
                        self.emit_expr(test);
                    }
                    self.push(" => {\n");
                }
                None => {
                    self.emit_indent();
                    self.push("else => {\n");
                }
            }
            self.indent += 1;
            for s in &case.consequent {
                // Skip `break` inside switch cases (Zig cases implicitly break)
                if matches!(s, Statement::BreakStatement(_)) {
                    continue;
                }
                self.emit_stmt(s);
            }
            self.indent -= 1;
            self.emit_indent();
            self.push("},\n");
        }
        self.indent -= 1;
        self.emit_indent();
        self.push("};\n");
    }

    fn emit_try_stmt(&mut self, ts: &TryStatement) {
        // JS try-catch-finally → Zig error union + defer block.
        //
        // JS:  try { body } catch (e) { catch_body } finally { finally_body }
        // Zig: defer { finally_body }
        //      const _try_result = _tryN: { body } catch |e| _catchN: { _ = e; catch_body };
        //      _ = _try_result;
        //
        // Zig's `defer` runs when the enclosing scope exits, whether by normal completion,
        // error propagation, or return. This matches JS finally semantics.
        //
        // throw inside try  → break :_tryN error.Unexpected
        // return inside try → break :_tryN value
        // return inside catch → break :_catchN value
        //
        // Limitation: return/throw inside the finally block are not intercepted,
        // as Zig defer bodies cannot alter control flow of the deferred scope.
        let try_label = format!("_try{}", self.try_counter);
        let catch_label = format!("_catch{}", self.try_counter);
        self.try_counter += 1;

        // Emit defer with finally body (before try-catch so it runs after both try and catch)
        if let Some(ref finalizer) = ts.finalizer {
            self.emit_indent();
            self.push("defer {\n");
            self.indent += 1;
            for s in &finalizer.body {
                self.emit_stmt(s);
            }
            self.indent -= 1;
            self.emit_indent();
            self.push("}\n");
        }

        // Enter try block
        self.try_label = Some(try_label.clone());

        self.emit_indent();
        // If the enclosing function has a non-void return type AND there is no
        // finalizer (finally), emit `return` so the try-catch value becomes the
        // function's return value. With a finalizer, defer {} captures cleanup
        // and there may be trailing statements after the try-catch-finally.
        let use_return = ts.finalizer.is_none()
            && self.current_fn.as_ref().map(|fn_name| {
                let ret = self.inferrer.get_fn_return_type(fn_name);
                ret != ZigType::Void
            }).unwrap_or(false);
        if use_return {
            self.push("return ");
        } else {
            self.push("_ = ");
        }
        self.push(&format!("{}: {{", try_label));
        self.push("\n");
        self.indent += 1;

        for s in &ts.block.body {
            self.emit_stmt(s);
        }

        self.try_label = None;
        self.indent -= 1;

        // Enter catch block
        self.emit_indent();
        self.push(&format!("\n}} catch {}: {{", catch_label));
        self.push("\n");
        self.indent += 1;

        self.catch_label = Some(catch_label.clone());
        if let Some(handler) = &ts.handler {
            for s in &handler.body.body {
                self.emit_stmt(s);
            }
        }
        self.catch_label = None;

        self.indent -= 1;
        self.emit_indent();
        self.push("};\n");

    }

    // ========== Expressions ==========

    fn emit_expr(&mut self, expr: &Expression) {
        match expr {
            Expression::NumericLiteral(lit) => {
                // Use raw source text when available (preserves hex/base), fallback to value
                if let Some(raw) = &lit.raw {
                    self.push(raw);
                } else if lit.value.fract() == 0.0 {
                    self.push(&format!("{}", lit.value as i64));
                } else {
                    self.push(&format!("{}", lit.value));
                }
            }

            Expression::StringLiteral(lit) => {
                self.push("\"");
                self.push(&lit.value);
                self.push("\"");
            }

            Expression::BooleanLiteral(lit) => {
                self.push(if lit.value { "true" } else { "false" });
            }

            Expression::NullLiteral(_) => {
                self.push("null");
            }

            Expression::BigIntLiteral(lit) => {
                if let Some(raw) = &lit.raw {
                    self.push(raw);
                } else {
                    self.push(&lit.value);
                }
            }

            Expression::Identifier(id) => {
                // Built-in global constants
                match id.name.as_str() {
                    "NaN" => { self.push("std.math.nan(f64)"); return; }
                    "Infinity" => { self.push("std.math.inf(f64)"); return; }
                    _ => {}
                }
                self.push(&Self::escape_keyword(id.name.as_str()));
            }

            Expression::ThisExpression(_) => {
                self.push("self");
            },

            Expression::BinaryExpression(bin) => {
                // Handle special operators that don't map 1:1 to Zig
                if bin.operator == BinaryOperator::Exponential {
                    // JS `**` is exponentiation; Zig's `**` is array repetition
                    self.push("std.math.pow(f64, @floatFromInt(");
                    self.emit_expr(&bin.left);
                    self.push("), @floatFromInt(");
                    self.emit_expr(&bin.right);
                    self.push("))");
                    return;
                }
                if bin.operator == BinaryOperator::Addition {
                    let left_ty = self.inferrer.infer_expr(&bin.left);
                    let right_ty = self.inferrer.infer_expr(&bin.right);
                    if left_ty == ZigType::String || right_ty == ZigType::String {
                        // Check if both operands are string literals (comptime concat)
                        let left_is_lit = Self::is_string_literal_expr(&bin.left);
                        let right_is_lit = Self::is_string_literal_expr(&bin.right);
                        if left_is_lit && right_is_lit {
                            self.emit_expr(&bin.left);
                            self.push(" ++ ");
                            self.emit_expr(&bin.right);
                        } else {
                            // Runtime string concat: use allocPrint with page_allocator
                            // Produces: std.fmt.allocPrint(js_allocator.g_alloc(), "{s}{s}", .{a, b}) catch @panic("OOM")
                            self.push("std.fmt.allocPrint(js_allocator.g_alloc(), \"{s}{s}\", .{ ");
                            self.emit_expr(&bin.left);
                            self.push(", ");
                            self.emit_expr(&bin.right);
                            self.push(" }) catch @panic(\"OOM\")");
                        }
                        return;
                    }
                }
                self.emit_expr(&bin.left);
                self.push(" ");
                self.push(self.map_binary_op(&bin.operator));
                self.push(" ");
                // Zig shift amount must be unsigned: i64 << n requires n: u6
                if bin.operator == BinaryOperator::ShiftLeft
                    || bin.operator == BinaryOperator::ShiftRight
                    || bin.operator == BinaryOperator::ShiftRightZeroFill
                {
                    self.push("@as(u6, @intCast(");
                    self.emit_expr(&bin.right);
                    self.push("))");
                } else {
                    self.emit_expr(&bin.right);
                }
            }

            Expression::LogicalExpression(logic) => {
                self.emit_expr(&logic.left);
                self.push(" ");
                self.push(self.map_logical_op(&logic.operator));
                self.push(" ");
                self.emit_expr(&logic.right);
            },

            Expression::UnaryExpression(unary) => {
                match unary.operator {
                    UnaryOperator::Typeof => {
                        self.push("@TypeOf(");
                        self.emit_expr(&unary.argument);
                        self.push(")");
                    }
                    UnaryOperator::Void | UnaryOperator::Delete => {
                        self.emit_expr(&unary.argument);
                    }
                    UnaryOperator::UnaryPlus => {
                        // Zig has no unary plus — emit argument as-is
                        self.emit_expr(&unary.argument);
                    }
                    UnaryOperator::BitwiseNot => {
                        // ~ on comptime_int needs explicit type cast
                        self.push("~@as(i64, ");
                        self.emit_expr(&unary.argument);
                        self.push(")");
                    }
                    _ => {
                        self.push(self.map_unary_op(&unary.operator));
                        self.push(" ");
                        self.emit_expr(&unary.argument);
                    }
                }
            }

            Expression::UpdateExpression(update) => {
                let op = match update.operator {
                    UpdateOperator::Increment => "+=",
                    UpdateOperator::Decrement => "-=",
                };
                self.emit_assign_target_from_simple(&update.argument);
                self.push(" ");
                self.push(op);
                self.push(" 1");
            }

            Expression::CallExpression(call) => {
                // Check builtin registry for known method/global calls
                if self.try_emit_builtin_call(call) {
                    // handled by builtin registry
                } else {
                    // Check if callee is a closure variable → emit __cl_callee.call(args)
                    let maybe_cl_name =
                        if let Expression::Identifier(callee_id) = &call.callee {
                            let cl_name = format!("__cl_{}", callee_id.name.as_str());
                            if self.closure_vars.contains(&cl_name) {
                                Some(cl_name)
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                    if let Some(cl_name) = maybe_cl_name {
                        self.push(&cl_name);
                        self.push(".call(");
                    } else {
                        // Default: emit callee(args)
                        self.emit_expr(&call.callee);
                        self.push("(");
                    }
                    for (i, arg) in call.arguments.iter().enumerate() {
                        if i > 0 {
                            self.push(", ");
                        }
                        self.emit_arg(arg);
                    }
                    self.push(")");
                }
            }

            Expression::NewExpression(ne) => {
                // Check for built-in constructors (Map, Set, etc.)
                if let Expression::Identifier(id) = &ne.callee {
                    match id.name.as_str() {
                        "Map" => {
                            self.push("js_map.JsMap.init(js_allocator.g_alloc())");
                            return;
                        }
                        "Set" => {
                            self.push("js_set.JsSet.init(js_allocator.g_alloc())");
                            return;
                        }
                        _ => {}
                    }
                }
                
                // Default: new ClassName(args) → ClassName.init(args)
                self.emit_expr(&ne.callee);
                self.push(".init(");
                for (i, arg) in ne.arguments.iter().enumerate() {
                    if i > 0 {
                        self.push(", ");
                    }
                    self.emit_arg(arg);
                }
                self.push(")");
            }

            Expression::StaticMemberExpression(mem) => {
                // Check if object is a dynamic access variable (uses HashMap)
                let is_dynamic = if let Expression::Identifier(id) = &mem.object {
                    self.inferrer.get_dynamic_access_vars().contains(id.name.as_str())
                } else {
                    false
                };

                if is_dynamic {
                    // Look up field type from the original object type to pick
                    // the correct JsValue variant accessor.
                    self.emit_expr(&mem.object);
                    self.push(".get(\"");
                    self.push(mem.property.name.as_str());
                    self.push("\").?");
                    let accessor = self.dynamic_field_accessor(&mem.object, mem.property.name.as_str());
                    self.push(&accessor);
                    return;
                }
                
                // Map JS .length to Zig .len for arrays and strings
                if mem.property.name.as_str() == "length" {
                    // Dynamic arrays (ArrayList): use .items.len
                    if let Expression::Identifier(id) = &mem.object
                        && self.inferrer.is_dynamic_array(id.name.as_str())
                    {
                        self.emit_expr(&mem.object);
                        self.push(".items.len");
                        return;
                    }
                    let obj_ty = self.inferrer.infer_expr(&mem.object);
                    if obj_ty == ZigType::String || matches!(obj_ty, ZigType::Array(_)) {
                        self.emit_expr(&mem.object);
                        self.push(".len");
                        return;
                    }
                }

                // Check builtin static properties (e.g., Math.PI → std.math.pi)
                if let Expression::Identifier(id) = &mem.object
                    && let Some(zig_expr) = self.builtins.lookup_property(id.name.as_str(), mem.property.name.as_str()) {
                        self.push(zig_expr);
                        return;
                    }

                self.emit_expr(&mem.object);
                self.push(".");
                self.push(mem.property.name.as_str());
            }

            Expression::ComputedMemberExpression(mem) => {
                // Check if object is a dynamic array (ArrayList)
                // Distinguish: function params with slice type use direct indexing;
                // locally-declared dynamic arrays use .items[...]
                if let Expression::Identifier(id) = &mem.object
                    && self.inferrer.is_dynamic_array(id.name.as_str())
                {
                    // Check if this variable is a parameter of the CURRENT function.
                    // If yes, it's a slice — use direct indexing.
                    // If no, it's a locally-declared ArrayList — use .items[...].
                    let is_current_fn_param = self.current_fn.as_ref()
                        .map(|fn_name| self.inferrer.is_fn_param_of(fn_name, id.name.as_str()))
                        .unwrap_or(false);
                    if is_current_fn_param {
                        self.emit_expr(&mem.object);
                        self.push("[");
                        self.emit_expr(&mem.expression);
                        self.push("]");
                        return;
                    }
                    // Locally-declared ArrayList - use .items[...]
                    self.emit_expr(&mem.object);
                    self.push(".items[");
                    self.emit_expr(&mem.expression);
                    self.push("]");
                    return;
                }

                // Check if object is a dynamic access variable (uses HashMap)
                let is_dynamic = if let Expression::Identifier(id) = &mem.object {
                    self.inferrer.get_dynamic_access_vars().contains(id.name.as_str())
                } else {
                    false
                };

                if is_dynamic {
                    // Generate: object.get(key).?
                    self.emit_expr(&mem.object);
                    self.push(".get(");
                    self.emit_expr(&mem.expression);
                    self.push(").?");
                    return;
                }

                // Check if object is a struct type and key is a string literal
                let obj_type = self.inferrer.infer_expr(&mem.object);
                if matches!(&obj_type, ZigType::Object { .. })
                    && let Expression::StringLiteral(s) = &mem.expression
                {
                    // String literal key → direct field access
                    self.emit_expr(&mem.object);
                    self.push(".");
                    self.push(s.value.as_str());
                    return;
                }

                // Check if object is an array type (ZigType::Array) - use direct indexing for slices
                if matches!(&obj_type, ZigType::Array(_)) {
                    self.emit_expr(&mem.object);
                    self.push("[");
                    self.emit_expr(&mem.expression);
                    self.push("]");
                    return;
                }

                // Fall through: string indexing or other cases
                self.emit_expr(&mem.object);
                self.push("[");
                self.emit_expr(&mem.expression);
                self.push("]");
            }

            Expression::PrivateFieldExpression(_) => {
                self.push("// TODO: private field");
            }

            Expression::AssignmentExpression(assign) => {
                // Check if assigning to a field of a dynamic access object (HashMap)
                if let AssignmentTarget::StaticMemberExpression(mem) = &assign.left {
                    if let Expression::Identifier(obj_id) = &mem.object {
                        let obj_name = obj_id.name.as_str();
                        let dyn_vars = self.inferrer.get_dynamic_access_vars();
                        if dyn_vars.contains(obj_name) {
                            // Generate: obj.put("field", JsValue{...}) catch @panic("OOM");
                            self.emit_expr(&mem.object);
                            self.push(".put(\"");
                            self.push(mem.property.name.as_str());
                            self.push("\", ");
                            self.emit_js_value_construction(&assign.right);
                            self.push(") catch @panic(\"OOM\")");
                            return;
                        }
                    }
                }

                self.emit_assign_target(&assign.left);
                self.push(" ");
                self.push(self.map_assign_op(&assign.operator));
                self.push(" ");
                self.emit_expr(&assign.right);
            }

            Expression::ConditionalExpression(cond) => {
                self.push("if (");
                self.emit_expr(&cond.test);
                self.push(") ");
                self.emit_expr(&cond.consequent);
                self.push(" else ");
                self.emit_expr(&cond.alternate);
            }

            Expression::ArrayExpression(arr) => {
                let elem_type = if arr.elements.is_empty() {
                    ZigType::I64  // empty array → default to i64
                } else {
                    arr.elements.iter().find_map(|elem| match elem {
                        ArrayExpressionElement::SpreadElement(_) => None,
                        ArrayExpressionElement::Elision(_) => None,
                        _ => elem.as_expression().map(|e| self.inferrer.infer_expr(e)),
                    }).unwrap_or(ZigType::Any)  // inference failed → Zig compile error
                };
                // If inference failed, elem_type = Any → "JsValue" → Zig compile error
                self.push(&format!("[_]{}{{", elem_type.to_zig_str()));
                for (i, elem) in arr.elements.iter().enumerate() {
                    if i > 0 {
                        self.push(", ");
                    }
                    self.emit_array_element(elem);
                }
                self.push("}");
            },

            Expression::ObjectExpression(obj) => {
                // Collect properties into categories for spread handling
                let mut normal_props: Vec<(&ObjectProperty, String, &Expression)> = Vec::new();
                let mut spread_props: Vec<&SpreadElement> = Vec::new();

                for prop in &obj.properties {
                    match prop {
                        ObjectPropertyKind::ObjectProperty(p) => {
                            if matches!(p.value, Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_)) {
                                continue;
                            }
                            let key_str = property_key_name(&p.key);
                            normal_props.push((p, key_str, &p.value));
                        }
                        ObjectPropertyKind::SpreadProperty(spread) => {
                            spread_props.push(spread);
                        }
                    }
                }

                // Case 1: Pure spread { ...expr } — just pass through the expression
                if normal_props.is_empty() && spread_props.len() == 1 {
                    self.emit_expr(&spread_props[0].argument);
                    return;
                }

                // Case 2: Spread with property overrides — copy then mutate
                // { a: 1, ...base, c: 3 }  →  var _tmp = base; _tmp.a = 1; _tmp.c = 3; _tmp
                if spread_props.len() == 1 {
                    self.push("(blk: {\n");
                    self.indent += 1;

                    // Always use @TypeOf(base) to get the correct type, whether it's a
                    // named struct (e.g., Base_objects) or an anonymous struct.
                    self.emit_indent();
                    self.push("var _tmp: @TypeOf(");
                    self.emit_expr(&spread_props[0].argument);
                    self.push(") = ");
                    self.emit_expr(&spread_props[0].argument);
                    self.push(";\n");
                    for (_p, key_str, val_expr) in &normal_props {
                        self.emit_indent();
                        self.push("_tmp.");
                        self.push(key_str);
                        self.push(" = ");
                        self.emit_expr(val_expr);
                        self.push(";\n");
                    }
                    self.emit_indent();
                    self.push("break :blk _tmp;\n");
                    self.indent -= 1;
                    self.emit_indent();
                    self.push("})");
                    return;
                }

                // Case 3: Multiple spreads — not supported in Zig (no dynamic struct merging)
                if spread_props.len() > 1 {
                    self.push("@compileError(\"object spread with multiple sources is not supported — use field-by-field assignment instead\")");
                    return;
                }

                // Case 0: No spreads — normal struct literal emission
                self.push(".{ ");
                let mut first = true;
                for (_, key_str, val_expr) in &normal_props {
                    if !first {
                        self.push(", ");
                    }
                    first = false;
                    self.push(".");
                    self.push(key_str);
                    self.push(" = ");
                    self.emit_expr(val_expr);
                }
                self.push(" }");
            },

            Expression::TemplateLiteral(tl) => {
                // Emit template literals as string literals when possible
                if tl.expressions.is_empty()
                    && let Some(quasi) = tl.quasis.first()
                        && let Some(cooked) = &quasi.value.cooked {
                            self.push("\"");
                            self.push(cooked.as_ref());
                            self.push("\"");
                            return;
                        }

                // Template literal with expressions: use std.fmt.allocPrint
                // e.g. `hello ${name}, you are ${age}` →
                //   std.fmt.allocPrint(js_allocator.g_alloc(), "hello {}{}!", .{ name, age }) catch @panic("OOM")
                self.push("std.fmt.allocPrint(js_allocator.g_alloc(), \"");
                // Build format string
                for (i, quasi) in tl.quasis.iter().enumerate() {
                    if let Some(cooked) = &quasi.value.cooked {
                        self.push(cooked.as_ref());
                    }
                    if i < tl.expressions.len() {
                        self.push("{}");
                    }
                }
                self.push("\", .{ ");
                for (i, expr) in tl.expressions.iter().enumerate() {
                    if i > 0 {
                        self.push(", ");
                    }
                    self.emit_expr(expr);
                }
                self.push(" }) catch @panic(\"OOM\")");
            }

            Expression::TaggedTemplateExpression(_) => {
                self.push("// TODO: tagged template");
            }

            Expression::ArrowFunctionExpression(arrow) => {
                // Look up by span start — covers return closures, callbacks, and assignments.
                // Struct definitions are already pre-generated during pre_scan_closures
                // and buffered in closure_structs for output after all functions.
                if let Some(ci) = self.closure_map.get(&arrow.span.start).cloned() {
                    // Emit struct literal only: ClosureName{ .cap1 = cap1, .cap2 = cap2 }
                    self.push(&ci.struct_name);
                    self.push("{ ");
                    for (i, (cap_name, _)) in ci.captured.iter().enumerate() {
                        if i > 0 {
                            self.push(", ");
                        }
                        self.push(".");
                        self.push(cap_name);
                        self.push(" = ");
                        self.push(cap_name);
                    }
                    self.push(" }");
                    return;
                }
                self.push("(@compileError(\"inline arrow function not yet supported - rewrite JS to use named functions\"))");
            }

            Expression::FunctionExpression(_) => {
                self.push("(@compileError(\"inline function not yet supported - rewrite JS to use named functions\"))");
            }

            Expression::AwaitExpression(ae) => {
                let task_var = format!("_t{}", self.task_counter);
                self.task_counter += 1;

                // emit: (blk: { var _tN = io.async(fn, .{io, args...}); defer _tN.cancel(io) catch {}; break :blk try _tN.await(io); })
                self.push("(blk: {\n");
                self.indent += 1;
                self.emit_indent();
                self.push("var ");
                self.push(&task_var);
                self.push(" = io.async(");

                // Extract the function and arguments from the await argument
                match &ae.argument {
                    Expression::CallExpression(call) => {
                        self.emit_expr(&call.callee);
                        self.push(", .{ io");
                        for arg in &call.arguments {
                            self.push(", ");
                            if let Some(expr) = arg.as_expression() {
                                self.emit_expr(expr);
                            } else {
                                self.push("undefined");
                            }
                        }
                        self.push(" });\n");
                    }
                    _ => {
                        // await non-call expression: treat as io.async(expr, .{io})
                        self.emit_expr(&ae.argument);
                        self.push(", .{ io });\n");
                    }
                }

                self.emit_indent();
                self.push("defer ");
                self.push(&task_var);
                self.push(".cancel(io) catch {};\n");

                self.emit_indent();
                self.push("break :blk try ");
                self.push(&task_var);
                self.push(".await(io);\n");

                self.indent -= 1;
                self.emit_indent();
                self.push("})");
            }

            Expression::ParenthesizedExpression(parens) => {
                self.push("(");
                self.emit_expr(&parens.expression);
                self.push(")");
            }

            Expression::SequenceExpression(seq) => {
                for (i, expr) in seq.expressions.iter().enumerate() {
                    if i > 0 {
                        self.push(", ");
                    }
                    self.emit_expr(expr);
                }
            }

            Expression::ChainExpression(chain) => {
                match &chain.expression {
                    ChainElement::CallExpression(call) => {
                        self.emit_expr(&call.callee);
                        self.push("(");
                        for (i, arg) in call.arguments.iter().enumerate() {
                            if i > 0 { self.push(", "); }
                            self.emit_arg(arg);
                        }
                        self.push(")");
                    }
                    ChainElement::StaticMemberExpression(mem) => {
                        self.emit_expr(&mem.object);
                        self.push(".");
                        self.push(mem.property.name.as_str());
                    }
                    ChainElement::ComputedMemberExpression(mem) => {
                        let obj_type = self.inferrer.infer_expr(&mem.object);
                        if matches!(&obj_type, ZigType::Object { .. })
                            && let Expression::StringLiteral(s) = &mem.expression
                        {
                            self.emit_expr(&mem.object);
                            self.push(".");
                            self.push(s.value.as_str());
                        } else {
                            self.emit_expr(&mem.object);
                            self.push("[");
                            self.emit_expr(&mem.expression);
                            self.push("]");
                        }
                    }
                    _ => {
                        self.push("/* chain element */");
                    }
                }
            }

            Expression::ClassExpression(_) => {
                self.push("// TODO: class expression");
            }

            Expression::MetaProperty(_) => {
                self.push("// TODO: meta property");
            }

            Expression::ImportExpression(_) => {
                self.push("// TODO: dynamic import");
            }

            Expression::Super(_) => {
                self.push("super");
            }

            Expression::RegExpLiteral(lit) => {
                // Extract pattern from raw source (e.g., "/world/" → "world", "/zig/g" → "zig")
                let pattern = if let Some(ref raw) = lit.raw {
                    let raw_str = raw.as_str();
                    if let Some(inner) = raw_str.strip_prefix('/') {
                        if let Some(end) = inner.rfind('/') {
                            &inner[..end]
                        } else {
                            inner
                        }
                    } else {
                        raw_str
                    }
                } else {
                    ""
                };
                self.push("\"");
                self.push(pattern);
                self.push("\"");
            }

            Expression::TSAsExpression(ts) => {
                let ty = self.inferrer.infer_expr(&ts.expression);
                self.push("@as(");
                self.push(&ty.to_zig_str());
                self.push(", ");
                self.emit_expr(&ts.expression);
                self.push(")");
            }

            Expression::TSTypeAssertion(ts) => {
                let ty = self.inferrer.infer_expr(&ts.expression);
                self.push("@as(");
                self.push(&ty.to_zig_str());
                self.push(", ");
                self.emit_expr(&ts.expression);
                self.push(")");
            }

            Expression::TSNonNullExpression(ts) => {
                self.emit_expr(&ts.expression);
                self.push(".?");
            }

            Expression::TSSatisfiesExpression(ts) => {
                self.emit_expr(&ts.expression);
            }

            Expression::TSInstantiationExpression(ts) => {
                self.emit_expr(&ts.expression);
            }

            Expression::YieldExpression(_) => {
                self.push("// TODO: yield");
            }

            Expression::V8IntrinsicExpression(_) => {
                self.push("// TODO: V8 intrinsic");
            }

            Expression::PrivateInExpression(_) => {
                self.push("// TODO: private in");
            }

            Expression::JSXElement(_) | Expression::JSXFragment(_) => {
                self.push("// TODO: JSX");
            }
        }
    }

    // ========== Builtin Call Helpers ==========

    /// Try to emit a call via the builtin registry.
    /// Returns true if the call was handled.
    fn try_emit_builtin_call(&mut self, call: &CallExpression) -> bool {
        // Case 1: obj.method(args) — StaticMemberExpression callee
        if let Expression::StaticMemberExpression(mem) = &call.callee {
            let obj_expr = &mem.object;
            let method_name = mem.property.name.as_str();

            // Dynamic array methods: use ArrayList directly (before any lookup)
            if let Expression::Identifier(id) = obj_expr
                && self.inferrer.is_dynamic_array(id.name.as_str()) {
                    self.emit_dynamic_array_method(id.name.as_str(), method_name, &call.arguments);
                    return true;
                }

            // ── Namespace lookup (Math.abs, console.log, Object.keys, …) ──
            // Use the object's identifier name (e.g. "Math", "console", "Object")
            if let Expression::Identifier(id) = obj_expr
                && let Some(trans) = self.builtins.lookup_method(id.name.as_str(), method_name) {
                    // Namespace call: template already bakes in the receiver.
                    // e.g. template "js_console.log({})" → just pass call arguments.
                    self.apply_builtin_template(trans, &call.arguments);
                    return true;
                }

                // ── Type-based lookup (arr.indexOf, str.toUpperCase, …) ──
                // e.g.  arr.indexOf(42) → key "array", template "js_array.indexOf({}, {})"
                let obj_ty = self.inferrer.infer_expr(obj_expr);
                if let Some(type_key) = Self::type_to_builtin_key(&obj_ty)
                    && let Some(trans) = self.builtins.lookup_method(type_key, method_name) {
                        // Type-based call: template expects receiver as {0}.
                        self.emit_builtin_method_call(trans, obj_expr, &call.arguments);
                        return true;
                    }

                // ── Regexp dispatch (re.test(str), re.exec(str)) ──
                // Simplified: regexp literals are emitted as pattern strings,
                // so re.test(str) → js_regexp.test_(str, re)
                if method_name == "test" {
                    self.push("js_regexp.test_(");
                    if let Some(arg0) = call.arguments.first() {
                        self.emit_arg(arg0);
                    }
                    self.push(", ");
                    self.emit_expr(obj_expr);
                    self.push(")");
                    return true;
                }
                if method_name == "exec" {
                    self.push("(js_regexp.exec(js_allocator.g_alloc(), ");
                    if let Some(arg0) = call.arguments.first() {
                        self.emit_arg(arg0);
                    }
                    self.push(", ");
                    self.emit_expr(obj_expr);
                    self.push(") catch null)");
                    // Also emit a follow-up if-block for the caller
                    // This is handled by the caller's if-clause in JS
                    return true;
                }
            }

        // Case 2: globalFunc(args) — Identifier callee
        if let Expression::Identifier(id) = &call.callee
            && let Some(trans) = self.builtins.lookup_global(id.name.as_str())
        {
            self.apply_builtin_template(trans, &call.arguments);
            return true;
        }

        false
    }

    /// Map a ZigType to a builtin lookup key ("array", "string", etc.)
    fn type_to_builtin_key(ty: &ZigType) -> Option<&'static str> {
        match ty {
            ZigType::String => Some("string"),
            ZigType::Array(_) => Some("array"),
            ZigType::Object { .. } => Some("object"),
            ZigType::Struct(s) => {
                match s.as_str() {
                    "Map" => Some("map"),
                    "Set" => Some("set"),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Emit a builtin method call, handling the receiver object.
    /// The template may use {} (all args) or {0}, {1} (positional).
    /// For type-dispatched calls, the receiver is implicitly arg 0.
    fn emit_builtin_method_call(
        &mut self,
        trans: &crate::builtins::BuiltinTranslation,
        receiver: &Expression,
        args: &oxc_allocator::Vec<'_, Argument>,
    ) {
        let template = &trans.template;

        // Check if template starts with a runtime function that needs allocator
        // e.g., "js_array.indexOf({}, {})" — receiver goes into {}
        // We need to replace {} with "receiver, arg0, arg1..."
        // and {0}, {1} with positional args

        // Collect all arg strings (receiver + call args)
        let mut all_args: Vec<String> = Vec::new();
        let empty_exports = std::collections::HashSet::new();
        let mut tmp = ZigCodegen {
            output: String::new(),
            indent: self.indent,
            inferrer: TypeInferrer::new(),
            diagnostics: &mut Vec::new(),
            in_top_level: self.in_top_level,
            task_counter: self.task_counter,
            builtins: self.builtins,
            closure_map: std::collections::HashMap::new(),
            closure_struct_defs: std::collections::HashMap::new(),
            fn_closure_spans: std::collections::HashMap::new(),
            closure_counter: 0,
            closure_structs: Vec::new(),
            cabi_wrappers: Vec::new(),
            cabi_exports: Vec::new(),
            string_return_fns: std::collections::HashSet::new(),
            closure_vars: std::collections::HashSet::new(),
            current_fn: None,
            exports: empty_exports,
            try_label: None,
            catch_label: None,
            try_counter: self.try_counter,
            temp_counter: 0,
            destructure_prelude: Vec::new(),
            current_class: None,
            object_type_defs: Vec::new(),
            current_obj_structs: Vec::new(),
            init_globals_code: Vec::new(),
        };
        tmp.emit_expr(receiver);
        all_args.push(tmp.output.clone());

        for arg in args.iter() {
            let mut tmp2 = ZigCodegen {
                output: String::new(),
                indent: self.indent,
                inferrer: TypeInferrer::new(),
                diagnostics: &mut Vec::new(),
                in_top_level: self.in_top_level,
                task_counter: self.task_counter,
                builtins: self.builtins,
                closure_map: std::collections::HashMap::new(),
                closure_struct_defs: std::collections::HashMap::new(),
                fn_closure_spans: std::collections::HashMap::new(),
                closure_counter: 0,
                closure_structs: Vec::new(),
                cabi_wrappers: Vec::new(),
                cabi_exports: Vec::new(),
                string_return_fns: std::collections::HashSet::new(),
                closure_vars: std::collections::HashSet::new(),
                current_fn: None,
                exports: std::collections::HashSet::new(),
                try_label: None,
                catch_label: None,
                try_counter: self.try_counter,
                temp_counter: 0,
                destructure_prelude: Vec::new(),
                current_class: None,
                object_type_defs: Vec::new(),
                current_obj_structs: Vec::new(),
                init_globals_code: Vec::new(),
            };
            tmp2.emit_arg(arg);
            all_args.push(tmp2.output.clone());
        }

        // Now apply template: {} = all_args joined, {0} = all_args[0], etc.
        let mut result = String::new();
        let mut chars = template.chars().peekable();
        let all_args_ref: Vec<&str> = all_args.iter().map(|s| s.as_str()).collect();
        while let Some(ch) = chars.next() {
            if ch == '{' {
                if let Some(&('0'..='9')) = chars.peek() {
                    let mut idx_str = String::new();
                    while let Some(&('0'..='9')) = chars.peek() {
                        idx_str.push(chars.next().unwrap());
                    }
                    if chars.peek() == Some(&'}') {
                        chars.next();
                    }
                    if let Ok(idx) = idx_str.parse::<usize>()
                        && let Some(arg) = all_args_ref.get(idx) {
                            result.push_str(arg);
                        }
                } else if chars.peek() == Some(&'}') {
                    chars.next();
                    result.push_str(&all_args_ref.join(", "));
                } else {
                    result.push(ch);
                }
            } else {
                result.push(ch);
            }
        }
        self.push(&result);
    }

    /// Emit direct ArrayList method calls for dynamic arrays
    /// (instead of going through js_array runtime functions).
    fn emit_dynamic_array_method(
        &mut self,
        obj_name: &str,
        method: &str,
        args: &oxc_allocator::Vec<'_, Argument>,
    ) {
        let escaped = Self::escape_keyword(obj_name);

        match method {
            "push" => {
                // arr.push(val) → arr.append(js_allocator.g_alloc(), val) catch {};
                // Zig 0.16: do NOT return the new length (blk expression return value ignored error)
                self.push(&escaped);
                self.push(".append(js_allocator.g_alloc(), ");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { self.push(", "); }
                    self.emit_arg(arg);
                }
                self.push(") catch {}");
            }
            "pop" => {
                self.push(&escaped);
                self.push(".pop() orelse null");
            }
            "shift" => {
                self.push("(blk: { if (");
                self.push(&escaped);
                self.push(".items.len == 0) break :blk @as(?i64, null); break :blk ");
                self.push(&escaped);
                self.push(".orderedRemove(0); })");
            }
            "unshift" => {
                // arr.unshift(val) → arr.insert(js_allocator.g_alloc(), 0, val) catch {};
                // Zig 0.16: do NOT return the new length
                self.push(&escaped);
                self.push(".insert(js_allocator.g_alloc(), 0, ");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { self.push(", "); }
                    self.emit_arg(arg);
                }
                self.push(") catch {}");
            }
            "splice" | "sort" | "reverse" => {
                self.push("@compileError(\"");
                self.push(method);
                self.push(" not yet implemented for dynamic array\")");
            }
            _ => {
                self.push("@compileError(\"unknown array method: ");
                self.push(method);
                self.push("\")");
            }
        }
    }

    /// Apply a builtin template by splitting on "{}" placeholders.
    fn apply_builtin_template(
        &mut self,
        trans: &crate::builtins::BuiltinTranslation,
        args: &oxc_allocator::Vec<'_, Argument>,
    ) {
        let template = &trans.template;
        // Collect formatted arguments
        let formatted_args: Vec<String> = args
            .iter()
            .map(|arg| {
                // Use a temp codegen to format the arg
                let empty_exports = HashSet::new();
                let mut tmp = ZigCodegen {
                    output: String::new(),
                    indent: self.indent,
                    inferrer: TypeInferrer::new(), // dummy, not used for emit_arg
                    diagnostics: &mut Vec::new(),
                    in_top_level: self.in_top_level,
                    task_counter: self.task_counter,
                    builtins: self.builtins,
                    closure_map: HashMap::new(),
                    closure_struct_defs: HashMap::new(),
                    fn_closure_spans: HashMap::new(),
                    closure_counter: 0,
                    closure_structs: Vec::new(),
                    cabi_wrappers: Vec::new(),
                    cabi_exports: Vec::new(),
                    string_return_fns: HashSet::new(),
                    closure_vars: HashSet::new(),
                    current_fn: None,
                    exports: empty_exports,
                    try_label: None,
                    catch_label: None,
                    try_counter: self.try_counter,
                    temp_counter: 0,
                    destructure_prelude: Vec::new(),
                    current_class: None,
                    object_type_defs: Vec::new(),
                    current_obj_structs: Vec::new(),
                    init_globals_code: Vec::new(),
                };
                tmp.emit_arg(arg);
                tmp.output
            })
            .collect();

        // Replace positional placeholders {0}, {1}, ...
        let mut result = String::new();
        let mut chars = template.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '{' {
                if let Some(&('0'..='9')) = chars.peek() {
                    let mut idx_str = String::new();
                    while let Some(&('0'..='9')) = chars.peek() {
                        idx_str.push(chars.next().unwrap());
                    }
                    // Skip closing }
                    if chars.peek() == Some(&'}') {
                        chars.next();
                    }
                    if let Ok(idx) = idx_str.parse::<usize>()
                        && let Some(arg) = formatted_args.get(idx)
                    {
                        result.push_str(arg);
                    }
                } else if chars.peek() == Some(&'}') {
                    // {} → all args comma-separated
                    chars.next();
                    result.push_str(&formatted_args.join(", "));
                } else {
                    result.push(ch);
                }
            } else {
                result.push(ch);
            }
        }

        // If no placeholders were found, result == template → just push template
        // Actually, some templates like `@abs({})` have `{}` → replace with first arg
        if result == *template {
            // Simple case: template has no positional args, use first arg
            if let Some(first) = formatted_args.first() {
                result = template.replace("{}", first);
            }
        }

        self.push(&result);
    }

    fn is_string_literal_expr(expr: &Expression) -> bool {
        matches!(expr, Expression::StringLiteral(_))
    }

    // ========== Expression Helpers ==========

    fn emit_arg(&mut self, arg: &Argument) {
        match arg {
            Argument::SpreadElement(spread) => {
                self.push("...");
                self.emit_expr(&spread.argument);
            }
            _ => {
                if let Some(expr) = arg.as_expression() {
                    self.emit_expr(expr);
                }
            }
        }
    }

    fn emit_array_element(&mut self, elem: &ArrayExpressionElement) {
        match elem {
            ArrayExpressionElement::SpreadElement(spread) => {
                self.push("...");
                self.emit_expr(&spread.argument);
            }
            ArrayExpressionElement::Elision(_) => {
                self.push("undefined");
            }
            _ => {
                if let Some(expr) = elem.as_expression() {
                    self.emit_expr(expr);
                }
            }
        }
    }

    fn emit_assign_target(&mut self, target: &AssignmentTarget) {
        match target {
            AssignmentTarget::AssignmentTargetIdentifier(id) => {
                self.push(&Self::escape_keyword(id.name.as_str()));
            }
            AssignmentTarget::ArrayAssignmentTarget(_) => {
                self.push("_/* array destructure */");
            }
            AssignmentTarget::ObjectAssignmentTarget(_) => {
                self.push("_/* object destructure */");
            }
            AssignmentTarget::StaticMemberExpression(mem) => {
                self.emit_expr(&mem.object);
                self.push(".");
                self.push(mem.property.name.as_str());
            }
            AssignmentTarget::ComputedMemberExpression(mem) => {
                let obj_type = self.inferrer.infer_expr(&mem.object);
                if matches!(&obj_type, ZigType::Object { .. })
                    && let Expression::StringLiteral(s) = &mem.expression
                {
                    self.emit_expr(&mem.object);
                    self.push(".");
                    self.push(s.value.as_str());
                } else {
                    self.emit_expr(&mem.object);
                    self.push("[");
                    self.emit_expr(&mem.expression);
                    self.push("]");
                }
            }
            AssignmentTarget::PrivateFieldExpression(_) => {
                self.push("_/* private field */");
            }
            AssignmentTarget::TSAsExpression(ts) => {
                self.emit_expr(&ts.expression);
            }
            AssignmentTarget::TSSatisfiesExpression(ts) => {
                self.emit_expr(&ts.expression);
            }
            AssignmentTarget::TSNonNullExpression(ts) => {
                self.emit_expr(&ts.expression);
                self.push(".?");
            }
            AssignmentTarget::TSTypeAssertion(ts) => {
                self.emit_expr(&ts.expression);
            }
        }
    }

    fn emit_assign_target_from_simple(&mut self, target: &SimpleAssignmentTarget) {
        match target {
            SimpleAssignmentTarget::AssignmentTargetIdentifier(id) => {
                self.push(&Self::escape_keyword(id.name.as_str()));
            }
            SimpleAssignmentTarget::StaticMemberExpression(mem) => {
                self.emit_expr(&mem.object);
                self.push(".");
                self.push(mem.property.name.as_str());
            }
            SimpleAssignmentTarget::ComputedMemberExpression(mem) => {
                let obj_type = self.inferrer.infer_expr(&mem.object);
                if matches!(&obj_type, ZigType::Object { .. })
                    && let Expression::StringLiteral(s) = &mem.expression
                {
                    self.emit_expr(&mem.object);
                    self.push(".");
                    self.push(s.value.as_str());
                } else {
                    self.emit_expr(&mem.object);
                    self.push("[");
                    self.emit_expr(&mem.expression);
                    self.push("]");
                }
            }
            SimpleAssignmentTarget::PrivateFieldExpression(_) => {
                self.push("_/* private */");
            }
            _ => {
                self.push("_/* ts assign target */");
            }
        }
    }

    // ========== Operator Mappings ==========

    fn map_binary_op(&self, op: &BinaryOperator) -> &'static str {
        match op {
            BinaryOperator::StrictEquality => "==",
            BinaryOperator::StrictInequality => "!=",
            BinaryOperator::Equality => "==",
            BinaryOperator::Inequality => "!=",
            BinaryOperator::Addition => "+",
            BinaryOperator::Subtraction => "-",
            BinaryOperator::Multiplication => "*",
            BinaryOperator::Division => "/",
            BinaryOperator::Remainder => "%",
            BinaryOperator::Exponential => "**",
            BinaryOperator::LessThan => "<",
            BinaryOperator::LessEqualThan => "<=",
            BinaryOperator::GreaterThan => ">",
            BinaryOperator::GreaterEqualThan => ">=",
            BinaryOperator::ShiftLeft => "<<",
            BinaryOperator::ShiftRight => ">>",
            BinaryOperator::ShiftRightZeroFill => ">>",
            BinaryOperator::BitwiseOR => "|",
            BinaryOperator::BitwiseXOR => "^",
            BinaryOperator::BitwiseAnd => "&",
            BinaryOperator::In => "== /* in */",
            BinaryOperator::Instanceof => "== /* instanceof */",
        }
    }

    fn map_logical_op(&self, op: &LogicalOperator) -> &'static str {
        match op {
            LogicalOperator::And => "and",
            LogicalOperator::Or => "or",
            LogicalOperator::Coalesce => "orelse",
        }
    }

    fn map_unary_op(&self, op: &UnaryOperator) -> &'static str {
        match op {
            UnaryOperator::UnaryPlus => "",  // Zig doesn't support unary plus
            UnaryOperator::UnaryNegation => "-",
            UnaryOperator::LogicalNot => "!",
            UnaryOperator::BitwiseNot => "~",
            UnaryOperator::Typeof => "@TypeOf",
            UnaryOperator::Void => "",
            UnaryOperator::Delete => "",
        }
    }

    fn map_assign_op(&self, op: &AssignmentOperator) -> &'static str {
        match op {
            AssignmentOperator::Assign => "=",
            AssignmentOperator::Addition => "+=",
            AssignmentOperator::Subtraction => "-=",
            AssignmentOperator::Multiplication => "*=",
            AssignmentOperator::Division => "/=",
            AssignmentOperator::Remainder => "%=",
            AssignmentOperator::Exponential => "**=",
            AssignmentOperator::ShiftLeft => "<<=",
            AssignmentOperator::ShiftRight => ">>=",
            AssignmentOperator::ShiftRightZeroFill => ">>>=",
            AssignmentOperator::BitwiseOR => "|=",
            AssignmentOperator::BitwiseXOR => "^=",
            AssignmentOperator::BitwiseAnd => "&=",
            AssignmentOperator::LogicalOr => "|=",
            AssignmentOperator::LogicalAnd => "&=",
            AssignmentOperator::LogicalNullish => "=?=",
        }
    }
}
