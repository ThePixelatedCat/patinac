use itertools::assert_equal;

use super::{Lexer, TokKind as T};

#[test]
fn single_char_tokens() {
    assert_equal(
        Lexer::lex("+-(.):"),
        [
            T::Plus.span(0..1),
            T::Minus.span(1..2),
            T::LParen.span(2..3),
            T::Dot.span(3..4),
            T::RParen.span(4..5),
            T::Colon.span(5..6),
            T::Eof.span(6..6),
        ],
    );
}

#[test]
fn unknown_input() {
    assert_equal(
        Lexer::lex("$$$$$$$+"),
        [T::Error.span(0..7), T::Plus.span(7..8), T::Eof.span(8..8)],
    );
}

#[test]
fn single_char_tokens_with_whitespace() {
    assert_equal(
        Lexer::lex("   + -  (.): "),
        [
            T::Plus.span(3..4),
            T::Minus.span(5..6),
            T::LParen.span(8..9),
            T::Dot.span(9..10),
            T::RParen.span(10..11),
            T::Colon.span(11..12),
            T::Eof.span(13..13),
        ],
    );
}

#[test]
fn maybe_multiple_char_tokens() {
    assert_equal(
        Lexer::lex("&&=<=_!=||**->"),
        [
            T::And.span(0..2),
            T::Eq.span(2..3),
            T::Leq.span(3..5),
            T::Underscore.span(5..6),
            T::Neq.span(6..8),
            T::Or.span(8..10),
            T::Exponent.span(10..12),
            T::Arrow.span(12..14),
            T::Eof.span(14..14),
        ],
    );
}

#[test]
fn keywords() {
    assert_equal(
        Lexer::lex("if Int record Byte let mut UInt enum Float = match Bool else Char fn"),
        [
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
            T::Eof.span(68..68),
        ],
    );
}

#[test]
fn comment() {
    assert_equal(
        Lexer::lex("//hello, world!\nif let"),
        [T::If.span(16..18), T::Let.span(19..22), T::Eof.span(22..22)],
    );
}

#[test]
fn literals() {
    assert_equal(
        Lexer::lex(r#"1 .5 0.211 1. true "test"'\n''\''"#),
        [
            T::IntLit.span(0..1),
            T::FloatLit.span(2..4),
            T::FloatLit.span(5..10),
            T::FloatLit.span(11..13),
            T::True.span(14..18),
            T::StringLit.span(19..25),
            T::CharLit.span(25..29),
            T::CharLit.span(29..33),
            T::Eof.span(33..33),
        ],
    );
}

#[test]
fn function() {
    let input = r#"
// this is a comment!
fn test(var: Type, var2_: Bool) ->
 
    let x = '\n' + "String content \"\\ test" + 7 / 27.3e-2 ** 4
    let mut chars = x.chars()
    if let Some(c) = chars.next() then
        x = x + c
    else if !var2_ then 
        x = x + ","
"#;

    let tokens = Lexer::lex(&input);
    assert_equal(
        tokens,
        [
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
            T::Arrow.span(55..57),
            T::LBrace.span(60..64),
            // `x` assignment
            T::Let.span(64..67),
            T::Ident.span(68..69),
            T::Eq.span(70..71),
            T::CharLit.span(72..76),
            T::Plus.span(77..78),
            T::StringLit.span(79..105),
            T::Plus.span(106..107),
            T::IntLit.span(108..109),
            T::FSlash.span(110..111),
            T::FloatLit.span(112..119),
            T::Exponent.span(120..122),
            T::IntLit.span(123..124),
            // `chars` assignment
            T::Let.span(129..132),
            T::Mut.span(133..136),
            T::Ident.span(137..142),
            T::Eq.span(143..144),
            T::Ident.span(145..146),
            T::Dot.span(146..147),
            T::Ident.span(147..152),
            T::LParen.span(152..153),
            T::RParen.span(153..154),
            // if
            T::If.span(159..161),
            T::Let.span(162..165),
            T::Ident.span(166..170),
            T::LParen.span(170..171),
            T::Ident.span(171..172),
            T::RParen.span(172..173),
            T::Eq.span(174..175),
            T::Ident.span(176..181),
            T::Dot.span(181..182),
            T::Ident.span(182..186),
            T::LParen.span(186..187),
            T::RParen.span(187..188),
            T::Then.span(189..193),
            T::LBrace.span(194..202),
            // `x` re-assignment
            T::Ident.span(202..203),
            T::Eq.span(204..205),
            T::Ident.span(206..207),
            T::Plus.span(208..209),
            T::Ident.span(210..211),
            // else if
            T::RBrace.span(212..216),
            T::Else.span(216..220),
            T::If.span(221..223),
            T::Bang.span(224..225),
            T::Ident.span(225..230),
            T::Then.span(231..235),
            T::LBrace.span(237..245),
            // `x` re-assignment
            T::Ident.span(245..246),
            T::Eq.span(247..248),
            T::Ident.span(249..250),
            T::Plus.span(251..252),
            T::StringLit.span(253..256),
            T::RBrace.span(257..257), // end if
            T::RBrace.span(257..257), // end fn
            T::Eof.span(257..257),
        ],
    );
}
