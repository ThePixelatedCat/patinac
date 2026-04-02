use itertools::Itertools;
use lex::{Lexer, TokKind};
use proptest::{collection::vec, prelude::*};

use crate::Parser;

proptest! {
    #[test]
    fn doesnt_crash_toks(in_toks in vec(TokKind::arb(), 8..=512)) {
        let raw = in_toks.iter().map(TokKind::reverse).join(" ");

        let _ = Parser::new(&raw, Lexer::lex(&raw).unwrap().into_iter().peekable()).parse();
    }

    #[test]
    fn doesnt_crash_string(s in r"\PC*") {
        if let Ok(toks) = Lexer::lex(&s) {
            let _ = Parser::new(&s, toks.into_iter().peekable()).parse();
        }
    }
}
