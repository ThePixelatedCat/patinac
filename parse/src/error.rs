use derive_more::Display;
use lex::TokKind;
use span::Span;

pub type Result<T> = errors::Result<T, ErrorKind>;
pub type Error = errors::Error<ErrorKind>;

impl ErrorKind {
    pub fn span(self, span: impl Into<Span>) -> Error {
        Error::new(self, span)
    }
}

#[derive(Debug, Display, PartialEq, Eq, Clone)]
pub enum ErrorKind {
    #[display("expected {expected}, found {found}")]
    Mismatched { expected: TokKind, found: TokKind },
    #[display("Unexpected token {_0}")]
    Unexpected(TokKind),
    #[display("unexpected end of file")]
    Eof,
}
