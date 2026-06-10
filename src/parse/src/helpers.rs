use std::range::Range;

use itertools::Itertools as _;

use ast::{Binding, Path, Ty};
use errors::HandledError;
use ident::{Ident, SpanIdent};

use crate::{ErrorKind, Parser, Result, TokKind};

impl Parser<'_> {
    pub(crate) fn err_next(
        &mut self,
        f: impl Fn(TokKind) -> ErrorKind,
        ctx: &[&'static str],
    ) -> HandledError {
        let token = match self.next() {
            Ok(t) => t,
            Err(e) => return e,
        };
        let mut err = f(token.kind).span(token.span);
        for ctx in ctx {
            err = err.with_static_ctx(ctx);
        }
        self.handler.err(err)
    }

    pub(crate) fn ty_annot(&mut self) -> Result<Option<Ty>> {
        self.consume_at(TokKind::Colon)
            .map(|_| self.ty())
            .transpose()
    }

    pub(crate) fn binding(&mut self) -> Result<Binding> {
        Ok(Binding {
            mutable: self.consume_at(TokKind::Mut).is_some(),
            pat: self.pattern()?,
            ty: self.ty_annot()?,
        })
    }

    pub(crate) fn ident(&mut self) -> Result<SpanIdent> {
        self.consume(TokKind::Ident)
            .map(|tok| Ident::new(self.src_of(tok)).span(tok.span))
    }

    pub(crate) fn path(&mut self) -> Result<(Path, Range<u32>)> {
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

    pub(crate) fn delimited_list<T, F>(
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
