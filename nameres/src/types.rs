use ast::types::{Param, Ty, TyKind};

use ident::Ident;

use crate::{
    Scope,
    error::{ErrorKind, Result},
    table::{AdtId, NameTable},
};

pub fn resolve_ty(table: &NameTable, adt_scope: &Scope<AdtId>, ty: Ty<Ident>) -> Result<Ty<AdtId>> {
    let kind = match ty.kind {
        TyKind::Int => TyKind::Int,
        TyKind::UInt => TyKind::UInt,
        TyKind::Byte => TyKind::Byte,
        TyKind::Float => TyKind::Float,
        TyKind::Char => TyKind::Char,
        TyKind::Bool => TyKind::Bool,
        TyKind::Tuple(tys) => TyKind::Tuple(resolve_tys(table, adt_scope, tys)?),
        TyKind::Fn { params, result } => {
            let params = params
                .into_iter()
                .map(|param| {
                    Ok(Param {
                        mutable: param.mutable,
                        ty: resolve_ty(table, &adt_scope, param.ty)?,
                    })
                })
                .collect::<Result<_>>()?;
            let result = Box::new(resolve_ty(table, &adt_scope, *result)?);
            TyKind::Fn { params, result }
        }
        TyKind::Adt(ident, args) => {
            let Some(ident) = adt_scope.get(&ident).copied() else {
                return Err(ErrorKind::UnknownType(TyKind::Adt(ident, args)).span(ty.span));
            };
            let args = resolve_tys(table, adt_scope, args)?;
            TyKind::Adt(ident, args)
        }
    };

    Ok(kind.span(ty.span))
}

fn resolve_tys(
    table: &NameTable,
    adt_scope: &Scope<AdtId>,
    tys: Vec<Ty<Ident>>,
) -> Result<Vec<Ty<AdtId>>> {
    tys.into_iter()
        .map(|ty| resolve_ty(table, adt_scope, ty))
        .collect()
}
