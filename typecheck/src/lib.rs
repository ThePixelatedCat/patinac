mod error;
mod infer;
mod substitute;
#[cfg(test)]
mod test;
mod types;
mod unify;

use ena::unify::{InPlaceUnificationTable, UnificationTable};

use errors::{ErrorHandler, Result};
use hir::{Hir, TyMap, VarId, exprs::ExprId, items::ExecKind, types::Ty};
use slotmap::SecondaryMap;
use span::Span;

use crate::types::{PartialTy, TyVar};

#[derive(Debug)]
struct Constraint {
    ty_a: PartialTy,
    ty_b: PartialTy,
    span: Span,
}

pub struct TypeChecker<'err> {
    table: InPlaceUnificationTable<TyVar>,
    constraints: Vec<Constraint>,
    substitution: SecondaryMap<ExprId, PartialTy>,
    ctx: SecondaryMap<VarId, PartialTy>,
    handler: ErrorHandler<'err>,
}

impl<'err> TypeChecker<'err> {
    pub fn new(handler: ErrorHandler<'err>) -> Self {
        Self {
            table: UnificationTable::new(),
            constraints: Vec::new(),
            substitution: SecondaryMap::new(),
            ctx: SecondaryMap::new(),
            handler,
        }
    }
}

impl TypeChecker<'_> {
    pub fn type_program(&mut self, hir: &mut Hir) -> Result<TyMap> {
        self.build_context(hir);

        for exec in hir.execs() {
            match &exec.kind {
                ExecKind::Const { val, .. } => {
                    if let Ok(val_ty) = self.infer_expr(hir, *val) {
                        self.constrain_eq(val_ty, self.ctx[exec.id].clone(), hir.expr_span(*val));
                    }
                }
                ExecKind::Fn { body, .. } => {
                    if let Ok(body_ty) = self.infer_expr(hir, *body) {
                        let PartialTy::Fn(_, ret_ty) = &self.ctx[exec.id] else {
                            unreachable!("ICE: Function was given non-function type during nameres")
                        };
                        self.constrain_eq(body_ty, *ret_ty.clone(), hir.expr_span(*body));
                    }
                }
            }
        }
        if let Some(main) = hir.main() {
            let ExecKind::Fn { body, .. } = &main.kind else {
                unreachable!("ICE")
            };
            if let Ok(body_ty) = self.infer_expr(hir, *body) {
                self.constrain_eq(body_ty, PartialTy::unit(), hir.expr_span(*body));
            }
        }

        self.unify();

        self.sub_all(hir)
    }

    fn build_context(&mut self, hir: &Hir) {
        self.ctx = hir
            .var_tys()
            .map(|(var, ty)| (var, self.convert(ty)))
            .collect::<SecondaryMap<_, _>>();
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
