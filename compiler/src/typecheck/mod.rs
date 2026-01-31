mod error;
mod infer;
#[cfg(test)]
mod test;
mod types;
mod unify;

use std::{collections::HashMap, iter, slice};

use crate::{
    helpers::{Span, Spanned},
    parser::ast::{Ast, Binding, BindingS, Bop, Expr, ExprS, Item, Unop},
    typecheck::{
        error::TypeErrorS,
        types::{Type, TypeId, TypeS},
    },
};

use ena::unify::{InPlace, UnificationTable, UnifyKey};
use error::{TypeError, TypeResult};

#[derive(Clone)]
pub struct BindingInfo {
    ty: Type,
    mutable: bool,
}

#[derive(Clone, Default)]
pub struct TypeChecker {
    env: HashMap<String, BindingInfo>,
    table: UnificationTable<InPlace<TypeId>>,
}

impl TypeChecker {
    fn get_binding(&self, ident: Spanned<&str>) -> Result<&BindingInfo, TypeErrorS> {
        self.env
            .get(ident.inner)
            .ok_or_else(|| TypeError::UnboundIdent(ident.inner.to_owned()).spanned(ident.span))
    }

    fn fresh_int_var(&mut self) -> Type {
        let id = self.table.len();
        let var = Type::IntVar((id as u32).into());
        self.table.new_key(var.clone());
        var
    }

    fn fresh_var(&mut self) -> Type {
        let id = self.table.len();
        let var = Type::Var((id as u32).into());
        self.table.new_key(var.clone());
        var
    }

    fn normalize(&mut self, ty: &TypeS) -> Option<TypeS> {
        ty.inner
            .id()
            .and_then(|var| match self.table.probe_value(var) {
                Type::Var(_) => None,
                bound_ty => Some(bound_ty.clone().spanned(ty.span)),
            })
    }

    pub fn new(ast: &Ast) -> Self {
        let mut new = Self {
            env: HashMap::with_capacity(ast.len() * 2 / 3),
            table: UnificationTable::new(),
        };

        // for item in ast {
        //     match &item.inner {
        //         Item::Const { name, ty, value } => {
        //             new.env.insert(name.clone(), BindingInfo { ty: Type::from(&ty.inner), mutable: false });
        //         },
        //         Item::Function {
        //             name,
        //             params,
        //             return_type,
        //             body,
        //         } => {
        //             new.env.insert(name.clone(), BindingInfo { ty: Type::Fn { params: params.iter().map(|Spanned { inner: Binding::Var { mutable,  }, .. }| b.inner.), result: Box::new(Type::from(&return_type.unwrap().inner)) }, mutable: false });
        //         },
        //         Item::Struct {
        //             name,
        //             generic_params,
        //             fields,
        //         } => todo!(),
        //         Item::Enum {
        //             name,
        //             generic_params,
        //             variants,
        //         } => todo!(),
        //     }
        // }

        new
    }
}
