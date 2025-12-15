use std::str::FromStr;

use lazy_static::lazy_static;
use regex::Regex;

use super::token::Token;

type Rule = fn(&str) -> Option<(Token, usize)>;

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

lazy_static! {
    static ref INT_REGEX: Regex = Regex::new(r#"^[\+-]?\d+"#).unwrap();
    static ref FLOAT_REGEX: Regex =
        Regex::new(r#"^[\+-]?((\d+\.(\d+)?)|(\.\d+))([Ee][\+-]?\d+)?"#).unwrap();
    static ref STRING_REGEX: Regex = Regex::new(r#"^"((\\"|\\\\|\\n)|[^\\"])*""#).unwrap();
    static ref CHAR_REGEX: Regex = Regex::new(r#"^'((\\'|\\\\|\\n)|[^\\'])'"#).unwrap();
    static ref IDENTIFIER_REGEX: Regex = Regex::new(r#"^[A-Za-z_]([A-Za-z_]|\d)*"#).unwrap();
}

pub(super) const RULES: [Rule; 41] = {
    use Token as T;
    [
        |input| {
            match_regex(input, &INT_REGEX)
                .map(|len| (Token::IntLit(i64::from_str(&input[..len]).unwrap()), len))
        },
        |input| {
            match_regex(input, &FLOAT_REGEX)
                .map(|len| (Token::FloatLit(f64::from_str(&input[..len]).unwrap()), len))
        },
        |input| {
            match_regex(input, &STRING_REGEX).map(|len| {
                (
                    Token::StringLit(
                        input[1..len - 1]
                            .replace("\\n", "\n")
                            .replace("\\\"", "\"")
                            .replace("\\\\", "\\"),
                    ),
                    len,
                )
            })
        },
        |input| {
            match_regex(input, &CHAR_REGEX).map(|len| {
                (
                    Token::CharLit(
                        input[1..len - 1]
                            .replace("\\n", "\n")
                            .replace("\\\'", "'")
                            .replace("\\\\", "\\")
                            .chars()
                            .next()
                            .unwrap(),
                    ),
                    len,
                )
            })
        },
        |input| match_single_char(input, '[').map(|len| (T::LBracket, len)),
        |input| match_single_char(input, ']').map(|len| (T::RBracket, len)),
        |input| match_single_char(input, '{').map(|len| (T::LBrace, len)),
        |input| match_single_char(input, '}').map(|len| (T::RBrace, len)),
        |input| match_single_char(input, '(').map(|len| (T::LParen, len)),
        |input| match_single_char(input, ')').map(|len| (T::RParen, len)),
        |input| match_single_char(input, '=').map(|len| (T::Eq, len)),
        |input| match_single_char(input, '&').map(|len| (T::Ampersand, len)),
        |input| match_single_char(input, '|').map(|len| (T::Bar, len)),
        |input| match_single_char(input, '!').map(|len| (T::Bang, len)),
        |input| match_single_char(input, '^').map(|len| (T::UpArrow, len)),
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
        |input| match_two_chars(input, '=', '=').map(|len| (T::Eqq, len)),
        |input| match_two_chars(input, '!', '=').map(|len| (T::Neq, len)),
        |input| match_two_chars(input, '&', '&').map(|len| (T::And, len)),
        |input| match_two_chars(input, '|', '|').map(|len| (T::Or, len)),
        |input| match_two_chars(input, '<', '=').map(|len| (T::Leq, len)),
        |input| match_two_chars(input, '>', '=').map(|len| (T::Geq, len)),
        |input| match_keyword(input, "let").map(|len| (T::Let, len)),
        |input| match_keyword(input, "fn").map(|len| (T::Fn, len)),
        |input| match_keyword(input, "if").map(|len| (T::If, len)),
        |input| match_keyword(input, "else").map(|len| (T::Else, len)),
        |input| match_keyword(input, "match").map(|len| (T::Match, len)),
        |input| match_keyword(input, "true").map(|len| (T::True, len)),
        |input| match_keyword(input, "false").map(|len| (T::False, len)),
        |input| {
            match_regex(input, &IDENTIFIER_REGEX)
                .map(|len| (Token::Ident(input[..len].into()), len))
        },
    ]
};
