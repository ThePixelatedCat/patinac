use std::range::Range;

use itertools::Itertools as _;
use pretty_assertions::assert_eq;
use proptest::prelude::*;

use crate::{Tok, TokKind as T, lex};

fn test_lex(src: &str) -> Result<Vec<Tok>, Vec<Range<u32>>> {
    let (out, errs): (Vec<_>, Vec<_>) = lex::lex(src).into_iter().partition_result();
    if errs.is_empty() { Ok(out) } else { Err(errs) }
}

#[test]
fn whitespace() {
    let src = "   + -  (.): ";
    assert_eq!(
        test_lex(src),
        Ok(vec![
            T::Whitespace.span(0..3),
            T::Plus.span(3..4),
            T::Whitespace.span(4..5),
            T::Minus.span(5..6),
            T::Whitespace.span(6..8),
            T::LParen.span(8..9),
            T::Dot.span(9..10),
            T::RParen.span(10..11),
            T::Colon.span(11..12),
            T::Whitespace.span(12..13)
        ]),
    );
}

#[test]
fn maybe_multiple_char_tokens() {
    let src = "&&=<=_!=||->::";
    assert_eq!(
        test_lex(src),
        Ok(vec![
            T::And.span(0..2),
            T::Eq.span(2..3),
            T::Leq.span(3..5),
            T::Underscore.span(5..6),
            T::Neq.span(6..8),
            T::Or.span(8..10),
            T::Arrow.span(10..12),
            T::PathSep.span(12..14)
        ]),
    );
}

#[test]
fn comment() {
    let src = "//hello, world!\nif let";
    assert_eq!(
        test_lex(src),
        Ok(vec![
            T::Whitespace.span(15..16),
            T::If.span(16..18),
            T::Whitespace.span(18..19),
            T::Let.span(19..22)
        ]),
    );
}

#[test]
fn literals() {
    let src = r#"1 0.21 1.5E-2true"test\n\"""#;
    assert_eq!(
        test_lex(src),
        Ok(vec![
            T::IntLit.span(0..1),
            T::Whitespace.span(1..2),
            T::FloatLit.span(2..6),
            T::Whitespace.span(6..7),
            T::FloatLit.span(7..13),
            T::True.span(13..17),
            T::StringLit.span(17..27),
        ]),
    );
}

#[test]
fn unicode_gibberish() {
    assert_eq!(test_lex("®"), Err(vec![Range::from(0..2)]));
}

#[test]
fn unicode_ident() {
    assert_eq!(test_lex("Москва東京π"), Ok(vec![T::Ident.span(0..20)]));
}

#[test]
fn eof_comment() {
    assert_eq!(test_lex("//"), Ok(vec![]));
}

proptest! {
    #[test]
    fn doesnt_crash(s in r"\PC{1,1024}") {
        let _ = test_lex(&s);
    }

    #[test]
    fn no_repeat_whitespace(s in r"\PC{1,1024}") {
        prop_assert!(lex::lex(&s).array_windows().all(|[a, b]| [a.map(|t| t.kind), b.map(|t| t.kind)] != [Ok(T::Whitespace), Ok(T::Whitespace)]))
    }
}
