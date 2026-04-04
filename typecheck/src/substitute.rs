use ast::exprs::{Arg, Expr, ExprKind, MatchArm};

use crate::error::{Error, ErrorKind};

use crate::types::Param;
use crate::{ConcreteTy, Ty, TypeChecker};

impl TypeChecker {
    fn sub_ty(&mut self, ty: Ty) -> Result<ConcreteTy, ErrorKind> {
        match ty {
            Ty::Int => Ok(ConcreteTy::Int),
            Ty::UInt => Ok(ConcreteTy::UInt),
            Ty::Byte => Ok(ConcreteTy::Byte),
            Ty::Float => Ok(ConcreteTy::Float),
            Ty::Bool => Ok(ConcreteTy::Bool),
            Ty::Char => Ok(ConcreteTy::Char),
            Ty::Array(ty) => Ok(ConcreteTy::Array(Box::new(self.sub_ty(*ty)?))),
            Ty::Tuple(tys) => Ok(ConcreteTy::Tuple(self.sub_ty_all(tys)?)),
            Ty::Func(params, return_ty) => {
                let params = params
                    .into_iter()
                    .map(|param| {
                        self.sub_ty(param.ty).map(|ty| Param {
                            mutable: param.mutable,
                            ty,
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let return_ty = Box::new(self.sub_ty(*return_ty)?);
                Ok(ConcreteTy::Func(params, return_ty))
            }
            Ty::Adt(ident, arg_tys) => Ok(ConcreteTy::Adt(ident, self.sub_ty_all(arg_tys)?)),
            Ty::Var(var) | Ty::IntVar(var) => {
                let root = self.table.find(var);
                self.table
                    .probe_value(root)
                    .map_or(Err(ErrorKind::UninferredType), |ty| self.sub_ty(ty))
            }
        }
    }

    fn sub_ty_all(&mut self, tys: Vec<Ty>) -> Result<Vec<ConcreteTy>, ErrorKind> {
        tys.into_iter().map(|ty| self.sub_ty(ty)).collect()
    }

    pub(super) fn sub_expr(&mut self, expr: Expr<Ty>) -> Result<Expr<ConcreteTy>, Error> {
        let ty = self.sub_ty(expr.ty).map_err(|err| err.span(expr.span))?;
        let kind = match expr.kind {
            ExprKind::Ident(ident) => ExprKind::Ident(ident),
            ExprKind::Lit(lit) => ExprKind::Lit(lit),
            ExprKind::Array(exprs) => ExprKind::Array(self.sub_expr_all(exprs)?),
            ExprKind::Tuple(exprs) => ExprKind::Tuple(self.sub_expr_all(exprs)?),
            ExprKind::CallExpr { func, args } => ExprKind::CallExpr {
                func: self.sub_expr_box(func)?,
                args: args
                    .into_iter()
                    .map(|arg| {
                        self.sub_expr(arg.val).map(|val| Arg {
                            label: arg.label,
                            mutable: arg.mutable,
                            val,
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            },
            ExprKind::InfixExpr { op, lhs, rhs } => ExprKind::InfixExpr {
                op,
                lhs: self.sub_expr_box(lhs)?,
                rhs: self.sub_expr_box(rhs)?,
            },
            ExprKind::UnaryExpr { op, expr } => ExprKind::UnaryExpr {
                op,
                expr: self.sub_expr_box(expr)?,
            },
            ExprKind::Let { binding, val } => ExprKind::Let {
                binding,
                val: self.sub_expr_box(val)?,
            },
            ExprKind::LambdaExpr {
                params,
                return_ty,
                body,
            } => ExprKind::LambdaExpr {
                params,
                return_ty,
                body: self.sub_expr_box(body)?,
            },
            ExprKind::IndexExpr { arr, idx } => ExprKind::IndexExpr {
                arr: self.sub_expr_box(arr)?,
                idx: self.sub_expr_box(idx)?,
            },
            ExprKind::FieldExpr { base, field } => ExprKind::FieldExpr {
                base: self.sub_expr_box(base)?,
                field,
            },
            ExprKind::If { cond, th, el } => ExprKind::If {
                cond: self.sub_expr_box(cond)?,
                th: self.sub_expr_box(th)?,
                el: el.map(|el| self.sub_expr(*el)).transpose()?.map(Box::new),
            },
            ExprKind::Match { scrutinee, arms } => {
                let scrutinee = self.sub_expr_box(scrutinee)?;
                let arms = arms
                    .into_iter()
                    .map(|arm| {
                        Ok(MatchArm {
                            pattern: arm.pattern,
                            guard: arm
                                .guard
                                .map(|guard| self.sub_expr_box(guard))
                                .transpose()?,
                            body: self.sub_expr_box(arm.body)?,
                            span: arm.span,
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                ExprKind::Match { scrutinee, arms }
            }
            ExprKind::For {
                pattern,
                iter,
                body,
            } => ExprKind::For {
                pattern,
                iter: self.sub_expr_box(iter)?,
                body: self.sub_expr_box(body)?,
            },
            ExprKind::While { cond, body } => ExprKind::While {
                cond: self.sub_expr_box(cond)?,
                body: self.sub_expr_box(body)?,
            },
            ExprKind::Break => ExprKind::Break,
            ExprKind::Continue => ExprKind::Continue,
            ExprKind::Return(expr) => ExprKind::Return(self.sub_expr_box(expr)?),
            ExprKind::Block(exprs) => ExprKind::Block(self.sub_expr_all(exprs)?),
        };
        Ok(kind.span_ty(expr.span, ty))
    }

    fn sub_expr_all(&mut self, exprs: Vec<Expr<Ty>>) -> Result<Vec<Expr<ConcreteTy>>, Error> {
        exprs.into_iter().map(|expr| self.sub_expr(expr)).collect()
    }

    fn sub_expr_box(&mut self, expr: Box<Expr<Ty>>) -> Result<Box<Expr<ConcreteTy>>, Error> {
        self.sub_expr(*expr).map(Box::new)
    }
}
