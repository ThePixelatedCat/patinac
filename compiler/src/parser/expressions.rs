use super::{
    ParseError, ParseResult, Parser, Token,
    ast::{Bop, Expr, Lit, Unop},
};

trait PrefixOperator {
    fn binding_power(&self) -> u8;
}

trait InfixOperator {
    fn binding_power(&self) -> (u8, u8);
}

impl PrefixOperator for Unop {
    fn binding_power(&self) -> u8 {
        match self {
            Unop::Neg | Unop::Not => 51,
        }
    }
}

impl InfixOperator for Bop {
    fn binding_power(&self) -> (u8, u8) {
        match self {
            Bop::Assign => (1, 2),
            Bop::Or => (3, 4),
            Bop::And => (5, 6),
            Bop::Eqq | Bop::Neq => (7, 8),
            Bop::Gt | Bop::Lt | Bop::Leq | Bop::Geq => (9, 10),
            Bop::BOr => (11, 12),
            Bop::Xor => (13, 14),
            Bop::BAnd => (15, 16),
            Bop::Add | Bop::Sub => (17, 18),
            Bop::Mul | Bop::Div => (19, 20),
            Bop::Exp => (22, 21),
        }
    }
}

impl<I: Iterator<Item = Token>> Parser<I> {
    pub fn expression(&mut self) -> ParseResult<Expr> {
        self.parse_expression(0)
    }

    fn parse_expression(&mut self, binding_power: u8) -> ParseResult<Expr> {
        let mut lhs = match self.peek() {
            Token::LParen => {
                self.next();
                let expr = self.expression()?;
                if self.consume_at(&Token::Comma) {
                    let mut exprs = vec![expr];
                    while !self.at(&Token::RParen) {
                        exprs.push(self.expression()?);

                        if !self.consume_at(&Token::Comma) {
                            break;
                        }
                    }
                    self.consume(&Token::RParen)?;

                    Expr::Literal(Lit::Tuple(exprs))
                } else {
                    self.consume(&Token::RParen)?;
                    expr
                }
            }
            Token::IntLit(_)
            | Token::FloatLit(_)
            | Token::StringLit(_)
            | Token::CharLit(_)
            | Token::True
            | Token::False => {
                let lit = match self.next().unwrap() {
                    Token::IntLit(int) => Lit::Int(int),
                    Token::FloatLit(float) => Lit::Float(float),
                    Token::StringLit(string) => Lit::Str(string),
                    Token::CharLit(char) => Lit::Char(char),
                    Token::True => Lit::Bool(true),
                    Token::False => Lit::Bool(false),
                    _ => unreachable!(),
                };
                Expr::Literal(lit)
            }
            Token::Ident(_) => {
                let Some(Token::Ident(ident)) = self.next() else {
                    unreachable!()
                };

                Expr::Ident(ident)
            }
            Token::If => {
                self.next();
                self.consume(&Token::LParen)?;
                let cond = self.expression()?;
                self.consume(&Token::RParen)?;

                let th = self.expression()?;

                let el = if self.consume_at(&Token::Else) {
                    Some(Box::new(self.expression()?))
                } else {
                    None
                };

                Expr::If {
                    cond: Box::new(cond),
                    th: Box::new(th),
                    el,
                }
            }
            op @ (Token::Minus | Token::Bang) => {
                let op = match op {
                    Token::Minus => Unop::Neg,
                    Token::Bang => Unop::Not,
                    _ => unreachable!(),
                };

                self.next();

                let right_binding_power = op.binding_power();
                let expr = self.parse_expression(right_binding_power)?;
                Expr::UnaryOp {
                    op,
                    expr: Box::new(expr),
                }
            }
            Token::Let => {
                self.next();

                let binding = self.binding()?;

                self.consume(&Token::Eq)?;
                let value = self.expression()?;

                Expr::Let {
                    binding,
                    value: Box::new(value),
                }
            }
            Token::Pipe => {
                let params = self.delimited_list(Self::binding, &Token::Pipe, &Token::Pipe)?;

                let return_type = if self.consume_at(&Token::Colon) {
                    Some(self.type_()?)
                } else {
                    None
                };

                self.consume(&Token::Arrow)?;

                let body = Box::new(self.expression()?);

                Expr::Lambda {
                    params,
                    return_type,
                    body,
                }
            }
            Token::LBrace => {
                self.next();

                let mut trailing = true;
                let mut exprs = Vec::new();
                while !self.at(&Token::RBrace) {
                    exprs.push(self.expression()?);

                    if self.consume_at(&Token::Semicolon) && self.at(&Token::RBrace) {
                        trailing = false;
                        break;
                    }
                }
                self.consume(&Token::RBrace)?;

                Expr::Block { exprs, trailing }
            }
            Token::LBracket => Expr::Literal(Lit::Array(self.delimited_list(
                Self::expression,
                &Token::LBracket,
                &Token::RBracket,
            )?)),
            token => {
                return Err(ParseError::UnexpectedToken(
                    token.to_string(),
                    Some("start of expression".into()),
                ));
            }
        };
        loop {
            let token = self.peek();
            let op = match token {
                Token::Eq => Bop::Assign,
                Token::Plus => Bop::Add,
                Token::Minus => Bop::Sub,
                Token::Times => Bop::Mul,
                Token::FSlash => Bop::Div,
                Token::Xor => Bop::Xor,
                Token::Ampersand => Bop::BAnd,
                Token::Pipe => Bop::BOr,
                Token::Exponent => Bop::Exp,
                Token::Eqq => Bop::Eqq,
                Token::Neq => Bop::Neq,
                Token::And => Bop::And,
                Token::Or => Bop::Or,
                Token::LAngle => Bop::Lt,
                Token::Leq => Bop::Leq,
                Token::RAngle => Bop::Gt,
                Token::Geq => Bop::Geq,
                Token::LParen => {
                    let args =
                        self.delimited_list(Self::expression, &Token::LParen, &Token::RParen)?;

                    return Ok(Expr::FnCall {
                        fun: Box::new(lhs),
                        args,
                    });
                }
                Token::Eof => break,
                Token::Else
                | Token::RParen // Delimiters
                | Token::RBrace
                | Token::RBracket 
                | Token::Comma
                | Token::Semicolon
                | Token::Fn
                | Token::Const
                | Token::Struct
                | Token::Enum => break,
                token => return Err(ParseError::UnexpectedToken(token.to_string(), None)),
            };

            let (left_binding_power, right_binding_power) = op.binding_power();

            if left_binding_power < binding_power {
                break;
            }

            self.next();

            let rhs = self.parse_expression(right_binding_power)?;
            lhs = Expr::BinaryOp {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }

        Ok(lhs)
    }
}
