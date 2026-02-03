use crate::{span, typecheck::types::Type};

use std::{error::Error, fmt::Display};

span! { TypeError as TypeErrorS }
#[derive(Debug, PartialEq)]
pub enum TypeError {
    UnboundIdent(String),
    MismatchedTypes { expected: Type, found: Type },
    CantInfer,
    Mutation(String),
    Infinite,
}

impl Display for TypeErrorS {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.inner {
            TypeError::UnboundIdent(ident) => {
                write!(f, "{}: identifider `{ident}` is unbound", self.span)
            }
            TypeError::MismatchedTypes {
                expected: type_a,
                found: type_b,
            } => write!(
                f,
                "{}: found type `{type_b}`, expected type `{type_a}`",
                self.span
            ),
            TypeError::CantInfer => write!(f, "can't infer type of expression at {}", self.span),
            TypeError::Mutation(name) => write!(
                f,
                "{}: attempted mutation of immutable variable {name}",
                self.span
            ),
            TypeError::Infinite => todo!(),
        }
    }
}

impl Error for TypeErrorS {}
