use std::sync::LazyLock;

use regex::Regex;

use super::TokenType;

type Rule = fn(&str) -> Option<(TokenType, usize)>;

fn match_phrase(i: &str, p: &str, t: TokenType) -> Option<(TokenType, usize)> {
    i.starts_with(p).then_some((t, p.len()))
}

fn match_regex(i: &str, r: &Regex, t: TokenType) -> Option<(TokenType, usize)> {
    r.find(i).map(|regex_match| (t, regex_match.end()))
}

static INT_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\d+").unwrap());
static FLOAT_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^((\d+\.(\d+)?)|(\.\d+))([Ee][\+-]?\d+)?").unwrap());
static STRING_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^"((\\"|\\\\|\\n)|[^\\"])*""#).unwrap());
static CHAR_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^'((\\'|\\\\|\\n)|[^\\'])'").unwrap());
static IDENT_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z_]([A-Za-z_]|\d)*").unwrap());

const RULES: [Rule; 53] = {
    use TokenType as T;
    [
        |i| match_regex(i, &INT_REGEX, T::IntLit),
        |i| match_regex(i, &FLOAT_REGEX, T::FloatLit),
        |i| match_regex(i, &STRING_REGEX, T::StringLit),
        |i| match_regex(i, &CHAR_REGEX, T::CharLit),
        |i| match_phrase(i, "[", T::LBracket),
        |i| match_phrase(i, "]", T::RBracket),
        |i| match_phrase(i, "{", T::LBrace),
        |i| match_phrase(i, "}", T::RBrace),
        |i| match_phrase(i, "(", T::LParen),
        |i| match_phrase(i, ")", T::RParen),
        |i| match_phrase(i, "=", T::Eq),
        |i| match_phrase(i, "&", T::Ampersand),
        |i| match_phrase(i, "|", T::Pipe),
        |i| match_phrase(i, "!", T::Bang),
        |i| match_phrase(i, "^", T::Xor),
        |i| match_phrase(i, "<", T::LAngle),
        |i| match_phrase(i, ">", T::RAngle),
        |i| match_phrase(i, "+", T::Plus),
        |i| match_phrase(i, "-", T::Minus),
        |i| match_phrase(i, "*", T::Times),
        |i| match_phrase(i, "/", T::FSlash),
        |i| match_phrase(i, "\\", T::BSlash),
        |i| match_phrase(i, ".", T::Dot),
        |i| match_phrase(i, ",", T::Comma),
        |i| match_phrase(i, ":", T::Colon),
        |i| match_phrase(i, ";", T::Semicolon),
        |i| match_phrase(i, "_", T::Underscore),
        |i| match_phrase(i, "->", T::Arrow),
        |i| match_phrase(i, "==", T::Eqq),
        |i| match_phrase(i, "!=", T::Neq),
        |i| match_phrase(i, "**", T::Exponent),
        |i| match_phrase(i, "&&", T::And),
        |i| match_phrase(i, "||", T::Or),
        |i| match_phrase(i, "<=", T::Leq),
        |i| match_phrase(i, ">=", T::Geq),
        |i| match_phrase(i, "Int", T::Int),
        |i| match_phrase(i, "UInt", T::UInt),
        |i| match_phrase(i, "Byte", T::Byte),
        |i| match_phrase(i, "Float", T::Float),
        |i| match_phrase(i, "Bool", T::Bool),
        |i| match_phrase(i, "Char", T::Char),
        |i| match_phrase(i, "let", T::Let),
        |i| match_phrase(i, "mut", T::Mut),
        |i| match_phrase(i, "const", T::Const),
        |i| match_phrase(i, "fn", T::Fn),
        |i| match_phrase(i, "struct", T::Struct),
        |i| match_phrase(i, "enum", T::Enum),
        |i| match_phrase(i, "if", T::If),
        |i| match_phrase(i, "else", T::Else),
        |i| match_phrase(i, "match", T::Match),
        |i| match_phrase(i, "true", T::True),
        |i| match_phrase(i, "false", T::False),
        |i| match_regex(i, &IDENT_REGEX, T::Ident),
    ]
};

pub(super) fn matches(input: &str) -> Option<(TokenType, usize)> {
    RULES
        .iter()
        .filter_map(|rule| rule(input))
        .rev() // reverse so that the first-listed element is returned in case of ambiguity (e.g. "match" as ident vs keyword)
        .max_by_key(|&(_, len)| len)
}
