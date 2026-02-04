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

fn process_escapes(input: &str) -> String {
    input
        .replace(r"\'", "\'")
        .replace(r#"\""#, "\"")
        .replace(r"\\", "\\")
        .replace(r"\n", "\n")
        .replace(r"\r", "\r")
        .replace(r"\t", "\t")
        .replace(r"\0", "\0")
}

impl<I: Iterator<Item = Token>> Parser<'_, I> {
    pub fn expr(&mut self) -> ParseResult<ExprS> {
        self.expr_inner(0)
    }

    fn expr_inner(&mut self, binding_power: u8) -> ParseResult<ExprS> {
        let mut lhs = match self.peek() {
            TokenType::IntLit => {
                let span = self.next().unwrap().span;
                let val = u64::from_str(self.str_at(span)).unwrap();
                Expr::Int(val).spanned(span)
            }
            TokenType::FloatLit => {
                let span = self.next().unwrap().span;
                let val = f64::from_str(self.str_at(span)).unwrap();
                Expr::Float(val).spanned(span)
            }
            TokenType::StringLit => {
                let span = self.next().unwrap().span;
                let val = process_escapes(self.str_at(span.start + 1..span.end - 1));
                Expr::String(val).spanned(span)
            }
            TokenType::CharLit => {
                let span = self.next().unwrap().span;
                let val = process_escapes(self.str_at(span.start + 1..span.end - 1))
                    .chars()
                    .next()
                    .unwrap();
                Expr::Char(val).spanned(span)
            }
            TokenType::True => Expr::Bool(true).spanned(self.next().unwrap().span),
            TokenType::False => Expr::Bool(false).spanned(self.next().unwrap().span),
            TokenType::LBracket => self.array_lit_expr()?,
            TokenType::Ident => self.ident_exprs()?,
            TokenType::LParen => self.paren_exprs()?,
            TokenType::If => self.if_expr()?,
            TokenType::Minus | TokenType::Bang => self.unop_expr()?,
            TokenType::Let => self.let_expr()?,
            TokenType::Fn => self.lambda_expr()?,
            TokenType::LBrace => self.block_expr()?,
            _ => {
                let token = self.next().unwrap();

                return Err(
                    ParseError::Unexpected(token.inner, "start of expression").spanned(token.span)
                );
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

                    let index = Box::new(self.expr()?);
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
                    } = self.delimited_list(Self::expr, TokenType::LParen, TokenType::RParen)?;

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

                    return Err(ParseError::Unexpected(token.inner, "end of expression")
                        .spanned(token.span));
                }
            };

            let (left_binding_power, right_binding_power) = op.binding_power();

            if left_binding_power < binding_power {
                break;
            }

            self.next();

            let rhs = self.expr_inner(right_binding_power)?;

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

    fn paren_exprs(&mut self) -> ParseResult<ExprS> {
        let start = self.next().unwrap().span.start;
        let expr = self.expr()?;

        let expr = if self.consume_at(TokenType::Comma) {
            let mut exprs = vec![expr];
            while !self.at(TokenType::RParen) {
                exprs.push(self.expr()?);

                if !self.consume_at(TokenType::Comma) {
                    break;
                }
            }

            Expr::Tuple(exprs)
        } else {
            expr.inner
        };

        let end = self.consume(TokenType::RParen)?.span.end;

        Ok(expr.spanned(start..end))
    }

    fn array_lit_expr(&mut self) -> ParseResult<ExprS> {
        let Spanned { inner: arr, span } =
            self.delimited_list(Self::expr, TokenType::LBracket, TokenType::RBracket)?;
        Ok(Expr::Array(arr).spanned(span))
    }

    fn ident_exprs(&mut self) -> ParseResult<ExprS> {
        let ident_token = self.next().unwrap();

        let ident = self.input[Range::from(ident_token.span)].to_string();

        if self.consume_at(TokenType::Eq) {
            let val = self.expr()?;

            let span = ident_token.span.start..val.span.end;

            Ok(Expr::Assign {
                ident: Spanned {
                    inner: ident,
                    span: ident_token.span,
                },
                value: val.into(),
            }
            .spanned(span))
        } else {
            Ok(Expr::Ident(ident).spanned(ident_token.span))
        }
    }

    fn if_expr(&mut self) -> ParseResult<ExprS> {
        let start = self.next().unwrap().span.start;

        self.consume(TokenType::LParen)?;
        let cond = self.expr()?;
        self.consume(TokenType::RParen)?;

        let th = self.expr()?;

        let el = if self.consume_at(TokenType::Else) {
            Some(Box::new(self.expr()?))
        } else {
            None
        };

        let end = el.as_ref().map_or(th.span.end, |e| e.span.end);

        Ok(Expr::If {
            cond: Box::new(cond),
            th: Box::new(th),
            el,
        }
        .spanned(start..end))
    }

    fn unop_expr(&mut self) -> ParseResult<ExprS> {
        let op_token = self.next().unwrap();

        let op = match op_token.inner {
            TokenType::Minus => Unop::Neg,
            TokenType::Bang => Unop::Not,
            _ => unreachable!("should only be called when next token is Minus or Bang"),
        };

        let right_binding_power = op.binding_power();
        let expr = self.expr_inner(right_binding_power)?;

        let span = op_token.span.start..expr.span.end;

        Ok(Expr::UnaryOp {
            op,
            expr: Box::new(expr),
        }
        .spanned(span))
    }

    fn let_expr(&mut self) -> ParseResult<ExprS> {
        let start = self.next().unwrap().span.start;

        let binding = self.binding()?;

        self.consume(TokenType::Eq)?;
        let value = self.expr()?;

        let span = start..value.span.end;

        Ok(Expr::Let {
            binding,
            value: Box::new(value),
        }
        .spanned(span))
    }

    fn lambda_expr(&mut self) -> ParseResult<ExprS> {
        let start = self.next().unwrap().span.start;

        let Spanned { inner: params, .. } =
            self.delimited_list(Self::binding, TokenType::LParen, TokenType::RParen)?;

        let return_type = if self.consume_at(TokenType::Colon) {
            Some(self.parse_ty()?)
        } else {
            None
        };

        self.consume(TokenType::Arrow)?;

        let body = Box::new(self.expr()?);
        let span = start..body.span.end;

        Ok(Expr::Lambda {
            params,
            return_type,
            body,
        }
        .spanned(span))
    }

    fn block_expr(&mut self) -> ParseResult<ExprS> {
        let start = self.next().unwrap().span.start;

        let mut trailing = true;

        let mut exprs = Vec::new();
        while !self.at(TokenType::RBrace) {
            exprs.push(self.expr()?);

            if self.consume_at(TokenType::Semicolon) && self.at(TokenType::RBrace) {
                trailing = false;
                break;
            }
        }

        let end = self.consume(TokenType::RBrace)?.span.end;

        Ok(Expr::Block { exprs, trailing }.spanned(start..end))
    }
}
