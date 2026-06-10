use hir::{BlockExpr, Expr, ExprId, Hir, InfixOp, LitExpr, PrefixOp, Stmt};

use crate::{
    TypeChecker,
    types::{Param, PartialTy},
};

impl TypeChecker<'_> {
    #[allow(
        clippy::too_many_lines,
        reason = "Any given arm is readable on it's own"
    )]
    pub(super) fn infer_expr(&mut self, hir: &Hir, expr: ExprId) -> PartialTy {
        let ty = match hir.expr_info(expr) {
            Expr::Ident(id) => self.var_ty(hir, *id).clone(),
            Expr::Lit(lit) => match lit {
                LitExpr::Int(_) => PartialTy::int_var(&mut self.table),
                LitExpr::Float(_) => PartialTy::Float,
                LitExpr::String(_) => todo!("String type"),
                LitExpr::Char(_) => PartialTy::Char,
                LitExpr::Bool(_) => PartialTy::Bool,
            },
            Expr::Array(exprs) => {
                let inner_ty = PartialTy::var(&mut self.table);
                for expr in exprs {
                    let ty = self.infer_expr(hir, *expr);
                    self.constrain_eq(ty, inner_ty.clone(), hir.expr_span(*expr));
                }
                PartialTy::Array(Box::new(inner_ty))
            }
            Expr::Tuple(exprs) => PartialTy::Tuple(self.infer_exprs(hir, exprs)),
            Expr::Call { func, args } => {
                let func_ty = self.infer_expr(hir, *func);
                let arg_tys = args
                    .iter()
                    .map(|arg| Param {
                        ty: self.infer_expr(hir, arg.val),
                        mutable: arg.mutable,
                        span: arg.span,
                    })
                    .collect();
                let ret_ty = PartialTy::var(&mut self.table);
                self.constrain_eq(
                    func_ty,
                    PartialTy::Fn(arg_tys, Box::new(ret_ty.clone())),
                    hir.expr_span(expr),
                );
                ret_ty
            }
            &Expr::Infix { op, lhs, rhs } => {
                let lhs_ty = self.infer_expr(hir, lhs);
                let rhs_ty = self.infer_expr(hir, rhs);
                match op {
                    InfixOp::Assign => {
                        self.constrain_eq(rhs_ty, lhs_ty, hir.expr_span(rhs));
                        PartialTy::unit()
                    }
                    InfixOp::Add | InfixOp::Sub | InfixOp::Mul | InfixOp::Div => {
                        let int_var = PartialTy::int_var(&mut self.table);
                        self.constrain_eq(lhs_ty, int_var.clone(), hir.expr_span(lhs));
                        self.constrain_eq(rhs_ty, int_var.clone(), hir.expr_span(rhs));
                        int_var
                    }
                    InfixOp::AddF | InfixOp::SubF | InfixOp::MulF | InfixOp::DivF => {
                        self.constrain_eq(lhs_ty, PartialTy::Float, hir.expr_span(lhs));
                        self.constrain_eq(rhs_ty, PartialTy::Float, hir.expr_span(rhs));
                        PartialTy::Float
                    }
                    InfixOp::Exp => {
                        self.constrain_eq(lhs_ty, PartialTy::Float, hir.expr_span(lhs));
                        let int_var = PartialTy::int_var(&mut self.table);
                        self.constrain_eq(rhs_ty, int_var, hir.expr_span(rhs));
                        PartialTy::Float
                    }
                    InfixOp::And | InfixOp::Or | InfixOp::Xor => {
                        self.constrain_eq(lhs_ty, PartialTy::Bool, hir.expr_span(lhs));
                        self.constrain_eq(rhs_ty, PartialTy::Bool, hir.expr_span(rhs));
                        PartialTy::Bool
                    }
                    InfixOp::Eqq | InfixOp::Neq => {
                        self.constrain_eq(rhs_ty, lhs_ty, hir.expr_span(rhs));
                        PartialTy::Bool
                    }
                    InfixOp::Gt | InfixOp::Lt | InfixOp::Geq | InfixOp::Leq => {
                        self.constrain_eq(lhs_ty, PartialTy::Float, hir.expr_span(lhs));
                        self.constrain_eq(rhs_ty, PartialTy::Float, hir.expr_span(rhs));
                        PartialTy::Bool
                    }
                }
            }
            &Expr::Prefix { op, expr } => {
                let expr_ty = self.infer_expr(hir, expr);
                match op {
                    PrefixOp::Not => {
                        self.constrain_eq(expr_ty, PartialTy::Bool, hir.expr_span(expr));
                        PartialTy::Bool
                    }
                    PrefixOp::Neg => {
                        self.constrain_eq(expr_ty, PartialTy::Float, hir.expr_span(expr));
                        PartialTy::Float
                    }
                }
            }
            &Expr::Index { arr, idx } => {
                let idx_ty = self.infer_expr(hir, idx);
                self.constrain_eq(idx_ty, PartialTy::UInt, hir.expr_span(idx));
                let arr_ty = self.infer_expr(hir, arr);
                let inner_ty = PartialTy::var(&mut self.table);
                self.constrain_eq(
                    arr_ty,
                    PartialTy::Array(Box::new(inner_ty.clone())),
                    hir.expr_span(arr),
                );
                inner_ty
            }
            &Expr::Field { base, field } => {
                let base_ty = self.infer_expr(hir, base);
                let field_ty = PartialTy::var(&mut self.table);
                self.constrain_field(base_ty, hir.expr_span(base), field_ty.clone(), field);
                field_ty
            }
            Expr::Lambda { params, body, .. } => {
                let param_tys = params
                    .iter()
                    .map(|id| {
                        let info = hir.var_info(*id);
                        Param {
                            ty: self.var_ty(hir, *id).clone(),
                            mutable: info.mutable,
                            span: info.span,
                        }
                    })
                    .collect();
                let body_ty = self.infer_expr(hir, *body);
                PartialTy::Fn(param_tys, Box::new(body_ty))
            }
            Expr::If { cond, th, el } => {
                let cond_ty = self.infer_expr(hir, *cond);
                self.constrain_eq(cond_ty, PartialTy::Bool, hir.expr_span(*cond));
                let th_ty = self.infer_block_expr(hir, th);
                match el {
                    Some(el) => {
                        let el_ty = self.infer_block_expr(hir, el);
                        self.constrain_eq(el_ty, th_ty.clone(), el.span);
                    }
                    None => self.constrain_eq(th_ty.clone(), PartialTy::unit(), th.span),
                }
                th_ty
            }
            Expr::For { id, iter, body } => {
                let iter_ty = self.infer_expr(hir, *iter);
                let var_ty = self.var_ty(hir, *id).clone();
                self.constrain_eq(var_ty, iter_ty, hir.expr_span(*iter));
                self.infer_block_expr(hir, body);
                PartialTy::unit()
            }
            Expr::Loop(body) => {
                self.infer_block_expr(hir, body);
                PartialTy::unit()
            }
            Expr::Break => todo!(),
            Expr::Continue => todo!(),
            Expr::Return(_) => todo!(),
            Expr::Block(block) => self.infer_block_expr(hir, block),

            Expr::Print(expr) => {
                let _ = self.infer_expr(hir, *expr);
                PartialTy::unit()
            }
        };
        self.substitution.insert(expr, ty.clone());
        ty
    }

    fn infer_exprs(&mut self, hir: &Hir, exprs: &[ExprId]) -> Vec<PartialTy> {
        exprs.iter().map(|&e| self.infer_expr(hir, e)).collect()
    }

    fn infer_block_expr(&mut self, hir: &Hir, block: &BlockExpr) -> PartialTy {
        let mut stmt_tys: Vec<_> = block
            .stmts
            .iter()
            .map(|stmt| match stmt {
                Stmt::Decl { id, val, .. } => {
                    let val_ty = self.infer_expr(hir, *val);
                    let var_ty = self.var_ty(hir, *id).clone();
                    self.constrain_eq(val_ty, var_ty, hir.expr_span(*val));
                    PartialTy::unit()
                }
                Stmt::Expr(expr) => self.infer_expr(hir, *expr),
            })
            .collect();
        stmt_tys.pop().unwrap_or_else(PartialTy::unit)
    }
}
