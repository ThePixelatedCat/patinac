mod rules;
#[cfg(test)]
mod test;
mod token;

pub use token::Token;

pub struct Lexer<'input> {
    input: &'input str,
    position: usize,
    eof: bool,
}

impl<'input> Iterator for Lexer<'input> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position >= self.input.len() {
            if self.eof {
                return None;
            }
            self.eof = true;
            Some(Token::Eof)
        } else {
            Some(self.next_token(&self.input[self.position..]))
        }
    }
}

impl<'input> Lexer<'input> {
    pub fn new(input: &'input str) -> Self {
        Self {
            input,
            position: 0,
            eof: false,
        }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        self.collect()
    }

    pub fn next_token(&mut self, input: &str) -> Token {
        self.valid_token(input)
            .unwrap_or_else(|| self.invalid_token(input))
    }

    /// Returns `None` if the lexer cannot find a token at the start of `input`.
    fn valid_token(&mut self, input: &str) -> Option<Token> {
        if input.starts_with("//") {
            self.position += input
                .char_indices()
                .find(|(_, c)| *c == '\n')
                .expect("expected newline to terminate comment")
                .0;
            self.next()
        } else if input.chars().next().unwrap().is_whitespace() {
            self.position += input
                .char_indices()
                .take_while(|(_, c)| c.is_whitespace())
                .last()
                .unwrap()
                .0
                + 1;
            self.next()
        } else {
            let (token, len) = rules::RULES
                .iter()
                .rev()
                .filter_map(|rule| rule(input))
                .max_by_key(|&(_, len)| len)?;

            self.position += len;
            Some(token)
        }
    }

    /// Always "succeeds", because it creates an error `Token`.
    fn invalid_token(&mut self, input: &str) -> Token {
        let start = self.position; // <- NEW!
        let len = input
            .char_indices()
            .map(|(pos, _)| pos)
            .find(|pos| self.valid_token(&input[*pos..]).is_some())
            .unwrap_or(input.len());
        debug_assert!(len <= input.len());

        self.position = start + len;
        Token::Error {
            start,
            end: self.position,
        }
    }
}
