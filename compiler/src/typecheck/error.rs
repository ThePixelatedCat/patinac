use crate::{span, typecheck::types::TypeS};

use std::{error::Error, fmt::Display};

pub type TypeResult<T = TypeS> = Result<T, TypeErrorS>;

span! { TypeError as TypeErrorS }
#[derive(Debug)]
pub enum TypeError {
    UnboundIdent(String),
    MismatchedTypes(TypeS, TypeS),
    WrongArgCount { needed: usize, provided: usize },
    CantInfer,
    Mutation(String),
    Infinite,
}

impl Display for TypeErrorS {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.inner {
            TypeError::UnboundIdent(ident) => {
                write!(f, "identifider `{ident}` at {} is unbound", self.span)
            }
            TypeError::MismatchedTypes(type_a, type_b) => write!(
                f,
                "mismatched types `{type_a}` and `{type_b}` at {:2}",
                self.span
            ),
            TypeError::WrongArgCount { needed, provided } => write!(
                f,
                "function call at {} has the wrong number of arguments, needs {needed}, provides {provided}",
                self.span
            ),
            TypeError::CantInfer => write!(f, "can't infer type of expression at {}", self.span),
            TypeError::Mutation(name) => write!(
                f,
                "attempted mutation of immutable variable {name} at {}",
                self.span
            ),
            TypeError::Infinite => todo!(),
        }
    }
}

impl Error for TypeErrorS {}
