use crate::helpers::{Spannable, Spnd};
use crate::lexer::TT;
use crate::parser::{ParseError, ParseResult};

use super::Parser;
use super::ast::{Bop, Expr, ExprS, Field, Item, ItemS, Pattern, Type, Unop, Variant};

fn parse_expr(input: &str) -> ExprS {
    let mut parser = Parser::new(input);
    parser.expr().unwrap()
}

fn parse_expr_err(input: &str) -> ParseResult<ExprS> {
    let mut parser = Parser::new(input);
    parser.expr()
}

fn parse_item(input: &str) -> ItemS {
    let mut parser = Parser::new(input);
    parser.item().unwrap()
}

fn parse_item_err(input: &str) -> ParseResult<ItemS> {
    let mut parser = Parser::new(input);
    parser.item()
}

fn parse_ast(input: &str) -> Vec<ItemS> {
    let mut parser = Parser::new(input);
    parser.file().unwrap()
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
        parse_expr_err("let a = 1 + 3 print(a)"),
        Err(ParseError::Unexpected(TT::Ident, "end of expression").span(14..19))
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
                .span(19..25)
            ),
            value: Expr::String("Hello, World!".into()).span(28..43)
        }
        .span(0..43)
    );

    assert_eq!(
        parse_item(r#"const ID = fn(x) -> x"#),
        Item::Const {
            name: "ID".into(),
            ty: None,
            value: Expr::Lambda {
                params: vec![
                    Pattern::Var {
                        mutable: false,
                        ident: "x".into(),
                        annotated_ty: None
                    }
                    .span(14..15)
                ],
                return_type: None,
                body: Expr::Ident("x".into()).span(20..21).into()
            }
            .span(11..21)
        }
        .span(0..21)
    );
}

#[test]
fn struct_items() {
    let item = parse_item(
r#"
record Foo<T, U>
    x: Char  ,
    bar: Bar<Baz<T>>
"#,
    );
    assert_eq!(
        item,
        Item::Struct {
            name: "Foo".into(),
            generic_params: vec!["T".into(), "U".into()],
            fields: vec![
                Field {
                    name: "x".into(),
                    ty: Type::Char.span(25..29)
                }
                .span(22..29),
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
                                    .span(50..51)
                                ]
                            }
                            .span(46..52)
                        ]
                    }
                    .span(42..53)
                }
                .span(37..53)
            ]
        }
        .span(1..54)
    );
}

#[test]
fn enum_items() {
    let item = parse_item(
r#"
enum Foo 
| X,
| Y(Bar),
| Z 
    baz: Baz, 
    fizz: Buzz
"#,
    );
    assert_eq!(
        item,
        Item::Enum {
            name: "Foo".into(),
            generic_params: vec![],
            variants: vec![
                Variant::Unit("X".into()).span(15..16),
                Variant::Tuple(
                    "Y".into(),
                    vec![
                        Type::Named {
                            name: "Bar".into(),
                            args: vec![]
                        }
                        .span(24..27)
                    ]
                )
                .span(22..28),
                Variant::Struct(
                    "Z".into(),
                    vec![
                        Field {
                            name: "baz".into(),
                            ty: Type::Named {
                                name: "Baz".into(),
                                args: vec![]
                            }
                            .span(42..45)
                        }
                        .span(38..45),
                        Field {
                            name: "fizz".into(),
                            ty: Type::Named {
                                name: "Buzz".into(),
                                args: vec![]
                            }
                            .span(53..57)
                        }
                        .span(47..57)
                    ]
                )
                .span(34..57),
            ]
        }
        .span(1..102)
    )
}

#[test]
fn function_items() {
    assert_eq!(
        parse_item(r#"fn sum(mut a, b: Byte) -> a + b"#),
        Item::Func {
            name: "sum".into(),
            params: vec![
                Pattern::Var {
                    mutable: true,
                    ident: "a".into(),
                    annotated_ty: None
                }
                .span(7..12),
                Pattern::Var {
                    mutable: false,
                    ident: "b".into(),
                    annotated_ty: Some(Type::Byte.span(17..21))
                }
                .span(14..21)
            ],
            return_ty: None,
            body: Expr::BinaryOp {
                op: Bop::Add,
                lhs: Expr::Ident("a".into()).span(26..27).into(),
                rhs: Expr::Ident("b".into()).span(30..31).into()
            }
            .span(26..31)
        }
        .span(0..31)
    )
}

#[test]
fn malformed_items() {
    assert_eq!(
        parse_item_err("const fn: Int = 5"),
        Err(ParseError::Mismatched {
            expected: TT::Ident,
            found: TT::Fn,
        }
        .span(6..8))
    );

    assert_eq!(
        parse_item_err("const NO_DICTS: {String: Int} = 5"),
        Err(ParseError::Unexpected(TT::Indent, "start of type name").span(16..17))
    );

    assert_eq!(
        parse_item_err("let global = 0"),
        Err(ParseError::Unexpected(TT::Let, "start of item").span(0..3))
    );

    assert_eq!(
        parse_item_err("record CSyntax { Int five }"),
        Err(ParseError::Mismatched {
            expected: TT::Ident,
            found: TT::Int,
        }
        .span(17..20))
    );

    assert_eq!(
        parse_item_err("enum NoComma { Bad Syntax }"),
        Err(ParseError::Unexpected(
            TT::Ident,
            "after variant name. expected one of `,` `(` `{`"
        )
        .span(19..25))
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

        record Foo<T, U> {
            x: String,
            bar: Bar<Baz<T>, [U]>,
        }"#,
    );

    assert_eq!(
        items[0],
        Item::Func {
            name: "wow_we_did_it".into(),
            params: vec![
                Pattern::Var {
                    mutable: true,
                    ident: "x".into(),
                    annotated_ty: None
                }
                .span(26..31),
                Pattern::Var {
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
                                        .span(46..47)
                                    ],
                                }
                                .span(42..48),
                                Type::Named {
                                    name: "U".into(),
                                    args: vec![],
                                }
                                .span(50..51)
                            ],
                        }
                        .span(38..52)
                    )
                }
                .span(33..52)
            ],
            return_ty: Some(
                Type::Fn(
                    vec![Type::Int.span(58..61)],
                    Type::Int.span(64..67).into()
                )
                .span(55..67)
            ),
            body: Expr::Block {
                exprs: vec![
                    Expr::Let {
                        binding: Pattern::Var {
                            mutable: true,
                            ident: "x".into(),
                            annotated_ty: Some(
                                Type::Tuple(vec![
                                    Type::Bool.span(98..102),
                                    Type::Named {
                                        name: "T".into(),
                                        args: vec![]
                                    }
                                    .span(104..105)
                                ])
                                .span(96..106)
                            )
                        }
                        .span(89..106),
                        value: Expr::BinaryOp {
                            op: Bop::Add,
                            lhs: Expr::Bool(true).span(109..113).into(),
                            rhs: Expr::FnCall {
                                fun: Expr::Ident("sin".into()).span(116..119).into(),
                                args: vec![Expr::Ident("y".into()).span(120..121)]
                            }
                            .span(116..122)
                            .into()
                        }
                        .span(109..122)
                        .into()
                    }
                    .span(85..122),
                    Expr::Assign {
                        ident: Spnd {
                            inner: "x".into(),
                            span: (136..137).into()
                        },
                        value: Expr::If {
                            cond: Expr::BinaryOp {
                                op: Bop::Lt,
                                lhs: Expr::Ident("bar".into()).span(144..147).into(),
                                rhs: Expr::Int(3).span(150..151).into()
                            }
                            .span(144..151)
                            .into(),
                            th: Expr::Block {
                                exprs: vec![
                                    Expr::Let {
                                        binding: Pattern::Var {
                                            mutable: false,
                                            ident: "baz".into(),
                                            annotated_ty: None
                                        }
                                        .span(175..178),
                                        value: Expr::BinaryOp {
                                            op: Bop::Add,
                                            lhs: Expr::FieldAccess {
                                                base: Expr::Ident("bar".into())
                                                    .span(181..184)
                                                    .into(),
                                                field: Spnd {
                                                    inner: "value".into(),
                                                    span: (185..190).into()
                                                }
                                            }
                                            .span(181..190)
                                            .into(),
                                            rhs: Expr::BinaryOp {
                                                op: Bop::Mul,
                                                lhs: Expr::Int(2).span(193..194).into(),
                                                rhs: Expr::Int(4).span(197..198).into()
                                            }
                                            .span(193..198)
                                            .into()
                                        }
                                        .span(181..198)
                                        .into()
                                    }
                                    .span(171..198),
                                    Expr::BinaryOp {
                                        op: Bop::Add,
                                        lhs: Expr::Ident("x".into()).span(216..217).into(),
                                        rhs: Expr::Int(1).span(220..221).into()
                                    }
                                    .span(216..221)
                                ],
                                trailing: false
                            }
                            .span(153..236)
                            .into(),
                            el: Some(
                                Expr::If {
                                    cond: Expr::BinaryOp {
                                        op: Bop::Leq,
                                        lhs: Expr::Ident("bar".into()).span(246..249).into(),
                                        rhs: Expr::Int(2).span(253..254).into()
                                    }
                                    .span(246..254)
                                    .into(),
                                    th: Expr::FnCall {
                                        fun: Expr::Ident("fizz".into()).span(272..276).into(),
                                        args: vec![
                                            Expr::Int(3).span(277..278),
                                            Expr::Float(5.1).span(280..283)
                                        ]
                                    }
                                    .span(272..284)
                                    .into(),
                                    el: None
                                }
                                .span(242..284)
                                .into()
                            )
                        }
                        .span(140..284)
                        .into()
                    }
                    .span(136..284),
                ],
                trailing: true
            }
            .span(71..294)
        }
        .span(9..294)
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
                    .span(338..344),
                }
                .span(335..344),
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
                                    .span(371..372)
                                ],
                            }
                            .span(367..373),
                            Type::Array(
                                Type::Named {
                                    name: "U".into(),
                                    args: vec![],
                                }
                                .span(376..377)
                                .into()
                            )
                            .span(375..378)
                        ],
                    }
                    .span(363..379),
                }
                .span(358..379)
            ]
        }
        .span(304..390)
    );
}
