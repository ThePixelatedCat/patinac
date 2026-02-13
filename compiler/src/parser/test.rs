#[cfg(test)]
mod exprs;
#[cfg(test)]
mod items;

use crate::helpers::{Spannable, Spnd};
use crate::parser::ast::TypeDef;

use super::Parser;
use super::ast::{Bop, Expr, Field, Item, Pattern, Type};

fn parse_ast(input: &str) -> Vec<Item> {
    let mut parser = Parser::new(input);
    parser.file().unwrap()
}

#[test]
fn file() {
    todo!("Fix Spans");

    #[rustfmt::skip]
    let items = parse_ast(
r#"
fn wow_we_did_it(mut x, bar: Bar<Baz<T>, U>): fn(Int): Int -> 
    let mut x: ( Bool, T) = true + sin(y)
    x = if (bar < 3) then
        let baz = bar.value + 2 * 4
        x + 1
    else if bar <= 2 then
        fizz(3, 5.1)

record Foo<T, U> 
    x: String,
    bar: Bar<Baz<T>, [U]>,
"#,
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
                Type::Fn(vec![Type::Int.span(58..61)], Type::Int.span(64..67).into()).span(55..67)
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
    );

    assert_eq!(
        items[1],
        Item::Record {
            def: TypeDef {
                name: "Foo".into(),
                generic_params: vec![
                    String::from("T").span(315..316),
                    String::from("U").span(318..320),
                ]
            },
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
    );
}
