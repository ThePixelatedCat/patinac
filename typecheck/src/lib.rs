mod check;
mod error;
mod infer;
#[cfg(test)]
mod test;
pub mod types;
//mod unify;

use std::{cell::RefCell, rc::Rc};

use ena::unify::{InPlaceUnificationTable, UnifyKey};

use error::{TypeError, TypeErrorS};
use types::{Ty, TypeVar};

#[derive(Clone)]
enum Constraint {
    TypeEqual(Ty, Ty),
    EitherTypeEqual(Ty, (Ty, Ty)),
    Int(Ty),
}

#[derive(Clone, Default)]
pub struct TypeChecker {
    table: InPlaceUnificationTable<TypeVar>,
    constraints: Vec<Constraint>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(clippy::cast_possible_truncation)]
    fn fresh_var(&mut self) -> Ty {
        let id = self.table.len() as u32;
        let var = Ty::Var(TypeVar::from_index(id));
        self.table.new_key(var.clone());
        var
    }

    fn constrain_eq(&mut self, a: Ty, b: Ty) {
        self.constraints.push(Constraint::TypeEqual(a, b));
    }

    fn constrain_either_eq(&mut self, a: Ty, tys: (Ty, Ty)) {
        self.constraints.push(Constraint::EitherTypeEqual(a, tys));
    }

    fn constrain_int(&mut self, a: Ty) {
        self.constraints.push(Constraint::Int(a));
    }

    #[allow(
        clippy::ref_option,
        reason = "niche use-cases, avoids using as_ref at callsite"
    )]
    fn convert(&self, ast_ty: &Option<AstTypeS>) -> Ty {
        ast_ty
            .as_ref()
            .map_or_else(|| self.fresh_var(), |ty| ty.inner.clone().into())
    }
}
