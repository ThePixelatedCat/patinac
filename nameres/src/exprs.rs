use ast::{
    Path,
    exprs::{Arg, Binding, Expr, ExprKind, MatchArm, Stmt},
};

use ident::Ident;
use smallvec::smallvec;

use crate::{
    Scope, bind_pat,
    error::{ErrorKind, Result},
    table::{AdtId, NameTable, VarId},
    types::resolve_ty,
};

pub fn resolve_expr(
    table: &mut NameTable,
    adt_scope: &Scope<AdtId>,
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
        ExprKind::Array(exprs) => {
            ExprKind::Array(resolve_exprs(table, adt_scope, var_scope, exprs)?)
        }
        ExprKind::Tuple(exprs) => {
            ExprKind::Tuple(resolve_exprs(table, adt_scope, var_scope, exprs)?)
        }
        ExprKind::InfixExpr { op, lhs, rhs } => ExprKind::InfixExpr {
            op,
            lhs: Box::new(resolve_expr(table, adt_scope, var_scope, *lhs)?),
            rhs: Box::new(resolve_expr(table, adt_scope, var_scope, *rhs)?),
        },
        ExprKind::UnaryExpr { op, expr } => ExprKind::UnaryExpr {
            op,
            expr: Box::new(resolve_expr(table, adt_scope, var_scope, *expr)?),
        },
        ExprKind::FieldExpr { base, field } => ExprKind::FieldExpr {
            base: Box::new(resolve_expr(table, adt_scope, var_scope, *base)?),
            field,
        },
        ExprKind::IndexExpr { arr, idx } => ExprKind::IndexExpr {
            arr: Box::new(resolve_expr(table, adt_scope, var_scope, *arr)?),
            idx: Box::new(resolve_expr(table, adt_scope, var_scope, *idx)?),
        },
        ExprKind::CallExpr { func, args } => {
            let func = Box::new(resolve_expr(table, adt_scope, var_scope, *func)?);
            let args = args
                .into_iter()
                .map(|arg| {
                    Ok(Arg {
                        mutable: arg.mutable,
                        val: resolve_expr(table, adt_scope, var_scope, arg.val)?,
                    })
                })
                .collect::<Result<_>>()?;
            ExprKind::CallExpr { func, args }
        }
        ExprKind::LambdaExpr {
            params,
            return_ty,
            body,
        } => {
            let mut var_scope = var_scope.clone();

            let params = params
                .into_iter()
                .map(|param| resolve_binding(table, adt_scope, param))
                .collect::<Result<Vec<_>>>()?;
            for p in &params {
                bind_pat(
                    table,
                    adt_scope,
                    &mut var_scope,
                    p.pat.clone(),
                    p.mutable,
                    p.ty.clone(),
                );
            }

            let return_ty = return_ty
                .map(|ty| resolve_ty(table, adt_scope, ty))
                .transpose()?;

            let body = Box::new(resolve_expr(table, adt_scope, &var_scope, *body)?);

            ExprKind::LambdaExpr {
                params,
                return_ty,
                body,
            }
        }
        ExprKind::If { cond, th, el } => ExprKind::If {
            cond: Box::new(resolve_expr(table, adt_scope, var_scope, *cond)?),
            th: Box::new(resolve_expr(table, adt_scope, var_scope, *th)?),
            el: el
                .map(|el| resolve_expr(table, adt_scope, var_scope, *el).map(Box::new))
                .transpose()?,
        },
        ExprKind::Match { scrutinee, arms } => {
            let scrutinee = Box::new(resolve_expr(table, adt_scope, var_scope, *scrutinee)?);
            let arms = arms
                .into_iter()
                .map(|arm| {
                    let mut var_scope = var_scope.clone();
                    bind_pat(
                        table,
                        adt_scope,
                        &mut var_scope,
                        arm.pat.clone(),
                        false,
                        None,
                    );

                    let body = resolve_expr(table, adt_scope, &var_scope, arm.body)?;

                    Ok(MatchArm {
                        pat: arm.pat,
                        body,
                        span: arm.span,
                    })
                })
                .collect::<Result<_>>()?;
            ExprKind::Match { scrutinee, arms }
        }
        ExprKind::For { pat, iter, body } => {
            let mut var_scope = var_scope.clone();
            bind_pat(table, adt_scope, &mut var_scope, pat.clone(), false, None);

            ExprKind::For {
                pat,
                iter: Box::new(resolve_expr(table, adt_scope, &var_scope, *iter)?),
                body: Box::new(resolve_expr(table, adt_scope, &var_scope, *body)?),
            }
        }
        ExprKind::Loop(body) => {
            ExprKind::Loop(Box::new(resolve_expr(table, adt_scope, var_scope, *body)?))
        }
        ExprKind::Break => ExprKind::Break,
        ExprKind::Continue => ExprKind::Continue,
        ExprKind::Return(expr) => {
            ExprKind::Return(Box::new(resolve_expr(table, adt_scope, var_scope, *expr)?))
        }
        ExprKind::Block(stmts) => {
            let mut var_scope = var_scope.clone();
            ExprKind::Block(
                stmts
                    .into_iter()
                    .map(|stmt| resolve_stmt(table, adt_scope, &mut var_scope, stmt))
                    .collect::<Result<_>>()?,
            )
        }
    };

    Ok(kind.span(expr.span))
}

fn resolve_exprs(
    table: &mut NameTable,
    adt_scope: &Scope<AdtId>,
    var_scope: &Scope<VarId>,
    exprs: Vec<Expr<(), Ident, Ident>>,
) -> Result<Vec<Expr<(), AdtId, VarId>>> {
    exprs
        .into_iter()
        .map(|expr| resolve_expr(table, adt_scope, var_scope, expr))
        .collect()
}

fn resolve_stmt(
    table: &mut NameTable,
    adt_scope: &Scope<AdtId>,
    var_scope: &mut Scope<VarId>,
    stmt: Stmt<(), Ident, Ident>,
) -> Result<Stmt<(), AdtId, VarId>> {
    match stmt {
        Stmt::Decl { binding, val, span } => {
            let binding = resolve_binding(table, adt_scope, binding)?;
            let val = Box::new(resolve_expr(table, adt_scope, var_scope, *val)?);

            bind_pat(
                table,
                adt_scope,
                var_scope,
                binding.pat.clone(),
                binding.mutable,
                binding.ty.clone(),
            );

            Ok(Stmt::Decl { binding, val, span })
        }
        Stmt::Expr(expr) => resolve_expr(table, adt_scope, var_scope, expr).map(Stmt::Expr),
    }
}

fn resolve_binding(
    table: &mut NameTable,
    adt_scope: &Scope<AdtId>,
    binding: Binding<Ident>,
) -> Result<Binding<AdtId>> {
    Ok(Binding {
        mutable: binding.mutable,
        pat: binding.pat,
        ty: binding
            .ty
            .map(|ty| resolve_ty(table, adt_scope, ty))
            .transpose()?,
    })
}
