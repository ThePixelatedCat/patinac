use derive_more::Display;

use errors::Error;
use span::Span;

use crate::TokKind;

#[derive(Debug, Display, PartialEq, Eq, Clone)]
pub enum ErrorKind {
    #[display("invalid token")]
    BadToken,
    #[display("expected {expected}, found {found}")]
    Mismatched { expected: TokKind, found: TokKind },
    #[display("unexpected token {_0}")]
    Unexpected(TokKind),
}

impl ErrorKind {
    pub fn span(self, span: impl Into<Span>) -> Error<Self> {
        Error::new(self, span)
    }
}
