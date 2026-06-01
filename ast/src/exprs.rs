use std::range::Range;

use ident::{Ident, SpanIdent};

use crate::{patterns::Pat, types::Ty};

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Decl {
        binding: Binding,
        val: Expr,
        span: Range<usize>,
    },
    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Range<usize>,
}

impl Expr {
    pub fn as_block(self, span: impl Into<Range<usize>>) -> BlockExpr {
        BlockExpr {
            stmts: vec![Stmt::Expr(self)],
            span: span.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Ident(Ident),
    Lit(LitExpr),
    Array(Vec<Expr>),
    Tuple(Vec<Expr>),
    Infix {
        op: InfixOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Prefix {
        op: PrefixOp,
        expr: Box<Expr>,
    },
    Field {
        base: Box<Expr>,
        field: SpanIdent,
    },
    Index {
        arr: Box<Expr>,
        idx: Box<Expr>,
    },
    Call {
        func: Box<Expr>,
        args: Vec<Arg>,
    },
    Lambda {
        params: Vec<Binding>,
        body: Box<Expr>,
    },
    If {
        cond: Box<Expr>,
        th: BlockExpr,
        el: Option<BlockExpr>,
    },
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    For {
        pat: Pat,
        iter: Box<Expr>,
        body: BlockExpr,
    },
    Loop(BlockExpr),
    Break,
    Continue,
    Return(Box<Expr>),
    Block(BlockExpr),

    Print(Box<Expr>),
}

impl ExprKind {
    pub fn span(self, span: impl Into<Range<usize>>) -> Expr {
        Expr {
            kind: self,
            span: span.into(),
        }
    }

    pub fn ident(string: &str) -> Self {
        Self::Ident(Ident::new(string))
    }

    pub const fn int(i: u64) -> Self {
        Self::Lit(LitExpr::Int(i))
    }

    pub const fn float(f: f64) -> Self {
        Self::Lit(LitExpr::Float(f))
    }

    pub const fn char(c: char) -> Self {
        Self::Lit(LitExpr::Char(c))
    }

    pub fn string(s: &str) -> Self {
        Self::Lit(LitExpr::String(String::from(s)))
    }

    pub const fn bool(b: bool) -> Self {
        Self::Lit(LitExpr::Bool(b))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LitExpr {
    Int(u64),
    Float(f64),
    Char(char),
    String(String),
    Bool(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Binding {
    pub mutable: bool,
    pub pat: Pat,
    pub ty: Option<Ty>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Arg {
    pub val: Expr,
    pub mutable: bool,
    pub span: Range<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pat: Pat,
    pub body: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockExpr {
    pub stmts: Vec<Stmt>,
    pub span: Range<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfixOp {
    Assign,
    Add,
    AddF,
    Sub,
    SubF,
    Mul,
    MulF,
    Div,
    DivF,
    Exp,
    And,
    Or,
    Xor,
    Eqq,
    Neq,
    Gt,
    Lt,
    Geq,
    Leq,
}

impl InfixOp {
    pub const fn binding_power(self) -> (u8, u8) {
        match self {
            Self::Assign => (1, 0),
            Self::Or => (3, 4),
            Self::And => (5, 6),
            Self::Eqq | Self::Neq => (7, 8),
            Self::Gt | Self::Lt | Self::Leq | Self::Geq => (9, 10),
            Self::Xor => (13, 14),
            Self::Add | Self::AddF | Self::Sub | Self::SubF => (17, 18),
            Self::Mul | Self::MulF | Self::Div | Self::DivF => (19, 20),
            Self::Exp => (22, 21),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefixOp {
    Not,
    Neg,
}

impl PrefixOp {
    pub const fn binding_power(self) -> u8 {
        match self {
            Self::Neg | Self::Not => 51,
        }
    }
}
