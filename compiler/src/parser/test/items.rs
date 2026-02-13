use crate::helpers::Spannable;
use crate::lexer::TT;
use crate::parser::ast::TypeDef;
use crate::parser::{
    ParseError, ParseResult, Parser,
    ast::{Bop, Expr, Field, Item, Pattern, Type, Variant},
};

fn parse_item(input: &str) -> Item {
    let mut parser = Parser::new(input);
    parser.item().unwrap()
}

fn parse_item_err(input: &str) -> ParseResult<Item> {
    let mut parser = Parser::new(input);
    parser.item()
}

#[test]
fn const_items() {
    assert_eq!(
        parse_item(r#"const HELLO_WORLD: String = "Hello, World!""#),
        Item::Const {
            name: "HELLO_WORLD".into(),
            ty: Some(
                Type::Named {
                    name: "String".into(),
                    args: vec![]
                }
                .span(19..25)
            ),
            value: Expr::String("Hello, World!".into()).span(28..43)
        }
    );

    assert_eq!(
        parse_item(r#"const ID = fn(x) -> x"#),
        Item::Const {
            name: "ID".into(),
            ty: None,
            value: Expr::Lambda {
                params: vec![
                    Pattern::Var {
                        mutable: false,
                        ident: "x".into(),
                        annotated_ty: None
                    }
                    .span(14..15)
                ],
                return_type: None,
                body: Expr::Ident("x".into()).span(20..21).into()
            }
            .span(11..21)
        }
    );
}

#[test]
fn struct_items() {
    let item = parse_item(
        r#"
record Foo<T, U>
    x: Char  ,
    bar: Bar<Baz<T>>
"#,
    );
    assert_eq!(
        item,
        Item::Record {
            def: TypeDef {
                name: "Foo".into(),
                generic_params: vec![
                    String::from("T").span(12..13),
                    String::from("U").span(15..16)
                ]
            },
            fields: vec![
                Field {
                    name: "x".into(),
                    ty: Type::Char.span(25..29)
                }
                .span(22..29),
                Field {
                    name: "bar".into(),
                    ty: Type::Named {
                        name: "Bar".into(),
                        args: vec![
                            Type::Named {
                                name: "Baz".into(),
                                args: vec![
                                    Type::Named {
                                        name: "T".into(),
                                        args: vec![]
                                    }
                                    .span(50..51)
                                ]
                            }
                            .span(46..52)
                        ]
                    }
                    .span(42..53)
                }
                .span(37..53)
            ]
        }
    );
}

#[test]
fn enum_items() {
    let item = parse_item(
        r#"
enum Foo 
| X
| Y(Bar)
| Z 
    baz: Baz, 
    fizz: Buzz
"#,
    );
    assert_eq!(
        item,
        Item::Enum {
            def: TypeDef {
                name: "Foo".into(),
                generic_params: vec![]
            },
            variants: vec![
                Variant::Unit("X".into()),
                Variant::Tuple(
                    "Y".into(),
                    vec![
                        Type::Named {
                            name: "Bar".into(),
                            args: vec![]
                        }
                        .span(19..22)
                    ]
                ),
                Variant::Struct(
                    "Z".into(),
                    vec![
                        Field {
                            name: "baz".into(),
                            ty: Type::Named {
                                name: "Baz".into(),
                                args: vec![]
                            }
                            .span(38..41)
                        }
                        .span(33..41),
                        Field {
                            name: "fizz".into(),
                            ty: Type::Named {
                                name: "Buzz".into(),
                                args: vec![]
                            }
                            .span(54..58)
                        }
                        .span(48..58)
                    ]
                )
            ]
        }
    )
}

#[test]
fn function_items() {
    assert_eq!(
        parse_item(r#"fn sum(mut a, b: Byte) -> a + b"#),
        Item::Func {
            name: "sum".into(),
            params: vec![
                Pattern::Var {
                    mutable: true,
                    ident: "a".into(),
                    annotated_ty: None
                }
                .span(7..12),
                Pattern::Var {
                    mutable: false,
                    ident: "b".into(),
                    annotated_ty: Some(Type::Byte.span(17..21))
                }
                .span(14..21)
            ],
            return_ty: None,
            body: Expr::BinaryOp {
                op: Bop::Add,
                lhs: Expr::Ident("a".into()).span(26..27).into(),
                rhs: Expr::Ident("b".into()).span(30..31).into()
            }
            .span(26..31)
        }
    )
}

#[test]
fn malformed_items() {
    assert_eq!(
        parse_item_err("const fn: Int = 5"),
        Err(ParseError::Mismatched {
            expected: TT::Ident,
            found: TT::Fn,
        }
        .span(6..8))
    );

    assert_eq!(
        parse_item_err("const NO_DICTS: [String: Int] = 5"),
        Err(ParseError::Mismatched {
            expected: TT::RBracket,
            found: TT::Colon
        }
        .span(23..24))
    );

    assert_eq!(
        parse_item_err("let global = 0"),
        Err(ParseError::Unexpected(TT::Let, "start of item").span(0..3))
    );

    assert_eq!(
        parse_item_err("record CSyntax { Int five }"),
        Err(ParseError::Mismatched {
            expected: TT::Ident,
            found: TT::Int,
        }
        .span(17..20))
    );

    assert_eq!(
        parse_item_err("enum NoComma { Bad Syntax }"),
        Err(
            ParseError::Unexpected(TT::Ident, "after variant name. expected one of `,` `(` `{`")
                .span(19..25)
        )
    )
}
