use ident::Ident;
use span::impl_span;

impl_span!(TyKind as Ty);

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub mutable: bool,
    pub ty: Ty,
}
