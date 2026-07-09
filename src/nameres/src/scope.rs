use foldhash::{HashMap, HashMapExt as _};
use ident::Ident;
use irs::{
    ModuleId, Package,
    ast::Path,
    hir::{Ty, TyId, VarId},
};
use slotmap::SecondaryMap;

use crate::error::ErrorKind;

#[derive(Debug)]
pub struct ScopeInfo<'pkg> {
    package: &'pkg Package,
    module: ModuleId,
    modules: SecondaryMap<ModuleId, ModuleScope>,
    assocs: HashMap<(Ty, Ident), VarId>,
}

impl<'pkg> ScopeInfo<'pkg> {
    pub fn new(package: &'pkg Package) -> Self {
        let mut modules = SecondaryMap::new();
        modules.insert(package.root(), ModuleScope::new(package, package.root()));
        Self {
            package,
            module: package.root(),
            modules,
            assocs: HashMap::new(),
        }
    }

    pub fn push_ty_scope(&mut self) {
        self.mod_scope_mut().ty_stack.push(HashMap::new());
    }

    pub fn pop_ty_scope(&mut self) {
        self.mod_scope_mut().ty_stack.pop();
    }

    pub fn push_var_scope(&mut self) {
        self.mod_scope_mut().var_stack.push(HashMap::new());
    }

    pub fn pop_var_scope(&mut self) {
        self.mod_scope_mut().var_stack.pop();
    }

    pub const fn module(&self) -> ModuleId {
        self.module
    }

    pub fn set_module(&mut self, module: ModuleId) {
        self.module = module;
        self.modules
            .entry(module)
            .expect("key still valid")
            .or_insert_with(|| ModuleScope::new(self.package, module));
    }

    // pub fn add_path(&mut self, path: &Path) {
    //     self.module_mut().insert(path, None, None);
    // }

    pub fn add_ty(&mut self, ident: Ident, ty: TyId) -> Option<TyScopeItem> {
        self.mod_scope_mut()
            .ty_scope_mut()
            .insert(ident, TyScopeItem::Ty(ty))
    }

    pub fn add_var(&mut self, ident: Ident, var: VarId) -> Option<VarId> {
        self.mod_scope_mut().var_scope_mut().insert(ident, var)
    }

    pub fn resolve_ty(&self, path: &Path) -> Option<TyId> {
        let mut curr_mod = self.mod_scope();
        for segment in path.iter().take(path.len() - 1) {
            curr_mod = &self.modules[curr_mod.get_mod(segment)?];
        }

        curr_mod.get_ty(path.end())
    }

    pub fn resolve_var(&self, path: &Path) -> Option<VarId> {
        let mut curr_mod = self.mod_scope();
        for segment in path.iter().take(path.len() - 1) {
            match curr_mod.get_ty_item(segment)? {
                TyScopeItem::Ty(ty) => {
                    todo!()
                    //return self.assocs.get((ty,))
                }
                TyScopeItem::Module(module) => curr_mod = &self.modules[module],
            }
        }

        curr_mod.get_var(path.end())
    }

    // pub fn get_ty(&self, path: &Path) -> Option<TyId> {
    //     self.get(path).and_then(|(ty, _)| ty)
    // }

    // pub fn get_var(&self, path: &Path) -> Option<VarId> {
    //     self.get(path).and_then(|(_, var)| var)
    // }

    // fn get(&self, path: &Path) -> Option<(Option<TyId>, Option<VarId>)> {
    //     self.stack
    //         .iter()
    //         .rev()
    //         .find_map(|scope| scope.get(path))
    //         .or_else(|| self.root.get(path))
    // }

    // pub fn import(&mut self, path: &Path) -> Result<(), ErrorKind> {
    //     match self.root.get(path) {
    //         None | Some((None, None)) => Err(ErrorKind::UnknownItem(path.end())),
    //         Some((ty, var)) => match self.root.insert(&path.end().into(), ty, var) {
    //             (old_ty, old_var)
    //                 if (old_ty.is_some() && ty.is_some())
    //                     || (old_var.is_some() && var.is_some()) =>
    //             {
    //                 Err(ErrorKind::DupItem(path.end()))
    //             }
    //             _ => Ok(()),
    //         },
    //     }
    // }

    // pub fn export(&mut self, name: Ident, ident: Ident, to: &mut Self) -> Result<(), ErrorKind> {
    //     match self.root.get(&ident.into()) {
    //         None => Err(ErrorKind::UnknownItem(ident)),
    //         Some((ty, var)) => {
    //             to.root.insert(&Path::new_const([name, ident]), ty, var);
    //             Ok(())
    //         }
    //     }
    // }

    fn mod_scope(&self) -> &ModuleScope {
        &self.modules[self.module]
    }

    fn mod_scope_mut(&mut self) -> &mut ModuleScope {
        &mut self.modules[self.module]
    }
}

#[derive(Debug)]
struct ModuleScope {
    var_stack: Vec<HashMap<Ident, VarId>>,
    ty_stack: Vec<HashMap<Ident, TyScopeItem>>,
}

#[derive(Debug, Clone, Copy)]
pub enum TyScopeItem {
    Ty(TyId),
    Module(ModuleId),
}

impl ModuleScope {
    fn new(package: &Package, module: ModuleId) -> Self {
        let children = package
            .get(module)
            .children
            .iter()
            .map(|&child| (package.get(child).name, TyScopeItem::Module(child)))
            .collect();
        Self {
            var_stack: vec![HashMap::new()],
            ty_stack: vec![children],
        }
    }

    fn get_ty_item(&self, ident: Ident) -> Option<TyScopeItem> {
        self.ty_stack
            .iter()
            .rev()
            .find_map(|scope| scope.get(&ident))
            .copied()
    }

    fn get_mod(&self, ident: Ident) -> Option<ModuleId> {
        self.get_ty_item(ident).and_then(|item| match item {
            TyScopeItem::Ty(_) => None,
            TyScopeItem::Module(module) => Some(module),
        })
    }

    fn get_ty(&self, ident: Ident) -> Option<TyId> {
        self.get_ty_item(ident).and_then(|item| match item {
            TyScopeItem::Ty(ty) => Some(ty),
            TyScopeItem::Module(_) => None,
        })
    }

    fn get_var(&self, ident: Ident) -> Option<VarId> {
        self.var_stack
            .iter()
            .rev()
            .find_map(|scope| scope.get(&ident))
            .copied()
    }

    fn var_scope_mut(&mut self) -> &mut HashMap<Ident, VarId> {
        self.var_stack.last_mut().expect("at least one scope")
    }

    fn ty_scope_mut(&mut self) -> &mut HashMap<Ident, TyScopeItem> {
        self.ty_stack.last_mut().expect("at least one scope")
    }
}

// #[derive(Debug, Default)]
// struct ScopeNode(ScopeNodeInner);

// impl ScopeNode {
//     fn get(&self, path: &Path) -> Option<(Option<TyId>, Option<VarId>)> {
//         self.get_node(path).map(|node| (node.ty, node.var))
//     }

//     fn get_node(&self, path: &Path) -> Option<&ScopeNodeInner> {
//         let mut curr_node = &self.0;
//         for ident in path.iter() {
//             curr_node = &curr_node
//                 .children
//                 .iter()
//                 .find(|(prefix, _)| *prefix == ident)?
//                 .1;
//         }
//         Some(curr_node)
//     }

//     fn insert(
//         &mut self,
//         path: &Path,
//         ty: Option<TyId>,
//         var: Option<VarId>,
//     ) -> (Option<TyId>, Option<VarId>) {
//         let mut curr_node = &mut self.0;
//         for ident in path.iter() {
//             let index = curr_node
//                 .children
//                 .iter()
//                 .position(|(prefix, _)| *prefix == ident);
//             curr_node = match index {
//                 Some(index) => &mut curr_node.children[index].1,
//                 None => {
//                     &mut curr_node
//                         .children
//                         .push_mut((
//                             ident,
//                             ScopeNodeInner {
//                                 ty: None,
//                                 var: None,
//                                 children: Vec::new(),
//                             },
//                         ))
//                         .1
//                 }
//             };
//         }

//         let (mut old_ty, mut old_var) = (None, None);
//         if let Some(ty) = ty {
//             old_ty = curr_node.ty.replace(ty);
//         }
//         if let Some(var) = var {
//             old_var = curr_node.var.replace(var);
//         }
//         (old_ty, old_var)
//     }
// }

// #[derive(Debug, Default, Clone)]
// struct ScopeNodeInner {
//     ty: Option<TyId>,
//     var: Option<VarId>,
//     children: Vec<(Ident, Self)>,
// }
