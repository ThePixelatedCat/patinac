use std::{iter, mem};

use crate::{ErrorKind, PartialTy, TyVar, TypeChecker, types::Param};

fn occurs_check(ty: &PartialTy, var: TyVar) -> Result<(), ErrorKind> {
    match ty {
        PartialTy::Int
        | PartialTy::UInt
        | PartialTy::Byte
        | PartialTy::Float
        | PartialTy::Bool
        | PartialTy::Char
        | PartialTy::Adt(_) => Ok(()),
        PartialTy::Tuple(tys) => tys.iter().try_for_each(|ty| occurs_check(ty, var)),
        PartialTy::Array(ty) => occurs_check(ty, var),
        PartialTy::Fn(params, ret) => {
            params
                .iter()
                .try_for_each(|param| occurs_check(&param.ty, var))?;
            occurs_check(&ret, var)
        }
        PartialTy::Var(this_var) | PartialTy::IntVar(this_var) => {
            if *this_var == var {
                Err(ErrorKind::Infinite(var, PartialTy::Var(*this_var)))
            } else {
                Ok(())
            }
        }
    }
}

impl TypeChecker {
    /// Unifies all types in the unification table, clearing all of our constraints in the process
    pub(super) fn unify(&mut self) -> crate::Result<()> {
        for constr in mem::take(&mut self.constraints) {
            self.unify_ty_ty(constr.ty_a, constr.ty_b)
                .map_err(|kind| kind.span(constr.span))?;
        }
        Ok(())
    }

    /// Recursively traverse two types until at least one is a type variable,
    /// at which point we unify them in the table,
    /// or until we can no longer traverse them or we know they're mismatched,
    /// at which point we error
    pub(super) fn unify_ty_ty(
        &mut self,
        unnorm_lhs: PartialTy,
        unnorm_rhs: PartialTy,
    ) -> Result<(), ErrorKind> {
        let lhs = self.normalize_ty(unnorm_lhs);
        let rhs = self.normalize_ty(unnorm_rhs);

        match (lhs, rhs) {
            (PartialTy::Int, PartialTy::Int)
            | (PartialTy::UInt, PartialTy::UInt)
            | (PartialTy::Byte, PartialTy::Byte)
            | (PartialTy::Float, PartialTy::Float)
            | (PartialTy::Bool, PartialTy::Bool)
            | (PartialTy::Char, PartialTy::Char) => Ok(()),
            (PartialTy::Tuple(lhs_inners), PartialTy::Tuple(rhs_inners)) => {
                if lhs_inners.len() != rhs_inners.len() {
                    return Err(ErrorKind::TypesNotEqual(
                        PartialTy::Tuple(lhs_inners),
                        PartialTy::Tuple(rhs_inners),
                    ));
                }
                self.unify_tys(lhs_inners, rhs_inners)
            }
            (PartialTy::Array(lhs_inner), PartialTy::Array(rhs_inner)) => {
                self.unify_ty_ty(*lhs_inner, *rhs_inner)
            }
            (PartialTy::Fn(lhs_params, lhs_ret), PartialTy::Fn(rhs_params, rhs_ret)) => {
                if lhs_params.len() != rhs_params.len() {
                    return Err(ErrorKind::ParamCount(
                        PartialTy::Fn(lhs_params, lhs_ret),
                        PartialTy::Fn(rhs_params, rhs_ret),
                    ));
                }
                iter::zip(lhs_params, rhs_params).try_for_each(|(l, r)| {
                    if l.mutable != r.mutable {
                        return Err(ErrorKind::ParamMutability(l, r));
                    }
                    self.unify_ty_ty(l.ty, r.ty)
                })?;
                self.unify_ty_ty(*lhs_ret, *rhs_ret)
            }
            (PartialTy::Adt(a), PartialTy::Adt(b)) if a == b => Ok(()),
            (PartialTy::IntVar(lhs_var), PartialTy::IntVar(rhs_var))
            | (PartialTy::Var(lhs_var), PartialTy::Var(rhs_var)) => {
                self.unify_var_var(lhs_var, rhs_var)
            }
            (PartialTy::Var(var), ty) | (ty, PartialTy::Var(var)) => {
                occurs_check(&ty, var)?;
                self.unify_var_value(var, ty)
            }
            (
                PartialTy::IntVar(int_var),
                int_ty @ (PartialTy::Int | PartialTy::UInt | PartialTy::Byte),
            )
            | (
                int_ty @ (PartialTy::Int | PartialTy::UInt | PartialTy::Byte),
                PartialTy::IntVar(int_var),
            ) => self.unify_var_value(int_var, int_ty),
            (lhs, rhs) => Err(ErrorKind::TypesNotEqual(lhs, rhs)),
        }
    }

    fn unify_tys(
        &mut self,
        left_tys: Vec<PartialTy>,
        right_tys: Vec<PartialTy>,
    ) -> Result<(), ErrorKind> {
        iter::zip(left_tys, right_tys).try_for_each(|(l, r)| self.unify_ty_ty(l, r))
    }

    fn unify_var_var(&mut self, lhs: TyVar, rhs: TyVar) -> Result<(), ErrorKind> {
        self.table
            .unify_var_var(lhs, rhs)
            .map_err(|(l, r)| ErrorKind::TypesNotEqual(l, r))
    }

    fn unify_var_value(&mut self, var: TyVar, ty: PartialTy) -> Result<(), ErrorKind> {
        self.table
            .unify_var_value(var, Some(ty))
            .map_err(|(l, r)| ErrorKind::TypesNotEqual(l, r))
    }

    fn normalize_ty(&mut self, ty: PartialTy) -> PartialTy {
        match ty {
            PartialTy::Int
            | PartialTy::UInt
            | PartialTy::Byte
            | PartialTy::Float
            | PartialTy::Bool
            | PartialTy::Char => ty,
            PartialTy::Tuple(tys) => {
                PartialTy::Tuple(tys.into_iter().map(|ty| self.normalize_ty(ty)).collect())
            }
            PartialTy::Array(ty) => PartialTy::Array(Box::new(self.normalize_ty(*ty))),
            PartialTy::Fn(params, ret) => {
                let params = params
                    .into_iter()
                    .map(|param| Param {
                        ty: self.normalize_ty(param.ty),
                        ..param
                    })
                    .collect();
                let ret = Box::new(self.normalize_ty(*ret));
                PartialTy::Fn(params, ret)
            }
            PartialTy::Adt(id) => PartialTy::Adt(id),
            PartialTy::Var(v) => match self.table.probe_value(v) {
                Some(ty) => self.normalize_ty(ty),
                None => PartialTy::Var(self.table.find(v)),
            },
            PartialTy::IntVar(v) => match self.table.probe_value(v) {
                Some(ty) => self.normalize_ty(ty),
                None => PartialTy::IntVar(self.table.find(v)),
            },
        }
    }
}
