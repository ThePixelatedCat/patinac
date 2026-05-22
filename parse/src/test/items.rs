use pretty_assertions::assert_eq;
use smallvec::smallvec;

use ast::{
    exprs::{Binding, ExprKind, InfixOp},
    items::{AdtItem, AdtKind, ExecItem, ExecKind, Field, Param, Variant},
    patterns::PatKind,
    types::TyKind,
};
use ident::Ident;
use span::Span;

use crate::{Parser, items::Item};

#[test]
fn const_items() {
    assert_eq!(
        Parser::parse_item(r#"const hello_world: String = "Hello, World!""#),
        Ok(Item::ExecItem(ExecItem {
            ident: Ident::new("hello_world").span(6..17),
            kind: ExecKind::Const {
                ty: Some(TyKind::string().span(19..25)),
                val: ExprKind::string("Hello, World!").span(28..43)
            },
        }))
    );

    assert_eq!(
        Parser::parse_item("const id = fn(x) -> x"),
        Ok(Item::ExecItem(ExecItem {
            ident: Ident::new("id").span(6..8),
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
            kind: AdtKind::Record(vec![
                Field {
                    ident: Ident::new("x").span(13..14),
                    ty: TyKind::Int.span(16..19),
                },
                Field {
                    ident: Ident::new("y").span(21..22),
                    ty: TyKind::Int.span(24..27),
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
            kind: AdtKind::Record(vec![
                Field {
                    ident: Ident::new("x").span(23..24),
                    ty: TyKind::Char.span(26..30),
                },
                Field {
                    ident: Ident::new("bar").span(38..41),
                    ty: TyKind::Adt(
                        Ident::new("Bar"),
                        vec![
                            TyKind::Adt(Ident::new("Baz"), vec![TyKind::named("T").span(51..52)])
                                .span(47..53)
                        ]
                    )
                    .span(43..54),
                }
            ])
        }))
    );
}

#[test]
fn enum_items() {
    let input = "
enum Foo {
    X(),
    Y(v: Bar),
    Z(baz: Baz, fizz: Buzz),
}
";

    assert_eq!(
        Parser::parse_item(input),
        Ok(Item::AdtItem(AdtItem {
            ident: Ident::new("Foo").span(6..9),
            generics: smallvec![],
            kind: AdtKind::Enum(vec![
                Variant {
                    ident: Ident::new("X").span(16..17),
                    fields: vec![]
                },
                Variant {
                    ident: Ident::new("Y").span(25..26),
                    fields: vec![Field {
                        ident: Ident::new("v").span(27..28),
                        ty: TyKind::named("Bar").span(30..33),
                    }],
                },
                Variant {
                    ident: Ident::new("Z").span(40..41),
                    fields: vec![
                        Field {
                            ident: Ident::new("baz").span(42..45),
                            ty: TyKind::named("Baz").span(47..50),
                        },
                        Field {
                            ident: Ident::new("fizz").span(52..56),
                            ty: TyKind::named("Buzz").span(58..62),
                        },
                    ]
                },
            ])
        }))
    );
}

#[test]
fn function_items() {
    assert_eq!(
        Parser::parse_item("fn sum(mut a: Byte, b: Byte) -> a = a + b"),
        Ok(Item::ExecItem(ExecItem {
            ident: Ident::new("sum").span(3..6),
            kind: ExecKind::Fn {
                generics: smallvec![],
                params: vec![
                    Param {
                        pat: PatKind::ident("a").span(11..12),
                        ty: TyKind::Byte.span(14..18),
                        mutable: true,
                        span: Span::from(7..18)
                    },
                    Param {
                        pat: PatKind::ident("b").span(20..21),
                        ty: TyKind::Byte.span(23..27),
                        mutable: false,
                        span: Span::from(20..27)
                    }
                ],
                ret_mut: false,
                ret_ty: TyKind::unit().span(28..29),
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
    );
}

#[test]
fn malformed_items() {
    assert!(Parser::parse_item("const fn: Int = 5").is_err(),);
    assert!(Parser::parse_item("const NO_DICTS: [String: Int] = 5").is_err(),);
    assert!(Parser::parse_item("let global = false").is_err(),);
}
