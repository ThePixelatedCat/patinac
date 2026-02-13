use std::ops::Range;

use crate::{
    helpers::{Spannable, Spnd},
    lexer::{TT, Token},
    parser::ast::{FieldS, VariantS},
};

use super::{
    ParseError, ParseResult, Parser,
    ast::{Field, Item, ItemS, Variant},
};

impl<I: Iterator<Item = Token>> Parser<'_, I> {
    pub fn file(&mut self) -> ParseResult<Vec<ItemS>> {
        let mut items = Vec::new();
        while !self.at(TT::Eof) {
            items.push(self.item()?);
        }
        Ok(items)
    }

    pub fn item(&mut self) -> ParseResult<ItemS> {
        match self.peek() {
            TT::Const => self.const_item(),
            TT::Fn => self.func_item(),
            TT::Record => self.struct_item(),
            TT::Enum => self.enum_item(),
            _ => {
                let token = self.next().unwrap();

                Err(ParseError::Unexpected(token.inner, "start of item").span(token.span))
            }
        }
    }

    fn const_item(&mut self) -> ParseResult<ItemS> {
        let start = self.next().unwrap().span.start;

        let name = self.ident()?.inner;

        let ty = if self.consume_at(TT::Colon) {
            Some(self.parse_ty()?)
        } else {
            None
        };

        self.consume(TT::Eq)?;
        let value = self.expr()?;

        let end = value.span.end;

        Ok(Item::Const { name, ty, value }.span(start..end))
    }

    fn func_item(&mut self) -> ParseResult<ItemS> {
        let start = self.next().unwrap().span.start;

        let name = self.ident()?.inner;

        let params = self.delimited_list(Self::pattern, TT::LParen, TT::RParen)?;

        let return_type = if self.consume_at(TT::Colon) {
            Some(self.parse_ty()?)
        } else {
            None
        };

        self.consume(TT::Arrow)?;

        let body = self.expr()?;

        let end = body.span.end;

        Ok(Item::Func {
            name,
            params: params.inner,
            return_ty: return_type,
            body,
        }
        .span(start..end))
    }

    fn struct_item(&mut self) -> ParseResult<ItemS> {
        let start = self.next().unwrap().span.start;

        let (name, generic_params) = self.type_name()?;

        let Spnd {
            inner: fields,
            span,
        } = self.fields()?;
        let end = span.end;

        Ok(Item::Struct {
            name,
            generic_params,
            fields,
        }
        .span(start..end))
    }

    fn enum_item(&mut self) -> ParseResult<ItemS> {
        let start = self.next().unwrap().span.start;

        let (name, generic_params) = self.type_name()?;

        let variants = Vec::new();
        while self.at(TT::Pipe) {
            variants.push(self.enum_variant()?);
        }
        let end = variants.last().map_or(with_end, |var| var.span.end);

        Ok(Item::Enum {
            name,
            generic_params,
            variants,
        }
        .span(start..end))
    }

    fn enum_variant(&mut self) -> ParseResult<VariantS> {
        let start = self.consume(TT::Pipe)?.span.start;

        let name = self.ident()?;

        Ok(match self.peek() {
            TT::Indent => {
                let Spnd {
                    inner: fields,
                    span: fields_span,
                } = self.fields()?;
                Variant::Struct(name.inner, fields).span(start..fields_span.end)
            }
            TT::LParen => {
                let Spnd { inner: vals, span } = self.delimited_list(
                    Self::parse_ty,
                    TT::LParen,
                    TT::RParen,
                )?;

                Variant::Tuple(name.inner, vals).span(start..span.end)
            }
            TT::Pipe => Variant::Unit(name.inner).span(name.span),
            _ => {
                let token = self.next().unwrap();

                return Err(ParseError::Unexpected(
                    token.inner,
                    "after variant name. expected one of `,` `(` `{`",
                )
                .span(token.span));
            }
        })
    }

    fn type_name(&mut self) -> ParseResult<(String, Vec<String>)> {
        let name = self.ident()?.inner;

        let generic_params = if self.at(TT::LAngle) {
            self.delimited_list(
                |this| this.ident().map(|v| v.inner),
                TT::LAngle,
                TT::RAngle,
            )?
            .inner
        } else {
            Vec::new()
        };

        Ok((name, generic_params))
    }

    fn fields(&mut self) -> ParseResult<Spnd<Vec<FieldS>>> {
        self.delimited_list(
            |this| {
                if this.peek() == TT::Ident {
                    let span = this.next().unwrap().span;

                    this.consume(TT::Colon)?;

                    let ty = this.parse_ty()?;
                    let end = ty.span.end;

                    Ok(Field {
                        name: this.input[Range::from(span)].to_string(),
                        ty,
                    }
                    .span(span.start..end))
                } else {
                    let token = this.next().unwrap();

                    Err(ParseError::Mismatched {
                        expected: TT::Ident,
                        found: token.inner,
                    }
                    .span(token.span))
                }
            },
            TT::Indent,
            TT::Dedent,
        )
    }
}
