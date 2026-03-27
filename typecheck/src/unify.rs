use std::{iter, mem};

use span::Spannable;

use crate::{ConstraintKind, Ty, TyVar, TypeChecker, TypeError, error::TypeErrorS};

impl Ty {
    fn occurs_check(&self, var: TyVar) -> Result<(), TypeError> {
        match self {
            Self::Int | Self::UInt | Self::Byte | Self::Float | Self::Bool | Self::Char => Ok(()),
            Self::Array(ty) => ty.occurs_check(var),
            Self::Tuple(tys) => tys.iter().try_for_each(|ty| ty.occurs_check(var)),
            Self::Func(param_tys, return_ty) => {
                param_tys.iter().try_for_each(|ty| ty.occurs_check(var))?;
                return_ty.occurs_check(var)
            }
            Self::Adt(_, args) => args.iter().try_for_each(|ty| ty.occurs_check(var)),
            Self::Var(this_var) | Self::IntVar(this_var) => {
                if *this_var == var {
                    Err(TypeError::Infinite(var, Self::Var(*this_var)))
                } else {
                    Ok(())
                }
            }
        }
    }
}

impl TypeChecker<'_> {
    pub(super) fn unify(&mut self) -> Result<(), TypeErrorS> {
        for constr in mem::take(&mut self.constraints) {
            match constr.kind {
                ConstraintKind::TypeEqual(left, right) => self.unify_ty_ty(left, right),
                ConstraintKind::EitherTypeEqual(_, _) => todo!(),
            }
            .map_err(|kind| kind.span(constr.span))?;
        }
        Ok(())
    }

    /// Recursively traverse two types until at least one is a type variable,
    /// at which point we unify them in the table,
    /// or until we can no longer traverse them or we know they're mismatched,
    /// at which point we error
    pub(super) fn unify_ty_ty(&mut self, unnorm_lhs: Ty, unnorm_rhs: Ty) -> Result<(), TypeError> {
        let lhs = self.normalize_ty(unnorm_lhs);
        let rhs = self.normalize_ty(unnorm_rhs);

        match (lhs, rhs) {
            (int_var @ Ty::IntVar(_), Ty::Var(var)) | (Ty::Var(var), int_var @ Ty::IntVar(_)) => {
                self.unify_var_value(var, int_var)
            }
            (Ty::Var(lhs_var), Ty::Var(rhs_var)) => self.unify_var_var(lhs_var, rhs_var),
            (Ty::Var(var), ty) | (ty, Ty::Var(var)) => {
                ty.occurs_check(var)?;
                self.unify_var_value(var, ty)
            }
            (Ty::IntVar(int_var), int_ty @ (Ty::Int | Ty::UInt | Ty::Byte))
            | (int_ty @ (Ty::Int | Ty::UInt | Ty::Byte), Ty::IntVar(int_var)) => {
                self.unify_var_value(int_var, int_ty)
            }
            (Ty::Int, Ty::Int)
            | (Ty::UInt, Ty::UInt)
            | (Ty::Byte, Ty::Byte)
            | (Ty::Float, Ty::Float)
            | (Ty::Bool, Ty::Bool)
            | (Ty::Char, Ty::Char) => Ok(()),
            (Ty::Array(lhs_inner), Ty::Array(rhs_inner)) => {
                self.unify_ty_ty(*lhs_inner, *rhs_inner)
            }
            (Ty::Tuple(lhs_inners), Ty::Tuple(rhs_inners)) => {
                self.unify_all(lhs_inners, rhs_inners)
            }
            (Ty::Func(lhs_params, lhs_return), Ty::Func(rhs_params, rhs_return)) => {
                self.unify_all(lhs_params, rhs_params)?;
                self.unify_ty_ty(*lhs_return, *rhs_return)
            }

            (Ty::Adt(name_a, args_a), Ty::Adt(name_b, args_b)) if name_a == name_b => {
                self.unify_all(args_a, args_b)
            }
            (lhs, rhs) => Err(TypeError::TypesNotEqual(lhs, rhs)),
        }
    }

    fn unify_all(&mut self, left_tys: Vec<Ty>, right_tys: Vec<Ty>) -> Result<(), TypeError> {
        iter::zip(left_tys, right_tys).try_for_each(|(l, r)| self.unify_ty_ty(l, r))
    }

    fn normalize_ty(&mut self, ty: Ty) -> Ty {
        match ty {
            Ty::Int | Ty::UInt | Ty::Byte | Ty::Float | Ty::Bool | Ty::Char => ty,
            Ty::Array(ty) => Ty::Array(Box::new(self.normalize_ty(*ty))),
            Ty::Tuple(tys) => Ty::Tuple(tys.into_iter().map(|ty| self.normalize_ty(ty)).collect()),
            Ty::Func(param_tys, return_ty) => {
                let param_tys = param_tys
                    .into_iter()
                    .map(|ty| self.normalize_ty(ty))
                    .collect();
                let return_ty = Box::new(self.normalize_ty(*return_ty));
                Ty::Func(param_tys, return_ty)
            }
            Ty::Var(v) => match self.table.probe_value(v) {
                Some(ty) => self.normalize_ty(ty),
                None => Ty::Var(self.table.find(v)),
            },
            Ty::IntVar(v) => match self.table.probe_value(v) {
                Some(ty) => self.normalize_ty(ty),
                None => Ty::IntVar(self.table.find(v)),
            },
            Ty::Adt(ident, arg_tys) => {
                let arg_tys = arg_tys
                    .into_iter()
                    .map(|ty| self.normalize_ty(ty))
                    .collect();
                Ty::Adt(ident, arg_tys)
            }
        }
    }

    fn unify_var_var(&mut self, lhs: TyVar, rhs: TyVar) -> Result<(), TypeError> {
        self.table
            .unify_var_var(lhs, rhs)
            .map_err(|(l, r)| TypeError::TypesNotEqual(l, r))
    }

    fn unify_var_value(&mut self, var: TyVar, ty: Ty) -> Result<(), TypeError> {
        self.table
            .unify_var_value(var, Some(ty))
            .map_err(|(l, r)| TypeError::TypesNotEqual(l, r))
    }

    // pub(super) fn unify_either(&self, ty: &Ty, opt_a: &Ty, opt_b: &Ty) -> Result<(), TypeError> {
    //     let snapshot = self.table.snapshot();

    //     match self.unify(opt_a, ty) {
    //         Ok(()) => {
    //             self.table.commit(snapshot);
    //             Ok(())
    //         }
    //         Err(TypeError::MismatchedTypes { expected, found })
    //             if expected == *opt_a && found == *ty =>
    //         {
    //             self.table.rollback_to(snapshot);
    //             self.unify(opt_b, ty)?;
    //             Ok(())
    //         }
    //         Err(e) => {
    //             self.table.rollback_to(snapshot);
    //             Err(e)
    //         }
    //     }
    // }
}
