// zigir/ops.rs
// Operator enums for IR expressions.

/// Binary arithmetic/comparison operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Lt,
    Gt,
    Le,
    Ge,
    Eq,
    Ne,
    StrictEq,
    StrictNe,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    UrShr,
    In,
    InstanceOf,
}

/// Unary operator (prefix).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaOp {
    Neg,    // -x
    Not,    // !x
    BitNot, // ~x
    TypeOf, // typeof x
    Void,   // void x
    Delete, // delete x (only valid in specific contexts)
}

/// Logical short-circuit operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogicalOp {
    And,     // &&
    Or,      // ||
    Nullish, // ??
}

/// Update operator (++ / --).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UpdateOp {
    Increment,
    Decrement,
}

/// Compound assignment operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssignOp {
    Assign,   // =
    Add,      // +=
    Sub,      // -=
    Mul,      // *=
    Div,      // /=
    Mod,      // %=
    Shl,      // <<=
    Shr,      // >>=
    BitAnd,   // &=
    BitOr,    // |=
    BitXor,   // ^=
    LogicAnd, // &&=
    LogicOr,  // ||=
    Nullish,  // ??=
}

impl AssignOp {
    /// Zig assignment operator string.
    pub fn to_zig_str(self) -> &'static str {
        match self {
            AssignOp::Assign => "=",
            AssignOp::Add => "+=",
            AssignOp::Sub => "-=",
            AssignOp::Mul => "*=",
            AssignOp::Div => "/=",
            AssignOp::Mod => "%=",
            AssignOp::Shl => "<<=",
            AssignOp::Shr => ">>=",
            AssignOp::BitAnd => "&=",
            AssignOp::BitOr => "|=",
            AssignOp::BitXor => "^=",
            AssignOp::LogicAnd => "and=",
            AssignOp::LogicOr => "or=",
            AssignOp::Nullish => "orelse=",
        }
    }

    /// For compound assignments on BigInt, convert to the equivalent BinOp.
    /// Returns None for non-arithmetic ops (Assign, LogicAnd, LogicOr, Nullish).
    pub fn to_bin_op(&self) -> Option<BinOp> {
        match self {
            Self::Add => Some(BinOp::Add),
            Self::Sub => Some(BinOp::Sub),
            Self::Mul => Some(BinOp::Mul),
            Self::Div => Some(BinOp::Div),
            Self::Mod => Some(BinOp::Mod),
            Self::Shl => Some(BinOp::Shl),
            Self::Shr => Some(BinOp::Shr),
            Self::BitAnd => Some(BinOp::BitAnd),
            Self::BitOr => Some(BinOp::BitOr),
            Self::BitXor => Some(BinOp::BitXor),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assign_op_zig_str() {
        assert_eq!(AssignOp::Assign.to_zig_str(), "=");
        assert_eq!(AssignOp::LogicAnd.to_zig_str(), "and=");
        assert_eq!(AssignOp::Nullish.to_zig_str(), "orelse=");
    }

    #[test]
    fn test_bin_op_coverage() {
        // Smoke test: all variants are constructible
        let ops = [
            BinOp::Add,
            BinOp::Sub,
            BinOp::Mul,
            BinOp::Div,
            BinOp::Mod,
            BinOp::Lt,
            BinOp::Gt,
            BinOp::Le,
            BinOp::Ge,
            BinOp::Eq,
            BinOp::Ne,
            BinOp::StrictEq,
            BinOp::StrictNe,
            BinOp::BitAnd,
            BinOp::BitOr,
            BinOp::BitXor,
            BinOp::Shl,
            BinOp::Shr,
            BinOp::UrShr,
            BinOp::In,
            BinOp::InstanceOf,
            BinOp::Pow,
        ];
        assert_eq!(ops.len(), 22);
    }
}
