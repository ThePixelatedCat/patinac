use ast::{
    exprs::{Arg, Binding, Expr, ExprKind, InfixOp, LitExpr, UnaryOp},
    patterns::Pat,
    types::Ty as AstTy,
};
use ident::Ident;
use itertools::Itertools;
use span::Span;

use crate::{
    ErrorKind, Result, Ty, TypeChecker,
    env::{BindingInfo, Ctx, TyEnv},
    types::Param,
};

impl TypeChecker {
    #[expect(unused)]
    pub(super) fn infer(
        &mut self,
        ty_env: &TyEnv,
        ctx: &mut Ctx,
        expr: Expr<()>,
    ) -> Result<Expr<Ty>> {
        let span = expr.span;
        match expr.kind {
            ExprKind::Ident(ident) => Self::infer_ident(ctx, span, ident),
            ExprKind::Lit(lit) => self.infer_lit(span, lit),
            ExprKind::Array(exprs) => self.infer_array(ty_env, ctx, span, exprs),
            ExprKind::Tuple(exprs) => self.infer_tuple(ty_env, ctx, span, exprs),
            ExprKind::CallExpr { func, args } => self.infer_call(ty_env, ctx, span, *func, args),
            ExprKind::InfixExpr { op, lhs, rhs } => {
                self.infer_binop(ty_env, ctx, span, op, *lhs, *rhs)
            }
            ExprKind::UnaryExpr { op, expr } => self.infer_unop(ty_env, ctx, span, op, *expr),
            ExprKind::IndexExpr { arr, idx } => self.infer_indexing(ty_env, ctx, span, *arr, *idx),
            ExprKind::FieldExpr { base, field } => todo!(),
            ExprKind::Let { binding, val } => self.infer_let(ty_env, ctx, span, binding, *val),
            ExprKind::LambdaExpr {
                params,
                return_ty,
                body,
            } => self.infer_lambda(ty_env, ctx, span, params, return_ty, *body),
            ExprKind::If { cond, th, el } => {
                self.infer_if(ty_env, ctx, span, *cond, *th, el.map(|v| *v))
            }
            ExprKind::Match { scrutinee, arms } => todo!(),
            ExprKind::For {
                pattern,
                iter,
                body,
            } => todo!(),
            ExprKind::While { cond, body } => todo!(),
            ExprKind::Break => todo!(),
            ExprKind::Continue => todo!(),
            ExprKind::Return(expr) => todo!(),
            ExprKind::Block(exprs) => self.infer_block(ty_env, ctx, span, exprs),
        }
    }

    fn infer_multi(
        &mut self,
        ty_env: &TyEnv,
        ctx: &mut Ctx,
        exprs: Vec<Expr<()>>,
    ) -> Result<Vec<Expr<Ty>>> {
        exprs
            .into_iter()
            .map(|e| self.infer(ty_env, ctx, e))
            .collect()
    }

    fn types_of(
        &mut self,
        ty_env: &TyEnv,
        ctx: &mut Ctx,
        exprs: Vec<Expr<()>>,
    ) -> Result<(Vec<Expr<Ty>>, Vec<Ty>)> {
        exprs
            .into_iter()
            .map(|e| {
                let e = self.infer(ty_env, ctx, e)?;
                let ty = e.ty.clone();
                Ok((e, ty))
            })
            .collect()
    }

    fn infer_ident(ctx: &Ctx, span: Span, ident: Ident) -> Result<Expr<Ty>> {
        ctx.get(ident, span)
            .map(|info| ExprKind::Ident(ident).span_ty(span, info.ty))
    }

    fn check_place_mut(&mut self, ty_env: &TyEnv, ctx: &mut Ctx, place: &Expr<Ty>) -> Result<()> {
        let span = place.span;
        match place.kind {
            ExprKind::Ident(ident) => ctx.get(ident, span).and_then(|info| {
                info.mutable
                    .then_some(())
                    .ok_or_else(|| ErrorKind::Mutation.span(span))
            }),
            ExprKind::FieldExpr { ref base, .. } | ExprKind::IndexExpr { arr: ref base, .. } => {
                self.check_place_mut(ty_env, ctx, &base)
            }
            _ => Err(ErrorKind::NotPlaceExpr.span(span)),
        }
    }

    fn check_places_unique(
        &mut self,
        ty_env: &TyEnv,
        ctx: &mut Ctx,
        place_a: &Expr<Ty>,
        place_b: &Expr<Ty>,
    ) -> Result<()> {
        match &place_b.kind {
            ExprKind::Ident(_) => (place_a == place_b)
                .then_some(())
                .ok_or_else(|| ErrorKind::OverlappingPlace(place_a.span).span(place_b.span)),
            ExprKind::FieldExpr { base, .. } | ExprKind::IndexExpr { arr: base, .. } => {
                self.check_places_unique(ty_env, ctx, place_a, base)
            }
            _ => Err(ErrorKind::NotPlaceExpr.span(place_b.span)),
        }
    }

    fn infer_lit(&mut self, span: Span, lit: LitExpr) -> Result<Expr<Ty>> {
        let ty = match lit {
            LitExpr::Int(_) => self.fresh_int_var(),
            LitExpr::Float(_) => Ty::Float,
            LitExpr::String(_) => Ty::string(),
            LitExpr::Char(_) => Ty::Char,
            LitExpr::Bool(_) => Ty::Bool,
        };

        Ok(ExprKind::Lit(lit).span_ty(span, ty))
    }

    fn infer_array(
        &mut self,
        ty_env: &TyEnv,
        ctx: &mut Ctx,
        span: Span,
        exprs: Vec<Expr<()>>,
    ) -> Result<Expr<Ty>> {
        let exprs = self.infer_multi(ty_env, ctx, exprs)?;

        let inner_ty = self.fresh_var();

        for expr in &exprs {
            self.constrain_eq(&expr, inner_ty.clone());
        }

        Ok(ExprKind::Array(exprs).span_ty(span, Ty::Array(Box::new(inner_ty))))
    }

    fn infer_tuple(
        &mut self,
        ty_env: &TyEnv,
        ctx: &mut Ctx,
        span: Span,
        exprs: Vec<Expr<()>>,
    ) -> Result<Expr<Ty>> {
        let (exprs, tys) = self.types_of(ty_env, ctx, exprs)?;
        Ok(ExprKind::Tuple(exprs).span_ty(span, Ty::Tuple(tys)))
    }

    fn infer_call(
        &mut self,
        ty_env: &TyEnv,
        ctx: &mut Ctx,
        span: Span,
        func: Expr<()>,
        args: Vec<Arg<()>>,
    ) -> Result<Expr<Ty>> {
        let func = self.infer(ty_env, ctx, func)?;

        let (args, arg_tys) = args
            .into_iter()
            .map(|arg| {
                let val = self.infer(ty_env, ctx, arg.val)?;

                if arg.mutable {
                    self.check_place_mut(ty_env, ctx, &val)?;
                }

                let ty = val.ty.clone();
                Ok((
                    Arg {
                        val,
                        mutable: arg.mutable,
                        label: arg.label,
                    },
                    Param {
                        mutable: arg.mutable,
                        ty,
                    },
                ))
            })
            .collect::<Result<(Vec<_>, Vec<_>)>>()?;

        for vec in (0..args.len()).permutations(2) {
            let [i, j] = vec[..] else { unreachable!() };
            let (a, b) = (&args[i], &args[j]);

            if a.mutable || b.mutable {
                self.check_places_unique(ty_env, ctx, &a.val, &b.val)?;
            }
        } // TODO optimise???

        let return_ty = self.fresh_var();

        self.constrain_eq(&func, Ty::Func(arg_tys, Box::new(return_ty.clone())));

        Ok(ExprKind::CallExpr {
            func: Box::new(func),
            args,
        }
        .span_ty(span, return_ty))
    }

    fn infer_binop(
        &mut self,
        ty_env: &TyEnv,
        ctx: &mut Ctx,
        span: Span,
        op: InfixOp,
        lhs: Expr<()>,
        rhs: Expr<()>,
    ) -> Result<Expr<Ty>> {
        let lhs = self.infer(ty_env, ctx, lhs)?;
        let rhs = self.infer(ty_env, ctx, rhs)?;

        let ty = match op {
            InfixOp::Assign => {
                self.check_place_mut(ty_env, ctx, &lhs)?;
                self.constrain_eq(&rhs, lhs.ty.clone());

                Ty::unit()
            }
            InfixOp::Add | InfixOp::Sub | InfixOp::Mul | InfixOp::Div | InfixOp::Rem => {
                let int_var = self.fresh_int_var();
                self.constrain_either_eq(lhs.ty.clone(), (Ty::Float, int_var), lhs.span);
                self.constrain_eq(&rhs, lhs.ty.clone());

                lhs.ty.clone()
            }
            InfixOp::Exp => {
                let int_var = self.fresh_int_var();
                self.constrain_either_eq(lhs.ty.clone(), (Ty::Float, int_var.clone()), lhs.span);
                self.constrain_eq(&rhs, int_var);

                lhs.ty.clone()
            }
            InfixOp::And | InfixOp::Or | InfixOp::Xor => {
                self.constrain_eq(&lhs, Ty::Bool);
                self.constrain_eq(&rhs, Ty::Bool);

                Ty::Bool
            }
            InfixOp::Eqq | InfixOp::Neq => {
                self.constrain_eq(&rhs, lhs.ty.clone());

                Ty::Bool
            }
            InfixOp::Gt | InfixOp::Lt | InfixOp::Geq | InfixOp::Leq => {
                let int_var = self.fresh_int_var();
                self.constrain_either_eq(lhs.ty.clone(), (Ty::Float, int_var), lhs.span);
                self.constrain_eq(&rhs, lhs.ty.clone());

                Ty::Bool
            }
        };

        Ok(ExprKind::InfixExpr {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        }
        .span_ty(span, ty))
    }

    fn infer_unop(
        &mut self,
        ty_env: &TyEnv,
        ctx: &mut Ctx,
        span: Span,
        op: UnaryOp,
        expr: Expr<()>,
    ) -> Result<Expr<Ty>> {
        let expr = self.infer(ty_env, ctx, expr)?;

        let ty = match op {
            UnaryOp::Not => {
                self.constrain_eq(&expr, Ty::Bool);

                Ty::Bool
            }
            UnaryOp::Neg => {
                let int_var = self.fresh_int_var();
                self.constrain_either_eq(expr.ty.clone(), (int_var, Ty::Float), expr.span);

                expr.ty.clone()
            }
        };

        Ok(ExprKind::UnaryExpr {
            op,
            expr: Box::new(expr),
        }
        .span_ty(span, ty))
    }

    fn infer_field(
        &mut self,
        ty_env: &TyEnv,
        ctx: &mut Ctx,
        span: Span,
        base: Expr<()>,
        field: Ident,
    ) -> Result<Expr<Ty>> {
        let base = self.infer(ty_env, ctx, base)?;

        let Ty::Adt(base_ty, _) = base.ty else {
            return Err(ErrorKind::PrimitiveTypeNoField(base.ty).span(span));
        };

        let field_ty = ty_env.get_field(base_ty, field, span)?;

        Ok(ExprKind::FieldExpr {
            base: Box::new(base),
            field,
        }
        .span_ty(span, field_ty))
    }

    fn infer_indexing(
        &mut self,
        ty_env: &TyEnv,
        ctx: &mut Ctx,
        span: Span,
        arr: Expr<()>,
        idx: Expr<()>,
    ) -> Result<Expr<Ty>> {
        let arr = self.infer(ty_env, ctx, arr)?;

        let inner_ty = self.fresh_var();
        self.constrain_eq(&arr, Ty::Array(Box::new(inner_ty.clone())));

        let idx = self.infer(ty_env, ctx, idx)?;
        self.constrain_eq(&idx, Ty::UInt);

        Ok(ExprKind::IndexExpr {
            arr: Box::new(arr),
            idx: Box::new(idx),
        }
        .span_ty(span, inner_ty))
    }

    fn infer_if(
        &mut self,
        ty_env: &TyEnv,
        ctx: &mut Ctx,
        span: Span,
        cond: Expr<()>,
        th: Expr<()>,
        el: Option<Expr<()>>,
    ) -> Result<Expr<Ty>> {
        let cond = self.infer(ty_env, ctx, cond)?;
        self.constrain_eq(&cond, Ty::Bool);

        let th = self.infer(ty_env, ctx, th)?;

        let el = el
            .map(|el| self.infer(ty_env, ctx, el))
            .transpose()?
            .map(Box::new);
        match &el {
            Some(el) => self.constrain_eq(&el, th.ty.clone()),
            None => self.constrain_eq(&th, Ty::unit()),
        }

        let th_ty = th.ty.clone();
        Ok(ExprKind::If {
            cond: Box::new(cond),
            th: Box::new(th),
            el,
        }
        .span_ty(span, th_ty))
    }

    fn infer_let(
        &mut self,
        ty_env: &TyEnv,
        ctx: &mut Ctx,
        span: Span,
        binding: Binding,
        val: Expr<()>,
    ) -> Result<Expr<Ty>> {
        let val = self.infer(ty_env, ctx, val)?;

        if let Some(ty) = &binding.ty {
            self.constrain_eq(&val, ty.into());
        }

        match &binding.pat {
            Pat::Ident { ident, subpat } => {
                ctx.insert(*ident, val.ty.clone(), binding.mutable);
            }
            Pat::Wildcard => {}
            _ => {
                todo!("tuple patterns are unimplemented")
            }
        }

        let ty = val.ty.clone();
        Ok(ExprKind::Let {
            binding,
            val: Box::new(val),
        }
        .span_ty(span, ty))
    }

    pub(super) fn infer_lambda(
        &mut self,
        ty_env: &TyEnv,
        ctx: &Ctx,
        span: Span,
        params: Vec<Binding>,
        return_ty: Option<AstTy>,
        body: Expr<()>,
    ) -> Result<Expr<Ty>> {
        let mut ctx: Ctx = ctx
            .clone()
            .into_iter()
            .map(|(ident, info)| {
                (
                    ident,
                    BindingInfo {
                        mutable: false,
                        ..info
                    },
                )
            })
            .collect();

        let mut param_tys = Vec::new();
        for param in &params {
            let param_ty = self.convert(param.ty.as_ref());

            match &param.pat {
                Pat::Ident { ident, subpat } => {
                    ctx.insert(*ident, param_ty.clone(), param.mutable);
                }
                Pat::Wildcard => {}
                _ => todo!("tuple patterns are unimplemented"),
            }

            param_tys.push(Param {
                mutable: param.mutable,
                ty: param_ty,
            });
        }

        let body = self.infer(ty_env, &mut ctx, body)?;

        if let Some(return_ty) = &return_ty {
            self.constrain_eq(&body, return_ty.into());
        }

        let body_ty = Box::new(body.ty.clone());

        Ok(ExprKind::LambdaExpr {
            params,
            return_ty,
            body: Box::new(body),
        }
        .span_ty(span, Ty::Func(param_tys, body_ty)))
    }

    fn infer_block(
        &mut self,
        ty_env: &TyEnv,
        ctx: &Ctx,
        span: Span,
        exprs: Vec<Expr<()>>,
    ) -> Result<Expr<Ty>> {
        let exprs = self.infer_multi(ty_env, &mut ctx.clone(), exprs)?;

        let ty = exprs.last().map_or_else(Ty::unit, |e| e.ty.clone());

        Ok(ExprKind::Block(exprs).span_ty(span, ty))
    }
}
