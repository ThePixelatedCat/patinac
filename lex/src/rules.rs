use std::sync::LazyLock;

use const_format::formatcp;
use regex::Regex;

use crate::TokKind;

macro_rules! rule {
    ($str:literal => $tok:ident) => {
        |i| match_phrase(i, $str, $crate::TokKind::$tok)
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
const RULES: [Rule; 62] = {
    [
        reg_rule!(INT => IntLit),
        reg_rule!(FLOAT => FloatLit),
        reg_rule!(CHAR => CharLit),
        reg_rule!(STRING => StringLit),
        rule!("(" => LParen),
        rule!(")" => RParen),
        rule!("{" => LBrace),
        rule!("}" => RBrace),
        rule!("[" => LBracket),
        rule!("]" => RBracket),
        rule!("=" => Eq),
        rule!("&" => Ampersand),
        rule!("|" => Pipe),
        rule!("!" => Bang),
        rule!("^" => Xor),
        rule!("<" => Lt),
        rule!(">" => Gt),
        rule!("+" => Plus),
        rule!("-" => Minus),
        rule!("*" => Times),
        rule!("/" => FSlash),
        rule!("\\" => BSlash),
        rule!("." => Dot),
        rule!("," => Comma),
        rule!(":" => Colon),
        rule!(";" => Semicolon),
        rule!("_" => Underscore),
        rule!("->" => Arrow),
        rule!("==" => Eqq),
        rule!("!=" => Neq),
        rule!("**" => Exponent),
        rule!("&&" => And),
        rule!("||" => Or),
        rule!("<=" => Leq),
        rule!(">=" => Geq),
        rule!("Int" => Int),
        rule!("UInt" => UInt),
        rule!("Byte" => Byte),
        rule!("Float" => Float),
        rule!("Bool" => Bool),
        rule!("Char" => Char),
        rule!("let" => Let),
        rule!("mut" => Mut),
        rule!("const" => Const),
        rule!("fn" => Fn),
        rule!("record" => Record),
        rule!("enum" => Enum),
        rule!("if" => If),
        rule!("then" => Then),
        rule!("else" => Else),
        rule!("for" => For),
        rule!("in" => In),
        rule!("while" => While),
        rule!("do" => Do),
        rule!("match" => Match),
        rule!("with" => With),
        rule!("return" => Return),
        rule!("break" => Break),
        rule!("continue" => Continue),
        rule!("true" => True),
        rule!("false" => False),
        reg_rule!(IDENT => Ident),
    ]
};

pub fn matches(input: &str) -> Option<(TokKind, usize)> {
    RULES
        .iter()
        .filter_map(|rule| rule(input))
        .rev() // reverse so that the first-listed element is returned in case of ambiguity (e.g. "match" as ident vs keyword)
        .max_by_key(|&(_, len)| len) // maximal munch
}
