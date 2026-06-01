use foldhash::HashSet;
use itertools::Itertools as _;
use smallvec::SmallVec;

use ast::exprs::{
    BlockExpr as AstBlockExpr, Expr as AstExpr, ExprKind, InfixOp as AstInfixOp,
    PrefixOp as AstPrefixOp, Stmt as AstStmt,
};
use errors::{ErrorHandler, Result, TryCollectEager as _};
use hir::{
    Hir, VarId,
    exprs::{
        Arg, BlockExpr as HirBlockExpr, Expr as HirExpr, ExprId, InfixOp as HirInfixOp, LitExpr,
        PrefixOp as HirPrefixOp, Stmt as HirStmt,
    },
    items::AdtId,
};
use ident::Ident;

use crate::{ErrorKind, Scope};

#[allow(
    clippy::too_many_lines,
    reason = "Any given arm is readable on it's own"
)]
pub fn resolve_expr(
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
                return Err(handler.err(ErrorKind::UnboundVariable.span(expr.span)));
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
            let rhs = resolve_expr(adt_scope, var_scope, hir, handler, *rhs);
            let lhs = resolve_expr(adt_scope, var_scope, hir, handler, *lhs)?;
            let op = convert_infix_op(op);
            if op == HirInfixOp::Assign {
                check_is_place(hir, handler, lhs)?;
            }
            HirExpr::Infix { op, lhs, rhs: rhs? }
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
            let args: Vec<Arg> = args
                .into_iter()
                .map(|arg| {
                    let val = resolve_expr(adt_scope, var_scope, hir, handler, arg.val)?;
                    if arg.mutable {
                        check_is_place(hir, handler, val)?;
                    }
                    Ok(Arg {
                        val,
                        mutable: arg.mutable,
                        span: arg.span,
                    })
                })
                .try_collect_eager()?;

            // Verify uniqueness of mutable arguments
            // TODO optimise???
            args.iter()
                .permutations(2)
                .map(|p| (p[0], p[1]))
                .filter(|(a, b)| a.mutable || b.mutable)
                .try_for_each(|(a, b)| {
                    if overlaps(hir, a.val, b.val) {
                        Err(handler.err(ErrorKind::OverlappingPlace(b.span).span(a.span)))
                    } else {
                        Ok(())
                    }
                })?;

            HirExpr::Call { func: func?, args }
        }
        ExprKind::Lambda { params, body } => {
            let mut var_scope = Scope::clone(var_scope);

            // Rebind all mutable captures as immutable within the lambda body
            let mut captures = HashSet::default();
            collect_captures(&mut captures, &body);
            // Remove unbound variables. This filters out the parameters (which are bound a few lines later)
            captures.retain(|ident| var_scope.contains_key(ident));
            for capture in &captures {
                let info = hir.var_info(var_scope[capture]);
                if info.mutable {
                    let id = hir.add_var(info.ident, false, info.span);
                    var_scope.insert(*capture, id);
                }
            }

            let params = params
                .into_iter()
                .map(|param| crate::resolve_binding(adt_scope, &mut var_scope, hir, handler, param))
                .try_collect_eager();
            let body = resolve_expr(adt_scope, &var_scope, hir, handler, *body);

            HirExpr::Lambda {
                params: params?,
                body: body?,
                captures: captures
                    .into_iter()
                    .map(|ident| var_scope[&ident])
                    .collect(),
            }
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
) -> Result<SmallVec<[ExprId; 3]>> {
    exprs
        .into_iter()
        .map(|expr| resolve_expr(adt_scope, var_scope, hir, handler, expr))
        .try_collect_eager()
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
        .map(|stmt| match stmt {
            AstStmt::Decl { binding, val, span } => {
                // val must be resolved before the binding, to ensure the declared variable isn't in scope within it's own declaration
                let val = resolve_expr(adt_scope, &var_scope, hir, handler, val);
                let id = crate::resolve_binding(adt_scope, &mut var_scope, hir, handler, binding);
                Ok(HirStmt::Decl {
                    id: id?,
                    val: val?,
                    span,
                })
            }
            AstStmt::Expr(expr) => {
                resolve_expr(adt_scope, &var_scope, hir, handler, expr).map(HirStmt::Expr)
            }
        })
        .try_collect_eager()?;
    Ok(HirBlockExpr {
        stmts,
        span: block_expr.span,
    })
}

fn check_is_place(hir: &Hir, handler: &mut ErrorHandler, place: ExprId) -> Result<()> {
    match hir.expr_info(place) {
        HirExpr::Ident(id) => {
            if hir.var_info(*id).mutable {
                Ok(())
            } else {
                Err(handler.err(ErrorKind::Mutation.span(hir.expr_span(place))))
            }
        }
        HirExpr::Field { base, .. } | HirExpr::Index { arr: base, .. } => {
            check_is_place(hir, handler, *base)
        }
        HirExpr::Call { .. } => todo!("Projections"),
        _ => Err(handler.err(ErrorKind::NotPlaceExpr.span(hir.expr_span(place)))),
    }
}

fn overlaps(hir: &Hir, a: ExprId, b: ExprId) -> bool {
    match (hir.expr_info(a), hir.expr_info(b)) {
        (HirExpr::Ident(a), HirExpr::Ident(b)) => a == b,
        (HirExpr::Ident(_), HirExpr::Index { arr, .. }) => overlaps(hir, a, *arr),
        (HirExpr::Ident(_), HirExpr::Field { base, .. }) => overlaps(hir, a, *base),
        (
            HirExpr::Index {
                arr: arr_a,
                idx: idx_a,
            },
            HirExpr::Index {
                arr: arr_b,
                idx: idx_b,
            },
        ) => {
            if let HirExpr::Lit(LitExpr::Int(idx_a)) = hir.expr_info(*idx_a)
                && let HirExpr::Lit(LitExpr::Int(idx_b)) = hir.expr_info(*idx_b)
            {
                idx_a == idx_b
            } else {
                overlaps(hir, *arr_a, *arr_b)
            }
        }
        (
            HirExpr::Field {
                base: base_a,
                field: field_a,
            },
            HirExpr::Field {
                base: base_b,
                field: field_b,
            },
        ) => (field_a.ident == field_b.ident) && overlaps(hir, *base_a, *base_b),
        (HirExpr::Index { arr, .. }, HirExpr::Field { base, .. }) => {
            overlaps(hir, *arr, b) || overlaps(hir, a, *base)
        }
        _ => false,
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
