mod error;
mod table;
#[cfg(test)]
mod test;

use std::iter;

use foldhash::{HashMap, HashSet};
use itertools::Itertools;
use smallvec::{SmallVec, smallvec};

use ast::{
    Ast, Path,
    exprs::{Arg, Binding, Expr, ExprKind, MatchArm, Stmt},
    items::{AdtItem, AdtKind, ExecItem, ExecKind, Field, Param},
    patterns::{Pat, PatKind},
};
use error::{ErrorKind, Result};
use ident::{Ident, SpanIdent};
use span::Span;
use types::{Param as ParamTy, Return, Ty};

pub use table::*;

type Scope<Id> = im::HashMap<Ident, Id, foldhash::fast::RandomState>;

pub fn resolve(
    ast: Ast<(), SpanIdent, Ident>,
) -> Result<(Vec<ExecItem<(), AdtId, VarId>>, NameTable)> {
    let mut adt_table = PartialAdtTable::default();
    let mut adt_scope = Scope::default();

    // Used to track the spans of the item idents, for error reporting in the case of duplicated items
    let mut spans: HashMap<AdtId, Span> = HashMap::default();
    for item in &ast.adts {
        if let Some(id) = adt_scope.get(&item.ident.ident) {
            return Err(ErrorKind::DupItem(item.ident.ident, spans[id]).span(item.ident.span));
        }

        let id = adt_table.reserve();
        adt_scope.insert(item.ident.ident, id);
        spans.insert(id, item.ident.span);
    }
    ast.adts
        .into_iter()
        .try_for_each(|adt| resolve_adt_item(&mut adt_table, &adt_scope, adt))?;

    let mut adt_table = adt_table.finalise();

    let mut var_table = PartialVarTable::default();
    let mut var_scope = Scope::default();

    // Used to track the spans of the item idents, for error reporting in the case of duplicated items
    let mut spans: HashMap<VarId, Span> = HashMap::default();
    for item in &ast.execs {
        if let Some(id) = var_scope.get(&item.ident) {
            return Err(ErrorKind::DupItem(item.ident, spans[id]).span(item.ident_span));
        }

        let id = var_table.reserve();
        var_scope.insert(item.ident, id);
        spans.insert(id, item.ident_span);
    }
    let execs = ast
        .execs
        .into_iter()
        .map(|exec| resolve_exec_item(&mut adt_table, &adt_scope, &mut var_table, &var_scope, exec))
        .try_collect()?;

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
    item: AdtItem<SpanIdent>,
) -> Result<()> {
    let &id = adt_scope.get(&item.ident.ident).expect(
        "all ast idents, including this one, should have already been inserted into the scope",
    );

    let generics: SmallVec<_> = item
        .generics
        .iter()
        .map(|&g| adt_table.insert(AdtInfo::param(g)))
        .collect();

    let mut scope = adt_scope.clone();
    scope.extend(iter::zip(
        item.generics.iter().map(|i| i.ident),
        generics.iter().copied(),
    ));

    match item.kind {
        AdtKind::Record(fields) => {
            let fields = resolve_fields(&scope, fields)?;

            //todo!("Constructors");
            // let fn_type = Ty::Fn {
            //     generics: generics.clone(),
            //     params: (),
            //     result: Box::new(Ty::Adt(res.id(), generics.iter().map(|p|)).span(item.ident.span)),
            // };

            adt_table.fulfill(
                id,
                AdtInfo {
                    ident: item.ident,
                    kind: AdtInfoKind::Record { generics, fields },
                },
            );
        }
        AdtKind::Enum(variants) => {
            let variants = variants
                .into_iter()
                .map(|variant| Ok((variant.ident.ident, resolve_fields(&scope, variant.fields)?)))
                .try_collect()?;
            adt_table.fulfill(
                id,
                AdtInfo {
                    ident: item.ident,
                    kind: AdtInfoKind::Enum { generics, variants },
                },
            );
        }
    }

    Ok(())
}

fn resolve_fields(
    scope: &Scope<AdtId>,
    fields: Vec<Field<SpanIdent>>,
) -> Result<HashMap<Ident, FieldInfo>> {
    fields
        .into_iter()
        .map(|field| {
            Ok((
                field.ident,
                FieldInfo {
                    ty: resolve_ty(scope, field.ty)?,
                    span: field.span,
                },
            ))
        })
        .collect()
}

fn resolve_exec_item(
    adt_table: &mut AdtTable,
    adt_scope: &Scope<AdtId>,
    var_table: &mut PartialVarTable,
    var_scope: &Scope<VarId>,
    item: ExecItem<(), SpanIdent, Ident>,
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
            ret_mut,
            ret_ty,
            body,
        } => {
            let mut adt_scope = adt_scope.clone();
            let mut var_scope = var_scope.clone();

            let generics: SmallVec<_> = old_generics
                .iter()
                .map(|&g| adt_table.insert(AdtInfo::param(g)))
                .collect();
            adt_scope.extend(iter::zip(
                old_generics.iter().map(|i| i.ident),
                generics.iter().copied(),
            ));

            let params: Vec<_> = params
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
                .try_collect()?;

            let ret_ty = resolve_ty(&adt_scope, ret_ty)?;

            let body = resolve_expr(adt_table, &adt_scope, var_table, &var_scope, body)?;

            let ty = Ty::Fn(
                params
                    .iter()
                    .map(|p| ParamTy {
                        mutable: p.mutable,
                        ty: p.ty.clone(),
                    })
                    .collect(),
                Box::new(Return {
                    mutable: ret_mut,
                    ty: ret_ty.clone(),
                }),
            );

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
                    ret_mut,
                    ret_ty,
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
    expr: Expr<(), SpanIdent, Ident>,
) -> Result<Expr<(), AdtId, VarId>> {
    let kind = match expr.kind {
        ExprKind::Path(path) => {
            if !path.prefix.is_empty() {
                todo!("handle paths")
            }

            match var_scope.get(&path.end) {
                Some(&ident) => ExprKind::Path(Path {
                    prefix: smallvec![],
                    end: ident,
                }),
                None => return Err(ErrorKind::UnboundVariable(path.end).span(expr.span)),
            }
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
                .try_collect()?;
            ExprKind::Call { func, args }
        }
        ExprKind::Lambda { params, body } => {
            let mut var_scope = var_scope.clone();

            // Rebind all mutable captures as immutable within the lambda body
            let mut captures = HashSet::default();
            collect_captures(&mut captures, &body);
            for capture in captures {
                // Unbound variables will be caught in a few lines anyway, so doesn't matter if don't rebind them as immutable
                // The only partially-resolved variables will be the top-level items, which are already always immutable
                if let Some(&id) = var_scope.get(&capture)
                    && let Some(info) = &var_table[id]
                    && info.mutable
                {
                    let ident = info.ident;
                    let id = var_table.insert(VarInfo {
                        mutable: false,
                        ..info.clone()
                    });
                    var_scope.insert(ident, id);
                }
            }

            let params = params
                .into_iter()
                .map(|param| {
                    resolve_binding(adt_table, adt_scope, var_table, &mut var_scope, param)
                })
                .try_collect()?;
            let body = Box::new(resolve_expr(
                adt_table, adt_scope, var_table, &var_scope, *body,
            )?);

            ExprKind::Lambda { params, body }
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
                .try_collect()?;
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
                    .try_collect()?,
            )
        }
    };

    Ok(kind.span(expr.span))
}

fn collect_captures(captures: &mut HashSet<Ident>, expr: &Expr<(), SpanIdent, Ident>) {
    match &expr.kind {
        ExprKind::Path(path) => {
            if !path.prefix.is_empty() {
                todo!("handle paths")
            }
            captures.insert(path.end);
        }
        ExprKind::Lit(_) | ExprKind::Break | ExprKind::Continue => {}
        ExprKind::Array(exprs) | ExprKind::Tuple(exprs) => {
            for e in exprs {
                collect_captures(captures, e);
            }
        }
        ExprKind::Lambda { body: e, .. }
        | ExprKind::Field { base: e, .. }
        | ExprKind::Unary { expr: e, .. }
        | ExprKind::Loop(e)
        | ExprKind::Return(e) => collect_captures(captures, e),
        ExprKind::For {
            iter: e1, body: e2, ..
        }
        | ExprKind::Infix {
            lhs: e1, rhs: e2, ..
        }
        | ExprKind::Index { arr: e1, idx: e2 } => {
            collect_captures(captures, e1);
            collect_captures(captures, e2);
        }
        ExprKind::Call { func, args } => {
            collect_captures(captures, func);
            for a in args {
                collect_captures(captures, &a.val);
            }
        }
        ExprKind::If { cond, th, el } => {
            collect_captures(captures, cond);
            collect_captures(captures, th);
            el.as_ref().inspect(|el| collect_captures(captures, el));
        }
        ExprKind::Match { scrutinee, arms } => {
            collect_captures(captures, scrutinee);
            for a in arms {
                collect_captures(captures, &a.body);
            }
        }
        ExprKind::Block(stmts) => {
            for s in stmts {
                match s {
                    Stmt::Decl { val, .. } => collect_captures(captures, val),
                    Stmt::Expr(expr) => collect_captures(captures, expr),
                }
            }
        }
    }
}

fn resolve_exprs(
    adt_table: &AdtTable,
    adt_scope: &Scope<AdtId>,
    var_table: &mut PartialVarTable,
    var_scope: &Scope<VarId>,
    exprs: Vec<Expr<(), SpanIdent, Ident>>,
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
    stmt: Stmt<(), SpanIdent, Ident>,
) -> Result<Stmt<(), AdtId, VarId>> {
    match stmt {
        Stmt::Decl { binding, val, span } => {
            // val must be resolved before binding, to ensure the declared variable isn't in scope within it's own declaration
            let val = resolve_expr(adt_table, adt_scope, var_table, var_scope, val)?;
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
    binding: Binding<SpanIdent, Ident>,
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

fn resolve_ty(adt_scope: &Scope<AdtId>, ty: Ty<SpanIdent>) -> Result<Ty<AdtId>> {
    match ty {
        Ty::Int => Ok(Ty::Int),
        Ty::UInt => Ok(Ty::UInt),
        Ty::Byte => Ok(Ty::Byte),
        Ty::Float => Ok(Ty::Float),
        Ty::Char => Ok(Ty::Char),
        Ty::Bool => Ok(Ty::Bool),
        Ty::Tuple(tys) => Ok(Ty::Tuple(resolve_tys(adt_scope, tys)?)),
        Ty::Fn(params, ret) => {
            let params = params
                .into_iter()
                .map(|param| {
                    Ok(ParamTy {
                        mutable: param.mutable,
                        ty: resolve_ty(adt_scope, param.ty)?,
                    })
                })
                .try_collect()?;
            let ret = Box::new(Return {
                mutable: ret.mutable,
                ty: resolve_ty(adt_scope, ret.ty)?,
            });
            Ok(Ty::Fn(params, ret))
        }
        Ty::Adt(ident, args) => match adt_scope.get(&ident.ident).copied() {
            Some(id) => Ok(Ty::Adt(id, resolve_tys(adt_scope, args)?)),
            None => Err(ErrorKind::UnknownType(Ty::Adt(ident, args)).span(ident.span)),
        },
    }
}

fn resolve_tys(adt_scope: &Scope<AdtId>, tys: Vec<Ty<SpanIdent>>) -> Result<Vec<Ty<AdtId>>> {
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
            let tys = match ty {
                Some(Ty::Tuple(tys)) => tys,
                _ => vec![],
            };

            let pats = iter::zip(
                old_pats,
                tys.into_iter().map(Some).chain(iter::repeat(None)),
            )
            .map(|(pat, ty)| {
                resolve_pat(adt_table, adt_scope, var_table, var_scope, pat, mutable, ty)
            })
            .collect();

            PatKind::Tuple(pats)
        }
    };

    kind.span(pat.span)
}
