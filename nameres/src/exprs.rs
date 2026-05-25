use foldhash::HashSet;

use ast::exprs::{
    BlockExpr as AstBlockExpr, Expr as AstExpr, ExprKind, InfixOp as AstInfixOp,
    PrefixOp as AstPrefixOp, Stmt as AstStmt,
};
use errors::{ErrorHandler, Result, TryCollectEager};
use hir::{
    Hir, VarId,
    exprs::{
        Arg, BlockExpr as HirBlockExpr, Expr as HirExpr, ExprId, InfixOp as HirInfixOp, Place,
        PrefixOp as HirPrefixOp, Stmt as HirStmt,
    },
    items::AdtId,
};
use ident::Ident;
use smallvec::SmallVec;

use crate::{ErrorKind, Scope};

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
        ExprKind::Infix { op, lhs, rhs } => match op {
            AstInfixOp::Assign => {
                let lhs = resolve_place(adt_scope, var_scope, hir, handler, *lhs);
                let rhs = resolve_expr(adt_scope, var_scope, hir, handler, *rhs);
                HirExpr::Assign(lhs?, rhs?)
            }
            _ => {
                let lhs = resolve_expr(adt_scope, var_scope, hir, handler, *lhs);
                let rhs = resolve_expr(adt_scope, var_scope, hir, handler, *rhs);
                HirExpr::Infix {
                    op: convert_infix_op(op),
                    lhs: lhs?,
                    rhs: rhs?,
                }
            }
        },
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
            todo!("Verify uniqueness of mutable arguments");
            // Verify uniqueness of mutable arguments
            args.iter()
                .permutations(2)
                .map(|p| (p[0], p[1]))
                .filter(|(a, b)| a.mutable || b.mutable)
                .try_for_each(|(a, b)| self.check_places_unique(hir, a.val, b.val))?; // TODO optimise???
            let func = resolve_expr(adt_scope, var_scope, hir, handler, *func);
            let args = args
                .into_iter()
                .map(|arg| {
                    if arg.mutable {
                        Ok(Arg::Mutable(resolve_place(
                            adt_scope, var_scope, hir, handler, arg.val,
                        )?))
                    } else {
                        Ok(Arg::Immutable(resolve_expr(
                            adt_scope, var_scope, hir, handler, arg.val,
                        )?))
                    }
                })
                .try_collect_eager();
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
                if let Some(&id) = var_scope.get(&capture)
                    && let info = hir.var_info(id)
                    && info.mutable
                {
                    let id = hir.add_var(info.ident, false, info.span);
                    var_scope.insert(capture, id);
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

fn resolve_place(
    adt_scope: &Scope<AdtId>,
    var_scope: &Scope<VarId>,
    hir: &mut Hir,
    handler: &mut ErrorHandler,
    place: AstExpr,
) -> Result<Place> {
    match place.kind {
        ExprKind::Ident(id) => {
            let var_id = var_scope[&id];
            if hir.var_info(var_id).mutable {
                Ok(Place::Ident(var_id))
            } else {
                Err(handler.err(ErrorKind::Mutation.span(place.span)))
            }
        }
        ExprKind::Field { base, field } => {
            let base = Box::new(resolve_place(adt_scope, var_scope, hir, handler, *base)?);
            Ok(Place::Field { base, field })
        }
        ExprKind::Index { arr, idx } => {
            let arr = Box::new(resolve_place(adt_scope, var_scope, hir, handler, *arr)?);
            let idx = resolve_expr(adt_scope, var_scope, hir, handler, *idx)?;
            Ok(Place::Index { arr, idx })
        }
        ExprKind::Call { .. } => todo!("Projections"),
        _ => Err(handler.err(ErrorKind::NotPlaceExpr.span(place.span))),
    }
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
        .try_collect_eager()?;
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

fn check_places_unique(&mut self, hir: &Hir, place_a: ExprId, place_b: ExprId) -> Result<()> {
    match hir.expr_info(place_b) {
        info @ Expr::Ident(_) => {
            if hir.expr_info(place_a) == info {
                Err(self.handler.err(
                    ErrorKind::OverlappingPlace(hir.expr_span(place_a))
                        .span(hir.expr_span(place_b)),
                ))
            } else {
                Ok(())
            }
        }
        Expr::Field { base, .. } | Expr::Index { arr: base, .. } => {
            self.check_places_unique(hir, place_a, *base)
        }
        Expr::Call { .. } => todo!("Projections"),
        _ => Err(self
            .handler
            .err(ErrorKind::NotPlaceExpr.span(hir.expr_span(place_b)))),
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

const fn convert_prefix_op(op: AstPrefixOp) -> HirPrefixOp {
    match op {
        AstPrefixOp::Not => HirPrefixOp::Not,
        AstPrefixOp::Neg => HirPrefixOp::Neg,
    }
}

/// # Panics
/// Panics if the op is [`Assign`][`AstInfixOp::Assign`]
const fn convert_infix_op(op: AstInfixOp) -> HirInfixOp {
    match op {
        AstInfixOp::Assign => panic!("Assignment should be handled seperately"),
        AstInfixOp::Add => HirInfixOp::Add,
        AstInfixOp::AddF => HirInfixOp::AddF,
        AstInfixOp::Sub => HirInfixOp::Sub,
        AstInfixOp::SubF => HirInfixOp::SubF,
        AstInfixOp::Mul => HirInfixOp::Mul,
        AstInfixOp::MulF => HirInfixOp::MulF,
        AstInfixOp::Div => HirInfixOp::Div,
        AstInfixOp::DivF => HirInfixOp::DivF,
        AstInfixOp::Exp => HirInfixOp::Exp,
        AstInfixOp::And => HirInfixOp::And,
        AstInfixOp::Or => HirInfixOp::Or,
        AstInfixOp::Xor => HirInfixOp::Xor,
        AstInfixOp::Eqq => HirInfixOp::Eqq,
        AstInfixOp::Neq => HirInfixOp::Neq,
        AstInfixOp::Gt => HirInfixOp::Gt,
        AstInfixOp::Lt => HirInfixOp::Lt,
        AstInfixOp::Geq => HirInfixOp::Geq,
        AstInfixOp::Leq => HirInfixOp::Leq,
    }
}
