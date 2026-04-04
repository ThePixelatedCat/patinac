mod exprs;
mod items;
mod prop;

use pretty_assertions::assert_eq;

use ast::{
    exprs::{Arg, Binding, ExprKind, InfixOp},
    items::{AdtDef, AdtItem, ExecItem, Field, GenericParam, Param},
    patterns::Pat,
    types::{Param as TyParam, TyKind},
};
use ident::Ident;
use lex::Lexer;
use span::Span;

use crate::Parser;

#[test]
fn file() {
    #[rustfmt::skip]
    let input =
r#"
fn wow_we_did_it(mut x: Bool, bar: Bar[Baz[T], U]): fn(mut Int) -> {} -> {
    let mut x: { Bool, T} = true + sin(y)
    x = if bar < 3 then {
        let baz = bar.value + 2 * 4
        x + 1
    } else if bar <= 2 then
        fizz(3, 5.1)
}
record Foo[T, U](x: String, bar: Bar[Baz[T], [U]])
"#;

    let items = Parser::parse(input, Lexer::lex(input).unwrap()).unwrap();

    assert_eq!(
        items.execs[0],
        ExecItem::Func {
            ident: Ident::new("wow_we_did_it"),
            generic_params: vec![],
            params: vec![
                Param {
                    mutable: true,
                    pat: Pat::Ident {
                        ident: Ident::new("x"),
                        subpat: None
                    },
                    ty: TyKind::Bool.span(25..29)
                },
                Param {
                    mutable: false,
                    pat: Pat::Ident {
                        ident: Ident::new("bar"),
                        subpat: None
                    },
                    ty: TyKind::Adt(
                        Ident::new("Bar"),
                        vec![
                            TyKind::Adt(
                                Ident::new("Baz"),
                                vec![TyKind::Adt(Ident::new("T"), vec![]).span(44..45)],
                            )
                            .span(40..46),
                            TyKind::Adt(Ident::new("U"), vec![]).span(48..49)
                        ]
                    )
                    .span(36..50)
                }
            ],
            return_ty: TyKind::Fn(
                vec![TyParam {
                    mutable: true,
                    ty: TyKind::Int.span(60..63)
                }],
                Box::new(TyKind::Tuple(vec![]).span(68..70))
            )
            .span(53..70),
            body: ExprKind::Block(vec![
                ExprKind::Let {
                    binding: Binding {
                        mutable: true,
                        pat: Pat::Ident {
                            ident: Ident::new("x"),
                            subpat: None
                        },
                        ty: Some(
                            TyKind::Tuple(vec![
                                TyKind::Bool.span(93..97),
                                TyKind::Adt(Ident::new("T"), vec![]).span(99..100)
                            ])
                            .span(91..101)
                        )
                    },
                    val: Box::new(
                        ExprKind::InfixExpr {
                            op: InfixOp::Add,
                            lhs: Box::new(ExprKind::bool(true).span(104..108)),
                            rhs: Box::new(
                                ExprKind::CallExpr {
                                    func: Box::new(ExprKind::ident("sin").span(111..114)),
                                    args: vec![Arg {
                                        mutable: false,
                                        label: None,
                                        val: ExprKind::ident("y").span(115..116)
                                    }]
                                }
                                .span(111..117)
                            )
                        }
                        .span(104..117)
                    )
                }
                .span(80..117),
                ExprKind::InfixExpr {
                    op: InfixOp::Assign,
                    lhs: Box::new(ExprKind::ident("x").span(122..123)),
                    rhs: ExprKind::If {
                        cond: Box::new(
                            ExprKind::InfixExpr {
                                op: InfixOp::Lt,
                                lhs: Box::new(ExprKind::ident("bar").span(129..132)),
                                rhs: Box::new(ExprKind::int(3).span(135..136))
                            }
                            .span(129..136)
                        ),
                        th: Box::new(
                            ExprKind::Block(vec![
                                ExprKind::Let {
                                    binding: Binding {
                                        mutable: false,
                                        pat: Pat::Ident {
                                            ident: Ident::new("baz"),
                                            subpat: None
                                        },
                                        ty: None
                                    },
                                    val: Box::new(
                                        ExprKind::InfixExpr {
                                            op: InfixOp::Add,
                                            lhs: Box::new(
                                                ExprKind::FieldExpr {
                                                    base: Box::new(
                                                        ExprKind::Ident(Ident::new("bar"))
                                                            .span(162..165)
                                                    ),
                                                    field: Ident::new("value")
                                                }
                                                .span(162..171)
                                            ),
                                            rhs: Box::new(
                                                ExprKind::InfixExpr {
                                                    op: InfixOp::Mul,
                                                    lhs: Box::new(ExprKind::int(2).span(174..175)),
                                                    rhs: Box::new(ExprKind::int(4).span(178..179))
                                                }
                                                .span(174..179)
                                            )
                                        }
                                        .span(162..179)
                                    )
                                }
                                .span(152..179),
                                ExprKind::InfixExpr {
                                    op: InfixOp::Add,
                                    lhs: Box::new(ExprKind::ident("x").span(188..189)),
                                    rhs: Box::new(ExprKind::int(1).span(192..193))
                                }
                                .span(188..193)
                            ])
                            .span(142..199)
                        ),
                        el: Some(Box::new(
                            ExprKind::If {
                                cond: Box::new(
                                    ExprKind::InfixExpr {
                                        op: InfixOp::Leq,
                                        lhs: Box::new(ExprKind::ident("bar").span(208..211)),
                                        rhs: Box::new(ExprKind::int(2).span(215..216))
                                    }
                                    .span(208..216)
                                ),
                                th: Box::new(
                                    ExprKind::CallExpr {
                                        func: Box::new(ExprKind::ident("fizz").span(230..234)),
                                        args: vec![
                                            Arg {
                                                mutable: false,
                                                label: None,
                                                val: ExprKind::int(3).span(235..236)
                                            },
                                            Arg {
                                                mutable: false,
                                                label: None,
                                                val: ExprKind::float(5.1).span(238..241)
                                            }
                                        ]
                                    }
                                    .span(230..242)
                                ),
                                el: None
                            }
                            .span(205..242)
                        ))
                    }
                    .span(126..242)
                    .into()
                }
                .span(122..242),
            ])
            .span(74..244)
        }
    );

    assert_eq!(
        items.adts[0],
        AdtItem::Record {
            def: AdtDef {
                ident: Ident::new("Foo"),
                generics: vec![GenericParam(Ident::new("T")), GenericParam(Ident::new("U")),]
            },
            fields: vec![
                Field {
                    ident: Ident::new("x"),
                    ty: TyKind::Adt(Ident::new("String"), vec![]).span(265..271),
                    span: Span::from(262..271)
                },
                Field {
                    ident: Ident::new("bar"),
                    ty: TyKind::Adt(
                        Ident::new("Bar"),
                        vec![
                            TyKind::Adt(
                                Ident::new("Baz"),
                                vec![TyKind::Adt(Ident::new("T"), vec![]).span(286..287)]
                            )
                            .span(282..288),
                            TyKind::Array(Box::new(
                                TyKind::Adt(Ident::new("U"), vec![]).span(291..292)
                            ))
                            .span(290..293),
                        ]
                    )
                    .span(278..294),
                    span: Span::from(273..294)
                }
            ]
        }
    );
}
