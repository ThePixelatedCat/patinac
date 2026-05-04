mod exprs;
mod items;
mod prop;

use pretty_assertions::assert_eq;
use smallvec::smallvec;

use ast::{
    exprs::{Arg, Binding, ExprKind, InfixOp, Stmt},
    items::{AdtItem, AdtKind, ExecItem, ExecKind, Field, Param, Return},
    patterns::PatKind,
    types::{Param as TyParam, TyKind},
};
use ident::Ident;
use span::Span;

use crate::Parser;

#[test]
fn file() {
    #[rustfmt::skip]
    let input =
r#"
fn testingfn(mut x: Bool, bar: Bar[Baz[T], U]): mut fn(mut Int) -> #()-> {
    let mut x: #(Bool, T) = true + sin(y)
    x = if bar < 3 then {
        let baz = bar.value + 2 * 4
        x + 1
    } else if bar <= 2 then
        fizz(3, 5.1)
}
record Foo[T, U](x: String, bar: Bar[Baz[T], Array[U]])
"#;

    let items = Parser::parse(lex::lex(input).unwrap()).unwrap();

    assert_eq!(
        items.execs[0],
        ExecItem {
            ident: Ident::new("testingfn"),
            ident_span: Span::from(4..13),
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
                result: Return {
                    mutable: true,
                    ty: TyKind::Fn {
                        params: vec![TyParam {
                            mutable: true,
                            ty: TyKind::Int.span(60..63)
                        }],
                        result: TyKind::Tuple(vec![]).span(68..71).into()
                    }
                    .span(53..71)
                },
                body: ExprKind::Block(vec![
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
                                th: ExprKind::Block(vec![
                                    Stmt::Decl {
                                        binding: Binding {
                                            mutable: false,
                                            pat: PatKind::ident("baz").span(156..159),
                                            ty: None
                                        },
                                        val: ExprKind::Infix {
                                            op: InfixOp::Add,
                                            lhs: ExprKind::Field {
                                                base: ExprKind::ident("bar").span(162..165).into(),
                                                field: Ident::new("value").span(166..171)
                                            }
                                            .span(162..171)
                                            .into(),
                                            rhs: ExprKind::Infix {
                                                op: InfixOp::Mul,
                                                lhs: ExprKind::int(2).span(174..175).into(),
                                                rhs: ExprKind::int(4).span(178..179).into()
                                            }
                                            .span(174..179)
                                            .into()
                                        }
                                        .span(162..179),
                                        span: Span::from(152..179)
                                    },
                                    Stmt::Expr(
                                        ExprKind::Infix {
                                            op: InfixOp::Add,
                                            lhs: ExprKind::ident("x").span(188..189).into(),
                                            rhs: ExprKind::int(1).span(192..193).into()
                                        }
                                        .span(188..193)
                                    )
                                ])
                                .span(142..199)
                                .into(),
                                el: Some(
                                    ExprKind::If {
                                        cond: ExprKind::Infix {
                                            op: InfixOp::Leq,
                                            lhs: ExprKind::ident("bar").span(208..211).into(),
                                            rhs: ExprKind::int(2).span(215..216).into()
                                        }
                                        .span(208..216)
                                        .into(),
                                        th: ExprKind::Call {
                                            func: ExprKind::ident("fizz").span(230..234).into(),
                                            args: vec![
                                                Arg {
                                                    mutable: false,
                                                    val: ExprKind::int(3).span(235..236)
                                                },
                                                Arg {
                                                    mutable: false,
                                                    val: ExprKind::float(5.1).span(238..241)
                                                }
                                            ]
                                        }
                                        .span(230..242)
                                        .into(),
                                        el: None
                                    }
                                    .span(205..242)
                                    .into()
                                )
                            }
                            .span(126..242)
                            .into()
                        }
                        .span(122..242)
                    ),
                ])
                .span(74..244)
            },
        }
    );

    assert_eq!(
        items.adts[0],
        AdtItem {
            ident: Ident::new("Foo").span(252..255),
            generics: smallvec![Ident::new("T"), Ident::new("U"),],
            span: Span::from(245..300),
            kind: AdtKind::Record(vec![
                Field {
                    ident: Ident::new("x"),
                    ty: TyKind::string().span(265..271),
                    span: Span::from(262..271)
                },
                Field {
                    ident: Ident::new("bar"),
                    ty: TyKind::Adt(
                        Ident::new("Bar"),
                        vec![
                            TyKind::Adt(Ident::new("Baz"), vec![TyKind::named("T").span(286..287)])
                                .span(282..288),
                            TyKind::array(TyKind::named("U").span(296..297)).span(290..298),
                        ]
                    )
                    .span(278..299),
                    span: Span::from(273..299)
                }
            ])
        }
    );
}
