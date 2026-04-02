use crate::types::{Ty, TyVar};
use span::{Spannable, Spnd};

use std::{error::Error, fmt::Display};

pub type TypeErrorS = Spnd<TypeError>;
impl Spannable for TypeError {}
#[derive(Debug, PartialEq, Eq)]
pub enum TypeError {
    UnboundIdent,
    TypesNotEqual(Ty, Ty),
    Mutation,
    Infinite(TyVar, Ty),
    UninferredType,
    NotPlaceExpr,
    UnknownType,
    MissingField,
}

impl Display for TypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Self::UnboundIdent => "unbound identifier".fmt(f),
            Self::TypesNotEqual(lhs, rhs) => write!(f, "types `{lhs}` and `{rhs}` are not equal",),
            Self::Mutation => "attempted mutation of immutable variable".fmt(f),
            Self::Infinite(var, ty) => todo!(),
            Self::UninferredType => "could not infer the type of this expression".fmt(f),
            Self::NotPlaceExpr => {
                "left hand side of an assignment must be a place expression".fmt(f)
            }
            Self::UnknownType => "unknown type".fmt(f),
            Self::MissingField => "field not found".fmt(f),
        }
    }
}

impl Error for TypeError {}
