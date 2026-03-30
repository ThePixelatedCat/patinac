mod rules;
#[cfg(test)]
mod test;
mod token;

pub use token::{Tok, TokKind};

pub struct Lexer<'input> {
    input: &'input str,
    output: Vec<Tok>,
    pos: usize,
    indent_levels: Vec<usize>,
    err: Option<usize>,
}

impl<'input> Lexer<'input> {
    pub fn lex(input: &'input str) -> Vec<Tok> {
        let mut lexer = Self::new(input);
        lexer.all_tokens();
        lexer.output
    }

    pub fn new(input: &'input str) -> Self {
        Self {
            input,
            output: Vec::with_capacity(input.len() / 4),
            pos: 0,
            indent_levels: vec![0],
            err: None,
        }
    }

    pub fn all_tokens(&mut self) {
        while self.pos < self.input.len() {
            self.next_token(&self.input[self.pos..]);
        }

        if self.output.last().is_none_or(|t| t.kind != TokKind::Eof) {
            self.output.push(TokKind::Eof.span(self.pos..self.pos));
        }
    }

    fn next_token(&mut self, input: &str) {
        if input.starts_with("//") {
            self.pos += input
                .find('\n')
                .expect("expected newline to terminate comment");
        } else if input.starts_with(['\n', '\r']) {
            let newlines = input
                .char_indices()
                .take_while(|(_, c)| *c == '\n' || *c == '\r')
                .last()
                .unwrap()
                .0
                + 1;
            self.pos += newlines;
            self.indentation(&input[newlines..]);
        } else if input.starts_with(|c: char| c.is_whitespace() && !(c == '\n' || c == '\r')) {
            self.pos += input
                .char_indices()
                .take_while(|(_, c)| c.is_whitespace() && !(*c == '\n' || *c == '\r'))
                .last()
                .unwrap()
                .0
                + 1;
        } else {
            match rules::matches(input) {
                Some((token, len)) => {
                    if let Some(start) = self.err {
                        self.output.push(TokKind::Error.span(start..self.pos));
                    }

                    self.output.push(token.span(self.pos..self.pos + len));

                    self.pos += len;
                }
                None => {
                    if self.err.is_none() {
                        self.err = Some(self.pos);
                    }
                    self.pos += 1;
                }
            }
        }
    }

    fn indentation(&mut self, input: &str) {
        let start = self.pos;

        let new_level = input
            .char_indices()
            .take_while(|(_, c)| *c == '\t' || *c == ' ')
            .last()
            .map_or(0, |(i, _)| i + 1);
        self.pos += new_level;

        let last_level = self.indent_levels.last().copied().unwrap();
        if input[new_level..].starts_with('\n') {
            self.pos += 1;
            self.indentation(&input[new_level + 1..]);
        } else if new_level > last_level {
            self.indent_levels.push(new_level);
            self.output.push(TokKind::LBrace.span(start..self.pos));
        } else if new_level < last_level {
            while new_level < self.indent_levels.last().copied().unwrap() {
                self.indent_levels.pop();
                self.output.push(TokKind::RBrace.span(start..self.pos));
            }
        }
    }
}
