mod error;
mod table;
#[cfg(test)]
mod test;

use std::iter;

use smallvec::{SmallVec, smallvec};

use ast::{
    Ast, Path,
    exprs::{Arg, Binding, Expr, ExprKind, MatchArm, Stmt},
    items::{AdtItem, AdtKind, ExecItem, ExecKind, Field, Param, Return, Variant},
    patterns::{Pat, PatKind},
    types::{Param as ParamTy, Ty, TyKind},
};
use ident::Ident;

use error::{ErrorKind, Result};
pub use table::{AdtId, AdtInfo, NameTable, VarId, VarInfo};

use crate::table::{AdtTable, PartialAdtTable, PartialVarTable};

type Scope<Id> = im::HashMap<Ident, Id, foldhash::fast::RandomState>;

pub fn resolve(ast: Ast<(), Ident, Ident>) -> Result<(Vec<ExecItem<(), AdtId, VarId>>, NameTable)> {
    let mut adt_table = PartialAdtTable::default();
    let mut adt_map = Scope::default();

    for item in &ast.adts {
        if adt_map
            .insert(item.ident.ident, adt_table.reserve())
            .is_some()
        {
            return Err(ErrorKind::DupItem(item.ident.ident).span(item.span));
        }
    }
    ast.adts
        .into_iter()
        .try_for_each(|adt| resolve_adt_item(&mut adt_table, &mut adt_map, adt))?;

    let mut adt_table = adt_table.finalise();

    let mut var_table = PartialVarTable::default();
    let mut var_map = Scope::default();

    for item in &ast.execs {
        if var_map.insert(item.ident, var_table.reserve()).is_some() {
            return Err(ErrorKind::DupItem(item.ident).span(item.ident_span));
        }
    }
    let execs = ast
        .execs
        .into_iter()
        .map(|exec| resolve_exec_item(&mut adt_table, &adt_map, &mut var_table, &mut var_map, exec))
        .collect::<Result<_>>()?;

    let var_table = var_table.finalise();

    Ok((
        execs,
        NameTable {
            adts: adt_table,
            vars: var_table,
        },
    ))
}

fn resolve_adt_item(
    adt_table: &mut PartialAdtTable,
    adt_scope: &Scope<AdtId>,
    //var_scope: &mut Scope<VarId>,
    item: AdtItem<Ident>,
) -> Result<()> {
    let &id = adt_scope.get(&item.ident.ident).expect(
        "all ast idents, including this one, should have already been inserted into the scope",
    );

    let generics: SmallVec<_> = item
        .generics
        .iter()
        .map(|&g| adt_table.insert(AdtInfo::Param(g)))
        .collect();

    let mut scope = adt_scope.clone();
    scope.extend(iter::zip(item.generics, generics.iter().copied()));

    let kind = match item.kind {
        AdtKind::Record(fields) => {
            let fields = resolve_fields(&scope, fields)?;

            //todo!("Constructors");
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
                        fields: resolve_fields(&scope, variant.fields)?,
                    })
                })
                .collect::<Result<_>>()?;
            AdtKind::Enum(variants)
        }
    };

    adt_table.fulfill(
        id,
        AdtInfo::Item(AdtItem {
            ident: item.ident,
            generics,
            span: item.span,
            kind,
        }),
    );

    Ok(())
}

fn resolve_fields(scope: &Scope<AdtId>, fields: Vec<Field<Ident>>) -> Result<Vec<Field<AdtId>>> {
    fields
        .into_iter()
        .map(|field| {
            Ok(Field {
                ident: field.ident,
                ty: resolve_ty(scope, field.ty)?,
                span: field.span,
            })
        })
        .collect()
}

fn resolve_exec_item(
    adt_table: &mut AdtTable,
    adt_scope: &Scope<AdtId>,
    var_table: &mut PartialVarTable,
    var_scope: &Scope<VarId>,
    item: ExecItem<(), Ident, Ident>,
) -> Result<ExecItem<(), AdtId, VarId>> {
    let &id = var_scope.get(&item.ident).expect(
        "all exec item idents, including this one, should have already been inserted into the scope",
    );

    match item.kind {
        ExecKind::Const { ty, val } => {
            let ty = ty.map(|ty| resolve_ty(adt_scope, ty)).transpose()?;
            let val = resolve_expr(adt_table, adt_scope, var_table, var_scope, val)?;

            var_table.fulfill(
                id,
                VarInfo {
                    ident: item.ident,
                    mutable: false,
                    ty: ty.clone(),
                    span: item.ident_span,
                },
            );

            Ok(ExecItem {
                ident: id,
                ident_span: item.ident_span,
                kind: ExecKind::Const { ty, val },
            })
        }
        ExecKind::Fn {
            generics: old_generics,
            params,
            result,
            body,
        } => {
            let mut adt_scope = adt_scope.clone();
            let mut var_scope = var_scope.clone();

            let generics: SmallVec<_> = old_generics
                .iter()
                .map(|&g| adt_table.insert(AdtInfo::Param(g)))
                .collect();
            adt_scope.extend(iter::zip(old_generics, generics.iter().copied()));

            let params = params
                .into_iter()
                .map(|p| {
                    let ty = resolve_ty(&adt_scope, p.ty)?;
                    let pat = resolve_pat(
                        adt_table,
                        &adt_scope,
                        var_table,
                        &mut var_scope,
                        p.pat,
                        p.mutable,
                        Some(ty.clone()),
                    );
                    Ok(Param {
                        mutable: p.mutable,
                        pat,
                        ty,
                    })
                })
                .collect::<Result<Vec<_>>>()?;

            let result = Return {
                mutable: result.mutable,
                ty: resolve_ty(&adt_scope, result.ty)?,
            };

            let body = resolve_expr(adt_table, &adt_scope, var_table, &mut var_scope, body)?;

            let ty = TyKind::Fn {
                params: params
                    .iter()
                    .map(|p| ParamTy {
                        mutable: p.mutable,
                        ty: p.ty.clone(),
                    })
                    .collect(),
                result: Box::new(result.ty.clone()),
            }
            .span(item.ident_span.end..result.ty.span.end);

            var_table.fulfill(
                id,
                VarInfo {
                    ident: item.ident,
                    mutable: false,
                    ty: Some(ty),
                    span: item.ident_span,
                },
            );

            Ok(ExecItem {
                ident: id,
                ident_span: item.ident_span,
                kind: ExecKind::Fn {
                    generics,
                    params,
                    result,
                    body,
                },
            })
        }
    }
}

fn resolve_expr(
    adt_table: &AdtTable,
    adt_scope: &Scope<AdtId>,
    var_table: &mut PartialVarTable,
    var_scope: &Scope<VarId>,
    expr: Expr<(), Ident, Ident>,
) -> Result<Expr<(), AdtId, VarId>> {
    let kind = match expr.kind {
        ExprKind::Path(path) => {
            if !path.prefix.is_empty() {
                todo!("handle paths")
            }

            let Some(&ident) = var_scope.get(&path.end) else {
                return Err(ErrorKind::UnboundVariable(path.end).span(expr.span));
            };
            ExprKind::Path(Path {
                prefix: smallvec![],
                end: ident,
            })
        }
        ExprKind::Lit(lit) => ExprKind::Lit(lit),
        ExprKind::Array(exprs) => ExprKind::Array(resolve_exprs(
            adt_table, adt_scope, var_table, var_scope, exprs,
        )?),
        ExprKind::Tuple(exprs) => ExprKind::Tuple(resolve_exprs(
            adt_table, adt_scope, var_table, var_scope, exprs,
        )?),
        ExprKind::Infix { op, lhs, rhs } => ExprKind::Infix {
            op,
            lhs: Box::new(resolve_expr(
                adt_table, adt_scope, var_table, var_scope, *lhs,
            )?),
            rhs: Box::new(resolve_expr(
                adt_table, adt_scope, var_table, var_scope, *rhs,
            )?),
        },
        ExprKind::Unary { op, expr } => ExprKind::Unary {
            op,
            expr: Box::new(resolve_expr(
                adt_table, adt_scope, var_table, var_scope, *expr,
            )?),
        },
        ExprKind::Field { base, field } => ExprKind::Field {
            base: Box::new(resolve_expr(
                adt_table, adt_scope, var_table, var_scope, *base,
            )?),
            field,
        },
        ExprKind::Index { arr, idx } => ExprKind::Index {
            arr: Box::new(resolve_expr(
                adt_table, adt_scope, var_table, var_scope, *arr,
            )?),
            idx: Box::new(resolve_expr(
                adt_table, adt_scope, var_table, var_scope, *idx,
            )?),
        },
        ExprKind::Call { func, args } => {
            let func = Box::new(resolve_expr(
                adt_table, adt_scope, var_table, var_scope, *func,
            )?);
            let args = args
                .into_iter()
                .map(|arg| {
                    Ok(Arg {
                        mutable: arg.mutable,
                        val: resolve_expr(adt_table, adt_scope, var_table, var_scope, arg.val)?,
                    })
                })
                .collect::<Result<_>>()?;
            ExprKind::Call { func, args }
        }
        ExprKind::Lamda {
            params,
            return_ty,
            body,
        } => {
            let mut var_scope = var_scope.clone();

            let params = params
                .into_iter()
                .map(|param| {
                    resolve_binding(adt_table, adt_scope, var_table, &mut var_scope, param)
                })
                .collect::<Result<Vec<_>>>()?;
            let return_ty = return_ty.map(|ty| resolve_ty(adt_scope, ty)).transpose()?;
            let body = Box::new(resolve_expr(
                adt_table,
                adt_scope,
                var_table,
                &mut var_scope,
                *body,
            )?);

            ExprKind::Lamda {
                params,
                return_ty,
                body,
            }
        }
        ExprKind::If { cond, th, el } => ExprKind::If {
            cond: Box::new(resolve_expr(
                adt_table, adt_scope, var_table, var_scope, *cond,
            )?),
            th: Box::new(resolve_expr(
                adt_table, adt_scope, var_table, var_scope, *th,
            )?),
            el: el
                .map(|el| {
                    resolve_expr(adt_table, adt_scope, var_table, var_scope, *el).map(Box::new)
                })
                .transpose()?,
        },
        ExprKind::Match { scrutinee, arms } => {
            let scrutinee = Box::new(resolve_expr(
                adt_table, adt_scope, var_table, var_scope, *scrutinee,
            )?);
            let arms = arms
                .into_iter()
                .map(|arm| {
                    let mut var_scope = var_scope.clone();
                    let pat = resolve_pat(
                        adt_table,
                        adt_scope,
                        var_table,
                        &mut var_scope,
                        arm.pat,
                        false,
                        None,
                    );
                    let body = resolve_expr(adt_table, adt_scope, var_table, &var_scope, arm.body)?;

                    Ok(MatchArm {
                        pat,
                        body,
                        span: arm.span,
                    })
                })
                .collect::<Result<_>>()?;
            ExprKind::Match { scrutinee, arms }
        }
        ExprKind::For { pat, iter, body } => {
            let mut var_scope = var_scope.clone();
            let pat = resolve_pat(
                adt_table,
                adt_scope,
                var_table,
                &mut var_scope,
                pat,
                false,
                None,
            );

            ExprKind::For {
                pat,
                iter: Box::new(resolve_expr(
                    adt_table, adt_scope, var_table, &var_scope, *iter,
                )?),
                body: Box::new(resolve_expr(
                    adt_table, adt_scope, var_table, &var_scope, *body,
                )?),
            }
        }
        ExprKind::Loop(body) => ExprKind::Loop(Box::new(resolve_expr(
            adt_table, adt_scope, var_table, var_scope, *body,
        )?)),
        ExprKind::Break => ExprKind::Break,
        ExprKind::Continue => ExprKind::Continue,
        ExprKind::Return(expr) => ExprKind::Return(Box::new(resolve_expr(
            adt_table, adt_scope, var_table, var_scope, *expr,
        )?)),
        ExprKind::Block(stmts) => {
            let mut var_scope = var_scope.clone();
            ExprKind::Block(
                stmts
                    .into_iter()
                    .map(|stmt| resolve_stmt(adt_table, adt_scope, var_table, &mut var_scope, stmt))
                    .collect::<Result<_>>()?,
            )
        }
    };

    Ok(kind.span(expr.span))
}

fn resolve_exprs(
    adt_table: &AdtTable,
    adt_scope: &Scope<AdtId>,
    var_table: &mut PartialVarTable,
    var_scope: &Scope<VarId>,
    exprs: Vec<Expr<(), Ident, Ident>>,
) -> Result<Vec<Expr<(), AdtId, VarId>>> {
    exprs
        .into_iter()
        .map(|expr| resolve_expr(adt_table, adt_scope, var_table, var_scope, expr))
        .collect()
}

fn resolve_stmt(
    adt_table: &AdtTable,
    adt_scope: &Scope<AdtId>,
    var_table: &mut PartialVarTable,
    var_scope: &mut Scope<VarId>,
    stmt: Stmt<(), Ident, Ident>,
) -> Result<Stmt<(), AdtId, VarId>> {
    match stmt {
        Stmt::Decl { binding, val, span } => {
            // Val must be resolved before binding, to ensure the declared variable isn't in scope during val
            let val = Box::new(resolve_expr(
                adt_table, adt_scope, var_table, var_scope, *val,
            )?);
            let binding = resolve_binding(adt_table, adt_scope, var_table, var_scope, binding)?;

            Ok(Stmt::Decl { binding, val, span })
        }
        Stmt::Expr(expr) => {
            resolve_expr(adt_table, adt_scope, var_table, var_scope, expr).map(Stmt::Expr)
        }
    }
}

fn resolve_binding(
    adt_table: &AdtTable,
    adt_scope: &Scope<AdtId>,
    var_table: &mut PartialVarTable,
    var_scope: &mut Scope<VarId>,
    binding: Binding<Ident, Ident>,
) -> Result<Binding<AdtId, VarId>> {
    let ty = binding.ty.map(|ty| resolve_ty(adt_scope, ty)).transpose()?;
    let pat = resolve_pat(
        adt_table,
        adt_scope,
        var_table,
        var_scope,
        binding.pat,
        binding.mutable,
        ty.clone(),
    );

    Ok(Binding {
        mutable: binding.mutable,
        pat,
        ty,
    })
}

fn resolve_ty(adt_scope: &Scope<AdtId>, ty: Ty<Ident>) -> Result<Ty<AdtId>> {
    let kind = match ty.kind {
        TyKind::Int => TyKind::Int,
        TyKind::UInt => TyKind::UInt,
        TyKind::Byte => TyKind::Byte,
        TyKind::Float => TyKind::Float,
        TyKind::Char => TyKind::Char,
        TyKind::Bool => TyKind::Bool,
        TyKind::Tuple(tys) => TyKind::Tuple(resolve_tys(adt_scope, tys)?),
        TyKind::Fn { params, result } => {
            let params = params
                .into_iter()
                .map(|param| {
                    Ok(ParamTy {
                        mutable: param.mutable,
                        ty: resolve_ty(&adt_scope, param.ty)?,
                    })
                })
                .collect::<Result<_>>()?;
            let result = Box::new(resolve_ty(&adt_scope, *result)?);
            TyKind::Fn { params, result }
        }
        TyKind::Adt(ident, args) => {
            let Some(id) = adt_scope.get(&ident).copied() else {
                return Err(ErrorKind::UnknownType(TyKind::Adt(ident, args)).span(ty.span));
            };
            let args = resolve_tys(adt_scope, args)?;
            TyKind::Adt(id, args)
        }
    };

    Ok(kind.span(ty.span))
}

fn resolve_tys(adt_scope: &Scope<AdtId>, tys: Vec<Ty<Ident>>) -> Result<Vec<Ty<AdtId>>> {
    tys.into_iter()
        .map(|ty| resolve_ty(adt_scope, ty))
        .collect()
}

fn resolve_pat(
    adt_table: &AdtTable,
    adt_scope: &Scope<AdtId>,
    var_table: &mut PartialVarTable,
    var_scope: &mut Scope<VarId>,
    pat: Pat<Ident>,
    mutable: bool,
    ty: Option<Ty<AdtId>>,
) -> Pat<VarId> {
    let kind = match pat.kind {
        PatKind::Wildcard => PatKind::Wildcard,
        PatKind::Literal { negate, lit } => PatKind::Literal { negate, lit },
        PatKind::Ident(ident) => {
            let id = var_table.insert(VarInfo {
                ident,
                mutable,
                ty,
                span: pat.span,
            });
            var_scope.insert(ident, id);
            PatKind::Ident(id)
        }
        PatKind::Constructor(ident, pats) => todo!(),
        PatKind::Tuple(old_pats) => {
            let tys = if let Some(Ty {
                kind: TyKind::Tuple(tys),
                ..
            }) = ty
            {
                tys
            } else {
                vec![]
            };

            let mut pats = Vec::new();
            for (pat, ty) in iter::zip(
                old_pats,
                tys.into_iter().map(Some).chain(iter::repeat(None)),
            ) {
                pats.push(resolve_pat(
                    adt_table, adt_scope, var_table, var_scope, pat, mutable, ty,
                ));
            }

            PatKind::Tuple(pats)
        }
    };

    kind.span(pat.span)
}
