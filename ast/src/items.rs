use std::range::Range;

use smallvec::SmallVec;

use ident::SpanIdent;

use crate::{exprs::Expr, patterns::Pat, types::Ty};

#[derive(Debug, PartialEq)]
pub struct ExecItem {
    pub ident: SpanIdent,
    pub kind: ExecKind,
}

#[derive(Debug, PartialEq)]
pub enum ExecKind {
    Const {
        ty: Ty,
        val: Expr,
    },
    Fn {
        generics: SmallVec<[SpanIdent; 4]>,
        params: Vec<Param>,
        ret_mut: bool,
        ret_ty: Ty,
        body: Expr,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub pat: Pat,
    pub ty: Ty,
    pub mutable: bool,
    pub span: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdtItem {
    pub ident: SpanIdent,
    pub generics: SmallVec<[SpanIdent; 4]>,
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
    pub ident: SpanIdent,
    pub ty: Ty,
}
