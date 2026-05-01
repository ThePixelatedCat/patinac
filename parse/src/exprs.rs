use itertools::Itertools;
use smallvec::{SmallVec, smallvec};
use std::str::FromStr;

use ast::{
    Path,
    exprs::{Arg, Expr, ExprKind, InfixOp, LitExpr, MatchArm, Stmt, UnaryOp},
};
use ident::{Ident, SpanIdent};
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

impl<'src, I: Iterator<Item = Tok<'src>>> Parser<'src, I> {
    pub(super) fn stmt(&mut self) -> Result<Stmt<(), Ident, Ident>> {
        match self.peek()? {
            TokKind::Let => {
                let start = self.consume(TokKind::Let)?.span.start;

                let binding = self.binding()?;
                self.consume(TokKind::Eq)?;
                let val = Box::new(self.expr()?);

                let span = Span::from(start..val.span.end);
                Ok(Stmt::Decl { binding, val, span })
            }
            _ => self.expr().map(Stmt::Expr),
        }
    }

    pub fn expr(&mut self) -> Result<Expr<(), Ident, Ident>> {
        self.expr_inner(0)
    }

    fn expr_inner(&mut self, binding_power: u8) -> Result<Expr<(), Ident, Ident>> {
        let mut lhs = match self.peek()? {
            TokKind::Ident => self.path_expr(),
            TokKind::IntLit
            | TokKind::FloatLit
            | TokKind::CharLit
            | TokKind::StringLit
            | TokKind::True
            | TokKind::False => self
                .lit_expr()
                .map(|(lit, span)| ExprKind::Lit(lit).span(span)),
            TokKind::LBracket => self.array_lit_expr(),
            TokKind::Hash => self.tuple_lit_expr(),
            TokKind::LBrace => self.block_expr(),
            TokKind::Minus | TokKind::Bang => self.unop_expr(),
            TokKind::Fn => self.lambda_expr(),
            TokKind::If => self.if_expr(),
            TokKind::Match => self.match_expr(),
            TokKind::For => self.for_expr(),
            TokKind::Loop => self.loop_expr(),
            TokKind::Break => self.break_expr(),
            TokKind::Continue => self.continue_expr(),
            TokKind::Return => self.return_expr(),
            _ => {
                let err_tok = self.next()?;
                let mut err = ErrorKind::Unexpected(err_tok.kind)
                    .span(err_tok.span)
                    .context("At start of expression");

                if err_tok.kind == TokKind::Let {
                    err = err.context("`let` is a statement, and can only be used within a block");
                }

                Err(err)
            }
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
                TokKind::Divide => InfixOp::Div,
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

    fn path_expr(&mut self) -> Result<Expr<(), Ident, Ident>> {
        let SpanIdent {
            ident: head,
            span: head_span,
        } = self.ident()?;

        let mut rest = SmallVec::new();
        let mut rest_end = 0;
        while self.consume_at(TokKind::PathSep).is_some() {
            let SpanIdent { ident, span } = self.ident()?;
            rest_end = span.end;
            rest.push(ident);
        }

        let (path, span) = rest.pop().map_or_else(
            || {
                (
                    Path {
                        prefix: smallvec![],
                        end: head,
                    },
                    head_span,
                )
            },
            |end| {
                rest.insert(0, head);
                (
                    Path { prefix: rest, end },
                    Span::from(head_span.start..rest_end),
                )
            },
        );

        Ok(ExprKind::Path(path).span(span))
    }

    pub fn lit_expr(&mut self) -> Result<(LitExpr, Span)> {
        match self.peek()? {
            TokKind::IntLit => self.int_lit_expr(),
            TokKind::FloatLit => self.float_lit_expr(),
            TokKind::CharLit => self.char_lit_expr(),
            TokKind::StringLit => self.string_lit_expr(),
            TokKind::True => Ok((LitExpr::Bool(true), self.consume(TokKind::True)?.span)),
            TokKind::False => Ok((LitExpr::Bool(false), self.consume(TokKind::False)?.span)),
            _ => Err(self.err_next(ErrorKind::Unexpected)),
        }
    }

    fn int_lit_expr(&mut self) -> Result<(LitExpr, Span)> {
        let tok = self.consume(TokKind::IntLit)?;
        let val = u64::from_str(tok.src).expect("lexer should not have produced invalid int token");
        Ok((LitExpr::Int(val), tok.span))
    }

    fn float_lit_expr(&mut self) -> Result<(LitExpr, Span)> {
        let tok = self.consume(TokKind::FloatLit)?;
        let val =
            f64::from_str(tok.src).expect("lexer should not have produced invalid float token");
        Ok((LitExpr::Float(val), tok.span))
    }

    fn char_lit_expr(&mut self) -> Result<(LitExpr, Span)> {
        let tok = self.consume(TokKind::CharLit)?;
        let val = process_escapes(&tok.src[1..tok.src.len() - 1])
            .chars()
            .exactly_one()
            .expect("lexer should not have produced char token with multiple characters");
        Ok((LitExpr::Char(val), tok.span))
    }

    fn string_lit_expr(&mut self) -> Result<(LitExpr, Span)> {
        let tok = self.consume(TokKind::StringLit)?;
        let val = process_escapes(&tok.src[1..tok.src.len() - 1]);
        Ok((LitExpr::String(val), tok.span))
    }

    fn array_lit_expr(&mut self) -> Result<Expr<(), Ident, Ident>> {
        self.delimited_list(Self::expr, TokKind::LBracket, TokKind::RBracket)
            .map(|(exprs, span)| ExprKind::Array(exprs).span(span))
    }

    fn tuple_lit_expr(&mut self) -> Result<Expr<(), Ident, Ident>> {
        let start = self.consume(TokKind::Hash)?.span.start;
        self.delimited_list(Self::expr, TokKind::LParen, TokKind::RParen)
            .map(|(exprs, span)| ExprKind::Tuple(exprs).span(start..span.end))
    }

    fn block_expr(&mut self) -> Result<Expr<(), Ident, Ident>> {
        let start = self.consume(TokKind::LBrace)?.span.start;

        let mut stmts = vec![];
        while !self.at(TokKind::RBrace) {
            stmts.push(self.stmt()?);
        }

        let end = self.consume(TokKind::RBrace)?.span.end;

        Ok(ExprKind::Block(stmts).span(start..end))
    }

    fn if_expr(&mut self) -> Result<Expr<(), Ident, Ident>> {
        let start = self.consume(TokKind::If)?.span.start;

        let cond = self.expr()?;

        self.consume(TokKind::Then)?;

        let th = self.expr()?;

        let el = self
            .consume_at(TokKind::Else)
            .map(|_| self.expr())
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

    fn for_expr(&mut self) -> Result<Expr<(), Ident, Ident>> {
        let start = self.consume(TokKind::For)?.span.start;

        let pat = self.pattern()?;

        self.consume(TokKind::In)?;

        let iter = self.expr()?;

        self.consume(TokKind::Do)?;

        let body = self.expr()?;

        let span = start..body.span.end;

        Ok(ExprKind::For {
            pat,
            iter: Box::new(iter),
            body: Box::new(body),
        }
        .span(span))
    }

    fn loop_expr(&mut self) -> Result<Expr<(), Ident, Ident>> {
        let start = self.consume(TokKind::Loop)?.span.start;

        let body = self.expr()?;

        let span = start..body.span.end;

        Ok(ExprKind::Loop(Box::new(body)).span(span))
    }

    fn match_expr(&mut self) -> Result<Expr<(), Ident, Ident>> {
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

    fn match_arm(&mut self) -> Result<MatchArm<(), Ident, Ident>> {
        let start = self.consume(TokKind::Pipe)?.span.start;

        let pat = self.pattern()?;
        self.consume(TokKind::Arrow)?;
        let body = self.expr()?;

        let span = Span::from(start..body.span.end);

        Ok(MatchArm { pat, body, span })
    }

    fn unop_expr(&mut self) -> Result<Expr<(), Ident, Ident>> {
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

    fn lambda_expr(&mut self) -> Result<Expr<(), Ident, Ident>> {
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

    fn return_expr(&mut self) -> Result<Expr<(), Ident, Ident>> {
        let start = self.consume(TokKind::Return)?.span.start;
        let expr = self.expr()?;
        let span = start..expr.span.end;
        Ok(ExprKind::Return(Box::new(expr)).span(span))
    }

    fn break_expr(&mut self) -> Result<Expr<(), Ident, Ident>> {
        let span = self.consume(TokKind::Break)?.span;
        Ok(ExprKind::Break.span(span))
    }

    fn continue_expr(&mut self) -> Result<Expr<(), Ident, Ident>> {
        let span = self.consume(TokKind::Continue)?.span;
        Ok(ExprKind::Continue.span(span))
    }

    fn dot_suffixes(&mut self, lhs: Expr<(), Ident, Ident>) -> Result<Expr<(), Ident, Ident>> {
        self.consume(TokKind::Dot)?;

        match self.peek()? {
            TokKind::Ident => {
                let field = self.ident()?;
                let span = lhs.span.start..field.span.end;

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
            _ => Err(self
                .err_next(ErrorKind::Unexpected)
                .context("Expected `[` or identifier")),
        }
    }

    fn call_suffix(&mut self, lhs: Expr<(), Ident, Ident>) -> Result<Expr<(), Ident, Ident>> {
        let start = lhs.span.start;

        let (args, Span { end, .. }) =
            self.delimited_list(Self::arg, TokKind::LParen, TokKind::RParen)?;

        Ok(ExprKind::CallExpr {
            func: Box::new(lhs),
            args,
        }
        .span(start..end))
    }

    fn arg(&mut self) -> Result<Arg<(), Ident, Ident>> {
        Ok(Arg {
            mutable: self.consume_at(TokKind::Mut).is_some(),
            val: self.expr()?,
        })
    }
}
