use ast::{
    AdtDef, AdtItem, Binding, ExecItem, ExprKind, Field, GenericParam, InfixOp, Param, Pat, Ty,
    TyKind, Variant,
};
use lex::TokKind;
use span::{Span, Spannable};

use crate::{ParseError, Parser, items::Item};

#[test]
fn const_items() {
    let mut interner = Default::default();

    let mut parser = Parser::new(
        r#"const hello_world: String = "Hello, World!""#,
        &mut interner,
    );
    assert_eq!(
        parser.item(),
        Ok(Item::ExecItem(ExecItem::Const {
            ident: parser.get_interned("hello_world"),
            ty: Some(Ty {
                kind: TyKind::Adt(parser.get_interned("String"), vec![]),
                span: Span::from(19..25)
            }),
            value: ExprKind::string("Hello, World!").span(28..43)
        }))
    );

    let mut parser = Parser::new(r#"const id = fn(x) -> x"#, &mut interner);
    assert_eq!(
        parser.item(),
        Ok(Item::ExecItem(ExecItem::Const {
            ident: parser.get_interned("id"),
            ty: None,
            value: ExprKind::LambdaExpr {
                params: vec![Binding {
                    mutable: false,
                    pat: Pat::Ident {
                        ident: parser.get_interned("x"),
                        subpat: None
                    },
                    ty: None
                }],
                return_ty: None,
                body: ExprKind::ident(parser.get_interned("x"))
                    .span(20..21)
                    .into()
            }
            .span(11..21)
        }))
    );
}

#[test]
fn struct_items() {
    let mut interner = Default::default();

    let mut parser = Parser::new("record Point(x: Int, y: Int)", &mut interner);
    assert_eq!(
        parser.item(),
        Ok(Item::AdtItem(AdtItem::Record {
            def: AdtDef {
                ident: parser.get_interned("Point"),
                generics: vec![]
            },
            fields: vec![
                Field {
                    ident: parser.get_interned("x"),
                    ty: Ty {
                        kind: TyKind::Int,
                        span: Span::from(16..19)
                    },
                    span: Span::from(13..19)
                },
                Field {
                    ident: parser.get_interned("y"),
                    ty: Ty {
                        kind: TyKind::Int,
                        span: Span::from(24..27)
                    },
                    span: Span::from(21..27)
                }
            ]
        }))
    );

    let input = "
record Foo[T, U](
    x: Char , 
    bar: Bar[Baz[T]]
    )";
    let mut parser = Parser::new(input, &mut interner);
    assert_eq!(
        parser.item(),
        Ok(Item::AdtItem(AdtItem::Record {
            def: AdtDef {
                ident: parser.get_interned("Foo"),
                generics: vec![
                    GenericParam(parser.get_interned("T")),
                    GenericParam(parser.get_interned("U")),
                ]
            },
            fields: vec![
                Field {
                    ident: parser.get_interned("x"),
                    ty: Ty {
                        kind: TyKind::Char,
                        span: Span::from(26..30)
                    },
                    span: Span::from(23..30)
                },
                Field {
                    ident: parser.get_interned("bar"),
                    ty: Ty {
                        kind: TyKind::Adt(
                            parser.get_interned("Bar"),
                            vec![Ty {
                                kind: TyKind::Adt(
                                    parser.get_interned("Baz"),
                                    vec![Ty {
                                        kind: TyKind::Adt(parser.get_interned("T"), vec![]),
                                        span: Span::from(51..52)
                                    }]
                                ),
                                span: Span::from(47..53)
                            }]
                        ),
                        span: Span::from(43..54)
                    },
                    span: Span::from(38..54)
                }
            ]
        }))
    );
}

#[test]
fn enum_items() {
    let mut interner = Default::default();

    let input = r#"
enum Foo
    | X()
        | Y(v: Bar)
| Z(baz: Baz, fizz: Buzz)
"#;

    let mut parser = Parser::new(input, &mut interner);
    assert_eq!(
        parser.item(),
        Ok(Item::AdtItem(AdtItem::Enum {
            def: AdtDef {
                ident: parser.get_interned("Foo"),
                generics: vec![]
            },
            variants: vec![
                Variant {
                    ident: parser.get_interned("X"),
                    fields: vec![]
                },
                Variant {
                    ident: parser.get_interned("Y"),
                    fields: vec![Field {
                        ident: parser.get_interned("v"),
                        ty: Ty {
                            kind: TyKind::Adt(parser.get_interned("Bar"), vec![]),
                            span: Span::from(35..38)
                        },
                        span: Span::from(32..38)
                    }],
                },
                Variant {
                    ident: parser.get_interned("Z"),
                    fields: vec![
                        Field {
                            ident: parser.get_interned("baz"),
                            ty: Ty {
                                kind: TyKind::Adt(parser.get_interned("Baz"), vec![]),
                                span: Span::from(49..52)
                            },
                            span: Span::from(44..52)
                        },
                        Field {
                            ident: parser.get_interned("fizz"),
                            ty: Ty {
                                kind: TyKind::Adt(parser.get_interned("Buzz"), vec![]),
                                span: Span::from(60..64)
                            },
                            span: Span::from(54..64)
                        },
                    ]
                },
            ]
        }))
    )
}

#[test]
fn function_items() {
    let mut interner = Default::default();

    let mut parser = Parser::new(
        "fn sum(mut a: Byte, b: Byte): {} -> a = a + b",
        &mut interner,
    );
    assert_eq!(
        parser.item(),
        Ok(Item::ExecItem(ExecItem::Func {
            ident: parser.get_interned("sum"),
            generic_params: vec![],
            params: vec![
                Param {
                    mutable: true,
                    pat: Pat::Ident {
                        ident: parser.get_interned("a"),
                        subpat: None
                    },
                    ty: Ty {
                        kind: TyKind::Byte,
                        span: Span::from(14..18)
                    }
                },
                Param {
                    mutable: false,
                    pat: Pat::Ident {
                        ident: parser.get_interned("b"),
                        subpat: None
                    },
                    ty: Ty {
                        kind: TyKind::Byte,
                        span: Span::from(23..27)
                    }
                }
            ],
            return_ty: Ty {
                kind: TyKind::Tuple(vec![]),
                span: Span::from(30..32)
            },
            body: ExprKind::Assign {
                place: Box::new(ExprKind::ident(parser.get_interned("a")).span(36..37)),
                val: Box::new(
                    ExprKind::InfixExpr {
                        op: InfixOp::Add,
                        lhs: Box::new(ExprKind::ident(parser.get_interned("a")).span(40..41)),
                        rhs: Box::new(ExprKind::ident(parser.get_interned("b")).span(44..45))
                    }
                    .span(40..45)
                )
            }
            .span(36..45)
        }))
    )
}

#[test]
fn malformed_items() {
    assert_eq!(
        Parser::new("const fn: Int = 5", &mut Default::default()).item(),
        Err(ParseError::Mismatched {
            expected: TokKind::Ident,
            found: TokKind::Fn,
        }
        .span(6..8))
    );

    assert_eq!(
        Parser::new("const NO_DICTS: [String: Int] = 5", &mut Default::default()).item(),
        Err(ParseError::Mismatched {
            expected: TokKind::RBracket,
            found: TokKind::Colon
        }
        .span(23..24))
    );

    assert_eq!(
        Parser::new("let global = false", &mut Default::default()).item(),
        Err(ParseError::Unexpected(TokKind::Let, "start of item").span(0..3))
    );
}
