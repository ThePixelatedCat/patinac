use itertools::Itertools;
use proptest::{collection::vec, prelude::*};
use span::Span;

use crate::Tok;

use super::{Lexer, TokKind as T};

#[test]
fn single_char_tokens() {
    assert_eq!(
        Lexer::lex("+-(.):"),
        Ok(vec![
            T::Plus.span(0..1),
            T::Minus.span(1..2),
            T::LParen.span(2..3),
            T::Dot.span(3..4),
            T::RParen.span(4..5),
            T::Colon.span(5..6),
        ]),
    );
}

#[test]
fn unknown_input() {
    assert_eq!(Lexer::lex("$$$$$$$+"), Err(vec![Span::from(0..7)]));
}

#[test]
fn single_char_tokens_with_whitespace() {
    assert_eq!(
        Lexer::lex("   + -  (.): "),
        Ok(vec![
            T::Plus.span(3..4),
            T::Minus.span(5..6),
            T::LParen.span(8..9),
            T::Dot.span(9..10),
            T::RParen.span(10..11),
            T::Colon.span(11..12),
        ]),
    );
}

#[test]
fn maybe_multiple_char_tokens() {
    assert_eq!(
        Lexer::lex("&&=<=_!=||**->"),
        Ok(vec![
            T::And.span(0..2),
            T::Eq.span(2..3),
            T::Leq.span(3..5),
            T::Underscore.span(5..6),
            T::Neq.span(6..8),
            T::Or.span(8..10),
            T::Exponent.span(10..12),
            T::Arrow.span(12..14),
        ]),
    );
}

#[test]
fn keywords() {
    assert_eq!(
        Lexer::lex("if Int record Byte let mut UInt enum Float = match Bool else Char fn"),
        Ok(vec![
            T::If.span(0..2),
            T::Int.span(3..6),
            T::Record.span(7..13),
            T::Byte.span(14..18),
            T::Let.span(19..22),
            T::Mut.span(23..26),
            T::UInt.span(27..31),
            T::Enum.span(32..36),
            T::Float.span(37..42),
            T::Eq.span(43..44),
            T::Match.span(45..50),
            T::Bool.span(51..55),
            T::Else.span(56..60),
            T::Char.span(61..65),
            T::Fn.span(66..68),
        ]),
    );
}

#[test]
fn comment() {
    assert_eq!(
        Lexer::lex("//hello, world!\nif let"),
        Ok(vec![T::If.span(16..18), T::Let.span(19..22)]),
    );
}

#[test]
fn literals() {
    assert_eq!(
        Lexer::lex(r#"1 0.21 1.5E-2 true "test"'\n''\''"#),
        Ok(vec![
            T::IntLit.span(0..1),
            T::FloatLit.span(2..6),
            T::FloatLit.span(7..13),
            T::True.span(14..18),
            T::StringLit.span(19..25),
            T::CharLit.span(25..29),
            T::CharLit.span(29..33),
        ]),
    );
}

#[test]
fn function() {
    let input = r#"
// this is a comment!
fn test(var: Type, var2_: Bool): Int -> {
 
    let x = '\n' + "String content \"\\ test" + 7 / 27.3e-2 ** 4
    let mut chars = x.chars()
    if let Some(c) = chars.next() then
        x = x + c
    else if !var2_ then 
        x = x + ","
}
"#;

    let tokens = Lexer::lex(&input);
    assert_eq!(
        tokens,
        Ok(vec![
            // function signature
            T::Fn.span(23..25),
            T::Ident.span(26..30),
            T::LParen.span(30..31),
            T::Ident.span(31..34),
            T::Colon.span(34..35),
            T::Ident.span(36..40),
            T::Comma.span(40..41),
            T::Ident.span(42..47),
            T::Colon.span(47..48),
            T::Bool.span(49..53),
            T::RParen.span(53..54),
            T::Colon.span(54..55),
            T::Int.span(56..59),
            T::Arrow.span(60..62),
            T::LBrace.span(63..64),
            // `x` assignment
            T::Let.span(71..74),
            T::Ident.span(75..76),
            T::Eq.span(77..78),
            T::CharLit.span(79..83),
            T::Plus.span(84..85),
            T::StringLit.span(86..112),
            T::Plus.span(113..114),
            T::IntLit.span(115..116),
            T::FSlash.span(117..118),
            T::FloatLit.span(119..126),
            T::Exponent.span(127..129),
            T::IntLit.span(130..131),
            // `chars` assignment
            T::Let.span(136..139),
            T::Mut.span(140..143),
            T::Ident.span(144..149),
            T::Eq.span(150..151),
            T::Ident.span(152..153),
            T::Dot.span(153..154),
            T::Ident.span(154..159),
            T::LParen.span(159..160),
            T::RParen.span(160..161),
            // if
            T::If.span(166..168),
            T::Let.span(169..172),
            T::Ident.span(173..177),
            T::LParen.span(177..178),
            T::Ident.span(178..179),
            T::RParen.span(179..180),
            T::Eq.span(181..182),
            T::Ident.span(183..188),
            T::Dot.span(188..189),
            T::Ident.span(189..193),
            T::LParen.span(193..194),
            T::RParen.span(194..195),
            T::Then.span(196..200),
            // `x` re-assignment
            T::Ident.span(209..210),
            T::Eq.span(211..212),
            T::Ident.span(213..214),
            T::Plus.span(215..216),
            T::Ident.span(217..218),
            // else if
            T::Else.span(223..227),
            T::If.span(228..230),
            T::Bang.span(231..232),
            T::Ident.span(232..237),
            T::Then.span(238..242),
            // `x` re-assignment
            T::Ident.span(252..253),
            T::Eq.span(254..255),
            T::Ident.span(256..257),
            T::Plus.span(258..259),
            T::StringLit.span(260..263),
            T::RBrace.span(264..265),
        ]),
    );
}

#[test]
fn unicode_gibberish() {
    assert_eq!(Lexer::lex("®"), Err(vec![Span::from(0..1)]));
}

#[test]
fn eof_comment() {
    assert_eq!(Lexer::lex("//"), Ok(Vec::<Tok>::new()));
}

proptest! {
    #[test]
    fn doesnt_crash(s in r"\PC*") {
        let _ = Lexer::lex(&s);
    }

    #[test]
    fn reverse(in_toks in vec(T::arb(), 8..=512)) {
        let raw = in_toks.iter().map(T::reverse).join(" ");

        let out_toks: Vec<_> = Lexer::lex(&raw).unwrap().into_iter().map(|tok| tok.kind).collect();

        prop_assert_eq!(in_toks, out_toks)
    }
}

// #[test]
// fn rand_regexes() {
//     let ints = rand_regex::Regex::compile(rules::INT, 20).unwrap();
//     ints.
// }
