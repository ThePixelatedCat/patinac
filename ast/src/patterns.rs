use ident::Ident;

use crate::exprs::LitExpr;

#[derive(Debug, Clone, PartialEq)]
pub enum Pat {
    Literal {
        negate: bool,
        literal: LitExpr,
    },
    Wildcard,
    Ident {
        ident: Ident,
        subpat: Option<Box<Pat>>,
    },
    Tuple(Vec<Pat>),
    Array(Vec<Pat>, Option<ArrayRestPat>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArrayRestPat {
    Discard,
    Name(Ident),
}
