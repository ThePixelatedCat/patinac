use std::range::Range;

use itertools::Itertools as _;
use pretty_assertions::assert_eq;
use proptest::prelude::*;

use crate::{Tok, TokKind as T, lex};

fn test_lex(src: &str) -> Result<Vec<Tok>, Vec<Range<u32>>> {
    let (out, errs): (Vec<_>, Vec<_>) = lex::lex(src).partition_result();
    if errs.is_empty() { Ok(out) } else { Err(errs) }
}

#[test]
fn unknown_input() {
    assert_eq!(
        test_lex("$$$$$$$+"),
        Err(vec![
            Range::from(0..1),
            Range::from(1..2),
            Range::from(2..3),
            Range::from(3..4),
            Range::from(4..5),
            Range::from(5..6),
            Range::from(6..7),
        ])
    );
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

#[allow(clippy::too_many_lines, reason = "It's a test function")]
#[test]
fn function() {
    let src = r#"
// this is a comment!
fn test(var: Type, var2_: Bool): Int -> {
 
    let x = "\n" + "String content \"\\ test" + 7 / 27.3e-2 ^  4
    let mut chars = x.chars()
    if let Some(c) = chars.next() 
        x = x + c
    else if !var2_  
        x = x + ","
}
"#;

    assert_eq!(
        test_lex(src),
        Ok(vec![
            T::Whitespace.span(0..1),
            T::Whitespace.span(22..23),
            // function signature
            T::Fn.span(23..25),
            T::Whitespace.span(25..26),
            T::Ident.span(26..30),
            T::LParen.span(30..31),
            T::Ident.span(31..34),
            T::Colon.span(34..35),
            T::Whitespace.span(35..36),
            T::Ident.span(36..40),
            T::Comma.span(40..41),
            T::Whitespace.span(41..42),
            T::Ident.span(42..47),
            T::Colon.span(47..48),
            T::Whitespace.span(48..49),
            T::Ident.span(49..53),
            T::RParen.span(53..54),
            T::Colon.span(54..55),
            T::Whitespace.span(55..56),
            T::Ident.span(56..59),
            T::Whitespace.span(59..60),
            T::Arrow.span(60..62),
            T::Whitespace.span(62..63),
            T::LBrace.span(63..64),
            T::Whitespace.span(64..71),
            // `x` assignment
            T::Let.span(71..74),
            T::Whitespace.span(74..75),
            T::Ident.span(75..76),
            T::Whitespace.span(76..77),
            T::Eq.span(77..78),
            T::Whitespace.span(78..79),
            T::StringLit.span(79..83),
            T::Whitespace.span(83..84),
            T::Plus.span(84..85),
            T::Whitespace.span(85..86),
            T::StringLit.span(86..112),
            T::Whitespace.span(112..113),
            T::Plus.span(113..114),
            T::Whitespace.span(114..115),
            T::IntLit.span(115..116),
            T::Whitespace.span(116..117),
            T::Divide.span(117..118),
            T::Whitespace.span(118..119),
            T::FloatLit.span(119..126),
            T::Whitespace.span(126..127),
            T::Exponent.span(127..128),
            T::Whitespace.span(128..130),
            T::IntLit.span(130..131),
            T::Whitespace.span(131..136),
            // `chars` assignment
            T::Let.span(136..139),
            T::Whitespace.span(139..140),
            T::Mut.span(140..143),
            T::Whitespace.span(143..144),
            T::Ident.span(144..149),
            T::Whitespace.span(149..150),
            T::Eq.span(150..151),
            T::Whitespace.span(151..152),
            T::Ident.span(152..153),
            T::Dot.span(153..154),
            T::Ident.span(154..159),
            T::LParen.span(159..160),
            T::RParen.span(160..161),
            T::Whitespace.span(161..166),
            // if
            T::If.span(166..168),
            T::Whitespace.span(168..169),
            T::Let.span(169..172),
            T::Whitespace.span(172..173),
            T::Ident.span(173..177),
            T::LParen.span(177..178),
            T::Ident.span(178..179),
            T::RParen.span(179..180),
            T::Whitespace.span(180..181),
            T::Eq.span(181..182),
            T::Whitespace.span(182..183),
            T::Ident.span(183..188),
            T::Dot.span(188..189),
            T::Ident.span(189..193),
            T::LParen.span(193..194),
            T::RParen.span(194..195),
            T::Whitespace.span(195..205),
            // `x` re-assignment
            T::Ident.span(205..206),
            T::Whitespace.span(206..207),
            T::Eq.span(207..208),
            T::Whitespace.span(208..209),
            T::Ident.span(209..210),
            T::Whitespace.span(210..211),
            T::Plus.span(211..212),
            T::Whitespace.span(212..213),
            T::Ident.span(213..214),
            T::Whitespace.span(214..219),
            // else if
            T::Else.span(219..223),
            T::Whitespace.span(223..224),
            T::If.span(224..226),
            T::Whitespace.span(226..227),
            T::Bang.span(227..228),
            T::Ident.span(228..233),
            T::Whitespace.span(233..244),
            // `x` re-assignment
            T::Ident.span(244..245),
            T::Whitespace.span(245..246),
            T::Eq.span(246..247),
            T::Whitespace.span(247..248),
            T::Ident.span(248..249),
            T::Whitespace.span(249..250),
            T::Plus.span(250..251),
            T::Whitespace.span(251..252),
            T::StringLit.span(252..255),
            T::Whitespace.span(255..256),
            T::RBrace.span(256..257),
            T::Whitespace.span(257..258),
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
}
