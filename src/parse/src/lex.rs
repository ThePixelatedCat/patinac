use std::range::Range;

use derive_more::Display;
use itertools::MultiPeek;
use logos::Logos;

pub type Lexer<'src> = MultiPeek<Box<dyn Iterator<Item = Result<Tok, Range<u32>>> + 'src>>;

/// Produces an iterator over tokens extracted from the source.
pub fn lex(src: &str) -> Lexer<'_> {
    let iter = TokKind::lexer(src).spanned().map(|(tok, span)| {
        let span = u32::try_from(span.start).expect("file too long")
            ..u32::try_from(span.end).expect("file too long");
        match tok {
            Ok(tok) => Ok(tok.span(span)),
            Err(()) => Err(Range::from(span)),
        }
    });
    let boxed_iter: Box<dyn Iterator<Item = _>> = Box::new(iter);
    itertools::multipeek(boxed_iter)
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub struct Tok {
    pub kind: TokKind,
    pub span: Range<u32>,
}

#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[derive(Logos, PartialEq, Eq, Debug, Display, Clone, Copy)]
#[logos(skip(r"//.*", allow_greedy = true))]
#[logos(subpattern dec_int = "([0-9][0-9_]*)")]
#[logos(subpattern escape = r#"((\\\\)|(\\")|(\\0)|(\\t)|(\\n)|(\\r)|(\\u\{[0-9a-fA-F]{1,6}\}))"#)]
pub enum TokKind {
    /* LITERALS */
    /// integer literal.
    #[regex("(?&dec_int)|(0b[0-1][0-1_]*)|(0o[0-7][0-7_]*)|(0x[0-9a-fA-F][0-9a-fA-F_]*)")]
    IntLit,
    /// float literal.
    #[regex(r"(?&dec_int)\.(?&dec_int)([Ee]-?(?&dec_int))?")]
    FloatLit,
    /// string literal.
    #[regex(r##"("([^"\\]|(?&escape))*")|((?s)#".*"#)"##, allow_greedy = true)]
    StringLit,

    /* DELIMITERS */
    /// `(`.
    #[display("(")]
    #[token("(")]
    LParen,
    /// `)`.
    #[display(")")]
    #[token(")")]
    RParen,
    /// `{`.
    #[display("{{")]
    #[token("{")]
    LBrace,
    /// `}`.
    #[display("}}")]
    #[token("}")]
    RBrace,
    /// `[`.
    #[display("[")]
    #[token("[")]
    LBracket,
    /// `]`.
    #[display("]")]
    #[token("]")]
    RBracket,

    /* SYMBOLS */
    /// `=`.
    #[display("=")]
    #[token("=")]
    Eq,
    /// `.`.
    #[display(".")]
    #[token(".")]
    Dot,
    /// `,`.
    #[display(",")]
    #[token(",")]
    Comma,
    /// `:`.
    #[display(":")]
    #[token(":")]
    Colon,
    /// `_`.
    #[display("_")]
    #[token("_")]
    Underscore,
    /// `->`.
    #[display("->")]
    #[token("->")]
    Arrow,
    /// `::`.
    #[display("::")]
    #[token("::")]
    PathSep,

    /* OPERATORS */
    /// `+`.
    #[display("+")]
    #[token("+")]
    Plus,
    /// `+.`.
    #[display("+.")]
    #[token("+.")]
    PlusF,
    /// `-`.
    #[display("-")]
    #[token("-")]
    Minus,
    /// `-.`.
    #[display("-.")]
    #[token("-.")]
    MinusF,
    /// `*`.
    #[display("*")]
    #[token("*")]
    Times,
    /// `*.`.
    #[display("*.")]
    #[token("*.")]
    TimesF,
    /// `/`.
    #[display("/")]
    #[token("/")]
    Divide,
    /// `/.`.
    #[display("/.")]
    #[token("/.")]
    DivideF,
    /// `**`.
    #[display("**")]
    #[token("**")]
    Exponent,
    /// `&&`.
    #[display("&&")]
    #[token("&&")]
    And,
    /// `||`.
    #[display("||")]
    #[token("||")]
    Or,
    /// `^`.
    #[display("^")]
    #[token("^")]
    Xor,
    /// `!`.
    #[display("!")]
    #[token("!")]
    Bang,
    /// `==`.
    #[display("==")]
    #[token("==")]
    Eqq,
    /// `!=`.
    #[display("!=")]
    #[token("!=")]
    Neq,
    /// `<`.
    #[display("<")]
    #[token("<")]
    Lt,
    /// `>`.
    #[display(">")]
    #[token(">")]
    Gt,
    /// `<=`.
    #[display("<=")]
    #[token("<=")]
    Leq,
    /// `>=`.
    #[display(">=")]
    #[token(">=")]
    Geq,

    /* KEYWORDS */
    /// `Int`.
    #[display("Int")]
    #[token("Int")]
    Int,
    /// `UInt`.
    #[display("UInt")]
    #[token("UInt")]
    UInt,
    /// `Byte`.
    #[display("Byte")]
    #[token("Byte")]
    Byte,
    /// `Float`.
    #[display("Float")]
    #[token("Float")]
    Float,
    /// `Bool`.
    #[display("Bool")]
    #[token("Bool")]
    Bool,
    /// `Char`.
    #[display("Char")]
    #[token("Char")]
    Char,
    /// `Fn`.
    #[display("Fn")]
    #[token("Fn")]
    FnTy,
    /// `let`.
    #[display("let")]
    #[token("let")]
    Let,
    /// `mut`.
    #[display("mut")]
    #[token("mut")]
    Mut,
    /// `import`.
    #[display("import")]
    #[token("import")]
    Import,
    /// `export`.
    #[display("export")]
    #[token("export")]
    Export,
    /// `opaque`.
    #[display("opaque")]
    #[token("opaque")]
    Opaque,
    /// `record`.
    #[display("record")]
    #[token("record")]
    Record,
    /// `union`.
    #[display("union")]
    #[token("union")]
    Union,
    /// `const`.
    #[display("const")]
    #[token("const")]
    Const,
    /// `fn`.
    #[display("fn")]
    #[token("fn")]
    Fn,
    /// `if`.
    #[display("if")]
    #[token("if")]
    If,
    /// `else`.
    #[display("else")]
    #[token("else")]
    Else,
    /// `match`.
    #[display("match")]
    #[token("match")]
    Match,
    /// `for`.
    #[display("for")]
    #[token("for")]
    For,
    /// `in`.
    #[display("in")]
    #[token("in")]
    In,
    /// `loop`.
    #[display("loop")]
    #[token("loop")]
    Loop,
    /// `return`.
    #[display("return")]
    #[token("return")]
    Return,
    /// `break`.
    #[display("break")]
    #[token("break")]
    Break,
    /// `continue`.
    #[display("continue")]
    #[token("continue")]
    Continue,
    /// `true`.
    #[display("true")]
    #[token("true")]
    True,
    /// `false`.
    #[display("false")]
    #[token("false")]
    False,

    /// `print`.
    #[display("print")]
    #[token("print")]
    Print,

    /* MISC */
    /// identifier.
    #[regex(r"\p{XID_Start}\p{XID_Continue}*")]
    Ident,
    /// whitespace.
    #[regex(r"\p{Pattern_White_Space}+")]
    Whitespace,
    /// end-of-file.
    #[cfg_attr(test, proptest(skip))]
    Eof,
}

impl TokKind {
    pub fn span(self, span: impl Into<Range<u32>>) -> Tok {
        Tok {
            kind: self,
            span: span.into(),
        }
    }

    /// Converts the token into a string that parses back into itself.
    ///
    /// Used for testing.
    #[allow(
        unused,
        reason = "It's used in tests, but the linter doesn't consider that apparently"
    )]
    pub(crate) fn reverse(self) -> String {
        match self {
            Self::IntLit => String::from("1"),
            Self::FloatLit => String::from("1.1"),
            Self::StringLit => String::from(r#""Hello, World!""#),
            Self::Ident => String::from("foo"),
            Self::Whitespace => String::from(" \t"),
            _ => self.to_string().trim_matches('`').to_string(),
        }
    }
}
