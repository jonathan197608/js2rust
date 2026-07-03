ZigIR 中间表示设计方案

js2rust Codegen IR Layer — Design & Implementation

2026-06-30  |  Version 1.0

# 1. 背景与动机

当前 js2rust 的代码生成（codegen）直接从 AST 遍历生成 Zig 源码字符串，存在以下核心问题：

无结构验证：format!() 拼接的字符串无法在生成阶段被验证，错误只能在 Zig 编译时暴露

状态爆炸：Codegen 持有 30+ 可变字段，包含 9 个计数器、嵌套 HashMap/HashSet、多层状态标志，难以推理

难以测试：测试只能比较最终字符串输出，无法验证中间结构的正确性

无法优化：没有中间表示，无法在类型推断和代码生成之间插入优化 pass

单后端锁定：string-based 输出天然绑定 Zig 语法，无法扩展到其他后端

ZigIR 是在 AST 和最终 Zig 源码之间引入的一层结构化中间表示。其设计定位为

中层级（Mid-level IR）——已经完成了 JS→Zig 的语义降级（如闭包降级为 struct+call、for-in 降级为 inline for），但尚未绑定具体的字符串格式。

## 1.1 ZigIR 在流水线中的位置

Current:  AST ──→ Codegen (string concat) ──→ Zig source

Proposed: AST ──→ Lower (AST→ZigIR) ──→ [Opt Passes] ──→ Emit (ZigIR→String) ──→ Zig source

Phase 1          Phase 2              Phase 3

| Phase | 输入 | 输出 | 职责 |
| --- | --- | --- | --- |
| Lower | AST + TypeCheckResult + JSDocData | ZigIR Module | 语义降级：JS 概念 → Zig 概念 |
| Opt Passes | ZigIR Module | ZigIR Module | 验证、去重、简化、注入调试信息 |
| Emit | ZigIR Module | String | 格式化输出为 Zig 源码文本 |

# 2. 类型系统设计

## 2.1 ZigType 复用

ZigIR 直接复用现有 native_proto::ZigType 枚举，不重新定义类型系统。ZigType 已经覆盖了所有需要的类型：Void, I64, F64, Bool, Str, ArrayList(T), Struct(fields), NamedStruct(name), JsAny, Opt(T), HashMap, Set, JsValue, Anytype, 闭包类型等。

## 2.2 新增辅助类型

/// IR 内部的标识符，已完成 Zig 关键字转义和 shadow 重命名

pub struct IrIdent {

/// 原始 JS 名称

pub js_name: String,

/// 转义后的 Zig 名称（已处理关键字冲突和 shadow）

pub zig_name: String,

}

/// 源码位置信息（用于生成 source map 和诊断信息）

pub struct SourceSpan {

pub js_line: usize,

pub js_col: usize,

pub js_file: String,

}

# 3. IR 节点体系

## 3.1 顶层结构 (Module)

/// 一个完整的 Zig 模块（一个 JS 文件的转译结果）

pub struct IrModule {

/// 模块名称（sanitized）

pub name: String,

/// 依赖导入列表

pub imports: Vec<IrImport>,

/// JSDoc @typedef 结构体定义

pub typedefs: Vec<IrTypedef>,

/// 闭包结构体定义（需要前置声明）

pub closure_structs: Vec<IrClosureStruct>,

/// 顶层声明（函数、变量、类、compileError）

pub declarations: Vec<IrDecl>,

/// 诊断信息

pub diagnostics: Vec<IrDiagnostic>,

/// C ABI 导出元数据

pub cabi_exports: Vec<IrCabiExport>,

}

## 3.2 导入与声明

pub struct IrImport {

pub module_name: String,   // sanitized Zig module name

pub items: Vec<(String, String)>,  // (imported_name, local_name)

}

pub enum IrDecl {

/// const/var 变量声明

Var(IrVarDecl),

/// function 声明（含 export/常规/C ABI）

Fn(IrFnDecl),

/// class 声明 → struct + init + methods

Class(IrClassDecl),

/// @compileError

CompileError { span: SourceSpan, msg: String },

}

## 3.3 变量声明

pub struct IrVarDecl {

pub name: IrIdent,

pub is_const: bool,          // const vs var

pub zig_type: Option<ZigType>, // None = 让 Zig 推断

pub init: Option<IrExpr>,    // None = 无初始化器（错误）

pub is_json_parse: bool,     // JSON.parse(@type) 特殊处理

pub needs_var_suppression: bool, // _ = &var; 抑制 unused 警告

}

## 3.4 函数声明

pub struct IrFnDecl {

pub name: IrIdent,

pub params: Vec<IrParam>,

pub return_type: ZigType,

pub body: IrBlock,

pub is_export: bool,         // pub export fn

pub is_async: bool,          // 接受 io: anytype 参数

pub can_throw: bool,         // 包含 throw/try-catch

pub is_cabi: bool,           // C ABI 导出函数

}

pub struct IrParam {

pub name: IrIdent,

pub zig_type: ZigType,       // Anytype = 类型未确定

}

pub struct IrBlock {

pub stmts: Vec<IrStmt>,

pub label: Option<String>,   // 带标签块（break :label）

}

# 3.5 语句节点 (IrStmt)

pub enum IrStmt {

// ── 变量与赋值 ──

VarDecl(IrVarDecl),

Assign { target: IrAssignTarget, op: AssignOp, value: IrExpr },

// ── 控制流 ──

If { cond: IrExpr, then: IrBlock, else_: Option<IrBlock> },

While { cond: IrExpr, body: IrBlock },

DoWhile { body: IrBlock, cond: IrExpr },

For { init: Option<Box<IrStmt>>, cond: Option<IrExpr>, update: Option<Box<IrStmt>>, body: IrBlock },

ForIn { var: IrIdent, iterable: IrExpr, body: IrBlock, is_struct: bool },

ForOf { var: IrIdent, iterable: IrExpr, body: IrBlock, is_async: bool },

Switch { expr: IrExpr, cases: Vec<IrSwitchCase> },

// ── 异常处理 ──

Try { try_block: IrBlock, catch_var: Option<IrIdent>, catch_block: IrBlock, finally: Option<IrBlock> },

Throw { value: IrExpr },

// ── 函数控制 ──

Return { value: Option<IrExpr> },

Break { label: Option<String> },

Continue { label: Option<String> },

// ── 表达式语句 ──

Expr(IrExpr),

// ── 块 ──

Block(IrBlock),

// ── 调试/诊断 ──

CompileError { span: SourceSpan, msg: String },

Comment(String),

}

pub enum IrAssignTarget {

Ident(IrIdent),

Member { object: IrExpr, field: String, is_pointer: bool },

Index { object: IrExpr, index: IrExpr },

Destructure(Vec<IrDestructureBinding>),

}

pub enum AssignOp { Assign, Add, Sub, Mul, Div, Mod, Shl, Shr, BitAnd, BitOr, BitXor, LogicAnd, LogicOr, Nullish }

## AssignOp 说明

Zig 赋值运算符与 JS 的对应关系：

| JS 运算符 | Zig 输出 | AssignOp 变体 |
| --- | --- | --- |
| = | = | Assign |
| += | += | Add |
| -= | -= | Sub |
| *= | *= | Mul |
| /= | /= | Div |
| %= | %= | Mod |
| <<= | <<= | Shl |
| &&= | and= | LogicAnd |
| ||= | or= | LogicOr |
| ??= | orelse= | Nullish |

# 3.6 表达式节点 (IrExpr)

pub enum IrExpr {

// ── 字面量 ──

IntLiteral(i64),

FloatLiteral(f64),

StringLiteral(String),

BoolLiteral(bool),

Null,

Undefined,

// ── 标识符引用 ──

Ident(IrIdent),

This,

// ── 运算 ──

Binary { op: BinOp, left: Box<IrExpr>, right: Box<IrExpr> },

Unary { op: UnaOp, operand: Box<IrExpr> },

Logical { op: LogicalOp, left: Box<IrExpr>, right: Box<IrExpr> },

Update { op: UpdateOp, target: IrAssignTarget, is_expr_stmt: bool },

Assign { op: AssignOp, target: IrAssignTarget, value: Box<IrExpr> },

// ── 调用 ──

Call(IrCallExpr),

BuiltinCall(IrBuiltinCall),   // runtime 方法：arr.push(), str.toUpperCase() 等

HostCall(IrHostCall),         // 宿主函数调用

// ── 成员访问 ──

FieldAccess { object: Box<IrExpr>, field: String, field_kind: FieldKind },

IndexAccess { object: Box<IrExpr>, index: Box<IrExpr>, index_kind: IndexKind },

ComputedField { object: Box<IrExpr>, key: Box<IrExpr>, key_kind: ComputedKeyKind },

// ── 对象/数组 ──

ArrayLiteral(IrArrayLiteral),

ObjectLiteral(IrObjectLiteral),

// ── 函数表达式 ──

ArrowFn(IrArrowFn),

Closure(IrClosure),           // 已降级为 struct + call() 模式

FnExpr(IrFnExpr),             // 函数表达式 → 命名函数引用

// ── 条件/模板 ──

Conditional { cond: Box<IrExpr>, then: Box<IrExpr>, else_: Box<IrExpr> },

TemplateLiteral { parts: Vec<String>, exprs: Vec<IrExpr> },

// ── 异步 ──

Await(IrAwaitExpr),

// ── 构造 ──

New(IrNewExpr),

// ── 块表达式 ──

BlockExpr { label: String, body: Vec<IrStmt>, result: Box<IrExpr> },

// ── 特殊 ──

Spread(Box<IrExpr>),

Typeof(Box<IrExpr>),

Void(Box<IrExpr>),

Paren(Box<IrExpr>),

Sequence(Vec<IrExpr>),

CompileError { span: SourceSpan, msg: String },

}

# 3.7 关键复合类型详解

## IrCallExpr — 函数调用

pub struct IrCallExpr {

pub callee: Box<IrExpr>,     // 被调函数

pub args: Vec<IrExpr>,       // 参数列表

pub call_kind: CallKind,     // 调用类型

}

pub enum CallKind {

Direct,                      // fn(args)

Method { object_type: MethodObjectKind },  // obj.method(args)

Closure,                     // closure_instance(.call)(args)

}

pub enum MethodObjectKind {

ArrayList,                   // .push()/.pop()/...

String,                      // .toUpperCase()/...

Map,                         // .get()/.set()/...

Set,                         // .add()/.has()/...

Date,                        // .getFullYear()/...

Class(String),               // 用户定义的类方法

JsAny,                       // 动态对象方法

Unknown,                     // 未知类型 → fallback

}

## IrBuiltinCall — 运行时内置方法

/// 预降级的 runtime 方法调用

/// 例：arr.push(x) → js_runtime.array_push(alloc, arr, x)

pub struct IrBuiltinCall {

pub module: BuiltinModule,    // js_array, js_string, js_date, ...

pub method: String,           // push, toUpperCase, getFullYear, ...

pub args: Vec<IrExpr>,        // 实参（含 alloc 插入）

pub return_type: ZigType,

}

pub enum BuiltinModule {

JsArray, JsString, JsDate, JsJson, JsObject,

JsNumber, JsSymbol, JsConsole, JsMath,

JsRegExp, JsTypedArray, JsUri, JsBigInt,

JsCollections,

}

## IrAwaitExpr — 异步等待

pub struct IrAwaitExpr {

pub task_var: IrIdent,              // _t0, _t1, ...

pub callee: Box<IrExpr>,            // 被等待的调用

pub args: Vec<IrExpr>,             // 调用参数

pub is_host_async: bool,           // 是否是 host _async wrapper

pub block_label: String,           // blk_N

}

## IrClosure — 闭包（降级后）

/// 闭包已降级为 struct + call() 方法模式

pub struct IrClosure {

pub struct_name: IrIdent,          // 闭包结构体名称

pub captured: Vec<IrCapture>,      // 捕获的变量

pub fn_param: IrParam,             // call() 方法的参数

pub return_type: ZigType,

pub body: IrBlock,                 // call() 方法体

pub instance_name: IrIdent,        // 闭包实例变量名

}

pub struct IrCapture {

pub name: IrIdent,

pub zig_type: ZigType,

pub is_mut: bool,                  // 值捕获 vs 引用捕获

}

## IrNewExpr — 构造表达式

pub struct IrNewExpr {

pub constructor: NewConstructor,   // 具体构造器类型

pub args: Vec<IrExpr>,

pub result_type: ZigType,

}

pub enum NewConstructor {

Map, Set,

Date(DateConstructorKind),

RegExp,

TypedArray(TypedArrayKind),

Class(String),                    // 自定义类

Error(String),                     // Error 构造

Unsupported(String),              // 生成 @compileError

}

pub enum DateConstructorKind {

Now,                              // new Date()

FromMillis,                       // new Date(millis)

FromString,                       // new Date(string)

FromComponents,                   // new Date(y,m,d,h,min,s,ms)

}

## FieldKind / IndexKind / ComputedKeyKind — 成员访问分类

pub enum FieldKind {

StructField,                      // obj.field

ArrayListLen,                     // arr.items.len

StringLen,                        // str.len

MapSetSize,                       // map.size()

MathConstant(String),             // std.math.pi

NumberConstant(String),           // std.math.floatMax(f64)

SymbolWellKnown(String),          // js_symbol.symbolIterator()

TypedArrayProp(String),           // .buffer/.byteLength/.byteOffset

}

pub enum IndexKind {

ArrayListItem,                    // arr.items[n]

SliceIndex,                       // arr[n]

}

pub enum ComputedKeyKind {

StructField,                      // @field(obj, key)

MapGet,                           // obj.get(key)

JsAnyGetByKey,                    // obj.getByKey(key, alloc)

ArrayListItem,                    // arr.items[key]

CompileError(String),             // 不支持的动态访问

}

# 4. AST → ZigIR 降级 (Lowering)

## 4.1 Lowerer 结构

/// AST → ZigIR 降级器

/// 替代当前 Codegen 中所有 emit_* 方法

pub struct Lowerer {

type_info: TypeCheckResult,        // 只读——类型推断结果

jsdoc_data: JSDocData,             // 只读——JSDoc 注解

host_fns: HostFnRegistry,          // 只读——宿主函数注册表

// ── 状态：名称管理 ──

name_mangler: NameMangler,         // 计数器 + shadow 重命名

// ── 状态：函数上下文 ──

fn_ctx: Option<FnContext>,         // 当前函数的作用域信息

// ── 状态：闭包管理 ──

closure_mgr: ClosureManager,       // 闭包结构体收集

// ── 输出 ──

diagnostics: Vec<IrDiagnostic>,

}

## 4.2 状态分层对比

| 当前 Codegen (30+ 字段) | Lowerer (分 3 个子结构) | 改进 |
| --- | --- | --- |
| output, indent | 移除（不再拼接字符串） | 根本性简化 |
| task_counter, arrow_counter, oc_counter, destructure_counter, for_of_counter, fn_expr_counter, label_counter, shadow_counter, try_label_counter | name_mangler: NameMangler { counters: HashMap + shadow_stack } | 9 个计数器 → 1 个结构 |
| current_fn, current_fn_is_export, seen_return, fn_has_throw, in_return_expr, in_expr_stmt | fn_ctx: Option<FnContext> | 6 个标志 → 1 个结构 |
| closure_vars, closure_instances, closure_defs, current_captured | closure_mgr: ClosureManager | 4 个集合 → 1 个管理器 |
| typedarray_vars, regexp_vars, class_names, current_class | fn_ctx 内的字段（只在函数内有效） | 4 个集合归入函数上下文 |
| errors, warnings, source | diagnostics: Vec<IrDiagnostic> | 错误信息标准化 |
| exported_functions, pending_expr_fns | 分两阶段处理（Lower 后重新排序） | 消除 def-before-use hack |

## 4.3 Lowerer 核心 API

impl Lowerer {

pub fn new(type_info: TypeCheckResult, jsdoc: JSDocData, host: HostFnRegistry) -> Self;

/// 主入口：降级一个完整的 Program

pub fn lower(&mut self, program: &Program) -> IrModule;

// ── 声明降级 ──

fn lower_fn_decl(&mut self, fd: &Function, is_export: bool) -> IrFnDecl;

fn lower_var_decl(&mut self, vd: &VariableDeclaration) -> IrVarDecl;

fn lower_class_decl(&mut self, cd: &Class) -> IrClassDecl;

// ── 语句降级 ──

fn lower_stmt(&mut self, stmt: &Statement) -> IrStmt;

fn lower_block(&mut self, body: &[Statement]) -> IrBlock;

// ── 表达式降级 ──

fn lower_expr(&mut self, expr: &Expression) -> IrExpr;

fn lower_call(&mut self, ce: &CallExpression) -> IrExpr;

fn lower_await(&mut self, ae: &AwaitExpression) -> IrAwaitExpr;

fn lower_new(&mut self, ne: &NewExpression) -> IrNewExpr;

}

## 4.4 关键降级规则

### 4.4.1 闭包降级

JS 闭包在 Lowering 阶段直接降级为 Zig struct + call() 方法：

// JS: const add = (a) => (b) => a + b;

// ZigIR:

IrClosure {

struct_name: "_closure_add_0",

captured: [IrCapture { name: "a", is_mut: false, zig_type: I64 }],

fn_param: IrParam { name: "b", zig_type: I64 },

return_type: I64,

body: IrBlock { stmts: [],

result: Binary(Add, Ident("a"), Ident("b")) },

}

### 4.4.2 For-In 降级

JS for-in 循环降级为 Zig inline for：

// JS: for (const key in obj) { ... }

// ZigIR:

IrStmt::ForIn {

var: IrIdent { js_name: "key", zig_name: "key" },

iterable: Ident("obj"),

body: IrBlock { ... },

is_struct: true,  // 决定 emit 为 inline for 还是 switch

}

### 4.4.3 Await 降级

JS await 在 Lowering 阶段生成 Io 异步模式的完整 IR：

// JS: const result = await fetchData(id);

// ZigIR:

IrExpr::Await(IrAwaitExpr {

task_var: IrIdent { js_name: "_t0", zig_name: "_t0" },

callee: HostCall { name: "fetchData" },

args: [Ident("id")],

is_host_async: true,

block_label: "blk_0",

})

### 4.4.4 Builtin 方法降级

已知的 JS 内置方法调用在 Lowering 阶段解析为 IrBuiltinCall，附带完整的参数转换：

// JS: arr.push(42)

// ZigIR:

IrExpr::BuiltinCall(IrBuiltinCall {

module: BuiltinModule::JsArray,

method: "push",

args: [Ident("arr"), IntLiteral(42)],

return_type: Void,

})

# 5. 优化 Pass

ZigIR 的结构化特性使得在 Emit 之前可以插入任意数量的变换 pass。每个 pass 接收 &IrModule，返回变换后的 IrModule（或原地修改）。

## 5.1 验证 Pass (Required)

| 验证项 | 检查逻辑 | 失败处理 |
| --- | --- | --- |
| 类型一致性 | 所有 IrExpr 的推断类型与 TypeCheckResult 一致 | 生成 IrDiagnostic::Error |
| 名称唯一性 | 同一作用域内无重复标识符 | 生成编译错误 |
| 闭包完整性 | 每个 IrClosure 的 captured 列表与实际引用匹配 | 生成警告 |
| C ABI 兼容性 | 所有 cabi_export 函数参数/返回值都是 C ABI 安全类型 | 降级为 const 别名（不导出） |
| Def-before-use | 所有标识符引用都有对应的声明 | 生成编译错误 |

## 5.2 优化 Pass (Optional)

| Pass | 优化内容 | 预期收益 |
| --- | --- | --- |
| DeadCodeElim | 删除未被引用的顶层声明和不可达代码 | 减小生成代码体积 |
| UnusedVarStrip | 删除 is_const && !used 的变量声明 | 消除 Zig unused 警告 |
| BuiltinDedup | 合并相同的 BuiltinCall 模式 | 减少重复 runtime 调用 |
| ConstantFold | 常量表达式折叠：1 + 2 → 3 | 提升运行时性能 |
| SourceMapGen | 为每个 IrStmt/IrExpr 附加 SourceSpan | 精确 source map |

## 5.3 Pass 执行框架

/// 优化 pass trait

pub trait IrPass {

fn name(&self) -> &str;

fn run(&self, module: &mut IrModule) -> PassResult;

}

pub struct PassResult {

pub modified: bool,          // 是否修改了 IR

pub diagnostics: Vec<IrDiagnostic>,

}

/// Pass 管道

pub struct PassPipeline {

passes: Vec<Box<dyn IrPass>>,

}

impl PassPipeline {

pub fn default() -> Self {

Self { passes: vec![

Box::new(ValidatePass::new()),

Box::new(DeadCodeElimPass::new()),

Box::new(ConstantFoldPass::new()),

Box::new(SourceMapGenPass::new()),

]}

}

pub fn run(&self, module: &mut IrModule) { ... }

}

# 6. ZigIR → String 发射 (Emit)

## 6.1 Emitter 结构

/// ZigIR → Zig 源码发射器

/// 纯函数式——只读取 IrModule，只写入 String

pub struct Emitter {

output: String,

indent: usize,

}

impl Emitter {

pub fn emit(module: &IrModule) -> String {

let mut e = Emitter { output: String::new(), indent: 0 };

e.emit_module(module);

e.output

}

fn emit_module(&mut self, m: &IrModule) {

// 1. imports

for imp in &m.imports { self.emit_import(imp); }

// 2. typedefs

for td in &m.typedefs { self.emit_typedef(td); }

// 3. closure structs (前置声明)

for cs in &m.closure_structs { self.emit_closure_struct(cs); }

// 4. declarations

for decl in &m.declarations { self.emit_decl(decl); }

}

fn emit_decl(&mut self, decl: &IrDecl) { ... }

fn emit_fn(&mut self, f: &IrFnDecl) { ... }

fn emit_var(&mut self, v: &IrVarDecl) { ... }

fn emit_class(&mut self, c: &IrClassDecl) { ... }

fn emit_stmt(&mut self, s: &IrStmt) { ... }

fn emit_expr(&mut self, e: &IrExpr) { ... }

}

## 6.2 Emit 算法：模式匹配 + 格式化

Emitter 对每个 IR 节点做简单的模式匹配和格式化，不含任何推断逻辑——所有决策在 Lowering 阶段已完成。

// 示例：IrExpr::FieldAccess 的 emit

fn emit_expr(&mut self, e: &IrExpr) {

match e {

IrExpr::FieldAccess { object, field, field_kind } => {

match field_kind {

FieldKind::ArrayListLen => {

self.emit_expr(object);

self.write(".items.len");

}

FieldKind::MapSetSize => {

self.emit_expr(object);

self.write(".size()");

}

FieldKind::MathConstant(c) => {

self.write(&format!("std.math.{}", c));

}

FieldKind::StructField => {

self.emit_expr(object);

self.write(&format!(".{}", field));

}

// ...

}

}

// ...

}

}

关键原则：Emitter 是一个纯格式化器，它不做类型推断、不做名称解析、不做控制流分析。所有语义决策在 Lowering 阶段已经完成，Emitter 只需忠实地将 IR 结构映射为 Zig 语法。

# 7. 迁移策略

## 7.1 渐进式迁移（3 个阶段）

| 阶段 | 内容 | 验证方式 | 预计周期 |
| --- | --- | --- | --- |
| Phase 0：基础设施 | 定义 ZigIR 类型（IrModule, IrDecl, IrStmt, IrExpr 及所有辅助类型），实现 IrIdent、SourceSpan 等基础类型 | 编译通过 + 单元测试 | 1-2 周 |
| Phase 1：Lowerer 开发 | 实现 Lowerer，逐个迁移 emit_* 方法为 lower_* 方法。每个方法迁移后立即写 IR 对比测试 | IR 快照测试 + 集成测试 | 3-5 周 |
| Phase 2：Emitter 开发 | 实现 Emitter，对每个 IR 节点实现 emit 逻辑。逐节点替换 Codegen 的直接输出 | 字符串对比测试（新旧输出一致） | 2-3 周 |
| Phase 3：优化 Pass | 实现 ValidatePass + DeadCodeElimPass + SourceMapGenPass，接入 pipeline | 回归测试 + source map 验证 | 1-2 周 |

## 7.2 双轨并行策略

在迁移期间，旧 Codegen 和新 Lowerer+Emitter 并行运行：

// pipeline.rs 中的双轨比较逻辑

let old_zig_code = Codegen::generate(program, type_info.clone(), jsdoc, ...);

let ir_module = Lowerer::new(type_info, jsdoc, host_fns).lower(program);

let new_zig_code = Emitter::emit(&ir_module);

if old_zig_code != new_zig_code {

// 记录差异，但不阻断构建

eprintln!("WARN: ZigIR output differs for {}", filename);

// 开发阶段：使用旧输出（保证正确性）

// 发布阶段：使用新输出

}

每个迁移的 emit_* 方法都有对应的 lower_* 方法和 emit 方法。当所有方法迁移完成后，移除 Codegen 路径。

## 7.3 测试策略

IR 快照测试：Lowerer 输出的 IrModule 序列化为 JSON，与 golden file 对比

字符串等价测试：Emitter 输出与旧 Codegen 输出字符串完全一致

集成测试：端到端测试——JS → ZigIR → Zig → 编译 → 运行

属性测试：随机生成 JsExpr，验证 Lowerer 不 panic 且生成的 IR 可被 Emitter 正确输出

# 8. 目录结构

js2zig-core/src/

zigir/

mod.rs                // pub mod 声明 + ZigIR 公开 API

types.rs              // IrModule, IrDecl, IrStmt, IrExpr, 所有辅助类型

ident.rs              // IrIdent, NameMangler

source_span.rs        // SourceSpan, IrDiagnostic

lower/

mod.rs              // Lowerer 主结构 + lower() 入口

expr.rs             // 表达式降级

stmt.rs             // 语句降级

helpers.rs          // 降级辅助方法

passes/

mod.rs              // IrPass trait + PassPipeline

validate.rs         // 验证 pass

dead_code.rs        // 死代码消除

const_fold.rs       // 常量折叠

source_map.rs       // 源码映射生成

emit/

mod.rs              // Emitter 主结构 + emit() 入口

expr.rs             // 表达式发射

stmt.rs             // 语句发射

helpers.rs          // 格式化辅助

总文件数：14 个 .rs 文件，与现有 codegen/ 结构对应，降低认知成本。

# 9. 收益总结

| 收益维度 | 当前状态 | ZigIR 后 | 量化指标 |
| --- | --- | --- | --- |
| 结构验证 | 无（字符串拼接），Zig 编译时才发现错误 | IR 层 ValidatePass 可在生成阶段捕获 | 编译错误反馈时间从秒级降至毫秒级 |
| 状态复杂度 | Codegen 30+ 可变字段 | Lowerer 3 个子结构 + Emitter 2 个字段 | 可变状态减少 80%+ |
| 测试粒度 | 只能端到端字符串对比 | IR 快照测试 + 单节点单元测试 + 属性测试 | 测试覆盖率提升 50%+ |
| 代码优化 | 无法优化 | DeadCodeElim + ConstantFold + BuiltinDedup | 生成代码体积减少 10-20% |
| Source Map | 粗粒度（per-file） | 精粒度（per-statement/expression） | 调试定位精度从文件级提升到行/列级 |
| 多后端潜力 | 仅 Zig | Emit 可替换（ZigEmitter, CEmitter, WasmEmitter） | 架构可扩展性 |
| 可维护性 | emit_expr 6640 行单文件 | lower/expr + emit/expr 各 ~1500 行 | 单文件行数减少 75% |

# 10. 风险与缓解

| 风险 | 影响 | 缓解措施 |
| --- | --- | --- |
| 迁移过程中引入功能回归 | 高 | 双轨并行 + 字符串等价测试 + 346 个现有测试全部通过 |
| IR 节点设计过于细粒度，Lowering 复杂度爆炸 | 中 | 先行验证最复杂的 5 个场景（闭包、类、await、builtin、try-catch） |
| 性能回退（IR 中间层增加开销） | 低 | 降低阶段（lower）本身不需要字符串分配，整体应更快；Emitter 的输出可缓存 |
| IR 序列化/反序列化复杂度 | 低 | IrModule 不需要跨进程传输；JSON 快照仅用于测试，非生产路径 |
| 与现有 pipeline.rs 耦合 | 中 | Lowerer 输出 IrModule，pipeline 只需替换 Codegen::generate→Lowerer::lower+Emitter::emit |

总结：ZigIR 方案通过引入结构化中间表示，将当前单一 "AST → 字符串" 的巨型 Codegen 分解为 Lower（降级）+ Pass（优化）+ Emit（发射）三个独立阶段。核心收益是：状态复杂度降低 80%+，可测试性从黑盒升级为白盒，支持生成时代码验证和优化，并为多后端扩展奠定基础。迁移采用双轨并行策略，确保每一步都可验证、可回退。

AI生成

AIGC标识: 64c02d21-5e67-450f-bff1-2ef47c66e95a
---
[DEGRADED MODE] Output generated using python-docx fallback. Missing dependency: pandoc.
Install pandoc for complete tracked-change rendering.