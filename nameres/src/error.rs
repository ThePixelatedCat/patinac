use std::{error, range::Range};

use derive_more::Display;

use errors::Error;
use ident::Ident;

#[derive(Debug, Display, PartialEq, Eq, Clone)]
pub enum ErrorKind {
    #[display("unbound variable")]
    UnboundVariable,
    #[display("cannot mutate this immutable value")]
    Mutation,
    #[display("expected this to be a mutable place")]
    NotPlaceExpr,
    #[display("this mutable place overlaps with {_0:?}")]
    OverlappingPlace(Range<usize>),
    #[display("duplicate item with name {_0} (first occurence at {_1:?})")]
    DupItem(Ident, Range<usize>),
    #[display("duplicate field {_0}")]
    DupFields(Ident),
    #[display("unknown type")]
    UnknownType,
    #[display(
        "invalid `main` function. The `main` function must take no parameters and return no value"
    )]
    InvalidMain,
}

impl error::Error for ErrorKind {}

impl ErrorKind {
    pub fn span(self, span: impl Into<Range<usize>>) -> Error<Self> {
        Error::new(self, span)
    }
}
