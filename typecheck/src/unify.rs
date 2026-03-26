use std::{cmp, iter};

use ena::unify::UnifyValue;

use super::{Ty, TypeChecker, TypeError, TypeVar};

impl UnifyValue for Ty {
    type Error = TypeError;

    #[allow(clippy::unnested_or_patterns, reason = "clarity")]
    fn unify_values(a: &Self, b: &Self) -> Result<Self, Self::Error> {
        match (a, b) {
            (Self::IntVar(id_a), Self::IntVar(id_b))
            | (Self::IntVar(id_a), Self::Var(id_b))
            | (Self::Var(id_b), Self::IntVar(id_a)) => Ok(Self::IntVar(cmp::min(*id_a, *id_b))),
            (Self::Var(id_a), Self::Var(id_b)) => Ok(Self::Var(cmp::min(*id_a, *id_b))),
            (Self::IntVar(_), int_ty @ (Self::Int | Self::UInt | Self::Byte))
            | (int_ty @ (Self::Int | Self::UInt | Self::Byte), Self::IntVar(_)) => {
                Ok(int_ty.clone())
            }
            (int_var @ Self::IntVar(_), ty) | (ty, int_var @ Self::IntVar(_)) => {
                Err(TypeError::MismatchedTypes {
                    expected: int_var.clone(),
                    found: ty.clone(),
                })
            }
            (Self::Var(id), ty) | (ty, Self::Var(id)) => {
                if ty.contains(*id) {
                    Err(TypeError::Infinite)
                } else {
                    Ok(ty.clone())
                }
            }
            (ty_a, ty_b) => {
                panic!("should never have to unify two bound types ({ty_a} and {ty_b})")
            }
        }
    }
}

impl Ty {
    fn contains(&self, var: TypeVar) -> bool {
        match &self {
            Self::Adt(args, ..) => args.iter().any(|ty| ty.contains(var)),
            Self::Var(id) | Self::IntVar(id) => *id == var,
            Self::Int | Self::UInt | Self::Byte | Self::Float | Self::Bool | Self::Char => false,
            Self::Array(ty) => ty.contains(var),
            Self::Tuple(tys) => tys.iter().any(|ty| ty.contains(var)),
            Self::Fn(param_tys, result_ty) => {
                result_ty.contains(var) || param_tys.iter().any(|ty| ty.contains(var))
            }
        }
    }
}

impl TypeChecker {
    /// Recursively traverse two types until at least one is a type variable,
    /// at which point we unify them in the table,
    /// or until we can no longer traverse them or we know they're mismatched,
    /// at which point we error
    pub(super) fn unify(&self, a: &Ty, b: &Ty) -> Result<(), TypeError> {
        if let Some(n_a) = self.normalise_id(a) {
            return self.unify(&n_a, b);
        } else if let Some(n_b) = self.normalise_id(b) {
            return self.unify(a, &n_b);
        }

        match (a, b) {
            (Ty::IntVar(id_a) | Ty::Var(id_a), Ty::IntVar(id_b) | Ty::Var(id_b)) => {
                self.table.borrow_mut().unify_var_var(*id_a, *id_b)
            }
            (Ty::IntVar(id) | Ty::Var(id), ty) | (ty, Ty::IntVar(id) | Ty::Var(id)) => {
                self.table.borrow_mut().unify_var_value(*id, ty.clone())
            }
            (Ty::Int, Ty::Int)
            | (Ty::UInt, Ty::UInt)
            | (Ty::Byte, Ty::Byte)
            | (Ty::Float, Ty::Float)
            | (Ty::Bool, Ty::Bool)
            | (Ty::Char, Ty::Char) => Ok(()),
            (Ty::Array(ty_a), Ty::Array(ty_b)) => self.unify(ty_a, ty_b),
            (Ty::Tuple(tys_a), Ty::Tuple(tys_b)) => self.unify_all(tys_a, tys_b),
            (Ty::Fn(param_tys_a, return_ty_a), Ty::Fn(param_tys_b, return_ty_b)) => self
                .unify(return_ty_a, return_ty_b)
                .and_then(|()| self.unify_all(param_tys_a, param_tys_b)),
            (Ty::Adt(name_a, args_a), Ty::Adt(name_b, args_b)) if name_a == name_b => {
                self.unify_all(args_a, args_b)
            }
            (ty_a, ty_b) => Err(TypeError::MismatchedTypes {
                expected: ty_a.clone(),
                found: ty_b.clone(),
            }),
        }
    }

    fn unify_all(&self, tys_a: &[Ty], tys_b: &[Ty]) -> Result<(), TypeError> {
        iter::zip(tys_a, tys_b).try_for_each(|(a, b)| self.unify(a, b))
    }

    pub(super) fn unify_either(&self, ty: &Ty, opt_a: &Ty, opt_b: &Ty) -> Result<(), TypeError> {
        let snapshot = self.table.borrow_mut().snapshot();

        match self.unify(opt_a, ty) {
            Ok(()) => {
                self.table.borrow_mut().commit(snapshot);
                Ok(())
            }
            Err(TypeError::MismatchedTypes { expected, found })
                if expected == *opt_a && found == *ty =>
            {
                self.table.borrow_mut().rollback_to(snapshot);
                self.unify(opt_b, ty)?;
                Ok(())
            }
            Err(e) => {
                self.table.borrow_mut().rollback_to(snapshot);
                Err(e)
            }
        }
    }
}
