use ident::SpanIdent;
use lex::{Tok, TokKind};
use span::Span;
use types::{Param, Return, Ty};

use crate::{ErrorKind, Parser, Result};

macro_rules! primitive {
    ($self:ident, $ty:ident) => {
        Ok((types::Ty::$ty, $self.consume(lex::TokKind::$ty)?.span))
    };
}

impl<'src, I: Iterator<Item = Tok<'src>>> Parser<'src, I> {
    pub fn ty(&mut self) -> Result<(Ty<SpanIdent>, Span)> {
        match self.peek()? {
            TokKind::Int => primitive!(self, Int),
            TokKind::UInt => primitive!(self, UInt),
            TokKind::Byte => primitive!(self, Byte),
            TokKind::Float => primitive!(self, Float),
            TokKind::Bool => primitive!(self, Bool),
            TokKind::Char => primitive!(self, Char),
            TokKind::Hash => self.tuple_ty(),
            TokKind::Fn => self.fn_ty(),
            TokKind::Ident => self.adt_ty(),
            _ => Err(self.err_next(ErrorKind::Unexpected)),
        }
    }

    fn tuple_ty(&mut self) -> Result<(Ty<SpanIdent>, Span)> {
        let start = self.consume(TokKind::Hash)?.span.start;
        self.delimited_list(
            |this| this.ty().map(|(ty, _)| ty),
            TokKind::LParen,
            TokKind::RParen,
        )
        .map(|(types, span)| (Ty::Tuple(types), Span::from(start..span.end)))
    }

    fn fn_ty(&mut self) -> Result<(Ty<SpanIdent>, Span)> {
        let start = self.consume(TokKind::Fn)?.span.start;

        let (params, _) = self.delimited_list(
            |this| {
                Ok(Param {
                    mutable: this.consume_at(TokKind::Mut).is_some(),
                    ty: this.ty()?.0,
                })
            },
            TokKind::LParen,
            TokKind::RParen,
        )?;
        self.consume(TokKind::Arrow)?;
        let ret_mut = self.consume_at(TokKind::Mut).is_some();
        let (ret_ty, ret_span) = self.ty()?;

        Ok((
            Ty::Fn(
                params,
                Box::new(Return {
                    mutable: ret_mut,
                    ty: ret_ty,
                }),
            ),
            Span::from(start..ret_span.end),
        ))
    }

    fn adt_ty(&mut self) -> Result<(Ty<SpanIdent>, Span)> {
        let ident = self.ident()?;

        let (generics, end) = if self.at(TokKind::LBracket) {
            let (generics, generics_span) = self.delimited_list(
                |this| this.ty().map(|(ty, _)| ty),
                TokKind::LBracket,
                TokKind::RBracket,
            )?;
            (generics, generics_span.end)
        } else {
            (vec![], ident.span.end)
        };

        Ok((Ty::Adt(ident, generics), Span::from(ident.span.start..end)))
    }

    pub fn ty_annot(&mut self) -> Result<Option<Ty<SpanIdent>>> {
        self.consume_at(TokKind::Colon)
            .map(|_| self.ty().map(|(ty, _)| ty))
            .transpose()
    }
}
