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
            IrExpr::Unary { op, operand, .. } => {
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
                // P2-CF-1: Use literal_truthiness instead of just BoolLiteral,
                // so that 0 ? a : b → b, "" ? a : b → b, null ? a : b → b, etc.
                let cond_bool = literal_truthiness(cond.as_ref());
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
            // Recurse into assignment RHS so `x = 1 + 2` can be folded
            IrExpr::Assign { value, .. } => Self::try_fold(value),
            // PowExpr / RemExpr / DivExpr always carry sub-expressions on both
            // sides — fold each so that constant sub-trees (e.g. `(1+2) % x`,
            // `Math.pow(2**3, n)`) still get simplified, even when the node
            // itself cannot be reduced to a literal.
            IrExpr::PowExpr { base, exp, .. } => {
                let mut changed = Self::try_fold(base);
                if Self::try_fold(exp) {
                    changed = true;
                }
                changed
            }
            IrExpr::RemExpr { left, right, .. } | IrExpr::DivExpr { left, right, .. } => {
                let mut changed = Self::try_fold(left);
                if Self::try_fold(right) {
                    changed = true;
                }
                changed
            }
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
                    // JS `/` always returns float (e.g., 5/2 === 2.5).
                    // Don't fold to integer result — let DivExpr handle it.
                    return None;
                }
                BinOp::Mod => {
                    if *b == 0 {
                        return None;
                    }
                    return Some(IrExpr::IntLiteral(a % b));
                }
                // JS bitwise ops operate on Int32 (32-bit signed), not i64.
                // `*a as i32` truncates and sign-extends exactly like
                // ToInt32, then we sign-extend the i32 result back to i64.
                BinOp::BitAnd => (*a as i32 & *b as i32) as i64,
                BinOp::BitOr => (*a as i32 | *b as i32) as i64,
                BinOp::BitXor => (*a as i32 ^ *b as i32) as i64,
                // JS shifts mask the count to 5 bits (& 0x1F), so any count
                // is valid (no guard needed). wrapping_shl/wrapping_shr apply
                // the mask and avoid debug-mode shift-overflow panics.
                // Shl/Shr are Int32 (sign-propagating for >>).
                BinOp::Shl => (*a as i32).wrapping_shl(*b as u32) as i64,
                BinOp::Shr => (*a as i32).wrapping_shr(*b as u32) as i64,
                // JS >>> converts to UInt32, shifts right (zero-fill), result is
                // always 0..2^32-1. wrapping_shr on u32 handles the 5-bit mask.
                BinOp::UrShr => (*a as u32).wrapping_shr(*b as u32) as i64,
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
            let b_val = *b;
            match op {
                BinOp::Add => Some(IrExpr::FloatLiteral(a_f + b_val)),
                BinOp::Sub => Some(IrExpr::FloatLiteral(a_f - b_val)),
                BinOp::Mul => Some(IrExpr::FloatLiteral(a_f * b_val)),
                BinOp::Div => {
                    if b_val == 0.0 {
                        return None;
                    }
                    Some(IrExpr::FloatLiteral(a_f / b_val))
                }
                BinOp::Mod => Some(IrExpr::FloatLiteral(a_f % b_val)),
                BinOp::Eq | BinOp::StrictEq => Some(IrExpr::BoolLiteral(a_f == b_val)),
                BinOp::Ne | BinOp::StrictNe => Some(IrExpr::BoolLiteral(a_f != b_val)),
                BinOp::Lt => Some(IrExpr::BoolLiteral(a_f < b_val)),
                BinOp::Le => Some(IrExpr::BoolLiteral(a_f <= b_val)),
                BinOp::Gt => Some(IrExpr::BoolLiteral(a_f > b_val)),
                BinOp::Ge => Some(IrExpr::BoolLiteral(a_f >= b_val)),
                _ => None,
            }
        }
        (IrExpr::FloatLiteral(a), IrExpr::IntLiteral(b)) => {
            let a_val = *a;
            let b_f = *b as f64;
            match op {
                BinOp::Add => Some(IrExpr::FloatLiteral(a_val + b_f)),
                BinOp::Sub => Some(IrExpr::FloatLiteral(a_val - b_f)),
                BinOp::Mul => Some(IrExpr::FloatLiteral(a_val * b_f)),
                BinOp::Div => {
                    if b_f == 0.0 {
                        return None;
                    }
                    Some(IrExpr::FloatLiteral(a_val / b_f))
                }
                BinOp::Mod => Some(IrExpr::FloatLiteral(a_val % b_f)),
                BinOp::Eq | BinOp::StrictEq => Some(IrExpr::BoolLiteral(a_val == b_f)),
                BinOp::Ne | BinOp::StrictNe => Some(IrExpr::BoolLiteral(a_val != b_f)),
                BinOp::Lt => Some(IrExpr::BoolLiteral(a_val < b_f)),
                BinOp::Le => Some(IrExpr::BoolLiteral(a_val <= b_f)),
                BinOp::Gt => Some(IrExpr::BoolLiteral(a_val > b_f)),
                BinOp::Ge => Some(IrExpr::BoolLiteral(a_val >= b_f)),
                _ => None,
            }
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
        // P2-CF-2: Undefined comparisons
        (IrExpr::Undefined, IrExpr::Undefined) => {
            if op == BinOp::Eq || op == BinOp::StrictEq {
                Some(IrExpr::BoolLiteral(true))
            } else if op == BinOp::Ne || op == BinOp::StrictNe {
                Some(IrExpr::BoolLiteral(false))
            } else {
                None
            }
        }
        // undefined == null is true (loose equality), undefined === null is false
        (IrExpr::Undefined, IrExpr::Null) | (IrExpr::Null, IrExpr::Undefined) => match op {
            BinOp::Eq => Some(IrExpr::BoolLiteral(true)),
            BinOp::Ne => Some(IrExpr::BoolLiteral(false)),
            BinOp::StrictEq => Some(IrExpr::BoolLiteral(false)),
            BinOp::StrictNe => Some(IrExpr::BoolLiteral(true)),
            _ => None,
        },
        _ => None,
    }
}

fn fold_unary(op: UnaOp, operand: &IrExpr) -> Option<IrExpr> {
    match (op, operand) {
        (UnaOp::Neg, IrExpr::IntLiteral(n)) => {
            // Use checked_neg to avoid overflow panic on i64::MIN
            // (in debug builds `-i64::MIN` panics). When negation overflows
            // we leave the expression unfolded rather than producing a
            // wrapping value.
            n.checked_neg().map(IrExpr::IntLiteral)
        }
        (UnaOp::Neg, IrExpr::FloatLiteral(n)) => Some(IrExpr::FloatLiteral(-n)),
        (UnaOp::Neg, IrExpr::BigIntLiteral(s)) => {
            // Negate by toggling the leading '-' prefix.
            // -0n === 0n in JS, so "-0" → "0" is correct.
            if let Some(rest) = s.strip_prefix('-') {
                Some(IrExpr::BigIntLiteral(rest.to_string()))
            } else {
                Some(IrExpr::BigIntLiteral(format!("-{}", s)))
            }
        }
        (UnaOp::Not, IrExpr::BoolLiteral(b)) => Some(IrExpr::BoolLiteral(!b)),
        // JS `~x` operates on Int32 (32-bit signed), not i64.
        // `as i32` truncates/sign-extends like ToInt32, we bitwise-NOT
        // the i32, then sign-extend back to i64. Example: ~0xFFFFFFFF
        // → !(−1) = 0, whereas the old i64 fold gave −4294967296.
        (UnaOp::BitNot, IrExpr::IntLiteral(n)) => Some(IrExpr::IntLiteral(!(*n as i32) as i64)),
        // Double negation: !!x → x (when inner is already a bool)
        (
            UnaOp::Not,
            IrExpr::Unary {
                op: UnaOp::Not,
                operand: inner,
                ..
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

/// Returns Some(true) if the literal is truthy, Some(false) if falsy,
/// None if truthiness cannot be determined at compile time.
fn literal_truthiness(expr: &IrExpr) -> Option<bool> {
    match expr {
        IrExpr::BoolLiteral(b) => Some(*b),
        IrExpr::Null => Some(false),
        IrExpr::Undefined => Some(false),
        IrExpr::IntLiteral(n) => Some(*n != 0),
        IrExpr::FloatLiteral(n) => Some(*n != 0.0 && !n.is_nan()),
        IrExpr::StringLiteral(s) => Some(!s.is_empty()),
        // P2-CF-3: BigIntLiteral: "0" and "-0" are falsy, any non-zero value is truthy.
        // Strip leading '-' so "-0" is correctly detected as falsy.
        IrExpr::BigIntLiteral(s) => {
            let stripped = s.strip_prefix('-').unwrap_or(s);
            Some(!stripped.chars().all(|c| c == '0'))
        }
        _ => None,
    }
}

fn fold_logical(op: LogicalOp, left: &IrExpr, right: &IrExpr) -> Option<IrExpr> {
    // Nullish coalescing: only null/undefined trigger short-circuit.
    if op == LogicalOp::Nullish {
        return match left {
            IrExpr::Null | IrExpr::Undefined => Some(right.clone()),
            // Known non-nullish literal → result is left (short-circuit)
            IrExpr::IntLiteral(_)
            | IrExpr::FloatLiteral(_)
            | IrExpr::StringLiteral(_)
            | IrExpr::BoolLiteral(_)
            | IrExpr::BigIntLiteral(_) => Some(left.clone()),
            _ => None,
        };
    }

    // And/Or: use JS truthiness for all literal types, not just booleans.
    let left_truthy = literal_truthiness(left)?;

    match op {
        LogicalOp::And => {
            if left_truthy {
                // truthy && right → right
                Some(right.clone())
            } else {
                // falsy && right → left (short-circuit)
                Some(left.clone())
            }
        }
        LogicalOp::Or => {
            if left_truthy {
                // truthy || right → left (short-circuit)
                Some(left.clone())
            } else {
                // falsy || right → right
                Some(right.clone())
            }
        }
        LogicalOp::Nullish => unreachable!(),
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
        IrExpr::BigIntLiteral(_) => Some("bigint".to_string()),
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
                operand_type: Some(ZigType::I64),
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
                operand_type: Some(ZigType::Bool),
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

    // ── P0-9: Int32 bitwise / shift / BitNot semantics ──

    /// Helper: build `op(a, b)`, run the fold pass, assert the result is
    /// `IntLiteral(expected)`. JS bitwise operators operate on Int32
    /// (32-bit signed), so the folded i64 must match the sign-extended
    /// Int32 result.
    fn assert_fold_binary_int(op: BinOp, a: i64, b: i64, expected: i64) {
        let mut module = make_module_with_body(vec![IrStmt::Return {
            value: Some(IrExpr::Binary {
                op,
                left: Box::new(IrExpr::IntLiteral(a)),
                right: Box::new(IrExpr::IntLiteral(b)),
                left_type: None,
                right_type: None,
            }),
        }]);
        let mut pass = ConstantFoldPass::new();
        let result = pass.run(&mut module);
        assert!(result.changed, "expected fold to fire");
        match first_return_value(&module) {
            IrExpr::IntLiteral(n) => assert_eq!(
                *n, expected,
                "binary fold of ({a}, {b}) gave {n}, expected {expected}"
            ),
            other => panic!("expected IntLiteral({expected}), got {:?}", other),
        }
    }

    /// Helper: build `op(n)`, run the fold pass, assert `IntLiteral(expected)`.
    fn assert_fold_unary_int(op: UnaOp, n: i64, expected: i64) {
        let mut module = make_module_with_body(vec![IrStmt::Return {
            value: Some(IrExpr::Unary {
                op,
                operand: Box::new(IrExpr::IntLiteral(n)),
                operand_type: Some(ZigType::I64),
            }),
        }]);
        let mut pass = ConstantFoldPass::new();
        let result = pass.run(&mut module);
        assert!(result.changed, "expected fold to fire");
        match first_return_value(&module) {
            IrExpr::IntLiteral(got) => assert_eq!(
                *got, expected,
                "unary fold of ({n}) gave {got}, expected {expected}"
            ),
            other => panic!("expected IntLiteral({expected}), got {:?}", other),
        }
    }

    #[test]
    fn test_fold_bitand_int32() {
        // 0xFFFFFFFF & 0xFFFFFFFF: ToInt32 = -1, -1 & -1 = -1.
        // Old i64 fold gave 4294967295 (treated as 64-bit) — wrong.
        assert_fold_binary_int(BinOp::BitAnd, 0xFFFF_FFFF, 0xFFFF_FFFF, -1);
    }

    #[test]
    fn test_fold_bitand_truncates_above_i32() {
        // 0x1_00000000 (2^32) as i32 = 0 (truncated to 32 bits). 0 & -1 = 0.
        assert_fold_binary_int(BinOp::BitAnd, 0x1_0000_0000, 0xFFFF_FFFF, 0);
    }

    #[test]
    fn test_fold_bitor_int32() {
        // 0x80000000 | 0: ToInt32 = -2147483648 | 0 = -2147483648.
        assert_fold_binary_int(BinOp::BitOr, 0x8000_0000, 0, -2147483648);
    }

    #[test]
    fn test_fold_bitxor_int32() {
        // 0xFFFFFFFF ^ 0xFFFFFFFF: -1 ^ -1 = 0.
        assert_fold_binary_int(BinOp::BitXor, 0xFFFF_FFFF, 0xFFFF_FFFF, 0);
    }

    #[test]
    fn test_fold_shl_int32() {
        // 1 << 31: 0x80000000 as i32 = -2147483648 (sign-extended to i64).
        assert_fold_binary_int(BinOp::Shl, 1, 31, -2147483648);
    }

    #[test]
    fn test_fold_shl_masks_count() {
        // 1 << 32: JS masks shift count to 5 bits → 32 & 0x1F = 0 → 1 << 0 = 1.
        assert_fold_binary_int(BinOp::Shl, 1, 32, 1);
    }

    #[test]
    fn test_fold_shr_int32() {
        // -1 >> 1: arithmetic (sign-propagating) shift keeps the sign → -1.
        assert_fold_binary_int(BinOp::Shr, -1, 1, -1);
    }

    #[test]
    fn test_fold_bitnot_int32() {
        // ~0 = -1 (coincidentally same as old i64 fold; sanity check).
        assert_fold_unary_int(UnaOp::BitNot, 0, -1);
        // ~0xFFFFFFFF: ToInt32 = -1, ~(-1) = 0.
        // Old i64 fold gave -4294967296 — wrong.
        assert_fold_unary_int(UnaOp::BitNot, 0xFFFF_FFFF, 0);
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
