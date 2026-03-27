use span::{Span, Spnd};

use super::{Binding, Ident, Pat, Ty};

#[derive(Debug, Clone, PartialEq)]
pub struct Expr<T> {
    pub kind: ExprKind<T>,
    pub span: Span,
    pub ty: T,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind<T> {
    Ident(Ident),
    Int(u64),
    Float(f64),
    String(String),
    Char(char),
    Bool(bool),
    Array(Vec<Expr<T>>),
    Tuple(Vec<Expr<T>>),
    App {
        func: Box<Expr<T>>,
        args: Vec<Expr<T>>,
    },
    BinOp {
        op: Bop,
        lhs: Box<Expr<T>>,
        rhs: Box<Expr<T>>,
    },
    UnaryOp {
        op: Unop,
        expr: Box<Expr<T>>,
    },
    Index {
        arr: Box<Expr<T>>,
        idx: Box<Expr<T>>,
    },
    FieldAccess {
        base: Box<Expr<T>>,
        field: Spnd<Ident>,
    },
    If {
        cond: Box<Expr<T>>,
        th: Box<Expr<T>>,
        el: Option<Box<Expr<T>>>,
    },
    For {
        pattern: Pat,
        iter: Box<Expr<T>>,
        body: Box<Expr<T>>,
    },
    While {
        cond: Box<Expr<T>>,
        body: Box<Expr<T>>,
    },
    Match {
        scrutinee: Box<Expr<T>>,
        arms: Vec<MatchArm<T>>,
    },
    Let {
        binding: Binding,
        val: Box<Expr<T>>,
    },
    Assign {
        ident: Ident,
        val: Box<Expr<T>>,
    },
    Lambda {
        params: Vec<Binding>,
        return_ty: Option<Ty>,
        body: Box<Expr<T>>,
    },
    Block(Vec<Expr<T>>),
}

impl ExprKind<()> {
    pub fn span(self, span: impl Into<Span>) -> Expr<()> {
        Expr {
            kind: self,
            span: span.into(),
            ty: (),
        }
    }
}

impl<T> ExprKind<T> {
    pub fn span_ty(self, span: impl Into<Span>, ty: T) -> Expr<T> {
        Expr {
            kind: self,
            span: span.into(),
            ty,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm<T> {
    pub pattern: Pat,
    pub guard: Option<Box<Expr<T>>>,
    pub body: Box<Expr<T>>,
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
