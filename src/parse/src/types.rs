use std::range::Range;

use ast::{FuncTy, Ty, TyKind};

use crate::{ErrorKind, Parser, Result, TokKind};

macro_rules! primitive {
    ($self:ident, $ty:ident) => {
        $self
            .consume($crate::TokKind::$ty)
            .map(|t| ast::TyKind::$ty.span(t.span))
    };
}

impl Parser<'_> {
    pub(crate) fn ty(&mut self) -> Result<Ty> {
        match self.peek()? {
            TokKind::Int => primitive!(self, Int),
            TokKind::UInt => primitive!(self, UInt),
            TokKind::Byte => primitive!(self, Byte),
            TokKind::Float => primitive!(self, Float),
            TokKind::Bool => primitive!(self, Bool),
            TokKind::Char => primitive!(self, Char),
            TokKind::LBracket => {
                let start = self.consume(TokKind::LBracket)?.span.start;
                let inner_ty = Box::new(self.ty()?);
                let end = self.consume(TokKind::RBracket)?.span.end;
                Ok(TyKind::Array(inner_ty).span(start..end))
            }
            TokKind::LParen => self
                .delimited_list(Self::ty, TokKind::LParen, TokKind::RParen)
                .map(|(tys, span)| TyKind::Tuple(tys).span(span)),
            TokKind::FnTy => {
                let start = self.consume(TokKind::FnTy)?.span.start;

                let (params, _) =
                    self.delimited_list(Self::func_ty, TokKind::LParen, TokKind::RParen)?;
                self.consume(TokKind::Arrow)?;
                let ret_ty = self.func_ty()?;

                let span = start..ret_ty.span.end;
                Ok(TyKind::Func(params, Box::new(ret_ty)).span(span))
            }
            TokKind::Ident => {
                let (path, span) = self.path()?;

                let (generics, end) = if self.at(TokKind::LBracket) {
                    self.delimited_list(Self::ty, TokKind::LBracket, TokKind::RBracket)
                        .map(|(g, s)| (g, s.end))?
                } else {
                    (vec![], span.end)
                };

                Ok(TyKind::Named(path, generics).span(span.start..end))
            }
            _ => Err(self.err_next(ErrorKind::Unexpected, &[])),
        }
    }

    fn func_ty(&mut self) -> Result<FuncTy> {
        let mut_tok = self.consume_at(TokKind::Mut);
        let ty = self.ty()?;

        let start = mut_tok.map_or(ty.span.start, |tok| tok.span.start);
        let span = Range::from(start..ty.span.end);

        Ok(FuncTy {
            ty,
            mutable: mut_tok.is_some(),
            span,
        })
    }
}
