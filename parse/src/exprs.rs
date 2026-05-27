use std::str::FromStr;

use itertools::Itertools;

use ast::exprs::{Arg, BlockExpr, Expr, ExprKind, InfixOp, LitExpr, MatchArm, PrefixOp, Stmt};
use span::Span;

use crate::{ErrorKind, Parser, Result, TokKind};

impl Parser<'_> {
    pub(crate) fn stmt(&mut self) -> Result<Stmt> {
        match self.peek()? {
            TokKind::Let => {
                let start = self.consume(TokKind::Let)?.span.start;

                let binding = self.binding()?;
                self.consume(TokKind::Eq)?;
                let val = self.expr()?;

                let span = Span::from(start..val.span.end);
                Ok(Stmt::Decl { binding, val, span })
            }
            _ => self.expr().map(Stmt::Expr),
        }
    }

    pub(crate) fn expr(&mut self) -> Result<Expr> {
        self.expr_inner(0)
    }

    fn expr_inner(&mut self, ref_binding_power: u8) -> Result<Expr> {
        let mut lhs = match self.peek()? {
            TokKind::Ident => self.ident_expr(),
            TokKind::IntLit
            | TokKind::FloatLit
            | TokKind::CharLit
            | TokKind::StringLit
            | TokKind::True
            | TokKind::False => self
                .lit_expr()
                .map(|(lit, span)| ExprKind::Lit(lit).span(span)),
            TokKind::LBracket => self.array_lit_expr(),
            TokKind::LParen => self.tuple_lit_expr(),
            TokKind::Minus | TokKind::Bang => self.unop_expr(),
            TokKind::Fn => self.lambda_expr(),
            TokKind::If => self.if_expr(),
            TokKind::For => self.for_expr(),
            TokKind::Loop => self.loop_expr(),
            TokKind::Break => self.break_expr(),
            TokKind::Continue => self.continue_expr(),
            TokKind::Return => self.return_expr(),
            TokKind::LBrace => self.block_expr().map(|block| {
                let span = block.span;
                ExprKind::Block(block).span(span)
            }),
            TokKind::Print => {
                let start = self.consume(TokKind::Print)?.span.start;
                let expr = self.expr()?;
                let span = start..expr.span.end;
                Ok(ExprKind::Print(Box::new(expr)).span(span))
            }
            err_tok => {
                let ctx = match err_tok {
                    TokKind::Let => {
                        ["`let` is a statement, and can only be used within a block"].as_slice()
                    }
                    TokKind::Match => ["`match` is postfix"].as_slice(),
                    _ => [].as_slice(),
                };
                Err(self.err_next(ErrorKind::Unexpected, ctx))
            }
        }?;

        loop {
            let op = match self.get_op(lhs)? {
                (new_lhs, Some(op)) => {
                    lhs = new_lhs;
                    op
                }
                (new_lhs, None) => return Ok(new_lhs),
            };

            let (left_binding_power, right_binding_power) = op.binding_power();

            if left_binding_power < ref_binding_power {
                return Ok(lhs);
            }

            self.next().unwrap(); // Skip over the already-parsed bop token

            let rhs = self.expr_inner(right_binding_power)?;

            let span = lhs.span.start..rhs.span.end;

            lhs = ExprKind::Infix {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            }
            .span(span);
        }
    }

    fn get_op(&mut self, lhs: Expr) -> Result<(Expr, Option<InfixOp>)> {
        // Handle whitespace-sensitive part
        if self.at_ws(TokKind::LParen) {
            let lhs = self.call_suffix(lhs)?;
            return self.get_op(lhs);
        }

        let op = match self.peek()? {
            // Attach suffix to current lhs and re-loop
            TokKind::Dot => {
                let lhs = self.dot_suffixes(lhs)?;
                return self.get_op(lhs);
            }
            // Continue current iteration with given binop
            TokKind::Eq => Some(InfixOp::Assign),
            TokKind::Plus => Some(InfixOp::Add),
            TokKind::PlusF => Some(InfixOp::AddF),
            TokKind::Minus => Some(InfixOp::Sub),
            TokKind::MinusF => Some(InfixOp::SubF),
            TokKind::Times => Some(InfixOp::Mul),
            TokKind::TimesF => Some(InfixOp::MulF),
            TokKind::Divide => Some(InfixOp::Div),
            TokKind::DivideF => Some(InfixOp::DivF),
            TokKind::Exponent => Some(InfixOp::Exp),
            TokKind::Eqq => Some(InfixOp::Eqq),
            TokKind::Neq => Some(InfixOp::Neq),
            TokKind::And => Some(InfixOp::And),
            TokKind::Or => Some(InfixOp::Or),
            TokKind::Xor => Some(InfixOp::Xor),
            TokKind::Lt => Some(InfixOp::Lt),
            TokKind::Leq => Some(InfixOp::Leq),
            TokKind::Gt => Some(InfixOp::Gt),
            TokKind::Geq => Some(InfixOp::Geq),
            _ => None,
        };

        Ok((lhs, op))
    }

    fn ident_expr(&mut self) -> Result<Expr> {
        self.ident().map(|i| ExprKind::Ident(i.ident).span(i.span))
    }

    pub(crate) fn lit_expr(&mut self) -> Result<(LitExpr, Span)> {
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

        let tok = self.peek()?;
        let f = match tok {
            TokKind::IntLit => |src| {
                LitExpr::Int(
                    u64::from_str(src).expect("lexer should not have produced invalid int token"),
                )
            },
            TokKind::FloatLit => |src| {
                LitExpr::Float(
                    f64::from_str(src).expect("lexer should not have produced invalid float token"),
                )
            },
            TokKind::CharLit => {
                |src: &str| {
                    LitExpr::Char(process_escapes(&src[1..src.len() - 1])
                    .chars()
                    .exactly_one()
                    .expect("lexer should not have produced char token with multiple characters"))
                }
            }
            TokKind::StringLit => {
                |src: &str| LitExpr::String(process_escapes(&src[1..src.len() - 1]))
            }
            TokKind::True => |_| LitExpr::Bool(true),
            TokKind::False => |_| LitExpr::Bool(false),
            _ => {
                return Err(self.err_next(ErrorKind::Unexpected, &["Expected a literal"]));
            }
        };

        self.consume(tok).map(|tok| (f(self.src_of(tok)), tok.span))
    }

    fn array_lit_expr(&mut self) -> Result<Expr> {
        self.delimited_list(Self::expr, TokKind::LBracket, TokKind::RBracket)
            .map(|(exprs, span)| ExprKind::Array(exprs).span(span))
    }

    fn tuple_lit_expr(&mut self) -> Result<Expr> {
        self.delimited_list(Self::expr, TokKind::LParen, TokKind::RParen)
            .map(|(exprs, span)| ExprKind::Tuple(exprs).span(span))
    }

    fn if_expr(&mut self) -> Result<Expr> {
        let start = self.consume(TokKind::If)?.span.start;

        let cond = self.expr();
        let th = self.block_expr();
        let el = self
            .consume_at(TokKind::Else)
            .map(|_| self.block_expr())
            .transpose()?;
        let th = th?;
        let cond = Box::new(cond?);

        let end = el.as_ref().map_or(th.span.end, |el| el.span.end);
        Ok(ExprKind::If { cond, th, el }.span(start..end))
    }

    fn for_expr(&mut self) -> Result<Expr> {
        let start = self.consume(TokKind::For)?.span.start;

        let pat = self.pattern();
        self.consume(TokKind::In)?;
        let iter = self.expr();
        let body = self.block_expr()?;
        let iter = Box::new(iter?);
        let pat = pat?;

        let span = start..body.span.end;
        Ok(ExprKind::For { pat, iter, body }.span(span))
    }

    fn loop_expr(&mut self) -> Result<Expr> {
        let start = self.consume(TokKind::Loop)?.span.start;

        let body = self.block_expr()?;

        let span = start..body.span.end;
        Ok(ExprKind::Loop(body).span(span))
    }

    fn unop_expr(&mut self) -> Result<Expr> {
        let op_token = self.next()?;

        let op = match op_token.kind {
            TokKind::Minus => PrefixOp::Neg,
            TokKind::Bang => PrefixOp::Not,
            _ => unreachable!("should only be called when next token is Minus or Bang"),
        };

        let expr = Box::new(self.expr_inner(op.binding_power())?);

        let span = op_token.span.start..expr.span.end;

        Ok(ExprKind::Prefix { op, expr }.span(span))
    }

    fn lambda_expr(&mut self) -> Result<Expr> {
        let start = self.consume(TokKind::Fn)?.span.start;

        let (params, _) = self.delimited_list(Self::binding, TokKind::LParen, TokKind::RParen)?;
        self.consume(TokKind::Arrow)?;
        let body = Box::new(self.expr()?);

        let span = start..body.span.end;
        Ok(ExprKind::Lambda { params, body }.span(span))
    }

    fn return_expr(&mut self) -> Result<Expr> {
        let start = self.consume(TokKind::Return)?.span.start;
        let expr = self.expr()?;
        let span = start..expr.span.end;
        Ok(ExprKind::Return(Box::new(expr)).span(span))
    }

    fn break_expr(&mut self) -> Result<Expr> {
        let span = self.consume(TokKind::Break)?.span;
        Ok(ExprKind::Break.span(span))
    }

    fn continue_expr(&mut self) -> Result<Expr> {
        let span = self.consume(TokKind::Continue)?.span;
        Ok(ExprKind::Continue.span(span))
    }

    fn block_expr(&mut self) -> Result<BlockExpr> {
        let start = self.consume(TokKind::LBrace)?.span.start;

        let mut stmts = vec![];
        while !self.at(TokKind::RBrace) {
            stmts.push(self.stmt());
        }

        let end = self.consume(TokKind::RBrace)?.span.end;

        Ok(BlockExpr {
            stmts: stmts.into_iter().try_collect()?,
            span: Span::from(start..end),
        })
    }

    fn dot_suffixes(&mut self, lhs: Expr) -> Result<Expr> {
        self.consume(TokKind::Dot)?;

        match self.peek()? {
            TokKind::Ident => {
                let field = self.ident()?;

                let span = lhs.span.start..field.span.end;
                Ok(ExprKind::Field {
                    base: Box::new(lhs),
                    field,
                }
                .span(span))
            }
            TokKind::LBracket => {
                self.consume(TokKind::LBracket)?;
                let idx = Box::new(self.expr()?);

                let span = lhs.span.start..self.consume(TokKind::RBracket)?.span.end;
                Ok(ExprKind::Index {
                    arr: Box::new(lhs),
                    idx,
                }
                .span(span))
            }
            TokKind::Match => {
                self.consume(TokKind::Match)?;
                let (arms, arms_span) = self.delimited_list(
                    |this| {
                        let pat = this.pattern()?;
                        this.consume(TokKind::Arrow)?;
                        let body = this.expr()?;

                        Ok(MatchArm { pat, body })
                    },
                    TokKind::LBrace,
                    TokKind::RBrace,
                )?;

                let span = lhs.span.start..arms_span.end;
                Ok(ExprKind::Match {
                    scrutinee: Box::new(lhs),
                    arms,
                }
                .span(span))
            }
            _ => Err(self.err_next(
                ErrorKind::Unexpected,
                &["Expected indexing, match, or field access"],
            )),
        }
    }

    fn call_suffix(&mut self, lhs: Expr) -> Result<Expr> {
        let start = lhs.span.start;

        let (args, Span { end, .. }) =
            self.delimited_list(Self::arg, TokKind::LParen, TokKind::RParen)?;

        Ok(ExprKind::Call {
            func: Box::new(lhs),
            args,
        }
        .span(start..end))
    }

    fn arg(&mut self) -> Result<Arg> {
        let mut_tok = self.consume_at(TokKind::Mut);
        let val = self.expr()?;

        let start = mut_tok.map_or(val.span.start, |tok| tok.span.start);
        let span = Span::from(start..val.span.end);

        Ok(Arg {
            mutable: mut_tok.is_some(),
            val,
            span,
        })
    }
}
