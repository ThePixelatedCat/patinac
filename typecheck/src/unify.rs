use std::{iter, mem};

use crate::{
    ConstraintKind, ErrorKind, PartialTy, TyVar, TypeChecker,
    error::Error,
    type_vars::{Param, Return},
};

fn occurs_check(ty: &PartialTy, var: TyVar) -> Result<(), ErrorKind> {
    match ty {
        PartialTy::Int
        | PartialTy::UInt
        | PartialTy::Byte
        | PartialTy::Float
        | PartialTy::Bool
        | PartialTy::Char => Ok(()),
        PartialTy::Tuple(tys) => tys.iter().try_for_each(|ty| occurs_check(ty, var)),
        PartialTy::Fn(params, ret) => {
            params
                .iter()
                .try_for_each(|param| occurs_check(&param.ty, var))?;
            occurs_check(&ret.ty, var)
        }
        PartialTy::Adt(_, args) => args.iter().try_for_each(|ty| occurs_check(ty, var)),
        PartialTy::Var(this_var) | PartialTy::IntVar(this_var) if *this_var == var => {
            Err(ErrorKind::Infinite(var, PartialTy::Var(*this_var)))
        }
        PartialTy::Var(_) | PartialTy::IntVar(_) => Ok(()),
    }
}

impl TypeChecker {
    /// Unifies all types in the unification table, clearing all of our constraints in the process
    pub(super) fn unify(&mut self) -> Result<(), Error> {
        for constr in mem::take(&mut self.constraints) {
            match constr.kind {
                ConstraintKind::TypeEqual(left, right) => self.unify_ty_ty(left, right),
                ConstraintKind::EitherTypeEqual(ty, options) => todo!(),
            }
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
            (int_var @ PartialTy::IntVar(_), PartialTy::Var(var))
            | (PartialTy::Var(var), int_var @ PartialTy::IntVar(_)) => {
                self.unify_var_value(var, int_var)
            }
            (PartialTy::Var(lhs_var), PartialTy::Var(rhs_var)) => {
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
                self.unify_all(lhs_inners, rhs_inners)
            }
            (PartialTy::Fn(lhs_params, lhs_return), PartialTy::Fn(rhs_params, rhs_return)) => {
                if lhs_params.len() != rhs_params.len() {
                    return Err(ErrorKind::ParamCount(
                        PartialTy::Fn(lhs_params, lhs_return),
                        PartialTy::Fn(rhs_params, rhs_return),
                    ));
                }
                iter::zip(lhs_params, rhs_params).try_for_each(|(l, r)| {
                    if l.mutable != r.mutable {
                        return Err(ErrorKind::ParamMutability(l, r));
                    }
                    self.unify_ty_ty(l.ty, r.ty)
                })?;
                if lhs_return.mutable != rhs_return.mutable {
                    return Err(ErrorKind::ReturnMutability(lhs_return, rhs_return));
                }
                self.unify_ty_ty(*lhs_return.ty, *rhs_return.ty)
            }

            (PartialTy::Adt(name_a, args_a), PartialTy::Adt(name_b, args_b))
                if name_a == name_b =>
            {
                self.unify_all(args_a, args_b)
            }
            (lhs, rhs) => Err(ErrorKind::TypesNotEqual(lhs, rhs)),
        }
    }

    fn unify_all(
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
            PartialTy::Fn(params, ret) => {
                let params = params
                    .into_iter()
                    .map(|param| Param {
                        ty: self.normalize_ty(param.ty),
                        ..param
                    })
                    .collect();
                let ret = Return {
                    mutable: ret.mutable,
                    ty: Box::new(self.normalize_ty(*ret.ty)),
                };
                PartialTy::Fn(params, ret)
            }
            PartialTy::Var(v) => match self.table.probe_value(v) {
                Some(ty) => self.normalize_ty(ty),
                None => PartialTy::Var(self.table.find(v)),
            },
            PartialTy::IntVar(v) => match self.table.probe_value(v) {
                Some(ty) => self.normalize_ty(ty),
                None => PartialTy::IntVar(self.table.find(v)),
            },
            PartialTy::Adt(ident, arg_tys) => {
                let arg_tys = arg_tys
                    .into_iter()
                    .map(|ty| self.normalize_ty(ty))
                    .collect();
                PartialTy::Adt(ident, arg_tys)
            }
        }
    }
}
