use ast::{Expr, ExprKind, MatchArm};
use span::Spannable;

use crate::error::{TypeError, TypeErrorS};

use super::{Ty, TypeChecker};

impl TypeChecker {
    fn sub_ty(&mut self, ty: Ty) -> Result<Ty, TypeError> {
        match ty {
            Ty::Int | Ty::UInt | Ty::Byte | Ty::Float | Ty::Bool | Ty::Char => Ok(ty),
            Ty::Array(ty) => Ok(Ty::Array(Box::new(self.sub_ty(*ty)?))),
            Ty::Tuple(tys) => Ok(Ty::Tuple(self.sub_ty_all(tys)?)),
            Ty::Func(param_tys, return_ty) => {
                let param_tys = self.sub_ty_all(param_tys)?;
                let return_ty = Box::new(self.sub_ty(*return_ty)?);
                Ok(Ty::Func(param_tys, return_ty))
            }
            Ty::Adt(ident, arg_tys) => Ok(Ty::Adt(ident, self.sub_ty_all(arg_tys)?)),
            Ty::Var(var) | Ty::IntVar(var) => {
                let root = self.table.find(var);
                self.table
                    .probe_value(root)
                    .map_or(Err(TypeError::UninferredType), |ty| self.sub_ty(ty))
            }
        }
    }

    fn sub_ty_all(&mut self, tys: Vec<Ty>) -> Result<Vec<Ty>, TypeError> {
        tys.into_iter().map(|ty| self.sub_ty(ty)).collect()
    }

    pub(super) fn sub_ast(&mut self, mut expr: Expr<Ty>) -> Result<Expr<Ty>, TypeErrorS> {
        expr.ty = self.sub_ty(expr.ty).map_err(|err| err.span(expr.span))?;
        expr.kind = match expr.kind {
            ExprKind::Ident(_)
            | ExprKind::Int(_)
            | ExprKind::Float(_)
            | ExprKind::String(_)
            | ExprKind::Char(_)
            | ExprKind::Bool(_) => expr.kind,
            ExprKind::Array(exprs) => ExprKind::Array(self.sub_ast_all(exprs)?),
            ExprKind::Tuple(exprs) => ExprKind::Tuple(self.sub_ast_all(exprs)?),
            ExprKind::CallExpr { func, args } => ExprKind::CallExpr {
                func: self.sub_ast_box(func)?,
                args: self.sub_ast_all(args)?,
            },
            ExprKind::InfixExpr { op, lhs, rhs } => ExprKind::InfixExpr {
                op,
                lhs: self.sub_ast_box(lhs)?,
                rhs: self.sub_ast_box(rhs)?,
            },
            ExprKind::UnaryExpr { op, expr } => ExprKind::UnaryExpr {
                op,
                expr: self.sub_ast_box(expr)?,
            },
            ExprKind::IndexExpr { arr, idx } => ExprKind::IndexExpr {
                arr: self.sub_ast_box(arr)?,
                idx: self.sub_ast_box(idx)?,
            },
            ExprKind::FieldExpr { base, field } => ExprKind::FieldExpr {
                base: self.sub_ast_box(base)?,
                field,
            },
            ExprKind::If { cond, th, el } => ExprKind::If {
                cond: self.sub_ast_box(cond)?,
                th: self.sub_ast_box(th)?,
                el: el.map(|el| self.sub_ast(*el)).transpose()?.map(Box::new),
            },
            ExprKind::For {
                pattern,
                iter,
                body,
            } => ExprKind::For {
                pattern,
                iter: self.sub_ast_box(iter)?,
                body: self.sub_ast_box(body)?,
            },
            ExprKind::While { cond, body } => ExprKind::While {
                cond: self.sub_ast_box(cond)?,
                body: self.sub_ast_box(body)?,
            },
            ExprKind::Match { scrutinee, arms } => {
                let scrutinee = self.sub_ast_box(scrutinee)?;
                let arms = arms
                    .into_iter()
                    .map(
                        |MatchArm {
                             pattern,
                             guard,
                             body,
                             span,
                         }| {
                            Ok(MatchArm {
                                pattern,
                                guard: guard
                                    .map(|guard| self.sub_ast(*guard))
                                    .transpose()?
                                    .map(Box::new),
                                body: self.sub_ast_box(body)?,
                                span,
                            })
                        },
                    )
                    .collect::<Result<Vec<_>, _>>()?;
                ExprKind::Match { scrutinee, arms }
            }
            ExprKind::Let { binding, val } => ExprKind::Let {
                binding,
                val: self.sub_ast_box(val)?,
            },
            ExprKind::Assign { ident, val } => ExprKind::Assign {
                ident,
                val: self.sub_ast_box(val)?,
            },
            ExprKind::LambdaExpr {
                params,
                return_ty,
                body,
            } => ExprKind::LambdaExpr {
                params,
                return_ty,
                body: self.sub_ast_box(body)?,
            },
            ExprKind::Block(exprs) => ExprKind::Block(self.sub_ast_all(exprs)?),
        };
        Ok(expr)
    }

    fn sub_ast_all(&mut self, exprs: Vec<Expr<Ty>>) -> Result<Vec<Expr<Ty>>, TypeErrorS> {
        exprs.into_iter().map(|expr| self.sub_ast(expr)).collect()
    }

    fn sub_ast_box(&mut self, expr: Box<Expr<Ty>>) -> Result<Box<Expr<Ty>>, TypeErrorS> {
        self.sub_ast(*expr).map(Box::new)
    }
}
