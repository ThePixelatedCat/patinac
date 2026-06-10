use derive_more::Display;

use errors::SpanError;

use crate::TokKind;

#[derive(Debug, Display, PartialEq, Eq, Clone)]
pub enum ErrorKind {
    #[display("invalid token")]
    BadToken,
    #[display("invalid unicode codepoint")]
    BadUnicodeEscape,
    #[display("expected {expected}, found {found}")]
    Mismatched { expected: TokKind, found: TokKind },
    #[display("unexpected token {_0}")]
    Unexpected(TokKind),
}

impl SpanError for ErrorKind {}
