use std::range::Range;

use ident::Ident;

use crate::exprs::LitExpr;

#[derive(Debug, Clone, PartialEq)]
pub struct Pat {
    pub kind: PatKind,
    pub span: Range<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PatKind {
    Literal { negate: bool, lit: LitExpr },
    Wildcard,
    Ident(Ident),
    Constructor(Ident, Vec<Pat>),
    Tuple(Vec<Pat>),
}

impl PatKind {
    pub fn span(self, span: impl Into<Range<usize>>) -> Pat {
        Pat {
            kind: self,
            span: span.into(),
        }
    }

    pub fn ident(string: &str) -> Self {
        Self::Ident(Ident::new(string))
    }
}
