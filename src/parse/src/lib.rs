//! Parses source text into an AST, reporting any errors along the way with as much recovery as possible.
//!
//! The entry point to this crate is the [`Parser`] type.

mod error;
mod exprs;
mod items;
mod lex;
mod patterns;
#[cfg(test)]
mod test;
mod types;

use std::range::Range;

use itertools::Itertools as _;

use errors::{ErrorHandler, HandledError, Result, SpanError as _};
use ident::{Ident, SpanIdent};
use irs::{
    ModuleId,
    ast::{Ast, Binding, Path, Ty},
};

use crate::{
    error::ErrorKind,
    items::Item,
    lex::{Lexer, Tok, TokKind},
};

/// Manages the state needing for parsing.
///
/// Construct with [`Parser::new()`], then produce an [`Ast`] (or errors) with [`Parser::parse()`].
pub struct Parser<'src> {
    module: ModuleId,
    src: &'src str,
    toks: Lexer<'src>,
    handler: ErrorHandler<'src>,
}

impl<'src> Parser<'src> {
    /// Constructs a [`Parser`] for `src`, reporting errors through `handler`.
    pub fn new(module: ModuleId, src: &'src str, handler: ErrorHandler<'src>) -> Self {
        Self {
            module,
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
                Ok(Item::VisItem(item)) => ast.vis_items.push(item),
                Ok(Item::TyItem(item)) => ast.ty_items.push(item),
                Ok(Item::ExecItem(item)) => ast.exec_items.push(item),
                Ok(Item::Impl(item)) => ast.impls.push(item),
                Err(_) => {}
            }
        }

        self.handler.checked(ast)
    }

    /// Constructs a [`Parser`] for `src`, using testing-suitable defaults for the [`ModuleId`] and [`ErrorHandler`].
    #[cfg(any(test, feature = "test"))]
    pub fn new_test(src: &'src str) -> Self {
        Self::new(ModuleId::default(), src, ErrorHandler::TEST)
    }

    /// Lexes the source and parses an expression in one function call, to simplify tests.
    /// # Errors
    /// If the source cannot be parsed as an expression.
    /// # Panics
    /// If the lexer produces an error.
    #[cfg(any(test, feature = "test"))]
    pub fn parse_expr(src: &'src str) -> Result<irs::ast::Expr> {
        Self::new_test(src).expr()
    }

    fn src_of(&self, tok: Tok) -> &'src str {
        let start = usize::try_from(tok.span.start).expect("why are you on 16bit");
        let end = usize::try_from(tok.span.end).expect("why are you on 16bit");
        &self.src[start..end]
    }

    /// Consumes the next token. Ignores whitespace.
    fn next(&mut self) -> Result<Tok> {
        self.toks
            .next()
            .unwrap_or_else(|| {
                let src_len = u32::try_from(self.src.len()).expect("file too long");
                Ok(TokKind::Eof.span(src_len..src_len))
            })
            .map_err(|span| self.err(ErrorKind::BadToken, span))
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
            .map_err(|span| self.err(ErrorKind::BadToken, span))?
            .map_or(TokKind::Eof, |tok| tok.kind);
        match tok {
            TokKind::Whitespace => {
                // Skip the whitespace and retry.
                self.peek()
            }
            _ => {
                // Reset the repeated peeking.
                self.toks.reset_peek();
                Ok(tok)
            }
        }
    }

    /// Checks if the next token is of the given kind. Ignores whitespace.
    fn at(&mut self, tok: TokKind) -> bool {
        self.peek().is_ok_and(|t| t == tok)
    }

    /// Checks if the next token is of the given kind. Respects whitespace.
    fn at_ws(&mut self, tok: TokKind) -> bool {
        let result = self
            .toks
            .peek()
            .copied()
            .transpose()
            .map(|opt_tok| opt_tok.map_or(TokKind::Eof, |tok| tok.kind))
            .map_or_else(
                |span| {
                    self.err(ErrorKind::BadToken, span);
                    false
                },
                |t| t == tok,
            );
        self.toks.reset_peek();
        result
    }

    /// Consumes the next token and checks that it was the expected kind. Ignores whitespace.
    fn consume(&mut self, expected: TokKind) -> Result<Tok> {
        self.next().and_then(|next| {
            if next.kind == expected {
                Ok(next)
            } else {
                Err(self.err(
                    ErrorKind::Mismatched {
                        expected,
                        found: next.kind,
                    },
                    next.span,
                ))
            }
        })
    }

    fn consume_at(&mut self, token: TokKind) -> Option<Tok> {
        self.at(token)
            .then(|| self.next().expect("known to be at a valid token"))
    }

    fn err(&mut self, error: ErrorKind, span: impl Into<Range<u32>>) -> HandledError {
        self.handler.err(error.span(span, self.module))
    }

    fn err_ctx(
        &mut self,
        error: ErrorKind,
        span: impl Into<Range<u32>>,
        ctx: &[&'static str],
    ) -> HandledError {
        let mut error = error.span(span, self.module);
        for ctx in ctx {
            error = error.with_static_ctx(ctx);
        }
        self.handler.err(error)
    }

    fn err_next(&mut self, f: impl Fn(TokKind) -> ErrorKind, ctx: &[&'static str]) -> HandledError {
        let token = match self.next() {
            Ok(t) => t,
            Err(e) => return e,
        };
        self.err_ctx(f(token.kind), token.span, ctx)
    }

    fn ty_annot(&mut self) -> Result<Option<Ty>> {
        self.consume_at(TokKind::Colon)
            .map(|_| self.ty())
            .transpose()
    }

    fn binding(&mut self) -> Result<Binding> {
        Ok(Binding {
            mutable: self.consume_at(TokKind::Mut).is_some(),
            pat: self.pattern()?,
            ty: self.ty_annot()?,
        })
    }

    fn ident(&mut self) -> Result<SpanIdent> {
        self.consume(TokKind::Ident)
            .map(|tok| Ident::new(self.src_of(tok)).span(tok.span))
    }

    fn path(&mut self) -> Result<(Path, Range<u32>)> {
        let ident = self.ident()?;
        let start = ident.span.start;

        let mut path = Path::single(ident.ident);
        let mut end = ident.span.end;

        while self.consume_at(TokKind::PathSep).is_some() {
            let ident = self.ident()?;
            end = ident.span.end;
            path.push(ident.ident);
        }

        Ok((path, Range::from(start..end)))
    }

    fn delimited_list<T, F>(
        &mut self,
        mut f: F,
        start: TokKind,
        end: TokKind,
    ) -> Result<(Vec<T>, Range<u32>)>
    where
        F: FnMut(&mut Self) -> Result<T>,
    {
        let start = self.consume(start)?.span.start;

        let mut items = Vec::new();
        while !self.at(end) {
            items.push(f(self));

            if self.consume_at(TokKind::Comma).is_none() {
                break;
            }
        }

        let end = self.consume(end)?.span.end;

        Ok((items.into_iter().try_collect()?, Range::from(start..end)))
    }
}
