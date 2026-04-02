mod error;
mod exprs;
mod helpers;
mod items;
#[cfg(test)]
mod test;
mod types;

use std::{iter::Peekable, ops::Range};

use ast::Ast;
use lex::{Tok, TokKind};
use span::Spannable;

pub use error::{ParseError, ParseResult};

use crate::{error::ParseErrorS, items::Item};

pub struct Parser<'input, I>
where
    I: Iterator<Item = Tok>,
{
    input: &'input str,
    tokens: Peekable<I>,
}

impl<'input, I: Iterator<Item = Tok>> Parser<'input, I> {
    pub fn new(input: &'input str, tokens: Peekable<I>) -> Self {
        Self { input, tokens }
    }
}

impl<'input, I: Iterator<Item = Tok>> Parser<'input, I> {
    pub fn parse(&mut self) -> Result<Ast<()>, Vec<ParseErrorS>> {
        let mut ast = Ast::default();
        let mut errs = Vec::new();

        while self.peek().is_ok() {
            match self.item() {
                Ok(Item::ExecItem(exec_item)) => ast.execs.push(exec_item),
                Ok(Item::AdtItem(adt_item)) => ast.adts.push(adt_item),
                Err(err) => {
                    errs.push(err);
                    self.skip_until(|tok| {
                        [TokKind::Fn, TokKind::Const, TokKind::Record, TokKind::Enum].contains(&tok)
                    });
                }
            }
        }

        if errs.is_empty() { Ok(ast) } else { Err(errs) }
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

    fn skip_until(&mut self, pred: impl Fn(TokKind) -> bool) {
        while let Ok(tok) = self.peek()
            && !pred(tok)
        {
            let _ = self.next();
        }
    }

    fn str_at(&self, span: impl Into<Range<usize>>) -> &'input str {
        &self.input[span.into()]
    }
}
