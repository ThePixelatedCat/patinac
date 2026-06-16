use std::range::Range;

use derive_more::From;

use ast::{ExecItem, ExecKind, Field, Param, TyItem, TyItemKind, Variant, VisItem};
use ident::SpanIdent;

use crate::{ErrorKind, Parser, Result, TokKind};

#[derive(Debug, PartialEq, From)]
pub enum Item {
    VisItem(VisItem),
    TyItem(TyItem),
    ExecItem(ExecItem),
}

impl Parser<'_> {
    pub(crate) fn item(&mut self) -> Result<Item> {
        match self.peek()? {
            TokKind::Import => self.import_item().map(Item::from),
            TokKind::Export => self.export_item().map(Item::from),
            TokKind::Record => self.record_item().map(Item::from),
            TokKind::Enum => self.enum_item().map(Item::from),
            TokKind::Const => self.const_item().map(Item::from),
            TokKind::Fn => self.func_item().map(Item::from),
            _ => Err(self.err_next(ErrorKind::Unexpected, &["expected the start of an item"])),
        }
    }

    fn import_item(&mut self) -> Result<VisItem> {
        self.consume(TokKind::Import)?;
        let (path, span) = self.path()?;
        Ok(VisItem::Import(path, span))
    }

    fn export_item(&mut self) -> Result<VisItem> {
        self.consume(TokKind::Export)?;
        let (idents, _) = self.delimited_list(Self::ident, TokKind::LBrace, TokKind::RBrace)?;
        Ok(VisItem::Export(idents))
    }

    fn record_item(&mut self) -> Result<TyItem> {
        self.consume(TokKind::Record)?;

        let ident = self.ident();
        let generics = self.generic_params();
        let (fields, _) = self.fields()?;
        let generics = generics?;
        let ident = ident?;

        Ok(TyItem {
            ident,
            generics,
            kind: TyItemKind::Record(fields),
        })
    }

    fn enum_item(&mut self) -> Result<TyItem> {
        self.consume(TokKind::Enum)?;

        let ident = self.ident();
        let generics = self.generic_params();
        let (variants, _) = self.delimited_list(
            |this| {
                let ident = this.ident()?;
                let (fields, _) = this.fields()?;
                Ok(Variant { ident, fields })
            },
            TokKind::LBrace,
            TokKind::RBrace,
        )?;
        let generics = generics?;
        let ident = ident?;

        Ok(TyItem {
            ident,
            generics,
            kind: TyItemKind::Enum(variants),
        })
    }

    fn fields(&mut self) -> Result<(Vec<Field>, Range<u32>)> {
        self.delimited_list(
            |this| {
                let public = this.consume_at(TokKind::Pub).is_some();
                let ident = this.ident()?;
                this.consume(TokKind::Colon)?;
                let ty = this.ty()?;

                Ok(Field { public, ident, ty })
            },
            TokKind::LParen,
            TokKind::RParen,
        )
    }

    fn const_item(&mut self) -> Result<ExecItem> {
        self.consume(TokKind::Const)?;

        let ident = self.ident();
        self.consume(TokKind::Colon)?;
        let ty = self.ty();
        self.consume(TokKind::Eq)?;
        let val = self.expr();

        Ok(ExecItem {
            ident: ident?,
            kind: ExecKind::Const { ty: ty?, val: val? },
        })
    }

    fn func_item(&mut self) -> Result<ExecItem> {
        self.consume(TokKind::Fn)?;

        let ident = self.ident()?;
        let generics = self.generic_params()?;
        let (params, _) = self.delimited_list(
            |this| {
                let mut_tok = this.consume_at(TokKind::Mut);
                let pat = this.pattern()?;
                this.consume(TokKind::Colon)?;
                let ty = this.ty()?;

                let start = mut_tok.map_or(pat.span.start, |tok| tok.span.start);
                let span = Range::from(start..ty.span.end);

                Ok(Param {
                    mutable: mut_tok.is_some(),
                    pat,
                    ty,
                    span,
                })
            },
            TokKind::LParen,
            TokKind::RParen,
        )?;
        self.consume(TokKind::Colon)?;
        let ret_mut = self.consume_at(TokKind::Mut).is_some();
        let ret_ty = self.ty()?;
        self.consume(TokKind::Eq)?;
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

    fn generic_params(&mut self) -> Result<Vec<SpanIdent>> {
        if self.at(TokKind::LBracket) {
            let (idents, _) =
                self.delimited_list(Self::ident, TokKind::LBracket, TokKind::RBracket)?;
            Ok(idents)
        } else {
            Ok(Vec::new())
        }
    }
}
