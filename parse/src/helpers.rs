use std::ops::Range;

use ast::{Binding, Ident, Pat};
use lex::{Tok, TokKind};
use span::{Span, Spannable, Spnd};

use crate::{ParseError, ParseResult, Parser};

impl<I: Iterator<Item = Tok>> Parser<'_, I> {
    pub fn err_next(&mut self, f: impl Fn(TokKind) -> ParseError) -> Spnd<ParseError> {
        let token = self.next().unwrap();

        f(token.kind).span(token.span)
    }

    pub fn binding(&mut self) -> ParseResult<Binding> {
        Ok(Binding {
            pat: self.pattern()?,
            ty: self.ty_annot()?,
        })
    }

    pub fn pattern(&mut self) -> ParseResult<Pat> {
        // TODO add other patterns

        let mutable = self.consume_at(TokKind::Mut);

        let Spnd(ident, _) = self.ident()?;

        Ok(Pat::Var { mutable, ident })
    }

    pub fn ident(&mut self) -> ParseResult<Spnd<Ident>> {
        let next = self.next().unwrap();

        match next.kind {
            TokKind::Ident => {
                let string = self.input[Range::from(next.span)].to_string();

                Ok(Spnd::span(
                    self.interner.get_or_intern(string).into(),
                    next.span,
                ))
            }
            other => Err(ParseError::Mismatched {
                expected: TokKind::Ident,
                found: other,
            }
            .span(next.span)),
        }
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
        self.strip(TokKind::Indent);

        let mut items = Vec::new();
        while !self.at(end) {
            items.push(f(self)?);

            if !self.consume_at(TokKind::Comma) {
                break;
            }
        }
        self.strip(TokKind::Dedent);
        let end = self.consume(end)?.span.end;

        Ok((items, Span::from(start..end)))
    }
}
