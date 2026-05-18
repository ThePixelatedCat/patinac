use derive_more::From;
use smallvec::{SmallVec, smallvec};

use ast::{
    items::{AdtItem, AdtKind, ExecItem, ExecKind, Field, Param, Variant},
    types::TyKind,
};
use errors::ResultExt;
use ident::{Ident, SpanIdent};
use lex::{Tok, TokKind};
use span::Span;

use crate::{ErrorKind, Parser, Result};

#[derive(From, PartialEq, Debug)]
pub enum Item {
    ExecItem(ExecItem),
    AdtItem(AdtItem),
}

impl<'src, I: Iterator<Item = Tok<'src>>> Parser<'src, I> {
    pub(crate) fn item(&mut self) -> Result<Item> {
        match self.peek()? {
            TokKind::Const => self.const_item().map(Item::from),
            TokKind::Fn => self.func_item().map(Item::from),
            TokKind::Record => self.record_item().map(Item::from),
            TokKind::Enum => self.enum_item().map(Item::from),
            _ => Err(self
                .err_next(ErrorKind::Unexpected)
                .with_ctx("expected the start of an item")),
        }
    }

    fn const_item(&mut self) -> Result<ExecItem> {
        self.consume(TokKind::Const)?;

        let ident = self.ident()?;
        let ty = self.ty_annot()?;
        self.consume(TokKind::Eq)?;
        let val = self.expr()?;

        Ok(ExecItem {
            ident,
            kind: ExecKind::Const { ty, val },
        })
    }

    fn func_item(&mut self) -> Result<ExecItem> {
        self.consume(TokKind::Fn)?;

        let ident = self.ident()?;
        let (generics, _) = self.generic_params()?;
        let (params, params_span) = self.delimited_list(
            |this| {
                let mutable = this.consume_at(TokKind::Mut).is_some();
                let pat = this.pattern()?;
                this.consume(TokKind::Colon)
                    .context("Type annotations are required on function parameters")?;
                let ty = this.ty()?;

                Ok(Param { mutable, pat, ty })
            },
            TokKind::LParen,
            TokKind::RParen,
        )?;
        let (ret_mut, ret_ty) = self
            .consume_at(TokKind::Colon)
            .map(|_| Ok((self.consume_at(TokKind::Mut).is_some(), self.ty()?)))
            .transpose()?
            .unwrap_or_else(|| {
                (
                    false,
                    TyKind::unit().span(params_span.end..params_span.end + 1),
                )
            });
        self.consume(TokKind::Arrow)?;
        let body = self.expr()?;

        Ok(ExecItem {
            ident,
            kind: ExecKind::Fn {
                generics,
                params,
                ret_mut,
                ret_ty,
                body,
            },
        })
    }

    fn record_item(&mut self) -> Result<AdtItem> {
        self.consume(TokKind::Record)?;

        let ident = self.ident()?;
        let (generics, _) = self.generic_params()?;
        let (fields, _) = self.fields()?;

        Ok(AdtItem {
            ident,
            generics,
            kind: AdtKind::Record(fields),
        })
    }

    fn enum_item(&mut self) -> Result<AdtItem> {
        self.consume(TokKind::Enum)?;

        let ident = self.ident()?;
        let (generics, _) = self.generic_params()?;
        let (variants, _) = self.delimited_list(
            |this| {
                let ident = this.ident()?;
                let (fields, _) = this.fields()?;
                Ok(Variant { ident, fields })
            },
            TokKind::LBrace,
            TokKind::RBrace,
        )?;

        Ok(AdtItem {
            ident,
            generics,
            kind: AdtKind::Enum(variants),
        })
    }

    fn fields(&mut self) -> Result<(Vec<Field>, Span)> {
        self.delimited_list(
            |this| {
                let ident = this.ident()?;
                this.consume(TokKind::Colon)?;
                let ty = this.ty()?;

                Ok(Field { ident, ty })
            },
            TokKind::LParen,
            TokKind::RParen,
        )
    }

    fn generic_params(&mut self) -> Result<(SmallVec<[SpanIdent; 4]>, Option<Span>)> {
        if self.at(TokKind::LBracket) {
            let (idents, span) = self.delimited_list(
                |this| {
                    this.consume(TokKind::Ident)
                        .map(|tok| Ident::new(tok.src).span(tok.span))
                },
                TokKind::LBracket,
                TokKind::RBracket,
            )?;

            Ok((idents.into(), Some(span)))
        } else {
            Ok((smallvec![], None))
        }
    }
}
