mod error;
mod infer;
mod substitute;
#[cfg(test)]
mod test;
mod type_vars;
mod unify;

use ena::unify::InPlaceUnificationTable;

use ast::{
    exprs::Expr,
    items::{ExecItem, ExecKind},
};
use itertools::Itertools;
use nameres::{AdtId, NameTable, VarId};
use span::Span;
use types::Ty;

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
}

impl TypeChecker {
    pub fn new() -> Self {
        Self::default()
    }
}

impl TypeChecker {
    pub fn type_program(
        &mut self,
        name_table: &mut NameTable,
        execs: Vec<ExecItem<(), AdtId, VarId>>,
    ) -> Result<Vec<ExecItem<Ty<AdtId>, AdtId, VarId>>> {
        let execs = execs
            .into_iter()
            .map(|exec| {
                let kind = match exec.kind {
                    ExecKind::Const { ty, val } => {
                        // Constraint generation
                        let val = self.infer_expr(name_table, val)?;

                        {
                            let ty = self.convert(ty.as_ref());
                            self.constrain_eq(&val, ty);
                        }

                        self.unify()?;

                        ExecKind::Const {
                            ty,
                            val: self.sub_expr(&mut name_table.vars, val)?,
                        }
                    }
                    ExecKind::Fn {
                        generics,
                        params,
                        ret_mut,
                        ret_ty,
                        body,
                    } => {
                        let body = self.infer_expr(name_table, body)?;

                        {
                            let ret_ty = self.convert(Some(&ret_ty));
                            self.constrain_eq(&body, ret_ty);
                        }

                        self.unify()?;

                        ExecKind::Fn {
                            generics,
                            params,
                            ret_mut,
                            ret_ty,
                            body: self.sub_expr(&mut name_table.vars, body)?,
                        }
                    }
                };

                Ok(ExecItem {
                    ident: exec.ident,
                    ident_span: exec.ident_span,
                    kind,
                })
            })
            .try_collect()?;

        Ok(execs)
    }

    fn fresh_var(&mut self) -> PartialTy {
        PartialTy::Var(self.table.new_key(None))
    }

    fn fresh_int_var(&mut self) -> PartialTy {
        PartialTy::IntVar(self.table.new_key(None))
    }

    fn constrain_eq(&mut self, a: &Expr<PartialTy, AdtId, VarId>, b: PartialTy) {
        self.constraints.push(Constraint {
            kind: ConstraintKind::TypeEqual(a.ty.clone(), b),
            span: a.span,
        });
    }

    fn constrain_either_eq(&mut self, a: PartialTy, tys: (PartialTy, PartialTy), span: Span) {
        self.constraints.push(Constraint {
            kind: ConstraintKind::EitherTypeEqual(a, tys),
            span,
        });
    }

    fn clear_constraints(&mut self) {
        self.constraints.clear();
    }

    fn convert(&mut self, ast_ty: Option<&Ty<AdtId>>) -> PartialTy {
        ast_ty.map_or_else(|| self.fresh_var(), PartialTy::from)
    }
}
