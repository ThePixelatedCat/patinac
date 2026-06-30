//! Lowers all [`Asts`][Ast] into a single [`Hir`], resolving stringly variable and type names into numeric identifiers along the way.

mod error;
mod exprs;
mod scope;
#[cfg(test)]
mod test;

use foldhash::{HashMap, HashMapExt as _};
use slotmap::SecondaryMap;

use errors::{ErrorHandler, Result, SpanError as _, TryCollectEager as _};
use irs::{
    ModuleId, Package,
    ast::{self, Ast, Binding, Pat, PatKind, Path, TyItem, TyItemKind, TyKind, VisItem},
    hir::{self, Field, Hir, Param, TyInfo, VarId, VarInfo},
};

use crate::{error::ErrorKind, scope::Scope};

struct ResolveInfo<'pkg, 'err> {
    package: &'pkg Package,
    asts: &'pkg SecondaryMap<ModuleId, Ast>,
    handler: ErrorHandler<'err>,
    hir: Hir,
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
    };
    let mut package_scope = Scope::new(info.package.root());
    info.resolve_module(info.package.root(), &mut package_scope, true);
    info.handler.checked(info.hir)
}

impl ResolveInfo<'_, '_> {
    fn resolve_module(&mut self, module: ModuleId, parent_scope: &mut Scope, is_root: bool) {
        let mut scope = Scope::new(module);

        for &child in &self.package.get(module).children {
            self.resolve_module(child, &mut scope, false);
        }
        let ast = &self.asts[module];

        if !ast.impls.is_empty() {
            todo!("Associated Items")
        }

        for ty in &ast.ty_items {
            match scope.get_ty(&ty.ident.ident.into()) {
                Some(_) => {
                    self.handler.err(
                        ErrorKind::DupItem(ty.ident.ident).span(ty.ident.span, scope.module()),
                    );
                }
                None => {
                    let id = self.hir.reserve_ty(ty.ident);
                    scope.add_ty(&ty.ident.ident.into(), id);
                }
            }
        }

        for exec in &ast.exec_items {
            match scope.get_var(&exec.ident.ident.into()) {
                Some(_) => {
                    self.handler.err(
                        ErrorKind::DupItem(exec.ident.ident).span(exec.ident.span, scope.module()),
                    );
                }
                None => {
                    let id = self.hir.add_var(VarInfo {
                        ident: exec.ident,
                        mutable: false,
                        global: true,
                        module: scope.module(),
                    });
                    scope.add_var(&exec.ident.ident.into(), id);
                }
            }
        }

        for vis in &ast.vis_items {
            if let VisItem::Import(path, span) = vis {
                scope
                    .import(path)
                    .map_err(|error| self.handler.err(error.span(*span, scope.module())))
                    .ok();
            }
        }

        for vis in &ast.vis_items {
            if let VisItem::Export(idents) = vis {
                for ident in idents {
                    let mut success = false;
                    // TEST
                    let path = Path::new_const([self.package.get(module).name, ident.ident]);

                    if let Some(ty) = scope.get_ty(&ident.ident.into()) {
                        parent_scope.add_ty(&path, ty);
                        success = true;
                    }

                    if let Some(var) = scope.get_var(&ident.ident.into()) {
                        parent_scope.add_var(&path, var);
                        success = true;
                    }

                    if !success {
                        self.handler.err(
                            ErrorKind::UnknownItem(ident.ident).span(ident.span, scope.module()),
                        );
                    }
                }
            }
        }

        for ty in &ast.ty_items {
            self.resolve_ty_item(&mut scope, ty);
        }

        if is_root
            && let Ok(Some(idx)) = self.find_main(scope.module(), &ast.exec_items)
            && let Ok(main) = self.resolve_exec_item(&mut scope, &ast.exec_items[idx])
        {
            self.hir.set_main(main);
        }

        let main_index = is_root
            .then(|| self.find_main(scope.module(), &ast.exec_items).ok()?)
            .flatten();

        for (index, exec) in ast.exec_items.iter().enumerate() {
            if let Ok(exec) = self.resolve_exec_item(&mut scope, exec) {
                if main_index.is_some_and(|main_index| main_index == index) {
                    self.hir.set_main(exec);
                } else {
                    self.hir.add_exec(exec);
                }
            }
        }
    }

    fn find_main(&mut self, module: ModuleId, execs: &[ast::ExecItem]) -> Result<Option<usize>> {
        for (idx, item) in execs.iter().enumerate() {
            if let ast::ExecKind::Func { params, ret_ty, .. } = &item.kind
                && item.ident.ident == "main"
            {
                return if params.is_empty() && ret_ty.kind == ast::TyKind::unit() {
                    Ok(Some(idx))
                } else {
                    Err(self
                        .handler
                        .err(ErrorKind::InvalidMain.span(item.ident.span, module)))
                };
            }
        }

        Ok(None)
    }

    fn resolve_ty_item(&mut self, scope: &mut Scope, item: &TyItem) {
        let id = scope
            .get_ty(&item.ident.ident.into())
            .expect("all items should have already been inserted into the scope");

        if !item.generics.is_empty() {
            todo!("Generics")
        }

        if item.opaque {
            todo!("Opaque Types")
        }

        match &item.kind {
            TyItemKind::Record(old_fields) => {
                let mut fields = HashMap::new();
                for old_field in old_fields {
                    let Ok(ty) = self.resolve_ty(scope, &old_field.ty) else {
                        continue;
                    };
                    let field = Field {
                        span: old_field.ident.span,
                        ty,
                    };
                    if fields.insert(old_field.ident.ident, field).is_some() {
                        self.handler.err(
                            ErrorKind::DupFields(old_field.ident.ident)
                                .span(item.ident.span, scope.module()),
                        );
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
                    module: scope.module(),
                });
                self.hir.add_var_ty(ctor, constructor_ty);
                scope.add_var(&item.ident.ident.into(), ctor);

                self.hir.fulfill_ty(id, TyInfo { fields, ctor });
            }
            TyItemKind::Union(_) => {
                todo!("Pattern Matching");
            }
        }
    }

    fn resolve_exec_item(
        &mut self,
        scope: &mut Scope,
        item: &ast::ExecItem,
    ) -> Result<hir::ExecItem> {
        let id = scope
            .get_var(&item.ident.ident.into())
            .expect("all items should have already been inserted into the scope");

        match &item.kind {
            ast::ExecKind::Const { ty, val } => {
                let val = self.resolve_expr(scope, val);
                let ty = self.resolve_ty(scope, ty)?;
                self.hir.add_var_ty(id, ty);

                Ok(hir::ExecItem {
                    module: scope.module(),
                    var: id,
                    kind: hir::ExecKind::Const(val?),
                })
            }
            ast::ExecKind::Func {
                generics,
                self_param,
                params,
                ret_ty,
                body,
            } => {
                if !generics.is_empty() {
                    todo!("Generics")
                }

                if self_param.is_some() {
                    todo!("Methods")
                }

                scope.push();

                let params = params
                    .iter()
                    .map(|p| {
                        let ty = self.resolve_ty(scope, &p.ty)?;
                        let id = self.resolve_pat(scope, &p.pat, p.mutable, Some(ty.clone()));
                        Ok((
                            id,
                            Param {
                                ty,
                                mutable: p.mutable,
                                span: p.span,
                            },
                        ))
                    })
                    .try_collect_eager();
                let body = self.resolve_expr(scope, body);

                scope.pop();

                let ret_ty = self.resolve_ty(scope, ret_ty)?;
                let (params, param_tys) = params?;

                self.hir
                    .add_var_ty(id, hir::Ty::Func(param_tys, Box::new(ret_ty)));

                Ok(hir::ExecItem {
                    module: scope.module(),
                    var: id,
                    kind: hir::ExecKind::Func {
                        params,
                        body: body?,
                    },
                })
            }
        }
    }

    fn resolve_binding(&mut self, scope: &mut Scope, binding: &Binding) -> Result<VarId> {
        let ty = binding
            .ty
            .as_ref()
            .map(|ty| self.resolve_ty(scope, ty))
            .transpose()?;
        Ok(self.resolve_pat(scope, &binding.pat, binding.mutable, ty))
    }

    fn resolve_ty(&mut self, scope: &Scope, ty: &ast::Ty) -> Result<hir::Ty> {
        match &ty.kind {
            TyKind::Int => Ok(hir::Ty::Int),
            TyKind::UInt => Ok(hir::Ty::UInt),
            TyKind::Byte => Ok(hir::Ty::Byte),
            TyKind::Float => Ok(hir::Ty::Float),
            TyKind::Bool => Ok(hir::Ty::Bool),
            TyKind::Array(ty) => Ok(hir::Ty::Array(Box::new(self.resolve_ty(scope, ty)?))),
            TyKind::Tuple(tys) => Ok(hir::Ty::Tuple(self.resolve_tys(scope, tys)?)),
            TyKind::Func(params, ret_ty) => {
                let params = params
                    .iter()
                    .map(|param| {
                        Ok(Param {
                            ty: self.resolve_ty(scope, &param.ty)?,
                            mutable: param.mutable,
                            span: param.span,
                        })
                    })
                    .try_collect_eager();
                let ret_ty = Box::new(self.resolve_ty(scope, ret_ty)?);
                Ok(hir::Ty::Func(params?, ret_ty))
            }
            TyKind::Named(path, args) => {
                if !args.is_empty() {
                    todo!("Generics")
                }

                match scope.get_ty(path) {
                    Some(id) => Ok(hir::Ty::Named(id)),
                    None => Err(self
                        .handler
                        .err(ErrorKind::UnknownType(path.end()).span(ty.span, scope.module()))),
                }
            }
        }
    }

    fn resolve_tys(&mut self, scope: &Scope, tys: &[ast::Ty]) -> Result<Vec<hir::Ty>> {
        tys.iter()
            .map(|ty| self.resolve_ty(scope, ty))
            .try_collect_eager()
    }

    fn resolve_pat(
        &mut self,
        scope: &mut Scope,
        pat: &Pat,
        mutable: bool,
        ty: Option<hir::Ty>,
    ) -> VarId {
        match pat.kind {
            PatKind::Ident(ident) => {
                let id = self.hir.add_var(VarInfo {
                    ident: ident.span(pat.span),
                    mutable,
                    global: false,
                    module: scope.module(),
                });
                if let Some(ty) = ty {
                    self.hir.add_var_ty(id, ty);
                }
                scope.add_var(&ident.into(), id);
                id
            }
            _ => todo!("Pattern Matching"),
        }
    }
}

#[cfg(any(test, feature = "test"))]
#[allow(clippy::unwrap_used, reason = "test utility")]
pub fn test_resolve_expr(input: &str) -> Result<(hir::ExprId, Hir)> {
    let mut info = ResolveInfo {
        package: &Package::default(),
        asts: &SecondaryMap::default(),
        handler: ErrorHandler::TEST,
        hir: Hir::default(),
    };
    let expr = parse::Parser::parse_expr(input).unwrap();
    let expr = info.resolve_expr(&mut Scope::new(ModuleId::default()), &expr)?;
    Ok((expr, info.hir))
}

#[cfg(any(test, feature = "test"))]
#[allow(clippy::unwrap_used, reason = "test utility")]
pub fn test_resolve_ast(src: &str) -> Result<Hir> {
    use ident::Ident;
    use irs::Module;

    let package = Package::new(Module {
        parent: None,
        name: Ident::new("root"),
        children: Vec::new(),
    });
    let mut asts = SecondaryMap::new();
    asts.insert(
        package.root(),
        parse::Parser::new_test(src).parse().unwrap(),
    );

    let mut info = ResolveInfo {
        package: &package,
        asts: &asts,
        handler: ErrorHandler::TEST,
        hir: Hir::default(),
    };
    info.resolve_module(package.root(), &mut Scope::new(ModuleId::default()), true);
    info.handler.checked(info.hir)
}
