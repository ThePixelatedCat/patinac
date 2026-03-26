#[cfg(test)]
mod exprs;
#[cfg(test)]
mod items;

use ast::{AdtDef, Binding, Bop, ExprKind, Field, GenericParam, Item, Pat, Ty, TyKind};
use span::{Span, Spnd};

use crate::Parser;

#[test]
fn file() {
    let mut interner = Default::default();

    #[rustfmt::skip]
    let input = 
r#"
fn wow_we_did_it(mut x, bar: Bar<Baz<T>, U>): fn(Int): Int -> 
    let mut x: ( Bool, T) = true + sin(y)
    x = if (bar < 3) then
        let baz = bar.value + 2 * 4
        x + 1
    else if bar <= 2 then
        fizz(3, 5.1)

record Foo<T, U>(x: String, bar: Bar<Baz<T>, [U]>)
"#;

    let mut parser = Parser::new(input, &mut interner);

    let items = parser.parse().unwrap();

    //todo!("Fix Spans");
    assert_eq!(
        items[0],
        Item::Func {
            ident: parser.get_ident("wow_we_did_it").unwrap(),
            params: vec![
                Binding {
                    pat: Pat::Var {
                        mutable: true,
                        ident: parser.get_ident("x").unwrap(),
                    },
                    ty: None
                },
                Binding {
                    pat: Pat::Var {
                        mutable: false,
                        ident: parser.get_ident("bar").unwrap(),
                    },
                    ty: Some(Ty {
                        kind: TyKind::Adt {
                            ident: parser.get_ident("Bar").unwrap(),
                            args: vec![
                                Ty {
                                    kind: TyKind::Adt {
                                        ident: parser.get_ident("Baz").unwrap(),
                                        args: vec![Ty {
                                            kind: TyKind::Adt {
                                                ident: parser.get_ident("T").unwrap(),
                                                args: vec![],
                                            },
                                            span: Span::from(46..47)
                                        }],
                                    },
                                    span: Span::from(42..48)
                                },
                                Ty {
                                    kind: TyKind::Adt {
                                        ident: parser.get_ident("U").unwrap(),
                                        args: vec![],
                                    },
                                    span: Span::from(50..51)
                                }
                            ],
                        },
                        span: Span::from(38..52)
                    })
                }
            ],
            return_ty: Some(Ty {
                kind: TyKind::Fn(
                    vec![Ty {
                        kind: TyKind::Int,
                        span: Span::from(58..61)
                    }],
                    Ty {
                        kind: TyKind::Int,
                        span: Span::from(64..67)
                    }
                    .into()
                ),
                span: Span::from(55..67)
            }),
            body: ExprKind::Block(vec![
                ExprKind::Let {
                    binding: Binding {
                        pat: Pat::Var {
                            mutable: true,
                            ident: parser.get_ident("x").unwrap(),
                        },
                        ty: Some(Ty {
                            kind: TyKind::Tuple(vec![
                                Ty {
                                    kind: TyKind::Bool,
                                    span: Span::from(98..102)
                                },
                                Ty {
                                    kind: TyKind::Adt {
                                        ident: parser.get_ident("T").unwrap(),
                                        args: vec![]
                                    },
                                    span: Span::from(104..105)
                                }
                            ]),
                            span: Span::from(96..106)
                        })
                    },
                    value: ExprKind::BinOp {
                        op: Bop::Add,
                        lhs: ExprKind::Bool(true).span(109..113).into(),
                        rhs: ExprKind::App {
                            func: ExprKind::Ident(parser.get_ident("sin").unwrap())
                                .span(116..119)
                                .into(),
                            args: vec![
                                ExprKind::Ident(parser.get_ident("y").unwrap()).span(120..121)
                            ]
                        }
                        .span(116..122)
                        .into()
                    }
                    .span(109..122)
                    .into()
                }
                .span(85..122),
                ExprKind::Assign {
                    ident: Spnd(parser.get_ident("x").unwrap(), (136..137).into()),
                    value: ExprKind::If {
                        cond: ExprKind::BinOp {
                            op: Bop::Lt,
                            lhs: ExprKind::Ident(parser.get_ident("bar").unwrap())
                                .span(144..147)
                                .into(),
                            rhs: ExprKind::Int(3).span(150..151).into()
                        }
                        .span(144..151)
                        .into(),
                        th: ExprKind::Block(vec![
                            ExprKind::Let {
                                binding: Binding {
                                    pat: Pat::Var {
                                        mutable: false,
                                        ident: parser.get_ident("baz").unwrap(),
                                    },
                                    ty: None
                                },
                                value: ExprKind::BinOp {
                                    op: Bop::Add,
                                    lhs: ExprKind::FieldAccess {
                                        base: ExprKind::Ident(parser.get_ident("bar").unwrap())
                                            .span(181..184)
                                            .into(),
                                        field: Spnd(
                                            parser.get_ident("value").unwrap(),
                                            (185..190).into()
                                        )
                                    }
                                    .span(181..190)
                                    .into(),
                                    rhs: ExprKind::BinOp {
                                        op: Bop::Mul,
                                        lhs: ExprKind::Int(2).span(193..194).into(),
                                        rhs: ExprKind::Int(4).span(197..198).into()
                                    }
                                    .span(193..198)
                                    .into()
                                }
                                .span(181..198)
                                .into()
                            }
                            .span(171..198),
                            ExprKind::BinOp {
                                op: Bop::Add,
                                lhs: ExprKind::Ident(parser.get_ident("x").unwrap())
                                    .span(216..217)
                                    .into(),
                                rhs: ExprKind::Int(1).span(220..221).into()
                            }
                            .span(216..221)
                        ])
                        .span(153..236)
                        .into(),
                        el: Some(
                            ExprKind::If {
                                cond: ExprKind::BinOp {
                                    op: Bop::Leq,
                                    lhs: ExprKind::Ident(parser.get_ident("bar").unwrap())
                                        .span(246..249)
                                        .into(),
                                    rhs: ExprKind::Int(2).span(253..254).into()
                                }
                                .span(246..254)
                                .into(),
                                th: ExprKind::App {
                                    func: ExprKind::Ident(parser.get_ident("fizz").unwrap())
                                        .span(272..276)
                                        .into(),
                                    args: vec![
                                        ExprKind::Int(3).span(277..278),
                                        ExprKind::Float(5.1).span(280..283)
                                    ]
                                }
                                .span(272..284)
                                .into(),
                                el: None
                            }
                            .span(242..284)
                            .into()
                        )
                    }
                    .span(140..284)
                    .into()
                }
                .span(136..284),
            ])
            .span(71..294)
        }
    );

    assert_eq!(
        items[1],
        Item::Record {
            def: AdtDef {
                ident: parser.get_ident("Foo").unwrap(),
                generics: vec![
                    GenericParam(Spnd::span(parser.get_ident("T").unwrap(), 315..316)),
                    GenericParam(Spnd::span(parser.get_ident("U").unwrap(), 318..320)),
                ]
            },
            fields: vec![
                Field {
                    ident: parser.get_ident("x").unwrap(),
                    ty: Ty {
                        kind: TyKind::Adt {
                            ident: parser.get_ident("String").unwrap(),
                            args: vec![],
                        },
                        span: Span::from(338..344)
                    },
                    span: Span::from(0..0)
                },
                Field {
                    ident: parser.get_ident("bar").unwrap(),
                    ty: Ty {
                        kind: TyKind::Adt {
                            ident: parser.get_ident("Bar").unwrap(),
                            args: vec![
                                Ty {
                                    kind: TyKind::Adt {
                                        ident: parser.get_ident("Baz").unwrap(),
                                        args: vec![Ty {
                                            kind: TyKind::Adt {
                                                ident: parser.get_ident("T").unwrap(),
                                                args: vec![],
                                            },
                                            span: Span::from(371..372)
                                        }],
                                    },
                                    span: Span::from(367..373)
                                },
                                Ty {
                                    kind: TyKind::Array(Box::new(Ty {
                                        kind: TyKind::Adt {
                                            ident: parser.get_ident("U").unwrap(),
                                            args: vec![],
                                        },
                                        span: Span::from(376..377)
                                    })),
                                    span: Span::from(375..378)
                                }
                            ],
                        },
                        span: Span::from(363..379)
                    },
                    span: Span::from(0..0)
                }
            ]
        }
    );
}
