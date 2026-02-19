use ego_tree::{NodeId, NodeMut, NodeRef, Tree};
use fnv::FnvHashMap;

use crate::{
    hir::Hir,
    ast::{Item, AstDef},
    typecheck::types::Type,
};

pub struct Resolver {
    scopes: Tree<Scope>,
    hir: Hir,
}

impl Default for Resolver {
    fn default() -> Self {
        Self {
            scopes: Tree::new(Scope::default()),
            hir: Hir::default(),
        }
    }
}

impl Resolver {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn resolve(&mut self, ast: Vec<Item>) {
        for item in ast {
            match item {
                Item::Const { name, ty, value } => todo!(),
                Item::Func {
                    name,
                    params,
                    return_ty,
                    body,
                } => todo!(),
                Item::Record {
                    def:
                        AstDef {
                            name,
                            generics: generic_params,
                        },
                    data,
                } => {
                    self.hir.type_defs.add_record(name, generic_params, fields);
                }
                Item::Enum { def, variants } => todo!(),
            }
        }

        todo!()
    }

    pub fn global_scope(&mut self) -> ScopeMut<'_> {
        self.scopes.root_mut()
    }

    pub fn parent_of(&self, scope: NodeId) -> Option<NodeId> {
        Some(self.scopes.get(scope)?.parent()?.id())
    }
}

#[derive(Clone)]
pub struct VarInfo {
    ty: Option<Type>,
    mutable: bool,
}

pub type ScopeRef<'a> = NodeRef<'a, Scope>;
pub type ScopeMut<'a> = NodeMut<'a, Scope>;

#[derive(Default)]
pub struct Scope {
    vars: FnvHashMap<String, VarInfo>,
}
