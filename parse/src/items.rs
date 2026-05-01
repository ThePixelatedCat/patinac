use derive_more::From;

use ast::items::{AdtItem, AdtKind, ExecItem, ExecKind, Field, Param, Variant};
use errors::ResultExt;
use ident::{Ident, SpanIdent};
use lex::{Tok, TokKind};
use smallvec::{SmallVec, smallvec};
use span::Span;

use crate::{ErrorKind, Parser, Result};

#[derive(From, PartialEq, Debug)]
pub enum Item {
    ExecItem(ExecItem<(), Ident, Ident>),
    AdtItem(AdtItem<Ident>),
}

impl<'src, I: Iterator<Item = Tok<'src>>> Parser<'src, I> {
    pub fn item(&mut self) -> Result<Item> {
        match self.peek()? {
            TokKind::Const => self.const_item().map(Item::from),
            TokKind::Fn => self.func_item().map(Item::from),
            TokKind::Record => self.record_item().map(Item::from),
            TokKind::Enum => self.enum_item().map(Item::from),
            _ => Err(self
                .err_next(ErrorKind::Unexpected)
                .context("at start of item")),
        }
    }

    fn const_item(&mut self) -> Result<ExecItem<(), Ident, Ident>> {
        let start = self.consume(TokKind::Const)?.span.start;

        let ident = self.ident()?;

        let ty = self.ty_annot()?;

        self.consume(TokKind::Eq)?;

        let val = self.expr()?;

        let span = Span::from(start..val.span.end);

        Ok(ExecItem {
            ident,
            kind: ExecKind::Const { ty, val },
            span,
        })
    }

    fn func_item(&mut self) -> Result<ExecItem<(), Ident, Ident>> {
        let start = self.consume(TokKind::Fn)?.span.start;

        let ident = self.ident()?;

        let (generics, _) = self.generic_params()?;

        let (params, _) = self.delimited_list(
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

        self.consume(TokKind::Colon)?;

        let return_ty = self.ty()?;

        self.consume(TokKind::Arrow)?;

        let body = self.expr()?;

        let span = Span::from(start..body.span.end);

        Ok(ExecItem {
            ident,
            kind: ExecKind::Func {
                generics,
                params,
                return_ty,
                body,
            },
            span,
        })
    }

    fn record_item(&mut self) -> Result<AdtItem<Ident>> {
        let start = self.consume(TokKind::Record)?.span.start;

        let ident = self.ident()?;
        let (generics, _) = self.generic_params()?;

        let (fields, fields_span) = self.fields()?;

        Ok(AdtItem {
            ident,
            generics,
            span: Span::from(start..fields_span.end),
            kind: AdtKind::Record(fields),
        })
    }

    fn enum_item(&mut self) -> Result<AdtItem<Ident>> {
        let start = self.consume(TokKind::Enum)?.span.start;

        let ident = self.ident()?;
        let (generics, generics_span) = self.generic_params()?;

        let mut variants = Vec::new();
        let mut variant_end = None;
        while self.consume_at(TokKind::Pipe).is_some() {
            let ident = self.ident()?;
            let (fields, fields_span) = self.fields()?;
            variant_end = Some(fields_span.end);
            variants.push(Variant { ident, fields });
        }

        let end = variant_end
            .or_else(|| generics_span.map(|s| s.end))
            .unwrap_or(ident.span.end);

        Ok(AdtItem {
            ident,
            generics,
            span: Span::from(start..end),
            kind: AdtKind::Enum(variants),
        })
    }

    fn fields(&mut self) -> Result<(Vec<Field<Ident>>, Span)> {
        self.delimited_list(
            |this| {
                let SpanIdent {
                    ident,
                    span: ident_span,
                } = this.ident()?;

                this.consume(TokKind::Colon)?;
                let ty = this.ty()?;

                let span = Span::from(ident_span.start..ty.span.end);

                Ok(Field { ident, ty, span })
            },
            TokKind::LParen,
            TokKind::RParen,
        )
    }

    fn generic_params(&mut self) -> Result<(SmallVec<[Ident; 4]>, Option<Span>)> {
        if self.at(TokKind::LBracket) {
            let (idents, span) = self.delimited_list(
                |this| this.consume(TokKind::Ident).map(|tok| Ident::new(tok.src)),
                TokKind::LBracket,
                TokKind::RBracket,
            )?;

            Ok((idents.into(), Some(span)))
        } else {
            Ok((smallvec![], None))
        }
    }
}
