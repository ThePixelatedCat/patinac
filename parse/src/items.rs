use ast::{AdtDef, Field, GenericParam, Item as _Item, Variant};
use lex::{Tok, TokKind};
use span::{Span, Spnd};

use crate::{ParseError, ParseResult, Parser};

type Item = _Item<()>;

impl<I: Iterator<Item = Tok>> Parser<'_, I> {
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

        let (params, _) = self.delimited_list(Self::binding, TokKind::LParen, TokKind::RParen)?;

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
            def: self.adt_def()?,
            fields: self.fields()?,
        })
    }

    fn enum_item(&mut self) -> ParseResult<Item> {
        self.consume(TokKind::Enum)?;

        let def = self.adt_def()?;

        let mut variants = Vec::new();

        while {
            self.strip_identation();
            self.consume_at(TokKind::Pipe)
        } {
            variants.push(Variant {
                ident: self.ident()?.0,
                fields: self.fields()?,
            });
        }

        Ok(Item::Enum { def, variants })
    }

    fn adt_def(&mut self) -> ParseResult<AdtDef> {
        let Spnd(ident, _) = self.ident()?;

        let generics = if self.at(TokKind::LAngle) {
            let (idents, _) = self.delimited_list(Self::ident, TokKind::LAngle, TokKind::RAngle)?;

            idents.into_iter().map(GenericParam).collect()
        } else {
            vec![]
        };

        Ok(AdtDef { ident, generics })
    }

    fn fields(&mut self) -> ParseResult<Vec<Field>> {
        self.delimited_list(
            |this| {
                let Spnd(ident, ident_span) = this.ident()?;

                this.consume(TokKind::Colon)?;
                let ty = this.ty()?;

                let span = Span::from(ident_span.start..ty.span.end);

                Ok(Field { ident, ty, span })
            },
            TokKind::LParen,
            TokKind::RParen,
        )
        .map(|(fields, _)| fields)
    }
}
