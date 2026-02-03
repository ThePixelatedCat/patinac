use std::{ops::Range, str::FromStr};

use crate::{
    helpers::{Span, Spanned},
    lexer::{Token, TokenType},
    parser::ast::ExprS,
};

use super::{
    ParseError, ParseResult, Parser,
    ast::{Bop, Expr, Unop},
};

impl<I: Iterator<Item = Token>> Parser<'_, I> {
    pub fn expression(&mut self) -> ParseResult<ExprS> {
        self.parse_expression(0)
    }

    #[allow(
        clippy::too_many_lines,
        reason = "still readable and segmented via the match"
    )]
    fn parse_expression(&mut self, binding_power: u8) -> ParseResult<ExprS> {
        let mut lhs = match self.peek() {
            TokenType::LParen => {
                let start = self.next().unwrap().span.start;
                let expr = self.expression()?;

                let expr = if self.consume_at(TokenType::Comma) {
                    let mut exprs = vec![expr];
                    while !self.at(TokenType::RParen) {
                        exprs.push(self.expression()?);

                        if !self.consume_at(TokenType::Comma) {
                            break;
                        }
                    }

                    Expr::Tuple(exprs)
                } else {
                    expr.inner
                };

                let end = self.consume(TokenType::RParen)?.span.end;

                expr.spanned(start..end)
            }
            TokenType::IntLit => {
                let token = self.next().unwrap();
                let val = u64::from_str(&self.input[Range::from(token.span)]).unwrap();
                Expr::Int(val).spanned(token.span)
            }
            TokenType::FloatLit => {
                let token = self.next().unwrap();
                let val = f64::from_str(&self.input[Range::from(token.span)]).unwrap();
                Expr::Float(val).spanned(token.span)
            }
            TokenType::StringLit => {
                let token = self.next().unwrap();
                let span = token.span;

                let val = self.input[span.start + 1..span.end - 1]
                    .replace("\\n", "\n")
                    .replace("\\\"", "\"")
                    .replace("\\\\", "\\");

                Expr::String(val).spanned(token.span)
            }
            TokenType::CharLit => {
                let token = self.next().unwrap();
                let span = token.span;

                let val = self.input[span.start + 1..span.end - 1]
                    .replace("\\n", "\n")
                    .replace("\\\'", "'")
                    .replace("\\\\", "\\")
                    .chars()
                    .next()
                    .unwrap();

                Expr::Char(val).spanned(token.span)
            }
            TokenType::True => Expr::Bool(true).spanned(self.next().unwrap().span),
            TokenType::False => Expr::Bool(false).spanned(self.next().unwrap().span),
            TokenType::LBracket => {
                let Spanned { inner: arr, span } = self.delimited_list(
                    Self::expression,
                    TokenType::LBracket,
                    TokenType::RBracket,
                )?;
                Expr::Array(arr).spanned(span)
            }
            TokenType::Ident => {
                let token = self.next().unwrap();
                let range: Range<_> = token.span.into();

                let ident = self.input[range].to_string();

                if self.consume_at(TokenType::Eq) {
                    let val = self.expression()?;

                    let end = val.span.end;

                    Expr::Assign {
                        ident: Spanned {
                            inner: ident,
                            span: token.span,
                        },
                        value: val.into(),
                    }
                    .spanned(token.span.start..end)
                } else {
                    Expr::Ident(ident).spanned(token.span)
                }
            }
            TokenType::If => {
                let start = self.next().unwrap().span.start;

                self.consume(TokenType::LParen)?;
                let cond = self.expression()?;
                self.consume(TokenType::RParen)?;

                let th = self.expression()?;

                let el = if self.consume_at(TokenType::Else) {
                    Some(Box::new(self.expression()?))
                } else {
                    None
                };

                let end = el.as_ref().map_or(th.span.end, |e| e.span.end);

                Expr::If {
                    cond: Box::new(cond),
                    th: Box::new(th),
                    el,
                }
                .spanned(start..end)
            }
            op @ (TokenType::Minus | TokenType::Bang) => {
                let op = match op {
                    TokenType::Minus => Unop::Neg,
                    TokenType::Bang => Unop::Not,
                    _ => unreachable!(),
                };

                let start = self.next().unwrap().span.start;

                let right_binding_power = op.binding_power();
                let expr = self.parse_expression(right_binding_power)?;

                let end = expr.span.end;

                Expr::UnaryOp {
                    op,
                    expr: Box::new(expr),
                }
                .spanned(start..end)
            }
            TokenType::Let => {
                let start = self.next().unwrap().span.start;

                let binding = self.binding()?;

                self.consume(TokenType::Eq)?;
                let value = self.expression()?;

                let end = value.span.end;

                Expr::Let {
                    binding,
                    value: Box::new(value),
                }
                .spanned(start..end)
            }
            TokenType::Fn => {
                let start = self.next().unwrap().span.start;

                let Spanned { inner: params, .. } =
                    self.delimited_list(Self::binding, TokenType::LParen, TokenType::RParen)?;

                let return_type = if self.consume_at(TokenType::Colon) {
                    Some(self.parse_ty()?)
                } else {
                    None
                };

                self.consume(TokenType::Arrow)?;

                let body = Box::new(self.expression()?);
                let end = body.span.end;

                Expr::Lambda {
                    params,
                    return_type,
                    body,
                }
                .spanned(start..end)
            }
            TokenType::LBrace => {
                let start = self.next().unwrap().span.start;

                let mut trailing = true;
                let mut exprs = Vec::new();
                while !self.at(TokenType::RBrace) {
                    exprs.push(self.expression()?);

                    if self.consume_at(TokenType::Semicolon) && self.at(TokenType::RBrace) {
                        trailing = false;
                        break;
                    }
                }
                let end = self.consume(TokenType::RBrace)?.span.end;

                Expr::Block { exprs, trailing }.spanned(start..end)
            }
            _ => {
                let token = self.next().unwrap();

                return Err(ParseError::Unexpected(
                    token.inner,
                    Some("start of expression".into()),
                )
                .spanned(token.span));
            }
        };
        loop {
            let op = match self.peek() {
                TokenType::Plus => Bop::Add,
                TokenType::Minus => Bop::Sub,
                TokenType::Times => Bop::Mul,
                TokenType::FSlash => Bop::Div,
                TokenType::Xor => Bop::Xor,
                TokenType::Ampersand => Bop::BAnd,
                TokenType::Pipe => Bop::BOr,
                TokenType::Exponent => Bop::Exp,
                TokenType::Eqq => Bop::Eqq,
                TokenType::Neq => Bop::Neq,
                TokenType::And => Bop::And,
                TokenType::Or => Bop::Or,
                TokenType::LAngle => Bop::Lt,
                TokenType::Leq => Bop::Leq,
                TokenType::RAngle => Bop::Gt,
                TokenType::Geq => Bop::Geq,
                TokenType::LBracket => {
                    self.next();

                    let start = lhs.span.start;

                    let index = Box::new(self.expression()?);
                    let end = self.consume(TokenType::RBracket)?.span.end;

                    lhs = Expr::Index {
                        arr: Box::new(lhs),
                        index,
                    }
                    .spanned(start..end);
                    continue;
                }
                TokenType::Dot => {
                    self.next();

                    let start = lhs.span.start;

                    let field = self.ident()?;
                    let end = field.span.end;

                    lhs = Expr::FieldAccess {
                        base: Box::new(lhs),
                        field,
                    }
                    .spanned(start..end);
                    continue;
                }
                TokenType::LParen => {
                    let start = lhs.span.start;

                    let Spanned {
                        inner: args,
                        span: Span { end, .. },
                    } = self.delimited_list(
                        Self::expression,
                        TokenType::LParen,
                        TokenType::RParen,
                    )?;

                    lhs = Expr::FnCall {
                        fun: Box::new(lhs),
                        args,
                    }
                    .spanned(start..end);
                    continue;
                }
                TokenType::Eof
                | TokenType::RParen
                | TokenType::RBrace
                | TokenType::RBracket
                | TokenType::Comma
                | TokenType::Semicolon
                | TokenType::Else
                | TokenType::Fn
                | TokenType::Const
                | TokenType::Struct
                | TokenType::Enum => break,
                _ => {
                    let token = self.next().unwrap();

                    return Err(ParseError::Unexpected(
                        token.inner,
                        Some("end of expression".into()),
                    )
                    .spanned(token.span));
                }
            };

            let (left_binding_power, right_binding_power) = op.binding_power();

            if left_binding_power < binding_power {
                break;
            }

            self.next();

            let rhs = self.parse_expression(right_binding_power)?;

            let start = lhs.span.start;
            let end = rhs.span.end;

            lhs = Expr::BinaryOp {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            }
            .spanned(start..end);
        }

        Ok(lhs)
    }
}
