mod error;
mod exprs;
mod helpers;
mod items;
#[cfg(test)]
mod test;
mod types;

use std::{iter::Peekable, ops::Range, result, vec};

use ast::Ast;
#[cfg(any(test, feature = "test"))]
use ast::exprs::Expr;
#[cfg(any(test, feature = "test"))]
use lex::Lexer;
use lex::{Tok, TokKind};

pub use crate::error::{Error, ErrorKind, Result};

use crate::items::Item;

pub struct Parser<'src, I: Iterator<Item = Tok>> {
    src: &'src str,
    toks: Peekable<I>,
}

impl<'src> Parser<'src, vec::IntoIter<Tok>> {
    pub fn parse(src: &'src str, toks: Vec<Tok>) -> result::Result<Ast<()>, Vec<Error>> {
        let mut parser = Self {
            src,
            toks: toks.into_iter().peekable(),
        };

        let mut ast = Ast::default();
        let mut errs = Vec::new();

        while parser.peek().is_ok() {
            match parser.item() {
                Ok(Item::ExecItem(exec_item)) => ast.execs.push(exec_item),
                Ok(Item::AdtItem(adt_item)) => ast.adts.push(adt_item),
                Err(err) => {
                    errs.push(err);
                    parser.skip_until(|tok| {
                        [TokKind::Fn, TokKind::Const, TokKind::Record, TokKind::Enum].contains(&tok)
                    });
                }
            }
        }

        if errs.is_empty() { Ok(ast) } else { Err(errs) }
    }

    #[cfg(any(test, feature = "test"))]
    pub fn parse_expr(src: &'src str) -> Result<Expr<()>> {
        Self {
            src,
            toks: Lexer::lex(src).unwrap().into_iter().peekable(),
        }
        .expr()
    }

    #[cfg(any(test, feature = "test"))]
    pub fn parse_item(src: &'src str) -> Result<Item> {
        Self {
            src,
            toks: Lexer::lex(src).unwrap().into_iter().peekable(),
        }
        .item()
    }
}

impl<'src, I: Iterator<Item = Tok>> Parser<'src, I> {
    /// Get the next token.
    fn next(&mut self) -> Result<Tok> {
        self.toks
            .next()
            .ok_or_else(|| ErrorKind::Eof.span(self.src.len()..self.src.len()))
    }

    /// Look-ahead one token and see what kind of token it is.
    fn peek(&mut self) -> Result<TokKind> {
        self.toks
            .peek()
            .map(|tok| tok.kind)
            .ok_or_else(|| ErrorKind::Eof.span(self.src.len()..self.src.len()))
    }

    /// Check if the next token is the same variant as another token.
    fn at(&mut self, token: TokKind) -> bool {
        self.peek().is_ok_and(|tok| tok == token)
    }

    /// Move forward one token in the input and check
    /// that we pass the kind of token we expect.
    fn consume(&mut self, expected: TokKind) -> Result<Tok> {
        self.next().and_then(|next| {
            if next.kind == expected {
                Ok(next)
            } else {
                Err(ErrorKind::Mismatched {
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

    fn str_at(&self, span: impl Into<Range<usize>>) -> &'src str {
        &self.src[span.into()]
    }
}
