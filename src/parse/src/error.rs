use derive_more::Display;

use errors::SpanError;

use crate::TokKind;

#[derive(Debug, Display, PartialEq, Eq, Clone)]
pub enum ErrorKind {
    #[display("invalid token")]
    BadToken,
    #[display("unexpected token {_0}")]
    Unexpected(TokKind),
    #[display("expected {expected}, found {found}")]
    Mismatched { expected: TokKind, found: TokKind },
    #[display("`self` must be the first parameter")]
    SelfNotFirst,
    #[display("invalid unicode codepoint")]
    BadUnicodeEscape,
    #[display("primitive type cannot have generic parameters")]
    PrimitiveGenerics,
    #[display("only type and def items can be public")]
    BadPub,
    #[display("only type  items can be opaque")]
    BadOpaque,
}

impl SpanError for ErrorKind {}
