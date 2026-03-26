use span::Span;

use crate::{Expr, Ident, Pat, Ty};

#[derive(Debug, Clone, PartialEq)]
pub enum ExecItem<T> {
    Const {
        ident: Ident,
        ty: Option<Ty>,
        value: Expr<T>,
    },
    Func {
        ident: Ident,
        params: Vec<Param>,
        return_ty: Ty,
        body: Expr<T>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub pat: Pat,
    pub ty: Ty,
}

pub enum AdtItem {
    Record { def: AdtDef, fields: Vec<Field> },
    Enum { def: AdtDef, variants: Vec<Variant> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub ident: Ident,
    pub ty: Ty,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    pub ident: Ident,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdtDef {
    pub ident: Ident,
    pub generics: Vec<Ident>,
}
