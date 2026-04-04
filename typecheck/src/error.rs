use thiserror::Error as ThisError;

use span::{Span, impl_span};

use crate::types::{Param, Ty, TyVar};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, PartialEq)]
pub struct Error {
    kind: ErrorKind,
    span: Span,
}

#[derive(ThisError, Debug, PartialEq)]
pub enum ErrorKind {
    #[error("unbound identifier")]
    UnboundIdent,
    #[error("types `{0}` and `{1}` are not equal")]
    TypesNotEqual(Ty, Ty),
    #[error("attempted mutation of immutable place")]
    Mutation,
    #[error("infinite (TEMP)")]
    Infinite(TyVar, Ty),
    #[error("could not infer the type of the expression")]
    UninferredType,
    #[error("left hand side of an assignment must be a place expression")]
    NotPlaceExpr,
    #[error("place overlaps with {0}")]
    OverlappingPlace(Span),
    #[error("unknown type")]
    UnknownType,
    #[error("field not found")]
    MissingField,
    #[error("{0} is a primitive type, therefore has no fields")]
    PrimitiveTypeNoField(Ty),
    #[error("{0} is {mut_0}, while {1} is {mut_1}", mut_0 = describe_mutability(.0.mutable), mut_1 = describe_mutability(.1.mutable))]
    ParamMutability(Param<Ty>, Param<Ty>),
}

fn describe_mutability(mutable: bool) -> &'static str {
    if mutable { "mutable" } else { "immutable" }
}

impl_span!(ErrorKind as Error);
