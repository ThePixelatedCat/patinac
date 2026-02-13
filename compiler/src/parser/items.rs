use crate::{
    helpers::Spannable,
    lexer::{TT, Token},
    parser::ast::{FieldS, TypeDef},
};

use super::{
    ParseError, ParseResult, Parser,
    ast::{Field, Item, Variant},
};

impl<I: Iterator<Item = Token>> Parser<'_, I> {
    pub fn file(&mut self) -> ParseResult<Vec<Item>> {
        let mut items = Vec::new();
        while !self.at(TT::Eof) {
            items.push(self.item()?);
        }
        Ok(items)
    }

    pub fn item(&mut self) -> ParseResult<Item> {
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

    fn const_item(&mut self) -> ParseResult<Item> {
        self.consume(TT::Const)?;

        let name = self.ident()?.inner;

        let ty = self.ty_annot()?;

        self.consume(TT::Eq)?;

        let value = self.expr()?;

        Ok(Item::Const { name, ty, value })
    }

    fn func_item(&mut self) -> ParseResult<Item> {
        self.consume(TT::Fn)?;

        let name = self.ident()?.inner;

        let params = self.delimited_list(Self::pattern, TT::LParen, TT::RParen)?;

        let return_type = if self.consume_at(TT::Colon) {
            Some(self.parse_ty()?)
        } else {
            None
        };

        self.consume(TT::Arrow)?;

        let body = self.expr()?;

        Ok(Item::Func {
            name,
            params: params.inner,
            return_ty: return_type,
            body,
        })
    }

    fn struct_item(&mut self) -> ParseResult<Item> {
        self.consume(TT::Record)?;

        Ok(Item::Record {
            def: self.type_def()?,
            fields: self.fields()?,
        })
    }

    fn enum_item(&mut self) -> ParseResult<Item> {
        self.consume(TT::Enum)?;

        let def = self.type_def()?;

        let mut variants = Vec::new();
        while self.at(TT::Pipe) {
            variants.push(self.enum_variant()?);
        }

        Ok(Item::Enum { def, variants })
    }

    fn enum_variant(&mut self) -> ParseResult<Variant> {
        self.consume(TT::Pipe)?;

        let name = self.ident()?;

        Ok(match self.peek() {
            TT::Indent => {
                let fields = self.fields()?;
                Variant::Struct(name.inner, fields)
            }
            TT::LParen => {
                let vals = self
                    .delimited_list(Self::parse_ty, TT::LParen, TT::RParen)?
                    .inner;

                Variant::Tuple(name.inner, vals)
            }
            TT::Pipe => Variant::Unit(name.inner),
            _ => {
                let token = self.next().unwrap();

                return Err(
                    ParseError::Unexpected(token.inner, "after variant name").span(token.span)
                );
            }
        })
    }

    fn type_def(&mut self) -> ParseResult<TypeDef> {
        let name = self.ident()?.inner;

        let generic_params = if self.at(TT::LAngle) {
            self.delimited_list(Self::ident, TT::LAngle, TT::RAngle)?
                .inner
        } else {
            Vec::new()
        };

        Ok(TypeDef {
            name,
            generic_params,
        })
    }

    fn fields(&mut self) -> ParseResult<Vec<FieldS>> {
        self.delimited_list(
            |this| {
                if this.peek() == TT::Ident {
                    let name = this.ident()?;
                    let start = name.span.start;

                    this.consume(TT::Colon)?;

                    let ty = this.parse_ty()?;
                    let end = ty.span.end;

                    Ok(Field {
                        name: name.inner,
                        ty,
                    }
                    .span(start..end))
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
        .map(|fields| fields.inner)
    }
}
