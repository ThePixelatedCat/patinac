mod error;
mod infer;
mod substitute;
#[cfg(test)]
mod test;
pub mod types;
mod unify;

use ena::unify::InPlaceUnificationTable;
use fnv::FnvHashMap;

use ast::{AdtItem, Ast, ExecItem, Expr, GenericParam};
use ident::Ident;
use span::{Span, Spannable};

use crate::error::{TypeError, TypeErrorS};
use crate::types::{Ty, TyVar};

#[derive(Default)]
struct AdtInfo {
    //generic_params: Vec<GenericParam>,
    fields: FnvHashMap<Ident, Ty>,
}

type AdtEnv = FnvHashMap<Ident, AdtInfo>;
type Env = im::HashMap<Ident, BindingInfo>;

#[derive(Clone)]
struct BindingInfo {
    ty: Ty,
    mutable: bool,
}

impl BindingInfo {
    const fn new(ty: Ty, mutable: bool) -> Self {
        Self { ty, mutable }
    }
}

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
    pub fn type_program(&mut self, Ast { adts, execs }: Ast<()>) -> Result<Ast<Ty>, TypeErrorS> {
        let mut ty_env = AdtEnv::default();

        ty_env.insert(Ident::new("String"), AdtInfo::default());

        for adt in &adts {
            match adt {
                AdtItem::Record { def, fields } => {
                    let fields = fields
                        .iter()
                        .map(|field| (field.ident, Ty::from(&field.ty)))
                        .collect();
                    ty_env.insert(
                        def.ident,
                        AdtInfo {
                            //generic_params: def.generics,
                            fields,
                        },
                    );
                }
                AdtItem::Enum { def, variants } => todo!(),
            }
        }

        let mut new_execs = Vec::new();
        for exec in execs {
            let new_exec = match exec {
                ExecItem::Const { ident, ty, val } => ExecItem::Const {
                    ident,
                    ty,
                    val: self.type_infer(&ty_env, val)?,
                },
                ExecItem::Func {
                    ident,
                    generic_params,
                    params,
                    return_ty,
                    body,
                } => todo!(),
            };
            new_execs.push(new_exec);
        }

        Ok(Ast {
            adts,
            execs: new_execs,
        })
    }

    pub fn type_infer(
        &mut self,
        ty_env: &AdtEnv,
        mut env: Env,
        expr: Expr<()>,
    ) -> Result<Expr<Ty>, TypeErrorS> {
        // Constraint generation
        let typed_expr = self.infer(ty_env, &mut env, expr)?;

        // Constraint solving
        self.unify()?;

        // Substitution
        let substituted_ast = self.sub_ast(typed_expr)?;

        Ok(substituted_ast)
    }

    fn fresh_var(&mut self) -> Ty {
        Ty::Var(self.table.new_key(None))
    }

    fn fresh_int_var(&mut self) -> Ty {
        Ty::IntVar(self.table.new_key(None))
    }

    fn constrain_eq(&mut self, a: Ty, b: Ty, span: Span) {
        self.constraints.push(Constraint {
            kind: ConstraintKind::TypeEqual(a, b),
            span,
        });
    }

    fn constrain_either_eq(&mut self, a: Ty, tys: (Ty, Ty), span: Span) {
        self.constraints.push(Constraint {
            kind: ConstraintKind::EitherTypeEqual(a, tys),
            span,
        });
    }

    fn get_field_ty(&self, base: Ty, field: Ident, span: Span) -> Result<Ty, TypeErrorS> {
        self.tys
            .get(&base)
            .ok_or_else(|| TypeError::UnknownType.span(span))?
            .fields
            .get(&field)
            .cloned()
            .ok_or_else(|| TypeError::MissingField.span(span))
    }

    // #[allow(
    //     clippy::ref_option,
    //     reason = "niche use-cases, avoids using as_ref at callsite"
    // )]
    // fn convert(&self, ast_ty: &Option<AstTypeS>) -> Ty {
    //     ast_ty
    //         .as_ref()
    //         .map_or_else(|| self.fresh_var(), |ty| ty.inner.clone().into())
    // }
}
