mod rules;
#[cfg(test)]
mod test;
mod token;

pub use token::{Tok, TokKind};

pub struct Lexer<'input> {
    input: &'input str,
    output: Vec<Tok>,
}

impl<'input> Lexer<'input> {
    pub fn lex(input: &'input str) -> Vec<Tok> {
        let mut lexer = Self {
            input,
            output: Vec::with_capacity(input.len() / 4),
        };
        lexer.all_tokens();
        lexer.output
    }

    pub fn all_tokens(&mut self) {
        let mut pos = 0;
        while let Some(token) = self.next_token(pos) {
            pos = token.span.end;
            self.output.push(token);
        }
    }

    fn next_token(&self, pos: usize) -> Option<Tok> {
        let input = self.get_rest(pos)?;

        if input.starts_with("//") {
            let comment_length = input
                .find(['\n', '\r'])
                .expect("expected newline to terminate comment");
            self.next_token(pos + comment_length)
        } else if input.starts_with(char::is_whitespace) {
            let whitespace_length = input
                .char_indices()
                .take_while(|(_, c)| c.is_whitespace())
                .last()
                .unwrap()
                .0;
            self.next_token(pos + whitespace_length + 1)
        } else {
            Some(rules::matches(input).map_or_else(
                || self.err_token(pos),
                |(token, len)| token.span(pos..pos + len),
            ))
        }
    }

    fn err_token(&self, pos: usize) -> Tok {
        let start = pos;
        let mut end = start;

        while let Some(input) = self.get_rest(end)
            && rules::matches(input).is_none()
        {
            end += 1;
        }

        TokKind::Error.span(start..end)
    }

    fn get_rest(&self, pos: usize) -> Option<&str> {
        (pos < self.input.len()).then(|| &self.input[pos..])
    }
}
