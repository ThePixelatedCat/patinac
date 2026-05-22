use std::result;

use derive_more::Display;

use ident::Ident;
use span::Span;

use crate::types::{Param, PartialTy, TyVar};

pub type Result<T> = result::Result<T, ()>;
pub type Error = errors::Error<ErrorKind>;

impl ErrorKind {
    pub fn span(self, span: impl Into<Span>) -> Error {
        Error::new(self, span)
    }
}

#[derive(Debug, Display, PartialEq, Eq, Clone)]
pub enum ErrorKind {
    #[display("expected `{_1}`, found `{_0}`")]
    TypesNotEqual(PartialTy, PartialTy),
    #[display("attempted mutation of immutable place")]
    Mutation,
    #[display("infinite (TEMP)")]
    Infinite(TyVar, PartialTy),
    #[display("could not infer a concrete type for this expression")]
    UninferredExprType,
    #[display("could not infer a concrete type for this variable")]
    UninferredVarType,
    #[display("the left hand side of an assignment must be a place expression")]
    NotPlaceExpr,
    #[display("this mutable place overlaps with {_0}")]
    OverlappingPlace(Span),
    #[display("type `{_0}` does not have a field named `{_1}`")]
    MissingField(PartialTy, Ident),
    #[display("{_0} is a primitive type, therefore has no fields")]
    PrimitiveTypeNoField(PartialTy),
    #[display("mismatched parameter count between type `{_0}` and type `{_1}`")]
    ParamCount(PartialTy, PartialTy),
    #[display("mismatched parameter mutability between parameters `{_0}` and `{_1}`")]
    ParamMutability(Param, Param),
}
