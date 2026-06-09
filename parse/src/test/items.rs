use pretty_assertions::assert_eq;
use smallvec::smallvec;

use ast::{
    ExecItem, ExecKind, ExprKind, Field, InfixOp, Param, PatKind, Path, TyItem, TyItemKind, TyKind,
    Variant,
};
use errors::ErrorHandler;
use ident::Ident;
use std::range::Range;

use crate::{Parser, items::Item};

#[test]
fn const_items() {
    assert_eq!(
        Parser::new(
            r#"const hello_world: String = "Hello, World!""#,
            ErrorHandler::TEST
        )
        .item(),
        Ok(Item::ExecItem(ExecItem {
            ident: Ident::new("hello_world").span(6..17),
            kind: ExecKind::Const {
                ty: TyKind::string().span(19..25),
                val: ExprKind::string("Hello, World!").span(28..43)
            },
        }))
    );
}

#[test]
fn record_items() {
    assert_eq!(
        Parser::new("record Point(x: Int, y: Int)", ErrorHandler::TEST).item(),
        Ok(Item::TyItem(TyItem {
            ident: Ident::new("Point").span(7..12),
            generics: smallvec![],
            kind: TyItemKind::Record(vec![
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
        Parser::new(input, ErrorHandler::TEST).item(),
        Ok(Item::TyItem(TyItem {
            ident: Ident::new("Foo").span(8..11),
            generics: smallvec![Ident::new("T").span(12..13), Ident::new("U").span(15..16),],
            kind: TyItemKind::Record(vec![
                Field {
                    ident: Ident::new("x").span(23..24),
                    ty: TyKind::Char.span(26..30),
                },
                Field {
                    ident: Ident::new("bar").span(38..41),
                    ty: TyKind::Named(
                        Path::single(Ident::new("Bar")),
                        vec![
                            TyKind::Named(
                                Path::single(Ident::new("Baz")),
                                vec![TyKind::named("T").span(51..52)]
                            )
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
        Parser::new(input, ErrorHandler::TEST).item(),
        Ok(Item::TyItem(TyItem {
            ident: Ident::new("Foo").span(6..9),
            generics: smallvec![],
            kind: TyItemKind::Enum(vec![
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
        Parser::new(
            "fn sum(mut a: Byte, b: Byte): () = a = a + b",
            ErrorHandler::TEST
        )
        .item(),
        Ok(Item::ExecItem(ExecItem {
            ident: Ident::new("sum").span(3..6),
            kind: ExecKind::Fn {
                generics: smallvec![],
                params: vec![
                    Param {
                        pat: PatKind::ident("a").span(11..12),
                        ty: TyKind::Byte.span(14..18),
                        mutable: true,
                        span: Range::from(7..18)
                    },
                    Param {
                        pat: PatKind::ident("b").span(20..21),
                        ty: TyKind::Byte.span(23..27),
                        mutable: false,
                        span: Range::from(20..27)
                    }
                ],
                ret_mut: false,
                ret_ty: TyKind::unit().span(30..32),
                body: ExprKind::Infix {
                    op: InfixOp::Assign,
                    lhs: ExprKind::ident("a").span(35..36).into(),
                    rhs: ExprKind::Infix {
                        op: InfixOp::Add,
                        lhs: ExprKind::ident("a").span(39..40).into(),
                        rhs: ExprKind::ident("b").span(43..44).into()
                    }
                    .span(39..44)
                    .into()
                }
                .span(35..44)
            }
        }))
    );
}

#[test]
fn malformed_items() {
    assert!(
        Parser::new("const fn: Int = 5", ErrorHandler::TEST)
            .item()
            .is_err(),
    );
    assert!(
        Parser::new("const NO_DICTS: [String: Int] = 5", ErrorHandler::TEST)
            .item()
            .is_err(),
    );
    assert!(
        Parser::new("let global = false", ErrorHandler::TEST)
            .item()
            .is_err(),
    );
}
