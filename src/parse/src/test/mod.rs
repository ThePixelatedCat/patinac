mod exprs;
mod items;
mod lex;

use itertools::Itertools as _;
use proptest::{collection::vec, prelude::*};

use errors::ErrorHandler;
use irs::ModuleId;

use crate::{Parser, TokKind};

proptest! {
    #[test]
    fn doesnt_crash(toks in vec(TokKind::arbitrary(), 8..=512)) {
        let raw = toks.iter().map(|t| t.reverse()).join(" ");
        let _ = Parser::new(ModuleId::default(), &raw, ErrorHandler::DUMMY).parse();
    }
}
