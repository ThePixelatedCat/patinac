use foldhash::fast::RandomState;

use ast::Path;
use hir::{TyId, VarId};
use ident::Ident;
use imbl::{GenericHashMap, shared_ptr::RcK};
use package::ModuleId;

use crate::error::{ErrorKind, ItemKind};
#[derive(Clone)]
pub struct Scope {
    module: ModuleId,
    scope: GenericHashMap<Ident, (ScopeItem, Visibility), RandomState, RcK>,
}

impl Scope {
    pub fn new(module: ModuleId) -> Self {
        Self {
            module,
            scope: GenericHashMap::default(),
        }
    }

    pub const fn module(&self) -> ModuleId {
        self.module
    }

    pub fn add_module(&mut self, ident: Ident, scope: Self) {
        self.scope
            .insert(ident, (ScopeItem::Module(scope), Visibility::Local));
    }

    pub fn add_ty(&mut self, ident: Ident, ty: TyId) {
        self.scope
            .insert(ident, (ScopeItem::Ty(ty), Visibility::Local));
    }

    pub fn add_var(&mut self, ident: Ident, var: VarId) {
        self.scope
            .insert(ident, (ScopeItem::Var(var), Visibility::Local));
    }

    pub fn import(&mut self, path: Path) -> Result<(), ErrorKind> {
        let name = path.end();
        if self.scope.contains_key(&name) {
            return Err(ErrorKind::DupItem(name));
        }
        let item = self.resolve(path, ItemKind::Unknown)?;
        self.scope.insert(name, (item.clone(), Visibility::Private));
        Ok(())
    }

    pub fn export(&mut self, ident: Ident) -> Result<(), ErrorKind> {
        match self.scope.get_mut(&ident) {
            Some(item) => match item.1 {
                Visibility::Private => Err(ErrorKind::Reexport),
                _ => {
                    item.1 = Visibility::Public;
                    Ok(())
                }
            },
            None => Err(ErrorKind::UnknownItem(ItemKind::Unknown, ident)),
        }
    }

    fn get_mod(&self, ident: Ident) -> Option<(&Scope, Visibility)> {
        match self.scope.get(&ident) {
            Some((ScopeItem::Module(module), vis)) => Some((module, *vis)),
            _ => None,
        }
    }

    pub fn get_ty(&self, ident: Ident) -> Option<TyId> {
        match self.scope.get(&ident) {
            Some((ScopeItem::Ty(ty), _)) => Some(*ty),
            _ => None,
        }
    }

    pub fn get_var(&self, ident: Ident) -> Option<VarId> {
        match self.scope.get(&ident) {
            Some((ScopeItem::Var(var), _)) => Some(*var),
            _ => None,
        }
    }

    fn resolve(&self, path: Path, item_kind: ItemKind) -> Result<&ScopeItem, ErrorKind> {
        match path.split() {
            (start, None) => self
                .scope
                .get(&start)
                .map(|(item, _)| item)
                .ok_or_else(|| ErrorKind::UnknownItem(item_kind, start)),
            (start, Some(rest)) => self
                .get_mod(start)
                .ok_or_else(|| ErrorKind::UnknownItem(ItemKind::Module, start))?
                .0
                .resolve_export(rest, item_kind),
        }
    }

    fn resolve_export(&self, path: Path, item_kind: ItemKind) -> Result<&ScopeItem, ErrorKind> {
        match path.split() {
            (start, None) => {
                let (item, vis) = self
                    .scope
                    .get(&start)
                    .ok_or_else(|| ErrorKind::UnknownItem(item_kind, start))?;
                if *vis == Visibility::Public {
                    Ok(item)
                } else {
                    Err(ErrorKind::NotVisible(item_kind, start))
                }
            }
            (start, Some(rest)) => {
                let (new_mod, vis) = self
                    .get_mod(start)
                    .ok_or_else(|| ErrorKind::UnknownItem(ItemKind::Module, start))?;
                if vis == Visibility::Public {
                    new_mod.resolve_export(rest, item_kind)
                } else {
                    Err(ErrorKind::NotVisible(ItemKind::Module, start))
                }
            }
        }
    }

    pub fn resolve_ty(&self, path: Path) -> Result<TyId, ErrorKind> {
        let name = path.end();
        let found_kind = match self.resolve(path, ItemKind::Type)? {
            ScopeItem::Ty(ty) => return Ok(*ty),
            ScopeItem::Module(_) => ItemKind::Module,
            ScopeItem::Var(_) => ItemKind::Value,
        };
        Err(ErrorKind::WrongKind(name, ItemKind::Type, found_kind))
    }

    pub fn resolve_var(&self, path: Path) -> Result<VarId, ErrorKind> {
        let name = path.end();
        let found_kind = match self.resolve(path, ItemKind::Value)? {
            ScopeItem::Var(var) => return Ok(*var),
            ScopeItem::Ty(_) => ItemKind::Type,
            ScopeItem::Module(_) => ItemKind::Module,
        };
        Err(ErrorKind::WrongKind(name, ItemKind::Value, found_kind))
    }
}

#[derive(Clone)]
enum ScopeItem {
    Module(Scope),
    Ty(TyId),
    Var(VarId),
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
