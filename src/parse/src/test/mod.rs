mod lex;

use std::{assert_matches, range::Range};

use itertools::Itertools as _;
use pretty_assertions::assert_eq;
use proptest::{collection::vec, prelude::*};

use errors::ErrorHandler;
use irs::ModuleId;

use irs::ast::{Arg, BlockExpr, ExprKind, InfixOp, PrefixOp, Stmt};

use crate::{Parser, TokKind};

proptest! {
    #[test]
    fn doesnt_crash(toks in vec(TokKind::arbitrary(), 8..=512)) {
        let raw = toks.iter().map(|t| t.reverse()).join(" ");
        let _ = Parser::new(ModuleId::default(), &raw, ErrorHandler::DUMMY).parse();
    }
}

#[test]
fn nested_tuples() {
    assert_eq!(
        Parser::new_test(r#"(42,(2),"end")"#).expr(),
        Ok(ExprKind::Tuple(vec![
            ExprKind::int(42).span(1..3),
            ExprKind::Tuple(vec![ExprKind::int(2).span(5..6)]).span(4..7),
            ExprKind::string("end").span(8..13)
        ])
        .span(0..14))
    );
}

#[test]
fn numeric_literals() {
    assert_eq!(
        Parser::new_test("0x10").expr(),
        Ok(ExprKind::int(16).span(0..4))
    );

    assert_eq!(
        Parser::new_test("-0b100").expr(),
        Ok(ExprKind::Prefix {
            op: PrefixOp::Neg,
            expr: ExprKind::int(4).span(1..6).into()
        }
        .span(0..6))
    );

    assert_eq!(
        Parser::new_test("-2.7768e10").expr(),
        Ok(ExprKind::Prefix {
            op: PrefixOp::Neg,
            expr: ExprKind::float(2.7768e10).span(1..10).into()
        }
        .span(0..10))
    );
}

#[test]
#[expect(clippy::needless_raw_string_hashes, reason = "false positive")]
fn strings() {
    assert_eq!(
        Parser::new_test(r#""I am a String!\n""#).expr(),
        Ok(ExprKind::string("I am a String!\n").span(0..18))
    );

    assert_eq!(
        Parser::new_test(r##"#""I am a String!\n""#"##).expr(),
        Ok(ExprKind::string(r#""I am a String!\n""#).span(0..22))
    );

    assert_eq!(
        Parser::new_test(r#""\"""#).expr(),
        Ok(ExprKind::string("\"").span(0..4))
    );

    assert_eq!(
        Parser::new_test(r#""\\n""#).expr(),
        Ok(ExprKind::string("\\n").span(0..5))
    );

    assert_eq!(
        Parser::new_test(r#""\u{1f308}""#).expr(),
        Ok(ExprKind::string("🌈").span(0..11))
    );
}

#[allow(clippy::too_many_lines, reason = "It's a test function")]
#[test]
fn precedence() {
    assert_eq!(
        Parser::new_test("4 + 2 * 3").expr(),
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
        Parser::new_test("4.0 *. 2.0 +. 3.0").expr(),
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
        Parser::new_test("4 - 2 - 3").expr(),
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
        Parser::new_test("4 ^ 2 ^ 3").expr(),
        Ok(ExprKind::Infix {
            op: InfixOp::Exp,
            lhs: ExprKind::int(4).span(0..1).into(),
            rhs: ExprKind::Infix {
                op: InfixOp::Exp,
                lhs: ExprKind::int(2).span(4..5).into(),
                rhs: ExprKind::int(3).span(8..9).into()
            }
            .span(4..9)
            .into()
        }
        .span(0..9))
    );

    assert_eq!(
        Parser::new_test("true || false && true").expr(),
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
        Parser::new_test("{3 >= 4} != true").expr(),
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
        Parser::new_test("{4 > 3} == true").expr(),
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
fn tuple_or_call() {
    assert_eq!(
        Parser::new_test("{foo()}").expr(),
        Ok(ExprKind::Block(
            ExprKind::Call {
                func: ExprKind::ident("foo").span(1..4).into(),
                args: vec![]
            }
            .span(1..6)
            .as_block(0..7)
        )
        .span(0..7))
    );
    assert_eq!(
        Parser::new_test("{foo ()}").expr(),
        Ok(ExprKind::Block(BlockExpr {
            stmts: vec![
                Stmt::Expr(ExprKind::ident("foo").span(1..4)),
                Stmt::Expr(ExprKind::Tuple(vec![]).span(5..7))
            ],
            span: Range::from(0..8)
        })
        .span(0..8))
    );
    assert_eq!(
        Parser::new_test("{foo(mut 1)}").expr(),
        Ok(ExprKind::Block(
            ExprKind::Call {
                func: ExprKind::ident("foo").span(1..4).into(),
                args: vec![Arg {
                    value: ExprKind::int(1).span(9..10),
                    mutable: true,
                    span: Range::from(5..10)
                }]
            }
            .span(1..11)
            .as_block(0..12)
        )
        .span(0..12))
    );
    assert_matches!(Parser::new_test("{foo (mut 1)}").expr(), Err(_));
}

#[test]
fn if_whitespace() {
    assert_eq!(
        Parser::new_test("{ if foo { bar } () }").expr(),
        Ok(ExprKind::Block(BlockExpr {
            stmts: vec![
                Stmt::Expr(
                    ExprKind::If {
                        cond: ExprKind::ident("foo").span(5..8).into(),
                        th: ExprKind::ident("bar").span(11..14).as_block(9..16),
                        el: None
                    }
                    .span(2..16)
                ),
                Stmt::Expr(ExprKind::Tuple(vec![]).span(17..19))
            ],
            span: Range::from(0..21)
        })
        .span(0..21))
    );

    assert_eq!(
        Parser::new_test("{ if foo { bar }() }").expr(),
        Ok(ExprKind::Block(
            ExprKind::Call {
                func: ExprKind::If {
                    cond: ExprKind::ident("foo").span(5..8).into(),
                    th: ExprKind::ident("bar").span(11..14).as_block(9..16),
                    el: None
                }
                .span(2..16)
                .into(),
                args: vec![]
            }
            .span(2..18)
            .as_block(0..20)
        )
        .span(0..20))
    );
}
