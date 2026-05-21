use foldhash::HashSet;
use itertools::Itertools;

use ast::exprs::{
    BlockExpr as AstBlockExpr, Expr as AstExpr, ExprKind, InfixOp as AstInfixOp,
    PrefixOp as AstPrefixOp, Stmt as AstStmt,
};
use errors::ErrorHandler;
use hir::{
    Hir, VarId, VarInfo,
    exprs::{
        Arg, BlockExpr as HirBlockExpr, Expr as HirExpr, ExprId, InfixOp as HirInfixOp,
        PrefixOp as HirPrefixOp, Stmt as HirStmt,
    },
    items::AdtId,
};
use ident::Ident;

use crate::{ErrorKind, Result, Scope};

pub(super) fn resolve_expr(
    adt_scope: &Scope<AdtId>,
    var_scope: &Scope<VarId>,
    hir: &mut Hir,
    handler: &mut ErrorHandler,
    expr: AstExpr,
) -> Result<ExprId> {
    let new_expr = match expr.kind {
        ExprKind::Ident(ident) => match var_scope.get(&ident) {
            Some(&id) => HirExpr::Ident(id),
            None => {
                handler.err(ErrorKind::UnboundVariable.span(expr.span));
                return Err(());
            }
        },
        ExprKind::Lit(lit) => HirExpr::Lit(crate::convert_lit(lit)),
        ExprKind::Array(exprs) => {
            HirExpr::Array(resolve_exprs(adt_scope, var_scope, hir, handler, exprs)?)
        }
        ExprKind::Tuple(exprs) => {
            HirExpr::Tuple(resolve_exprs(adt_scope, var_scope, hir, handler, exprs)?)
        }
        ExprKind::Infix { op, lhs, rhs } => {
            let lhs = resolve_expr(adt_scope, var_scope, hir, handler, *lhs);
            let rhs = resolve_expr(adt_scope, var_scope, hir, handler, *rhs);
            HirExpr::Infix {
                op: convert_infix_op(op),
                lhs: lhs?,
                rhs: rhs?,
            }
        }
        ExprKind::Prefix { op, expr } => HirExpr::Prefix {
            op: convert_prefix_op(op),
            expr: resolve_expr(adt_scope, var_scope, hir, handler, *expr)?,
        },
        ExprKind::Field { base, field } => HirExpr::Field {
            base: resolve_expr(adt_scope, var_scope, hir, handler, *base)?,
            field,
        },
        ExprKind::Index { arr, idx } => {
            let arr = resolve_expr(adt_scope, var_scope, hir, handler, *arr);
            let idx = resolve_expr(adt_scope, var_scope, hir, handler, *idx);
            HirExpr::Index {
                arr: arr?,
                idx: idx?,
            }
        }
        ExprKind::Call { func, args } => {
            let func = resolve_expr(adt_scope, var_scope, hir, handler, *func);
            let args = args
                .into_iter()
                .map(|arg| {
                    Ok(Arg {
                        mutable: arg.mutable,
                        val: resolve_expr(adt_scope, var_scope, hir, handler, arg.val)?,
                    })
                })
                .try_collect();
            HirExpr::Call {
                func: func?,
                args: args?,
            }
        }
        ExprKind::Lambda { params, body } => {
            let mut var_scope = Scope::clone(var_scope);

            let mut captures = HashSet::default();
            collect_captures(&mut captures, &body);
            // Rebind all mutable captures as immutable within the lambda body
            for capture in captures {
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
                    var_scope.insert(capture, id);
                }
            }

            let params = params
                .into_iter()
                .map(|param| crate::resolve_binding(adt_scope, &mut var_scope, hir, handler, param))
                .try_collect()?;
            let body = resolve_expr(adt_scope, &var_scope, hir, handler, *body)?;

            HirExpr::Lambda { params, body }
        }
        ExprKind::If { cond, th, el } => {
            let cond = resolve_expr(adt_scope, var_scope, hir, handler, *cond);
            let th = resolve_block_expr(adt_scope, var_scope, hir, handler, th);
            let el = el
                .map(|el| resolve_block_expr(adt_scope, var_scope, hir, handler, el))
                .transpose();
            HirExpr::If {
                cond: cond?,
                th: th?,
                el: el?,
            }
        }
        ExprKind::Match { .. } => todo!("Pattern Matching"),
        ExprKind::For { pat, iter, body } => {
            let iter = resolve_expr(adt_scope, var_scope, hir, handler, *iter);
            let mut var_scope = Scope::clone(var_scope);
            let id = crate::resolve_pat(&mut var_scope, hir, pat, false, None);
            let body = resolve_block_expr(adt_scope, &var_scope, hir, handler, body);
            HirExpr::For {
                id,
                iter: iter?,
                body: body?,
            }
        }
        ExprKind::Loop(body) => HirExpr::Loop(resolve_block_expr(
            adt_scope, var_scope, hir, handler, body,
        )?),
        ExprKind::Break => HirExpr::Break,
        ExprKind::Continue => HirExpr::Continue,
        ExprKind::Return(expr) => {
            HirExpr::Return(resolve_expr(adt_scope, var_scope, hir, handler, *expr)?)
        }
        ExprKind::Block(stmts) => HirExpr::Block(resolve_block_expr(
            adt_scope, var_scope, hir, handler, stmts,
        )?),
        ExprKind::Print(expr) => {
            HirExpr::Print(resolve_expr(adt_scope, var_scope, hir, handler, *expr)?)
        }
    };

    Ok(hir.add_expr(new_expr, expr.span))
}

fn resolve_exprs(
    adt_scope: &Scope<AdtId>,
    var_scope: &Scope<VarId>,
    hir: &mut Hir,
    handler: &mut ErrorHandler,
    exprs: Vec<AstExpr>,
) -> Result<Vec<ExprId>> {
    exprs
        .into_iter()
        .map(|expr| resolve_expr(adt_scope, var_scope, hir, handler, expr))
        .try_collect()
}

fn resolve_block_expr(
    adt_scope: &Scope<AdtId>,
    var_scope: &Scope<VarId>,
    hir: &mut Hir,
    handler: &mut ErrorHandler,
    block_expr: AstBlockExpr,
) -> Result<HirBlockExpr> {
    let mut var_scope = Scope::clone(var_scope);
    let stmts = block_expr
        .stmts
        .into_iter()
        .map(|s| resolve_stmt(adt_scope, &mut var_scope, hir, handler, s))
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
    handler: &mut ErrorHandler,
    stmt: AstStmt,
) -> Result<HirStmt> {
    match stmt {
        AstStmt::Decl { binding, val, span } => {
            // val must be resolved before the binding, to ensure the declared variable isn't in scope within it's own declaration
            let val = resolve_expr(adt_scope, var_scope, hir, handler, val);
            let id = crate::resolve_binding(adt_scope, var_scope, hir, handler, binding);

            Ok(HirStmt::Decl {
                id: id?,
                val: val?,
                span,
            })
        }
        AstStmt::Expr(expr) => {
            resolve_expr(adt_scope, var_scope, hir, handler, expr).map(HirStmt::Expr)
        }
    }
}

fn collect_captures(captures: &mut HashSet<Ident>, expr: &AstExpr) {
    match &expr.kind {
        ExprKind::Ident(ident) => {
            captures.insert(*ident);
        }
        ExprKind::Lit(_) | ExprKind::Break | ExprKind::Continue => {}
        ExprKind::Array(exprs) | ExprKind::Tuple(exprs) => {
            for e in exprs {
                collect_captures(captures, e);
            }
        }
        ExprKind::Lambda { body: e, .. }
        | ExprKind::Field { base: e, .. }
        | ExprKind::Prefix { expr: e, .. }
        | ExprKind::Print(e)
        | ExprKind::Return(e) => collect_captures(captures, e),
        ExprKind::Infix {
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
        ExprKind::Match { scrutinee, arms } => {
            collect_captures(captures, scrutinee);
            for a in arms {
                collect_captures(captures, &a.body);
            }
        }
        ExprKind::If { cond, th, el } => {
            collect_captures(captures, cond);
            collect_block_captures(captures, th);
            el.as_ref()
                .inspect(|el| collect_block_captures(captures, el));
        }
        ExprKind::For { iter, body, .. } => {
            collect_captures(captures, iter);
            collect_block_captures(captures, body);
        }
        ExprKind::Loop(stmts) | ExprKind::Block(stmts) => collect_block_captures(captures, stmts),
    }
}

fn collect_block_captures(captures: &mut HashSet<Ident>, block: &AstBlockExpr) {
    for s in &block.stmts {
        match s {
            AstStmt::Decl { val, .. } => collect_captures(captures, val),
            AstStmt::Expr(expr) => collect_captures(captures, expr),
        }
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
        op, InfixOp, Assign, Add, AddF, Sub, SubF, Mul, MulF, Div, DivF, Exp, And, Or, Xor, Eqq,
        Neq, Gt, Lt, Geq, Leq
    )
}
