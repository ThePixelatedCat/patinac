use ident::{Ident, SpanIdent};
use smallvec::SmallVec;
use span::Span;

use crate::{exprs::Expr, patterns::Pat, types::Ty};

#[derive(Debug, Clone, PartialEq)]
pub struct ExecItem<TyInfo, AdtIdent, VarIdent> {
    pub ident: SpanIdent,
    pub kind: ExecKind<TyInfo, AdtIdent, VarIdent>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExecKind<T, A, V> {
    Const {
        ty: Option<Ty<A>>,
        val: Expr<T, A, V>,
    },
    Func {
        generics: SmallVec<[A; 4]>,
        params: Vec<Param<A>>,
        return_ty: Ty<A>,
        body: Expr<T, A, V>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param<A> {
    pub mutable: bool,
    pub pat: Pat,
    pub ty: Ty<A>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdtItem<AdtIdent> {
    pub ident: SpanIdent,
    pub generics: SmallVec<[AdtIdent; 4]>,
    pub span: Span,
    pub kind: AdtKind<AdtIdent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdtKind<A> {
    Record(Vec<Field<A>>),
    Enum(Vec<Variant<A>>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant<A> {
    pub ident: SpanIdent,
    pub fields: Vec<Field<A>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field<A> {
    pub ident: Ident,
    pub ty: Ty<A>,
    pub span: Span,
}
