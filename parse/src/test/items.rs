use pretty_assertions::assert_eq;

use ast::{
    exprs::{Binding, ExprKind, InfixOp},
    items::{AdtDef, AdtItem, ExecItem, Field, GenericParam, Param, Variant},
    patterns::PatKind,
    types::{Ty, TyKind},
};
use ident::Ident;
use lex::TokKind;
use span::Span;

use crate::{ErrorKind, Parser, items::Item};

#[test]
fn const_items() {
    assert_eq!(
        Parser::parse_item(r#"const hello_world: String = "Hello, World!""#),
        Ok(Item::ExecItem(ExecItem::Const {
            ident: Ident::new("hello_world"),
            ty: Some(Ty {
                kind: TyKind::Adt(Ident::new("String"), vec![]),
                span: Span::from(19..25)
            }),
            val: ExprKind::string("Hello, World!").span(28..43)
        }))
    );

    assert_eq!(
        Parser::parse_item(r#"const id = fn(x) -> x"#),
        Ok(Item::ExecItem(ExecItem::Const {
            ident: Ident::new("id"),
            ty: None,
            val: ExprKind::LambdaExpr {
                params: vec![Binding {
                    mutable: false,
                    pat: PatKind::Ident {
                        ident: Ident::new("x"),
                        subpat: None
                    }
                    .span(14..15),
                    ty: None
                }],
                return_ty: None,
                body: ExprKind::ident("x").span(20..21).into()
            }
            .span(11..21)
        }))
    );
}

#[test]
fn struct_items() {
    assert_eq!(
        Parser::parse_item("record Point(x: Int, y: Int)"),
        Ok(Item::AdtItem(AdtItem::Record {
            def: AdtDef {
                ident: Ident::new("Point"),
                generics: vec![]
            },
            fields: vec![
                Field {
                    ident: Ident::new("x"),
                    ty: Ty {
                        kind: TyKind::Int,
                        span: Span::from(16..19)
                    },
                    span: Span::from(13..19)
                },
                Field {
                    ident: Ident::new("y"),
                    ty: Ty {
                        kind: TyKind::Int,
                        span: Span::from(24..27)
                    },
                    span: Span::from(21..27)
                }
            ]
        }))
    );

    let input = "
record Foo[T, U](
    x: Char , 
    bar: Bar[Baz[T]]
    )";
    assert_eq!(
        Parser::parse_item(input),
        Ok(Item::AdtItem(AdtItem::Record {
            def: AdtDef {
                ident: Ident::new("Foo"),
                generics: vec![GenericParam(Ident::new("T")), GenericParam(Ident::new("U")),]
            },
            fields: vec![
                Field {
                    ident: Ident::new("x"),
                    ty: Ty {
                        kind: TyKind::Char,
                        span: Span::from(26..30)
                    },
                    span: Span::from(23..30)
                },
                Field {
                    ident: Ident::new("bar"),
                    ty: Ty {
                        kind: TyKind::Adt(
                            Ident::new("Bar"),
                            vec![Ty {
                                kind: TyKind::Adt(
                                    Ident::new("Baz"),
                                    vec![Ty {
                                        kind: TyKind::Adt(Ident::new("T"), vec![]),
                                        span: Span::from(51..52)
                                    }]
                                ),
                                span: Span::from(47..53)
                            }]
                        ),
                        span: Span::from(43..54)
                    },
                    span: Span::from(38..54)
                }
            ]
        }))
    );
}

#[test]
fn enum_items() {
    let input = r#"
enum Foo
    | X()
        | Y(v: Bar)
| Z(baz: Baz, fizz: Buzz)
"#;

    assert_eq!(
        Parser::parse_item(input),
        Ok(Item::AdtItem(AdtItem::Enum {
            def: AdtDef {
                ident: Ident::new("Foo"),
                generics: vec![]
            },
            variants: vec![
                Variant {
                    ident: Ident::new("X"),
                    fields: vec![]
                },
                Variant {
                    ident: Ident::new("Y"),
                    fields: vec![Field {
                        ident: Ident::new("v"),
                        ty: Ty {
                            kind: TyKind::Adt(Ident::new("Bar"), vec![]),
                            span: Span::from(35..38)
                        },
                        span: Span::from(32..38)
                    }],
                },
                Variant {
                    ident: Ident::new("Z"),
                    fields: vec![
                        Field {
                            ident: Ident::new("baz"),
                            ty: Ty {
                                kind: TyKind::Adt(Ident::new("Baz"), vec![]),
                                span: Span::from(49..52)
                            },
                            span: Span::from(44..52)
                        },
                        Field {
                            ident: Ident::new("fizz"),
                            ty: Ty {
                                kind: TyKind::Adt(Ident::new("Buzz"), vec![]),
                                span: Span::from(60..64)
                            },
                            span: Span::from(54..64)
                        },
                    ]
                },
            ]
        }))
    )
}

#[test]
fn function_items() {
    assert_eq!(
        Parser::parse_item("fn sum(mut a: Byte, b: Byte): {} -> a = a + b"),
        Ok(Item::ExecItem(ExecItem::Func {
            ident: Ident::new("sum"),
            generic_params: vec![],
            params: vec![
                Param {
                    mutable: true,
                    pat: PatKind::Ident {
                        ident: Ident::new("a"),
                        subpat: None
                    }
                    .span(11..12),
                    ty: Ty {
                        kind: TyKind::Byte,
                        span: Span::from(14..18)
                    }
                },
                Param {
                    mutable: false,
                    pat: PatKind::Ident {
                        ident: Ident::new("b"),
                        subpat: None
                    }
                    .span(20..21),
                    ty: Ty {
                        kind: TyKind::Byte,
                        span: Span::from(23..27)
                    }
                }
            ],
            return_ty: Ty {
                kind: TyKind::Tuple(vec![]),
                span: Span::from(30..32)
            },
            body: ExprKind::InfixExpr {
                op: InfixOp::Assign,
                lhs: Box::new(ExprKind::ident("a").span(36..37)),
                rhs: Box::new(
                    ExprKind::InfixExpr {
                        op: InfixOp::Add,
                        lhs: Box::new(ExprKind::ident("a").span(40..41)),
                        rhs: Box::new(ExprKind::ident("b").span(44..45))
                    }
                    .span(40..45)
                )
            }
            .span(36..45)
        }))
    )
}

#[test]
fn malformed_items() {
    assert_eq!(
        Parser::parse_item("const fn: Int = 5"),
        Err(ErrorKind::Mismatched {
            expected: TokKind::Ident,
            found: TokKind::Fn,
        }
        .span(6..8))
    );

    assert_eq!(
        Parser::parse_item("const NO_DICTS: [String: Int] = 5"),
        Err(ErrorKind::Mismatched {
            expected: TokKind::RBracket,
            found: TokKind::Colon
        }
        .span(23..24))
    );

    assert_eq!(
        Parser::parse_item("let global = false"),
        Err(ErrorKind::Unexpected(TokKind::Let, "start of item").span(0..3))
    );
}
