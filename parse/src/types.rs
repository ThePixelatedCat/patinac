use ast::types::{Param, Return, Ty, TyKind};
use lex::{Tok, TokKind};

use crate::{ErrorKind, Parser, Result};

macro_rules! primitive {
    ($self:ident, $ty:ident) => {
        $self
            .consume(lex::TokKind::$ty)
            .map(|t| ast::types::TyKind::$ty.span(t.span))
    };
}

impl<'src, I: Iterator<Item = Tok<'src>>> Parser<'src, I> {
    pub(crate) fn ty(&mut self) -> Result<Ty> {
        match self.peek()? {
            TokKind::Int => primitive!(self, Int),
            TokKind::UInt => primitive!(self, UInt),
            TokKind::Byte => primitive!(self, Byte),
            TokKind::Float => primitive!(self, Float),
            TokKind::Bool => primitive!(self, Bool),
            TokKind::Char => primitive!(self, Char),
            TokKind::Hash => {
                let start = self.consume(TokKind::Hash)?.span.start;
                let (types, span) =
                    self.delimited_list(Self::ty, TokKind::LParen, TokKind::RParen)?;
                Ok(TyKind::Tuple(types).span(start..span.end))
            }
            TokKind::Fn => {
                let start = self.consume(TokKind::Fn)?.span.start;

                let (params, _) = self.delimited_list(
                    |this| {
                        Ok(Param {
                            mutable: this.consume_at(TokKind::Mut).is_some(),
                            ty: this.ty()?,
                        })
                    },
                    TokKind::LParen,
                    TokKind::RParen,
                )?;
                self.consume(TokKind::Arrow)?;
                let ret_mut = self.consume_at(TokKind::Mut).is_some();
                let ret_ty = Box::new(self.ty()?);

                let span = start..ret_ty.span.end;
                Ok(TyKind::Fn(
                    params,
                    Return {
                        mutable: ret_mut,
                        ty: ret_ty,
                    },
                )
                .span(span))
            }
            TokKind::Ident => {
                let ident = self.ident()?;

                let (generics, end) = if self.at(TokKind::LBracket) {
                    self.delimited_list(Self::ty, TokKind::LBracket, TokKind::RBracket)
                        .map(|(g, s)| (g, s.end))?
                } else {
                    (vec![], ident.span.end)
                };

                Ok(TyKind::Adt(ident.ident, generics).span(ident.span.start..end))
            }
            _ => Err(self.err_next(ErrorKind::Unexpected, &[])),
        }
    }
}
