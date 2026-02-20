use crate::helpers::Span;

use super::{Expr, Pattern, Ty};

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Const {
        ident: String,
        ty: Option<Ty>,
        value: Expr,
    },
    Func {
        ident: String,
        params: Vec<Pattern>,
        return_ty: Option<Ty>,
        body: Expr,
    },
    Record {
        def: AdtDef,
        fields: Vec<Field>,
    },
    Enum {
        def: AdtDef,
        variants: Vec<Variant>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub ident: String,
    pub ty: Ty,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    pub ident: String,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdtDef {
    pub ident: String,
    pub generics: Vec<GenericParam>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericParam {
    pub ident: String,
    pub span: Span,
}
