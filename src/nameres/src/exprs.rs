use foldhash::HashSet;
use itertools::Itertools as _;

use errors::{Result, TryCollectEager as _};
use irs::{
    ast::{self, ExprKind},
    hir::{self, Arg, ExprId, LitExpr, VarId},
};

use crate::{ErrorKind, ResolveInfo};

impl ResolveInfo<'_, '_> {
    pub fn resolve_expr(&mut self, expr: &ast::Expr) -> Result<ExprId> {
        let new_expr = match &expr.kind {
            ExprKind::Var(path) => match self.scopes.resolve_var(path) {
                Ok(id) => hir::Expr::Var(id),
                Err(e) => return Err(self.err(e, expr.span)),
            },
            ExprKind::Lit(lit) => {
                let lit = match lit {
                    ast::LitExpr::Int(i) => hir::LitExpr::Int(*i),
                    ast::LitExpr::Float(f) => hir::LitExpr::Float(*f),
                    ast::LitExpr::String(s) => hir::LitExpr::String(s.clone()),
                    ast::LitExpr::Bool(b) => hir::LitExpr::Bool(*b),
                };
                hir::Expr::Lit(lit)
            }
            ExprKind::Array(exprs) => hir::Expr::Array(self.resolve_exprs(exprs)?),
            ExprKind::Tuple(exprs) => hir::Expr::Tuple(self.resolve_exprs(exprs)?),
            ExprKind::Infix { op, lhs, rhs } => {
                let rhs = self.resolve_expr(rhs);
                let lhs = self.resolve_expr(lhs)?;
                match convert_infix_op(*op) {
                    Some(op) => hir::Expr::Infix { op, lhs, rhs: rhs? },
                    None => {
                        self.assert_is_place(lhs);
                        hir::Expr::Assign {
                            place: lhs,
                            value: rhs?,
                        }
                    }
                }
            }
            ExprKind::Prefix { op, expr } => hir::Expr::Prefix {
                op: convert_prefix_op(*op),
                expr: self.resolve_expr(expr)?,
            },
            ExprKind::Field { base, field } => hir::Expr::Field {
                base: self.resolve_expr(base)?,
                field: *field,
            },
            ExprKind::Index { array, index } => {
                let array = self.resolve_expr(array);
                let index = self.resolve_expr(index);
                hir::Expr::Index {
                    array: array?,
                    index: index?,
                }
            }
            ExprKind::Call { func, args } => {
                let func = self.resolve_expr(func);
                let args: Vec<Arg> = args
                    .iter()
                    .map(|arg| {
                        let val = self.resolve_expr(&arg.value)?;
                        if arg.mutable {
                            self.assert_is_place(val);
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
                        if self.overlaps(a.value, b.value) {
                            Err(self.err(ErrorKind::OverlappingArgs(a.span, b.span), a.span))
                        } else {
                            Ok(())
                        }
                    })?;

                hir::Expr::Call { func: func?, args }
            }
            ExprKind::MethodCall { base, method, args } => {
                let base = self.resolve_expr(base);
                let args: Vec<Arg> = args
                    .iter()
                    .map(|arg| {
                        let val = self.resolve_expr(&arg.value)?;
                        if arg.mutable {
                            self.assert_is_place(val);
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
                        if self.overlaps(a.value, b.value) {
                            Err(self.err(ErrorKind::OverlappingArgs(a.span, b.span), a.span))
                        } else {
                            Ok(())
                        }
                    })?;

                hir::Expr::MethodCall {
                    base: base?,
                    method: *method,
                    args,
                }
            }
            ExprKind::Lambda { params, body } => {
                self.scopes.push_scope();

                // Rebind all captures within the lambda body, making them all immutable.
                let mut captures = HashSet::default();
                self.collect_captures(&mut captures, body);
                let captures = captures
                    .into_iter()
                    .map(|capture| {
                        let info = self.hir.var_info(capture);
                        let rebinding = self.hir.add_var(hir::VarInfo {
                            mutable: false,
                            ..info
                        });
                        self.scopes.add_var(info.ident.ident, rebinding);
                        (capture, rebinding)
                    })
                    .collect();

                let params = params
                    .iter()
                    .map(|param| self.resolve_binding(param))
                    .try_collect_eager();
                let body = self.resolve_expr(body);

                self.scopes.pop_scope();

                hir::Expr::Lambda {
                    params: params?,
                    body: body?,
                    captures,
                }
            }
            ExprKind::If { cond, th, el } => {
                let cond = self.resolve_expr(cond);
                let th = self.resolve_block_expr(th);
                let el = el
                    .as_ref()
                    .map(|el| self.resolve_block_expr(el))
                    .transpose();
                hir::Expr::If {
                    cond: cond?,
                    th: th?,
                    el: el?,
                }
            }
            ExprKind::Match { .. } => todo!("Pattern Matching"),
            ExprKind::For { pat, iter, body } => {
                let iter = self.resolve_expr(iter);

                self.scopes.push_scope();
                let id = self.resolve_pat(pat, false, None);
                let body = self.resolve_block_expr(body);
                self.scopes.pop_scope();

                hir::Expr::For {
                    id,
                    iter: iter?,
                    body: body?,
                }
            }
            ExprKind::Loop(body) => hir::Expr::Loop(self.resolve_block_expr(body)?),
            ExprKind::Break => hir::Expr::Break,
            ExprKind::Continue => hir::Expr::Continue,
            ExprKind::Return(expr) => hir::Expr::Return(self.resolve_expr(expr)?),
            ExprKind::Block(stmts) => hir::Expr::Block(self.resolve_block_expr(stmts)?),
            ExprKind::Print(expr) => hir::Expr::Print(self.resolve_expr(expr)?),
        };

        Ok(self.hir.add_expr(new_expr, expr.span))
    }

    fn resolve_exprs(&mut self, exprs: &[ast::Expr]) -> Result<Vec<ExprId>> {
        exprs
            .iter()
            .map(|expr| self.resolve_expr(expr))
            .try_collect_eager()
    }

    fn resolve_block_expr(&mut self, block_expr: &ast::BlockExpr) -> Result<hir::BlockExpr> {
        self.scopes.push_scope();
        let stmts = block_expr
            .stmts
            .iter()
            .map(|stmt| match stmt {
                ast::Stmt::Decl {
                    binding,
                    value,
                    span,
                } => {
                    // val must be resolved before the binding, to ensure the declared variable isn't in scope within it's own declaration
                    let value = self.resolve_expr(value);
                    let var = self.resolve_binding(binding);
                    Ok(hir::Stmt::Decl {
                        var: var?,
                        value: value?,
                        span: *span,
                    })
                }
                ast::Stmt::Expr(expr) => self.resolve_expr(expr).map(hir::Stmt::Expr),
            })
            .try_collect_eager()?;
        self.scopes.pop_scope();

        Ok(hir::BlockExpr {
            stmts,
            span: block_expr.span,
        })
    }

    fn assert_is_place(&mut self, place: ExprId) {
        match self.hir.expr(place) {
            hir::Expr::Var(id) => {
                let info = self.hir.var_info(*id);
                if !info.mutable {
                    self.err(
                        ErrorKind::Mutation(info.ident.ident),
                        self.hir.expr_span(place),
                    );
                }
            }
            hir::Expr::Field { base, .. } | hir::Expr::Index { array: base, .. } => {
                self.assert_is_place(*base);
            }
            _ => {
                self.err(ErrorKind::NotPlaceExpr, self.hir.expr_span(place));
            }
        }
    }

    fn overlaps(&self, a: ExprId, b: ExprId) -> bool {
        match (self.hir.expr(a), self.hir.expr(b)) {
            (hir::Expr::Var(a), hir::Expr::Var(b)) => a == b,
            (hir::Expr::Var(_), hir::Expr::Index { array: arr, .. }) => self.overlaps(a, *arr),
            (hir::Expr::Var(_), hir::Expr::Field { base, .. }) => self.overlaps(a, *base),
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
                if let hir::Expr::Lit(LitExpr::Int(idx_a)) = self.hir.expr(*idx_a)
                    && let hir::Expr::Lit(LitExpr::Int(idx_b)) = self.hir.expr(*idx_b)
                {
                    idx_a == idx_b
                } else {
                    self.overlaps(*arr_a, *arr_b)
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
            ) => (field_a.ident == field_b.ident) && self.overlaps(*base_a, *base_b),
            (hir::Expr::Index { array: arr, .. }, hir::Expr::Field { base, .. }) => {
                self.overlaps(*arr, b) || self.overlaps(a, *base)
            }
            _ => false,
        }
    }

    fn collect_captures(&self, captures: &mut HashSet<VarId>, expr: &ast::Expr) {
        match &expr.kind {
            ExprKind::Var(path) => {
                // Unbound variables are either parameters, which don't need capturing, or actually unbound, which will produce an error anyway.
                if let Ok(id) = self.scopes.resolve_var(path)
                    && !self.hir.var_info(id).global
                {
                    captures.insert(id);
                }
            }
            ExprKind::Lit(_) | ExprKind::Break | ExprKind::Continue => {}
            ExprKind::Array(exprs) | ExprKind::Tuple(exprs) => {
                for e in exprs {
                    self.collect_captures(captures, e);
                }
            }
            ExprKind::Lambda { body: e, .. }
            | ExprKind::Field { base: e, .. }
            | ExprKind::Prefix { expr: e, .. }
            | ExprKind::Print(e)
            | ExprKind::Return(e) => self.collect_captures(captures, e),
            ExprKind::Infix {
                lhs: e1, rhs: e2, ..
            }
            | ExprKind::Index {
                array: e1,
                index: e2,
            } => {
                self.collect_captures(captures, e1);
                self.collect_captures(captures, e2);
            }
            ExprKind::Call { func, args } => {
                self.collect_captures(captures, func);
                for a in args {
                    self.collect_captures(captures, &a.value);
                }
            }
            ExprKind::MethodCall { base, args, .. } => {
                self.collect_captures(captures, base);
                for a in args {
                    self.collect_captures(captures, &a.value);
                }
            }
            ExprKind::Match { scrutinee, arms } => {
                self.collect_captures(captures, scrutinee);
                for a in arms {
                    self.collect_captures(captures, &a.body);
                }
            }
            ExprKind::If { cond, th, el } => {
                self.collect_captures(captures, cond);
                self.collect_block_captures(captures, th);
                el.as_ref()
                    .inspect(|el| self.collect_block_captures(captures, el));
            }
            ExprKind::For { iter, body, .. } => {
                self.collect_captures(captures, iter);
                self.collect_block_captures(captures, body);
            }
            ExprKind::Loop(stmts) | ExprKind::Block(stmts) => {
                self.collect_block_captures(captures, stmts);
            }
        }
    }

    fn collect_block_captures(&self, captures: &mut HashSet<VarId>, block: &ast::BlockExpr) {
        for ast::Stmt::Decl { value: expr, .. } | ast::Stmt::Expr(expr) in &block.stmts {
            self.collect_captures(captures, expr);
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
