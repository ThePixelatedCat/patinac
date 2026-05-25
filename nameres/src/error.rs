use derive_more::Display;

use errors::Error;
use ident::Ident;
use span::Span;

#[derive(Debug, Display, PartialEq, Eq, Clone)]
pub enum ErrorKind {
    #[display("unbound variable")]
    UnboundVariable,
    #[display("cannot mutate this immutable value")]
    Mutation,
    #[display("expected this to be a mutable place")]
    NotPlaceExpr,
    #[display("this mutable place overlaps with {_0}")]
    OverlappingPlace(Span),
    #[display("duplicate item with name {_0} (first occurence at {_1})")]
    DupItem(Ident, Span),
    #[display("duplicate field {_0}")]
    DupFields(Ident),
    #[display("unknown type")]
    UnknownType,
    #[display("this type expects {_0} arguments but has {_1}")]
    GenericCount(usize, usize),
    #[display(
        "invalid `main` function. The `main` function must take no parameters and return no value"
    )]
    InvalidMain,
}

impl std::error::Error for ErrorKind {}

impl ErrorKind {
    pub fn span(self, span: impl Into<Span>) -> Error<Self> {
        Error::new(self, span)
    }
}
