use ast::{exprs::Binding, types::Ty};
use ident::{Ident, SpanIdent};
use itertools::Itertools;
use lex::{Tok, TokKind};
use span::Span;

use crate::{ErrorKind, Parser, Result};

impl<'src, I: Iterator<Item = Tok<'src>>> Parser<'src, I> {
    pub(crate) fn err_next(&mut self, f: impl Fn(TokKind) -> ErrorKind, ctx: &[&'static str]) {
        let Ok(token) = self.next() else { return };
        let mut err = f(token.kind).span(token.span);
        for ctx in ctx {
            err.add_static_ctx(ctx);
        }
        self.err(err);
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
        self.consume(TokKind::Ident).map(|tok| SpanIdent {
            ident: Ident::new(tok.src),
            span: tok.span,
        })
    }

    pub(crate) fn delimited_list<T, F>(
        &mut self,
        mut f: F,
        start: TokKind,
        end: TokKind,
    ) -> Result<(Vec<T>, Span)>
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

        Ok((items.into_iter().try_collect()?, Span::from(start..end)))
    }
}
