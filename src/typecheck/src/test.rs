use std::{assert_matches, range::Range};

use ena::unify::UnificationTable;
use slotmap::SecondaryMap;

use errors::{ErrorHandler, Result};
use irs::{
    ModuleId,
    hir::{Param, Ty},
};

use crate::TypeChecker;

#[allow(clippy::unwrap_used, reason = "test utility")]
fn check_expr(src: &str) -> Result<Ty> {
    let (expr, mut hir) = nameres::test_resolve_expr(src).unwrap();

    let mut checker = TypeChecker {
        table: UnificationTable::new(),
        constraints: Vec::new(),
        substitution: SecondaryMap::new(),
        ctx: SecondaryMap::new(),
        handler: ErrorHandler::TEST,
    };
    checker.infer_expr(&hir, ModuleId::default(), expr);
    checker.unify(&hir);

    Ok(checker.sub_all(&mut hir)?.remove(expr).unwrap())
}

#[test]
fn type_of_if_single_branch() {
    let input = "if true {false ()}";
    assert_eq!(check_expr(input), Ok(Ty::unit()));
}

#[test]
fn type_of_if_single_branch_err() {
    assert_matches!(check_expr("if true { 5.0 }"), Err(_));
}

#[test]
fn type_of_if() {
    assert_eq!(check_expr("if true { 5.0 } else { -3.0 }"), Ok(Ty::Float));
}

#[test]
fn type_of_if_err() {
    assert_matches!(check_expr("if true { 5.0 } else { false }"), Err(_));
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
    assert_matches!(check_expr("[1, 2.0]"), Err(_));
}

#[test]
fn vars() {
    assert_eq!(check_expr("{let mut a: Byte = 1 a = 2}"), Ok(Ty::unit()));
}

#[test]
fn inc() {
    assert_eq!(
        check_expr("fn(mut a) -> a = a +. 1.0"),
        Ok(Ty::Func(
            vec![Param {
                mutable: true,
                ty: Ty::Float,
                span: Range::from(7..8)
            }],
            Ty::unit().into()
        ))
    );
}

#[test]
fn maths() {
    assert_matches!(check_expr("1 + 1.0"), Err(_));
    assert_eq!(check_expr("1.0 +. 1.0"), Ok(Ty::Float));
    assert_eq!(check_expr("{let mut a = 5 let b = 5 a = b b}"), Ok(Ty::Int));
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
