# ZigIR 迁移任务列表

> 基于 ZigIR 设计方案 v1.0，结合当前 codegen 模块分析，拆解为可执行任务。
> 每个任务完成后必须通过 361 单元测试 + 双轨字符串等价验证。

---

## 阶段 -1：结构预热（为 ZigIR 铺路）

> 目的：建立与 lower/ 子模块 1:1 对应的 codegen 模块边界，降低后续迁移的认知成本。

### P-1.1 提取 class 代码生成 → `codegen/class.rs`
- 从 stmt.rs 迁出 ~538 行：`emit_class`, `collect_implicit_class_fields`, `emit_class_method`, `emit_class_constructor`, `emit_class_regular_method`, `emit_stmt_with_this_rewrite`, `emit_static_field_init`, 以及辅助函数 `property_key_name`, `is_constructor_method`, `expr_to_default_str`, `scan_ret_expr_type`
- 同时删除 helpers.rs 中重复的 `property_key_name` 方法（仅保留 stmt.rs 中的完整版，改为 `pub(crate)` 自由函数）
- **验证**：361 tests pass

### P-1.2 提取 closures 代码生成 → `codegen/closures.rs`
- 从 stmt.rs 迁出 ~778 行：`emit_arrow_function`, `emit_fn_expr`, `emit_closure_struct`, `detect_mutated_vars_in_stmts`, `detect_mutated_in_stmt`/`detect_mutated_in_expr`, `collect_captured_vars`, `detect_fn_body_captures`, `collect_local_declarations`, `collect_idents_from_stmt`/`collect_idents_from_expr`
- **验证**：361 tests pass

### P-1.3 提取 lookup 表 → `codegen/tables.rs`
- 从 expr.rs 迁出 `math_one_arg_desc()` (~110 行) 和 `string_runtime_desc()` (~230 行)，以及 `MathOneArgDesc`, `StringRuntimeDesc` 结构体定义
- 纯数据，无 `&mut self`，无行为变化
- **验证**：361 tests pass

### P-1.4 Console 内建去重
- builtins.rs 中 `emit_builtin_console` 的 3 个分支（log/error/warn）逻辑几乎一致，统一为查表 + 单段逻辑
- 净减 ~45 行
- **验证**：361 tests pass

---

## 阶段 0：ZigIR 基础设施

> 目的：定义完整的 IR 类型体系，新增代码零风险。

### 0.1 创建 `zigir/` 模块骨架
- 创建 `js2zig-core/src/zigir/` 目录
- 创建 `zigir/mod.rs`：公开 API 声明
- 更新 `lib.rs`：添加 `pub mod zigir;`
- **验证**：cargo build 编译通过（空模块）

### 0.2 实现基础类型：IrIdent + SourceSpan + IrDiagnostic
- `zigir/ident.rs`：`IrIdent { js_name, zig_name }` + `NameMangler`
  - 复用现有 helpers.rs 中的 `zig_safe_name()` 逻辑
  - `IrIdent::new(js_name: &str) -> Self` 自动完成关键字转义
- `zigir/source_span.rs`：`SourceSpan { js_line, js_col, js_file }`
- `zigir/diagnostic.rs`：`IrDiagnostic { level, span, message }`, `DiagnosticLevel { Warning, Error }`
- 单元测试：构造 + 访问 + 序列化
- **验证**：cargo test 新增测试 pass

### 0.3 实现辅助枚举：运算符 + 字段类型 + 调用类型
- `zigir/ops.rs`：`BinOp`, `UnaOp`, `LogicalOp`, `UpdateOp`, `AssignOp`（参考设计文档 §3.5-3.6）
- `zigir/kinds.rs`：`FieldKind`, `IndexKind`, `ComputedKeyKind`, `CallKind`, `MethodObjectKind`
  - 与现有 codegen 的硬编码字符串常量对应
- `zigir/builtins.rs`：`BuiltinModule` 枚举（JsArray, JsString, JsDate 等 14 种）
- 单元测试：枚举覆盖 + Debug 格式化
- **验证**：cargo test 新增测试 pass

### 0.4 实现顶层 IR 类型：IrModule + IrImport + IrTypedef + IrClosureStruct
- `zigir/types.rs`（上层）：
  - `IrModule { name, imports, typedefs, closure_structs, declarations, diagnostics, cabi_exports }`
  - `IrImport { module_name, items }`
  - `IrTypedef { name, fields, is_opaque }`
  - `IrClosureStruct { name, captured, fn_param, return_type, body }`
  - `IrCabiExport { name, params, return_type }`
- 单元测试：构造完整但最小的 IrModule
- **验证**：cargo test 新增测试 pass

### 0.5 实现声明 IR：IrDecl + IrVarDecl + IrFnDecl + IrClassDecl
- 同在 `zigir/types.rs`：
  - `IrDecl` 枚举：Var / Fn / Class / CompileError
  - `IrVarDecl { name, is_const, zig_type, init, is_json_parse, needs_var_suppression }`
  - `IrFnDecl { name, params, return_type, body, is_export, is_async, can_throw, is_cabi }`
  - `IrParam { name, zig_type }`
  - `IrBlock { stmts, label }`
  - `IrClassDecl { name, fields, constructor, methods, static_inits, extends }`
  - `IrClassField { name, zig_type, default }`
  - `IrClassMethod { name, params, return_type, body, is_static }`
- 单元测试：构造各种 IrDecl 变体
- **验证**：cargo test 新增测试 pass

### 0.6 实现语句 IR：IrStmt 全部变体
- `zigir/types.rs`（续）：
  - `IrStmt` 枚举的 ~15 个变体（VarDecl, Assign, If, While, DoWhile, For, ForIn, ForOf, Switch, Try, Throw, Return, Break, Continue, Expr, Block, CompileError, Comment）
  - `IrAssignTarget` 枚举：Ident / Member / Index / Destructure
  - `IrDestructureBinding { pattern, default }`
  - `IrSwitchCase { test, body }`
- 单元测试：构造各种 IrStmt 变体
- **验证**：cargo test 新增测试 pass

### 0.7 实现表达式 IR：IrExpr 全部变体
- `zigir/types.rs`（续）：
  - `IrExpr` 枚举的 ~20 个变体（字面量、标识符、运算、调用、成员访问、对象/数组、闭包/箭头、条件、模板、异步、构造、特殊）
  - `IrCallExpr`, `IrBuiltinCall`, `IrHostCall`, `IrAwaitExpr`
  - `IrClosure { struct_name, captured, fn_param, return_type, body, instance_name }`
  - `IrCapture { name, zig_type, is_mutable }`
  - `IrArrowFn { params, return_type, body, is_concise }`
  - `IrFnExpr { name, params, return_type, body }`
  - `IrArrayLiteral { elements, spread_indices }`
  - `IrObjectLiteral { fields, spreads }`
  - `IrNewExpr { callee, args }`
- 单元测试：构造各种 IrExpr 变体
- **验证**：cargo test 新增测试 pass

### 0.8 IrModule 序列化/反序列化 + JSON 快照基础设施
- 为 IrModule 实现 `serde::Serialize` / `serde::Deserialize`（derive 宏）
- 添加 `serde`, `serde_json` 到 Cargo.toml 的 dev-dependencies
- 单元测试：round-trip 序列化（构造 → JSON → 反序列化 → 比较）
- **验证**：cargo test 新增测试 pass

---

## 阶段 1：Lowerer 开发（AST → ZigIR）

> 目的：实现 Lowerer，逐方法将 emit_* 迁移为 lower_*，双轨并行验证。
> 每个子任务完成后对比新旧输出字符串，保证等价。

### 1.1 Lowerer 骨架 + pipeline 双轨框架
- 创建 `zigir/lower/mod.rs`：`Lowerer` 结构体（持有 type_info, jsdoc_data, names, closures 等 Lowerer 状态）
- 创建 `zigir/lower/helpers.rs`：共享降级辅助方法
- 在 `pipeline.rs` 中添加双轨比较逻辑：
  ```
  old_zig = Codegen::generate(...)
  ir_module = Lowerer::new(...).lower(program)
  // 开发阶段：仅记录差异，使用旧输出
  ```
- **验证**：编译通过，旧路径不变

### 1.2 实现 lower_program + lower_imports + lower_typedefs
- `lower_program(program) -> IrModule`：遍历 AST 顶层，分发到各 lower_*
- `lower_imports() -> Vec<IrImport>`：基于 type_info 中的 runtime 依赖推断需要的 imports
- `lower_typedefs() -> Vec<IrTypedef>`：基于 jsdoc_data 转换 @typedef 注解
- **验证**：对 3 个简单测试用例（无函数体）验证 IR 生成 + 字符串等价

### 1.3 实现 lower_var_decl + lower_fn_decl（基本声明）
- `lower_var_decl(decl) -> IrDecl::Var`：变量声明，含类型信息、初始化表达式
- `lower_fn_decl(decl) -> IrDecl::Fn`：函数声明，含参数类型、返回类型、异步标记
- 先处理无闭包/无 class 的简单函数
- **验证**：对 10 个现有基本测试用例验证双轨等价

### 1.4 实现 lower_stmt —— 控制流（if/while/do-while/for/switch）
- `lower_if`, `lower_while`, `lower_do_while`, `lower_for`, `lower_switch`
- 含 labeled 循环支持
- **验证**：对 15 个控制流测试用例验证双轨等价

### 1.5 实现 lower_stmt —— for-in / for-of / 迭代器
- `lower_for_in`, `lower_for_of`
- 处理 struct vs map 的 for-in 区分逻辑
- **验证**：对 for-in/for-of 测试用例验证双轨等价

### 1.6 实现 lower_expr —— 字面量 + 标识符 + 运算表达式
- `lower_literal`, `lower_ident`, `lower_binary`, `lower_unary`, `lower_logical`, `lower_update`, `lower_assignment`
- 对应 codegen/expr.rs 的基础表达式部分
- **验证**：对 20 个运算表达式测试用例验证双轨等价

### 1.7 实现 lower_expr —— 调用 + 成员访问
- `lower_call`, `lower_member_access`, `lower_computed_member`
- 区分 Direct/Method/Closure 调用类型
- 处理 optional chaining (?.)
- **验证**：对 15 个调用/成员访问测试用例验证双轨等价

### 1.8 实现 lower_builtin_call —— 13 个类别逐一迁移
- 对应 builtins.rs 中的 13 个 `emit_builtin_*` 函数
- 每个类别转为 `lower_builtin_<category>() -> Option<IrExpr::BuiltinCall>`
- 核心变化：不再直接拼接字符串，而是返回结构化的 `IrBuiltinCall { module, method, args, return_type }`
- 按类别逐一实现并验证：Math → Console → JSON → Symbol → RegExp → Number → Global → Constructors → Date → Object → Map/Set → String → Array
- **验证**：每迁移完一个类别，跑全量 361 tests 确认双轨等价

### 1.9 实现 lower_expr —— 数组/对象字面量 + 解构
- `lower_array_literal`, `lower_object_literal`, `lower_destructure`
- 处理 spread 语法
- **验证**：对解构和字面量测试用例验证双轨等价

### 1.10 实现 lower_stmt —— 闭包 + 箭头函数 + 函数表达式
- `lower_arrow_fn`, `lower_fn_expr`, `lower_closure_struct`
- 闭包捕获分析（复用 ClosureManager 逻辑）
- 对应 codegen/closures.rs
- **验证**：对 10 个闭包/箭头测试用例验证双轨等价

### 1.11 实现 lower_stmt —— class 声明
- `lower_class`, `lower_class_constructor`, `lower_class_method`
- 处理 this 重写、默认字段值、静态初始化
- 对应 codegen/class.rs
- **验证**：对 10 个 class 测试用例验证双轨等价

### 1.12 实现 lower_stmt —— try-catch / throw
- `lower_try`, `lower_throw`
- 处理 catch 变量绑定、finally 块
- **验证**：对 13 个 try-catch 测试用例验证双轨等价

### 1.13 实现 lower_expr —— template literal + string concat + 条件表达式
- `lower_template_literal`, `lower_string_concat`, `lower_conditional`
- 处理多行模板、嵌套表达式
- **验证**：对 10 个相关测试用例验证双轨等价

### 1.14 实现 lower_expr —— await + async
- `lower_await`：
- 处理 host async 调用、task 变量生成、block label
- **验证**：对 await 相关测试用例验证双轨等价

### 1.15 实现 lower_expr —— new + typeof + void + 其他边缘表达式
- `lower_new`, `lower_typeof`, `lower_void`, `lower_paren`, `lower_sequence`
- **验证**：对剩余表达式测试用例验证双轨等价

### 1.16 Lowerer 全量切换 + 旧 Codegen 移除
- 全量 361 测试双轨等价验证
- pipeline.rs 中默认使用 Lowerer+Emitter 路径
- 保留旧路径作为 `--legacy-codegen` 回退选项（可选）
- **验证**：全量回归（cargo test + clippy + fmt + MDN e2e）

---

## 阶段 2：Emitter 开发（ZigIR → String）

> 目的：实现纯格式化 Emitter，将 IrModule 转为 Zig 源码字符串。
> Emitter 是纯函数——不做类型推断、不做名称解析、不做控制流分析。

### 2.1 Emitter 骨架
- 创建 `zigir/emit/mod.rs`：`Emitter { output: String, indent: usize }`
- `Emitter::emit(module: &IrModule) -> String` 入口
- **验证**：编译通过

### 2.2 实现 emit_module + emit_import + emit_typedef + emit_closure_struct
- IrModule 的四段式输出：imports → typedefs → closure structs → declarations
- **验证**：对 Phase 1.2 的 3 个简单用例验证字符串输出

### 2.3 实现 emit_decl —— var 声明
- `emit_var_decl`：const/var + 类型注解 + 初始化器
- 处理 JSON.parse 特殊格式、未使用变量抑制（`_ = &var`）
- **验证**：双轨等价

### 2.4 实现 emit_decl —— fn 声明
- `emit_fn_decl`：pub fn / pub export fn + 参数列表 + 返回类型 + body
- 处理 async（io: anytype 参数）、C ABI 签名
- **验证**：双轨等价

### 2.5 实现 emit_decl —— class 声明
- `emit_class_decl`：struct 定义 + init + methods + static field init
- **验证**：双轨等价

### 2.6 实现 emit_stmt —— 控制流
- `emit_if`, `emit_while`, `emit_do_while`, `emit_for`, `emit_switch`
- Labeled 块 + break/continue
- **验证**：双轨等价

### 2.7 实现 emit_stmt —— for-in / for-of
- inline for vs switch 分发
- **验证**：双轨等价

### 2.8 实现 emit_stmt —— try-catch / throw
- try/catch/finally 的 Zig 模式
- **验证**：双轨等价

### 2.9 实现 emit_expr —— 字面量 + 运算符
- 所有字面量格式化 + BinOp/UnaOp/LogicalOp/UpdateOp/AssignOp 到 Zig 的映射
- **验证**：双轨等价

### 2.10 实现 emit_expr —— 调用 + 成员访问
- Direct/Method/Closure 调用的格式化
- Optional chaining 的 null-check 展开
- **验证**：双轨等价

### 2.11 实现 emit_expr —— BuiltinCall
- 13 个 BuiltinModule 的格式化
- 这一步直接复用 builtins.rs 中的格式化逻辑，但数据来自 IrBuiltinCall 结构
- **验证**：双轨等价

### 2.12 实现 emit_expr —— 数组/对象/解构
- 数组构造、对象构造、spread 展开
- **验证**：双轨等价

### 2.13 实现 emit_expr —— 闭包/箭头/函数表达式
- 闭包 struct + call() 方法 + 实例化
- **验证**：双轨等价

### 2.14 实现 emit_expr —— 剩余表达式类型
- Conditional, TemplateLiteral, Await, New, Typeof, Void, Paren, Sequence, Spread
- **验证**：双轨等价

### 2.15 Emitter 全量验证 + 最终切换
- 全量 361 测试字符串等价（Emitter 输出 == 旧 Codegen 输出）
- 删除旧 Codegen 路径
- 移除 `--legacy-codegen` 回退
- **验证**：全量回归（cargo test + clippy + fmt + MDN e2e）

---

## 阶段 3：优化 Pass

> 目的：利用 IR 结构化特性实现验证和优化。

### 3.1 IrPass trait + PassPipeline 框架
- `zigir/passes/mod.rs`：`IrPass` trait, `PassResult`, `PassPipeline`
- 默认 pipeline 配置
- **验证**：空 pipeline 通过

### 3.2 实现 ValidatePass
- 类型一致性检查：所有 IrExpr 推断类型与 TypeCheckResult 一致
- 名称唯一性：同一作用域无重复标识符
- 闭包完整性：captured 列表与实际引用匹配
- C ABI 兼容性：导出函数参数/返回值都是 C ABI 安全类型
- **验证**：针对已知问题场景的单元测试

### 3.3 实现 DeadCodeElimPass
- 删除未被引用的顶层声明
- 删除不可达代码（return 之后的语句）
- **验证**：生成代码体积减少 + 功能不变

### 3.4 实现 ConstantFoldPass
- 常量表达式折叠：`1 + 2 → 3`, `"a" ++ "b" → "ab"`
- **验证**：特定测试用例

### 3.5 实现 SourceMapGenPass
- 为每个 IrStmt/IrExpr 附加 SourceSpan
- 输出 source map JSON
- **验证**：source map 正确性

---

## 时间估算

| 阶段 | 任务数 | 预计周期 | 备注 |
|------|--------|---------|------|
| -1 结构预热 | 4 | 3-4 天 | 低风险，每步可验证 |
| 0 基础设施 | 8 | 1-2 周 | 纯新增代码，零风险 |
| 1 Lowerer | 16 | 3-5 周 | 核心工作量，双轨并行 |
| 2 Emitter | 15 | 2-3 周 | 纯格式化，逻辑简单 |
| 3 Opt Passes | 5 | 1-2 周 | 可选，按需实现 |
| **总计** | **48** | **8-14 周** | |

## 验证策略

- **阶段 -1 / 0**：每步 `cargo test` (361 pass) + clippy + fmt
- **阶段 1 / 2**：双轨并行——每个 lower_*/emit_* 完成后对比新旧输出字符串，确保一致
- **阶段 3**：回归测试 + 专项测试（生成代码体积、source map 正确性）
- **里程碑**：每个阶段结束时执行完整回归（cargo test + clippy + fmt + MDN e2e + example projects）
