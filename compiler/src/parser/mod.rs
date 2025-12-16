mod ast;
mod expressions;
mod hierarchy;
#[cfg(test)]
mod test;

use std::error::Error;
use std::fmt::Display;
use std::mem::discriminant;
use std::{iter::Peekable, mem::Discriminant};

use crate::lexer::{Lexer, Token};

type TokenType = Discriminant<Token>;

#[derive(Debug)]
pub enum ParseError {
    NoToken,
    MismatchedToken { expected: TokenType, found: TokenType },
    UnexpectedToken(TokenType)
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::NoToken => write!(f, "expected another token"),
            ParseError::MismatchedToken { expected, found } => 
                write!(f, "expected token {expected:?}, found token {found:?}"),
            ParseError::UnexpectedToken(token) =>
                write!(f, "unexpected token `{token:?}`"),
        }
    }
}

impl Error for ParseError {}

pub struct Parser<I>
where
    I: Iterator<Item = Token>,
{
    tokens: Peekable<I>,
}

impl<'input> Parser<Lexer<'input>> {
    pub fn new(input: &'input str) -> Parser<Lexer<'input>> {
        Parser {
            tokens: Lexer::new(input).peekable(),
        }
    }
}

impl<I: Iterator<Item = Token>> Parser<I> {
    /// Look-ahead one token and see what kind of token it is.
    pub(crate) fn peek(&mut self) -> &Token {
        self.tokens.peek().unwrap_or(&Token::Eof)
    }

    /// Check if the next token is the same variant as another token.
    pub(crate) fn at(&mut self, other: TokenType) -> bool {
        discriminant(self.peek()) == other
    }

    /// Get the next token.
    pub(crate) fn next(&mut self) -> Option<Token> {
        self.tokens.next()
    }

    /// Move forward one token in the input and check
    /// that we pass the kind of token we expect.
    pub(crate) fn consume(&mut self, expected: TokenType) -> Result<(), ParseError> {
        let token = self.next().ok_or(ParseError::NoToken)?;
        if discriminant(&token) != expected {
            Err(ParseError::MismatchedToken { expected, found: discriminant(&token) })
        } else {
            Ok(())
        }
    }
}
