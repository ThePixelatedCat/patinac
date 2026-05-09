use itertools::Itertools;

use ast::{
    exprs::{Arg, Binding, Expr, ExprKind, InfixOp, LitExpr, MatchArm, Stmt, UnaryOp},
    patterns::Pat,
};
use ident::SpanIdent;
use nameres::{AdtId, AdtInfoKind, NameTable, VarId};
use span::Span;

use crate::{
    ErrorKind, PartialTy, Result, TypeChecker,
    type_vars::{Param, Return},
};

impl TypeChecker {
    pub(super) fn infer_expr(
        &mut self,
        name_table: &NameTable,
        expr: Expr<(), AdtId, VarId>,
    ) -> Result<Expr<PartialTy, AdtId, VarId>> {
        let span = expr.span;
        match expr.kind {
            ExprKind::Path(path) => {
                if !path.prefix.is_empty() {
                    todo!("handle paths")
                }

                Ok(self.infer_ident(name_table, span, path.end))
            }
            ExprKind::Lit(lit) => Ok(self.infer_lit(span, lit)),
            ExprKind::Array(exprs) => self.infer_array(name_table, span, exprs),
            ExprKind::Tuple(exprs) => self.infer_tuple(name_table, span, exprs),
            ExprKind::Call { func, args } => self.infer_call(name_table, span, *func, args),
            ExprKind::Infix { op, lhs, rhs } => self.infer_binop(name_table, span, op, *lhs, *rhs),
            ExprKind::Unary { op, expr } => self.infer_unop(name_table, span, op, *expr),
            ExprKind::Index { arr, idx } => self.infer_indexing(name_table, span, *arr, *idx),
            ExprKind::Field { base, field } => self.infer_field(name_table, span, *base, field),
            ExprKind::Lambda { params, body } => self.infer_lambda(name_table, span, params, *body),
            ExprKind::If { cond, th, el } => {
                self.infer_if(name_table, span, *cond, *th, el.map(|v| *v))
            }
            ExprKind::Match { scrutinee, arms } => {
                self.infer_match(name_table, span, *scrutinee, arms)
            }
            ExprKind::For { pat, iter, body } => {
                self.infer_for(name_table, span, pat, *iter, *body)
            }
            ExprKind::Loop(body) => self.infer_loop(name_table, span, *body),
            ExprKind::Break => todo!(),
            ExprKind::Continue => todo!(),
            ExprKind::Return(expr) => todo!(),
            ExprKind::Block(stmts) => self.infer_block(name_table, span, stmts),
        }
    }

    fn infer_multi(
        &mut self,
        name_table: &NameTable,
        exprs: Vec<Expr<(), AdtId, VarId>>,
    ) -> Result<Vec<Expr<PartialTy, AdtId, VarId>>> {
        exprs
            .into_iter()
            .map(|e| self.infer_expr(name_table, e))
            .collect()
    }

    fn types_of(
        &mut self,
        name_table: &NameTable,
        exprs: Vec<Expr<(), AdtId, VarId>>,
    ) -> Result<(Vec<Expr<PartialTy, AdtId, VarId>>, Vec<PartialTy>)> {
        exprs
            .into_iter()
            .map(|e| {
                let e = self.infer_expr(name_table, e)?;
                let ty = e.ty.clone();
                Ok((e, ty))
            })
            .collect()
    }

    fn infer_ident(
        &mut self,
        name_table: &NameTable,
        span: Span,
        ident: VarId,
    ) -> Expr<PartialTy, AdtId, VarId> {
        let ty = self.convert(name_table.vars[ident].ty.as_ref());
        ExprKind::ident_id(ident).span_ty(span, ty)
    }

    fn check_place_mut(
        &mut self,
        name_table: &NameTable,
        place: &Expr<PartialTy, AdtId, VarId>,
    ) -> Result<()> {
        let span = place.span;
        match &place.kind {
            ExprKind::Path(path) => {
                if !path.prefix.is_empty() {
                    todo!("handle paths")
                }

                name_table.vars[path.end]
                    .mutable
                    .then_some(())
                    .ok_or_else(|| ErrorKind::Mutation.span(span))
            }
            ExprKind::Field { base, .. } | ExprKind::Index { arr: base, .. } => {
                self.check_place_mut(name_table, base)
            }
            ExprKind::Call { .. } => todo!("Projections"),
            _ => Err(ErrorKind::NotPlaceExpr.span(span)),
        }
    }

    fn check_places_unique(
        &mut self,
        place_a: &Expr<PartialTy, AdtId, VarId>,
        place_b: &Expr<PartialTy, AdtId, VarId>,
    ) -> Result<()> {
        match &place_b.kind {
            ExprKind::Path(_) => (place_a != place_b)
                .then_some(())
                .ok_or_else(|| ErrorKind::OverlappingPlace(place_a.span).span(place_b.span)),
            ExprKind::Field { base, .. } | ExprKind::Index { arr: base, .. } => {
                self.check_places_unique(place_a, base)
            }
            ExprKind::Call { .. } => todo!("Projections"),
            _ => Err(ErrorKind::NotPlaceExpr.span(place_b.span)),
        }
    }

    fn infer_lit(&mut self, span: Span, lit: LitExpr) -> Expr<PartialTy, AdtId, VarId> {
        let ty = match &lit {
            LitExpr::Int(_) => self.fresh_int_var(),
            LitExpr::Float(_) => PartialTy::Float,
            LitExpr::String(_) => PartialTy::string(),
            LitExpr::Char(_) => PartialTy::Char,
            LitExpr::Bool(_) => PartialTy::Bool,
        };
        ExprKind::Lit(lit).span_ty(span, ty)
    }

    fn infer_array(
        &mut self,
        name_table: &NameTable,
        span: Span,
        exprs: Vec<Expr<(), AdtId, VarId>>,
    ) -> Result<Expr<PartialTy, AdtId, VarId>> {
        let exprs = self.infer_multi(name_table, exprs)?;

        let inner_ty = self.fresh_var();
        for expr in &exprs {
            self.constrain_eq(&expr, inner_ty.clone());
        }

        Ok(ExprKind::Array(exprs).span_ty(span, PartialTy::array(inner_ty)))
    }

    fn infer_tuple(
        &mut self,
        name_table: &NameTable,
        span: Span,
        exprs: Vec<Expr<(), AdtId, VarId>>,
    ) -> Result<Expr<PartialTy, AdtId, VarId>> {
        let (exprs, tys) = self.types_of(name_table, exprs)?;
        Ok(ExprKind::Tuple(exprs).span_ty(span, PartialTy::Tuple(tys)))
    }

    fn infer_call(
        &mut self,
        name_table: &NameTable,
        span: Span,
        func: Expr<(), AdtId, VarId>,
        args: Vec<Arg<(), AdtId, VarId>>,
    ) -> Result<Expr<PartialTy, AdtId, VarId>> {
        let func = Box::new(self.infer_expr(name_table, func)?);

        let (args, arg_tys): (Vec<_>, Vec<_>) = args
            .into_iter()
            .map(|arg| {
                let val = self.infer_expr(name_table, arg.val)?;

                if arg.mutable {
                    self.check_place_mut(name_table, &val)?;
                }

                let ty = val.ty.clone();
                Ok((
                    Arg {
                        mutable: arg.mutable,
                        val,
                    },
                    Param {
                        mutable: arg.mutable,
                        ty,
                    },
                ))
            })
            .try_collect()?;

        for (a, b) in (0..args.len())
            .permutations(2)
            .map(|p| (&args[p[0]], &args[p[1]]))
            .filter(|(a, b)| a.mutable || b.mutable)
        {
            self.check_places_unique(&a.val, &b.val)?;
        } // TODO optimise???

        let return_ty = self.fresh_var();

        self.constrain_eq(
            &func,
            PartialTy::Fn(
                arg_tys,
                Box::new(Return {
                    mutable: false,
                    ty: return_ty.clone(),
                }),
            ),
        );

        Ok(ExprKind::Call { func, args }.span_ty(span, return_ty))
    }

    fn infer_binop(
        &mut self,
        name_table: &NameTable,
        span: Span,
        op: InfixOp,
        lhs: Expr<(), AdtId, VarId>,
        rhs: Expr<(), AdtId, VarId>,
    ) -> Result<Expr<PartialTy, AdtId, VarId>> {
        let lhs = Box::new(self.infer_expr(name_table, lhs)?);
        let rhs = Box::new(self.infer_expr(name_table, rhs)?);

        let ty = match op {
            InfixOp::Assign => {
                self.check_place_mut(name_table, &lhs)?;
                self.constrain_eq(&rhs, lhs.ty.clone());

                PartialTy::unit()
            }
            InfixOp::Add | InfixOp::Sub | InfixOp::Mul | InfixOp::Div | InfixOp::Rem => {
                let int_var = self.fresh_int_var();
                self.constrain_either_eq(lhs.ty.clone(), (PartialTy::Float, int_var), lhs.span);
                self.constrain_eq(&rhs, lhs.ty.clone());

                lhs.ty.clone()
            }
            InfixOp::Exp => {
                let int_var = self.fresh_int_var();
                self.constrain_either_eq(
                    lhs.ty.clone(),
                    (PartialTy::Float, int_var.clone()),
                    lhs.span,
                );
                self.constrain_eq(&rhs, int_var);

                lhs.ty.clone()
            }
            InfixOp::And | InfixOp::Or | InfixOp::Xor => {
                self.constrain_eq(&lhs, PartialTy::Bool);
                self.constrain_eq(&rhs, PartialTy::Bool);

                PartialTy::Bool
            }
            InfixOp::Eqq | InfixOp::Neq => {
                self.constrain_eq(&rhs, lhs.ty.clone());

                PartialTy::Bool
            }
            InfixOp::Gt | InfixOp::Lt | InfixOp::Geq | InfixOp::Leq => {
                let int_var = self.fresh_int_var();
                self.constrain_either_eq(lhs.ty.clone(), (PartialTy::Float, int_var), lhs.span);
                self.constrain_eq(&rhs, lhs.ty.clone());

                PartialTy::Bool
            }
        };

        Ok(ExprKind::Infix { op, lhs, rhs }.span_ty(span, ty))
    }

    fn infer_unop(
        &mut self,
        name_table: &NameTable,
        span: Span,
        op: UnaryOp,
        expr: Expr<(), AdtId, VarId>,
    ) -> Result<Expr<PartialTy, AdtId, VarId>> {
        let expr = Box::new(self.infer_expr(name_table, expr)?);

        let ty = match op {
            UnaryOp::Not => {
                self.constrain_eq(&expr, PartialTy::Bool);

                PartialTy::Bool
            }
            UnaryOp::Neg => {
                let int_var = self.fresh_int_var();
                self.constrain_either_eq(expr.ty.clone(), (int_var, PartialTy::Float), expr.span);

                expr.ty.clone()
            }
        };

        Ok(ExprKind::Unary { op, expr }.span_ty(span, ty))
    }

    fn infer_field(
        &mut self,
        name_table: &NameTable,
        span: Span,
        base: Expr<(), AdtId, VarId>,
        field: SpanIdent,
    ) -> Result<Expr<PartialTy, AdtId, VarId>> {
        let base = Box::new(self.infer_expr(name_table, base)?);

        let PartialTy::Adt(base_ty, _) = base.ty else {
            return Err(ErrorKind::PrimitiveTypeNoField(base.ty).span(span));
        };
        let AdtInfoKind::Record { fields, .. } = &name_table.adts[base_ty].kind else {
            return Err(ErrorKind::MissingField.span(span));
        };

        let field_ty = PartialTy::from(
            &fields
                .get(&field.ident)
                .cloned()
                .ok_or_else(|| ErrorKind::MissingField.span(span))?
                .ty,
        );

        Ok(ExprKind::Field { base, field }.span_ty(span, field_ty))
    }

    fn infer_indexing(
        &mut self,
        name_table: &NameTable,
        span: Span,
        arr: Expr<(), AdtId, VarId>,
        idx: Expr<(), AdtId, VarId>,
    ) -> Result<Expr<PartialTy, AdtId, VarId>> {
        let arr = Box::new(self.infer_expr(name_table, arr)?);

        let inner_ty = self.fresh_var();
        self.constrain_eq(&arr, PartialTy::array(inner_ty.clone()));

        let idx = Box::new(self.infer_expr(name_table, idx)?);
        self.constrain_eq(&idx, PartialTy::UInt);

        Ok(ExprKind::Index { arr, idx }.span_ty(span, inner_ty))
    }

    fn infer_if(
        &mut self,
        name_table: &NameTable,
        span: Span,
        cond: Expr<(), AdtId, VarId>,
        th: Expr<(), AdtId, VarId>,
        el: Option<Expr<(), AdtId, VarId>>,
    ) -> Result<Expr<PartialTy, AdtId, VarId>> {
        let cond = Box::new(self.infer_expr(name_table, cond)?);
        self.constrain_eq(&cond, PartialTy::Bool);

        let th = Box::new(self.infer_expr(name_table, th)?);

        let el = el
            .map(|el| self.infer_expr(name_table, el))
            .transpose()?
            .map(Box::new);
        match &el {
            Some(el) => self.constrain_eq(&el, th.ty.clone()),
            None => self.constrain_eq(&th, PartialTy::unit()),
        }

        let th_ty = th.ty.clone();
        Ok(ExprKind::If { cond, th, el }.span_ty(span, th_ty))
    }

    fn infer_match(
        &mut self,
        name_table: &NameTable,
        span: Span,
        scrutinee: Expr<(), AdtId, VarId>,
        arms: Vec<MatchArm<(), AdtId, VarId>>,
    ) -> Result<Expr<PartialTy, AdtId, VarId>> {
        let scrutinee = Box::new(self.infer_expr(name_table, scrutinee)?);

        let ty = self.fresh_var();
        let arms = arms
            .into_iter()
            .map(|arm| {
                let body = self.infer_expr(name_table, arm.body)?;
                self.constrain_eq(&body, ty.clone());

                Ok(MatchArm {
                    pat: arm.pat,
                    body,
                    span: arm.span,
                })
            })
            .try_collect()?;

        Ok(ExprKind::Match { scrutinee, arms }.span_ty(span, ty))
    }

    fn infer_for(
        &mut self,
        name_table: &NameTable,
        span: Span,
        pat: Pat<VarId>,
        iter: Expr<(), AdtId, VarId>,
        body: Expr<(), AdtId, VarId>,
    ) -> Result<Expr<PartialTy, AdtId, VarId>> {
        let iter = Box::new(self.infer_expr(name_table, iter)?);
        let body = Box::new(self.infer_expr(name_table, body)?);
        Ok(ExprKind::For { pat, iter, body }.span_ty(span, PartialTy::unit()))
    }

    fn infer_loop(
        &mut self,
        name_table: &NameTable,
        span: Span,
        body: Expr<(), AdtId, VarId>,
    ) -> Result<Expr<PartialTy, AdtId, VarId>> {
        let body = Box::new(self.infer_expr(name_table, body)?);
        Ok(ExprKind::Loop(body).span_ty(span, PartialTy::unit()))
    }

    fn infer_lambda(
        &mut self,
        name_table: &NameTable,
        span: Span,
        params: Vec<Binding<AdtId, VarId>>,
        body: Expr<(), AdtId, VarId>,
    ) -> Result<Expr<PartialTy, AdtId, VarId>> {
        let param_tys = params
            .iter()
            .map(|p| Param {
                mutable: p.mutable,
                ty: self.convert(p.ty.as_ref()),
            })
            .collect();

        let body = Box::new(self.infer_expr(name_table, body)?);

        let ty = PartialTy::Fn(
            param_tys,
            Box::new(Return {
                mutable: false,
                ty: body.ty.clone(),
            }),
        );

        Ok(ExprKind::Lambda { params, body }.span_ty(span, ty))
    }

    fn infer_block(
        &mut self,
        name_table: &NameTable,
        span: Span,
        stmts: Vec<Stmt<(), AdtId, VarId>>,
    ) -> Result<Expr<PartialTy, AdtId, VarId>> {
        let stmts: Vec<_> = stmts
            .into_iter()
            .map(|s| self.infer_stmt(name_table, s))
            .try_collect()?;

        let ty = stmts
            .last()
            .and_then(|s| match s {
                Stmt::Decl { .. } => None,
                Stmt::Expr(expr) => Some(expr.ty.clone()),
            })
            .unwrap_or_else(PartialTy::unit);

        Ok(ExprKind::Block(stmts).span_ty(span, ty))
    }

    fn infer_stmt(
        &mut self,
        name_table: &NameTable,
        stmt: Stmt<(), AdtId, VarId>,
    ) -> Result<Stmt<PartialTy, AdtId, VarId>> {
        match stmt {
            Stmt::Decl { binding, val, span } => {
                let val = self.infer_expr(name_table, val)?;
                binding
                    .ty
                    .as_ref()
                    .inspect(|&ty| self.constrain_eq(&val, ty.into()));

                Ok(Stmt::Decl { binding, val, span })
            }
            Stmt::Expr(expr) => self.infer_expr(name_table, expr).map(Stmt::Expr),
        }
    }
}
