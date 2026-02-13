use crate::{helpers::{Spannable, Spnd}, lexer::TT};
use std::{error::Error, fmt::Display};

pub type ParseResult<T> = Result<T, ParseErrorS>;

pub type ParseErrorS = Spnd<ParseError>;
impl Spannable for ParseError {}
#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    Mismatched {
        expected: TT,
        found: TT,
    },
    Unexpected(TT, &'static str),
    Missing,
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mismatched { expected, found } => {
                write!(f, "expected token {expected}, found token {found}")
            }
            Self::Unexpected(token, desc) => {
                write!(f, "unexpected token `{token}` at {desc}")
            }
            Self::Missing => "expected another token".fmt(f),
        }
    }
}

impl Error for ParseError {}
