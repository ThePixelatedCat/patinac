use std::ops::Range;

use crate::{
    ast::Pattern,
    helpers::{Span, Spannable, Spnd},
    lexer::{Tok, TokKind},
};

use super::{ParseError, ParseResult, Parser};

impl<I: Iterator<Item = Tok>> Parser<'_, I> {
    pub fn err_next(&mut self, f: impl Fn(TokKind) -> ParseError) -> Spnd<ParseError> {
        let token = self.next().unwrap();

        f(token.kind).span(token.span)
    }

    pub fn pattern(&mut self) -> ParseResult<Pattern> {
        let mutable = self.consume_at(TokKind::Mut);

        let (ident, _) = self.ident()?;

        let ty_annotation = self.ty_annot()?;

        Ok(Pattern::Var {
            mutable,
            ident,
            ty_annotation,
        })
    }

    pub fn ident(&mut self) -> ParseResult<(String, Span)> {
        let next = self.next().unwrap();

        match next.kind {
            TokKind::Ident => Ok((self.input[Range::from(next.span)].to_string(), next.span)),
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
