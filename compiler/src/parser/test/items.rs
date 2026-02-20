use crate::ast::{AdtDef, Bop, ExprKind, Field, GenericParam, Item, Pattern, Ty, TyKind, Variant};
use crate::helpers::{Span, Spannable, Spnd};
use crate::lexer::TokKind;
use crate::parser::{ParseError, Parser};

#[test]
fn const_items() {
    let mut interner = Default::default();

    let mut parser = Parser::new(
        r#"const hello_world: String = "Hello, World!""#,
        &mut interner,
    );
    assert_eq!(
        parser.item(),
        Ok(Item::Const {
            ident: parser.get_ident("hello_world").unwrap(),
            ty: Some(Ty {
                kind: TyKind::Adt {
                    ident: parser.get_ident("String").unwrap(),
                    args: vec![]
                },
                span: Span::from(19..25)
            }),
            value: ExprKind::String(String::from("Hello, World!")).span(28..43)
        })
    );

    let mut parser = Parser::new(r#"const id = fn(x) -> x"#, &mut interner);
    assert_eq!(
        parser.item(),
        Ok(Item::Const {
            ident: parser.get_ident("id").unwrap(),
            ty: None,
            value: ExprKind::Lambda {
                params: vec![Pattern::Var {
                    mutable: false,
                    ident: parser.get_ident("x").unwrap(),
                    ty_annotation: None
                }],
                return_type: None,
                body: ExprKind::Ident(parser.get_ident("x").unwrap())
                    .span(20..21)
                    .into()
            }
            .span(11..21)
        })
    );
}

#[test]
fn struct_items() {
    let mut interner = Default::default();

    let mut parser = Parser::new("record Point(x: Int, y: Int)", &mut interner);
    assert_eq!(
        parser.item(),
        Ok(Item::Record {
            def: AdtDef {
                ident: parser.get_ident("Point").unwrap(),
                generics: vec![]
            },
            fields: vec![
                Field {
                    ident: parser.get_ident("x").unwrap(),
                    ty: Ty {
                        kind: TyKind::Int,
                        span: Span::from(16..19)
                    },
                    span: Span::from(13..19)
                },
                Field {
                    ident: parser.get_ident("y").unwrap(),
                    ty: Ty {
                        kind: TyKind::Int,
                        span: Span::from(24..27)
                    },
                    span: Span::from(21..27)
                }
            ]
        })
    );

    let input = "
record Foo<T, U>(
    x: Char , 
    bar: Bar<Baz<T>>
    )";
    let mut parser = Parser::new(input, &mut interner);
    assert_eq!(
        parser.item(),
        Ok(Item::Record {
            def: AdtDef {
                ident: parser.get_ident("Foo").unwrap(),
                generics: vec![
                    GenericParam(Spnd::span(parser.get_ident("T").unwrap(), 12..13)),
                    GenericParam(Spnd::span(parser.get_ident("U").unwrap(), 15..16)),
                ]
            },
            fields: vec![
                Field {
                    ident: parser.get_ident("x").unwrap(),
                    ty: Ty {
                        kind: TyKind::Char,
                        span: Span::from(26..30)
                    },
                    span: Span::from(23..30)
                },
                Field {
                    ident: parser.get_ident("bar").unwrap(),
                    ty: Ty {
                        kind: TyKind::Adt {
                            ident: parser.get_ident("Bar").unwrap(),
                            args: vec![Ty {
                                kind: TyKind::Adt {
                                    ident: parser.get_ident("Baz").unwrap(),
                                    args: vec![Ty {
                                        kind: TyKind::Adt {
                                            ident: parser.get_ident("T").unwrap(),
                                            args: vec![]
                                        },
                                        span: Span::from(51..52)
                                    }]
                                },
                                span: Span::from(47..53)
                            }]
                        },
                        span: Span::from(43..54)
                    },
                    span: Span::from(38..54)
                }
            ]
        })
    );
}

#[test]
fn enum_items() {
    let mut interner = Default::default();

    let input = r#"
enum Foo
    | X()
        | Y(v: Bar)
| Z(baz: Baz, fizz: Buzz)
"#;

    let mut parser = Parser::new(input, &mut interner);
    assert_eq!(
        parser.item(),
        Ok(Item::Enum {
            def: AdtDef {
                ident: parser.get_ident("Foo").unwrap(),
                generics: vec![]
            },
            variants: vec![
                Variant {
                    ident: parser.get_ident("X").unwrap(),
                    fields: vec![]
                },
                Variant {
                    ident: parser.get_ident("Y").unwrap(),
                    fields: vec![Field {
                        ident: parser.get_ident("v").unwrap(),
                        ty: Ty {
                            kind: TyKind::Adt {
                                ident: parser.get_ident("Bar").unwrap(),
                                args: vec![]
                            },
                            span: Span::from(35..38)
                        },
                        span: Span::from(32..38)
                    }],
                },
                Variant {
                    ident: parser.get_ident("Z").unwrap(),
                    fields: vec![
                        Field {
                            ident: parser.get_ident("baz").unwrap(),
                            ty: Ty {
                                kind: TyKind::Adt {
                                    ident: parser.get_ident("Baz").unwrap(),
                                    args: vec![]
                                },
                                span: Span::from(49..52)
                            },
                            span: Span::from(44..52)
                        },
                        Field {
                            ident: parser.get_ident("fizz").unwrap(),
                            ty: Ty {
                                kind: TyKind::Adt {
                                    ident: parser.get_ident("Buzz").unwrap(),
                                    args: vec![]
                                },
                                span: Span::from(60..64)
                            },
                            span: Span::from(54..64)
                        },
                    ]
                },
            ]
        })
    )
}

#[test]
fn function_items() {
    let mut interner = Default::default();

    let mut parser = Parser::new("fn sum(mut a, b: Byte) -> a + b", &mut interner);
    assert_eq!(
        parser.item(),
        Ok(Item::Func {
            ident: parser.get_ident("sum").unwrap(),
            params: vec![
                Pattern::Var {
                    mutable: true,
                    ident: parser.get_ident("a").unwrap(),
                    ty_annotation: None
                },
                Pattern::Var {
                    mutable: false,
                    ident: parser.get_ident("b").unwrap(),
                    ty_annotation: Some(Ty {
                        kind: TyKind::Byte,
                        span: Span::from(17..21)
                    })
                },
            ],
            return_ty: None,
            body: ExprKind::BinaryOp {
                op: Bop::Add,
                lhs: ExprKind::Ident(parser.get_ident("a").unwrap())
                    .span(26..27)
                    .into(),
                rhs: ExprKind::Ident(parser.get_ident("b").unwrap())
                    .span(30..31)
                    .into()
            }
            .span(26..31)
        })
    )
}

#[test]
fn malformed_items() {
    assert_eq!(
        Parser::new("const fn: Int = 5", &mut Default::default()).item(),
        Err(ParseError::Mismatched {
            expected: TokKind::Ident,
            found: TokKind::Fn,
        }
        .span(6..8))
    );

    assert_eq!(
        Parser::new("const NO_DICTS: [String: Int] = 5", &mut Default::default()).item(),
        Err(ParseError::Mismatched {
            expected: TokKind::RBracket,
            found: TokKind::Colon
        }
        .span(23..24))
    );

    assert_eq!(
        Parser::new("let global = false", &mut Default::default()).item(),
        Err(ParseError::Unexpected(TokKind::Let, "start of item").span(0..3))
    );
}
