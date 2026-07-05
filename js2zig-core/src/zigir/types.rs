// zigir/types.rs
// Core IR node types: module, declarations, statements, expressions.

use crate::types::ZigType;
use crate::zigir::builtins::BuiltinModule;
use crate::zigir::ident::IrIdent;
use crate::zigir::kinds::{CallKind, ComputedKeyKind, FieldKind, IndexKind, NewConstructor};
use crate::zigir::ops::{AssignOp, BinOp, LogicalOp, UnaOp, UpdateOp};
use crate::zigir::source_span::{IrDiagnostic, SourceSpan};

// ═══════════════════════════════════════════════════════
//  Top-level: IrModule
// ═══════════════════════════════════════════════════════

/// A complete Zig module (one JS file's transpilation result).
#[derive(Debug, Clone)]
pub struct IrModule {
    /// Module name (sanitized).
    pub name: String,
    /// Dependency imports.
    pub imports: Vec<IrImport>,
    /// JSDoc @typedef struct definitions.
    pub typedefs: Vec<IrTypedef>,
    /// Closure struct definitions (prepended before declarations in output).
    pub closure_structs: Vec<IrClosureStruct>,
    /// Top-level declarations (functions, variables, classes, compile errors).
    pub declarations: Vec<IrDecl>,
    /// Diagnostic messages.
    pub diagnostics: Vec<IrDiagnostic>,
    /// C ABI export metadata.
    pub cabi_exports: Vec<IrCabiExport>,
}

impl IrModule {
    pub fn new(name: String) -> Self {
        Self {
            name,
            imports: Vec::new(),
            typedefs: Vec::new(),
            closure_structs: Vec::new(),
            declarations: Vec::new(),
            diagnostics: Vec::new(),
            cabi_exports: Vec::new(),
        }
    }
}

/// Import declaration.
#[derive(Debug, Clone)]
pub struct IrImport {
    /// Sanitized Zig module name (e.g. "js_array", "std.json").
    pub module_name: String,
    /// (imported_name, local_alias) pairs.
    pub items: Vec<(String, String)>,
}

/// JSDoc @typedef struct definition.
#[derive(Debug, Clone)]
pub struct IrTypedef {
    pub name: String,
    pub fields: Vec<IrTypedefField>,
    pub is_opaque: bool,
    /// Whether to generate a `toJson()` method (always true for non-opaque
    /// typedefs, matching old Codegen behavior).
    pub has_to_json: bool,
}

#[derive(Debug, Clone)]
pub struct IrTypedefField {
    pub name: String,
    /// Zig type string (from jsdoc_type_to_zig, not yet parsed into ZigType).
    pub zig_type: String,
    pub optional: bool,
}

/// Closure struct definition (prepended at module level).
#[derive(Debug, Clone)]
pub struct IrClosureStruct {
    pub name: IrIdent,
    pub captured: Vec<IrCapture>,
    pub fn_params: Vec<IrParam>,
    pub return_type: ZigType,
    pub body: IrBlock,
}

/// C ABI export metadata.
#[derive(Debug, Clone)]
pub struct IrCabiExport {
    pub name: String,
    pub params: Vec<IrParam>,
    pub return_type: ZigType,
    /// Whether the exported function is async (uses io.async wrapper).
    pub is_async: bool,
    /// Whether the function can throw (error union return type).
    pub can_throw: bool,
    /// If return type is `ZigType::NamedStruct`, the struct name is extracted here.
    pub ret_struct_name: Option<String>,
}

// ═══════════════════════════════════════════════════════
//  Declarations: IrDecl
// ═══════════════════════════════════════════════════════

/// Top-level declaration.
#[derive(Debug, Clone)]
pub enum IrDecl {
    /// const/var variable declaration.
    Var(IrVarDecl),
    /// function declaration (export/regular/C ABI).
    Fn(IrFnDecl),
    /// class declaration → struct + init + methods.
    Class(IrClassDecl),
    /// @compileError at top level.
    CompileError { span: SourceSpan, msg: String },
}

// ── Variable declaration ──────────────────────────────

#[derive(Debug, Clone)]
pub struct IrVarDecl {
    pub name: IrIdent,
    pub is_const: bool,
    pub zig_type: Option<ZigType>,
    pub init: Option<IrExpr>,
    pub is_json_parse: bool,
    pub needs_var_suppression: bool,
}

// ── Function declaration ──────────────────────────────

#[derive(Debug, Clone)]
pub struct IrFnDecl {
    pub name: IrIdent,
    pub params: Vec<IrParam>,
    pub return_type: ZigType,
    pub body: IrBlock,
    pub is_export: bool,
    pub is_async: bool,
    pub can_throw: bool,
    pub is_cabi: bool,
}

#[derive(Debug, Clone)]
pub struct IrParam {
    pub name: IrIdent,
    pub zig_type: ZigType,
    /// Whether this parameter is unused in the function body.
    /// If true, the Emitter will prefix the param name with `_` and add
    /// a `_ = _param;` suppression statement at the start of the body.
    pub is_unused: bool,
}

/// A sequence of statements with an optional label.
#[derive(Debug, Clone)]
pub struct IrBlock {
    pub stmts: Vec<IrStmt>,
    pub label: Option<String>,
}

/// Kind of for-in iteration.
#[derive(Debug, Clone, PartialEq)]
pub enum IrForInKind {
    /// HashMap/dynamic object: iterator-based (`var __it = obj.iterator(); while (...)`)
    HashMapIter,
    /// Static struct with known fields: unrolled loop (one iteration per field).
    StructUnroll { fields: Vec<String> },
    /// Unknown/unsupported type → compile error.
    Unsupported,
}

/// Kind of for-of iteration.
#[derive(Debug, Clone, PartialEq)]
pub enum IrForOfKind {
    /// Array/ArrayList iteration: `for (iterable) |var| { ... }`
    Array,
    /// Map/Set iteration: `var __it = obj.inner.iterator(); while (__it.next()) |__kv| { ... }`
    MapSetIter { is_map: bool },
    /// `for await...of` is not supported.
    AsyncUnsupported,
}

impl IrBlock {
    pub fn new(stmts: Vec<IrStmt>) -> Self {
        Self { stmts, label: None }
    }

    pub fn with_label(stmts: Vec<IrStmt>, label: String) -> Self {
        Self {
            stmts,
            label: Some(label),
        }
    }
}

// ── Class declaration ─────────────────────────────────

#[derive(Debug, Clone)]
pub struct IrClassDecl {
    pub name: IrIdent,
    pub fields: Vec<IrClassField>,
    pub constructor: Option<IrClassMethod>,
    pub methods: Vec<IrClassMethod>,
    pub static_inits: Vec<IrExpr>,
    pub extends: Option<String>,
}

#[derive(Debug, Clone)]
pub struct IrClassField {
    pub name: String,
    pub zig_type: ZigType,
    pub default: Option<IrExpr>,
}

#[derive(Debug, Clone)]
pub struct IrClassMethod {
    pub name: String,
    pub params: Vec<IrParam>,
    pub return_type: ZigType,
    pub body: IrBlock,
    pub is_static: bool,
}

// ═══════════════════════════════════════════════════════
//  Statements: IrStmt
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub enum IrStmt {
    // ── Variable & assignment ───────────────────────
    VarDecl(IrVarDecl),
    Assign {
        target: IrAssignTarget,
        op: AssignOp,
        value: IrExpr,
    },

    // ── Control flow ────────────────────────────────
    If {
        cond: IrExpr,
        then: IrBlock,
        else_: Option<IrBlock>,
    },
    While {
        cond: IrExpr,
        body: IrBlock,
        label: Option<String>,
    },
    DoWhile {
        body: IrBlock,
        cond: IrExpr,
        label: Option<String>,
    },
    For {
        init: Option<Box<IrStmt>>,
        cond: Option<IrExpr>,
        update: Option<Box<IrStmt>>,
        body: IrBlock,
        label: Option<String>,
    },
    /// for-in: iterating over object keys.
    /// - `HashMapIter`: `var __it = obj.iterator(); while (__it.next()) |__kv| { const var = __kv.key_ptr.*; ... }`
    /// - `StructUnroll`: unrolled loop — one iteration per struct field with `const var = "fieldName"`
    ForIn {
        var: IrIdent,
        iterable: IrExpr,
        body: IrBlock,
        kind: IrForInKind,
        label: Option<String>,
    },
    /// for-of: iterating over array, Map, Set values.
    /// - `Array`: `for (iterable) |var| { ... }` (or `for (iterable.items) |var| { ... }` for ArrayList)
    /// - `MapSetIter`: `var __it = obj.inner.iterator(); while (__it.next()) |__kv| { const var = __kv.key_ptr.*; ... }`
    ForOf {
        var: IrIdent,
        /// Destructured variable names for Map iteration (e.g. `[key, val]`).
        destructure_vars: Vec<IrIdent>,
        iterable: IrExpr,
        /// If the iterable is an ArrayList variable, append `.items`.
        iterable_is_arraylist: bool,
        body: IrBlock,
        kind: IrForOfKind,
        is_async: bool,
        label: Option<String>,
    },
    Switch {
        expr: IrExpr,
        cases: Vec<IrSwitchCase>,
    },

    // ── Exception handling ──────────────────────────
    Try {
        try_block: IrBlock,
        catch_var: Option<IrIdent>,
        catch_var_referenced: bool,
        catch_block: IrBlock,
        finally: Option<IrBlock>,
        /// Whether the try body contains a `throw` (directly, not inside
        /// a nested try-catch). Drives B1/B2 optimization in Emitter.
        has_throw: bool,
        /// Whether the try body contains a nested TryStatement.
        has_nested_try: bool,
    },
    Throw {
        value: IrExpr,
    },

    // ── Function control ────────────────────────────
    Return {
        value: Option<IrExpr>,
    },
    Break {
        label: Option<String>,
    },
    Continue {
        label: Option<String>,
    },

    // ── Expression statement ────────────────────────
    Expr(IrExpr),

    // ── Block ───────────────────────────────────────
    Block(IrBlock),

    // ── Destructuring declaration ───────────────────
    /// Object or array destructuring: `const {a, b} = obj` / `const [a, b] = arr`
    /// Expanded into temp variable + individual binding declarations by the Emitter.
    DestructureDecl(IrDestructureDecl),

    // ── Debug / diagnostics ─────────────────────────
    CompileError {
        span: SourceSpan,
        msg: String,
    },
    Comment(String),
}

/// Assignment target (lhs of an assignment).
#[derive(Debug, Clone)]
pub enum IrAssignTarget {
    /// Simple identifier.
    Ident(IrIdent),
    /// Member field: `obj.field`
    Member {
        object: Box<IrExpr>,
        field: String,
        is_pointer: bool,
    },
    /// Index access: `obj[idx]`
    Index {
        object: Box<IrExpr>,
        index: Box<IrExpr>,
    },
    /// Destructuring assignment.
    Destructure(Vec<IrDestructureBinding>),
}

#[derive(Debug, Clone)]
pub struct IrDestructureBinding {
    pub pattern: IrIdent,
    pub default: Option<IrExpr>,
}

/// Destructuring declaration: `const {a, b} = expr` or `const [a, b] = expr`.
///
/// The Lowerer extracts all binding information; the Emitter expands this into
/// a temp variable assignment followed by individual `const/var` declarations
/// for each binding.
#[derive(Debug, Clone)]
pub struct IrDestructureDecl {
    /// Temp variable name (e.g. `_js_dest_0`) if the init expression needs
    /// to be evaluated only once. `None` means inline the init expression.
    pub temp_name: Option<String>,
    /// The init expression (RHS of the destructuring).
    pub init: IrExpr,
    /// Kind of destructuring (object vs array) with source type info.
    pub kind: IrDestructureKind,
    /// Individual binding declarations.
    pub bindings: Vec<IrDestructureBindingDecl>,
}

/// Whether the destructure source is a struct (direct field access),
/// a HashMap (.get("key")), or an ArrayList (.items[i]).
#[derive(Debug, Clone)]
pub enum IrDestructureKind {
    Object {
        /// True if the source is a struct with known fields → use `.field` access.
        /// False if HashMap or unknown → use `.get("key")` access.
        is_struct: bool,
        /// If struct, the set of field names that exist. Used to determine
        /// whether a key maps to a real field or needs a default.
        struct_field_names: Option<Vec<String>>,
    },
    Array {
        /// True if the source is an ArrayList → use `.items[i]` access.
        is_arraylist: bool,
    },
}

/// A single binding in a destructuring declaration.
#[derive(Debug, Clone)]
pub struct IrDestructureBindingDecl {
    /// The variable name being bound.
    pub name: IrIdent,
    /// Whether this binding is `const` (never mutated) or `var`.
    pub is_const: bool,
    /// The access pattern for extracting the value.
    pub access: IrDestructureAccess,
    /// Optional default value expression.
    pub default: Option<IrExpr>,
}

/// How to access the value for a destructuring binding.
#[derive(Debug, Clone)]
pub enum IrDestructureAccess {
    /// Object field: `source.field` (struct) or `source.get("key")` (HashMap)
    ObjectField {
        /// Variable name of the source (temp var or inline init expr string).
        source: String,
        /// The property key name.
        key: String,
        /// Whether the key exists as a struct field (determines .field vs .get()).
        is_struct_field: bool,
    },
    /// Array index: `source[i]` (slice) or `source.items[i]` (ArrayList)
    ArrayIndex {
        /// Variable name of the source.
        source: String,
        /// The index position.
        index: usize,
    },
}

/// A single switch case.
#[derive(Debug, Clone)]
pub struct IrSwitchCase {
    /// None = default case.
    pub test: Option<IrExpr>,
    pub body: Vec<IrStmt>,
}

// ═══════════════════════════════════════════════════════
//  Expressions: IrExpr
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub enum IrExpr {
    // ── Literals ────────────────────────────────────
    IntLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),
    BoolLiteral(bool),
    Null,
    Undefined,

    // ── Identifier reference ────────────────────────
    Ident(IrIdent),
    This,

    // ── Arithmetic / comparison ─────────────────────
    Binary {
        op: BinOp,
        left: Box<IrExpr>,
        right: Box<IrExpr>,
    },
    Unary {
        op: UnaOp,
        operand: Box<IrExpr>,
    },
    Logical {
        op: LogicalOp,
        left: Box<IrExpr>,
        right: Box<IrExpr>,
    },
    Update {
        op: UpdateOp,
        target: Box<IrAssignTarget>,
        is_expr_stmt: bool,
    },
    Assign {
        op: AssignOp,
        target: Box<IrAssignTarget>,
        value: Box<IrExpr>,
    },

    // ── Calls ───────────────────────────────────────
    Call(IrCallExpr),
    BuiltinCall(IrBuiltinCall),
    HostCall(IrHostCall),

    // ── Member access ───────────────────────────────
    FieldAccess {
        object: Box<IrExpr>,
        field: String,
        field_kind: FieldKind,
    },
    IndexAccess {
        object: Box<IrExpr>,
        index: Box<IrExpr>,
        index_kind: IndexKind,
    },
    ComputedField {
        object: Box<IrExpr>,
        key: Box<IrExpr>,
        key_kind: ComputedKeyKind,
    },

    // ── Object / Array ──────────────────────────────
    ArrayLiteral(IrArrayLiteral),
    ObjectLiteral(IrObjectLiteral),

    // ── Function expressions ────────────────────────
    ArrowFn(IrArrowFn),
    Closure(IrClosure),
    FnExpr(IrFnExpr),

    // ── Conditional / template ──────────────────────
    Conditional {
        cond: Box<IrExpr>,
        then: Box<IrExpr>,
        else_: Box<IrExpr>,
    },
    TemplateLiteral {
        parts: Vec<String>,
        exprs: Vec<IrExpr>,
        /// Zig format specifier for each interpolated expression.
        /// E.g. ["{s}", "{d}"] means first expr is a string, second is numeric.
        format_specs: Vec<String>,
    },

    // ── Async ───────────────────────────────────────
    Await(IrAwaitExpr),

    // ── Construction ────────────────────────────────
    New(IrNewExpr),

    // ── Block expression ────────────────────────────
    BlockExpr {
        label: String,
        body: Vec<IrStmt>,
        result: Box<IrExpr>,
    },

    // ── String formatting ──────────────────────────
    /// Runtime string concatenation via std.fmt.allocPrint.
    /// Generated when `+` has a string operand (JS coercion semantics).
    AllocPrint {
        /// Zig format string (already escaped for std.fmt).
        fmt: String,
        /// Interpolation arguments (may be empty → plain string literal).
        args: Vec<IrExpr>,
    },

    // ── Special ─────────────────────────────────────
    Spread(Box<IrExpr>),
    Typeof(Box<IrExpr>),
    Void(Box<IrExpr>),
    Paren(Box<IrExpr>),
    Sequence(Vec<IrExpr>),
    CompileError {
        span: SourceSpan,
        msg: String,
    },

    // ── Array callback inlining ─────────────────────
    /// Inline expansion of array callback methods (forEach, some, every, filter,
    /// find, findIndex, findLast, findLastIndex, map, reduce).
    ///
    /// Instead of emitting `js_array.method(callback)`, the Emitter expands
    /// these into Zig for/while loops with the callback body unwrapped.
    ArrayCallbackInline(Box<IrArrayCallbackInline>),

    /// Inline expansion of array non-callback methods (includes, indexOf,
    /// lastIndexOf, join, slice, splice, at, concat, copyWithin, fill).
    ///
    /// Instead of emitting `js_array.method(args)`, the Emitter expands
    /// these into Zig block expressions or statements.
    ArrayMethodInline(Box<IrArrayMethodInline>),
}

// ── Call types ─────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct IrCallExpr {
    pub callee: Box<IrExpr>,
    pub args: Vec<IrExpr>,
    pub call_kind: CallKind,
}

#[derive(Debug, Clone)]
pub struct IrBuiltinCall {
    pub module: BuiltinModule,
    pub method: String,
    /// The receiver object variable name (e.g., "str" in `str.toUpperCase()`).
    /// Used by the Emitter to insert as the first runtime argument after allocator.
    pub obj_name: Option<String>,
    pub args: Vec<IrExpr>,
    pub return_type: ZigType,
}

#[derive(Debug, Clone)]
pub struct IrHostCall {
    pub name: String,
    pub args: Vec<IrExpr>,
    pub return_type: ZigType,
    pub is_async: bool,
}

// ── Await ──────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct IrAwaitExpr {
    pub task_var: IrIdent,
    pub callee: Box<IrExpr>,
    pub args: Vec<IrExpr>,
    pub is_host_async: bool,
    pub block_label: String,
}

// ── Closure ────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct IrClosure {
    pub struct_name: IrIdent,
    pub captured: Vec<IrCapture>,
    pub fn_params: Vec<IrParam>,
    pub return_type: ZigType,
    pub body: IrBlock,
    pub instance_name: IrIdent,
}

#[derive(Debug, Clone)]
pub struct IrCapture {
    pub name: IrIdent,
    pub zig_type: ZigType,
    pub is_mut: bool,
}

// ── Arrow function ─────────────────────────────────────

#[derive(Debug, Clone)]
pub struct IrArrowFn {
    pub params: Vec<IrParam>,
    pub return_type: ZigType,
    pub body: IrBlock,
    pub is_concise: bool,
}

// ── Function expression ────────────────────────────────

#[derive(Debug, Clone)]
pub struct IrFnExpr {
    pub name: Option<IrIdent>,
    pub params: Vec<IrParam>,
    pub return_type: ZigType,
    pub body: IrBlock,
}

// ── Array literal ──────────────────────────────────────

#[derive(Debug, Clone)]
pub struct IrArrayLiteral {
    pub elements: Vec<IrExpr>,
    pub spread_indices: Vec<usize>,
}

// ── Object literal ─────────────────────────────────────

#[derive(Debug, Clone)]
pub struct IrObjectLiteral {
    pub fields: Vec<IrObjectField>,
    pub spreads: Vec<IrExpr>,
}

#[derive(Debug, Clone)]
pub struct IrObjectField {
    pub key: String,
    pub value: IrExpr,
    pub is_computed: bool,
}

// ── New expression ─────────────────────────────────────

#[derive(Debug, Clone)]
pub struct IrNewExpr {
    pub constructor: NewConstructor,
    pub args: Vec<IrExpr>,
    pub result_type: ZigType,
}

// ═══════════════════════════════════════════════════════
//  Array callback inlining
// ═══════════════════════════════════════════════════════

/// Which array callback method is being inlined.
#[derive(Debug, Clone, PartialEq)]
pub enum ArrayCallbackKind {
    ForEach,
    Some,
    Every,
    Filter,
    Find,
    FindIndex,
    FindLast,
    FindLastIndex,
    Map,
    Reduce,
}

/// Data for inline expansion of array callback methods.
///
/// The Emitter uses this to generate a Zig loop instead of
/// a runtime `js_array.method(callback)` call.
#[derive(Debug, Clone)]
pub struct IrArrayCallbackInline {
    /// Which callback method (forEach, some, every, etc.)
    pub kind: ArrayCallbackKind,
    /// Name of the array object being iterated (e.g., "arr").
    pub obj_name: String,
    /// The Zig type of array elements (for filter's ArrayList type).
    pub elem_type: ZigType,
    /// The callback element parameter name (e.g., "x" from `(x) => ...`),
    /// or "_" if unused by the callback body.
    pub elem_param: String,
    /// Whether the callback takes an index parameter (2nd param).
    pub has_idx_param: bool,
    /// The callback index parameter name (e.g., "i" from `(x, i) => ...`),
    /// or "_" if present but unused, or "" if no index param.
    pub idx_param: String,
    /// The callback body statements (already lowered to IR).
    /// For concise arrow bodies, this is a single ExpressionStatement.
    pub body: Vec<IrStmt>,
    /// For reduce: the initial accumulator value expression.
    pub reduce_init: Option<IrExpr>,
}

/// Which array non-callback method is being inlined.
#[derive(Debug, Clone, PartialEq)]
pub enum ArrayMethodKind {
    /// `arr.includes(target)` → for loop with == check
    Includes,
    /// `arr.indexOf(target)` → for loop with break on match
    IndexOf,
    /// `arr.lastIndexOf(target)` → backward while loop
    LastIndexOf,
    /// `arr.join(sep)` → std.io.Writer.Allocating
    Join,
    /// `arr.slice([start[, end]])` → ArrayList appendSlice
    Slice,
    /// `arr.splice(start, count[, ...items])` → orderedRemove loop
    Splice,
    /// `arr.at(idx)` → __at_idx with negative index support
    At,
    /// `arr.concat(...arrays)` → ArrayList append
    Concat,
    /// `arr.copyWithin(target, start, end)` → indexed copy loop
    CopyWithin,
    /// `arr.fill(val[, start[, end]])` → for loop elem.* assignment
    Fill,
}

/// Data for inline expansion of array non-callback methods.
#[derive(Debug, Clone)]
pub struct IrArrayMethodInline {
    /// Which method (includes, indexOf, etc.)
    pub kind: ArrayMethodKind,
    /// Name of the array object being operated on (e.g., "arr").
    pub obj_name: String,
    /// The Zig type of array elements (for ArrayList type in slice/concat/splice).
    pub elem_type: ZigType,
    /// Method arguments (already lowered to IR).
    pub args: Vec<IrExpr>,
}

// ═══════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zigir::kinds::DateConstructorKind;

    #[test]
    fn test_ir_module_construction() {
        let m = IrModule::new("main".to_string());
        assert_eq!(m.name, "main");
        assert!(m.imports.is_empty());
        assert!(m.declarations.is_empty());
    }

    #[test]
    fn test_ir_var_decl() {
        let v = IrVarDecl {
            name: IrIdent::new("x"),
            is_const: true,
            zig_type: Some(ZigType::I64),
            init: Some(IrExpr::IntLiteral(42)),
            is_json_parse: false,
            needs_var_suppression: false,
        };
        assert_eq!(v.name.zig_name, "x");
        assert!(v.is_const);
    }

    #[test]
    fn test_ir_fn_decl() {
        let f = IrFnDecl {
            name: IrIdent::new("add"),
            params: vec![
                IrParam {
                    name: IrIdent::new("a"),
                    zig_type: ZigType::I64,
                    is_unused: false,
                },
                IrParam {
                    name: IrIdent::new("b"),
                    zig_type: ZigType::I64,
                    is_unused: false,
                },
            ],
            return_type: ZigType::I64,
            body: IrBlock::new(vec![IrStmt::Return {
                value: Some(IrExpr::Binary {
                    op: BinOp::Add,
                    left: Box::new(IrExpr::Ident(IrIdent::new("a"))),
                    right: Box::new(IrExpr::Ident(IrIdent::new("b"))),
                }),
            }]),
            is_export: true,
            is_async: false,
            can_throw: false,
            is_cabi: false,
        };
        assert_eq!(f.params.len(), 2);
        assert!(f.is_export);
    }

    #[test]
    fn test_ir_if_stmt() {
        let stmt = IrStmt::If {
            cond: IrExpr::BoolLiteral(true),
            then: IrBlock::new(vec![IrStmt::Expr(IrExpr::IntLiteral(1))]),
            else_: Some(IrBlock::new(vec![IrStmt::Expr(IrExpr::IntLiteral(2))])),
        };
        assert!(matches!(stmt, IrStmt::If { .. }));
    }

    #[test]
    fn test_ir_try_catch() {
        let stmt = IrStmt::Try {
            try_block: IrBlock::new(vec![]),
            catch_var: Some(IrIdent::new("e")),
            catch_var_referenced: false,
            catch_block: IrBlock::new(vec![]),
            finally: None,
            has_throw: false,
            has_nested_try: false,
        };
        assert!(matches!(stmt, IrStmt::Try { .. }));
    }

    #[test]
    fn test_ir_call_expr() {
        let call = IrCallExpr {
            callee: Box::new(IrExpr::Ident(IrIdent::new("foo"))),
            args: vec![IrExpr::IntLiteral(1), IrExpr::IntLiteral(2)],
            call_kind: CallKind::Direct,
        };
        assert_eq!(call.args.len(), 2);
    }

    #[test]
    fn test_ir_builtin_call() {
        let bc = IrBuiltinCall {
            module: BuiltinModule::JsArray,
            method: "push".to_string(),
            obj_name: None,
            args: vec![IrExpr::Ident(IrIdent::new("x"))],
            return_type: ZigType::Void,
        };
        assert_eq!(bc.module.module_path(), "js_array");
    }

    #[test]
    fn test_ir_closure() {
        let closure = IrClosure {
            struct_name: IrIdent::new("_closure_0"),
            captured: vec![IrCapture {
                name: IrIdent::new("a"),
                zig_type: ZigType::I64,
                is_mut: false,
            }],
            fn_params: vec![IrParam {
                name: IrIdent::new("b"),
                zig_type: ZigType::I64,
                is_unused: false,
            }],
            return_type: ZigType::I64,
            body: IrBlock::new(vec![]),
            instance_name: IrIdent::new("_cl_0"),
        };
        assert_eq!(closure.captured.len(), 1);
    }

    #[test]
    fn test_ir_new_expr_date() {
        let ne = IrNewExpr {
            constructor: NewConstructor::Date(DateConstructorKind::FromMillis),
            args: vec![IrExpr::IntLiteral(1000)],
            result_type: ZigType::NamedStruct("JsDate".to_string()),
        };
        assert!(matches!(
            ne.constructor,
            NewConstructor::Date(DateConstructorKind::FromMillis)
        ));
    }

    #[test]
    fn test_ir_array_literal_with_spread() {
        let arr = IrArrayLiteral {
            elements: vec![
                IrExpr::IntLiteral(1),
                IrExpr::IntLiteral(2),
                IrExpr::Spread(Box::new(IrExpr::Ident(IrIdent::new("rest")))),
            ],
            spread_indices: vec![2],
        };
        assert_eq!(arr.spread_indices.len(), 1);
    }

    #[test]
    fn test_ir_object_literal() {
        let obj = IrObjectLiteral {
            fields: vec![IrObjectField {
                key: "name".to_string(),
                value: IrExpr::StringLiteral("foo".to_string()),
                is_computed: false,
            }],
            spreads: vec![],
        };
        assert_eq!(obj.fields.len(), 1);
    }

    #[test]
    fn test_ir_arrow_fn_concise() {
        let arrow = IrArrowFn {
            params: vec![IrParam {
                name: IrIdent::new("x"),
                zig_type: ZigType::I64,
                is_unused: false,
            }],
            return_type: ZigType::I64,
            body: IrBlock::new(vec![]),
            is_concise: true,
        };
        assert!(arrow.is_concise);
    }

    #[test]
    fn test_ir_switch_case() {
        let case = IrSwitchCase {
            test: Some(IrExpr::IntLiteral(1)),
            body: vec![IrStmt::Break { label: None }],
        };
        assert!(case.test.is_some());
        assert_eq!(case.body.len(), 1);
    }

    #[test]
    fn test_ir_for_loop() {
        let for_stmt = IrStmt::For {
            label: None,
            init: Some(Box::new(IrStmt::VarDecl(IrVarDecl {
                name: IrIdent::new("i"),
                is_const: true,
                zig_type: Some(ZigType::I64),
                init: Some(IrExpr::IntLiteral(0)),
                is_json_parse: false,
                needs_var_suppression: false,
            }))),
            cond: Some(IrExpr::Binary {
                op: BinOp::Lt,
                left: Box::new(IrExpr::Ident(IrIdent::new("i"))),
                right: Box::new(IrExpr::IntLiteral(10)),
            }),
            update: Some(Box::new(IrStmt::Expr(IrExpr::Update {
                op: UpdateOp::Increment,
                target: Box::new(IrAssignTarget::Ident(IrIdent::new("i"))),
                is_expr_stmt: false,
            }))),
            body: IrBlock::new(vec![]),
        };
        assert!(matches!(for_stmt, IrStmt::For { .. }));
    }

    #[test]
    fn test_ir_class_decl() {
        let cls = IrClassDecl {
            name: IrIdent::new("Foo"),
            fields: vec![IrClassField {
                name: "x".to_string(),
                zig_type: ZigType::I64,
                default: None,
            }],
            constructor: None,
            methods: vec![IrClassMethod {
                name: "getX".to_string(),
                params: vec![],
                return_type: ZigType::I64,
                body: IrBlock::new(vec![]),
                is_static: false,
            }],
            static_inits: vec![],
            extends: None,
        };
        assert_eq!(cls.fields.len(), 1);
        assert_eq!(cls.methods.len(), 1);
    }

    #[test]
    fn test_ir_template_literal() {
        let tl = IrExpr::TemplateLiteral {
            parts: vec!["Hello, ".to_string(), "!".to_string()],
            exprs: vec![IrExpr::Ident(IrIdent::new("name"))],
            format_specs: vec!["{s}".to_string()],
        };
        assert!(matches!(tl, IrExpr::TemplateLiteral { .. }));
    }

    #[test]
    fn test_ir_await_expr() {
        let aw = IrAwaitExpr {
            task_var: IrIdent::new("_t0"),
            callee: Box::new(IrExpr::HostCall(IrHostCall {
                name: "fetch_data".to_string(),
                args: vec![],
                return_type: ZigType::I64,
                is_async: true,
            })),
            args: vec![],
            is_host_async: true,
            block_label: "blk_0".to_string(),
        };
        assert!(aw.is_host_async);
    }

    #[test]
    fn test_ir_assign_target_destructure() {
        let target = IrAssignTarget::Destructure(vec![
            IrDestructureBinding {
                pattern: IrIdent::new("a"),
                default: None,
            },
            IrDestructureBinding {
                pattern: IrIdent::new("b"),
                default: Some(IrExpr::IntLiteral(0)),
            },
        ]);
        if let IrAssignTarget::Destructure(bindings) = target {
            assert_eq!(bindings.len(), 2);
        } else {
            panic!("expected Destructure");
        }
    }
}
