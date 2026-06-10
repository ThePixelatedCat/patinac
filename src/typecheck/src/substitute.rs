use std::mem;

use itertools::Itertools as _;

use errors::{Result, SpanError as _, TryCollectEager as _};
use hir::{ExprId, Hir, Param, Ty};
use slotmap::SecondaryMap;

use crate::{Table, TypeChecker, error::ErrorKind, types::PartialTy};

impl TypeChecker<'_> {
    pub(super) fn sub_all(&mut self, hir: &mut Hir) -> Result<SecondaryMap<ExprId, Ty>> {
        // Don't even try if we have outstanding errors
        self.handler.checked(())?;

        let expr_map = mem::take(&mut self.substitution)
            .iter()
            .map(|(expr, (ty, module))| match sub_ty(&mut self.table, ty) {
                Ok(ty) => Ok((expr, ty)),
                Err(()) => Err(self
                    .handler
                    .err(ErrorKind::UninferredExprType.span(hir.expr_span(expr), *module))),
            })
            .try_collect_eager();

        let () = mem::take(&mut self.ctx)
            .iter()
            .map(|(var, ty)| match sub_ty(&mut self.table, ty) {
                Ok(ty) => {
                    hir.add_var_ty(var, ty);
                    Ok(())
                }
                Err(()) => {
                    let var_info = hir.var_info(var);
                    Err(self
                        .handler
                        .err(ErrorKind::UninferredVarType.span(var_info.span, var_info.module)))
                }
            })
            .try_collect_eager()?;

        expr_map
    }
}

fn sub_ty(table: &mut Table, ty: &PartialTy) -> Result<Ty, ()> {
    match ty {
        PartialTy::Int => Ok(Ty::Int),
        PartialTy::UInt => Ok(Ty::UInt),
        PartialTy::Byte => Ok(Ty::Byte),
        PartialTy::Float => Ok(Ty::Float),
        PartialTy::Bool => Ok(Ty::Bool),
        PartialTy::Char => Ok(Ty::Char),
        PartialTy::Tuple(tys) => Ok(Ty::Tuple(
            tys.iter().map(|ty| sub_ty(table, ty)).try_collect_eager()?,
        )),
        PartialTy::Array(ty) => Ok(Ty::Array(Box::new(sub_ty(table, ty)?))),
        PartialTy::Fn(params, ret) => {
            let params = params
                .iter()
                .map(|param| {
                    Ok(Param {
                        ty: sub_ty(table, &param.ty)?,
                        mutable: param.mutable,
                        span: param.span,
                    })
                })
                .try_collect()?;
            let ret = Box::new(sub_ty(table, ret)?);
            Ok(Ty::Fn(params, ret))
        }
        PartialTy::Named(id) => Ok(Ty::Named(*id)),
        PartialTy::Var(var) => table
            .probe_value(*var)
            .map_or(Err(()), |ty| sub_ty(table, &ty)),
        PartialTy::IntVar(var) => table
            .probe_value(*var)
            .map_or(Ok(Ty::Int), |ty| sub_ty(table, &ty)),
    }
}
