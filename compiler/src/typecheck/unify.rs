use std::{cmp, convert::Infallible, iter};

use ena::unify::{UnifyKey, UnifyValue};

use super::{Type, TypeChecker, TypeError, TypeId, TypeResult, TypeS};
use crate::helpers::Spanned;

macro_rules! spanned {
    ($pattern: pat) => {
        Spanned {
            inner: $pattern,
            ..
        }
    };
}

impl UnifyValue for Type {
    type Error = Infallible;

    fn unify_values(a: &Self, b: &Self) -> Result<Self, Self::Error> {
        match (a, b) {
            (Type::IntVar(id_a), Type::IntVar(id_b)) => {
                Ok(Type::IntVar(cmp::min(id_a.index(), id_b.index()).into()))
            }
            (Type::IntVar(id_a), Type::Var(id_b)) | (Type::Var(id_b), Type::IntVar(id_a)) => {
                Ok(Type::IntVar(cmp::min(id_a.index(), id_b.index()).into()))
            }
            (Type::Var(id_a), Type::Var(id_b)) => {
                Ok(Type::Var(cmp::min(id_a.index(), id_b.index()).into()))
            }
            (Type::IntVar(_), integer @ (Type::Int | Type::UInt | Type::Byte))
            | (integer @ (Type::Int | Type::UInt | Type::Byte), Type::IntVar(_)) => {
                Ok(integer.clone())
            }
            (ty, Type::Var(_)) | (Type::Var(_), ty) => Ok(ty.clone()),
            (ty_a, ty_b) => {
                panic!("shouldn't be unifying two concrete types {ty_a:?} and {ty_b:?}")
            }
        }
    }
}

impl TypeChecker {
    pub fn unify(&mut self, ty_a: &TypeS, ty_b: &TypeS) -> TypeResult<()> {
        if let Some(n_a) = self.normalize(&ty_a) {
            return self.unify(&n_a, ty_b);
        } else if let Some(n_b) = self.normalize(&ty_b) {
            return self.unify(ty_a, &n_b);
        }

        match (ty_a, ty_b) {
            (spanned! {Type::Int}, spanned! {Type::Int})
            | (spanned! {Type::UInt}, spanned! {Type::UInt})
            | (spanned! {Type::Byte}, spanned! {Type::Byte})
            | (spanned! {Type::Float}, spanned! {Type::Float})
            | (spanned! {Type::Bool}, spanned! {Type::Bool})
            | (spanned! {Type::Char}, spanned! {Type::Char}) => Ok(()),
            (spanned! {Type::Array(ty_a)}, spanned! {Type::Array(ty_b)}) => self.unify(ty_a, ty_b),
            (spanned! {Type::Tuple(tys_a)}, spanned! {Type::Tuple(tys_b)}) => {
                self.unify_all(tys_a, tys_b)
            }
            (
                spanned! {Type::Func(param_tys_a, return_ty_a)},
                spanned! {Type::Func(param_tys_b, return_ty_b)},
            ) => self
                .unify(return_ty_a, return_ty_b)
                .and_then(|()| self.unify_all(param_tys_a, param_tys_b)),
            (
                spanned! {Type::Var(a_id) | Type::IntVar(a_id)},
                spanned! {Type::Var(b_id) | Type::IntVar(b_id)},
            ) => Ok(self.table.unify_var_var(*a_id, *b_id).expect("infallible")),
            (
                spanned! {Type::Named { name: name_a, generics: args_a }},
                spanned! {Type::Named { name: name_b, generics: args_b }},
            ) if name_a == name_b => self.unify_all(args_a, args_b),
            (spanned! {Type::Var(id)}, bound_ty) | (bound_ty, spanned! {Type::Var(id)}) => {
                if !self.occurs((*id).into(), &bound_ty) {
                    Ok(self
                        .table
                        .unify_var_value(*id, bound_ty.inner.clone())
                        .expect("infallible"))
                } else {
                    Err(TypeError::Infinite.spanned(bound_ty.span))
                }
            }
            (
                spanned! {Type::IntVar(id)},
                integer @ spanned!(Type::Int | Type::UInt | Type::Byte),
            )
            | (
                integer @ spanned!(Type::Int | Type::UInt | Type::Byte),
                spanned! {Type::IntVar(id)},
            ) => Ok(self
                .table
                .unify_var_value(*id, integer.inner.clone())
                .expect("infallible")),
            (ty_a, ty_b) => Err(TypeError::MismatchedTypes(ty_a.clone(), ty_b.clone())
                .spanned(((*ty_a).span.start)..((*ty_b).span.end))),
        }
    }

    fn unify_all(&mut self, tys_a: &[TypeS], tys_b: &[TypeS]) -> TypeResult<()> {
        iter::zip(tys_a, tys_b).try_for_each(|(a, b)| self.unify(a, b))
    }

    fn occurs(&mut self, var: TypeId, ty: &TypeS) -> bool {
        if let Some(n_ty) = self.normalize(ty) {
            return self.occurs(var, &n_ty);
        };

        match &ty.inner {
            Type::Named { generics: args, .. } => args.iter().any(|ty| self.occurs(var, ty)),
            Type::Var(_) | Type::IntVar(_) => false,
            Type::Int | Type::UInt | Type::Byte | Type::Float | Type::Bool | Type::Char => false,
            Type::Array(inner_ty) => self.occurs(var, inner_ty),
            Type::Tuple(tys) => tys.iter().any(|ty| self.occurs(var, ty)),
            Type::Func(param_tys, result_ty) => {
                self.occurs(var, result_ty) || param_tys.iter().any(|ty| self.occurs(var, ty))
            }
        }
    }
}
