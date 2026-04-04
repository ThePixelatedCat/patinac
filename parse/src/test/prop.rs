use itertools::Itertools;
use proptest::{collection::vec, prelude::*};

use lex::{Lexer, TokKind};

use crate::Parser;

proptest! {
    #[test]
    fn doesnt_crash_toks(in_toks in vec(TokKind::arb(), 8..=512)) {
        let raw = in_toks.iter().map(TokKind::reverse).join(" ");

        if let Ok(toks) = Lexer::lex(&raw) {
            let _ = Parser::parse(&raw, toks);
        }
    }

    #[test]
    fn doesnt_crash_string(s in r"\PC*") {
        if let Ok(toks) = Lexer::lex(&s) {
            let _ = Parser::parse(&s, toks);
        }
    }
}
