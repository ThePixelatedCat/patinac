use pretty_assertions::assert_eq;

use ast::{
    exprs::{Arg, Binding, Expr, ExprKind, InfixOp, LitExpr, MatchArm, Stmt},
    items::{AdtItem, AdtKind, ExecItem, ExecKind, Field, Param, Return, Variant},
    patterns::PatKind,
    types::{Param as ParamTy, TyKind},
};
use ident::Ident;
use parse::Parser;
use smallvec::smallvec;
use span::Span;

use crate::{
    AdtId, AdtInfo, NameTable, Result, Scope, VarId, VarInfo,
    error::ErrorKind,
    resolve, resolve_expr,
    table::{AdtTable, PartialAdtTable, PartialVarTable, VarTable},
};

fn test_resolve_expr(input: &str) -> Result<(Expr<(), AdtId, VarId>, NameTable)> {
    let expr = Parser::parse_expr(input).unwrap();

    let adt_table = AdtTable::default();
    let mut var_table = PartialVarTable::default();
    let expr = resolve_expr(
        &adt_table,
        &Scope::default(),
        &mut var_table,
        &Scope::default(),
        expr,
    )?;
    Ok((
        expr,
        NameTable {
            adts: adt_table,
            vars: var_table.finalise(),
        },
    ))
}

fn test_resolve_full(input: &str) -> Result<(Vec<ExecItem<(), AdtId, VarId>>, NameTable)> {
    resolve(Parser::parse(lex::lex(input).unwrap()).unwrap())
}

#[test]
fn lambda() {
    let input = "{
    let mut a = 5
    let b = 6
    fn(c) -> a + b + c
}";

    let mut expected_table = NameTable::default();
    let a = expected_table.vars.insert(VarInfo {
        ident: Ident::new("a"),
        mutable: true,
        ty: None,
        span: Span::from(14..15),
    });
    let b = expected_table.vars.insert(VarInfo {
        ident: Ident::new("b"),
        mutable: false,
        ty: None,
        span: Span::from(28..29),
    });
    let a_immut = expected_table.vars.insert(VarInfo {
        ident: Ident::new("a"),
        mutable: false,
        ty: None,
        span: Span::from(14..15),
    });
    let c = expected_table.vars.insert(VarInfo {
        ident: Ident::new("c"),
        mutable: false,
        ty: None,
        span: Span::from(41..42),
    });

    assert_eq!(
        test_resolve_expr(input),
        Ok((
            ExprKind::Block(vec![
                Stmt::Decl {
                    binding: Binding {
                        mutable: true,
                        pat: PatKind::Ident(a).span(14..15),
                        ty: None
                    },
                    val: ExprKind::int(5).span(18..19),
                    span: Span::from(6..19)
                },
                Stmt::Decl {
                    binding: Binding {
                        mutable: false,
                        pat: PatKind::Ident(b).span(28..29),
                        ty: None
                    },
                    val: ExprKind::int(6).span(32..33),
                    span: Span::from(24..33)
                },
                Stmt::Expr(
                    ExprKind::Lambda {
                        params: vec![Binding {
                            mutable: false,
                            pat: PatKind::Ident(c).span(41..42),
                            ty: None
                        }],
                        return_ty: None,
                        body: ExprKind::Infix {
                            op: InfixOp::Add,
                            lhs: ExprKind::Infix {
                                op: InfixOp::Add,
                                lhs: ExprKind::ident_id(a_immut).span(47..48).into(),
                                rhs: ExprKind::ident_id(b).span(51..52).into(),
                            }
                            .span(47..52)
                            .into(),
                            rhs: ExprKind::ident_id(c).span(55..56).into(),
                        }
                        .span(47..56)
                        .into()
                    }
                    .span(38..56)
                )
            ])
            .span(0..58),
            expected_table
        ))
    );
}

#[test]
fn match_() {
    let input = "
    match 5 with
    | x -> x
    | -1 -> 0
    | #(a, b) -> #(b, a)
";

    let mut expected_table = NameTable::default();
    let x = expected_table.vars.insert(VarInfo {
        ident: Ident::new("x"),
        mutable: false,
        ty: None,
        span: Span::from(24..25),
    });
    let a = expected_table.vars.insert(VarInfo {
        ident: Ident::new("a"),
        mutable: false,
        ty: None,
        span: Span::from(53..54),
    });
    let b = expected_table.vars.insert(VarInfo {
        ident: Ident::new("b"),
        mutable: false,
        ty: None,
        span: Span::from(56..57),
    });

    assert_eq!(
        test_resolve_expr(input),
        Ok((
            ExprKind::Match {
                scrutinee: ExprKind::int(5).span(11..12).into(),
                arms: vec![
                    MatchArm {
                        pat: PatKind::Ident(x).span(24..25),
                        body: ExprKind::ident_id(x).span(29..30),
                        span: Span::from(22..30)
                    },
                    MatchArm {
                        pat: PatKind::Literal {
                            negate: true,
                            lit: LitExpr::Int(1)
                        }
                        .span(37..39),
                        body: ExprKind::int(0).span(43..44),
                        span: Span::from(35..44)
                    },
                    MatchArm {
                        pat: PatKind::Tuple(vec![
                            PatKind::Ident(a).span(53..54),
                            PatKind::Ident(b).span(56..57)
                        ])
                        .span(51..58),
                        body: ExprKind::Tuple(vec![
                            ExprKind::ident_id(b).span(64..65),
                            ExprKind::ident_id(a).span(67..68)
                        ])
                        .span(62..69),
                        span: Span::from(49..69)
                    }
                ]
            }
            .span(5..69),
            expected_table
        ))
    );
}

#[test]
fn for_() {
    let input = "
    for x in [1, 2, 3] do x + 5
";

    let mut expected_table = NameTable::default();
    let x = expected_table.vars.insert(VarInfo {
        ident: Ident::new("x"),
        mutable: false,
        ty: None,
        span: Span::from(9..10),
    });

    assert_eq!(
        test_resolve_expr(input),
        Ok((
            ExprKind::For {
                pat: PatKind::Ident(x).span(9..10),
                iter: ExprKind::Array(vec![
                    ExprKind::int(1).span(15..16),
                    ExprKind::int(2).span(18..19),
                    ExprKind::int(3).span(21..22)
                ])
                .span(14..23)
                .into(),
                body: ExprKind::Infix {
                    op: InfixOp::Add,
                    lhs: ExprKind::ident_id(x).span(27..28).into(),
                    rhs: ExprKind::int(5).span(31..32).into()
                }
                .span(27..32)
                .into()
            }
            .span(5..32),
            expected_table
        ))
    );
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
    let id_1 = expected_table.vars.insert(VarInfo {
        ident: Ident::new("a"),
        mutable: false,
        ty: Some(TyKind::UInt.span(14..18)),
        span: Span::from(11..12),
    });
    let id_2 = expected_table.vars.insert(VarInfo {
        ident: Ident::new("a"),
        mutable: false,
        ty: None,
        span: Span::from(32..33),
    });
    let id_3 = expected_table.vars.insert(VarInfo {
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
    let fib = expected_table.vars.insert(VarInfo {
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
    let n = expected_table.vars.insert(VarInfo {
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
                kind: ExecKind::Fn {
                    generics: smallvec![],
                    params: vec![Param {
                        mutable: false,
                        pat: PatKind::Ident(n).span(12..13),
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
                                op: InfixOp::Add,
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

#[test]
fn unbound_var() {
    assert_eq!(
        test_resolve_expr("a + 5"),
        Err(ErrorKind::UnboundVariable(Ident::new("a")).span(0..1))
    );
}

#[test]
fn consts() {
    let input = "
    const B = A * 2
    const A = 5
";

    let mut expected_table = NameTable::default();
    let b = expected_table.vars.insert(VarInfo {
        ident: Ident::new("B"),
        mutable: false,
        ty: None,
        span: Span::from(11..12),
    });
    let a = expected_table.vars.insert(VarInfo {
        ident: Ident::new("A"),
        mutable: false,
        ty: None,
        span: Span::from(31..32),
    });

    assert_eq!(
        test_resolve_full(input),
        Ok((
            vec![
                ExecItem {
                    ident: b,
                    ident_span: Span::from(11..12),
                    kind: ExecKind::Const {
                        ty: None,
                        val: ExprKind::Infix {
                            op: InfixOp::Mul,
                            lhs: ExprKind::ident_id(a).span(15..16).into(),
                            rhs: ExprKind::int(2).span(19..20).into()
                        }
                        .span(15..20)
                    }
                },
                ExecItem {
                    ident: a,
                    ident_span: Span::from(31..32),
                    kind: ExecKind::Const {
                        ty: None,
                        val: ExprKind::int(5).span(35..36)
                    }
                }
            ],
            expected_table
        ))
    );
}

#[test]
fn list() {
    let input = r#"
    record List[T](head: Link[T])

    enum Link[T]
    | Cons(elem: T, next: Link[T])
    | Nil()

    fn cons[T](list: List[T], elem: T): List[T] -> "todo"
"#;

    let mut adt_table = PartialAdtTable::default();

    let list_adt = adt_table.reserve();
    let link = adt_table.reserve();

    let list_t = adt_table.insert(AdtInfo::Param(Ident::new("T")));
    let link_t = adt_table.insert(AdtInfo::Param(Ident::new("T")));

    adt_table.fulfill(
        list_adt,
        AdtInfo::Item(AdtItem {
            ident: Ident::new("List").span(12..16),
            generics: smallvec![list_t],
            span: Span::from(5..34),
            kind: AdtKind::Record(vec![Field {
                ident: Ident::new("head"),
                ty: TyKind::Adt(link, vec![TyKind::Adt(list_t, vec![]).span(31..32)]).span(26..33),
                span: Span::from(20..33),
            }]),
        }),
    );
    adt_table.fulfill(
        link,
        AdtInfo::Item(AdtItem {
            ident: Ident::new("Link").span(45..49),
            generics: smallvec![link_t],
            span: Span::from(40..99),
            kind: AdtKind::Enum(vec![
                Variant {
                    ident: Ident::new("Cons").span(59..63),
                    fields: vec![
                        Field {
                            ident: Ident::new("elem"),
                            ty: TyKind::Adt(link_t, vec![]).span(70..71),
                            span: Span::from(64..71),
                        },
                        Field {
                            ident: Ident::new("next"),
                            ty: TyKind::Adt(link, vec![TyKind::Adt(link_t, vec![]).span(84..85)])
                                .span(79..86),
                            span: Span::from(73..86),
                        },
                    ],
                },
                Variant {
                    ident: Ident::new("Nil").span(94..97),
                    fields: vec![],
                },
            ]),
        }),
    );

    let fn_t = adt_table.insert(AdtInfo::Param(Ident::new("T")));

    let mut table = NameTable {
        adts: adt_table.finalise(),
        vars: VarTable::default(),
    };

    let cons = table.vars.insert(VarInfo {
        ident: Ident::new("cons"),
        mutable: false,
        ty: Some(
            TyKind::Fn {
                params: vec![
                    ParamTy {
                        mutable: false,
                        ty: TyKind::Adt(list_adt, vec![TyKind::Adt(fn_t, vec![]).span(127..128)])
                            .span(122..129),
                    },
                    ParamTy {
                        mutable: false,
                        ty: TyKind::Adt(fn_t, vec![]).span(137..138),
                    },
                ],
                result: TyKind::Adt(list_adt, vec![TyKind::Adt(fn_t, vec![]).span(146..147)])
                    .span(141..148)
                    .into(),
            }
            .span(112..148),
        ),
        span: Span::from(108..112),
    });
    let list = table.vars.insert(VarInfo {
        ident: Ident::new("list"),
        mutable: false,
        ty: Some(
            TyKind::Adt(list_adt, vec![TyKind::Adt(fn_t, vec![]).span(127..128)]).span(122..129),
        ),
        span: Span::from(116..120),
    });
    let elem = table.vars.insert(VarInfo {
        ident: Ident::new("elem"),
        mutable: false,
        ty: Some(TyKind::Adt(fn_t, vec![]).span(137..138)),
        span: Span::from(131..135),
    });

    assert_eq!(
        test_resolve_full(input),
        Ok((
            vec![ExecItem {
                ident: cons,
                ident_span: Span::from(108..112),
                kind: ExecKind::Fn {
                    generics: smallvec![fn_t],
                    params: vec![
                        Param {
                            mutable: false,
                            pat: PatKind::Ident(list).span(116..120),
                            ty: TyKind::Adt(
                                list_adt,
                                vec![TyKind::Adt(fn_t, vec![]).span(127..128)]
                            )
                            .span(122..129),
                        },
                        Param {
                            mutable: false,
                            pat: PatKind::Ident(elem).span(131..135),
                            ty: TyKind::Adt(fn_t, vec![]).span(137..138),
                        },
                    ],
                    result: Return {
                        mutable: false,
                        ty: TyKind::Adt(list_adt, vec![TyKind::Adt(fn_t, vec![]).span(146..147)])
                            .span(141..148)
                    },
                    body: ExprKind::string("todo").span(152..158)
                }
            }],
            table
        ))
    );
}

#[test]
fn unknown_type() {
    assert_eq!(
        test_resolve_expr(r#"{let x: String = "Hello, World!"}"#),
        Err(ErrorKind::UnknownType(TyKind::string()).span(8..14))
    );
}

#[test]
fn rebinding() {
    let input = "
    const A = 5
    const A = 6
";

    assert_eq!(
        test_resolve_full(input),
        Err(ErrorKind::DupItem(Ident::new("A"), Span::from(11..12)).span(27..28))
    );

    let input = "
    record Foo()
    record Foo(val: UInt)
";

    assert_eq!(
        test_resolve_full(input),
        Err(ErrorKind::DupItem(Ident::new("Foo"), Span::from(12..15)).span(29..32))
    );
}
