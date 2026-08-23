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
use ident::SpanIdent;
use irs::{
    ModuleId,
    hir::{DefKind, ExprId, Hir, Ty, VarId},
};

use crate::types::{PartialTy, TyVar};

type Table = InPlaceUnificationTable<TyVar>;

struct TypeChecker<'handler> {
    table: Table,
    constraints: Vec<Constraint>,
    substitution: SecondaryMap<ExprId, (PartialTy, ModuleId)>,
    ctx: SecondaryMap<VarId, PartialTy>,
    handler: ErrorHandler<'handler>,
}

#[derive(Debug)]
enum Constraint {
    Eq(PartialTy, PartialTy, Range<u32>, ModuleId),
    Field(PartialTy, Range<u32>, PartialTy, SpanIdent, ModuleId),
    Method(PartialTy, Range<u32>, PartialTy, SpanIdent, ModuleId),
}

/// Runs typechecking on the provided [`Hir`], reporting errors through the provided [`ErrorHandler`].
///
/// # Errors
/// Returns an error if any types don't match or can't be inferred.
pub fn type_hir(hir: &mut Hir, handler: ErrorHandler<'_>) -> Result<SecondaryMap<ExprId, Ty>> {
    let mut checker = TypeChecker {
        table: UnificationTable::new(),
        constraints: Vec::new(),
        substitution: SecondaryMap::new(),
        ctx: SecondaryMap::new(),
        handler,
    };

    for exec in hir.execs() {
        match &exec.kind {
            DefKind::Const(val) => {
                let initialiser_ty = checker.infer_expr(hir, exec.module, *val);
                let binding_ty = checker.var_ty(hir, exec.var).clone();
                checker.constrain_eq(initialiser_ty, binding_ty, hir.expr_span(*val), exec.module);
            }
            DefKind::Func { body, .. } => {
                let body_ty = checker.infer_expr(hir, exec.module, *body);
                let PartialTy::Fn(_, ret_ty) = checker.var_ty(hir, exec.var) else {
                    unreachable!("function was given non-function type during nameres")
                };
                let ret_ty = *ret_ty.clone();
                checker.constrain_eq(body_ty, ret_ty, hir.expr_span(*body), exec.module);
            }
        }
    }
    if let Some(main) = hir.main() {
        let DefKind::Func { body, .. } = &main.kind else {
            unreachable!("ICE")
        };
        let body_ty = checker.infer_expr(hir, main.module, *body);
        checker.constrain_eq(
            body_ty,
            PartialTy::unit(),
            hir.expr_span(*body),
            main.module,
        );
    }

    checker.unify(hir);

    checker.sub_all(hir)
}

impl TypeChecker<'_> {
    fn var_ty(&mut self, hir: &Hir, var: VarId) -> &PartialTy {
        let table = &mut self.table;
        self.ctx
            .entry(var)
            .expect("keys are never removed from the context")
            .or_insert_with(|| types::convert(table, hir.try_var_ty(var)))
    }

    fn constrain_eq(
        &mut self,
        ty_a: PartialTy,
        ty_b: PartialTy,
        span: Range<u32>,
        module: ModuleId,
    ) {
        self.constraints
            .push(Constraint::Eq(ty_a, ty_b, span, module));
    }

    fn constrain_field(
        &mut self,
        base_ty: PartialTy,
        base_span: Range<u32>,
        field_ty: PartialTy,
        field_name: SpanIdent,
        module: ModuleId,
    ) {
        self.constraints.push(Constraint::Field(
            base_ty, base_span, field_ty, field_name, module,
        ));
    }

    fn constrain_method(
        &mut self,
        base_ty: PartialTy,
        base_span: Range<u32>,
        method_ty: PartialTy,
        method_name: SpanIdent,
        module: ModuleId,
    ) {
        self.constraints.push(Constraint::Method(
            base_ty,
            base_span,
            method_ty,
            method_name,
            module,
        ));
    }
}
