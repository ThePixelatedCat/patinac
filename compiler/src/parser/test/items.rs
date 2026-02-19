use crate::ast::{
    AdtDef, Bop, ExprKind, Field, GenericParam, Generics, Item, Pattern, Ty, TyKind, Variant,
    VariantData,
};
use crate::helpers::{Span, Spannable};
use crate::lexer::TokKind;
use crate::parser::{ParseError, ParseResult, Parser};

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
            ident: "HELLO_WORLD".into(),
            ty: Some(Ty {
                kind: TyKind::Adt {
                    ident: "String".into(),
                    args: vec![]
                },
                span: Span::from(19..25)
            }),
            value: ExprKind::String("Hello, World!".into()).span(28..43)
        }
    );

    assert_eq!(
        parse_item(r#"const ID = fn(x) -> x"#),
        Item::Const {
            ident: "ID".into(),
            ty: None,
            value: ExprKind::Lambda {
                params: vec![Pattern::Var {
                    mutable: false,
                    ident: "x".into(),
                    ty_annotation: None
                }],
                return_type: None,
                body: ExprKind::Ident("x".into()).span(20..21).into()
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
            def: AdtDef {
                ident: "Foo".into(),
                generics: Some(Generics {
                    params: vec![
                        GenericParam {
                            ident: String::from("T"),
                            span: Span::from(12..13)
                        },
                        GenericParam {
                            ident: String::from("U"),
                            span: Span::from(15..16)
                        }
                    ],
                    span: Span::from(11..17)
                })
            },
            data: VariantData::Record(vec![
                Field {
                    ident: "x".into(),
                    ty: Ty {
                        kind: TyKind::Char,
                        span: Span::from(25..29)
                    },
                    span: Span::from(22..29)
                },
                Field {
                    ident: "bar".into(),
                    ty: Ty {
                        kind: TyKind::Adt {
                            ident: "Bar".into(),
                            args: vec![Ty {
                                kind: TyKind::Adt {
                                    ident: "Baz".into(),
                                    args: vec![Ty {
                                        kind: TyKind::Adt {
                                            ident: "T".into(),
                                            args: vec![]
                                        },
                                        span: Span::from(50..51)
                                    }]
                                },
                                span: Span::from(46..52)
                            }]
                        },
                        span: Span::from(42..53)
                    },
                    span: Span::from(37..53)
                }
            ])
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
            def: AdtDef {
                ident: "Foo".into(),
                generics: None
            },
            variants: vec![
                Variant {
                    ident: "X".into(),
                    data: VariantData::Unit
                },
                Variant {
                    ident: "Y".into(),
                    data: VariantData::Tuple(vec![Ty {
                        kind: TyKind::Adt {
                            ident: "Bar".into(),
                            args: vec![]
                        },
                        span: Span::from(19..22)
                    }]),
                },
                Variant {
                    ident: "Z".into(),
                    data: VariantData::Record(vec![
                        Field {
                            ident: "baz".into(),
                            ty: Ty {
                                kind: TyKind::Adt {
                                    ident: "Baz".into(),
                                    args: vec![]
                                },
                                span: Span::from(38..41)
                            },
                            span: Span::from(33..41)
                        },
                        Field {
                            ident: "fizz".into(),
                            ty: Ty {
                                kind: TyKind::Adt {
                                    ident: "Buzz".into(),
                                    args: vec![]
                                },
                                span: Span::from(54..58)
                            },
                            span: Span::from(48..58)
                        },
                    ])
                },
            ]
        }
    )
}

#[test]
fn function_items() {
    assert_eq!(
        parse_item(r#"fn sum(mut a, b: Byte) -> a + b"#),
        Item::Func {
            ident: "sum".into(),
            params: vec![
                Pattern::Var {
                    mutable: true,
                    ident: "a".into(),
                    ty_annotation: None
                },
                Pattern::Var {
                    mutable: false,
                    ident: "b".into(),
                    ty_annotation: Some(Ty {
                        kind: TyKind::Byte,
                        span: Span::from(17..21)
                    })
                },
            ],
            return_ty: None,
            body: ExprKind::BinaryOp {
                op: Bop::Add,
                lhs: ExprKind::Ident("a".into()).span(26..27).into(),
                rhs: ExprKind::Ident("b".into()).span(30..31).into()
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
            expected: TokKind::Ident,
            found: TokKind::Fn,
        }
        .span(6..8))
    );

    assert_eq!(
        parse_item_err("const NO_DICTS: [String: Int] = 5"),
        Err(ParseError::Mismatched {
            expected: TokKind::RBracket,
            found: TokKind::Colon
        }
        .span(23..24))
    );

    assert_eq!(
        parse_item_err("let global = 0"),
        Err(ParseError::Unexpected(TokKind::Let, "start of item").span(0..3))
    );
}
