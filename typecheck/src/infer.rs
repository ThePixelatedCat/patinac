use itertools::Itertools;

use hir::{
    AdtInfo, Hir, VarId,
    exprs::{Arg, Binding, BlockExpr, Expr, ExprId, InfixOp, LitExpr, MatchArm, PrefixOp, Stmt},
    patterns::Pat,
};
use ident::SpanIdent;

use crate::{
    ErrorKind, PartialTy, Result, TypeChecker,
    type_vars::{Param, Return},
};

impl TypeChecker {
    pub(super) fn infer_expr(&mut self, hir: &Hir, expr: ExprId) -> Result<PartialTy> {
        let span = hir.expr_span(expr);
        match hir.expr_info(expr) {
            Expr::Ident(id) => Ok(self.infer_ident(hir, *id)),
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
            Expr::Match { scrutinee, arms } => self.infer_match(hir, *scrutinee, arms),
            Expr::For { pat, iter, body } => self.infer_for(hir, pat, *iter, body),
            Expr::Loop(body) => self.infer_loop(hir, body),
            Expr::Break => todo!(),
            Expr::Continue => todo!(),
            Expr::Return(expr) => todo!(),
            Expr::Block(block) => self.infer_block_expr(hir, block),
        }
        .inspect(|ty| {
            self.substitution.insert(expr, ty.clone());
        })
    }

    fn infer_exprs(&mut self, hir: &Hir, exprs: &[ExprId]) -> Result<Vec<PartialTy>> {
        exprs.iter().map(|&e| self.infer_expr(hir, e)).collect()
    }

    fn infer_ident(&mut self, hir: &Hir, id: VarId) -> PartialTy {
        self.convert(hir.var_info(id).ty.as_ref())
    }

    fn check_place_mut(&mut self, hir: &Hir, place: ExprId) -> Result<()> {
        let span = hir.expr_span(place);
        match hir.expr_info(place) {
            Expr::Ident(id) => {
                if hir.var_info(*id).mutable {
                    Ok(())
                } else {
                    Err(ErrorKind::Mutation.span(span))
                }
            }
            Expr::Field { base, .. } | Expr::Index { arr: base, .. } => {
                self.check_place_mut(hir, *base)
            }
            Expr::Call { .. } => todo!("Projections"),
            _ => Err(ErrorKind::NotPlaceExpr.span(span)),
        }
    }

    fn check_places_unique(&mut self, hir: &Hir, place_a: ExprId, place_b: ExprId) -> Result<()> {
        match hir.expr_info(place_b) {
            info @ Expr::Ident(_) => {
                if hir.expr_info(place_a) != info {
                    Ok(())
                } else {
                    Err(ErrorKind::OverlappingPlace(hir.expr_span(place_a))
                        .span(hir.expr_span(place_b)))
                }
            }
            Expr::Field { base, .. } | Expr::Index { arr: base, .. } => {
                self.check_places_unique(hir, place_a, *base)
            }
            Expr::Call { .. } => todo!("Projections"),
            _ => Err(ErrorKind::NotPlaceExpr.span(hir.expr_span(place_b))),
        }
    }

    fn infer_lit(&mut self, lit: &LitExpr) -> PartialTy {
        match &lit {
            LitExpr::Int(_) => self.fresh_int_var(),
            LitExpr::Float(_) => PartialTy::Float,
            LitExpr::String(_) => PartialTy::string(),
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
        Ok(PartialTy::array(inner_ty))
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
            .try_for_each(|(a, b)| self.check_places_unique(hir, a.val, b.val))?; // TODO optimise???

        let func_ty = self.infer_expr(hir, func)?;
        let arg_tys = args
            .iter()
            .map(|arg| {
                let ty = self.infer_expr(hir, arg.val)?;

                if arg.mutable {
                    self.check_place_mut(hir, arg.val)?;
                }

                Ok(Param {
                    mutable: arg.mutable,
                    ty,
                })
            })
            .try_collect()?;
        let return_ty = self.fresh_var();

        self.constrain_eq(
            func_ty,
            PartialTy::Fn(
                arg_tys,
                Return {
                    mutable: false,
                    ty: Box::new(return_ty.clone()),
                },
            ),
            hir.expr_span(func),
        );

        Ok(return_ty)
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
                self.check_place_mut(hir, lhs)?;
                self.constrain_eq(rhs_ty, lhs_ty, hir.expr_span(rhs));
                Ok(PartialTy::unit())
            }
            InfixOp::Add | InfixOp::Sub | InfixOp::Mul | InfixOp::Div | InfixOp::Rem => {
                let int_var = self.fresh_int_var();
                self.constrain_either_eq(
                    lhs_ty.clone(),
                    (PartialTy::Float, int_var),
                    hir.expr_span(lhs),
                );
                self.constrain_eq(rhs_ty, lhs_ty.clone(), hir.expr_span(rhs));
                Ok(lhs_ty)
            }
            InfixOp::Exp => {
                let int_var = self.fresh_int_var();
                self.constrain_either_eq(
                    lhs_ty.clone(),
                    (PartialTy::Float, int_var.clone()),
                    hir.expr_span(lhs),
                );
                self.constrain_eq(rhs_ty, int_var, hir.expr_span(rhs));
                Ok(lhs_ty)
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
                let int_var = self.fresh_int_var();
                self.constrain_either_eq(
                    lhs_ty.clone(),
                    (PartialTy::Float, int_var),
                    hir.expr_span(lhs),
                );
                self.constrain_eq(rhs_ty, lhs_ty, hir.expr_span(rhs));
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
                let int_var = self.fresh_int_var();
                self.constrain_either_eq(
                    expr_ty.clone(),
                    (int_var, PartialTy::Float),
                    hir.expr_span(expr),
                );
                Ok(expr_ty)
            }
        }
    }

    fn infer_indexing(&mut self, hir: &Hir, arr: ExprId, idx: ExprId) -> Result<PartialTy> {
        let arr_ty = self.infer_expr(hir, arr)?;
        let inner_ty = self.fresh_var();
        self.constrain_eq(
            arr_ty,
            PartialTy::array(inner_ty.clone()),
            hir.expr_span(arr),
        );

        let idx_ty = self.infer_expr(hir, idx)?;
        self.constrain_eq(idx_ty, PartialTy::UInt, hir.expr_span(idx));

        Ok(inner_ty)
    }

    fn infer_field(&mut self, hir: &Hir, base: ExprId, field: SpanIdent) -> Result<PartialTy> {
        let base_ty = self.infer_expr(hir, base)?;

        let PartialTy::Adt(base_ty, _) = base_ty else {
            return Err(ErrorKind::PrimitiveTypeNoField(base_ty).span(hir.expr_span(base)));
        };
        let AdtInfo::Record { fields, .. } = &hir.adt_info(base_ty) else {
            return Err(ErrorKind::MissingField.span(field.span));
        };

        let field_ty = PartialTy::from(
            &fields
                .get(&field.ident)
                .ok_or_else(|| ErrorKind::MissingField.span(field.span))?
                .ty,
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
                self.constrain_eq(el_ty, th_ty.clone(), hir.expr_span(el))
            }
            None => self.constrain_eq(th_ty.clone(), PartialTy::unit(), hir.expr_span(th)),
        }

        Ok(th_ty)
    }

    fn infer_match(
        &mut self,
        hir: &Hir,
        scrutinee: ExprId,
        arms: &[MatchArm],
    ) -> Result<PartialTy> {
        let scrutinee_ty = self.infer_expr(hir, scrutinee)?;

        let ty = self.fresh_var();
        arms.iter().try_for_each(|arm| {
            self.infer_expr(hir, arm.body)
                .map(|body_ty| self.constrain_eq(body_ty, ty.clone(), hir.expr_span(arm.body)))
        })?;

        Ok(ty)
    }

    fn infer_for(
        &mut self,
        hir: &Hir,
        pat: &Pat,
        iter: ExprId,
        body: &BlockExpr,
    ) -> Result<PartialTy> {
        let iter_ty = self.infer_expr(hir, iter)?;
        let body_ty = self.infer_block_expr(hir, body)?;
        Ok(PartialTy::unit())
    }

    fn infer_loop(&mut self, hir: &Hir, body: &BlockExpr) -> Result<PartialTy> {
        let body_ty = self.infer_block_expr(hir, body)?;
        Ok(PartialTy::unit())
    }

    fn infer_lambda(&mut self, hir: &Hir, params: &[Binding], body: ExprId) -> Result<PartialTy> {
        let param_tys = params
            .iter()
            .map(|p| Param {
                mutable: p.mutable,
                ty: self.convert(p.ty.as_ref()),
            })
            .collect();
        let body_ty = self.infer_expr(hir, body)?;

        Ok(PartialTy::Fn(
            param_tys,
            Return {
                mutable: false,
                ty: Box::new(body_ty.clone()),
            },
        ))
    }

    fn infer_block_expr(&mut self, hir: &Hir, block: &BlockExpr) -> Result<PartialTy> {
        let mut stmt_tys: Vec<_> = block
            .0
            .iter()
            .map(|s| self.infer_stmt(hir, s))
            .try_collect()?;
        Ok(stmt_tys.pop().unwrap_or_else(PartialTy::unit))
    }

    fn infer_stmt(&mut self, hir: &Hir, stmt: &Stmt) -> Result<PartialTy> {
        match stmt {
            Stmt::Decl { binding, val, .. } => {
                let val_ty = self.infer_expr(hir, *val)?;
                let annot_ty = self.convert(binding.ty.as_ref());
                self.constrain_eq(val_ty, annot_ty, hir.expr_span(*val));
                Ok(PartialTy::unit())
            }
            Stmt::Expr(expr) => self.infer_expr(hir, *expr),
        }
    }
}
