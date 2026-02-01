use std::sync::LazyLock;

use regex::Regex;

use super::TokenType;

type Rule = fn(&str) -> Option<(TokenType, usize)>;

fn match_single_char(input: &str, c: char) -> Option<usize> {
    input.chars().next().and_then(|ch| (ch == c).then_some(1))
}

fn match_two_chars(input: &str, first: char, second: char) -> Option<usize> {
    if input.len() >= 2
        && let Some(_) = match_single_char(input, first)
        && let Some(_) = match_single_char(&input[1..], second)
    {
        Some(2)
    } else {
        None
    }
}

fn match_keyword(input: &str, keyword: &str) -> Option<usize> {
    input.starts_with(keyword).then_some(keyword.len())
}

fn match_regex(input: &str, r: &Regex) -> Option<usize> {
    r.find(input).map(|regex_match| regex_match.end())
}

static INT_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\d+").unwrap());
static FLOAT_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^((\d+\.(\d+)?)|(\.\d+))([Ee][\+-]?\d+)?").unwrap());
static STRING_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^"((\\"|\\\\|\\n)|[^\\"])*""#).unwrap());
static CHAR_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^'((\\'|\\\\|\\n)|[^\\'])'").unwrap());
static IDENTIFIER_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z_]([A-Za-z_]|\d)*").unwrap());

pub(super) const RULES: [Rule; 53] = {
    use TokenType as T;
    [
        |input| match_regex(input, &INT_REGEX).map(|len| (T::IntLit, len)),
        |input| match_regex(input, &FLOAT_REGEX).map(|len| (T::FloatLit, len)),
        |input| match_regex(input, &STRING_REGEX).map(|len| (T::StringLit, len)),
        |input| match_regex(input, &CHAR_REGEX).map(|len| (T::CharLit, len)),
        |input| match_single_char(input, '[').map(|len| (T::LBracket, len)),
        |input| match_single_char(input, ']').map(|len| (T::RBracket, len)),
        |input| match_single_char(input, '{').map(|len| (T::LBrace, len)),
        |input| match_single_char(input, '}').map(|len| (T::RBrace, len)),
        |input| match_single_char(input, '(').map(|len| (T::LParen, len)),
        |input| match_single_char(input, ')').map(|len| (T::RParen, len)),
        |input| match_single_char(input, '=').map(|len| (T::Eq, len)),
        |input| match_single_char(input, '&').map(|len| (T::Ampersand, len)),
        |input| match_single_char(input, '|').map(|len| (T::Pipe, len)),
        |input| match_single_char(input, '!').map(|len| (T::Bang, len)),
        |input| match_single_char(input, '^').map(|len| (T::Xor, len)),
        |input| match_single_char(input, '<').map(|len| (T::LAngle, len)),
        |input| match_single_char(input, '>').map(|len| (T::RAngle, len)),
        |input| match_single_char(input, '+').map(|len| (T::Plus, len)),
        |input| match_single_char(input, '-').map(|len| (T::Minus, len)),
        |input| match_single_char(input, '*').map(|len| (T::Times, len)),
        |input| match_single_char(input, '/').map(|len| (T::FSlash, len)),
        |input| match_single_char(input, '\\').map(|len| (T::BSlash, len)),
        |input| match_single_char(input, '.').map(|len| (T::Dot, len)),
        |input| match_single_char(input, ',').map(|len| (T::Comma, len)),
        |input| match_single_char(input, ':').map(|len| (T::Colon, len)),
        |input| match_single_char(input, ';').map(|len| (T::Semicolon, len)),
        |input| match_single_char(input, '_').map(|len| (T::Underscore, len)),
        |input| match_two_chars(input, '-', '>').map(|len| (T::Arrow, len)),
        |input| match_two_chars(input, '=', '=').map(|len| (T::Eqq, len)),
        |input| match_two_chars(input, '!', '=').map(|len| (T::Neq, len)),
        |input| match_two_chars(input, '*', '*').map(|len| (T::Exponent, len)),
        |input| match_two_chars(input, '&', '&').map(|len| (T::And, len)),
        |input| match_two_chars(input, '|', '|').map(|len| (T::Or, len)),
        |input| match_two_chars(input, '<', '=').map(|len| (T::Leq, len)),
        |input| match_two_chars(input, '>', '=').map(|len| (T::Geq, len)),
        |input| match_keyword(input, "Int").map(|len| (T::Int, len)),
        |input| match_keyword(input, "UInt").map(|len| (T::UInt, len)),
        |input| match_keyword(input, "Byte").map(|len| (T::Byte, len)),
        |input| match_keyword(input, "Float").map(|len| (T::Float, len)),
        |input| match_keyword(input, "Bool").map(|len| (T::Bool, len)),
        |input| match_keyword(input, "Char").map(|len| (T::Char, len)),
        |input| match_keyword(input, "let").map(|len| (T::Let, len)),
        |input| match_keyword(input, "mut").map(|len| (T::Mut, len)),
        |input| match_keyword(input, "const").map(|len| (T::Const, len)),
        |input| match_keyword(input, "fn").map(|len| (T::Fn, len)),
        |input| match_keyword(input, "struct").map(|len| (T::Struct, len)),
        |input| match_keyword(input, "enum").map(|len| (T::Enum, len)),
        |input| match_keyword(input, "if").map(|len| (T::If, len)),
        |input| match_keyword(input, "else").map(|len| (T::Else, len)),
        |input| match_keyword(input, "match").map(|len| (T::Match, len)),
        |input| match_keyword(input, "true").map(|len| (T::True, len)),
        |input| match_keyword(input, "false").map(|len| (T::False, len)),
        |input| match_regex(input, &IDENTIFIER_REGEX).map(|len| (T::Ident, len)),
    ]
};
