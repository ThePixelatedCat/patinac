use std::{error::Error, fmt::Display};

use lex::TokKind;
use span::{Spannable, Spnd};

pub type ParseResult<T> = Result<T, ParseErrorS>;

pub type ParseErrorS = Spnd<ParseError>;
impl Spannable for ParseError {}
#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    Mismatched { expected: TokKind, found: TokKind },
    Unexpected(TokKind, &'static str),
    Eof,
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mismatched { expected, found } => {
                write!(f, "expected {expected}, found {found}")
            }
            Self::Unexpected(token, desc) => {
                write!(f, "unexpected token {token} at {desc}")
            }
            Self::Eof => "unexpected end of file".fmt(f),
        }
    }
}

impl Error for ParseError {}
