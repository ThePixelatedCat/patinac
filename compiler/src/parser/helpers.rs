use std::ops::Range;

use crate::{
    helpers::Spanned,
    lexer::{Token, TokenType},
};

use super::{
    ParseError, ParseResult, Parser,
    ast::{Binding, BindingS, Type, TypeS},
};

impl<I: Iterator<Item = Token>> Parser<'_, I> {
    pub fn binding(&mut self) -> ParseResult<BindingS> {
        let mutable = self.at(TokenType::Mut);
        let mut_start = mutable.then(|| self.next().unwrap().span.start);

        let ident = self.ident()?;

        let start = mut_start.unwrap_or(ident.span.start);

        let type_annotation = if self.consume_at(TokenType::Colon) {
            Some(self.type_()?)
        } else {
            None
        };

        let end = type_annotation
            .as_ref()
            .map_or(ident.span.end, |ty| ty.span.end);

        Ok(Binding::Var {
            mutable,
            ident: ident.inner,
            annotated_ty: type_annotation,
        }
        .spanned(start..end))
    }

    pub fn type_(&mut self) -> ParseResult<TypeS> {
        Ok(match self.peek() {
            TokenType::Int => Type::Int.spanned(self.next().unwrap().span),
            TokenType::UInt => Type::UInt.spanned(self.next().unwrap().span),
            TokenType::Byte => Type::Byte.spanned(self.next().unwrap().span),
            TokenType::Float => Type::Float.spanned(self.next().unwrap().span),
            TokenType::Bool => Type::Bool.spanned(self.next().unwrap().span),
            TokenType::Char => Type::Char.spanned(self.next().unwrap().span),
            TokenType::Ident => {
                let span = self.next().unwrap().span;
                let name = self.input[Range::from(span)].to_string();

                let start = span.start;

                let (generics, end) = if self.at(TokenType::LAngle) {
                    let Spanned {
                        inner: generics,
                        span: generics_span,
                    } = self.delimited_list(Self::type_, TokenType::LAngle, TokenType::RAngle)?;
                    (generics, generics_span.end)
                } else {
                    (Vec::new(), span.end)
                };

                Type::Named {
                    name,
                    args: generics,
                }
                .spanned(start..end)
            }
            TokenType::LBracket => {
                let start = self.next().unwrap().span.start;

                let inner_type = self.type_()?;

                let end = self.consume(TokenType::RBracket)?.span.end;

                Type::Array(Box::new(inner_type)).spanned(start..end)
            }
            TokenType::LParen => {
                let Spanned { inner: types, span } =
                    self.delimited_list(Self::type_, TokenType::LParen, TokenType::RParen)?;
                Type::Tuple(types).spanned(span)
            }
            TokenType::Fn => {
                let start = self.next().unwrap().span.start;

                let Spanned { inner: params, .. } =
                    self.delimited_list(Self::type_, TokenType::LParen, TokenType::RParen)?;

                self.consume(TokenType::Colon)?;
                let result = Box::new(self.type_()?);

                let end = result.span.end;

                Type::Fn(params, result).spanned(start..end)
            }
            _ => {
                let token = self.next().unwrap();

                return Err(
                    ParseError::Unexpected(token.inner, Some("start of type name".into()))
                        .spanned(token.span),
                );
            }
        })
    }

    pub fn ident(&mut self) -> ParseResult<Spanned<String>> {
        if self.peek() == TokenType::Ident {
            let span = self.next().unwrap().span;

            Ok(Spanned::span(self.input[Range::from(span)].to_string(), span))
        } else {
            let token = self.next().unwrap();

            Err(ParseError::Mismatched {
                expected: TokenType::Ident,
                found: token.inner,
            }
            .spanned(token.span))
        }
    }

    pub fn delimited_list<T, F>(
        &mut self,
        mut f: F,
        start: TokenType,
        end: TokenType,
    ) -> ParseResult<Spanned<Vec<T>>>
    where
        F: FnMut(&mut Self) -> ParseResult<T>,
    {
        let start = self.consume(start)?.span.start;

        let mut items = Vec::new();
        while !self.at(end) {
            items.push(f(self)?);

            if !self.consume_at(TokenType::Comma) {
                break;
            }
        }
        let end = self.consume(end)?.span.end;

        Ok(Spanned::span(items, start..end))
    }
}
