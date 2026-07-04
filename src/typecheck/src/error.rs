use derive_more::Display;

use errors::SpanError;
use ident::Ident;

use crate::types::PartialTy;

#[derive(Debug, Display, PartialEq, Eq, Clone)]
pub enum ErrorKind {
    #[display("expected `{_1}`, found `{_0}`")]
    TypesNotEqual(PartialTy, PartialTy),
    #[display("infinite (TEMP)")]
    Infinite,
    #[display("could not infer a concrete type for this expression")]
    UninferredExprType,
    #[display("could not infer a concrete type for this variable")]
    UninferredVarType,
    #[display("type `{_0}` does not have a field named `{_1}`")]
    MissingField(PartialTy, Ident),
    #[display("ype {_0} does not have any fields")]
    NoFieldsType(PartialTy),
    #[display("type `{_0}` does not have a method named `{_1}`")]
    MissingMethod(PartialTy, Ident),
}

impl SpanError for ErrorKind {}
