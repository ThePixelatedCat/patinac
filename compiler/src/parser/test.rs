use super::Parser;
use super::ast::{Bop, Expr, Lit, Stmt, Unop};

fn parse_expr(input: &str) -> Expr {
    let mut parser = Parser::new(input);
    parser.expression().unwrap()
}

fn parse_stmt(input: &str) -> Stmt {
    let mut parser = Parser::new(input);
    parser.statement().unwrap()
}

#[test]
fn parse_expression() {
    let expr = parse_expr("42");
    assert_eq!(expr, Expr::Literal(Lit::Int(42)));

    let expr = parse_expr("  2.7768");
    assert_eq!(expr, Expr::Literal(Lit::Float(2.7768)));

    let expr = parse_expr(r#""I am a String!""#);
    assert_eq!(expr, Expr::Literal(Lit::Str("I am a String!".into())));

    let expr = parse_expr("foo");
    assert_eq!(expr, Expr::Ident("foo".to_string()));

    let expr = parse_expr("bar (  x, 2)");
    assert_eq!(
        expr,
        Expr::FnCall {
            fun: Expr::Ident("bar".into()).into(),
            args: vec![Expr::Ident("x".into()), Lit::Int(2).into(),],
        }
    );

    let expr = parse_expr("!  is_visible");
    assert_eq!(
        expr,
        Expr::UnaryOp {
            op: Unop::Not,
            expr: Expr::Ident("is_visible".to_string()).into(),
        }
    );

    let expr = parse_expr("-(-13)");
    assert_eq!(
        expr,
        Expr::UnaryOp {
            op: Unop::Neg,
            expr: Lit::Int(-13).into(),
        }
    );

    let expr = parse_expr("if (0.5) foo()");
    assert_eq!(
        expr,
        Expr::If {
            cond: Lit::Float(0.5).into(),
            th: Expr::FnCall {
                fun: Expr::Ident("foo".into()).into(),
                args: Vec::new()
            }
            .into(),
            el: None
        }
    );

    let expr = parse_expr("if (0.5) foo else bar");
    assert_eq!(
        expr,
        Expr::If {
            cond: Lit::Float(0.5).into(),
            th: Expr::Ident("foo".into()).into(),
            el: Some(Expr::Ident("bar".into()).into())
        }
    );
}

#[test]
fn parse_binary_expressions() {
    let expr = parse_expr("4 + 2 * 3");
    assert_eq!(
        expr,
        Expr::BinaryOp {
            op: Bop::Add,
            lhs: Lit::Int(4).into(),
            rhs: Box::new(Expr::BinaryOp {
                op: Bop::Mul,
                lhs: Lit::Int(2).into(),
                rhs: Lit::Int(3).into()
            })
        }
    );

    let expr = parse_expr("4 * 2 + 3");
    assert_eq!(
        expr,
        Expr::BinaryOp {
            op: Bop::Add,
            lhs: Box::new(Expr::BinaryOp {
                op: Bop::Mul,
                lhs: Lit::Int(4).into(),
                rhs: Lit::Int(2).into()
            }),
            rhs: Lit::Int(3).into(),
        }
    );

    let expr = parse_expr("4 - 2 - 3");
    assert_eq!(
        expr,
        Expr::BinaryOp {
            op: Bop::Sub,
            lhs: Box::new(Expr::BinaryOp {
                op: Bop::Sub,
                lhs: Lit::Int(4).into(),
                rhs: Lit::Int(2).into()
            }),
            rhs: Lit::Int(3).into(),
        }
    );

    let expr = parse_expr("4 ^ 2 ^ 3");
    assert_eq!(
        expr,
        Expr::BinaryOp {
            op: Bop::Exp,
            lhs: Lit::Int(4).into(),
            rhs: Box::new(Expr::BinaryOp {
                op: Bop::Exp,
                lhs: Lit::Int(2).into(),
                rhs: Lit::Int(3).into()
            })
        }
    );
}

#[test]
fn parse_statements() {
    let stmt = parse_stmt("let x = 7 + sin(3.);");
    assert_eq!(
        stmt,
        Stmt::Let {
            mutable: false,
            ident: "x".into(),
            type_annotation: None,
            value: Expr::BinaryOp {
                op: Bop::Add,
                lhs: Lit::Int(7).into(),
                rhs: Expr::FnCall {
                    fun: Expr::Ident("sin".into()).into(),
                    args: vec![Lit::Float(3.0).into()]
                }
                .into()
            }
        }
    );

    let stmt = parse_stmt("let mut y: Int = 7;");
    assert_eq!(
        stmt,
        Stmt::Let {
            mutable: true,
            ident: "y".into(),
            type_annotation: Some("Int".into()),
            value: Lit::Int(7).into()
        }
    );

    let stmt = parse_stmt("y = 3 + 7 * 0.5;");
    assert_eq!(
        stmt,
        Stmt::Assign {
            ident: "y".into(),
            value: Expr::BinaryOp {
                op: Bop::Add,
                lhs: Lit::Int(3).into(),
                rhs: Expr::BinaryOp {
                    op: Bop::Mul,
                    lhs: Lit::Int(7).into(),
                    rhs: Lit::Float(0.5).into()
                }
                .into()
            }
        }
    );

    let stmt = parse_stmt("1;");
    assert_eq!(stmt, Stmt::Expr(Lit::Int(1).into()));
}
