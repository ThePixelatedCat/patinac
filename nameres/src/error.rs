use derive_more::Display;

use ast::types::TyKind;
use ident::Ident;
use span::Span;

pub type Result<T> = errors::Result<T, ErrorKind>;
pub type Error = errors::Error<ErrorKind>;

#[derive(Debug, Display, PartialEq, Eq, Clone)]
pub enum ErrorKind {
    #[display("Unbound variable {_0}")]
    UnboundVariable(Ident),
    #[display("Duplicate item with name {_0}")]
    DupItem(Ident),
    #[display("Unknown type {_0}")]
    UnknownType(TyKind<Ident>),
}

impl std::error::Error for ErrorKind {}

impl ErrorKind {
    pub fn span(self, span: impl Into<Span>) -> Error {
        Error::span(self, span)
    }
}
