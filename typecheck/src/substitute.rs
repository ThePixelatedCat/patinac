use itertools::Itertools;

use ast::exprs::{Arg, Expr, ExprKind, MatchArm, Stmt};
use nameres::{AdtId, VarId, VarTable};
use types::{Param, Return, Ty};

use crate::error::{Error, ErrorKind};

use crate::{PartialTy, TypeChecker};

impl TypeChecker {
    fn sub_ty(&mut self, ty: PartialTy) -> Result<Ty<AdtId>, ErrorKind> {
        match ty {
            PartialTy::Int => Ok(Ty::Int),
            PartialTy::UInt => Ok(Ty::UInt),
            PartialTy::Byte => Ok(Ty::Byte),
            PartialTy::Float => Ok(Ty::Float),
            PartialTy::Bool => Ok(Ty::Bool),
            PartialTy::Char => Ok(Ty::Char),
            PartialTy::Tuple(tys) => Ok(Ty::Tuple(self.sub_tys(tys)?)),
            PartialTy::Fn(params, ret) => {
                let params = params
                    .into_iter()
                    .map(|param| {
                        self.sub_ty(param.ty).map(|ty| Param {
                            mutable: param.mutable,
                            ty,
                        })
                    })
                    .try_collect()?;
                let ret = Box::new(Return {
                    mutable: ret.mutable,
                    ty: self.sub_ty(ret.ty)?,
                });
                Ok(Ty::Fn(params, ret))
            }
            PartialTy::Adt(ident, arg_tys) => Ok(Ty::Adt(ident, self.sub_tys(arg_tys)?)),
            PartialTy::Var(var) | PartialTy::IntVar(var) => {
                let root = self.table.find(var);
                self.table
                    .probe_value(root)
                    .map_or(Err(ErrorKind::UninferredType), |ty| self.sub_ty(ty))
            }
        }
    }

    fn sub_tys(&mut self, tys: Vec<PartialTy>) -> Result<Vec<Ty<AdtId>>, ErrorKind> {
        tys.into_iter().map(|ty| self.sub_ty(ty)).collect()
    }

    pub(super) fn sub_expr(
        &mut self,
        var_table: &mut VarTable,
        expr: Expr<PartialTy, AdtId, VarId>,
    ) -> Result<Expr<Ty<AdtId>, AdtId, VarId>, Error> {
        let ty = self.sub_ty(expr.ty).map_err(|err| err.span(expr.span))?;
        let kind = match expr.kind {
            ExprKind::Path(path) => {
                if !path.prefix.is_empty() {
                    todo!("handle paths")
                }

                var_table[path.end].ty = Some(ty.clone());
                ExprKind::Path(path)
            }
            ExprKind::Lit(lit) => ExprKind::Lit(lit),
            ExprKind::Array(exprs) => ExprKind::Array(self.sub_exprs(var_table, exprs)?),
            ExprKind::Tuple(exprs) => ExprKind::Tuple(self.sub_exprs(var_table, exprs)?),
            ExprKind::Call { func, args } => ExprKind::Call {
                func: self.sub_expr_box(var_table, func)?,
                args: args
                    .into_iter()
                    .map(|arg| {
                        self.sub_expr(var_table, arg.val).map(|val| Arg {
                            mutable: arg.mutable,
                            val,
                        })
                    })
                    .try_collect()?,
            },
            ExprKind::Infix { op, lhs, rhs } => ExprKind::Infix {
                op,
                lhs: self.sub_expr_box(var_table, lhs)?,
                rhs: self.sub_expr_box(var_table, rhs)?,
            },
            ExprKind::Unary { op, expr } => ExprKind::Unary {
                op,
                expr: self.sub_expr_box(var_table, expr)?,
            },
            ExprKind::Lambda { params, body } => ExprKind::Lambda {
                params,
                body: self.sub_expr_box(var_table, body)?,
            },
            ExprKind::Index { arr, idx } => ExprKind::Index {
                arr: self.sub_expr_box(var_table, arr)?,
                idx: self.sub_expr_box(var_table, idx)?,
            },
            ExprKind::Field { base, field } => ExprKind::Field {
                base: self.sub_expr_box(var_table, base)?,
                field,
            },
            ExprKind::If { cond, th, el } => ExprKind::If {
                cond: self.sub_expr_box(var_table, cond)?,
                th: self.sub_expr_box(var_table, th)?,
                el: el
                    .map(|el| self.sub_expr(var_table, *el))
                    .transpose()?
                    .map(Box::new),
            },
            ExprKind::Match { scrutinee, arms } => {
                let scrutinee = self.sub_expr_box(var_table, scrutinee)?;
                let arms = arms
                    .into_iter()
                    .map(|arm| {
                        Ok(MatchArm {
                            pat: arm.pat,
                            body: self.sub_expr(var_table, arm.body)?,
                            span: arm.span,
                        })
                    })
                    .try_collect()?;
                ExprKind::Match { scrutinee, arms }
            }
            ExprKind::For { pat, iter, body } => ExprKind::For {
                pat,
                iter: self.sub_expr_box(var_table, iter)?,
                body: self.sub_expr_box(var_table, body)?,
            },
            ExprKind::Loop(body) => ExprKind::Loop(self.sub_expr_box(var_table, body)?),
            ExprKind::Break => ExprKind::Break,
            ExprKind::Continue => ExprKind::Continue,
            ExprKind::Return(expr) => ExprKind::Return(self.sub_expr_box(var_table, expr)?),
            ExprKind::Block(stmts) => ExprKind::Block(
                stmts
                    .into_iter()
                    .map(|s| self.sub_stmt(var_table, s))
                    .try_collect()?,
            ),
        };
        Ok(kind.span_ty(expr.span, ty))
    }

    fn sub_exprs(
        &mut self,
        var_table: &mut VarTable,
        exprs: Vec<Expr<PartialTy, AdtId, VarId>>,
    ) -> Result<Vec<Expr<Ty<AdtId>, AdtId, VarId>>, Error> {
        exprs
            .into_iter()
            .map(|expr| self.sub_expr(var_table, expr))
            .collect()
    }

    fn sub_expr_box(
        &mut self,
        var_table: &mut VarTable,
        expr: Box<Expr<PartialTy, AdtId, VarId>>,
    ) -> Result<Box<Expr<Ty<AdtId>, AdtId, VarId>>, Error> {
        self.sub_expr(var_table, *expr).map(Box::new)
    }

    fn sub_stmt(
        &mut self,
        var_table: &mut VarTable,
        stmt: Stmt<PartialTy, AdtId, VarId>,
    ) -> Result<Stmt<Ty<AdtId>, AdtId, VarId>, Error> {
        match stmt {
            Stmt::Decl { binding, val, span } => Ok(Stmt::Decl {
                binding,
                val: self.sub_expr(var_table, val)?,
                span,
            }),
            Stmt::Expr(expr) => self.sub_expr(var_table, expr).map(Stmt::Expr),
        }
    }
}
