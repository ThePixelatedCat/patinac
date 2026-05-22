use errors::{Result, TEST_HANDLER};
use hir::Hir;
use parse::Parser;

use crate::{Scope, exprs, resolve};

fn test_resolve_expr(input: &str) -> Result<Hir> {
    let expr = Parser::parse_expr(input).unwrap();
    let mut hir = Hir::default();
    let mut handler = TEST_HANDLER;
    exprs::resolve_expr(
        &Scope::default(),
        &Scope::default(),
        &mut hir,
        &mut handler,
        expr,
    )?;
    Ok(hir)
}

fn test_resolve_full(input: &str) -> Result<Hir> {
    resolve(
        Parser::new(input, TEST_HANDLER).parse().unwrap(),
        TEST_HANDLER,
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
    assert!(test_resolve_expr("a + 5").is_err(),);
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
    assert!(test_resolve_expr(r#"{let x: String = "Hello, World!"}"#).is_err());
}

#[test]
fn rebinding() {
    let input = "
    const A = 5
    const A = 6
";
    assert!(test_resolve_full(input).is_err(),);

    let input = "
    record Foo()
    record Foo(val: UInt)
";
    assert!(test_resolve_full(input).is_err(),);
}
