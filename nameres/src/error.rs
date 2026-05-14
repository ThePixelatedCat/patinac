use derive_more::Display;

use errors::Error;
use ident::Ident;
use span::Span;

pub type Result<T> = errors::Result<T, ErrorKind>;

#[derive(Debug, Display, PartialEq, Eq, Clone)]
pub enum ErrorKind {
    #[display("Unbound variable")]
    UnboundVariable,
    #[display("Duplicate item with name {_0} (first occurence at {_1})")]
    DupItem(Ident, Span),
    #[display("Unknown type")]
    UnknownType,
    #[display("This type expects {_0} arguments but has {_1}")]
    GenericCount(usize, usize),
}

impl std::error::Error for ErrorKind {}

impl ErrorKind {
    pub fn span(self, span: impl Into<Span>) -> Error<ErrorKind> {
        Error::span(self, span)
    }
}
