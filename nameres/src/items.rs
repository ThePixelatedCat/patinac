use std::iter;

use ast::{
    items::{AdtItem, AdtKind, ExecItem, ExecKind, Field, Param, Variant},
    types::{Param as ParamTy, TyKind},
};

use ident::Ident;
use smallvec::SmallVec;

use crate::{
    Scope, bind_pat,
    error::{ErrorKind, Result},
    exprs::resolve_expr,
    table::{AdtId, AdtInfo, NameTable, VarId, VarInfo},
    types::resolve_ty,
};

pub fn resolve_adt_item(
    table: &mut NameTable,
    adt_scope: &mut Scope<AdtId>,
    var_scope: &mut Scope<VarId>,
    item: AdtItem<Ident>,
) -> Result<()> {
    let generics: SmallVec<_> = item
        .generics
        .iter()
        .map(|&g| table.insert_adt(AdtInfo::Param(g)))
        .collect();

    let res = table.reserve_adt();
    match adt_scope.insert(item.ident.ident, res.id()) {
        Some(old_id) if let AdtInfo::Item(old_item) = &res.table()[old_id] => {
            return Err(ErrorKind::DupItem {
                ident: item.ident.ident,
                first: old_item.span,
            }
            .span(item.span));
        }
        _ => {}
    }

    let mut scope = adt_scope.clone();
    scope.extend(iter::zip(item.generics, generics.iter().copied()));

    let kind = match item.kind {
        AdtKind::Record(fields) => {
            let fields = resolve_fields(res.table(), &scope, fields)?;

            // let fn_type = TyKind::Fn {
            //     generics: generics.clone(),
            //     params: (),
            //     result: Box::new(TyKind::Adt(res.id(), generics.iter().map(|p|)).span(item.ident.span)),
            // };

            AdtKind::Record(fields)
        }
        AdtKind::Enum(variants) => {
            let variants = variants
                .into_iter()
                .map(|variant| {
                    Ok(Variant {
                        ident: variant.ident,
                        fields: resolve_fields(res.table(), &scope, variant.fields)?,
                    })
                })
                .collect::<Result<_>>()?;
            AdtKind::Enum(variants)
        }
    };

    res.check_in(AdtInfo::Item(AdtItem {
        ident: item.ident,
        generics,
        span: item.span,
        kind,
    }));

    Ok(())
}

fn resolve_fields(
    table: &NameTable,
    scope: &Scope<AdtId>,
    fields: Vec<Field<Ident>>,
) -> Result<Vec<Field<AdtId>>> {
    fields
        .into_iter()
        .map(|field| {
            Ok(Field {
                ident: field.ident,
                ty: resolve_ty(table, scope, field.ty)?,
                span: field.span,
            })
        })
        .collect()
}

pub fn resolve_exec_item(
    table: &mut NameTable,
    adt_scope: &Scope<AdtId>,
    var_scope: &mut Scope<VarId>,
    item: ExecItem<(), Ident, Ident>,
) -> Result<ExecItem<(), AdtId, VarId>> {
    let (kind, ty) = match item.kind {
        ExecKind::Const { ty, val } => {
            let ty = ty.map(|ty| resolve_ty(table, adt_scope, ty)).transpose()?;
            let val = resolve_expr(table, adt_scope, var_scope, val)?;

            (
                ExecKind::Const {
                    ty: ty.clone(),
                    val,
                },
                ty,
            )
        }
        ExecKind::Func {
            generics: old_generics,
            params,
            return_ty,
            body,
        } => {
            let mut adt_scope = adt_scope.clone();
            let mut var_scope = var_scope.clone();

            let generics: SmallVec<_> = old_generics
                .iter()
                .map(|&g| table.insert_adt(AdtInfo::Param(g)))
                .collect();
            adt_scope.extend(iter::zip(old_generics, generics.iter().copied()));

            let params = params
                .into_iter()
                .map(|p| {
                    Ok(Param {
                        mutable: p.mutable,
                        pat: p.pat,
                        ty: resolve_ty(table, &adt_scope, p.ty)?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            for p in &params {
                bind_pat(
                    table,
                    &adt_scope,
                    &mut var_scope,
                    p.pat.clone(),
                    p.mutable,
                    Some(p.ty.clone()),
                );
            }

            let return_ty = resolve_ty(table, &adt_scope, return_ty)?;

            let body = resolve_expr(table, &adt_scope, &var_scope, body)?;

            let ty = TyKind::Fn {
                params: params
                    .iter()
                    .map(|p| ParamTy {
                        mutable: p.mutable,
                        ty: p.ty.clone(),
                    })
                    .collect(),
                result: Box::new(return_ty.clone()),
            }
            .span(item.ident.span.end..return_ty.span.end);

            (
                ExecKind::Func {
                    generics,
                    params,
                    return_ty,
                    body,
                },
                Some(ty),
            )
        }
    };

    let id = table.insert_var(VarInfo {
        ident: item.ident.ident,
        mutable: false,
        ty,
        span: item.ident.span,
    });
    var_scope.insert(item.ident.ident, id);

    Ok(ExecItem {
        ident: item.ident,
        kind,
        span: item.span,
    })
}
