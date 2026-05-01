use ast::patterns::{Pat, PatKind};
use ident::SpanIdent;
use lex::{Tok, TokKind};

use crate::{ErrorKind, Parser, Result};

impl<'src, I: Iterator<Item = Tok<'src>>> Parser<'src, I> {
    pub fn pattern(&mut self) -> Result<Pat> {
        match self.peek()? {
            TokKind::Minus
            | TokKind::IntLit
            | TokKind::FloatLit
            | TokKind::CharLit
            | TokKind::StringLit
            | TokKind::True
            | TokKind::False => {
                let negate_tok = self.consume_at(TokKind::Minus);
                let (lit, lit_span) = self.lit_expr()?;

                let start = negate_tok
                    .as_ref()
                    .map_or(lit_span.start, |tok| tok.span.start);

                Ok(PatKind::Literal {
                    negate: negate_tok.is_some(),
                    lit,
                }
                .span(start..lit_span.end))
            }
            TokKind::Underscore => {
                Ok(PatKind::Wildcard.span(self.consume(TokKind::Underscore)?.span))
            }
            TokKind::Ident => {
                let SpanIdent {
                    ident,
                    span: ident_span,
                } = self.ident()?;

                if self.at(TokKind::LParen) {
                    let (fields, fields_span) =
                        self.delimited_list(Self::pattern, TokKind::LParen, TokKind::RParen)?;
                    Ok(PatKind::Constructor(ident, fields).span(ident_span.start..fields_span.end))
                } else {
                    Ok(PatKind::Ident(ident).span(ident_span))
                }
            }
            TokKind::Hash => {
                let start = self.consume(TokKind::Hash)?.span.start;
                self.delimited_list(Self::pattern, TokKind::LParen, TokKind::RParen)
                    .map(|(pats, span)| PatKind::Tuple(pats).span(start..span.end))
            }
            _ => Err(self.err_next(ErrorKind::Unexpected)),
        }
    }
}
