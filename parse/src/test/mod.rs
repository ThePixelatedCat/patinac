#[cfg(test)]
mod exprs;
#[cfg(test)]
mod items;

use ast::{
    AdtDef, AdtItem, Arg, Binding, ExecItem, ExprKind, Field, GenericParam, InfixOp, Param, Pat,
    PlaceExpr, TyKind,
};
use span::Span;

use crate::Parser;

#[test]
fn file() {
    let mut interner = Default::default();

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

    let mut parser = Parser::new(input, &mut interner);

    let items = parser.parse().unwrap();

    assert_eq!(
        items.execs[0],
        ExecItem::Func {
            ident: parser.get_interned("wow_we_did_it"),
            generic_params: vec![],
            params: vec![
                Param {
                    mutable: true,
                    pat: Pat::Ident {
                        ident: parser.get_interned("x"),
                        subpat: None
                    },
                    ty: TyKind::Bool.span(25..29)
                },
                Param {
                    mutable: false,
                    pat: Pat::Ident {
                        ident: parser.get_interned("bar"),
                        subpat: None
                    },
                    ty: TyKind::Adt(
                        parser.get_interned("Bar"),
                        vec![
                            TyKind::Adt(
                                parser.get_interned("Baz"),
                                vec![TyKind::Adt(parser.get_interned("T"), vec![]).span(44..45)],
                            )
                            .span(40..46),
                            TyKind::Adt(parser.get_interned("U"), vec![]).span(48..49)
                        ]
                    )
                    .span(36..50)
                }
            ],
            return_ty: TyKind::Fn(
                vec![(true, TyKind::Int.span(60..63))],
                Box::new(TyKind::Tuple(vec![]).span(68..70))
            )
            .span(53..70),
            body: ExprKind::Block(vec![
                ExprKind::Let {
                    binding: Binding {
                        mutable: true,
                        pat: Pat::Ident {
                            ident: parser.get_interned("x"),
                            subpat: None
                        },
                        ty: Some(
                            TyKind::Tuple(vec![
                                TyKind::Bool.span(93..97),
                                TyKind::Adt(parser.get_interned("T"), vec![]).span(99..100)
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
                                    func: Box::new(
                                        ExprKind::ident(parser.get_interned("sin")).span(111..114)
                                    ),
                                    args: vec![Arg {
                                        mutable: false,
                                        label: None,
                                        val: ExprKind::ident(parser.get_interned("y"))
                                            .span(115..116)
                                    }]
                                }
                                .span(111..117)
                            )
                        }
                        .span(104..117)
                    )
                }
                .span(80..117),
                ExprKind::Assign {
                    place: Box::new(ExprKind::ident(parser.get_interned("x")).span(122..123)),
                    val: ExprKind::If {
                        cond: Box::new(
                            ExprKind::InfixExpr {
                                op: InfixOp::Lt,
                                lhs: Box::new(
                                    ExprKind::ident(parser.get_interned("bar")).span(129..132)
                                ),
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
                                            ident: parser.get_interned("baz"),
                                            subpat: None
                                        },
                                        ty: None
                                    },
                                    val: Box::new(
                                        ExprKind::InfixExpr {
                                            op: InfixOp::Add,
                                            lhs: Box::new(
                                                ExprKind::Place(PlaceExpr::Field(
                                                    Box::new(PlaceExpr::Ident(
                                                        parser.get_interned("bar")
                                                    )),
                                                    parser.get_interned("value")
                                                ))
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
                                    lhs: Box::new(
                                        ExprKind::ident(parser.get_interned("x")).span(188..189)
                                    ),
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
                                        lhs: Box::new(
                                            ExprKind::ident(parser.get_interned("bar"))
                                                .span(208..211)
                                        ),
                                        rhs: Box::new(ExprKind::int(2).span(215..216))
                                    }
                                    .span(208..216)
                                ),
                                th: Box::new(
                                    ExprKind::CallExpr {
                                        func: Box::new(
                                            ExprKind::ident(parser.get_interned("fizz"))
                                                .span(230..234)
                                        ),
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
                ident: parser.get_interned("Foo"),
                generics: vec![
                    GenericParam(parser.get_interned("T")),
                    GenericParam(parser.get_interned("U")),
                ]
            },
            fields: vec![
                Field {
                    ident: parser.get_interned("x"),
                    ty: TyKind::Adt(parser.get_interned("String"), vec![]).span(265..271),
                    span: Span::from(262..271)
                },
                Field {
                    ident: parser.get_interned("bar"),
                    ty: TyKind::Adt(
                        parser.get_interned("Bar"),
                        vec![
                            TyKind::Adt(
                                parser.get_interned("Baz"),
                                vec![TyKind::Adt(parser.get_interned("T"), vec![]).span(286..287)]
                            )
                            .span(282..288),
                            TyKind::Array(Box::new(
                                TyKind::Adt(parser.get_interned("U"), vec![]).span(291..292)
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
