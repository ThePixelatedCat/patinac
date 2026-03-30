mod error;
mod exprs;
mod helpers;
mod items;
#[cfg(test)]
mod test;
mod types;

use std::{iter::Peekable, ops::Range, vec::IntoIter};

use string_interner::DefaultStringInterner;

use ast::{Ast, Ident};
use lex::{Lexer, Tok, TokKind};
use span::Spannable;

pub use error::{ParseError, ParseResult};

use crate::items::Item;

pub struct Parser<'input, I>
where
    I: Iterator<Item = Tok>,
{
    input: &'input str,
    interner: &'input mut DefaultStringInterner,
    tokens: Peekable<I>,
}

impl<'input> Parser<'input, IntoIter<Tok>> {
    pub fn new(input: &'input str, interner: &'input mut DefaultStringInterner) -> Self {
        Parser {
            input,
            interner,
            tokens: Lexer::lex(input).into_iter().peekable(),
        }
    }
}

impl<'input, I: Iterator<Item = Tok>> Parser<'input, I> {
    pub fn parse(&mut self) -> ParseResult<Ast<()>> {
        let mut ast = Ast::default();
        while self.peek().is_ok() {
            match self.item()? {
                Item::ExecItem(exec_item) => ast.execs.push(exec_item),
                Item::AdtItem(adt_item) => ast.adts.push(adt_item),
            }
        }
        Ok(ast)
    }

    pub fn get_interned(&mut self, name: &str) -> Ident {
        self.interner.get_or_intern(name).into()
    }

    /// Get the next token.
    fn next(&mut self) -> ParseResult<Tok> {
        self.tokens
            .next()
            .ok_or_else(|| ParseError::Eof.span(self.input.len()..self.input.len()))
    }

    /// Look-ahead one token and see what kind of token it is.
    fn peek(&mut self) -> ParseResult<TokKind> {
        self.tokens
            .peek()
            .map(|tok| tok.kind)
            .ok_or_else(|| ParseError::Eof.span(self.input.len()..self.input.len()))
    }

    /// Check if the next token is the same variant as another token.
    fn at(&mut self, token: TokKind) -> bool {
        self.peek().is_ok_and(|tok| tok == token)
    }

    /// Move forward one token in the input and check
    /// that we pass the kind of token we expect.
    fn consume(&mut self, expected: TokKind) -> ParseResult<Tok> {
        self.next().and_then(|next| {
            if next.kind == expected {
                Ok(next)
            } else {
                Err(ParseError::Mismatched {
                    expected,
                    found: next.kind,
                }
                .span(next.span))
            }
        })
    }

    fn consume_get_at(&mut self, token: TokKind) -> Option<Tok> {
        self.at(token).then(|| self.next().unwrap())
    }

    fn consume_at(&mut self, token: TokKind) -> bool {
        let at = self.at(token);
        if at {
            self.next().unwrap();
        }
        at
    }

    fn str_at(&self, span: impl Into<Range<usize>>) -> &'input str {
        &self.input[span.into()]
    }
}
