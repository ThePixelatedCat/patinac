mod rules;
#[cfg(test)]
mod test;
mod token;

pub use token::{Token, TokenType};

pub struct Lexer<'input> {
    input: &'input str,
    pos: usize,
    indent: usize
}

impl Iterator for Lexer<'_> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        Some(if self.pos >= self.input.len() {
            TokenType::Eof.spanned(self.pos..self.pos)
        } else {
            let input = &self.input[self.pos..];
            self.valid_token(input)
                .unwrap_or_else(|| self.invalid_token(input))
        })
    }
}

impl<'input> Lexer<'input> {
    pub const fn new(input: &'input str) -> Self {
        Self { input, pos: 0, indent: 0 }
    }

    /// Returns `None` if the lexer cannot find a token at the start of `input`.
    fn valid_token(&mut self, input: &str) -> Option<Token> {
        if input.starts_with("//") {
            self.pos += input
                .find('\n')
                .expect("expected newline to terminate comment");
            self.next()
        } else if input.starts_with("\n") {
            let newlines = input
                .char_indices()
                .take_while(|(_, c)| *c == '\n' || *c == '\r')
                .last()
                .unwrap()
                .0
                + 1;
            self.pos += newlines;
            self.indentation(&input[newlines..])
        } else if input.chars().next().unwrap().is_whitespace() {
            self.pos += input
                .char_indices()
                .take_while(|(_, c)| c.is_whitespace() && !(*c == '\n' || *c == '\r') )
                .last()
                .unwrap()
                .0
                + 1;
            self.next()
        } else {
            let (token, len) = rules::matches(input)?;

            let token = token.spanned(self.pos..self.pos + len);
            self.pos += len;

            Some(token)
        }
    }

    fn indentation(&mut self, input: &str) -> Option<Token> {
        let start = self.pos;
        let old_indent = self.indent;

        self.indent = input
            .char_indices()
            .take_while(|(_, c)| *c == '\t' || *c == ' ')
            .last()
            .map_or(0, |(i, _)| i + 1);
        println!("{}", self.indent);
        self.pos += self.indent;

        if input[self.indent..].starts_with("\n") {
            self.pos += 1;
            self.indentation(&input[self.indent + 1..])
        } else if self.indent > old_indent {
            Some(TokenType::Indent.spanned(start..self.pos))
        } else if self.indent < old_indent {
            Some(TokenType::Dedent.spanned(start..self.pos))
        } else {
            self.next()
        }
    }

    /// Always "succeeds", because it creates an error `TokenType`.
    fn invalid_token(&mut self, input: &str) -> Token {
        let start = self.pos;
        let len = input
            .char_indices()
            .map(|(pos, _)| pos)
            .find(|pos| self.valid_token(&input[*pos..]).is_some())
            .unwrap_or(input.len());

        self.pos = start + len;
        TokenType::Error.spanned(start..self.pos)
    }
}
