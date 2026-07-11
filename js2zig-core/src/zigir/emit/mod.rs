// zigir/emit/mod.rs
// ZigIR → Zig source code emission:纯格式化，无语义决策。
//
// Emitter 接受 &IrModule，逐节点遍历 IR 树，生成格式化的 Zig 源代码字符串。
// 所有语义决策（类型推断、名称篡改、闭包分析）已在 Lowerer 阶段完成。
// Emitter 只负责将 IR 节点映射为 Zig 代码文本。

pub mod builtins;
pub mod expr;
pub mod helpers;
pub mod stmt;

use crate::zigir::types::{IrDecl, IrModule};

use std::collections::HashSet;

use helpers::EmitterHelpers;

// ═══════════════════════════════════════════════════════
//  Emitter struct
// ═══════════════════════════════════════════════════════

/// ZigIR → Zig source code emitter.
///
/// Generates Zig source code from structured IR instead of AST + inline type lookups. The Emitter is stateless with
/// respect to semantics — all meaning is encoded in the IR nodes.
///
/// Internal state:
/// - `output`: accumulated Zig source buffer
/// - `indent`: current indentation level (4 spaces per level)
pub struct Emitter {
    output: String,
    indent: usize,
    /// When inside a try block, this holds the label that `throw` should break to.
    inside_try_block: Option<String>,
    /// Counter for generating unique try-block labels (_js_try_blk_N).
    try_label_counter: u32,
    /// Counter for generating unique block labels (for array literal labeled blocks).
    label_counter: u32,
    /// Buffer for static block init code. When non-empty, `emit_module_inner`
    /// generates a `pub fn init_js2rust() !void { ... }` at the end of the
    /// module so the orchestrator's `init_js2rust()` will call it, ensuring
    /// static blocks execute at runtime (Zig's lazy analysis would otherwise
    /// skip unreferenced top-level `const` declarations).
    static_init_buffer: String,
    /// Buffer for static deinit code. When non-empty, `emit_module_inner`
    /// generates a `pub fn deinit_js2rust() void { ... }` that calls
    /// `.deinit(alloc)` on all static Map/Set/ArrayList fields.
    static_deinit_buffer: String,
    /// Set of class names that have `needs_deinit = true` (contain Map/Set/ArrayList fields).
    /// Used by `emit_var_decl` to determine whether a local variable of a NamedStruct
    /// type should get `defer varname.deinit(alloc)`.
    class_needs_deinit: HashSet<String>,
}

// ── EmitterHelpers trait implementation ───────────────

impl EmitterHelpers for Emitter {
    fn output(&self) -> &str {
        &self.output
    }
    fn output_mut(&mut self) -> &mut String {
        &mut self.output
    }
    fn indent(&self) -> usize {
        self.indent
    }
    fn indent_mut(&mut self) -> &mut usize {
        &mut self.indent
    }
}

impl Default for Emitter {
    fn default() -> Self {
        Self::new()
    }
}

impl Emitter {
    pub fn new() -> Self {
        Self {
            output: String::new(),
            indent: 0,
            inside_try_block: None,
            try_label_counter: 0,
            label_counter: 0,
            static_init_buffer: String::new(),
            static_deinit_buffer: String::new(),
            class_needs_deinit: HashSet::new(),
        }
    }

    /// Emit an IrExpr to a separate string (for inline embedding in templates).
    /// Temporarily swaps the output buffer so the expression is captured alone.
    pub(crate) fn emit_expr_inline(expr: &crate::zigir::types::IrExpr) -> String {
        let mut sub_emitter = Self::new();
        sub_emitter.emit_expr(expr);
        sub_emitter.output.trim().to_string()
    }

    /// Like `emit_expr_inline`, but initializes the sub-emitter's `label_counter`
    /// to `label_offset` instead of 0, ensuring that any labeled blocks generated
    /// inside the sub-expression use label numbers ≥ `label_offset`.
    /// Returns `(rendered_string, updated_label_counter)`.
    pub(crate) fn emit_expr_inline_with_label_offset(
        expr: &crate::zigir::types::IrExpr,
        label_offset: u32,
    ) -> (String, u32) {
        let mut sub_emitter = Self::new();
        sub_emitter.label_counter = label_offset;
        sub_emitter.emit_expr(expr);
        let new_counter = sub_emitter.label_counter;
        (sub_emitter.output.trim().to_string(), new_counter)
    }

    /// Emit a complete IrModule to a Zig source string.
    pub fn emit_module(module: &IrModule) -> String {
        let mut emitter = Self::new();
        emitter.emit_module_inner(module);
        emitter.output
    }

    fn emit_module_inner(&mut self, module: &IrModule) {
        // 1. Emit closure struct definitions (prepended before declarations).
        //    Closure struct definitions are emitted before declarations.
        for closure_struct in &module.closure_structs {
            self.emit_closure_struct(closure_struct);
        }

        // 2. Emit JSDoc @typedef struct definitions.
        for typedef in &module.typedefs {
            self.emit_typedef(typedef);
        }

        // 3. Emit top-level declarations (functions, variables, classes).
        for decl in &module.declarations {
            self.emit_decl(decl);
        }

        // 4. If any class had static {} blocks, generate init_js2rust()
        //    so the orchestrator calls it from root init_js2rust().
        //    This ensures static blocks execute at runtime rather than being
        //    skipped by Zig's lazy analysis of unreferenced top-level `const`.
        if !self.static_init_buffer.is_empty() {
            self.writeln("pub fn init_js2rust() !void {");
            self.indent_push();
            let buf = std::mem::take(&mut self.static_init_buffer);
            self.write(&buf);
            self.indent_pop();
            self.writeln("}");
        }

        // 5. If any static Map/Set/ArrayList fields need deinit, generate
        //    deinit_js2rust() so the orchestrator calls it from root deinit_js2rust().
        if !self.static_deinit_buffer.is_empty() {
            self.writeln("pub fn deinit_js2rust() void {");
            self.indent_push();
            let buf = std::mem::take(&mut self.static_deinit_buffer);
            self.write(&buf);
            self.indent_pop();
            self.writeln("}");
        }
    }

    // ── Declaration dispatch ─────────────────────────

    fn emit_decl(&mut self, decl: &IrDecl) {
        match decl {
            IrDecl::Var(var_decl) => self.emit_var_decl(var_decl),
            IrDecl::Fn(fn_decl) => self.emit_fn_decl(fn_decl),
            IrDecl::Class(class_decl) => self.emit_class_decl(class_decl),
            IrDecl::CompileError { span: _, msg } => {
                // Toplevel "errors" are emitted as comments (soft diagnostics),
                // not @compileError — soft diagnostics.
                if msg.starts_with("toplevel") || msg.starts_with("skipped unused") {
                    self.writeln(&format!("// error: {}", msg));
                } else {
                    self.writeln(&format!("@compileError(\"{}\");", msg));
                }
            }
        }
    }

    /// Return the next try-label id and advance the counter.
    fn next_try_label(&mut self) -> u32 {
        let n = self.try_label_counter;
        self.try_label_counter += 1;
        n
    }

    /// Return the next block label (e.g., `blk_0`, `blk_1`) and advance the counter.
    fn next_label(&mut self) -> String {
        let n = self.label_counter;
        self.label_counter += 1;
        format!("blk_{}", n)
    }

    /// Peek at the current label counter without advancing (for generating unique temp var names).
    fn peek_label_id(&self) -> u32 {
        self.label_counter
    }
}

// ═══════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ZigType;
    use crate::zigir::ident::IrIdent;
    use crate::zigir::types::{IrBlock, IrDecl, IrFnDecl, IrTypedef, IrTypedefField, IrVarDecl};

    #[test]
    fn test_emit_empty_module() {
        let module = IrModule::new("test".to_string());
        let output = Emitter::emit_module(&module);
        assert!(output.is_empty());
    }

    #[test]
    fn test_emit_typedef() {
        let mut module = IrModule::new("test".to_string());
        module.typedefs.push(IrTypedef {
            name: "Point".to_string(),
            fields: vec![
                IrTypedefField {
                    name: "x".to_string(),
                    zig_type: "f64".to_string(),
                    optional: false,
                },
                IrTypedefField {
                    name: "y".to_string(),
                    zig_type: "f64".to_string(),
                    optional: false,
                },
            ],
            is_opaque: false,
            has_to_json: true,
        });
        let output = Emitter::emit_module(&module);
        assert!(output.contains("const Point = struct {"));
        assert!(output.contains("x: f64,"));
        assert!(output.contains("y: f64,"));
        assert!(output.contains("pub fn toJson"));
    }

    #[test]
    fn test_emit_var_decl() {
        let mut module = IrModule::new("test".to_string());
        module.declarations.push(IrDecl::Var(IrVarDecl::new_const(
            "x",
            Some(ZigType::I64),
            Some(crate::zigir::types::IrExpr::IntLiteral(42)),
        )));
        let output = Emitter::emit_module(&module);
        assert!(output.contains("const x = 42"));
    }

    #[test]
    fn test_emit_fn_decl() {
        let module = crate::zigir::passes::make_clean_add_module();
        let output = Emitter::emit_module(&module);
        assert!(output.contains("pub fn add(a: i64, b: i64) i64"));
    }

    #[test]
    fn test_emit_fn_decl_with_throw() {
        let mut module = IrModule::new("test".to_string());
        module.declarations.push(IrDecl::Fn(IrFnDecl {
            name: IrIdent::new("mayFail"),
            params: vec![],
            return_type: ZigType::I64,
            body: IrBlock::new(vec![]),
            is_export: false,
            is_async: false,
            can_throw: true,
            is_cabi: false,
            typeof_return_body: None,
        }));
        let output = Emitter::emit_module(&module);
        assert!(output.contains("fn mayFail() !i64 {"));
    }

    #[test]
    fn test_emit_class_with_deinit() {
        use crate::zigir::types::{IrClassDecl, IrClassField, IrClassMethod};

        let mut module = IrModule::new("test".to_string());
        // Class with a Map field → needs_deinit = true → generates deinit() method
        module.declarations.push(IrDecl::Class(IrClassDecl {
            name: IrIdent::new("Cache"),
            fields: vec![
                IrClassField {
                    name: "data".to_string(),
                    zig_type: ZigType::NamedStruct("Map".to_string()),
                    default: None,
                },
                IrClassField {
                    name: "name".to_string(),
                    zig_type: ZigType::I64,
                    default: None,
                },
            ],
            constructor: None,
            methods: vec![IrClassMethod {
                name: "get".to_string(),
                params: vec![],
                return_type: ZigType::Void,
                body: IrBlock::new(vec![]),
                is_static: false,
            }],
            static_inits: vec![],
            static_blocks: vec![],
            extends: None,
            needs_deinit: true,
        }));
        let output = Emitter::emit_module(&module);
        assert!(output.contains("pub fn deinit(self: *@This(), alloc: std.mem.Allocator) void {"));
        assert!(output.contains("self.data.deinit(alloc);"));
        // I64 field should NOT appear in deinit body
        let deinit_start = output.find("pub fn deinit").unwrap();
        let deinit_end = output[deinit_start..].find('}').unwrap() + deinit_start;
        let deinit_body = &output[deinit_start..deinit_end];
        assert!(!deinit_body.contains("self.name.deinit"));
    }

    #[test]
    fn test_emit_class_without_deinit() {
        use crate::zigir::types::{IrClassDecl, IrClassField};

        let mut module = IrModule::new("test".to_string());
        // Class with only I64 fields → needs_deinit = false → no deinit() method
        module.declarations.push(IrDecl::Class(IrClassDecl {
            name: IrIdent::new("Point"),
            fields: vec![
                IrClassField {
                    name: "x".to_string(),
                    zig_type: ZigType::I64,
                    default: None,
                },
                IrClassField {
                    name: "y".to_string(),
                    zig_type: ZigType::I64,
                    default: None,
                },
            ],
            constructor: None,
            methods: vec![],
            static_inits: vec![],
            static_blocks: vec![],
            extends: None,
            needs_deinit: false,
        }));
        let output = Emitter::emit_module(&module);
        assert!(!output.contains("pub fn deinit"));
    }

    #[test]
    fn test_emit_map_var_auto_cleanup() {
        let mut module = IrModule::new("test".to_string());
        module.declarations.push(IrDecl::Var(IrVarDecl {
            name: IrIdent::new("m"),
            is_const: false,
            zig_type: Some(ZigType::NamedStruct("Map".to_string())),
            init: None,
            is_json_parse: false,
            needs_var_suppression: true,
            needs_const_suppression: false,
            needs_deinit: true,
        }));
        let output = Emitter::emit_module(&module);
        assert!(output.contains("defer m.deinit(js_allocator.allocator());"));
    }

    #[test]
    fn test_emit_map_var_returned_no_deinit() {
        // When needs_deinit is false (ownership transferred via return),
        // no defer deinit should be emitted.
        let mut module = IrModule::new("test".to_string());
        module.declarations.push(IrDecl::Var(IrVarDecl {
            name: IrIdent::new("m"),
            is_const: true,
            zig_type: Some(ZigType::NamedStruct("Map".to_string())),
            init: None,
            is_json_parse: false,
            needs_var_suppression: false,
            needs_const_suppression: false,
            needs_deinit: false, // cleared by ownership transfer pass
        }));
        let output = Emitter::emit_module(&module);
        assert!(!output.contains("defer m.deinit"));
    }

    #[test]
    fn test_emit_static_map_field_deinit() {
        use crate::zigir::types::{IrClassDecl, IrClassField};

        let mut module = IrModule::new("test".to_string());
        // Class with static Map field → deinit_js2rust() generated
        module.declarations.push(IrDecl::Class(IrClassDecl {
            name: IrIdent::new("Registry"),
            fields: vec![IrClassField {
                name: "name".to_string(),
                zig_type: ZigType::I64,
                default: None,
            }],
            constructor: None,
            methods: vec![],
            static_inits: vec![(
                "entries".to_string(),
                crate::zigir::types::IrExpr::Ident(IrIdent::new("Map")),
                ZigType::NamedStruct("Map".to_string()),
            )],
            static_blocks: vec![],
            extends: None,
            needs_deinit: false,
        }));
        let output = Emitter::emit_module(&module);
        assert!(output.contains("pub fn deinit_js2rust() void {"));
        assert!(output.contains("__Registry_entries.deinit(js_allocator.allocator());"));
    }
}
