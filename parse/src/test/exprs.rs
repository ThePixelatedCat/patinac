use ast::{Binding, Bop, Expr, ExprKind, Pat, Ty, TyKind, Unop};
use span::{Span, Spannable, Spnd};
use lex::TokKind;

use crate::{ParseError, ParseResult, Parser};

fn parse_expr(input: &str) -> ParseResult<Expr> {
    let mut interner = Default::default();
    let mut parser = Parser::new(input, &mut interner);
    parser.expr()
}

#[test]
fn lit_expressions() {
    assert_eq!(parse_expr("42"), Ok(ExprKind::Int(42).span(0..2)));

    assert_eq!(
        parse_expr("  2.7768"),
        Ok(ExprKind::Float(2.7768).span(2..8))
    );

    assert_eq!(
        parse_expr(r#""I am a Str!""#),
        Ok(ExprKind::String(String::from("I am a Str!")).span(0..13))
    );

    assert_eq!(parse_expr(r"'\''"), Ok(ExprKind::Char('\'').span(0..4)));

    assert_eq!(
        parse_expr(r#"(42,(2,),"end")"#),
        Ok(ExprKind::Tuple(vec![
            ExprKind::Int(42).span(1..3),
            ExprKind::Tuple(vec![ExprKind::Int(2).span(5..6)]).span(4..8),
            ExprKind::String(String::from("end")).span(9..14)
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
            ExprKind::Int(1).span(7..8),
            ExprKind::Block(vec![ExprKind::Int(4).span(18..19)]).span(10..24),
            ExprKind::Int(3).span(30..31),
            ExprKind::Int(2).span(37..38)
        ])
        .span(1..40))
    );

    let mut interner = Default::default();
    let mut parser = Parser::new("foo", &mut interner);
    assert_eq!(
        parser.expr(),
        Ok(ExprKind::Ident(parser.get_ident("foo").unwrap()).span(0..3))
    );
}

#[test]
fn unop_expressions() {
    let mut interner = Default::default();
    let mut parser = Parser::new("!  is_visible", &mut interner);
    assert_eq!(
        parser.expr(),
        Ok(ExprKind::UnaryOp {
            op: Unop::Not,
            expr: ExprKind::Ident(parser.get_ident("is_visible").unwrap())
                .span(3..13)
                .into(),
        }
        .span(0..13))
    );

    assert_eq!(
        parse_expr("-(-13)"),
        Ok(ExprKind::UnaryOp {
            op: Unop::Neg,
            expr: ExprKind::UnaryOp {
                op: Unop::Neg,
                expr: ExprKind::Int(13).span(3..5).into(),
            }
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
        Ok(ExprKind::BinaryOp {
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
        .span(0..9))
    );

    assert_eq!(
        parse_expr("4 * 2 + 3"),
        Ok(ExprKind::BinaryOp {
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
        .span(0..9))
    );

    assert_eq!(
        parse_expr("4 - 2 - 3"),
        Ok(ExprKind::BinaryOp {
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
        .span(0..9))
    );

    assert_eq!(
        parse_expr("4 ** 2 ** 3"),
        Ok(ExprKind::BinaryOp {
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
        .span(0..11))
    );

    assert_eq!(
        parse_expr("4 ^ 2 ^ 3"),
        Ok(ExprKind::BinaryOp {
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
        .span(0..9))
    );

    assert_eq!(
        parse_expr("true || false && true"),
        Ok(ExprKind::BinaryOp {
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
        .span(0..21))
    );

    assert_eq!(
        parse_expr("3 & 1 | 5"),
        Ok(ExprKind::BinaryOp {
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
        .span(0..9))
    );

    assert_eq!(
        parse_expr("(3 >= 4) != true"),
        Ok(ExprKind::BinaryOp {
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
        .span(0..16))
    );

    assert_eq!(
        parse_expr("(4 > 3) == true"),
        Ok(ExprKind::BinaryOp {
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
        .span(0..15))
    );
}

#[test]
fn compound_expressions() {
    let mut interner = Default::default();

    let mut parser = Parser::new("bar (  x, 2)", &mut interner);
    assert_eq!(
        parser.expr(),
        Ok(ExprKind::FnCall {
            fun: ExprKind::Ident(parser.get_ident("bar").unwrap())
                .span(0..3)
                .into(),
            args: vec![
                ExprKind::Ident(parser.get_ident("x").unwrap()).span(7..8),
                ExprKind::Int(2).span(10..11),
            ],
        }
        .span(0..12))
    );

    let mut parser = Parser::new("if 0.5 then foo()", &mut interner);
    assert_eq!(
        parser.expr(),
        Ok(ExprKind::If {
            cond: ExprKind::Float(0.5).span(3..6).into(),
            th: ExprKind::FnCall {
                fun: ExprKind::Ident(parser.get_ident("foo").unwrap())
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
            cond: ExprKind::Float(0.5).span(3..6).into(),
            th: ExprKind::Ident(parser.get_ident("foo").unwrap())
                .span(12..15)
                .into(),
            el: Some(
                ExprKind::Ident(parser.get_ident("bar").unwrap())
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
            cond: ExprKind::Ident(parser.get_ident("a").unwrap())
                .span(3..4)
                .into(),
            th: ExprKind::If {
                cond: ExprKind::Ident(parser.get_ident("b").unwrap())
                    .span(13..14)
                    .into(),
                th: ExprKind::Ident(parser.get_ident("x").unwrap())
                    .span(20..21)
                    .into(),
                el: Some(
                    ExprKind::Ident(parser.get_ident("y").unwrap())
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

    let mut parser = Parser::new("(fn(a, b: Int) -> a + b)(1, 2)", &mut interner);
    assert_eq!(
        parser.expr(),
        Ok(ExprKind::FnCall {
            fun: ExprKind::Lambda {
                params: vec![
                    Binding {
                        pat: Pat::Var {
                            mutable: false,
                            ident: parser.get_ident("a").unwrap(),
                        },
                        ty: None
                    },
                    Binding {
                        pat: Pat::Var {
                            mutable: false,
                            ident: parser.get_ident("b").unwrap(),
                        },
                        ty: Some(Ty {
                            kind: TyKind::Int,
                            span: Span::from(10..13)
                        })
                    }
                ],
                return_ty: None,
                body: ExprKind::BinaryOp {
                    op: Bop::Add,
                    lhs: ExprKind::Ident(parser.get_ident("a").unwrap())
                        .span(18..19)
                        .into(),
                    rhs: ExprKind::Ident(parser.get_ident("b").unwrap())
                        .span(22..23)
                        .into()
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
        .span(0..30))
    );

    assert_eq!(
        parse_expr("[1, 2, 3][1-1]"),
        Ok(ExprKind::Index {
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
        .span(0..14))
    );

    let mut parser = Parser::new("self._0", &mut interner);
    assert_eq!(
        parser.expr(),
        Ok(ExprKind::FieldAccess {
            base: ExprKind::Ident(parser.get_ident("self").unwrap())
                .span(0..4)
                .into(),
            field: Spnd(parser.get_ident("_0").unwrap(), (5..7).into())
        }
        .span(0..7))
    );
}

#[test]
fn var_expressions() {
    let mut interner = Default::default();

    let mut parser = Parser::new("let x = 7 + sin(3.)", &mut interner);
    assert_eq!(
        parser.expr(),
        Ok(ExprKind::Let {
            binding: Binding {
                pat: Pat::Var {
                    mutable: false,
                    ident: parser.get_ident("x").unwrap(),
                },
                ty: None
            },
            value: ExprKind::BinaryOp {
                op: Bop::Add,
                lhs: ExprKind::Int(7).span(8..9).into(),
                rhs: ExprKind::FnCall {
                    fun: ExprKind::Ident(parser.get_ident("sin").unwrap())
                        .span(12..15)
                        .into(),
                    args: vec![ExprKind::Float(3.0).span(16..18)]
                }
                .span(12..19)
                .into()
            }
            .span(8..19)
            .into()
        }
        .span(0..19))
    );

    let mut parser = Parser::new("let mut y: UInt = 7", &mut interner);
    assert_eq!(
        parser.expr(),
        Ok(ExprKind::Let {
            binding: Binding {
                pat: Pat::Var {
                    mutable: true,
                    ident: parser.get_ident("y").unwrap(),
                },
                ty: Some(Ty {
                    kind: TyKind::UInt,
                    span: Span::from(11..15)
                })
            },
            value: ExprKind::Int(7).span(18..19).into()
        }
        .span(0..19))
    );

    let mut parser = Parser::new("y = 3 + 7 * 0.5", &mut interner);
    assert_eq!(
        parser.expr(),
        Ok(ExprKind::Assign {
            ident: Spnd(parser.get_ident("y").unwrap(), (0..1).into()),
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
        .span(0..15))
    );
}

#[test]
fn block_expressions() {
    let mut interner = Default::default();

    let input = "
    let mut y = 5
    3 + 1 - 2
    y = 1
    if y < 3 then
        let a = 5
        a
    else 32
";
    let mut parser = Parser::new(input, &mut interner);
    assert_eq!(
        parser.expr(),
        Ok(ExprKind::Block(vec![
            ExprKind::Let {
                binding: Binding {
                    pat: Pat::Var {
                        mutable: true,
                        ident: parser.get_ident("y").unwrap(),
                    },
                    ty: None
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
                ident: Spnd(parser.get_ident("y").unwrap(), (37..38).into()),
                value: ExprKind::Int(1).span(41..42).into()
            }
            .span(37..42),
            ExprKind::If {
                cond: ExprKind::BinaryOp {
                    op: Bop::Lt,
                    lhs: ExprKind::Ident(parser.get_ident("y").unwrap())
                        .span(50..51)
                        .into(),
                    rhs: ExprKind::Int(3).span(54..55).into()
                }
                .span(50..55)
                .into(),
                th: ExprKind::Block(vec![
                    ExprKind::Let {
                        binding: Binding {
                            pat: Pat::Var {
                                mutable: false,
                                ident: parser.get_ident("a").unwrap(),
                            },
                            ty: None
                        },
                        value: ExprKind::Int(5).span(77..78).into()
                    }
                    .span(69..78),
                    ExprKind::Ident(parser.get_ident("a").unwrap()).span(87..88)
                ])
                .span(61..93)
                .into(),
                el: Some(ExprKind::Int(32).span(98..100).into())
            }
            .span(47..100)
        ])
        .span(1..101))
    );
}

#[test]
fn malformed_expressions() {
    assert_eq!(
        parse_expr("[1, 3, 4, 5"),
        Err(ParseError::Mismatched {
            expected: TokKind::RBracket,
            found: TokKind::Eof
        }
        .span(11..11))
    );
    assert_eq!(
        parse_expr("*5"),
        Err(ParseError::Unexpected(TokKind::Times, "start of expression").span(0..1))
    );
}
