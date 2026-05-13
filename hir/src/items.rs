use smallvec::SmallVec;

use ident::{Ident, SpanIdent};
use span::Span;
use types::Ty;

use crate::{exprs::ExprId, patterns::Pat};

#[derive(Debug, PartialEq)]
pub struct ExecItem {
    pub ident: SpanIdent,
    pub kind: ExecKind,
}

#[derive(Debug, PartialEq)]
pub enum ExecKind {
    Const {
        ty: Option<Ty>,
        val: ExprId,
    },
    Fn {
        generics: SmallVec<[SpanIdent; 4]>,
        params: Vec<Param>,
        ret_mut: bool,
        ret_ty: Ty,
        body: ExprId,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub mutable: bool,
    pub pat: Pat,
    pub ty: Ty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdtItem {
    pub ident: SpanIdent,
    pub generics: SmallVec<[SpanIdent; 4]>,
    pub span: Span,
    pub kind: AdtKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdtKind {
    Record(Vec<Field>),
    Enum(Vec<Variant>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    pub ident: SpanIdent,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub ident: Ident,
    pub ty: Ty,
    pub span: Span,
}
