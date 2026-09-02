use std::{debug_assert_matches, range::Range, str::CharIndices};

use itertools::Itertools as _;

use irs::ast::{Arg, BlockExpr, Expr, ExprKind, InfixOp, LitExpr, MatchArm, PrefixOp, Stmt};

use crate::{ErrorKind, Parser, Result, TokKind};

impl Parser<'_> {
    pub(crate) fn stmt(&mut self) -> Result<Stmt> {
        match self.peek()?.kind {
            TokKind::Let => {
                let start = self.consume(TokKind::Let)?.span.start;

                let binding = self.binding()?;
                self.consume(TokKind::Eq)?;
                let value = self.expr()?;

                let span = Range::from(start..value.span.end);
                Ok(Stmt::Decl {
                    binding,
                    value,
                    span,
                })
            }
            _ => self.expr().map(Stmt::Expr),
        }
    }

    pub(crate) fn expr(&mut self) -> Result<Expr> {
        self.expr_inner(0)
    }

    fn expr_inner(&mut self, ref_binding_power: u8) -> Result<Expr> {
        let mut lhs = match self.peek()?.kind {
            TokKind::Ident => self.var_expr(),
            TokKind::IntLit
            | TokKind::FloatLit
            | TokKind::StringLit
            | TokKind::True
            | TokKind::False => self
                .lit_expr()
                .map(|(lit, span)| ExprKind::Lit(lit).span(span)),
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
                let msg = match err_tok {
                    TokKind::Let => {
                        Some("`let` is a statement, and can only be used within a block")
                    }
                    TokKind::Match => Some("`match` is postfix"),
                    _ => None,
                };
                Err(self.unexpected(msg))
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

            self.next()
                .expect("this token was previously peeked, so we know it's valid"); // Skip over the already-parsed bop token

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

        let op = match self.peek()?.kind {
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
            TokKind::Lt => Some(InfixOp::Lt),
            TokKind::Leq => Some(InfixOp::Leq),
            TokKind::Gt => Some(InfixOp::Gt),
            TokKind::Geq => Some(InfixOp::Geq),
            _ => None,
        };

        Ok((lhs, op))
    }

    fn var_expr(&mut self) -> Result<Expr> {
        self.path()
            .map(|(path, span)| ExprKind::Var(path).span(span))
    }

    fn process_escapes(&self, src: &str, start: u32) -> Result<String> {
        let mut chars = src.char_indices();
        let mut out = String::new();
        while let Some((_, c)) = chars.next() {
            let c = match c {
                '\\' => match chars.next().expect("lexer produced standalone `\\`") {
                    (_, '\\') => '\\',
                    (_, '\'') => '\'',
                    (_, '"') => '"',
                    (_, '0') => '\0',
                    (_, 'n') => '\n',
                    (_, 'r') => '\r',
                    (_, 't') => '\t',
                    (i, 'u') => self.process_unicode_escape(
                        &mut chars,
                        start,
                        u32::try_from(i).expect("file too long") + 2,
                    )?,
                    (_, c) => c,
                },
                c => c,
            };
            out.push(c);
        }
        Ok(out)
    }

    fn process_unicode_escape(
        &self,
        chars: &mut CharIndices<'_>,
        start: u32,
        start_offset: u32,
    ) -> Result<char> {
        debug_assert_matches!(
            chars.next(),
            Some((_, '{')),
            "lexer produced invalid unicode escape"
        );

        let (mut end_offset, value) = chars
            .next()
            .expect("lexer produced unterminated unicode escape");
        let mut value = value
            .to_digit(16)
            .expect("lexer produced unicode escape with invalid digit");

        loop {
            match chars.next() {
                Some((_, '}')) => break,
                Some((i, c)) => {
                    let digit = c
                        .to_digit(16)
                        .expect("lexer produced unicode escape with invalid digit");
                    value = (value << 4u8) + digit;
                    end_offset = i;
                }
                None => {
                    unreachable!("lexer produced unterminated unicode escape")
                }
            }
        }

        let end_offset = u32::try_from(end_offset).expect("file too long");
        char::from_u32(value).ok_or_else(|| {
            self.err(
                ErrorKind::BadUnicodeEscape,
                Range::from(start + start_offset..start + end_offset + 1),
            )
        })
    }

    pub(crate) fn lit_expr(&mut self) -> Result<(LitExpr, Range<u32>)> {
        let tok = self.next()?;
        let src = self.src_of(tok);
        let lit = match tok.kind {
            TokKind::IntLit => {
                let num = match src.get(0..2) {
                    Some("0b") => u64::from_str_radix(&src[2..], 2),
                    Some("0o") => u64::from_str_radix(&src[2..], 8),
                    Some("0x") => u64::from_str_radix(&src[2..], 16),
                    _ => src.parse(),
                };
                LitExpr::Int(num.expect("ICE: lexer produced invalid int token"))
            }
            TokKind::FloatLit => LitExpr::Float(
                src.parse()
                    .expect("ICE: lexer produced invalid float token"),
            ),
            TokKind::StringLit => {
                if src.starts_with('#') {
                    LitExpr::String(src[2..src.len() - 2].to_string())
                } else {
                    LitExpr::String(
                        self.process_escapes(&src[1..src.len() - 1], tok.span.start + 1)?,
                    )
                }
            }
            TokKind::True => LitExpr::Bool(true),
            TokKind::False => LitExpr::Bool(false),
            _ => {
                return Err(self.err(ErrorKind::Unexpected(tok.kind, None), tok.span));
            }
        };
        Ok((lit, tok.span))
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
            span: Range::from(start..end),
        })
    }

    fn dot_suffixes(&mut self, lhs: Expr) -> Result<Expr> {
        self.consume(TokKind::Dot)?;

        match self.peek()?.kind {
            TokKind::Ident => {
                let start = lhs.span.start;
                let field = self.ident()?;

                if self.at_ws(TokKind::LParen) {
                    let start = lhs.span.start;

                    let (args, Range { end, .. }) =
                        self.delimited_list(Self::arg, TokKind::LParen, TokKind::RParen)?;

                    Ok(ExprKind::MethodCall {
                        base: Box::new(lhs),
                        method: field,
                        args,
                    }
                    .span(start..end))
                } else {
                    Ok(ExprKind::Field {
                        base: Box::new(lhs),
                        field,
                    }
                    .span(start..field.span.end))
                }
            }
            TokKind::Match => {
                self.consume(TokKind::Match)?;

                self.consume(TokKind::LBrace)?;
                let mut arms = Vec::new();
                while !self.at(TokKind::RBrace) {
                    let pat = self.pattern()?;
                    self.consume(TokKind::Arrow)?;
                    let body = self.expr()?;
                    arms.push(MatchArm { pat, body });
                }
                let arms_end = self.consume(TokKind::RBrace)?.span.end;

                let span = lhs.span.start..arms_end;
                Ok(ExprKind::Match {
                    scrutinee: Box::new(lhs),
                    arms,
                }
                .span(span))
            }
            _ => Err(self.unexpected(None)),
        }
    }

    fn call_suffix(&mut self, lhs: Expr) -> Result<Expr> {
        let start = lhs.span.start;

        let (args, Range { end, .. }) =
            self.delimited_list(Self::arg, TokKind::LParen, TokKind::RParen)?;

        Ok(ExprKind::Call {
            func: Box::new(lhs),
            args,
        }
        .span(start..end))
    }

    fn arg(&mut self) -> Result<Arg> {
        let mut_tok = self.consume_at(TokKind::Mut);
        let value = self.expr()?;

        let start = mut_tok.map_or(value.span.start, |tok| tok.span.start);
        let span = Range::from(start..value.span.end);

        Ok(Arg {
            mutable: mut_tok.is_some(),
            value,
            span,
        })
    }
}
