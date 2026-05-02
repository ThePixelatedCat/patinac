#[cfg(test)]
mod test;
mod token;

use logos::Logos;
use rangemap::set::RangeSet;
use span::Span;

pub use token::{Tok, TokKind};

/// Tokenises raw, UTF-8 source code
///
/// # Errors
/// If any invalid tokens are encountered, the function will continue, but will return an error with the span of every invalid token
pub fn lex(src: &str) -> Result<Vec<Tok<'_>>, Vec<Span>> {
    let mut out = Vec::new();
    let mut errs = RangeSet::new();
    let mut has_err = false;

    for (tok, span) in TokKind::lexer(src).spanned() {
        match tok {
            Ok(tok) if !has_err => {
                out.push(tok.span(src, span));
            }
            Err(()) => {
                has_err = true;
                errs.insert(span);
            }
            _ => {}
        }
    }

    if has_err {
        Err(errs.into_iter().map(Span::from).collect())
    } else {
        Ok(out)
    }
}
