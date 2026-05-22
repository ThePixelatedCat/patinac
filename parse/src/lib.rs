mod error;
mod exprs;
mod helpers;
mod items;
mod lex;
mod patterns;
#[cfg(test)]
mod test;
mod types;

use ast::Ast;
use errors::{ErrorHandler, Result};
use lex::{Lexer, Tok, TokKind};

use crate::{error::ErrorKind, items::Item};

pub struct Parser<'src> {
    toks: Lexer<'src>,
    handler: ErrorHandler<'src>,
}

impl<'src> Parser<'src> {
    pub fn new(src: &'src str, handler: ErrorHandler<'src>) -> Self {
        Self {
            toks: lex::lex(src),
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

        while self.try_peek().is_some() {
            match self.item() {
                Ok(Item::ExecItem(exec_item)) => ast.execs.push(exec_item),
                Ok(Item::AdtItem(adt_item)) => ast.adts.push(adt_item),
                Err(_) => {
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

    /// Get the next token, producing an error if we're at EOF
    fn next(&mut self) -> Result<Tok<'src>> {
        self.toks
            .next()
            .ok_or_else(|| self.handler.err(ErrorKind::Eof.span(0..0)))?
            .map_err(|span| self.handler.err(ErrorKind::BadToken.span(span)))
    }

    /// Look-ahead one token and see what kind of token it is, producing an error if we're at EOF
    fn peek(&mut self) -> Result<TokKind> {
        self.toks
            .peek()
            .ok_or_else(|| self.handler.err(ErrorKind::Eof.span(0..0)))?
            .map_err(|span| self.handler.err(ErrorKind::BadToken.span(span)))
            .map(|tok| tok.kind)
    }

    /// Look-ahead one token and see what kind of token it is, returning None if we're at EOF
    fn try_peek(&mut self) -> Option<TokKind> {
        self.toks
            .peek()?
            .map_err(|span| self.handler.err(ErrorKind::BadToken.span(span)))
            .ok()
            .map(|tok| tok.kind)
    }

    /// Check if the next token is the same variant as another token.
    fn at(&mut self, token: TokKind) -> bool {
        self.try_peek().is_some_and(|tok| tok == token)
    }

    /// Move forward one token in the input and check
    /// that we pass the kind of token we expect.
    fn consume(&mut self, expected: TokKind) -> Result<Tok<'src>> {
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

    fn consume_at(&mut self, token: TokKind) -> Option<Tok<'src>> {
        self.at(token).then(|| self.next().unwrap())
    }
}
