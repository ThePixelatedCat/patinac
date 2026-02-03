use crate::lexer::TokenType;
use std::{error::Error, fmt::Display};

pub type ParseResult<T> = Result<T, ParseError>;

#[derive(Debug, PartialEq)]
pub enum ParseError {
    Mismatched {
        expected: TokenType,
        found: TokenType,
    },
    Unexpected(TokenType, Option<String>),
    Missing,
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mismatched { expected, found } => {
                write!(f, "expected token {expected}, found token {found}")
            }
            Self::Unexpected(token, Some(desc)) => {
                write!(f, "unexpected token `{token}` at {desc}")
            }
            Self::Unexpected(token, None) => write!(f, "unexpected token `{token:?}`"),
            Self::Missing => "expected another token".fmt(f),
        }
    }
}

impl Error for ParseError {}
