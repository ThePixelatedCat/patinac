use span::Span;

mod exprs;
mod items;

pub use exprs::*;
pub use items::*;
use string_interner::DefaultSymbol;

pub struct Ast<T> {
    adts: Vec<AdtItem>,
    execs: Vec<ExecItem<T>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ident(DefaultSymbol);

impl From<DefaultSymbol> for Ident {
    fn from(value: DefaultSymbol) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    pub pat: Pat,
    pub ty: Option<Ty>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pat {
    Tuple(Vec<Pat>),
    Var { mutable: bool, ident: Ident },
    Discard,
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
    Adt(Ident, Vec<Ty>),
}
