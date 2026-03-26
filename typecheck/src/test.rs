use super::{Ty, TypeChecker, TypeError};
use crate::{
    helpers::{Span, Spannable, Spnd},
    parser::{Parser, ast::Expr},
    typecheck::error::TypeErrorS,
};

#[test]
fn unify() {
    let checker = TypeChecker::default();

    let t = checker.fresh_var();
    let u = checker.fresh_var();

    let tuple_a = Ty::Tuple(vec![t.clone(), Ty::UInt]);
    let tuple_b = Ty::Tuple(vec![Ty::Int, u.clone()]);

    assert_eq!(checker.unify(&tuple_a, &tuple_b), Ok(()));

    let option_t = Ty::Adt(String::from("Option"), vec![t]);
    let option_u = Ty::Adt(String::from("Option"), vec![u]);

    assert_eq!(
        checker.unify(&option_t, &option_u),
        Err(TypeError::MismatchedTypes {
            expected: Ty::Int,
            found: Ty::UInt
        })
    );
}

fn parse_expr(input: &str) -> Expr {
    Parser::new(input).expr().unwrap()
}

fn check_expr(input: &str) -> Result<Ty, TypeErrorS> {
    let mut checker = TypeChecker::default();
    let ty = checker.infer(&parse_expr(input))?;
    Ok(checker.normalise(ty))
}

fn check_full(input: &str) -> Result<(), TypeErrorS> {
    TypeChecker::default().check(&Parser::new(input).file().unwrap())
}

#[test]
fn type_of_if_single_branch() {
    let input = "if true then 
    5;
";
    assert_eq!(check_expr(input), Ok(Ty::unit()))
}

#[test]
fn type_of_if_single_branch_err() {
    assert_eq!(
        check_expr("if true then 5.0"),
        Err(TypeError::MismatchedTypes {
            expected: Ty::unit(),
            found: Ty::Float
        }
        .span(13..16))
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
        Err(TypeError::MismatchedTypes {
            expected: Ty::string(),
            found: Ty::Bool
        }
        .span(25..30))
    )
}

#[test]
fn arrays() {
    assert_eq!(check_expr("[1, 2, 9 / 3, 4, -5][0]"), Ok(Ty::Int));
}

#[test]
fn vars() {
    let mut checker = TypeChecker::default();

    let ty = checker.infer(&parse_expr("let mut a: Byte = 1")).unwrap();
    assert_eq!(checker.normalise(ty), Ty::unit());

    let ty = checker.infer(&parse_expr("a = 2")).unwrap();
    assert_eq!(checker.normalise(ty), Ty::unit());
}

#[test]
fn lambdas() {
    let input = "fn(a, b): UInt -> a + b";

    let mut checker = TypeChecker::default();

    let ty_unbound = checker.infer(&parse_expr(input)).unwrap();

    let Ty::Func(param_tys, return_ty) = checker.normalise(ty_unbound) else {
        panic!()
    };

    assert_eq!(*return_ty, Ty::UInt);
    for ty in param_tys {
        assert_eq!(ty, Ty::UInt)
    }
}

#[test]
fn maths() {
    assert!(matches!(
        check_expr("1 + 1.0"),
        Err(Spnd {
            inner: TypeError::MismatchedTypes {
                expected: Ty::IntVar(_),
                found: Ty::Float
            },
            span: Span { start: 4, end: 7 }
        })
    ));
    assert_eq!(check_expr("1.0 + 1.0"), Ok(Ty::Float))
}

#[test]
fn type_of_int() {
    let inputs = ["let a = 5", "a", "let b: Int = 1", "a + b"];

    let mut checker = TypeChecker::default();

    checker.infer(&parse_expr(inputs[0])).unwrap();
    let ty_unbound = checker.infer(&parse_expr(inputs[1])).unwrap();

    assert!(matches!(ty_unbound, Ty::IntVar(_)));

    checker.infer(&parse_expr(inputs[2])).unwrap();
    let ty_bound = checker.infer(&parse_expr(inputs[3])).unwrap();

    assert_eq!(checker.normalise(ty_bound), Ty::Int);
}

#[test]
fn type_of_block() {
    let input = "
    let mut y: UInt = 5
    3 + 1 - 2
    y = 256
    if y < 3 then
        let a = -5
        a
    else 32
";
    let expr = parse_expr(input);
    let mut checker = TypeChecker::default();
    let ty = checker.infer(&expr).unwrap();

    assert_eq!(checker.normalise(ty), Ty::Int);
}

#[test]
fn shadowing() {
    let input = r#"
    let a = 5
    let a: String = "Hello, World"
        let a = 2
    a
"#;

    let expr = parse_expr(input);
    let mut checker = TypeChecker::default();
    let ty = checker.infer(&expr).unwrap();

    assert_eq!(checker.normalise(ty), Ty::string());
}

#[test]
fn recursion() {
    let input = "
fn fac(n: UInt) -> 
    if n == 0 then
        1 
    else 
        n * fac(n - 1)
";

    assert_eq!(check_full(input), Ok(()));

    //assert_eq!(check_full("const UHOH = UHOH + 1"), Err(TypeError::Infinite.spanned(13..17)));
}
