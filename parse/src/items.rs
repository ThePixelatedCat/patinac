use ast::{AdtDef, AdtItem, ExecItem, Field, GenericParam, Param, Variant};
use lex::{Tok, TokKind};
use span::Span;

use crate::{ParseError, ParseResult, Parser};

#[derive(PartialEq, Debug)]
pub enum Item {
    ExecItem(ExecItem<()>),
    AdtItem(AdtItem),
}

impl From<ExecItem<()>> for Item {
    fn from(value: ExecItem<()>) -> Self {
        Self::ExecItem(value)
    }
}

impl From<AdtItem> for Item {
    fn from(value: AdtItem) -> Self {
        Self::AdtItem(value)
    }
}

impl<I: Iterator<Item = Tok>> Parser<'_, I> {
    pub fn item(&mut self) -> ParseResult<Item> {
        match self.peek()? {
            TokKind::Const => self.const_item(),
            TokKind::Fn => self.func_item(),
            TokKind::Record => self.record_item(),
            TokKind::Enum => self.enum_item(),
            _ => Err(self.err_next(|tk| ParseError::Unexpected(tk, "start of item"))),
        }
    }

    fn const_item(&mut self) -> ParseResult<Item> {
        self.consume(TokKind::Const)?;

        let (ident, _) = self.ident()?;

        let ty = self.ty_annot()?;

        self.consume(TokKind::Eq)?;

        let value = self.expr()?;

        Ok(ExecItem::Const {
            ident,
            ty,
            val: value,
        }
        .into())
    }

    fn func_item(&mut self) -> ParseResult<Item> {
        self.consume(TokKind::Fn)?;

        let (ident, _) = self.ident()?;

        let (params, _) = self.delimited_list(Self::param, TokKind::LParen, TokKind::RParen)?;

        self.consume(TokKind::Colon)?;

        let return_ty = self.ty()?;

        self.consume(TokKind::Arrow)?;

        let body = self.expr()?;

        Ok(ExecItem::Func {
            ident,
            generic_params: vec![],
            params,
            return_ty,
            body,
        }
        .into())
    }

    fn param(&mut self) -> ParseResult<Param> {
        let mutable = self.consume_at(TokKind::Mut);
        let pat = self.pattern()?;
        self.consume(TokKind::Colon)?;
        let ty = self.ty()?;

        Ok(Param { mutable, pat, ty })
    }

    fn record_item(&mut self) -> ParseResult<Item> {
        self.consume(TokKind::Record)?;

        Ok(AdtItem::Record {
            def: self.adt_def()?,
            fields: self.fields()?,
        }
        .into())
    }

    fn enum_item(&mut self) -> ParseResult<Item> {
        self.consume(TokKind::Enum)?;

        let def = self.adt_def()?;

        let mut variants = Vec::new();
        while self.consume_at(TokKind::Pipe) {
            variants.push(Variant {
                ident: self.ident()?.0,
                fields: self.fields()?,
            });
        }

        Ok(AdtItem::Enum { def, variants }.into())
    }

    fn adt_def(&mut self) -> ParseResult<AdtDef> {
        Ok(AdtDef {
            ident: self.ident()?.0,
            generics: self.generic_params()?,
        })
    }

    fn fields(&mut self) -> ParseResult<Vec<Field>> {
        self.delimited_list(
            |this| {
                let (ident, ident_span) = this.ident()?;

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

    fn generic_params(&mut self) -> ParseResult<Vec<GenericParam>> {
        if self.at(TokKind::LBracket) {
            let (idents, _) =
                self.delimited_list(Self::ident, TokKind::LBracket, TokKind::RBracket)?;

            Ok(idents
                .into_iter()
                .map(|(ident, _)| GenericParam(ident))
                .collect())
        } else {
            Ok(vec![])
        }
    }
}
