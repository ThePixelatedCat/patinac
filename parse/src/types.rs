use ast::{Ty, TyKind};
use lex::{Tok, TokKind};
use span::{Span, Spnd};

use crate::{ParseError, ParseResult, Parser};

macro_rules! primitive {
    ($self:ident, $ty:ident) => {
        Ok(Ty {
            kind: ast::TyKind::$ty,
            span: $self.consume(lex::TokKind::$ty).unwrap().span,
        })
    };
}

impl<I: Iterator<Item = Tok>> Parser<'_, I> {
    pub fn ty(&mut self) -> ParseResult<Ty> {
        match self.peek() {
            TokKind::Int => primitive!(self, Int),
            TokKind::UInt => primitive!(self, UInt),
            TokKind::Byte => primitive!(self, Byte),
            TokKind::Float => primitive!(self, Float),
            TokKind::Bool => primitive!(self, Bool),
            TokKind::Char => primitive!(self, Char),
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

        let (generics, end) = if self.at(TokKind::Lt) {
            let (generics, generics_span) =
                self.delimited_list(Self::ty, TokKind::Lt, TokKind::Gt)?;
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
