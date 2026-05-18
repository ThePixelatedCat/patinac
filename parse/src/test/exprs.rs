use pretty_assertions::assert_eq;

use ast::{
    exprs::{Arg, Binding, BlockExpr, ExprKind, InfixOp, LitExpr, MatchArm, PrefixOp, Stmt},
    patterns::PatKind,
    types::TyKind,
};
use ident::Ident;
use lex::TokKind;
use span::Span;

use crate::{ErrorKind, Parser};

#[test]
fn lit_expressions() {
    assert_eq!(Parser::parse_expr("42"), Ok(ExprKind::int(42).span(0..2)));

    assert_eq!(
        Parser::parse_expr("  2.7768"),
        Ok(ExprKind::float(2.7768).span(2..8))
    );

    assert_eq!(
        Parser::parse_expr(r#""I am a Str!""#),
        Ok(ExprKind::string("I am a Str!").span(0..13))
    );

    assert_eq!(
        Parser::parse_expr(r"'\''"),
        Ok(ExprKind::char('\'').span(0..4))
    );

    assert_eq!(
        Parser::parse_expr(r#"#(42,#(2,),"end")"#),
        Ok(ExprKind::Tuple(vec![
            ExprKind::int(42).span(2..4),
            ExprKind::Tuple(vec![ExprKind::int(2).span(7..8)]).span(5..10),
            ExprKind::string("end").span(11..16)
        ])
        .span(0..17))
    );

    let array = Parser::parse_expr(
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

    assert_eq!(
        Parser::parse_expr("foo"),
        Ok(ExprKind::ident("foo").span(0..3))
    );
}

#[test]
fn unop_expressions() {
    assert_eq!(
        Parser::parse_expr("!  is_visible"),
        Ok(ExprKind::Prefix {
            op: PrefixOp::Not,
            expr: ExprKind::ident("is_visible").span(3..13).into(),
        }
        .span(0..13))
    );

    assert_eq!(
        Parser::parse_expr("-{-13}"),
        Ok(ExprKind::Prefix {
            op: PrefixOp::Neg,
            expr: ExprKind::Block(
                ExprKind::Prefix {
                    op: PrefixOp::Neg,
                    expr: ExprKind::int(13).span(3..5).into(),
                }
                .span(2..5)
                .as_block(1..6)
            )
            .span(1..6)
            .into()
        }
        .span(0..6))
    );
}

#[test]
fn binop_expressions() {
    assert_eq!(
        Parser::parse_expr("4 + 2 * 3"),
        Ok(ExprKind::Infix {
            op: InfixOp::Add,
            lhs: ExprKind::int(4).span(0..1).into(),
            rhs: ExprKind::Infix {
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
        Parser::parse_expr("4.0 *. 2.0 +. 3.0"),
        Ok(ExprKind::Infix {
            op: InfixOp::AddF,
            lhs: ExprKind::Infix {
                op: InfixOp::MulF,
                lhs: ExprKind::float(4.0).span(0..3).into(),
                rhs: ExprKind::float(2.0).span(7..10).into()
            }
            .span(0..10)
            .into(),
            rhs: ExprKind::float(3.0).span(14..17).into(),
        }
        .span(0..17))
    );

    assert_eq!(
        Parser::parse_expr("4 - 2 - 3"),
        Ok(ExprKind::Infix {
            op: InfixOp::Sub,
            lhs: ExprKind::Infix {
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
        Parser::parse_expr("4 ** 2 ** 3"),
        Ok(ExprKind::Infix {
            op: InfixOp::Exp,
            lhs: ExprKind::int(4).span(0..1).into(),
            rhs: ExprKind::Infix {
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
        Parser::parse_expr("4 ^ 2 ^ 3"),
        Ok(ExprKind::Infix {
            op: InfixOp::Xor,
            lhs: ExprKind::Infix {
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
        Parser::parse_expr("true || false && true"),
        Ok(ExprKind::Infix {
            op: InfixOp::Or,
            lhs: ExprKind::bool(true).span(0..4).into(),
            rhs: ExprKind::Infix {
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
        Parser::parse_expr("{3 >= 4} != true"),
        Ok(ExprKind::Infix {
            op: InfixOp::Neq,
            lhs: ExprKind::Block(
                ExprKind::Infix {
                    op: InfixOp::Geq,
                    lhs: ExprKind::int(3).span(1..2).into(),
                    rhs: ExprKind::int(4).span(6..7).into(),
                }
                .span(1..7)
                .as_block(0..8)
            )
            .span(0..8)
            .into(),
            rhs: ExprKind::bool(true).span(12..16).into()
        }
        .span(0..16))
    );

    assert_eq!(
        Parser::parse_expr("{4 > 3} == true"),
        Ok(ExprKind::Infix {
            op: InfixOp::Eqq,
            lhs: ExprKind::Block(
                ExprKind::Infix {
                    op: InfixOp::Gt,
                    lhs: ExprKind::int(4).span(1..2).into(),
                    rhs: ExprKind::int(3).span(5..6).into(),
                }
                .span(1..6)
                .as_block(0..7)
            )
            .span(0..7)
            .into(),
            rhs: ExprKind::bool(true).span(11..15).into()
        }
        .span(0..15))
    );
}

#[test]
fn compound_expressions() {
    assert_eq!(
        Parser::parse_expr("bar (  mut x, 2, bar)"),
        Ok(ExprKind::Call {
            func: ExprKind::ident("bar").span(0..3).into(),
            args: vec![
                Arg {
                    mutable: true,
                    val: ExprKind::ident("x").span(11..12)
                },
                Arg {
                    mutable: false,
                    val: ExprKind::int(2).span(14..15)
                },
                Arg {
                    mutable: false,
                    val: ExprKind::ident("bar").span(17..20)
                },
            ],
        }
        .span(0..21))
    );

    assert_eq!(
        Parser::parse_expr("{fn(mut a, b: Int) -> a + b}(mut 1, 2)"),
        Ok(ExprKind::Call {
            func: ExprKind::Block(
                ExprKind::Lambda {
                    params: vec![
                        Binding {
                            mutable: true,
                            pat: PatKind::ident("a").span(8..9),
                            ty: None
                        },
                        Binding {
                            mutable: false,
                            pat: PatKind::ident("b").span(11..12),
                            ty: Some(TyKind::Int.span(14..17))
                        }
                    ],
                    body: ExprKind::Infix {
                        op: InfixOp::Add,
                        lhs: ExprKind::ident("a").span(22..23).into(),
                        rhs: ExprKind::ident("b").span(26..27).into()
                    }
                    .span(22..27)
                    .into()
                }
                .span(1..27)
                .as_block(0..28)
            )
            .span(0..28)
            .into(),
            args: vec![
                Arg {
                    mutable: true,
                    val: ExprKind::int(1).span(33..34)
                },
                Arg {
                    mutable: false,
                    val: ExprKind::int(2).span(36..37).into()
                },
            ]
        }
        .span(0..38))
    );

    assert_eq!(
        Parser::parse_expr("[1, 2, 3].[1-1]"),
        Ok(ExprKind::Index {
            arr: ExprKind::Array(vec![
                ExprKind::int(1).span(1..2),
                ExprKind::int(2).span(4..5),
                ExprKind::int(3).span(7..8)
            ])
            .span(0..9)
            .into(),
            idx: ExprKind::Infix {
                op: InfixOp::Sub,
                lhs: ExprKind::int(1).span(11..12).into(),
                rhs: ExprKind::int(1).span(13..14).into()
            }
            .span(11..14)
            .into()
        }
        .span(0..15))
    );

    assert_eq!(
        Parser::parse_expr("foo.bar"),
        Ok(ExprKind::Field {
            base: ExprKind::ident("foo").span(0..3).into(),
            field: Ident::new("bar").span(4..7)
        }
        .span(0..7))
    );
}

#[test]
fn var_expressions() {
    assert_eq!(
        Parser::parse_stmt("let x = 7 + sin(3.0)"),
        Ok(Stmt::Decl {
            binding: Binding {
                mutable: false,
                pat: PatKind::ident("x").span(4..5),
                ty: None
            },
            val: ExprKind::Infix {
                op: InfixOp::Add,
                lhs: ExprKind::int(7).span(8..9).into(),
                rhs: ExprKind::Call {
                    func: ExprKind::ident("sin").span(12..15).into(),
                    args: vec![Arg {
                        mutable: false,
                        val: ExprKind::float(3.0).span(16..19)
                    }]
                }
                .span(12..20)
                .into()
            }
            .span(8..20)
            .into(),
            span: Span::from(0..20)
        })
    );

    assert_eq!(
        Parser::parse_stmt("let mut y: Float = 7.0"),
        Ok(Stmt::Decl {
            binding: Binding {
                mutable: true,
                pat: PatKind::ident("y").span(8..9),
                ty: Some(TyKind::Float.span(11..16))
            },
            val: ExprKind::float(7.0).span(19..22).into(),
            span: Span::from(0..22)
        })
    );

    assert_eq!(
        Parser::parse_expr("y = 3 + 7 * 0.5"),
        Ok(ExprKind::Infix {
            op: InfixOp::Assign,
            lhs: ExprKind::ident("y").span(0..1).into(),
            rhs: ExprKind::Infix {
                op: InfixOp::Add,
                lhs: ExprKind::int(3).span(4..5).into(),
                rhs: ExprKind::Infix {
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
fn control_exprs() {
    assert_eq!(
        Parser::parse_expr("if 0.5 { foo() }"),
        Ok(ExprKind::If {
            cond: ExprKind::float(0.5).span(3..6).into(),
            th: ExprKind::Call {
                func: ExprKind::ident("foo").span(9..12).into(),
                args: Vec::new()
            }
            .span(9..14)
            .as_block(7..16),
            el: None
        }
        .span(0..16))
    );

    assert_eq!(
        Parser::parse_expr("if 0.5 { foo } else { bar }"),
        Ok(ExprKind::If {
            cond: ExprKind::float(0.5).span(3..6).into(),
            th: ExprKind::ident("foo").span(9..12).as_block(7..14),
            el: Some(ExprKind::ident("bar").span(22..25).as_block(20..27))
        }
        .span(0..27))
    );

    let input = "
    maybe.match {
        Some(v) -> v,
        None() -> panic(),
        -2 -> 2,
        #(a, b) -> #(b, a),
    }
";
    assert_eq!(
        Parser::parse_expr(input),
        Ok(ExprKind::Match {
            scrutinee: ExprKind::ident("maybe").span(5..10).into(),
            arms: vec![
                MatchArm {
                    pat: PatKind::Constructor(
                        Ident::new("Some"),
                        vec![PatKind::ident("v").span(32..33)]
                    )
                    .span(27..34),
                    body: ExprKind::ident("v").span(38..39),
                },
                MatchArm {
                    pat: PatKind::Constructor(Ident::new("None"), vec![]).span(49..55),
                    body: ExprKind::Call {
                        func: ExprKind::ident("panic").span(59..64).into(),
                        args: vec![]
                    }
                    .span(59..66),
                },
                MatchArm {
                    pat: PatKind::Literal {
                        negate: true,
                        lit: LitExpr::Int(2)
                    }
                    .span(76..78),
                    body: ExprKind::int(2).span(82..83),
                },
                MatchArm {
                    pat: PatKind::Tuple(vec![
                        PatKind::ident("a").span(95..96),
                        PatKind::ident("b").span(98..99)
                    ])
                    .span(93..100),
                    body: ExprKind::Tuple(vec![
                        ExprKind::ident("b").span(106..107),
                        ExprKind::ident("a").span(109..110)
                    ])
                    .span(104..111),
                }
            ]
        }
        .span(5..118))
    );

    assert_eq!(
        Parser::parse_expr(r#"for _ in ["Hello ", "World!"] { continue }"#),
        Ok(ExprKind::For {
            pat: PatKind::Wildcard.span(4..5),
            iter: ExprKind::Array(vec![
                ExprKind::string("Hello ").span(10..18),
                ExprKind::string("World!").span(20..28)
            ])
            .span(9..29)
            .into(),
            body: ExprKind::Continue.span(32..40).as_block(30..42),
        }
        .span(0..42))
    );

    assert_eq!(
        Parser::parse_expr("loop { break }"),
        Ok(ExprKind::Loop(ExprKind::Break.span(7..12).as_block(5..14)).span(0..14))
    );
}

#[test]
fn block_expressions() {
    let input = "{\
let mut y = 5
3 + 1 - 2
y = 1
if y < 3 {
    let a = 5
    a
} else { 32 }
}";

    assert_eq!(
        Parser::parse_expr(input),
        Ok(ExprKind::Block(BlockExpr {
            stmts: vec![
                Stmt::Decl {
                    binding: Binding {
                        mutable: true,
                        pat: PatKind::ident("y").span(9..10),
                        ty: None
                    },
                    val: ExprKind::int(5).span(13..14).into(),
                    span: Span::from(1..14)
                },
                Stmt::Expr(
                    ExprKind::Infix {
                        op: InfixOp::Sub,
                        lhs: ExprKind::Infix {
                            op: InfixOp::Add,
                            lhs: ExprKind::int(3).span(15..16).into(),
                            rhs: ExprKind::int(1).span(19..20).into()
                        }
                        .span(15..20)
                        .into(),
                        rhs: ExprKind::int(2).span(23..24).into()
                    }
                    .span(15..24)
                ),
                Stmt::Expr(
                    ExprKind::Infix {
                        op: InfixOp::Assign,
                        lhs: ExprKind::ident("y").span(25..26).into(),
                        rhs: ExprKind::int(1).span(29..30).into()
                    }
                    .span(25..30)
                ),
                Stmt::Expr(
                    ExprKind::If {
                        cond: ExprKind::Infix {
                            op: InfixOp::Lt,
                            lhs: ExprKind::ident("y").span(34..35).into(),
                            rhs: ExprKind::int(3).span(38..39).into()
                        }
                        .span(34..39)
                        .into(),
                        th: BlockExpr {
                            stmts: vec![
                                Stmt::Decl {
                                    binding: Binding {
                                        mutable: false,
                                        pat: PatKind::ident("a").span(50..51),
                                        ty: None
                                    },
                                    val: ExprKind::int(5).span(54..55).into(),
                                    span: Span::from(46..55)
                                },
                                Stmt::Expr(ExprKind::ident("a").span(60..61))
                            ],
                            span: Span::from(40..63)
                        },
                        el: Some(ExprKind::int(32).span(71..73).as_block(69..75))
                    }
                    .span(31..75)
                )
            ],
            span: Span::from(0..77)
        })
        .span(0..77))
    );
}

#[test]
fn malformed_expressions() {
    assert_eq!(
        Parser::parse_expr("let x = 7 + sin(3.0)"),
        Err(ErrorKind::Unexpected(TokKind::Let)
            .span(0..3)
            .with_ctx("`let` is a statement, and can only be used within a block"))
    );
    assert_eq!(
        Parser::parse_expr("[1, 3, 4, 5"),
        Err(ErrorKind::Eof.span(0..0))
    );
    assert_eq!(
        Parser::parse_expr("*5"),
        Err(ErrorKind::Unexpected(TokKind::Times).span(0..1))
    );
    assert_eq!(
        Parser::parse_stmt("let foo: fn(let UInt) -> UInt = fn()"),
        Err(ErrorKind::Unexpected(TokKind::Let).span(12..15))
    );
    assert_eq!(
        Parser::parse_expr("foo.0"),
        Err(ErrorKind::Unexpected(TokKind::IntLit)
            .span(4..5)
            .with_ctx("Expected `[` or identifier"))
    );
}
