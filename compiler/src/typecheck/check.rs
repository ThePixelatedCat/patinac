use std::iter;

use crate::{
    helpers::Spanned,
    parser::ast::{Pattern, PatternS, ExprS, Item, ItemS},
};

use super::{BindingInfo, Type, TypeChecker, TypeErrorS};

impl TypeChecker {
    pub fn check(&mut self, ast: &[ItemS]) -> Result<(), TypeErrorS> {
        for item in ast {
            match &item.inner {
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
                                        |Spanned {
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
                Item::Struct {
                    name,
                    generic_params,
                    fields,
                } => todo!(),
                Item::Enum {
                    name,
                    generic_params,
                    variants,
                } => todo!(),
            }
        }

        for item in ast {
            match &item.inner {
                Item::Const { name, value, .. } => self.check_const(name, value)?,
                Item::Func {
                    name, params, body, ..
                } => self.check_func(name, params, body)?,
                Item::Struct {
                    name,
                    generic_params,
                    fields,
                } => todo!(),
                Item::Enum {
                    name,
                    generic_params,
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
            .map_err(|e| e.spanned(value.span))
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
                Spanned {
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
            .map_err(|e| e.spanned(body.span))
    }
}
