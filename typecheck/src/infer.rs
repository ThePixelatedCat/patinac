use crate::Constraint;

use super::{Ty, TypeChecker, TypeError, TypeErrorS};
use ast::{Binding, Bop, Expr, ExprKind, Ident, Unop};
use span::{Span, SpanErr, Spannable, Spnd};

type Env = im::HashMap<Ident, BindingInfo>;

#[derive(Clone)]
struct BindingInfo {
    ty: Ty,
    mutable: bool,
}

impl TypeChecker {
    pub fn infer(&mut self, env: Env, expr: Expr<()>) -> Result<Expr<Ty>, TypeErrorS> {
        let span = expr.span;
        match expr.kind {
            ExprKind::Ident(ident) => self.infer_ident(env, span, ident),
            ExprKind::Int(i) => {
                let ty = if (i as i32) > i32::MAX {
                    Ty::UInt
                } else {
                    let int_var = self.fresh_var();
                    self.constrain_int(int_var.clone());
                    int_var
                };

                Ok(ExprKind::Int(i).span_ty(span, ty))
            }
            ExprKind::Float(f) => Ok(ExprKind::Float(f).span_ty(span, Ty::Float)),
            ExprKind::String(s) => Ok(ExprKind::String(s).span_ty(span, Ty::string())),
            ExprKind::Char(c) => Ok(ExprKind::Char(c).span_ty(span, Ty::Char)),
            ExprKind::Bool(b) => Ok(ExprKind::Bool(b).span_ty(span, Ty::Bool)),
            ExprKind::Array(vals) => self.infer_array(vals),
            ExprKind::Tuple(vals) => self.infer_tuple(env, span, vals),
            ExprKind::App { func, args } => self.infer_app(env, span, *func, args),
            ExprKind::BinOp { op, lhs, rhs } => self.infer_binop(env, span, op, *lhs, *rhs),
            ExprKind::UnaryOp { op, expr } => self.infer_unop(env, span, op, *expr),
            ExprKind::Index { arr, idx } => self.infer_indexing(env, span, *arr, *idx),
            ExprKind::FieldAccess { base, field } => todo!(),
            ExprKind::If { cond, th, el } => self.infer_if(env, span, *cond, *th, el.map(|v| *v)),
            ExprKind::For {
                pattern,
                iter,
                body,
            } => todo!(),
            ExprKind::While { cond, body } => todo!(),
            ExprKind::Match { scrutinee, arms } => todo!(),
            ExprKind::Let { binding, value } => self.infer_let(binding, value),
            ExprKind::Assign { ident, value } => self.infer_assign(ident.as_deref(), value),
            ExprKind::Lambda {
                params,
                return_type,
                body,
            } => self.infer_fun(params, return_type.as_ref(), body),
            ExprKind::Block { exprs, trailing } => self.infer_block(exprs, *trailing),
        }
    }

    fn types_of(
        &mut self,
        env: Env,
        exprs: Vec<Expr<()>>,
    ) -> Result<(Vec<Expr<Ty>>, Vec<Ty>), TypeErrorS> {
        Ok(exprs
            .into_iter()
            .map(|e| self.infer(env.clone(), e))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .unzip())
    }

    fn infer_ident(&self, env: Env, span: Span, ident: Ident) -> Result<Expr<Ty>, TypeErrorS> {
        let info = env.get(&ident).ok_or(TypeError::UnboundIdent.span(span))?;
        Ok(ExprKind::Ident(ident).span_ty(span, info.ty.clone()))
    }

    fn infer_array(&mut self, exprs: Vec<Expr<()>>) -> Result<Expr<Ty>, TypeErrorS> {
        let inner_ty = self.fresh_var();

        for expr in exprs {
            let this_ty = self.infer(expr)?;

            self.unify(&inner_ty, &this_ty).span_err(expr.span)?;
        }

        Ok(Ty::Array(Box::new(inner_ty)))
    }

    fn infer_tuple(
        &mut self,
        env: Env,
        span: Span,
        vals: Vec<Expr<()>>,
    ) -> Result<Expr<Ty>, TypeErrorS> {
        let (vals, tys) = self.types_of(env, vals)?;
        Ok(ExprKind::Tuple(vals).span_ty(span, Ty::Tuple(tys)))
    }

    fn infer_app(
        &mut self,
        env: Env,
        span: Span,
        func: Expr<()>,
        args: Vec<Expr<()>>,
    ) -> Result<Expr<Ty>, TypeErrorS> {
        let (args, arg_tys) = self.types_of(env.clone(), args)?;

        let func = self.infer(env.clone(), func)?;

        let return_ty = self.fresh_var();
        self.constrain_eq(
            func.ty.clone(),
            Ty::Func(arg_tys, Box::new(return_ty.clone())),
        );

        Ok(ExprKind::App {
            func: Box::new(func),
            args,
        }
        .span_ty(span, return_ty))
    }

    fn infer_binop(
        &mut self,
        env: Env,
        span: Span,
        op: Bop,
        lhs: Expr<()>,
        rhs: Expr<()>,
    ) -> Result<Expr<Ty>, TypeErrorS> {
        let lhs = self.infer(env.clone(), lhs)?;
        let rhs = self.infer(env.clone(), rhs)?;

        let ty = match op {
            Bop::Add | Bop::Sub | Bop::Mul | Bop::Div => {
                self.unify_either(&lhs_ty, &self.fresh_int_var(), &Ty::Float)
                    .span_err(lhs.span)?;

                self.unify(&lhs_ty, &rhs_ty).span_err(rhs.span)?;

                Ok(lhs_ty)
            }
            Bop::Exp => {
                self.unify_either(&lhs_ty, &self.fresh_int_var(), &Ty::Float)
                    .span_err(lhs.span)?;

                self.unify(&self.fresh_int_var(), &rhs_ty)
                    .span_err(rhs.span)?;

                Ok(lhs_ty)
            }
            Bop::BOr | Bop::BAnd => {
                let int_var = self.fresh_int_var();

                self.unify(&int_var, &lhs_ty).span_err(lhs.span)?;

                self.unify(&int_var, &rhs_ty).span_err(rhs.span)?;

                Ok(int_var)
            }
            Bop::And | Bop::Or | Bop::Xor => {
                self.constrain_eq(lhs.ty.clone(), Ty::Bool);
                self.constrain_eq(rhs.ty.clone(), Ty::Bool);

                Ty::Bool
            }
            Bop::Eqq | Bop::Neq => {
                self.constrain_eq(lhs.ty.clone(), rhs.ty.clone());

                Ty::Bool
            }
            Bop::Gt | Bop::Lt | Bop::Geq | Bop::Leq => {
                self.unify_either(&lhs_ty, &self.fresh_int_var(), &Ty::Float)
                    .span_err(lhs.span)?;

                self.unify(&lhs_ty, &rhs_ty).span_err(rhs.span)?;

                Ok(Ty::Bool)
            }
        };

        Ok(ExprKind::BinOp {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        }
        .span_ty(span, ty))
    }

    fn infer_unop(
        &mut self,
        env: Env,
        span: Span,
        op: Unop,
        expr: Expr<()>,
    ) -> Result<Expr<Ty>, TypeErrorS> {
        let expr = self.infer(env, expr)?;

        let ty = match op {
            Unop::Not => {
                self.constrain_eq(expr.ty.clone(), Ty::Bool);

                Ty::Bool
            }
            Unop::Neg => {
                self.constrain_either_eq(expr.ty.clone(), (Ty::Int, Ty::Float)); //TODO any int

                expr.ty.clone()
            }
        };

        Ok(ExprKind::UnaryOp {
            op,
            expr: Box::new(expr),
        }
        .span_ty(span, ty))
    }

    fn infer_indexing(
        &mut self,
        env: Env,
        span: Span,
        arr: Expr<()>,
        index: Expr<()>,
    ) -> Result<Expr<Ty>, TypeErrorS> {
        let arr = self.infer(env.clone(), arr)?;

        let inner_ty = self.fresh_var();
        self.constrain_eq(arr.ty.clone(), Ty::Array(Box::new(inner_ty.clone())));

        let idx = self.infer(env.clone(), index)?;
        self.constrain_eq(idx.ty.clone(), Ty::UInt);

        Ok(ExprKind::Index {
            arr: Box::new(arr),
            idx: Box::new(idx),
        }
        .span_ty(span, inner_ty))
    }

    fn infer_if(
        &mut self,
        env: Env,
        span: Span,
        cond: Expr<()>,
        th: Expr<()>,
        el: Option<Expr<()>>,
    ) -> Result<Expr<Ty>, TypeErrorS> {
        let cond = self.infer(env.clone(), cond)?;
        self.constrain_eq(cond.ty.clone(), Ty::Bool);

        let th = self.infer(env.clone(), th)?;

        let el = el
            .map(|el| self.infer(env.clone(), el).map(Box::new))
            .transpose()?;
        self.constrain_eq(
            th.ty.clone(),
            el.as_ref().map_or_else(Ty::unit, |el| el.ty.clone()),
        );

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
        env: Env,
        span: Span,
        binding: Binding,
        val: Expr<()>,
    ) -> Result<Expr<Ty>, TypeErrorS> {
        let Pattern::Var {
            mutable,
            ident,
            annotated_ty,
        } = &binding.inner;

        let val_ty = self.infer(val)?;

        if let Some(ty) = annotated_ty {
            let annot_ty = ty.inner.clone().into();
            self.unify(&annot_ty, &val_ty).span_err(val.span)?;
        }

        self.env.insert(
            ident.to_owned(),
            BindingInfo {
                ty: val_ty,
                mutable: *mutable,
            },
        );

        Ok(Ty::unit())
    }

    fn infer_assign(&mut self, ident: Spnd<&str>, val: &Expr) -> Result<Expr<Ty>, TypeErrorS> {
        let val_ty = self.infer(val)?;

        let info = self.get_binding(ident.inner).span_err(ident.span)?;

        if !info.mutable {
            return Err(TypeError::Mutation(ident.inner.to_owned()).span(ident.span));
        }

        self.unify(&info.ty, &val_ty).span_err(val.span)?;

        Ok(Ty::unit())
    }

    pub(super) fn infer_fun(
        &self,
        params: &[PatternS],
        return_ty: Option<&AstTypeS>,
        body: &Expr,
    ) -> Result<Expr<Ty>, TypeErrorS> {
        let mut snapshot = self.clone();

        let mut param_tys = Vec::new();
        for param in params {
            let Pattern::Var {
                mutable,
                ident,
                annotated_ty,
            } = &param.inner;

            let param_ty = annotated_ty
                .as_ref()
                .map_or_else(|| snapshot.fresh_var(), |ty| ty.inner.clone().into());

            param_tys.push(param_ty.clone());

            let binding = BindingInfo {
                ty: param_ty,
                mutable: *mutable,
            };

            snapshot.env.insert(ident.to_owned(), binding);
        }

        let body_ty = snapshot.infer(body)?;

        if let Some(ty) = return_ty {
            let return_ty = ty.inner.clone().into();
            snapshot.unify(&return_ty, &body_ty).span_err(body.span)?;
        }

        Ok(Ty::Func(param_tys, Box::new(body_ty)))
    }

    fn infer_block(&self, exprs: &[Expr], trailing: bool) -> Result<Expr<Ty>, TypeErrorS> {
        let mut snapshot = self.clone();

        let mut last = Option::None;
        for expr in exprs {
            last = Some(snapshot.infer(expr)?);
        }

        Ok(if trailing && let Some(ty) = last {
            ty
        } else {
            Ty::unit()
        })
    }
}
