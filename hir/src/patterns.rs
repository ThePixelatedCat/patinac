use ident::Ident;
use span::Span;

use crate::{VarId, exprs::LitExpr};

#[derive(Debug, Clone, PartialEq)]
pub struct Pat {
    pub kind: PatKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PatKind {
    Literal { negate: bool, lit: LitExpr },
    Wildcard,
    Ident(Ident, VarId),
    Constructor(Ident, Vec<Pat>),
    Tuple(Vec<Pat>),
}

impl PatKind {
    pub fn span(self, span: impl Into<Span>) -> Pat {
        Pat {
            kind: self,
            span: span.into(),
        }
    }

    pub fn ident(string: &str) -> Self {
        Self::Ident(Ident::new(string), VarId::new())
    }
}
