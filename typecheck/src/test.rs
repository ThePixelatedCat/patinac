use std::sync::LazyLock;

use ena::unify::UnifyKey;
use ident::Ident;
use nameres::AdtId;
use parse::Parser;

use crate::{ErrorKind, PartialTy, Result, Ty, TypeChecker, type_vars::TyVar};
use types::{Param, Return};

fn check_expr(input: &str) -> Result<Ty<AdtId>> {
    let mut checker = TypeChecker::new();

    let typed_expr = checker.infer_expr(
        &TY_ENV,
        &mut Ctx::default(),
        Parser::parse_expr(input).unwrap(),
    )?;
    checker.unify()?;
    checker.sub_expr(typed_expr).map(|e| e.ty)
}

fn check_full(input: &str) -> Result<()> {
    let toks = lex::lex(input).unwrap();
    let ast = Parser::new(toks).parse().unwrap();
    TypeChecker::new().type_program(ast)?;
    Ok(())
}

#[test]
fn type_of_if_single_branch() {
    let input = r#"if true then {"Hi" {}}"#;
    assert_eq!(check_expr(input), Ok(Ty::unit()))
}

#[test]
fn type_of_if_single_branch_err() {
    assert_eq!(
        check_expr("if true then 5.0"),
        Err(ErrorKind::TypesNotEqual(PartialTy::Float, PartialTy::unit()).span(13..16))
    )
}

#[test]
fn type_of_if() {
    assert_eq!(check_expr("if true then 5.0 else -3.0"), Ok(Ty::Float))
}

#[test]
fn type_of_if_err() {
    assert_eq!(
        check_expr(r#"if true then "true" else false"#),
        Err(ErrorKind::TypesNotEqual(PartialTy::Bool, PartialTy::string()).span(25..30))
    )
}

#[test]
fn arrays() {
    assert_eq!(check_expr("[1, 2, 9 / 3, 4, -5].[0]"), Ok(Ty::Int));
}

#[test]
fn vars() {
    assert_eq!(check_expr("{let mut a: Byte = 1 a = 2}"), Ok(Ty::unit()));
}

#[test]
fn lambdas() {
    assert_eq!(
        check_expr("fn(mut a, b): UInt -> {a = a + b a}"),
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
            Box::new(Return {
                mutable: false,
                ty: Ty::UInt
            })
        ))
    );
}

#[test]
fn maths() {
    assert_eq!(
        check_expr("1 + 1.0"),
        Err(
            ErrorKind::TypesNotEqual(PartialTy::IntVar(TyVar::from_index(0)), PartialTy::Float)
                .span(4..7)
        )
    );
    assert_eq!(check_expr("1.0 + 1.0"), Ok(Ty::Float))
}

#[test]
fn type_of_int() {
    assert_eq!(check_expr("{let a = 5 let b: Int = 1 a + b}"), Ok(Ty::Int));
}

#[test]
fn type_of_block() {
    let input = "{
    let mut y: UInt = 5
    3 + 1 - 2
    y = 256
    if y < 3 then {
        let a = -5
        a
    } else 32
}";

    assert_eq!(check_expr(input), Ok(Ty::Int));
}

#[test]
fn shadowing() {
    let input = r#"{
    let a: UInt = 5
    let a = "Hello, World"
    {let a = true}
    a
}"#;

    assert_eq!(check_expr(input), Ok(Ty::string()));
}

#[test]
fn recursion() {
    let input = "
fn fac(n: UInt): UInt -> 
    if n == 0 then
        1 
    else 
        n * fac(n - 1)
";

    assert_eq!(check_full(input), Ok(()));
}

#[test]
fn option() {
    let input = "\
enum Option[T] 
| Some(v: T)
| None()

fn map[T, U](self: Option[T], f: fn(T) -> U): Option[U] -> 
    match self with
    | Some(v) -> Some(f(v))
    | None -> None
";

    assert_eq!(check_full(input), Ok(()))
}
