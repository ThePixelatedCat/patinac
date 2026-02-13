use std::iter;

use crate::{
    helpers::{SpanErr, Spnd},
    parser::ast::{ExprS, Item, Pattern, PatternS},
};

use super::{BindingInfo, Type, TypeChecker, TypeErrorS};

impl TypeChecker {
    pub fn check(&mut self, ast: &[Item]) -> Result<(), TypeErrorS> {
        for item in ast {
            match &item {
                Item::Const { name, ty, .. } => {
                    self.env.insert(
                        name.clone(),
                        BindingInfo {
                            ty: self.convert(ty),
                            mutable: false,
                        },
                    );
                }
                Item::Func {
                    name,
                    params,
                    return_ty,
                    ..
                } => {
                    self.env.insert(
                        name.clone(),
                        BindingInfo {
                            ty: Type::Fn(
                                params
                                    .iter()
                                    .map(
                                        |Spnd {
                                             inner: Pattern::Var { annotated_ty, .. },
                                             ..
                                         }| {
                                            self.convert(annotated_ty)
                                        },
                                    )
                                    .collect(),
                                Box::new(self.convert(return_ty)),
                            ),
                            mutable: false,
                        },
                    );
                }
                Item::Record {
                    def,
                    fields,
                } => todo!(),
                Item::Enum {
                    def,
                    variants,
                } => todo!(),
            }
        }

        for item in ast {
            match &item {
                Item::Const { name, value, .. } => self.check_const(name, value)?,
                Item::Func {
                    name, params, body, ..
                } => self.check_func(name, params, body)?,
                Item::Record {
                    def,
                    fields,
                } => todo!(),
                Item::Enum {
                    def,
                    variants,
                } => todo!(),
            }
        }

        Ok(())
    }

    fn check_const(&self, name: &str, value: &ExprS) -> Result<(), TypeErrorS> {
        let BindingInfo { ty: binding_ty, .. } = self
            .get_binding(name)
            .expect("was added to env in initial iteration");

        let val_ty = self.clone().type_of(value)?;

        self.unify(binding_ty, &val_ty)
            .span_err(value.span)
    }

    fn check_func(&self, name: &str, params: &[PatternS], body: &ExprS) -> Result<(), TypeErrorS> {
        let BindingInfo {
            ty: Type::Fn(param_tys, return_ty),
            ..
        } = self
            .get_binding(name)
            .expect("was added to env in initial iteration")
        else {
            unreachable!("type of previously-added binding is always a function")
        };

        let mut snapshot = self.clone();

        iter::zip(params, param_tys).for_each(
            |(
                Spnd {
                    inner: Pattern::Var { mutable, ident, .. },
                    ..
                },
                ty,
            )| {
                snapshot.env.insert(
                    ident.clone(),
                    BindingInfo {
                        ty: ty.clone(),
                        mutable: *mutable,
                    },
                );
            },
        );

        let body_ty = snapshot.type_of(body)?;

        self.unify(return_ty, &body_ty)
            .span_err(body.span)
    }
}
