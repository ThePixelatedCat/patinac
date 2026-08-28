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

use errors::{ErrorHandler, HandledError, Result};
use ident::{Ident, SpanIdent};
use irs::{
    ModuleId,
    ast::{Ast, Binding, Path, Ty},
};

use crate::{
    error::ErrorKind,
    items::Item,
    lex::{Tok, TokKind},
};

/// Manages the state needing for parsing.
///
/// Construct with [`Parser::new()`], then produce an [`Ast`] (or errors) with [`Parser::parse()`].
pub struct Parser<'src> {
    module: ModuleId,
    handler: ErrorHandler<'src>,
    src: &'src str,
    toks: Vec<Result<Tok, Range<u32>>>,
    pos: usize,
}

impl<'src> Parser<'src> {
    /// Constructs a [`Parser`] for `src`, reporting errors through `handler`.
    pub fn new(module: ModuleId, src: &'src str, handler: ErrorHandler<'src>) -> Self {
        Self {
            module,
            handler,
            src,
            toks: lex::lex(src),
            pos: 0,
        }
    }

    /// Constructs a [`Parser`] for `src`, using testing-suitable defaults for the [`ModuleId`] and [`ErrorHandler`].
    #[cfg(test)]
    pub fn new_test(src: &'src str) -> Self {
        Self::new(ModuleId::default(), src, ErrorHandler::test())
    }

    /// Parses the source this [`Parser`] was constructed with into an [`Ast`].
    ///
    /// # Errors
    /// If parsing produces any errors, they will be sent to the [`ErrorHandler`] this [`Parser`] was constructed with,
    /// and an error will be returned.
    pub fn parse(mut self) -> Result<Ast> {
        let mut ast = Ast::default();

        while !self.peek().is_ok_and(|tok| tok.kind == TokKind::Eof) {
            match self.item() {
                Ok(Item::Import(item)) => ast.imports.push(item),
                Ok(Item::TyItem(item)) => ast.ty_items.push(item),
                Ok(Item::DefItem(item)) => ast.def_items.push(item),
                Ok(Item::BlockItem(item)) => ast.block_items.push(item),
                Err(_) => self.skip_to(&[
                    TokKind::Import,
                    TokKind::Pub,
                    TokKind::Opaque,
                    TokKind::Type,
                    TokKind::Def,
                    TokKind::Impl,
                ]),
            }
        }

        self.handler.checked(ast)
    }

    fn src_of(&self, tok: Tok) -> &'src str {
        let start = usize::try_from(tok.span.start).expect("why are you on 16bit");
        let end = usize::try_from(tok.span.end).expect("why are you on 16bit");
        &self.src[start..end]
    }

    fn get_tok(&self, pos: usize) -> Result<Tok> {
        self.toks
            .get(pos)
            .copied()
            .unwrap_or_else(|| {
                let src_len = u32::try_from(self.src.len()).expect("file too long") - 1;
                Ok(TokKind::Eof.span(src_len..src_len))
            })
            .map_err(|span| self.err(ErrorKind::BadToken, span))
    }

    /// Consumes the next token. Ignores whitespace.
    fn next(&mut self) -> Result<Tok> {
        let mut tok = self.get_tok(self.pos)?;
        self.pos += 1;
        while tok.kind == TokKind::Whitespace {
            tok = self.get_tok(self.pos)?;
            self.pos += 1;
        }
        debug_assert_ne!(
            tok.kind,
            TokKind::Whitespace,
            "`next` should never return a whitespace token"
        );
        Ok(tok)
    }

    /// Peeks the current token. Ignores whitespace.
    fn peek(&self) -> Result<Tok> {
        let mut tok = self.get_tok(self.pos)?;
        let mut offset = 1;
        while tok.kind == TokKind::Whitespace {
            tok = self.get_tok(self.pos + offset)?;
            offset += 1;
        }
        debug_assert_ne!(
            tok.kind,
            TokKind::Whitespace,
            "`peek` should never return a whitespace token"
        );
        Ok(tok)
    }

    // Peeks the current token. Respects whitespace.
    fn peek_ws(&self) -> Result<Tok> {
        self.get_tok(self.pos)
    }

    /// Checks if the next token is of the given kind. Ignores whitespace.
    fn at(&self, tok: TokKind) -> bool {
        self.peek().is_ok_and(|t| t.kind == tok)
    }

    /// Checks if the next token is of the given kind. Respects whitespace.
    fn at_ws(&self, tok: TokKind) -> bool {
        self.peek_ws().is_ok_and(|t| t.kind == tok)
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
        self.at(token).then(|| {
            self.next()
                .expect("known to be at a valid token because `at` returned true")
        })
    }

    /// Consumes tokens up until any of the provided kinds, or until end-of-file.
    fn skip_to(&mut self, kinds: &[TokKind]) {
        while !self
            .peek()
            .is_ok_and(|tok| tok.kind == TokKind::Eof || kinds.contains(&tok.kind))
        {
            self.pos += 1;
        }
    }

    fn err(&self, error: ErrorKind, span: Range<u32>) -> HandledError {
        self.handler.report(error, span, self.module)
    }

    fn unexpected(&mut self, msg: Option<&'static str>) -> HandledError {
        let token = match self.next() {
            Ok(t) => t,
            Err(e) => return e,
        };
        self.err(ErrorKind::Unexpected(token.kind, msg), token.span)
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
