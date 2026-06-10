use std::range::Range;

use derive_more::Display;

use errors::SpanError;
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
    OverlappingPlace(Range<u32>),
    #[display("duplicate item with name {_0} (first occurence at {_1:?})")]
    DupItem(Ident, Range<u32>),
    #[display("duplicate field {_0}")]
    DupFields(Ident),
    #[display("unknown type")]
    UnknownType,
    #[display(
        "invalid `main` function. The `main` function must take no parameters and return no value"
    )]
    InvalidMain,
}

impl SpanError for ErrorKind {}
