//! Performs typechecking on the [`Hir`], producing a mapping of expressions to their types, and filling in the Hir's mapping of variables to types.

mod error;
mod infer;
mod substitute;
#[cfg(test)]
mod test;
mod types;
mod unify;

use std::range::Range;

use ena::unify::{InPlaceUnificationTable, UnificationTable};
use slotmap::SecondaryMap;

use errors::{ErrorHandler, Result};
use hir::{ExecKind, ExprId, Hir, Ty, VarId};
use ident::SpanIdent;

use crate::types::{PartialTy, TyVar};

type Table = InPlaceUnificationTable<TyVar>;

struct TypeChecker<'handler> {
    table: Table,
    constraints: Vec<Constraint>,
    substitution: SecondaryMap<ExprId, PartialTy>,
    ctx: SecondaryMap<VarId, PartialTy>,
    handler: ErrorHandler<'handler>,
}

#[derive(Debug)]
enum Constraint {
    Eq(PartialTy, PartialTy, Range<u32>),
    Field(PartialTy, Range<u32>, PartialTy, SpanIdent),
}

/// Runs typechecking on the provided [`Hir`], reporting errors through the provided [`ErrorHandler`].
pub fn type_hir<'handler>(
    hir: &mut Hir,
    handler: ErrorHandler<'handler>,
) -> Result<SecondaryMap<ExprId, Ty>> {
    let mut checker = TypeChecker {
        table: UnificationTable::new(),
        constraints: Vec::new(),
        substitution: SecondaryMap::new(),
        ctx: SecondaryMap::new(),
        handler,
    };

    for exec in hir.execs() {
        match &exec.kind {
            ExecKind::Const { val, .. } => {
                let val_ty = checker.infer_expr(hir, *val);
                let var_ty = checker.var_ty(hir, exec.id).clone();
                checker.constrain_eq(val_ty, var_ty, hir.expr_span(*val));
            }
            ExecKind::Fn { body, .. } => {
                let body_ty = checker.infer_expr(hir, *body);
                let PartialTy::Fn(_, ret_ty) = checker.var_ty(hir, exec.id) else {
                    unreachable!("function was given non-function type during nameres")
                };
                let ret_ty = *ret_ty.clone();
                checker.constrain_eq(body_ty, ret_ty, hir.expr_span(*body));
            }
        }
    }
    if let Some(main) = hir.main() {
        let ExecKind::Fn { body, .. } = &main.kind else {
            unreachable!("ICE")
        };
        let body_ty = checker.infer_expr(hir, *body);
        checker.constrain_eq(body_ty, PartialTy::unit(), hir.expr_span(*body));
    }

    checker.unify(hir);

    checker.sub_all(hir)
}

impl TypeChecker<'_> {
    fn var_ty(&mut self, hir: &Hir, var: VarId) -> &PartialTy {
        let table = &mut self.table;
        self.ctx
            .entry(var)
            .unwrap()
            .or_insert_with(|| types::convert(table, hir.try_var_ty(var)))
    }

    fn constrain_eq(&mut self, ty_a: PartialTy, ty_b: PartialTy, span: Range<u32>) {
        self.constraints.push(Constraint::Eq(ty_a, ty_b, span));
    }

    fn constrain_field(
        &mut self,
        base_ty: PartialTy,
        base_span: Range<u32>,
        field_ty: PartialTy,
        field_name: SpanIdent,
    ) {
        self.constraints
            .push(Constraint::Field(base_ty, base_span, field_ty, field_name));
    }
}
