mod error;
mod infer;
#[cfg(test)]
mod test;
mod types;
mod unify;

use std::{cell::RefCell, rc::Rc};

use ena::unify::{InPlace, UnificationTable, UnifyKey};
use im::HashMap;

use crate::{
    helpers::Spanned,
    parser::ast::{Ast, Binding, Item},
    typecheck::{
        error::TypeErrorS,
        types::{Type, TypeId},
    },
};

use error::TypeError;

#[derive(Clone)]
pub struct BindingInfo {
    ty: Type,
    mutable: bool,
}

#[derive(Clone, Default)]
pub struct TypeChecker {
    env: HashMap<String, BindingInfo>,
    table: Rc<RefCell<UnificationTable<InPlace<TypeId>>>>,
}

impl TypeChecker {
    fn get_binding(&self, ident: &str) -> Result<&BindingInfo, TypeError> {
        self.env
            .get(ident)
            .ok_or_else(|| TypeError::UnboundIdent(ident.to_owned()))
    }

    #[allow(clippy::cast_possible_truncation)]
    fn fresh_int_var(&self) -> Type {
        let id = self.table.borrow_mut().len() as u32;
        let var = Type::IntVar(TypeId::from_index(id));
        self.table.borrow_mut().new_key(var.clone());
        var
    }

    #[allow(clippy::cast_possible_truncation)]
    fn fresh_var(&self) -> Type {
        let id = self.table.borrow_mut().len() as u32;
        let var = Type::Var(TypeId::from_index(id));
        self.table.borrow_mut().new_key(var.clone());
        var
    }

    fn normalise_id(&self, ty: &Type) -> Option<Type> {
        ty.id()
            .and_then(|id| match self.table.borrow_mut().probe_value(id) {
                Type::Var(_) | Type::IntVar(_) => None,
                bound_ty => Some(bound_ty),
            })
    }

    fn normalise(&self, ty: Type) -> Type {
        match ty {
            Type::Int | Type::UInt | Type::Byte | Type::Float | Type::Bool | Type::Char => ty,
            Type::Array(mut ty) => {
                *ty = self.normalise(*ty);
                Type::Array(ty)
            }
            Type::Tuple(tys) => Type::Tuple(tys.into_iter().map(|ty| self.normalise(ty)).collect()),
            Type::Fn(param_tys, mut return_ty) => {
                *return_ty = self.normalise(*return_ty);
                Type::Fn(
                    param_tys.into_iter().map(|ty| self.normalise(ty)).collect(),
                    return_ty,
                )
            }
            Type::Var(id) | Type::IntVar(id) => self.table.borrow_mut().probe_value(id),
            Type::Named { name, args } => Type::Named {
                name,
                args: args.into_iter().map(|ty| self.normalise(ty)).collect(),
            },
        }
    }

    pub fn new(ast: &Ast) -> Self {
        Self {
            env: HashMap::new(),
            table: Rc::default(),
        }
    }

    pub fn check(&mut self, ast: &Ast) -> Result<(), TypeErrorS> {
        for item in ast {
            match &item.inner {
                Item::Const { name, ty, .. } => {
                    self.env.insert(
                        name.clone(),
                        BindingInfo {
                            ty: ty.inner.clone().into(),
                            mutable: false,
                        },
                    );
                }
                Item::Function {
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
                                             inner: Binding::Var { annotated_ty, .. },
                                             ..
                                         }| {
                                            annotated_ty.as_ref().map_or_else(
                                                || self.fresh_var(),
                                                |ty| ty.inner.clone().into(),
                                            )
                                        },
                                    )
                                    .collect(),
                                Box::new(
                                    return_ty.as_ref().map_or_else(
                                        || self.fresh_var(),
                                        |ty| ty.inner.clone().into(),
                                    ),
                                ),
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
        };

        for item in ast {
            match &item.inner {
                Item::Const { name, ty, value } => {
                    let val_ty = self.clone().type_of(value)?;
                    let BindingInfo { ty: binding_ty, .. } = self.get_binding(name).expect("was added to env in initial iteration");
                    self.unify(binding_ty, &val_ty).map_err(|e| e.spanned(value.span))?;
                },
                Item::Function { name, body, .. } => {
                    let body_ty = self.clone().type_of(body)?;
                    let BindingInfo { ty: binding_ty, .. } = self.get_binding(name).expect("was added to env in initial iteration");
                    self.unify(binding_ty, &body_ty).map_err(|e| e.spanned(body.span))?;
                },
                Item::Struct { name, generic_params, fields } => todo!(),
                Item::Enum { name, generic_params, variants } => todo!(),
            }
        }

        Ok(())
    }
}
