#[cfg(test)]
mod exprs;
// #[cfg(test)]
// mod items;

use ast::{
    AdtDef, Binding, ExecItem, ExprKind, Field, GenericParam, InfixOp, Param, Pat, Ty, TyKind,
};
use span::{Span, Spnd};

use crate::Parser;

// #[test]
// fn file() {
//     let mut interner = Default::default();

//     #[rustfmt::skip]
//     let input =
// r#"
// fn wow_we_did_it(mut x: Bool, bar: Bar<Baz<T>, U>): fn(Int): Int ->
//     let mut x: ( Bool, T) = true + sin(y)
//     x = if (bar < 3) then
//         let baz = bar.value + 2 * 4
//         x + 1
//     else if bar <= 2 then
//         fizz(3, 5.1)

// record Foo<T, U>(x: String, bar: Bar<Baz<T>, [U]>)
// "#;

//     let mut parser = Parser::new(input, &mut interner);

//     let items = parser.parse().unwrap();

//     //todo!("Fix Spans");
//     assert_eq!(
//         items.execs[0],
//         ExecItem::Func {
//             ident: parser.get_interned("wow_we_did_it"),
//             generic_params: vec![],
//             params: vec![
//                 Param {
//                     pat: Pat::Ident {
//                         mutable: true,
//                         ident: parser.get_interned("x"),
//                     },
//                     ty: None
//                 },
//                 Binding {
//                     pat: Pat::Ident {
//                         mutable: false,
//                         ident: parser.get_interned("bar"),
//                     },
//                     ty: Some(Ty {
//                         kind: TyKind::Adt {
//                             ident: parser.get_interned("Bar"),
//                             args: vec![
//                                 Ty {
//                                     kind: TyKind::Adt {
//                                         ident: parser.get_interned("Baz"),
//                                         args: vec![Ty {
//                                             kind: TyKind::Adt {
//                                                 ident: parser.get_interned("T"),
//                                                 args: vec![],
//                                             },
//                                             span: Span::from(46..47)
//                                         }],
//                                     },
//                                     span: Span::from(42..48)
//                                 },
//                                 Ty {
//                                     kind: TyKind::Adt {
//                                         ident: parser.get_interned("U"),
//                                         args: vec![],
//                                     },
//                                     span: Span::from(50..51)
//                                 }
//                             ],
//                         },
//                         span: Span::from(38..52)
//                     })
//                 }
//             ],
//             return_ty: Some(Ty {
//                 kind: TyKind::Fn(
//                     vec![Ty {
//                         kind: TyKind::Int,
//                         span: Span::from(58..61)
//                     }],
//                     Ty {
//                         kind: TyKind::Int,
//                         span: Span::from(64..67)
//                     }
//                     .into()
//                 ),
//                 span: Span::from(55..67)
//             }),
//             body: ExprKind::Block(vec![
//                 ExprKind::Let {
//                     binding: Binding {
//                         pat: Pat::Ident {
//                             mutable: true,
//                             ident: parser.get_interned("x"),
//                         },
//                         ty: Some(Ty {
//                             kind: TyKind::Tuple(vec![
//                                 Ty {
//                                     kind: TyKind::Bool,
//                                     span: Span::from(98..102)
//                                 },
//                                 Ty {
//                                     kind: TyKind::Adt {
//                                         ident: parser.get_interned("T"),
//                                         args: vec![]
//                                     },
//                                     span: Span::from(104..105)
//                                 }
//                             ]),
//                             span: Span::from(96..106)
//                         })
//                     },
//                     val: ExprKind::InfixExpr {
//                         op: InfixOp::Add,
//                         lhs: ExprKind::Bool(true).span(109..113).into(),
//                         rhs: ExprKind::CallExpr {
//                             func: ExprKind::Ident(parser.get_interned("sin"))
//                                 .span(116..119)
//                                 .into(),
//                             args: vec![ExprKind::Ident(parser.get_interned("y")).span(120..121)]
//                         }
//                         .span(116..122)
//                         .into()
//                     }
//                     .span(109..122)
//                     .into()
//                 }
//                 .span(85..122),
//                 ExprKind::Assign {
//                     ident: Spnd(parser.get_interned("x"), (136..137).into()),
//                     val: ExprKind::If {
//                         cond: ExprKind::InfixExpr {
//                             op: InfixOp::Lt,
//                             lhs: ExprKind::Ident(parser.get_interned("bar"))
//                                 .span(144..147)
//                                 .into(),
//                             rhs: ExprKind::Int(3).span(150..151).into()
//                         }
//                         .span(144..151)
//                         .into(),
//                         th: ExprKind::Block(vec![
//                             ExprKind::Let {
//                                 binding: Binding {
//                                     pat: Pat::Ident {
//                                         mutable: false,
//                                         ident: parser.get_interned("baz"),
//                                     },
//                                     ty: None
//                                 },
//                                 val: ExprKind::InfixExpr {
//                                     op: InfixOp::Add,
//                                     lhs: ExprKind::FieldExpr {
//                                         base: ExprKind::Ident(parser.get_interned("bar"))
//                                             .span(181..184)
//                                             .into(),
//                                         field: Spnd(
//                                             parser.get_interned("value"),
//                                             (185..190).into()
//                                         )
//                                     }
//                                     .span(181..190)
//                                     .into(),
//                                     rhs: ExprKind::InfixExpr {
//                                         op: InfixOp::Mul,
//                                         lhs: ExprKind::Int(2).span(193..194).into(),
//                                         rhs: ExprKind::Int(4).span(197..198).into()
//                                     }
//                                     .span(193..198)
//                                     .into()
//                                 }
//                                 .span(181..198)
//                                 .into()
//                             }
//                             .span(171..198),
//                             ExprKind::InfixExpr {
//                                 op: InfixOp::Add,
//                                 lhs: ExprKind::Ident(parser.get_interned("x"))
//                                     .span(216..217)
//                                     .into(),
//                                 rhs: ExprKind::Int(1).span(220..221).into()
//                             }
//                             .span(216..221)
//                         ])
//                         .span(153..236)
//                         .into(),
//                         el: Some(
//                             ExprKind::If {
//                                 cond: ExprKind::InfixExpr {
//                                     op: InfixOp::Leq,
//                                     lhs: ExprKind::Ident(parser.get_interned("bar"))
//                                         .span(246..249)
//                                         .into(),
//                                     rhs: ExprKind::Int(2).span(253..254).into()
//                                 }
//                                 .span(246..254)
//                                 .into(),
//                                 th: ExprKind::CallExpr {
//                                     func: ExprKind::Ident(parser.get_interned("fizz"))
//                                         .span(272..276)
//                                         .into(),
//                                     args: vec![
//                                         ExprKind::Int(3).span(277..278),
//                                         ExprKind::Float(5.1).span(280..283)
//                                     ]
//                                 }
//                                 .span(272..284)
//                                 .into(),
//                                 el: None
//                             }
//                             .span(242..284)
//                             .into()
//                         )
//                     }
//                     .span(140..284)
//                     .into()
//                 }
//                 .span(136..284),
//             ])
//             .span(71..294)
//         }
//     );

//     assert_eq!(
//         items.adts[0],
//         Item::Record {
//             def: AdtDef {
//                 ident: parser.get_interned("Foo"),
//                 generics: vec![
//                     GenericParam(Spnd::span(parser.get_interned("T"), 315..316)),
//                     GenericParam(Spnd::span(parser.get_interned("U"), 318..320)),
//                 ]
//             },
//             fields: vec![
//                 Field {
//                     ident: parser.get_interned("x"),
//                     ty: Ty {
//                         kind: TyKind::Adt {
//                             ident: parser.get_interned("String"),
//                             args: vec![],
//                         },
//                         span: Span::from(338..344)
//                     },
//                     span: Span::from(0..0)
//                 },
//                 Field {
//                     ident: parser.get_interned("bar"),
//                     ty: Ty {
//                         kind: TyKind::Adt {
//                             ident: parser.get_interned("Bar"),
//                             args: vec![
//                                 Ty {
//                                     kind: TyKind::Adt {
//                                         ident: parser.get_interned("Baz"),
//                                         args: vec![Ty {
//                                             kind: TyKind::Adt {
//                                                 ident: parser.get_interned("T"),
//                                                 args: vec![],
//                                             },
//                                             span: Span::from(371..372)
//                                         }],
//                                     },
//                                     span: Span::from(367..373)
//                                 },
//                                 Ty {
//                                     kind: TyKind::Array(Box::new(Ty {
//                                         kind: TyKind::Adt {
//                                             ident: parser.get_interned("U"),
//                                             args: vec![],
//                                         },
//                                         span: Span::from(376..377)
//                                     })),
//                                     span: Span::from(375..378)
//                                 }
//                             ],
//                         },
//                         span: Span::from(363..379)
//                     },
//                     span: Span::from(0..0)
//                 }
//             ]
//         }
//     );
// }
