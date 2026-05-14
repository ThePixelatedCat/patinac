use ena::unify::UnifyKey;

use hir::types::{Param, Return};
use parse::Parser;

use crate::{ErrorKind, PartialTy, Result, Ty, TypeChecker, types::TyVar};

fn check_expr(input: &str) -> Result<Ty> {
    let expr = Parser::parse_expr(input).unwrap();
    let (expr, hir) = nameres::test_resolve_expr(expr).unwrap();
    let mut checker = TypeChecker::default();
    checker.infer_expr(&hir, expr)?;
    checker.unify()?;
    let mut expr_mapping = checker.sub_all(&hir)?;
    Ok(expr_mapping.remove(expr).expect(&format!("{expr:?}")))
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

// #[test]
// fn type_of_if_err() {
//     assert_eq!(
//         check_expr(r#"if true then "true" else false"#),
//         Err(ErrorKind::TypesNotEqual(PartialTy::Bool, PartialTy::string()).span(25..30))
//     )
// }

#[test]
fn array() {
    assert_eq!(
        check_expr("[1.0, 2.0, 9.0 / 3.0, 4.0, -5.0].[0]"),
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
fn lambdas() {
    assert_eq!(
        check_expr("fn(mut a, b: Float) -> {a = a + b a}"),
        Ok(Ty::Fn(
            vec![
                Param {
                    mutable: true,
                    ty: Ty::UInt
                },
                Param {
                    mutable: false,
                    ty: Ty::UInt
                }
            ],
            Return {
                mutable: false,
                ty: Box::new(Ty::UInt)
            }
        ))
    );
}

#[test]
fn maths() {
    assert_eq!(
        check_expr("1 + 1.0"),
        Err(
            ErrorKind::TypesNotEqual(PartialTy::IntVar(TyVar::from_index(0)), PartialTy::Float)
                .span(0..1)
        )
    );
    assert_eq!(check_expr("1.0 + 1.0"), Ok(Ty::Float))
}

#[test]
fn type_of_int() {
    assert_eq!(
        check_expr("{let a = 5.0 let b: Float = 1.0 a + b}"),
        Ok(Ty::Float)
    );
}

#[test]
fn type_of_block() {
    let input = "{
    let mut y: Float = 5.0
    3.0 + 1.0 - 2.0
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
fn fac(n: Float): Float -> 
    if n == 0.0 { 1.0 } else { n * fac(n - 1.0) }      
";

    assert_eq!(check_full(input), Ok(()));
}

#[test]
fn option() {
    let input = "\
enum Option[T] {
    Some(v: T),
    None()
}

fn map[T, U](self: Option[T], f: fn(T) -> U): Option[U] -> 
    self.match {
        Some(v) -> f(v),
        None() -> None()
    }
";

    assert_eq!(check_full(input), Ok(()))
}
