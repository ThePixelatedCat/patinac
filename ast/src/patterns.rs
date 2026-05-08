use ident::Ident;
use span::Span;

use crate::exprs::LitExpr;

#[derive(Debug, Clone, PartialEq)]
pub struct Pat<VarIdent> {
    pub kind: PatKind<VarIdent>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PatKind<V> {
    Literal { negate: bool, lit: LitExpr },
    Wildcard,
    Ident(V),
    Constructor(Ident, Vec<Pat<V>>),
    Tuple(Vec<Pat<V>>),
}

impl<V> PatKind<V> {
    pub fn span(self, span: impl Into<Span>) -> Pat<V> {
        Pat {
            kind: self,
            span: span.into(),
        }
    }
}

impl PatKind<Ident> {
    pub fn ident(string: &str) -> Self {
        Self::Ident(Ident::new(string))
    }
}
