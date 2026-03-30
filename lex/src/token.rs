use span::Span;
use std::fmt::Display;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Tok {
    pub kind: TokKind,
    pub span: Span,
}
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
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
    True,
    False,
    // Misc
    Ident,
    Error,
    Eof,
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
                Self::LParen => "(",
                Self::RParen => ")",
                Self::LBrace => "INDENT",
                Self::RBrace => "DEDENT",
                Self::LBracket => "[",
                Self::RBracket => "]",
                Self::Eq => "=",
                Self::Ampersand => "&",
                Self::Pipe => "|",
                Self::Bang => "!",
                Self::Lt => "<",
                Self::Gt => ">",
                Self::Plus => "+",
                Self::Minus => "-",
                Self::Times => "*",
                Self::FSlash => "/",
                Self::BSlash => "\\",
                Self::Dot => ".",
                Self::Comma => ",",
                Self::Colon => ":",
                Self::Semicolon => ";",
                Self::Underscore => "_",
                Self::Arrow => "->",
                Self::Exponent => "**",
                Self::And => "&&",
                Self::Or => "||",
                Self::Xor => "^",
                Self::Eqq => "==",
                Self::Neq => "!=",
                Self::Leq => "<=",
                Self::Geq => ">=",
                Self::Int => "Int",
                Self::UInt => "UInt",
                Self::Byte => "Byte",
                Self::Float => "Float",
                Self::Bool => "Bool",
                Self::Char => "Char",
                Self::Let => "let",
                Self::Mut => "mut",
                Self::Const => "const",
                Self::Fn => "fn",
                Self::Record => "struct",
                Self::Enum => "enum",
                Self::If => "if",
                Self::Then => "then",
                Self::Else => "else",
                Self::For => "for",
                Self::In => "in",
                Self::While => "while",
                Self::Do => "do",
                Self::Match => "match",
                Self::With => "with",
                Self::True => "true",
                Self::False => "false",
                Self::Ident => "identifier",
                Self::Error => "ERROR",
                Self::Eof => "eof",
            }
        )
    }
}

impl TokKind {
    pub fn span(self, span: impl Into<Span>) -> Tok {
        Tok {
            kind: self,
            span: span.into(),
        }
    }
}
