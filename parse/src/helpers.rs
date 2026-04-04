use ast::{exprs::Binding, patterns::Pat};
use ident::Ident;
use lex::{Tok, TokKind};
use span::Span;

use crate::{ErrorKind, Parser, Result, error::Error};

impl<I: Iterator<Item = Tok>> Parser<'_, I> {
    pub fn err_next(&mut self, f: impl Fn(TokKind) -> ErrorKind) -> Error {
        let token = match self.next() {
            Ok(token) => token,
            Err(err) => return err,
        };

        f(token.kind).span(token.span)
    }

    pub fn binding(&mut self) -> Result<Binding> {
        Ok(Binding {
            mutable: self.consume_at(TokKind::Mut),
            pat: self.pattern()?,
            ty: self.ty_annot()?,
        })
    }

    pub fn pattern(&mut self) -> Result<Pat> {
        // TODO add other patterns

        Ok(Pat::Ident {
            ident: self.ident()?.0,
            subpat: None,
        })
    }

    pub fn ident(&mut self) -> Result<(Ident, Span)> {
        self.consume(TokKind::Ident)
            .map(|ident| (Ident::new(self.str_at(ident.span)), ident.span))
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

            if !self.consume_at(TokKind::Comma) {
                break;
            }
        }

        let end = self.consume(end)?.span.end;

        Ok((items, Span::from(start..end)))
    }
}
