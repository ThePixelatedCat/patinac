use std::collections::HashMap;

use ena::unify::UnificationTable;

use super::{Type, TypeChecker, TypeError};
use crate::{
    helpers::Spanned,
    parser::{Parser, ast::ExprS},
};

#[test]
fn unify() {
    let mut checker = TypeChecker {
        env: HashMap::new(),
        table: UnificationTable::new(),
    };

    let uint = Type::UInt.spanned(0..1);
    let int = Type::Int.spanned(0..1);

    let t = checker.fresh_var().spanned(0..1);
    let u = checker.fresh_var().spanned(0..1);

    let tuple_a = Type::Tuple(vec![t.clone(), uint.clone()]).spanned(0..1);
    let tuple_b = Type::Tuple(vec![int.clone(), u.clone()]).spanned(0..1);

    assert!(checker.unify(&tuple_a, &tuple_b).is_ok());

    let option_t = Type::Named {
        name: String::from("Option"),
        generics: vec![t],
    }
    .spanned(0..1);
    let option_u = Type::Named {
        name: String::from("Option"),
        generics: vec![u],
    }
    .spanned(0..1);

    assert_eq!(
        checker.unify(&option_t, &option_u),
        Err(TypeError::MismatchedTypes(int, uint).spanned(0..1))
    );
}

fn parse_expr(input: &str) -> ExprS {
    Parser::new(input).expression().unwrap()
}

#[test]
fn typecheck_int() {
    let inputs = ["let a = 5", "a", "let b: Int = 1", "a + b"];

    let mut checker = TypeChecker::default();

    checker.type_of(&parse_expr(inputs[0])).unwrap();
    let ty_unbound = checker.type_of(&parse_expr(inputs[1])).unwrap();

    assert!(matches!(
        ty_unbound,
        Spanned {
            inner: Type::IntVar(_),
            ..
        }
    ));

    checker.type_of(&parse_expr(inputs[2])).unwrap();
    let ty_bound = checker.type_of(&parse_expr(inputs[3])).unwrap();

    assert_eq!(checker.normalize(&ty_bound).unwrap().inner, Type::Int);
}

#[test]
fn typecheck_block() {
    let input = "
    {
        let mut y: Int = 5;
        3 + 1 - 2;
        y = 256;
        if (y < 3) {
            let a = -5;
            a
        } else 32;
    }";
    let expr = parse_expr(input);
    let types = TypeChecker::default().type_of(&expr).unwrap();

    assert_eq!(types.inner, Type::unit());
}
