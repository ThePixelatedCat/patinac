mod error;
mod exprs;
mod helpers;
mod items;
mod lex;
mod patterns;
#[cfg(test)]
mod test;
mod types;

use std::ops::Range;

use ast::Ast;
use errors::{ErrorHandler, Result};
use lex::{Lexer, Tok, TokKind};

use crate::{error::ErrorKind, items::Item};

pub struct Parser<'src> {
    src: &'src str,
    toks: Lexer<'src>,
    handler: ErrorHandler<'src>,
}

impl<'src> Parser<'src> {
    pub fn new(src: &'src str, handler: ErrorHandler<'src>) -> Self {
        Self {
            src,
            toks: lex::lex(src),
            handler,
        }
    }

    /// Parses the source this was constructed with into an AST
    ///
    /// # Errors
    /// If parsing produces any errors, an error will be returned, but only after the rest of parsing is complete
    pub fn parse(mut self) -> Result<Ast> {
        let mut ast = Ast::default();

        while !self.at(TokKind::Eof) {
            match self.item() {
                Ok(Item::ExecItem(exec_item)) => ast.execs.push(exec_item),
                Ok(Item::AdtItem(adt_item)) => ast.adts.push(adt_item),
                Err(_) => {}
            }
        }

        self.handler.checked(ast)
    }

    /// Lexes the source and parses an expression in one function call, to simplify tests
    /// # Errors
    /// If the source cannot be parsed as an expression
    /// # Panics
    /// If the lexer produces an error
    #[cfg(any(test, feature = "test"))]
    pub fn parse_expr(src: &'src str) -> Result<ast::exprs::Expr> {
        Self::new(src, errors::TEST_HANDLER).expr()
    }

    fn src_of(&self, tok: Tok) -> &'src str {
        &self.src[Range::from(tok.span)]
    }

    /// Get the next token
    ///
    /// Ignores whitespace
    fn next(&mut self) -> Result<Tok> {
        self.toks
            .next()
            .unwrap_or_else(|| Ok(TokKind::Eof.span(self.src.len()..self.src.len())))
            .map_err(|span| self.handler.err(ErrorKind::BadToken.span(span)))
            .and_then(|tok| match tok.kind {
                TokKind::Whitespace => self.next(),
                _ => Ok(tok),
            })
    }

    /// Look-ahead one token
    ///
    /// Ignores whitespace
    fn peek(&mut self) -> Result<TokKind> {
        let tok = self
            .toks
            .peek()
            .copied()
            .transpose()
            .map_err(|span| self.handler.err(ErrorKind::BadToken.span(span)))?
            .map_or(TokKind::Eof, |tok| tok.kind);
        match tok {
            TokKind::Whitespace => {
                // Skip the whitespace and retry
                self.toks.next();
                self.peek()
            }
            _ => Ok(tok),
        }
    }

    /// Check if the next token is the same variant as another token
    ///
    /// Ignores whitespace
    fn at(&mut self, tok: TokKind) -> bool {
        self.peek() == Ok(tok)
    }

    /// Check if the next token is the same variant as another token
    ///
    /// Respects whitespace
    fn at_ws(&mut self, token: TokKind) -> bool {
        match self.toks.peek() {
            None => false,
            Some(Err(span)) => {
                self.handler.err(ErrorKind::BadToken.span(*span));
                false
            }
            Some(Ok(tok)) => tok.kind == token,
        }
    }

    /// Move forward one token in the input and check that we pass the kind of token we expect
    ///
    /// Ignores whitespace
    fn consume(&mut self, expected: TokKind) -> Result<Tok> {
        self.next().and_then(|next| {
            if next.kind == expected {
                Ok(next)
            } else {
                Err(self.handler.err(
                    ErrorKind::Mismatched {
                        expected,
                        found: next.kind,
                    }
                    .span(next.span),
                ))
            }
        })
    }

    fn consume_at(&mut self, token: TokKind) -> Option<Tok> {
        self.at(token).then(|| self.next().unwrap())
    }
}
