// zigir/lower/expr/call.rs
// Call, new, await expression lowering + helpers.

use oxc_ast::ast::*;

use crate::native_builtins::BuiltinCall;
use crate::types::ZigType;
use crate::zigir::builtins::BuiltinModule;
use crate::zigir::ident::IrIdent;
use crate::zigir::kinds::{CallKind, FieldKind};

use super::super::cabi::builtin_call_to_ir;
use super::Lowerer;

impl Lowerer {
    /// Check for unsupported global object calls (Atomics, Reflect, etc.)
    /// that should produce compile errors instead of silent code generation.
    pub(super) fn check_unsupported_call(&self, ce: &CallExpression) -> Option<String> {
        // Match patterns: Atomics.load(), Reflect.apply(), Map.groupBy(), etc.
        match &ce.callee {
            Expression::StaticMemberExpression(mem) => {
                if let Expression::Identifier(id) = &mem.object {
                    let obj_name = id.name.as_str();
                    let method_name = mem.property.name.as_str();
                    match obj_name {
                        "Atomics" => Some(format!(
                            "Atomics.{}() is not supported (shared memory atomics are not available in Zig)",
                            method_name
                        )),
                        "Reflect" => Some(format!(
                            "Reflect.{}() is not supported (meta-programming API is not available)",
                            method_name
                        )),
                        "Object" => match method_name {
                            "getOwnPropertySymbols" => Some(
                                "Object.getOwnPropertySymbols() is not supported (Symbol keys are not available in js2zig)".to_string(),
                            ),
                            _ => None,
                        },
                        "Map" if method_name == "groupBy" => Some(
                            "Map.groupBy() is not supported (requires iterable grouping)".to_string(),
                        ),
                        "Promise" => Some(format!(
                            "Promise.{}() is not supported (use async/await + host functions instead)",
                            method_name
                        )),
                        "Intl" => Some(format!(
                            "Intl.{}() is not supported (use Zig/C library for internationalization)",
                            method_name
                        )),
                        _ => None,
                    }
                } else {
                    None
                }
            }
            Expression::Identifier(id) => match id.name.as_str() {
                name @ "Atomics" | name @ "Reflect" | name @ "Promise" | name @ "Intl" => {
                    Some(format!("{} is not supported in js2zig", name))
                }
                _ => None,
            },
            _ => None,
        }
    }

    /// Lower a call expression.
    ///
    /// Routing priority:
    /// 1. Builtin detection → `IrBuiltinCall`
    /// 2. Closure / nested function call → `IrCall { call_kind: Closure }`
    /// 3. Host function call → `IrHostCall`
    /// 4. Direct user function → `IrCall { call_kind: Direct }`
    /// 5. Method call → `IrCall { call_kind: Method { .. } }`
    /// 6. IIFE / expression callee → `IrCall { call_kind: Closure }`
    pub(super) fn lower_call(&mut self, ce: &CallExpression) -> crate::zigir::types::IrExpr {
        use crate::zigir::types::IrExpr;

        // ── Step 0: Check for unsupported global objects ──
        if let Some(err_msg) = self.check_unsupported_call(ce) {
            return self.compile_error_expr(ce.span, err_msg);
        }

        // ── Step 1: Builtin detection ──
        if let Some(builtin) = crate::native_builtins::detect_builtin_call(ce) {
            // ── Step 1a: Array callback inlining ──
            // NOTE: `try_inline_array_callback` parses the callback directly
            // from AST (via `parse_callback_inline`), so the call arguments
            // must NOT be pre-lowered at this point. Pre-lowering would invoke
            // `lower_arrow_fn` on the arrow-function argument, which registers
            // a closure struct in `ClosureManager`. When the call expression
            // is then replaced by `ArrayCallbackInline`, that registered
            // closure struct becomes dead code that fails Zig `ast-check`
            // (anytype parameter mismatch). Defer `lower_args` until after
            // this check so the callback is lowered only via
            // `parse_callback_inline`, which embeds its body in the inline IR
            // without registering a closure struct.
            if let Some(inlined) = self.try_inline_array_callback(ce, &builtin) {
                return inlined;
            }

            // Callback inline did not fire — safe to lower arguments now.
            let args = self.lower_args(&ce.arguments);

            // ── Step 1b: Array non-callback method inlining ──
            if let Some(inlined) = self.try_inline_array_method(ce, &builtin, &args) {
                return inlined;
            }

            // ── Step 1c: eval() → compile error ──
            if matches!(builtin, crate::native_builtins::BuiltinCall::Eval) {
                return self.compile_error_expr(ce.span, "eval() is not supported (security risk, cannot dynamically execute at compile time)");
            }

            // ── Step 1d: ES2025 Set operations → @compileError ──
            if matches!(
                builtin,
                crate::native_builtins::BuiltinCall::SetUnion
                    | crate::native_builtins::BuiltinCall::SetIntersection
                    | crate::native_builtins::BuiltinCall::SetDifference
                    | crate::native_builtins::BuiltinCall::SetSymmetricDifference
                    | crate::native_builtins::BuiltinCall::SetIsSubsetOf
                    | crate::native_builtins::BuiltinCall::SetIsSupersetOf
                    | crate::native_builtins::BuiltinCall::SetIsDisjointFrom
            ) {
                let method_name = match &builtin {
                    crate::native_builtins::BuiltinCall::SetUnion => "union",
                    crate::native_builtins::BuiltinCall::SetIntersection => "intersection",
                    crate::native_builtins::BuiltinCall::SetDifference => "difference",
                    crate::native_builtins::BuiltinCall::SetSymmetricDifference => {
                        "symmetricDifference"
                    }
                    crate::native_builtins::BuiltinCall::SetIsSubsetOf => "isSubsetOf",
                    crate::native_builtins::BuiltinCall::SetIsSupersetOf => "isSupersetOf",
                    crate::native_builtins::BuiltinCall::SetIsDisjointFrom => "isDisjointFrom",
                    // Unreachable: the enclosing `if matches!` block only enters
                    // for the 7 Set variants above. Use a safe fallback instead
                    // of unreachable!() (P2-3: fragile catch-all elimination).
                    _ => "unknown",
                };
                return self.compile_error_expr(
                    ce.span,
                    format!(
                        "Set.prototype.{}() is not supported (ES2025 Set operation)",
                        method_name
                    ),
                );
            }

            let (module, method, return_type) = builtin_call_to_ir(&builtin);

            // JSON.parse can throw SyntaxError at runtime — mark function as can_throw
            // so the emitter's `catch return error.JsThrow` is valid.
            if matches!(builtin, crate::native_builtins::BuiltinCall::JsonParse)
                && let Some(ctx) = self.fn_ctx.as_mut()
            {
                ctx.has_catchable_error = true;
            }
            let obj_name = Self::extract_callee_object_name_static(&ce.callee);

            // ── Fix string-variable methods misidentified as array ──
            // detect_builtin_call only checks if the callee object is a StringLiteral,
            // not if it's a variable of type string. Fix up the module/method here.
            let (module, method, return_type) = if let Some(name) = &obj_name {
                if let Some(var_type) = self.type_info.var_types.get(name.as_str()) {
                    if matches!(var_type, ZigType::Str) && module == BuiltinModule::JsArray {
                        match method.as_str() {
                            "at" => (BuiltinModule::JsString, "at".into(), ZigType::Str),
                            "indexOf" => (BuiltinModule::JsString, "indexOf".into(), ZigType::I64),
                            "includes" => {
                                (BuiltinModule::JsString, "includes".into(), ZigType::Bool)
                            }
                            "lastIndexOf" => {
                                (BuiltinModule::JsString, "lastIndexOf".into(), ZigType::I64)
                            }
                            "slice" => (BuiltinModule::JsString, "slice".into(), ZigType::Str),
                            _ => (module, method, return_type),
                        }
                    } else if matches!(var_type, ZigType::F64 | ZigType::I64)
                        && module == BuiltinModule::JsDate
                        && method == "toString"
                    {
                        // R8-NumberToString: detect_builtin_call routes any
                        // `.toString()` on a non-literal receiver to
                        // DateToString because it has no type info at the
                        // AST layer. F64/I64 variables (e.g. `const n = 42;`)
                        // must produce `js_number.toString(...)` instead of
                        // the semantically wrong `js_date.toString(...)`
                        // (which would also be a Zig compile error since f64
                        // has no such method).
                        (BuiltinModule::JsNumber, "toString".into(), ZigType::Str)
                    } else if matches!(var_type, ZigType::BigInt)
                        && module == BuiltinModule::JsDate
                        && (method == "toString" || method == "toLocaleString")
                    {
                        // R8-P1-4: BigInt variable `.toString(radix?)` /
                        // `.toLocaleString()` is misrouted to DateToString /
                        // DateToLocaleString by detect_builtin_call (no type
                        // info at the AST layer). Rewrite to JsBigInt so
                        // emit_bigint_builtin propagates the radix argument
                        // (toString) or supplies the default 10 (toLocaleString).
                        // Note: the legacy BigInt Step 1.5 interception in the
                        // non-builtin path is dead code because
                        // detect_builtin_call always returns Some for
                        // `.toString()`, so this builtin-path rewrite is the
                        // actual fix path for BigInt variables.
                        (BuiltinModule::JsBigInt, method, ZigType::Str)
                    } else if let ZigType::NamedStruct(n) = var_type {
                        if Self::is_typedarray_type(n) {
                            let ta_mod = BuiltinModule::JsTypedArray;
                            match method.as_str() {
                                "set" => (ta_mod, "set".into(), ZigType::Void),
                                "get" => (ta_mod, "get".into(), ZigType::I64),
                                "fill" => (ta_mod, "fill".into(), ZigType::Void),
                                "slice" => {
                                    (ta_mod, "slice".into(), ZigType::NamedStruct(n.clone()))
                                }
                                "copyWithin" => (ta_mod, "copyWithin".into(), ZigType::Void),
                                _ => (module, method, return_type),
                            }
                        } else {
                            (module, method, return_type)
                        }
                    } else {
                        (module, method, return_type)
                    }
                } else {
                    (module, method, return_type)
                }
            } else {
                (module, method, return_type)
            };

            // ── Extract regex metadata for match/matchAll/search ──
            let regex_info = Self::extract_regex_info(ce, &builtin);

            // R8-P1-25: matchAll with non-/g literal RegExp → compile error
            // JS spec §22.1.3.19: String.prototype.matchAll called with a
            // non-global RegExp argument must throw TypeError. For literal
            // RegExp we know at compile time whether /g is present.
            if matches!(builtin, BuiltinCall::StringMatchAll)
                && let Some(ref ri) = regex_info
                && !ri.is_var_ref
                && !ri.has_global
            {
                return self.compile_error_expr(
                    ce.span,
                    "String.prototype.matchAll called with a non-global RegExp argument (TypeError)",
                );
            }

            // R8-P1-25: replaceAll with non-/g literal RegExp → compile error
            // JS spec §22.1.3.18: String.prototype.replaceAll called with a
            // non-global RegExp argument must throw TypeError.
            if matches!(builtin, BuiltinCall::StringReplaceAll)
                && let Some(ref ri) = regex_info
                && !ri.is_var_ref
                && !ri.has_global
            {
                return self.compile_error_expr(
                    ce.span,
                    "String.prototype.replaceAll called with a non-global RegExp argument (TypeError)",
                );
            }

            // ── Derive TypedArray type suffix for JsTypedArray calls ──
            let ta_type_suffix = if module == BuiltinModule::JsTypedArray {
                obj_name.as_ref().and_then(|name| {
                    self.type_info.var_types.get(name.as_str()).and_then(|zt| {
                        if let ZigType::NamedStruct(n) = zt {
                            Self::typedarray_type_suffix(n).map(|s| s.to_string())
                        } else {
                            None
                        }
                    })
                })
            } else {
                None
            };

            // ── Fix Object.keys/getOwnPropertyNames for struct-typed arguments ──
            // Object.keys(obj) / Object.getOwnPropertyNames(obj) where obj is a Zig struct
            // (not HashMap) needs a different runtime function (keysStruct /
            // getOwnPropertyNamesStruct) that uses comptime reflection.
            let method = if module == BuiltinModule::JsObject
                && (method == "keys" || method == "getOwnPropertyNames")
            {
                let ident_name = match args.first() {
                    Some(IrExpr::Ident(ident)) => Some(ident.zig_name.as_str()),
                    Some(IrExpr::TypedIdent { ident, .. }) => Some(ident.zig_name.as_str()),
                    _ => None,
                };
                if let Some(name) = ident_name {
                    if let Some(var_type) = self.type_info.var_types.get(name) {
                        if matches!(var_type, ZigType::Struct(_)) {
                            if method == "keys" {
                                "keysStruct".into()
                            } else {
                                "getOwnPropertyNamesStruct".into()
                            }
                        } else {
                            method
                        }
                    } else {
                        method
                    }
                } else {
                    method
                }
            } else {
                method
            };

            // ── Handle complex receiver expressions (method chaining) ──
            // When the receiver is a CallExpression (e.g., encodeURIComponent(str).replace(...)),
            // extract_callee_object_name_static returns None. We lower the receiver expression
            // and store it in obj_expr so the Emitter can inline it.
            let mut inner_expr = if obj_name.is_none() {
                if let Expression::StaticMemberExpression(sme) = &ce.callee {
                    match &sme.object {
                        // Complex receivers: lower the entire expression so the Emitter
                        // can inline it (e.g., new Date(0).getFullYear()).
                        Expression::CallExpression(_) | Expression::NewExpression(_) => {
                            Some(self.lower_expr(&sme.object))
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            } else {
                None
            };

            // ── Method chaining: re-attempt inline when inner expr is ArrayCallbackInline/ArrayMethodInline ──
            // For `arr.filter(x => x > 1).map(x => x * 2)`, the inner `.filter()` was inlined
            // as ArrayCallbackInline, but the outer `.map()` fell through because extract_callee_object_name_static
            // returned None (receiver is a CallExpression, not an Identifier).
            // We give it a second chance: use the inner inline's element type, generate a temp var name,
            // and construct the outer inline directly.
            if let Some(inner) = inner_expr.take() {
                let chain_elem_type = match &inner {
                    IrExpr::ArrayCallbackInline(cb) => Some(cb.elem_type.clone()),
                    IrExpr::ArrayMethodInline(mm) => Some(mm.elem_type.clone()),
                    _ => None,
                };

                if let Some(elem_type) = chain_elem_type {
                    // Try to re-attempt callback inline with a synthetic obj_name
                    if let Some(inlined) =
                        self.try_inline_array_callback_with_chain(ce, &builtin, &elem_type, &inner)
                    {
                        return inlined;
                    }

                    // Try to re-attempt method inline with a synthetic obj_name
                    if let Some(inlined) = self
                        .try_inline_array_method_with_chain(ce, &builtin, &args, &elem_type, &inner)
                    {
                        return inlined;
                    }

                    // Both failed — fall through to BuiltinCall with obj_expr
                    inner_expr = Some(inner);
                } else {
                    inner_expr = Some(inner);
                }
            }

            let obj_expr = inner_expr.map(Box::new);

            return IrExpr::BuiltinCall(crate::zigir::types::IrBuiltinCall {
                module,
                method,
                obj_name,
                obj_expr,
                args,
                return_type,
                regex_info,
                ta_type_suffix,
            });
        }

        // ── Non-builtin path: lower arguments ──
        // Reached only when `detect_builtin_call` returned None (not a builtin)
        // — in that case no `lower_args` call happened in Step 1, so do it here.
        let args = self.lower_args(&ce.arguments);

        // ── Step 1.5: BigInt variable method interception ──
        // `b.toString()` or `b.valueOf()` where `b` is a known BigInt variable.
        // detect_builtin_call routes `.toString()` to DateToString by default.
        // For BigInt-typed variables, we intercept here using type lookup.
        if let Expression::StaticMemberExpression(sme) = &ce.callee
            && let Expression::Identifier(id) = &sme.object
        {
            let var_name = id.name.as_str();
            let var_type = self.type_info.var_types.get(var_name).cloned().or_else(|| {
                self.fn_ctx
                    .as_ref()
                    .and_then(|ctx| ctx.fn_local_types.get(var_name).cloned())
            });
            if let Some(ZigType::BigInt) = var_type {
                let method = sme.property.name.as_str();
                match method {
                    "toString" => {
                        return IrExpr::BuiltinCall(crate::zigir::types::IrBuiltinCall {
                            module: BuiltinModule::JsBigInt,
                            method: "toString".into(),
                            obj_name: Some(var_name.to_string()),
                            obj_expr: None,
                            args,
                            return_type: ZigType::Str,
                            regex_info: None,
                            ta_type_suffix: None,
                        });
                    }
                    "valueOf" => {
                        return IrExpr::BuiltinCall(crate::zigir::types::IrBuiltinCall {
                            module: BuiltinModule::JsBigInt,
                            method: "valueOf".into(),
                            obj_name: Some(var_name.to_string()),
                            obj_expr: None,
                            args,
                            return_type: ZigType::BigInt,
                            regex_info: None,
                            ta_type_suffix: None,
                        });
                    }
                    "toLocaleString" => {
                        return IrExpr::BuiltinCall(crate::zigir::types::IrBuiltinCall {
                            module: BuiltinModule::JsBigInt,
                            method: "toLocaleString".into(),
                            obj_name: Some(var_name.to_string()),
                            obj_expr: None,
                            args,
                            return_type: ZigType::Str,
                            regex_info: None,
                            ta_type_suffix: None,
                        });
                    }
                    _ => {} // other methods fall through
                }
            }
        }

        // ── Step 1.6: RegExp variable method interception ──
        // `r.test(s)` or `r.exec(s)` where `r` is a known RegExp variable.
        // detect_builtin_call only identifies RegExpTest/RegExpExec for RegExpLiteral receivers.
        // For variable receivers, we intercept here using regexp_vars tracking.
        if let Expression::StaticMemberExpression(sme) = &ce.callee
            && let Expression::Identifier(id) = &sme.object
        {
            let var_name = id.name.as_str();
            if let Some(ctx) = &self.fn_ctx
                && ctx.regexp_vars.contains(var_name)
            {
                let method = sme.property.name.as_str();
                match method {
                    "test" => {
                        return IrExpr::BuiltinCall(crate::zigir::types::IrBuiltinCall {
                            module: BuiltinModule::JsRegExp,
                            method: "test".into(),
                            obj_name: Some(var_name.to_string()),
                            obj_expr: None,
                            args,
                            return_type: ZigType::Bool,
                            regex_info: Some(crate::zigir::types::IrRegexInfo {
                                pattern: None,
                                has_global: false,
                                is_var_ref: true,
                                var_name: Some(var_name.to_string()),
                            }),
                            ta_type_suffix: None,
                        });
                    }
                    "exec" => {
                        return IrExpr::BuiltinCall(crate::zigir::types::IrBuiltinCall {
                            module: BuiltinModule::JsRegExp,
                            method: "exec".into(),
                            obj_name: Some(var_name.to_string()),
                            obj_expr: None,
                            args,
                            return_type: ZigType::JsAny,
                            regex_info: Some(crate::zigir::types::IrRegexInfo {
                                pattern: None,
                                has_global: false,
                                is_var_ref: true,
                                var_name: Some(var_name.to_string()),
                            }),
                            ta_type_suffix: None,
                        });
                    }
                    _ => {} // other methods fall through
                }
            }
        }

        // ── Step 2: Identify callee pattern ──
        match &ce.callee {
            // Identifier callee: direct function call, host call, or closure
            Expression::Identifier(id) => {
                let name = id.name.as_str();

                // Host function call: starts with "host_"
                if let Some(host_name) = name.strip_prefix("host_") {
                    let is_async = self.async_host_fns.contains(name);
                    let return_type = self.infer_host_return_type(host_name);
                    return IrExpr::HostCall(crate::zigir::types::IrHostCall {
                        name: host_name.to_string(),
                        args,
                        return_type,
                        is_async,
                    });
                }

                // Closure / nested function call
                if self
                    .closure_mgr
                    .current_captured
                    .iter()
                    .any(|(n, _, _)| n.as_str() == name)
                    || self.is_closure_instance(name)
                {
                    return IrExpr::Call(crate::zigir::types::IrCallExpr {
                        callee: Box::new(IrExpr::Ident(IrIdent::new(name))),
                        args,
                        call_kind: CallKind::Closure,
                    });
                }

                // Nested function call: rewrite to name.call(args)
                if let Some(ctx) = &self.fn_ctx
                    && ctx.is_nested_fn(name)
                {
                    let callee_ident = self.make_ident(name);
                    return IrExpr::Call(crate::zigir::types::IrCallExpr {
                        callee: Box::new(IrExpr::Ident(callee_ident)),
                        args,
                        call_kind: CallKind::Closure,
                    });
                }

                // Direct user function call
                let args = self.pack_rest_args_if_needed(name, args);
                IrExpr::Call(crate::zigir::types::IrCallExpr {
                    callee: Box::new(IrExpr::Ident(IrIdent::new(name))),
                    args,
                    call_kind: CallKind::Direct,
                })
            }

            // Static member expression callee: obj.method()
            Expression::StaticMemberExpression(mem) => {
                let method_name = mem.property.name.as_str();
                let obj_expr = self.lower_expr(&mem.object);

                // Determine method object type for CallKind::Method
                let object_type = self.infer_method_object_kind(&mem.object);

                IrExpr::Call(crate::zigir::types::IrCallExpr {
                    callee: Box::new(IrExpr::FieldAccess {
                        object: Box::new(obj_expr),
                        field: method_name.to_string(),
                        field_kind: FieldKind::StructField,
                    }),
                    args,
                    call_kind: CallKind::Method { object_type },
                })
            }

            // Function expression / arrow function callee (IIFE)
            Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_) => {
                // IIFE: emit the function then call it
                let callee = self.lower_expr(&ce.callee);
                IrExpr::Call(crate::zigir::types::IrCallExpr {
                    callee: Box::new(callee),
                    args,
                    call_kind: CallKind::Closure,
                })
            }

            // Parenthesized expression containing function
            Expression::ParenthesizedExpression(_) => {
                let callee = self.lower_expr(&ce.callee);
                IrExpr::Call(crate::zigir::types::IrCallExpr {
                    callee: Box::new(callee),
                    args,
                    call_kind: CallKind::Closure,
                })
            }

            // Any other callee type (computed member, etc.)
            _ => {
                let callee = self.lower_expr(&ce.callee);
                IrExpr::Call(crate::zigir::types::IrCallExpr {
                    callee: Box::new(callee),
                    args,
                    call_kind: CallKind::Direct,
                })
            }
        }
    }

    /// Determine the MethodObjectKind for a method call's receiver object.
    pub(super) fn infer_method_object_kind(
        &self,
        obj: &Expression,
    ) -> crate::zigir::kinds::MethodObjectKind {
        use crate::zigir::kinds::MethodObjectKind;

        match obj {
            Expression::Identifier(id) => {
                if let Some(zig_type) = self.type_info.var_types.get(id.name.as_str()) {
                    match zig_type {
                        ZigType::ArrayList(_) => MethodObjectKind::ArrayList,
                        ZigType::Str => MethodObjectKind::String,
                        ZigType::NamedStruct(name) => match name.as_str() {
                            "Map" => MethodObjectKind::Map,
                            "Set" => MethodObjectKind::Set,
                            "Date" => MethodObjectKind::Date,
                            other => {
                                if self.class_names.contains(other) {
                                    MethodObjectKind::Class(other.to_string())
                                } else {
                                    MethodObjectKind::Unknown
                                }
                            }
                        },
                        ZigType::JsAny | ZigType::Anytype => MethodObjectKind::JsAny,
                        _ => MethodObjectKind::Unknown,
                    }
                } else {
                    MethodObjectKind::Unknown
                }
            }
            _ => MethodObjectKind::Unknown,
        }
    }

    /// Check if a name refers to a closure instance.
    pub(super) fn is_closure_instance(&self, name: &str) -> bool {
        self.closure_mgr.closure_instances.contains(name)
    }

    /// Infer the return type of a host function.
    /// Looks up the return type from `type_info.host_return_types`, which is
    /// populated during the type inference pass. Falls back to `ZigType::JsAny`
    /// if the function is not registered.
    pub(super) fn infer_host_return_type(&self, host_name: &str) -> ZigType {
        // The HashMap key includes the "host_" prefix (e.g. "host_add")
        let full_name = format!("host_{}", host_name);
        self.type_info
            .host_return_types
            .get(&full_name)
            .cloned()
            .unwrap_or(ZigType::JsAny)
    }

    /// Lower an await expression.
    pub(super) fn lower_await(&mut self, ae: &AwaitExpression) -> crate::zigir::types::IrExpr {
        let task_var = IrIdent::new(&self.name_mangler.next_name("_t"));
        let block_label = format!("blk_{}", self.name_mangler.peek_count("_t"));

        // Check if this is an async host function call
        if let Expression::CallExpression(call) = &ae.argument {
            let args: Vec<_> = call
                .arguments
                .iter()
                .filter_map(|a| a.as_expression().map(|e| self.lower_expr(e)))
                .collect();

            // Host async: callee is an identifier found in async_host_fns
            if let Expression::Identifier(id) = &call.callee {
                let name = id.name.as_str();
                if self.async_host_fns.contains(name) {
                    return crate::zigir::types::IrExpr::Await(crate::zigir::types::IrAwaitExpr {
                        task_var,
                        callee: Box::new(crate::zigir::types::IrExpr::Ident(IrIdent::new(name))),
                        args,
                        is_host_async: true,
                        block_label,
                    });
                }
            }

            // Regular async call (non-host)
            let callee = self.lower_expr(&call.callee);
            return crate::zigir::types::IrExpr::Await(crate::zigir::types::IrAwaitExpr {
                task_var,
                callee: Box::new(callee),
                args,
                is_host_async: false,
                block_label,
            });
        }

        // Non-call await (unusual but valid JS)
        let argument = self.lower_expr(&ae.argument);
        crate::zigir::types::IrExpr::Await(crate::zigir::types::IrAwaitExpr {
            task_var,
            callee: Box::new(argument),
            args: vec![],
            is_host_async: false,
            block_label,
        })
    }

    /// Lower a new expression.
    pub(super) fn lower_new(&mut self, ne: &NewExpression) -> crate::zigir::types::IrExpr {
        use crate::zigir::kinds::NewConstructor;

        // Determine constructor kind from callee
        let constructor = match &ne.callee {
            Expression::Identifier(id) => match id.name.as_str() {
                "Map" => NewConstructor::Map,
                "Set" => NewConstructor::Set,
                "Date" => {
                    // Determine DateConstructorKind from arguments
                    let kind = if ne.arguments.is_empty() {
                        crate::zigir::kinds::DateConstructorKind::Now
                    } else if ne.arguments.len() >= 2 {
                        crate::zigir::kinds::DateConstructorKind::FromComponents
                    } else if let Some(first_arg) = ne.arguments.first()
                        && let Some(expr) = first_arg.as_expression()
                    {
                        // Detect if argument is a string literal
                        let is_string = matches!(expr, Expression::StringLiteral(_));
                        if is_string {
                            crate::zigir::kinds::DateConstructorKind::FromString
                        } else {
                            crate::zigir::kinds::DateConstructorKind::FromMillis
                        }
                    } else {
                        crate::zigir::kinds::DateConstructorKind::Now
                    };
                    NewConstructor::Date(kind)
                }
                "RegExp" => NewConstructor::RegExp,
                "Int8Array" | "Uint8Array" | "Uint8ClampedArray" | "Int16Array" | "Uint16Array"
                | "Int32Array" | "Uint32Array" | "Float32Array" | "Float64Array"
                | "BigInt64Array" | "BigUint64Array" => {
                    let kind = match id.name.as_str() {
                        "Int8Array" => crate::zigir::kinds::TypedArrayKind::Int8Array,
                        "Uint8Array" => crate::zigir::kinds::TypedArrayKind::Uint8Array,
                        "Uint8ClampedArray" => {
                            crate::zigir::kinds::TypedArrayKind::Uint8ClampedArray
                        }
                        "Int16Array" => crate::zigir::kinds::TypedArrayKind::Int16Array,
                        "Uint16Array" => crate::zigir::kinds::TypedArrayKind::Uint16Array,
                        "Int32Array" => crate::zigir::kinds::TypedArrayKind::Int32Array,
                        "Uint32Array" => crate::zigir::kinds::TypedArrayKind::Uint32Array,
                        "Float32Array" => crate::zigir::kinds::TypedArrayKind::Float32Array,
                        "Float64Array" => crate::zigir::kinds::TypedArrayKind::Float64Array,
                        "BigInt64Array" => crate::zigir::kinds::TypedArrayKind::BigInt64Array,
                        "BigUint64Array" => crate::zigir::kinds::TypedArrayKind::BigUint64Array,
                        _ => crate::zigir::kinds::TypedArrayKind::Float64Array,
                    };
                    NewConstructor::TypedArray(kind)
                }
                "Error" => NewConstructor::Error("Error".to_string()),
                "TypeError" => NewConstructor::Error("TypeError".to_string()),
                "RangeError" => NewConstructor::Error("RangeError".to_string()),
                // Wrapper constructors — emit argument value directly (no wrapper object in Zig)
                "String" => {
                    return if let Some(first_arg) = ne.arguments.first()
                        && let Some(expr) = first_arg.as_expression()
                    {
                        self.lower_expr(expr)
                    } else {
                        crate::zigir::types::IrExpr::StringLiteral("".to_string())
                    };
                }
                "Number" => {
                    return if let Some(first_arg) = ne.arguments.first()
                        && let Some(expr) = first_arg.as_expression()
                    {
                        self.lower_expr(expr)
                    } else {
                        crate::zigir::types::IrExpr::IntLiteral(0)
                    };
                }
                "Boolean" => {
                    return if let Some(first_arg) = ne.arguments.first()
                        && let Some(expr) = first_arg.as_expression()
                    {
                        self.lower_expr(expr)
                    } else {
                        crate::zigir::types::IrExpr::BoolLiteral(false)
                    };
                }
                name if self.class_names.contains(name) => NewConstructor::Class(name.to_string()),
                // Known-unsupported constructors → structured Unsupported (Emitter generates proper @compileError)
                "ArrayBuffer" | "SharedArrayBuffer" | "Function" | "Promise" | "WeakMap"
                | "WeakSet" | "DataView" => {
                    NewConstructor::Unsupported(id.name.as_str().to_string())
                }
                other => {
                    let span = oxc_span::GetSpan::span(ne);
                    return self.compile_error_expr(
                        span,
                        format!("Unsupported NewExpression: new {}()", other),
                    );
                }
            },
            Expression::StaticMemberExpression(mem) => {
                if let Expression::Identifier(id) = &mem.object {
                    let obj_name = id.name.as_str();
                    let method_name = mem.property.name.as_str();
                    match obj_name {
                        "Intl" => {
                            let span = oxc_span::GetSpan::span(ne);
                            return self.compile_error_expr(
                                span,
                                format!(
                                    "Intl.{}() is not supported (use Zig/C library for internationalization)",
                                    method_name
                                ),
                            );
                        }
                        _ => {
                            let span = oxc_span::GetSpan::span(ne);
                            return self.compile_error_expr(span, "Unsupported NewExpression");
                        }
                    }
                } else {
                    let span = oxc_span::GetSpan::span(ne);
                    return self.compile_error_expr(span, "Unsupported NewExpression");
                }
            }
            _ => {
                let span = oxc_span::GetSpan::span(ne);
                return self.compile_error_expr(span, "Unsupported NewExpression");
            }
        };

        let args = self.lower_args(&ne.arguments);

        let result_type = match &constructor {
            NewConstructor::Map => ZigType::NamedStruct("Map".to_string()),
            NewConstructor::Set => ZigType::NamedStruct("Set".to_string()),
            NewConstructor::Date(_) => ZigType::NamedStruct("Date".to_string()),
            NewConstructor::RegExp => ZigType::NamedStruct("JsRegExp".to_string()),
            NewConstructor::TypedArray(_) => ZigType::NamedStruct("TypedArray".to_string()),
            NewConstructor::Class(name) => ZigType::NamedStruct(name.clone()),
            NewConstructor::Error(_) => ZigType::JsAny,
            _ => ZigType::JsAny,
        };

        crate::zigir::types::IrExpr::New(crate::zigir::types::IrNewExpr {
            constructor,
            args,
            result_type,
        })
    }

    /// If the callee has a synthetic rest param (from `arguments` usage),
    /// pack extra args beyond the declared param count into a `[]const JsAny` slice.
    ///
    /// For `function sum() { return arguments[0]; }` called as `sum(1, 2, 3)`:
    /// - declared_count = 0 (no explicit params)
    /// - All args [1, 2, 3] become rest → packed into `Spread(ArrayLiteral{ JsAny.from(1), ... })`
    /// - Emitter renders: `sum(&[_]JsAny{ JsAny.from(1), JsAny.from(2), JsAny.from(3) })`
    ///
    /// If the callee already has a Spread arg (e.g., `sum(...arr)`), pass it as-is.
    fn pack_rest_args_if_needed(
        &self,
        callee_name: &str,
        args: Vec<crate::zigir::types::IrExpr>,
    ) -> Vec<crate::zigir::types::IrExpr> {
        use crate::zigir::types::{IrArrayLiteral, IrCallExpr, IrExpr};

        if !self
            .type_info
            .functions_needing_synthetic_rest
            .contains(callee_name)
        {
            return args;
        }

        // Account for async `io` param (injected by lower_fn_params, not in fn_param_types)
        let is_async = self
            .type_info
            .is_async
            .get(callee_name)
            .copied()
            .unwrap_or(false);
        let declared_count = self
            .type_info
            .fn_param_types
            .get(callee_name)
            .map(|params| params.len())
            .unwrap_or(0)
            + if is_async { 1 } else { 0 };

        if args.len() <= declared_count {
            // No extra args — pass empty slice for rest param
            let mut result = args;
            result.push(IrExpr::Spread(Box::new(IrExpr::ArrayLiteral(
                IrArrayLiteral {
                    elements: vec![],
                    spread_indices: vec![],
                },
            ))));
            return result;
        }

        // Split into regular and rest args
        let mut regular_args = args;
        let rest_args = regular_args.split_off(declared_count);

        // If rest_args is a single Spread, pass as-is (e.g., sum(...arr))
        if rest_args.len() == 1 && matches!(rest_args[0], IrExpr::Spread(_)) {
            regular_args.extend(rest_args);
            return regular_args;
        }

        // Pack rest args into Spread(ArrayLiteral{ JsAny.from(arg), ... })
        let rest_elements: Vec<IrExpr> = rest_args
            .into_iter()
            .map(|arg| {
                IrExpr::Call(IrCallExpr {
                    callee: Box::new(IrExpr::FieldAccess {
                        object: Box::new(IrExpr::Ident(IrIdent::new("JsAny"))),
                        field: "from".to_string(),
                        field_kind: FieldKind::StructField,
                    }),
                    args: vec![arg],
                    call_kind: CallKind::Direct,
                })
            })
            .collect();

        regular_args.push(IrExpr::Spread(Box::new(IrExpr::ArrayLiteral(
            IrArrayLiteral {
                elements: rest_elements,
                spread_indices: vec![],
            },
        ))));
        regular_args
    }
}
