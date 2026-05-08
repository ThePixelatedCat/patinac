use smallvec::SmallVec;

use ident::{Ident, SpanIdent};
use span::Span;
use types::Ty;

use crate::{exprs::Expr, patterns::Pat};

#[derive(Debug, PartialEq)]
pub struct ExecItem<TyInfo, AdtIdent, VarIdent> {
    pub ident: VarIdent,
    pub ident_span: Span,
    pub kind: ExecKind<TyInfo, AdtIdent, VarIdent>,
}

#[derive(Debug, PartialEq)]
pub enum ExecKind<T, A, V> {
    Const {
        ty: Option<Ty<A>>,
        val: Expr<T, A, V>,
    },
    Fn {
        generics: SmallVec<[A; 4]>,
        params: Vec<Param<A, V>>,
        ret_mut: bool,
        ret_ty: Ty<A>,
        body: Expr<T, A, V>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param<A, V> {
    pub mutable: bool,
    pub pat: Pat<V>,
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
