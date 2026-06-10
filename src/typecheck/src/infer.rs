use hir::{BlockExpr, Expr, ExprId, Hir, InfixOp, LitExpr, PrefixOp, Stmt};
use package::ModuleId;

use crate::{
    TypeChecker,
    types::{Param, PartialTy},
};

impl TypeChecker<'_> {
    #[allow(
        clippy::too_many_lines,
        reason = "Any given arm is readable on it's own"
    )]
    pub(super) fn infer_expr(&mut self, hir: &Hir, module: ModuleId, expr: ExprId) -> PartialTy {
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
                    let ty = self.infer_expr(hir, module, *expr);
                    self.constrain_eq(ty, inner_ty.clone(), hir.expr_span(*expr), module);
                }
                PartialTy::Array(Box::new(inner_ty))
            }
            Expr::Tuple(exprs) => PartialTy::Tuple(self.infer_exprs(hir, module, exprs)),
            Expr::Call { func, args } => {
                let func_ty = self.infer_expr(hir, module, *func);
                let arg_tys = args
                    .iter()
                    .map(|arg| Param {
                        ty: self.infer_expr(hir, module, arg.val),
                        mutable: arg.mutable,
                        span: arg.span,
                    })
                    .collect();
                let ret_ty = PartialTy::var(&mut self.table);
                self.constrain_eq(
                    func_ty,
                    PartialTy::Fn(arg_tys, Box::new(ret_ty.clone())),
                    hir.expr_span(expr),
                    module,
                );
                ret_ty
            }
            &Expr::Infix { op, lhs, rhs } => {
                let lhs_ty = self.infer_expr(hir, module, lhs);
                let rhs_ty = self.infer_expr(hir, module, rhs);
                match op {
                    InfixOp::Assign => {
                        self.constrain_eq(rhs_ty, lhs_ty, hir.expr_span(rhs), module);
                        PartialTy::unit()
                    }
                    InfixOp::Add | InfixOp::Sub | InfixOp::Mul | InfixOp::Div => {
                        let int_var = PartialTy::int_var(&mut self.table);
                        self.constrain_eq(lhs_ty, int_var.clone(), hir.expr_span(lhs), module);
                        self.constrain_eq(rhs_ty, int_var.clone(), hir.expr_span(rhs), module);
                        int_var
                    }
                    InfixOp::AddF | InfixOp::SubF | InfixOp::MulF | InfixOp::DivF => {
                        self.constrain_eq(lhs_ty, PartialTy::Float, hir.expr_span(lhs), module);
                        self.constrain_eq(rhs_ty, PartialTy::Float, hir.expr_span(rhs), module);
                        PartialTy::Float
                    }
                    InfixOp::Exp => {
                        self.constrain_eq(lhs_ty, PartialTy::Float, hir.expr_span(lhs), module);
                        let int_var = PartialTy::int_var(&mut self.table);
                        self.constrain_eq(rhs_ty, int_var, hir.expr_span(rhs), module);
                        PartialTy::Float
                    }
                    InfixOp::And | InfixOp::Or | InfixOp::Xor => {
                        self.constrain_eq(lhs_ty, PartialTy::Bool, hir.expr_span(lhs), module);
                        self.constrain_eq(rhs_ty, PartialTy::Bool, hir.expr_span(rhs), module);
                        PartialTy::Bool
                    }
                    InfixOp::Eqq | InfixOp::Neq => {
                        self.constrain_eq(rhs_ty, lhs_ty, hir.expr_span(rhs), module);
                        PartialTy::Bool
                    }
                    InfixOp::Gt | InfixOp::Lt | InfixOp::Geq | InfixOp::Leq => {
                        self.constrain_eq(lhs_ty, PartialTy::Float, hir.expr_span(lhs), module);
                        self.constrain_eq(rhs_ty, PartialTy::Float, hir.expr_span(rhs), module);
                        PartialTy::Bool
                    }
                }
            }
            &Expr::Prefix { op, expr } => {
                let expr_ty = self.infer_expr(hir, module, expr);
                match op {
                    PrefixOp::Not => {
                        self.constrain_eq(expr_ty, PartialTy::Bool, hir.expr_span(expr), module);
                        PartialTy::Bool
                    }
                    PrefixOp::Neg => {
                        self.constrain_eq(expr_ty, PartialTy::Float, hir.expr_span(expr), module);
                        PartialTy::Float
                    }
                }
            }
            &Expr::Index { arr, idx } => {
                let idx_ty = self.infer_expr(hir, module, idx);
                self.constrain_eq(idx_ty, PartialTy::UInt, hir.expr_span(idx), module);
                let arr_ty = self.infer_expr(hir, module, arr);
                let inner_ty = PartialTy::var(&mut self.table);
                self.constrain_eq(
                    arr_ty,
                    PartialTy::Array(Box::new(inner_ty.clone())),
                    hir.expr_span(arr),
                    module,
                );
                inner_ty
            }
            &Expr::Field { base, field } => {
                let base_ty = self.infer_expr(hir, module, base);
                let field_ty = PartialTy::var(&mut self.table);
                self.constrain_field(
                    base_ty,
                    hir.expr_span(base),
                    field_ty.clone(),
                    field,
                    module,
                );
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
                let body_ty = self.infer_expr(hir, module, *body);
                PartialTy::Fn(param_tys, Box::new(body_ty))
            }
            Expr::If { cond, th, el } => {
                let cond_ty = self.infer_expr(hir, module, *cond);
                self.constrain_eq(cond_ty, PartialTy::Bool, hir.expr_span(*cond), module);
                let th_ty = self.infer_block_expr(hir, module, th);
                match el {
                    Some(el) => {
                        let el_ty = self.infer_block_expr(hir, module, el);
                        self.constrain_eq(el_ty, th_ty.clone(), el.span, module);
                    }
                    None => self.constrain_eq(th_ty.clone(), PartialTy::unit(), th.span, module),
                }
                th_ty
            }
            Expr::For { id, iter, body } => {
                let iter_ty = self.infer_expr(hir, module, *iter);
                let var_ty = self.var_ty(hir, *id).clone();
                self.constrain_eq(var_ty, iter_ty, hir.expr_span(*iter), module);
                self.infer_block_expr(hir, module, body);
                PartialTy::unit()
            }
            Expr::Loop(body) => {
                self.infer_block_expr(hir, module, body);
                PartialTy::unit()
            }
            Expr::Break => todo!(),
            Expr::Continue => todo!(),
            Expr::Return(_) => todo!(),
            Expr::Block(block) => self.infer_block_expr(hir, module, block),

            Expr::Print(expr) => {
                let _ = self.infer_expr(hir, module, *expr);
                PartialTy::unit()
            }
        };
        self.substitution.insert(expr, (ty.clone(), module));
        ty
    }

    fn infer_exprs(&mut self, hir: &Hir, module: ModuleId, exprs: &[ExprId]) -> Vec<PartialTy> {
        exprs
            .iter()
            .map(|&e| self.infer_expr(hir, module, e))
            .collect()
    }

    fn infer_block_expr(&mut self, hir: &Hir, module: ModuleId, block: &BlockExpr) -> PartialTy {
        let mut stmt_tys: Vec<_> = block
            .stmts
            .iter()
            .map(|stmt| match stmt {
                Stmt::Decl { id, val, .. } => {
                    let val_ty = self.infer_expr(hir, module, *val);
                    let var_ty = self.var_ty(hir, *id).clone();
                    self.constrain_eq(val_ty, var_ty, hir.expr_span(*val), module);
                    PartialTy::unit()
                }
                Stmt::Expr(expr) => self.infer_expr(hir, module, *expr),
            })
            .collect();
        stmt_tys.pop().unwrap_or_else(PartialTy::unit)
    }
}
