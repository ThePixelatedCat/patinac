use super::{BindingInfo, Type, TypeChecker, TypeError, TypeResult};
use crate::helpers::{Span, Spanned};
use crate::parser::ast::{Ast, Binding, BindingS, Bop, Expr, ExprS, Item, Unop};

impl TypeChecker {
    pub fn type_of(&mut self, expr: &ExprS) -> TypeResult {
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
            Expr::Str(_) => Ok(Type::string()),
            Expr::Char(_) => Ok(Type::Char),
            Expr::Bool(_) => Ok(Type::Bool),
            Expr::Array(vals) => self.type_of_array(vals),
            Expr::Tuple(vals) => self.type_of_tuple(vals),
            Expr::FnCall { fun, args } => self.type_of_fn_call(fun, args, expr.span),
            Expr::BinaryOp { op, lhs, rhs } => self.type_of_binary_op(*op, lhs, rhs, expr.span),
            Expr::UnaryOp { op, expr } => self.type_of_unary_op(*op, expr),
            Expr::Index { arr, index } => self.type_of_indexing(arr, index),
            Expr::FieldAccess { base, field } => todo!(),
            Expr::If { cond, th, el } => self.type_of_if(cond, th, el.as_deref()),
            Expr::Let { binding, value } => self.type_of_let(binding, value, expr.span),
            Expr::Assign { ident, value } => {
                self.type_of_assign(ident.as_deref(), value)
            }
            Expr::Lambda {
                params,
                return_type,
                body,
            } => todo!(),
            Expr::Block { exprs, trailing } => self.type_of_block(exprs, *trailing, &expr.span),
        }
    }

    fn type_of_ident(&self, ident: Spanned<&str>) -> TypeResult {
        self.get_binding(ident)
            .map(|b| b.ty.clone())
    }

    fn type_of_array(&mut self, vals: &[ExprS]) -> TypeResult {
        let inner_ty = match vals.first() {
            Some(e) => self.type_of(e)?,
            None => self.fresh_var(),
        };

        vals[1..].iter().try_for_each(|v| {
            let this_ty = self.type_of(v)?;
            self.unify(&inner_ty, &this_ty).map_err(|e| e.spanned(v.span))
        })?;

        Ok(Type::Array(Box::new(inner_ty)))
    }

    fn type_of_tuple(&mut self, vals: &[ExprS]) -> TypeResult {
        Ok(Type::Tuple(
            vals.iter()
                .map(|e| self.type_of(e))
                .collect::<TypeResult<_>>()?,
        ))
    }

    fn type_of_fn_call(&mut self, fun: &ExprS, args: &Vec<ExprS>, span: Span) -> TypeResult {
        todo!("rewrite");
        // let fn_ty = self.type_of(fun)?;

        // self.unify(a, b)

        // if param_tys.len() != args.len() {
        //     return Err(TypeError::WrongArgCount {
        //         needed: param_tys.len(),
        //         provided: args.len(),
        //     }
        //     .spanned(span));
        // }

        // iter::zip(param_tys, args).try_for_each(|(p, a)| {
        //     let arg_ty = self.type_of(a)?;

        //     if p == arg_ty {
        //         Ok(())
        //     } else {
        //         Err(TypeError::MismatchedTypes {
        //             found: p,
        //             expected: arg_ty,
        //         }
        //         .spanned(a.span))
        //     }
        // })?;

        // Ok(result_ty)
    }

    fn type_of_binary_op(&mut self, op: Bop, lhs: &ExprS, rhs: &ExprS, span: Span) -> TypeResult {
        let (lhs_ty, rhs_ty) = (self.type_of(lhs)?, self.type_of(rhs)?);
        match op {
            Bop::Add
            | Bop::Sub
            | Bop::Mul
            | Bop::Div
            | Bop::Exp
            | Bop::BOr
            | Bop::BAnd
            | Bop::Gt
            | Bop::Lt
            | Bop::Geq
            | Bop::Leq => {
                let lhs_var = self.fresh_int_var();
                let rhs_var = self.fresh_int_var();

                self.unify(&lhs_ty, &lhs_var)?;
                self.unify(&rhs_ty, &rhs_var)?;

                self.unify(&lhs_ty, &rhs_ty)?;

                Ok(lhs_ty)
            }
            Bop::And | Bop::Or | Bop::Xor => {
                self.unify(&lhs_ty, &Type::Bool)?;
                self.unify(&rhs_ty, &Type::Bool)?;
                Ok(Type::Bool)
            }
            Bop::Eqq | Bop::Neq => {
                self.unify(&lhs_ty, &rhs_ty)?;
                Ok(Type::Bool)
            }
        }
    }

    fn type_of_unary_op(&mut self, op: Unop, expr: &ExprS) -> TypeResult {
        let expr_ty = self.type_of(expr)?;

        match op {
            Unop::Not => {
                self.unify(&expr_ty, &Type::Bool)?;
            }
            // TODO confirm logic??
            Unop::Neg => {
                self.unify(&expr_ty, &Type::Int)
                    .or_else(|_| self.unify(&expr_ty, &Type::Float))?;
            }
        }

        Ok(expr_ty)
    }

    fn type_of_indexing(&mut self, arr: &ExprS, index: &ExprS) -> TypeResult {
        let index_type = self.type_of(index)?;
        self.unify(&index_type, &Type::UInt)?;

        let arr_type = self.type_of(arr)?;
        let arr_var =
            Type::Array(Box::new(self.fresh_var()));
        self.unify(&arr_type, &arr_var)?;

        Ok(arr_type)
    }

    fn type_of_if(&mut self, cond: &ExprS, th: &ExprS, el: Option<&ExprS>) -> TypeResult {
        let cond_type = self.type_of(cond)?;
        self.unify(&cond_type, &Type::Bool).map_err(|e| e.spanned(cond.span))?;

        let th_type = self.type_of(th)?;

        match el {
            Some(el) => {
                let el_type = self.type_of(el)?;
                self.unify(&th_type, &el_type).map_err(|e| e.spanned(el.span))?;
            },
            None => self.unify(&Type::unit(), &th_type).map_err(|e| e.spanned(th.span))?,
        };

        Ok(th_type)
    }

    fn type_of_let(&mut self, binding: &BindingS, value: &ExprS) -> TypeResult {
        let Binding::Var {
            mutable,
            ident,
            type_annotation,
        } = &binding.inner;

        let expr_ty = self.type_of(value)?;

        if let Some(annotated_ty) = type_annotation {
            self.unify(&expr_ty, &annotated_ty.inner.clone().into()).map_err(|e| e.spanned(value.span))?;
        }

        self.env.insert(
            ident.to_owned(),
            BindingInfo {
                ty: expr_ty,
                mutable: *mutable,
            },
        );

        Ok(Type::unit())
    }

    fn type_of_assign(&mut self, ident: Spanned<&str>, value: &ExprS) -> TypeResult {
        let assigned_ty = self.type_of(value)?;

        let info = self.get_binding(ident)?;

        if !info.mutable {
            return Err(TypeError::Mutation(ident.inner.to_owned()).spanned(ident.span));
        }

        self.unify(&info.ty.clone(), &assigned_ty).map_err(|e| e.spanned(value.span))?;

        Ok(Type::unit())
    }

    fn type_of_block(&self, exprs: &[ExprS], trailing: bool, span: &Span) -> TypeResult {
        todo!()
        // let types = self.check(exprs)?;

        // Ok(if trailing && let Some(last) = types.last() {
        //     last.clone()
        // } else {
        //     Type::unit().spanned(span)
        // })
    }
}
