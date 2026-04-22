use std::sync::LazyLock;

use const_format::formatcp;
use regex::Regex;

use crate::TokKind;

macro_rules! rule {
    ($str:literal => $tok:ident) => {
        |i| match_phrase(i, $str, $crate::TokKind::$tok)
    };
    (_ => $tok:ident) => {
        |i| {
            match_phrase(
                i,
                $crate::TokKind::$tok.to_string().trim_matches('`'),
                $crate::TokKind::$tok,
            )
        }
    };
}

macro_rules! reg_rule {
    ($regex:expr => $tok:ident) => {{
        static REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new($regex).unwrap());
        |i| match_regex(i, &REGEX, $crate::TokKind::$tok)
    }};
}

fn match_phrase(i: &str, p: &str, t: TokKind) -> Option<(TokKind, usize)> {
    i.starts_with(p).then_some((t, p.len()))
}

fn match_regex(i: &str, r: &Regex, t: TokKind) -> Option<(TokKind, usize)> {
    r.find(i).map(|regex_match| (t, regex_match.end()))
}

pub const DEC_INT: &str = "([0-9][0-9_]*)";
pub const BIN_INT: &str = "(0b[0-1][0-1_]*)";
pub const OCT_INT: &str = "(0o[0-7][0-7_]*)";
pub const HEX_INT: &str = "(0x[0-9a-fA-F][0-9a-fA-F_]*)";
pub const INT: &str = formatcp!("^{DEC_INT}|{BIN_INT}|{OCT_INT}|{HEX_INT}");

pub const EXPONENT: &str = formatcp!("([Ee]-?{DEC_INT})");
pub const FLOAT: &str = formatcp!(r"^{DEC_INT}\.{DEC_INT}{EXPONENT}?");

pub const ESCAPE: &str =
    r#"((\\\\)|(\\')|(\\")|(\\0)|(\\t)|(\\n)|(\\r)|(\\u\{[0-9a-fA-F]{1,6}\}))"#;
pub const CHAR: &str = formatcp!(r"^'([^\t\n\r'\\]|{ESCAPE})'");
#[allow(
    clippy::needless_raw_string_hashes,
    reason = "not sure what clippy is on but it's definitely needed"
)]
pub const STRING: &str = formatcp!(r##"^("([^"\\]|{ESCAPE})*")|((?s)#".*"#)"##);

pub const IDENT: &str = "^[A-Za-z][A-Za-z_0-9]*";

type Rule = fn(&str) -> Option<(TokKind, usize)>;
const RULES: [Rule; 62] = [
    reg_rule!(INT => IntLit),
    reg_rule!(FLOAT => FloatLit),
    reg_rule!(CHAR => CharLit),
    reg_rule!(STRING => StringLit),
    rule!(_ => LParen),
    rule!(_ => RParen),
    rule!(_ => LBrace),
    rule!(_ => RBrace),
    rule!(_ => LBracket),
    rule!(_ => RBracket),
    rule!(_ => Eq),
    rule!(_ => Ampersand),
    rule!(_ => Pipe),
    rule!(_ => Bang),
    rule!(_ => Xor),
    rule!(_ => Lt),
    rule!(_ => Gt),
    rule!(_ => Plus),
    rule!(_ => Minus),
    rule!(_ => Times),
    rule!(_ => Divide),
    rule!(_ => BSlash),
    rule!(_ => Dot),
    rule!(_ => Comma),
    rule!(_ => Colon),
    rule!(_ => Semicolon),
    rule!(_ => Underscore),
    rule!(_ => Arrow),
    rule!(_ => Eqq),
    rule!(_ => Neq),
    rule!(_ => Exponent),
    rule!(_ => And),
    rule!(_ => Or),
    rule!(_ => Leq),
    rule!(_ => Geq),
    rule!(_ => Int),
    rule!(_ => UInt),
    rule!(_ => Byte),
    rule!(_ => Float),
    rule!(_ => Bool),
    rule!(_ => Char),
    rule!(_ => Let),
    rule!(_ => Mut),
    rule!(_ => Const),
    rule!(_ => Fn),
    rule!(_ => Record),
    rule!(_ => Enum),
    rule!(_ => If),
    rule!(_ => Then),
    rule!(_ => Else),
    rule!(_ => For),
    rule!(_ => In),
    rule!(_ => While),
    rule!(_ => Do),
    rule!(_ => Match),
    rule!(_ => With),
    rule!(_ => Return),
    rule!(_ => Break),
    rule!(_ => Continue),
    rule!(_ => True),
    rule!(_ => False),
    reg_rule!(IDENT => Ident),
];

pub fn matches(input: &str) -> Option<(TokKind, usize)> {
    RULES
        .iter()
        .filter_map(|rule| rule(input))
        .rev() // reverse so that the first-listed element is returned in case of ambiguity (e.g. "match" as ident vs keyword)
        .max_by_key(|&(_, len)| len) // maximal munch
}
