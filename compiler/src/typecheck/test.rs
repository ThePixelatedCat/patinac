use super::{Type, TypeChecker, TypeError};
use crate::{
    helpers::{Span, Spanned},
    parser::{Parser, ast::ExprS},
    typecheck::error::TypeErrorS,
};

#[test]
fn unify() {
    let checker = TypeChecker::default();

    let t = checker.fresh_var();
    let u = checker.fresh_var();

    let tuple_a = Type::Tuple(vec![t.clone(), Type::UInt]);
    let tuple_b = Type::Tuple(vec![Type::Int, u.clone()]);

    assert_eq!(checker.unify(&tuple_a, &tuple_b), Ok(()));

    let option_t = Type::Named {
        name: String::from("Option"),
        args: vec![t],
    };
    let option_u = Type::Named {
        name: String::from("Option"),
        args: vec![u],
    };

    assert_eq!(
        checker.unify(&option_t, &option_u),
        Err(TypeError::MismatchedTypes {
            expected: Type::Int,
            found: Type::UInt
        })
    );
}

fn parse_expr(input: &str) -> ExprS {
    Parser::new(input).expression().unwrap()
}

fn check_expr(input: &str) -> Result<Type, TypeErrorS> {
    let mut checker = TypeChecker::default();
    let ty = checker.type_of(&parse_expr(input))?;
    Ok(checker.normalise(ty))
}

#[test]
fn type_of_if_single_branch() {
    assert_eq!(check_expr("if (true) {5;}"), Ok(Type::unit()))
}

#[test]
fn type_of_if_single_branch_err() {
    assert_eq!(
        check_expr("if (true) 5.0"),
        Err(TypeError::MismatchedTypes {
            expected: Type::unit(),
            found: Type::Float
        }
        .spanned(10..13))
    )
}

#[test]
fn type_of_if() {
    assert_eq!(check_expr("if (true) 5.0 else -3.0"), Ok(Type::Float))
}

#[test]
fn type_of_if_err() {
    assert_eq!(
        check_expr(r#"if (true) "true" else false"#),
        Err(TypeError::MismatchedTypes {
            expected: Type::string(),
            found: Type::Bool
        }
        .spanned(22..27))
    )
}

#[test]
fn arrays() {
    let mut checker = TypeChecker::default();

    assert_eq!(check_expr("[1, 2, 9 / 3, 4, -5][0]"), Ok(Type::Int));
}

#[test]
fn vars() {
    let mut checker = TypeChecker::default();

    let ty = checker.type_of(&parse_expr("let mut a: Byte = 1")).unwrap();
    assert_eq!(checker.normalise(ty), Type::unit());

    let ty = checker.type_of(&parse_expr("a = 2")).unwrap();
    assert_eq!(checker.normalise(ty), Type::unit());
}

#[test]
fn lambdas() {
    let input = "fn(a, b): UInt -> a + b";

    let mut checker = TypeChecker::default();

    let ty_unbound = checker.type_of(&parse_expr(input)).unwrap();

    let Type::Fn(param_tys, return_ty) = checker.normalise(ty_unbound) else {
        panic!()
    };

    assert_eq!(*return_ty, Type::UInt);
    for ty in param_tys {
        assert_eq!(ty, Type::UInt)
    }
}

#[test]
fn maths() {
    assert!(matches!(
        check_expr("1 + 1.0"),
        Err(Spanned {
            inner: TypeError::MismatchedTypes {
                expected: Type::IntVar(_),
                found: Type::Float
            },
            span: Span { start: 4, end: 7 }
        })
    ));
    assert_eq!(check_expr("1.0 + 1.0"), Ok(Type::Float))
}

#[test]
fn type_of_int() {
    let inputs = ["let a = 5", "a", "let b: Int = 1", "a + b"];

    let mut checker = TypeChecker::default();

    checker.type_of(&parse_expr(inputs[0])).unwrap();
    let ty_unbound = checker.type_of(&parse_expr(inputs[1])).unwrap();

    assert!(matches!(ty_unbound, Type::IntVar(_)));

    checker.type_of(&parse_expr(inputs[2])).unwrap();
    let ty_bound = checker.type_of(&parse_expr(inputs[3])).unwrap();

    assert_eq!(checker.normalise(ty_bound), Type::Int);
}

#[test]
fn type_of_block() {
    let input = "{
        let mut y: UInt = 5;
        3 + 1 - 2;
        y = 256;
        if (y < 3) {
            let a = -5;
            a
        } else 32
    }";
    let expr = parse_expr(input);
    let mut checker = TypeChecker::default();
    let ty = checker.type_of(&expr).unwrap();

    assert_eq!(checker.normalise(ty), Type::Int);
}

#[test]
fn shadowing() {
    let input = r#"{
        let a = 5;
        let a: String = "Hello, World";
        {let a = 2};
        a
    }"#;

    let expr = parse_expr(input);
    let mut checker = TypeChecker::default();
    let ty = checker.type_of(&expr).unwrap();

    assert_eq!(checker.normalise(ty), Type::string());
}
