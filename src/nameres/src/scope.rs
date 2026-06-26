use foldhash::fast::RandomState;

use ident::Ident;
use imbl::{GenericHashMap, shared_ptr::RcK};
use irs::{
    ast::Path,
    hir::{TyId, VarId},
};
use package::ModuleId;

use crate::error::{ErrorKind, ItemKind};

type Table<T> = GenericHashMap<Ident, (T, Visibility), RandomState, RcK>;

#[derive(Clone, PartialEq)]
pub struct Scope {
    module: ModuleId,
    mods: Table<Self>,
    tys: Table<TyId>,
    vars: Table<VarId>,
}

impl Scope {
    pub fn new(module: ModuleId) -> Self {
        Self {
            module,
            mods: Table::default(),
            tys: Table::default(),
            vars: Table::default(),
        }
    }

    pub const fn module(&self) -> ModuleId {
        self.module
    }

    pub fn add_module(&mut self, ident: Ident, scope: Self) {
        self.mods.insert(ident, (scope, Visibility::Local));
    }

    pub fn add_ty(&mut self, ident: Ident, ty: TyId) {
        self.tys.insert(ident, (ty, Visibility::Local));
    }

    pub fn add_var(&mut self, ident: Ident, var: VarId) {
        self.vars.insert(ident, (var, Visibility::Local));
    }

    pub fn import(&mut self, path: Path) -> Result<(), ErrorKind> {
        let (module, ident) = self.resolve_start(path)?;
        let module = module.ok_or(ErrorKind::SelfImport)?;

        let (mut add_scope, mut add_ty, mut add_var) = (None, None, None);
        if let Some((scope, vis)) = module.mods.get(&ident) {
            if self.mods.contains_key(&ident) {
                return Err(ErrorKind::DupItem(ident));
            }
            check_vis(*vis, ident, ItemKind::Module)?;
            add_scope = Some(scope.clone());
        }

        if let Some((ty, vis)) = module.tys.get(&ident) {
            if self.tys.contains_key(&ident) {
                return Err(ErrorKind::DupItem(ident));
            }
            check_vis(*vis, ident, ItemKind::Type)?;
            add_ty = Some(*ty);
        }

        if let Some((var, vis)) = module.vars.get(&ident) {
            if self.vars.contains_key(&ident) {
                return Err(ErrorKind::DupItem(ident));
            }
            check_vis(*vis, ident, ItemKind::Value)?;
            add_var = Some(*var);
        }

        if (&add_scope, add_ty, add_var) == (&None, None, None) {
            return Err(ErrorKind::UnknownItem(ItemKind::Unknown, ident));
        }

        if let Some(scope) = add_scope {
            self.mods.insert(ident, (scope, Visibility::Private));
        }
        if let Some(ty) = add_ty {
            self.tys.insert(ident, (ty, Visibility::Private));
        }
        if let Some(var) = add_var {
            self.vars.insert(ident, (var, Visibility::Private));
        }

        Ok(())
    }

    pub fn export(&mut self, ident: Ident) -> Result<(), ErrorKind> {
        let success = if let Some((_, vis)) = self.mods.get_mut(&ident) {
            match vis {
                Visibility::Private => return Err(ErrorKind::Reexport),
                _ => *vis = Visibility::Public,
            }
            true
        } else if let Some((_, vis)) = self.tys.get_mut(&ident) {
            match vis {
                Visibility::Private => return Err(ErrorKind::Reexport),
                _ => *vis = Visibility::Public,
            }
            true
        } else if let Some((_, vis)) = self.vars.get_mut(&ident) {
            match vis {
                Visibility::Private => return Err(ErrorKind::Reexport),
                _ => *vis = Visibility::Public,
            }
            true
        } else {
            false
        };

        if success {
            Ok(())
        } else {
            Err(ErrorKind::UnknownItem(ItemKind::Unknown, ident))
        }
    }

    pub fn get_ty(&self, ident: Ident) -> Option<TyId> {
        self.tys.get(&ident).map(|(ty, _)| *ty)
    }

    pub fn get_var(&self, ident: Ident) -> Option<VarId> {
        self.vars.get(&ident).map(|(var, _)| *var)
    }

    fn resolve_start(&self, path: Path) -> Result<(Option<&Self>, Ident), ErrorKind> {
        match path.split() {
            (start, None) => Ok((None, start)),
            (start, Some(rest)) => {
                let mut module = &self
                    .mods
                    .get(&start)
                    .ok_or(ErrorKind::UnknownItem(ItemKind::Module, start))?
                    .0;
                let mut path = rest;
                loop {
                    match path.split() {
                        (start, None) => break Ok((Some(module), start)),
                        (start, Some(rest)) => {
                            let (new_module, vis) = module
                                .mods
                                .get(&start)
                                .ok_or(ErrorKind::UnknownItem(ItemKind::Module, start))?;
                            check_vis(*vis, start, ItemKind::Module)?;
                            module = new_module;
                            path = rest;
                        }
                    }
                }
            }
        }
    }

    fn resolve<T>(
        &self,
        get_table: impl Fn(&Self) -> &Table<T>,
        path: Path,
        item_kind: ItemKind,
    ) -> Result<&T, ErrorKind> {
        let (module, ident) = self.resolve_start(path)?;
        match module {
            None => get_table(self)
                .get(&ident)
                .map(|(item, _)| item)
                .ok_or(ErrorKind::UnknownItem(item_kind, ident)),
            Some(module) => {
                let (item, vis) = get_table(module)
                    .get(&ident)
                    .ok_or(ErrorKind::UnknownItem(item_kind, ident))?;
                check_vis(*vis, ident, item_kind)?;
                Ok(item)
            }
        }
    }

    pub fn resolve_ty(&self, path: Path) -> Result<TyId, ErrorKind> {
        self.resolve(|this| &this.tys, path, ItemKind::Type)
            .copied()
    }

    pub fn resolve_var(&self, path: Path) -> Result<VarId, ErrorKind> {
        self.resolve(|this| &this.vars, path, ItemKind::Value)
            .copied()
    }
}

fn check_vis(vis: Visibility, name: Ident, kind: ItemKind) -> Result<(), ErrorKind> {
    if vis == Visibility::Public {
        Ok(())
    } else {
        Err(ErrorKind::NotVisible(kind, name))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Visibility {
    /// Only visible within this module.
    Private,
    /// Visible within this module and any child modules. Default.
    Local,
    /// Visible within this module, it's parent, and any child modules.
    Public,
}
