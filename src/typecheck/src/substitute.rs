use std::mem;

use itertools::Itertools as _;

use errors::{Result, TryCollectEager as _};
use hir::{ExprId, Hir, Param, Ty};
use slotmap::SecondaryMap;

use crate::{TypeChecker, error::ErrorKind, types::PartialTy};

impl TypeChecker<'_> {
    fn sub_ty(&mut self, ty: &PartialTy) -> Result<Ty, ()> {
        match ty {
            PartialTy::Int => Ok(Ty::Int),
            PartialTy::UInt => Ok(Ty::UInt),
            PartialTy::Byte => Ok(Ty::Byte),
            PartialTy::Float => Ok(Ty::Float),
            PartialTy::Bool => Ok(Ty::Bool),
            PartialTy::Char => Ok(Ty::Char),
            PartialTy::Tuple(tys) => Ok(Ty::Tuple(self.sub_tys(tys)?)),
            PartialTy::Array(ty) => Ok(Ty::Array(Box::new(self.sub_ty(ty)?))),
            PartialTy::Fn(params, ret) => {
                let params = params
                    .iter()
                    .map(|param| {
                        Ok(Param {
                            ty: self.sub_ty(&param.ty)?,
                            mutable: param.mutable,
                            span: param.span,
                        })
                    })
                    .try_collect()?;
                let ret = Box::new(self.sub_ty(ret)?);
                Ok(Ty::Fn(params, ret))
            }
            PartialTy::Named(id) => Ok(Ty::Named(*id)),
            PartialTy::Var(var) => self
                .table
                .probe_value(*var)
                .map_or(Err(()), |ty| self.sub_ty(&ty)),
            PartialTy::IntVar(var) => self
                .table
                .probe_value(*var)
                .map_or(Ok(Ty::Int), |ty| self.sub_ty(&ty)),
        }
    }

    fn sub_tys(&mut self, tys: &[PartialTy]) -> Result<Vec<Ty>, ()> {
        tys.iter().map(|ty| self.sub_ty(ty)).try_collect()
    }

    pub(super) fn sub_all(&mut self, hir: &mut Hir) -> Result<SecondaryMap<ExprId, Ty>> {
        // Don't even try if we have outstanding errors
        let () = self.handler.checked(())?;

        let expr_map = mem::take(&mut self.substitution)
            .iter()
            .map(|(expr, ty)| match self.sub_ty(ty) {
                Ok(ty) => Ok((expr, ty)),
                Err(()) => Err(self
                    .handler
                    .err(ErrorKind::UninferredExprType.span(hir.expr_span(expr)))),
            })
            .try_collect_eager();

        let () = mem::take(&mut self.ctx)
            .iter()
            .map(|(var, ty)| match self.sub_ty(ty) {
                Ok(ty) => {
                    hir.add_var_ty(var, ty);
                    Ok(())
                }
                Err(()) => Err(self
                    .handler
                    .err(ErrorKind::UninferredVarType.span(hir.var_info(var).span))),
            })
            .try_collect_eager()?;

        Ok(expr_map?)
    }
}
