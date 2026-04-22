use ident::Ident;
use span::impl_span;

use crate::exprs::LitExpr;

impl_span!(PatKind as Pat);

#[derive(Debug, Clone, PartialEq)]
pub enum PatKind {
    Literal {
        negate: bool,
        lit: LitExpr,
    },
    Wildcard,
    Ident {
        ident: Ident,
        subpat: Option<Box<Pat>>,
    },
    Constructor(Ident, Vec<Pat>),
    Tuple(Vec<Pat>),
}
