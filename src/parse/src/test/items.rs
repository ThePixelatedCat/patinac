use std::{assert_matches, range::Range};

use pretty_assertions::assert_eq;

use ident::Ident;
use irs::ast::{
    ExecItem, ExecKind, ExprKind, Field, Impl, InfixOp, Param, PatKind, Path, TyItem, TyItemKind,
    TyKind, Variant, VisItem,
};

use crate::{Parser, items::Item};

#[test]
fn vis_items() {
    assert_eq!(
        Parser::new_test("import foo::bar::baz",).item(),
        Ok(Item::VisItem(VisItem::Import(
            Path::new_const([Ident::new("foo"), Ident::new("bar"), Ident::new("baz")]),
            Range::from(7..20)
        )))
    );

    assert_eq!(
        Parser::new_test("export { foo, bar, baz, }",).item(),
        Ok(Item::VisItem(VisItem::Export(vec![
            Ident::new("foo").span(9..12),
            Ident::new("bar").span(14..17),
            Ident::new("baz").span(19..22)
        ])))
    );
}

#[test]
fn records() {
    assert_eq!(
        Parser::new_test("opaque record Point(x: Int, y: Int)",).item(),
        Ok(Item::TyItem(TyItem {
            opaque: true,
            ident: Ident::new("Point").span(14..19),
            generics: vec![],
            kind: TyItemKind::Record(vec![
                Field {
                    ident: Ident::new("x").span(20..21),
                    ty: TyKind::Int.span(23..26),
                },
                Field {
                    ident: Ident::new("y").span(28..29),
                    ty: TyKind::Int.span(31..34),
                }
            ])
        }))
    );

    let input = "
record Foo[T, U](
    x: Bool , 
    bar: Bar[Baz[T]]
    )";
    assert_eq!(
        Parser::new_test(input).item(),
        Ok(Item::TyItem(TyItem {
            opaque: false,
            ident: Ident::new("Foo").span(8..11),
            generics: vec![Ident::new("T").span(12..13), Ident::new("U").span(15..16),],
            kind: TyItemKind::Record(vec![
                Field {
                    ident: Ident::new("x").span(23..24),
                    ty: TyKind::Bool.span(26..30),
                },
                Field {
                    ident: Ident::new("bar").span(38..41),
                    ty: TyKind::Named(
                        Path::single(Ident::new("Bar")),
                        vec![
                            TyKind::Named(
                                Path::single(Ident::new("Baz")),
                                vec![TyKind::named("T").span(51..52)]
                            )
                            .span(47..53)
                        ]
                    )
                    .span(43..54),
                }
            ])
        }))
    );
}

#[test]
fn unions() {
    let input = "
union XY {
    X(),
    Y(baz: Baz, fizz: Buzz),
}
";

    assert_eq!(
        Parser::new_test(input).item(),
        Ok(Item::TyItem(TyItem {
            opaque: false,
            ident: Ident::new("XY").span(7..9),
            generics: vec![],
            kind: TyItemKind::Union(vec![
                Variant {
                    ident: Ident::new("X").span(16..17),
                    fields: vec![]
                },
                Variant {
                    ident: Ident::new("Y").span(25..26),
                    fields: vec![
                        Field {
                            ident: Ident::new("baz").span(27..30),
                            ty: TyKind::named("Baz").span(32..35),
                        },
                        Field {
                            ident: Ident::new("fizz").span(37..41),
                            ty: TyKind::named("Buzz").span(43..47),
                        },
                    ]
                },
            ])
        }))
    );
}

#[test]
fn impls() {
    let input = "
    impl Foo {
        fn bar(mut self): () = ()
        const BAZ: () = ()
    }
";
    assert_eq!(
        Parser::new_test(input).item(),
        Ok(Item::Impl(Impl {
            ty: Ident::new("Foo").span(10..13),
            items: vec![
                ExecItem {
                    ident: Ident::new("bar").span(27..30),
                    kind: ExecKind::Func {
                        generics: vec![],
                        self_param: Some((true, Range::from(31..39))),
                        params: vec![],
                        ret_ty: TyKind::unit().span(42..44),
                        body: ExprKind::unit().span(47..49)
                    }
                },
                ExecItem {
                    ident: Ident::new("BAZ").span(64..67),
                    kind: ExecKind::Const {
                        ty: TyKind::unit().span(69..71),
                        val: ExprKind::unit().span(74..76)
                    }
                }
            ]
        }))
    )
}

#[test]
fn consts() {
    assert_eq!(
        Parser::new_test(r#"const hello_world: String = "Hello, World!""#,).item(),
        Ok(Item::ExecItem(ExecItem {
            ident: Ident::new("hello_world").span(6..17),
            kind: ExecKind::Const {
                ty: TyKind::named("String").span(19..25),
                val: ExprKind::string("Hello, World!").span(28..43)
            },
        }))
    );
}

#[test]
fn functions() {
    assert_eq!(
        Parser::new_test("fn sum(mut a: Byte, b: Byte): () = a = a + b").item(),
        Ok(Item::ExecItem(ExecItem {
            ident: Ident::new("sum").span(3..6),
            kind: ExecKind::Func {
                generics: vec![],
                self_param: None,
                params: vec![
                    Param {
                        pat: PatKind::ident("a").span(11..12),
                        ty: TyKind::Byte.span(14..18),
                        mutable: true,
                        span: Range::from(7..18)
                    },
                    Param {
                        pat: PatKind::ident("b").span(20..21),
                        ty: TyKind::Byte.span(23..27),
                        mutable: false,
                        span: Range::from(20..27)
                    }
                ],
                ret_ty: TyKind::unit().span(30..32),
                body: ExprKind::Infix {
                    op: InfixOp::Assign,
                    lhs: ExprKind::ident("a").span(35..36).into(),
                    rhs: ExprKind::Infix {
                        op: InfixOp::Add,
                        lhs: ExprKind::ident("a").span(39..40).into(),
                        rhs: ExprKind::ident("b").span(43..44).into()
                    }
                    .span(39..44)
                    .into()
                }
                .span(35..44)
            }
        }))
    );
}

#[test]
fn primitive_types() {
    assert_eq!(
        Parser::new_test("const Int: Int = 0").item(),
        Ok(Item::ExecItem(ExecItem {
            ident: Ident::new("Int").span(6..9),
            kind: ExecKind::Const {
                ty: TyKind::Int.span(11..14),
                val: ExprKind::int(0).span(17..18)
            }
        }))
    );

    assert_matches!(Parser::new_test("const Int: Int<Int> = 0").item(), Err(_));
}

#[test]
fn malformed_items() {
    assert_matches!(Parser::new_test("const fn: Int = 5").item(), Err(_));
    assert_matches!(
        Parser::new_test("const NO_DICTS: [String: Int] = 5").item(),
        Err(_)
    );
    assert_matches!(Parser::new_test("let global = false").item(), Err(_));
    assert_matches!(
        Parser::new_test("fn foo(a: Int, self): () = ()").item(),
        Err(_)
    )
}
