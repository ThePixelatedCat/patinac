use foldhash::{HashMap, HashMapExt as _};
use ident::Ident;
use irs::{
    ModuleId, Package,
    ast::Path,
    hir::{TyId, VarId},
};
use slotmap::SecondaryMap;

use crate::error::ErrorKind;

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
        modules.insert(package.root(), ModuleScope::new());
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
            .or_default();
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
            .ok_or_else(|| ErrorKind::UnknownType(path.end()))?;
        if module != self.module && vis == Visibility::Private {
            Err(ErrorKind::PrivateItem(path.end()))
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
                .ok_or_else(|| ErrorKind::UnknownVar(path.end()))
        } else {
            let module = self.resolve_base(path)?;
            let (vis, id) = self.modules[module]
                .get_def(path.end())
                .ok_or_else(|| ErrorKind::UnknownVar(path.end()))?;
            if module != self.module && vis == Visibility::Private {
                Err(ErrorKind::PrivateItem(path.end()))
            } else {
                Ok(id)
            }
        }
    }

    /// Resolves the module of a path, up until the final segment.
    pub fn resolve_base(&self, path: &Path) -> Result<ModuleId, ErrorKind> {
        let mut curr_mod = self.module;
        for segment in path.iter().take(path.len() - 1) {
            curr_mod = self.get_child(curr_mod, segment)?;
        }
        Ok(curr_mod)
    }

    fn get_child(&self, parent: ModuleId, ident: Ident) -> Result<ModuleId, ErrorKind> {
        self.package
            .get(parent)
            .children
            .iter()
            .copied()
            .find(|&child| self.package.get(child).name == ident)
            .ok_or_else(|| ErrorKind::UnknownModule(ident))
    }

    pub fn import(&mut self, path: &Path) -> Result<(), ErrorKind> {
        let module = self.resolve_base(path)?;
        let ty = self.modules[module].get_ty(path.end());
        let def = self.modules[module].get_def(path.end());

        if ty.is_none() && def.is_none() {
            return Err(ErrorKind::UnknownItem(path.end()));
        }
        todo!()

        // if module != self.module && vis == Visibility::Private {
        //     Err(ErrorKind::PrivateItem(path.end()))
        // } else {
        //     Ok(id)
        // }
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
    //child_visibilities: SecondaryMap<ModuleId, Visibility>,
}

impl ModuleScope {
    fn new() -> Self {
        Self::default()
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
