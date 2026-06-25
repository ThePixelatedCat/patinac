//! Lowers all [`Asts`][Ast] into a single [`Hir`], resolving stringly variable and type names into numeric identifiers along the way.

mod error;
mod exprs;
mod scope;
#[cfg(test)]
mod test;

use foldhash::{HashMap, HashMapExt as _};

use ast::{Ast, Binding, PackageAsts, Pat, PatKind, TyItem, TyItemKind, TyKind, VisItem};
use errors::{ErrorHandler, Result, SpanError as _, TryCollectEager as _};
use hir::{Field, Hir, Param, TyInfo, VarId, VarInfo};
use package::{ModuleId, Package};

use crate::{error::ErrorKind, scope::Scope};

/// Resolves and lowers the provided [`Package`] into a single [`Hir`].
///
/// # Errors
/// Returns an error if there are any unbound variables, undefined types, or multiple items with the same name.
pub fn resolve(package: &Package, mut asts: PackageAsts, mut handler: ErrorHandler) -> Result<Hir> {
    let mut hir = Hir::default();
    let root = package.root();
    resolve_module(package, &mut asts, root, &mut hir, &mut handler, true);
    handler.checked(hir)
}

fn resolve_module(
    package: &Package,
    asts: &mut PackageAsts,
    module: ModuleId,
    hir: &mut Hir,
    handler: &mut ErrorHandler,
    is_root: bool,
) -> Scope {
    let mut scope = Scope::new(module);
    for &child in &package.get(module).children {
        scope.add_module(
            package.get(child).name,
            resolve_module(package, asts, child, hir, handler, false),
        );
    }
    resolve_ast(&mut scope, asts.take(module), hir, handler, is_root);
    scope
}

fn resolve_ast(
    scope: &mut Scope,
    mut ast: Ast,
    hir: &mut Hir,
    handler: &mut ErrorHandler,
    is_root: bool,
) {
    for ty in &ast.ty_items {
        match scope.get_ty(ty.ident.ident) {
            Some(_) => {
                handler.err(ErrorKind::DupItem(ty.ident.ident).span(ty.ident.span, scope.module()));
            }
            None => {
                let id = hir.reserve_ty(ty.ident);
                scope.add_ty(ty.ident.ident, id);
            }
        }
    }

    for exec in &ast.exec_items {
        match scope.get_var(exec.ident.ident) {
            Some(_) => {
                handler.err(
                    ErrorKind::DupItem(exec.ident.ident).span(exec.ident.span, scope.module()),
                );
            }
            None => {
                let id = hir.add_var(VarInfo {
                    ident: exec.ident,
                    mutable: false,
                    global: true,
                    module: scope.module(),
                });
                scope.add_var(exec.ident.ident, id);
            }
        }
    }

    for vis in ast.vis_items {
        match vis {
            VisItem::Import(path, span) => {
                if let Err(error) = scope.import(path) {
                    handler.err(error.span(span, scope.module()));
                }
            }
            VisItem::Export(idents) => {
                for ident in idents {
                    if let Err(error) = scope.export(ident.ident) {
                        handler.err(error.span(ident.span, scope.module()));
                    }
                }
            }
        }
    }

    for ty in ast.ty_items {
        resolve_ty_item(scope, hir, handler, ty);
    }

    if is_root
        && let Ok(Some(idx)) = find_main(scope.module(), handler, &ast.exec_items)
        && let Ok(main) = resolve_exec_item(scope, hir, handler, ast.exec_items.remove(idx))
    {
        hir.set_main(main);
    }

    for exec in ast.exec_items {
        if let Ok(exec) = resolve_exec_item(scope, hir, handler, exec) {
            hir.add_exec(exec);
        }
    }
}

fn find_main(
    module: ModuleId,
    error_handler: &mut ErrorHandler,
    execs: &[ast::ExecItem],
) -> Result<Option<usize>> {
    for (idx, item) in execs.iter().enumerate() {
        if let ast::ExecKind::Func { params, ret_ty, .. } = &item.kind
            && item.ident.ident == "main"
        {
            return if params.is_empty() && ret_ty.kind == ast::TyKind::unit() {
                Ok(Some(idx))
            } else {
                Err(error_handler.err(ErrorKind::InvalidMain.span(item.ident.span, module)))
            };
        }
    }

    Ok(None)
}

fn resolve_ty_item(scope: &mut Scope, hir: &mut Hir, handler: &mut ErrorHandler, item: TyItem) {
    let id = scope.get_ty(item.ident.ident).expect(
        "all ast idents, including this one, should have already been inserted into the scope",
    );

    if !item.generics.is_empty() {
        todo!("Generics")
    }

    if item.opaque {
        todo!("Opaque Types")
    }

    match item.kind {
        TyItemKind::Record(old_fields) => {
            let mut fields = HashMap::new();
            for old_field in old_fields {
                let Ok(ty) = resolve_ty(scope, handler, old_field.ty) else {
                    continue;
                };
                let field = Field {
                    span: old_field.ident.span,
                    ty,
                };
                if fields.insert(old_field.ident.ident, field).is_some() {
                    handler.err(
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
            let ctor = hir.add_var(VarInfo {
                ident: item.ident,
                mutable: false,
                global: true,
                module: scope.module(),
            });
            hir.add_var_ty(ctor, constructor_ty);
            scope.add_var(item.ident.ident, ctor);

            hir.fulfill_ty(id, TyInfo { fields, ctor });
        }
        TyItemKind::Union(_) => {
            todo!("Pattern Matching");
        }
    }
}

fn resolve_exec_item(
    scope: &Scope,
    hir: &mut Hir,
    handler: &mut ErrorHandler,
    item: ast::ExecItem,
) -> Result<hir::ExecItem> {
    let id = scope.get_var(item.ident.ident).expect(
        "all exec item idents, including this one, should have already been inserted into the scope",
    );

    match item.kind {
        ast::ExecKind::Const { ty, val } => {
            let val = exprs::resolve_expr(scope, hir, handler, val);
            hir.add_var_ty(id, resolve_ty(scope, handler, ty)?);

            Ok(hir::ExecItem {
                module: scope.module(),
                var: id,
                kind: hir::ExecKind::Const(val?),
            })
        }
        ast::ExecKind::Func {
            generics,
            params,
            ret_ty,
            body,
        } => {
            if !generics.is_empty() {
                todo!("Generics")
            }

            let mut scope = Scope::clone(scope);

            let params = params
                .into_iter()
                .map(|p| {
                    let ty = resolve_ty(&scope, handler, p.ty)?;
                    let id = resolve_pat(&mut scope, hir, &p.pat, p.mutable, Some(ty.clone()));
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
            let body = exprs::resolve_expr(&scope, hir, handler, body);
            let ret_ty = resolve_ty(&scope, handler, ret_ty)?;
            let (params, param_tys) = params?;

            hir.add_var_ty(id, hir::Ty::Func(param_tys, Box::new(ret_ty)));

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

fn resolve_binding(
    scope: &mut Scope,
    hir: &mut Hir,
    handler: &mut ErrorHandler,
    binding: Binding,
) -> Result<VarId> {
    let ty = binding
        .ty
        .map(|ty| resolve_ty(scope, handler, ty))
        .transpose()?;
    Ok(resolve_pat(scope, hir, &binding.pat, binding.mutable, ty))
}

fn resolve_ty(scope: &Scope, handler: &mut ErrorHandler, ty: ast::Ty) -> Result<hir::Ty> {
    match ty.kind {
        TyKind::Int => Ok(hir::Ty::Int),
        TyKind::UInt => Ok(hir::Ty::UInt),
        TyKind::Byte => Ok(hir::Ty::Byte),
        TyKind::Float => Ok(hir::Ty::Float),
        TyKind::Bool => Ok(hir::Ty::Bool),
        TyKind::Array(ty) => Ok(hir::Ty::Array(Box::new(resolve_ty(scope, handler, *ty)?))),
        TyKind::Tuple(tys) => Ok(hir::Ty::Tuple(resolve_tys(scope, handler, tys)?)),
        TyKind::Func(params, ret_ty) => {
            let params = params
                .into_iter()
                .map(|param| {
                    Ok(Param {
                        ty: resolve_ty(scope, handler, param.ty)?,
                        mutable: param.mutable,
                        span: param.span,
                    })
                })
                .try_collect_eager();
            let ret_ty = Box::new(resolve_ty(scope, handler, *ret_ty)?);
            Ok(hir::Ty::Func(params?, ret_ty))
        }
        TyKind::Named(path, args) => {
            if !args.is_empty() {
                todo!("Generics")
            }

            match scope.resolve_ty(path) {
                Ok(id) => Ok(hir::Ty::Named(id)),
                Err(error) => Err(handler.err(error.span(ty.span, scope.module()))),
            }
        }
    }
}

fn resolve_tys(
    scope: &Scope,
    handler: &mut ErrorHandler,
    tys: Vec<ast::Ty>,
) -> Result<Vec<hir::Ty>> {
    tys.into_iter()
        .map(|ty| resolve_ty(scope, handler, ty))
        .try_collect_eager()
}

fn resolve_pat(
    scope: &mut Scope,
    hir: &mut Hir,
    pat: &Pat,
    mutable: bool,
    ty: Option<hir::Ty>,
) -> VarId {
    match pat.kind {
        PatKind::Ident(ident) => {
            let id = hir.add_var(VarInfo {
                ident: ident.span(pat.span),
                mutable,
                global: false,
                module: scope.module(),
            });
            if let Some(ty) = ty {
                hir.add_var_ty(id, ty);
            }
            scope.add_var(ident, id);
            id
        }
        _ => todo!("Pattern Matching"),
    }
}

#[cfg(any(test, feature = "test"))]
#[allow(clippy::unwrap_used, reason = "test utility")]
pub fn test_resolve_expr(input: &str) -> Result<(hir::ExprId, Hir)> {
    let expr = parse::Parser::parse_expr(input).unwrap();
    let mut hir = Hir::default();
    let mut handler = ErrorHandler::TEST;
    let expr = exprs::resolve_expr(
        &Scope::new(ModuleId::default()),
        &mut hir,
        &mut handler,
        expr,
    )?;
    Ok((expr, hir))
}

#[cfg(any(test, feature = "test"))]
#[allow(clippy::unwrap_used, reason = "test utility")]
pub fn test_resolve_ast(src: &str) -> Result<Hir> {
    let mut hir = Hir::default();
    let mut handler = ErrorHandler::TEST;
    resolve_ast(
        &mut Scope::new(ModuleId::default()),
        parse::Parser::new_test(src).parse().unwrap(),
        &mut hir,
        &mut handler,
        true,
    );
    handler.checked(hir)
}
