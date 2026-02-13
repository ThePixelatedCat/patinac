use crate::helpers::{Spannable, Spnd};
use crate::lexer::TT;
use crate::parser::{
    ParseError, ParseResult, Parser,
    ast::{Bop, Expr, ExprS, Pattern, Type, Unop},
};

fn parse_expr(input: &str) -> ExprS {
    let mut parser = Parser::new(input);
    parser.expr().unwrap()
}

fn parse_expr_err(input: &str) -> ParseResult<ExprS> {
    let mut parser = Parser::new(input);
    parser.expr()
}

#[test]
fn lit_expressions() {
    assert_eq!(parse_expr("42"), Expr::Int(42).span(0..2));

    assert_eq!(parse_expr("  2.7768"), Expr::Float(2.7768).span(2..8));

    assert_eq!(
        parse_expr(r#""I am a Str!""#),
        Expr::String("I am a Str!".into()).span(0..13)
    );

    assert_eq!(parse_expr(r"'\''"), Expr::Char('\'').span(0..4));

    assert_eq!(
        parse_expr(r#"(42,(2,),"end")"#),
        Expr::Tuple(vec![
            Expr::Int(42).span(1..3),
            Expr::Tuple(vec![Expr::Int(2).span(5..6)]).span(4..8),
            Expr::String("end".into()).span(9..14)
        ])
        .span(0..15)
    );

    assert_eq!(
        parse_expr("[1, 4, 3, 2]"),
        Expr::Array(vec![
            Expr::Int(1).span(1..2),
            Expr::Int(4).span(4..5),
            Expr::Int(3).span(7..8),
            Expr::Int(2).span(10..11)
        ])
        .span(0..12)
    );

    assert_eq!(parse_expr("foo"), Expr::Ident("foo".into()).span(0..3));
}

#[test]
fn unop_expressions() {
    assert_eq!(
        parse_expr("!  is_visible"),
        Expr::UnaryOp {
            op: Unop::Not,
            expr: Expr::Ident("is_visible".into()).span(3..13).into(),
        }
        .span(0..13)
    );

    assert_eq!(
        parse_expr("-(-13)"),
        Expr::UnaryOp {
            op: Unop::Neg,
            expr: Expr::UnaryOp {
                op: Unop::Neg,
                expr: Expr::Int(13).span(3..5).into(),
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
        Expr::BinaryOp {
            op: Bop::Add,
            lhs: Expr::Int(4).span(0..1).into(),
            rhs: Expr::BinaryOp {
                op: Bop::Mul,
                lhs: Expr::Int(2).span(4..5).into(),
                rhs: Expr::Int(3).span(8..9).into()
            }
            .span(4..9)
            .into()
        }
        .span(0..9)
    );

    assert_eq!(
        parse_expr("4 * 2 + 3"),
        Expr::BinaryOp {
            op: Bop::Add,
            lhs: Expr::BinaryOp {
                op: Bop::Mul,
                lhs: Expr::Int(4).span(0..1).into(),
                rhs: Expr::Int(2).span(4..5).into()
            }
            .span(0..5)
            .into(),
            rhs: Expr::Int(3).span(8..9).into(),
        }
        .span(0..9)
    );

    assert_eq!(
        parse_expr("4 - 2 - 3"),
        Expr::BinaryOp {
            op: Bop::Sub,
            lhs: Expr::BinaryOp {
                op: Bop::Sub,
                lhs: Expr::Int(4).span(0..1).into(),
                rhs: Expr::Int(2).span(4..5).into()
            }
            .span(0..5)
            .into(),
            rhs: Expr::Int(3).span(8..9).into(),
        }
        .span(0..9)
    );

    assert_eq!(
        parse_expr("4 ** 2 ** 3"),
        Expr::BinaryOp {
            op: Bop::Exp,
            lhs: Expr::Int(4).span(0..1).into(),
            rhs: Expr::BinaryOp {
                op: Bop::Exp,
                lhs: Expr::Int(2).span(5..6).into(),
                rhs: Expr::Int(3).span(10..11).into()
            }
            .span(5..11)
            .into()
        }
        .span(0..11)
    );

    assert_eq!(
        parse_expr("4 ^ 2 ^ 3"),
        Expr::BinaryOp {
            op: Bop::Xor,
            lhs: Expr::BinaryOp {
                op: Bop::Xor,
                lhs: Expr::Int(4).span(0..1).into(),
                rhs: Expr::Int(2).span(4..5).into()
            }
            .span(0..5)
            .into(),
            rhs: Expr::Int(3).span(8..9).into(),
        }
        .span(0..9)
    );

    assert_eq!(
        parse_expr("true || false && true"),
        Expr::BinaryOp {
            op: Bop::Or,
            lhs: Expr::Bool(true).span(0..4).into(),
            rhs: Expr::BinaryOp {
                op: Bop::And,
                lhs: Expr::Bool(false).span(8..13).into(),
                rhs: Expr::Bool(true).span(17..21).into(),
            }
            .span(8..21)
            .into()
        }
        .span(0..21)
    );

    assert_eq!(
        parse_expr("3 & 1 | 5"),
        Expr::BinaryOp {
            op: Bop::BOr,
            lhs: Expr::BinaryOp {
                op: Bop::BAnd,
                lhs: Expr::Int(3).span(0..1).into(),
                rhs: Expr::Int(1).span(4..5).into(),
            }
            .span(0..5)
            .into(),
            rhs: Expr::Int(5).span(8..9).into()
        }
        .span(0..9)
    );

    assert_eq!(
        parse_expr("(3 >= 4) != true"),
        Expr::BinaryOp {
            op: Bop::Neq,
            lhs: Expr::BinaryOp {
                op: Bop::Geq,
                lhs: Expr::Int(3).span(1..2).into(),
                rhs: Expr::Int(4).span(6..7).into(),
            }
            .span(0..8)
            .into(),
            rhs: Expr::Bool(true).span(12..16).into()
        }
        .span(0..16)
    );

    assert_eq!(
        parse_expr("(4 > 3) == true"),
        Expr::BinaryOp {
            op: Bop::Eqq,
            lhs: Expr::BinaryOp {
                op: Bop::Gt,
                lhs: Expr::Int(4).span(1..2).into(),
                rhs: Expr::Int(3).span(5..6).into(),
            }
            .span(0..7)
            .into(),
            rhs: Expr::Bool(true).span(11..15).into()
        }
        .span(0..15)
    );
}

#[test]
fn compound_expressions() {
    assert_eq!(
        parse_expr("bar (  x, 2)"),
        Expr::FnCall {
            fun: Expr::Ident("bar".into()).span(0..3).into(),
            args: vec![
                Expr::Ident("x".into()).span(7..8),
                Expr::Int(2).span(10..11),
            ],
        }
        .span(0..12)
    );

    assert_eq!(
        parse_expr("if 0.5 then foo()"),
        Expr::If {
            cond: Expr::Float(0.5).span(3..6).into(),
            th: Expr::FnCall {
                fun: Expr::Ident("foo".into()).span(12..15).into(),
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
        Expr::If {
            cond: Expr::Float(0.5).span(3..6).into(),
            th: Expr::Ident("foo".into()).span(12..15).into(),
            el: Some(Expr::Ident("bar".into()).span(21..24).into())
        }
        .span(0..24)
    );

    assert_eq!(
        parse_expr("if a then if b then x else y"),
        Expr::If {
            cond: Expr::Ident("a".into()).span(3..4).into(),
            th: Expr::If {
                cond: Expr::Ident("b".into()).span(13..14).into(),
                th: Expr::Ident("x".into()).span(20..21).into(),
                el: Some(Expr::Ident("y".into()).span(27..28).into())
            }
            .span(10..28)
            .into(),
            el: None
        }
        .span(0..28)
    );

    assert_eq!(
        parse_expr("(fn(a, b: Int) -> a + b)(1, 2)"),
        Expr::FnCall {
            fun: Expr::Lambda {
                params: vec![
                    Pattern::Var {
                        mutable: false,
                        ident: "a".into(),
                        annotated_ty: None
                    }
                    .span(4..5),
                    Pattern::Var {
                        mutable: false,
                        ident: "b".into(),
                        annotated_ty: Some(Type::Int.span(10..13))
                    }
                    .span(7..13)
                ],
                return_type: None,
                body: Expr::BinaryOp {
                    op: Bop::Add,
                    lhs: Expr::Ident("a".into()).span(18..19).into(),
                    rhs: Expr::Ident("b".into()).span(22..23).into()
                }
                .span(18..23)
                .into()
            }
            .span(0..24)
            .into(),
            args: vec![
                Expr::Int(1).span(25..26).into(),
                Expr::Int(2).span(28..29).into()
            ]
        }
        .span(0..30)
    );

    assert_eq!(
        parse_expr("[1, 2, 3][1-1]"),
        Expr::Index {
            arr: Expr::Array(vec![
                Expr::Int(1).span(1..2),
                Expr::Int(2).span(4..5),
                Expr::Int(3).span(7..8)
            ])
            .span(0..9)
            .into(),
            index: Expr::BinaryOp {
                op: Bop::Sub,
                lhs: Expr::Int(1).span(10..11).into(),
                rhs: Expr::Int(1).span(12..13).into()
            }
            .span(10..13)
            .into()
        }
        .span(0..14)
    );

    assert_eq!(
        parse_expr("self._0"),
        Expr::FieldAccess {
            base: Expr::Ident("self".into()).span(0..4).into(),
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
        Expr::Let {
            binding: Pattern::Var {
                mutable: false,
                ident: "x".into(),
                annotated_ty: None
            }
            .span(4..5),
            value: Expr::BinaryOp {
                op: Bop::Add,
                lhs: Expr::Int(7).span(8..9).into(),
                rhs: Expr::FnCall {
                    fun: Expr::Ident("sin".into()).span(12..15).into(),
                    args: vec![Expr::Float(3.0).span(16..18)]
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
        Expr::Let {
            binding: Pattern::Var {
                mutable: true,
                ident: "y".into(),
                annotated_ty: Some(Type::UInt.span(11..15))
            }
            .span(4..15),
            value: Expr::Int(7).span(18..19).into()
        }
        .span(0..19)
    );

    assert_eq!(
        parse_expr("y = 3 + 7 * 0.5"),
        Expr::Assign {
            ident: Spnd {
                inner: "y".into(),
                span: (0..1).into()
            },
            value: Expr::BinaryOp {
                op: Bop::Add,
                lhs: Expr::Int(3).span(4..5).into(),
                rhs: Expr::BinaryOp {
                    op: Bop::Mul,
                    lhs: Expr::Int(7).span(8..9).into(),
                    rhs: Expr::Float(0.5).span(12..15).into()
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
    else 32;
",
    );
    assert_eq!(
        expr,
        Expr::Block {
            exprs: vec![
                Expr::Let {
                    binding: Pattern::Var {
                        mutable: true,
                        ident: "y".into(),
                        annotated_ty: None
                    }
                    .span(9..14),
                    value: Expr::Int(5).span(17..18).into()
                }
                .span(5..18),
                Expr::BinaryOp {
                    op: Bop::Sub,
                    lhs: Expr::BinaryOp {
                        op: Bop::Add,
                        lhs: Expr::Int(3).span(23..24).into(),
                        rhs: Expr::Int(1).span(27..28).into()
                    }
                    .span(23..28)
                    .into(),
                    rhs: Expr::Int(2).span(31..32).into()
                }
                .span(23..32),
                Expr::Assign {
                    ident: Spnd {
                        inner: "y".into(),
                        span: (37..38).into()
                    },
                    value: Expr::Int(1).span(41..42).into()
                }
                .span(37..42),
                Expr::If {
                    cond: Expr::BinaryOp {
                        op: Bop::Lt,
                        lhs: Expr::Ident("y".into()).span(50..51).into(),
                        rhs: Expr::Int(3).span(54..55).into()
                    }
                    .span(50..55)
                    .into(),
                    th: Expr::Block {
                        exprs: vec![
                            Expr::Let {
                                binding: Pattern::Var {
                                    mutable: false,
                                    ident: "a".into(),
                                    annotated_ty: None
                                }
                                .span(73..74),
                                value: Expr::Int(5).span(77..78).into()
                            }
                            .span(69..78),
                            Expr::Ident("a".to_string()).span(87..88)
                        ],
                        trailing: true
                    }
                    .span(61..93)
                    .into(),
                    el: Some(Expr::Int(32).span(98..100).into())
                }
                .span(47..100)
            ],
            trailing: false
        }
        .span(1..102)
    );
}

#[test]
fn malformed_expressions() {
    assert_eq!(
        parse_expr_err("[1, 3, 4, 5"),
        Err(ParseError::Mismatched {
            expected: TT::RBracket,
            found: TT::Eof
        }
        .span(11..11))
    );
    assert_eq!(
        parse_expr_err("*5"),
        Err(ParseError::Unexpected(TT::Times, "start of expression").span(0..1))
    );
    assert_eq!(
        parse_expr_err("print(5, 2;)"),
        Err(ParseError::Mismatched {
            expected: TT::RParen,
            found: TT::Semicolon
        }
        .span(10..11))
    );
}
