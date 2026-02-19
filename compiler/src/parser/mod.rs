mod error;
mod expressions;
mod helpers;
mod items;
#[cfg(test)]
mod test;
mod types;

use crate::{
    helpers::Spannable,
    lexer::{Lexer, Tok, TokKind},
};
use std::{iter::Peekable, ops::Range, vec::IntoIter};

pub use error::{ParseError, ParseResult};

pub struct Parser<'input, I>
where
    I: Iterator<Item = Tok>,
{
    input: &'input str,
    tokens: Peekable<I>,
}

impl<'input> Parser<'input, IntoIter<Tok>> {
    pub fn new(input: &'input str) -> Self {
        Parser {
            input,
            tokens: Lexer::lex(input).into_iter().peekable(),
        }
    }
}

impl<'input, I: Iterator<Item = Tok>> Parser<'input, I> {
    /// Get the next token.
    fn next(&mut self) -> Option<Tok> {
        self.tokens.next()
    }

    /// Look-ahead one token and see what kind of token it is.
    fn peek(&mut self) -> TokKind {
        self.tokens.peek().map_or(TokKind::Eof, |token| token.kind)
    }

    /// Check if the next token is the same variant as another token.
    fn at(&mut self, token: TokKind) -> bool {
        self.peek() == token
    }

    /// Move forward one token in the input and check
    /// that we pass the kind of token we expect.
    fn consume(&mut self, expected: TokKind) -> ParseResult<Tok> {
        let next = self.next().ok_or_else(|| ParseError::Missing.span(0..0))?;
        if next.kind == expected {
            Ok(next)
        } else {
            Err(ParseError::Mismatched {
                expected,
                found: next.kind,
            }
            .span(next.span))
        }
    }

    fn consume_at(&mut self, token: TokKind) -> bool {
        let at = self.at(token);
        if at {
            self.next();
        }
        at
    }

    fn str_at(&self, span: impl Into<Range<usize>>) -> &'input str {
        &self.input[span.into()]
    }
}
