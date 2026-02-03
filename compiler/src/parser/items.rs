use std::ops::Range;

use crate::{
    helpers::Spanned,
    lexer::{Token, TokenType},
    parser::ast::FieldS,
};

use super::{
    ParseError, ParseResult, Parser,
    ast::{Ast, Field, Item, ItemS, Variant},
};

impl<I: Iterator<Item = Token>> Parser<'_, I> {
    pub fn file(&mut self) -> ParseResult<Ast> {
        let mut items = Vec::new();
        while !self.at(TokenType::Eof) {
            items.push(self.item()?);
        }
        Ok(items)
    }

    pub fn item(&mut self) -> ParseResult<ItemS> {
        match self.peek() {
            TokenType::Const => self.const_item(),
            TokenType::Fn => self.func_item(),
            TokenType::Struct => self.struct_item(),
            TokenType::Enum => self.enum_item(),
            _ => {
                let token = self.next().unwrap();

                Err(
                    ParseError::Unexpected(token.inner, Some("start of item".into()))
                        .spanned(token.span),
                )
            }
        }
    }

    fn const_item(&mut self) -> ParseResult<ItemS> {
        let start = self.next().unwrap().span.start;

        let name = self.ident()?.inner;

        self.consume(TokenType::Colon)?;
        let ty = self.type_()?;

        self.consume(TokenType::Eq)?;
        let value = self.expression()?;

        let end = value.span.end;

        Ok(Item::Const { name, ty, value }.spanned(start..end))
    }

    fn func_item(&mut self) -> ParseResult<ItemS> {
        let start = self.next().unwrap().span.start;

        let name = self.ident()?.inner;

        let params = self.delimited_list(Self::binding, TokenType::LParen, TokenType::RParen)?;

        let return_type = if self.consume_at(TokenType::Colon) {
            Some(self.type_()?)
        } else {
            None
        };

        self.consume(TokenType::Arrow)?;

        let body = self.expression()?;

        let end = body.span.end;

        Ok(Item::Func {
            name,
            params: params.inner,
            return_ty: return_type,
            body,
        }
        .spanned(start..end))
    }

    fn struct_item(&mut self) -> ParseResult<ItemS> {
        let start = self.next().unwrap().span.start;

        let (name, generic_params) = self.type_name()?;

        let Spanned {
            inner: fields,
            span,
        } = self.fields()?;
        let end = span.end;

        Ok(Item::Struct {
            name,
            generic_params,
            fields,
        }
        .spanned(start..end))
    }

    fn enum_item(&mut self) -> ParseResult<ItemS> {
        let start = self.next().unwrap().span.start;

        let (name, generic_params) = self.type_name()?;

        let Spanned {
            inner: variants,
            span: variants_span,
        } = self.delimited_list(
            |this| {
                let name = this.ident()?;
                let start = name.span.start;

                Ok(match this.peek() {
                    TokenType::LBrace => {
                        let Spanned {
                            inner: fields,
                            span: fields_span,
                        } = this.fields()?;
                        Variant::Struct(name.inner, fields).spanned(start..fields_span.end)
                    }
                    TokenType::LParen => {
                        let Spanned { inner: vals, span } =
                            this.delimited_list(Self::type_, TokenType::LParen, TokenType::RParen)?;

                        Variant::Tuple(name.inner, vals).spanned(start..span.end)
                    }
                    TokenType::Comma => Variant::Unit(name.inner).spanned(name.span),
                    _ => {
                        let token = this.next().unwrap();

                        return Err(ParseError::Unexpected(
                            token.inner,
                            Some("after variant name. expected one of `,` `(` `{`".into()),
                        )
                        .spanned(token.span));
                    }
                })
            },
            TokenType::LBrace,
            TokenType::RBrace,
        )?;

        Ok(Item::Enum {
            name,
            generic_params,
            variants,
        }
        .spanned(start..variants_span.end))
    }

    fn type_name(&mut self) -> ParseResult<(String, Vec<String>)> {
        let name = self.ident()?.inner;

        let generic_params = if self.at(TokenType::LAngle) {
            self.delimited_list(
                |this| this.ident().map(|v| v.inner),
                TokenType::LAngle,
                TokenType::RAngle,
            )?
            .inner
        } else {
            Vec::new()
        };

        Ok((name, generic_params))
    }

    fn fields(&mut self) -> ParseResult<Spanned<Vec<FieldS>>> {
        self.delimited_list(
            |this| {
                if this.peek() == TokenType::Ident {
                    let span = this.next().unwrap().span;

                    this.consume(TokenType::Colon)?;

                    let ty = this.type_()?;
                    let end = ty.span.end;

                    Ok(Field {
                        name: this.input[Range::from(span)].to_string(),
                        ty,
                    }
                    .spanned(span.start..end))
                } else {
                    let token = this.next().unwrap();

                    Err(ParseError::Mismatched {
                        expected: TokenType::Ident,
                        found: token.inner,
                    }
                    .spanned(token.span))
                }
            },
            TokenType::LBrace,
            TokenType::RBrace,
        )
    }
}
