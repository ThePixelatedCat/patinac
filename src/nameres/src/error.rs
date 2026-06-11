use std::range::Range;

use derive_more::Display;

use errors::SpanError;
use ident::Ident;

#[derive(Debug, Display, PartialEq, Eq)]
pub enum ErrorKind {
    #[display("{_0} `{_1}` is not visible")]
    NotVisible(ItemKind, Ident),
    #[display("unresolved {_0} `{_1}`")]
    UnknownItem(ItemKind, Ident),
    #[display("cannot export imports")]
    Reexport,
    #[display("cannot import an item from a module into itself")]
    SelfImport,
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
