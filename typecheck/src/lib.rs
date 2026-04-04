mod env;
mod error;
mod infer;
mod substitute;
#[cfg(test)]
mod test;
pub mod types;
mod unify;

use std::iter;

use ena::unify::InPlaceUnificationTable;
use fnv::FnvHashMap;

use ast::{
    Ast,
    exprs::Expr,
    items::{AdtItem, ExecItem, Field},
    patterns::Pat,
    types::{Ty as AstTy, TyKind as AstTyKind},
};
use ident::Ident;
use span::Span;

pub use crate::error::{Error, ErrorKind, Result};
use crate::{
    env::{BindingInfo, Ctx, TyEnv, TyInfo},
    types::{ConcreteTy, Param, Ty, TyVar},
};

struct Constraint {
    kind: ConstraintKind,
    span: Span,
}

enum ConstraintKind {
    TypeEqual(Ty, Ty),
    EitherTypeEqual(Ty, (Ty, Ty)),
}

#[derive(Default)]
pub struct TypeChecker {
    table: InPlaceUnificationTable<TyVar>,
    constraints: Vec<Constraint>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self::default()
    }
}

impl TypeChecker {
    pub fn type_program(&mut self, Ast { adts, execs }: Ast<()>) -> Result<Ast<ConcreteTy>> {
        let (ty_env, mut ctx) = Self::build_env(&adts);

        self.populate_ctx(&mut ctx, &execs);

        let execs = execs
            .into_iter()
            .map(|exec| {
                self.clear_constraints();

                match exec {
                    ExecItem::Const { ident, ty, val } => {
                        // Constraint generation
                        let typed_val = self.infer(&ty_env, &mut ctx.clone(), val)?;
                        self.constrain_eq(
                            &typed_val,
                            ctx.get(ident, 0..0)
                                .expect("all items were previously inserted into ctx")
                                .ty,
                        );

                        // Constraint solving
                        self.unify()?;

                        Ok(ExecItem::Const {
                            ident,
                            ty,
                            val: self.sub_expr(typed_val)?,
                        })
                    }
                    ExecItem::Func {
                        ident,
                        generic_params,
                        params,
                        return_ty: return_ty_ast,
                        body,
                    } => {
                        let mut ctx = ctx.clone();

                        let Ok(BindingInfo {
                            ty: Ty::Func(param_tys, return_ty),
                            ..
                        }) = ctx.get(ident, 0..0)
                        else {
                            unreachable!("all items were previously inserted into ctx")
                        };

                        for (pat, ty, mutable) in
                            iter::zip(&params, param_tys).map(|(p, ty)| (&p.pat, ty.ty, ty.mutable))
                        {
                            match pat {
                                Pat::Ident { ident, subpat } => {
                                    ctx.insert(*ident, ty, mutable);
                                }
                                Pat::Wildcard => {}
                                _ => todo!("tuple patterns are unimplemented"),
                            }
                        }

                        let body = self.infer(&ty_env, &mut ctx, body)?;
                        self.constrain_eq(&body, *return_ty);

                        self.unify()?;

                        Ok(ExecItem::Func {
                            ident,
                            generic_params,
                            params,
                            return_ty: return_ty_ast,
                            body: self.sub_expr(body)?,
                        })
                    }
                }
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Ast { adts, execs })
    }

    fn build_env(adts: &[AdtItem]) -> (TyEnv, Ctx) {
        let mut ty_env = TyEnv::default();
        ty_env.insert(Ident::new("String"), TyInfo::default());

        let mut ctx = Ctx::default();

        for adt in adts {
            match adt {
                AdtItem::Record { def, fields } => {
                    let field_map = fields
                        .iter()
                        .map(|field| (field.ident, Ty::from(&field.ty)))
                        .collect();
                    ty_env.insert(
                        def.ident,
                        TyInfo {
                            //generic_params: def.generics,
                            fields: field_map,
                        },
                    );

                    let field_params = fields
                        .iter()
                        .map(|field| Param {
                            mutable: false,
                            ty: Ty::from(&field.ty),
                        })
                        .collect();
                    ctx.insert(
                        def.ident,
                        Ty::Func(field_params, Box::new(Ty::Adt(def.ident, vec![]))),
                        false,
                    );
                }
                AdtItem::Enum { def, variants } => todo!(),
            }
        }

        (ty_env, ctx)
    }

    fn populate_ctx(&mut self, ctx: &mut Ctx, execs: &[ExecItem<()>]) {
        for exec in execs {
            match exec {
                ExecItem::Const { ident, ty, .. } => {
                    ctx.insert(*ident, self.convert(ty.as_ref()), false)
                }
                ExecItem::Func {
                    ident,
                    generic_params,
                    params,
                    return_ty,
                    ..
                } => {
                    ctx.insert(
                        *ident,
                        Ty::Func(
                            params
                                .iter()
                                .map(|param| Param {
                                    mutable: param.mutable,
                                    ty: (&param.ty).into(),
                                })
                                .collect(),
                            Box::new(return_ty.into()),
                        ),
                        false,
                    );
                }
            }
        }
    }

    fn fresh_var(&mut self) -> Ty {
        Ty::Var(self.table.new_key(None))
    }

    fn fresh_int_var(&mut self) -> Ty {
        Ty::IntVar(self.table.new_key(None))
    }

    fn constrain_eq(&mut self, a: &Expr<Ty>, b: Ty) {
        self.constraints.push(Constraint {
            kind: ConstraintKind::TypeEqual(a.ty.clone(), b),
            span: a.span,
        });
    }

    fn constrain_either_eq(&mut self, a: Ty, tys: (Ty, Ty), span: Span) {
        self.constraints.push(Constraint {
            kind: ConstraintKind::EitherTypeEqual(a, tys),
            span,
        });
    }

    fn clear_constraints(&mut self) {
        self.constraints.clear();
    }

    fn convert(&mut self, ast_ty: Option<&AstTy>) -> Ty {
        ast_ty.map_or_else(|| self.fresh_var(), Ty::from)
    }
}
