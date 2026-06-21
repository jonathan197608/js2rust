use oxc_ast::ast::*;
use std::collections::{HashMap, HashSet};

use crate::codegen::collect_binding_names;

// ============================================================
// Diagnostic types
// ============================================================

#[derive(Debug, Clone, PartialEq)]
pub enum DiagnosticKind {
    Error,
    Warning,
}

/// A diagnostic message with optional source location.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub kind: DiagnosticKind,
    pub message: String,
    /// Byte offset range in the merged source text, if available.
    pub span: Option<(usize, usize)>,
}

impl Diagnostic {
    /// Create a new diagnostic without source location.
    pub fn new(kind: DiagnosticKind, message: String) -> Self {
        Self { kind, message, span: None }
    }

    /// Attach a source span to this diagnostic.
    pub fn with_span(mut self, start: usize, end: usize) -> Self {
        self.span = Some((start, end));
        self
    }

    /// Format the diagnostic with line:column info from source text.
    pub fn format_with_source(&self, source: &str) -> String {
        let prefix = match self.kind {
            DiagnosticKind::Error => "error",
            DiagnosticKind::Warning => "warning",
        };
        match self.span {
            Some((start, _end)) => {
                let (line, col) = byte_offset_to_line_col(source, start);
                format!("{}: [{}:{}] {}", prefix, line, col, self.message)
            }
            None => format!("{}: {}", prefix, self.message),
        }
    }
}

/// Convert a byte offset in source text to 1-based line:column.
fn byte_offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let offset = offset.min(source.len());
    let prefix = &source[..offset];
    let line = prefix.bytes().filter(|&b| b == b'\n').count() + 1;
    let last_newline = prefix.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let col = offset - last_newline + 1;
    (line, col)
}

// ============================================================
// ZigType
// ============================================================

#[derive(Debug, Clone, PartialEq)]
pub enum ZigType {
    I64,
    I32,
    Usize,
    F64,
    F32,
    Bool,
    String,
    Null,
    Void,
    Array(Box<ZigType>),
    /// Slice type for function parameters ([]const T).
    /// Unlike Array (\[_\]T), Slice does not require a compile-time size.
    Slice(Box<ZigType>),
    Optional(Box<ZigType>),
    FunctionPtr(Box<ZigFuncSig>),
    /// Named struct type (e.g., class-based: "Rectangle")
    Struct(String),
    /// Anonymous object struct with known field types (e.g., `{ name: "Alice", age: 30 }`)
    Object {
        fields: Vec<(String, ZigType)>,
    },
    /// Union type (e.g., `number | string` from heterogeneous returns)
    /// Generated as a tagged union in Zig.
    Union(Vec<ZigType>),
    /// Dynamic JS value type — maps to Zig `JsValue` union enum.
    /// Used for `var` declarations with value-type expressions (string/number/bool).
    JsValue,
    /// General-purpose container type — maps to Zig `JsAny`.
    /// Used for dynamic arrays (ArrayList), dynamic objects (HashMap), and nested structures.
    JsAny,
    /// TypedArray element types
    I16,
    I8,
    U32,
    U16,
    U8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ZigFuncSig {
    pub params: Vec<ZigType>,
    pub return_type: Box<ZigType>,
}

impl ZigType {
    pub fn to_zig_str(&self) -> String {
        match self {
            ZigType::I64 => "i64".to_string(),
            ZigType::I32 => "i32".to_string(),
            ZigType::I16 => "i16".to_string(),
            ZigType::I8 => "i8".to_string(),
            ZigType::Usize => "usize".to_string(),
            ZigType::U32 => "u32".to_string(),
            ZigType::U16 => "u16".to_string(),
            ZigType::U8 => "u8".to_string(),
            ZigType::F64 => "f64".to_string(),
            ZigType::F32 => "f32".to_string(),
            ZigType::Bool => "bool".to_string(),
            ZigType::String => "[]const u8".to_string(),
            ZigType::Null => "null".to_string(),
            ZigType::Void => "void".to_string(),
            ZigType::JsValue => "JsValue".to_string(),
            ZigType::JsAny => "JsAny".to_string(),
            ZigType::Array(elem) => format!("[_]{}", elem.to_zig_str()),
            ZigType::Slice(elem) => format!("[]const {}", elem.to_zig_str()),
            ZigType::Optional(inner) => format!("?{}", inner.to_zig_str()),
            ZigType::FunctionPtr(sig) => {
                let params: Vec<String> = sig.params.iter().map(|p| p.to_zig_str()).collect();
                format!("fn ({}) {}", params.join(", "), sig.return_type.to_zig_str())
            }
            ZigType::Struct(name) => name.clone(),
            ZigType::Object { .. } => "JsAny".to_string(),
            ZigType::Union(_) => "JsValue".to_string(),
        }
    }

    /// Return the C ABI compatible type string.
    /// JsValue/JsAny are represented as i64 over C ABI (extracting .int field).
    pub fn to_cabi_str(&self) -> String {
        match self {
            ZigType::JsValue | ZigType::JsAny => "i64".to_string(),
            _ => self.to_zig_str(),
        }
    }

    pub fn is_numeric(&self) -> bool {
        matches!(self, ZigType::I64 | ZigType::I32 | ZigType::F64 | ZigType::F32 | ZigType::Usize)
    }

    /// Widen: if both numeric, pick wider; if JsAny involved → JsAny; otherwise JsValue
    pub fn widen(left: &ZigType, right: &ZigType) -> ZigType {
        // JsAny absorbs everything (most general container)
        if matches!(left, ZigType::JsAny) || matches!(right, ZigType::JsAny) {
            return ZigType::JsAny;
        }
        // JsValue absorbs non-JsAny types
        if matches!(left, ZigType::JsValue) || matches!(right, ZigType::JsValue) {
            return ZigType::JsValue;
        }
        match (left, right) {
            (ZigType::F64, _) | (_, ZigType::F64) => ZigType::F64,
            (ZigType::F32, _) | (_, ZigType::F32) => ZigType::F64,
            (ZigType::I64, _) | (_, ZigType::I64) => ZigType::I64,
            (ZigType::I32, ZigType::I32) => ZigType::I32,
            (ZigType::Usize, ZigType::Usize) => ZigType::Usize,
            // Union types: merge and simplify
            (ZigType::Union(l), ZigType::Union(r)) => {
                let mut merged = l.clone();
                merged.extend(r.clone());
                ZigType::simplify_union(merged)
            }
            (ZigType::Union(u), other) | (other, ZigType::Union(u)) => {
                let mut merged = u.clone();
                merged.push(other.clone());
                ZigType::simplify_union(merged)
            }
            _ => ZigType::JsValue,
        }
    }

    /// Create a union type, simplifying if possible
    pub fn make_union(types: Vec<ZigType>) -> ZigType {
        if types.is_empty() {
            return ZigType::Void;
        }
        if types.len() == 1 {
            return types[0].clone();
        }
        // Deduplicate
        let mut dedup = Vec::new();
        for t in types {
            if !dedup.iter().any(|d| d == &t) {
                dedup.push(t);
            }
        }
        if dedup.len() == 1 {
            return dedup[0].clone();
        }
        ZigType::simplify_union(dedup)
    }

    /// Simplify a union type: flatten Optionals, widen numerics, etc.
    fn simplify_union(types: Vec<ZigType>) -> ZigType {
        // Defensive: empty union → JsValue
        if types.is_empty() {
            return ZigType::JsValue;
        }
        // Defensive: single-element union → flatten
        if types.len() == 1 {
            return types[0].clone();
        }

        // Optional flattening: JS `T | null` → Zig `?T`
        let has_null = types.iter().any(|t| matches!(t, ZigType::Null));
        if has_null {
            let non_null: Vec<ZigType> = types
                .into_iter()
                .filter(|t| !matches!(t, ZigType::Null))
                .collect();
            let simplified = ZigType::simplify_union(non_null);
            return ZigType::Optional(Box::new(simplified));
        }

        // JsAny absorbs everything (most general container type)
        if types.iter().any(|t| matches!(t, ZigType::JsAny)) {
            return ZigType::JsAny;
        }

        // JsValue absorbs non-JsAny types
        if types.iter().any(|t| matches!(t, ZigType::JsValue)) {
            return ZigType::JsValue;
        }

        // If all numeric, widen to the widest type
        if types.iter().all(|t| t.is_numeric()) {
            let mut result = types[0].clone();
            for t in &types[1..] {
                result = ZigType::widen(&result, t);
            }
            return result;
        }

        ZigType::Union(types)
    }

    /// Check if this type is a primitive value type (int, float, bool, string, null).
    pub fn is_value_type(&self) -> bool {
        matches!(self, ZigType::I64 | ZigType::I32 | ZigType::F64 | ZigType::F32
            | ZigType::Bool | ZigType::String | ZigType::Null | ZigType::Usize)
    }

    /// Extract the element type from container types (Array/Slice).
    /// Returns `I64` as the default for non-container types.
    pub fn element_type(&self) -> ZigType {
        match self {
            ZigType::Array(elem) | ZigType::Slice(elem) => *elem.clone(),
            _ => ZigType::I64,
        }
    }

    /// Check if this type is a static array or object (Layer 1).
    pub fn is_static_aggregate(&self) -> bool {
        matches!(self, ZigType::Array(_) | ZigType::Object { .. } | ZigType::Slice(_))
    }
}

// ============================================================
// Constant expression detection
// ============================================================

/// Determine if an expression is a compile-time constant.
/// Used to decide whether a `const` variable gets a precise Zig type (Layer 1)
/// or falls back to JsAny (Layer 3).
///
/// A constant expression is:
/// - A literal (number, string, boolean, null, bigint)
/// - A binary expression where both operands are constant
/// - A unary expression where the argument is constant
/// - A template literal with no interpolated expressions
/// - An array where all elements are constant
/// - An object where all property values are constant
/// - A parenthesized expression wrapping a constant
pub fn is_constant_expr(expr: &Expression) -> bool {
    match expr {
        // Literals — always constant
        Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_) => true,

        // Binary expression — constant if both operands are constant
        Expression::BinaryExpression(bin) => {
            is_constant_expr(&bin.left) && is_constant_expr(&bin.right)
        }

        // Unary expression — constant if argument is constant
        Expression::UnaryExpression(un) => is_constant_expr(&un.argument),

        // Template literal — constant only if no interpolated expressions
        Expression::TemplateLiteral(tl) => tl.expressions.is_empty(),

        // Array — constant if all elements are constant
        Expression::ArrayExpression(arr) => {
            arr.elements.iter().all(|elem| {
                elem.as_expression().is_some_and(is_constant_expr)
            })
        }

        // Object — constant if all property values are constant (no spread)
        Expression::ObjectExpression(obj) => {
            obj.properties.iter().all(|prop| match prop {
                ObjectPropertyKind::ObjectProperty(p) => is_constant_expr(&p.value),
                ObjectPropertyKind::SpreadProperty(_) => false,
            })
        }

        // Parenthesized — constant if inner is constant
        Expression::ParenthesizedExpression(p) => is_constant_expr(&p.expression),

        // Everything else is not constant
        _ => false,
    }
}

// ============================================================
// BindingInfo
// ============================================================

#[derive(Debug, Clone)]
pub struct BindingInfo {
    pub zig_type: ZigType,
    pub is_const: bool,
}

// ============================================================
// ParamConstraint — how a parameter is used, constraining its type
// ============================================================

#[derive(Debug, Clone)]
enum ParamConstraint {
    /// Used in binary expression: (other_operand_type, operator, is_left_operand)
    BinaryWith(ZigType, BinaryOperator, bool),
    /// Passed as argument to a function: (callee_name, arg_index)
    CallArg(String, usize),
    /// Used with a unary operator
    UnaryOp(UnaryOperator),
    /// Used as update target (++/--)
    Update,
    /// Used as if/while test condition
    Condition,
    /// Used in return position only (no type constraint from this alone)
    ReturnPos,
    /// Parameter was referenced but the usage doesn't constrain its type
    /// (e.g., member access obj.prop, array index arr[0])
    Referenced,
    /// Parameter is the iterable target of a for-of loop (e.g., `for (const x of arr)`)
    IteratedInto,
}

// ============================================================
// TypeInferrer
// ============================================================

pub struct TypeInferrer {
    env: HashMap<String, BindingInfo>,
    fn_return_types: HashMap<String, ZigType>,
    fn_param_types: HashMap<String, Vec<ZigType>>,
    fn_param_names: HashMap<String, Vec<String>>,
    diagnostics: Vec<Diagnostic>,
    /// Variables whose objects need HashMap-based dynamic property access
    /// (detected when computed member expr uses a variable key, e.g., obj[key])
    dynamic_access_vars: HashSet<String>,
    /// Variables whose arrays need ArrayList-based dynamic storage
    /// (detected when push/pop/shift/unshift/splice/sort/reverse is called on them)
    dynamic_arrays: HashSet<String>,
    /// Persistent storage for local variable types per function.
    /// Key: function name, Value: map from local variable name to its type.
    /// Populated during inference, used by codegen via get_var_type.
    fn_local_types: HashMap<String, HashMap<String, ZigType>>,
    /// Current function being analyzed (set by register_fn_env, cleared by unregister).
    /// Used to populate fn_local_types correctly.
    /// Also set by codegen before generating each function body,
    /// so that get_var_type() can look up fn_local_types.
    pub(crate) current_fn: Option<String>,
    /// Host function return types: func_name → return_type
    host_return_types: HashMap<String, ZigType>,
    /// Host function parameter types: func_name → param_types
    host_param_types: HashMap<String, Vec<ZigType>>,
    /// Host struct field types: struct_name → Vec<(field_name, field_type)>
    /// Used to infer member access on async host function return values.
    host_struct_fields: HashMap<String, Vec<(String, ZigType)>>,
}

/// Pre-indexed function data for O(1) lookup.
/// Built once at the start of inference, eliminating repeated linear scans
/// of `program.body` (previously ~7Nx for N functions).
struct FnIndex<'a> {
    /// All top-level statements (for initialization scans)
    top_level: &'a [Statement<'a>],
    /// fn_name → body statements
    fn_bodies: HashMap<String, &'a [Statement<'a>]>,
    /// fn_name → formal parameters (with defaults)
    fn_params: HashMap<String, &'a FormalParameters<'a>>,
}

impl<'a> FnIndex<'a> {
    fn build(program: &'a Program<'a>) -> Self {
        let mut fn_bodies: HashMap<String, &'a [Statement<'a>]> = HashMap::new();
        let mut fn_params: HashMap<String, &'a FormalParameters<'a>> = HashMap::new();

        for stmt in &program.body {
            match stmt {
                Statement::FunctionDeclaration(fd) => {
                    if let Some(id) = &fd.id {
                        let name = id.name.to_string();
                        fn_params.insert(name.clone(), &fd.params);
                        if let Some(body) = &fd.body {
                            fn_bodies.insert(name, body.statements.as_slice());
                        }
                    }
                }
                Statement::VariableDeclaration(vd) => {
                    for decl in &vd.declarations {
                        let mut names = Vec::new();
                        collect_binding_names(&decl.id, &mut names);
                        if names.len() == 1
                            && !names[0].starts_with("test_")
                            && let Some(init) = &decl.init
                        {
                            match init {
                                Expression::ArrowFunctionExpression(a) => {
                                    fn_params.insert(names[0].clone(), &a.params);
                                    fn_bodies.insert(names[0].clone(), a.body.statements.as_slice());
                                }
                                Expression::FunctionExpression(fe) => {
                                    fn_params.insert(names[0].clone(), &fe.params);
                                    if let Some(body) = &fe.body {
                                        fn_bodies.insert(names[0].clone(), body.statements.as_slice());
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        FnIndex { top_level: &program.body, fn_bodies, fn_params }
    }

    fn has_body(&self, fn_name: &str) -> bool {
        self.fn_bodies.contains_key(fn_name)
    }

    fn params(&self, fn_name: &str) -> Option<&&'a FormalParameters<'a>> {
        self.fn_params.get(fn_name)
    }

    fn body_stmts(&self, fn_name: &str) -> Option<&&'a [Statement<'a>]> {
        self.fn_bodies.get(fn_name)
    }
}

// ============================================================
// Generic AST walkers — shared structural traversal for
// detect_dynamic_access and collect_constraints passes.
// ============================================================

/// Event dispatched by walk_stmt for each expression or sub-statement encountered.
enum WalkEvent<'a> {
    Expr(&'a Expression<'a>),
    Stmt(&'a Statement<'a>),
}

/// Walk all sub-statements and sub-expressions within a statement.
/// Dispatches `WalkEvent::Expr` for every expression and `WalkEvent::Stmt`
/// for every nested statement. This is pure structural traversal — no business logic.
fn walk_stmt<'a, F>(stmt: &'a Statement<'a>, on_event: &mut F)
where
    F: FnMut(WalkEvent<'a>),
{
    match stmt {
        Statement::VariableDeclaration(v) => {
            for decl in &v.declarations {
                if let Some(init) = &decl.init {
                    on_event(WalkEvent::Expr(init));
                }
            }
        }
        Statement::ExpressionStatement(e) => on_event(WalkEvent::Expr(&e.expression)),
        Statement::ReturnStatement(r) => {
            if let Some(arg) = &r.argument {
                on_event(WalkEvent::Expr(arg));
            }
        }
        Statement::IfStatement(i) => {
            on_event(WalkEvent::Expr(&i.test));
            on_event(WalkEvent::Stmt(&i.consequent));
            if let Some(alt) = &i.alternate {
                on_event(WalkEvent::Stmt(alt));
            }
        }
        Statement::ForStatement(f) => {
            if let Some(init) = &f.init
                && let Some(e) = init.as_expression()
            {
                on_event(WalkEvent::Expr(e));
            }
            if let Some(test) = &f.test {
                on_event(WalkEvent::Expr(test));
            }
            if let Some(update) = &f.update {
                on_event(WalkEvent::Expr(update));
            }
            on_event(WalkEvent::Stmt(&f.body));
        }
        Statement::WhileStatement(w) => {
            on_event(WalkEvent::Expr(&w.test));
            on_event(WalkEvent::Stmt(&w.body));
        }
        Statement::DoWhileStatement(d) => {
            on_event(WalkEvent::Stmt(&d.body));
            on_event(WalkEvent::Expr(&d.test));
        }
        Statement::BlockStatement(b) => {
            for s in &b.body {
                on_event(WalkEvent::Stmt(s));
            }
        }
        Statement::SwitchStatement(s) => {
            on_event(WalkEvent::Expr(&s.discriminant));
            for case in &s.cases {
                if let Some(test) = &case.test {
                    on_event(WalkEvent::Expr(test));
                }
                for st in &case.consequent {
                    on_event(WalkEvent::Stmt(st));
                }
            }
        }
        Statement::FunctionDeclaration(f) => {
            if let Some(body) = &f.body {
                for s in &body.statements {
                    on_event(WalkEvent::Stmt(s));
                }
            }
        }
        Statement::ThrowStatement(t) => on_event(WalkEvent::Expr(&t.argument)),
        Statement::TryStatement(t) => {
            for s in &t.block.body {
                on_event(WalkEvent::Stmt(s));
            }
            if let Some(h) = &t.handler {
                for s in &h.body.body {
                    on_event(WalkEvent::Stmt(s));
                }
            }
            if let Some(f) = &t.finalizer {
                for s in &f.body {
                    on_event(WalkEvent::Stmt(s));
                }
            }
        }
        Statement::LabeledStatement(l) => on_event(WalkEvent::Stmt(&l.body)),
        Statement::ForOfStatement(f) => {
            on_event(WalkEvent::Expr(&f.right));
            on_event(WalkEvent::Stmt(&f.body));
        }
        Statement::ForInStatement(f) => {
            on_event(WalkEvent::Expr(&f.right));
            on_event(WalkEvent::Stmt(&f.body));
        }
        _ => {}
    }
}

/// Walk all immediate child expressions of an expression node.
/// Pure structural recursion — callers add their own specialized logic
/// before or after calling this.
fn walk_expr_children<'a>(
    expr: &'a Expression<'a>,
    on_expr: &mut dyn FnMut(&'a Expression<'a>),
) {
    match expr {
        Expression::BinaryExpression(b) => {
            on_expr(&b.left);
            on_expr(&b.right);
        }
        Expression::LogicalExpression(l) => {
            on_expr(&l.left);
            on_expr(&l.right);
        }
        Expression::UnaryExpression(u) => on_expr(&u.argument),
        Expression::CallExpression(c) => {
            on_expr(&c.callee);
            for arg in &c.arguments {
                if let Some(e) = arg.as_expression() {
                    on_expr(e);
                }
            }
        }
        Expression::AssignmentExpression(a) => on_expr(&a.right),
        Expression::ConditionalExpression(c) => {
            on_expr(&c.test);
            on_expr(&c.consequent);
            on_expr(&c.alternate);
        }
        Expression::ArrayExpression(a) => {
            for elem in &a.elements {
                if let Some(e) = elem.as_expression() {
                    on_expr(e);
                }
            }
        }
        Expression::ObjectExpression(o) => {
            for prop in &o.properties {
                match prop {
                    ObjectPropertyKind::ObjectProperty(p) => on_expr(&p.value),
                    ObjectPropertyKind::SpreadProperty(s) => on_expr(&s.argument),
                }
            }
        }
        Expression::ParenthesizedExpression(p) => on_expr(&p.expression),
        Expression::SequenceExpression(s) => {
            for e in &s.expressions {
                on_expr(e);
            }
        }
        Expression::ComputedMemberExpression(m) => {
            on_expr(&m.object);
            on_expr(&m.expression);
        }
        Expression::StaticMemberExpression(m) => on_expr(&m.object),
        Expression::AwaitExpression(a) => on_expr(&a.argument),
        Expression::YieldExpression(y) => {
            if let Some(arg) = &y.argument {
                on_expr(arg);
            }
        }
        Expression::NewExpression(n) => {
            for arg in &n.arguments {
                if let Some(e) = arg.as_expression() {
                    on_expr(e);
                }
            }
        }
        // Bodies of function/arrow expressions are handled by walk_stmt,
        // not by this expr-level walker.
        Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_) => {}
        _ => {}
    }
}

impl Default for TypeInferrer {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeInferrer {
    pub fn new() -> Self {
        Self {
            env: HashMap::new(),
            fn_return_types: HashMap::new(),
            fn_param_types: HashMap::new(),
            fn_param_names: HashMap::new(),
            diagnostics: Vec::new(),
            dynamic_access_vars: HashSet::new(),
            dynamic_arrays: HashSet::new(),
            fn_local_types: HashMap::new(),
            current_fn: None,
            host_return_types: HashMap::new(),
            host_param_types: HashMap::new(),
            host_struct_fields: HashMap::new(),
        }
    }

    // ============================================================
    // Main entry point
    // ============================================================

    pub fn infer_program(&mut self, program: &Program) {
        // Pre-pass: detect variables accessed with dynamic (non-literal) keys.
        // These variables must use HashMap instead of struct.
        self.detect_dynamic_access(program);

        // Pre-pass: detect arrays that have mutation methods called on them
        // (push/pop/shift/unshift/splice/sort/reverse).
        // These arrays must use ArrayList instead of fixed-size [_]T.
        self.detect_dynamic_arrays(program);

        // Build O(1) function index to eliminate repeated program.body scans
        let index = FnIndex::build(program);

        // Pass 1: collect assignments, register function signatures
        self.collect_top_level(&index);
        // Pass 2: infer function params from body usages
        self.infer_all_fn_params(&index);
        // Pass 3: infer return types with known param types
        self.infer_all_return_types(&index);
        // Pass 4: cross-function propagation + call-site propagation
        self.propagate_cross_fn(&index);
        // Pass 5: validate and report
        self.validate_types();
        // Pass 6: register all function params permanently for codegen queries
        self.install_all_fn_params();
    }

    // ============================================================
    // Public query methods
    // ============================================================

    pub fn diagnostics(&self) -> &[Diagnostic] { &self.diagnostics }
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.kind == DiagnosticKind::Error)
    }

    pub fn get_var_type(&self, name: &str) -> ZigType {
        // Check temporary env first (populated during inference)
        if let Some(ty) = self.env.get(name).map(|bi| bi.zig_type.clone()) {
            eprintln!("[DEBUG] get_var_type('{}') = {:?} (from env), current_fn={:?}", name, ty, self.current_fn);
            return ty;
        }
        // Fall back to persistent fn_local_types (populated during inference,
        // used during codegen when env is not populated).
        if let Some(ref fn_name) = self.current_fn
            && let Some(local_map) = self.fn_local_types.get(fn_name)
            && let Some(ty) = local_map.get(name)
        {
            eprintln!("[DEBUG] get_var_type('{}') = {:?} (from fn_local_types['{}'])", name, ty, fn_name);
            return ty.clone();
        }
        // If current_fn is set but var not found, try all functions (edge case)
        for (fn_name, local_map) in &self.fn_local_types {
            if let Some(ty) = local_map.get(name) {
                eprintln!("[DEBUG] get_var_type('{}') = {:?} (from fn_local_types['{}'] fallback)", name, ty, fn_name);
                return ty.clone();
            }
        }
        eprintln!("[DEBUG] get_var_type('{}') = JsValue (not found!), current_fn={:?}, fn_local_types keys={:?}", name, self.current_fn, self.fn_local_types.keys().collect::<Vec<_>>());
        ZigType::JsValue
    }

    /// Check if a variable is a struct type (for codegen to detect struct field access).
    /// This is used to avoid emitting .asString()/.asI64() etc. for struct fields.
    pub fn is_struct_var(&self, name: &str) -> bool {
        let ty = self.get_var_type(name);
        matches!(ty, ZigType::Struct(_))
    }

    /// Register a temporary binding (e.g., for-of loop variable) so that
    /// codegen can look up its type during expression emission.
    pub fn register_binding(&mut self, name: &str, ty: ZigType) {
        self.env.insert(name.to_string(), BindingInfo { zig_type: ty, is_const: true });
    }

    /// Clear the temporary environment after inference.
    /// This should be called before codegen to ensure get_var_type()
    /// uses fn_local_types instead of stale env entries.
    pub fn clear_env(&mut self) {
        self.env.clear();
    }

    /// Save the current environment (for temporary modifications).
    /// Returns a clone of the current env for later restoration.
    pub fn save_env(&self) -> std::collections::HashMap<String, BindingInfo> {
        self.env.clone()
    }

    /// Restore the environment from a saved state.
    pub fn restore_env(&mut self, saved: std::collections::HashMap<String, BindingInfo>) {
        self.env = saved;
    }

    /// Check if a variable name is in the dynamic_arrays set
    /// (i.e., push/pop/shift/unshift/splice/sort/reverse was called on it)
    pub fn is_dynamic_array(&self, name: &str) -> bool {
        self.dynamic_arrays.contains(name)
    }

    /// Mark a variable as a dynamic array (e.g., assigned from slice() return value)
    pub fn mark_as_dynamic_array(&mut self, name: &str) {
        self.dynamic_arrays.insert(name.to_string());
    }

    /// Check if a name is a parameter of a specific function.
    pub fn is_fn_param_of(&self, fn_name: &str, param_name: &str) -> bool {
        self.fn_param_names.get(fn_name)
            .map(|params| params.iter().any(|p| p == param_name))
            .unwrap_or(false)
    }

    /// Set the current function context (used during codegen closure scanning).
    /// This ensures get_var_type looks up variables in the correct function's scope.
    pub fn set_current_fn(&mut self, fn_name: &str) {
        self.current_fn = Some(fn_name.to_string());
    }

    /// Clear the current function context.
    pub fn clear_current_fn(&mut self) {
        self.current_fn = None;
    }

    /// Check if a name is a parameter of any known function.
    pub fn is_fn_param(&self, name: &str) -> bool {
        self.fn_param_names.values().any(|params| params.iter().any(|p| p == name))
    }

    pub fn get_fn_return_type(&self, name: &str) -> ZigType {
        self.fn_return_types.get(name).cloned().unwrap_or(ZigType::JsValue)
    }

    /// Debug helper: print fn_local_types for a given function
    pub fn debug_print_fn_local_types(&self, fn_name: &str) {
        if let Some(local_map) = self.fn_local_types.get(fn_name) {
            eprintln!("[DEBUG] fn_local_types['{}'] = {:?}", fn_name, local_map);
        } else {
            eprintln!("[DEBUG] fn_local_types['{}'] NOT FOUND", fn_name);
        }
    }

    pub fn all_fn_return_types(&self) -> HashMap<String, ZigType> {
        self.fn_return_types.clone()
    }

    /// Determine the Zig type for a variable based on the three-layer type system.
    ///
    /// Layer 1 (ZigType): `const` + constant expression → precise Zig type
    /// Layer 2 (JsValue): `var` + value type (no dynamic usage) → JsValue
    /// Layer 3 (JsAny): dynamic arrays/objects, or `const` with non-constant init
    pub fn infer_var_type(&self, name: &str, init: &Expression, is_const: bool) -> ZigType {
        let is_constant = is_constant_expr(init);

        // Rule 1: const + constant expression → precise Zig type (Layer 1)
        if is_const && is_constant {
            return self.infer_expr(init);
        }

        // Rule 2.3: const + new ClassName() → Struct (preserve class type)
        if is_const
            && let Expression::NewExpression(_) = init {
                return self.infer_expr(init);
            }

        // Rule 2.3b: const + arrow/function expression → FunctionPtr (preserve callable type)
        if is_const && matches!(init,
            Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_)
        ) {
            return self.infer_expr(init);
        }

        // Rule 2.3c: const + await expression → infer from awaited value
        // This allows `const user = await fetch_user(name)` to get the host
        // function's return type (e.g., Struct("FetchUserResult"))
        if is_const && matches!(init, Expression::AwaitExpression(_)) {
            return self.infer_expr(init);
        }

        // Rule 2.3d: const + CallExpression → infer return type
        // Allows `const v = m.get("a")` to get ?i64 (not JsAny)
        if is_const && matches!(init, Expression::CallExpression(_)) {
            let ret_ty = self.infer_expr(init);
            if ret_ty != ZigType::JsValue {
                return ret_ty;
            }
        }

        // Rule 2.4: const but not constant → JsAny (Layer 3)
        if is_const {
            return ZigType::JsAny;
        }

        // var declaration — check usage context for dynamic arrays/objects
        let is_dyn_array = self.dynamic_arrays.contains(name);
        let is_dyn_obj = self.dynamic_access_vars.contains(name);

        // Dynamic usage → JsAny (Layer 3)
        if is_dyn_array || is_dyn_obj {
            return ZigType::JsAny;
        }

        let init_type = self.infer_expr(init);

        // Rule 2.2/2.3: var + static array/object (no dynamic usage) → keep type
        if init_type.is_static_aggregate() {
            return init_type;
        }

        // Rule 2.1: var + value type → keep init type (supports loop accumulators)
        // JS transpilable code typically doesn't reassign variables to different types.
        // Keeping the precise type enables correct Zig codegen for loops and arithmetic.
        if init_type.is_value_type() {
            return init_type;
        }

        // Function expressions keep their type
        if matches!(init_type, ZigType::FunctionPtr(_)) {
            return init_type;
        }

        // Default → JsValue
        ZigType::JsValue
    }

    pub fn get_fn_param_types(&self, name: &str) -> Vec<ZigType> {
        self.fn_param_types.get(name).cloned().unwrap_or_default()
    }

    /// Infer the type of an arrow function parameter from how it's used in the body.
    /// Uses the same body-driven constraint approach as regular function params.
    /// `arrow_body` is the body of the ArrowFunctionExpression.
    pub fn infer_arrow_param_type(
        &self,
        param_name: &str,
        arrow_body: &oxc_ast::ast::FunctionBody,
    ) -> ZigType {
        let mut constraints = Vec::new();
        for s in &arrow_body.statements {
            self.collect_constraints_in_stmt(param_name, s, &mut constraints);
        }
        if constraints.is_empty() {
            return ZigType::I64; // aggressive default for arrow params
        }
        self.resolve_param_type(&constraints)
    }

    /// After all inference passes, register all function params in the env
    /// so that codegen's closure pre-scan can look up captured variable types.
    /// NOTE: This is a temporary workaround. Proper scoping should be implemented
    /// to avoid cross-scope variable name conflicts.
    fn install_all_fn_params(&mut self) {
        for (fn_name, param_types) in &self.fn_param_types {
            if let Some(param_names) = self.fn_param_names.get(fn_name) {
                for (i, pn) in param_names.iter().enumerate() {
                    let ty = if i < param_types.len() {
                        param_types[i].clone()
                    } else {
                        ZigType::I64
                    };
                    // Do NOT overwrite an existing variable with a more accurate type.
                    // This avoids cross-scope conflicts (e.g., a param named "base"
                    // overwriting a top-level object variable also named "base").
                    if let Some(existing) = self.env.get_mut(pn) {
                        // Only upgrade from Any to a concrete type; never downgrade.
                        if existing.zig_type == ZigType::JsValue {
                            existing.zig_type = ty;
                        }
                    } else {
                        self.env.insert(pn.clone(), BindingInfo { zig_type: ty, is_const: true });
                    }
                }
            }
        }
    }

    // ============================================================
    // PASS 1: collect top-level declarations
    // ============================================================

    fn collect_top_level(&mut self, index: &FnIndex) {
        // First: register all function names for cross-reference
        for name in index.fn_bodies.keys() {
            self.fn_param_types.entry(name.clone()).or_default();
            self.fn_return_types.entry(name.clone()).or_insert(ZigType::Void);
        }
        // Also register bodyless functions (host functions with params only)
        for (name, params) in &index.fn_params {
            if !self.fn_param_types.contains_key(name) {
                self.fn_param_types.entry(name.clone()).or_default();
                self.fn_return_types.entry(name.clone()).or_insert(ZigType::Void);
            }
            let pnames: Vec<String> = params.items.iter()
                .map(|p| {
                    let mut n = Vec::new();
                    collect_binding_names(&p.pattern, &mut n);
                    // Use "_" as placeholder for destructured/anonymous params
                    // to avoid generating empty parameter names in Zig output
                    let name = n.into_iter().next().unwrap_or_default();
                    if name.is_empty() { "_".to_string() } else { name }
                })
                .collect();
            if !pnames.is_empty() {
                self.fn_param_names.insert(name.clone(), pnames);
            }
        }

        // Second: infer assignment types and register function sigs
        for stmt in index.top_level {
            match stmt {
                Statement::VariableDeclaration(vd) => {
                    let is_const = matches!(vd.kind, VariableDeclarationKind::Const);
                    for decl in &vd.declarations {
                        let mut names = Vec::new();
                        collect_binding_names(&decl.id, &mut names);
                        let ty = decl.init.as_ref()
                            .map(|init| {
                                if names.len() == 1 {
                                    // Single binding — apply three-layer type inference
                                    self.infer_var_type(&names[0], init, is_const)
                                } else {
                                    // Destructured — infer from expression
                                    self.infer_expr(init)
                                }
                            })
                            .unwrap_or(ZigType::JsValue);

                        for name in &names {
                            self.env.insert(name.clone(), BindingInfo { zig_type: ty.clone(), is_const });
                        }

                        if names.len() == 1
                            && !names[0].starts_with("test_")
                            && let Some(init) = &decl.init
                                && let Some(sig) = self.extract_fn_sig(init) {
                                    let fn_name = &names[0];
                                    self.fn_return_types.insert(fn_name.clone(), (*sig.return_type).clone());
                                    self.fn_param_types.insert(fn_name.clone(), sig.params);
                                    if !self.fn_param_names.contains_key(fn_name) {
                                        self.fn_param_names.insert(fn_name.clone(), Vec::new());
                                    }
                                }
                    }
                }
                Statement::FunctionDeclaration(fd) => {
                    if let Some(id) = &fd.id {
                        let name = id.name.to_string();
                        let sig = ZigFuncSig {
                            params: self.fn_param_types.get(&name).cloned().unwrap_or_default(),
                            return_type: Box::new(
                                self.fn_return_types.get(&name).cloned().unwrap_or(ZigType::Void),
                            ),
                        };
                        self.env.insert(name, BindingInfo {
                            zig_type: ZigType::FunctionPtr(Box::new(sig)),
                            is_const: true,
                        });
                    }
                }
                _ => {}
            }
        }
    }

    // ============================================================
    // PASS 2: infer function params from body usages
    // ============================================================

    fn infer_all_fn_params(&mut self, index: &FnIndex) {
        // Collect fn names first to avoid borrow issues
        let fn_names: Vec<String> = self.fn_param_types.keys().cloned().collect();

        for fn_name in &fn_names {
            // Check if this function has a body; if not, only validate defaults
            if !index.has_body(fn_name) {
                if let Some(pnames) = self.fn_param_names.get(fn_name) {
                    let params = self.fn_param_types.get(fn_name).cloned().unwrap_or_default();
                    for (i, pname) in pnames.iter().enumerate() {
                        if i < params.len() && params[i] == ZigType::JsValue {
                            self.diagnostics.push(Diagnostic::new(
                                DiagnosticKind::Error,
                                format!(
                                    "function '{}' parameter '{}' has no body to infer from",
                                    fn_name, pname
                                ),
                            ));
                        }
                    }
                }
                continue;
            }

            let param_names: Vec<String> = self.fn_param_names
                .get(fn_name)
                .cloned()
                .unwrap_or_default();

            let mut inferred: Vec<ZigType> = Vec::new();

            for (i, pname) in param_names.iter().enumerate() {
                // Check default value first
                let default_ty = self.get_param_default_type(fn_name, i, index);

                if default_ty != ZigType::JsValue {
                    inferred.push(default_ty);
                    continue;
                }

                // Analyze body usages — O(1) body lookup via FnIndex
                let body_stmts = index.body_stmts(fn_name)
                    .copied()
                    .unwrap_or(&[]);
                let mut constraints = Vec::new();
                for s in body_stmts {
                    self.collect_constraints_in_stmt(pname, s, &mut constraints);
                }
                if constraints.is_empty() {
                    self.diagnostics.push(Diagnostic::new(
                        DiagnosticKind::Error,
                        format!(
                            "function '{}' parameter '{}' is never referenced in the function body",
                            fn_name, pname
                        ),
                    ));
                    inferred.push(ZigType::JsValue);
                } else {
                    inferred.push(self.resolve_param_type(&constraints));
                }
            }

            self.fn_param_types.insert(fn_name.clone(), inferred);
        }
    }

    /// Get the default value type for parameter index i of function fn_name
    fn get_param_default_type(&self, fn_name: &str, param_idx: usize, index: &FnIndex) -> ZigType {
        if let Some(params) = index.params(fn_name)
            && param_idx < params.items.len()
        {
            return params.items[param_idx].initializer.as_ref()
                .map(|d| self.infer_expr(d))
                .unwrap_or(ZigType::JsValue);
        }
        ZigType::JsValue
    }

    // ============================================================
    // Parameter constraint collection (walking AST)
    // ============================================================

    fn collect_constraints_in_stmt(
        &self,
        param_name: &str,
        stmt: &Statement,
        constraints: &mut Vec<ParamConstraint>,
    ) {
        // Specialized constraint checks for specific statement types.
        // These happen before structural traversal so the constraints are
        // recorded even if walk_stmt would also trigger them via expr walk.
        match stmt {
            Statement::ReturnStatement(rs) => {
                if let Some(arg) = &rs.argument
                    && self.is_name_ref_expr(arg, param_name)
                {
                    constraints.push(ParamConstraint::ReturnPos);
                }
            }
            Statement::IfStatement(ifs)
                if self.is_name_ref_expr(&ifs.test, param_name) =>
            {
                constraints.push(ParamConstraint::Condition);
            }
            Statement::WhileStatement(ws)
                if self.is_name_ref_expr(&ws.test, param_name) =>
            {
                constraints.push(ParamConstraint::Condition);
            }
            Statement::ForOfStatement(fos)
                if self.is_name_ref_expr(&fos.right, param_name) =>
            {
                constraints.push(ParamConstraint::IteratedInto);
            }
            _ => {}
        }

        // Structural traversal via shared walker
        walk_stmt(stmt, &mut |event| match event {
            WalkEvent::Expr(e) => self.collect_constraints_in_expr(param_name, e, constraints),
            WalkEvent::Stmt(s) => self.collect_constraints_in_stmt(param_name, s, constraints),
        });
    }

    fn collect_constraints_in_expr(
        &self,
        param_name: &str,
        expr: &Expression,
        constraints: &mut Vec<ParamConstraint>,
    ) {
        match expr {
            Expression::BinaryExpression(bin) => {
                let left_is = self.is_name_ref_expr(&bin.left, param_name);
                let right_is = self.is_name_ref_expr(&bin.right, param_name);
                if left_is || right_is {
                    let other_ty = if left_is {
                        self.infer_expr(&bin.right)
                    } else {
                        self.infer_expr(&bin.left)
                    };
                    constraints.push(ParamConstraint::BinaryWith(
                        other_ty, bin.operator, left_is,
                    ));
                } else {
                    self.collect_constraints_in_expr(param_name, &bin.left, constraints);
                    self.collect_constraints_in_expr(param_name, &bin.right, constraints);
                }
            }
            Expression::LogicalExpression(logic) => {
                self.collect_constraints_in_expr(param_name, &logic.left, constraints);
                self.collect_constraints_in_expr(param_name, &logic.right, constraints);
            }
            Expression::UnaryExpression(unary) => {
                if self.is_name_ref_expr(&unary.argument, param_name) {
                    constraints.push(ParamConstraint::UnaryOp(unary.operator));
                } else {
                    self.collect_constraints_in_expr(param_name, &unary.argument, constraints);
                }
            }
            Expression::CallExpression(call) => {
                for (i, arg) in call.arguments.iter().enumerate() {
                    if let Some(e) = arg.as_expression() {
                        if self.is_name_ref_expr(e, param_name) {
                            if let Some(callee_name) = self.get_callee_name(&call.callee) {
                                constraints.push(ParamConstraint::CallArg(callee_name, i));
                            }
                        } else {
                            self.collect_constraints_in_expr(param_name, e, constraints);
                        }
                    }
                }
                self.collect_constraints_in_expr(param_name, &call.callee, constraints);
            }
            Expression::AssignmentExpression(assign) => {
                self.collect_constraints_in_expr(param_name, &assign.right, constraints);
            }
            Expression::ConditionalExpression(cond) => {
                self.collect_constraints_in_expr(param_name, &cond.test, constraints);
                self.collect_constraints_in_expr(param_name, &cond.consequent, constraints);
                self.collect_constraints_in_expr(param_name, &cond.alternate, constraints);
            }
            Expression::ArrayExpression(arr) => {
                for elem in &arr.elements {
                    if let Some(e) = elem.as_expression() {
                        self.collect_constraints_in_expr(param_name, e, constraints);
                    }
                }
            }
            Expression::ParenthesizedExpression(p) => {
                self.collect_constraints_in_expr(param_name, &p.expression, constraints);
            }
            Expression::SequenceExpression(seq) => {
                for e in &seq.expressions {
                    self.collect_constraints_in_expr(param_name, e, constraints);
                }
            }
            Expression::TemplateLiteral(tl) => {
                for e in &tl.expressions {
                    self.collect_constraints_in_expr(param_name, e, constraints);
                }
            }
            Expression::UpdateExpression(up)
                if self.is_name_ref_simple_target(&up.argument, param_name) => {
                    constraints.push(ParamConstraint::Update);
                }
            Expression::StaticMemberExpression(mem) => {
                if self.is_name_ref_expr(&mem.object, param_name) {
                    constraints.push(ParamConstraint::Referenced);
                }
                self.collect_constraints_in_expr(param_name, &mem.object, constraints);
            }
            Expression::ComputedMemberExpression(mem) => {
                if self.is_name_ref_expr(&mem.object, param_name) {
                    constraints.push(ParamConstraint::Referenced);
                }
                self.collect_constraints_in_expr(param_name, &mem.object, constraints);
                self.collect_constraints_in_expr(param_name, &mem.expression, constraints);
            }
            Expression::AwaitExpression(await_expr) => {
                self.collect_constraints_in_expr(param_name, &await_expr.argument, constraints);
            }
            Expression::YieldExpression(yield_expr) => {
                if let Some(arg) = &yield_expr.argument {
                    self.collect_constraints_in_expr(param_name, arg, constraints);
                }
            }
            // Descend into nested function/arrow bodies (closures capturing outer params)
            Expression::ArrowFunctionExpression(arrow) => {
                for s in &arrow.body.statements {
                    self.collect_constraints_in_stmt(param_name, s, constraints);
                }
            }
            Expression::FunctionExpression(fe) => {
                if let Some(body) = &fe.body {
                    for s in &body.statements {
                        self.collect_constraints_in_stmt(param_name, s, constraints);
                    }
                }
            }
            // Descend into object expressions (e.g., { x: param })
            Expression::ObjectExpression(obj) => {
                for prop in &obj.properties {
                    match prop {
                        oxc_ast::ast::ObjectPropertyKind::ObjectProperty(p) => {
                            self.collect_constraints_in_expr(param_name, &p.value, constraints);
                        }
                        oxc_ast::ast::ObjectPropertyKind::SpreadProperty(s) => {
                            self.collect_constraints_in_expr(param_name, &s.argument, constraints);
                        }
                    }
                }
            }
            // Descend into new expressions (e.g., new Foo(param))
            Expression::NewExpression(new_expr) => {
                for arg in &new_expr.arguments {
                    if let Some(e) = arg.as_expression() {
                        self.collect_constraints_in_expr(param_name, e, constraints);
                    }
                }
            }
            // Catch-all: if the expression IS the parameter identifier,
            // mark it as referenced (handles const x = param, someVar = param, etc.)
            e if self.is_name_ref_expr(e, param_name) => {
                constraints.push(ParamConstraint::Referenced);
            }
            _ => {}
        }
    }

    fn is_name_ref_expr(&self, expr: &Expression, name: &str) -> bool {
        matches!(expr, Expression::Identifier(id) if id.name.as_str() == name)
    }

    fn is_name_ref_simple_target(&self, target: &SimpleAssignmentTarget, name: &str) -> bool {
        matches!(target,
            SimpleAssignmentTarget::AssignmentTargetIdentifier(id)
            if id.name.as_str() == name
        )
    }

    fn get_callee_name(&self, callee: &Expression) -> Option<String> {
        match callee {
            Expression::Identifier(id) => Some(id.name.to_string()),
            Expression::StaticMemberExpression(mem) => {
                if let Expression::Identifier(obj) = &mem.object {
                    Some(format!("{}.{}", obj.name, mem.property.name))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    // ============================================================
    // Constraint resolution
    // ============================================================

    fn resolve_param_type(&self, constraints: &[ParamConstraint]) -> ZigType {
        let types: Vec<ZigType> = constraints.iter()
            .map(|c| self.constraint_to_type(c))
            .filter(|t| *t != ZigType::JsValue) // skip non-constraining
            .collect();

        if types.is_empty() {
            return ZigType::I64; // aggressive default: numeric
        }

        if types.iter().all(|t| *t == types[0]) {
            return types[0].clone();
        }

        if types.iter().all(|t| t.is_numeric()) {
            let mut result = types[0].clone();
            for t in &types[1..] {
                result = ZigType::widen(&result, t);
                if result == ZigType::JsValue {
                    return ZigType::I64;
                }
            }
            return result;
        }

        // Heterogeneous types: create Union instead of falling back to Any
        ZigType::make_union(types)
    }

    fn constraint_to_type(&self, c: &ParamConstraint) -> ZigType {
        match c {
            ParamConstraint::BinaryWith(other, op, _is_left) => match op {
                BinaryOperator::Addition => {
                    if *other == ZigType::String { ZigType::String }
                    else if other.is_numeric() { other.clone() }
                    else { ZigType::I64 }
                }
                BinaryOperator::Subtraction
                | BinaryOperator::Multiplication
                | BinaryOperator::Remainder
                | BinaryOperator::Exponential => {
                    if other.is_numeric() { other.clone() } else { ZigType::I64 }
                }
                BinaryOperator::Division => ZigType::I64,  // transpiler uses @divTrunc (int div)
                BinaryOperator::ShiftLeft
                | BinaryOperator::ShiftRight
                | BinaryOperator::ShiftRightZeroFill => ZigType::I64,
                BinaryOperator::BitwiseOR
                | BinaryOperator::BitwiseXOR
                | BinaryOperator::BitwiseAnd => ZigType::I64,
                // Comparison ops don't constrain:
                BinaryOperator::StrictEquality
                | BinaryOperator::StrictInequality
                | BinaryOperator::Equality
                | BinaryOperator::Inequality
                | BinaryOperator::LessThan
                | BinaryOperator::LessEqualThan
                | BinaryOperator::GreaterThan
                | BinaryOperator::GreaterEqualThan
                | BinaryOperator::In
                | BinaryOperator::Instanceof => ZigType::JsValue,
            },
            ParamConstraint::CallArg(callee, arg_idx) => {
                self.builtin_param_type(callee, *arg_idx)
                    .or_else(|| {
                        self.fn_param_types.get(callee)
                            .and_then(|params| params.get(*arg_idx))
                            .cloned()
                    })
                    .unwrap_or(ZigType::JsValue)
            }
            ParamConstraint::UnaryOp(op) => match op {
                UnaryOperator::LogicalNot => ZigType::Bool,
                UnaryOperator::BitwiseNot => ZigType::I64,
                UnaryOperator::UnaryNegation | UnaryOperator::UnaryPlus => ZigType::I64,
                UnaryOperator::Typeof => ZigType::JsValue,
                UnaryOperator::Void => ZigType::JsValue,
                UnaryOperator::Delete => ZigType::JsValue,
            },
            ParamConstraint::Update => ZigType::I64,
            ParamConstraint::Condition => ZigType::JsValue,
            ParamConstraint::ReturnPos => ZigType::JsValue,
            ParamConstraint::Referenced => ZigType::JsValue,
            ParamConstraint::IteratedInto => ZigType::Slice(Box::new(ZigType::I64)),
        }
    }

    fn builtin_param_type(&self, callee: &str, arg_idx: usize) -> Option<ZigType> {
        // Check host function param types first
        if let Some(params) = self.host_param_types.get(callee) {
            return params.get(arg_idx).cloned();
        }

        match (callee, arg_idx) {
            ("Math.abs" | "Math.sqrt" | "Math.sin" | "Math.cos" | "Math.tan"
                | "Math.log" | "Math.floor" | "Math.ceil" | "Math.round"
                | "Math.trunc" | "Math.sign" | "Math.cbrt" | "Math.exp", 0) => Some(ZigType::I64),
            ("Math.pow" | "Math.min" | "Math.max", _) => Some(ZigType::I64),
            ("parseInt" | "parseFloat" | "Number" | "String" | "Boolean", 0) => Some(ZigType::JsValue),
            ("JSON.stringify", 0) => Some(ZigType::JsValue),
            ("JSON.parse", 0) => Some(ZigType::String),
            ("console.log" | "console.warn" | "console.error"
                | "console.info" | "console.debug", _) => Some(ZigType::JsValue),
            ("Array.isArray" | "isNaN", 0) => Some(ZigType::JsValue),
            ("encodeURIComponent" | "decodeURIComponent", 0) => Some(ZigType::String),
            _ => None,
        }
    }

    // ============================================================
    // PASS 3: infer return types
    // ============================================================

    fn infer_all_return_types(&mut self, index: &FnIndex) {
        let fn_names: Vec<String> = self.fn_return_types.keys().cloned().collect();

        for fn_name in &fn_names {
            // Register params and local vars in env for this function's body analysis
            let saved = self.register_fn_env(fn_name, index);

            // Walk body to collect return types — O(1) via FnIndex
            let body_stmts = index.body_stmts(fn_name)
                .copied()
                .unwrap_or(&[]);
            let ret = self.infer_return_type_from_stmts(body_stmts);

            self.fn_return_types.insert(fn_name.clone(), ret);

            // Restore env
            self.unregister_params_for_fn(saved);
        }

        // Fixup pass: re-compute return types now that callee return types are known.
        // This fixes functions like `useSafeDivide` whose return type depends on
        // the return type of `safeDivide` (processed in an arbitrary order above).
        for _ in 0..2 {
            let mut any_changed = false;
            for fn_name in &fn_names {
                let saved = self.register_fn_env(fn_name, index);
                let body_stmts = index.body_stmts(fn_name).copied().unwrap_or(&[]);
                let ret = self.infer_return_type_from_stmts(body_stmts);
                let old = self.fn_return_types.get(fn_name).cloned();
                if old.as_ref() != Some(&ret) {
                    self.fn_return_types.insert(fn_name.clone(), ret);
                    any_changed = true;
                }
                self.unregister_params_for_fn(saved);
            }
            if !any_changed { break; }
        }
    }

    /// Register both parameter bindings and local variable declarations
    /// for a function, returning saved env entries for later restoration.
    fn register_fn_env(
        &mut self,
        fn_name: &str,
        index: &FnIndex,
    ) -> Vec<(String, Option<BindingInfo>)> {
        let mut saved = Vec::new();

        // 0. Set current function for fn_local_types tracking
        self.current_fn = Some(fn_name.to_string());

        // 1. Register parameters
        let param_types = self.fn_param_types.get(fn_name).cloned().unwrap_or_default();
        if let Some(params) = index.params(fn_name) {
            for (i, param) in params.items.iter().enumerate() {
                let mut names = Vec::new();
                collect_binding_names(&param.pattern, &mut names);
                let ty = if i < param_types.len() {
                    param_types[i].clone()
                } else if let Some(d) = &param.initializer {
                    self.infer_expr(d)
                } else {
                    ZigType::JsValue
                };
                for pn in &names {
                    let old = self.env.remove(pn);
                    saved.push((pn.clone(), old));
                    self.env.insert(pn.clone(), BindingInfo { zig_type: ty.clone(), is_const: true });
                    // Also register in fn_local_types for codegen (after env is cleared)
                    self.fn_local_types.entry(fn_name.to_string()).or_insert_with(HashMap::new).insert(pn.clone(), ty.clone());
                }
            }
        }

        // 2. Register local variables from function body
        let body_stmts = index.body_stmts(fn_name).copied().unwrap_or(&[]);
        self.register_local_decls(body_stmts, &mut saved);

        saved
    }

    fn unregister_params_for_fn(&mut self, saved: Vec<(String, Option<BindingInfo>)>) {
        // Clear current function tracking
        self.current_fn = None;
        for (name, old) in saved {
            self.env.remove(&name);
            if let Some(bi) = old {
                self.env.insert(name, bi);
            }
        }
    }

    /// Walk statements and register local variable declarations in env.
    fn register_local_decls(
        &mut self,
        stmts: &[Statement],
        saved: &mut Vec<(String, Option<BindingInfo>)>,
    ) {
        for stmt in stmts {
            // Handle top-level VariableDeclaration directly.
            // walk_stmt only emits Expr(init) for VariableDeclarations, never
            // Stmt(VariableDeclaration), so we must handle them here at the top level.
            if let Statement::VariableDeclaration(vd) = stmt {
                let is_const = matches!(vd.kind, VariableDeclarationKind::Const);
                for decl in &vd.declarations {
                    if let Some(init) = &decl.init {
                        self.register_binding_with_expr(&decl.id, init, is_const, saved);
                    }
                }
            }

            // ForStatement VariableDeclaration init — not reached by walk_stmt
            if let Statement::ForStatement(f) = stmt
                && let Some(init_box) = &f.init
            {
                let fi_ref: &ForStatementInit = init_box;
                if let ForStatementInit::VariableDeclaration(v) = fi_ref {
                    let is_const = matches!(v.kind, VariableDeclarationKind::Const);
                    for decl in &v.declarations {
                        if let Some(init_expr) = &decl.init {
                            self.register_binding_with_expr(&decl.id, init_expr, is_const, saved);
                        }
                    }
                }
            }

            // Walk for nested VariableDeclarations (inside if/while/for/etc. bodies).
            walk_stmt(stmt, &mut |event| {
                if let WalkEvent::Stmt(Statement::VariableDeclaration(vd)) = event {
                    let is_const = matches!(vd.kind, VariableDeclarationKind::Const);
                    for decl in &vd.declarations {
                        if let Some(init) = &decl.init {
                            self.register_binding_with_expr(&decl.id, init, is_const, saved);
                        }
                    }
                }
            });
        }
    }

    /// Register a binding pattern with its initializer expression.
    fn register_binding_with_expr(
        &mut self,
        pattern: &BindingPattern,
        init: &Expression,
        is_const: bool,
        saved: &mut Vec<(String, Option<BindingInfo>)>,
    ) {
        let mut names = Vec::new();
        collect_binding_names(pattern, &mut names);

        // For destructured patterns, try to infer individual binding types
        let types: Vec<ZigType> = match pattern {
            BindingPattern::ObjectPattern(obj_pat) => {
                if let Expression::ObjectExpression(obj_expr) = init {
                    self.infer_destructured_obj_types(obj_pat, obj_expr)
                } else if let Expression::Identifier(id) = init {
                    // If init is an identifier with Object type, extract field types
                    let obj_type = self.get_var_type(id.name.as_str());
                    if let ZigType::Object { fields } = &obj_type {
                        let mut types = Vec::new();
                        for prop in &obj_pat.properties {
                            let name = match &prop.key {
                                oxc_ast::ast::PropertyKey::StaticIdentifier(id) => id.name.as_str(),
                                _ => {
                                    types.push(ZigType::JsValue);
                                    continue;
                                }
                            };
                            let ty = fields.iter()
                                .find(|(n, _)| n == name)
                                .map(|(_, ty)| ty.clone())
                                .unwrap_or(ZigType::JsValue);
                            types.push(ty);
                        }
                        types
                    } else {
                        let overall = self.infer_expr(init);
                        std::iter::repeat_n(overall, names.len()).collect()
                    }
                } else {
                    let overall = self.infer_expr(init);
                    std::iter::repeat_n(overall, names.len()).collect()
                }
            }
            BindingPattern::ArrayPattern(arr_pat) => {
                if let Expression::ArrayExpression(arr_expr) = init {
                    self.infer_destructured_array_types(arr_pat, arr_expr)
                } else {
                    let overall = self.infer_expr(init);
                    // If init is an array/slice type, unwrap element type for each binding
                    let elem_ty = match &overall {
                        ZigType::Array(elem) | ZigType::Slice(elem) => Some(elem.as_ref().clone()),
                        _ => None,
                    };
                    if let Some(elem) = elem_ty {
                        std::iter::repeat_n(elem, names.len()).collect()
                    } else {
                        std::iter::repeat_n(overall, names.len()).collect()
                    }
                }
            }
            _ => {
                // Special case: const + arrow function → infer return type with
                // arrow params properly registered in env (fixes closure return type).
                if is_const && names.len() == 1 {
                    if let Expression::ArrowFunctionExpression(arrow) = init {
                        let arrow_params: Vec<(String, ZigType)> = arrow.params.items.iter()
                            .map(|p| {
                                let mut pnames = Vec::new();
                                collect_binding_names(&p.pattern, &mut pnames);
                                let pname = pnames.into_iter().next().unwrap_or_default();
                                let pname = if pname.is_empty() { "_".to_string() } else { pname };
                                let ptype = self.infer_arrow_param_type(&pname, &arrow.body);
                                (pname, ptype)
                            })
                            .collect();

                        let ret = self.infer_return_type_from_arrow_with_params(arrow, &arrow_params);
                        let sig = ZigFuncSig {
                            params: arrow_params.iter().map(|(_, t)| t.clone()).collect(),
                            return_type: Box::new(ret),
                        };
                        vec![ZigType::FunctionPtr(Box::new(sig))]
                    } else {
                        vec![self.infer_var_type(&names[0], init, is_const)]
                    }
                } else if names.len() == 1 {
                    // Single binding — apply three-layer type inference
                    vec![self.infer_var_type(&names[0], init, is_const)]
                } else {
                    vec![self.infer_expr(init)]
                }
            }
        };

        for (i, pn) in names.iter().enumerate() {
            let ty = types.get(i).cloned().unwrap_or_else(|| self.infer_expr(init));
            let old = self.env.remove(pn);
            saved.push((pn.clone(), old));
            self.env.insert(pn.clone(), BindingInfo { zig_type: ty.clone(), is_const });
            // Also persist the type in fn_local_types for codegen use
            if let Some(ref fn_name) = self.current_fn {
                eprintln!("[DEBUG] register_fn_env: inserting var '{}' with type {:?} into fn_local_types['{}']", pn, ty, fn_name);
                self.fn_local_types
                    .entry(fn_name.clone())
                    .or_default()
                    .insert(pn.clone(), ty);
            }
        }
    }

    /// For destructured object bindings, infer each field's type from the object literal.
    fn infer_destructured_obj_types(
        &self,
        obj_pat: &oxc_ast::ast::ObjectPattern,
        obj_expr: &oxc_ast::ast::ObjectExpression,
    ) -> Vec<ZigType> {
        let mut types = Vec::new();
        for prop in &obj_pat.properties {
            let name = match &prop.key {
                oxc_ast::ast::PropertyKey::StaticIdentifier(id) => id.name.as_str(),
                _ => {
                    types.push(ZigType::JsValue);
                    continue;
                }
            };
            let found_type = obj_expr.properties.iter().find_map(|p| {
                if let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(op) = p {
                    let prop_name = match &op.key {
                        oxc_ast::ast::PropertyKey::StaticIdentifier(id) => id.name.as_str(),
                        _ => return None,
                    };
                    if prop_name == name {
                        return Some(self.infer_expr(&op.value));
                    }
                }
                None
            }).unwrap_or(ZigType::JsValue);
            types.push(found_type);
        }
        types
    }

    /// For destructured array bindings, infer each element's type from the array literal.
    fn infer_destructured_array_types(
        &self,
        _arr_pat: &oxc_ast::ast::ArrayPattern,
        arr_expr: &oxc_ast::ast::ArrayExpression,
    ) -> Vec<ZigType> {
        arr_expr.elements.iter().map(|elem| {
            elem.as_expression().map(|e| self.infer_expr(e)).unwrap_or(ZigType::JsValue)
        }).collect()
    }

    // ============================================================
    // PASS 4: cross-function propagation
    // ============================================================

    fn propagate_cross_fn(&mut self, index: &FnIndex) {
        let mut changed = true;
        for _ in 0..5 {
            if !changed { break; }
            changed = false;

            // Forward propagation: param passed to another function
            let fn_names: Vec<String> = self.fn_param_types.keys().cloned().collect();
            for fn_name in &fn_names {
                if !index.has_body(fn_name) { continue; }

                if let Some(param_names) = self.fn_param_names.get(fn_name) {
                    let mut updates: Vec<(usize, ZigType)> = Vec::new();

                    // First pass: collect propagations (immutable borrows)
                    for i in 0..param_names.len() {
                        let types = self.fn_param_types.get(fn_name);
                        // Allow re-consideration of I64 (default) params too, so that
                        // callee's concrete types (e.g. f64) can propagate back to caller.
                        if types.is_some_and(|t| i < t.len() && t[i] != ZigType::JsValue && t[i] != ZigType::I64) {
                            continue;
                        }
                        if let Some(propagated) = self.find_direct_forward(
                            &param_names[i], fn_name, index,
                        ) {
                            updates.push((i, propagated));
                        }
                    }

                    // Second pass: apply updates (mutable borrow)
                    if !updates.is_empty() {
                        if let Some(current) = self.fn_param_types.get_mut(fn_name) {
                            for (i, ty) in updates {
                                if i < current.len() {
                                    current[i] = ty;
                                }
                            }
                        }
                        changed = true;
                    }
                }
            }

            // Call-site propagation: argument type → parameter type
            let site_changed = self.propagate_from_call_sites(index);
            if site_changed {
                changed = true;
            }

            // Phase 3: Widen local variables based on function return type.
            // If a function returns f64 and a returned local variable is i64,
            // widen the variable to f64 so that `return cleaned;` compiles.
            for fn_name in &fn_names {
                let ret = self.fn_return_types.get(fn_name).cloned();
                if ret != Some(ZigType::F64) { continue; }
                if let Some(local_map) = self.fn_local_types.get_mut(fn_name) {
                    let mut widened: Vec<String> = Vec::new();
                    for (var_name, var_type) in local_map.iter_mut() {
                        if *var_type == ZigType::I64 {
                            *var_type = ZigType::F64;
                            widened.push(var_name.clone());
                        }
                    }
                    // Also update env if this function is currently registered
                    if self.current_fn.as_deref() == Some(fn_name) {
                        for var_name in &widened {
                            if let Some(bi) = self.env.get_mut(var_name) {
                                bi.zig_type = ZigType::F64;
                            }
                        }
                    }
                }
            }
        }
    }

    /// Walk every call expression in `program`.  For each argument
    /// whose type is concrete (not Any), propagate it to the
    /// corresponding parameter of the callee.
    /// Returns `true` if any parameter type was updated.
    fn propagate_from_call_sites(&mut self, index: &FnIndex) -> bool {
        let mut changed = false;
        let mut propagations: Vec<(String, usize, ZigType)> = Vec::new();

        // Scan top-level statements
        for stmt in index.top_level {
            self.collect_call_site_propagations(stmt, &mut propagations);
        }

        // P1: Scan inside function bodies for nested call sites.
        // (e.g., function main() { helper(42); } → propagate i64 to helper's param)
        for body_stmts in index.fn_bodies.values() {
            for stmt in *body_stmts {
                self.collect_call_site_propagations(stmt, &mut propagations);
            }
        }

        for (fn_name, param_idx, ty) in propagations {
            if let Some(params) = self.fn_param_types.get_mut(&fn_name)
                && param_idx < params.len()
            {
                let current = &params[param_idx];
                // Propagate if current is Any, or is the default I64 with no real constraint
                if *current == ZigType::JsValue || *current == ZigType::I64 {
                    params[param_idx] = ty;
                    changed = true;
                }
            }
        }

        changed
    }

    /// Recursively walk a statement, collecting (callee, param_idx, arg_type)
    /// triples from every CallExpression found.
    fn collect_call_site_propagations(
        &self,
        stmt: &Statement,
        propagations: &mut Vec<(String, usize, ZigType)>,
    ) {
        // walk_stmt doesn't handle ForStatement VariableDeclaration init
        if let Statement::ForStatement(f) = stmt
            && let Some(init_box) = &f.init
        {
            let fi_ref: &ForStatementInit = init_box;
            if let ForStatementInit::VariableDeclaration(v) = fi_ref {
                for decl in &v.declarations {
                    if let Some(init) = &decl.init {
                        self.collect_from_expr(init, propagations);
                    }
                }
            }
        }

        walk_stmt(stmt, &mut |event| match event {
            WalkEvent::Expr(e) => self.collect_from_expr(e, propagations),
            WalkEvent::Stmt(s) => self.collect_call_site_propagations(s, propagations),
        });
    }

    /// Walk an expression; if it is a CallExpression, propagate arg types.
    fn collect_from_expr(
        &self,
        expr: &Expression,
        propagations: &mut Vec<(String, usize, ZigType)>,
    ) {
        if let Expression::CallExpression(call) = expr {
            self.collect_from_call(call, propagations);
        }
        walk_expr_children(expr, &mut |e| self.collect_from_expr(e, propagations));
    }

    /// For a single CallExpression, infer each argument's type and
    /// record a propagation (callee, arg_idx, arg_type) if the
    /// argument's type is concrete.
    fn collect_from_call(
        &self,
        call: &CallExpression,
        propagations: &mut Vec<(String, usize, ZigType)>,
    ) {
        let callee_name = match self.get_callee_name(&call.callee) {
            Some(name) => name,
            None => return,
        };

        if self.is_builtin_name(&callee_name) {
            return;
        }

        for (i, arg) in call.arguments.iter().enumerate() {
            if let Some(expr) = arg.as_expression() {
                let arg_type = self.infer_expr(expr);
                if arg_type != ZigType::JsValue {
                    propagations.push((callee_name.clone(), i, arg_type));
                }
            }
        }
    }

    fn is_builtin_name(&self, name: &str) -> bool {
        matches!(name,
            "console.log" | "console.error" | "console.warn"
            | "console.info" | "console.debug"
            | "Math.abs" | "Math.sqrt" | "Math.sin" | "Math.cos"
            | "Math.tan" | "Math.log" | "Math.floor" | "Math.ceil"
            | "Math.round" | "Math.trunc" | "Math.sign" | "Math.cbrt"
            | "Math.exp" | "Math.pow" | "Math.min" | "Math.max"
            | "parseInt" | "parseFloat" | "Number" | "String"
            | "Boolean" | "JSON.stringify" | "JSON.parse"
            | "Array.isArray" | "isNaN"
            | "encodeURIComponent" | "decodeURIComponent"
        )
    }

    fn find_direct_forward(
        &self, param_name: &str, fn_name: &str, index: &FnIndex,
    ) -> Option<ZigType> {
        let body_stmts = index.body_stmts(fn_name).copied()?;
        self.forward_in_stmts(param_name, body_stmts)
    }

    fn forward_in_stmts(&self, param_name: &str, stmts: &[Statement]) -> Option<ZigType> {
        let mut result: Option<ZigType> = None;
        for stmt in stmts {
            walk_stmt(stmt, &mut |event| {
                if result.is_some() { return; }
                if let WalkEvent::Expr(e) = event
                    && let Some(ty) = self.check_forward_call(param_name, e) {
                        result = Some(ty);
                }
            });
            if result.is_some() { break; }
        }
        result
    }

    fn check_forward_call(&self, param_name: &str, expr: &Expression) -> Option<ZigType> {
        if let Expression::CallExpression(call) = expr {
            let callee_name = self.get_callee_name(&call.callee)?;
            for (i, arg) in call.arguments.iter().enumerate() {
                if let Some(e) = arg.as_expression()
                    && self.is_name_ref_expr(e, param_name) {
                        if let Some(params) = self.fn_param_types.get(&callee_name) {
                            return params.get(i).cloned();
                        }
                        return self.builtin_param_type(&callee_name, i);
                    }
            }
        }
        None
    }

    // ============================================================
    // PASS 5: validation
    // ============================================================

    fn validate_types(&mut self) {
        for (name, ret) in &self.fn_return_types {
            if *ret == ZigType::JsValue {
                self.diagnostics.push(Diagnostic::new(
                    DiagnosticKind::Warning,
                    format!("cannot infer return type of '{}', using JsValue", name),
                ));
            }
        }

        for (name, params) in &self.fn_param_types {
            for (i, ty) in params.iter().enumerate() {
                if *ty == ZigType::JsValue {
                    let pname = self.fn_param_names.get(name)
                        .and_then(|ns| ns.get(i))
                        .map(|s| s.as_str())
                        .unwrap_or("?");
                    self.diagnostics.push(Diagnostic::new(
                        DiagnosticKind::Warning,
                        format!(
                            "cannot infer type of parameter '{}' (#{}) in '{}', using JsValue",
                            pname, i, name
                        ),
                    ));
                }
            }
        }

        for (name, info) in &self.env {
            if info.zig_type == ZigType::JsValue {
                let name_str = name.as_str();
                match name_str {
                    "undefined" | "NaN" | "Infinity" => continue,
                    _ => {
                        self.diagnostics.push(Diagnostic::new(
                            DiagnosticKind::Warning,
                            format!("cannot infer type of variable '{}', using JsValue", name),
                        ));
                    }
                }
            }
        }
    }

    // ============================================================
    // Expression type inference
    // ============================================================

    pub fn infer_expr(&self, expr: &Expression) -> ZigType {
        match expr {
            Expression::NumericLiteral(lit) => {
                // Use parsed value: fractional part → F64, else I64
                if lit.value.fract() != 0.0 || lit.value.is_infinite() || lit.value.is_nan() {
                    ZigType::F64
                } else {
                    ZigType::I64
                }
            }
            Expression::StringLiteral(_) => ZigType::String,
            Expression::TemplateLiteral(_) => ZigType::String,
            Expression::BooleanLiteral(_) => ZigType::Bool,
            Expression::NullLiteral(_) => ZigType::Optional(Box::new(ZigType::Void)),
            Expression::RegExpLiteral(_) => ZigType::String, // regex patterns are strings
            Expression::BigIntLiteral(_) => ZigType::I64,
            Expression::Identifier(id) => match id.name.as_str() {
                "undefined" => ZigType::Optional(Box::new(ZigType::Void)),
                "NaN" | "Infinity" => ZigType::F64,
                _ => self.get_var_type(&id.name),
            },
            Expression::BinaryExpression(bin) => {
                let left_ty = self.infer_expr(&bin.left);
                let right_ty = self.infer_expr(&bin.right);
                match &bin.operator {
                    BinaryOperator::StrictEquality
                    | BinaryOperator::StrictInequality
                    | BinaryOperator::Equality
                    | BinaryOperator::Inequality
                    | BinaryOperator::LessThan
                    | BinaryOperator::LessEqualThan
                    | BinaryOperator::GreaterThan
                    | BinaryOperator::GreaterEqualThan
                    | BinaryOperator::In
                    | BinaryOperator::Instanceof => ZigType::Bool,
                    BinaryOperator::Addition => {
                        if left_ty == ZigType::String || right_ty == ZigType::String {
                            ZigType::String
                        } else {
                            ZigType::widen(&left_ty, &right_ty)
                        }
                    }
                    BinaryOperator::Subtraction
                    | BinaryOperator::Multiplication
                    | BinaryOperator::Remainder
                    | BinaryOperator::Exponential => ZigType::widen(&left_ty, &right_ty),
                    BinaryOperator::Division => ZigType::I64,  // @divTrunc, integer division
                    BinaryOperator::ShiftLeft
                    | BinaryOperator::ShiftRight
                    | BinaryOperator::ShiftRightZeroFill
                    | BinaryOperator::BitwiseOR
                    | BinaryOperator::BitwiseXOR
                    | BinaryOperator::BitwiseAnd => ZigType::I64,
                }
            }
            Expression::LogicalExpression(logic) => {
                let left_ty = self.infer_expr(&logic.left);
                let right_ty = self.infer_expr(&logic.right);
                match logic.operator {
                    LogicalOperator::And | LogicalOperator::Or => {
                        if left_ty == right_ty { left_ty } else { ZigType::make_union(vec![left_ty, right_ty]) }
                    }
                    LogicalOperator::Coalesce => ZigType::Optional(Box::new(left_ty)),
                }
            }
            Expression::UnaryExpression(unary) => match unary.operator {
                UnaryOperator::LogicalNot => ZigType::Bool,
                UnaryOperator::BitwiseNot => ZigType::I64,
                UnaryOperator::UnaryPlus => self.infer_expr(&unary.argument),
                UnaryOperator::UnaryNegation => self.infer_expr(&unary.argument),
                UnaryOperator::Typeof => ZigType::String,
                UnaryOperator::Void => ZigType::Void,
                UnaryOperator::Delete => ZigType::Bool,
            },
            Expression::UpdateExpression(_) => ZigType::I64,
            Expression::CallExpression(call) => self.infer_call_return(&call.callee),
            Expression::AssignmentExpression(assign) => self.infer_expr(&assign.right),
            Expression::ConditionalExpression(cond) => {
                let cons = self.infer_expr(&cond.consequent);
                let alt = self.infer_expr(&cond.alternate);
                if cons == alt { cons } else { ZigType::make_union(vec![cons, alt]) }
            }
            Expression::ArrayExpression(arr) => {
                if arr.elements.is_empty() { return ZigType::JsValue; }
                let first_ty = arr.elements.iter().find_map(|elem| match elem {
                    ArrayExpressionElement::SpreadElement(_) | ArrayExpressionElement::Elision(_) => None,
                    _ => elem.as_expression().map(|e| self.infer_expr(e)),
                });
                if let Some(ref ty) = first_ty {
                    let all_same = arr.elements.iter().all(|elem| match elem {
                        ArrayExpressionElement::SpreadElement(_) | ArrayExpressionElement::Elision(_) => true,
                        _ => elem.as_expression().map(|e| self.infer_expr(e) == *ty).unwrap_or(true),
                    });
                    if all_same && *ty != ZigType::JsValue {
                        return ZigType::Array(Box::new(ty.clone()));
                    }
                }
                ZigType::Array(Box::new(first_ty.unwrap_or(ZigType::JsValue)))
            }
            Expression::ParenthesizedExpression(p) => self.infer_expr(&p.expression),
            Expression::SequenceExpression(seq) => {
                seq.expressions.last().map(|e| self.infer_expr(e)).unwrap_or(ZigType::Void)
            }
            Expression::ArrowFunctionExpression(arrow) => {
                let ret = self.infer_return_type_from_arrow(arrow);
                ZigType::FunctionPtr(Box::new(ZigFuncSig {
                    params: vec![ZigType::JsValue; arrow.params.items.len()],
                    return_type: Box::new(ret),
                }))
            }
            Expression::TSAsExpression(ts) => self.infer_expr(&ts.expression),
            Expression::TSTypeAssertion(ts) => self.infer_expr(&ts.expression),
            Expression::TSNonNullExpression(ts) => self.infer_expr(&ts.expression),
            Expression::TSSatisfiesExpression(ts) => self.infer_expr(&ts.expression),
            Expression::TSInstantiationExpression(ts) => self.infer_expr(&ts.expression),
            Expression::StaticMemberExpression(mem) => self.infer_member_expr(mem),
            Expression::ComputedMemberExpression(mem) => self.infer_computed_member_expr(mem),
            Expression::ObjectExpression(obj) => self.infer_object_expr(obj),
            Expression::NewExpression(ne) => {
                // Check for built-in constructors
                if let Expression::Identifier(id) = &ne.callee {
                    match id.name.as_str() {
                        "Map" => return ZigType::Struct("Map".to_string()),
                        "Set" => return ZigType::Struct("Set".to_string()),
                        // TypedArray constructors -> Zig slice types
                        "Int8Array" => return ZigType::Slice(Box::new(ZigType::I8)),
                        "Uint8Array" => return ZigType::Slice(Box::new(ZigType::U8)),
                        "Uint8ClampedArray" => return ZigType::Slice(Box::new(ZigType::U8)),
                        "Int16Array" => return ZigType::Slice(Box::new(ZigType::I16)),
                        "Uint16Array" => return ZigType::Slice(Box::new(ZigType::U16)),
                        "Int32Array" => return ZigType::Slice(Box::new(ZigType::I32)),
                        "Uint32Array" => return ZigType::Slice(Box::new(ZigType::U32)),
                        "Float32Array" => return ZigType::Slice(Box::new(ZigType::F32)),
                        "Float64Array" => return ZigType::Slice(Box::new(ZigType::F64)),
                        // For class constructors, return Struct with class name
                        _ => return ZigType::Struct(id.name.to_string()),
                    }
                }
                ZigType::JsValue
            }
            Expression::FunctionExpression(_) => ZigType::JsValue,
            Expression::AwaitExpression(await_expr) => self.infer_expr(&await_expr.argument),
            _ => ZigType::JsValue,
        }
    }

    fn infer_object_expr(&self, obj: &ObjectExpression) -> ZigType {
        let mut fields: Vec<(String, ZigType)> = Vec::new();
        for prop in &obj.properties {
            match prop {
                oxc_ast::ast::ObjectPropertyKind::ObjectProperty(p) => {
                    let field_name = match &p.key {
                        oxc_ast::ast::PropertyKey::StaticIdentifier(id) => id.name.to_string(),
                        oxc_ast::ast::PropertyKey::StringLiteral(s) => s.value.to_string(),
                        _ => continue, // skip computed keys
                    };
                    let field_type = self.infer_expr(&p.value);
                    // Merge: if field already exists (from spread), override it
                    if let Some(pos) = fields.iter().position(|(n, _)| n == &field_name) {
                        fields[pos] = (field_name, field_type);
                    } else {
                        fields.push((field_name, field_type));
                    }
                }
                oxc_ast::ast::ObjectPropertyKind::SpreadProperty(spread) => {
                    let base_type = self.infer_expr(&spread.argument);
                    match base_type {
                        ZigType::Object { fields: base_fields } => {
                            for (name, ty) in &base_fields {
                                if !fields.iter().any(|(n, _)| n == name) {
                                    fields.push((name.clone(), ty.clone()));
                                }
                            }
                        }
                        _ => {
                            // Spread of non-Object type: can't merge fields
                            return ZigType::JsValue;
                        }
                    }
                }
            }
        }
        ZigType::Object { fields }
    }

    fn infer_member_expr(&self, mem: &StaticMemberExpression) -> ZigType {
        // this.field → all class fields are i64
        if matches!(&mem.object, Expression::ThisExpression(_)) {
            return ZigType::I64;
        }
        if let Expression::Identifier(id) = &mem.object {
            let obj_name = id.name.as_str();
            let prop = mem.property.name.as_str();
            let obj_type = self.get_var_type(obj_name);
            match &obj_type {
                ZigType::String => match prop {
                    "length" => ZigType::I64,
                    "charAt" | "slice" | "substring" | "toLowerCase" | "toUpperCase"
                        | "trim" | "padStart" | "padEnd" | "repeat" | "concat"
                        | "replace" | "replaceAll" | "split" | "toString" => ZigType::String,
                    "indexOf" | "lastIndexOf" | "search" | "charCodeAt" | "codePointAt" => ZigType::I64,
                    "includes" | "startsWith" | "endsWith" => ZigType::Bool,
                    _ => ZigType::JsValue,
                },
                ZigType::Array(elem) => match prop {
                    "length" => ZigType::I64,
                    "push" | "unshift" => ZigType::Usize,
                    "pop" | "shift" => ZigType::Optional(elem.clone()),
                    "splice" => ZigType::Array(elem.clone()),
                    "indexOf" | "lastIndexOf" => ZigType::I64,
                    "includes" => ZigType::Bool,
                    "join" | "reverse" | "sort" | "flat" | "flatMap" => ZigType::Array(elem.clone()),
                    "slice" | "concat" => ZigType::Array(elem.clone()),
                    _ => ZigType::JsValue,
                },
                ZigType::Slice(_) => match prop {
                    "length" => ZigType::I64,
                    _ => ZigType::JsValue,
                },
                ZigType::Object { fields } => {
                    // Look up the property in the object's field list
                    fields.iter()
                        .find(|(n, _)| n == prop)
                        .map(|(_, ty)| ty.clone())
                        .unwrap_or(ZigType::JsValue)
                }
                ZigType::Struct(struct_name) => {
                    // Check host struct fields first (async host function return types)
                    eprintln!("[DEBUG] infer_member_expr: Struct('{}'), prop='{}', host_struct_fields has {} entries", struct_name, prop, self.host_struct_fields.len());
                    if let Some(fields) = self.host_struct_fields.get(struct_name) {
                        eprintln!("[DEBUG] infer_member_expr: found fields for '{}': {:?}", struct_name, fields);
                        let result = fields.iter()
                            .find(|(n, _)| n == prop)
                            .map(|(_, ty)| ty.clone())
                            .unwrap_or(ZigType::JsValue);
                        eprintln!("[DEBUG] infer_member_expr: result for '{}.{}' = {:?}", struct_name, prop, result);
                        return result;
                    }
                    // Map/Set: .size property returns usize
                    if (struct_name == "Map" || struct_name == "Set") && prop == "size" {
                        return ZigType::Usize;
                    }
                    // Named struct from class: can't enumerate fields at infer time
                    // Fall through to Any — class fields are all i64 anyway
                    ZigType::JsValue
                }
                // Dynamic array (JsAny): .length returns i64
                ZigType::JsAny => {
                    if self.dynamic_arrays.contains(obj_name) && prop == "length" {
                        return ZigType::I64;
                    }
                    ZigType::JsValue
                }
                _ => match (obj_name, prop) {
                    ("Math", "PI" | "E" | "LN2" | "LN10" | "LOG2E" | "LOG10E"
                        | "SQRT2" | "SQRT1_2") => ZigType::F64,
                    ("Number", "MAX_SAFE_INTEGER" | "MIN_SAFE_INTEGER" | "MAX_VALUE"
                        | "MIN_VALUE" | "POSITIVE_INFINITY" | "NEGATIVE_INFINITY"
                        | "EPSILON") => ZigType::F64,
                    _ => ZigType::JsValue,
                },
            }
        } else {
            ZigType::JsValue
        }
    }

    fn infer_computed_member_expr(&self, mem: &ComputedMemberExpression) -> ZigType {
        // Check if object is a dynamic access variable (uses HashMap)
        // If so, person[key] returns JsValue (?Any type)
        if let Expression::Identifier(id) = &mem.object
            && self.dynamic_access_vars.contains(id.name.as_str()) {
                return ZigType::JsValue;  // JsValue
            }

        // Check if object is a dynamic array (ArrayList) — elements are JsAny
        // This check is needed because get_var_type may return JsValue (fallback)
        // for dynamic array variables that aren't in the env during codegen.
        if let Expression::Identifier(id) = &mem.object
            && self.dynamic_arrays.contains(id.name.as_str()) {
                return ZigType::JsAny;
            }

        let obj_type = self.infer_expr(&mem.object);
        match &obj_type {
            ZigType::Array(elem) | ZigType::Slice(elem) => elem.as_ref().clone(),
            ZigType::String => ZigType::I32,
            ZigType::Object { fields } => {
                // If the computed key is a string literal, we can resolve it
                if let Expression::StringLiteral(s) = &mem.expression {
                    let key = s.value.as_str();
                    fields.iter()
                        .find(|(n, _)| n == key)
                        .map(|(_, ty)| ty.clone())
                        .unwrap_or(ZigType::JsValue)
                } else {
                    // For non-literal keys, return Any (JsValue)
                    ZigType::JsValue
                }
            }
            // Dynamic arrays (ArrayList) store JsAny elements
            ZigType::JsAny => ZigType::JsAny,
            _ => ZigType::JsValue,
        }
    }
    // Return type inference
    // ============================================================

    /// Infer arrow return type with arrow params temporarily registered.
    /// This fixes the case where `x` in `(x) => x + base` is unknown (Any)
    /// because arrow params aren't in the TypeInferrer's env by default.
    pub fn infer_return_type_from_arrow_with_params(
        &mut self,
        arrow: &ArrowFunctionExpression,
        arrow_params: &[(String, ZigType)],
    ) -> ZigType {
        // Temporarily register arrow params so infer_expr can find their types
        let mut saved: Vec<(String, Option<BindingInfo>)> = Vec::new();
        for (pname, ptype) in arrow_params {
            let old = self.env.remove(pname.as_str());
            saved.push((pname.clone(), old));
            self.env.insert(
                pname.clone(),
                BindingInfo {
                    zig_type: ptype.clone(),
                    is_const: true,
                },
            );
        }

        let result = self.infer_return_type_from_arrow(arrow);

        // Restore original env state
        for (pname, old) in saved {
            if let Some(bi) = old {
                self.env.insert(pname, bi);
            } else {
                self.env.remove(&pname);
            }
        }

        result
    }

    pub fn infer_return_type_from_arrow(&self, arrow: &ArrowFunctionExpression) -> ZigType {
        if arrow.expression {
            if let Some(first) = arrow.body.statements.first() {
                match first {
                    Statement::ExpressionStatement(es) => return self.infer_expr(&es.expression),
                    Statement::ReturnStatement(rs) => {
                        return rs.argument.as_ref().map(|a| self.infer_expr(a)).unwrap_or(ZigType::Void);
                    }
                    _ => {}
                }
            }
            ZigType::Void
        } else {
            self.infer_return_type_from_stmts(&arrow.body.statements)
        }
    }

    pub fn infer_return_type_from_function_body(
        &self,
        body: &Option<oxc_allocator::Box<'_, FunctionBody<'_>>>,
    ) -> ZigType {
        match body {
            Some(b) => self.infer_return_type_from_stmts(&b.statements),
            None => ZigType::Void,
        }
    }

    fn infer_return_type_from_stmts(&self, stmts: &[Statement]) -> ZigType {
        let mut ret_types: Vec<ZigType> = Vec::new();
        self.collect_return_types(stmts, &mut ret_types);

        if ret_types.is_empty() {
            return ZigType::Void;
        }
        if ret_types.iter().all(|t| t == &ret_types[0]) {
            return ret_types[0].clone();
        }

        let first_non_void = ret_types.iter().find(|t| **t != ZigType::Void);
        if let Some(ty) = first_non_void {
            if ret_types.iter().filter(|t| **t != ZigType::Void).all(|t| *t == *ty) {
                return ty.clone();
            }
            let non_void: Vec<&ZigType> = ret_types.iter().filter(|t| **t != ZigType::Void).collect();
            if non_void.iter().all(|t| t.is_numeric()) {
                let mut result = first_non_void.unwrap().clone();
                for ty in &non_void[1..] {
                    result = ZigType::widen(&result, ty);
                    if matches!(result, ZigType::JsValue | ZigType::JsAny) { break; }
                }
                if !matches!(result, ZigType::JsValue | ZigType::JsAny) { return result; }
            }
        }

        // Heterogeneous return types: create Union instead of falling back to Any
        let non_void_types: Vec<ZigType> = ret_types.into_iter()
            .filter(|t| *t != ZigType::Void)
            .collect();
        ZigType::make_union(non_void_types)
    }

    fn collect_return_types(&self, stmts: &[Statement], out: &mut Vec<ZigType>) {
        for stmt in stmts {
            self.collect_return_types_rec(stmt, out);
        }
    }

    /// Recurse into a statement to find return statements.
    /// Handles ReturnStatement directly; uses walk_stmt for structural traversal
    /// of all other statement types.
    fn collect_return_types_rec(&self, stmt: &Statement, out: &mut Vec<ZigType>) {
        if let Statement::ReturnStatement(rs) = stmt {
            out.push(rs.argument.as_ref()
                .map(|a| self.infer_expr(a))
                .unwrap_or(ZigType::Void));
            return;
        }
        // Don't descend into inner function declarations
        if matches!(stmt, Statement::FunctionDeclaration(_)) {
            return;
        }
        walk_stmt(stmt, &mut |event| {
            if let WalkEvent::Stmt(s) = event {
                self.collect_return_types_rec(s, out);
            }
        });
    }

    // ============================================================
    // Call return type inference
    // ============================================================

    fn infer_call_return(&self, callee: &Expression) -> ZigType {
        match callee {
            Expression::Identifier(id) => {
                let name = id.name.as_str();
                if let Some(ret) = self.fn_return_types.get(name) {
                    return ret.clone();
                }
                // Look up local variable in env (e.g., arrow function assigned to const)
                if let Some(bi) = self.env.get(name)
                    && let ZigType::FunctionPtr(sig) = &bi.zig_type {
                        return (*sig.return_type).clone();
                    }
                // Check host function return types (sync and async)
                if let Some(ret) = self.host_return_types.get(name) {
                    return ret.clone();
                }
                self.builtin_return_type(name)
            }
            Expression::StaticMemberExpression(mem) => {
                if let Expression::Identifier(id) = &mem.object {
                    let obj_name = id.name.as_str();
                    let prop_name = mem.property.name.as_str();
                    // Special case: TypedArray.from() returns slice type
                    if obj_name == "Int32Array" && prop_name == "from" {
                        return ZigType::Slice(Box::new(ZigType::I32));
                    }
                    if obj_name == "Uint8Array" && prop_name == "from" {
                        return ZigType::Slice(Box::new(ZigType::U8));
                    }
                    if obj_name == "Float64Array" && prop_name == "from" {
                        return ZigType::Slice(Box::new(ZigType::F64));
                    }
                    let method_key = format!("{}.{}", obj_name, prop_name);
                    // Check builtins first (console.log, Math.abs, etc.)
                    let builtin = self.builtin_return_type(&method_key);
                    if builtin != ZigType::JsValue {
                        return builtin;
                    }
                    // Check for regexp methods (re.test(str), re.exec(str))
                    // Regexp literals are emitted as strings, so String-typed vars
                    // with .test()/.exec() methods are regexp dispatch.
                    let prop_name = mem.property.name.as_str();
                    if let Some(var_info) = self.env.get(obj_name)
                        && var_info.zig_type == ZigType::String {
                            return match prop_name {
                                // String search methods
                                "includes" | "startsWith" | "endsWith" => ZigType::Bool,
                                "indexOf" | "lastIndexOf" | "search" | "charCodeAt" | "codePointAt" => ZigType::I64,
                                // String transform methods
                                "charAt" | "slice" | "substring" | "toLowerCase" | "toUpperCase"
                                    | "trim" | "trimStart" | "trimEnd" | "padStart" | "padEnd"
                                    | "repeat" | "concat" | "replace" | "replaceAll" | "toString" => ZigType::String,
                                "split" => ZigType::Array(Box::new(ZigType::String)),
                                // Regexp methods (re.test/exec on string-typed vars used as patterns)
                                "test" => ZigType::Bool,
                                "exec" => ZigType::Optional(Box::new(ZigType::String)),
                                _ => ZigType::JsValue,
                            };
                        }
                    // Check if obj_name is a local variable (e.g., rect.area())
                    // Class structs: fields and method returns are all i64.
                    if let Some(var_info) = self.env.get(obj_name)
                        && let ZigType::Struct(s) = &var_info.zig_type {
                            // Map/Set: method-specific return types
                            if s == "Map" {
                                return match prop_name {
                                    "get" => ZigType::Optional(Box::new(ZigType::I64)),
                                    "set" | "delete" | "has" => ZigType::Bool,
                                    "size" => ZigType::Usize,
                                    "clear" => ZigType::Void,
                                    _ => ZigType::JsValue,
                                };
                            }
                            if s == "Set" {
                                return match prop_name {
                                    "add" | "delete" | "has" => ZigType::Bool,
                                    "size" => ZigType::Usize,
                                    "clear" => ZigType::Void,
                                    _ => ZigType::JsValue,
                                };
                            }
                            // Generic struct: assume i64 return
                            return ZigType::I64;
                        }
                    // Check array methods (arr.pop(), arr.push(), etc.)
                    if let Some(var_info) = self.env.get(obj_name)
                        && let ZigType::Array(elem) = &var_info.zig_type {
                            return match prop_name {
                                "length" => ZigType::I64,
                                "push" | "unshift" => ZigType::I64,
                                "pop" | "shift" => ZigType::Optional(elem.clone()),
                                "indexOf" | "lastIndexOf" => ZigType::I64,
                                "includes" => ZigType::Bool,
                                "join" | "reverse" | "sort" | "slice" | "concat" => ZigType::Array(elem.clone()),
                                "map" | "filter" | "flatMap" | "flat" => ZigType::Array(elem.clone()),
                                "reduce" => ZigType::JsAny,
                                "some" | "every" => ZigType::Bool,
                                "find" => ZigType::Optional(elem.clone()),
                                "findIndex" => ZigType::I64,
                                "forEach" => ZigType::Void,
                                _ => ZigType::JsValue,
                            };
                        }
                }
                ZigType::JsValue
            }
            _ => ZigType::JsValue,
        }
    }

    fn builtin_return_type(&self, name: &str) -> ZigType {
        // Check host function return types first
        if let Some(ret) = self.host_return_types.get(name) {
            return ret.clone();
        }

        match name {
            "parseInt" => ZigType::I64,
            "parseFloat" | "Number" | "Math.random" | "Math.floor" | "Math.ceil"
                | "Math.round" | "Math.abs" | "Math.sqrt" | "Math.sin" | "Math.cos"
                | "Math.tan" | "Math.log" | "Math.pow" | "Math.min" | "Math.max"
                | "Math.trunc" | "Math.sign" | "Math.cbrt" | "Math.exp" => ZigType::F64,
            "String" | "JSON.parse" => ZigType::String,
            "Boolean" | "Array.isArray" | "isNaN" | "Number.isInteger"
                | "Number.isNaN" | "Number.isFinite" => ZigType::Bool,
            "encodeURIComponent" | "decodeURIComponent" => ZigType::String,
            "console.log" | "console.warn" | "console.error" | "console.info"
                | "console.debug" | "console.assert" | "console.table" | "console.dir"
                | "console.time" | "console.timeEnd" | "console.group" | "console.groupEnd"
                | "console.clear" | "console.count" | "console.trace" => ZigType::Void,
            _ => ZigType::JsValue,
        }
    }

    /// Add a host function return type.
    pub fn add_host_return_type(&mut self, name: String, return_type: ZigType) {
        self.host_return_types.insert(name, return_type);
    }

    /// Register host function parameter types for type inference.
    /// When a variable is passed to a host function, its type is inferred from
    /// the host function's parameter type.
    pub fn register_host_param_types(&mut self, host_params: &HashMap<String, Vec<ZigType>>) {
        self.host_param_types = host_params.clone();
    }

    /// Register host struct field types for member access inference.
    /// When code accesses `struct_var.field_name`, the inferrer looks up
    /// the field type from this map (keyed by Zig struct name).
    pub fn register_host_struct_fields(&mut self, fields: &HashMap<String, Vec<(String, ZigType)>>) {
        self.host_struct_fields = fields.clone();
    }

    // ============================================================
    // Helpers
    // ============================================================

    fn extract_fn_sig(&self, expr: &Expression) -> Option<ZigFuncSig> {
        match expr {
            Expression::ArrowFunctionExpression(arrow) => {
                let ret = self.infer_return_type_from_arrow(arrow);
                Some(ZigFuncSig {
                    params: vec![ZigType::JsValue; arrow.params.items.len()],
                    return_type: Box::new(ret),
                })
            }
            Expression::FunctionExpression(fe) => {
                let ret = self.infer_return_type_from_function_body(&fe.body);
                Some(ZigFuncSig {
                    params: vec![ZigType::JsValue; fe.params.items.len()],
                    return_type: Box::new(ret),
                })
            }
            _ => None,
        }
    }

    /// Returns the set of variable names accessed with a dynamic (non-literal) key.
    pub fn get_dynamic_access_vars(&self) -> &HashSet<String> {
        &self.dynamic_access_vars
    }

    /// Pre-pass: detect variables that are accessed with a dynamic (non-literal) key.
    /// These variables must use HashMap instead of struct.
    fn detect_dynamic_access(&mut self, program: &Program) {
        for stmt in &program.body {
            self.detect_dynamic_access_stmt(stmt);
        }
    }

    fn detect_dynamic_access_stmt(&mut self, stmt: &Statement) {
        // Special case: ForStatement with VariableDeclaration init.
        // walk_stmt only handles expression inits via as_expression();
        // VariableDeclaration inits (e.g., for (var i = ...)) need explicit handling.
        if let Statement::ForStatement(f) = stmt
            && let Some(init_box) = &f.init
        {
            // Deref coercion: &Box<FI> → &FI
            let fi_ref: &ForStatementInit = init_box;
            if let ForStatementInit::VariableDeclaration(v) = fi_ref {
                for decl in &v.declarations {
                    if let Some(init) = &decl.init {
                        self.detect_dynamic_access_expr(init);
                    }
                }
            }
        }

        walk_stmt(stmt, &mut |event| match event {
            WalkEvent::Expr(e) => self.detect_dynamic_access_expr(e),
            WalkEvent::Stmt(s) => self.detect_dynamic_access_stmt(s),
        });
    }

    fn detect_dynamic_access_assign_target(&mut self, target: &AssignmentTarget) {
        // Use as_member_expression() to check if target is a MemberExpression
        if let Some(mem) = target.as_member_expression()
            && let MemberExpression::ComputedMemberExpression(cm) = mem
                && !matches!(&cm.expression, Expression::StringLiteral(_) | Expression::NumericLiteral(_))
                    && let Expression::Identifier(id) = &cm.object {
                        self.dynamic_access_vars.insert(id.name.to_string());
                    }
    }

    fn detect_dynamic_access_expr(&mut self, expr: &Expression) {
        // Specialized detection: ComputedMemberExpression with non-literal key
        // Exclude string and numeric literals — they are static indexing, not dynamic access.
        if let Expression::ComputedMemberExpression(mem) = expr
            && !matches!(&mem.expression, Expression::StringLiteral(_) | Expression::NumericLiteral(_))
                && let Expression::Identifier(id) = &mem.object {
                    self.dynamic_access_vars.insert(id.name.to_string());
                }

        // AssignmentExpression also needs to check the left side (AssignmentTarget)
        if let Expression::AssignmentExpression(a) = expr {
            self.detect_dynamic_access_assign_target(&a.left);
        }

        // Structural recursion via shared walker
        walk_expr_children(expr, &mut |e| self.detect_dynamic_access_expr(e));

        // walk_expr_children skips function bodies; recurse manually
        if let Expression::FunctionExpression(fe) = expr
            && let Some(body) = &fe.body
        {
            for s in &body.statements {
                self.detect_dynamic_access_stmt(s);
            }
        }
    }

    /// Pre-pass: detect arrays that have mutation methods called on them
    /// (push/pop/shift/unshift/splice/sort/reverse).
    /// These arrays must use ArrayList instead of fixed-size [_]T.
    fn detect_dynamic_arrays(&mut self, program: &Program) {
        for stmt in &program.body {
            self.detect_dynamic_arrays_stmt(stmt);
        }
    }

    fn detect_dynamic_arrays_stmt(&mut self, stmt: &Statement) {
        // Check for variable declarations that assign from a dynamic array
        if let Statement::VariableDeclaration(vd) = stmt {
            for decl in &vd.declarations {
                if let Some(init) = &decl.init
                    && let Expression::Identifier(id) = init
                    && self.dynamic_arrays.contains(id.name.as_str())
                {
                    // If assigning from a dynamic array, mark the new variable as dynamic
                    if let oxc_ast::ast::BindingPattern::BindingIdentifier(bi) = &decl.id {
                        self.dynamic_arrays.insert(bi.name.to_string());
                    }
                }
                // Check for assignment from array-returning methods (slice, filter, map, concat)
                // Always treat the result as a dynamic array (ArrayList), because these
                // methods always return a new array in generated Zig code.
                if let Some(init) = &decl.init
                    && let Expression::CallExpression(call) = init
                    && let Expression::StaticMemberExpression(mem) = &call.callee
                    && let Expression::Identifier(_obj_id) = &mem.object
                {
                    let method = mem.property.name.as_str();
                    // Methods that return a new array
                    if matches!(method, "slice" | "filter" | "map" | "concat")
                        && let oxc_ast::ast::BindingPattern::BindingIdentifier(bi) = &decl.id
                    {
                        self.dynamic_arrays.insert(bi.name.to_string());
                    }
                }
            }
        }

        walk_stmt(stmt, &mut |event| match event {
            WalkEvent::Expr(e) => self.detect_dynamic_arrays_expr(e),
            WalkEvent::Stmt(s) => self.detect_dynamic_arrays_stmt(s),
        });
    }

    fn detect_dynamic_arrays_expr(&mut self, expr: &Expression) {
        // Detect arr.push(), arr.pop(), arr.shift(), arr.unshift(),
        // arr.splice(), arr.sort(), arr.reverse()
        if let Expression::CallExpression(call) = expr
            && let Expression::StaticMemberExpression(mem) = &call.callee
            && let Expression::Identifier(id) = &mem.object
        {
            let method = mem.property.name.as_str();
            if matches!(method, "push" | "pop" | "shift" | "unshift" | "splice" | "sort" | "reverse") {
                self.dynamic_arrays.insert(id.name.to_string());
            }
        }

        // Check for assignment expressions: x = arr (where arr is dynamic)
        if let Expression::AssignmentExpression(assign) = expr
            && let Some(target_id) = Self::get_assignment_target_id(&assign.left)
            && let Expression::Identifier(src_id) = &assign.right
            && self.dynamic_arrays.contains(src_id.name.as_str())
        {
            self.dynamic_arrays.insert(target_id);
        }

        // Structural recursion via shared walker
        walk_expr_children(expr, &mut |e| self.detect_dynamic_arrays_expr(e));

        // walk_expr_children skips function bodies; recurse manually
        if let Expression::FunctionExpression(fe) = expr
            && let Some(body) = &fe.body
        {
            for s in &body.statements {
                self.detect_dynamic_arrays_stmt(s);
            }
        }
    }

    /// Helper: get the variable name from an assignment target
    fn get_assignment_target_id(target: &AssignmentTarget) -> Option<String> {
        match target {
            AssignmentTarget::AssignmentTargetIdentifier(id) => Some(id.name.to_string()),
            _ => None,
        }
    }

}
