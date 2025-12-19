use super::{
    ParseError, ParseResult, Parser, Token,
    ast::{Ast, Field, Item, Variant},
};

impl<I: Iterator<Item = Token>> Parser<I> {
    pub fn file(&mut self) -> ParseResult<Ast> {
        let mut items = Vec::new();
        while !self.at(&Token::Eof) {
            items.push(self.item()?);
        }
        Ok(items)
    }

    pub fn item(&mut self) -> ParseResult<Item> {
        Ok(match self.peek() {
            Token::Const => {
                self.next();

                let ident = self.ident()?;

                self.consume(&Token::Colon)?;
                let ty = self.type_()?;

                self.consume(&Token::Eq)?;
                let value = self.expression()?;

                Item::Const { ident, ty, value }
            }
            Token::Fn => {
                self.next();

                let name = self.ident()?;

                let params = self.delimited_list(Self::binding, &Token::LParen, &Token::RParen)?;

                let return_type = if self.consume_at(&Token::Colon) {
                    Some(self.type_()?)
                } else {
                    None
                };

                self.consume(&Token::Arrow)?;

                let body = self.expression()?;

                Item::Function {
                    name,
                    params,
                    return_type,
                    body,
                }
            }
            Token::Struct => {
                self.next();

                let (name, generic_params) = self.type_name()?;

                Item::Struct {
                    name,
                    generic_params,
                    fields: self.fields()?,
                }
            }
            Token::Enum => {
                self.next();

                let (name, generic_params) = self.type_name()?;

                let variants = self.delimited_list(
                    |this| {
                        let variant_name = this.ident()?;

                        Ok(match this.peek() {
                            Token::LBrace => Variant::Struct(variant_name, this.fields()?),
                            Token::LParen => Variant::Tuple(
                                variant_name,
                                this.delimited_list(Self::type_, &Token::LParen, &Token::RParen)?,
                            ),
                            Token::Comma => Variant::Unit(variant_name),
                            token => {
                                return Err(ParseError::MismatchedToken {
                                    expected: "one of `,` `(` `{`".into(),
                                    found: token.to_string(),
                                });
                            }
                        })
                    },
                    &Token::LBrace,
                    &Token::RBrace,
                )?;

                Item::Enum {
                    name,
                    generic_params,
                    variants,
                }
            }
            token => {
                return Err(ParseError::UnexpectedToken(
                    token.to_string(),
                    Some("start of item".into()),
                ));
            }
        })
    }

    fn type_name(&mut self) -> ParseResult<(String, Vec<String>)> {
        let name = self.ident()?;

        let generic_params = if self.at(&Token::LAngle) {
            self.delimited_list(Self::ident, &Token::LAngle, &Token::RAngle)?
        } else {
            Vec::new()
        };

        Ok((name, generic_params))
    }

    fn fields(&mut self) -> ParseResult<Vec<Field>> {
        self.delimited_list(
            |this| {
                let name = this.ident()?;

                this.consume(&Token::Colon)?;
                let ty = this.type_()?;

                Ok(Field { name, ty })
            },
            &Token::LBrace,
            &Token::RBrace,
        )
    }
}
