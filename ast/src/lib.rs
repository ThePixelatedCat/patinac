use span::Span;

mod exprs;
mod items;

pub use exprs::*;
pub use items::*;
use string_interner::DefaultSymbol;

#[derive(Default)]
pub struct Ast<T> {
    pub adts: Vec<AdtItem>,
    pub execs: Vec<ExecItem<T>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ident(DefaultSymbol);

impl From<DefaultSymbol> for Ident {
    fn from(value: DefaultSymbol) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Binding {
    pub mutable: bool,
    pub pat: Pat,
    pub ty: Option<Ty>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pat {
    Literal {
        negate: bool,
        literal: LitExpr,
    },
    Wildcard,
    Ident {
        ident: Ident,
        subpat: Option<Box<Pat>>,
    },
    Tuple(Vec<Pat>),
    Array(Vec<Pat>, Option<ArrayRestPat>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArrayRestPat {
    Discard,
    Name(Ident),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ty {
    pub kind: TyKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TyKind {
    Int,
    UInt,
    Byte,
    Float,
    Char,
    Bool,
    Array(Box<Ty>),
    Tuple(Vec<Ty>),
    Fn(Vec<(bool, Ty)>, Box<Ty>),
    Adt(Ident, Vec<Ty>),
}
