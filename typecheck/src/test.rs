use errors::{Result, TEST_HANDLER};
use hir::types::{Param, Ty};
use parse::Parser;
use span::Span;

use crate::TypeChecker;

fn check_expr(input: &str) -> Result<Ty> {
    let expr = Parser::parse_expr(input).unwrap();
    let (expr, mut hir) = nameres::test_resolve_expr(expr).unwrap();

    let mut checker = TypeChecker::new(TEST_HANDLER);
    checker.build_context(&hir);
    checker.infer_expr(&hir, expr)?;
    checker.unify();

    Ok(checker.sub_all(&mut hir)?.ty(expr).clone())
}

fn check_full(input: &str) -> Result<()> {
    let ast = Parser::new(input, TEST_HANDLER).parse().unwrap();
    let mut hir = nameres::resolve(ast, TEST_HANDLER).unwrap();
    TypeChecker::new(TEST_HANDLER).type_program(&mut hir)?;
    Ok(())
}

#[test]
fn type_of_if_single_branch() {
    let input = "if true {false #()}";
    assert_eq!(check_expr(input), Ok(Ty::unit()));
}

#[test]
fn type_of_if_single_branch_err() {
    assert!(check_expr("if true { 5.0 }").is_err());
}

#[test]
fn type_of_if() {
    assert_eq!(check_expr("if true { 5.0 } else { -3.0 }"), Ok(Ty::Float));
}

#[test]
fn type_of_if_err() {
    assert!(check_expr("if true { 5.0 } else { false }").is_err());
}

#[test]
fn array() {
    assert_eq!(
        check_expr("[1.0, 2.0, 9.0 /. 3.0, 4.0, -5.0].[0]"),
        Ok(Ty::Float)
    );
}

#[test]
fn mismatched_array() {
    assert!(check_expr("[1, 2.0]").is_err(),);
}

#[test]
fn vars() {
    assert_eq!(check_expr("{let mut a: Byte = 1 a = 2}"), Ok(Ty::unit()));
}

#[test]
fn inc() {
    assert_eq!(
        check_expr("fn(mut a) -> a = a +. 1.0"),
        Ok(Ty::Fn(
            vec![Param {
                mutable: true,
                ty: Ty::Float,
                span: Span::from(3..8)
            }],
            Ty::unit().into()
        ))
    );
}

#[test]
fn maths() {
    assert!(check_expr("1 + 1.0").is_err());
    assert_eq!(check_expr("1.0 +. 1.0"), Ok(Ty::Float));
    assert!(check_expr("{let mut a = 5 let b = 5 a = b}").is_err());
}

#[test]
fn type_of_int() {
    assert_eq!(check_expr("{let a = 5 let b: Int = 1 a + b}"), Ok(Ty::Int));
}

#[test]
fn type_of_block() {
    let input = "{
    let mut y: Float = 5.0
    3.0 +. 1.0 -. 2.0
    y = 256.0
    if y < 3.0 {
        let a = -5.0
        a
    } else {32.0}
}";

    assert_eq!(check_expr(input), Ok(Ty::Float));
}

#[test]
fn shadowing() {
    let input = "{
        let a: UInt = 5
        let a = 6.0
        {let a = true}
        a
    }";

    assert_eq!(check_expr(input), Ok(Ty::Float));
}

#[test]
fn recursion() {
    let input = "
fn fac(n: UInt): UInt -> 
    if n == 0 { 1 } else { n * fac(n - 1) }      
";

    assert_eq!(check_full(input), Ok(()));
}

#[test]
fn consts() {
    let input = "
    const B: UInt = A * 2
    const A = 5
";
    assert!(check_full(input).is_ok());
}

#[test]
fn fields() {
    let input = "
    record Foo(x: Int)

    fn bar(foo: Foo): Int -> foo.x
";
    assert!(check_full(input).is_ok());

    let input = "
    record Foo(x: Int)

    fn bar(foo: Foo): Int -> foo.y
";
    assert!(check_full(input).is_err());
}
