use crate::{
    ast::{Ty, TyKind},
    helpers::{Span, Spnd},
    lexer::{Tok, TokKind},
};

use super::{ParseError, ParseResult, Parser};

macro_rules! primitive_ty {
    ($self:ident, $ty:ident) => {
        Ok(Ty {
            kind: $crate::ast::TyKind::$ty,
            span: $self.consume($crate::lexer::TokKind::$ty).unwrap().span,
        })
    };
}

impl<I: Iterator<Item = Tok>> Parser<'_, I> {
    pub fn ty(&mut self) -> ParseResult<Ty> {
        match self.peek() {
            TokKind::Int => primitive_ty!(self, Int),
            TokKind::UInt => primitive_ty!(self, UInt),
            TokKind::Byte => primitive_ty!(self, Byte),
            TokKind::Float => primitive_ty!(self, Float),
            TokKind::Bool => primitive_ty!(self, Bool),
            TokKind::Char => primitive_ty!(self, Char),
            TokKind::LBracket => self.array_ty(),
            TokKind::LParen => self.tuple_ty(),
            TokKind::Fn => self.fn_ty(),
            TokKind::Ident => self.ast_ty(),
            _ => Err(self.err_next(|tk| ParseError::Unexpected(tk, "start of type name"))),
        }
    }

    fn array_ty(&mut self) -> ParseResult<Ty> {
        let start = self.consume(TokKind::LBracket)?.span.start;

        let inner_type = self.ty()?;

        let end = self.consume(TokKind::RBracket)?.span.end;

        Ok(Ty {
            kind: TyKind::Array(Box::new(inner_type)),
            span: Span::from(start..end),
        })
    }

    fn tuple_ty(&mut self) -> ParseResult<Ty> {
        let (types, span) = self.delimited_list(Self::ty, TokKind::LParen, TokKind::RParen)?;

        Ok(Ty {
            kind: TyKind::Tuple(types),
            span,
        })
    }

    fn fn_ty(&mut self) -> ParseResult<Ty> {
        let start = self.consume(TokKind::Fn)?.span.start;

        let (params, _) = self.delimited_list(Self::ty, TokKind::LParen, TokKind::RParen)?;

        self.consume(TokKind::Colon)?;
        let result = Box::new(self.ty()?);

        let end = result.span.end;

        Ok(Ty {
            kind: TyKind::Fn(params, result),
            span: Span::from(start..end),
        })
    }

    fn ast_ty(&mut self) -> ParseResult<Ty> {
        let Spnd(ident, span) = self.ident()?;

        let start = span.start;

        let (generics, end) = if self.at(TokKind::LAngle) {
            let (generics, generics_span) =
                self.delimited_list(Self::ty, TokKind::LAngle, TokKind::RAngle)?;
            (generics, generics_span.end)
        } else {
            (Vec::new(), span.end)
        };

        Ok(Ty {
            kind: TyKind::Adt {
                ident,
                args: generics,
            },
            span: Span::from(start..end),
        })
    }

    pub fn ty_annot(&mut self) -> ParseResult<Option<Ty>> {
        self.consume_at(TokKind::Colon)
            .then(|| self.ty())
            .transpose()
    }
}
