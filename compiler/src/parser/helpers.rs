use std::ops::Range;

use crate::{
    helpers::{Spannable, Spnd},
    lexer::{TT, Token},
};

use super::{
    ParseError, ParseResult, Parser,
    ast::{Pattern, PatternS, Type, TypeS},
};

impl<I: Iterator<Item = Token>> Parser<'_, I> {
    pub fn pattern(&mut self) -> ParseResult<PatternS> {
        let mutable = self.at(TT::Mut);
        let mut_start = mutable.then(|| self.next().unwrap().span.start);

        let ident = self.ident()?;

        let start = mut_start.unwrap_or(ident.span.start);

        let type_annotation = if self.consume_at(TT::Colon) {
            Some(self.parse_ty()?)
        } else {
            None
        };

        let end = type_annotation
            .as_ref()
            .map_or(ident.span.end, |ty| ty.span.end);

        Ok(Pattern::Var {
            mutable,
            ident: ident.inner,
            annotated_ty: type_annotation,
        }
        .span(start..end))
    }

    pub fn parse_ty(&mut self) -> ParseResult<TypeS> {
        Ok(match self.peek() {
            TT::Int => Type::Int.span(self.next().unwrap().span),
            TT::UInt => Type::UInt.span(self.next().unwrap().span),
            TT::Byte => Type::Byte.span(self.next().unwrap().span),
            TT::Float => Type::Float.span(self.next().unwrap().span),
            TT::Bool => Type::Bool.span(self.next().unwrap().span),
            TT::Char => Type::Char.span(self.next().unwrap().span),
            TT::Ident => {
                let span = self.next().unwrap().span;
                let name = self.input[Range::from(span)].to_string();

                let start = span.start;

                let (generics, end) = if self.at(TT::LAngle) {
                    let Spnd {
                        inner: generics,
                        span: generics_span,
                    } =
                        self.delimited_list(Self::parse_ty, TT::LAngle, TT::RAngle)?;
                    (generics, generics_span.end)
                } else {
                    (Vec::new(), span.end)
                };

                Type::Named {
                    name,
                    args: generics,
                }
                .span(start..end)
            }
            TT::LBracket => {
                let start = self.next().unwrap().span.start;

                let inner_type = self.parse_ty()?;

                let end = self.consume(TT::RBracket)?.span.end;

                Type::Array(Box::new(inner_type)).span(start..end)
            }
            TT::LParen => {
                let Spnd { inner: types, span } =
                    self.delimited_list(Self::parse_ty, TT::LParen, TT::RParen)?;
                Type::Tuple(types).span(span)
            }
            TT::Fn => {
                let start = self.next().unwrap().span.start;

                let Spnd { inner: params, .. } =
                    self.delimited_list(Self::parse_ty, TT::LParen, TT::RParen)?;

                self.consume(TT::Colon)?;
                let result = Box::new(self.parse_ty()?);

                let end = result.span.end;

                Type::Fn(params, result).span(start..end)
            }
            _ => {
                let token = self.next().unwrap();

                return Err(
                    ParseError::Unexpected(token.inner, "start of type name").span(token.span)
                );
            }
        })
    }

    pub fn ident(&mut self) -> ParseResult<Spnd<String>> {
        if self.peek() == TT::Ident {
            let span = self.next().unwrap().span;

            Ok(Spnd::span(
                self.input[Range::from(span)].to_string(),
                span,
            ))
        } else {
            let token = self.next().unwrap();

            Err(ParseError::Mismatched {
                expected: TT::Ident,
                found: token.inner,
            }
            .span(token.span))
        }
    }

    pub fn delimited_list<T, F>(
        &mut self,
        mut f: F,
        start: TT,
        end: TT,
    ) -> ParseResult<Spnd<Vec<T>>>
    where
        F: FnMut(&mut Self) -> ParseResult<T>,
    {
        let start = self.consume(start)?.span.start;

        let mut items = Vec::new();
        while !self.at(end) {
            items.push(f(self)?);

            if !self.consume_at(TT::Comma) {
                break;
            }
        }
        let end = self.consume(end)?.span.end;

        Ok(Spnd::span(items, start..end))
    }
}
