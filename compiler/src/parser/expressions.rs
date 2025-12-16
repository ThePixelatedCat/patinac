use super::{
    Parser, Token, ParseError,
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
            Bop::Or => (1, 2),
            Bop::And => (3, 4),
            Bop::Eqq | Bop::Neq => (5, 6),
            Bop::Gt | Bop::Lt | Bop::Leq | Bop::Geq => (7, 8),
            Bop::Add | Bop::Sub => (9, 10),
            Bop::Mul | Bop::Div => (11, 12),
            Bop::Exp => (22, 21), // <- This binds stronger to the left!
        }
    }
}

impl<I: Iterator<Item = Token>> Parser<I> {
    pub fn expression(&mut self) -> Result<Expr, ParseError> {
        self.parse_expression(0)
    }

    pub fn parse_expression(&mut self, binding_power: u8) -> Result<Expr, ParseError> {
        let mut lhs = match self.peek() {
            Token::LParen => {
                self.next();
                let expr = self.expression()?;
                self.consume(Token::RParen.ty())?;
                expr
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
                let ident = Expr::Ident(ident);

                if self.at(Token::LParen.ty()) {
                    self.next();

                    let mut args = Vec::new();
                    while !self.at(Token::RParen.ty()) {
                        args.push(self.expression()?);

                        if self.at(Token::Comma.ty()) {
                            self.next();
                        }
                    }
                    self.consume(Token::RParen.ty())?;

                    Expr::FnCall {
                        fun: Box::new(ident),
                        args,
                    }
                } else {
                    ident
                }
            }
            Token::If => {
                self.next();
                self.consume(Token::LParen.ty())?;
                let cond = self.expression()?;
                self.consume(Token::RParen.ty())?;

                let th = self.expression()?;

                let el = if self.at(Token::Else.ty()) {
                    self.next();
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
            token => panic!("Unknown start of expression: `{:?}`", token),
        };
        loop {
            let token = self.peek();
            let op = match token {
                Token::Plus => Bop::Add,
                Token::Minus => Bop::Sub,
                Token::Times => Bop::Mul,
                Token::FSlash => Bop::Div,
                Token::UpArrow => Bop::Exp,
                Token::Eqq => Bop::Eqq,
                Token::Neq => Bop::Neq,
                Token::And => Bop::And,
                Token::Or => Bop::Or,
                Token::LAngle => Bop::Lt,
                Token::Leq => Bop::Leq,
                Token::RAngle => Bop::Gt,
                Token::Geq => Bop::Geq,
                Token::Eof => break,
                Token::RParen | Token::RBrace | Token::Comma | Token::Semicolon | Token::Else => {
                    break;
                }
                token => return Err(ParseError::UnexpectedToken(token.ty())),
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
