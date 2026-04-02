mod rules;
#[cfg(test)]
mod test;
mod token;

use span::Span;
pub use token::{Tok, TokKind};

pub struct Lexer<'input> {
    input: &'input str,
    output: Vec<Tok>,
    errors: Vec<Span>,
}

impl<'input> Lexer<'input> {
    pub fn lex(input: &'input str) -> Result<Vec<Tok>, Vec<Span>> {
        let mut lexer = Self {
            input,
            output: Vec::with_capacity(input.len() / 4),
            errors: Vec::new(),
        };
        lexer.all_tokens();
        if lexer.errors.is_empty() {
            Ok(lexer.output)
        } else {
            Err(lexer.errors)
        }
    }

    fn all_tokens(&mut self) {
        let mut pos = 0;
        while let Some(token) = self.next_token(pos) {
            match token {
                Ok(token) => {
                    pos = token.span.end;
                    self.output.push(token);
                }
                Err(span) => {
                    pos = span.end;
                    self.errors.push(span);
                }
            }
        }
    }

    fn next_token(&self, pos: usize) -> Option<Result<Tok, Span>> {
        let input = self.get_rest(pos)?;

        if input.starts_with("//") {
            self.next_token(pos + input.find(['\n', '\r'])?)
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
                || Err(self.find_err(pos)),
                |(token, len)| Ok(token.span(pos..pos + len)),
            ))
        }
    }

    fn find_err(&self, pos: usize) -> Span {
        let start = pos;
        let mut end = start;

        while let Some(input) = self.get_rest(end)
            && rules::matches(input).is_none()
        {
            end += 1;
        }

        Span::from(start..end)
    }

    fn get_rest(&self, pos: usize) -> Option<&str> {
        (pos < self.input.len())
            .then(|| self.input.get(pos..))
            .flatten()
    }
}
