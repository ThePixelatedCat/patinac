#[cfg(any(test, feature = "test"))]
use proptest::{arbitrary::Arbitrary, prelude::Strategy};
use span::Span;
use std::fmt::Display;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Tok<'src> {
    pub kind: TokKind,
    pub span: Span,
    pub src: &'src str,
}

#[cfg_attr(any(test, feature = "test"), derive(proptest_derive::Arbitrary))]
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
#[repr(u8)]
pub enum TokKind {
    // Literals
    IntLit,
    FloatLit,
    StringLit,
    CharLit,
    // Delimiters
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    // Symbols
    Eq,
    Ampersand,
    Pipe,
    Bang,
    Plus,
    Minus,
    Times,
    FSlash,
    BSlash,
    Dot,
    Comma,
    Colon,
    Semicolon,
    Underscore,
    Arrow,
    // Operators
    Exponent,
    And,
    Or,
    Xor,
    Eqq,
    Neq,
    Lt,
    Gt,
    Leq,
    Geq,
    // Keywords
    Int,
    UInt,
    Byte,
    Float,
    Bool,
    Char,
    Let,
    Mut,
    Const,
    Fn,
    Record,
    Enum,
    If,
    Then,
    Else,
    For,
    In,
    While,
    Do,
    Match,
    With,
    Return,
    Break,
    Continue,
    True,
    False,
    // Misc
    Ident,
}

impl Display for TokKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::IntLit => "int literal",
                Self::FloatLit => "float literal",
                Self::StringLit => "string literal",
                Self::CharLit => "char literal",
                Self::LParen => "`(`",
                Self::RParen => "`)`",
                Self::LBrace => "`{`",
                Self::RBrace => "`}`",
                Self::LBracket => "`[`",
                Self::RBracket => "`]`",
                Self::Eq => "`=`",
                Self::Ampersand => "`&`",
                Self::Pipe => "`|`",
                Self::Bang => "`!`",
                Self::Lt => "`<`",
                Self::Gt => "`>`",
                Self::Plus => "`+`",
                Self::Minus => "`-`",
                Self::Times => "`*`",
                Self::FSlash => "`/`",
                Self::BSlash => r"`\`",
                Self::Dot => "`.`",
                Self::Comma => "`,`",
                Self::Colon => "`:`",
                Self::Semicolon => "`;`",
                Self::Underscore => "`_`",
                Self::Arrow => "`->`",
                Self::Exponent => "`**`",
                Self::And => "`&&`",
                Self::Or => "`||`",
                Self::Xor => "`^`",
                Self::Eqq => "`==`",
                Self::Neq => "`!=`",
                Self::Leq => "`<=`",
                Self::Geq => "`>=`",
                Self::Int => "`Int`",
                Self::UInt => "`UInt`",
                Self::Byte => "`Byte`",
                Self::Float => "`Float`",
                Self::Bool => "`Bool`",
                Self::Char => "`Char`",
                Self::Let => "`let`",
                Self::Mut => "`mut`",
                Self::Const => "`const`",
                Self::Fn => "`fn`",
                Self::Record => "`record`",
                Self::Enum => "`enum`",
                Self::If => "`if`",
                Self::Then => "`then`",
                Self::Else => "`else`",
                Self::For => "`for`",
                Self::In => "`in`",
                Self::While => "`while`",
                Self::Do => "`do`",
                Self::Match => "`match`",
                Self::With => "`with`",
                Self::Return => "`return`",
                Self::Break => "`break`",
                Self::Continue => "`continue`",
                Self::True => "`true`",
                Self::False => "`false`",
                Self::Ident => "identifier",
            }
        )
    }
}

impl TokKind {
    pub fn span(self, src: &str, span: impl Into<Span>) -> Tok<'_> {
        let span = span.into();
        Tok {
            kind: self,
            span,
            src: &src[span.start..span.end],
        }
    }

    #[cfg(any(test, feature = "test"))]
    pub fn reverse(&self) -> String {
        match self {
            Self::IntLit => String::from("1"),
            Self::FloatLit => String::from("1.1"),
            Self::StringLit => String::from(r#""Hello, World!""#),
            Self::CharLit => String::from("'a'"),
            Self::Ident => String::from("foo"),
            _ => self.to_string().trim_matches('`').to_string(),
        }
    }

    #[cfg(any(test, feature = "test"))]
    pub fn arb() -> impl Strategy<Value = Self> {
        Self::arbitrary()
    }
}
