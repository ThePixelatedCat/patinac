//! Lowers all [`Asts`][Ast] into a single [`Hir`], resolving stringly variable and type names into numeric identifiers along the way.

mod error;
mod exprs;
mod scope;
#[cfg(test)]
mod test;

use itertools::Itertools as _;

use ast::{Ast, Binding, PackageAsts, Pat, PatKind, TyItem, TyItemKind, TyKind, VisItem};
use errors::{ErrorHandler, HandledError, Result, SpanError as _, TryCollectEager as _};
use hir::{ExprId, Hir, Param, TyInfo, VarId};
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
                let id = hir.add_var(exec.ident.ident, false, exec.ident.span, scope.module());
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
    let execs: Vec<_> = ast
        .exec_items
        .into_iter()
        .flat_map(|exec| resolve_exec_item(scope, hir, handler, exec))
        .collect();
    hir.add_execs(execs);
}

fn find_main(
    module: ModuleId,
    error_handler: &mut ErrorHandler,
    execs: &[ast::ExecItem],
) -> Result<Option<usize>> {
    for (idx, item) in execs.iter().enumerate() {
        if let ast::ExecKind::Fn { params, ret_ty, .. } = &item.kind
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

    match item.kind {
        TyItemKind::Record(fields) => {
            let fields: Vec<_> = fields
                .into_iter()
                .flat_map(|field| {
                    Ok::<_, HandledError>((field.ident, resolve_ty(scope, handler, field.ty)?))
                })
                .collect();

            if let Some((dup, _)) = fields.iter().duplicates_by(|(id, _)| id).next() {
                handler.err(ErrorKind::DupFields(dup.ident).span(item.ident.span, scope.module()));
                return;
            }

            let constructor_ty = hir::Ty::Func(
                fields
                    .iter()
                    .map(|(ident, ty)| Param {
                        ty: ty.clone(),
                        mutable: false,
                        span: ident.span,
                    })
                    .collect(),
                Box::new(hir::Ty::Named(id)),
            );
            let constructor_id =
                hir.add_var(item.ident.ident, false, item.ident.span, scope.module());
            hir.add_var_ty(constructor_id, constructor_ty);
            scope.add_var(item.ident.ident, constructor_id);

            hir.fulfill_ty(
                id,
                TyInfo {
                    fields: fields.into(),
                    constructor_id,
                },
            );
        }
        TyItemKind::Enum(_) => {
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
                id,
                kind: hir::ExecKind::Const { val: val? },
            })
        }
        ast::ExecKind::Fn {
            generics,
            params,
            ret_mut,
            ret_ty,
            body,
        } => {
            if !generics.is_empty() {
                todo!("Generics")
            }

            if ret_mut {
                todo!("Projections")
            }

            let mut scope = Scope::clone(scope);

            let params = params
                .into_iter()
                .map(|p| {
                    let ty = resolve_ty(&scope, handler, p.ty)?;
                    let id = resolve_pat(&mut scope, hir, p.pat, p.mutable, Some(ty.clone()));
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
                id,
                kind: hir::ExecKind::Fn {
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
    Ok(resolve_pat(scope, hir, binding.pat, binding.mutable, ty))
}

fn resolve_ty(scope: &Scope, handler: &mut ErrorHandler, ty: ast::Ty) -> Result<hir::Ty> {
    match ty.kind {
        TyKind::Int => Ok(hir::Ty::Int),
        TyKind::UInt => Ok(hir::Ty::UInt),
        TyKind::Byte => Ok(hir::Ty::Byte),
        TyKind::Float => Ok(hir::Ty::Float),
        TyKind::Char => Ok(hir::Ty::Char),
        TyKind::Bool => Ok(hir::Ty::Bool),
        TyKind::Array(ty) => Ok(hir::Ty::Array(Box::new(resolve_ty(scope, handler, *ty)?))),
        TyKind::Tuple(tys) => Ok(hir::Ty::Tuple(resolve_tys(scope, handler, tys)?)),
        TyKind::Func(params, ret) => {
            if ret.mutable {
                todo!("Projections")
            }

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
            let ret_ty = Box::new(resolve_ty(scope, handler, ret.ty)?);
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
    pat: Pat,
    mutable: bool,
    ty: Option<hir::Ty>,
) -> VarId {
    match pat.kind {
        PatKind::Ident(ident) => {
            let id = hir.add_var(ident, mutable, pat.span, scope.module());
            if let Some(ty) = ty {
                hir.add_var_ty(id, ty);
            }
            scope.add_var(ident, id);
            id
        }
        _ => todo!("Pattern Matching"),
    }
}

fn convert_lit(lit: ast::LitExpr) -> hir::LitExpr {
    match lit {
        ast::LitExpr::Int(i) => hir::LitExpr::Int(i),
        ast::LitExpr::Float(f) => hir::LitExpr::Float(f),
        ast::LitExpr::Char(c) => hir::LitExpr::Char(c),
        ast::LitExpr::String(s) => hir::LitExpr::String(s),
        ast::LitExpr::Bool(b) => hir::LitExpr::Bool(b),
    }
}

#[cfg(any(test, feature = "test"))]
#[allow(clippy::unwrap_used, reason = "test utility")]
pub fn test_resolve_expr(input: &str) -> Result<(ExprId, Hir)> {
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
