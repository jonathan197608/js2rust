// zigir/lower/cabi.rs
// C ABI export metadata and utility/query methods.

use oxc_ast::ast::*;

use crate::types::ZigType;
use crate::zigir::builtins::BuiltinModule;
use crate::zigir::ident::IrIdent;
use crate::zigir::source_span::{DiagnosticLevel, IrDiagnostic, SourceSpan};
use crate::zigir::types::{IrCabiExport, IrDecl, IrExpr, IrParam};

use super::Lowerer;
use super::helpers::FnContext;

// ï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½T
//  CABI export metadata
// ï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½T

impl Lowerer {
    /// Build C ABI export metadata from the lowered declarations.
    ///
    /// Corresponds to the cabi_exports construction in old
    /// `native_proto::transpile_js_inner()`.
    pub(super) fn build_cabi_exports(&self, declarations: &[IrDecl]) -> Vec<IrCabiExport> {
        let mut exports = Vec::new();
        for decl in declarations {
            if let IrDecl::Fn(f) = decl
                && f.is_export
            {
                // Keep all params including Anytype — pipeline will detect them
                // and route to emit_const_alias instead of C ABI wrapper.
                let params: Vec<IrParam> = f
                    .params
                    .iter()
                    .map(|p| IrParam {
                        name: p.name.clone(),
                        zig_type: p.zig_type.clone(),
                        is_unused: p.is_unused,
                        is_rest: false,
                    })
                    .collect();
                let ret_struct_name = if let ZigType::NamedStruct(ref s) = f.return_type {
                    Some(s.clone())
                } else {
                    None
                };
                exports.push(IrCabiExport {
                    name: f.name.zig_name.clone(),
                    params,
                    return_type: f.return_type.clone(),
                    is_async: f.is_async,
                    can_throw: f.can_throw,
                    ret_struct_name,
                });
            }
        }
        exports
    }
}

// ── Utility methods ──

type CallbackInlineParts = (
    crate::zigir::types::ArrayCallbackKind,
    String,
    bool,
    String,
    Vec<crate::zigir::types::IrStmt>,
    Option<IrExpr>,
);

impl Lowerer {
    /// Check whether a function name is in the exported set.
    pub(super) fn is_export_fn(&self, fn_name: Option<&str>) -> bool {
        if let Some(ref exported) = self.exported_functions {
            fn_name.is_some_and(|name| exported.contains(name))
        } else {
            false
        }
    }

    /// Convert a `Span` to an `IrDiagnostic` with source location.
    pub(super) fn span_to_source_span(&self, span: Span) -> SourceSpan {
        let offset = span.start as usize;
        let mut line: usize = 1;
        let mut col: usize = 1;
        for (i, ch) in self.source.char_indices() {
            if i >= offset {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
        SourceSpan {
            js_line: line,
            js_col: col,
            js_file: String::new(),
        }
    }

    /// Add an error diagnostic.
    pub(super) fn add_error(&mut self, span: SourceSpan, msg: impl Into<String>) {
        self.diagnostics.push(IrDiagnostic {
            level: DiagnosticLevel::Error,
            span: Some(span),
            message: msg.into(),
        });
    }

    /// Build an IrExpr::CompileError from a Span and message.
    /// Convenience wrapper that converts Span → SourceSpan automatically.
    pub(super) fn compile_error_expr(&self, span: Span, msg: impl Into<String>) -> IrExpr {
        IrExpr::CompileError {
            span: self.span_to_source_span(span),
            msg: msg.into(),
        }
    }

    /// Create an IrIdent for the given JS name, applying shadow renaming.
    pub(super) fn make_ident(&self, js_name: &str) -> IrIdent {
        self.name_mangler.make_ident(js_name)
    }

    /// Build IrCapture list from raw capture tuples (name, zig_type, is_mut).
    /// Shared by closure.rs, decl.rs, and function.rs.
    pub(super) fn make_ir_captures(
        &self,
        captures: Vec<(String, ZigType, bool)>,
    ) -> Vec<crate::zigir::types::IrCapture> {
        captures
            .into_iter()
            .map(|(name, zig_type, is_mut)| crate::zigir::types::IrCapture {
                name: self.make_ident(&name),
                zig_type,
                is_mut,
            })
            .collect()
    }

    /// Detect nested closure capture: returns the names of captured variables
    /// that are fields on the enclosing closure (i.e. in `current_captured`).
    pub(super) fn detect_nested_captures(
        &self,
        captures: &[(String, ZigType, bool)],
    ) -> Vec<String> {
        captures
            .iter()
            .filter(|(name, _, _)| {
                self.closure_mgr
                    .current_captured
                    .iter()
                    .any(|(n, _, _)| n == name)
            })
            .map(|(name, _, _)| name.clone())
            .collect()
    }

    pub(super) fn lower_args<'a>(
        &mut self,
        args: &oxc_allocator::Vec<'a, Argument<'a>>,
    ) -> Vec<IrExpr> {
        args.iter()
            .map(|arg| match arg {
                Argument::SpreadElement(se) => {
                    IrExpr::Spread(Box::new(self.lower_expr(&se.argument)))
                }
                _ => {
                    let expr = arg
                        .as_expression()
                        .expect("non-SpreadElement Argument is always an Expression");
                    self.lower_expr(expr)
                }
            })
            .collect()
    }

    /// Try to inline an array non-callback method (includes, indexOf, lastIndexOf,
    /// join, slice, splice, at, concat, copyWithin, fill) when we have the
    /// object variable name. Returns `IrExpr::ArrayMethodInline` if inlinable.
    pub(super) fn try_inline_array_method(
        &self,
        ce: &CallExpression,
        builtin: &crate::native_builtins::BuiltinCall,
        args: &[IrExpr],
    ) -> Option<IrExpr> {
        use crate::zigir::types::{IrArrayMethodInline, IrExpr};

        // Never inline array methods for string variables — these should go through
        // BuiltinCall (JsString) instead. Check variable type from TypeInfo.
        let obj_name_raw = Self::extract_callee_object_name_static(&ce.callee);
        if let Some(name) = &obj_name_raw
            && let Some(var_type) = self.type_info.var_types.get(name.as_str())
        {
            if matches!(var_type, ZigType::Str) {
                return None;
            }
            if let ZigType::NamedStruct(n) = var_type
                && Self::is_typedarray_type(n)
            {
                return None;
            }
        }
        let obj_name = obj_name_raw?;

        let kind = Self::resolve_method_kind(builtin)?;

        let elem_type = self
            .type_info
            .array_element_types
            .get(obj_name.as_str())
            .cloned()
            .unwrap_or(ZigType::JsAny);

        Some(IrExpr::ArrayMethodInline(Box::new(IrArrayMethodInline {
            kind,
            obj_name,
            obj_expr: None,
            elem_type,
            args: args.to_vec(),
        })))
    }

    /// Shared logic: resolve callback kind from BuiltinCall.
    fn resolve_callback_kind(
        builtin: &crate::native_builtins::BuiltinCall,
    ) -> Option<crate::zigir::types::ArrayCallbackKind> {
        use crate::native_builtins::BuiltinCall as BC;
        use crate::zigir::types::ArrayCallbackKind;
        match builtin {
            BC::ArrayForEach | BC::SetForEach => Some(ArrayCallbackKind::ForEach),
            BC::ArraySome => Some(ArrayCallbackKind::Some),
            BC::ArrayEvery => Some(ArrayCallbackKind::Every),
            BC::ArrayFilter => Some(ArrayCallbackKind::Filter),
            BC::ArrayFind => Some(ArrayCallbackKind::Find),
            BC::ArrayFindIndex => Some(ArrayCallbackKind::FindIndex),
            BC::ArrayFindLast => Some(ArrayCallbackKind::FindLast),
            BC::ArrayFindLastIndex => Some(ArrayCallbackKind::FindLastIndex),
            BC::ArrayMap => Some(ArrayCallbackKind::Map),
            BC::ArrayReduce => Some(ArrayCallbackKind::Reduce),
            BC::ArrayReduceRight => Some(ArrayCallbackKind::ReduceRight),
            BC::ArraySort => Some(ArrayCallbackKind::Sort),
            BC::ArrayToSorted => Some(ArrayCallbackKind::ToSorted),
            BC::ArrayFlatMap => Some(ArrayCallbackKind::FlatMap),
            _ => None,
        }
    }

    /// Shared logic: resolve method kind from BuiltinCall.
    fn resolve_method_kind(
        builtin: &crate::native_builtins::BuiltinCall,
    ) -> Option<crate::zigir::types::ArrayMethodKind> {
        use crate::native_builtins::BuiltinCall as BC;
        use crate::zigir::types::ArrayMethodKind;
        match builtin {
            BC::ArrayIncludes => Some(ArrayMethodKind::Includes),
            BC::ArrayIndexOf => Some(ArrayMethodKind::IndexOf),
            BC::ArrayLastIndexOf => Some(ArrayMethodKind::LastIndexOf),
            BC::ArrayJoin => Some(ArrayMethodKind::Join),
            BC::ArraySlice => Some(ArrayMethodKind::Slice),
            BC::ArraySplice => Some(ArrayMethodKind::Splice),
            BC::ArrayAt => Some(ArrayMethodKind::At),
            BC::ArrayConcat => Some(ArrayMethodKind::Concat),
            BC::ArrayCopyWithin => Some(ArrayMethodKind::CopyWithin),
            BC::ArrayFill => Some(ArrayMethodKind::Fill),
            BC::ArrayWith => Some(ArrayMethodKind::With),
            BC::ArrayToReversed => Some(ArrayMethodKind::ToReversed),
            BC::ArrayToSorted => Some(ArrayMethodKind::ToSorted),
            BC::ArrayToSpliced => Some(ArrayMethodKind::ToSpliced),
            _ => None,
        }
    }

    /// Shared logic: parse callback parameters and lower the body from a CallExpression.
    /// Returns (kind, elem_param, has_idx_param, idx_param, ir_body, reduce_init) or None.
    fn parse_callback_inline(
        &mut self,
        ce: &CallExpression,
        kind: crate::zigir::types::ArrayCallbackKind,
    ) -> Option<CallbackInlineParts> {
        use crate::zigir::types::ArrayCallbackKind;

        let first_arg = ce.arguments.first()?.as_expression()?;

        let (params, body) = match first_arg {
            Expression::ArrowFunctionExpression(arrow) => (&arrow.params, &arrow.body),
            Expression::FunctionExpression(f) => match &f.body {
                Some(b) => (&f.params, b),
                None => return None,
            },
            _ => return None,
        };

        let elem_param_raw = params
            .items
            .first()
            .and_then(|p| crate::infer::binding_name(&p.pattern))
            .unwrap_or("_")
            .to_string();
        let idx_param_raw = params
            .items
            .get(1)
            .and_then(|p| crate::infer::binding_name(&p.pattern));
        let has_idx_param = idx_param_raw.is_some();

        // Check if parameters are actually used in the callback body.
        let elem_used = body
            .statements
            .iter()
            .any(|s| Self::ast_stmt_uses_ident(&elem_param_raw, s));
        let elem_param = if elem_used {
            elem_param_raw
        } else {
            "_".to_string()
        };

        let idx_param = if let Some(idx_name) = idx_param_raw {
            if idx_name != "_"
                && body
                    .statements
                    .iter()
                    .any(|s| Self::ast_stmt_uses_ident(idx_name, s))
            {
                idx_name.to_string()
            } else {
                "_".to_string()
            }
        } else {
            String::new()
        };

        // Lower the callback body
        let ir_body: Vec<crate::zigir::types::IrStmt> =
            body.statements.iter().map(|s| self.lower_stmt(s)).collect();

        // Reduce init value
        let reduce_init = if matches!(kind, ArrayCallbackKind::Reduce | ArrayCallbackKind::ReduceRight) && ce.arguments.len() >= 2 {
            ce.arguments
                .get(1)
                .and_then(|a| a.as_expression())
                .map(|e| self.lower_expr(e))
        } else {
            None
        };

        Some((
            kind,
            elem_param,
            has_idx_param,
            idx_param,
            ir_body,
            reduce_init,
        ))
    }

    /// Try to inline an array callback method (forEach, some, every, filter, find,
    /// findIndex, findLast, findLastIndex, map, reduce) when the first argument
    /// is an ArrowFunctionExpression or FunctionExpression.
    ///
    /// Returns `IrExpr::ArrayCallbackInline` if inlinable, `None` otherwise.
    pub(super) fn try_inline_array_callback(
        &mut self,
        ce: &CallExpression,
        builtin: &crate::native_builtins::BuiltinCall,
    ) -> Option<IrExpr> {
        use crate::zigir::types::{IrArrayCallbackInline, IrExpr};

        let kind = Self::resolve_callback_kind(builtin)?;
        let (kind, elem_param, has_idx_param, idx_param, ir_body, reduce_init) =
            self.parse_callback_inline(ce, kind)?;

        let obj_name = self.extract_callee_object_name(ce)?;
        let elem_type = self
            .type_info
            .array_element_types
            .get(obj_name.as_str())
            .cloned()
            .unwrap_or(ZigType::JsAny);

        // Determine collection kind based on variable type
        let collection_kind = self
            .type_info
            .var_types
            .get(obj_name.as_str())
            .map(|t| {
                if matches!(t, ZigType::NamedStruct(s) if s == "Map") {
                    crate::zigir::types::CollectionKind::Map
                } else if matches!(t, ZigType::NamedStruct(s) if s == "Set") {
                    crate::zigir::types::CollectionKind::Set
                } else {
                    crate::zigir::types::CollectionKind::Array
                }
            })
            .unwrap_or(crate::zigir::types::CollectionKind::Array);

        Some(IrExpr::ArrayCallbackInline(Box::new(
            IrArrayCallbackInline {
                kind,
                collection_kind,
                obj_name,
                obj_expr: None,
                elem_type,
                elem_param,
                has_idx_param,
                idx_param,
                body: ir_body,
                reduce_init,
            },
        )))
    }

    /// Try to inline an array callback method when the receiver is a chained expression
    /// (e.g., `arr.filter(x => x > 1).map(x => x * 2)`). The inner expression has already
    /// been lowered (typically to `ArrayCallbackInline` or `ArrayMethodInline`), and we know
    /// its element type. We construct the outer callback inline with a synthetic `obj_name`
    /// (a temp var placeholder) and store the inner expression in `obj_expr` so the emitter
    /// can render it inline.
    pub(super) fn try_inline_array_callback_with_chain(
        &mut self,
        ce: &CallExpression,
        builtin: &crate::native_builtins::BuiltinCall,
        elem_type: &ZigType,
        inner_expr: &IrExpr,
    ) -> Option<IrExpr> {
        use crate::zigir::types::{IrArrayCallbackInline, IrExpr};

        let kind = Self::resolve_callback_kind(builtin)?;
        let (kind, elem_param, has_idx_param, idx_param, ir_body, reduce_init) =
            self.parse_callback_inline(ce, kind)?;

        let chain_obj_name = self.name_mangler.next_name("__chain");

        Some(IrExpr::ArrayCallbackInline(Box::new(
            IrArrayCallbackInline {
                kind,
                collection_kind: crate::zigir::types::CollectionKind::Array,
                obj_name: chain_obj_name,
                obj_expr: Some(Box::new(inner_expr.clone())),
                elem_type: elem_type.clone(),
                elem_param,
                has_idx_param,
                idx_param,
                body: ir_body,
                reduce_init,
            },
        )))
    }

    /// Try to inline an array non-callback method when the receiver is a chained expression.
    /// Similar to `try_inline_array_callback_with_chain` but for methods like join, slice, etc.
    pub(super) fn try_inline_array_method_with_chain(
        &mut self,
        _ce: &CallExpression,
        builtin: &crate::native_builtins::BuiltinCall,
        args: &[IrExpr],
        elem_type: &ZigType,
        inner_expr: &IrExpr,
    ) -> Option<IrExpr> {
        use crate::zigir::types::{IrArrayMethodInline, IrExpr};

        let kind = Self::resolve_method_kind(builtin)?;

        let chain_obj_name = self.name_mangler.next_name("__chain");

        Some(IrExpr::ArrayMethodInline(Box::new(IrArrayMethodInline {
            kind,
            obj_name: chain_obj_name,
            obj_expr: Some(Box::new(inner_expr.clone())),
            elem_type: elem_type.clone(),
            args: args.to_vec(),
        })))
    }

    /// Extract the object variable name from a CallExpression's callee.
    pub(super) fn extract_callee_object_name(&self, ce: &CallExpression) -> Option<String> {
        Self::extract_callee_object_name_static(&ce.callee)
    }

    /// Extract the object variable name from a callee Expression.
    pub(super) fn extract_callee_object_name_static(callee: &Expression) -> Option<String> {
        match callee {
            Expression::StaticMemberExpression(mem) => match &mem.object {
                Expression::Identifier(id) => Some(id.name.as_str().to_string()),
                Expression::StringLiteral(sl) => {
                    // "hello".method() — format as Zig string literal
                    let escaped = crate::zigir::emit::helpers::escape_zig_string(sl.value.as_str());
                    Some(format!("\"{}\"", escaped))
                }
                Expression::TemplateLiteral(tl)
                    if tl.quasis.len() == 1 && tl.expressions.is_empty() =>
                {
                    // `hello`.method() — same as StringLiteral
                    let escaped =
                        crate::zigir::emit::helpers::escape_zig_string(&tl.quasis[0].value.raw);
                    Some(format!("\"{}\"", escaped))
                }
                _ => None,
            },
            _ => None,
        }
    }

    pub(super) fn is_typedarray_type(name: &str) -> bool {
        matches!(
            name,
            "Int8Array"
                | "Uint8Array"
                | "Uint8ClampedArray"
                | "Int16Array"
                | "Uint16Array"
                | "Int32Array"
                | "Uint32Array"
                | "Float32Array"
                | "Float64Array"
                | "BigInt64Array"
                | "BigUint64Array"
        )
    }

    /// Check if an expression is a RegExp: either a literal `/pattern/` or `new RegExp(...)`.
    pub(super) fn is_regexp_expr(expr: &Expression) -> bool {
        match expr {
            Expression::RegExpLiteral(_) => true,
            Expression::NewExpression(ne) => {
                if let Expression::Identifier(id) = &ne.callee {
                    id.name.as_str() == "RegExp"
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    pub(super) fn typedarray_type_suffix(name: &str) -> Option<&'static str> {
        match name {
            "Int8Array" => Some("I8"),
            "Uint8Array" | "Uint8ClampedArray" => Some("U8"),
            "Int16Array" => Some("I16"),
            "Uint16Array" => Some("U16"),
            "Int32Array" => Some("I32"),
            "Uint32Array" => Some("U32"),
            "Float32Array" => Some("F32"),
            "Float64Array" => Some("F64"),
            "BigInt64Array" => Some("I64"),
            "BigUint64Array" => Some("U64"),
            _ => None,
        }
    }

    /// Extract regex metadata from the first argument of a String.match/matchAll/search call,
    /// or from the receiver of a RegExpTest/RegExpExec call.
    /// Returns `None` for all other builtin calls.
    pub(super) fn extract_regex_info(
        ce: &CallExpression,
        builtin: &crate::native_builtins::BuiltinCall,
    ) -> Option<crate::zigir::types::IrRegexInfo> {
        use crate::native_builtins::BuiltinCall;
        use oxc_ast::ast::Expression;

        match builtin {
            BuiltinCall::StringMatch | BuiltinCall::StringMatchAll | BuiltinCall::StringSearch => {}
            // RegExpTest/RegExpExec: extract pattern from the *receiver* (callee object)
            BuiltinCall::RegExpTest | BuiltinCall::RegExpExec => {
                return if let Expression::StaticMemberExpression(sme) = &ce.callee {
                    match &sme.object {
                        Expression::RegExpLiteral(re) => {
                            let pattern = re.regex.pattern.text.as_str();
                            let escaped = pattern.replace('\\', "\\\\").replace('"', "\\\"");
                            Some(crate::zigir::types::IrRegexInfo {
                                pattern: Some(escaped),
                                has_global: false,
                                is_var_ref: false,
                                var_name: None,
                            })
                        }
                        Expression::Identifier(id) => Some(crate::zigir::types::IrRegexInfo {
                            pattern: None,
                            has_global: false,
                            is_var_ref: true,
                            var_name: Some(id.name.to_string()),
                        }),
                        _ => None,
                    }
                } else {
                    None
                };
            }
            _ => return None,
        }

        if let Some(first_arg) = ce.arguments.first()
            && let Some(expr) = first_arg.as_expression()
        {
            match expr {
                Expression::RegExpLiteral(re) => {
                    let pattern = re.regex.pattern.text.as_str();
                    let escaped = pattern.replace('\\', "\\\\").replace('"', "\\\"");
                    let has_global = re.raw.as_ref().is_some_and(|raw| {
                        let raw_str = raw.as_str();
                        raw_str.rfind('/').is_some_and(|idx| {
                            let flags_part = &raw_str[idx + 1..];
                            flags_part.contains('g')
                        })
                    });
                    Some(crate::zigir::types::IrRegexInfo {
                        pattern: Some(escaped),
                        has_global,
                        is_var_ref: false,
                        var_name: None,
                    })
                }
                Expression::Identifier(id) => Some(crate::zigir::types::IrRegexInfo {
                    pattern: None,
                    has_global: false,
                    is_var_ref: true,
                    var_name: Some(id.name.as_str().to_string()),
                }),
                _ => None,
            }
        } else {
            None
        }
    }

    // ï¿½ï¿½ï¿½ï¿½ AST ident-usage helpers ï¿½ï¿½ï¿½ï¿½ï¿½ï¿½ï¿½ï¿½ï¿½ï¿½ï¿½ï¿½ï¿½ï¿½ï¿½ï¿½ï¿½ï¿½ï¿½ï¿½ï¿½ï¿½ï¿½ï¿½ï¿½ï¿½ï¿½ï¿½ï¿½ï¿½ï¿½ï¿½ï¿½ï¿½ï¿½ï¿½ï¿½ï¿½ï¿½ï¿½ï¿½ï¿½
    // AST ident-usage helpers — used to check whether a callback parameter is actually referenced.

    pub(super) fn ast_stmt_uses_ident(ident: &str, stmt: &Statement) -> bool {
        match stmt {
            Statement::ReturnStatement(r) => r
                .argument
                .as_ref()
                .is_some_and(|e| Self::ast_expr_uses_ident(ident, e)),
            Statement::ExpressionStatement(e) => Self::ast_expr_uses_ident(ident, &e.expression),
            Statement::BlockStatement(b) => {
                b.body.iter().any(|s| Self::ast_stmt_uses_ident(ident, s))
            }
            _ => false,
        }
    }

    pub(super) fn ast_expr_uses_ident(ident: &str, expr: &Expression) -> bool {
        match expr {
            Expression::Identifier(id) => id.name.as_str() == ident,
            Expression::BinaryExpression(b) => {
                Self::ast_expr_uses_ident(ident, &b.left)
                    || Self::ast_expr_uses_ident(ident, &b.right)
            }
            Expression::UnaryExpression(u) => Self::ast_expr_uses_ident(ident, &u.argument),
            Expression::StaticMemberExpression(m) => Self::ast_expr_uses_ident(ident, &m.object),
            Expression::ComputedMemberExpression(m) => {
                Self::ast_expr_uses_ident(ident, &m.object)
                    || Self::ast_expr_uses_ident(ident, &m.expression)
            }
            Expression::CallExpression(c) => {
                Self::ast_expr_uses_ident(ident, &c.callee)
                    || c.arguments.iter().any(|a| match a.as_expression() {
                        Some(e) => Self::ast_expr_uses_ident(ident, e),
                        None => false,
                    })
            }
            Expression::ParenthesizedExpression(p) => {
                Self::ast_expr_uses_ident(ident, &p.expression)
            }
            Expression::ConditionalExpression(c) => {
                Self::ast_expr_uses_ident(ident, &c.test)
                    || Self::ast_expr_uses_ident(ident, &c.consequent)
                    || Self::ast_expr_uses_ident(ident, &c.alternate)
            }
            Expression::NumericLiteral(_)
            | Expression::StringLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
            | Expression::BigIntLiteral(_)
            | Expression::RegExpLiteral(_) => false,
            // Conservative: assume identifier MAY appear in unhandled variants
            _ => true,
        }
    }
}

// ï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½T
//  FnContext management
// ï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½T

impl Lowerer {
    /// Enter a function context. Saves any existing context (nesting support).
    pub(super) fn enter_fn(
        &mut self,
        name: &str,
        is_export: bool,
        return_type: Option<ZigType>,
    ) -> Option<FnContext> {
        let old = self.fn_ctx.take();
        self.fn_ctx = Some(FnContext::new(name, is_export, return_type));
        old
    }

    /// Exit the current function context. Restores the previous one if any.
    pub(super) fn exit_fn(&mut self, saved: Option<FnContext>) -> FnContext {
        let ctx = self.fn_ctx.take().expect("exit_fn called without enter_fn");
        self.fn_ctx = saved;
        ctx
    }
}

/// Map a BuiltinCall to (BuiltinModule, method_name, return_type).
pub fn builtin_call_to_ir(
    bc: &crate::native_builtins::BuiltinCall,
) -> (BuiltinModule, String, ZigType) {
    use crate::native_builtins::BuiltinCall;

    match bc {
        // Math
        BuiltinCall::MathAbs => (BuiltinModule::JsMath, "abs".into(), ZigType::F64),
        BuiltinCall::MathFloor => (BuiltinModule::JsMath, "floor".into(), ZigType::F64),
        BuiltinCall::MathCeil => (BuiltinModule::JsMath, "ceil".into(), ZigType::F64),
        BuiltinCall::MathRound => (BuiltinModule::JsMath, "round".into(), ZigType::F64),
        BuiltinCall::MathSqrt => (BuiltinModule::JsMath, "sqrt".into(), ZigType::F64),
        BuiltinCall::MathRandom => (BuiltinModule::JsMath, "random".into(), ZigType::F64),
        BuiltinCall::MathPow => (BuiltinModule::JsMath, "pow".into(), ZigType::F64),
        BuiltinCall::MathMax => (BuiltinModule::JsMath, "max".into(), ZigType::F64),
        BuiltinCall::MathMin => (BuiltinModule::JsMath, "min".into(), ZigType::F64),
        BuiltinCall::MathHypot => (BuiltinModule::JsMath, "hypot".into(), ZigType::F64),
        BuiltinCall::MathSin => (BuiltinModule::JsMath, "sin".into(), ZigType::F64),
        BuiltinCall::MathCos => (BuiltinModule::JsMath, "cos".into(), ZigType::F64),
        BuiltinCall::MathTan => (BuiltinModule::JsMath, "tan".into(), ZigType::F64),
        BuiltinCall::MathAsin => (BuiltinModule::JsMath, "asin".into(), ZigType::F64),
        BuiltinCall::MathAcos => (BuiltinModule::JsMath, "acos".into(), ZigType::F64),
        BuiltinCall::MathAtan => (BuiltinModule::JsMath, "atan".into(), ZigType::F64),
        BuiltinCall::MathAtan2 => (BuiltinModule::JsMath, "atan2".into(), ZigType::F64),
        BuiltinCall::MathLog => (BuiltinModule::JsMath, "log".into(), ZigType::F64),
        BuiltinCall::MathLog10 => (BuiltinModule::JsMath, "log10".into(), ZigType::F64),
        BuiltinCall::MathLog2 => (BuiltinModule::JsMath, "log2".into(), ZigType::F64),
        BuiltinCall::MathExp => (BuiltinModule::JsMath, "exp".into(), ZigType::F64),
        BuiltinCall::MathSign => (BuiltinModule::JsMath, "sign".into(), ZigType::F64),
        BuiltinCall::MathTrunc => (BuiltinModule::JsMath, "trunc".into(), ZigType::F64),
        BuiltinCall::MathCbrt => (BuiltinModule::JsMath, "cbrt".into(), ZigType::F64),
        BuiltinCall::MathExpm1 => (BuiltinModule::JsMath, "expm1".into(), ZigType::F64),
        BuiltinCall::MathSinh => (BuiltinModule::JsMath, "sinh".into(), ZigType::F64),
        BuiltinCall::MathCosh => (BuiltinModule::JsMath, "cosh".into(), ZigType::F64),
        BuiltinCall::MathTanh => (BuiltinModule::JsMath, "tanh".into(), ZigType::F64),
        BuiltinCall::MathAsinh => (BuiltinModule::JsMath, "asinh".into(), ZigType::F64),
        BuiltinCall::MathAcosh => (BuiltinModule::JsMath, "acosh".into(), ZigType::F64),
        BuiltinCall::MathAtanh => (BuiltinModule::JsMath, "atanh".into(), ZigType::F64),
        BuiltinCall::MathClz32 => (BuiltinModule::JsMath, "clz32".into(), ZigType::I64),
        BuiltinCall::MathFround => (BuiltinModule::JsMath, "fround".into(), ZigType::F64),
        BuiltinCall::MathImul => (BuiltinModule::JsMath, "imul".into(), ZigType::I64),
        BuiltinCall::MathLog1p => (BuiltinModule::JsMath, "log1p".into(), ZigType::F64),

        // Console
        BuiltinCall::ConsoleLog => (BuiltinModule::JsConsole, "log".into(), ZigType::Void),
        BuiltinCall::ConsoleError => (BuiltinModule::JsConsole, "error".into(), ZigType::Void),
        BuiltinCall::ConsoleWarn => (BuiltinModule::JsConsole, "warn".into(), ZigType::Void),

        // JSON
        BuiltinCall::JsonStringify => (BuiltinModule::JsJson, "stringify".into(), ZigType::Str),
        BuiltinCall::JsonParse => (BuiltinModule::JsJson, "parse".into(), ZigType::JsAny),

        // Global functions
        BuiltinCall::ParseInt => (BuiltinModule::JsUri, "parseInt".into(), ZigType::F64),
        BuiltinCall::ParseFloat => (BuiltinModule::JsUri, "parseFloat".into(), ZigType::F64),
        BuiltinCall::IsNaN => (BuiltinModule::JsUri, "isNaN".into(), ZigType::Bool),
        BuiltinCall::IsFinite => (BuiltinModule::JsUri, "isFinite".into(), ZigType::Bool),
        BuiltinCall::EncodeURIComponent => (
            BuiltinModule::JsUri,
            "encodeURIComponent".into(),
            ZigType::Str,
        ),
        BuiltinCall::DecodeURIComponent => (
            BuiltinModule::JsUri,
            "decodeURIComponent".into(),
            ZigType::Str,
        ),
        BuiltinCall::EncodeURI => (BuiltinModule::JsUri, "encodeURI".into(), ZigType::Str),
        BuiltinCall::DecodeURI => (BuiltinModule::JsUri, "decodeURI".into(), ZigType::Str),
        BuiltinCall::Eval => (BuiltinModule::JsUri, "eval".into(), ZigType::Void),

        // Constructors
        BuiltinCall::NumberConstructor => {
            (BuiltinModule::JsNumber, "constructor".into(), ZigType::F64)
        }
        BuiltinCall::StringConstructor => {
            (BuiltinModule::JsString, "constructor".into(), ZigType::Str)
        }
        BuiltinCall::BooleanConstructor => (
            BuiltinModule::JsNumber,
            "booleanConstructor".into(),
            ZigType::Bool,
        ),
        BuiltinCall::BigIntConstructor => (
            BuiltinModule::JsBigInt,
            "bigIntConstructor".into(),
            ZigType::BigInt,
        ),
        BuiltinCall::ObjectConstructor => (
            BuiltinModule::JsObject,
            "constructor".into(),
            ZigType::JsAny,
        ),
        BuiltinCall::SymbolConstructor => (
            BuiltinModule::JsSymbol,
            "constructor".into(),
            ZigType::JsSymbol,
        ),

        // Number static methods
        BuiltinCall::NumberIsNaN => (BuiltinModule::JsNumber, "isNaN".into(), ZigType::Bool),
        BuiltinCall::NumberIsFinite => (BuiltinModule::JsNumber, "isFinite".into(), ZigType::Bool),
        BuiltinCall::NumberIsInteger => {
            (BuiltinModule::JsNumber, "isInteger".into(), ZigType::Bool)
        }
        BuiltinCall::NumberIsSafeInteger => (
            BuiltinModule::JsNumber,
            "isSafeInteger".into(),
            ZigType::Bool,
        ),
        BuiltinCall::NumberParseInt => (BuiltinModule::JsNumber, "parseInt".into(), ZigType::F64),
        BuiltinCall::NumberParseFloat => {
            (BuiltinModule::JsNumber, "parseFloat".into(), ZigType::F64)
        }

        // Number instance methods
        BuiltinCall::NumberToFixed => (BuiltinModule::JsNumber, "toFixed".into(), ZigType::Str),
        BuiltinCall::NumberToExponential => (
            BuiltinModule::JsNumber,
            "toExponential".into(),
            ZigType::Str,
        ),
        BuiltinCall::NumberToPrecision => {
            (BuiltinModule::JsNumber, "toPrecision".into(), ZigType::Str)
        }

        // String instance methods
        BuiltinCall::StringIndexOf => (BuiltinModule::JsString, "indexOf".into(), ZigType::I64),
        BuiltinCall::StringIncludes => (BuiltinModule::JsString, "includes".into(), ZigType::Bool),
        BuiltinCall::StringStartsWith => {
            (BuiltinModule::JsString, "startsWith".into(), ZigType::Bool)
        }
        BuiltinCall::StringEndsWith => (BuiltinModule::JsString, "endsWith".into(), ZigType::Bool),
        BuiltinCall::StringLastIndexOf => {
            (BuiltinModule::JsString, "lastIndexOf".into(), ZigType::I64)
        }
        BuiltinCall::StringTrim => (BuiltinModule::JsString, "trim".into(), ZigType::Str),
        BuiltinCall::StringSplit => (
            BuiltinModule::JsString,
            "split".into(),
            ZigType::ArrayList(Box::new(ZigType::Str)),
        ),
        BuiltinCall::StringPadStart => (BuiltinModule::JsString, "padStart".into(), ZigType::Str),
        BuiltinCall::StringPadEnd => (BuiltinModule::JsString, "padEnd".into(), ZigType::Str),
        BuiltinCall::StringTrimStart => (BuiltinModule::JsString, "trimStart".into(), ZigType::Str),
        BuiltinCall::StringTrimEnd => (BuiltinModule::JsString, "trimEnd".into(), ZigType::Str),
        BuiltinCall::StringToUpperCase => {
            (BuiltinModule::JsString, "toUpperCase".into(), ZigType::Str)
        }
        BuiltinCall::StringToLowerCase => {
            (BuiltinModule::JsString, "toLowerCase".into(), ZigType::Str)
        }
        BuiltinCall::StringCharAt => (BuiltinModule::JsString, "charAt".into(), ZigType::Str),
        BuiltinCall::StringCharCodeAt => {
            (BuiltinModule::JsString, "charCodeAt".into(), ZigType::I64)
        }
        BuiltinCall::StringCodePointAt => {
            (BuiltinModule::JsString, "codePointAt".into(), ZigType::I64)
        }
        BuiltinCall::StringConcat => (BuiltinModule::JsString, "concat".into(), ZigType::Str),
        BuiltinCall::StringSlice => (BuiltinModule::JsString, "slice".into(), ZigType::Str),
        BuiltinCall::StringReplace => (BuiltinModule::JsString, "replace".into(), ZigType::Str),
        BuiltinCall::StringReplaceAll => {
            (BuiltinModule::JsString, "replaceAll".into(), ZigType::Str)
        }
        BuiltinCall::StringRepeat => (BuiltinModule::JsString, "repeat".into(), ZigType::Str),
        BuiltinCall::StringSubstring => (BuiltinModule::JsString, "substring".into(), ZigType::Str),
        BuiltinCall::StringAt => (BuiltinModule::JsString, "at".into(), ZigType::Str),
        BuiltinCall::StringMatch => (BuiltinModule::JsString, "match".into(), ZigType::JsAny),
        BuiltinCall::StringSearch => (BuiltinModule::JsString, "search".into(), ZigType::I64),
        BuiltinCall::StringFromCharCode => {
            (BuiltinModule::JsString, "fromCharCode".into(), ZigType::Str)
        }
        BuiltinCall::StringFromCodePoint => (
            BuiltinModule::JsString,
            "fromCodePoint".into(),
            ZigType::Str,
        ),
        BuiltinCall::StringMatchAll => (BuiltinModule::JsString, "matchAll".into(), ZigType::JsAny),
        BuiltinCall::StringLocaleCompare => (
            BuiltinModule::JsString,
            "localeCompare".into(),
            ZigType::I64,
        ),
        BuiltinCall::StringNormalize => (BuiltinModule::JsString, "normalize".into(), ZigType::Str),
        BuiltinCall::StringToLocaleUpperCase => (
            BuiltinModule::JsString,
            "toLocaleUpperCase".into(),
            ZigType::Str,
        ),
        BuiltinCall::StringToLocaleLowerCase => (
            BuiltinModule::JsString,
            "toLocaleLowerCase".into(),
            ZigType::Str,
        ),

        // Array methods
        BuiltinCall::ArrayPush => (BuiltinModule::JsArray, "push".into(), ZigType::I64),
        BuiltinCall::ArrayPop => (BuiltinModule::JsArray, "pop".into(), ZigType::JsAny),
        BuiltinCall::ArrayShift => (BuiltinModule::JsArray, "shift".into(), ZigType::JsAny),
        BuiltinCall::ArrayUnshift => (BuiltinModule::JsArray, "unshift".into(), ZigType::I64),
        BuiltinCall::ArrayReverse => (BuiltinModule::JsArray, "reverse".into(), ZigType::JsAny),
        BuiltinCall::ArraySort => (BuiltinModule::JsArray, "sort".into(), ZigType::JsAny),
        BuiltinCall::ArrayIndexOf => (BuiltinModule::JsArray, "indexOf".into(), ZigType::I64),
        BuiltinCall::ArrayIncludes => (BuiltinModule::JsArray, "includes".into(), ZigType::Bool),
        BuiltinCall::ArrayJoin => (BuiltinModule::JsArray, "join".into(), ZigType::Str),
        BuiltinCall::ArraySlice => (
            BuiltinModule::JsArray,
            "slice".into(),
            ZigType::ArrayList(Box::new(ZigType::JsAny)),
        ),
        BuiltinCall::ArraySplice => (
            BuiltinModule::JsArray,
            "splice".into(),
            ZigType::ArrayList(Box::new(ZigType::JsAny)),
        ),
        BuiltinCall::ArrayConcat => (
            BuiltinModule::JsArray,
            "concat".into(),
            ZigType::ArrayList(Box::new(ZigType::JsAny)),
        ),
        BuiltinCall::ArrayAt => (BuiltinModule::JsArray, "at".into(), ZigType::JsAny),
        BuiltinCall::ArrayLastIndexOf => {
            (BuiltinModule::JsArray, "lastIndexOf".into(), ZigType::I64)
        }
        BuiltinCall::ArrayCopyWithin => {
            (BuiltinModule::JsArray, "copyWithin".into(), ZigType::JsAny)
        }
        BuiltinCall::ArrayForEach => (BuiltinModule::JsArray, "forEach".into(), ZigType::Void),
        BuiltinCall::ArrayMap => (
            BuiltinModule::JsArray,
            "map".into(),
            ZigType::ArrayList(Box::new(ZigType::JsAny)),
        ),
        BuiltinCall::ArrayFilter => (
            BuiltinModule::JsArray,
            "filter".into(),
            ZigType::ArrayList(Box::new(ZigType::JsAny)),
        ),
        BuiltinCall::ArrayReduce => (BuiltinModule::JsArray, "reduce".into(), ZigType::JsAny),
        BuiltinCall::ArraySome => (BuiltinModule::JsArray, "some".into(), ZigType::Bool),
        BuiltinCall::ArrayEvery => (BuiltinModule::JsArray, "every".into(), ZigType::Bool),
        BuiltinCall::ArrayFlat => (
            BuiltinModule::JsArray,
            "flat".into(),
            ZigType::ArrayList(Box::new(ZigType::JsAny)),
        ),
        BuiltinCall::ArrayFlatMap => (
            BuiltinModule::JsArray,
            "flatMap".into(),
            ZigType::ArrayList(Box::new(ZigType::JsAny)),
        ),
        BuiltinCall::ArrayFind => (BuiltinModule::JsArray, "find".into(), ZigType::JsAny),
        BuiltinCall::ArrayFindIndex => (BuiltinModule::JsArray, "findIndex".into(), ZigType::I64),
        BuiltinCall::ArrayFindLast => (BuiltinModule::JsArray, "findLast".into(), ZigType::JsAny),
        BuiltinCall::ArrayFindLastIndex => {
            (BuiltinModule::JsArray, "findLastIndex".into(), ZigType::I64)
        }
        BuiltinCall::ArrayReduceRight => {
            (BuiltinModule::JsArray, "reduceRight".into(), ZigType::JsAny)
        }
        BuiltinCall::ArrayFill => (BuiltinModule::JsArray, "fill".into(), ZigType::JsAny),
        BuiltinCall::ArrayWith => (
            BuiltinModule::JsArray,
            "with".into(),
            ZigType::ArrayList(Box::new(ZigType::JsAny)),
        ),
        BuiltinCall::ArrayToReversed => (
            BuiltinModule::JsArray,
            "toReversed".into(),
            ZigType::ArrayList(Box::new(ZigType::JsAny)),
        ),
        BuiltinCall::ArrayToSorted => (
            BuiltinModule::JsArray,
            "toSorted".into(),
            ZigType::ArrayList(Box::new(ZigType::JsAny)),
        ),
        BuiltinCall::ArrayToSpliced => (
            BuiltinModule::JsArray,
            "toSpliced".into(),
            ZigType::ArrayList(Box::new(ZigType::JsAny)),
        ),
        BuiltinCall::ArrayKeys => (
            BuiltinModule::JsArray,
            "keys".into(),
            ZigType::ArrayList(Box::new(ZigType::I64)),
        ),
        BuiltinCall::ArrayValues => (
            BuiltinModule::JsArray,
            "values".into(),
            ZigType::ArrayList(Box::new(ZigType::JsAny)),
        ),
        BuiltinCall::ArrayEntries => (
            BuiltinModule::JsArray,
            "entries".into(),
            ZigType::ArrayList(Box::new(ZigType::JsAny)),
        ),
        BuiltinCall::ArrayFrom => (
            BuiltinModule::JsArray,
            "from".into(),
            ZigType::ArrayList(Box::new(ZigType::JsAny)),
        ),
        BuiltinCall::ArrayOf => (
            BuiltinModule::JsArray,
            "of".into(),
            ZigType::ArrayList(Box::new(ZigType::JsAny)),
        ),
        BuiltinCall::ArrayIsArray => (BuiltinModule::JsArray, "isArray".into(), ZigType::Bool),
        BuiltinCall::TypedArraySubarray => (
            BuiltinModule::JsTypedArray,
            "subarray".into(),
            ZigType::JsAny,
        ),

        // Map/Set
        BuiltinCall::MapSet => (
            BuiltinModule::JsCollections,
            "set".into(),
            ZigType::NamedStruct("Map".into()),
        ),
        BuiltinCall::MapGet => (BuiltinModule::JsCollections, "get".into(), ZigType::JsAny),
        BuiltinCall::MapHas => (BuiltinModule::JsCollections, "has".into(), ZigType::Bool),
        BuiltinCall::MapDelete => (BuiltinModule::JsCollections, "delete".into(), ZigType::Bool),
        BuiltinCall::MapKeys => (
            BuiltinModule::JsCollections,
            "keys".into(),
            ZigType::ArrayList(Box::new(ZigType::Str)),
        ),
        BuiltinCall::MapValues => (
            BuiltinModule::JsCollections,
            "values".into(),
            ZigType::ArrayList(Box::new(ZigType::JsAny)),
        ),
        BuiltinCall::MapEntries => (
            BuiltinModule::JsCollections,
            "entries".into(),
            ZigType::ArrayList(Box::new(ZigType::JsAny)),
        ),
        BuiltinCall::MapClear => (BuiltinModule::JsCollections, "clear".into(), ZigType::Void),
        BuiltinCall::SetAdd => (
            BuiltinModule::JsCollections,
            "add".into(),
            ZigType::NamedStruct("Set".into()),
        ),
        BuiltinCall::SetForEach => (
            BuiltinModule::JsCollections,
            "forEach".into(),
            ZigType::Void,
        ),
        BuiltinCall::SetKeys => (
            BuiltinModule::JsCollections,
            "keys".into(),
            ZigType::ArrayList(Box::new(ZigType::JsAny)),
        ),
        BuiltinCall::SetValues => (
            BuiltinModule::JsCollections,
            "values".into(),
            ZigType::ArrayList(Box::new(ZigType::JsAny)),
        ),
        BuiltinCall::SetEntries => (
            BuiltinModule::JsCollections,
            "entries".into(),
            ZigType::ArrayList(Box::new(ZigType::JsAny)),
        ),

        // Date static
        BuiltinCall::DateNow => (BuiltinModule::JsDate, "now".into(), ZigType::I64),
        BuiltinCall::DateParse => (BuiltinModule::JsDate, "parse".into(), ZigType::I64),
        BuiltinCall::DateUTC => (BuiltinModule::JsDate, "utc".into(), ZigType::I64),

        // Date instance
        BuiltinCall::DateGetTime => (BuiltinModule::JsDate, "getTime".into(), ZigType::I64),
        BuiltinCall::DateGetFullYear => (BuiltinModule::JsDate, "getFullYear".into(), ZigType::I64),
        BuiltinCall::DateGetMonth => (BuiltinModule::JsDate, "getMonth".into(), ZigType::I64),
        BuiltinCall::DateGetDate => (BuiltinModule::JsDate, "getDate".into(), ZigType::I64),
        BuiltinCall::DateGetDay => (BuiltinModule::JsDate, "getDay".into(), ZigType::I64),
        BuiltinCall::DateGetHours => (BuiltinModule::JsDate, "getHours".into(), ZigType::I64),
        BuiltinCall::DateGetMinutes => (BuiltinModule::JsDate, "getMinutes".into(), ZigType::I64),
        BuiltinCall::DateGetSeconds => (BuiltinModule::JsDate, "getSeconds".into(), ZigType::I64),
        BuiltinCall::DateGetMilliseconds => (
            BuiltinModule::JsDate,
            "getMilliseconds".into(),
            ZigType::I64,
        ),
        BuiltinCall::DateGetTimezoneOffset => (
            BuiltinModule::JsDate,
            "getTimezoneOffset".into(),
            ZigType::I64,
        ),
        BuiltinCall::DateToISOString => (BuiltinModule::JsDate, "toISOString".into(), ZigType::Str),
        BuiltinCall::DateToString => (BuiltinModule::JsDate, "toString".into(), ZigType::Str),
        BuiltinCall::DateToDateString => {
            (BuiltinModule::JsDate, "toDateString".into(), ZigType::Str)
        }
        BuiltinCall::DateToTimeString => {
            (BuiltinModule::JsDate, "toTimeString".into(), ZigType::Str)
        }
        BuiltinCall::DateToLocaleString => {
            (BuiltinModule::JsDate, "toLocaleString".into(), ZigType::Str)
        }
        BuiltinCall::DateGetUTCFullYear => {
            (BuiltinModule::JsDate, "getUTCFullYear".into(), ZigType::I64)
        }
        BuiltinCall::DateGetUTCMonth => (BuiltinModule::JsDate, "getUTCMonth".into(), ZigType::I64),
        BuiltinCall::DateGetUTCDate => (BuiltinModule::JsDate, "getUTCDate".into(), ZigType::I64),
        BuiltinCall::DateGetUTCDay => (BuiltinModule::JsDate, "getUTCDay".into(), ZigType::I64),
        BuiltinCall::DateGetUTCHours => (BuiltinModule::JsDate, "getUTCHours".into(), ZigType::I64),
        BuiltinCall::DateGetUTCMinutes => {
            (BuiltinModule::JsDate, "getUTCMinutes".into(), ZigType::I64)
        }
        BuiltinCall::DateGetUTCSeconds => {
            (BuiltinModule::JsDate, "getUTCSeconds".into(), ZigType::I64)
        }
        BuiltinCall::DateGetUTCMilliseconds => (
            BuiltinModule::JsDate,
            "getUTCMilliseconds".into(),
            ZigType::I64,
        ),
        BuiltinCall::DateToJSON => (BuiltinModule::JsDate, "toJSON".into(), ZigType::Str),
        BuiltinCall::DateValueOf => (BuiltinModule::JsDate, "valueOf".into(), ZigType::I64),
        BuiltinCall::DateSetFullYear => (BuiltinModule::JsDate, "setFullYear".into(), ZigType::I64),
        BuiltinCall::DateSetMonth => (BuiltinModule::JsDate, "setMonth".into(), ZigType::I64),
        BuiltinCall::DateSetDate => (BuiltinModule::JsDate, "setDate".into(), ZigType::I64),
        BuiltinCall::DateSetHours => (BuiltinModule::JsDate, "setHours".into(), ZigType::I64),
        BuiltinCall::DateSetMinutes => (BuiltinModule::JsDate, "setMinutes".into(), ZigType::I64),
        BuiltinCall::DateSetSeconds => (BuiltinModule::JsDate, "setSeconds".into(), ZigType::I64),
        BuiltinCall::DateSetMilliseconds => (
            BuiltinModule::JsDate,
            "setMilliseconds".into(),
            ZigType::I64,
        ),
        BuiltinCall::DateSetUTCFullYear => {
            (BuiltinModule::JsDate, "setUTCFullYear".into(), ZigType::I64)
        }
        BuiltinCall::DateSetUTCMonth => (BuiltinModule::JsDate, "setUTCMonth".into(), ZigType::I64),
        BuiltinCall::DateSetUTCDate => (BuiltinModule::JsDate, "setUTCDate".into(), ZigType::I64),
        BuiltinCall::DateSetUTCHours => (BuiltinModule::JsDate, "setUTCHours".into(), ZigType::I64),
        BuiltinCall::DateSetUTCMinutes => {
            (BuiltinModule::JsDate, "setUTCMinutes".into(), ZigType::I64)
        }
        BuiltinCall::DateSetUTCSeconds => {
            (BuiltinModule::JsDate, "setUTCSeconds".into(), ZigType::I64)
        }
        BuiltinCall::DateSetUTCMilliseconds => (
            BuiltinModule::JsDate,
            "setUTCMilliseconds".into(),
            ZigType::I64,
        ),
        BuiltinCall::DateSetTime => (BuiltinModule::JsDate, "setTime".into(), ZigType::I64),
        BuiltinCall::DateToUTCString => (BuiltinModule::JsDate, "toUTCString".into(), ZigType::Str),

        // Object static
        BuiltinCall::ObjectKeys => (
            BuiltinModule::JsObject,
            "keys".into(),
            ZigType::ArrayList(Box::new(ZigType::Str)),
        ),
        BuiltinCall::ObjectValues => (
            BuiltinModule::JsObject,
            "values".into(),
            ZigType::ArrayList(Box::new(ZigType::JsAny)),
        ),
        BuiltinCall::ObjectEntries => (
            BuiltinModule::JsObject,
            "entries".into(),
            ZigType::ArrayList(Box::new(ZigType::JsAny)),
        ),
        BuiltinCall::ObjectFromEntries => (
            BuiltinModule::JsObject,
            "fromEntries".into(),
            ZigType::JsAny,
        ),
        BuiltinCall::ObjectAssign => (BuiltinModule::JsObject, "assign".into(), ZigType::JsAny),
        BuiltinCall::ObjectFreeze => (BuiltinModule::JsObject, "freeze".into(), ZigType::JsAny),
        BuiltinCall::ObjectSeal => (BuiltinModule::JsObject, "seal".into(), ZigType::JsAny),
        BuiltinCall::ObjectPreventExtensions => (
            BuiltinModule::JsObject,
            "preventExtensions".into(),
            ZigType::JsAny,
        ),
        BuiltinCall::ObjectHasOwn => (BuiltinModule::JsObject, "hasOwn".into(), ZigType::Bool),
        BuiltinCall::ObjectIs => (BuiltinModule::JsObject, "is".into(), ZigType::Bool),
        BuiltinCall::ObjectGetOwnPropertyNames => (
            BuiltinModule::JsObject,
            "getOwnPropertyNames".into(),
            ZigType::ArrayList(Box::new(ZigType::Str)),
        ),
        BuiltinCall::ObjectCreate => (BuiltinModule::JsObject, "create".into(), ZigType::JsAny),
        BuiltinCall::ObjectDefineProperty => (
            BuiltinModule::JsObject,
            "defineProperty".into(),
            ZigType::JsAny,
        ),
        BuiltinCall::ObjectGetPrototypeOf => (
            BuiltinModule::JsObject,
            "getPrototypeOf".into(),
            ZigType::JsAny,
        ),
        BuiltinCall::ObjectDefineProperties => (
            BuiltinModule::JsObject,
            "defineProperties".into(),
            ZigType::JsAny,
        ),
        BuiltinCall::ObjectGetOwnPropertyDescriptor => (
            BuiltinModule::JsObject,
            "getOwnPropertyDescriptor".into(),
            ZigType::JsAny,
        ),
        BuiltinCall::ObjectSetPrototypeOf => (
            BuiltinModule::JsObject,
            "setPrototypeOf".into(),
            ZigType::JsAny,
        ),
        BuiltinCall::ObjectIsSealed => (BuiltinModule::JsObject, "isSealed".into(), ZigType::Bool),
        BuiltinCall::ObjectIsFrozen => (BuiltinModule::JsObject, "isFrozen".into(), ZigType::Bool),
        BuiltinCall::ObjectIsExtensible => (
            BuiltinModule::JsObject,
            "isExtensible".into(),
            ZigType::Bool,
        ),
        BuiltinCall::ObjectGroupBy => (BuiltinModule::JsObject, "groupBy".into(), ZigType::JsAny),

        // Symbol
        BuiltinCall::SymbolFor => (BuiltinModule::JsSymbol, "for".into(), ZigType::JsSymbol), // Emitter renames "for" ï¿½ï¿½ "symbolFor" to avoid Zig keyword
        BuiltinCall::SymbolKeyFor => (BuiltinModule::JsSymbol, "keyFor".into(), ZigType::Str),

        // RegExp
        BuiltinCall::RegExpTest => (BuiltinModule::JsRegExp, "test".into(), ZigType::Bool),
        BuiltinCall::RegExpExec => (BuiltinModule::JsRegExp, "exec".into(), ZigType::JsAny),
    }
}

// ï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½T
//  Free helper functions for destructuring
// ï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½Tï¿½T

/// Check if an init expression may have side effects (needs temp variable in destructure).
pub fn init_may_have_side_effects(init: &Expression) -> bool {
    matches!(
        init,
        Expression::CallExpression(_)
            | Expression::NewExpression(_)
            | Expression::AssignmentExpression(_)
            | Expression::UpdateExpression(_)
    )
}

/// Extract the string name of a property key (static identifier, string literal, private id).
pub fn property_key_name(key: &PropertyKey) -> Option<String> {
    match key {
        PropertyKey::StaticIdentifier(id) => Some(id.name.to_string()),
        PropertyKey::StringLiteral(sl) => Some(sl.value.to_string()),
        PropertyKey::PrivateIdentifier(id) => Some(id.name.to_string()),
        _ => None,
    }
}
