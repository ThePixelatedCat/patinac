use ast::{Binding, Expr, ExprKind, Ident, InfixOp, Pat, UnaryOp};
use span::{Span, Spannable};

use super::{BindingInfo, Ty, TypeChecker, TypeError, TypeErrorS};

type Env = im::HashMap<Ident, BindingInfo>;

impl TypeChecker<'_> {
    #[expect(unused)]
    pub(super) fn infer(&mut self, env: &mut Env, expr: Expr<()>) -> Result<Expr<Ty>, TypeErrorS> {
        let span = expr.span;
        match expr.kind {
            ExprKind::Ident(ident) => self.infer_ident(env, span, ident),
            ExprKind::Int(i) => Ok(ExprKind::Int(i).span_ty(span, self.fresh_int_var())),
            ExprKind::Float(f) => Ok(ExprKind::Float(f).span_ty(span, Ty::Float)),
            ExprKind::String(s) => Ok(ExprKind::String(s).span_ty(span, Ty::string(self.interner))),
            ExprKind::Char(c) => Ok(ExprKind::Char(c).span_ty(span, Ty::Char)),
            ExprKind::Bool(b) => Ok(ExprKind::Bool(b).span_ty(span, Ty::Bool)),
            ExprKind::Array(exprs) => self.infer_array(env, span, exprs),
            ExprKind::Tuple(exprs) => self.infer_tuple(env, span, exprs),
            ExprKind::CallExpr { func, args } => self.infer_app(env, span, *func, args),
            ExprKind::InfixExpr { op, lhs, rhs } => self.infer_binop(env, span, op, *lhs, *rhs),
            ExprKind::UnaryExpr { op, expr } => self.infer_unop(env, span, op, *expr),
            ExprKind::IndexExpr { arr, idx } => self.infer_indexing(env, span, *arr, *idx),
            ExprKind::FieldExpr { base, field } => todo!(),
            ExprKind::If { cond, th, el } => self.infer_if(env, span, *cond, *th, el.map(|v| *v)),
            ExprKind::For {
                pattern,
                iter,
                body,
            } => todo!(),
            ExprKind::While { cond, body } => todo!(),
            ExprKind::Match { scrutinee, arms } => todo!(),
            ExprKind::Let { binding, val } => self.infer_let(env, span, binding, *val),
            ExprKind::Assign { ident, val } => self.infer_assign(env, span, ident, *val),
            ExprKind::LambdaExpr {
                params,
                return_ty,
                body,
            } => self.infer_lambda(env, span, params, return_ty, *body),
            ExprKind::Block(exprs) => self.infer_block(env, span, exprs),
        }
    }

    fn infer_multi(
        &mut self,
        env: &mut Env,
        exprs: Vec<Expr<()>>,
    ) -> Result<Vec<Expr<Ty>>, TypeErrorS> {
        exprs.into_iter().map(|e| self.infer(env, e)).collect()
    }

    fn types_of(
        &mut self,
        env: &mut Env,
        exprs: Vec<Expr<()>>,
    ) -> Result<(Vec<Expr<Ty>>, Vec<Ty>), TypeErrorS> {
        Ok(self
            .infer_multi(env, exprs)?
            .into_iter()
            .map(|e| {
                let ty = e.ty.clone();
                (e, ty)
            })
            .unzip())
    }

    fn infer_ident(&self, env: &Env, span: Span, ident: Ident) -> Result<Expr<Ty>, TypeErrorS> {
        let info = env
            .get(&ident)
            .ok_or_else(|| TypeError::UnboundIdent.span(span))?;
        Ok(ExprKind::Ident(ident).span_ty(span, info.ty.clone()))
    }

    fn infer_array(
        &mut self,
        env: &mut Env,
        span: Span,
        exprs: Vec<Expr<()>>,
    ) -> Result<Expr<Ty>, TypeErrorS> {
        let exprs = self.infer_multi(env, exprs)?;

        let inner_ty = self.fresh_var();

        for expr in &exprs {
            self.constrain_eq(expr.ty.clone(), inner_ty.clone(), expr.span);
        }

        Ok(ExprKind::Array(exprs).span_ty(span, Ty::Array(Box::new(inner_ty))))
    }

    fn infer_tuple(
        &mut self,
        env: &mut Env,
        span: Span,
        exprs: Vec<Expr<()>>,
    ) -> Result<Expr<Ty>, TypeErrorS> {
        let (exprs, tys) = self.types_of(env, exprs)?;
        Ok(ExprKind::Tuple(exprs).span_ty(span, Ty::Tuple(tys)))
    }

    fn infer_app(
        &mut self,
        env: &mut Env,
        span: Span,
        func: Expr<()>,
        args: Vec<Expr<()>>,
    ) -> Result<Expr<Ty>, TypeErrorS> {
        let (args, arg_tys) = self.types_of(env, args)?;

        let func = self.infer(env, func)?;

        let return_ty = self.fresh_var();
        self.constrain_eq(
            func.ty.clone(),
            Ty::Func(arg_tys, Box::new(return_ty.clone())),
            func.span,
        );

        Ok(ExprKind::CallExpr {
            func: Box::new(func),
            args,
        }
        .span_ty(span, return_ty))
    }

    fn infer_binop(
        &mut self,
        env: &mut Env,
        span: Span,
        op: InfixOp,
        lhs: Expr<()>,
        rhs: Expr<()>,
    ) -> Result<Expr<Ty>, TypeErrorS> {
        let lhs = self.infer(env, lhs)?;
        let rhs = self.infer(env, rhs)?;

        let ty = match op {
            InfixOp::Add | InfixOp::Sub | InfixOp::Mul | InfixOp::Div => {
                let int_var = self.fresh_int_var();
                self.constrain_either_eq(lhs.ty.clone(), (Ty::Float, int_var), lhs.span);
                self.constrain_eq(rhs.ty.clone(), lhs.ty.clone(), rhs.span);

                lhs.ty.clone()
            }
            InfixOp::Exp => {
                let int_var = self.fresh_int_var();
                self.constrain_either_eq(lhs.ty.clone(), (Ty::Float, int_var.clone()), lhs.span);
                self.constrain_eq(rhs.ty.clone(), int_var, rhs.span);

                lhs.ty.clone()
            }
            InfixOp::BOr | InfixOp::BAnd => {
                let int_var = self.fresh_int_var();

                self.constrain_eq(lhs.ty.clone(), int_var.clone(), lhs.span);
                self.constrain_eq(rhs.ty.clone(), int_var.clone(), rhs.span);

                int_var
            }
            InfixOp::And | InfixOp::Or | InfixOp::Xor => {
                self.constrain_eq(lhs.ty.clone(), Ty::Bool, lhs.span);
                self.constrain_eq(rhs.ty.clone(), Ty::Bool, rhs.span);

                Ty::Bool
            }
            InfixOp::Eqq | InfixOp::Neq => {
                self.constrain_eq(lhs.ty.clone(), rhs.ty.clone(), rhs.span);

                Ty::Bool
            }
            InfixOp::Gt | InfixOp::Lt | InfixOp::Geq | InfixOp::Leq => {
                let int_var = self.fresh_int_var();
                self.constrain_either_eq(lhs.ty.clone(), (Ty::Float, int_var), lhs.span);
                self.constrain_eq(rhs.ty.clone(), lhs.ty.clone(), rhs.span);

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
        env: &mut Env,
        span: Span,
        op: UnaryOp,
        expr: Expr<()>,
    ) -> Result<Expr<Ty>, TypeErrorS> {
        let expr = self.infer(env, expr)?;

        let ty = match op {
            UnaryOp::Not => {
                self.constrain_eq(expr.ty.clone(), Ty::Bool, expr.span);

                Ty::Bool
            }
            UnaryOp::Neg => {
                self.constrain_either_eq(expr.ty.clone(), (Ty::Int, Ty::Float), expr.span); //TODO any int

                expr.ty.clone()
            }
        };

        Ok(ExprKind::UnaryExpr {
            op,
            expr: Box::new(expr),
        }
        .span_ty(span, ty))
    }

    fn infer_indexing(
        &mut self,
        env: &mut Env,
        span: Span,
        arr: Expr<()>,
        index: Expr<()>,
    ) -> Result<Expr<Ty>, TypeErrorS> {
        let arr = self.infer(env, arr)?;

        let inner_ty = self.fresh_var();
        self.constrain_eq(
            arr.ty.clone(),
            Ty::Array(Box::new(inner_ty.clone())),
            arr.span,
        );

        let idx = self.infer(env, index)?;
        self.constrain_eq(idx.ty.clone(), Ty::UInt, idx.span);

        Ok(ExprKind::IndexExpr {
            arr: Box::new(arr),
            idx: Box::new(idx),
        }
        .span_ty(span, inner_ty))
    }

    fn infer_if(
        &mut self,
        env: &mut Env,
        span: Span,
        cond: Expr<()>,
        th: Expr<()>,
        el: Option<Expr<()>>,
    ) -> Result<Expr<Ty>, TypeErrorS> {
        let cond = self.infer(env, cond)?;
        self.constrain_eq(cond.ty.clone(), Ty::Bool, cond.span);

        let th = self.infer(env, th)?;

        let el = el.map(|el| self.infer(env, el)).transpose()?.map(Box::new);
        match &el {
            Some(el) => {
                self.constrain_eq(el.ty.clone(), th.ty.clone(), el.span);
            }
            None => {
                self.constrain_eq(th.ty.clone(), Ty::unit(), th.span);
            }
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
        env: &mut Env,
        span: Span,
        binding: Binding,
        val: Expr<()>,
    ) -> Result<Expr<Ty>, TypeErrorS> {
        let val = self.infer(env, val)?;

        if let Some(ty) = &binding.ty {
            self.constrain_eq(val.ty.clone(), ty.clone().into(), val.span);
        }

        match &binding.pat {
            Pat::Tuple(_) => {
                todo!("tuple patterns are unimplemented")
            }
            Pat::Ident { mutable, ident } => {
                env.insert(*ident, BindingInfo::new(val.ty.clone(), *mutable));
            }
            Pat::Discard => {}
        }

        let ty = val.ty.clone();
        Ok(ExprKind::Let {
            binding,
            val: Box::new(val),
        }
        .span_ty(span, ty))
    }

    fn infer_assign(
        &mut self,
        env: &mut Env,
        span: Span,
        ident: Ident,
        val: Expr<()>,
    ) -> Result<Expr<Ty>, TypeErrorS> {
        let val = self.infer(env, val)?;

        let info = env
            .get(&ident)
            .ok_or_else(|| TypeError::UnboundIdent.span(span))?;

        if !info.mutable {
            return Err(TypeError::Mutation.span(span));
        }

        self.constrain_eq(val.ty.clone(), info.ty.clone(), val.span);

        Ok(ExprKind::Assign {
            ident,
            val: Box::new(val),
        }
        .span_ty(span, Ty::unit()))
    }

    pub(super) fn infer_lambda(
        &mut self,
        env: &Env,
        span: Span,
        params: Vec<Binding>,
        return_ty: Option<ast::Ty>,
        body: Expr<()>,
    ) -> Result<Expr<Ty>, TypeErrorS> {
        let mut env = env.clone();

        let mut param_tys = Vec::new();
        for param in &params {
            let param_ty = param
                .ty
                .as_ref()
                .map_or_else(|| self.fresh_var(), |ty| ty.clone().into());

            match param.pat {
                Pat::Tuple(_) => {
                    todo!("tuple patterns are unimplemented")
                }
                Pat::Ident { mutable, ident } => {
                    env.insert(ident, BindingInfo::new(param_ty.clone(), mutable));
                }
                Pat::Discard => {}
            }

            param_tys.push(param_ty);
        }

        let body = self.infer(&mut env, body)?;

        if let Some(ty) = &return_ty {
            self.constrain_eq(body.ty.clone(), ty.clone().into(), body.span);
        }

        let body_ty = body.ty.clone();

        Ok(ExprKind::LambdaExpr {
            params,
            return_ty,
            body: Box::new(body),
        }
        .span_ty(span, Ty::Func(param_tys, Box::new(body_ty))))
    }

    fn infer_block(
        &mut self,
        env: &Env,
        span: Span,
        exprs: Vec<Expr<()>>,
    ) -> Result<Expr<Ty>, TypeErrorS> {
        let exprs = self.infer_multi(&mut env.clone(), exprs)?;

        let ty = exprs.last().map_or_else(Ty::unit, |e| e.ty.clone());

        Ok(ExprKind::Block(exprs).span_ty(span, ty))
    }
}
