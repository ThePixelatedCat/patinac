mod error;
mod infer;
mod substitute;
#[cfg(test)]
mod test;
mod types;
mod unify;

use ena::unify::InPlaceUnificationTable;

use hir::{Hir, TyMap, VarId, exprs::ExprId, items::ExecKind, types::Ty};
use slotmap::SecondaryMap;
use span::Span;

pub use crate::error::{Error, ErrorKind, Result};
use crate::types::{PartialTy, TyVar};

#[derive(Debug)]
struct Constraint {
    ty_a: PartialTy,
    ty_b: PartialTy,
    span: Span,
}

#[derive(Default)]
pub struct TypeChecker {
    table: InPlaceUnificationTable<TyVar>,
    constraints: Vec<Constraint>,
    substitution: SecondaryMap<ExprId, PartialTy>,
    ctx: SecondaryMap<VarId, PartialTy>,
}

impl TypeChecker {
    pub fn type_program(&mut self, hir: &mut Hir) -> Result<TyMap> {
        self.build_context(hir);
        for exec in &hir.execs {
            match &exec.kind {
                ExecKind::Const { val, .. } => {
                    let val_ty = self.infer_expr(hir, *val)?;
                    self.constrain_eq(val_ty, self.ctx[exec.ident].clone(), hir.expr_span(*val));
                }
                ExecKind::Fn {
                    params,
                    ret_ty,
                    body,
                } => {
                    let body_ty = self.infer_expr(hir, *body)?;
                    self.constrain_eq(body_ty, ret_ty.into(), hir.expr_span(*body));
                }
            }
        }
        self.unify()?;
        self.sub_all(hir)
    }

    fn build_context(&mut self, hir: &Hir) {
        self.ctx = hir
            .var_info
            .iter()
            .map(|(var, info)| (var, self.convert(info.ty.as_ref())))
            .collect::<SecondaryMap<_, _>>()
            .into()
    }

    fn fresh_var(&mut self) -> PartialTy {
        PartialTy::Var(self.table.new_key(None))
    }

    fn fresh_int_var(&mut self) -> PartialTy {
        PartialTy::IntVar(self.table.new_key(None))
    }

    fn constrain_eq(&mut self, ty_a: PartialTy, ty_b: PartialTy, span: Span) {
        self.constraints.push(Constraint { ty_a, ty_b, span });
    }

    fn convert(&mut self, ast_ty: Option<&Ty>) -> PartialTy {
        ast_ty.map_or_else(|| self.fresh_var(), PartialTy::from)
    }
}
