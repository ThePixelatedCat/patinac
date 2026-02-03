use crate::helpers::Spanned;
use crate::lexer::TokenType;
use crate::parser::{ParseError, ParseResult};

use super::Parser;
use super::ast::{Ast, Binding, Bop, Expr, ExprS, Field, Item, ItemS, Type, Unop, Variant};

fn parse_expr(input: &str) -> ExprS {
    let mut parser = Parser::new(input);
    parser.expression().unwrap()
}

fn parse_expr_err(input: &str) -> ParseResult<ExprS> {
    let mut parser = Parser::new(input);
    parser.expression()
}

fn parse_item(input: &str) -> ItemS {
    let mut parser = Parser::new(input);
    parser.item().unwrap()
}

fn parse_item_err(input: &str) -> ParseResult<ItemS> {
    let mut parser = Parser::new(input);
    parser.item()
}

fn parse_ast(input: &str) -> Ast {
    let mut parser = Parser::new(input);
    parser.file().unwrap()
}

#[test]
fn lit_expressions() {
    assert_eq!(parse_expr("42"), Expr::Int(42).spanned(0..2));

    assert_eq!(parse_expr("  2.7768"), Expr::Float(2.7768).spanned(2..8));

    assert_eq!(
        parse_expr(r#""I am a Str!""#),
        Expr::String("I am a Str!".into()).spanned(0..13)
    );

    assert_eq!(parse_expr(r"'\''"), Expr::Char('\'').spanned(0..4));

    assert_eq!(
        parse_expr(r#"(42,(2,),"end")"#),
        Expr::Tuple(vec![
            Expr::Int(42).spanned(1..3),
            Expr::Tuple(vec![Expr::Int(2).spanned(5..6)]).spanned(4..8),
            Expr::String("end".into()).spanned(9..14)
        ])
        .spanned(0..15)
    );

    assert_eq!(
        parse_expr("[1, 4, 3, 2]"),
        Expr::Array(vec![
            Expr::Int(1).spanned(1..2),
            Expr::Int(4).spanned(4..5),
            Expr::Int(3).spanned(7..8),
            Expr::Int(2).spanned(10..11)
        ])
        .spanned(0..12)
    );

    assert_eq!(parse_expr("foo"), Expr::Ident("foo".into()).spanned(0..3));
}

#[test]
fn unop_expressions() {
    assert_eq!(
        parse_expr("!  is_visible"),
        Expr::UnaryOp {
            op: Unop::Not,
            expr: Expr::Ident("is_visible".into()).spanned(3..13).into(),
        }
        .spanned(0..13)
    );

    assert_eq!(
        parse_expr("-(-13)"),
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
fn binop_expressions() {
    assert_eq!(
        parse_expr("4 + 2 * 3"),
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

    assert_eq!(
        parse_expr("4 * 2 + 3"),
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

    assert_eq!(
        parse_expr("4 - 2 - 3"),
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

    assert_eq!(
        parse_expr("4 ** 2 ** 3"),
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

    assert_eq!(
        parse_expr("4 ^ 2 ^ 3"),
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

    assert_eq!(
        parse_expr("true || false && true"),
        Expr::BinaryOp {
            op: Bop::Or,
            lhs: Expr::Bool(true).spanned(0..4).into(),
            rhs: Expr::BinaryOp {
                op: Bop::And,
                lhs: Expr::Bool(false).spanned(8..13).into(),
                rhs: Expr::Bool(true).spanned(17..21).into(),
            }
            .spanned(8..21)
            .into()
        }
        .spanned(0..21)
    );

    assert_eq!(
        parse_expr("3 & 1 | 5"),
        Expr::BinaryOp {
            op: Bop::BOr,
            lhs: Expr::BinaryOp {
                op: Bop::BAnd,
                lhs: Expr::Int(3).spanned(0..1).into(),
                rhs: Expr::Int(1).spanned(4..5).into(),
            }
            .spanned(0..5)
            .into(),
            rhs: Expr::Int(5).spanned(8..9).into()
        }
        .spanned(0..9)
    );

    assert_eq!(
        parse_expr("(3 >= 4) != true"),
        Expr::BinaryOp {
            op: Bop::Neq,
            lhs: Expr::BinaryOp {
                op: Bop::Geq,
                lhs: Expr::Int(3).spanned(1..2).into(),
                rhs: Expr::Int(4).spanned(6..7).into(),
            }
            .spanned(0..8)
            .into(),
            rhs: Expr::Bool(true).spanned(12..16).into()
        }
        .spanned(0..16)
    );

    assert_eq!(
        parse_expr("(4 > 3) == true"),
        Expr::BinaryOp {
            op: Bop::Eqq,
            lhs: Expr::BinaryOp {
                op: Bop::Gt,
                lhs: Expr::Int(4).spanned(1..2).into(),
                rhs: Expr::Int(3).spanned(5..6).into(),
            }
            .spanned(0..7)
            .into(),
            rhs: Expr::Bool(true).spanned(11..15).into()
        }
        .spanned(0..15)
    );
}

#[test]
fn compound_expressions() {
    assert_eq!(
        parse_expr("bar (  x, 2)"),
        Expr::FnCall {
            fun: Expr::Ident("bar".into()).spanned(0..3).into(),
            args: vec![
                Expr::Ident("x".into()).spanned(7..8),
                Expr::Int(2).spanned(10..11),
            ],
        }
        .spanned(0..12)
    );

    assert_eq!(
        parse_expr("if (0.5) foo()"),
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

    assert_eq!(
        parse_expr("if (0.5) foo else bar"),
        Expr::If {
            cond: Expr::Float(0.5).spanned(4..7).into(),
            th: Expr::Ident("foo".into()).spanned(9..12).into(),
            el: Some(Expr::Ident("bar".into()).spanned(18..21).into())
        }
        .spanned(0..21)
    );

    assert_eq!(
        parse_expr("(fn(a, b: Int) -> a + b)(1, 2)"),
        Expr::FnCall {
            fun: Expr::Lambda {
                params: vec![
                    Binding::Var {
                        mutable: false,
                        ident: "a".into(),
                        annotated_ty: None
                    }
                    .spanned(4..5),
                    Binding::Var {
                        mutable: false,
                        ident: "b".into(),
                        annotated_ty: Some(Type::Int.spanned(10..13))
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

    assert_eq!(
        parse_expr("[1, 2, 3][1-1]"),
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

    assert_eq!(
        parse_expr("self._0"),
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
fn var_expressions() {
    assert_eq!(
        parse_expr("let x = 7 + sin(3.);"),
        Expr::Let {
            binding: Binding::Var {
                mutable: false,
                ident: "x".into(),
                annotated_ty: None
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

    assert_eq!(
        parse_expr("let mut y: UInt = 7"),
        Expr::Let {
            binding: Binding::Var {
                mutable: true,
                ident: "y".into(),
                annotated_ty: Some(Type::UInt.spanned(11..15))
            }
            .spanned(4..15),
            value: Expr::Int(7).spanned(18..19).into()
        }
        .spanned(0..19)
    );

    assert_eq!(
        parse_expr("y = 3 + 7 * 0.5"),
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
fn block_expressions() {
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
                        annotated_ty: None
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
                                    annotated_ty: None
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
fn malformed_expressions() {
    assert_eq!(
        parse_expr_err("[1, 3, 4, 5"),
        Err(ParseError::Mismatched {
            expected: TokenType::RBracket,
            found: TokenType::Eof
        }
        .spanned(11..11))
    );
    assert_eq!(
        parse_expr_err("*5"),
        Err(
            ParseError::Unexpected(TokenType::Times, Some("start of expression".into()))
                .spanned(0..1)
        )
    );
    assert_eq!(
        parse_expr_err("let a = 1 + 3 print(a)"),
        Err(
            ParseError::Unexpected(TokenType::Ident, Some("end of expression".into()))
                .spanned(14..19)
        )
    );
    assert_eq!(
        parse_expr_err("print(5, 2;)"),
        Err(ParseError::Mismatched {
            expected: TokenType::RParen,
            found: TokenType::Semicolon
        }
        .spanned(10..11))
    );
}

#[test]
fn const_items() {
    assert_eq!(
        parse_item(r#"const HELLO_WORLD: String = "Hello, World!""#),
        Item::Const {
            name: "HELLO_WORLD".into(),
            ty: Some(
                Type::Named {
                    name: "String".into(),
                    args: vec![]
                }
                .spanned(19..25)
            ),
            value: Expr::String("Hello, World!".into()).spanned(28..43)
        }
        .spanned(0..43)
    );

    assert_eq!(
        parse_item(r#"const ID = fn(x) -> x"#),
        Item::Const {
            name: "ID".into(),
            ty: None,
            value: Expr::Lambda {
                params: vec![
                    Binding::Var {
                        mutable: false,
                        ident: "x".into(),
                        annotated_ty: None
                    }
                    .spanned(14..15)
                ],
                return_type: None,
                body: Expr::Ident("x".into()).spanned(20..21).into()
            }
            .spanned(11..21)
        }
        .spanned(0..21)
    );
}

#[test]
fn struct_items() {
    let item = parse_item(
        r#"
        struct Foo<T, U> {
            x: Char  ,
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
                    ty: Type::Char.spanned(43..47)
                }
                .spanned(40..47),
                Field {
                    name: "bar".into(),
                    ty: Type::Named {
                        name: "Bar".into(),
                        args: vec![
                            Type::Named {
                                name: "Baz".into(),
                                args: vec![
                                    Type::Named {
                                        name: "T".into(),
                                        args: vec![]
                                    }
                                    .spanned(76..77)
                                ]
                            }
                            .spanned(72..78)
                        ]
                    }
                    .spanned(68..79)
                }
                .spanned(63..79)
            ]
        }
        .spanned(9..89)
    )
}

#[test]
fn enum_items() {
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
                            args: vec![]
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
                                args: vec![]
                            }
                            .spanned(75..78)
                        }
                        .spanned(71..78),
                        Field {
                            name: "fizz".into(),
                            ty: Type::Named {
                                name: "Buzz".into(),
                                args: vec![]
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
fn function_items() {
    assert_eq!(
        parse_item(r#"fn sum(mut a, b: Byte) -> a + b"#),
        Item::Func {
            name: "sum".into(),
            params: vec![
                Binding::Var {
                    mutable: true,
                    ident: "a".into(),
                    annotated_ty: None
                }
                .spanned(7..12),
                Binding::Var {
                    mutable: false,
                    ident: "b".into(),
                    annotated_ty: Some(Type::Byte.spanned(17..21))
                }
                .spanned(14..21)
            ],
            return_ty: None,
            body: Expr::BinaryOp {
                op: Bop::Add,
                lhs: Expr::Ident("a".into()).spanned(26..27).into(),
                rhs: Expr::Ident("b".into()).spanned(30..31).into()
            }
            .spanned(26..31)
        }
        .spanned(0..31)
    )
}

#[test]
fn malformed_items() {
    assert_eq!(
        parse_item_err("const fn: Int = 5"),
        Err(ParseError::Mismatched {
            expected: TokenType::Ident,
            found: TokenType::Fn,
        }
        .spanned(6..8))
    );

    assert_eq!(
        parse_item_err("const NO_DICTS: {String: Int} = 5"),
        Err(
            ParseError::Unexpected(TokenType::LBrace, Some("start of type name".into()))
                .spanned(16..17)
        )
    );

    assert_eq!(
        parse_item_err("let global = 0"),
        Err(ParseError::Unexpected(TokenType::Let, Some("start of item".into())).spanned(0..3))
    );

    assert_eq!(
        parse_item_err("struct CSyntax { Int five }"),
        Err(ParseError::Mismatched {
            expected: TokenType::Ident,
            found: TokenType::Int,
        }
        .spanned(17..20))
    );

    assert_eq!(
        parse_item_err("enum NoComma { Bad Syntax }"),
        Err(ParseError::Unexpected(
            TokenType::Ident,
            Some("after variant name. expected one of `,` `(` `{`".into())
        )
        .spanned(19..25))
    )
}

#[test]
fn file() {
    let items = parse_ast(
        r#"
        fn wow_we_did_it(mut x, bar: Bar<Baz<T>, U>): fn(Int): Int -> {
            let mut x: ( Bool, T) = true + sin(y);
            x = if (bar < 3) {
                let baz = bar.value + 2 * 4;
                x + 1;
            } else if (bar <= 2)
                fizz(3, 5.1)
        }

        struct Foo<T, U> {
            x: String,
            bar: Bar<Baz<T>, [U]>,
        }"#,
    );

    assert_eq!(
        items[0],
        Item::Func {
            name: "wow_we_did_it".into(),
            params: vec![
                Binding::Var {
                    mutable: true,
                    ident: "x".into(),
                    annotated_ty: None
                }
                .spanned(26..31),
                Binding::Var {
                    mutable: false,
                    ident: "bar".into(),
                    annotated_ty: Some(
                        Type::Named {
                            name: "Bar".into(),
                            args: vec![
                                Type::Named {
                                    name: "Baz".into(),
                                    args: vec![
                                        Type::Named {
                                            name: "T".into(),
                                            args: vec![],
                                        }
                                        .spanned(46..47)
                                    ],
                                }
                                .spanned(42..48),
                                Type::Named {
                                    name: "U".into(),
                                    args: vec![],
                                }
                                .spanned(50..51)
                            ],
                        }
                        .spanned(38..52)
                    )
                }
                .spanned(33..52)
            ],
            return_ty: Some(
                Type::Fn(
                    vec![Type::Int.spanned(58..61)],
                    Type::Int.spanned(64..67).into()
                )
                .spanned(55..67)
            ),
            body: Expr::Block {
                exprs: vec![
                    Expr::Let {
                        binding: Binding::Var {
                            mutable: true,
                            ident: "x".into(),
                            annotated_ty: Some(
                                Type::Tuple(vec![
                                    Type::Bool.spanned(98..102),
                                    Type::Named {
                                        name: "T".into(),
                                        args: vec![]
                                    }
                                    .spanned(104..105)
                                ])
                                .spanned(96..106)
                            )
                        }
                        .spanned(89..106),
                        value: Expr::BinaryOp {
                            op: Bop::Add,
                            lhs: Expr::Bool(true).spanned(109..113).into(),
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
                                            annotated_ty: None
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
                        name: "String".into(),
                        args: vec![],
                    }
                    .spanned(338..344),
                }
                .spanned(335..344),
                Field {
                    name: "bar".into(),
                    ty: Type::Named {
                        name: "Bar".into(),
                        args: vec![
                            Type::Named {
                                name: "Baz".into(),
                                args: vec![
                                    Type::Named {
                                        name: "T".into(),
                                        args: vec![],
                                    }
                                    .spanned(371..372)
                                ],
                            }
                            .spanned(367..373),
                            Type::Array(
                                Type::Named {
                                    name: "U".into(),
                                    args: vec![],
                                }
                                .spanned(376..377)
                                .into()
                            )
                            .spanned(375..378)
                        ],
                    }
                    .spanned(363..379),
                }
                .spanned(358..379)
            ]
        }
        .spanned(304..390)
    );
}
