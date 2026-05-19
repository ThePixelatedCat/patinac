use std::{mem, result};

use itertools::Itertools;

use hir::{
    Hir, TyMap,
    types::{Param, Return, Ty},
};
use slotmap::SecondaryMap;

use crate::error::{ErrorKind, Result};

use crate::{PartialTy, TypeChecker};

impl TypeChecker {
    fn sub_ty(&mut self, ty: PartialTy) -> result::Result<Ty, ErrorKind> {
        match ty {
            PartialTy::Int => Ok(Ty::Int),
            PartialTy::UInt => Ok(Ty::UInt),
            PartialTy::Byte => Ok(Ty::Byte),
            PartialTy::Float => Ok(Ty::Float),
            PartialTy::Bool => Ok(Ty::Bool),
            PartialTy::Char => Ok(Ty::Char),
            PartialTy::Tuple(tys) => Ok(Ty::Tuple(self.sub_tys(tys)?)),
            PartialTy::Array(ty) => Ok(Ty::Array(Box::new(self.sub_ty(*ty)?))),
            PartialTy::Fn(params, ret) => {
                let params = params
                    .into_iter()
                    .map(|param| {
                        self.sub_ty(param.ty).map(|ty| Param {
                            mutable: param.mutable,
                            ty,
                        })
                    })
                    .try_collect()?;
                let ret = Return {
                    mutable: ret.mutable,
                    ty: Box::new(self.sub_ty(*ret.ty)?),
                };
                Ok(Ty::Fn(params, ret))
            }
            PartialTy::Adt(id) => Ok(Ty::Adt(id)),
            PartialTy::Var(var) | PartialTy::IntVar(var) => self
                .table
                .probe_value(var)
                .map_or(Err(ErrorKind::UninferredType), |ty| self.sub_ty(ty)),
        }
    }

    fn sub_tys(&mut self, tys: Vec<PartialTy>) -> result::Result<Vec<Ty>, ErrorKind> {
        tys.into_iter().map(|ty| self.sub_ty(ty)).collect()
    }

    pub(super) fn sub_all(&mut self, hir: &Hir) -> Result<TyMap> {
        Ok(mem::take(&mut self.substitution)
            .into_iter()
            .map(|(expr, ty)| {
                Ok((
                    expr,
                    self.sub_ty(ty)
                        .map_err(|err| err.span(hir.expr_span(expr)))?,
                ))
            })
            .collect::<Result<SecondaryMap<_, _>>>()?
            .into())
    }
}
