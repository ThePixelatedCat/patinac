use ena::unify::UnifyKey;

use hir::types::{Param, Return};
use parse::Parser;

use crate::{ErrorKind, PartialTy, Result, Ty, TypeChecker, types::TyVar};

fn check_expr(input: &str) -> Result<Ty> {
    let (expr, hir) = nameres::test_resolve_expr(Parser::parse_expr(input).unwrap()).unwrap();

    let mut checker = TypeChecker::default();
    let ctx = checker.build_context(&hir);
    checker.infer_expr(&ctx, &hir, expr)?;
    checker.unify()?;

    Ok(checker.sub_all(&hir)?.get(expr).clone())
}

fn check_full(input: &str) -> Result<()> {
    let toks = lex::lex(input).unwrap();
    let ast = Parser::new(toks).parse().unwrap();
    let mut hir = nameres::resolve(ast).unwrap();
    TypeChecker::default().type_program(&mut hir)?;
    Ok(())
}

#[test]
fn type_of_if_single_branch() {
    let input = "if true {false #()}";
    assert_eq!(check_expr(input), Ok(Ty::unit()))
}

#[test]
fn type_of_if_single_branch_err() {
    assert_eq!(
        check_expr("if true { 5.0 }"),
        Err(ErrorKind::TypesNotEqual(PartialTy::Float, PartialTy::unit()).span(8..15))
    )
}

#[test]
fn type_of_if() {
    assert_eq!(check_expr("if true { 5.0 } else { -3.0 }"), Ok(Ty::Float))
}

#[test]
fn type_of_if_err() {
    assert_eq!(
        check_expr(r#"if true { 5.0 } else { false }"#),
        Err(ErrorKind::TypesNotEqual(PartialTy::Bool, PartialTy::Float).span(21..30))
    )
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
    assert_eq!(
        check_expr("[1, 2.0]"),
        Err(
            ErrorKind::TypesNotEqual(PartialTy::Float, PartialTy::IntVar(TyVar::from_index(1)))
                .span(4..7)
        )
    );
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
                ty: Ty::Float
            }],
            Return {
                mutable: false,
                ty: Ty::unit().into()
            }
        ))
    );
}

#[test]
fn maths() {
    assert_eq!(
        check_expr("1 + 1.0"),
        Err(
            ErrorKind::TypesNotEqual(PartialTy::Float, PartialTy::IntVar(TyVar::from_index(1)))
                .span(4..7)
        )
    );
    assert_eq!(check_expr("1.0 +. 1.0"), Ok(Ty::Float));
    assert_eq!(
        check_expr("{let mut a = 5 let b = 5 a = b}"),
        Err(ErrorKind::UninferredType.span(13..14))
    );
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
    assert_eq!(check_full(input), Err(ErrorKind::MissingField.span(58..59)));
}
