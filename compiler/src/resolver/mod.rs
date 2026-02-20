use ego_tree::{NodeId, NodeMut, NodeRef, Tree};
use fnv::FnvHashMap;

use crate::{
    ast::{AdtDef, Item},
    hir::Hir,
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
                Item::Const { ident, ty, value } => todo!(),
                Item::Func {
                    ident,
                    params,
                    return_ty,
                    body,
                } => todo!(),
                Item::Record {
                    def: AdtDef { ident, generics },
                    fields,
                } => {
                    self.hir.type_defs.add_record(ident, generics, data);
                }
                Item::Enum {
                    def: AdtDef { ident, generics },
                    variants,
                } => {
                    self.hir.type_defs.add_enum(ident, generics, data);
                }
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
