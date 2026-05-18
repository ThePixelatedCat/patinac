use derive_more::Display;

use span::Span;

use crate::types::{Param, PartialTy, Return, TyVar};

pub type Result<T> = errors::Result<T, ErrorKind>;
pub type Error = errors::Error<ErrorKind>;

impl ErrorKind {
    pub fn span(self, span: impl Into<Span>) -> Error {
        Error::new(self, span)
    }
}

#[derive(Debug, Display, PartialEq, Eq, Clone)]
pub enum ErrorKind {
    #[display("types `{_0}` and `{_1}` are not equal")]
    TypesNotEqual(PartialTy, PartialTy),
    #[display("attempted mutation of immutable place")]
    Mutation,
    #[display("infinite (TEMP)")]
    Infinite(TyVar, PartialTy),
    #[display("could not infer the type of the expression")]
    UninferredType,
    #[display("left hand side of an assignment must be a place expression")]
    NotPlaceExpr,
    #[display("mutable place overlaps with {_0}")]
    OverlappingPlace(Span),
    #[display("field not found")]
    MissingField,
    #[display("{_0} is a primitive type, therefore has no fields")]
    PrimitiveTypeNoField(PartialTy),
    #[display("mismatched parameter count between type `{_0}` and type `{_1}`")]
    ParamCount(PartialTy, PartialTy),
    #[display("mismatched parameter mutability between `{_0}` and `{_1}`")]
    ParamMutability(Param, Param),
    #[display("mismatched return type mutability between `{_0}` and `{_1}`")]
    ReturnMutability(Return, Return),
}
