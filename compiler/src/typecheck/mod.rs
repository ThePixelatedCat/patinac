mod check;
mod error;
mod infer;
#[cfg(test)]
mod test;
pub mod types;
mod unify;

use std::{cell::RefCell, rc::Rc};

use ena::unify::{InPlaceUnificationTable, UnifyKey};
use im::HashMap;

use crate::parser::ast::TypeS as AstTypeS;

use error::{TypeError, TypeErrorS};
use types::{Type, TypeId};

#[derive(Clone, Default)]
pub struct TypeChecker {
    table: Rc<RefCell<InPlaceUnificationTable<TypeId>>>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self::default()
    }

    fn get_binding(&self, ident: &str) -> Result<&BindingInfo, TypeError> {
        self.env
            .get(ident)
            .ok_or_else(|| TypeError::UnboundIdent(ident.to_owned()))
    }

    #[allow(clippy::cast_possible_truncation)]
    fn fresh_int_var(&self) -> Type {
        let id = self.table.borrow_mut().len() as u32;
        let var = Type::IntVar(TypeId::from_index(id));
        self.table.borrow_mut().new_key(var.clone());
        var
    }

    #[allow(clippy::cast_possible_truncation)]
    fn fresh_var(&self) -> Type {
        let id = self.table.borrow_mut().len() as u32;
        let var = Type::Var(TypeId::from_index(id));
        self.table.borrow_mut().new_key(var.clone());
        var
    }

    fn normalise_id(&self, ty: &Type) -> Option<Type> {
        ty.id()
            .and_then(|id| match self.table.borrow_mut().probe_value(id) {
                Type::Var(_) | Type::IntVar(_) => None,
                bound_ty => Some(bound_ty),
            })
    }

    fn normalise(&self, ty: Type) -> Type {
        match ty {
            Type::Int | Type::UInt | Type::Byte | Type::Float | Type::Bool | Type::Char => ty,
            Type::Array(mut ty) => {
                *ty = self.normalise(*ty);
                Type::Array(ty)
            }
            Type::Tuple(tys) => Type::Tuple(tys.into_iter().map(|ty| self.normalise(ty)).collect()),
            Type::Fn(param_tys, mut return_ty) => {
                *return_ty = self.normalise(*return_ty);
                Type::Fn(
                    param_tys.into_iter().map(|ty| self.normalise(ty)).collect(),
                    return_ty,
                )
            }
            Type::Var(id) | Type::IntVar(id) => self.table.borrow_mut().probe_value(id),
            Type::Adt(name, args) => Type::Adt(
                name,
                args.into_iter().map(|ty| self.normalise(ty)).collect(),
            ),
        }
    }

    #[allow(
        clippy::ref_option,
        reason = "niche use-cases, avoids using as_ref at callsite"
    )]
    fn convert(&self, ast_ty: &Option<AstTypeS>) -> Type {
        ast_ty
            .as_ref()
            .map_or_else(|| self.fresh_var(), |ty| ty.inner.clone().into())
    }
}
