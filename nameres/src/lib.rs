mod error;
#[cfg(test)]
mod test;

use foldhash::{HashMap, HashSet};
use itertools::Itertools;

use ast::{
    Ast,
    exprs::{
        Binding as AstBinding, BlockExpr as AstBlockExpr, Expr as AstExpr, ExprKind,
        InfixOp as AstInfixOp, LitExpr as AstLitExpr, PrefixOp as AstPrefixOp, Stmt as AstStmt,
    },
    items::{AdtItem, AdtKind, ExecItem as AstExecItem, ExecKind as AstExecKind},
    patterns::{Pat as AstPat, PatKind as AstPatKind},
    types::{Ty as AstTy, TyKind as AstTyKind},
};
use hir::{
    Hir, VarId, VarInfo,
    exprs::{
        Arg as HirArg, BlockExpr as HirBlockExpr, Expr as HirExpr, ExprId, InfixOp as HirInfixOp,
        LitExpr as HirLitExpr, PrefixOp as HirPrefixOp, Stmt as HirStmt,
    },
    items::{AdtId, AdtInfo, ExecItem as HirExecItem, ExecKind as HirExecKind, Param},
    types::{Param as ParamTy, Return, Ty as HirTy},
};
use ident::Ident;

use crate::error::{ErrorKind, Result};

type Scope<Id> = im_rc::HashMap<Ident, Id, foldhash::fast::RandomState>;

/// # Errors
/// Returns an error if there are any unbound variables, undefined types, or multiple items with the same name
pub fn resolve(ast: Ast) -> Result<Hir> {
    let mut hir = Hir::default();

    let mut adt_scope = Scope::default();
    let mut var_scope = Scope::default();

    for item in &ast.adts {
        if let Some(&id) = adt_scope.get(&item.ident.ident) {
            return Err(
                ErrorKind::DupItem(item.ident.ident, hir.adt_ident(id).span).span(item.ident.span)
            );
        }

        let id = hir.reserve_adt(item.ident);
        adt_scope.insert(item.ident.ident, id);
    }
    ast.adts
        .into_iter()
        .try_for_each(|adt| resolve_adt_item(&adt_scope, &mut var_scope, &mut hir, adt))?;

    for item in &ast.execs {
        if let Some(&id) = var_scope.get(&item.ident.ident) {
            return Err(
                ErrorKind::DupItem(item.ident.ident, hir.var_ident(id).span).span(item.ident.span)
            );
        }

        let id = hir.reserve_var(item.ident);
        var_scope.insert(item.ident.ident, id);
    }
    hir.execs = ast
        .execs
        .into_iter()
        .map(|exec| resolve_exec_item(&adt_scope, &var_scope, &mut hir, exec))
        .try_collect()?;

    Ok(hir)
}

#[cfg(any(test, feature = "test"))]
pub fn test_resolve_expr(expr: AstExpr) -> Result<(ExprId, Hir)> {
    let mut hir = Hir::default();
    let expr = resolve_expr(&Scope::default(), &Scope::default(), &mut hir, expr)?;
    Ok((expr, hir))
}

fn resolve_adt_item(
    adt_scope: &Scope<AdtId>,
    var_scope: &mut Scope<VarId>,
    hir: &mut Hir,
    item: AdtItem,
) -> Result<()> {
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
                .map(|field| Ok((field.ident.ident, resolve_ty(adt_scope, field.ty)?)))
                .try_collect()?;

            let constructor_ty = HirTy::Fn(
                fields
                    .iter()
                    .map(|(_, ty)| ParamTy {
                        mutable: false,
                        ty: ty.clone(),
                    })
                    .collect(),
                Return {
                    mutable: false,
                    ty: Box::new(HirTy::Adt(id)),
                },
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
                    fields: HashMap::from_iter(fields),
                },
            );
        }
        AdtKind::Enum(_) => {
            todo!("Enums (Pattern Matching)");
        }
    }

    Ok(())
}

fn resolve_exec_item(
    adt_scope: &Scope<AdtId>,
    var_scope: &Scope<VarId>,
    hir: &mut Hir,
    item: AstExecItem,
) -> Result<HirExecItem> {
    let &id = var_scope.get(&item.ident.ident).expect(
        "all exec item idents, including this one, should have already been inserted into the scope",
    );

    match item.kind {
        AstExecKind::Const { ty, val } => {
            let ty = ty.map(|ty| resolve_ty(adt_scope, ty)).transpose()?;
            let val = resolve_expr(adt_scope, var_scope, hir, val)?;

            hir.fulfill_var(
                id,
                VarInfo {
                    mutable: false,
                    ty: ty.clone(),
                },
            );

            Ok(HirExecItem {
                ident: id,
                kind: HirExecKind::Const { ty, val },
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

            let mut var_scope = var_scope.clone();

            let params: Vec<_> = params
                .into_iter()
                .map(|p| {
                    let ty = resolve_ty(adt_scope, p.ty)?;
                    let id = resolve_pat(&mut var_scope, hir, p.pat, p.mutable, Some(ty.clone()));
                    Ok(Param {
                        mutable: p.mutable,
                        id,
                        ty,
                    })
                })
                .try_collect()?;
            let ret_ty = resolve_ty(adt_scope, ret_ty)?;
            let body = resolve_expr(adt_scope, &var_scope, hir, body)?;

            let ty = HirTy::Fn(
                params
                    .iter()
                    .map(|p| ParamTy {
                        mutable: p.mutable,
                        ty: p.ty.clone(),
                    })
                    .collect(),
                Return {
                    mutable: ret_mut,
                    ty: Box::new(ret_ty.clone()),
                },
            );

            hir.fulfill_var(
                id,
                VarInfo {
                    mutable: false,
                    ty: Some(ty),
                },
            );

            Ok(HirExecItem {
                ident: id,
                kind: HirExecKind::Fn {
                    params,
                    ret_ty,
                    body,
                },
            })
        }
    }
}

fn resolve_expr(
    adt_scope: &Scope<AdtId>,
    var_scope: &Scope<VarId>,
    hir: &mut Hir,
    expr: AstExpr,
) -> Result<ExprId> {
    let new_expr = match expr.kind {
        ExprKind::Ident(ident) => match var_scope.get(&ident) {
            Some(&id) => HirExpr::Ident(id),
            None => return Err(ErrorKind::UnboundVariable.span(expr.span)),
        },
        ExprKind::Lit(lit) => HirExpr::Lit(convert_lit(lit)),
        ExprKind::Array(exprs) => HirExpr::Array(resolve_exprs(adt_scope, var_scope, hir, exprs)?),
        ExprKind::Tuple(exprs) => HirExpr::Tuple(resolve_exprs(adt_scope, var_scope, hir, exprs)?),
        ExprKind::Infix { op, lhs, rhs } => HirExpr::Infix {
            op: convert_infix_op(op),
            lhs: resolve_expr(adt_scope, var_scope, hir, *lhs)?,
            rhs: resolve_expr(adt_scope, var_scope, hir, *rhs)?,
        },
        ExprKind::Prefix { op, expr } => HirExpr::Prefix {
            op: convert_prefix_op(op),
            expr: resolve_expr(adt_scope, var_scope, hir, *expr)?,
        },
        ExprKind::Field { base, field } => HirExpr::Field {
            base: resolve_expr(adt_scope, var_scope, hir, *base)?,
            field,
        },
        ExprKind::Index { arr, idx } => HirExpr::Index {
            arr: resolve_expr(adt_scope, var_scope, hir, *arr)?,
            idx: resolve_expr(adt_scope, var_scope, hir, *idx)?,
        },
        ExprKind::Call { func, args } => {
            let func = resolve_expr(adt_scope, var_scope, hir, *func)?;
            let args = args
                .into_iter()
                .map(|arg| {
                    Ok(HirArg {
                        mutable: arg.mutable,
                        val: resolve_expr(adt_scope, var_scope, hir, arg.val)?,
                    })
                })
                .try_collect()?;
            HirExpr::Call { func, args }
        }
        ExprKind::Lambda { params, body } => {
            let mut var_scope = var_scope.clone();

            // Rebind all mutable captures as immutable within the lambda body
            for capture in collect_captures(&body) {
                // Unbound variables will be caught in a few lines anyway, so doesn't matter if don't rebind them as immutable
                // The only partially-resolved variables will be the top-level items, which are already always immutable
                if let Some(&id) = var_scope.get(&capture)
                    && let Some(info) = hir.try_var_info(id)
                    && info.mutable
                {
                    let ident = hir.var_ident(id);
                    let id = hir.add_var(
                        ident,
                        VarInfo {
                            mutable: false,
                            ..info.clone()
                        },
                    );
                    var_scope.insert(ident.ident, id);
                }
            }

            let params = params
                .into_iter()
                .map(|param| resolve_binding(adt_scope, &mut var_scope, hir, param))
                .try_collect()?;
            let body = resolve_expr(adt_scope, &var_scope, hir, *body)?;

            HirExpr::Lambda { params, body }
        }
        ExprKind::If { cond, th, el } => HirExpr::If {
            cond: resolve_expr(adt_scope, var_scope, hir, *cond)?,
            th: resolve_block_expr(adt_scope, var_scope, hir, th)?,
            el: el
                .map(|el| resolve_block_expr(adt_scope, var_scope, hir, el))
                .transpose()?,
        },
        ExprKind::Match { .. } => todo!("Match (Pattern Matching)"),
        ExprKind::For { pat, iter, body } => {
            let iter = resolve_expr(adt_scope, var_scope, hir, *iter)?;
            let mut var_scope = var_scope.clone();
            let id = resolve_pat(&mut var_scope, hir, pat, false, None);
            let body = resolve_block_expr(adt_scope, &var_scope, hir, body)?;
            HirExpr::For { id, iter, body }
        }
        ExprKind::Loop(body) => HirExpr::Loop(resolve_block_expr(adt_scope, var_scope, hir, body)?),
        ExprKind::Break => HirExpr::Break,
        ExprKind::Continue => HirExpr::Continue,
        ExprKind::Return(expr) => HirExpr::Return(resolve_expr(adt_scope, var_scope, hir, *expr)?),
        ExprKind::Block(stmts) => {
            HirExpr::Block(resolve_block_expr(adt_scope, var_scope, hir, stmts)?)
        }
    };

    Ok(hir.add_expr(new_expr, expr.span))
}

fn collect_captures(expr: &AstExpr) -> HashSet<Ident> {
    let mut captures = HashSet::default();
    collect_captures_inner(&mut captures, expr);
    captures
}

fn collect_captures_inner(captures: &mut HashSet<Ident>, expr: &AstExpr) {
    match &expr.kind {
        ExprKind::Ident(ident) => {
            captures.insert(*ident);
        }
        ExprKind::Lit(_) | ExprKind::Break | ExprKind::Continue => {}
        ExprKind::Array(exprs) | ExprKind::Tuple(exprs) => {
            for e in exprs {
                collect_captures_inner(captures, e);
            }
        }
        ExprKind::Lambda { body: e, .. }
        | ExprKind::Field { base: e, .. }
        | ExprKind::Prefix { expr: e, .. }
        | ExprKind::Return(e) => collect_captures_inner(captures, e),
        ExprKind::Infix {
            lhs: e1, rhs: e2, ..
        }
        | ExprKind::Index { arr: e1, idx: e2 } => {
            collect_captures_inner(captures, e1);
            collect_captures_inner(captures, e2);
        }
        ExprKind::Call { func, args } => {
            collect_captures_inner(captures, func);
            for a in args {
                collect_captures_inner(captures, &a.val);
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            collect_captures_inner(captures, scrutinee);
            for a in arms {
                collect_captures_inner(captures, &a.body);
            }
        }
        ExprKind::If { cond, th, el } => {
            collect_captures_inner(captures, cond);
            collect_block_captures(captures, th);
            el.as_ref()
                .inspect(|el| collect_block_captures(captures, el));
        }
        ExprKind::For { iter, body, .. } => {
            collect_captures_inner(captures, iter);
            collect_block_captures(captures, body);
        }
        ExprKind::Loop(stmts) | ExprKind::Block(stmts) => collect_block_captures(captures, stmts),
    }
}

fn collect_block_captures(captures: &mut HashSet<Ident>, block: &AstBlockExpr) {
    for s in &block.stmts {
        match s {
            AstStmt::Decl { val, .. } => collect_captures_inner(captures, val),
            AstStmt::Expr(expr) => collect_captures_inner(captures, expr),
        }
    }
}

fn resolve_exprs(
    adt_scope: &Scope<AdtId>,
    var_scope: &Scope<VarId>,
    hir: &mut Hir,
    exprs: Vec<AstExpr>,
) -> Result<Vec<ExprId>> {
    exprs
        .into_iter()
        .map(|expr| resolve_expr(adt_scope, var_scope, hir, expr))
        .collect()
}

fn resolve_block_expr(
    adt_scope: &Scope<AdtId>,
    var_scope: &Scope<VarId>,
    hir: &mut Hir,
    block_expr: AstBlockExpr,
) -> Result<HirBlockExpr> {
    let mut var_scope = var_scope.clone();
    let stmts = block_expr
        .stmts
        .into_iter()
        .map(|s| resolve_stmt(adt_scope, &mut var_scope, hir, s))
        .try_collect()?;
    Ok(HirBlockExpr {
        stmts,
        span: block_expr.span,
    })
}

fn resolve_stmt(
    adt_scope: &Scope<AdtId>,
    var_scope: &mut Scope<VarId>,
    hir: &mut Hir,
    stmt: AstStmt,
) -> Result<HirStmt> {
    match stmt {
        AstStmt::Decl { binding, val, span } => {
            // val must be resolved before the binding, to ensure the declared variable isn't in scope within it's own declaration
            let val = resolve_expr(adt_scope, var_scope, hir, val)?;
            let id = resolve_binding(adt_scope, var_scope, hir, binding)?;

            Ok(HirStmt::Decl { id, val, span })
        }
        AstStmt::Expr(expr) => resolve_expr(adt_scope, var_scope, hir, expr).map(HirStmt::Expr),
    }
}

fn resolve_binding(
    adt_scope: &Scope<AdtId>,
    var_scope: &mut Scope<VarId>,
    hir: &mut Hir,
    binding: AstBinding,
) -> Result<VarId> {
    let ty = binding.ty.map(|ty| resolve_ty(adt_scope, ty)).transpose()?;
    Ok(resolve_pat(
        var_scope,
        hir,
        binding.pat,
        binding.mutable,
        ty,
    ))
}

fn resolve_ty(adt_scope: &Scope<AdtId>, ty: AstTy) -> Result<HirTy> {
    match ty.kind {
        AstTyKind::Int => Ok(HirTy::Int),
        AstTyKind::UInt => Ok(HirTy::UInt),
        AstTyKind::Byte => Ok(HirTy::Byte),
        AstTyKind::Float => Ok(HirTy::Float),
        AstTyKind::Char => Ok(HirTy::Char),
        AstTyKind::Bool => Ok(HirTy::Bool),
        AstTyKind::Tuple(tys) => Ok(HirTy::Tuple(resolve_tys(adt_scope, tys)?)),
        AstTyKind::Fn(params, ret) => {
            let params = params
                .into_iter()
                .map(|param| {
                    Ok(ParamTy {
                        mutable: param.mutable,
                        ty: resolve_ty(adt_scope, param.ty)?,
                    })
                })
                .try_collect()?;
            let ret = Return {
                mutable: ret.mutable,
                ty: Box::new(resolve_ty(adt_scope, *ret.ty)?),
            };
            Ok(HirTy::Fn(params, ret))
        }
        AstTyKind::Adt(ident, mut args) => {
            if ident == "Array" {
                match args.len() {
                    1 => resolve_ty(adt_scope, args.swap_remove(0))
                        .map(Box::new)
                        .map(HirTy::Array),
                    len => Err(ErrorKind::GenericCount(1, len).span(ty.span)),
                }
            } else {
                if !args.is_empty() {
                    todo!("Generics")
                }

                match adt_scope.get(&ident).copied() {
                    Some(id) => Ok(HirTy::Adt(id)),
                    None => Err(ErrorKind::UnknownType.span(ty.span)),
                }
            }
        }
    }
}

fn resolve_tys(adt_scope: &Scope<AdtId>, tys: Vec<AstTy>) -> Result<Vec<HirTy>> {
    tys.into_iter()
        .map(|ty| resolve_ty(adt_scope, ty))
        .try_collect()
}

fn resolve_pat(
    var_scope: &mut Scope<VarId>,
    hir: &mut Hir,
    pat: AstPat,
    mutable: bool,
    ty: Option<HirTy>,
) -> VarId {
    match pat.kind {
        AstPatKind::Ident(ident) => {
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

macro_rules! convert_op {
    ($op:ident, $enum_name:ident, $($variant:ident),*) => {
        match $op {
            $(ast::exprs::$enum_name::$variant => hir::exprs::$enum_name::$variant),*
        }
    };
}

const fn convert_prefix_op(op: AstPrefixOp) -> HirPrefixOp {
    convert_op!(op, PrefixOp, Not, Neg)
}

const fn convert_infix_op(op: AstInfixOp) -> HirInfixOp {
    convert_op!(
        op, InfixOp, Assign, Add, AddF, Sub, SubF, Mul, MulF, Div, DivF, Exp, Rem, RemF, And, Or,
        Xor, Eqq, Neq, Gt, Lt, Geq, Leq
    )
}
