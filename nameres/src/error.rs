use derive_more::Display;

use ident::{Ident, SpanIdent};
use span::Span;
use types::Ty;

pub type Result<T> = errors::Result<T, ErrorKind>;
pub type Error = errors::Error<ErrorKind>;

#[derive(Debug, Display, PartialEq, Eq, Clone)]
pub enum ErrorKind {
    #[display("Unbound variable {_0}")]
    UnboundVariable(Ident),
    #[display("Duplicate item with name {_0} (first occurence at {_1})")]
    DupItem(Ident, Span),
    #[display("Unknown type {_0}")]
    UnknownType(Ty<SpanIdent>),
}

impl std::error::Error for ErrorKind {}

impl ErrorKind {
    pub fn span(self, span: impl Into<Span>) -> Error {
        Error::span(self, span)
    }
}
