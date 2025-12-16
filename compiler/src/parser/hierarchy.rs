use std::cell::LazyCell;

use super::{Parser, ParseError, Token, ast::Stmt};

impl<I: Iterator<Item = Token>> Parser<I> {
    pub fn statement(&mut self) -> Result<Stmt, ParseError> {
        Ok(match self.peek() {
            Token::Let => {
                let ident_discrim = LazyCell::new(|| Token::Ident(String::new()).ty());

                self.next();
                let mutable = self.at(Token::Mut.ty());
                if mutable {
                    self.next();
                }

                let ident = match self.next() {
                    Some(Token::Ident(ident)) => ident,
                    Some(token) => return Err(ParseError::MismatchedToken { expected: *ident_discrim, found: token.ty() }),
                    None => return Err(ParseError::NoToken)
                };

                let type_annotation = if self.at(Token::Colon.ty()) {
                    self.next();
                    match self.next() {
                        Some(Token::Ident(ty)) => Some(ty),
                        Some(token) => return Err(ParseError::MismatchedToken { expected: *ident_discrim, found: token.ty() }),
                        None => return Err(ParseError::NoToken)
                    }
                } else {
                    None
                };

                self.consume(Token::Eq.ty())?;
                let value = self.expression()?;
                self.consume(Token::Semicolon.ty())?;

                Stmt::Let {
                    mutable,
                    ident,
                    type_annotation,
                    value,
                }
            }
            Token::Ident(_) => {
                let Token::Ident(ident) = self.next().unwrap() else {
                    unreachable!()
                };
                self.consume(Token::Eq.ty())?;
                let value = self.expression()?;
                self.consume(Token::Semicolon.ty())?;
                Stmt::Assign { ident, value }
            }
            _ => {
                let expr = self.expression()?;
                self.consume(Token::Semicolon.ty()).map(|_| Stmt::Expr(expr))?
            }
        })
    }
}
