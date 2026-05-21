use pretty_assertions::assert_eq;

use hir::Hir;
use ident::Ident;
use parse::{TEST_HANDLER, Parser};
use span::Span;

use crate::{
    Scope,
    error::{ErrorKind, Result},
    resolve, resolve_expr,
};

fn test_resolve_expr(input: &str) -> Result<Hir> {
    let expr = Parser::parse_expr(input).unwrap();
    let mut hir = Hir::default();
    resolve_expr(&Scope::default(), &Scope::default(), &mut hir, expr)?;
    Ok(hir)
}

fn test_resolve_full(input: &str) -> Result<Hir> {
    resolve(
        Parser::new(lex::lex(input).unwrap(), TEST_HANDLER)
            .parse()
            .unwrap(),
    )
}

#[test]
fn lambda() {
    let input = "{
    let mut a = 5
    let b = 6
    fn(c) -> a + b + c
}";
    assert!(test_resolve_expr(input).is_ok());
}

#[test]
fn for_() {
    assert!(test_resolve_expr("for x in [1, 2, 3] { x + 5 }").is_ok());
}

#[test]
fn shadowing() {
    let input = r#"{
    let a: UInt = 5
    let a = "Hello, World"
    {let a = true}
    a
}"#;
    assert!(test_resolve_expr(input).is_ok());
}

#[test]
fn fib() {
    let input = "
    fn fib(n: UInt): UInt ->
        if n <= 1 { n } else { fib(n - 1) + fib(n - 2) }         
";
    assert!(test_resolve_full(input).is_ok());
}

#[test]
fn unbound_var() {
    assert_eq!(
        test_resolve_expr("a + 5").unwrap_err(),
        ErrorKind::UnboundVariable.span(0..1)
    );
}

#[test]
fn consts() {
    let input = "
    const B = A * 2
    const A = 5
";
    assert!(test_resolve_full(input).is_ok());
}

#[test]
fn list() {
    let input = r#"
    record List(head: Link)

    record Link(elem: Int, next: Link)

    fn cons(list: List, elem: Int): List -> "todo"
"#;
    assert!(test_resolve_full(input).is_ok());
}

#[test]
fn unknown_type() {
    assert_eq!(
        test_resolve_expr(r#"{let x: String = "Hello, World!"}"#).unwrap_err(),
        ErrorKind::UnknownType.span(8..14)
    );
}

#[test]
fn rebinding() {
    let input = "
    const A = 5
    const A = 6
";
    assert_eq!(
        test_resolve_full(input).unwrap_err(),
        ErrorKind::DupItem(Ident::new("A"), Span::from(11..12)).span(27..28)
    );

    let input = "
    record Foo()
    record Foo(val: UInt)
";
    assert_eq!(
        test_resolve_full(input).unwrap_err(),
        ErrorKind::DupItem(Ident::new("Foo"), Span::from(12..15)).span(29..32)
    );
}
