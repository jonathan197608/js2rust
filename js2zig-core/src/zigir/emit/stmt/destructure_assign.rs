// zigir/emit/stmt/destructure_assign.rs
// Destructuring declaration and assignment statement emission.

use crate::zigir::emit::Emitter;
use crate::zigir::emit::helpers::EmitterHelpers;
use crate::zigir::ops::AssignOp;
use crate::zigir::types::{IrAssignTarget, IrDestructureDecl};

impl Emitter {
    pub(super) fn emit_destructure_decl(&mut self, data: &IrDestructureDecl) {
        use crate::zigir::types::{IrDestructureAccess, IrDestructureKind};

        // Step 1: Emit temp variable if needed
        if let Some(temp) = &data.temp_name {
            self.write_indent();
            self.write("const ");
            self.write(temp);
            self.write(" = ");
            self.emit_expr(&data.init);
            self.writeln(";");
        }

        // Step 2: Emit each binding declaration
        for binding in &data.bindings {
            let kw = if binding.is_const { "const" } else { "var" };
            self.write_indent();
            self.write(&format!("{} {} = ", kw, binding.name.zig_name));

            match &binding.access {
                IrDestructureAccess::ObjectField {
                    source,
                    key,
                    is_struct_field,
                } => {
                    match &data.kind {
                        IrDestructureKind::Object { is_struct, .. } => {
                            if *is_struct && *is_struct_field {
                                // Struct with known field: direct field access
                                self.write(&format!("{}.{}", source, key));
                            } else if *is_struct && !is_struct_field {
                                // Struct but field not found
                                if let Some(default) = &binding.default {
                                    // Use default directly
                                    self.emit_expr(default);
                                } else {
                                    // Error: field not in struct and no default
                                    self.write(&format!(
                                        "/* error: key '{}' not in struct */",
                                        key
                                    ));
                                }
                            } else {
                                // HashMap / unknown: use .get("key")
                                if let Some(default) = &binding.default {
                                    // Type-aware conversion: .asBool(), .asI64(), .value.string
                                    let conv = self.infer_hashmap_conv(default);
                                    self.write(&format!(
                                        "if ({}.get(\"{}\")) |v| v{} else ",
                                        source, key, conv
                                    ));
                                    self.emit_expr(default);
                                } else {
                                    // No default: raw .get() returns ?JsAny
                                    self.write(&format!("{}.get(\"{}\")", source, key));
                                }
                            }
                        }
                        IrDestructureKind::Array { .. } => {
                            // Shouldn't happen — object access in array destructure
                            self.write(&format!("{}.{}", source, key));
                        }
                    }
                }
                IrDestructureAccess::ArrayIndex { source, index } => {
                    match &data.kind {
                        IrDestructureKind::Array { is_arraylist } => {
                            if *is_arraylist {
                                // ArrayList: bounds-safe .items[i] access
                                if let Some(default) = &binding.default {
                                    self.write(&format!(
                                        "if ({}.items.len > {}) {}.items[{}] else ",
                                        source, index, source, index
                                    ));
                                    self.emit_expr(default);
                                } else {
                                    self.write(&format!("{}.items[{}]", source, index));
                                }
                            } else {
                                // Slice/array: direct [i] access
                                self.write(&format!("{}[{}]", source, index));
                                if let Some(default) = &binding.default {
                                    self.write(" orelse ");
                                    self.emit_expr(default);
                                }
                            }
                        }
                        IrDestructureKind::Object { .. } => {
                            self.write(&format!("{}[{}]", source, index));
                        }
                    }
                }
            }

            self.writeln(";");
        }
    }

    /// Infer the HashMap value conversion method based on the default expression type.
    /// Returns ".asBool()" for bool defaults, ".value.string" for string defaults,
    /// and ".asI64()" for numeric defaults.
    pub(super) fn infer_hashmap_conv(&self, default: &crate::zigir::types::IrExpr) -> &'static str {
        use crate::zigir::types::IrExpr;
        match default {
            IrExpr::BoolLiteral(_) => ".asBool()",
            IrExpr::StringLiteral(_) => ".value.string",
            _ => ".asI64()",
        }
    }

    pub(super) fn emit_assign_stmt(
        &mut self,
        target: &IrAssignTarget,
        op: AssignOp,
        value: &crate::zigir::types::IrExpr,
    ) {
        self.write_indent();
        self.emit_assign_inline(target, op, value);
        self.write(";\n");
    }

    /// Emit assignment inline (no indent, no semicolon, no newline).
    /// Used in while continuation blocks where the assignment is part of
    /// a single-line expression like `: ({ i += 1; })`.
    pub(super) fn emit_assign_inline(
        &mut self,
        target: &IrAssignTarget,
        op: AssignOp,
        value: &crate::zigir::types::IrExpr,
    ) {
        self.emit_compound_assign(target, op, value);
    }
}
