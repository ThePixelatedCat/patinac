use pretty_assertions::assert_eq;
use smallvec::smallvec;

use ast::{
    exprs::{Binding, ExprKind, InfixOp},
    items::{AdtItem, AdtKind, ExecItem, ExecKind, Field, Param, Variant},
    patterns::PatKind,
};
use ident::Ident;
use lex::TokKind;
use span::Span;
use types::Ty;

use crate::{ErrorKind, Parser, items::Item};

#[test]
fn const_items() {
    assert_eq!(
        Parser::parse_item(r#"const hello_world: String = "Hello, World!""#),
        Ok(Item::ExecItem(ExecItem {
            ident: Ident::new("hello_world"),
            ident_span: Span::from(6..17),
            kind: ExecKind::Const {
                ty: Some(Ty::string_span(19..25)),
                val: ExprKind::string("Hello, World!").span(28..43)
            },
        }))
    );

    assert_eq!(
        Parser::parse_item("const id = fn(x) -> x"),
        Ok(Item::ExecItem(ExecItem {
            ident: Ident::new("id"),
            ident_span: Span::from(6..8),
            kind: ExecKind::Const {
                ty: None,
                val: ExprKind::Lambda {
                    params: vec![Binding {
                        mutable: false,
                        pat: PatKind::ident("x").span(14..15),
                        ty: None
                    }],
                    body: ExprKind::ident("x").span(20..21).into()
                }
                .span(11..21)
            },
        }))
    );
}

#[test]
fn record_items() {
    assert_eq!(
        Parser::parse_item("record Point(x: Int, y: Int)"),
        Ok(Item::AdtItem(AdtItem {
            ident: Ident::new("Point").span(7..12),
            generics: smallvec![],
            span: Span::from(0..28),
            kind: AdtKind::Record(vec![
                Field {
                    ident: Ident::new("x"),
                    ty: Ty::Int,
                    span: Span::from(13..19)
                },
                Field {
                    ident: Ident::new("y"),
                    ty: Ty::Int,
                    span: Span::from(21..27)
                }
            ])
        }))
    );

    let input = "
record Foo[T, U](
    x: Char , 
    bar: Bar[Baz[T]]
    )";
    assert_eq!(
        Parser::parse_item(input),
        Ok(Item::AdtItem(AdtItem {
            ident: Ident::new("Foo").span(8..11),
            generics: smallvec![Ident::new("T").span(12..13), Ident::new("U").span(15..16),],
            span: Span::from(1..60),
            kind: AdtKind::Record(vec![
                Field {
                    ident: Ident::new("x"),
                    ty: Ty::Char,
                    span: Span::from(23..30)
                },
                Field {
                    ident: Ident::new("bar"),
                    ty: Ty::Adt(
                        Ident::new("Bar").span(43..46),
                        vec![Ty::Adt(
                            Ident::new("Baz").span(47..50),
                            vec![Ty::named_span("T", 51..52)]
                        )]
                    ),
                    span: Span::from(38..54)
                }
            ])
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
        Ok(Item::AdtItem(AdtItem {
            ident: Ident::new("Foo").span(6..9),
            generics: smallvec![],
            span: Span::from(1..65),
            kind: AdtKind::Enum(vec![
                Variant {
                    ident: Ident::new("X").span(16..17),
                    fields: vec![]
                },
                Variant {
                    ident: Ident::new("Y").span(30..31),
                    fields: vec![Field {
                        ident: Ident::new("v"),
                        ty: Ty::named_span("Bar", 35..38),
                        span: Span::from(32..38)
                    }],
                },
                Variant {
                    ident: Ident::new("Z").span(42..43),
                    fields: vec![
                        Field {
                            ident: Ident::new("baz"),
                            ty: Ty::named_span("Baz", 49..52),
                            span: Span::from(44..52)
                        },
                        Field {
                            ident: Ident::new("fizz"),
                            ty: Ty::named_span("Buzz", 60..64),
                            span: Span::from(54..64)
                        },
                    ]
                },
            ])
        }))
    )
}

#[test]
fn function_items() {
    assert_eq!(
        Parser::parse_item("fn sum(mut a: Byte, b: Byte) -> a = a + b"),
        Ok(Item::ExecItem(ExecItem {
            ident: Ident::new("sum"),
            ident_span: Span::from(3..6),
            kind: ExecKind::Fn {
                generics: smallvec![],
                params: vec![
                    Param {
                        mutable: true,
                        pat: PatKind::ident("a").span(11..12),
                        ty: Ty::Byte
                    },
                    Param {
                        mutable: false,
                        pat: PatKind::ident("b").span(20..21),
                        ty: Ty::Byte
                    }
                ],
                ret_mut: false,
                ret_ty: Ty::unit(),
                body: ExprKind::Infix {
                    op: InfixOp::Assign,
                    lhs: ExprKind::ident("a").span(32..33).into(),
                    rhs: ExprKind::Infix {
                        op: InfixOp::Add,
                        lhs: ExprKind::ident("a").span(36..37).into(),
                        rhs: ExprKind::ident("b").span(40..41).into()
                    }
                    .span(36..41)
                    .into()
                }
                .span(32..41)
            }
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
        Err(ErrorKind::Unexpected(TokKind::LBracket).span(16..17))
    );

    assert_eq!(
        Parser::parse_item("let global = false"),
        Err(ErrorKind::Unexpected(TokKind::Let)
            .span(0..3)
            .context("at start of item"))
    );
}
