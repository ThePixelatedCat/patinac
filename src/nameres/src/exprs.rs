use foldhash::HashSet;
use itertools::Itertools as _;
use package::ModuleId;

use ast::ExprKind;
use errors::{ErrorHandler, Result, SpanError as _, TryCollectEager as _};
use hir::{Arg, ExprId, Hir, LitExpr, VarId};

use crate::{ErrorKind, Scope};

#[allow(
    clippy::too_many_lines,
    reason = "Any given arm is readable on it's own"
)]
pub fn resolve_expr(
    scope: &Scope,
    hir: &mut Hir,
    handler: &mut ErrorHandler,
    expr: ast::Expr,
) -> Result<ExprId> {
    let new_expr = match expr.kind {
        ExprKind::Var(path) => match scope.resolve_var(path) {
            Ok(id) => hir::Expr::Var(id),
            Err(error) => {
                return Err(handler.err(error.span(expr.span, scope.module())));
            }
        },
        ExprKind::Lit(lit) => {
            let lit = match lit {
                ast::LitExpr::Int(i) => hir::LitExpr::Int(i),
                ast::LitExpr::Float(f) => hir::LitExpr::Float(f),
                ast::LitExpr::String(s) => hir::LitExpr::String(s),
                ast::LitExpr::Bool(b) => hir::LitExpr::Bool(b),
            };
            hir::Expr::Lit(lit)
        }
        ExprKind::Array(exprs) => hir::Expr::Array(resolve_exprs(scope, hir, handler, exprs)?),
        ExprKind::Tuple(exprs) => hir::Expr::Tuple(resolve_exprs(scope, hir, handler, exprs)?),
        ExprKind::Infix { op, lhs, rhs } => {
            let rhs = resolve_expr(scope, hir, handler, *rhs);
            let lhs = resolve_expr(scope, hir, handler, *lhs)?;
            match convert_infix_op(op) {
                Some(op) => hir::Expr::Infix { op, lhs, rhs: rhs? },
                None => {
                    check_is_place(hir, scope.module(), handler, lhs)?;
                    hir::Expr::Assign {
                        place: lhs,
                        value: rhs?,
                    }
                }
            }
        }
        ExprKind::Prefix { op, expr } => hir::Expr::Prefix {
            op: convert_prefix_op(op),
            expr: resolve_expr(scope, hir, handler, *expr)?,
        },
        ExprKind::Field { base, field } => hir::Expr::Field {
            base: resolve_expr(scope, hir, handler, *base)?,
            field,
        },
        ExprKind::Index { array, index } => {
            let array = resolve_expr(scope, hir, handler, *array);
            let index = resolve_expr(scope, hir, handler, *index);
            hir::Expr::Index {
                array: array?,
                index: index?,
            }
        }
        ExprKind::Call { func, args } => {
            let func = resolve_expr(scope, hir, handler, *func);
            let args: Vec<Arg> = args
                .into_iter()
                .map(|arg| {
                    let val = resolve_expr(scope, hir, handler, arg.value)?;
                    if arg.mutable {
                        check_is_place(hir, scope.module(), handler, val)?;
                    }
                    Ok(Arg {
                        value: val,
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
                    if overlaps(hir, a.value, b.value) {
                        Err(handler
                            .err(ErrorKind::OverlappingPlace(b.span).span(a.span, scope.module())))
                    } else {
                        Ok(())
                    }
                })?;

            hir::Expr::Call { func: func?, args }
        }
        ExprKind::Lambda { params, body } => {
            let mut scope = Scope::clone(scope);

            // Rebind all captures within the lambda body, making them all immutable.
            let mut captures = HashSet::default();
            collect_captures(&scope, hir, &mut captures, &body);
            let captures = captures
                .into_iter()
                .map(|capture| {
                    let info = hir.var_info(capture);
                    let rebinding = hir.add_var(hir::VarInfo {
                        mutable: false,
                        ..info
                    });
                    scope.add_var(info.ident.ident, rebinding);
                    (capture, rebinding)
                })
                .collect();

            let params = params
                .into_iter()
                .map(|param| crate::resolve_binding(&mut scope, hir, handler, param))
                .try_collect_eager();
            let body = resolve_expr(&scope, hir, handler, *body);

            hir::Expr::Lambda {
                params: params?,
                body: body?,
                captures,
            }
        }
        ExprKind::If { cond, th, el } => {
            let cond = resolve_expr(scope, hir, handler, *cond);
            let th = resolve_block_expr(scope, hir, handler, th);
            let el = el
                .map(|el| resolve_block_expr(scope, hir, handler, el))
                .transpose();
            hir::Expr::If {
                cond: cond?,
                th: th?,
                el: el?,
            }
        }
        ExprKind::Match { .. } => todo!("Pattern Matching"),
        ExprKind::For { pat, iter, body } => {
            let iter = resolve_expr(scope, hir, handler, *iter);
            let mut scope = Scope::clone(scope);
            let id = crate::resolve_pat(&mut scope, hir, pat, false, None);
            let body = resolve_block_expr(&scope, hir, handler, body);
            hir::Expr::For {
                id,
                iter: iter?,
                body: body?,
            }
        }
        ExprKind::Loop(body) => hir::Expr::Loop(resolve_block_expr(scope, hir, handler, body)?),
        ExprKind::Break => hir::Expr::Break,
        ExprKind::Continue => hir::Expr::Continue,
        ExprKind::Return(expr) => hir::Expr::Return(resolve_expr(scope, hir, handler, *expr)?),
        ExprKind::Block(stmts) => hir::Expr::Block(resolve_block_expr(scope, hir, handler, stmts)?),
        ExprKind::Print(expr) => hir::Expr::Print(resolve_expr(scope, hir, handler, *expr)?),
    };

    Ok(hir.add_expr(new_expr, expr.span))
}

fn resolve_exprs(
    scope: &Scope,
    hir: &mut Hir,
    handler: &mut ErrorHandler,
    exprs: Vec<ast::Expr>,
) -> Result<Vec<ExprId>> {
    exprs
        .into_iter()
        .map(|expr| resolve_expr(scope, hir, handler, expr))
        .try_collect_eager()
}

fn resolve_block_expr(
    scope: &Scope,
    hir: &mut Hir,
    handler: &mut ErrorHandler,
    block_expr: ast::BlockExpr,
) -> Result<hir::BlockExpr> {
    let mut scope = Scope::clone(scope);
    let stmts = block_expr
        .stmts
        .into_iter()
        .map(|stmt| match stmt {
            ast::Stmt::Decl {
                binding,
                value,
                span,
            } => {
                // val must be resolved before the binding, to ensure the declared variable isn't in scope within it's own declaration
                let value = resolve_expr(&scope, hir, handler, value);
                let var = crate::resolve_binding(&mut scope, hir, handler, binding);
                Ok(hir::Stmt::Decl {
                    var: var?,
                    value: value?,
                    span,
                })
            }
            ast::Stmt::Expr(expr) => resolve_expr(&scope, hir, handler, expr).map(hir::Stmt::Expr),
        })
        .try_collect_eager()?;
    Ok(hir::BlockExpr {
        stmts,
        span: block_expr.span,
    })
}

fn check_is_place(
    hir: &Hir,
    module: ModuleId,
    handler: &mut ErrorHandler,
    place: ExprId,
) -> Result<()> {
    match hir.expr(place) {
        hir::Expr::Var(id) => {
            if hir.var_info(*id).mutable {
                Ok(())
            } else {
                Err(handler.err(ErrorKind::Mutation.span(hir.expr_span(place), module)))
            }
        }
        hir::Expr::Field { base, .. } | hir::Expr::Index { array: base, .. } => {
            check_is_place(hir, module, handler, *base)
        }
        hir::Expr::Call { .. } => todo!("Projections"),
        _ => Err(handler.err(ErrorKind::NotPlaceExpr.span(hir.expr_span(place), module))),
    }
}

fn overlaps(hir: &Hir, a: ExprId, b: ExprId) -> bool {
    match (hir.expr(a), hir.expr(b)) {
        (hir::Expr::Var(a), hir::Expr::Var(b)) => a == b,
        (hir::Expr::Var(_), hir::Expr::Index { array: arr, .. }) => overlaps(hir, a, *arr),
        (hir::Expr::Var(_), hir::Expr::Field { base, .. }) => overlaps(hir, a, *base),
        (
            hir::Expr::Index {
                array: arr_a,
                index: idx_a,
            },
            hir::Expr::Index {
                array: arr_b,
                index: idx_b,
            },
        ) => {
            if let hir::Expr::Lit(LitExpr::Int(idx_a)) = hir.expr(*idx_a)
                && let hir::Expr::Lit(LitExpr::Int(idx_b)) = hir.expr(*idx_b)
            {
                idx_a == idx_b
            } else {
                overlaps(hir, *arr_a, *arr_b)
            }
        }
        (
            hir::Expr::Field {
                base: base_a,
                field: field_a,
            },
            hir::Expr::Field {
                base: base_b,
                field: field_b,
            },
        ) => (field_a.ident == field_b.ident) && overlaps(hir, *base_a, *base_b),
        (hir::Expr::Index { array: arr, .. }, hir::Expr::Field { base, .. }) => {
            overlaps(hir, *arr, b) || overlaps(hir, a, *base)
        }
        _ => false,
    }
}

fn collect_captures(scope: &Scope, hir: &Hir, captures: &mut HashSet<VarId>, expr: &ast::Expr) {
    match &expr.kind {
        ExprKind::Var(path) => {
            // Only add capture if it's bound.
            // Unbound variables are either parameters, which don't need capturing, or actually unbound, which will produce an error anyway.
            if let Some(id) = scope.get_var(path.start())
                && !hir.var_info(id).global
            {
                captures.insert(id);
            }
        }
        ExprKind::Lit(_) | ExprKind::Break | ExprKind::Continue => {}
        ExprKind::Array(exprs) | ExprKind::Tuple(exprs) => {
            for e in exprs {
                collect_captures(scope, hir, captures, e);
            }
        }
        ExprKind::Lambda { body: e, .. }
        | ExprKind::Field { base: e, .. }
        | ExprKind::Prefix { expr: e, .. }
        | ExprKind::Print(e)
        | ExprKind::Return(e) => collect_captures(scope, hir, captures, e),
        ExprKind::Infix {
            lhs: e1, rhs: e2, ..
        }
        | ExprKind::Index {
            array: e1,
            index: e2,
        } => {
            collect_captures(scope, hir, captures, e1);
            collect_captures(scope, hir, captures, e2);
        }
        ExprKind::Call { func, args } => {
            collect_captures(scope, hir, captures, func);
            for a in args {
                collect_captures(scope, hir, captures, &a.value);
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            collect_captures(scope, hir, captures, scrutinee);
            for a in arms {
                collect_captures(scope, hir, captures, &a.body);
            }
        }
        ExprKind::If { cond, th, el } => {
            collect_captures(scope, hir, captures, cond);
            collect_block_captures(scope, hir, captures, th);
            el.as_ref()
                .inspect(|el| collect_block_captures(scope, hir, captures, el));
        }
        ExprKind::For { iter, body, .. } => {
            collect_captures(scope, hir, captures, iter);
            collect_block_captures(scope, hir, captures, body);
        }
        ExprKind::Loop(stmts) | ExprKind::Block(stmts) => {
            collect_block_captures(scope, hir, captures, stmts);
        }
    }
}

fn collect_block_captures(
    scope: &Scope,
    hir: &Hir,
    captures: &mut HashSet<VarId>,
    block: &ast::BlockExpr,
) {
    for s in &block.stmts {
        match s {
            ast::Stmt::Decl { value, .. } => collect_captures(scope, hir, captures, value),
            ast::Stmt::Expr(expr) => collect_captures(scope, hir, captures, expr),
        }
    }
}

const fn convert_prefix_op(op: ast::PrefixOp) -> hir::PrefixOp {
    match op {
        ast::PrefixOp::Not => hir::PrefixOp::Not,
        ast::PrefixOp::Neg => hir::PrefixOp::Neg,
    }
}

/// Converts an ast infix operator to a hir infix operator, returning `None` if the ast operator was `Assign`.
const fn convert_infix_op(op: ast::InfixOp) -> Option<hir::InfixOp> {
    match op {
        ast::InfixOp::Assign => None,
        ast::InfixOp::Add => Some(hir::InfixOp::Add),
        ast::InfixOp::AddF => Some(hir::InfixOp::AddF),
        ast::InfixOp::Sub => Some(hir::InfixOp::Sub),
        ast::InfixOp::SubF => Some(hir::InfixOp::SubF),
        ast::InfixOp::Mul => Some(hir::InfixOp::Mul),
        ast::InfixOp::MulF => Some(hir::InfixOp::MulF),
        ast::InfixOp::Div => Some(hir::InfixOp::Div),
        ast::InfixOp::DivF => Some(hir::InfixOp::DivF),
        ast::InfixOp::Exp => Some(hir::InfixOp::Exp),
        ast::InfixOp::And => Some(hir::InfixOp::And),
        ast::InfixOp::Or => Some(hir::InfixOp::Or),
        ast::InfixOp::Eqq => Some(hir::InfixOp::Eqq),
        ast::InfixOp::Neq => Some(hir::InfixOp::Neq),
        ast::InfixOp::Gt => Some(hir::InfixOp::Gt),
        ast::InfixOp::Lt => Some(hir::InfixOp::Lt),
        ast::InfixOp::Geq => Some(hir::InfixOp::Geq),
        ast::InfixOp::Leq => Some(hir::InfixOp::Leq),
    }
}
