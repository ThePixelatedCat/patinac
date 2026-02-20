use crate::helpers::Span;

mod exprs;
mod items;

pub use exprs::*;
pub use items::*;
use string_interner::DefaultSymbol;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ident(DefaultSymbol);

impl From<DefaultSymbol> for Ident {
    fn from(value: DefaultSymbol) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pattern {
    Var {
        mutable: bool,
        ident: Ident,
        ty_annotation: Option<Ty>,
    },
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
    Bool,
    Char,
    Array(Box<Ty>),
    Tuple(Vec<Ty>),
    Fn(Vec<Ty>, Box<Ty>),
    Adt { ident: Ident, args: Vec<Ty> },
}
