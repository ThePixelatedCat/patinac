use ident::Ident;
use span::impl_span;

use crate::exprs::LitExpr;

impl_span!(PatKind as Pat);

#[derive(Debug, Clone, PartialEq)]
pub enum PatKind {
    Literal { negate: bool, lit: LitExpr },
    Wildcard,
    Ident(Ident),
    Constructor(Ident, Vec<Pat>),
    Tuple(Vec<Pat>),
}

impl PatKind {
    pub fn ident(string: &str) -> Self {
        Self::Ident(Ident::new(string))
    }
}
