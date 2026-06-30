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

    pub fn add_ty(&mut self, path: &Path, ty: TyId) {
        self.scope_mut().insert(path, Some(ty), None);
    }

    pub fn add_var(&mut self, path: &Path, var: VarId) {
        self.scope_mut().insert(path, None, Some(var));
    }

    pub fn get_ty(&self, path: &Path) -> Option<TyId> {
        self.get(path, |(ty, _)| ty)
    }

    pub fn get_var(&self, path: &Path) -> Option<VarId> {
        self.get(path, |(_, var)| var)
    }

    fn get<T>(
        &self,
        path: &Path,
        pick_elem: impl Fn((Option<TyId>, Option<VarId>)) -> Option<T>,
    ) -> Option<T> {
        self.stack
            .iter()
            .rev()
            .find_map(|scope| pick_elem(scope.get(path)))
            .or_else(|| pick_elem(self.root.get(path)))
    }

    pub fn import(&mut self, path: &Path) -> Result<(), ErrorKind> {
        match self.root.get(path) {
            (None, None) => Err(ErrorKind::UnknownItem(path.end())),
            (ty, var) => match self.root.insert(&path.end().into(), ty, var) {
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

    fn scope_mut(&mut self) -> &mut ScopeNode {
        self.stack.last_mut().unwrap_or(&mut self.root)
    }
}

#[derive(Debug, Default)]
struct ScopeNode(ScopeNodeInner);

impl ScopeNode {
    fn get(&self, path: &Path) -> (Option<TyId>, Option<VarId>) {
        self.get_node(path)
            .map_or((None, None), |node| (node.ty, node.var))
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

// impl<T> PathTrieNode<T> {
//     fn get_child(&self, ident: Ident) -> Option<&PathTrieNode<T>> {
//         self.children.iter().find(|child| child.prefix == ident)
//     }

//     fn get_child_mut(&mut self, ident: Ident) -> Option<&mut PathTrieNode<T>> {
//         self.children.iter_mut().find(|child| child.prefix == ident)
//     }

//     fn insert_child(&mut self, ident: Ident, value: Option<T>) {
//         self.children.push(Self {
//             prefix: ident,
//             value,
//             children: Vec::new(),
//         });
//     }

//     fn insert_child_mut(&mut self, ident: Ident, value: Option<T>) -> &mut Self {
//         self.children.push_mut(Self {
//             prefix: ident,
//             value,
//             children: Vec::new(),
//         })
//     }
// }
