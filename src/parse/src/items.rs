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
    DefItem(DefItem),
}

impl Parser<'_> {
    pub(crate) fn item(&mut self) -> Result<Item> {
        let public = self.consume_at(TokKind::Pub);

        match self.peek()?.kind {
            TokKind::Import => self.import_item().map(Item::from),
            TokKind::Opaque => {
                self.consume(TokKind::Opaque)?;
                match self.peek()?.kind {
                    TokKind::Type => self.type_item(public.is_some(), true).map(Item::from),
                    _ => Err(self.unexpected(Some("opaque must be followed by a type item"))),
                }
            }
            TokKind::Type => self.type_item(public.is_some(), false).map(Item::from),
            TokKind::Impl => self.impl_item().map(Item::from),
            TokKind::Def => self.def_item(public.is_some()).map(Item::from),
            _ => match public {
                Some(_) => Err(self.unexpected(Some("expected `type`, `opaque`, or `def`"))),
                None => Err(self.unexpected(None)),
            },
        }
    }

    fn import_item(&mut self) -> Result<Import> {
        self.consume(TokKind::Import)?;
        let (path, span) = self.path()?;
        Ok(Import(path, span))
    }

    fn type_item(&mut self, public: bool, opaque: bool) -> Result<TyItem> {
        self.consume(TokKind::Type)?;

        let ident = self.ident()?;
        let generics = self.generic_params()?;

        let kind = match self.peek()?.kind {
            TokKind::LParen => self.record_kind()?,
            TokKind::LBrace => self.union_kind()?,
            _ => return Err(self.unexpected(Some("expected `(` for a record or `{` for a union"))),
        };

        Ok(TyItem {
            public,
            opaque,
            ident,
            generics,
            kind,
        })
    }

    fn record_kind(&mut self) -> Result<TyItemKind> {
        self.fields().map(|(fields, _)| TyItemKind::Record(fields))
    }

    fn union_kind(&mut self) -> Result<TyItemKind> {
        self.delimited_list(
            |this| {
                let ident = this.ident()?;
                let (fields, _) = this.fields()?;
                Ok(Variant { ident, fields })
            },
            TokKind::LBrace,
            TokKind::RBrace,
        )
        .map(|(variants, _)| TyItemKind::Union(variants))
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
        let span = self.consume(TokKind::Impl)?.span;

        let ty = self.ty()?;

        self.consume(TokKind::LBrace)?;
        let mut items = vec![];
        while self.consume_at(TokKind::RBrace).is_none() {
            let item = match self.item()? {
                Item::Import(item) => return Err(self.err(ErrorKind::NotDefInImpl, item.1)),
                Item::TyItem(item) => {
                    return Err(self.err(ErrorKind::NotDefInImpl, item.ident.span));
                }
                Item::BlockItem(BlockItem::Impl { span, .. }) => {
                    return Err(self.err(ErrorKind::NotDefInImpl, span));
                }
                Item::DefItem(item) => item,
            };

            items.push(item);
        }

        Ok(BlockItem::Impl { span, ty, items })
    }

    fn def_item(&mut self, public: bool) -> Result<DefItem> {
        self.consume(TokKind::Def)?;

        let ident = self.ident()?;
        let generics = self.generic_params()?;
        let kind = match self.peek()?.kind {
            TokKind::LParen => self.func_kind()?,
            TokKind::Colon => self.const_kind()?,
            _ => {
                return Err(
                    self.unexpected(Some("expected `(` for a function or `=` for a constant"))
                );
            }
        };

        Ok(DefItem {
            public,
            ident,
            generics,
            kind,
        })
    }

    fn const_kind(&mut self) -> Result<DefKind> {
        self.consume(TokKind::Colon)?;
        let ty = self.ty();
        self.consume(TokKind::Eq)?;
        let val = self.expr();
        Ok(DefKind::Const { ty: ty?, val: val? })
    }

    fn func_kind(&mut self) -> Result<DefKind> {
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

        Ok(DefKind::Func {
            self_param,
            params,
            ret_ty,
            body,
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
