use std::{mem, result};

use itertools::Itertools;

use hir::{
    Hir,
    exprs::ExprId,
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
            PartialTy::Adt(ident, arg_tys) => Ok(Ty::Adt(ident, self.sub_tys(arg_tys)?)),
            PartialTy::Var(var) | PartialTy::IntVar(var) => {
                let root = self.table.find(var);
                self.table
                    .probe_value(root)
                    .map_or(Err(ErrorKind::UninferredType), |ty| self.sub_ty(ty))
            }
        }
    }

    fn sub_tys(&mut self, tys: Vec<PartialTy>) -> result::Result<Vec<Ty>, ErrorKind> {
        tys.into_iter().map(|ty| self.sub_ty(ty)).collect()
    }

    pub(super) fn sub_all(&mut self, hir: &Hir) -> Result<SecondaryMap<ExprId, Ty>> {
        mem::take(&mut self.substitution)
            .into_iter()
            .map(|(expr, ty)| {
                Ok((
                    expr,
                    self.sub_ty(ty)
                        .map_err(|err| err.span(hir.expr_span(expr)))?,
                ))
            })
            .collect()
    }
}
