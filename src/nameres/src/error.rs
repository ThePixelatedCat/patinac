use std::range::Range;

use derive_more::Display;

use errors::SpanError;
use ident::Ident;

#[derive(Debug, Display, PartialEq, Eq)]
pub enum ErrorKind {
    #[display("unbound variable `{_0}`")]
    UnboundValue(Ident),
    #[display("{_0} `{_1}` found but not visible")]
    NotVisible(ItemKind, Ident),
    #[display("can't find {_0} `{_1}`")]
    UnknownItem(ItemKind, Ident),
    #[display("expected `{_0}` to be a {_1}, but found a {_2}")]
    WrongKind(Ident, ItemKind, ItemKind),
    #[display("cannot export imports")]
    Reexport,
    #[display("cannot mutate this immutable value")]
    Mutation,
    #[display("expected this to be a mutable place")]
    NotPlaceExpr,
    #[display("this mutable place overlaps with {_0:?}")]
    OverlappingPlace(Range<u32>),
    #[display("duplicate item with name `{_0}`")]
    DupItem(Ident),
    #[display("duplicate field `{_0}`")]
    DupFields(Ident),
    #[display(
        "invalid `main` function. The `main` function must take no parameters and return no value"
    )]
    InvalidMain,
}

impl SpanError for ErrorKind {}

#[derive(Clone, Copy, Debug, Display, PartialEq, Eq)]
#[display(rename_all = "lowercase")]
pub enum ItemKind {
    Value,
    Type,
    Module,
    #[display("item")]
    Unknown,
}
