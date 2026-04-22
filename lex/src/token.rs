use displaydoc::Display;
#[cfg(any(test, feature = "test"))]
use proptest::{arbitrary::Arbitrary, prelude::Strategy};
use span::Span;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Tok<'src> {
    pub kind: TokKind,
    pub span: Span,
    pub src: &'src str,
}

#[cfg_attr(any(test, feature = "test"), derive(proptest_derive::Arbitrary))]
#[derive(Display, PartialEq, Eq, Debug, Clone, Copy)]
pub enum TokKind {
    /* LITERALS */
    /// int literal
    IntLit,
    /// float literal
    FloatLit,
    /// string literal
    StringLit,
    /// char literal
    CharLit,

    /* DELIMITERS */
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `{{`
    LBrace,
    /// `}}`
    RBrace,
    /// `[`
    LBracket,
    /// `]`
    RBracket,

    /* SYMBOLS */
    /// `=`
    Eq,
    /// `&`
    Ampersand,
    /// `|`
    Pipe,
    /// `\`
    BSlash,
    /// `.`
    Dot,
    /// `,`
    Comma,
    /// `:`
    Colon,
    /// `;`
    Semicolon,
    /// `_`
    Underscore,
    /// `->`
    Arrow,

    /* OPERATORS */
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `*`
    Times,
    /// `/`
    Divide,
    /// `**`
    Exponent,
    /// `&&`
    And,
    /// `||`
    Or,
    /// `^`
    Xor,
    /// `!`
    Bang,
    /// `==`
    Eqq,
    /// `!=`
    Neq,
    /// `<`
    Lt,
    /// `>`
    Gt,
    /// `<=`
    Leq,
    /// `>=`
    Geq,

    /* KEYWORDS */
    /// `Int`
    Int,
    /// `UInt`
    UInt,
    /// `Byte`
    Byte,
    /// `Float`
    Float,
    /// `Bool`
    Bool,
    /// `Char`
    Char,
    /// `let`
    Let,
    /// `mut`
    Mut,
    /// `const`
    Const,
    /// `fn`
    Fn,
    /// `record`
    Record,
    /// `enum`
    Enum,
    /// `if`
    If,
    /// `then`
    Then,
    /// `else`
    Else,
    /// `for`
    For,
    /// `in`
    In,
    /// `while`
    While,
    /// `do`
    Do,
    /// `match`
    Match,
    /// `with`
    With,
    /// `return`
    Return,
    /// `break`
    Break,
    /// `continue`
    Continue,
    /// `true`
    True,
    /// `false`
    False,

    /* MISC */
    /// Identifier
    Ident,
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
