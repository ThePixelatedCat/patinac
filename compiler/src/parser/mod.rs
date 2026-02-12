pub mod ast;
mod error;
mod expressions;
mod helpers;
mod items;
#[cfg(test)]
mod test;

use crate::lexer::{Lexer, Token, TokenType};
use std::{iter::{Peekable}, ops::Range, vec::IntoIter};

pub use error::{ParseError, ParseResult};

pub struct Parser<'input, I>
where
    I: Iterator<Item = Token>,
{
    input: &'input str,
    tokens: Peekable<I>,
}

impl<'input> Parser<'input, IntoIter<Token>> {
    pub fn new(input: &'input str) -> Self {
        Parser {
            input,
            tokens: Lexer::lex(input).into_iter().peekable(),
        }
    }
}

impl<'input, I: Iterator<Item = Token>> Parser<'input, I> {
    /// Get the next token.
    fn next(&mut self) -> Option<Token> {
        self.tokens.next()
    }

    /// Look-ahead one token and see what kind of token it is.
    fn peek(&mut self) -> TokenType {
        self.tokens
            .peek()
            .map_or(TokenType::Eof, |token| token.inner)
    }

    /// Check if the next token is the same variant as another token.
    fn at(&mut self, token: TokenType) -> bool {
        self.peek() == token
    }

    /// Move forward one token in the input and check
    /// that we pass the kind of token we expect.
    fn consume(&mut self, expected: TokenType) -> ParseResult<Token> {
        let next = self
            .next()
            .ok_or_else(|| ParseError::Missing.spanned(0..0))?;
        if next.inner == expected {
            Ok(next)
        } else {
            Err(ParseError::Mismatched {
                expected,
                found: next.inner,
            }
            .spanned(next.span))
        }
    }

    fn consume_at(&mut self, token: TokenType) -> bool {
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
