use pretty_assertions::assert_eq;

use ast::{
    exprs::{Arg, Binding, Expr, ExprKind, InfixOp, Stmt},
    items::{ExecItem, ExecKind, Param, Return},
    patterns::PatKind,
    types::{Param as ParamTy, TyKind},
};
use ident::Ident;
use parse::Parser;
use smallvec::smallvec;
use span::Span;

use crate::{AdtId, NameTable, Result, Scope, VarId, VarInfo, resolve, resolve_expr};

fn test_resolve_expr(input: &str) -> Result<(Expr<(), AdtId, VarId>, NameTable)> {
    let expr = Parser::parse_expr(input).unwrap();

    let mut table = NameTable::default();
    let expr = resolve_expr(&mut table, &Scope::default(), &Scope::default(), expr)?;
    Ok((expr, table))
}

fn test_resolve_full(input: &str) -> Result<(Vec<ExecItem<(), AdtId, VarId>>, NameTable)> {
    resolve(Parser::parse(lex::lex(input).unwrap()).unwrap())
}

#[test]
fn shadowing() {
    let input = r#"{ 
    let a: UInt = 5 
    let a = "Hello, World" 
    {let a = true} 
    a 
}"#;

    let mut expected_table = NameTable::default();
    let id_1 = expected_table.insert_var(VarInfo {
        ident: Ident::new("a"),
        mutable: false,
        ty: Some(TyKind::UInt.span(14..18)),
        span: Span::from(11..12),
    });
    let id_2 = expected_table.insert_var(VarInfo {
        ident: Ident::new("a"),
        mutable: false,
        ty: None,
        span: Span::from(32..33),
    });
    let id_3 = expected_table.insert_var(VarInfo {
        ident: Ident::new("a"),
        mutable: false,
        ty: None,
        span: Span::from(61..62),
    });

    assert_eq!(
        test_resolve_expr(input),
        Ok((
            ExprKind::Block(vec![
                Stmt::Decl {
                    binding: Binding {
                        mutable: false,
                        pat: PatKind::Ident(id_1).span(11..12),
                        ty: Some(TyKind::UInt.span(14..18))
                    },
                    val: ExprKind::int(5).span(21..22).into(),
                    span: Span::from(7..22)
                },
                Stmt::Decl {
                    binding: Binding {
                        mutable: false,
                        pat: PatKind::Ident(id_2).span(32..33),
                        ty: None
                    },
                    val: ExprKind::string("Hello, World").span(36..50).into(),
                    span: Span::from(28..50)
                },
                Stmt::Expr(
                    ExprKind::Block(vec![Stmt::Decl {
                        binding: Binding {
                            mutable: false,
                            pat: PatKind::Ident(id_3).span(61..62),
                            ty: None
                        },
                        val: ExprKind::bool(true).span(65..69).into(),
                        span: Span::from(57..69)
                    },])
                    .span(56..70)
                ),
                Stmt::Expr(ExprKind::ident_id(id_2).span(76..77))
            ])
            .span(0..80),
            expected_table
        ))
    );
}

#[test]
fn fib() {
    let input = "
    fn fib(n: UInt): UInt -> 
        if n <= 1 then
            n
        else 
            fib(n - 1) + fib(n - 2)
";

    let mut expected_table = NameTable::default();
    let fib = expected_table.insert_var(VarInfo {
        ident: Ident::new("fib"),
        mutable: false,
        ty: Some(
            TyKind::Fn {
                params: vec![ParamTy {
                    mutable: false,
                    ty: TyKind::UInt.span(15..19),
                }],
                result: TyKind::UInt.span(22..26).into(),
            }
            .span(11..26),
        ),
        span: Span::from(8..11),
    });
    let n = expected_table.insert_var(VarInfo {
        ident: Ident::new("n"),
        mutable: false,
        ty: Some(TyKind::UInt.span(15..19)),
        span: Span::from(12..13),
    });

    assert_eq!(
        test_resolve_full(input),
        Ok((
            vec![ExecItem {
                ident: fib,
                ident_span: Span::from(8..11),
                kind: ExecKind::Func {
                    generics: smallvec![],
                    params: vec![Param {
                        mutable: false,
                        pat: PatKind::ident_id(n).span(12..13),
                        ty: TyKind::UInt.span(15..19)
                    }],
                    result: Return {
                        mutable: false,
                        ty: TyKind::UInt.span(22..26).into()
                    },
                    body: ExprKind::If {
                        cond: ExprKind::Infix {
                            op: InfixOp::Leq,
                            lhs: ExprKind::ident_id(n).span(42..43).into(),
                            rhs: ExprKind::int(1).span(47..48).into()
                        }
                        .span(42..48)
                        .into(),
                        th: ExprKind::ident_id(n).span(66..67).into(),
                        el: Some(
                            ExprKind::Infix {
                                op: InfixOp::Sub,
                                lhs: ExprKind::Call {
                                    func: ExprKind::ident_id(fib).span(94..97).into(),
                                    args: vec![Arg {
                                        mutable: false,
                                        val: ExprKind::Infix {
                                            op: InfixOp::Sub,
                                            lhs: ExprKind::ident_id(n).span(98..99).into(),
                                            rhs: ExprKind::int(1).span(102..103).into()
                                        }
                                        .span(98..103)
                                    }]
                                }
                                .span(94..104)
                                .into(),
                                rhs: ExprKind::Call {
                                    func: ExprKind::ident_id(fib).span(107..110).into(),
                                    args: vec![Arg {
                                        mutable: false,
                                        val: ExprKind::Infix {
                                            op: InfixOp::Sub,
                                            lhs: ExprKind::ident_id(n).span(111..112).into(),
                                            rhs: ExprKind::int(2).span(115..116).into()
                                        }
                                        .span(111..116)
                                    }]
                                }
                                .span(107..117)
                                .into()
                            }
                            .span(94..117)
                            .into()
                        )
                    }
                    .span(39..117)
                }
            }],
            expected_table
        ))
    );
}
