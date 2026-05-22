use std::result;

use derive_more::Display;

use span::Span;

use crate::TokKind;

pub type Result<T> = result::Result<T, ()>;
pub type Error = errors::Error<ErrorKind>;

impl ErrorKind {
    pub fn span(self, span: impl Into<Span>) -> Error {
        Error::new(self, span)
    }
}

#[derive(Debug, Display, PartialEq, Eq, Clone)]
pub enum ErrorKind {
    #[display("invalid token")]
    BadToken,
    #[display("expected {expected}, found {found}")]
    Mismatched { expected: TokKind, found: TokKind },
    #[display("Unexpected token {_0}")]
    Unexpected(TokKind),
    #[display("unexpected end of file")]
    Eof,
}
