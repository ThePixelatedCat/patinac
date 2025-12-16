use std::mem::{self, Discriminant};

#[derive(PartialEq, Debug, Clone)]
pub enum Token {
    // Literals
    IntLit(i64),
    FloatLit(f64),
    StringLit(String),
    CharLit(char),
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
    Bar,
    Bang,
    UpArrow,
    LAngle,
    RAngle,
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
    // Operators
    And,
    Or,
    Eqq,
    Neq,
    Leq,
    Geq,
    // Keywords
    Let,
    Mut,
    Fn,
    If,
    Else,
    Match,
    True,
    False,
    // Misc
    Ident(String),
    Error { start: usize, end: usize },
    Eof,
}

impl Token {
    pub const fn ty(&self) -> Discriminant<Self> {
        mem::discriminant(self)
    }
}
