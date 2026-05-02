use ast::exprs::Binding;
use ident::{Ident, SpanIdent};
use lex::{Tok, TokKind};
use span::Span;

use crate::{Error, ErrorKind, Parser, Result};

impl<'src, I: Iterator<Item = Tok<'src>>> Parser<'src, I> {
    pub fn err_next(&mut self, f: impl Fn(TokKind) -> ErrorKind) -> Error {
        let token = match self.next() {
            Ok(token) => token,
            Err(err) => return err,
        };

        f(token.kind).span(token.span)
    }

    pub fn binding(&mut self) -> Result<Binding<Ident, Ident>> {
        Ok(Binding {
            mutable: self.consume_at(TokKind::Mut).is_some(),
            pat: self.pattern()?,
            ty: self.ty_annot()?,
        })
    }

    pub fn ident(&mut self) -> Result<SpanIdent> {
        self.consume(TokKind::Ident).map(|ident| SpanIdent {
            ident: Ident::new(ident.src),
            span: ident.span,
        })
    }

    pub fn delimited_list<T, F>(
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
            items.push(f(self)?);

            if self.consume_at(TokKind::Comma).is_none() {
                break;
            }
        }

        let end = self.consume(end)?.span.end;

        Ok((items, Span::from(start..end)))
    }
}
