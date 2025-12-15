use super::{Lexer, token::Token as T};

/// walks `$tokens` and compares them to the given kinds.
macro_rules! assert_tokens {
    ($tokens:ident, [$($token:expr,)*]) => {
        {
            let mut iter = $tokens.iter();
            $(
                let token = iter.next().expect("not enough tokens");
                assert_eq!(*token, $token);
            )*
        }
    };
}

#[test]
fn single_char_tokens() {
    let mut lexer = Lexer::new("+-(.):");
    let tokens = lexer.tokenize();
    assert_tokens!(
        tokens,
        [
            T::Plus,
            T::Minus,
            T::LParen,
            T::Dot,
            T::RParen,
            T::Colon,
            T::Eof,
        ]
    );
}

#[test]
fn unknown_input() {
    let mut lexer = Lexer::new("{$$$$$$$+");
    let tokens = lexer.tokenize();
    assert_tokens!(
        tokens,
        [T::LBrace, T::Error { start: 1, end: 8 }, T::Plus, T::Eof,]
    );
}

#[test]
fn single_char_tokens_with_whitespace() {
    let mut lexer = Lexer::new("   + -  (.): ");
    let tokens = lexer.tokenize();
    assert_tokens!(
        tokens,
        [
            T::Plus,
            T::Minus,
            T::LParen,
            T::Dot,
            T::RParen,
            T::Colon,
            T::Eof,
        ]
    );
}

#[test]
fn maybe_multiple_char_tokens() {
    let mut lexer = Lexer::new("&&=<=_!=||");
    let tokens = lexer.tokenize();
    assert_tokens!(
        tokens,
        [T::And, T::Eq, T::Leq, T::Underscore, T::Neq, T::Or, T::Eof,]
    );
}

#[test]
fn keywords() {
    let mut lexer = Lexer::new("if let = match else fn");
    let tokens: Vec<_> = lexer.tokenize();
    assert_tokens!(
        tokens,
        [T::If, T::Let, T::Eq, T::Match, T::Else, T::Fn, T::Eof,]
    );
}

#[test]
fn comment() {
    let mut lexer = Lexer::new("//hello, world!\nif let");
    let tokens: Vec<_> = lexer.tokenize();
    assert_tokens!(tokens, [T::If, T::Let,]);
}

#[test]
fn literals() {
    let mut lexer = Lexer::new(r#"1 .5 0.211 -1. true "test"'\n'"#);
    let tokens: Vec<_> = lexer.tokenize();
    assert_tokens!(
        tokens,
        [
            T::IntLit(1),
            T::FloatLit(0.5),
            T::FloatLit(0.211),
            T::FloatLit(-1.0),
            T::True,
            T::StringLit("test".into()),
            T::CharLit('\n'),
            T::Eof,
        ]
    );
}

#[test]
fn function() {
    let input = r#"
        // this is a comment!
        fn test(var: Type, var2_: bool) {
            let x = '\n' + "String content \"\\ test" + 7 / 27.3e-2^4;
            let chars = x.chars();
            if let Some(c) = chars.next() {
                x = x + c;
            } else if !var2_ {
                x = x + ",";
            }
        }
    "#;
    let mut lexer = Lexer::new(&input);
    let tokens = lexer.tokenize();
    assert_tokens!(
        tokens,
        [
            // function signature
            T::Fn,
            T::Ident("test".into()),
            T::LParen,
            T::Ident("var".into()),
            T::Colon,
            T::Ident("Type".into()),
            T::Comma,
            T::Ident("var2_".into()),
            T::Colon,
            T::Ident("bool".into()),
            T::RParen,
            T::LBrace,
            // `x` assignment
            T::Let,
            T::Ident("x".into()),
            T::Eq,
            T::CharLit('\n'),
            T::Plus,
            T::StringLit("String content \"\\ test".into()),
            T::Plus,
            T::IntLit(7),
            T::FSlash,
            T::FloatLit(27.3e-2),
            T::UpArrow,
            T::IntLit(4),
            T::Semicolon,
            // `chars` assignment
            T::Let,
            T::Ident("chars".into()),
            T::Eq,
            T::Ident("x".into()),
            T::Dot,
            T::Ident("chars".into()),
            T::LParen,
            T::RParen,
            T::Semicolon,
            // if
            T::If,
            T::Let,
            T::Ident("Some".into()),
            T::LParen,
            T::Ident("c".into()),
            T::RParen,
            T::Eq,
            T::Ident("chars".into()),
            T::Dot,
            T::Ident("next".into()),
            T::LParen,
            T::RParen,
            T::LBrace,
            // `x` re-assignment
            T::Ident("x".into()),
            T::Eq,
            T::Ident("x".into()),
            T::Plus,
            T::Ident("c".into()),
            T::Semicolon,
            // else if
            T::RBrace,
            T::Else,
            T::If,
            T::Bang,
            T::Ident("var2_".into()),
            T::LBrace,
            // `x` re-assignment
            T::Ident("x".into()),
            T::Eq,
            T::Ident("x".into()),
            T::Plus,
            T::StringLit(",".into()),
            T::Semicolon,
            T::RBrace, // end if
            T::RBrace, // end fn
            T::Eof,
        ]
    );
}
