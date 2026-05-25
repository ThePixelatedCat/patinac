use derive_more::Display;

use errors::Error;
use ident::Ident;
use span::Span;

use crate::types::{Param, PartialTy, TyVar};

#[derive(Debug, Display, PartialEq, Eq, Clone)]
pub enum ErrorKind {
    #[display("expected `{_1}`, found `{_0}`")]
    TypesNotEqual(PartialTy, PartialTy),
    #[display("infinite (TEMP)")]
    Infinite(TyVar, PartialTy),
    #[display("could not infer a concrete type for this expression")]
    UninferredExprType,
    #[display("could not infer a concrete type for this variable")]
    UninferredVarType,
    #[display("type `{_0}` does not have a field named `{_1}`")]
    MissingField(PartialTy, Ident),
    #[display("ype {_0} does not have any fields")]
    NoFieldsType(PartialTy),
    #[display("mismatched parameter count between type `{_0}` and type `{_1}`")]
    ParamCount(PartialTy, PartialTy),
    #[display("mismatched parameter mutability between parameters `{_0}` and `{_1}`")]
    ParamMutability(Param, Param),
}

impl ErrorKind {
    pub fn span(self, span: impl Into<Span>) -> Error<Self> {
        Error::new(self, span)
    }
}
