use itertools::Itertools;
use proptest::{collection::vec, prelude::*};

use lex::TokKind;

use crate::Parser;

proptest! {
    #[test]
    fn doesnt_crash_toks(in_toks in vec(TokKind::arb(), 8..=512)) {
        let raw = in_toks.iter().map(TokKind::reverse).join(" ");

        if let Ok(toks) = lex::lex(&raw) {
            let _ = Parser::parse(toks);
        }
    }

    #[test]
    fn doesnt_crash_string(s in r"\PC*") {
        if let Ok(toks) = lex::lex(&s) {
            let _ = Parser::parse(toks);
        }
    }
}
