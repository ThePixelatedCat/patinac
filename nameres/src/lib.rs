mod error;
mod exprs;
#[cfg(test)]
mod test;

use itertools::Itertools;

use ast::{
    Ast,
    exprs::{Binding, LitExpr as AstLitExpr},
    items::{AdtItem, AdtKind, ExecItem as AstExecItem, ExecKind as AstExecKind},
    patterns::{Pat, PatKind},
    types::{Ty as AstTy, TyKind as AstTyKind},
};
use errors::ErrorHandler;
use hir::{
    Hir, VarId, VarInfo,
    exprs::{ExprId, LitExpr as HirLitExpr},
    items::{AdtId, AdtInfo, ExecItem as HirExecItem, ExecKind as HirExecKind},
    types::{Param as ParamTy, Ty as HirTy},
};
use ident::Ident;

use crate::error::{ErrorKind, Result};

type Scope<Id> = im_rc::HashMap<Ident, Id, foldhash::fast::RandomState>;

/// # Errors
/// Returns an error if there are any unbound variables, undefined types, or multiple items with the same name
pub fn resolve(mut ast: Ast, mut handler: ErrorHandler) -> Result<Hir> {
    let mut hir = Hir::default();

    let mut adt_scope = Scope::default();
    let mut var_scope = Scope::default();

    for adt in &ast.adts {
        match adt_scope.get(&adt.ident.ident) {
            Some(&id) => handler.err(
                ErrorKind::DupItem(adt.ident.ident, hir.adt_ident(id).span).span(adt.ident.span),
            ),
            None => {
                let id = hir.reserve_adt(adt.ident);
                adt_scope.insert(adt.ident.ident, id);
            }
        }
    }
    for adt in ast.adts {
        resolve_adt_item(&adt_scope, &mut var_scope, &mut hir, &mut handler, adt);
    }

    for exec in &ast.execs {
        match var_scope.get(&exec.ident.ident) {
            Some(&id) => handler.err(
                ErrorKind::DupItem(exec.ident.ident, hir.var_ident(id).span).span(exec.ident.span),
            ),
            None => {
                let id = hir.reserve_var(exec.ident);
                var_scope.insert(exec.ident.ident, id);
            }
        }
    }
    if let Some(idx) = find_main(&mut handler, &ast.execs)?
        && let Ok(main) = resolve_exec_item(
            &adt_scope,
            &var_scope,
            &mut hir,
            &mut handler,
            ast.execs.remove(idx),
        )
    {
        hir.set_main(main);
    }
    hir.execs = ast
        .execs
        .into_iter()
        .flat_map(|exec| resolve_exec_item(&adt_scope, &var_scope, &mut hir, &mut handler, exec))
        .collect();

    if handler.has_err() { Err(()) } else { Ok(hir) }
}

fn find_main(error_handler: &mut ErrorHandler, execs: &[AstExecItem]) -> Result<Option<usize>> {
    for (idx, item) in execs.iter().enumerate() {
        if let AstExecKind::Fn { params, ret_ty, .. } = &item.kind
            && item.ident.ident == "main"
        {
            if params.is_empty() && ret_ty.kind == AstTyKind::unit() {
                return Ok(Some(idx));
            } else {
                error_handler.err(ErrorKind::InvalidMain.span(item.ident.span));
                return Err(());
            }
        }
    }

    Ok(None)
}

fn resolve_adt_item(
    adt_scope: &Scope<AdtId>,
    var_scope: &mut Scope<VarId>,
    hir: &mut Hir,
    handler: &mut ErrorHandler,
    item: AdtItem,
) {
    let &id = adt_scope.get(&item.ident.ident).expect(
        "all ast idents, including this one, should have already been inserted into the scope",
    );

    if !item.generics.is_empty() {
        todo!("Generics")
    }

    match item.kind {
        AdtKind::Record(fields) => {
            let fields: Vec<_> = fields
                .into_iter()
                .flat_map(|field| {
                    Ok::<_, ()>((field.ident.ident, resolve_ty(adt_scope, handler, field.ty)?))
                })
                .collect();

            if let Some((dup, _)) = fields.iter().duplicates_by(|(id, _)| id).next() {
                handler.err(ErrorKind::DupFields(*dup).span(item.ident.span));
                return;
            }

            let constructor_ty = HirTy::Fn(
                fields
                    .iter()
                    .map(|(_, ty)| ParamTy {
                        mutable: false,
                        ty: ty.clone(),
                    })
                    .collect(),
                Box::new(HirTy::Adt(id)),
            );
            let constructor_id = hir.add_var(
                item.ident,
                VarInfo {
                    mutable: false,
                    ty: Some(constructor_ty),
                },
            );
            var_scope.insert(item.ident.ident, constructor_id);

            hir.fulfill_adt(
                id,
                AdtInfo {
                    fields: fields.into(),
                },
            );
        }
        AdtKind::Enum(_) => {
            todo!("Pattern Matching");
        }
    }
}

fn resolve_exec_item(
    adt_scope: &Scope<AdtId>,
    var_scope: &Scope<VarId>,
    hir: &mut Hir,
    handler: &mut ErrorHandler,
    item: AstExecItem,
) -> Result<HirExecItem> {
    let &id = var_scope.get(&item.ident.ident).expect(
        "all exec item idents, including this one, should have already been inserted into the scope",
    );

    match item.kind {
        AstExecKind::Const { ty, val } => {
            let ty = ty.map(|ty| resolve_ty(adt_scope, handler, ty)).transpose();
            let val = exprs::resolve_expr(adt_scope, var_scope, hir, handler, val);

            hir.fulfill_var(
                id,
                VarInfo {
                    mutable: false,
                    ty: ty?,
                },
            );

            Ok(HirExecItem {
                ident: id,
                kind: HirExecKind::Const { val: val? },
            })
        }
        AstExecKind::Fn {
            generics,
            params,
            ret_mut,
            ret_ty,
            body,
        } => {
            if !generics.is_empty() {
                todo!("Generics")
            }

            if ret_mut {
                todo!("Projections")
            }

            let mut var_scope = Scope::clone(var_scope);

            let params = params
                .into_iter()
                .map(|p| {
                    let ty = resolve_ty(adt_scope, handler, p.ty)?;
                    let id = resolve_pat(&mut var_scope, hir, p.pat, p.mutable, Some(ty.clone()));
                    Ok((
                        id,
                        ParamTy {
                            ty,
                            mutable: p.mutable,
                        },
                    ))
                })
                .try_collect();
            let body = exprs::resolve_expr(adt_scope, &var_scope, hir, handler, body)?;
            let (params, param_tys) = params?;

            hir.fulfill_var(
                id,
                VarInfo {
                    mutable: false,
                    ty: Some(HirTy::Fn(
                        param_tys,
                        Box::new(resolve_ty(adt_scope, handler, ret_ty)?),
                    )),
                },
            );

            Ok(HirExecItem {
                ident: id,
                kind: HirExecKind::Fn { params, body },
            })
        }
    }
}

fn resolve_binding(
    adt_scope: &Scope<AdtId>,
    var_scope: &mut Scope<VarId>,
    hir: &mut Hir,
    handler: &mut ErrorHandler,
    binding: Binding,
) -> Result<VarId> {
    let ty = binding
        .ty
        .map(|ty| resolve_ty(adt_scope, handler, ty))
        .transpose()?;
    Ok(resolve_pat(
        var_scope,
        hir,
        binding.pat,
        binding.mutable,
        ty,
    ))
}

fn resolve_ty(adt_scope: &Scope<AdtId>, handler: &mut ErrorHandler, ty: AstTy) -> Result<HirTy> {
    match ty.kind {
        AstTyKind::Int => Ok(HirTy::Int),
        AstTyKind::UInt => Ok(HirTy::UInt),
        AstTyKind::Byte => Ok(HirTy::Byte),
        AstTyKind::Float => Ok(HirTy::Float),
        AstTyKind::Char => Ok(HirTy::Char),
        AstTyKind::Bool => Ok(HirTy::Bool),
        AstTyKind::Tuple(tys) => Ok(HirTy::Tuple(resolve_tys(adt_scope, handler, tys)?)),
        AstTyKind::Fn(params, ret) => {
            if ret.mutable {
                todo!("Projections")
            }

            let params = params
                .into_iter()
                .map(|param| {
                    Ok(ParamTy {
                        mutable: param.mutable,
                        ty: resolve_ty(adt_scope, handler, param.ty)?,
                    })
                })
                .try_collect()?;
            let ret = Box::new(resolve_ty(adt_scope, handler, *ret.ty)?);
            Ok(HirTy::Fn(params, ret))
        }
        AstTyKind::Adt(ident, mut args) => {
            if ident == "Array" {
                match args.len() {
                    1 => resolve_ty(adt_scope, handler, args.swap_remove(0))
                        .map(Box::new)
                        .map(HirTy::Array),
                    len => {
                        handler.err(ErrorKind::GenericCount(1, len).span(ty.span));
                        Err(())
                    }
                }
            } else {
                if !args.is_empty() {
                    todo!("Generics")
                }

                match adt_scope.get(&ident).copied() {
                    Some(id) => Ok(HirTy::Adt(id)),
                    None => {
                        handler.err(ErrorKind::UnknownType.span(ty.span));
                        Err(())
                    }
                }
            }
        }
    }
}

fn resolve_tys(
    adt_scope: &Scope<AdtId>,
    handler: &mut ErrorHandler,
    tys: Vec<AstTy>,
) -> Result<Vec<HirTy>> {
    tys.into_iter()
        .map(|ty| resolve_ty(adt_scope, handler, ty))
        .try_collect()
}

fn resolve_pat(
    var_scope: &mut Scope<VarId>,
    hir: &mut Hir,
    pat: Pat,
    mutable: bool,
    ty: Option<HirTy>,
) -> VarId {
    match pat.kind {
        PatKind::Ident(ident) => {
            let id = hir.add_var(ident.span(pat.span), VarInfo { mutable, ty });
            var_scope.insert(ident, id);
            id
        }
        _ => todo!("Pattern Matching"),
    }
}

fn convert_lit(lit: AstLitExpr) -> HirLitExpr {
    match lit {
        AstLitExpr::Int(i) => HirLitExpr::Int(i),
        AstLitExpr::Float(f) => HirLitExpr::Float(f),
        AstLitExpr::Char(c) => HirLitExpr::Char(c),
        AstLitExpr::String(s) => HirLitExpr::String(s),
        AstLitExpr::Bool(b) => HirLitExpr::Bool(b),
    }
}

#[cfg(any(test, feature = "test"))]
pub fn test_resolve_expr(expr: ast::exprs::Expr) -> Result<(ExprId, Hir)> {
    use errors::TEST_HANDLER;

    let mut hir = Hir::default();
    let mut handler = TEST_HANDLER;
    let expr = exprs::resolve_expr(
        &Scope::default(),
        &Scope::default(),
        &mut hir,
        &mut handler,
        expr,
    )?;
    Ok((expr, hir))
}
