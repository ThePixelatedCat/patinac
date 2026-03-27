use crate::types::Ty;
use span::{Spannable, Spnd};

use std::{error::Error, fmt::Display};

pub type TypeErrorS = Spnd<TypeError>;
impl Spannable for TypeError {}
#[derive(Debug, PartialEq)]
pub enum TypeError {
    UnboundIdent,
    MismatchedTypes { expected: Ty, found: Ty },
    Mutation,
    Infinite,
}

impl Display for TypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Self::UnboundIdent => "unbound identifier".fmt(f),
            Self::MismatchedTypes {
                expected: type_a,
                found: type_b,
            } => write!(f, "found type `{type_b}`, expected type `{type_a}`",),
            Self::Mutation => "attempted mutation of immutable variable".fmt(f),
            Self::Infinite => "infinite cycle of types".fmt(f),
        }
    }
}

impl Error for TypeError {}
