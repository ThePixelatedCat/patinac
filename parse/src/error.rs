use thiserror::Error as ThisError;

use lex::TokKind;
use span::impl_span;

pub type Result<T> = std::result::Result<T, Error>;

impl_span!(ErrorKind as Error);

#[derive(ThisError, Debug, PartialEq, Eq, Clone)]
pub enum ErrorKind {
    #[error("expected {expected}, found {found}")]
    Mismatched { expected: TokKind, found: TokKind },
    #[error("unexpected token {0} at {1}")]
    Unexpected(TokKind, &'static str),
    #[error("unexpected end of file")]
    Eof,
}
