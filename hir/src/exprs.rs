use slotmap::new_key_type;

use ident::{Ident, SpanIdent};
use span::Span;
use types::Ty;

use crate::{VarId, patterns::Pat};

new_key_type! { pub struct ExprId; }

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Decl {
        binding: Binding,
        val: ExprId,
        span: Span,
    },
    Expr(ExprId),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Ident(Ident, VarId),
    Lit(LitExpr),
    Array(Vec<ExprId>),
    Tuple(Vec<ExprId>),
    Infix {
        op: InfixOp,
        lhs: ExprId,
        rhs: ExprId,
    },
    Unary {
        op: UnaryOp,
        expr: ExprId,
    },
    Field {
        base: ExprId,
        field: SpanIdent,
    },
    Index {
        arr: ExprId,
        idx: ExprId,
    },
    Call {
        func: ExprId,
        args: Vec<Arg>,
    },
    Lambda {
        params: Vec<Binding>,
        body: ExprId,
    },
    If {
        cond: ExprId,
        th: BlockExpr,
        el: Option<BlockExpr>,
    },
    Match {
        scrutinee: ExprId,
        arms: Vec<MatchArm>,
    },
    For {
        pat: Pat,
        iter: ExprId,
        body: BlockExpr,
    },
    Loop(BlockExpr),
    Break,
    Continue,
    Return(ExprId),
    Block(BlockExpr),
}

impl Expr {
    pub fn ident_str(string: &str) -> Self {
        Self::Ident(Ident::new(string), VarId::new())
    }

    pub fn ident_id(ident: Ident) -> Self {
        Self::Ident(ident, VarId::new())
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
    pub mutable: bool,
    pub val: ExprId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pat: Pat,
    pub body: ExprId,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockExpr(pub Vec<Stmt>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfixOp {
    Assign,
    Add,
    Sub,
    Mul,
    Div,
    Exp,
    Rem,
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
            Self::Add | Self::Sub => (17, 18),
            Self::Mul | Self::Div | Self::Rem => (19, 20),
            Self::Exp => (22, 21),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
    Neg,
}

impl UnaryOp {
    pub const fn binding_power(self) -> u8 {
        match self {
            Self::Neg | Self::Not => 51,
        }
    }
}
