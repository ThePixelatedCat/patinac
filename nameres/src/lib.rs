mod error;
mod exprs;
mod items;
mod table;
mod types;

use std::iter;

use ast::{
    Ast,
    items::ExecItem,
    patterns::{Pat, PatKind},
    types::{Ty, TyKind},
};

use ident::Ident;

use error::Result;
use items::{resolve_adt_item, resolve_exec_item};
use table::{AdtId, NameTable, VarId, VarInfo};

type Scope<Id> = im::HashMap<Ident, Id, foldhash::fast::RandomState>;

pub fn resolve(ast: Ast<(), Ident, Ident>) -> Result<(Vec<ExecItem<(), AdtId, VarId>>, NameTable)> {
    let mut table = NameTable::default();
    let mut adt_map = Scope::default();
    let mut var_map = Scope::default();

    ast.adts
        .into_iter()
        .try_for_each(|adt| resolve_adt_item(&mut table, &mut adt_map, &mut var_map, adt))?;

    let execs = ast
        .execs
        .into_iter()
        .map(|exec| resolve_exec_item(&mut table, &adt_map, &mut var_map, exec))
        .collect::<Result<_>>()?;

    Ok((execs, table))
}

fn bind_pat(
    table: &mut NameTable,
    adt_scope: &Scope<AdtId>,
    var_scope: &mut Scope<VarId>,
    pat: Pat,
    mutable: bool,
    ty: Option<Ty<AdtId>>,
) {
    match pat.kind {
        PatKind::Literal { .. } | PatKind::Wildcard => {}
        PatKind::Ident(ident) => {
            let id = table.insert_var(VarInfo {
                ident,
                mutable,
                ty,
                span: pat.span,
            });
            var_scope.insert(ident, id);
        }
        PatKind::Constructor(ident, pats) => {}
        PatKind::Tuple(pats) => {
            let tys = if let Some(Ty {
                kind: TyKind::Tuple(tys),
                ..
            }) = ty
            {
                tys
            } else {
                vec![]
            };

            for (pat, ty) in iter::zip(pats, tys.into_iter().map(Some).chain(iter::repeat(None))) {
                bind_pat(table, adt_scope, var_scope, pat, mutable, ty);
            }
        }
    }
}
