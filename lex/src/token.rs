use displaydoc::Display;
use logos::Logos;
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
#[derive(Logos, PartialEq, Eq, Debug, Display, Clone, Copy)]
#[logos(skip(r"(\p{Pattern_White_Space}+)|(//.*)", allow_greedy = true))]
#[logos(subpattern dec_int = "([0-9][0-9_]*)")]
#[logos(subpattern escape = r#"((\\\\)|(\\')|(\\")|(\\0)|(\\t)|(\\n)|(\\r)|(\\u\{[0-9a-fA-F]{1,6}\}))"#)]
pub enum TokKind {
    /* LITERALS */
    /// integer literal
    #[regex("(?&dec_int)|(0b[0-1][0-1_]*)|(0o[0-7][0-7_]*)|(0x[0-9a-fA-F][0-9a-fA-F_]*)")]
    IntLit,
    /// float literal
    #[regex(r"(?&dec_int)\.(?&dec_int)([Ee]-?(?&dec_int))?")]
    FloatLit,
    /// string literal
    #[regex(r##"("([^"\\]|(?&escape))*")|((?s)#".*"#)"##, allow_greedy = true)]
    StringLit,
    /// character literal
    #[regex(r"'([^\t\n\r'\\]|(?&escape))'")]
    CharLit,

    /* DELIMITERS */
    /// `(`
    #[token("(")]
    LParen,
    /// `)`
    #[token(")")]
    RParen,
    /// `{{`
    #[token("{")]
    LBrace,
    /// `}}`
    #[token("}")]
    RBrace,
    /// `[`
    #[token("[")]
    LBracket,
    /// `]`
    #[token("]")]
    RBracket,

    /* SYMBOLS */
    /// `=`
    #[token("=")]
    Eq,
    /// `&`
    #[token("&")]
    Ampersand,
    /// `|`
    #[token("|")]
    Pipe,
    /// `\`
    #[token("\\")]
    BSlash,
    /// `.`
    #[token(".")]
    Dot,
    /// `,`
    #[token(",")]
    Comma,
    /// `:`
    #[token(":")]
    Colon,
    /// `;`
    #[token(";")]
    Semicolon,
    /// `_`
    #[token("_")]
    Underscore,
    /// `->`
    #[token("->")]
    Arrow,
    /// `::`
    #[token("::")]
    PathSep,
    /// `#`
    #[token("#")]
    Hash,

    /* OPERATORS */
    /// `+`
    #[token("+")]
    Plus,
    /// `-`
    #[token("-")]
    Minus,
    /// `*`
    #[token("*")]
    Times,
    /// `/`
    #[token("/")]
    Divide,
    /// `**`
    #[token("**")]
    Exponent,
    /// `&&`
    #[token("&&")]
    And,
    /// `||`
    #[token("||")]
    Or,
    /// `^`
    #[token("^")]
    Xor,
    /// `!`
    #[token("!")]
    Bang,
    /// `==`
    #[token("==")]
    Eqq,
    /// `!=`
    #[token("!=")]
    Neq,
    /// `<`
    #[token("<")]
    Lt,
    /// `>`
    #[token(">")]
    Gt,
    /// `<=`
    #[token("<=")]
    Leq,
    /// `>=`
    #[token(">=")]
    Geq,

    /* KEYWORDS */
    /// `Int`
    #[token("Int")]
    Int,
    /// `UInt`
    #[token("UInt")]
    UInt,
    /// `Byte`
    #[token("Byte")]
    Byte,
    /// `Float`
    #[token("Float")]
    Float,
    /// `Bool`
    #[token("Bool")]
    Bool,
    /// `Char`
    #[token("Char")]
    Char,
    /// `let`
    #[token("let")]
    Let,
    /// `mut`
    #[token("mut")]
    Mut,
    /// `const`
    #[token("const")]
    Const,
    /// `fn`
    #[token("fn")]
    Fn,
    /// `record`
    #[token("record")]
    Record,
    /// `enum`
    #[token("enum")]
    Enum,
    /// `if`
    #[token("if")]
    If,
    /// `then`
    #[token("then")]
    Then,
    /// `else`
    #[token("else")]
    Else,
    /// `match`
    #[token("match")]
    Match,
    /// `with`
    #[token("with")]
    With,
    /// `for`
    #[token("for")]
    For,
    /// `in`
    #[token("in")]
    In,
    /// `do`
    #[token("do")]
    Do,
    /// `loop`
    #[token("loop")]
    Loop,
    /// `return`
    #[token("return")]
    Return,
    /// `break`
    #[token("break")]
    Break,
    /// `continue`
    #[token("continue")]
    Continue,
    /// `true`
    #[token("true")]
    True,
    /// `false`
    #[token("false")]
    False,

    /* MISC */
    /// identifier
    #[regex(r"\p{XID_Start}\p{XID_Continue}*")]
    Ident,
    /// end of file
    Eof,
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

    /// Converts the token into a string that parses back into itself
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

    /// A strategy that produces random tokens, excluding [Eof][Self::Eof]
    #[cfg(any(test, feature = "test"))]
    pub fn arb() -> impl Strategy<Value = Self> {
        Self::arbitrary().prop_filter("skipped eof", |&t| t != Self::Eof)
    }
}
