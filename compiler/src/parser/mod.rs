pub mod ast;
mod error;
mod expressions;
mod helpers;
mod items;
#[cfg(test)]
mod test;

use crate::lexer::{Lexer, Token, TokenType};
use std::iter::Peekable;

pub use error::{ParseError, ParseResult};

pub struct Parser<'input, I>
where
    I: Iterator<Item = Token>,
{
    input: &'input str,
    tokens: Peekable<I>,
}

impl<'input> Parser<'input, Lexer<'input>> {
    pub fn new(input: &'input str) -> Self {
        Parser {
            input,
            tokens: Lexer::new(input).peekable(),
        }
    }
}

impl<I: Iterator<Item = Token>> Parser<'_, I> {
    /// Look-ahead one token and see what kind of token it is.
    pub(crate) fn peek(&mut self) -> TokenType {
        self.tokens
            .peek()
            .map_or(TokenType::Eof, |token| token.inner)
    }

    /// Check if the next token is the same variant as another token.
    pub(crate) fn at(&mut self, token: TokenType) -> bool {
        self.peek() == token
    }

    pub(crate) fn consume_at(&mut self, token: TokenType) -> bool {
        let at = self.at(token);
        if at {
            self.next();
        }
        at
    }

    /// Get the next token.
    pub(crate) fn next(&mut self) -> Option<Token> {
        self.tokens.next()
    }

    /// Move forward one token in the input and check
    /// that we pass the kind of token we expect.
    pub(crate) fn consume(&mut self, expected: TokenType) -> ParseResult<Token> {
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
}
