use crate::{span, typecheck::types::Type};

use std::{error::Error, fmt::Display};

span! { TypeError as TypeErrorS }
#[derive(Debug, PartialEq)]
pub enum TypeError {
    UnboundIdent(String),
    MismatchedTypes { expected: Type, found: Type },
    Mutation(String),
    Infinite,
}

impl Display for TypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Self::UnboundIdent(ident) => {
                write!(f, "identifider `{ident}` is unbound")
            }
            Self::MismatchedTypes {
                expected: type_a,
                found: type_b,
            } => write!(f, "found type `{type_b}`, expected type `{type_a}`",),
            Self::Mutation(name) => write!(f, "attempted mutation of immutable variable {name}",),
            Self::Infinite => "infinite cycle of types".fmt(f),
        }
    }
}

impl Error for TypeError {}
