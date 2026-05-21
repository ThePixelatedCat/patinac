mod exprs;
mod items;
mod prop;

use pretty_assertions::assert_eq;
use smallvec::smallvec;

use ast::{
    exprs::{Arg, Binding, BlockExpr, ExprKind, InfixOp, Stmt},
    items::{AdtItem, AdtKind, ExecItem, ExecKind, Field, Param},
    patterns::PatKind,
    types::{Param as ParamTy, Return, TyKind},
};
use ident::Ident;
use span::Span;

use crate::{TEST_HANDLER, Parser};

#[test]
fn file() {
    #[rustfmt::skip]
    let input =
"
fn testingfn(mut x: Bool, bar: Bar[Baz[T], U]): mut fn(mut Int) -> #()-> {
    let mut x: #(Bool, T) = true + sin(y)
    x = if bar < 3 {
        let baz = bar.value + 2 * 4
        x + 1
    } else {
        fizz(3, 5.1)
    }
}
record Foo[T, U](x: String, bar: Bar[Baz[T], Array[U]])
";

    let items = Parser::new(lex::lex(input).unwrap(), TEST_HANDLER)
        .parse()
        .unwrap();

    assert_eq!(
        items.execs[0],
        ExecItem {
            ident: Ident::new("testingfn").span(4..13),
            kind: ExecKind::Fn {
                generics: smallvec![],
                params: vec![
                    Param {
                        mutable: true,
                        pat: PatKind::ident("x").span(18..19),
                        ty: TyKind::Bool.span(21..25)
                    },
                    Param {
                        mutable: false,
                        pat: PatKind::ident("bar").span(27..30),
                        ty: TyKind::Adt(
                            Ident::new("Bar"),
                            vec![
                                TyKind::Adt(
                                    Ident::new("Baz"),
                                    vec![TyKind::named("T").span(40..41)],
                                )
                                .span(36..42),
                                TyKind::named("U").span(44..45)
                            ]
                        )
                        .span(32..46)
                    }
                ],
                ret_mut: true,
                ret_ty: TyKind::Fn(
                    vec![ParamTy {
                        mutable: true,
                        ty: TyKind::Int.span(60..63)
                    }],
                    Return {
                        mutable: false,
                        ty: TyKind::unit().span(68..71).into()
                    }
                )
                .span(53..71),
                body: ExprKind::Block(BlockExpr {
                    stmts: vec![
                        Stmt::Decl {
                            binding: Binding {
                                mutable: true,
                                pat: PatKind::ident("x").span(88..89),
                                ty: Some(
                                    TyKind::Tuple(vec![
                                        TyKind::Bool.span(93..97),
                                        TyKind::named("T").span(99..100)
                                    ])
                                    .span(91..101)
                                )
                            },
                            val: ExprKind::Infix {
                                op: InfixOp::Add,
                                lhs: ExprKind::bool(true).span(104..108).into(),
                                rhs: ExprKind::Call {
                                    func: ExprKind::ident("sin").span(111..114).into(),
                                    args: vec![Arg {
                                        mutable: false,
                                        val: ExprKind::ident("y").span(115..116)
                                    }]
                                }
                                .span(111..117)
                                .into()
                            }
                            .span(104..117),
                            span: Span::from(80..117)
                        },
                        Stmt::Expr(
                            ExprKind::Infix {
                                op: InfixOp::Assign,
                                lhs: ExprKind::ident("x").span(122..123).into(),
                                rhs: ExprKind::If {
                                    cond: ExprKind::Infix {
                                        op: InfixOp::Lt,
                                        lhs: ExprKind::ident("bar").span(129..132).into(),
                                        rhs: ExprKind::int(3).span(135..136).into()
                                    }
                                    .span(129..136)
                                    .into(),
                                    th: BlockExpr {
                                        stmts: vec![
                                            Stmt::Decl {
                                                binding: Binding {
                                                    mutable: false,
                                                    pat: PatKind::ident("baz").span(151..154),
                                                    ty: None
                                                },
                                                val: ExprKind::Infix {
                                                    op: InfixOp::Add,
                                                    lhs: ExprKind::Field {
                                                        base: ExprKind::ident("bar")
                                                            .span(157..160)
                                                            .into(),
                                                        field: Ident::new("value").span(161..166)
                                                    }
                                                    .span(157..166)
                                                    .into(),
                                                    rhs: ExprKind::Infix {
                                                        op: InfixOp::Mul,
                                                        lhs: ExprKind::int(2).span(169..170).into(),
                                                        rhs: ExprKind::int(4).span(173..174).into()
                                                    }
                                                    .span(169..174)
                                                    .into()
                                                }
                                                .span(157..174),
                                                span: Span::from(147..174)
                                            },
                                            Stmt::Expr(
                                                ExprKind::Infix {
                                                    op: InfixOp::Add,
                                                    lhs: ExprKind::ident("x").span(183..184).into(),
                                                    rhs: ExprKind::int(1).span(187..188).into()
                                                }
                                                .span(183..188)
                                            )
                                        ],
                                        span: Span::from(137..194)
                                    },
                                    el: Some(
                                        ExprKind::Call {
                                            func: ExprKind::ident("fizz").span(210..214).into(),
                                            args: vec![
                                                Arg {
                                                    mutable: false,
                                                    val: ExprKind::int(3).span(215..216)
                                                },
                                                Arg {
                                                    mutable: false,
                                                    val: ExprKind::float(5.1).span(218..221)
                                                }
                                            ]
                                        }
                                        .span(210..222)
                                        .as_block(200..228)
                                    )
                                }
                                .span(126..228)
                                .into()
                            }
                            .span(122..228)
                        ),
                    ],
                    span: Span::from(74..230)
                })
                .span(74..230)
            },
        }
    );

    assert_eq!(
        items.adts[0],
        AdtItem {
            ident: Ident::new("Foo").span(238..241),
            generics: smallvec![
                Ident::new("T").span(242..243),
                Ident::new("U").span(245..246),
            ],
            kind: AdtKind::Record(vec![
                Field {
                    ident: Ident::new("x").span(248..249),
                    ty: TyKind::string().span(251..257),
                },
                Field {
                    ident: Ident::new("bar").span(259..262),
                    ty: TyKind::Adt(
                        Ident::new("Bar"),
                        vec![
                            TyKind::Adt(Ident::new("Baz"), vec![TyKind::named("T").span(272..273)])
                                .span(268..274),
                            TyKind::array(TyKind::named("U").span(282..283)).span(276..284),
                        ]
                    )
                    .span(264..285),
                }
            ])
        }
    );
}
