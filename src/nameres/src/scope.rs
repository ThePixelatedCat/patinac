use foldhash::fast::RandomState;

use ast::Path;
use hir::{TyId, VarId};
use ident::Ident;
use package::ModuleId;

type ImFoldHashMap<K, V> = im_rc::HashMap<K, V, RandomState>;

#[derive(Clone)]
pub struct Scope<'pkg> {
    module: ModuleId,
    mods: ImFoldHashMap<&'pkg str, Self>,
    tys: ImFoldHashMap<Ident, TyId>,
    vars: ImFoldHashMap<Ident, VarId>,
}

impl<'pkg> Scope<'pkg> {
    pub fn new(module: ModuleId) -> Self {
        Self {
            module,
            mods: ImFoldHashMap::default(),
            tys: ImFoldHashMap::default(),
            vars: ImFoldHashMap::default(),
        }
    }

    pub const fn module(&self) -> ModuleId {
        self.module
    }

    pub fn add_module(&mut self, name: &'pkg str, scope: Self) {
        self.mods.insert(name, scope);
    }

    pub fn add_ty(&mut self, ident: Ident, ty: TyId) {
        self.tys.insert(ident, ty);
    }

    pub fn add_var(&mut self, ident: Ident, var: VarId) {
        self.vars.insert(ident, var);
    }

    pub fn get_ty(&self, ident: Ident) -> Option<TyId> {
        self.tys.get(&ident).copied()
    }

    pub fn get_var(&self, ident: Ident) -> Option<VarId> {
        self.vars.get(&ident).copied()
    }

    pub fn resolve_ty(&self, path: Path) -> Option<TyId> {
        match path.split() {
            (start, None) => self.tys.get(&start).copied(),
            (start, Some(rest)) => self.mods.get(&*start.str())?.resolve_ty(rest),
        }
    }

    pub fn resolve_var(&self, path: Path) -> Option<VarId> {
        match path.split() {
            (start, None) => self.vars.get(&start).copied(),
            (start, Some(rest)) => self.mods.get(&*start.str())?.resolve_var(rest),
        }
    }
}
