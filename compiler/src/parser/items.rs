use crate::{
    ast::{AdtDef, Field, GenericParam, Generics, Item, Variant, VariantData},
    helpers::Span,
    lexer::{Tok, TokKind},
};

use super::{ParseError, ParseResult, Parser};

impl<I: Iterator<Item = Tok>> Parser<'_, I> {
    pub fn file(&mut self) -> ParseResult<Vec<Item>> {
        let mut items = Vec::new();
        while !self.at(TokKind::Eof) {
            items.push(self.item()?);
        }
        Ok(items)
    }

    pub fn item(&mut self) -> ParseResult<Item> {
        match self.peek() {
            TokKind::Const => self.const_item(),
            TokKind::Fn => self.func_item(),
            TokKind::Record => self.record_item(),
            TokKind::Enum => self.enum_item(),
            _ => Err(self.err_next(|tk| ParseError::Unexpected(tk, "start of item"))),
        }
    }

    fn const_item(&mut self) -> ParseResult<Item> {
        self.consume(TokKind::Const)?;

        let name = self.ident()?.0;

        let ty = self.ty_annot()?;

        self.consume(TokKind::Eq)?;

        let value = self.expr()?;

        Ok(Item::Const {
            ident: name,
            ty,
            value,
        })
    }

    fn func_item(&mut self) -> ParseResult<Item> {
        self.consume(TokKind::Fn)?;

        let name = self.ident()?.0;

        let (params, _) = self.delimited_list(Self::pattern, TokKind::LParen, TokKind::RParen)?;

        let return_ty = self.ty_annot()?;

        self.consume(TokKind::Arrow)?;

        let body = self.expr()?;

        Ok(Item::Func {
            ident: name,
            params,
            return_ty,
            body,
        })
    }

    fn record_item(&mut self) -> ParseResult<Item> {
        self.consume(TokKind::Record)?;

        Ok(Item::Record {
            def: self.type_def()?,
            data: self.variant_data()?,
        })
    }

    fn enum_item(&mut self) -> ParseResult<Item> {
        self.consume(TokKind::Enum)?;

        let def = self.type_def()?;

        let mut variants = Vec::new();
        while self.consume_at(TokKind::Pipe) {
            variants.push(self.variant()?);
        }

        Ok(Item::Enum { def, variants })
    }

    fn variant(&mut self) -> ParseResult<Variant> {
        let (ident, _) = self.ident()?;

        let data = self.variant_data()?;

        Ok(Variant { ident, data })
    }

    fn variant_data(&mut self) -> ParseResult<VariantData> {
        Ok(match self.peek() {
            TokKind::Indent => VariantData::Record(self.fields()?),
            TokKind::LParen => {
                let (vals, _) = self.delimited_list(Self::ty, TokKind::LParen, TokKind::RParen)?;

                VariantData::Tuple(vals)
            }
            _ => VariantData::Unit,
        })
    }

    fn type_def(&mut self) -> ParseResult<AdtDef> {
        let name = self.ident()?.0;

        let generics = self
            .at(TokKind::LAngle)
            .then(|| {
                let (idents, span) =
                    self.delimited_list(Self::ident, TokKind::LAngle, TokKind::RAngle)?;
                let params = idents
                    .into_iter()
                    .map(|(ident, span)| GenericParam { ident, span })
                    .collect();

                Ok(Generics { params, span })
            })
            .transpose()?;

        Ok(AdtDef {
            ident: name,
            generics,
        })
    }

    fn fields(&mut self) -> ParseResult<Vec<Field>> {
        self.delimited_list(
            |this| {
                let (name, name_span) = this.ident()?;

                this.consume(TokKind::Colon)?;
                let ty = this.ty()?;

                let span = Span::from(name_span.start..ty.span.end);

                Ok(Field {
                    ident: name,
                    ty,
                    span,
                })
            },
            TokKind::Indent,
            TokKind::Dedent,
        )
        .map(|(fields, _)| fields)
    }
}
