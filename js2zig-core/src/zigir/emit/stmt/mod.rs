// zigir/emit/stmt/mod.rs
// Statement-level Zig emission: dispatch and inline cases.

pub mod control_flow;
pub mod decl;
pub mod destructure_assign;

use crate::zigir::emit::Emitter;
use crate::zigir::emit::helpers::{EmitterHelpers, escape_zig_string};
use crate::zigir::types::{IrBlock, IrStmt};
use control_flow::{ForOfInfo, TryInfo};

impl Emitter {
    /// Emit all statements in a block (without the label — label is emitted by the caller).
    pub(super) fn emit_block_stmts_unlabeled(&mut self, block: &IrBlock) {
        for stmt in &block.stmts {
            self.emit_stmt(stmt);
        }
    }

    /// Emit a single statement.
    pub(crate) fn emit_stmt(&mut self, stmt: &IrStmt) {
        match stmt {
            IrStmt::VarDecl(var_decl) => self.emit_var_decl(var_decl),

            IrStmt::Assign { target, op, value } => {
                self.emit_assign_stmt(target, *op, value);
            }

            IrStmt::If { cond, then, else_ } => {
                self.emit_if_stmt(cond, then, else_);
            }

            IrStmt::While { cond, body, label } => {
                self.emit_while_stmt(cond, body, label);
            }

            IrStmt::DoWhile { body, cond, label } => {
                self.emit_do_while_stmt(body, cond, label);
            }

            IrStmt::For {
                init,
                cond,
                update,
                body,
                label,
            } => {
                self.emit_for_stmt(init, cond, update, body, label);
            }

            IrStmt::ForIn {
                var,
                iterable,
                body,
                kind,
                label,
            } => {
                self.emit_for_in_stmt(var, iterable, body, kind, label);
            }

            IrStmt::ForOf {
                var,
                destructure_vars,
                iterable,
                iterable_is_arraylist,
                body,
                kind,
                is_async,
                label,
            } => {
                self.emit_for_of_stmt(&ForOfInfo {
                    var,
                    destructure_vars,
                    iterable,
                    iterable_is_arraylist: *iterable_is_arraylist,
                    body,
                    kind,
                    is_async: *is_async,
                    label,
                });
            }

            IrStmt::Switch { expr, cases } => {
                self.emit_switch_stmt(expr, cases);
            }

            IrStmt::Try {
                try_block,
                catch_var,
                catch_var_referenced,
                catch_block,
                finally,
                has_throw,
                has_nested_try,
            } => {
                self.emit_try_stmt(&TryInfo {
                    try_block,
                    catch_var,
                    catch_var_referenced: *catch_var_referenced,
                    finally,
                    has_throw: *has_throw,
                    has_nested_try: *has_nested_try,
                    catch_block,
                });
            }

            IrStmt::Throw { value, error_name } => {
                // Evaluate throw value for side effects, then break/return error.
                self.write_indent();
                self.write("_ = ");
                self.emit_expr(value);
                self.write(";\n");

                let err = error_name.as_deref().unwrap_or("JsThrow");
                let try_label = self.inside_try_block.clone();
                if let Some(label) = try_label {
                    // Inside try block: break to the labeled block with error
                    self.writeln(&format!(
                        "break :{} @as(anyerror!void, error.{});",
                        label, err,
                    ));
                } else {
                    // Outside try block: return error from function
                    self.writeln(&format!("return error.{};", err));
                }
            }

            IrStmt::Return { value } => {
                self.write_indent();
                if let Some(val) = value {
                    self.write("return ");
                    self.emit_expr(val);
                    self.write(";\n");
                } else {
                    self.write("return;\n");
                }
            }

            IrStmt::Break { label } => {
                self.write_indent();
                if let Some(l) = label {
                    self.write(&format!("break :{};\n", l));
                } else {
                    self.write("break;\n");
                }
            }

            IrStmt::Continue { label } => {
                self.write_indent();
                if let Some(l) = label {
                    self.write(&format!("continue :{};\n", l));
                } else {
                    self.write("continue;\n");
                }
            }

            IrStmt::Expr(expr) => {
                // ArrayCallbackInline with forEach/Map/Set generates its own
                // statement-level output — no _ = prefix or ; suffix needed.
                let is_self_contained = matches!(
                    expr,
                    crate::zigir::types::IrExpr::ArrayCallbackInline(inline_data)
                        if matches!(inline_data.kind, crate::zigir::types::ArrayCallbackKind::ForEach)
                );

                if is_self_contained {
                    self.write_indent();
                    self.emit_expr(expr);
                    self.write("\n");
                } else {
                    self.write_indent();
                    // Assignment and Update (is_expr_stmt=true) don't need `_ = ` prefix
                    // — they are statements in Zig. Other expressions need `_ = ` to
                    // discard non-void return values.
                    let needs_discard = !matches!(
                        expr,
                        crate::zigir::types::IrExpr::Assign { .. }
                            | crate::zigir::types::IrExpr::Update {
                                is_expr_stmt: true,
                                ..
                            }
                    );
                    if needs_discard {
                        self.write("_ = ");
                    }
                    self.emit_expr(expr);
                    self.write(";\n");
                }
            }

            IrStmt::Block(block) => {
                if block.transparent {
                    // Transparent block: emit children flat at current indent
                    // without {} braces (used for multi-declarator variable
                    // declarations that must not introduce a new scope).
                    for stmt in &block.stmts {
                        self.emit_stmt(stmt);
                    }
                } else {
                    // Emit label before opening brace if present (Zig requires `label: { ... }`)
                    if let Some(label) = &block.label {
                        self.writeln(&format!("{}: {{", label));
                    } else {
                        self.writeln("{");
                    }
                    self.indent_push();
                    self.emit_block_stmts_unlabeled(block);
                    self.indent_pop();
                    self.writeln("}");
                }
            }

            IrStmt::CompileError { span, msg } => {
                let loc = format!("{}:{}", span.js_line, span.js_col);
                self.writeln(&format!(
                    "@compileError(\"{} (at {})\");",
                    escape_zig_string(msg),
                    loc
                ));
            }

            IrStmt::Comment(text) => {
                self.writeln(&format!("// {}", text));
            }

            IrStmt::DestructureDecl(data) => {
                self.emit_destructure_decl(data);
            }

            IrStmt::NestedFnDecl {
                struct_def,
                instance,
            } => {
                self.emit_closure_struct(struct_def);
                if let Some(closure) = instance {
                    self.write_indent();
                    self.write(&format!("const {} = ", closure.instance_name.zig_name));
                    self.emit_expr(&crate::zigir::types::IrExpr::Closure(closure.clone()));
                    self.write(";\n");
                }
            }
        }
    }
}
