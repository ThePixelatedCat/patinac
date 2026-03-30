use span::Span;

use super::{Binding, Ident, Pat, Ty};

#[derive(Debug, Clone, PartialEq)]
pub struct Expr<T> {
    pub kind: ExprKind<T>,
    pub span: Span,
    pub ty: T,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind<T> {
    Place(PlaceExpr<T>),
    Lit(LitExpr),
    Array(Vec<Expr<T>>),
    Tuple(Vec<Expr<T>>),
    InfixExpr {
        op: InfixOp,
        lhs: Box<Expr<T>>,
        rhs: Box<Expr<T>>,
    },
    UnaryExpr {
        op: UnaryOp,
        expr: Box<Expr<T>>,
    },
    FieldExpr {
        base: Box<Expr<T>>,
        field: Ident,
    },
    IndexExpr {
        arr: Box<Expr<T>>,
        idx: Box<Expr<T>>,
    },
    CallExpr {
        func: Box<Expr<T>>,
        args: Vec<Arg<T>>,
    },
    LambdaExpr {
        params: Vec<Binding>,
        return_ty: Option<Ty>,
        body: Box<Expr<T>>,
    },
    Let {
        binding: Binding,
        val: Box<Expr<T>>,
    },
    Assign {
        place: Box<Expr<T>>,
        val: Box<Expr<T>>,
    },
    If {
        cond: Box<Expr<T>>,
        th: Box<Expr<T>>,
        el: Option<Box<Expr<T>>>,
    },
    Match {
        scrutinee: Box<Expr<T>>,
        arms: Vec<MatchArm<T>>,
    },
    Loop {
        label: Option<LoopLabel>,
        kind: LoopKind<T>,
    },
    Break(Option<LoopLabel>),
    Continue(Option<LoopLabel>),
    Return(Box<Expr<T>>),
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

    pub fn ident(ident: Ident) -> Self {
        Self::Place(PlaceExpr::Ident(ident))
    }

    pub fn int(i: u64) -> Self {
        Self::Lit(LitExpr::Int(i))
    }

    pub fn float(f: f64) -> Self {
        Self::Lit(LitExpr::Float(f))
    }

    pub fn char(c: char) -> Self {
        Self::Lit(LitExpr::Char(c))
    }

    pub fn string(s: &str) -> Self {
        Self::Lit(LitExpr::String(String::from(s)))
    }

    pub fn bool(b: bool) -> Self {
        Self::Lit(LitExpr::Bool(b))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlaceExpr<T> {
    Ident(Ident),
    Field(Box<PlaceExpr<T>>, Ident),
    Index(Box<PlaceExpr<T>>, Box<Expr<T>>),
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
pub struct Arg<T> {
    pub mutable: bool,
    pub label: Option<Pat>,
    pub val: Expr<T>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm<T> {
    pub pattern: Pat,
    pub guard: Option<Box<Expr<T>>>,
    pub body: Box<Expr<T>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopLabel(Ident);

#[derive(Debug, Clone, PartialEq)]
pub enum LoopKind<T> {
    For {
        pattern: Pat,
        iter: Box<Expr<T>>,
        body: Box<Expr<T>>,
    },
    While {
        cond: Box<Expr<T>>,
        body: Box<Expr<T>>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfixOp {
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
