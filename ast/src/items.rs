use ident::Ident;
use span::Span;

use crate::{Pat, exprs::Expr, types::Ty};

#[derive(Debug, Clone, PartialEq)]
pub enum ExecItem<T> {
    Const {
        ident: Ident,
        ty: Option<Ty>,
        val: Expr<T>,
    },
    Func {
        ident: Ident,
        generic_params: Vec<GenericParam>,
        params: Vec<Param>,
        return_ty: Ty,
        body: Expr<T>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub mutable: bool,
    pub pat: Pat,
    pub ty: Ty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdtItem {
    Record { def: AdtDef, fields: Vec<Field> },
    Enum { def: AdtDef, variants: Vec<Variant> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdtDef {
    pub ident: Ident,
    pub generics: Vec<GenericParam>,
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
pub struct GenericParam(pub Ident);
