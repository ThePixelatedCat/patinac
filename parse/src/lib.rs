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
use errors::ErrorHandler;
#[cfg(any(test, feature = "test"))]
use errors::TEST_HANDLER;
use lex::{Tok, TokKind};

pub use crate::error::{Error, ErrorKind, Result};

use crate::items::Item;

pub struct Parser<'src, I: Iterator<Item = Tok<'src>>> {
    toks: Peekable<I>,
    handler: ErrorHandler<'src>,
}

impl<'src> Parser<'src, vec::IntoIter<Tok<'src>>> {
    pub fn new(toks: Vec<Tok<'src>>, handler: ErrorHandler<'src>) -> Self {
        Self {
            toks: toks.into_iter().peekable(),
            handler,
        }
    }

    /// Parses the tokens this was constructed with into an AST
    ///
    /// # Errors
    /// If parsing any expression errors, an error will be returned, with a list of every error that occured.
    /// At most one error will be reported per item
    pub fn parse(mut self) -> Result<Ast> {
        let mut ast = Ast::default();

        while self.try_peek().is_ok() {
            match self.item() {
                Ok(Item::ExecItem(exec_item)) => ast.execs.push(exec_item),
                Ok(Item::AdtItem(adt_item)) => ast.adts.push(adt_item),
                Err(()) => {
                    // // Skip to next item
                    // while let Ok(tok) = self.peek()
                    //     && ![TokKind::Fn, TokKind::Const, TokKind::Record, TokKind::Enum]
                    //         .contains(&tok)
                    // {
                    //     let _ = self.next();
                    // }
                }
            }
        }

        if self.handler.has_err() {
            Err(())
        } else {
            Ok(ast)
        }
    }

    #[cfg(any(test, feature = "test"))]
    pub fn parse_stmt(src: &'src str) -> Result<ast::exprs::Stmt> {
        Self::new(lex::lex(src).unwrap(), TEST_HANDLER).stmt()
    }

    #[cfg(any(test, feature = "test"))]
    pub fn parse_expr(src: &'src str) -> Result<ast::exprs::Expr> {
        Self::new(lex::lex(src).unwrap(), TEST_HANDLER).expr()
    }

    #[cfg(any(test, feature = "test"))]
    pub fn parse_item(src: &'src str) -> Result<Item> {
        Self::new(lex::lex(src).unwrap(), TEST_HANDLER).item()
    }
}

impl<'src, I: Iterator<Item = Tok<'src>>> Parser<'src, I> {
    /// Get the next token.
    fn next(&mut self) -> Result<Tok<'src>> {
        self.toks
            .next()
            .ok_or_else(|| self.handler.err(ErrorKind::Eof.span(0..0)))
    }

    /// Look-ahead one token and see what kind of token it is.
    fn peek(&mut self) -> Result<TokKind> {
        self.toks
            .peek()
            .map(|tok| tok.kind)
            .ok_or_else(|| self.handler.err(ErrorKind::Eof.span(0..0)))
    }

    /// Look-ahead one token and see what kind of token it is, returning any error rather than immediately reporting it.
    fn try_peek(&mut self) -> result::Result<TokKind, Error> {
        self.toks
            .peek()
            .map(|tok| tok.kind)
            .ok_or_else(|| ErrorKind::Eof.span(0..0))
    }

    /// Check if the next token is the same variant as another token.
    fn at(&mut self, token: TokKind) -> bool {
        self.try_peek().is_ok_and(|tok| tok == token)
    }

    /// Move forward one token in the input and check
    /// that we pass the kind of token we expect.
    fn consume(&mut self, expected: TokKind) -> Result<Tok<'src>> {
        self.next().and_then(|next| {
            if next.kind == expected {
                Ok(next)
            } else {
                self.handler.err(
                    ErrorKind::Mismatched {
                        expected,
                        found: next.kind,
                    }
                    .span(next.span),
                );
                Err(())
            }
        })
    }

    fn consume_at(&mut self, token: TokKind) -> Option<Tok<'src>> {
        self.at(token).then(|| self.next().unwrap())
    }
}
