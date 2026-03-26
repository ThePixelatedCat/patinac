use span::{Span, Spnd};

use super::{Binding, Ident, Pat, Ty};

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}
#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Ident(Ident),
    Int(u64),
    Float(f64),
    String(String),
    Char(char),
    Bool(bool),
    Array(Vec<Expr>),
    Tuple(Vec<Expr>),
    FnCall {
        fun: Box<Expr>,
        args: Vec<Expr>,
    },
    BinaryOp {
        op: Bop,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    UnaryOp {
        op: Unop,
        expr: Box<Expr>,
    },
    Index {
        arr: Box<Expr>,
        index: Box<Expr>,
    },
    FieldAccess {
        base: Box<Expr>,
        field: Spnd<Ident>,
    },
    If {
        cond: Box<Expr>,
        th: Box<Expr>,
        el: Option<Box<Expr>>,
    },
    For {
        pattern: Pat,
        iter: Box<Expr>,
        body: Box<Expr>,
    },
    While {
        cond: Box<Expr>,
        body: Box<Expr>,
    },
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    Let {
        binding: Binding,
        value: Box<Expr>,
    },
    Assign {
        ident: Spnd<Ident>,
        value: Box<Expr>,
    },
    Lambda {
        params: Vec<Binding>,
        return_ty: Option<Ty>,
        body: Box<Expr>,
    },
    Block(Vec<Expr>),
}

impl ExprKind {
    pub fn span(self, span: impl Into<Span>) -> Expr {
        Expr {
            kind: self,
            span: span.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pat,
    pub guard: Option<Box<Expr>>,
    pub body: Box<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bop {
    Add,
    Sub,
    Mul,
    Div,
    Exp,
    And,
    Or,
    Xor,
    BOr,
    BAnd,
    Gt,
    Lt,
    Eqq,
    Neq,
    Geq,
    Leq,
}

impl Bop {
    pub const fn binding_power(self) -> (u8, u8) {
        match self {
            Self::Or => (3, 4),
            Self::And => (5, 6),
            Self::Eqq | Self::Neq => (7, 8),
            Self::Gt | Self::Lt | Self::Leq | Self::Geq => (9, 10),
            Self::BOr => (11, 12),
            Self::Xor => (13, 14),
            Self::BAnd => (15, 16),
            Self::Add | Self::Sub => (17, 18),
            Self::Mul | Self::Div => (19, 20),
            Self::Exp => (22, 21),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unop {
    Not,
    Neg,
}

impl Unop {
    pub const fn binding_power(self) -> u8 {
        match self {
            Self::Neg | Self::Not => 51,
        }
    }
}
