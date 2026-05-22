use std::mem;

use itertools::Itertools;

use hir::{
    Hir, TyMap,
    types::{Param, Ty},
};

use crate::error::{ErrorKind, Result};

use crate::{PartialTy, TypeChecker};

impl TypeChecker<'_> {
    fn sub_ty(&mut self, ty: PartialTy) -> Result<Ty> {
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
                        Ok(Param {
                            ty: self.sub_ty(param.ty)?,
                            mutable: param.mutable,
                            span: param.span,
                        })
                    })
                    .try_collect()?;
                let ret = Box::new(self.sub_ty(*ret)?);
                Ok(Ty::Fn(params, ret))
            }
            PartialTy::Adt(id) => Ok(Ty::Adt(id)),
            PartialTy::Var(var) | PartialTy::IntVar(var) => self
                .table
                .probe_value(var)
                .map_or(Err(()), |ty| self.sub_ty(ty)),
        }
    }

    fn sub_tys(&mut self, tys: Vec<PartialTy>) -> Result<Vec<Ty>> {
        tys.into_iter().map(|ty| self.sub_ty(ty)).collect()
    }

    pub(super) fn sub_all(&mut self, hir: &Hir) -> Result<TyMap> {
        // Don't even try if we have outstanding errors
        if self.handler.has_err() {
            return Err(());
        }

        let expr_map = mem::take(&mut self.substitution)
            .into_iter()
            .map(|(expr, ty)| match self.sub_ty(ty) {
                Ok(ty) => Ok((expr, ty)),
                Err(()) => {
                    self.handler
                        .err(ErrorKind::UninferredExprType.span(hir.expr_span(expr)));
                    Err(())
                }
            })
            .try_collect();
        let var_map = mem::take(&mut self.ctx)
            .into_iter()
            .map(|(var, ty)| match self.sub_ty(ty) {
                Ok(ty) => Ok((var, ty)),
                Err(()) => {
                    self.handler
                        .err(ErrorKind::UninferredExprType.span(hir.var_ident(var).span));
                    Err(())
                }
            })
            .try_collect();

        Ok(TyMap::new(expr_map?, var_map?))
    }
}
