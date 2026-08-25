use std::range::Range;

use derive_more::From;

use ident::SpanIdent;
use irs::ast::{BlockItem, DefItem, DefKind, Field, Import, Param, TyItem, TyItemKind, Variant};

use crate::{ErrorKind, Parser, Result, TokKind};

#[derive(Debug, PartialEq, From)]
pub enum Item {
    Import(Import),
    TyItem(TyItem),
    BlockItem(BlockItem),
    ExecItem(DefItem),
}

impl Parser<'_> {
    pub(crate) fn item(&mut self) -> Result<Item> {
        let public = self.consume_at(TokKind::Pub);

        match self.peek()?.kind {
            TokKind::Import => self.import_item().map(Item::from),
            TokKind::Opaque => {
                self.consume(TokKind::Opaque)?;
                match self.peek()?.kind {
                    TokKind::Record => self.record_item(public.is_some(), true).map(Item::from),
                    TokKind::Union => self.union_item(public.is_some(), true).map(Item::from),
                    _ => Err(self.err_next(ErrorKind::Unexpected, &["expected a type item"])),
                }
            }
            TokKind::Record => self.record_item(public.is_some(), false).map(Item::from),
            TokKind::Union => self.union_item(public.is_some(), false).map(Item::from),
            TokKind::Impl => self.impl_item().map(Item::from),
            TokKind::Const => self.const_item(public.is_some()).map(Item::from),
            TokKind::Fn => self.func_item(public.is_some()).map(Item::from),
            _ => Err(self.err_next(ErrorKind::Unexpected, &["expected the start of an item"])),
        }
    }

    fn import_item(&mut self) -> Result<Import> {
        self.consume(TokKind::Import)?;
        let (path, span) = self.path()?;
        Ok(Import(path, span))
    }

    fn record_item(&mut self, public: bool, opaque: bool) -> Result<TyItem> {
        self.consume(TokKind::Record)?;

        let ident = self.ident();
        let generics = self.generic_params();
        let (fields, _) = self.fields()?;
        let generics = generics?;
        let ident = ident?;

        Ok(TyItem {
            public,
            opaque,
            ident,
            generics,
            kind: TyItemKind::Record(fields),
        })
    }

    fn union_item(&mut self, public: bool, opaque: bool) -> Result<TyItem> {
        self.consume(TokKind::Union)?;

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
            public,
            opaque,
            ident,
            generics,
            kind: TyItemKind::Union(variants),
        })
    }

    fn fields(&mut self) -> Result<(Vec<Field>, Range<u32>)> {
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

    fn impl_item(&mut self) -> Result<BlockItem> {
        self.consume(TokKind::Impl)?;

        let ty = self.ty()?;

        self.consume(TokKind::LBrace)?;
        let mut items = vec![];
        while self.consume_at(TokKind::RBrace).is_none() {
            let item = match self.peek()?.kind {
                TokKind::Const => self.const_item(),
                TokKind::Fn => self.func_item(),
                _ => {
                    Err(self.err_next(ErrorKind::Unexpected, &["expected a function or constant"]))
                }
            }?;
            items.push(item);
        }

        Ok(BlockItem::Impl { ty, items })
    }

    fn const_item(&mut self, public: bool) -> Result<DefItem> {
        self.consume(TokKind::Const)?;

        let ident = self.ident();
        self.consume(TokKind::Colon)?;
        let ty = self.ty();
        self.consume(TokKind::Eq)?;
        let val = self.expr();

        Ok(DefItem {
            public,
            ident: ident?,
            kind: DefKind::Const { ty: ty?, val: val? },
        })
    }

    fn func_item(&mut self, public: bool) -> Result<DefItem> {
        self.consume(TokKind::Fn)?;

        let ident = self.ident()?;
        let generics = self.generic_params()?;

        self.consume(TokKind::LParen)?;

        let mut self_param = None;
        let mut params = Vec::new();
        while !self.at(TokKind::RParen) {
            let mut_tok = self.consume_at(TokKind::Mut);

            if self
                .peek()
                .is_ok_and(|tok| tok.kind == TokKind::Ident && self.src_of(tok) == "self")
            {
                let self_tok = self
                    .next()
                    .expect("should only enter this branch if the next token is okay");

                if self_param.is_some() || !params.is_empty() {
                    return Err(self.err(ErrorKind::SelfNotFirst, self_tok.span));
                }

                let start = mut_tok.map_or(self_tok.span.start, |tok| tok.span.start);
                let span = Range::from(start..self_tok.span.end);

                self_param = Some((mut_tok.is_some(), span));
            } else {
                let pat = self.pattern()?;
                self.consume(TokKind::Colon)?;
                let ty = self.ty()?;

                let start = mut_tok.map_or(pat.span.start, |tok| tok.span.start);
                let span = Range::from(start..ty.span.end);

                params.push(Param {
                    mutable: mut_tok.is_some(),
                    pat,
                    ty,
                    span,
                });
            }

            if self.consume_at(TokKind::Comma).is_none() {
                break;
            }
        }
        self.consume(TokKind::RParen)?;

        self.consume(TokKind::Colon)?;
        let ret_ty = self.ty()?;
        self.consume(TokKind::Eq)?;
        let body = self.expr()?;

        Ok(DefItem {
            public,
            ident,
            kind: DefKind::Func {
                generics,
                self_param,
                params,
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
