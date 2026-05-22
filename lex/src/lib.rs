#[cfg(test)]
mod test;
mod token;

use logos::Logos;
use std::iter::Peekable;

use span::Span;

pub use token::{Tok, TokKind};

pub type Lexer<'src> = Peekable<Box<dyn Iterator<Item = Result<Tok<'src>, Span>> + 'src>>;

/// Tokenises raw, UTF-8 source code
///
/// # Errors
/// If any invalid tokens are encountered, the function will continue, but will return an error with the span of every invalid token
pub fn lex(src: &str) -> Lexer<'_> {
    let iter = TokKind::lexer(src).spanned().map(|(tok, span)| match tok {
        Ok(tok) => Ok(tok.span(src, span)),
        Err(()) => Err(Span::from(span)),
    });
    let boxed_iter = Box::new(iter) as Box<dyn Iterator<Item = _>>;
    boxed_iter.peekable()
    // let mut out = Vec::new();
    // let mut errs = Vec::new();

    // for (tok, span) in TokKind::lexer(src).spanned() {
    //     match tok {
    //         Ok(tok) if errs.is_empty() => {
    //             out.push(tok.span(src, span));
    //         }
    //         Err(()) => errs.push(Span::from(span)),
    //         _ => {}
    //     }
    // }

    // if errs.is_empty() { Ok(out) } else { Err(errs) }
}
