use std::range::Range;

use derive_more::Display;

use errors::SpanError;
use ident::Ident;

#[derive(Debug, Display, PartialEq, Eq)]
pub enum ErrorKind {
    #[display("unresolved {_0} `{_1}`")]
    UnknownName(ItemKind, Ident),
    #[display("{_0} `{_1}` is not visible")]
    PrivateItem(ItemKind, Ident),
    #[display("cannot mutate this immutable value")]
    Mutation,
    #[display("expected this to be a mutable place")]
    NotPlaceExpr,
    #[display("these mutable arguments overlap")]
    OverlappingPlaces(Range<u32>, Range<u32>),
    #[display("duplicate {_0} `{_1}`")]
    DupItem(ItemKind, Ident),
    #[display("duplicate field `{_0}`")]
    DupFields(Ident),
    #[display(
        "invalid `main` function. The `main` function must take no parameters and return no value"
    )]
    InvalidMain,
    #[display("`self` parameters can only appear on functions within `impl` blocks")]
    SelfOutsideImpl,
}

#[derive(Clone, Copy, Debug, Display, PartialEq, Eq)]
pub enum ItemKind {
    #[display("module")]
    Module,
    #[display("type")]
    Type,
    #[display("value")]
    Value,
    #[display("item")]
    Unknown,
}

impl SpanError for ErrorKind {}
