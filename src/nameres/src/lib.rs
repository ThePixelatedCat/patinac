//! Lowers all [`Asts`][Ast] into a single [`Hir`], resolving stringly variable and type names into numeric identifiers along the way.

mod error;
mod exprs;
mod scope;

use std::range::Range;

use foldhash::{HashMap, HashMapExt as _};
use slotmap::SecondaryMap;

use errors::{ErrorHandler, HandledError, Result, TryCollectEager as _};
use irs::{
    ModuleId, Package,
    ast::{self, Ast, Binding, BlockItem, Import, Pat, PatKind, Path, TyItem, TyItemKind, TyKind},
    hir::{self, Field, Hir, Param, TyInfo, VarId, VarInfo},
};

use crate::{
    error::{ErrorKind, ItemKind},
    scope::{ScopeInfo, Visibility},
};

struct ResolveInfo<'pkg, 'err> {
    package: &'pkg Package,
    asts: &'pkg SecondaryMap<ModuleId, Ast>,
    handler: ErrorHandler<'err>,
    hir: Hir,
    scopes: ScopeInfo<'pkg>,
}

/// Resolves and lowers the provided [`Package`] into a single [`Hir`].
///
/// # Errors
/// Returns an error if there are any unbound variables, undefined types, or multiple items with the same name.
pub fn resolve(
    package: &Package,
    asts: &SecondaryMap<ModuleId, Ast>,
    handler: ErrorHandler,
) -> Result<Hir> {
    let mut info = ResolveInfo {
        package,
        asts,
        handler,
        hir: Hir::default(),
        scopes: ScopeInfo::new(package),
    };
    info.resolve_module(package.root(), true);
    info.handler.checked(info.hir)
}

impl ResolveInfo<'_, '_> {
    fn resolve_module(&mut self, module: ModuleId, is_root: bool) {
        for &child in &self.package.get(module).children {
            self.resolve_module(child, false);
        }

        self.scopes.set_module(module);

        let ast = &self.asts[module];

        for ty in &ast.ty_items {
            let id = self.hir.reserve_ty(ty.ident);
            let vis = if ty.public {
                Visibility::Public
            } else {
                Visibility::Private
            };
            if self.scopes.add_ty(vis, ty.ident.ident, id).is_some() {
                self.err(
                    ErrorKind::DuplicateItem(ItemKind::Type, ty.ident.ident),
                    ty.ident.span,
                );
            }
        }

        for ty in &ast.ty_items {
            self.resolve_ty_item(ty);
        }

        for block in &ast.block_items {
            match block {
                BlockItem::Impl { span, ty, items } => {
                    todo!("Impl Blocks")
                }
            }
        }

        for def in &ast.def_items {
            let id = self.hir.add_var(VarInfo {
                ident: def.ident,
                mutable: false,
                global: true,
                module: self.scopes.module(),
            });
            let vis = if def.public {
                Visibility::Public
            } else {
                Visibility::Private
            };
            if self.scopes.add_def(vis, def.ident.ident, id).is_some() {
                self.err(
                    ErrorKind::DuplicateItem(ItemKind::Value, def.ident.ident),
                    def.ident.span,
                );
            }
        }

        for Import(path, span) in &ast.imports {
            if let Err(e) = self.scopes.import(path) {
                self.err(e, *span);
            }
        }

        for block in &ast.block_items {
            self.resolve_block_item(block);
        }

        let main_index = is_root
            .then(|| self.find_main(&ast.def_items).ok()?)
            .flatten();

        for (index, def) in ast.def_items.iter().enumerate() {
            if let Ok(def) = self.resolve_def_item(&def.ident.ident.into(), def) {
                if main_index.is_some_and(|main_index| main_index == index) {
                    self.hir.set_main(def);
                } else {
                    self.hir.add_def(def);
                }
            }
        }
    }

    fn find_main(&self, execs: &[ast::DefItem]) -> Result<Option<usize>> {
        for (idx, item) in execs.iter().enumerate() {
            if let ast::DefKind::Func { params, ret_ty, .. } = &item.kind
                && item.ident.ident == "main"
            {
                return if params.is_empty() && ret_ty.kind == ast::TyKind::unit() {
                    Ok(Some(idx))
                } else {
                    Err(self.err(ErrorKind::InvalidMain, item.ident.span))
                };
            }
        }

        Ok(None)
    }

    fn resolve_ty_item(&mut self, item: &TyItem) {
        let id = self
            .scopes
            .resolve_ty(&item.ident.ident.into())
            .expect("all items should have already been inserted into the scope");

        if !item.generics.is_empty() {
            todo!("Generics")
        }

        match &item.kind {
            TyItemKind::Record(old_fields) => {
                let mut fields = HashMap::new();
                for old_field in old_fields {
                    let Ok(ty) = self.resolve_ty(&old_field.ty) else {
                        continue;
                    };
                    let field = Field {
                        span: old_field.ident.span,
                        ty,
                    };
                    if fields.insert(old_field.ident.ident, field).is_some() {
                        self.err(ErrorKind::DupFields(old_field.ident.ident), item.ident.span);
                    }
                }

                let constructor_ty = hir::Ty::Func(
                    fields
                        .values()
                        .map(|field| Param {
                            ty: field.ty.clone(),
                            mutable: false,
                            span: field.span,
                        })
                        .collect(),
                    Box::new(hir::Ty::Named(id)),
                );
                let ctor = self.hir.add_var(VarInfo {
                    ident: item.ident,
                    mutable: false,
                    global: true,
                    module: self.scopes.module(),
                });
                let ctor_vis = if item.opaque || !item.public {
                    Visibility::Private
                } else {
                    Visibility::Public
                };
                self.hir.add_var_ty(ctor, constructor_ty);
                self.scopes.add_def(ctor_vis, item.ident.ident, ctor);

                self.hir.fulfill_ty(
                    id,
                    TyInfo {
                        opaque: item.opaque,
                        fields,
                        ctor,
                        module: self.scopes.module(),
                    },
                );
            }
            TyItemKind::Union(_) => {
                todo!("Unions");
            }
        }
    }

    fn resolve_block_item(&mut self, item: &BlockItem) -> Result<()> {
        match item {
            BlockItem::Impl { span, ty, items } => {
                todo!("Impl Blocks")
                // let ty = self.resolve_ty(ty)?;

                // for item in items {
                //     let mut path = ty_path.clone();
                //     path.push(item.ident.ident);
                //     if let Ok(exec) = self.resolve_exec_item(&path, item, Some(&ty)) {
                //         self.hir.add_exec(exec);
                //     }
                // }

                // Ok(())
            }
        }
    }

    fn resolve_def_item(&mut self, path: &Path, item: &ast::DefItem) -> Result<hir::DefItem> {
        if !item.generics.is_empty() {
            todo!("Generics")
        }

        let id = self
            .scopes
            .resolve_var(path)
            .expect("all items should have already been inserted into the scope");

        match &item.kind {
            ast::DefKind::Const { ty, val } => {
                let val = self.resolve_expr(val);
                let ty = self.resolve_ty(ty)?;
                self.hir.add_var_ty(id, ty);

                Ok(hir::DefItem {
                    module: self.scopes.module(),
                    var: id,
                    kind: hir::DefKind::Const(val?),
                })
            }
            ast::DefKind::Func {
                self_param,
                params,
                ret_ty,
                body,
            } => {
                self.scopes.push_scope();

                let self_param = self_param.map(|(mutable, span)| {
                    todo!("Methods")
                    // let Some(ty) = parent_ty else {
                    //     return Err(self.err(ErrorKind::SelfOutsideImpl, span));
                    // };
                    // let id = self.resolve_pat(
                    //     &ast::PatKind::Ident(Ident::new("self")).span((span.end - 4)..span.end),
                    //     mutable,
                    //     Some(hir::Ty::Named(ty)),
                    // );
                    // Ok((
                    //     id,
                    //     Param {
                    //         ty: hir::Ty::Named(ty),
                    //         mutable,
                    //         span,
                    //     },
                    // ))
                });

                let params = self_param
                    .into_iter()
                    .chain(params.iter().map(|p| {
                        let ty = self.resolve_ty(&p.ty)?;
                        let id = self.resolve_pat(&p.pat, p.mutable, Some(ty.clone()));
                        Ok((
                            id,
                            Param {
                                ty,
                                mutable: p.mutable,
                                span: p.span,
                            },
                        ))
                    }))
                    .try_collect_eager();

                let body = self.resolve_expr(body);

                self.scopes.pop_scope();

                let ret_ty = self.resolve_ty(ret_ty)?;
                let (params, param_tys) = params?;

                self.hir
                    .add_var_ty(id, hir::Ty::Func(param_tys, Box::new(ret_ty)));

                Ok(hir::DefItem {
                    module: self.scopes.module(),
                    var: id,
                    kind: hir::DefKind::Func {
                        params,
                        body: body?,
                    },
                })
            }
        }
    }

    fn resolve_binding(&mut self, binding: &Binding) -> Result<VarId> {
        let ty = binding
            .ty
            .as_ref()
            .map(|ty| self.resolve_ty(ty))
            .transpose()?;
        Ok(self.resolve_pat(&binding.pat, binding.mutable, ty))
    }

    fn resolve_ty(&mut self, ty: &ast::Ty) -> Result<hir::Ty> {
        match &ty.kind {
            TyKind::Int => Ok(hir::Ty::Int),
            TyKind::UInt => Ok(hir::Ty::UInt),
            TyKind::Byte => Ok(hir::Ty::Byte),
            TyKind::Float => Ok(hir::Ty::Float),
            TyKind::Bool => Ok(hir::Ty::Bool),
            TyKind::Array(ty) => Ok(hir::Ty::Array(Box::new(self.resolve_ty(ty)?))),
            TyKind::Tuple(tys) => Ok(hir::Ty::Tuple(self.resolve_tys(tys)?)),
            TyKind::Func(params, ret_ty) => {
                let params = params
                    .iter()
                    .map(|param| {
                        Ok(Param {
                            ty: self.resolve_ty(&param.ty)?,
                            mutable: param.mutable,
                            span: param.span,
                        })
                    })
                    .try_collect_eager();
                let ret_ty = Box::new(self.resolve_ty(ret_ty)?);
                Ok(hir::Ty::Func(params?, ret_ty))
            }
            TyKind::Named(path, args) => {
                if !args.is_empty() {
                    todo!("Generics")
                }

                match self.scopes.resolve_ty(path) {
                    Ok(id) => Ok(hir::Ty::Named(id)),
                    Err(e) => Err(self.err(e, ty.span)),
                }
            }
        }
    }

    fn resolve_tys(&mut self, tys: &[ast::Ty]) -> Result<Vec<hir::Ty>> {
        tys.iter().map(|ty| self.resolve_ty(ty)).try_collect_eager()
    }

    fn resolve_pat(&mut self, pat: &Pat, mutable: bool, ty: Option<hir::Ty>) -> VarId {
        match pat.kind {
            PatKind::Ident(ident) => {
                let id = self.hir.add_var(VarInfo {
                    ident: ident.span(pat.span),
                    mutable,
                    global: false,
                    module: self.scopes.module(),
                });
                if let Some(ty) = ty {
                    self.hir.add_var_ty(id, ty);
                }
                self.scopes.add_var(ident, id);
                id
            }
            _ => todo!("Pattern Matching"),
        }
    }

    fn err(&self, error: ErrorKind, span: Range<u32>) -> HandledError {
        self.handler.report(error, span, self.scopes.module())
    }
}
