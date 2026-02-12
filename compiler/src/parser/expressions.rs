use std::{ops::Range, str::FromStr};

use crate::{
    helpers::{Span, Spanned},
    lexer::{Token, TokenType as TT},
    parser::{
        ParseError, ParseResult, Parser,
        ast::{Bop, Expr, ExprS, MatchArm, MatchArmS, Unop},
    },
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
            TT::Ident => self.ident_exprs(),
            TT::IntLit => self.int_lit_expr(),
            TT::FloatLit => self.float_lit_expr(),
            TT::CharLit => self.char_lit_expr(),
            TT::StringLit => self.string_lit_expr(),
            TT::True => Ok(Expr::Bool(true).spanned(self.consume(TT::True).unwrap().span)),
            TT::False => Ok(Expr::Bool(false).spanned(self.consume(TT::False).unwrap().span)),
            TT::LBracket => self.array_lit_expr(),
            TT::LParen => self.paren_exprs(),
            TT::If => self.if_expr(),
            TT::For => self.for_expr(),
            TT::While => self.while_expr(),
            TT::Match => self.match_expr(),
            TT::Minus | TT::Bang => self.unop_expr(),
            TT::Let => self.let_expr(),
            TT::Fn => self.lambda_expr(),
            TT::Indent => self.block_expr(),
            _ => {
                let token = self.next().unwrap();

                Err(ParseError::Unexpected(token.inner, "start of expression").spanned(token.span))
            }
        }?;

        loop {
            let op = match self.peek() {
                // Attach suffix to current lhs and re-loop
                TT::LBracket => {
                    lhs = self.index_suffix(lhs)?;
                    continue;
                }
                TT::Dot => {
                    lhs = self.field_suffix(lhs)?;
                    continue;
                }
                TT::LParen => {
                    lhs = self.call_suffix(lhs)?;
                    continue;
                }
                // Continue current iteration with given binop
                TT::Plus => Bop::Add,
                TT::Minus => Bop::Sub,
                TT::Times => Bop::Mul,
                TT::FSlash => Bop::Div,
                TT::Xor => Bop::Xor,
                TT::Ampersand => Bop::BAnd,
                TT::Pipe => Bop::BOr,
                TT::Exponent => Bop::Exp,
                TT::Eqq => Bop::Eqq,
                TT::Neq => Bop::Neq,
                TT::And => Bop::And,
                TT::Or => Bop::Or,
                TT::LAngle => Bop::Lt,
                TT::Leq => Bop::Leq,
                TT::RAngle => Bop::Gt,
                TT::Geq => Bop::Geq,
                // End of expression, terminate loop and return current lhs
                TT::Eof
                | TT::RParen // End of parenthesised expr
                | TT::Dedent // End of block expr
                | TT::RBracket // End of array index expr
                | TT::Comma // List delimiter
                | TT::Semicolon // Stmt seperator
                | TT::Then // Next part of compound expr
                | TT::Else
                | TT::Do
                | TT::With
                | TT::Fn // New item
                | TT::Const
                | TT::Struct
                | TT::Enum => return Ok(lhs),
                _ => {
                    // let token = self.next().unwrap();

                    // return Err(ParseError::Unexpected(token.inner, "end of expression")
                    //     .spanned(token.span));
                    return Ok(lhs)
                }
            };

            let (left_binding_power, right_binding_power) = op.binding_power();

            if left_binding_power < binding_power {
                return Ok(lhs);
            }

            self.next(); // Skip over the already-parsed bop token

            let rhs = self.expr_inner(right_binding_power)?;

            let span = lhs.span.start..rhs.span.end;

            lhs = Expr::BinaryOp {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            }
            .spanned(span);
        }
    }

    fn ident_exprs(&mut self) -> ParseResult<ExprS> {
        let ident_token = self.consume(TT::Ident)?;

        let ident = self.input[Range::from(ident_token.span)].to_string();

        if self.consume_at(TT::Eq) {
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

    fn int_lit_expr(&mut self) -> ParseResult<ExprS> {
        let span = self.consume(TT::IntLit)?.span;
        let val = u64::from_str(self.str_at(span)).unwrap();
        Ok(Expr::Int(val).spanned(span))
    }

    fn float_lit_expr(&mut self) -> ParseResult<ExprS> {
        let span = self.consume(TT::FloatLit)?.span;
        let val = f64::from_str(self.str_at(span)).unwrap();
        Ok(Expr::Float(val).spanned(span))
    }

    fn char_lit_expr(&mut self) -> ParseResult<ExprS> {
        let span = self.consume(TT::CharLit)?.span;
        let val = process_escapes(self.str_at(span.start + 1..span.end - 1))
            .chars()
            .next()
            .unwrap();
        Ok(Expr::Char(val).spanned(span))
    }

    fn string_lit_expr(&mut self) -> ParseResult<ExprS> {
        let span = self.consume(TT::StringLit)?.span;
        let val = process_escapes(self.str_at(span.start + 1..span.end - 1));
        Ok(Expr::String(val).spanned(span))
    }

    fn array_lit_expr(&mut self) -> ParseResult<ExprS> {
        let Spanned { inner: arr, span } =
            self.delimited_list(Self::expr, TT::LBracket, TT::RBracket)?;
        Ok(Expr::Array(arr).spanned(span))
    }

    fn paren_exprs(&mut self) -> ParseResult<ExprS> {
        let start = self.consume(TT::LParen)?.span.start;
        let expr = self.expr()?;

        let expr = if self.consume_at(TT::Comma) {
            let mut exprs = vec![expr];
            while !self.at(TT::RParen) {
                exprs.push(self.expr()?);

                if !self.consume_at(TT::Comma) {
                    break;
                }
            }

            Expr::Tuple(exprs)
        } else {
            expr.inner
        };

        let end = self.consume(TT::RParen)?.span.end;

        Ok(expr.spanned(start..end))
    }

    fn if_expr(&mut self) -> ParseResult<ExprS> {
        let start = self.consume(TT::If)?.span.start;

        let cond = self.expr()?;

        self.consume(TT::Then)?;

        let th = self.expr()?;

        let el = if self.consume_at(TT::Else) {
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

    fn for_expr(&mut self) -> ParseResult<ExprS> {
        let start = self.consume(TT::For)?.span.start;

        let pattern = self.pattern()?;

        self.consume(TT::In)?;

        let iter = self.expr()?;

        self.consume(TT::Do)?;

        let body = self.expr()?;

        let span = start..body.span.end;

        Ok(Expr::For {
            pattern,
            iter: Box::new(iter),
            body: Box::new(body),
        }
        .spanned(span))
    }

    fn while_expr(&mut self) -> ParseResult<ExprS> {
        let start = self.consume(TT::While)?.span.start;

        let cond = self.expr()?;

        self.consume(TT::Do)?;

        let body = self.expr()?;

        let span = start..body.span.end;

        Ok(Expr::While {
            cond: Box::new(cond),
            body: Box::new(body),
        }
        .spanned(span))
    }

    fn match_expr(&mut self) -> ParseResult<ExprS> {
        let start = self.consume(TT::Match)?.span.start;

        let scrutinee = self.expr()?;

        let with_end = self.consume(TT::With)?.span.end;

        let mut arms = Vec::new();
        while self.at(TT::Pipe) {
            arms.push(self.match_arm()?);
        }
        let end = arms.last().map_or(with_end, |arm| arm.span.end);

        let span = start..end;

        Ok(Expr::Match {
            scrutinee: Box::new(scrutinee),
            arms,
        }
        .spanned(span))
    }

    fn match_arm(&mut self) -> ParseResult<MatchArmS> {
        let start = self.consume(TT::Pipe)?.span.start;

        let pattern = self.pattern()?;

        let guard = if self.consume_at(TT::If) {
            Some(Box::new(self.expr()?))
        } else {
            None
        };

        self.consume(TT::Arrow)?;

        let body = self.expr()?;

        let span = start..body.span.end;

        Ok(MatchArm {
            pattern,
            guard,
            body: Box::new(body),
        }
        .spanned(span))
    }

    fn unop_expr(&mut self) -> ParseResult<ExprS> {
        let op_token = self.next().unwrap();

        let op = match op_token.inner {
            TT::Minus => Unop::Neg,
            TT::Bang => Unop::Not,
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
        let start = self.consume(TT::Let)?.span.start;

        let binding = self.pattern()?;

        self.consume(TT::Eq)?;
        let value = self.expr()?;

        let span = start..value.span.end;

        Ok(Expr::Let {
            binding,
            value: Box::new(value),
        }
        .spanned(span))
    }

    fn lambda_expr(&mut self) -> ParseResult<ExprS> {
        let start = self.consume(TT::Fn)?.span.start;

        let Spanned { inner: params, .. } =
            self.delimited_list(Self::pattern, TT::LParen, TT::RParen)?;

        let return_type = if self.consume_at(TT::Colon) {
            Some(self.parse_ty()?)
        } else {
            None
        };

        self.consume(TT::Arrow)?;

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
        let start = self.consume(TT::Indent)?.span.start;

        let mut trailing = true;

        let mut exprs = Vec::new();
        while !self.at(TT::Dedent) {
            exprs.push(self.expr()?);

            if self.consume_at(TT::Semicolon) && self.at(TT::Dedent) {
                trailing = false;
                break;
            }
        }

        let end = self.consume(TT::Dedent)?.span.end;

        Ok(Expr::Block { exprs, trailing }.spanned(start..end))
    }

    fn index_suffix(&mut self, lhs: ExprS) -> ParseResult<ExprS> {
        self.consume(TT::LBracket)?;

        let index = Box::new(self.expr()?);

        let start = lhs.span.start;
        let end = self.consume(TT::RBracket)?.span.end;

        Ok(Expr::Index {
            arr: Box::new(lhs),
            index,
        }
        .spanned(start..end))
    }

    fn field_suffix(&mut self, lhs: ExprS) -> ParseResult<ExprS> {
        self.consume(TT::Dot)?;

        let field = self.ident()?;
        let span = lhs.span.start..field.span.end;

        Ok(Expr::FieldAccess {
            base: Box::new(lhs),
            field,
        }
        .spanned(span))
    }

    fn call_suffix(&mut self, lhs: ExprS) -> ParseResult<ExprS> {
        let start = lhs.span.start;

        let Spanned {
            inner: args,
            span: Span { end, .. },
        } = self.delimited_list(Self::expr, TT::LParen, TT::RParen)?;

        Ok(Expr::FnCall {
            fun: Box::new(lhs),
            args,
        }
        .spanned(start..end))
    }
}
