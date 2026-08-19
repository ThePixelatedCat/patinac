use std::range::Range;

use derive_more::Display;

use errors::SpanError;
use ident::Ident;

#[derive(Debug, Display, PartialEq, Eq)]
pub enum ErrorKind {
    #[display("unresolved item `{_0}`")]
    UnknownItem(Ident),
    #[display("unresolved module `{_0}`")]
    UnknownModule(Ident),
    #[display("unresolved type `{_0}`")]
    UnknownType(Ident),
    #[display("unresolved variable `{_0}`")]
    UnknownVar(Ident),
    #[display("item `{_0}` is not visible")]
    PrivateItem(Ident),
    #[display("cannot mutate this immutable value")]
    Mutation,
    #[display("expected this to be a mutable place")]
    NotPlaceExpr,
    #[display("these mutable arguments overlap")]
    OverlappingPlaces(Range<u32>, Range<u32>),
    #[display("duplicate item `{_0}`")]
    DupItem(Ident),
    #[display("duplicate field `{_0}`")]
    DupFields(Ident),
    #[display(
        "invalid `main` function. The `main` function must take no parameters and return no value"
    )]
    InvalidMain,
    #[display("`self` parameters can only appear on functions within `impl` blocks")]
    SelfOutsideImpl,
}

impl SpanError for ErrorKind {}
