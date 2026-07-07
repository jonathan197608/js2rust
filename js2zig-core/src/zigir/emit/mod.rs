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
    /// Counter for generating unique static init/block names (_static_init_N).
    static_init_counter: u32,
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
            static_init_counter: 0,
        }
    }

    /// Emit an IrExpr to a separate string (for inline embedding in templates).
    /// Temporarily swaps the output buffer so the expression is captured alone.
    pub(crate) fn emit_expr_inline(expr: &crate::zigir::types::IrExpr) -> String {
        let mut sub_emitter = Self::new();
        sub_emitter.emit_expr(expr);
        sub_emitter.output.trim().to_string()
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
    use crate::zigir::types::{
        IrBlock, IrFnDecl, IrParam, IrStmt, IrTypedef, IrTypedefField, IrVarDecl,
    };

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
        module.declarations.push(IrDecl::Var(IrVarDecl {
            name: IrIdent::new("x"),
            is_const: true,
            zig_type: Some(ZigType::I64),
            init: Some(crate::zigir::types::IrExpr::IntLiteral(42)),
            is_json_parse: false,
            needs_var_suppression: false,
        }));
        let output = Emitter::emit_module(&module);
        assert!(output.contains("const x = 42"));
    }

    #[test]
    fn test_emit_fn_decl() {
        let mut module = IrModule::new("test".to_string());
        module.declarations.push(IrDecl::Fn(IrFnDecl {
            name: IrIdent::new("add"),
            params: vec![
                IrParam {
                    name: IrIdent::new("a"),
                    zig_type: ZigType::I64,
                    is_unused: false,
                    is_rest: false,
                },
                IrParam {
                    name: IrIdent::new("b"),
                    zig_type: ZigType::I64,
                    is_unused: false,
                    is_rest: false,
                },
            ],
            return_type: ZigType::I64,
            body: IrBlock::new(vec![IrStmt::Return {
                value: Some(crate::zigir::types::IrExpr::Binary {
                    op: crate::zigir::ops::BinOp::Add,
                    left: Box::new(crate::zigir::types::IrExpr::Ident(IrIdent::new("a"))),
                    right: Box::new(crate::zigir::types::IrExpr::Ident(IrIdent::new("b"))),
                    left_type: None,
                    right_type: None,
                }),
            }]),
            is_export: true,
            is_async: false,
            can_throw: false,
            is_cabi: false,
            typeof_return_body: None,
        }));

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
}
