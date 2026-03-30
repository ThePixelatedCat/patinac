use ast::{Arg, Binding, Expr, ExprKind, InfixOp, Pat, Ty, TyKind, UnaryOp};
use lex::TokKind;
use span::{Span, Spannable};

use crate::{ParseError, ParseResult, Parser};

fn parse_expr(input: &str) -> ParseResult<Expr<()>> {
    let mut interner = Default::default();
    let mut parser = Parser::new(input, &mut interner);
    parser.expr()
}

#[test]
fn lit_expressions() {
    assert_eq!(parse_expr("42"), Ok(ExprKind::int(42).span(0..2)));

    assert_eq!(
        parse_expr("  2.7768"),
        Ok(ExprKind::float(2.7768).span(2..8))
    );

    assert_eq!(
        parse_expr(r#""I am a Str!""#),
        Ok(ExprKind::string("I am a Str!").span(0..13))
    );

    assert_eq!(parse_expr(r"'\''"), Ok(ExprKind::char('\'').span(0..4)));

    assert_eq!(
        parse_expr(r#"{42,{2,},"end"}"#),
        Ok(ExprKind::Tuple(vec![
            ExprKind::int(42).span(1..3),
            ExprKind::Tuple(vec![ExprKind::int(2).span(5..6)]).span(4..8),
            ExprKind::string("end").span(9..14)
        ])
        .span(0..15))
    );

    let array = parse_expr(
        "
[
    1,
        4
    ,
    3,
    2
]
",
    );
    assert_eq!(
        array,
        Ok(ExprKind::Array(vec![
            ExprKind::int(1).span(7..8),
            ExprKind::int(4).span(18..19),
            ExprKind::int(3).span(30..31),
            ExprKind::int(2).span(37..38)
        ])
        .span(1..40))
    );

    let mut interner = Default::default();
    let mut parser = Parser::new("foo", &mut interner);
    assert_eq!(
        parser.expr(),
        Ok(ExprKind::ident(parser.get_interned("foo")).span(0..3))
    );
}

#[test]
fn unop_expressions() {
    let mut interner = Default::default();
    let mut parser = Parser::new("!  is_visible", &mut interner);
    assert_eq!(
        parser.expr(),
        Ok(ExprKind::UnaryExpr {
            op: UnaryOp::Not,
            expr: ExprKind::ident(parser.get_interned("is_visible"))
                .span(3..13)
                .into(),
        }
        .span(0..13))
    );

    assert_eq!(
        parse_expr("-{-13}"),
        Ok(ExprKind::UnaryExpr {
            op: UnaryOp::Neg,
            expr: ExprKind::Block(vec![
                ExprKind::UnaryExpr {
                    op: UnaryOp::Neg,
                    expr: ExprKind::int(13).span(3..5).into(),
                }
                .span(2..5)
            ])
            .span(1..6)
            .into()
        }
        .span(0..6))
    );
}

#[test]
fn binop_expressions() {
    assert_eq!(
        parse_expr("4 + 2 * 3"),
        Ok(ExprKind::InfixExpr {
            op: InfixOp::Add,
            lhs: ExprKind::int(4).span(0..1).into(),
            rhs: ExprKind::InfixExpr {
                op: InfixOp::Mul,
                lhs: ExprKind::int(2).span(4..5).into(),
                rhs: ExprKind::int(3).span(8..9).into()
            }
            .span(4..9)
            .into()
        }
        .span(0..9))
    );

    assert_eq!(
        parse_expr("4 * 2 + 3"),
        Ok(ExprKind::InfixExpr {
            op: InfixOp::Add,
            lhs: ExprKind::InfixExpr {
                op: InfixOp::Mul,
                lhs: ExprKind::int(4).span(0..1).into(),
                rhs: ExprKind::int(2).span(4..5).into()
            }
            .span(0..5)
            .into(),
            rhs: ExprKind::int(3).span(8..9).into(),
        }
        .span(0..9))
    );

    assert_eq!(
        parse_expr("4 - 2 - 3"),
        Ok(ExprKind::InfixExpr {
            op: InfixOp::Sub,
            lhs: ExprKind::InfixExpr {
                op: InfixOp::Sub,
                lhs: ExprKind::int(4).span(0..1).into(),
                rhs: ExprKind::int(2).span(4..5).into()
            }
            .span(0..5)
            .into(),
            rhs: ExprKind::int(3).span(8..9).into(),
        }
        .span(0..9))
    );

    assert_eq!(
        parse_expr("4 ** 2 ** 3"),
        Ok(ExprKind::InfixExpr {
            op: InfixOp::Exp,
            lhs: ExprKind::int(4).span(0..1).into(),
            rhs: ExprKind::InfixExpr {
                op: InfixOp::Exp,
                lhs: ExprKind::int(2).span(5..6).into(),
                rhs: ExprKind::int(3).span(10..11).into()
            }
            .span(5..11)
            .into()
        }
        .span(0..11))
    );

    assert_eq!(
        parse_expr("4 ^ 2 ^ 3"),
        Ok(ExprKind::InfixExpr {
            op: InfixOp::Xor,
            lhs: ExprKind::InfixExpr {
                op: InfixOp::Xor,
                lhs: ExprKind::int(4).span(0..1).into(),
                rhs: ExprKind::int(2).span(4..5).into()
            }
            .span(0..5)
            .into(),
            rhs: ExprKind::int(3).span(8..9).into(),
        }
        .span(0..9))
    );

    assert_eq!(
        parse_expr("true || false && true"),
        Ok(ExprKind::InfixExpr {
            op: InfixOp::Or,
            lhs: ExprKind::bool(true).span(0..4).into(),
            rhs: ExprKind::InfixExpr {
                op: InfixOp::And,
                lhs: ExprKind::bool(false).span(8..13).into(),
                rhs: ExprKind::bool(true).span(17..21).into(),
            }
            .span(8..21)
            .into()
        }
        .span(0..21))
    );

    assert_eq!(
        parse_expr("{3 >= 4} != true"),
        Ok(ExprKind::InfixExpr {
            op: InfixOp::Neq,
            lhs: ExprKind::Block(vec![
                ExprKind::InfixExpr {
                    op: InfixOp::Geq,
                    lhs: ExprKind::int(3).span(1..2).into(),
                    rhs: ExprKind::int(4).span(6..7).into(),
                }
                .span(1..7)
            ])
            .span(0..8)
            .into(),
            rhs: ExprKind::bool(true).span(12..16).into()
        }
        .span(0..16))
    );

    assert_eq!(
        parse_expr("{4 > 3} == true"),
        Ok(ExprKind::InfixExpr {
            op: InfixOp::Eqq,
            lhs: ExprKind::Block(vec![
                ExprKind::InfixExpr {
                    op: InfixOp::Gt,
                    lhs: ExprKind::int(4).span(1..2).into(),
                    rhs: ExprKind::int(3).span(5..6).into(),
                }
                .span(1..6)
            ])
            .span(0..7)
            .into(),
            rhs: ExprKind::bool(true).span(11..15).into()
        }
        .span(0..15))
    );
}

#[test]
fn compound_expressions() {
    let mut interner = Default::default();

    let mut parser = Parser::new("bar (  mut x, 2, .foo = bar)", &mut interner);
    assert_eq!(
        parser.expr(),
        Ok(ExprKind::CallExpr {
            func: ExprKind::ident(parser.get_interned("bar"))
                .span(0..3)
                .into(),
            args: vec![
                Arg {
                    mutable: true,
                    label: None,
                    val: ExprKind::ident(parser.get_interned("x")).span(11..12)
                },
                Arg {
                    mutable: false,
                    label: None,
                    val: ExprKind::int(2).span(14..15)
                },
                Arg {
                    mutable: false,
                    label: Some(Pat::Ident {
                        ident: parser.get_interned("foo"),
                        subpat: None
                    }),
                    val: ExprKind::ident(parser.get_interned("bar")).span(24..27)
                },
            ],
        }
        .span(0..28))
    );

    let mut parser = Parser::new("if 0.5 then foo()", &mut interner);
    assert_eq!(
        parser.expr(),
        Ok(ExprKind::If {
            cond: ExprKind::float(0.5).span(3..6).into(),
            th: ExprKind::CallExpr {
                func: ExprKind::ident(parser.get_interned("foo"))
                    .span(12..15)
                    .into(),
                args: Vec::new()
            }
            .span(12..17)
            .into(),
            el: None
        }
        .span(0..17))
    );

    let mut parser = Parser::new("if 0.5 then foo else bar", &mut interner);
    assert_eq!(
        parser.expr(),
        Ok(ExprKind::If {
            cond: ExprKind::float(0.5).span(3..6).into(),
            th: ExprKind::ident(parser.get_interned("foo"))
                .span(12..15)
                .into(),
            el: Some(
                ExprKind::ident(parser.get_interned("bar"))
                    .span(21..24)
                    .into()
            )
        }
        .span(0..24))
    );

    let mut parser = Parser::new("if a then if b then x else y", &mut interner);
    assert_eq!(
        parser.expr(),
        Ok(ExprKind::If {
            cond: ExprKind::ident(parser.get_interned("a")).span(3..4).into(),
            th: ExprKind::If {
                cond: ExprKind::ident(parser.get_interned("b"))
                    .span(13..14)
                    .into(),
                th: ExprKind::ident(parser.get_interned("x"))
                    .span(20..21)
                    .into(),
                el: Some(
                    ExprKind::ident(parser.get_interned("y"))
                        .span(27..28)
                        .into()
                )
            }
            .span(10..28)
            .into(),
            el: None
        }
        .span(0..28))
    );

    let mut parser = Parser::new("{fn(mut a, b: Int) -> a + b}(mut 1, .b = 2)", &mut interner);
    assert_eq!(
        parser.expr(),
        Ok(ExprKind::CallExpr {
            func: ExprKind::Block(vec![
                ExprKind::LambdaExpr {
                    params: vec![
                        Binding {
                            mutable: true,
                            pat: Pat::Ident {
                                ident: parser.get_interned("a"),
                                subpat: None
                            },
                            ty: None
                        },
                        Binding {
                            mutable: false,
                            pat: Pat::Ident {
                                ident: parser.get_interned("b"),
                                subpat: None
                            },
                            ty: Some(Ty {
                                kind: TyKind::Int,
                                span: Span::from(14..17)
                            })
                        }
                    ],
                    return_ty: None,
                    body: ExprKind::InfixExpr {
                        op: InfixOp::Add,
                        lhs: ExprKind::ident(parser.get_interned("a"))
                            .span(22..23)
                            .into(),
                        rhs: ExprKind::ident(parser.get_interned("b"))
                            .span(26..27)
                            .into()
                    }
                    .span(22..27)
                    .into()
                }
                .span(1..27)
            ])
            .span(0..28)
            .into(),
            args: vec![
                Arg {
                    mutable: true,
                    label: None,
                    val: ExprKind::int(1).span(33..34)
                },
                Arg {
                    mutable: false,
                    label: Some(Pat::Ident {
                        ident: parser.get_interned("b"),
                        subpat: None
                    }),
                    val: ExprKind::int(2).span(41..42).into()
                },
            ]
        }
        .span(0..43))
    );

    assert_eq!(
        parse_expr("[1, 2, 3].[1-1]"),
        Ok(ExprKind::IndexExpr {
            arr: ExprKind::Array(vec![
                ExprKind::int(1).span(1..2),
                ExprKind::int(2).span(4..5),
                ExprKind::int(3).span(7..8)
            ])
            .span(0..9)
            .into(),
            idx: ExprKind::InfixExpr {
                op: InfixOp::Sub,
                lhs: ExprKind::int(1).span(11..12).into(),
                rhs: ExprKind::int(1).span(13..14).into()
            }
            .span(11..14)
            .into()
        }
        .span(0..15))
    );
}

#[test]
fn var_expressions() {
    let mut interner = Default::default();

    let mut parser = Parser::new("let x = 7 + sin(3.0)", &mut interner);
    assert_eq!(
        parser.expr(),
        Ok(ExprKind::Let {
            binding: Binding {
                mutable: false,
                pat: Pat::Ident {
                    ident: parser.get_interned("x"),
                    subpat: None
                },
                ty: None
            },
            val: ExprKind::InfixExpr {
                op: InfixOp::Add,
                lhs: ExprKind::int(7).span(8..9).into(),
                rhs: ExprKind::CallExpr {
                    func: ExprKind::ident(parser.get_interned("sin"))
                        .span(12..15)
                        .into(),
                    args: vec![Arg {
                        mutable: false,
                        label: None,
                        val: ExprKind::float(3.0).span(16..19)
                    }]
                }
                .span(12..20)
                .into()
            }
            .span(8..20)
            .into()
        }
        .span(0..20))
    );

    let mut parser = Parser::new("let mut y: UInt = 7", &mut interner);
    assert_eq!(
        parser.expr(),
        Ok(ExprKind::Let {
            binding: Binding {
                mutable: true,
                pat: Pat::Ident {
                    ident: parser.get_interned("y"),
                    subpat: None
                },
                ty: Some(Ty {
                    kind: TyKind::UInt,
                    span: Span::from(11..15)
                })
            },
            val: ExprKind::int(7).span(18..19).into()
        }
        .span(0..19))
    );

    let mut parser = Parser::new("y = 3 + 7 * 0.5", &mut interner);
    assert_eq!(
        parser.expr(),
        Ok(ExprKind::Assign {
            place: Box::new(ExprKind::ident(parser.get_interned("y")).span(0..1)),
            val: ExprKind::InfixExpr {
                op: InfixOp::Add,
                lhs: ExprKind::int(3).span(4..5).into(),
                rhs: ExprKind::InfixExpr {
                    op: InfixOp::Mul,
                    lhs: ExprKind::int(7).span(8..9).into(),
                    rhs: ExprKind::float(0.5).span(12..15).into()
                }
                .span(8..15)
                .into()
            }
            .span(4..15)
            .into()
        }
        .span(0..15))
    );
}

#[test]
fn block_expressions() {
    let mut interner = Default::default();

    let input = "{\
let mut y = 5
3 + 1 - 2
y = 1
if y < 3 then {
    let a = 5
    a
} else 32
}";
    let mut parser = Parser::new(input, &mut interner);
    assert_eq!(
        parser.expr(),
        Ok(ExprKind::Block(vec![
            ExprKind::Let {
                binding: Binding {
                    mutable: true,
                    pat: Pat::Ident {
                        ident: parser.get_interned("y"),
                        subpat: None
                    },
                    ty: None
                },
                val: ExprKind::int(5).span(13..14).into()
            }
            .span(1..14),
            ExprKind::InfixExpr {
                op: InfixOp::Sub,
                lhs: ExprKind::InfixExpr {
                    op: InfixOp::Add,
                    lhs: ExprKind::int(3).span(15..16).into(),
                    rhs: ExprKind::int(1).span(19..20).into()
                }
                .span(15..20)
                .into(),
                rhs: ExprKind::int(2).span(23..24).into()
            }
            .span(15..24),
            ExprKind::Assign {
                place: Box::new(ExprKind::ident(parser.get_interned("y")).span(25..26)),
                val: ExprKind::int(1).span(29..30).into()
            }
            .span(25..30),
            ExprKind::If {
                cond: ExprKind::InfixExpr {
                    op: InfixOp::Lt,
                    lhs: ExprKind::ident(parser.get_interned("y"))
                        .span(50..51)
                        .into(),
                    rhs: ExprKind::int(3).span(54..55).into()
                }
                .span(50..55)
                .into(),
                th: ExprKind::Block(vec![
                    ExprKind::Let {
                        binding: Binding {
                            mutable: false,
                            pat: Pat::Ident {
                                ident: parser.get_interned("a"),
                                subpat: None
                            },
                            ty: None
                        },
                        val: ExprKind::int(5).span(77..78).into()
                    }
                    .span(69..78),
                    ExprKind::ident(parser.get_interned("a")).span(87..88)
                ])
                .span(61..93)
                .into(),
                el: Some(ExprKind::int(32).span(98..100).into())
            }
            .span(47..100)
        ])
        .span(1..101))
    );
}

#[test]
fn malformed_expressions() {
    assert_eq!(parse_expr("[1, 3, 4, 5"), Err(ParseError::Eof.span(11..11)));
    assert_eq!(
        parse_expr("*5"),
        Err(ParseError::Unexpected(TokKind::Times, "start of expression").span(0..1))
    );
}
