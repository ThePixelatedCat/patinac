use ast::{Binding, Pat};
use ident::Ident;
use lex::{Tok, TokKind};
use span::{Span, Spannable};

use crate::{ParseError, ParseResult, Parser, error::ParseErrorS};

impl<I: Iterator<Item = Tok>> Parser<'_, I> {
    pub fn err_next(&mut self, f: impl Fn(TokKind) -> ParseError) -> ParseErrorS {
        let token = match self.next() {
            Ok(token) => token,
            Err(err) => return err,
        };

        f(token.kind).span(token.span)
    }

    pub fn binding(&mut self) -> ParseResult<Binding> {
        Ok(Binding {
            mutable: self.consume_at(TokKind::Mut),
            pat: self.pattern()?,
            ty: self.ty_annot()?,
        })
    }

    pub fn pattern(&mut self) -> ParseResult<Pat> {
        // TODO add other patterns

        Ok(Pat::Ident {
            ident: self.ident()?.0,
            subpat: None,
        })
    }

    pub fn ident(&mut self) -> ParseResult<(Ident, Span)> {
        self.consume(TokKind::Ident)
            .map(|ident| (Ident::new(self.str_at(ident.span)), ident.span))
    }

    pub fn delimited_list<T, F>(
        &mut self,
        mut f: F,
        start: TokKind,
        end: TokKind,
    ) -> ParseResult<(Vec<T>, Span)>
    where
        F: FnMut(&mut Self) -> ParseResult<T>,
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
