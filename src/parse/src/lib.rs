//! Parses source text into an AST, reporting any errors along the way with as much recovery as possible.
//!
//! The entry point to this crate is the [`Parser`] type.

mod error;
mod exprs;
mod helpers;
mod items;
mod lex;
mod patterns;
#[cfg(test)]
mod test;
mod types;

use ast::{Ast, Expr};
use errors::{ErrorHandler, Result};
use lex::{Lexer, Tok, TokKind};

use crate::{error::ErrorKind, items::Item};

/// Manages the state needing for parsing.
///
/// Construct with [`Parser::new()`], then produce an [`Ast`] (or errors) with [`Parser::parse()`].
pub struct Parser<'src> {
    src: &'src str,
    toks: Lexer<'src>,
    handler: ErrorHandler<'src>,
}

impl<'src> Parser<'src> {
    /// Constructs a [`Parser`] for `src`, reporting errors through `handler`.
    pub fn new(src: &'src str, handler: ErrorHandler<'src>) -> Self {
        Self {
            src,
            toks: lex::lex(src),
            handler,
        }
    }

    /// Parses the source this [`Parser`] was constructed with into an [`Ast`].
    ///
    /// # Errors
    /// If parsing produces any errors, they will be sent to the [`ErrorHandler`] this [`Parser`] was constructed with,
    /// and an error will be returned.
    pub fn parse(mut self) -> Result<Ast> {
        let mut ast = Ast::default();

        while !self.at(TokKind::Eof) {
            match self.item() {
                Ok(Item::ExecItem(exec_item)) => ast.execs.push(exec_item),
                Ok(Item::TyItem(ty_item)) => ast.tys.push(ty_item),
                Err(_) => {}
            }
        }

        self.handler.checked(ast)
    }

    /// Lexes the source and parses an expression in one function call, to simplify tests.
    /// # Errors
    /// If the source cannot be parsed as an expression.
    /// # Panics
    /// If the lexer produces an error.
    #[cfg(any(test, feature = "test"))]
    pub fn parse_expr(src: &'src str) -> Result<Expr> {
        Self::new(src, ErrorHandler::TEST).expr()
    }

    fn src_of(&self, tok: Tok) -> &'src str {
        &self.src[tok.span.start as usize..tok.span.end as usize]
    }

    /// Consumes the next token. Ignores whitespace.
    fn next(&mut self) -> Result<Tok> {
        self.toks
            .next()
            .unwrap_or_else(|| {
                let src_len = u32::try_from(self.src.len()).expect("file too long");
                Ok(TokKind::Eof.span(src_len..src_len))
            })
            .map_err(|span| self.handler.err(ErrorKind::BadToken.span(span)))
            .and_then(|tok| match tok.kind {
                TokKind::Whitespace => self.next(),
                _ => Ok(tok),
            })
    }

    /// Peeks one token. Ignores whitespace.
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

    /// Checks if the next token is of the given kind. Ignores whitespace.
    fn at(&mut self, tok: TokKind) -> bool {
        self.peek().is_ok_and(|t| t == tok)
    }

    /// Checks if the next token is of the given kind. Respects whitespace.
    fn at_ws(&mut self, tok: TokKind) -> bool {
        self.toks
            .peek()
            .copied()
            .transpose()
            .map(|opt_tok| opt_tok.map_or(TokKind::Eof, |tok| tok.kind))
            .map_or_else(
                |span| {
                    self.handler.err(ErrorKind::BadToken.span(span));
                    false
                },
                |t| t == tok,
            )
    }

    /// Consumes the next token and checks that it was the expected kind. Ignores whitespace.
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
        self.at(token)
            .then(|| self.next().expect("known to be at a valid token"))
    }
}
