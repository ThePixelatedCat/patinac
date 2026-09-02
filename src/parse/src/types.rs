use std::range::Range;

use irs::ast::{ParamTy, Ty, TyKind};

use crate::{Parser, Result, TokKind};

impl Parser<'_> {
    pub(crate) fn ty(&mut self) -> Result<Ty> {
        match self.peek()?.kind {
            TokKind::LParen => self
                .delimited_list(Self::ty, TokKind::LParen, TokKind::RParen)
                .map(|(tys, span)| TyKind::Tuple(tys).span(span)),
            TokKind::FnTy => {
                let start = self.consume(TokKind::FnTy)?.span.start;

                let (params, _) =
                    self.delimited_list(Self::param_ty, TokKind::LParen, TokKind::RParen)?;
                self.consume(TokKind::Arrow)?;
                let ret_ty = self.ty()?;

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
                let span = Range::from(span.start..end);
                Ok(TyKind::Named(path, generics).span(span))
            }
            _ => Err(self.unexpected(None)),
        }
    }

    fn param_ty(&mut self) -> Result<ParamTy> {
        let mut_tok = self.consume_at(TokKind::Mut);
        let ty = self.ty()?;

        let start = mut_tok.map_or(ty.span.start, |tok| tok.span.start);
        let span = Range::from(start..ty.span.end);

        Ok(ParamTy {
            ty,
            mutable: mut_tok.is_some(),
            span,
        })
    }
}
