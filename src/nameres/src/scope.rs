use ident::Ident;
use irs::{
    ModuleId,
    ast::Path,
    hir::{TyId, VarId},
};

use crate::error::ErrorKind;

#[derive(Debug)]
pub struct Scope {
    module: ModuleId,
    root: ScopeNode,
    stack: Vec<ScopeNode>,
}

impl Scope {
    pub fn new(module: ModuleId) -> Self {
        Self {
            module,
            root: ScopeNode::default(),
            stack: Vec::new(),
        }
    }

    pub fn push(&mut self) {
        self.stack.push(ScopeNode::default());
    }

    pub fn pop(&mut self) {
        self.stack.pop();
    }

    pub const fn module(&self) -> ModuleId {
        self.module
    }

    pub fn add_path(&mut self, path: &Path) {
        self.scope_mut().insert(path, None, None);
    }

    pub fn add_ty(&mut self, path: &Path, ty: TyId) -> Option<TyId> {
        self.scope_mut().insert(path, Some(ty), None).0
    }

    pub fn add_var(&mut self, path: &Path, var: VarId) -> Option<VarId> {
        self.scope_mut().insert(path, None, Some(var)).1
    }

    pub fn get_ty(&self, path: &Path) -> Option<TyId> {
        self.get(path).and_then(|(ty, _)| ty)
    }

    pub fn get_var(&self, path: &Path) -> Option<VarId> {
        self.get(path).and_then(|(_, var)| var)
    }

    fn get(&self, path: &Path) -> Option<(Option<TyId>, Option<VarId>)> {
        self.stack
            .iter()
            .rev()
            .find_map(|scope| scope.get(path))
            .or_else(|| self.root.get(path))
    }

    pub fn import(&mut self, path: &Path) -> Result<(), ErrorKind> {
        match self.root.get(path) {
            None | Some((None, None)) => Err(ErrorKind::UnknownItem(path.end())),
            Some((ty, var)) => match self.root.insert(&path.end().into(), ty, var) {
                (old_ty, old_var)
                    if (old_ty.is_some() && ty.is_some())
                        || (old_var.is_some() && var.is_some()) =>
                {
                    Err(ErrorKind::DupItem(path.end()))
                }
                _ => Ok(()),
            },
        }
    }

    pub fn export(&mut self, name: Ident, ident: Ident, to: &mut Self) -> Result<(), ErrorKind> {
        match self.root.get(&ident.into()) {
            None => Err(ErrorKind::UnknownItem(ident)),
            Some((ty, var)) => {
                to.root.insert(&Path::new_const([name, ident]), ty, var);
                Ok(())
            }
        }
    }

    fn scope_mut(&mut self) -> &mut ScopeNode {
        self.stack.last_mut().unwrap_or(&mut self.root)
    }
}

#[derive(Debug, Default)]
struct ScopeNode(ScopeNodeInner);

impl ScopeNode {
    fn get(&self, path: &Path) -> Option<(Option<TyId>, Option<VarId>)> {
        self.get_node(path).map(|node| (node.ty, node.var))
    }

    fn get_node(&self, path: &Path) -> Option<&ScopeNodeInner> {
        let mut curr_node = &self.0;
        for ident in path.iter() {
            curr_node = &curr_node
                .children
                .iter()
                .find(|(prefix, _)| *prefix == ident)?
                .1;
        }
        Some(curr_node)
    }

    fn insert(
        &mut self,
        path: &Path,
        ty: Option<TyId>,
        var: Option<VarId>,
    ) -> (Option<TyId>, Option<VarId>) {
        let mut curr_node = &mut self.0;
        for ident in path.iter() {
            let index = curr_node
                .children
                .iter()
                .position(|(prefix, _)| *prefix == ident);
            curr_node = match index {
                Some(index) => &mut curr_node.children[index].1,
                None => {
                    &mut curr_node
                        .children
                        .push_mut((
                            ident,
                            ScopeNodeInner {
                                ty: None,
                                var: None,
                                children: Vec::new(),
                            },
                        ))
                        .1
                }
            };
        }

        let (mut old_ty, mut old_var) = (None, None);
        if let Some(ty) = ty {
            old_ty = curr_node.ty.replace(ty);
        }
        if let Some(var) = var {
            old_var = curr_node.var.replace(var);
        }
        (old_ty, old_var)
    }
}

#[derive(Debug, Default, Clone)]
struct ScopeNodeInner {
    ty: Option<TyId>,
    var: Option<VarId>,
    children: Vec<(Ident, Self)>,
}
