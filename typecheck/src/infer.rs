use itertools::Itertools;

use hir::{
    Hir, VarId,
    exprs::{Arg, BlockExpr, Expr, ExprId, InfixOp, LitExpr, PrefixOp, Stmt},
};
use ident::SpanIdent;

use crate::{ErrorKind, PartialTy, Result, TypeChecker, types::Param};

impl TypeChecker {
    pub(super) fn infer_expr(&mut self, hir: &Hir, expr: ExprId) -> Result<PartialTy> {
        match hir.expr_info(expr) {
            Expr::Ident(id) => Ok(self.ctx[*id].clone()),
            Expr::Lit(lit) => Ok(self.infer_lit(lit)),
            Expr::Array(exprs) => self.infer_array(hir, exprs),
            Expr::Tuple(exprs) => self.infer_tuple(hir, exprs),
            Expr::Call { func, args } => self.infer_call(hir, *func, args),
            Expr::Infix { op, lhs, rhs } => self.infer_infix(hir, *op, *lhs, *rhs),
            Expr::Prefix { op, expr } => self.infer_prefix(hir, *op, *expr),
            Expr::Index { arr, idx } => self.infer_indexing(hir, *arr, *idx),
            Expr::Field { base, field } => self.infer_field(hir, *base, *field),
            Expr::Lambda { params, body } => self.infer_lambda(hir, params, *body),
            Expr::If { cond, th, el } => self.infer_if(hir, *cond, th, el.as_ref()),
            Expr::For { id, iter, body } => self.infer_for(hir, *id, *iter, body),
            Expr::Loop(body) => self.infer_loop(hir, body),
            Expr::Break => todo!(),
            Expr::Continue => todo!(),
            Expr::Return(_) => todo!(),
            Expr::Block(block) => self.infer_block_expr(hir, block),
        }
        .inspect(|ty| {
            self.substitution.insert(expr, ty.clone());
        })
    }

    fn infer_exprs(&mut self, hir: &Hir, exprs: &[ExprId]) -> Result<Vec<PartialTy>> {
        exprs.iter().map(|&e| self.infer_expr(hir, e)).collect()
    }

    fn infer_lit(&mut self, lit: &LitExpr) -> PartialTy {
        match &lit {
            LitExpr::Int(_) => self.fresh_int_var(),
            LitExpr::Float(_) => PartialTy::Float,
            LitExpr::String(_) => todo!("String type"),
            LitExpr::Char(_) => PartialTy::Char,
            LitExpr::Bool(_) => PartialTy::Bool,
        }
    }

    fn infer_array(&mut self, hir: &Hir, exprs: &[ExprId]) -> Result<PartialTy> {
        let inner_ty = self.fresh_var();
        for &expr in exprs {
            let ty = self.infer_expr(hir, expr)?;
            self.constrain_eq(ty, inner_ty.clone(), hir.expr_span(expr));
        }
        Ok(PartialTy::Array(Box::new(inner_ty)))
    }

    fn infer_tuple(&mut self, hir: &Hir, exprs: &[ExprId]) -> Result<PartialTy> {
        let tys = self.infer_exprs(hir, exprs)?;
        Ok(PartialTy::Tuple(tys))
    }

    fn infer_call(&mut self, hir: &Hir, func: ExprId, args: &[Arg]) -> Result<PartialTy> {
        // Verify uniqueness of mutable arguments
        args.iter()
            .permutations(2)
            .map(|p| (p[0], p[1]))
            .filter(|(a, b)| a.mutable || b.mutable)
            .try_for_each(|(a, b)| check_places_unique(hir, a.val, b.val))?; // TODO optimise???

        let func_ty = self.infer_expr(hir, func)?;
        let arg_tys = args
            .iter()
            .map(|arg| {
                let ty = self.infer_expr(hir, arg.val)?;

                if arg.mutable {
                    check_place_mut(hir, arg.val)?;
                }

                Ok(Param {
                    mutable: arg.mutable,
                    ty,
                })
            })
            .try_collect()?;
        let ret_ty = self.fresh_var();

        self.constrain_eq(
            func_ty,
            PartialTy::Fn(arg_tys, Box::new(ret_ty.clone())),
            hir.expr_span(func),
        );

        Ok(ret_ty)
    }

    fn infer_infix(
        &mut self,

        hir: &Hir,
        op: InfixOp,
        lhs: ExprId,
        rhs: ExprId,
    ) -> Result<PartialTy> {
        let lhs_ty = self.infer_expr(hir, lhs)?;
        let rhs_ty = self.infer_expr(hir, rhs)?;
        match op {
            InfixOp::Assign => {
                check_place_mut(hir, lhs)?;
                self.constrain_eq(rhs_ty, lhs_ty, hir.expr_span(rhs));
                Ok(PartialTy::unit())
            }
            InfixOp::Add | InfixOp::Sub | InfixOp::Mul | InfixOp::Div => {
                let int_var = self.fresh_int_var();
                self.constrain_eq(lhs_ty, int_var.clone(), hir.expr_span(lhs));
                self.constrain_eq(rhs_ty, int_var.clone(), hir.expr_span(rhs));
                Ok(int_var)
            }
            InfixOp::AddF | InfixOp::SubF | InfixOp::MulF | InfixOp::DivF => {
                self.constrain_eq(lhs_ty, PartialTy::Float, hir.expr_span(lhs));
                self.constrain_eq(rhs_ty, PartialTy::Float, hir.expr_span(rhs));
                Ok(PartialTy::Float)
            }
            InfixOp::Exp => {
                self.constrain_eq(lhs_ty, PartialTy::Float, hir.expr_span(lhs));
                let int_var = self.fresh_int_var();
                self.constrain_eq(rhs_ty, int_var, hir.expr_span(rhs));
                Ok(PartialTy::Float)
            }
            InfixOp::And | InfixOp::Or | InfixOp::Xor => {
                self.constrain_eq(lhs_ty, PartialTy::Bool, hir.expr_span(lhs));
                self.constrain_eq(rhs_ty, PartialTy::Bool, hir.expr_span(rhs));
                Ok(PartialTy::Bool)
            }
            InfixOp::Eqq | InfixOp::Neq => {
                self.constrain_eq(rhs_ty, lhs_ty, hir.expr_span(rhs));
                Ok(PartialTy::Bool)
            }
            InfixOp::Gt | InfixOp::Lt | InfixOp::Geq | InfixOp::Leq => {
                self.constrain_eq(lhs_ty, PartialTy::Float, hir.expr_span(lhs));
                self.constrain_eq(rhs_ty, PartialTy::Float, hir.expr_span(rhs));
                Ok(PartialTy::Bool)
            }
        }
    }

    fn infer_prefix(&mut self, hir: &Hir, op: PrefixOp, expr: ExprId) -> Result<PartialTy> {
        let expr_ty = self.infer_expr(hir, expr)?;
        match op {
            PrefixOp::Not => {
                self.constrain_eq(expr_ty, PartialTy::Bool, hir.expr_span(expr));
                Ok(PartialTy::Bool)
            }
            PrefixOp::Neg => {
                self.constrain_eq(expr_ty, PartialTy::Float, hir.expr_span(expr));
                Ok(PartialTy::Float)
            }
        }
    }

    fn infer_indexing(&mut self, hir: &Hir, arr: ExprId, idx: ExprId) -> Result<PartialTy> {
        let arr_ty = self.infer_expr(hir, arr)?;
        let inner_ty = self.fresh_var();
        self.constrain_eq(
            arr_ty,
            PartialTy::Array(Box::new(inner_ty.clone())),
            hir.expr_span(arr),
        );

        let idx_ty = self.infer_expr(hir, idx)?;
        self.constrain_eq(idx_ty, PartialTy::UInt, hir.expr_span(idx));

        Ok(inner_ty)
    }

    fn infer_field(&mut self, hir: &Hir, base: ExprId, field: SpanIdent) -> Result<PartialTy> {
        let base_ty = self.infer_expr(hir, base)?;

        let PartialTy::Adt(base_ty) = base_ty else {
            return Err(ErrorKind::PrimitiveTypeNoField(base_ty).span(hir.expr_span(base)));
        };

        let field_ty = PartialTy::from(
            hir.adt_info(base_ty)
                .fields
                .get_ty(field.ident)
                .ok_or_else(|| ErrorKind::MissingField.span(field.span))?,
        );

        Ok(field_ty)
    }

    fn infer_if(
        &mut self,

        hir: &Hir,
        cond: ExprId,
        th: &BlockExpr,
        el: Option<&BlockExpr>,
    ) -> Result<PartialTy> {
        let cond_ty = self.infer_expr(hir, cond)?;
        self.constrain_eq(cond_ty, PartialTy::Bool, hir.expr_span(cond));

        let th_ty = self.infer_block_expr(hir, th)?;

        match el {
            Some(el) => {
                let el_ty = self.infer_block_expr(hir, el)?;
                self.constrain_eq(el_ty, th_ty.clone(), el.span);
            }
            None => self.constrain_eq(th_ty.clone(), PartialTy::unit(), th.span),
        }

        Ok(th_ty)
    }

    fn infer_for(
        &mut self,

        hir: &Hir,
        id: VarId,
        iter: ExprId,
        body: &BlockExpr,
    ) -> Result<PartialTy> {
        let iter_ty = self.infer_expr(hir, iter)?;
        self.constrain_eq(self.ctx[id].clone(), iter_ty, hir.expr_span(iter));

        self.infer_block_expr(hir, body)?;

        Ok(PartialTy::unit())
    }

    fn infer_loop(&mut self, hir: &Hir, body: &BlockExpr) -> Result<PartialTy> {
        self.infer_block_expr(hir, body)?;
        Ok(PartialTy::unit())
    }

    fn infer_lambda(&mut self, hir: &Hir, params: &[VarId], body: ExprId) -> Result<PartialTy> {
        let param_tys = params
            .iter()
            .map(|id| Param {
                mutable: hir.var_info(*id).mutable,
                ty: self.ctx[*id].clone(),
            })
            .collect();
        let body_ty = self.infer_expr(hir, body)?;

        Ok(PartialTy::Fn(param_tys, Box::new(body_ty)))
    }

    fn infer_block_expr(&mut self, hir: &Hir, block: &BlockExpr) -> Result<PartialTy> {
        let mut stmt_tys: Vec<_> = block
            .stmts
            .iter()
            .map(|s| self.infer_stmt(hir, s))
            .try_collect()?;
        Ok(stmt_tys.pop().unwrap_or_else(PartialTy::unit))
    }

    fn infer_stmt(&mut self, hir: &Hir, stmt: &Stmt) -> Result<PartialTy> {
        match stmt {
            Stmt::Decl { id, val, .. } => {
                let val_ty = self.infer_expr(hir, *val)?;
                self.constrain_eq(val_ty, self.ctx[*id].clone(), hir.expr_span(*val));

                Ok(PartialTy::unit())
            }
            Stmt::Expr(expr) => self.infer_expr(hir, *expr),
        }
    }
}

fn check_place_mut(hir: &Hir, place: ExprId) -> Result<()> {
    let span = hir.expr_span(place);
    match hir.expr_info(place) {
        Expr::Ident(id) => {
            if hir.var_info(*id).mutable {
                Ok(())
            } else {
                Err(ErrorKind::Mutation.span(span))
            }
        }
        Expr::Field { base, .. } | Expr::Index { arr: base, .. } => check_place_mut(hir, *base),
        Expr::Call { .. } => todo!("Projections"),
        _ => Err(ErrorKind::NotPlaceExpr.span(span)),
    }
}

fn check_places_unique(hir: &Hir, place_a: ExprId, place_b: ExprId) -> Result<()> {
    match hir.expr_info(place_b) {
        info @ Expr::Ident(_) => {
            if hir.expr_info(place_a) == info {
                Err(ErrorKind::OverlappingPlace(hir.expr_span(place_a))
                    .span(hir.expr_span(place_b)))
            } else {
                Ok(())
            }
        }
        Expr::Field { base, .. } | Expr::Index { arr: base, .. } => {
            check_places_unique(hir, place_a, *base)
        }
        Expr::Call { .. } => todo!("Projections"),
        _ => Err(ErrorKind::NotPlaceExpr.span(hir.expr_span(place_b))),
    }
}
