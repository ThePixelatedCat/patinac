mod error;
mod exprs;
mod helpers;
mod items;
mod patterns;
#[cfg(test)]
mod test;
mod types;

use std::{iter::Peekable, result, vec};

use ast::Ast;
use ident::{Ident, SpanIdent};
use lex::{Tok, TokKind};

pub use crate::error::{Error, ErrorKind, Result};

use crate::items::Item;

pub struct Parser<'src, I: Iterator<Item = Tok<'src>>> {
    toks: Peekable<I>,
}

impl<'src> Parser<'src, vec::IntoIter<Tok<'src>>> {
    pub fn new(toks: Vec<Tok<'src>>) -> Self {
        Self {
            toks: toks.into_iter().peekable(),
        }
    }

    pub fn parse(mut self) -> result::Result<Ast<(), SpanIdent, Ident>, Vec<Error>> {
        let mut ast = Ast::default();
        let mut errs = Vec::new();

        while self.peek().is_ok() {
            match self.item() {
                Ok(Item::ExecItem(exec_item)) => ast.execs.push(exec_item),
                Ok(Item::AdtItem(adt_item)) => ast.adts.push(adt_item),
                Err(err) => {
                    errs.push(err);
                    // Skip to next item
                    while let Ok(tok) = self.peek()
                        && ![TokKind::Fn, TokKind::Const, TokKind::Record, TokKind::Enum]
                            .contains(&tok)
                    {
                        let _ = self.next();
                    }
                }
            }
        }

        if errs.is_empty() { Ok(ast) } else { Err(errs) }
    }

    #[cfg(any(test, feature = "test"))]
    pub fn parse_stmt(src: &'src str) -> Result<ast::exprs::Stmt<(), SpanIdent, Ident>> {
        Self::new(lex::lex(src).unwrap()).stmt()
    }

    #[cfg(any(test, feature = "test"))]
    pub fn parse_expr(src: &'src str) -> Result<ast::exprs::Expr<(), SpanIdent, Ident>> {
        Self::new(lex::lex(src).unwrap()).expr()
    }

    #[cfg(any(test, feature = "test"))]
    pub fn parse_item(src: &'src str) -> Result<Item> {
        Self::new(lex::lex(src).unwrap()).item()
    }
}

impl<'src, I: Iterator<Item = Tok<'src>>> Parser<'src, I> {
    /// Get the next token.
    fn next(&mut self) -> Result<Tok<'src>> {
        self.toks.next().ok_or_else(|| ErrorKind::Eof.span(0..0))
    }

    /// Look-ahead one token and see what kind of token it is.
    fn peek(&mut self) -> Result<TokKind> {
        self.toks
            .peek()
            .map(|tok| tok.kind)
            .ok_or_else(|| ErrorKind::Eof.span(0..0))
    }

    /// Check if the next token is the same variant as another token.
    fn at(&mut self, token: TokKind) -> bool {
        self.peek().is_ok_and(|tok| tok == token)
    }

    /// Move forward one token in the input and check
    /// that we pass the kind of token we expect.
    fn consume(&mut self, expected: TokKind) -> Result<Tok<'src>> {
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

    fn consume_at(&mut self, token: TokKind) -> Option<Tok<'src>> {
        self.at(token).then(|| self.next().unwrap())
    }
}
