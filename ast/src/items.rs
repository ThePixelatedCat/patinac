use span::{Span, Spnd};

use super::{Binding, Expr, Ident, Ty};

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Const {
        ident: Ident,
        ty: Option<Ty>,
        value: Expr,
    },
    Func {
        ident: Ident,
        params: Vec<Binding>,
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
    pub generics: Vec<GenericParam>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericParam(pub Spnd<Ident>);
