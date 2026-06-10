use std::{iter, range::Range};

use errors::Error;
use hir::Hir;
use ident::SpanIdent;

use crate::{
    Constraint, Table, TypeChecker,
    error::ErrorKind,
    types::{Param, PartialTy, TyVar},
};

impl TypeChecker<'_> {
    /// Unifies all types in the unification table.
    pub(super) fn unify(&mut self, hir: &Hir) {
        for constr in &self.constraints {
            match constr {
                Constraint::Eq(ty_a, ty_b, span) => {
                    if let Err(err) = unify_ty_ty(&mut self.table, *span, ty_a, ty_b) {
                        self.handler.err(err);
                    }
                }
                Constraint::Field(base_ty, base_span, field_ty, field_name) => {
                    if let Err(err) = unify_field_ty(
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
}

fn unify_field_ty(
    table: &mut Table,
    hir: &Hir,
    base_ty: &PartialTy,
    base_span: Range<usize>,
    field_ty: &PartialTy,
    field_name: SpanIdent,
) -> Result<(), Error<ErrorKind>> {
    let base_ty = normalize_ty(table, base_ty);

    let base_id = match base_ty {
        PartialTy::Named(id) => id,
        PartialTy::Var(_) => {
            return Err(ErrorKind::UninferredVarType
                .span(base_span)
                .with_static_ctx("type must be known by this point"));
        }
        no_fields_ty => {
            return Err(ErrorKind::NoFieldsType(no_fields_ty).span(base_span));
        }
    };

    let Some(decl_field_ty) = hir.ty_info(base_id).fields.get_ty(field_name.ident) else {
        return Err(ErrorKind::MissingField(base_ty, field_name.ident).span(field_name.span));
    };

    unify_ty_ty(table, field_name.span, field_ty, &decl_field_ty.into())
}

/// Recursively traverse two types until at least one is a type variable,
/// at which point we unify them in the table,
/// or until we can no longer traverse them or we know they're mismatched,
/// at which point we error.
fn unify_ty_ty(
    table: &mut Table,
    span: Range<usize>,
    unnorm_lhs: &PartialTy,
    unnorm_rhs: &PartialTy,
) -> Result<(), Error<ErrorKind>> {
    let lhs = normalize_ty(table, unnorm_lhs);
    let rhs = normalize_ty(table, unnorm_rhs);

    match (lhs, rhs) {
        (PartialTy::Int, PartialTy::Int)
        | (PartialTy::UInt, PartialTy::UInt)
        | (PartialTy::Byte, PartialTy::Byte)
        | (PartialTy::Float, PartialTy::Float)
        | (PartialTy::Bool, PartialTy::Bool)
        | (PartialTy::Char, PartialTy::Char) => Ok(()),
        (PartialTy::Tuple(lhs_elems), PartialTy::Tuple(rhs_elems)) => {
            if lhs_elems.len() != rhs_elems.len() {
                return Err(ErrorKind::TypesNotEqual(
                    PartialTy::Tuple(lhs_elems),
                    PartialTy::Tuple(rhs_elems),
                )
                .span(span));
            }
            iter::zip(lhs_elems, rhs_elems).try_for_each(|(l, r)| unify_ty_ty(table, span, &l, &r))
        }
        (PartialTy::Array(lhs_inner), PartialTy::Array(rhs_inner)) => {
            unify_ty_ty(table, span, &lhs_inner, &rhs_inner)
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
                unify_ty_ty(table, r.span, &r.ty, &l.ty)
            })?;
            unify_ty_ty(table, span, &lhs_ret, &rhs_ret)
        }
        (PartialTy::Named(a), PartialTy::Named(b)) if a == b => Ok(()),
        (PartialTy::IntVar(lhs_var), PartialTy::IntVar(rhs_var))
        | (PartialTy::Var(lhs_var), PartialTy::Var(rhs_var)) => table
            .unify_var_var(lhs_var, rhs_var)
            .map_err(|(l, r)| ErrorKind::TypesNotEqual(l, r).span(span)),
        (PartialTy::Var(var), ty) | (ty, PartialTy::Var(var)) => {
            if occurs_check(&ty, var) {
                return Err(ErrorKind::Infinite.span(span));
            }
            unify_var_value(table, span, var, ty)
        }
        (
            PartialTy::IntVar(int_var),
            int_ty @ (PartialTy::Int | PartialTy::UInt | PartialTy::Byte),
        )
        | (
            int_ty @ (PartialTy::Int | PartialTy::UInt | PartialTy::Byte),
            PartialTy::IntVar(int_var),
        ) => unify_var_value(table, span, int_var, int_ty),
        (lhs, rhs) => Err(ErrorKind::TypesNotEqual(lhs, rhs).span(span)),
    }
}

fn unify_var_value(
    table: &mut Table,
    span: Range<usize>,
    var: TyVar,
    ty: PartialTy,
) -> Result<(), Error<ErrorKind>> {
    table
        .unify_var_value(var, Some(ty))
        .map_err(|(l, r)| ErrorKind::TypesNotEqual(l, r).span(span))
}

pub(crate) fn normalize_ty(table: &mut Table, ty: &PartialTy) -> PartialTy {
    match ty {
        PartialTy::Int
        | PartialTy::UInt
        | PartialTy::Byte
        | PartialTy::Float
        | PartialTy::Bool
        | PartialTy::Char => ty.clone(),
        PartialTy::Tuple(tys) => {
            PartialTy::Tuple(tys.iter().map(|ty| normalize_ty(table, ty)).collect())
        }
        PartialTy::Array(ty) => PartialTy::Array(Box::new(normalize_ty(table, ty))),
        PartialTy::Fn(params, ret) => {
            let params = params
                .iter()
                .map(|param| Param {
                    ty: normalize_ty(table, &param.ty),
                    mutable: param.mutable,
                    span: param.span,
                })
                .collect();
            let ret = Box::new(normalize_ty(table, ret));
            PartialTy::Fn(params, ret)
        }
        PartialTy::Named(id) => PartialTy::Named(*id),
        PartialTy::Var(v) => match table.probe_value(*v) {
            Some(ty) => normalize_ty(table, &ty),
            None => PartialTy::Var(table.find(*v)),
        },
        PartialTy::IntVar(v) => match table.probe_value(*v) {
            Some(ty) => normalize_ty(table, &ty),
            None => PartialTy::IntVar(table.find(*v)),
        },
    }
}

fn occurs_check(ty: &PartialTy, var: TyVar) -> bool {
    match ty {
        PartialTy::Int
        | PartialTy::UInt
        | PartialTy::Byte
        | PartialTy::Float
        | PartialTy::Bool
        | PartialTy::Char
        | PartialTy::Named(_) => false,
        PartialTy::Tuple(tys) => tys.iter().any(|ty| occurs_check(ty, var)),
        PartialTy::Array(ty) => occurs_check(ty, var),
        PartialTy::Fn(params, ret) => {
            occurs_check(ret, var) || params.iter().any(|param| occurs_check(&param.ty, var))
        }
        PartialTy::Var(this_var) | PartialTy::IntVar(this_var) => *this_var == var,
    }
}
