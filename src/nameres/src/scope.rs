use foldhash::{HashMap, HashMapExt as _};
use ident::Ident;
use irs::{
    ModuleId, Package,
    ast::Path,
    hir::{TyId, VarId},
};
use slotmap::SecondaryMap;

use crate::error::{ErrorKind, ItemKind};

#[derive(Debug)]
pub struct ScopeInfo<'pkg> {
    package: &'pkg Package,
    module: ModuleId,
    local_stack: Vec<HashMap<Ident, VarId>>,
    modules: SecondaryMap<ModuleId, ModuleScope>,
}

impl<'pkg> ScopeInfo<'pkg> {
    pub fn new(package: &'pkg Package) -> Self {
        let mut modules = SecondaryMap::new();
        modules.insert(package.root(), Self::new_module(package, package.root()));
        Self {
            package,
            module: package.root(),
            local_stack: Vec::new(),
            modules,
        }
    }

    pub fn push_scope(&mut self) {
        self.local_stack.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        self.local_stack.pop();
    }

    pub const fn module(&self) -> ModuleId {
        self.module
    }

    pub fn set_module(&mut self, module: ModuleId) {
        self.module = module;
        self.modules
            .entry(module)
            .expect("key still valid")
            .or_insert_with(|| Self::new_module(self.package, module));
    }

    fn new_module(package: &Package, module: ModuleId) -> ModuleScope {
        let mut scope = ModuleScope::default();
        scope.modules.extend(
            package
                .get(module)
                .children
                .iter()
                .copied()
                .map(|module| (package.get(module).name, (Visibility::Public, module))),
        );
        scope
    }

    pub fn add_ty(&mut self, vis: Visibility, ident: Ident, id: TyId) -> Option<TyId> {
        self.mod_scope_mut()
            .tys
            .insert(ident, (vis, id))
            .map(|(_, id)| id)
    }

    pub fn add_def(&mut self, vis: Visibility, ident: Ident, id: VarId) -> Option<VarId> {
        self.mod_scope_mut()
            .defs
            .insert(ident, (vis, id))
            .map(|(_, id)| id)
    }

    pub fn add_var(&mut self, ident: Ident, id: VarId) -> Option<VarId> {
        self.local_stack
            .last_mut()
            .expect("Tried to add local variable with no local scope on stack")
            .insert(ident, id)
    }

    pub fn resolve_ty(&self, path: &Path) -> Result<TyId, ErrorKind> {
        let module = self.resolve_base(path)?;
        let (vis, id) = self.modules[module]
            .get_ty(path.end())
            .ok_or_else(|| ErrorKind::UnknownName(ItemKind::Type, path.end()))?;
        if module != self.module && vis == Visibility::Private {
            Err(ErrorKind::PrivateItem(ItemKind::Type, path.end()))
        } else {
            Ok(id)
        }
    }

    pub fn resolve_var(&self, path: &Path) -> Result<VarId, ErrorKind> {
        if path.len() == 1 {
            self.local_stack
                .iter()
                .rev()
                .find_map(|scope| scope.get(&path.end()).copied())
                .or_else(|| self.mod_scope().get_def(path.end()).map(|(_, id)| id))
                .ok_or_else(|| ErrorKind::UnknownName(ItemKind::Value, path.end()))
        } else {
            let module = self.resolve_base(path)?;
            let (vis, id) = self.modules[module]
                .get_def(path.end())
                .ok_or_else(|| ErrorKind::UnknownName(ItemKind::Value, path.end()))?;
            if vis == Visibility::Private {
                Err(ErrorKind::PrivateItem(ItemKind::Value, path.end()))
            } else {
                Ok(id)
            }
        }
    }

    /// Resolves the module of a path, up until the final segment.
    pub fn resolve_base(&self, path: &Path) -> Result<ModuleId, ErrorKind> {
        let mut curr_mod = self.module;
        for segment in path.iter().take(path.len() - 1) {
            let (vis, new_mod) = self.get_child(curr_mod, segment)?;

            if curr_mod != self.module && vis == Visibility::Private {
                return Err(ErrorKind::PrivateItem(ItemKind::Module, path.end()));
            }

            curr_mod = new_mod;
        }
        Ok(curr_mod)
    }

    fn get_child(
        &self,
        parent: ModuleId,
        ident: Ident,
    ) -> Result<(Visibility, ModuleId), ErrorKind> {
        self.modules[parent]
            .get_mod(ident)
            .ok_or_else(|| ErrorKind::UnknownName(ItemKind::Module, ident))
    }

    pub fn import(&mut self, path: &Path) -> Result<(), ErrorKind> {
        let base = self.resolve_base(path)?;
        let module = self.modules[base].get_mod(path.end());
        let ty = self.modules[base].get_ty(path.end());
        let def = self.modules[base].get_def(path.end());

        if module.is_none() && ty.is_none() && def.is_none() {
            return Err(ErrorKind::UnknownName(ItemKind::Unknown, path.end()));
        }

        if let Some((vis, module)) = module {
            if base != self.module && vis == Visibility::Private {
                return Err(ErrorKind::PrivateItem(ItemKind::Module, path.end()));
            }

            if self
                .mod_scope_mut()
                .modules
                .insert(path.end(), (Visibility::Private, module))
                .is_some()
            {
                return Err(ErrorKind::DuplicateItem(ItemKind::Module, path.end()));
            }
        }

        if let Some((vis, ty)) = ty {
            if base != self.module && vis == Visibility::Private {
                return Err(ErrorKind::PrivateItem(ItemKind::Type, path.end()));
            }

            if self
                .mod_scope_mut()
                .tys
                .insert(path.end(), (Visibility::Private, ty))
                .is_some()
            {
                return Err(ErrorKind::DuplicateItem(ItemKind::Type, path.end()));
            }
        }

        if let Some((vis, def)) = def {
            if base != self.module && vis == Visibility::Private {
                return Err(ErrorKind::PrivateItem(ItemKind::Value, path.end()));
            }

            if self
                .mod_scope_mut()
                .defs
                .insert(path.end(), (Visibility::Private, def))
                .is_some()
            {
                return Err(ErrorKind::DuplicateItem(ItemKind::Value, path.end()));
            }
        }

        Ok(())
    }

    fn mod_scope(&self) -> &ModuleScope {
        &self.modules[self.module]
    }

    fn mod_scope_mut(&mut self) -> &mut ModuleScope {
        &mut self.modules[self.module]
    }
}

#[derive(Debug, Default)]
struct ModuleScope {
    defs: HashMap<Ident, (Visibility, VarId)>,
    tys: HashMap<Ident, (Visibility, TyId)>,
    modules: HashMap<Ident, (Visibility, ModuleId)>,
}

impl ModuleScope {
    pub fn get_mod(&self, ident: Ident) -> Option<(Visibility, ModuleId)> {
        self.modules.get(&ident).copied()
    }

    pub fn get_ty(&self, ident: Ident) -> Option<(Visibility, TyId)> {
        self.tys.get(&ident).copied()
    }

    pub fn get_def(&self, ident: Ident) -> Option<(Visibility, VarId)> {
        self.defs.get(&ident).copied()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Visibility {
    Private,
    Public,
}
