use crate::helpers::Spanned;

use super::Parser;
use super::ast::{Ast, Binding, Bop, Expr, ExprS, Field, Item, ItemS, Type, Unop, Variant};

fn parse_expr(input: &str) -> ExprS {
    let mut parser = Parser::new(input);
    parser.expression().unwrap()
}

fn parse_item(input: &str) -> ItemS {
    let mut parser = Parser::new(input);
    parser.item().unwrap()
}

fn parse_ast(input: &str) -> Ast {
    let mut parser = Parser::new(input);
    parser.file().unwrap()
}

#[test]
fn parse_lit_expressions() {
    let expr = parse_expr("42");
    assert_eq!(expr, Expr::Int(42).spanned(0..2));

    let expr = parse_expr("  2.7768");
    assert_eq!(expr, Expr::Float(2.7768).spanned(2..8));

    let expr = parse_expr(r#""I am a Str!""#);
    assert_eq!(expr, Expr::Str("I am a Str!".into()).spanned(0..13));

    let expr = parse_expr(r#"(42,(2,),"end")"#);
    assert_eq!(
        expr,
        Expr::Tuple(vec![
            Expr::Int(42).spanned(1..3),
            Expr::Tuple(vec![Expr::Int(2).spanned(5..6)]).spanned(4..8),
            Expr::Str("end".into()).spanned(9..14)
        ])
        .spanned(0..15)
    );

    let expr = parse_expr("[1, 4, 3, 2]");
    assert_eq!(
        expr,
        Expr::Array(vec![
            Expr::Int(1).spanned(1..2),
            Expr::Int(4).spanned(4..5),
            Expr::Int(3).spanned(7..8),
            Expr::Int(2).spanned(10..11)
        ])
        .spanned(0..12)
    );

    let expr = parse_expr("foo");
    assert_eq!(expr, Expr::Ident("foo".into()).spanned(0..3));
}

#[test]
fn parse_unop_expressions() {
    let expr = parse_expr("!  is_visible");
    assert_eq!(
        expr,
        Expr::UnaryOp {
            op: Unop::Not,
            expr: Expr::Ident("is_visible".into()).spanned(3..13).into(),
        }
        .spanned(0..13)
    );

    let expr = parse_expr("-(-13)");
    assert_eq!(
        expr,
        Expr::UnaryOp {
            op: Unop::Neg,
            expr: Expr::UnaryOp {
                op: Unop::Neg,
                expr: Expr::Int(13).spanned(3..5).into(),
            }
            .spanned(1..6)
            .into()
        }
        .spanned(0..6)
    );
}

#[test]
fn parse_binop_expressions() {
    let expr = parse_expr("4 + 2 * 3");
    assert_eq!(
        expr,
        Expr::BinaryOp {
            op: Bop::Add,
            lhs: Expr::Int(4).spanned(0..1).into(),
            rhs: Expr::BinaryOp {
                op: Bop::Mul,
                lhs: Expr::Int(2).spanned(4..5).into(),
                rhs: Expr::Int(3).spanned(8..9).into()
            }
            .spanned(4..9)
            .into()
        }
        .spanned(0..9)
    );

    let expr = parse_expr("4 * 2 + 3");
    assert_eq!(
        expr,
        Expr::BinaryOp {
            op: Bop::Add,
            lhs: Expr::BinaryOp {
                op: Bop::Mul,
                lhs: Expr::Int(4).spanned(0..1).into(),
                rhs: Expr::Int(2).spanned(4..5).into()
            }
            .spanned(0..5)
            .into(),
            rhs: Expr::Int(3).spanned(8..9).into(),
        }
        .spanned(0..9)
    );

    let expr = parse_expr("4 - 2 - 3");
    assert_eq!(
        expr,
        Expr::BinaryOp {
            op: Bop::Sub,
            lhs: Expr::BinaryOp {
                op: Bop::Sub,
                lhs: Expr::Int(4).spanned(0..1).into(),
                rhs: Expr::Int(2).spanned(4..5).into()
            }
            .spanned(0..5)
            .into(),
            rhs: Expr::Int(3).spanned(8..9).into(),
        }
        .spanned(0..9)
    );

    let expr = parse_expr("4 ** 2 ** 3");
    assert_eq!(
        expr,
        Expr::BinaryOp {
            op: Bop::Exp,
            lhs: Expr::Int(4).spanned(0..1).into(),
            rhs: Expr::BinaryOp {
                op: Bop::Exp,
                lhs: Expr::Int(2).spanned(5..6).into(),
                rhs: Expr::Int(3).spanned(10..11).into()
            }
            .spanned(5..11)
            .into()
        }
        .spanned(0..11)
    );

    let expr = parse_expr("4 ^ 2 ^ 3");
    assert_eq!(
        expr,
        Expr::BinaryOp {
            op: Bop::Xor,
            lhs: Expr::BinaryOp {
                op: Bop::Xor,
                lhs: Expr::Int(4).spanned(0..1).into(),
                rhs: Expr::Int(2).spanned(4..5).into()
            }
            .spanned(0..5)
            .into(),
            rhs: Expr::Int(3).spanned(8..9).into(),
        }
        .spanned(0..9)
    );
}

#[test]
fn parse_compound_expressions() {
    let expr = parse_expr("bar (  x, 2)");
    assert_eq!(
        expr,
        Expr::FnCall {
            fun: Expr::Ident("bar".into()).spanned(0..3).into(),
            args: vec![
                Expr::Ident("x".into()).spanned(7..8),
                Expr::Int(2).spanned(10..11),
            ],
        }
        .spanned(0..12)
    );

    let expr = parse_expr("if (0.5) foo()");
    assert_eq!(
        expr,
        Expr::If {
            cond: Expr::Float(0.5).spanned(4..7).into(),
            th: Expr::FnCall {
                fun: Expr::Ident("foo".into()).spanned(9..12).into(),
                args: Vec::new()
            }
            .spanned(9..14)
            .into(),
            el: None
        }
        .spanned(0..14)
    );

    let expr = parse_expr("if (0.5) foo else bar");
    assert_eq!(
        expr,
        Expr::If {
            cond: Expr::Float(0.5).spanned(4..7).into(),
            th: Expr::Ident("foo".into()).spanned(9..12).into(),
            el: Some(Expr::Ident("bar".into()).spanned(18..21).into())
        }
        .spanned(0..21)
    );

    let expr = parse_expr("(fn(a, b: Int) -> a + b)(1, 2)");
    assert_eq!(
        expr,
        Expr::FnCall {
            fun: Expr::Lambda {
                params: vec![
                    Binding::Var {
                        mutable: false,
                        ident: "a".into(),
                        type_annotation: None
                    }
                    .spanned(4..5),
                    Binding::Var {
                        mutable: false,
                        ident: "b".into(),
                        type_annotation: Some(
                            Type::Named {
                                name: "Int".into(),
                                generics: vec![]
                            }
                            .spanned(10..13)
                        )
                    }
                    .spanned(7..13)
                ],
                return_type: None,
                body: Expr::BinaryOp {
                    op: Bop::Add,
                    lhs: Expr::Ident("a".into()).spanned(18..19).into(),
                    rhs: Expr::Ident("b".into()).spanned(22..23).into()
                }
                .spanned(18..23)
                .into()
            }
            .spanned(0..24)
            .into(),
            args: vec![
                Expr::Int(1).spanned(25..26).into(),
                Expr::Int(2).spanned(28..29).into()
            ]
        }
        .spanned(0..30)
    );

    let expr = parse_expr("[1, 2, 3][1-1]");
    assert_eq!(
        expr,
        Expr::Index {
            arr: Expr::Array(vec![
                Expr::Int(1).spanned(1..2),
                Expr::Int(2).spanned(4..5),
                Expr::Int(3).spanned(7..8)
            ])
            .spanned(0..9)
            .into(),
            index: Expr::BinaryOp {
                op: Bop::Sub,
                lhs: Expr::Int(1).spanned(10..11).into(),
                rhs: Expr::Int(1).spanned(12..13).into()
            }
            .spanned(10..13)
            .into()
        }
        .spanned(0..14)
    );

    let expr = parse_expr("self._0");
    assert_eq!(
        expr,
        Expr::FieldAccess {
            base: Expr::Ident("self".into()).spanned(0..4).into(),
            field: Spanned {
                inner: "_0".into(),
                span: (5..7).into()
            }
        }
        .spanned(0..7)
    );
}

#[test]
fn parse_var_expresssions() {
    let expr = parse_expr("let x = 7 + sin(3.);");
    assert_eq!(
        expr,
        Expr::Let {
            binding: Binding::Var {
                mutable: false,
                ident: "x".into(),
                type_annotation: None
            }
            .spanned(4..5),
            value: Expr::BinaryOp {
                op: Bop::Add,
                lhs: Expr::Int(7).spanned(8..9).into(),
                rhs: Expr::FnCall {
                    fun: Expr::Ident("sin".into()).spanned(12..15).into(),
                    args: vec![Expr::Float(3.0).spanned(16..18)]
                }
                .spanned(12..19)
                .into()
            }
            .spanned(8..19)
            .into()
        }
        .spanned(0..19)
    );

    let expr = parse_expr("let mut y: Int = 7");
    assert_eq!(
        expr,
        Expr::Let {
            binding: Binding::Var {
                mutable: true,
                ident: "y".into(),
                type_annotation: Some(
                    Type::Named {
                        name: "Int".into(),
                        generics: vec![]
                    }
                    .spanned(11..14)
                )
            }
            .spanned(4..14),
            value: Expr::Int(7).spanned(17..18).into()
        }
        .spanned(0..18)
    );

    let expr = parse_expr("y = 3 + 7 * 0.5");
    assert_eq!(
        expr,
        Expr::Assign {
            ident: Spanned {
                inner: "y".into(),
                span: (0..1).into()
            },
            value: Expr::BinaryOp {
                op: Bop::Add,
                lhs: Expr::Int(3).spanned(4..5).into(),
                rhs: Expr::BinaryOp {
                    op: Bop::Mul,
                    lhs: Expr::Int(7).spanned(8..9).into(),
                    rhs: Expr::Float(0.5).spanned(12..15).into()
                }
                .spanned(8..15)
                .into()
            }
            .spanned(4..15)
            .into()
        }
        .spanned(0..15)
    );
}

#[test]
fn parse_block_expressions() {
    let expr = parse_expr(
        "
    {
        let mut y = 5;
        3 + 1 - 2;
        y = 1;
        if (y < 3) {
            let a = 5;
            a
        } else 32;
    }",
    );
    assert_eq!(
        expr,
        Expr::Block {
            exprs: vec![
                Expr::Let {
                    binding: Binding::Var {
                        mutable: true,
                        ident: "y".into(),
                        type_annotation: None
                    }
                    .spanned(19..24),
                    value: Expr::Int(5).spanned(27..28).into()
                }
                .spanned(15..28),
                Expr::BinaryOp {
                    op: Bop::Sub,
                    lhs: Expr::BinaryOp {
                        op: Bop::Add,
                        lhs: Expr::Int(3).spanned(38..39).into(),
                        rhs: Expr::Int(1).spanned(42..43).into()
                    }
                    .spanned(38..43)
                    .into(),
                    rhs: Expr::Int(2).spanned(46..47).into()
                }
                .spanned(38..47),
                Expr::Assign {
                    ident: Spanned {
                        inner: "y".into(),
                        span: (57..58).into()
                    },
                    value: Expr::Int(1).spanned(61..62).into()
                }
                .spanned(57..62),
                Expr::If {
                    cond: Expr::BinaryOp {
                        op: Bop::Lt,
                        lhs: Expr::Ident("y".into()).spanned(76..77).into(),
                        rhs: Expr::Int(3).spanned(80..81).into()
                    }
                    .spanned(76..81)
                    .into(),
                    th: Expr::Block {
                        exprs: vec![
                            Expr::Let {
                                binding: Binding::Var {
                                    mutable: false,
                                    ident: "a".into(),
                                    type_annotation: None
                                }
                                .spanned(101..102),
                                value: Expr::Int(5).spanned(105..106).into()
                            }
                            .spanned(97..106),
                            Expr::Ident("a".to_string()).spanned(120..121)
                        ],
                        trailing: true
                    }
                    .spanned(83..131)
                    .into(),
                    el: Some(Expr::Int(32).spanned(137..139).into())
                }
                .spanned(72..139)
            ],
            trailing: false
        }
        .spanned(5..146)
    );
}

#[test]
fn parse_const_items() {
    let item = parse_item(r#"const HELLO_WORLD: Str = "Hello, World!""#);
    assert_eq!(
        item,
        Item::Const {
            name: "HELLO_WORLD".into(),
            ty: Type::Named {
                name: "Str".into(),
                generics: vec![]
            }
            .spanned(19..22),
            value: Expr::Str("Hello, World!".into()).spanned(25..40)
        }
        .spanned(0..40)
    );
}

#[test]
fn parse_struct_items() {
    let item = parse_item(
        r#"
        struct Foo<T, U> {
            x: Str,
            bar: Bar<Baz<T>>
        }"#,
    );
    assert_eq!(
        item,
        Item::Struct {
            name: "Foo".into(),
            generic_params: vec!["T".into(), "U".into()],
            fields: vec![
                Field {
                    name: "x".into(),
                    ty: Type::Named {
                        name: "Str".into(),
                        generics: vec![]
                    }
                    .spanned(43..46)
                }
                .spanned(40..46),
                Field {
                    name: "bar".into(),
                    ty: Type::Named {
                        name: "Bar".into(),
                        generics: vec![
                            Type::Named {
                                name: "Baz".into(),
                                generics: vec![
                                    Type::Named {
                                        name: "T".into(),
                                        generics: vec![]
                                    }
                                    .spanned(73..74)
                                ]
                            }
                            .spanned(69..75)
                        ]
                    }
                    .spanned(65..76)
                }
                .spanned(60..76)
            ]
        }
        .spanned(9..86)
    )
}

#[test]
fn parse_enum_items() {
    let item = parse_item(
        r#"
        enum Foo {
            X,
            Y(Bar),
            Z { baz:Baz, fizz: Buzz }
        }"#,
    );
    assert_eq!(
        item,
        Item::Enum {
            name: "Foo".into(),
            generic_params: vec![],
            variants: vec![
                Variant::Unit("X".into()).spanned(32..33),
                Variant::Tuple(
                    "Y".into(),
                    vec![
                        Type::Named {
                            name: "Bar".into(),
                            generics: vec![]
                        }
                        .spanned(49..52)
                    ]
                )
                .spanned(47..53),
                Variant::Struct(
                    "Z".into(),
                    vec![
                        Field {
                            name: "baz".into(),
                            ty: Type::Named {
                                name: "Baz".into(),
                                generics: vec![]
                            }
                            .spanned(75..78)
                        }
                        .spanned(71..78),
                        Field {
                            name: "fizz".into(),
                            ty: Type::Named {
                                name: "Buzz".into(),
                                generics: vec![]
                            }
                            .spanned(86..90)
                        }
                        .spanned(80..90)
                    ]
                )
                .spanned(67..92),
            ]
        }
        .spanned(9..102)
    )
}

#[test]
fn parse_function_items() {
    let item = parse_item(r#"fn sum(mut a, b: Int) -> a + b"#);
    assert_eq!(
        item,
        Item::Function {
            name: "sum".into(),
            params: vec![
                Binding::Var {
                    mutable: true,
                    ident: "a".into(),
                    type_annotation: None
                }
                .spanned(7..12),
                Binding::Var {
                    mutable: false,
                    ident: "b".into(),
                    type_annotation: Some(
                        Type::Named {
                            name: "Int".into(),
                            generics: vec![]
                        }
                        .spanned(17..20)
                    )
                }
                .spanned(14..20)
            ],
            return_type: None,
            body: Expr::BinaryOp {
                op: Bop::Add,
                lhs: Expr::Ident("a".into()).spanned(25..26).into(),
                rhs: Expr::Ident("b".into()).spanned(29..30).into()
            }
            .spanned(25..30)
        }
        .spanned(0..30)
    )
}

#[test]
fn parse_file() {
    let items = parse_ast(
        r#"
        fn wow_we_did_it(mut x, bar: Bar<Baz<T>, U>): fn(Int): Int -> {
            let mut x: (Float, T) = -7.0 + sin(y);
            x = if (bar < 3) {
                let baz = bar.value + 2 * 4;
                x + 1;
            } else if (bar <= 2)
                fizz(3, 5.1)
        }

        struct Foo<T, U> {
            x: Str,
            bar: Bar<Baz<T>, [U]>,
        }"#,
    );

    assert_eq!(
        items[0],
        Item::Function {
            name: "wow_we_did_it".into(),
            params: vec![
                Binding::Var {
                    mutable: true,
                    ident: "x".into(),
                    type_annotation: None
                }
                .spanned(26..31),
                Binding::Var {
                    mutable: false,
                    ident: "bar".into(),
                    type_annotation: Some(
                        Type::Named {
                            name: "Bar".into(),
                            generics: vec![
                                Type::Named {
                                    name: "Baz".into(),
                                    generics: vec![
                                        Type::Named {
                                            name: "T".into(),
                                            generics: vec![],
                                        }
                                        .spanned(46..47)
                                    ],
                                }
                                .spanned(42..48),
                                Type::Named {
                                    name: "U".into(),
                                    generics: vec![],
                                }
                                .spanned(50..51)
                            ],
                        }
                        .spanned(38..52)
                    )
                }
                .spanned(33..52)
            ],
            return_type: Some(
                Type::Fn {
                    params: vec![
                        Type::Named {
                            name: "Int".into(),
                            generics: vec![]
                        }
                        .spanned(58..61)
                    ],
                    result: Type::Named {
                        name: "Int".into(),
                        generics: vec![]
                    }
                    .spanned(64..67)
                    .into()
                }
                .spanned(55..67)
            ),
            body: Expr::Block {
                exprs: vec![
                    Expr::Let {
                        binding: Binding::Var {
                            mutable: true,
                            ident: "x".into(),
                            type_annotation: Some(
                                Type::Tuple(vec![
                                    Type::Named {
                                        name: "Float".into(),
                                        generics: vec![]
                                    }
                                    .spanned(97..102),
                                    Type::Named {
                                        name: "T".into(),
                                        generics: vec![]
                                    }
                                    .spanned(104..105)
                                ])
                                .spanned(96..106)
                            )
                        }
                        .spanned(89..106),
                        value: Expr::BinaryOp {
                            op: Bop::Add,
                            lhs: Expr::UnaryOp {
                                op: Unop::Neg,
                                expr: Expr::Float(7.0).spanned(110..113).into()
                            }
                            .spanned(109..113)
                            .into(),
                            rhs: Expr::FnCall {
                                fun: Expr::Ident("sin".into()).spanned(116..119).into(),
                                args: vec![Expr::Ident("y".into()).spanned(120..121)]
                            }
                            .spanned(116..122)
                            .into()
                        }
                        .spanned(109..122)
                        .into()
                    }
                    .spanned(85..122),
                    Expr::Assign {
                        ident: Spanned {
                            inner: "x".into(),
                            span: (136..137).into()
                        },
                        value: Expr::If {
                            cond: Expr::BinaryOp {
                                op: Bop::Lt,
                                lhs: Expr::Ident("bar".into()).spanned(144..147).into(),
                                rhs: Expr::Int(3).spanned(150..151).into()
                            }
                            .spanned(144..151)
                            .into(),
                            th: Expr::Block {
                                exprs: vec![
                                    Expr::Let {
                                        binding: Binding::Var {
                                            mutable: false,
                                            ident: "baz".into(),
                                            type_annotation: None
                                        }
                                        .spanned(175..178),
                                        value: Expr::BinaryOp {
                                            op: Bop::Add,
                                            lhs: Expr::FieldAccess {
                                                base: Expr::Ident("bar".into())
                                                    .spanned(181..184)
                                                    .into(),
                                                field: Spanned {
                                                    inner: "value".into(),
                                                    span: (185..190).into()
                                                }
                                            }
                                            .spanned(181..190)
                                            .into(),
                                            rhs: Expr::BinaryOp {
                                                op: Bop::Mul,
                                                lhs: Expr::Int(2).spanned(193..194).into(),
                                                rhs: Expr::Int(4).spanned(197..198).into()
                                            }
                                            .spanned(193..198)
                                            .into()
                                        }
                                        .spanned(181..198)
                                        .into()
                                    }
                                    .spanned(171..198),
                                    Expr::BinaryOp {
                                        op: Bop::Add,
                                        lhs: Expr::Ident("x".into()).spanned(216..217).into(),
                                        rhs: Expr::Int(1).spanned(220..221).into()
                                    }
                                    .spanned(216..221)
                                ],
                                trailing: false
                            }
                            .spanned(153..236)
                            .into(),
                            el: Some(
                                Expr::If {
                                    cond: Expr::BinaryOp {
                                        op: Bop::Leq,
                                        lhs: Expr::Ident("bar".into()).spanned(246..249).into(),
                                        rhs: Expr::Int(2).spanned(253..254).into()
                                    }
                                    .spanned(246..254)
                                    .into(),
                                    th: Expr::FnCall {
                                        fun: Expr::Ident("fizz".into()).spanned(272..276).into(),
                                        args: vec![
                                            Expr::Int(3).spanned(277..278),
                                            Expr::Float(5.1).spanned(280..283)
                                        ]
                                    }
                                    .spanned(272..284)
                                    .into(),
                                    el: None
                                }
                                .spanned(242..284)
                                .into()
                            )
                        }
                        .spanned(140..284)
                        .into()
                    }
                    .spanned(136..284),
                ],
                trailing: true
            }
            .spanned(71..294)
        }
        .spanned(9..294)
    );

    assert_eq!(
        items[1],
        Item::Struct {
            name: "Foo".into(),
            generic_params: vec!["T".into(), "U".into(),],
            fields: vec![
                Field {
                    name: "x".into(),
                    ty: Type::Named {
                        name: "Str".into(),
                        generics: vec![],
                    }
                    .spanned(338..341),
                }
                .spanned(335..341),
                Field {
                    name: "bar".into(),
                    ty: Type::Named {
                        name: "Bar".into(),
                        generics: vec![
                            Type::Named {
                                name: "Baz".into(),
                                generics: vec![
                                    Type::Named {
                                        name: "T".into(),
                                        generics: vec![],
                                    }
                                    .spanned(368..369)
                                ],
                            }
                            .spanned(364..370),
                            Type::Array(
                                Type::Named {
                                    name: "U".into(),
                                    generics: vec![],
                                }
                                .spanned(373..374)
                                .into()
                            )
                            .spanned(372..375)
                        ],
                    }
                    .spanned(360..376),
                }
                .spanned(355..376)
            ]
        }
        .spanned(304..387)
    );
}
