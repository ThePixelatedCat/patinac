use super::{BindingInfo, Type, TypeChecker, TypeError, TypeErrorS};
use crate::helpers::Spanned;
use crate::parser::ast::{Binding, BindingS, Bop, Expr, ExprS, TypeS as AstTypeS, Unop};

impl TypeChecker {
    pub fn type_of(&mut self, expr: &ExprS) -> Result<Type, TypeErrorS> {
        match &expr.inner {
            Expr::Ident(ident) => self.type_of_ident(Spanned {
                inner: ident,
                span: expr.span,
            }),
            Expr::Int(v) => Ok(if *v > i64::MAX as u64 {
                Type::UInt
            } else {
                self.fresh_int_var()
            }),
            Expr::Float(_) => Ok(Type::Float),
            Expr::String(_) => Ok(Type::string()),
            Expr::Char(_) => Ok(Type::Char),
            Expr::Bool(_) => Ok(Type::Bool),
            Expr::Array(vals) => self.type_of_array(vals),
            Expr::Tuple(vals) => self.type_of_tuple(vals),
            Expr::FnCall { fun, args } => self.type_of_fn_call(fun, args),
            Expr::BinaryOp { op, lhs, rhs } => self.type_of_binary_op(*op, lhs, rhs),
            Expr::UnaryOp { op, expr } => self.type_of_unary_op(*op, expr),
            Expr::Index { arr, index } => self.type_of_indexing(arr, index),
            Expr::FieldAccess { base, field } => todo!(),
            Expr::If { cond, th, el } => self.type_of_if(cond, th, el.as_deref()),
            Expr::Let { binding, value } => self.type_of_let(binding, value),
            Expr::Assign { ident, value } => self.type_of_assign(ident.as_deref(), value),
            Expr::Lambda {
                params,
                return_type,
                body,
            } => self.type_of_lambda(params, return_type.as_ref(), body),
            Expr::Block { exprs, trailing } => self.type_of_block(exprs, *trailing),
        }
    }

    fn type_of_ident(&self, ident: Spanned<&str>) -> Result<Type, TypeErrorS> {
        self.get_binding(ident).map(|b| b.ty.clone())
    }

    fn type_of_array(&mut self, exprs: &[ExprS]) -> Result<Type, TypeErrorS> {
        let inner_ty = self.fresh_var();

        for expr in exprs {
            let this_ty = self.type_of(expr)?;

            self.unify(&inner_ty, &this_ty)
                .map_err(|e| e.spanned(expr.span))?;
        }

        Ok(Type::Array(Box::new(inner_ty)))
    }

    fn type_of_tuple(&mut self, vals: &[ExprS]) -> Result<Type, TypeErrorS> {
        vals.iter()
            .map(|e| self.type_of(e))
            .collect::<Result<_, _>>()
            .map(Type::Tuple)
    }

    fn type_of_fn_call(&mut self, fun: &ExprS, args: &[ExprS]) -> Result<Type, TypeErrorS> {
        let fun_ty = self.type_of(fun)?;

        let arg_tys = args
            .iter()
            .map(|arg| self.type_of(arg))
            .collect::<Result<_, _>>()?;
        let return_ty = self.fresh_var();
        let fun_var = Type::Fn(arg_tys, Box::new(return_ty.clone()));

        self.unify(&fun_var, &fun_ty)
            .map_err(|e| e.spanned(fun.span))?;

        Ok(return_ty)
    }

    fn type_of_binary_op(&mut self, op: Bop, lhs: &ExprS, rhs: &ExprS) -> Result<Type, TypeErrorS> {
        let (lhs_ty, rhs_ty) = (self.type_of(lhs)?, self.type_of(rhs)?);

        match op {
            Bop::Add | Bop::Sub | Bop::Mul | Bop::Div => {
                self.unify_either(&lhs_ty, &self.fresh_int_var(), &Type::Float)
                    .map_err(|e| e.spanned(lhs.span))?;

                self.unify(&lhs_ty, &rhs_ty)
                    .map_err(|e| e.spanned(rhs.span))?;

                Ok(lhs_ty)
            }
            Bop::Exp => {
                self.unify_either(&lhs_ty, &self.fresh_int_var(), &Type::Float)
                    .map_err(|e| e.spanned(lhs.span))?;

                self.unify(&self.fresh_int_var(), &rhs_ty)
                    .map_err(|e| e.spanned(rhs.span))?;

                Ok(lhs_ty)
            }
            Bop::BOr | Bop::BAnd => {
                let int_var = self.fresh_int_var();

                self.unify(&int_var, &lhs_ty)
                    .map_err(|e| e.spanned(lhs.span))?;

                self.unify(&int_var, &rhs_ty)
                    .map_err(|e| e.spanned(rhs.span))?;

                Ok(int_var)
            }
            Bop::And | Bop::Or | Bop::Xor => {
                self.unify(&Type::Bool, &lhs_ty)
                    .map_err(|e| e.spanned(lhs.span))?;

                self.unify(&Type::Bool, &rhs_ty)
                    .map_err(|e| e.spanned(rhs.span))?;

                Ok(Type::Bool)
            }
            Bop::Eqq | Bop::Neq => {
                self.unify(&lhs_ty, &rhs_ty)
                    .map_err(|e| e.spanned(rhs.span))?;

                Ok(Type::Bool)
            }
            Bop::Gt | Bop::Lt | Bop::Geq | Bop::Leq => {
                self.unify_either(&lhs_ty, &self.fresh_int_var(), &Type::Float)
                    .map_err(|e| e.spanned(lhs.span))?;

                self.unify(&lhs_ty, &rhs_ty)
                    .map_err(|e| e.spanned(rhs.span))?;

                Ok(Type::Bool)
            }
        }
    }

    fn type_of_unary_op(&mut self, op: Unop, expr: &ExprS) -> Result<Type, TypeErrorS> {
        let expr_ty = self.type_of(expr)?;

        match op {
            Unop::Not => {
                self.unify(&Type::Bool, &expr_ty)
                    .map_err(|e| e.spanned(expr.span))?;

                Ok(expr_ty)
            }
            Unop::Neg => {
                self.unify_either(&expr_ty, &Type::Int, &Type::Float)
                    .map_err(|e| e.spanned(expr.span))?;

                Ok(expr_ty)
            }
        }
    }

    fn type_of_indexing(&mut self, arr: &ExprS, index: &ExprS) -> Result<Type, TypeErrorS> {
        let index_ty = self.type_of(index)?;
        self.unify(&Type::UInt, &index_ty)
            .map_err(|e| e.spanned(index.span))?;

        let arr_ty = Type::Array(Box::new(self.fresh_var()));
        let expr_ty = self.type_of(arr)?;
        self.unify(&arr_ty, &expr_ty)
            .map_err(|e| e.spanned(arr.span))?;

        match self.normalise(arr_ty) {
            Type::Array(inner_ty) => Ok(*inner_ty),
            _ => unreachable!(),
        }
    }

    fn type_of_if(
        &mut self,
        cond: &ExprS,
        th: &ExprS,
        el: Option<&ExprS>,
    ) -> Result<Type, TypeErrorS> {
        let cond_ty = self.type_of(cond)?;
        self.unify(&Type::Bool, &cond_ty)
            .map_err(|e| e.spanned(cond.span))?;

        let th_ty = self.type_of(th)?;

        if let Some(el) = el {
            let el_ty = self.type_of(el)?;
            self.unify(&th_ty, &el_ty).map_err(|e| e.spanned(el.span))?;
        } else {
            self.unify(&Type::unit(), &th_ty)
                .map_err(|e| e.spanned(th.span))?;
        }

        Ok(th_ty)
    }

    fn type_of_let(&mut self, binding: &BindingS, val: &ExprS) -> Result<Type, TypeErrorS> {
        let Binding::Var {
            mutable,
            ident,
            annotated_ty,
        } = &binding.inner;

        let val_ty = self.type_of(val)?;

        if let Some(ty) = annotated_ty {
            let annot_ty = ty.inner.clone().into();
            self.unify(&annot_ty, &val_ty)
                .map_err(|e| e.spanned(val.span))?;
        }

        self.env.insert(
            ident.to_owned(),
            BindingInfo {
                ty: val_ty,
                mutable: *mutable,
            },
        );

        Ok(Type::unit())
    }

    fn type_of_assign(&mut self, ident: Spanned<&str>, val: &ExprS) -> Result<Type, TypeErrorS> {
        let val_ty = self.type_of(val)?;

        let info = self.get_binding(ident)?;

        if !info.mutable {
            return Err(TypeError::Mutation(ident.inner.to_owned()).spanned(ident.span));
        }

        self.unify(&info.ty, &val_ty)
            .map_err(|e| e.spanned(val.span))?;

        Ok(Type::unit())
    }

    fn type_of_lambda(
        &self,
        params: &[BindingS],
        return_ty: Option<&AstTypeS>,
        body: &ExprS,
    ) -> Result<Type, TypeErrorS> {
        let mut snapshot = self.clone();

        let mut param_tys = Vec::new();
        for param in params {
            let Binding::Var {
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

        let body_ty = snapshot.type_of(body)?;

        if let Some(ty) = return_ty {
            let return_ty = ty.inner.clone().into();
            snapshot
                .unify(&return_ty, &body_ty)
                .map_err(|e| e.spanned(body.span))?;
        }

        Ok(Type::Fn(param_tys, Box::new(body_ty)))
    }

    fn type_of_block(&self, exprs: &[ExprS], trailing: bool) -> Result<Type, TypeErrorS> {
        let mut snapshot = self.clone();

        let mut last = Option::None;
        for expr in exprs {
            last = Some(snapshot.type_of(expr)?);
        }

        Ok(if trailing && let Some(ty) = last {
            ty
        } else {
            Type::unit()
        })
    }
}
