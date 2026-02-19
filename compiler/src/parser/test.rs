#[cfg(test)]
mod exprs;
#[cfg(test)]
mod items;

use crate::{
    ast::{GenericParam, Generics, Ty, VariantData},
    helpers::{Span, Spnd},
};

use super::Parser;
use crate::ast::{AdtDef, Bop, ExprKind, Field, Item, Pattern, TyKind};

fn parse_ast(input: &str) -> Vec<Item> {
    let mut parser = Parser::new(input);
    parser.file().unwrap()
}

#[test]
fn file() {
    todo!("Fix Spans");

    #[rustfmt::skip]
    let items = parse_ast(
r#"
fn wow_we_did_it(mut x, bar: Bar<Baz<T>, U>): fn(Int): Int -> 
    let mut x: ( Bool, T) = true + sin(y)
    x = if (bar < 3) then
        let baz = bar.value + 2 * 4
        x + 1
    else if bar <= 2 then
        fizz(3, 5.1)

record Foo<T, U> 
    x: String,
    bar: Bar<Baz<T>, [U]>,
"#,
    );

    assert_eq!(
        items[0],
        Item::Func {
            ident: "wow_we_did_it".into(),
            params: vec![
                Pattern::Var {
                    mutable: true,
                    ident: "x".into(),
                    ty_annotation: None
                },
                Pattern::Var {
                    mutable: false,
                    ident: "bar".into(),
                    ty_annotation: Some(Ty {
                        kind: TyKind::Adt {
                            ident: "Bar".into(),
                            args: vec![
                                Ty {
                                    kind: TyKind::Adt {
                                        ident: "Baz".into(),
                                        args: vec![Ty {
                                            kind: TyKind::Adt {
                                                ident: "T".into(),
                                                args: vec![],
                                            },
                                            span: Span::from(46..47)
                                        }],
                                    },
                                    span: Span::from(42..48)
                                },
                                Ty {
                                    kind: TyKind::Adt {
                                        ident: "U".into(),
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
            body: ExprKind::Block {
                exprs: vec![
                    ExprKind::Let {
                        binding: Pattern::Var {
                            mutable: true,
                            ident: "x".into(),
                            ty_annotation: Some(Ty {
                                kind: TyKind::Tuple(vec![
                                    Ty {
                                        kind: TyKind::Bool,
                                        span: Span::from(98..102)
                                    },
                                    Ty {
                                        kind: TyKind::Adt {
                                            ident: "T".into(),
                                            args: vec![]
                                        },
                                        span: Span::from(104..105)
                                    }
                                ]),
                                span: Span::from(96..106)
                            })
                        },
                        value: ExprKind::BinaryOp {
                            op: Bop::Add,
                            lhs: ExprKind::Bool(true).span(109..113).into(),
                            rhs: ExprKind::FnCall {
                                fun: ExprKind::Ident("sin".into()).span(116..119).into(),
                                args: vec![ExprKind::Ident("y".into()).span(120..121)]
                            }
                            .span(116..122)
                            .into()
                        }
                        .span(109..122)
                        .into()
                    }
                    .span(85..122),
                    ExprKind::Assign {
                        ident: Spnd {
                            inner: "x".into(),
                            span: (136..137).into()
                        },
                        value: ExprKind::If {
                            cond: ExprKind::BinaryOp {
                                op: Bop::Lt,
                                lhs: ExprKind::Ident("bar".into()).span(144..147).into(),
                                rhs: ExprKind::Int(3).span(150..151).into()
                            }
                            .span(144..151)
                            .into(),
                            th: ExprKind::Block {
                                exprs: vec![
                                    ExprKind::Let {
                                        binding: Pattern::Var {
                                            mutable: false,
                                            ident: "baz".into(),
                                            ty_annotation: None
                                        },
                                        value: ExprKind::BinaryOp {
                                            op: Bop::Add,
                                            lhs: ExprKind::FieldAccess {
                                                base: ExprKind::Ident("bar".into())
                                                    .span(181..184)
                                                    .into(),
                                                field: Spnd {
                                                    inner: "value".into(),
                                                    span: (185..190).into()
                                                }
                                            }
                                            .span(181..190)
                                            .into(),
                                            rhs: ExprKind::BinaryOp {
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
                                    ExprKind::BinaryOp {
                                        op: Bop::Add,
                                        lhs: ExprKind::Ident("x".into()).span(216..217).into(),
                                        rhs: ExprKind::Int(1).span(220..221).into()
                                    }
                                    .span(216..221)
                                ],
                                trailing: false
                            }
                            .span(153..236)
                            .into(),
                            el: Some(
                                ExprKind::If {
                                    cond: ExprKind::BinaryOp {
                                        op: Bop::Leq,
                                        lhs: ExprKind::Ident("bar".into()).span(246..249).into(),
                                        rhs: ExprKind::Int(2).span(253..254).into()
                                    }
                                    .span(246..254)
                                    .into(),
                                    th: ExprKind::FnCall {
                                        fun: ExprKind::Ident("fizz".into()).span(272..276).into(),
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
                ],
                trailing: true
            }
            .span(71..294)
        }
    );

    assert_eq!(
        items[1],
        Item::Record {
            def: AdtDef {
                ident: "Foo".into(),
                generics: Some(Generics {
                    params: vec![
                        GenericParam {
                            ident: String::from("T"),
                            span: Span::from(315..316)
                        },
                        GenericParam {
                            ident: String::from("U"),
                            span: Span::from(318..320)
                        },
                    ],
                    span: Span::from(0..0)
                })
            },
            data: VariantData::Record(vec![
                Field {
                    ident: "x".into(),
                    ty: Ty {
                        kind: TyKind::Adt {
                            ident: "String".into(),
                            args: vec![],
                        },
                        span: Span::from(338..344)
                    },
                    span: Span::from(0..0)
                },
                Field {
                    ident: "bar".into(),
                    ty: Ty {
                        kind: TyKind::Adt {
                            ident: "Bar".into(),
                            args: vec![
                                Ty {
                                    kind: TyKind::Adt {
                                        ident: "Baz".into(),
                                        args: vec![Ty {
                                            kind: TyKind::Adt {
                                                ident: "T".into(),
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
                                            ident: "U".into(),
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
            ])
        }
    );
}
