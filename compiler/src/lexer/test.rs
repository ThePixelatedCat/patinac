use itertools::assert_equal;

use super::{Lexer, TokenType as T};

#[test]
fn single_char_tokens() {
    assert_equal(
        Lexer::lex("+-(.):"),
        [
            T::Plus.spanned(0..1),
            T::Minus.spanned(1..2),
            T::LParen.spanned(2..3),
            T::Dot.spanned(3..4),
            T::RParen.spanned(4..5),
            T::Colon.spanned(5..6),
            T::Eof.spanned(6..6),
        ],
    );
}

#[test]
fn unknown_input() {
    assert_equal(
        Lexer::lex("$$$$$$$+"),
        [
            T::Error.spanned(0..7),
            T::Plus.spanned(7..8),
            T::Eof.spanned(8..8),
        ],
    );
}

#[test]
fn single_char_tokens_with_whitespace() {
    assert_equal(
        Lexer::lex("   + -  (.): "),
        [
            T::Plus.spanned(3..4),
            T::Minus.spanned(5..6),
            T::LParen.spanned(8..9),
            T::Dot.spanned(9..10),
            T::RParen.spanned(10..11),
            T::Colon.spanned(11..12),
            T::Eof.spanned(13..13),
        ],
    );
}

#[test]
fn maybe_multiple_char_tokens() {
    assert_equal(
        Lexer::lex("&&=<=_!=||**->"),
        [
            T::And.spanned(0..2),
            T::Eq.spanned(2..3),
            T::Leq.spanned(3..5),
            T::Underscore.spanned(5..6),
            T::Neq.spanned(6..8),
            T::Or.spanned(8..10),
            T::Exponent.spanned(10..12),
            T::Arrow.spanned(12..14),
            T::Eof.spanned(14..14),
        ],
    );
}

#[test]
fn keywords() {
    assert_equal(
        Lexer::lex("if Int struct Byte let mut UInt enum Float = match Bool else Char fn"),
        [
            T::If.spanned(0..2),
            T::Int.spanned(3..6),
            T::Struct.spanned(7..13),
            T::Byte.spanned(14..18),
            T::Let.spanned(19..22),
            T::Mut.spanned(23..26),
            T::UInt.spanned(27..31),
            T::Enum.spanned(32..36),
            T::Float.spanned(37..42),
            T::Eq.spanned(43..44),
            T::Match.spanned(45..50),
            T::Bool.spanned(51..55),
            T::Else.spanned(56..60),
            T::Char.spanned(61..65),
            T::Fn.spanned(66..68),
            T::Eof.spanned(68..68),
        ],
    );
}

#[test]
fn comment() {
    assert_equal(
        Lexer::lex("//hello, world!\nif let"),
        [
            T::If.spanned(16..18),
            T::Let.spanned(19..22),
            T::Eof.spanned(22..22),
        ],
    );
}

#[test]
fn literals() {
    assert_equal(
        Lexer::lex(r#"1 .5 0.211 1. true "test"'\n''\''"#),
        [
            T::IntLit.spanned(0..1),
            T::FloatLit.spanned(2..4),
            T::FloatLit.spanned(5..10),
            T::FloatLit.spanned(11..13),
            T::True.spanned(14..18),
            T::StringLit.spanned(19..25),
            T::CharLit.spanned(25..29),
            T::CharLit.spanned(29..33),
            T::Eof.spanned(33..33),
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
            T::Fn.spanned(23..25),
            T::Ident.spanned(26..30),
            T::LParen.spanned(30..31),
            T::Ident.spanned(31..34),
            T::Colon.spanned(34..35),
            T::Ident.spanned(36..40),
            T::Comma.spanned(40..41),
            T::Ident.spanned(42..47),
            T::Colon.spanned(47..48),
            T::Bool.spanned(49..53),
            T::RParen.spanned(53..54),
            T::Arrow.spanned(55..57),
            T::Indent.spanned(60..64),
            // `x` assignment
            T::Let.spanned(64..67),
            T::Ident.spanned(68..69),
            T::Eq.spanned(70..71),
            T::CharLit.spanned(72..76),
            T::Plus.spanned(77..78),
            T::StringLit.spanned(79..105),
            T::Plus.spanned(106..107),
            T::IntLit.spanned(108..109),
            T::FSlash.spanned(110..111),
            T::FloatLit.spanned(112..119),
            T::Exponent.spanned(120..122),
            T::IntLit.spanned(123..124),
            // `chars` assignment
            T::Let.spanned(129..132),
            T::Mut.spanned(133..136),
            T::Ident.spanned(137..142),
            T::Eq.spanned(143..144),
            T::Ident.spanned(145..146),
            T::Dot.spanned(146..147),
            T::Ident.spanned(147..152),
            T::LParen.spanned(152..153),
            T::RParen.spanned(153..154),
            // if
            T::If.spanned(159..161),
            T::Let.spanned(162..165),
            T::Ident.spanned(166..170),
            T::LParen.spanned(170..171),
            T::Ident.spanned(171..172),
            T::RParen.spanned(172..173),
            T::Eq.spanned(174..175),
            T::Ident.spanned(176..181),
            T::Dot.spanned(181..182),
            T::Ident.spanned(182..186),
            T::LParen.spanned(186..187),
            T::RParen.spanned(187..188),
            T::Then.spanned(189..193),
            T::Indent.spanned(194..202),
            // `x` re-assignment
            T::Ident.spanned(202..203),
            T::Eq.spanned(204..205),
            T::Ident.spanned(206..207),
            T::Plus.spanned(208..209),
            T::Ident.spanned(210..211),
            // else if
            T::Dedent.spanned(212..216),
            T::Else.spanned(216..220),
            T::If.spanned(221..223),
            T::Bang.spanned(224..225),
            T::Ident.spanned(225..230),
            T::Then.spanned(231..235),
            T::Indent.spanned(236..244),
            // `x` re-assignment
            T::Ident.spanned(244..245),
            T::Eq.spanned(246..247),
            T::Ident.spanned(248..249),
            T::Plus.spanned(250..251),
            T::StringLit.spanned(252..255),
            T::Dedent.spanned(256..256), // end if
            T::Dedent.spanned(256..256), // end fn
            T::Eof.spanned(256..256),
        ],
    );
}
