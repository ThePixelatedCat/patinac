use ident::Ident;
use span::{Span, impl_span};

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
    Fn(Vec<Param>, Box<Ty>),
    Adt(Ident, Vec<Ty>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub mutable: bool,
    pub ty: Ty,
}

impl_span!(TyKind as Ty);
