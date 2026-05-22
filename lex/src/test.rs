use itertools::Itertools;
use pretty_assertions::assert_eq;
use proptest::{collection::vec, prelude::*};

use span::Span;

use crate::{TokKind as T, token::Tok};

fn lex(src: &str) -> Result<Vec<Tok<'_>>, Vec<Span>> {
    let mut out = Vec::new();
    let mut errs = Vec::new();

    for tok in crate::lex(src) {
        match tok {
            Ok(tok) => out.push(tok),
            Err(span) => errs.push(span),
        }
    }

    if errs.is_empty() { Ok(out) } else { Err(errs) }
}

#[test]
fn single_char_tokens() {
    let src = "+-(.):";
    assert_eq!(
        lex(src),
        Ok(vec![
            T::Plus.span(src, 0..1),
            T::Minus.span(src, 1..2),
            T::LParen.span(src, 2..3),
            T::Dot.span(src, 3..4),
            T::RParen.span(src, 4..5),
            T::Colon.span(src, 5..6),
        ]),
    );
}

#[test]
fn unknown_input() {
    assert_eq!(lex("$$$$$$$+"), Err(vec![Span::from(0..7)]));
}

#[test]
fn single_char_tokens_with_whitespace() {
    let src = "   + -  (.): ";
    assert_eq!(
        lex(src),
        Ok(vec![
            T::Plus.span(src, 3..4),
            T::Minus.span(src, 5..6),
            T::LParen.span(src, 8..9),
            T::Dot.span(src, 9..10),
            T::RParen.span(src, 10..11),
            T::Colon.span(src, 11..12),
        ]),
    );
}

#[test]
fn maybe_multiple_char_tokens() {
    let src = "&&=<=_!=||**->::";
    assert_eq!(
        lex(src),
        Ok(vec![
            T::And.span(src, 0..2),
            T::Eq.span(src, 2..3),
            T::Leq.span(src, 3..5),
            T::Underscore.span(src, 5..6),
            T::Neq.span(src, 6..8),
            T::Or.span(src, 8..10),
            T::Exponent.span(src, 10..12),
            T::Arrow.span(src, 12..14),
            T::PathSep.span(src, 14..16)
        ]),
    );
}

#[test]
fn keywords() {
    let src = "if Int record Byte let mut UInt enum Float = match Bool else Char fn";
    assert_eq!(
        lex(src),
        Ok(vec![
            T::If.span(src, 0..2),
            T::Int.span(src, 3..6),
            T::Record.span(src, 7..13),
            T::Byte.span(src, 14..18),
            T::Let.span(src, 19..22),
            T::Mut.span(src, 23..26),
            T::UInt.span(src, 27..31),
            T::Enum.span(src, 32..36),
            T::Float.span(src, 37..42),
            T::Eq.span(src, 43..44),
            T::Match.span(src, 45..50),
            T::Bool.span(src, 51..55),
            T::Else.span(src, 56..60),
            T::Char.span(src, 61..65),
            T::Fn.span(src, 66..68),
        ]),
    );
}

#[test]
fn comment() {
    let src = "//hello, world!\nif let";
    assert_eq!(
        lex(src),
        Ok(vec![T::If.span(src, 16..18), T::Let.span(src, 19..22)]),
    );
}

#[test]
fn literals() {
    let src = r#"1 0.21 1.5E-2 true "test"'\n''\''"#;
    assert_eq!(
        lex(src),
        Ok(vec![
            T::IntLit.span(src, 0..1),
            T::FloatLit.span(src, 2..6),
            T::FloatLit.span(src, 7..13),
            T::True.span(src, 14..18),
            T::StringLit.span(src, 19..25),
            T::CharLit.span(src, 25..29),
            T::CharLit.span(src, 29..33),
        ]),
    );
}

#[test]
fn function() {
    let src = r#"
// this is a comment!
fn test(var: Type, var2_: Bool): Int -> {
 
    let x = '\n' + "String content \"\\ test" + 7 / 27.3e-2 ** 4
    let mut chars = x.chars()
    if let Some(c) = chars.next() 
        x = x + c
    else if !var2_  
        x = x + ","
}
"#;

    assert_eq!(
        lex(src),
        Ok(vec![
            // function signature
            T::Fn.span(src, 23..25),
            T::Ident.span(src, 26..30),
            T::LParen.span(src, 30..31),
            T::Ident.span(src, 31..34),
            T::Colon.span(src, 34..35),
            T::Ident.span(src, 36..40),
            T::Comma.span(src, 40..41),
            T::Ident.span(src, 42..47),
            T::Colon.span(src, 47..48),
            T::Bool.span(src, 49..53),
            T::RParen.span(src, 53..54),
            T::Colon.span(src, 54..55),
            T::Int.span(src, 56..59),
            T::Arrow.span(src, 60..62),
            T::LBrace.span(src, 63..64),
            // `x` assignment
            T::Let.span(src, 71..74),
            T::Ident.span(src, 75..76),
            T::Eq.span(src, 77..78),
            T::CharLit.span(src, 79..83),
            T::Plus.span(src, 84..85),
            T::StringLit.span(src, 86..112),
            T::Plus.span(src, 113..114),
            T::IntLit.span(src, 115..116),
            T::Divide.span(src, 117..118),
            T::FloatLit.span(src, 119..126),
            T::Exponent.span(src, 127..129),
            T::IntLit.span(src, 130..131),
            // `chars` assignment
            T::Let.span(src, 136..139),
            T::Mut.span(src, 140..143),
            T::Ident.span(src, 144..149),
            T::Eq.span(src, 150..151),
            T::Ident.span(src, 152..153),
            T::Dot.span(src, 153..154),
            T::Ident.span(src, 154..159),
            T::LParen.span(src, 159..160),
            T::RParen.span(src, 160..161),
            // if
            T::If.span(src, 166..168),
            T::Let.span(src, 169..172),
            T::Ident.span(src, 173..177),
            T::LParen.span(src, 177..178),
            T::Ident.span(src, 178..179),
            T::RParen.span(src, 179..180),
            T::Eq.span(src, 181..182),
            T::Ident.span(src, 183..188),
            T::Dot.span(src, 188..189),
            T::Ident.span(src, 189..193),
            T::LParen.span(src, 193..194),
            T::RParen.span(src, 194..195),
            // `x` re-assignment
            T::Ident.span(src, 205..206),
            T::Eq.span(src, 207..208),
            T::Ident.span(src, 209..210),
            T::Plus.span(src, 211..212),
            T::Ident.span(src, 213..214),
            // else if
            T::Else.span(src, 219..223),
            T::If.span(src, 224..226),
            T::Bang.span(src, 227..228),
            T::Ident.span(src, 228..233),
            // `x` re-assignment
            T::Ident.span(src, 244..245),
            T::Eq.span(src, 246..247),
            T::Ident.span(src, 248..249),
            T::Plus.span(src, 250..251),
            T::StringLit.span(src, 252..255),
            T::RBrace.span(src, 256..257),
        ]),
    );
}

#[test]
fn unicode_gibberish() {
    assert_eq!(lex("®"), Err(vec![Span::from(0..2)]));
}

#[test]
fn unicode_ident() {
    assert_eq!(
        lex("Москва東京π"),
        Ok(vec![T::Ident.span("Москва東京π", 0..20)])
    );
}

#[test]
fn eof_comment() {
    assert_eq!(lex("//"), Ok(Vec::<Tok>::new()));
}

proptest! {
    #[test]
    fn doesnt_crash(s in r"\PC*") {
        let _ = lex(&s);
    }

    #[test]
    fn reverse(in_toks in vec(T::arb(), 8..=512)) {
        let raw = in_toks.iter().map(T::reverse).join(" ");

        let out_toks: Vec<_> = lex(&raw).unwrap().into_iter().map(|tok| tok.kind).collect();

        prop_assert_eq!(in_toks, out_toks);
    }
}
