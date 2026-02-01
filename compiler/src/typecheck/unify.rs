use std::{cmp, convert::Infallible, iter};

use ena::unify::{UnifyKey, UnifyValue};

use super::{Type, TypeChecker, TypeError, TypeId};

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
    pub fn unify(&mut self, ty_a: &Type, ty_b: &Type) -> Result<(), TypeError> {
        if let Some(n_a) = self.normalize(&ty_a) {
            return self.unify(&n_a, ty_b);
        } else if let Some(n_b) = self.normalize(&ty_b) {
            return self.unify(ty_a, &n_b);
        }

        match (ty_a, ty_b) {
            (Type::Int, Type::Int)
            | (Type::UInt, Type::UInt)
            | (Type::Byte, Type::Byte)
            | (Type::Float, Type::Float)
            | (Type::Bool, Type::Bool)
            | (Type::Char, Type::Char) => Ok(()),
            (Type::Array(ty_a), Type::Array(ty_b)) => self.unify(ty_a, ty_b),
            (Type::Tuple(tys_a), Type::Tuple(tys_b)) => self.unify_all(tys_a, tys_b),
            (Type::Func(param_tys_a, return_ty_a), Type::Func(param_tys_b, return_ty_b)) => self
                .unify(return_ty_a, return_ty_b)
                .and_then(|()| self.unify_all(param_tys_a, param_tys_b)),
            (Type::Var(a_id) | Type::IntVar(a_id), Type::Var(b_id) | Type::IntVar(b_id)) => {
                Ok(self.table.unify_var_var(*a_id, *b_id).expect("infallible"))
            }
            (
                Type::Named {
                    name: name_a,
                    generics: args_a,
                },
                Type::Named {
                    name: name_b,
                    generics: args_b,
                },
            ) if name_a == name_b => self.unify_all(args_a, args_b),
            (Type::Var(id), bound_ty) | (bound_ty, Type::Var(id)) => {
                if !self.occurs((*id).into(), &bound_ty) {
                    Ok(self
                        .table
                        .unify_var_value(*id, bound_ty.clone())
                        .expect("infallible"))
                } else {
                    Err(TypeError::Infinite)
                }
            }
            (Type::IntVar(id), integer @ (Type::Int | Type::UInt | Type::Byte))
            | (integer @ (Type::Int | Type::UInt | Type::Byte), Type::IntVar(id)) => Ok(self
                .table
                .unify_var_value(*id, integer.clone())
                .expect("infallible")),
            (ty_a, ty_b) => Err(TypeError::MismatchedTypes { expected: ty_a.clone(), found: ty_b.clone() }),
        }
    }

    fn unify_all(&mut self, tys_a: &[Type], tys_b: &[Type]) -> Result<(), TypeError> {
        iter::zip(tys_a, tys_b).try_for_each(|(a, b)| self.unify(a, b))
    }

    fn occurs(&mut self, var: TypeId, ty: &Type) -> bool {
        if let Some(n_ty) = self.normalize(ty) {
            return self.occurs(var, &n_ty);
        }

        match &ty {
            Type::Named { generics: args, .. } => args.iter().any(|ty| self.occurs(var, ty)),
            Type::Var(id) | Type::IntVar(id) => *id == var,
            Type::Int | Type::UInt | Type::Byte | Type::Float | Type::Bool | Type::Char => false,
            Type::Array(inner_ty) => self.occurs(var, inner_ty),
            Type::Tuple(tys) => tys.iter().any(|ty| self.occurs(var, ty)),
            Type::Func(param_tys, result_ty) => {
                self.occurs(var, result_ty) || param_tys.iter().any(|ty| self.occurs(var, ty))
            }
        }
    }
}
