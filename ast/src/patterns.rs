use ident::Ident;
use span::impl_span;

use crate::exprs::LitExpr;

impl_span!(PatKind<VarIdent> as Pat<VarIdent>);

#[derive(Debug, Clone, PartialEq)]
pub enum PatKind<VarIdent> {
    Literal { negate: bool, lit: LitExpr },
    Wildcard,
    Ident(VarIdent),
    Constructor(Ident, Vec<Pat<VarIdent>>),
    Tuple(Vec<Pat<VarIdent>>),
}

impl PatKind<Ident> {
    pub fn ident(string: &str) -> Self {
        Self::Ident(Ident::new(string))
    }
}
