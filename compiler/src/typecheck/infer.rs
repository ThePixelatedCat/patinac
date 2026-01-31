use super::{BindingInfo, Type, TypeChecker, TypeError, TypeResult, TypeS};
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
                Type::GInt
            }),
            Expr::Float(_) => Ok(Type::Float.spanned(expr.span)),
            Expr::Str(_) => Ok(Type::string().spanned(expr.span)),
            Expr::Char(_) => Ok(Type::Char.spanned(expr.span)),
            Expr::Bool(_) => Ok(Type::Bool.spanned(expr.span)),
            Expr::Array(vals) => self.type_of_array(vals, expr.span),
            Expr::Tuple(vals) => self.type_of_tuple(vals, expr.span),
            Expr::FnCall { fun, args } => self.type_of_fn_call(fun, args, expr.span),
            Expr::BinaryOp { op, lhs, rhs } => self.type_of_binary_op(*op, lhs, rhs, expr.span),
            Expr::UnaryOp { op, expr } => self.type_of_unary_op(*op, expr),
            Expr::Index { arr, index } => self.type_of_indexing(arr, index),
            Expr::FieldAccess { base, field } => todo!(),
            Expr::If { cond, th, el } => self.type_of_if(cond, th, el.as_deref()),
            Expr::Let { binding, value } => self.type_of_let(binding, value, expr.span),
            Expr::Assign { ident, value } => {
                self.type_of_assign(ident.as_deref(), value, expr.span)
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
            .map(|b| b.ty.clone().spanned(ident.span))
    }

    fn type_of_array(&mut self, vals: &[ExprS], span: Span) -> TypeResult {
        let inner_ty = match vals.first() {
            Some(e) => self.type_of(e)?,
            None => self.fresh_var().spanned(span),
        };

        vals[1..].iter().try_for_each(|v| {
            let this_ty = self.type_of(v)?;
            self.unify(&inner_ty, &this_ty)
        })?;

        Ok(Type::Array(Box::new(inner_ty)).spanned(span))
    }

    fn type_of_tuple(&mut self, vals: &[ExprS], span: Span) -> TypeResult {
        Ok(Type::Tuple(
            vals.iter()
                .map(|e| self.type_of(e))
                .collect::<TypeResult<_>>()?,
        )
        .spanned(span))
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
        match op {
            Bop::Add | Bop::Sub | Bop::Mul | Bop::Div | Bop::Exp => {
                let (lhs_ty, rhs_ty) = (self.type_of(lhs)?, self.type_of(rhs)?);

                if !lhs_ty.is_numeric() {
                    return Err(TypeError::NotNumeric(lhs_ty).spanned(lhs.span));
                }

                if !rhs_ty.is_numeric() {
                    return Err(TypeError::NotNumeric(rhs_ty).spanned(rhs.span));
                }

                if lhs_ty != rhs_ty {
                    return Err(TypeError::MismatchedTypes {
                        found: rhs_ty,
                        expected: lhs_ty,
                    }
                    .spanned(rhs.span));
                }

                Ok(lhs_ty)
            }
            Bop::And | Bop::Or | Bop::Xor => {
                self.unify(&self.type_of(lhs)?, &Type::Bool.spanned(lhs.span));
                self.unify(&self.type_of(rhs)?, &Type::Bool.spanned(rhs.span));
                Ok(Type::Bool.spanned(span))
            }
            Bop::BOr | Bop::BAnd => {
                let (lhs_ty, rhs_ty) = (self.type_of(lhs)?, self.type_of(rhs)?);

                if !lhs_ty.is_integer() {
                    return Err(TypeError::NotInteger(lhs_ty).spanned(lhs.span));
                }

                if !rhs_ty.is_integer() {
                    return Err(TypeError::NotInteger(rhs_ty).spanned(rhs.span));
                }

                if lhs_ty != rhs_ty {
                    return Err(TypeError::MismatchedTypes {
                        found: rhs_ty,
                        expected: lhs_ty,
                    }
                    .spanned(rhs.span));
                }

                Ok(lhs_ty)
            }
            Bop::Eqq | Bop::Neq => {
                self.unify(&self.type_of(lhs)?, &self.type_of(rhs)?)?;
                Ok(Type::Bool.spanned(span))
            }
            Bop::Gt | Bop::Lt | Bop::Geq | Bop::Leq => {
                let (lhs_ty, rhs_ty) = (self.type_of(lhs)?, self.type_of(rhs)?);

                if !lhs_ty.is_numeric() {
                    return Err(TypeError::NotNumeric(lhs_ty).spanned(lhs.span));
                }

                if !rhs_ty.is_numeric() {
                    return Err(TypeError::NotNumeric(rhs_ty).spanned(rhs.span));
                }

                Ok(Type::Bool)
            }
        }
    }

    fn type_of_unary_op(&mut self, op: Unop, expr: &ExprS) -> TypeResult {
        match op {
            Unop::Not => {
                self.unify(&self.type_of(expr)?, &Type::Bool.spanned(expr.span))?;
                Ok(Type::Bool.spanned(expr.span))
            }
            Unop::Neg => match self.type_of(expr)? {
                Type::GInt => Ok(Type::Int),
                ty @ (Type::Int | Type::Float) => Ok(ty),
                other => Err(todo!()),
            },
        }
    }

    fn type_of_indexing(&mut self, arr: &ExprS, index: &ExprS) -> TypeResult {
        let index_type = self.type_of(index)?;
        self.unify(&index_type, &Type::UInt.spanned(index_type.span))?;

        let arr_type = self.type_of(arr)?;
        let arr_var =
            Type::Array(Box::new(self.fresh_var().spanned(arr.span))).spanned(arr_type.span);
        self.unify(&arr_type, &arr_var)?;

        Ok(arr_type)
    }

    fn type_of_if(&mut self, cond: &ExprS, th: &ExprS, el: Option<&ExprS>) -> TypeResult {
        let cond_type = self.type_of(cond)?;
        self.unify(&cond_type, &Type::Bool.spanned(cond_type.span));

        let th_type = self.type_of(th)?;

        let el_type = match el {
            Some(el) => self.type_of(el)?,
            None => Type::unit().spanned(th.span.end..th.span.end + 1),
        };

        self.unify(&th_type, &el_type);
        Ok(th_type)
    }

    fn type_of_let(&mut self, binding: &BindingS, value: &ExprS, span: Span) -> TypeResult {
        let Binding::Var {
            mutable,
            ident,
            type_annotation,
        } = &binding.inner;

        let expr_ty = self.type_of(value)?;

        if let Some(annotated_ty) = type_annotation {
            self.unify(&expr_ty, &annotated_ty.clone().into())?;
        }

        self.env.insert(
            ident.to_owned(),
            BindingInfo {
                ty: expr_ty.inner,
                mutable: *mutable,
            },
        );

        Ok(Type::unit().spanned(span))
    }

    fn type_of_assign(&mut self, ident: Spanned<&str>, value: &ExprS, span: Span) -> TypeResult {
        let assigned_ty = self.type_of(value)?;

        let info = self.get_binding(ident)?;

        if !info.mutable {
            return Err(TypeError::Mutation(ident.inner.to_owned()).spanned(span));
        }

        self.unify(&info.ty.clone().spanned(ident.span), &assigned_ty)?;

        Ok(Type::unit().spanned(span))
    }

    fn type_of_block(&self, exprs: &[ExprS], trailing: bool, span: &Span) -> TypeResult {
        let types = self.check(exprs)?;

        Ok(if trailing && let Some(last) = types.last() {
            last.clone()
        } else {
            Type::unit().spanned(span)
        })
    }
}
