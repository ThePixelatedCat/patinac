use crate::ast::{Bop, Expr, ExprKind, Pattern, Ty, TyKind, Unop};
use crate::helpers::{Span, Spannable, Spnd};
use crate::lexer::TokKind;
use crate::parser::{ParseError, ParseResult, Parser};

fn parse_expr(input: &str) -> Expr {
    let mut parser = Parser::new(input);
    parser.expr().unwrap()
}

fn parse_expr_err(input: &str) -> ParseResult<Expr> {
    let mut parser = Parser::new(input);
    parser.expr()
}

#[test]
fn lit_expressions() {
    assert_eq!(parse_expr("42"), ExprKind::Int(42).span(0..2));

    assert_eq!(parse_expr("  2.7768"), ExprKind::Float(2.7768).span(2..8));

    assert_eq!(
        parse_expr(r#""I am a Str!""#),
        ExprKind::String("I am a Str!".into()).span(0..13)
    );

    assert_eq!(parse_expr(r"'\''"), ExprKind::Char('\'').span(0..4));

    assert_eq!(
        parse_expr(r#"(42,(2,),"end")"#),
        ExprKind::Tuple(vec![
            ExprKind::Int(42).span(1..3),
            ExprKind::Tuple(vec![ExprKind::Int(2).span(5..6)]).span(4..8),
            ExprKind::String("end".into()).span(9..14)
        ])
        .span(0..15)
    );

    let array = parse_expr("
[
    1,
        4
    ,
    3,
    2
]
");
    assert_eq!(
        array,
        ExprKind::Array(vec![
            ExprKind::Int(1).span(7..8),
            ExprKind::Block(vec![ExprKind::Int(4).span(18..19)]).span(10..24),
            ExprKind::Int(3).span(30..31),
            ExprKind::Int(2).span(37..38)
        ])
        .span(1..40)
    );

    assert_eq!(parse_expr("foo"), ExprKind::Ident("foo".into()).span(0..3));
}

#[test]
fn unop_expressions() {
    assert_eq!(
        parse_expr("!  is_visible"),
        ExprKind::UnaryOp {
            op: Unop::Not,
            expr: ExprKind::Ident("is_visible".into()).span(3..13).into(),
        }
        .span(0..13)
    );

    assert_eq!(
        parse_expr("-(-13)"),
        ExprKind::UnaryOp {
            op: Unop::Neg,
            expr: ExprKind::UnaryOp {
                op: Unop::Neg,
                expr: ExprKind::Int(13).span(3..5).into(),
            }
            .span(1..6)
            .into()
        }
        .span(0..6)
    );
}

#[test]
fn binop_expressions() {
    assert_eq!(
        parse_expr("4 + 2 * 3"),
        ExprKind::BinaryOp {
            op: Bop::Add,
            lhs: ExprKind::Int(4).span(0..1).into(),
            rhs: ExprKind::BinaryOp {
                op: Bop::Mul,
                lhs: ExprKind::Int(2).span(4..5).into(),
                rhs: ExprKind::Int(3).span(8..9).into()
            }
            .span(4..9)
            .into()
        }
        .span(0..9)
    );

    assert_eq!(
        parse_expr("4 * 2 + 3"),
        ExprKind::BinaryOp {
            op: Bop::Add,
            lhs: ExprKind::BinaryOp {
                op: Bop::Mul,
                lhs: ExprKind::Int(4).span(0..1).into(),
                rhs: ExprKind::Int(2).span(4..5).into()
            }
            .span(0..5)
            .into(),
            rhs: ExprKind::Int(3).span(8..9).into(),
        }
        .span(0..9)
    );

    assert_eq!(
        parse_expr("4 - 2 - 3"),
        ExprKind::BinaryOp {
            op: Bop::Sub,
            lhs: ExprKind::BinaryOp {
                op: Bop::Sub,
                lhs: ExprKind::Int(4).span(0..1).into(),
                rhs: ExprKind::Int(2).span(4..5).into()
            }
            .span(0..5)
            .into(),
            rhs: ExprKind::Int(3).span(8..9).into(),
        }
        .span(0..9)
    );

    assert_eq!(
        parse_expr("4 ** 2 ** 3"),
        ExprKind::BinaryOp {
            op: Bop::Exp,
            lhs: ExprKind::Int(4).span(0..1).into(),
            rhs: ExprKind::BinaryOp {
                op: Bop::Exp,
                lhs: ExprKind::Int(2).span(5..6).into(),
                rhs: ExprKind::Int(3).span(10..11).into()
            }
            .span(5..11)
            .into()
        }
        .span(0..11)
    );

    assert_eq!(
        parse_expr("4 ^ 2 ^ 3"),
        ExprKind::BinaryOp {
            op: Bop::Xor,
            lhs: ExprKind::BinaryOp {
                op: Bop::Xor,
                lhs: ExprKind::Int(4).span(0..1).into(),
                rhs: ExprKind::Int(2).span(4..5).into()
            }
            .span(0..5)
            .into(),
            rhs: ExprKind::Int(3).span(8..9).into(),
        }
        .span(0..9)
    );

    assert_eq!(
        parse_expr("true || false && true"),
        ExprKind::BinaryOp {
            op: Bop::Or,
            lhs: ExprKind::Bool(true).span(0..4).into(),
            rhs: ExprKind::BinaryOp {
                op: Bop::And,
                lhs: ExprKind::Bool(false).span(8..13).into(),
                rhs: ExprKind::Bool(true).span(17..21).into(),
            }
            .span(8..21)
            .into()
        }
        .span(0..21)
    );

    assert_eq!(
        parse_expr("3 & 1 | 5"),
        ExprKind::BinaryOp {
            op: Bop::BOr,
            lhs: ExprKind::BinaryOp {
                op: Bop::BAnd,
                lhs: ExprKind::Int(3).span(0..1).into(),
                rhs: ExprKind::Int(1).span(4..5).into(),
            }
            .span(0..5)
            .into(),
            rhs: ExprKind::Int(5).span(8..9).into()
        }
        .span(0..9)
    );

    assert_eq!(
        parse_expr("(3 >= 4) != true"),
        ExprKind::BinaryOp {
            op: Bop::Neq,
            lhs: ExprKind::BinaryOp {
                op: Bop::Geq,
                lhs: ExprKind::Int(3).span(1..2).into(),
                rhs: ExprKind::Int(4).span(6..7).into(),
            }
            .span(0..8)
            .into(),
            rhs: ExprKind::Bool(true).span(12..16).into()
        }
        .span(0..16)
    );

    assert_eq!(
        parse_expr("(4 > 3) == true"),
        ExprKind::BinaryOp {
            op: Bop::Eqq,
            lhs: ExprKind::BinaryOp {
                op: Bop::Gt,
                lhs: ExprKind::Int(4).span(1..2).into(),
                rhs: ExprKind::Int(3).span(5..6).into(),
            }
            .span(0..7)
            .into(),
            rhs: ExprKind::Bool(true).span(11..15).into()
        }
        .span(0..15)
    );
}

#[test]
fn compound_expressions() {
    assert_eq!(
        parse_expr("bar (  x, 2)"),
        ExprKind::FnCall {
            fun: ExprKind::Ident("bar".into()).span(0..3).into(),
            args: vec![
                ExprKind::Ident("x".into()).span(7..8),
                ExprKind::Int(2).span(10..11),
            ],
        }
        .span(0..12)
    );

    assert_eq!(
        parse_expr("if 0.5 then foo()"),
        ExprKind::If {
            cond: ExprKind::Float(0.5).span(3..6).into(),
            th: ExprKind::FnCall {
                fun: ExprKind::Ident("foo".into()).span(12..15).into(),
                args: Vec::new()
            }
            .span(12..17)
            .into(),
            el: None
        }
        .span(0..17)
    );

    assert_eq!(
        parse_expr("if 0.5 then foo else bar"),
        ExprKind::If {
            cond: ExprKind::Float(0.5).span(3..6).into(),
            th: ExprKind::Ident("foo".into()).span(12..15).into(),
            el: Some(ExprKind::Ident("bar".into()).span(21..24).into())
        }
        .span(0..24)
    );

    assert_eq!(
        parse_expr("if a then if b then x else y"),
        ExprKind::If {
            cond: ExprKind::Ident("a".into()).span(3..4).into(),
            th: ExprKind::If {
                cond: ExprKind::Ident("b".into()).span(13..14).into(),
                th: ExprKind::Ident("x".into()).span(20..21).into(),
                el: Some(ExprKind::Ident("y".into()).span(27..28).into())
            }
            .span(10..28)
            .into(),
            el: None
        }
        .span(0..28)
    );

    assert_eq!(
        parse_expr("(fn(a, b: Int) -> a + b)(1, 2)"),
        ExprKind::FnCall {
            fun: ExprKind::Lambda {
                params: vec![
                    Pattern::Var {
                        mutable: false,
                        ident: "a".into(),
                        ty_annotation: None
                    },
                    Pattern::Var {
                        mutable: false,
                        ident: "b".into(),
                        ty_annotation: Some(Ty {
                            kind: TyKind::Int,
                            span: Span::from(10..13)
                        })
                    },
                ],
                return_type: None,
                body: ExprKind::BinaryOp {
                    op: Bop::Add,
                    lhs: ExprKind::Ident("a".into()).span(18..19).into(),
                    rhs: ExprKind::Ident("b".into()).span(22..23).into()
                }
                .span(18..23)
                .into()
            }
            .span(0..24)
            .into(),
            args: vec![
                ExprKind::Int(1).span(25..26).into(),
                ExprKind::Int(2).span(28..29).into()
            ]
        }
        .span(0..30)
    );

    assert_eq!(
        parse_expr("[1, 2, 3][1-1]"),
        ExprKind::Index {
            arr: ExprKind::Array(vec![
                ExprKind::Int(1).span(1..2),
                ExprKind::Int(2).span(4..5),
                ExprKind::Int(3).span(7..8)
            ])
            .span(0..9)
            .into(),
            index: ExprKind::BinaryOp {
                op: Bop::Sub,
                lhs: ExprKind::Int(1).span(10..11).into(),
                rhs: ExprKind::Int(1).span(12..13).into()
            }
            .span(10..13)
            .into()
        }
        .span(0..14)
    );

    assert_eq!(
        parse_expr("self._0"),
        ExprKind::FieldAccess {
            base: ExprKind::Ident("self".into()).span(0..4).into(),
            field: Spnd {
                inner: "_0".into(),
                span: (5..7).into()
            }
        }
        .span(0..7)
    );
}

#[test]
fn var_expressions() {
    assert_eq!(
        parse_expr("let x = 7 + sin(3.);"),
        ExprKind::Let {
            binding: Pattern::Var {
                mutable: false,
                ident: "x".into(),
                ty_annotation: None
            },
            value: ExprKind::BinaryOp {
                op: Bop::Add,
                lhs: ExprKind::Int(7).span(8..9).into(),
                rhs: ExprKind::FnCall {
                    fun: ExprKind::Ident("sin".into()).span(12..15).into(),
                    args: vec![ExprKind::Float(3.0).span(16..18)]
                }
                .span(12..19)
                .into()
            }
            .span(8..19)
            .into()
        }
        .span(0..19)
    );

    assert_eq!(
        parse_expr("let mut y: UInt = 7"),
        ExprKind::Let {
            binding: Pattern::Var {
                mutable: true,
                ident: "y".into(),
                ty_annotation: Some(Ty {
                    kind: TyKind::UInt,
                    span: Span::from(11..15)
                })
            },
            value: ExprKind::Int(7).span(18..19).into()
        }
        .span(0..19)
    );

    assert_eq!(
        parse_expr("y = 3 + 7 * 0.5"),
        ExprKind::Assign {
            ident: Spnd {
                inner: "y".into(),
                span: (0..1).into()
            },
            value: ExprKind::BinaryOp {
                op: Bop::Add,
                lhs: ExprKind::Int(3).span(4..5).into(),
                rhs: ExprKind::BinaryOp {
                    op: Bop::Mul,
                    lhs: ExprKind::Int(7).span(8..9).into(),
                    rhs: ExprKind::Float(0.5).span(12..15).into()
                }
                .span(8..15)
                .into()
            }
            .span(4..15)
            .into()
        }
        .span(0..15)
    );
}

#[test]
fn block_expressions() {
    let expr = parse_expr(
        "
    let mut y = 5
    3 + 1 - 2
    y = 1
    if y < 3 then
        let a = 5
        a
    else 32
",
    );
    assert_eq!(
        expr,
        ExprKind::Block(vec![
            ExprKind::Let {
                binding: Pattern::Var {
                    mutable: true,
                    ident: "y".into(),
                    ty_annotation: None
                },
                value: ExprKind::Int(5).span(17..18).into()
            }
            .span(5..18),
            ExprKind::BinaryOp {
                op: Bop::Sub,
                lhs: ExprKind::BinaryOp {
                    op: Bop::Add,
                    lhs: ExprKind::Int(3).span(23..24).into(),
                    rhs: ExprKind::Int(1).span(27..28).into()
                }
                .span(23..28)
                .into(),
                rhs: ExprKind::Int(2).span(31..32).into()
            }
            .span(23..32),
            ExprKind::Assign {
                ident: Spnd {
                    inner: "y".into(),
                    span: (37..38).into()
                },
                value: ExprKind::Int(1).span(41..42).into()
            }
            .span(37..42),
            ExprKind::If {
                cond: ExprKind::BinaryOp {
                    op: Bop::Lt,
                    lhs: ExprKind::Ident("y".into()).span(50..51).into(),
                    rhs: ExprKind::Int(3).span(54..55).into()
                }
                .span(50..55)
                .into(),
                th: ExprKind::Block(vec![
                    ExprKind::Let {
                        binding: Pattern::Var {
                            mutable: false,
                            ident: "a".into(),
                            ty_annotation: None
                        },
                        value: ExprKind::Int(5).span(77..78).into()
                    }
                    .span(69..78),
                    ExprKind::Ident("a".to_string()).span(87..88)
                ])
                .span(61..93)
                .into(),
                el: Some(ExprKind::Int(32).span(98..100).into())
            }
            .span(47..100)
        ])
        .span(1..101)
    );
}

#[test]
fn malformed_expressions() {
    assert_eq!(
        parse_expr_err("[1, 3, 4, 5"),
        Err(ParseError::Mismatched {
            expected: TokKind::RBracket,
            found: TokKind::Eof
        }
        .span(11..11))
    );
    assert_eq!(
        parse_expr_err("*5"),
        Err(ParseError::Unexpected(TokKind::Times, "start of expression").span(0..1))
    );
}
