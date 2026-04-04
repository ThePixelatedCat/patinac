use std::str::FromStr;

use ast::exprs::{Arg, Expr, ExprKind, InfixOp, LitExpr, MatchArm, UnaryOp};
use lex::{Tok, TokKind};
use span::Span;

use crate::{ErrorKind, Parser, Result};

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

impl<I: Iterator<Item = Tok>> Parser<'_, I> {
    pub fn expr(&mut self) -> Result<Expr<()>> {
        self.expr_inner(0)
    }

    fn expr_inner(&mut self, binding_power: u8) -> Result<Expr<()>> {
        let mut lhs =
            match self.peek()? {
                TokKind::Ident => self.ident_expr(),
                TokKind::IntLit => self.int_lit_expr(),
                TokKind::FloatLit => self.float_lit_expr(),
                TokKind::CharLit => self.char_lit_expr(),
                TokKind::StringLit => self.string_lit_expr(),
                TokKind::True => Ok(ExprKind::Lit(LitExpr::Bool(true))
                    .span(self.consume(TokKind::True).unwrap().span)),
                TokKind::False => Ok(ExprKind::Lit(LitExpr::Bool(false))
                    .span(self.consume(TokKind::False).unwrap().span)),
                TokKind::LBracket => self.array_lit_expr(),
                TokKind::LBrace => self.brace_exprs(),
                TokKind::Minus | TokKind::Bang => self.unop_expr(),
                TokKind::Fn => self.lambda_expr(),
                TokKind::Let => self.let_expr(),
                TokKind::If => self.if_expr(),
                TokKind::Match => self.match_expr(),
                TokKind::For => self.for_expr(),
                TokKind::While => self.while_expr(),
                TokKind::Break => self.break_expr(),
                TokKind::Continue => self.continue_expr(),
                TokKind::Return => self.return_expr(),
                _ => Err(self.err_next(|tk| ErrorKind::Unexpected(tk, "start of expression"))),
            }?;

        loop {
            let Ok(peeked) = self.peek() else { break };
            let op = match peeked {
                // Attach suffix to current lhs and re-loop
                TokKind::Dot => {
                    lhs = self.dot_suffixes(lhs)?;
                    continue;
                }
                TokKind::LParen => {
                    lhs = self.call_suffix(lhs)?;
                    continue;
                }
                // Continue current iteration with given binop
                TokKind::Eq => InfixOp::Assign,
                TokKind::Plus => InfixOp::Add,
                TokKind::Minus => InfixOp::Sub,
                TokKind::Times => InfixOp::Mul,
                TokKind::FSlash => InfixOp::Div,
                TokKind::Xor => InfixOp::Xor,
                TokKind::Exponent => InfixOp::Exp,
                TokKind::Eqq => InfixOp::Eqq,
                TokKind::Neq => InfixOp::Neq,
                TokKind::And => InfixOp::And,
                TokKind::Or => InfixOp::Or,
                TokKind::Lt => InfixOp::Lt,
                TokKind::Leq => InfixOp::Leq,
                TokKind::Gt => InfixOp::Gt,
                TokKind::Geq => InfixOp::Geq,
                _ => break,
            };

            let (left_binding_power, right_binding_power) = op.binding_power();

            if left_binding_power < binding_power {
                break;
            }

            self.next()?; // Skip over the already-parsed bop token

            let rhs = self.expr_inner(right_binding_power)?;

            let span = lhs.span.start..rhs.span.end;

            lhs = ExprKind::InfixExpr {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            }
            .span(span);
        }
        Ok(lhs)
    }

    fn ident_expr(&mut self) -> Result<Expr<()>> {
        self.ident()
            .map(|(ident, ident_span)| ExprKind::Ident(ident).span(ident_span))
    }

    fn int_lit_expr(&mut self) -> Result<Expr<()>> {
        let span = self.consume(TokKind::IntLit)?.span;
        let val = u64::from_str(self.str_at(span)).unwrap();
        Ok(ExprKind::Lit(LitExpr::Int(val)).span(span))
    }

    fn float_lit_expr(&mut self) -> Result<Expr<()>> {
        let span = self.consume(TokKind::FloatLit)?.span;
        let val = f64::from_str(self.str_at(span)).unwrap();
        Ok(ExprKind::Lit(LitExpr::Float(val)).span(span))
    }

    fn char_lit_expr(&mut self) -> Result<Expr<()>> {
        let span = self.consume(TokKind::CharLit)?.span;
        let val = process_escapes(self.str_at(span.start + 1..span.end - 1))
            .chars()
            .next()
            .unwrap();
        Ok(ExprKind::Lit(LitExpr::Char(val)).span(span))
    }

    fn string_lit_expr(&mut self) -> Result<Expr<()>> {
        let span = self.consume(TokKind::StringLit)?.span;
        let val = process_escapes(self.str_at(span.start + 1..span.end - 1));
        Ok(ExprKind::Lit(LitExpr::String(val)).span(span))
    }

    fn array_lit_expr(&mut self) -> Result<Expr<()>> {
        self.delimited_list(Self::expr, TokKind::LBracket, TokKind::RBracket)
            .map(|(arr, span)| ExprKind::Array(arr).span(span))
    }

    fn brace_exprs(&mut self) -> Result<Expr<()>> {
        let start = self.consume(TokKind::LBrace)?.span.start;

        if let Some(brace) = self.consume_get_at(TokKind::RBrace) {
            return Ok(ExprKind::Tuple(vec![]).span(start..brace.span.end));
        }

        let mut exprs = vec![self.expr()?];
        let tuple = self.consume_at(TokKind::Comma);

        while !self.at(TokKind::RBrace) {
            exprs.push(self.expr()?);

            if tuple && !self.consume_at(TokKind::Comma) {
                break;
            }
        }

        let end = self.consume(TokKind::RBrace)?.span.end;

        let expr = if tuple {
            ExprKind::Tuple(exprs)
        } else {
            ExprKind::Block(exprs)
        };
        Ok(expr.span(start..end))
    }

    fn if_expr(&mut self) -> Result<Expr<()>> {
        let start = self.consume(TokKind::If)?.span.start;

        let cond = self.expr()?;

        self.consume(TokKind::Then)?;

        let th = self.expr()?;

        let el = self
            .consume_at(TokKind::Else)
            .then(|| self.expr())
            .transpose()?
            .map(Box::new);

        let end = el.as_ref().map_or(th.span.end, |e| e.span.end);

        Ok(ExprKind::If {
            cond: Box::new(cond),
            th: Box::new(th),
            el,
        }
        .span(start..end))
    }

    fn for_expr(&mut self) -> Result<Expr<()>> {
        let start = self.consume(TokKind::For)?.span.start;

        let pattern = self.pattern()?;

        self.consume(TokKind::In)?;

        let iter = self.expr()?;

        self.consume(TokKind::Do)?;

        let body = self.expr()?;

        let span = start..body.span.end;

        Ok(ExprKind::For {
            pattern,
            iter: Box::new(iter),
            body: Box::new(body),
        }
        .span(span))
    }

    fn while_expr(&mut self) -> Result<Expr<()>> {
        let start = self.consume(TokKind::While)?.span.start;

        let cond = self.expr()?;

        self.consume(TokKind::Do)?;

        let body = self.expr()?;

        let span = start..body.span.end;

        Ok(ExprKind::While {
            cond: Box::new(cond),
            body: Box::new(body),
        }
        .span(span))
    }

    fn match_expr(&mut self) -> Result<Expr<()>> {
        let start = self.consume(TokKind::Match)?.span.start;

        let scrutinee = self.expr()?;

        let with_end = self.consume(TokKind::With)?.span.end;

        let mut arms = Vec::new();
        while self.at(TokKind::Pipe) {
            arms.push(self.match_arm()?);
        }
        let end = arms.last().map_or(with_end, |arm| arm.span.end);

        let span = start..end;

        Ok(ExprKind::Match {
            scrutinee: Box::new(scrutinee),
            arms,
        }
        .span(span))
    }

    fn match_arm(&mut self) -> Result<MatchArm<()>> {
        let start = self.consume(TokKind::Pipe)?.span.start;

        let pattern = self.pattern()?;

        let guard = self
            .consume_at(TokKind::If)
            .then(|| self.expr())
            .transpose()?
            .map(Box::new);

        self.consume(TokKind::Arrow)?;

        let body = Box::new(self.expr()?);

        let span = Span::from(start..body.span.end);

        Ok(MatchArm {
            pattern,
            guard,
            body,
            span,
        })
    }

    fn unop_expr(&mut self) -> Result<Expr<()>> {
        let op_token = self.next()?;

        let op = match op_token.kind {
            TokKind::Minus => UnaryOp::Neg,
            TokKind::Bang => UnaryOp::Not,
            _ => unreachable!("should only be called when next token is Minus or Bang"),
        };

        let expr = self.expr_inner(op.binding_power())?;

        let span = op_token.span.start..expr.span.end;

        Ok(ExprKind::UnaryExpr {
            op,
            expr: Box::new(expr),
        }
        .span(span))
    }

    fn let_expr(&mut self) -> Result<Expr<()>> {
        let start = self.consume(TokKind::Let)?.span.start;

        let binding = self.binding()?;
        self.consume(TokKind::Eq)?;
        let val = self.expr()?;

        let span = start..val.span.end;
        Ok(ExprKind::Let {
            binding,
            val: Box::new(val),
        }
        .span(span))
    }

    fn lambda_expr(&mut self) -> Result<Expr<()>> {
        let start = self.consume(TokKind::Fn)?.span.start;

        let (params, _) = self.delimited_list(Self::binding, TokKind::LParen, TokKind::RParen)?;

        let return_ty = self.ty_annot()?;

        self.consume(TokKind::Arrow)?;

        let body = Box::new(self.expr()?);

        let span = start..body.span.end;
        Ok(ExprKind::LambdaExpr {
            params,
            return_ty,
            body,
        }
        .span(span))
    }

    fn return_expr(&mut self) -> Result<Expr<()>> {
        let start = self.consume(TokKind::Return)?.span.start;
        let expr = self.expr()?;
        let span = start..expr.span.end;
        Ok(ExprKind::Return(Box::new(expr)).span(span))
    }

    fn break_expr(&mut self) -> Result<Expr<()>> {
        let span = self.consume(TokKind::Break)?.span;
        Ok(ExprKind::Break.span(span))
    }

    fn continue_expr(&mut self) -> Result<Expr<()>> {
        let span = self.consume(TokKind::Continue)?.span;
        Ok(ExprKind::Continue.span(span))
    }

    fn dot_suffixes(&mut self, lhs: Expr<()>) -> Result<Expr<()>> {
        self.consume(TokKind::Dot)?;

        match self.peek()? {
            TokKind::Ident => {
                let (field, field_span) = self.ident()?;
                let span = lhs.span.start..field_span.end;

                Ok(ExprKind::FieldExpr {
                    base: Box::new(lhs),
                    field,
                }
                .span(span))
            }
            TokKind::LBracket => {
                self.consume(TokKind::LBracket)?;
                let idx = Box::new(self.expr()?);

                let span = lhs.span.start..self.consume(TokKind::RBracket)?.span.end;
                Ok(ExprKind::IndexExpr {
                    arr: Box::new(lhs),
                    idx,
                }
                .span(span))
            }
            _ => Err(self.err_next(|tk| ErrorKind::Unexpected(tk, "following dot in place"))),
        }
    }

    fn call_suffix(&mut self, lhs: Expr<()>) -> Result<Expr<()>> {
        let start = lhs.span.start;

        let (args, Span { end, .. }) =
            self.delimited_list(Self::arg, TokKind::LParen, TokKind::RParen)?;

        Ok(ExprKind::CallExpr {
            func: Box::new(lhs),
            args,
        }
        .span(start..end))
    }

    fn arg(&mut self) -> Result<Arg<()>> {
        let mutable = self.consume_at(TokKind::Mut);

        let label = self
            .consume_at(TokKind::Dot)
            .then(|| {
                let label = self.pattern()?;
                self.consume(TokKind::Eq)?;
                Ok(label)
            })
            .transpose()?;

        let val = self.expr()?;

        Ok(Arg {
            mutable,
            label,
            val,
        })
    }
}
