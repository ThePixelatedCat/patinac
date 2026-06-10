mod exprs;
mod items;
mod lex;

use std::range::Range;

use itertools::Itertools as _;
use pretty_assertions::assert_eq;
use proptest::{collection::vec, prelude::*};
use smallvec::smallvec;

use ast::{
    Arg, Binding, BlockExpr, ExecItem, ExecKind, ExprKind, Field, InfixOp, Param, ParamTy, PatKind,
    Path, Return, Stmt, TyItem, TyItemKind, TyKind,
};
use errors::ErrorHandler;
use ident::Ident;
use package::ModuleId;

use crate::{Parser, TokKind};

#[test]
#[allow(clippy::too_many_lines, reason = "It's a test function")]
#[allow(clippy::unwrap_used, reason = "It's a test function")]
fn file() {
    #[rustfmt::skip]
    let input =
"
fn testingfn(mut x: Bool, bar: Bar[Baz[T], U]): mut Fn(mut Int) ->  () = {
    let mut x:  (Bool, T) = true + sin(y)
    x = if bar < 3 {
        let baz = bar.value + 2 * 4
        x + 1
    } else {
        fizz(3, 5.1)
    }
}
record Foo[T, U](x: String, bar: Bar[Baz[T], [U]])
";

    let ast = Parser::new_test(input).parse().unwrap();

    assert_eq!(
        ast.execs[0],
        ExecItem {
            ident: Ident::new("testingfn").span(4..13),
            kind: ExecKind::Fn {
                generics: smallvec![],
                params: vec![
                    Param {
                        pat: PatKind::ident("x").span(18..19),
                        ty: TyKind::Bool.span(21..25),
                        mutable: true,
                        span: Range::from(14..25)
                    },
                    Param {
                        pat: PatKind::ident("bar").span(27..30),
                        ty: TyKind::Named(
                            Path::single(Ident::new("Bar")),
                            vec![
                                TyKind::Named(
                                    Path::single(Ident::new("Baz")),
                                    vec![TyKind::named("T").span(40..41)],
                                )
                                .span(36..42),
                                TyKind::named("U").span(44..45)
                            ]
                        )
                        .span(32..46),
                        mutable: false,
                        span: Range::from(27..46)
                    }
                ],
                ret_mut: true,
                ret_ty: TyKind::Fn(
                    vec![ParamTy {
                        ty: TyKind::Int.span(60..63),
                        mutable: true,
                        span: Range::from(56..63)
                    }],
                    Return {
                        mutable: false,
                        ty: TyKind::unit().span(69..71).into()
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
                                    .span(92..101)
                                )
                            },
                            val: ExprKind::Infix {
                                op: InfixOp::Add,
                                lhs: ExprKind::bool(true).span(104..108).into(),
                                rhs: ExprKind::Call {
                                    func: ExprKind::ident("sin").span(111..114).into(),
                                    args: vec![Arg {
                                        val: ExprKind::ident("y").span(115..116),
                                        mutable: false,
                                        span: Range::from(115..116)
                                    }]
                                }
                                .span(111..117)
                                .into()
                            }
                            .span(104..117),
                            span: Range::from(80..117)
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
                                                span: Range::from(147..174)
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
                                        span: Range::from(137..194)
                                    },
                                    el: Some(
                                        ExprKind::Call {
                                            func: ExprKind::ident("fizz").span(210..214).into(),
                                            args: vec![
                                                Arg {
                                                    val: ExprKind::int(3).span(215..216),
                                                    mutable: false,
                                                    span: Range::from(215..216)
                                                },
                                                Arg {
                                                    val: ExprKind::float(5.1).span(218..221),
                                                    mutable: false,
                                                    span: Range::from(218..221)
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
                    span: Range::from(74..230)
                })
                .span(74..230)
            },
        }
    );

    assert_eq!(
        ast.tys[0],
        TyItem {
            ident: Ident::new("Foo").span(238..241),
            generics: smallvec![
                Ident::new("T").span(242..243),
                Ident::new("U").span(245..246),
            ],
            kind: TyItemKind::Record(vec![
                Field {
                    ident: Ident::new("x").span(248..249),
                    ty: TyKind::named("String").span(251..257),
                },
                Field {
                    ident: Ident::new("bar").span(259..262),
                    ty: TyKind::Named(
                        Path::single(Ident::new("Bar")),
                        vec![
                            TyKind::Named(
                                Path::single(Ident::new("Baz")),
                                vec![TyKind::named("T").span(272..273)]
                            )
                            .span(268..274),
                            TyKind::Array(TyKind::named("U").span(277..278).into()).span(276..279),
                        ]
                    )
                    .span(264..280),
                }
            ])
        }
    );
}

proptest! {
    #[test]
    fn doesnt_crash(toks in vec(TokKind::arbitrary(), 8..=512)) {
        let raw = toks.iter().map(|t| t.reverse()).join(" ");
        let _ = Parser::new(ModuleId::default(), &raw, ErrorHandler::DUMMY).parse();
    }
}
