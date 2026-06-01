use std::{iter, range::Range};

use ena::unify::InPlaceUnificationTable;

use errors::Error;
use hir::Hir;
use ident::SpanIdent;

use crate::{
    Constraint, TypeChecker,
    error::ErrorKind,
    types::{Param, PartialTy, TyVar},
};

fn occurs_check(span: Range<usize>, ty: &PartialTy, var: TyVar) -> Result<(), Error<ErrorKind>> {
    match ty {
        PartialTy::Int
        | PartialTy::UInt
        | PartialTy::Byte
        | PartialTy::Float
        | PartialTy::Bool
        | PartialTy::Char
        | PartialTy::Adt(_) => Ok(()),
        PartialTy::Tuple(tys) => tys.iter().try_for_each(|ty| occurs_check(span, ty, var)),
        PartialTy::Array(ty) => occurs_check(span, ty, var),
        PartialTy::Fn(params, ret) => {
            params
                .iter()
                .try_for_each(|param| occurs_check(param.span, &param.ty, var))?;
            occurs_check(span, ret, var)
        }
        PartialTy::Var(this_var) | PartialTy::IntVar(this_var) => {
            if *this_var == var {
                Err(ErrorKind::Infinite(var, PartialTy::Var(*this_var)).span(span))
            } else {
                Ok(())
            }
        }
    }
}

impl TypeChecker<'_> {
    /// Unifies all types in the unification table.
    pub(super) fn unify(&mut self, hir: &Hir) {
        for constr in &self.constraints {
            match constr {
                Constraint::Eq(ty_a, ty_b, span) => {
                    if let Err(err) = Self::unify_ty_ty(&mut self.table, *span, ty_a, ty_b) {
                        self.handler.err(err);
                    }
                }
                Constraint::HasField(base_ty, base_span, field_ty, field_name) => {
                    if let Err(err) = Self::unify_field_ty(
                        &mut self.table,
                        hir,
                        base_ty,
                        *base_span,
                        field_ty,
                        *field_name,
                    ) {
                        self.handler.err(err);
                    }
                }
            }
        }
    }

    fn unify_field_ty(
        table: &mut InPlaceUnificationTable<TyVar>,
        hir: &Hir,
        base_ty: &PartialTy,
        base_span: Range<usize>,
        field_ty: &PartialTy,
        field_name: SpanIdent,
    ) -> Result<(), Error<ErrorKind>> {
        let base_ty = Self::normalize_ty(table, base_ty);

        let base_id = match base_ty {
            PartialTy::Adt(id) => id,
            PartialTy::Var(_) => {
                return Err(ErrorKind::UninferredVarType
                    .span(base_span)
                    .with_static_ctx("type must be known by this point"));
            }
            no_fields_ty => {
                return Err(ErrorKind::NoFieldsType(no_fields_ty).span(base_span));
            }
        };

        let Some(decl_field_ty) = hir.adt_info(base_id).fields.get_ty(field_name.ident) else {
            return Err(ErrorKind::MissingField(base_ty, field_name.ident).span(field_name.span));
        };

        Self::unify_ty_ty(table, field_name.span, field_ty, &decl_field_ty.into())
    }

    /// Recursively traverse two types until at least one is a type variable,
    /// at which point we unify them in the table,
    /// or until we can no longer traverse them or we know they're mismatched,
    /// at which point we error.
    pub(super) fn unify_ty_ty(
        table: &mut InPlaceUnificationTable<TyVar>,
        span: Range<usize>,
        unnorm_lhs: &PartialTy,
        unnorm_rhs: &PartialTy,
    ) -> Result<(), Error<ErrorKind>> {
        let lhs = Self::normalize_ty(table, unnorm_lhs);
        let rhs = Self::normalize_ty(table, unnorm_rhs);

        match (lhs, rhs) {
            (PartialTy::Int, PartialTy::Int)
            | (PartialTy::UInt, PartialTy::UInt)
            | (PartialTy::Byte, PartialTy::Byte)
            | (PartialTy::Float, PartialTy::Float)
            | (PartialTy::Bool, PartialTy::Bool)
            | (PartialTy::Char, PartialTy::Char) => Ok(()),
            (PartialTy::Tuple(lhs_inners), PartialTy::Tuple(rhs_inners)) => {
                if lhs_inners.len() != rhs_inners.len() {
                    return Err(ErrorKind::TypesNotEqual(
                        PartialTy::Tuple(lhs_inners),
                        PartialTy::Tuple(rhs_inners),
                    )
                    .span(span));
                }
                Self::unify_tys(table, span, &lhs_inners, &rhs_inners)
            }
            (PartialTy::Array(lhs_inner), PartialTy::Array(rhs_inner)) => {
                Self::unify_ty_ty(table, span, &lhs_inner, &rhs_inner)
            }
            (PartialTy::Fn(lhs_params, lhs_ret), PartialTy::Fn(rhs_params, rhs_ret)) => {
                if lhs_params.len() != rhs_params.len() {
                    return Err(ErrorKind::ParamCount(
                        PartialTy::Fn(lhs_params, lhs_ret),
                        PartialTy::Fn(rhs_params, rhs_ret),
                    )
                    .span(span));
                }
                // Intentionally do the parameters "backwards" for proper errors (variance or something)
                iter::zip(rhs_params, lhs_params).try_for_each(|(r, l)| {
                    if r.mutable != l.mutable {
                        let span = l.span;
                        return Err(ErrorKind::ParamMutability(r, l).span(span));
                    }
                    Self::unify_ty_ty(table, r.span, &r.ty, &l.ty)
                })?;
                Self::unify_ty_ty(table, span, &lhs_ret, &rhs_ret)
            }
            (PartialTy::Adt(a), PartialTy::Adt(b)) if a == b => Ok(()),
            (PartialTy::IntVar(lhs_var), PartialTy::IntVar(rhs_var))
            | (PartialTy::Var(lhs_var), PartialTy::Var(rhs_var)) => {
                Self::unify_var_var(table, span, lhs_var, rhs_var)
            }
            (PartialTy::Var(var), ty) | (ty, PartialTy::Var(var)) => {
                occurs_check(span, &ty, var)?;
                Self::unify_var_value(table, span, var, ty)
            }
            (
                PartialTy::IntVar(int_var),
                int_ty @ (PartialTy::Int | PartialTy::UInt | PartialTy::Byte),
            )
            | (
                int_ty @ (PartialTy::Int | PartialTy::UInt | PartialTy::Byte),
                PartialTy::IntVar(int_var),
            ) => Self::unify_var_value(table, span, int_var, int_ty),
            (lhs, rhs) => Err(ErrorKind::TypesNotEqual(lhs, rhs).span(span)),
        }
    }

    fn unify_tys(
        table: &mut InPlaceUnificationTable<TyVar>,
        constr_span: Range<usize>,
        left_tys: &[PartialTy],
        right_tys: &[PartialTy],
    ) -> Result<(), Error<ErrorKind>> {
        iter::zip(left_tys, right_tys)
            .try_for_each(|(l, r)| Self::unify_ty_ty(table, constr_span, l, r))
    }

    fn unify_var_var(
        table: &mut InPlaceUnificationTable<TyVar>,
        span: Range<usize>,
        lhs: TyVar,
        rhs: TyVar,
    ) -> Result<(), Error<ErrorKind>> {
        table
            .unify_var_var(lhs, rhs)
            .map_err(|(l, r)| ErrorKind::TypesNotEqual(l, r).span(span))
    }

    fn unify_var_value(
        table: &mut InPlaceUnificationTable<TyVar>,
        span: Range<usize>,
        var: TyVar,
        ty: PartialTy,
    ) -> Result<(), Error<ErrorKind>> {
        table
            .unify_var_value(var, Some(ty))
            .map_err(|(l, r)| ErrorKind::TypesNotEqual(l, r).span(span))
    }

    pub(crate) fn normalize_ty(
        table: &mut InPlaceUnificationTable<TyVar>,
        ty: &PartialTy,
    ) -> PartialTy {
        match ty {
            PartialTy::Int
            | PartialTy::UInt
            | PartialTy::Byte
            | PartialTy::Float
            | PartialTy::Bool
            | PartialTy::Char => ty.clone(),
            PartialTy::Tuple(tys) => {
                PartialTy::Tuple(tys.iter().map(|ty| Self::normalize_ty(table, ty)).collect())
            }
            PartialTy::Array(ty) => PartialTy::Array(Box::new(Self::normalize_ty(table, ty))),
            PartialTy::Fn(params, ret) => {
                let params = params
                    .iter()
                    .map(|param| Param {
                        ty: Self::normalize_ty(table, &param.ty),
                        mutable: param.mutable,
                        span: param.span,
                    })
                    .collect();
                let ret = Box::new(Self::normalize_ty(table, ret));
                PartialTy::Fn(params, ret)
            }
            PartialTy::Adt(id) => PartialTy::Adt(*id),
            PartialTy::Var(v) => match table.probe_value(*v) {
                Some(ty) => Self::normalize_ty(table, &ty),
                None => PartialTy::Var(table.find(*v)),
            },
            PartialTy::IntVar(v) => match table.probe_value(*v) {
                Some(ty) => Self::normalize_ty(table, &ty),
                None => PartialTy::IntVar(table.find(*v)),
            },
        }
    }
}
