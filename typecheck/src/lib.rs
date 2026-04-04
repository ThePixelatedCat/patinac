mod env;
mod error;
mod infer;
mod substitute;
#[cfg(test)]
mod test;
pub mod types;
mod unify;

use ena::unify::InPlaceUnificationTable;
use fnv::FnvHashMap;

use ast::{
    Ast,
    exprs::Expr,
    items::{AdtItem, ExecItem, Field},
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
                Ok(match exec {
                    ExecItem::Const { ident, ty, val } => ExecItem::Const {
                        ident,
                        ty,
                        val: self.type_infer(&ty_env, ctx.clone(), val)?,
                    },
                    ExecItem::Func {
                        ident,
                        generic_params,
                        params,
                        return_ty,
                        body,
                    } => todo!(),
                })
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

    pub fn type_infer(
        &mut self,
        ty_env: &TyEnv,
        mut ctx: Ctx,
        expr: Expr<()>,
    ) -> Result<Expr<ConcreteTy>> {
        self.clear_constraints();

        // Constraint generation
        let typed_expr = self.infer(ty_env, &mut ctx, expr)?;

        // Constraint solving
        self.unify()?;

        // Substitution
        let substituted_ast = self.sub_expr(typed_expr)?;

        Ok(substituted_ast)
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
