use std::str::FromStr;

use ast::{Bop, Expr as _Expr, ExprKind, MatchArm, Unop};
use lex::{Tok, TokKind};
use span::Span;

use crate::{ParseError, ParseResult, Parser};

type Expr = _Expr<()>;

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
    pub fn expr(&mut self) -> ParseResult<Expr> {
        self.expr_inner(0)
    }

    fn expr_inner(&mut self, binding_power: u8) -> ParseResult<Expr> {
        let mut lhs = match self.peek() {
            TokKind::Ident => self.ident_exprs(),
            TokKind::IntLit => self.int_lit_expr(),
            TokKind::FloatLit => self.float_lit_expr(),
            TokKind::CharLit => self.char_lit_expr(),
            TokKind::StringLit => self.string_lit_expr(),
            TokKind::True => {
                Ok(ExprKind::Bool(true).span(self.consume(TokKind::True).unwrap().span))
            }
            TokKind::False => {
                Ok(ExprKind::Bool(false).span(self.consume(TokKind::False).unwrap().span))
            }
            TokKind::LBracket => self.array_lit_expr(),
            TokKind::LParen => self.paren_exprs(),
            TokKind::If => self.if_expr(),
            TokKind::For => self.for_expr(),
            TokKind::While => self.while_expr(),
            TokKind::Match => self.match_expr(),
            TokKind::Minus | TokKind::Bang => self.unop_expr(),
            TokKind::Let => self.let_expr(),
            TokKind::Fn => self.lambda_expr(),
            TokKind::LBrace => self.block_expr(),
            _ => Err(self.err_next(|tk| ParseError::Unexpected(tk, "start of expression"))),
        }?;

        loop {
            let op = match self.peek() {
                // Attach suffix to current lhs and re-loop
                TokKind::LBracket => {
                    lhs = self.index_suffix(lhs)?;
                    continue;
                }
                TokKind::Dot => {
                    lhs = self.field_suffix(lhs)?;
                    continue;
                }
                TokKind::LParen => {
                    lhs = self.call_suffix(lhs)?;
                    continue;
                }
                // Continue current iteration with given binop
                TokKind::Plus => Bop::Add,
                TokKind::Minus => Bop::Sub,
                TokKind::Times => Bop::Mul,
                TokKind::FSlash => Bop::Div,
                TokKind::Xor => Bop::Xor,
                TokKind::Ampersand => Bop::BAnd,
                TokKind::Pipe => Bop::BOr,
                TokKind::Exponent => Bop::Exp,
                TokKind::Eqq => Bop::Eqq,
                TokKind::Neq => Bop::Neq,
                TokKind::And => Bop::And,
                TokKind::Or => Bop::Or,
                TokKind::Lt => Bop::Lt,
                TokKind::Leq => Bop::Leq,
                TokKind::Gt => Bop::Gt,
                TokKind::Geq => Bop::Geq,
                // Stmt seperator, consume then terminate + return
                TokKind::Semicolon => {
                    self.next();
                    return Ok(lhs);
                }
                _ => return Ok(lhs),
            };

            let (left_binding_power, right_binding_power) = op.binding_power();

            if left_binding_power < binding_power {
                return Ok(lhs);
            }

            self.next(); // Skip over the already-parsed bop token

            let rhs = self.expr_inner(right_binding_power)?;

            let span = lhs.span.start..rhs.span.end;

            lhs = ExprKind::BinOp {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            }
            .span(span);
        }
    }

    fn ident_exprs(&mut self) -> ParseResult<Expr> {
        let ident = self.ident()?;
        let ident_span = ident.1;

        if self.consume_at(TokKind::Eq) {
            let val = self.expr()?;

            let span = ident_span.start..val.span.end;

            Ok(ExprKind::Assign {
                ident,
                val: Box::new(val),
            }
            .span(span))
        } else {
            Ok(ExprKind::Ident(ident.0).span(ident_span))
        }
    }

    fn int_lit_expr(&mut self) -> ParseResult<Expr> {
        let span = self.consume(TokKind::IntLit)?.span;
        let val = u64::from_str(self.str_at(span)).unwrap();
        Ok(ExprKind::Int(val).span(span))
    }

    fn float_lit_expr(&mut self) -> ParseResult<Expr> {
        let span = self.consume(TokKind::FloatLit)?.span;
        let val = f64::from_str(self.str_at(span)).unwrap();
        Ok(ExprKind::Float(val).span(span))
    }

    fn char_lit_expr(&mut self) -> ParseResult<Expr> {
        let span = self.consume(TokKind::CharLit)?.span;
        let val = process_escapes(self.str_at(span.start + 1..span.end - 1))
            .chars()
            .next()
            .unwrap();
        Ok(ExprKind::Char(val).span(span))
    }

    fn string_lit_expr(&mut self) -> ParseResult<Expr> {
        let span = self.consume(TokKind::StringLit)?.span;
        let val = process_escapes(self.str_at(span.start + 1..span.end - 1));
        Ok(ExprKind::String(val).span(span))
    }

    fn array_lit_expr(&mut self) -> ParseResult<Expr> {
        let (arr, span) = self.delimited_list(Self::expr, TokKind::LBracket, TokKind::RBracket)?;
        Ok(ExprKind::Array(arr).span(span))
    }

    fn paren_exprs(&mut self) -> ParseResult<Expr> {
        let start = self.consume(TokKind::LParen)?.span.start;
        let expr = self.expr()?;

        let expr = if self.consume_at(TokKind::Comma) {
            let mut exprs = vec![expr];
            while !self.at(TokKind::RParen) {
                exprs.push(self.expr()?);

                if !self.consume_at(TokKind::Comma) {
                    break;
                }
            }

            ExprKind::Tuple(exprs)
        } else {
            expr.kind
        };

        let end = self.consume(TokKind::RParen)?.span.end;

        Ok(expr.span(start..end))
    }

    fn if_expr(&mut self) -> ParseResult<Expr> {
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

    fn for_expr(&mut self) -> ParseResult<Expr> {
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

    fn while_expr(&mut self) -> ParseResult<Expr> {
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

    fn match_expr(&mut self) -> ParseResult<Expr> {
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

    fn match_arm(&mut self) -> ParseResult<MatchArm<()>> {
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

    fn unop_expr(&mut self) -> ParseResult<Expr> {
        let op_token = self.next().unwrap();

        let op = match op_token.kind {
            TokKind::Minus => Unop::Neg,
            TokKind::Bang => Unop::Not,
            _ => unreachable!("should only be called when next token is Minus or Bang"),
        };

        let right_binding_power = op.binding_power();
        let expr = self.expr_inner(right_binding_power)?;

        let span = op_token.span.start..expr.span.end;

        Ok(ExprKind::UnaryOp {
            op,
            expr: Box::new(expr),
        }
        .span(span))
    }

    fn let_expr(&mut self) -> ParseResult<Expr> {
        let start = self.consume(TokKind::Let)?.span.start;

        let binding = self.binding()?;

        self.consume(TokKind::Eq)?;
        let value = self.expr()?;

        let span = start..value.span.end;

        Ok(ExprKind::Let {
            binding,
            val: Box::new(value),
        }
        .span(span))
    }

    fn lambda_expr(&mut self) -> ParseResult<Expr> {
        let start = self.consume(TokKind::Fn)?.span.start;

        let (params, _) = self.delimited_list(Self::binding, TokKind::LParen, TokKind::RParen)?;

        let return_type = self.ty_annot()?;

        self.consume(TokKind::Arrow)?;

        let body = Box::new(self.expr()?);
        let span = start..body.span.end;

        Ok(ExprKind::Lambda {
            params,
            return_ty: return_type,
            body,
        }
        .span(span))
    }

    fn block_expr(&mut self) -> ParseResult<Expr> {
        let start = self.consume(TokKind::LBrace)?.span.start;

        let mut exprs = Vec::new();
        while !self.at(TokKind::RBrace) {
            exprs.push(self.expr()?);
        }

        let end = self.consume(TokKind::RBrace)?.span.end;

        Ok(ExprKind::Block(exprs).span(start..end))
    }

    fn index_suffix(&mut self, lhs: Expr) -> ParseResult<Expr> {
        self.consume(TokKind::LBracket)?;

        let index = Box::new(self.expr()?);

        let start = lhs.span.start;
        let end = self.consume(TokKind::RBracket)?.span.end;

        Ok(ExprKind::Index {
            arr: Box::new(lhs),
            idx: index,
        }
        .span(start..end))
    }

    fn field_suffix(&mut self, lhs: Expr) -> ParseResult<Expr> {
        self.consume(TokKind::Dot)?;

        let field = self.ident()?;
        let span = lhs.span.start..field.1.end;

        Ok(ExprKind::FieldAccess {
            base: Box::new(lhs),
            field,
        }
        .span(span))
    }

    fn call_suffix(&mut self, lhs: Expr) -> ParseResult<Expr> {
        let start = lhs.span.start;

        let (args, Span { end, .. }) =
            self.delimited_list(Self::expr, TokKind::LParen, TokKind::RParen)?;

        Ok(ExprKind::App {
            func: Box::new(lhs),
            args,
        }
        .span(start..end))
    }
}
