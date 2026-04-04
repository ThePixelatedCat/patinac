use thiserror::Error as ThisError;

use lex::TokKind;
use span::{Span, impl_span};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, PartialEq, Eq)]
pub struct Error {
    kind: ErrorKind,
    span: Span,
}

impl Error {
    pub const fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    pub const fn span(&self) -> Span {
        self.span
    }
}

#[derive(ThisError, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    #[error("expected {expected}, found {found}")]
    Mismatched { expected: TokKind, found: TokKind },
    #[error("unexpected token {0} at {1}")]
    Unexpected(TokKind, &'static str),
    #[error("unexpected end of file")]
    Eof,
}

impl_span!(ErrorKind as Error);
