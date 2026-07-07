// zigir/lower/expr/call.rs
// Call, new, await expression lowering + helpers.

use oxc_ast::ast::*;

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
                            "getOwnPropertyNames" => Some(
                                "Object.getOwnPropertyNames() is not yet implemented in js2zig".to_string(),
                            ),
                            _ => None,
                        },
                        "Map" if method_name == "groupBy" => Some(
                            "Map.groupBy() is not supported (requires iterable grouping)".to_string(),
                        ),
                        _ => None,
                    }
                } else {
                    None
                }
            }
            Expression::Identifier(id) => match id.name.as_str() {
                name @ "Atomics" | name @ "Reflect" => {
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
            return IrExpr::CompileError {
                span: self.span_to_source_span(ce.span),
                msg: err_msg,
            };
        }

        let args: Vec<IrExpr> = ce
            .arguments
            .iter()
            .map(|arg| {
                match arg {
                    Argument::SpreadElement(se) => {
                        IrExpr::Spread(Box::new(self.lower_expr(&se.argument)))
                    }
                    // Argument inherits all Expression variants
                    _ => {
                        // All Expression variants are directly accessible
                        let expr = arg.as_expression().unwrap();
                        self.lower_expr(expr)
                    }
                }
            })
            .collect();

        // ── Step 1: Builtin detection ──
        if let Some(builtin) = crate::native_builtins::detect_builtin_call(ce) {
            // ── Step 1a: Array callback inlining ──
            if let Some(inlined) = self.try_inline_array_callback(ce, &builtin) {
                return inlined;
            }

            // ── Step 1b: Array non-callback method inlining ──
            if let Some(inlined) = self.try_inline_array_method(ce, &builtin, &args) {
                return inlined;
            }

            // ── Step 1c: eval() → compile error ──
            if matches!(builtin, crate::native_builtins::BuiltinCall::Eval) {
                return IrExpr::CompileError {
                    span: self.span_to_source_span(ce.span),
                    msg: "eval() is not supported (security risk, cannot dynamically execute at compile time)".to_string(),
                };
            }

            let (module, method, return_type) = builtin_call_to_ir(&builtin);
            let obj_name = Self::extract_callee_object_name_static(&ce.callee);

            // ── Fix string-variable methods misidentified as array ──
            // detect_builtin_call only checks if the callee object is a StringLiteral,
            // not if it's a variable of type string. Fix up the module/method here.
            let (module, method, return_type) = if let Some(name) = &obj_name {
                if let Some(var_type) = self.type_info.var_types.get(name.as_str()) {
                    if matches!(var_type, ZigType::Str)
                        && module == BuiltinModule::JsArray
                    {
                        match method.as_str() {
                            "at" => (
                                BuiltinModule::JsString,
                                "at".into(),
                                ZigType::Str,
                            ),
                            "indexOf" => (
                                BuiltinModule::JsString,
                                "indexOf".into(),
                                ZigType::I64,
                            ),
                            "includes" => (
                                BuiltinModule::JsString,
                                "includes".into(),
                                ZigType::Bool,
                            ),
                            "lastIndexOf" => (
                                BuiltinModule::JsString,
                                "lastIndexOf".into(),
                                ZigType::I64,
                            ),
                            "slice" => (
                                BuiltinModule::JsString,
                                "slice".into(),
                                ZigType::Str,
                            ),
                            _ => (module, method, return_type),
                        }
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
            let obj_name = Self::extract_callee_object_name_static(&ce.callee);

            // ── Extract regex metadata for match/matchAll/search ──
            let regex_info = Self::extract_regex_info(ce, &builtin);

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

            // ── Fix Object.keys for struct-typed arguments ──
            // Object.keys(obj) where obj is a Zig struct (not HashMap) needs
            // a different runtime function (keysStruct) that uses comptime reflection.
            let method = if module == BuiltinModule::JsObject && method == "keys" {
                if let Some(IrExpr::Ident(ident)) = args.first() {
                    if let Some(var_type) = self.type_info.var_types.get(ident.zig_name.as_str()) {
                        if matches!(var_type, ZigType::Struct(_)) {
                            "keysStruct".into()
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
            let obj_expr = if obj_name.is_none() {
                if let Expression::StaticMemberExpression(sme) = &ce.callee {
                    match &sme.object {
                        // Complex receivers: lower the entire expression so the Emitter
                        // can inline it (e.g., new Date(0).getFullYear()).
                        Expression::CallExpression(_) | Expression::NewExpression(_) => {
                            Some(Box::new(self.lower_expr(&sme.object)))
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            } else {
                None
            };

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

        // ── Step 1.5: RegExp variable method interception ──
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
                            "Date" | "JsDate" => MethodObjectKind::Date,
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
    pub(super) fn infer_host_return_type(&self, _host_name: &str) -> ZigType {
        // TODO: look up host function return type from type_info
        ZigType::JsAny
    }

    /// Lower an await expression.
    pub(super) fn lower_await(&mut self, ae: &AwaitExpression) -> crate::zigir::types::IrExpr {
        let task_var = IrIdent::new(&self.name_mangler.next_name("_t"));
        let block_label = format!("blk_{}", self.name_mangler.peek_count("_t"));

        // Check if this is an async host function call
        if let Expression::CallExpression(call) = &ae.argument {
            if let Expression::Identifier(id) = &call.callee {
                let name = id.name.as_str();
                if self.async_host_fns.contains(name) {
                    // Host async function: emit as host.{name}_async
                    let args: Vec<_> = call
                        .arguments
                        .iter()
                        .filter_map(|a| a.as_expression().map(|e| self.lower_expr(e)))
                        .collect();
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
            let args: Vec<_> = call
                .arguments
                .iter()
                .filter_map(|a| a.as_expression().map(|e| self.lower_expr(e)))
                .collect();
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
                | "Int32Array" | "Uint32Array" | "Float32Array" | "Float64Array" => {
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
                    }
                }
                "Number" => {
                    return if let Some(first_arg) = ne.arguments.first()
                        && let Some(expr) = first_arg.as_expression()
                    {
                        self.lower_expr(expr)
                    } else {
                        crate::zigir::types::IrExpr::IntLiteral(0)
                    }
                }
                "Boolean" => {
                    return if let Some(first_arg) = ne.arguments.first()
                        && let Some(expr) = first_arg.as_expression()
                    {
                        self.lower_expr(expr)
                    } else {
                        crate::zigir::types::IrExpr::BoolLiteral(false)
                    }
                }
                name if self.class_names.contains(name) => NewConstructor::Class(name.to_string()),
                // Known-unsupported constructors → structured Unsupported (Emitter generates proper @compileError)
                "ArrayBuffer" | "SharedArrayBuffer" | "Function" | "Promise" | "WeakMap"
                | "WeakSet" | "DataView" => {
                    NewConstructor::Unsupported(id.name.as_str().to_string())
                }
                other => {
                    let span = oxc_span::GetSpan::span(ne);
                    return crate::zigir::types::IrExpr::CompileError {
                        span: self.span_to_source_span(span),
                        msg: format!("Unsupported NewExpression: new {}()", other),
                    };
                }
            },
            _ => {
                let span = oxc_span::GetSpan::span(ne);
                return crate::zigir::types::IrExpr::CompileError {
                    span: self.span_to_source_span(span),
                    msg: "Unsupported NewExpression".to_string(),
                };
            }
        };

        let args: Vec<crate::zigir::types::IrExpr> = ne
            .arguments
            .iter()
            .map(|arg| match arg {
                Argument::SpreadElement(se) => {
                    crate::zigir::types::IrExpr::Spread(Box::new(self.lower_expr(&se.argument)))
                }
                _ => {
                    let expr = arg.as_expression().unwrap();
                    self.lower_expr(expr)
                }
            })
            .collect();

        let result_type = match &constructor {
            NewConstructor::Map => ZigType::NamedStruct("Map".to_string()),
            NewConstructor::Set => ZigType::NamedStruct("Set".to_string()),
            NewConstructor::Date(_) => ZigType::NamedStruct("JsDate".to_string()),
            NewConstructor::RegExp => ZigType::JsAny,
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
}
