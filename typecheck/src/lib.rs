mod error;
mod infer;
mod substitute;
#[cfg(test)]
mod test;
pub mod types;
mod unify;

use ast::{Ast, Expr};
use ena::unify::{InPlaceUnificationTable, UnificationTable};

use error::{TypeError, TypeErrorS};
use span::Span;
use string_interner::DefaultStringInterner;
use types::{Ty, TyVar};

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

pub struct TypeChecker<'a> {
    table: InPlaceUnificationTable<TyVar>,
    constraints: Vec<Constraint>,
    interner: &'a mut DefaultStringInterner,
}

impl<'a> TypeChecker<'a> {
    pub fn new(interner: &'a mut DefaultStringInterner) -> Self {
        Self {
            table: UnificationTable::default(),
            constraints: Vec::default(),
            interner,
        }
    }
}

impl TypeChecker<'_> {
    pub fn type_program(&mut self, ast: Ast<()>) -> Result<Ast<Ty>, TypeErrorS> {
        todo!()
    }

    pub fn type_infer(&mut self, expr: Expr<()>) -> Result<Expr<Ty>, TypeErrorS> {
        // Constraint generation
        let typed_expr = self.infer(&mut im::HashMap::default(), expr)?;

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
