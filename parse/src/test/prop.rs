use itertools::Itertools;
use proptest::{collection::vec, prelude::*};

use errors::DUMMY_HANDLER;
use lex::TokKind;

use crate::Parser;

proptest! {
    #[test]
    fn doesnt_crash_toks(in_toks in vec(TokKind::arb(), 8..=512)) {
        let raw = in_toks.iter().map(TokKind::reverse).join(" ");

        if let Ok(toks) = lex::lex(&raw) {
            let _ = Parser::new(toks, DUMMY_HANDLER).parse();
        }
    }

    #[test]
    fn doesnt_crash_string(s in r"\PC*") {
        if let Ok(toks) = lex::lex(&s) {
            let _ = Parser::new(toks, DUMMY_HANDLER).parse();
        }
    }
}
