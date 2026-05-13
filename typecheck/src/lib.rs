mod error;
mod infer;
mod substitute;
#[cfg(test)]
mod test;
mod type_vars;
mod unify;

use ena::unify::InPlaceUnificationTable;

use hir::{
    AdtId, Hir, VarId,
    exprs::{Expr, ExprId},
    items::{ExecItem, ExecKind},
    types::Ty,
};
use itertools::Itertools;
use slotmap::SecondaryMap;
use span::Span;

pub use crate::error::{Error, ErrorKind, Result};
use crate::type_vars::{PartialTy, TyVar};

struct Constraint {
    kind: ConstraintKind,
    span: Span,
}

enum ConstraintKind {
    TypeEqual(PartialTy, PartialTy),
    EitherTypeEqual(PartialTy, (PartialTy, PartialTy)),
}

#[derive(Default)]
pub struct TypeChecker {
    table: InPlaceUnificationTable<TyVar>,
    constraints: Vec<Constraint>,
    substitution: SecondaryMap<ExprId, PartialTy>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self::default()
    }
}

impl TypeChecker {
    pub fn type_program(&mut self, hir: &mut Hir) -> Result<SecondaryMap<ExprId, Ty>> {
        for exec in &hir.execs {
            match &exec.kind {
                ExecKind::Const { ty, val } => {
                    let val_ty = self.infer_expr(hir, *val)?;
                    let annot_ty = self.convert(ty.as_ref());
                    self.constrain_eq(val_ty, annot_ty, hir.expr_span(*val));
                }
                ExecKind::Fn {
                    generics,
                    params,
                    ret_mut,
                    ret_ty,
                    body,
                } => {
                    let body_ty = self.infer_expr(hir, *body)?;
                    let ret_ty = ret_ty.into();
                    self.constrain_eq(body_ty, ret_ty, hir.expr_span(*body));
                }
            }
        }
        self.unify();
        self.sub_all(hir)
    }

    fn fresh_var(&mut self) -> PartialTy {
        PartialTy::Var(self.table.new_key(None))
    }

    fn fresh_int_var(&mut self) -> PartialTy {
        PartialTy::IntVar(self.table.new_key(None))
    }

    fn constrain_eq(&mut self, a: PartialTy, b: PartialTy, span: Span) {
        self.constraints.push(Constraint {
            kind: ConstraintKind::TypeEqual(a, b),
            span,
        });
    }

    fn constrain_either_eq(&mut self, a: PartialTy, tys: (PartialTy, PartialTy), span: Span) {
        self.constraints.push(Constraint {
            kind: ConstraintKind::EitherTypeEqual(a, tys),
            span,
        });
    }

    fn convert(&mut self, ast_ty: Option<&Ty>) -> PartialTy {
        ast_ty.map_or_else(|| self.fresh_var(), PartialTy::from)
    }
}
