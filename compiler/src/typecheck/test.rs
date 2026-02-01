use std::collections::HashMap;

use ena::unify::UnificationTable;

use super::{Type, TypeChecker, TypeError};
use crate::parser::{Parser, ast::ExprS};

#[test]
fn unify() {
    let mut checker = TypeChecker {
        env: HashMap::new(),
        table: UnificationTable::new(),
    };

    let t = checker.fresh_var();
    let u = checker.fresh_var();

    let tuple_a = Type::Tuple(vec![t.clone(), Type::UInt]);
    let tuple_b = Type::Tuple(vec![Type::Int, u.clone()]);

    assert!(checker.unify(&tuple_a, &tuple_b).is_ok());

    let option_t = Type::Named {
        name: String::from("Option"),
        generics: vec![t],
    };
    let option_u = Type::Named {
        name: String::from("Option"),
        generics: vec![u],
    };

    assert_eq!(
        checker.unify(&option_t, &option_u),
        Err(TypeError::MismatchedTypes { expected: Type::Int, found: Type::UInt })
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

    assert!(matches!(ty_unbound, Type::IntVar(_)));

    checker.type_of(&parse_expr(inputs[2])).unwrap();
    let ty_bound = checker.type_of(&parse_expr(inputs[3])).unwrap();

    assert_eq!(checker.normalize(&ty_bound).unwrap(), Type::Int);
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

    assert_eq!(types, Type::unit());
}
