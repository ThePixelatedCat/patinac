use ident::{Ident, SpanIdent};
use smallvec::smallvec;
use span::Span;

use crate::{Path, patterns::Pat, types::Ty};

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt<TyInfo, AdtIdent, VarIdent> {
    Decl {
        binding: Binding<AdtIdent>,
        val: Box<Expr<TyInfo, AdtIdent, VarIdent>>,
        span: Span,
    },
    Expr(Expr<TyInfo, AdtIdent, VarIdent>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expr<TyInfo, AdtIdent, VarIdent> {
    pub kind: ExprKind<TyInfo, AdtIdent, VarIdent>,
    pub span: Span,
    pub ty: TyInfo,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind<T, A, V> {
    Path(Path<A, V>),
    Lit(LitExpr),
    Array(Vec<Expr<T, A, V>>),
    Tuple(Vec<Expr<T, A, V>>),
    InfixExpr {
        op: InfixOp,
        lhs: Box<Expr<T, A, V>>,
        rhs: Box<Expr<T, A, V>>,
    },
    UnaryExpr {
        op: UnaryOp,
        expr: Box<Expr<T, A, V>>,
    },
    FieldExpr {
        base: Box<Expr<T, A, V>>,
        field: SpanIdent,
    },
    IndexExpr {
        arr: Box<Expr<T, A, V>>,
        idx: Box<Expr<T, A, V>>,
    },
    CallExpr {
        func: Box<Expr<T, A, V>>,
        args: Vec<Arg<T, A, V>>,
    },
    LambdaExpr {
        params: Vec<Binding<A>>,
        return_ty: Option<Ty<A>>,
        body: Box<Expr<T, A, V>>,
    },
    If {
        cond: Box<Expr<T, A, V>>,
        th: Box<Expr<T, A, V>>,
        el: Option<Box<Expr<T, A, V>>>,
    },
    Match {
        scrutinee: Box<Expr<T, A, V>>,
        arms: Vec<MatchArm<T, A, V>>,
    },
    For {
        pat: Pat,
        iter: Box<Expr<T, A, V>>,
        body: Box<Expr<T, A, V>>,
    },
    Loop(Box<Expr<T, A, V>>),
    Break,
    Continue,
    Return(Box<Expr<T, A, V>>),
    Block(Vec<Stmt<T, A, V>>),
}

impl<A, V> ExprKind<(), A, V> {
    pub fn span(self, span: impl Into<Span>) -> Expr<(), A, V> {
        Expr {
            kind: self,
            span: span.into(),
            ty: (),
        }
    }
}

impl<T, A> ExprKind<T, A, Ident> {
    pub fn ident(string: &str) -> Self {
        Self::Path(Path {
            prefix: smallvec![],
            end: Ident::new(string),
        })
    }
}

impl<T, A, V> ExprKind<T, A, V> {
    pub fn span_ty(self, span: impl Into<Span>, ty: T) -> Expr<T, A, V> {
        Expr {
            kind: self,
            span: span.into(),
            ty,
        }
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
pub struct Binding<AdtIdent> {
    pub mutable: bool,
    pub pat: Pat,
    pub ty: Option<Ty<AdtIdent>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Arg<T, A, V> {
    pub mutable: bool,
    pub val: Expr<T, A, V>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm<T, A, V> {
    pub pat: Pat,
    pub body: Expr<T, A, V>,
    pub span: Span,
}

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
