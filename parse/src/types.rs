use ast::types::{Param, Ty, TyKind};
use ident::{Ident, SpanIdent};
use lex::{Tok, TokKind};

use crate::{ErrorKind, Parser, Result};

macro_rules! primitive {
    ($self:ident, $ty:ident) => {
        Ok(ast::types::TyKind::$ty.span($self.consume(lex::TokKind::$ty)?.span))
    };
}

impl<'src, I: Iterator<Item = Tok<'src>>> Parser<'src, I> {
    pub fn ty(&mut self) -> Result<Ty<Ident>> {
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

    fn tuple_ty(&mut self) -> Result<Ty<Ident>> {
        let start = self.consume(TokKind::Hash)?.span.start;
        self.delimited_list(Self::ty, TokKind::LParen, TokKind::RParen)
            .map(|(types, span)| TyKind::Tuple(types).span(start..span.end))
    }

    fn fn_ty(&mut self) -> Result<Ty<Ident>> {
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

        let result = Box::new(self.ty()?);

        let span = start..result.span.end;
        Ok(TyKind::Fn { params, result }.span(span))
    }

    fn adt_ty(&mut self) -> Result<Ty<Ident>> {
        let SpanIdent {
            ident,
            span: ident_span,
        } = self.ident()?;

        let (generics, end) = if self.at(TokKind::LBracket) {
            let (generics, generics_span) =
                self.delimited_list(Self::ty, TokKind::LBracket, TokKind::RBracket)?;
            (generics, generics_span.end)
        } else {
            (vec![], ident_span.end)
        };

        Ok(TyKind::Adt(ident, generics).span(ident_span.start..end))
    }

    pub fn ty_annot(&mut self) -> Result<Option<Ty<Ident>>> {
        self.consume_at(TokKind::Colon)
            .map(|_| self.ty())
            .transpose()
    }
}
