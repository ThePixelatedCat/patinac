use super::{Lexer, Token, TokenType as T};

macro_rules! count {
    () => (0usize);
    ( $x:tt $($xs:tt)* ) => (1usize + count!($($xs)*));
}

/// walks `$tokens` and compares them to the given kinds.
macro_rules! assert_tokens {
    ($tokens:ident, [$($token:expr,)*]) => {
        {
            assert_eq!($tokens.len(), count!($($token)*), "mismatched token counts");
            let mut iter = $tokens.iter();
            $(
                let token = iter.next().unwrap();
                assert_eq!(*token, $token);
            )*
        }
    };
}

fn tokenize(lexer: &mut Lexer<'_>) -> Vec<Token> {
    lexer.collect()
}

#[test]
fn single_char_tokens() {
    let mut lexer = Lexer::new("+-(.):");
    let tokens = tokenize(&mut lexer);
    assert_tokens!(
        tokens,
        [
            T::Plus.spanned(0..1),
            T::Minus.spanned(1..2),
            T::LParen.spanned(2..3),
            T::Dot.spanned(3..4),
            T::RParen.spanned(4..5),
            T::Colon.spanned(5..6),
            T::Eof.spanned(6..6),
        ]
    );
}

#[test]
fn unknown_input() {
    let mut lexer = Lexer::new("{$$$$$$$+");
    let tokens = tokenize(&mut lexer);
    assert_tokens!(
        tokens,
        [
            T::LBrace.spanned(0..1),
            T::Error.spanned(1..8),
            T::Plus.spanned(8..9),
            T::Eof.spanned(9..9),
        ]
    );
}

#[test]
fn single_char_tokens_with_whitespace() {
    let mut lexer = Lexer::new("   + -  (.): ");
    let tokens = tokenize(&mut lexer);
    assert_tokens!(
        tokens,
        [
            T::Plus.spanned(3..4),
            T::Minus.spanned(5..6),
            T::LParen.spanned(8..9),
            T::Dot.spanned(9..10),
            T::RParen.spanned(10..11),
            T::Colon.spanned(11..12),
            T::Eof.spanned(13..13),
        ]
    );
}

#[test]
fn maybe_multiple_char_tokens() {
    let mut lexer = Lexer::new("&&=<=_!=||**->");
    let tokens = tokenize(&mut lexer);
    assert_tokens!(
        tokens,
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
        ]
    );
}

#[test]
fn keywords() {
    let mut lexer =
        Lexer::new("if Int struct Byte let mut UInt enum Float = match Bool else Char fn");
    let tokens: Vec<_> = tokenize(&mut lexer);
    assert_tokens!(
        tokens,
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
        ]
    );
}

#[test]
fn comment() {
    let mut lexer = Lexer::new("//hello, world!\nif let");
    let tokens: Vec<_> = tokenize(&mut lexer);
    assert_tokens!(
        tokens,
        [
            T::If.spanned(16..18),
            T::Let.spanned(19..22),
            T::Eof.spanned(22..22),
        ]
    );
}

#[test]
fn literals() {
    let mut lexer = Lexer::new(r#"1 .5 0.211 1. true "test"'\n''\''"#);
    let tokens: Vec<_> = tokenize(&mut lexer);
    assert_tokens!(
        tokens,
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
        ]
    );
}

#[test]
fn function() {
    let input = r#"
        // this is a comment!
        fn test(var: Type, var2_: Bool) {
            let x = '\n' + "String content \"\\ test" + 7 / 27.3e-2 ** 4;
            let mut chars = x.chars();
            if let Some(c) = chars.next() {
                x = x + c;
            } else if !var2_ {
                x = x + ",";
            }
        }
    "#;
    let mut lexer = Lexer::new(&input);
    let tokens = tokenize(&mut lexer);
    assert_tokens!(
        tokens,
        [
            // function signature
            T::Fn.spanned(39..41),
            T::Ident.spanned(42..46),
            T::LParen.spanned(46..47),
            T::Ident.spanned(47..50),
            T::Colon.spanned(50..51),
            T::Ident.spanned(52..56),
            T::Comma.spanned(56..57),
            T::Ident.spanned(58..63),
            T::Colon.spanned(63..64),
            T::Bool.spanned(65..69),
            T::RParen.spanned(69..70),
            T::LBrace.spanned(71..72),
            // `x` assignment
            T::Let.spanned(85..88),
            T::Ident.spanned(89..90),
            T::Eq.spanned(91..92),
            T::CharLit.spanned(93..97),
            T::Plus.spanned(98..99),
            T::StringLit.spanned(100..126),
            T::Plus.spanned(127..128),
            T::IntLit.spanned(129..130),
            T::FSlash.spanned(131..132),
            T::FloatLit.spanned(133..140),
            T::Exponent.spanned(141..143),
            T::IntLit.spanned(144..145),
            T::Semicolon.spanned(145..146),
            // `chars` assignment
            T::Let.spanned(159..162),
            T::Mut.spanned(163..166),
            T::Ident.spanned(167..172),
            T::Eq.spanned(173..174),
            T::Ident.spanned(175..176),
            T::Dot.spanned(176..177),
            T::Ident.spanned(177..182),
            T::LParen.spanned(182..183),
            T::RParen.spanned(183..184),
            T::Semicolon.spanned(184..185),
            // if
            T::If.spanned(198..200),
            T::Let.spanned(201..204),
            T::Ident.spanned(205..209),
            T::LParen.spanned(209..210),
            T::Ident.spanned(210..211),
            T::RParen.spanned(211..212),
            T::Eq.spanned(213..214),
            T::Ident.spanned(215..220),
            T::Dot.spanned(220..221),
            T::Ident.spanned(221..225),
            T::LParen.spanned(225..226),
            T::RParen.spanned(226..227),
            T::LBrace.spanned(228..229),
            // `x` re-assignment
            T::Ident.spanned(246..247),
            T::Eq.spanned(248..249),
            T::Ident.spanned(250..251),
            T::Plus.spanned(252..253),
            T::Ident.spanned(254..255),
            T::Semicolon.spanned(255..256),
            // else if
            T::RBrace.spanned(269..270),
            T::Else.spanned(271..275),
            T::If.spanned(276..278),
            T::Bang.spanned(279..280),
            T::Ident.spanned(280..285),
            T::LBrace.spanned(286..287),
            // `x` re-assignment
            T::Ident.spanned(304..305),
            T::Eq.spanned(306..307),
            T::Ident.spanned(308..309),
            T::Plus.spanned(310..311),
            T::StringLit.spanned(312..315),
            T::Semicolon.spanned(315..316),
            T::RBrace.spanned(329..330), // end if
            T::RBrace.spanned(339..340), // end fn
            T::Eof.spanned(345..345),
        ]
    );
}
