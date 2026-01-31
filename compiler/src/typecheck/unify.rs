use std::iter;

use super::{Type, TypeChecker, TypeError, TypeResult, TypeS};
use crate::helpers::Spanned;

macro_rules! spanned {
    ($pattern: pat) => {
        Spanned {
            inner: $pattern,
            ..
        }
    };
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
}
