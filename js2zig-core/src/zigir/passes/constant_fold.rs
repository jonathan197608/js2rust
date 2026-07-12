// zigir/passes/constant_fold.rs
// ConstantFoldPass — fold constant arithmetic and string expressions.
//
// Supported folds:
//   - Integer arithmetic: 1 + 2 → 3
//   - Float arithmetic: 1.5 + 2.5 → 4.0
//   - Boolean logic on constants: true && false → false
//   - String concatenation of literals: "a" ++ "b" → "ab"
//   - Unary on constants: -42 → -42, !true → false
//   - typeof on literals: typeof 42 → "number"

use crate::zigir::ops::{BinOp, LogicalOp, UnaOp};
use crate::zigir::passes::{IrPass, PassResult};
use crate::zigir::types::{IrAssignTarget, IrBlock, IrExpr, IrModule, IrStmt};

use std::cell::RefCell;

use super::walk;

/// Constant folding pass.
///
/// Replaces constant expressions with their computed values.
/// Does NOT fold expressions that might have side effects.
pub struct ConstantFoldPass;

impl ConstantFoldPass {
    pub fn new() -> Self {
        Self
    }

    /// Try to fold an expression into a simpler constant form.
    /// Returns true if any change was made.
    ///
    /// Note: this uses selective traversal (not walk.rs) because folding inside
    /// Call/BuiltinCall args can change how the emitter handles string content,
    /// causing regressions (e.g. embedded quotes in console.log args).
    fn try_fold(expr: &mut IrExpr) -> bool {
        match expr {
            IrExpr::Binary {
                op, left, right, ..
            } => {
                // First, recursively fold children
                let mut changed = false;
                if Self::try_fold(left) {
                    changed = true;
                }
                if Self::try_fold(right) {
                    changed = true;
                }
                // Then try to fold this binary operation
                if let Some(result) = fold_binary(*op, left, right) {
                    *expr = result;
                    true
                } else {
                    changed
                }
            }
            IrExpr::Unary { op, operand } => {
                let changed = Self::try_fold(operand);
                if let Some(result) = fold_unary(*op, operand) {
                    *expr = result;
                    true
                } else {
                    changed
                }
            }
            IrExpr::Logical {
                op, left, right, ..
            } => {
                let mut changed = false;
                if Self::try_fold(left) {
                    changed = true;
                }
                if Self::try_fold(right) {
                    changed = true;
                }
                if let Some(result) = fold_logical(*op, left, right) {
                    *expr = result;
                    true
                } else {
                    changed
                }
            }
            IrExpr::Conditional { cond, then, else_ } => {
                let mut changed = false;
                if Self::try_fold(cond) {
                    changed = true;
                }
                // Check condition before mutating then/else_
                let cond_bool = if let IrExpr::BoolLiteral(b) = cond.as_ref() {
                    Some(*b)
                } else {
                    None
                };
                if Self::try_fold(then) {
                    changed = true;
                }
                if Self::try_fold(else_) {
                    changed = true;
                }
                // If condition is a known boolean, eliminate the branch
                if let Some(b) = cond_bool {
                    let replacement = if b {
                        (**then).clone()
                    } else {
                        (**else_).clone()
                    };
                    *expr = replacement;
                    return true;
                }
                changed
            }
            IrExpr::Paren(inner) => {
                let mut changed = Self::try_fold(inner);
                // Unwrap paren around a literal
                if matches!(
                    inner.as_ref(),
                    IrExpr::IntLiteral(_)
                        | IrExpr::FloatLiteral(_)
                        | IrExpr::BoolLiteral(_)
                        | IrExpr::BigIntLiteral(_)
                ) {
                    *expr = (**inner).clone();
                    changed = true;
                }
                changed
            }
            IrExpr::Sequence(exprs) => Self::try_fold_iter(exprs),
            IrExpr::AllocPrint { fmt, args } => {
                let changed = Self::try_fold_iter(args);
                if args.is_empty() {
                    let s = fmt.clone();
                    *expr = IrExpr::StringLiteral(s);
                    return true;
                }
                changed
            }
            IrExpr::Typeof(inner) => {
                // Check typeof before mutating, since we might replace the expr
                let typeof_result = typeof_literal(inner);
                let changed = Self::try_fold(inner);
                if let Some(s) = typeof_result {
                    *expr = IrExpr::StringLiteral(s);
                    return true;
                }
                changed
            }
            // Recurse into compound expressions
            IrExpr::ArrayLiteral(arr) => Self::try_fold_iter(&mut arr.elements),
            IrExpr::ObjectLiteral(obj) => Self::try_fold_object_items(&mut obj.items),
            IrExpr::TemplateLiteral { exprs, .. } => Self::try_fold_iter(exprs),
            IrExpr::Spread(e) | IrExpr::Void(e) => Self::try_fold(e),
            _ => false,
        }
    }

    fn try_fold_iter(exprs: &mut [IrExpr]) -> bool {
        let mut changed = false;
        for e in exprs {
            if Self::try_fold(e) {
                changed = true;
            }
        }
        changed
    }

    fn try_fold_object_items(items: &mut [crate::zigir::types::IrObjectItem]) -> bool {
        use crate::zigir::types::IrObjectItem;
        let mut changed = false;
        for item in items {
            let expr = match item {
                IrObjectItem::Field(f) => &mut f.value,
                IrObjectItem::Spread(e) => e,
            };
            if Self::try_fold(expr) {
                changed = true;
            }
        }
        changed
    }

    /// Fold constants in a statement.
    fn fold_stmt(stmt: &mut IrStmt) -> bool {
        let changed = RefCell::new(false);
        walk::for_each_stmt_child_mut(
            stmt,
            &mut |block| {
                *changed.borrow_mut() |= Self::fold_block(block);
            },
            &mut |s| {
                *changed.borrow_mut() |= Self::fold_stmt(s);
            },
            &mut |e| {
                *changed.borrow_mut() |= Self::try_fold(e);
            },
            &mut |t| {
                *changed.borrow_mut() |= Self::fold_target(t);
            },
        );
        changed.into_inner()
    }

    /// Fold constants inside an assign target's sub-expressions.
    fn fold_target(target: &mut IrAssignTarget) -> bool {
        let mut changed = false;
        walk::for_each_target_child_mut(target, &mut |e| {
            changed |= Self::try_fold(e);
        });
        changed
    }

    fn fold_block(block: &mut IrBlock) -> bool {
        let mut changed = false;
        for stmt in &mut block.stmts {
            if Self::fold_stmt(stmt) {
                changed = true;
            }
        }
        changed
    }
}

impl IrPass for ConstantFoldPass {
    fn name(&self) -> &'static str {
        "constant-fold"
    }

    fn description(&self) -> &'static str {
        "Folds constant arithmetic, string, and boolean expressions"
    }

    fn run(&mut self, module: &mut IrModule) -> PassResult {
        let mut changed = false;

        // Fold in all declarations
        for decl in &mut module.declarations {
            let ch = RefCell::new(false);
            walk::for_each_decl_child_mut(
                decl,
                &mut |block| {
                    *ch.borrow_mut() |= Self::fold_block(block);
                },
                &mut |e| {
                    *ch.borrow_mut() |= Self::try_fold(e);
                },
            );
            if ch.into_inner() {
                changed = true;
            }
        }

        // Fold in closure structs
        for cs in &mut module.closure_structs {
            if Self::fold_block(&mut cs.body) {
                changed = true;
            }
        }

        if changed {
            PassResult::changed()
        } else {
            PassResult::unchanged()
        }
    }
}

impl Default for ConstantFoldPass {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════
//  Fold helpers
// ═══════════════════════════════════════════════════════

fn fold_binary(op: BinOp, left: &IrExpr, right: &IrExpr) -> Option<IrExpr> {
    match (left, right) {
        // Integer arithmetic
        (IrExpr::IntLiteral(a), IrExpr::IntLiteral(b)) => {
            let result = match op {
                BinOp::Add => a.checked_add(*b)?,
                BinOp::Sub => a.checked_sub(*b)?,
                BinOp::Mul => a.checked_mul(*b)?,
                BinOp::Div => {
                    if *b == 0 {
                        return None;
                    }
                    a.checked_div(*b)?
                }
                BinOp::Mod => {
                    if *b == 0 {
                        return None;
                    }
                    *a % *b
                }
                BinOp::BitAnd => a & b,
                BinOp::BitOr => a | b,
                BinOp::BitXor => a ^ b,
                BinOp::Shl => {
                    if *b < 0 || *b >= 64 {
                        return None;
                    }
                    a << *b
                }
                BinOp::Shr => {
                    if *b < 0 || *b >= 64 {
                        return None;
                    }
                    a >> *b
                }
                BinOp::Eq | BinOp::StrictEq => return Some(IrExpr::BoolLiteral(a == b)),
                BinOp::Ne | BinOp::StrictNe => return Some(IrExpr::BoolLiteral(a != b)),
                BinOp::Lt => return Some(IrExpr::BoolLiteral(a < b)),
                BinOp::Le => return Some(IrExpr::BoolLiteral(a <= b)),
                BinOp::Gt => return Some(IrExpr::BoolLiteral(a > b)),
                BinOp::Ge => return Some(IrExpr::BoolLiteral(a >= b)),
                _ => return None,
            };
            Some(IrExpr::IntLiteral(result))
        }
        // Float arithmetic
        (IrExpr::FloatLiteral(a), IrExpr::FloatLiteral(b)) => {
            let result = match op {
                BinOp::Add => a + b,
                BinOp::Sub => a - b,
                BinOp::Mul => a * b,
                BinOp::Div => {
                    if *b == 0.0 {
                        return None;
                    }
                    a / b
                }
                BinOp::Mod => a % b,
                BinOp::Eq | BinOp::StrictEq => return Some(IrExpr::BoolLiteral(a == b)),
                BinOp::Ne | BinOp::StrictNe => return Some(IrExpr::BoolLiteral(a != b)),
                BinOp::Lt => return Some(IrExpr::BoolLiteral(a < b)),
                BinOp::Le => return Some(IrExpr::BoolLiteral(a <= b)),
                BinOp::Gt => return Some(IrExpr::BoolLiteral(a > b)),
                BinOp::Ge => return Some(IrExpr::BoolLiteral(a >= b)),
                _ => return None,
            };
            Some(IrExpr::FloatLiteral(result))
        }
        // Mixed int + float → promote to float
        (IrExpr::IntLiteral(a), IrExpr::FloatLiteral(b)) => {
            let a_f = *a as f64;
            let result = match op {
                BinOp::Add => a_f + b,
                BinOp::Sub => a_f - b,
                BinOp::Mul => a_f * b,
                BinOp::Div => a_f / b,
                _ => return None,
            };
            Some(IrExpr::FloatLiteral(result))
        }
        (IrExpr::FloatLiteral(a), IrExpr::IntLiteral(b)) => {
            let b_f = *b as f64;
            let result = match op {
                BinOp::Add => a + b_f,
                BinOp::Sub => a - b_f,
                BinOp::Mul => a * b_f,
                BinOp::Div => a / b_f,
                _ => return None,
            };
            Some(IrExpr::FloatLiteral(result))
        }
        // String concatenation
        (IrExpr::StringLiteral(a), IrExpr::StringLiteral(b)) => {
            if op == BinOp::Add {
                Some(IrExpr::StringLiteral(format!("{}{}", a, b)))
            } else {
                None
            }
        }
        // Boolean comparisons
        (IrExpr::BoolLiteral(a), IrExpr::BoolLiteral(b)) => {
            let result = match op {
                BinOp::Eq | BinOp::StrictEq => a == b,
                BinOp::Ne | BinOp::StrictNe => a != b,
                _ => return None,
            };
            Some(IrExpr::BoolLiteral(result))
        }
        // Null comparisons
        (IrExpr::Null, IrExpr::Null) => {
            if op == BinOp::Eq || op == BinOp::StrictEq {
                Some(IrExpr::BoolLiteral(true))
            } else if op == BinOp::Ne || op == BinOp::StrictNe {
                Some(IrExpr::BoolLiteral(false))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn fold_unary(op: UnaOp, operand: &IrExpr) -> Option<IrExpr> {
    match (op, operand) {
        (UnaOp::Neg, IrExpr::IntLiteral(n)) => Some(IrExpr::IntLiteral(-n)),
        (UnaOp::Neg, IrExpr::FloatLiteral(n)) => Some(IrExpr::FloatLiteral(-n)),
        (UnaOp::Not, IrExpr::BoolLiteral(b)) => Some(IrExpr::BoolLiteral(!b)),
        (UnaOp::BitNot, IrExpr::IntLiteral(n)) => Some(IrExpr::IntLiteral(!n)),
        // Double negation: !!x → x (when inner is already a bool)
        (
            UnaOp::Not,
            IrExpr::Unary {
                op: UnaOp::Not,
                operand: inner,
            },
        ) => {
            if matches!(inner.as_ref(), IrExpr::BoolLiteral(_)) {
                // We can't take ownership from &, so clone
                Some((**inner).clone())
            } else {
                None
            }
        }
        _ => None,
    }
}

fn fold_logical(op: LogicalOp, left: &IrExpr, right: &IrExpr) -> Option<IrExpr> {
    match (op, left, right) {
        // Short-circuit on known booleans
        (LogicalOp::And, IrExpr::BoolLiteral(false), _) => Some(IrExpr::BoolLiteral(false)),
        (LogicalOp::And, IrExpr::BoolLiteral(true), _) => {
            // true && right → right
            Some(right.clone())
        }
        (LogicalOp::Or, IrExpr::BoolLiteral(true), _) => Some(IrExpr::BoolLiteral(true)),
        (LogicalOp::Or, IrExpr::BoolLiteral(false), _) => {
            // false || right → right
            Some(right.clone())
        }
        (LogicalOp::Nullish, IrExpr::Null, right) => Some(right.clone()),
        (LogicalOp::Nullish, IrExpr::Undefined, right) => Some(right.clone()),
        _ => None,
    }
}

/// Compute the `typeof` result string for literal expressions.
fn typeof_literal(expr: &IrExpr) -> Option<String> {
    match expr {
        IrExpr::IntLiteral(_) | IrExpr::FloatLiteral(_) => Some("number".to_string()),
        IrExpr::StringLiteral(_) => Some("string".to_string()),
        IrExpr::BoolLiteral(_) => Some("boolean".to_string()),
        IrExpr::Null => Some("object".to_string()), // JS typeof null === "object"
        IrExpr::Undefined => Some("undefined".to_string()),
        IrExpr::ArrowFn(_) | IrExpr::Closure(_) | IrExpr::FnExpr(_) => Some("function".to_string()),
        _ => None,
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
    use crate::zigir::types::{IrBlock, IrDecl, IrFnDecl, IrStmt};

    fn first_return_value(module: &IrModule) -> &IrExpr {
        let IrDecl::Fn(f) = &module.declarations[0] else {
            panic!("expected FnDecl at declarations[0]");
        };
        let IrStmt::Return { value: Some(expr) } = &f.body.stmts[0] else {
            panic!("expected Return with value at stmts[0]");
        };
        expr
    }

    #[test]
    fn test_fold_int_add() {
        let mut module = make_module_with_body(vec![IrStmt::Return {
            value: Some(IrExpr::Binary {
                op: BinOp::Add,
                left: Box::new(IrExpr::IntLiteral(1)),
                right: Box::new(IrExpr::IntLiteral(2)),
                left_type: None,
                right_type: None,
            }),
        }]);
        let mut pass = ConstantFoldPass::new();
        let result = pass.run(&mut module);
        assert!(result.changed);
        match first_return_value(&module) {
            IrExpr::IntLiteral(n) => assert_eq!(*n, 3),
            other => panic!("expected IntLiteral(3), got {:?}", other),
        }
    }

    #[test]
    fn test_fold_float_mul() {
        let mut module = make_module_with_body(vec![IrStmt::Return {
            value: Some(IrExpr::Binary {
                op: BinOp::Mul,
                left: Box::new(IrExpr::FloatLiteral(1.5)),
                right: Box::new(IrExpr::FloatLiteral(2.0)),
                left_type: None,
                right_type: None,
            }),
        }]);
        let mut pass = ConstantFoldPass::new();
        let result = pass.run(&mut module);
        assert!(result.changed);
        match first_return_value(&module) {
            IrExpr::FloatLiteral(n) => assert_eq!(*n, 3.0),
            other => panic!("expected FloatLiteral(3.0), got {:?}", other),
        }
    }

    #[test]
    fn test_fold_string_concat() {
        let mut module = make_module_with_body(vec![IrStmt::Return {
            value: Some(IrExpr::Binary {
                op: BinOp::Add,
                left: Box::new(IrExpr::StringLiteral("Hello, ".to_string())),
                right: Box::new(IrExpr::StringLiteral("world!".to_string())),
                left_type: None,
                right_type: None,
            }),
        }]);
        let mut pass = ConstantFoldPass::new();
        let result = pass.run(&mut module);
        assert!(result.changed);
        match first_return_value(&module) {
            IrExpr::StringLiteral(s) => assert_eq!(s, "Hello, world!"),
            other => panic!("expected StringLiteral, got {:?}", other),
        }
    }

    #[test]
    fn test_fold_unary_negate() {
        let mut module = make_module_with_body(vec![IrStmt::Return {
            value: Some(IrExpr::Unary {
                op: UnaOp::Neg,
                operand: Box::new(IrExpr::IntLiteral(42)),
            }),
        }]);
        let mut pass = ConstantFoldPass::new();
        let result = pass.run(&mut module);
        assert!(result.changed);
        match first_return_value(&module) {
            IrExpr::IntLiteral(n) => assert_eq!(*n, -42),
            other => panic!("expected IntLiteral(-42), got {:?}", other),
        }
    }

    #[test]
    fn test_fold_not_bool() {
        let mut module = make_module_with_body(vec![IrStmt::Return {
            value: Some(IrExpr::Unary {
                op: UnaOp::Not,
                operand: Box::new(IrExpr::BoolLiteral(true)),
            }),
        }]);
        let mut pass = ConstantFoldPass::new();
        let result = pass.run(&mut module);
        assert!(result.changed);
        match first_return_value(&module) {
            IrExpr::BoolLiteral(b) => assert!(!b),
            other => panic!("expected BoolLiteral(false), got {:?}", other),
        }
    }

    #[test]
    fn test_fold_conditional_true() {
        let mut module = make_module_with_body(vec![IrStmt::Return {
            value: Some(IrExpr::Conditional {
                cond: Box::new(IrExpr::BoolLiteral(true)),
                then: Box::new(IrExpr::IntLiteral(1)),
                else_: Box::new(IrExpr::IntLiteral(2)),
            }),
        }]);
        let mut pass = ConstantFoldPass::new();
        let result = pass.run(&mut module);
        assert!(result.changed);
        match first_return_value(&module) {
            IrExpr::IntLiteral(n) => assert_eq!(*n, 1),
            other => panic!("expected IntLiteral(1), got {:?}", other),
        }
    }

    #[test]
    fn test_fold_and_false() {
        let mut module = make_module_with_body(vec![IrStmt::Return {
            value: Some(IrExpr::Logical {
                op: LogicalOp::And,
                left: Box::new(IrExpr::BoolLiteral(false)),
                right: Box::new(IrExpr::Ident(IrIdent::new("x"))),
                left_type: Some(crate::types::ZigType::Bool),
                right_type: None,
            }),
        }]);
        let mut pass = ConstantFoldPass::new();
        let result = pass.run(&mut module);
        assert!(result.changed);
        match first_return_value(&module) {
            IrExpr::BoolLiteral(b) => assert!(!b),
            other => panic!("expected BoolLiteral(false), got {:?}", other),
        }
    }

    #[test]
    fn test_fold_typeof_int() {
        let mut module = make_module_with_body(vec![IrStmt::Return {
            value: Some(IrExpr::Typeof(Box::new(IrExpr::IntLiteral(42)))),
        }]);
        let mut pass = ConstantFoldPass::new();
        let result = pass.run(&mut module);
        assert!(result.changed);
        match first_return_value(&module) {
            IrExpr::StringLiteral(s) => assert_eq!(s, "number"),
            other => panic!("expected StringLiteral(\"number\"), got {:?}", other),
        }
    }

    #[test]
    fn test_no_fold_dynamic() {
        let mut module = make_module_with_body(vec![IrStmt::Return {
            value: Some(IrExpr::Binary {
                op: BinOp::Add,
                left: Box::new(IrExpr::Ident(IrIdent::new("x"))),
                right: Box::new(IrExpr::IntLiteral(1)),
                left_type: None,
                right_type: None,
            }),
        }]);
        let mut pass = ConstantFoldPass::new();
        let result = pass.run(&mut module);
        assert!(!result.changed); // can't fold: x is dynamic
    }

    #[test]
    fn test_fold_nested() {
        // (1 + 2) + (3 + 4) → 10
        let mut module = make_module_with_body(vec![IrStmt::Return {
            value: Some(IrExpr::Binary {
                op: BinOp::Add,
                left: Box::new(IrExpr::Binary {
                    op: BinOp::Add,
                    left: Box::new(IrExpr::IntLiteral(1)),
                    right: Box::new(IrExpr::IntLiteral(2)),
                    left_type: None,
                    right_type: None,
                }),
                right: Box::new(IrExpr::Binary {
                    op: BinOp::Add,
                    left: Box::new(IrExpr::IntLiteral(3)),
                    right: Box::new(IrExpr::IntLiteral(4)),
                    left_type: None,
                    right_type: None,
                }),
                left_type: None,
                right_type: None,
            }),
        }]);
        let mut pass = ConstantFoldPass::new();
        let result = pass.run(&mut module);
        assert!(result.changed);
        match first_return_value(&module) {
            IrExpr::IntLiteral(n) => assert_eq!(*n, 10),
            other => panic!("expected IntLiteral(10), got {:?}", other),
        }
    }

    #[test]
    fn test_fold_allocprint_no_args() {
        let mut module = make_module_with_body(vec![IrStmt::Return {
            value: Some(IrExpr::AllocPrint {
                fmt: "hello".to_string(),
                args: vec![],
            }),
        }]);
        let mut pass = ConstantFoldPass::new();
        let result = pass.run(&mut module);
        assert!(result.changed);
        match first_return_value(&module) {
            IrExpr::StringLiteral(s) => assert_eq!(s, "hello"),
            other => panic!("expected StringLiteral(\"hello\"), got {:?}", other),
        }
    }

    #[test]
    fn test_fold_null_equality() {
        let mut module = make_module_with_body(vec![IrStmt::Return {
            value: Some(IrExpr::Binary {
                op: BinOp::Eq,
                left: Box::new(IrExpr::Null),
                right: Box::new(IrExpr::Null),
                left_type: None,
                right_type: None,
            }),
        }]);
        let mut pass = ConstantFoldPass::new();
        let result = pass.run(&mut module);
        assert!(result.changed);
        match first_return_value(&module) {
            IrExpr::BoolLiteral(b) => assert!(b),
            other => panic!("expected BoolLiteral(true), got {:?}", other),
        }
    }

    // Helper to create a module with a function wrapping the given body
    fn make_module_with_body(body: Vec<IrStmt>) -> IrModule {
        let mut module = IrModule::new("test".to_string());
        module.declarations.push(IrDecl::Fn(IrFnDecl {
            name: IrIdent::new("test_fn"),
            params: vec![],
            return_type: ZigType::I64,
            body: IrBlock::new(body),
            is_export: false,
            is_async: false,
            can_throw: false,
            is_cabi: false,
            typeof_return_body: None,
        }));
        module
    }
}
