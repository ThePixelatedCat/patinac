use std::{iter, range::Range};

use errors::ErrorHandler;
use ident::SpanIdent;
use irs::{
    ModuleId,
    hir::{Hir, Ty},
};

use crate::{
    Constraint, Table, TypeChecker,
    error::ErrorKind,
    types::{Param, PartialTy, TyVar},
};

impl TypeChecker<'_> {
    /// Unifies all types in the unification table.
    pub(super) fn unify(&mut self, hir: &Hir) {
        for constr in &self.constraints {
            match constr {
                Constraint::Eq(ty_a, ty_b, span, module) => {
                    unify_ty_ty(
                        &mut self.table,
                        &mut self.handler,
                        *span,
                        *module,
                        ty_a,
                        ty_b,
                    );
                }
                Constraint::Field(base_ty, base_span, field_ty, field_name, module) => {
                    unify_field_ty(
                        &mut self.table,
                        &mut self.handler,
                        hir,
                        base_ty,
                        *base_span,
                        field_ty,
                        *field_name,
                        *module,
                    );
                }
                Constraint::Method(base_ty, base_span, method_ty, method_name, module) => {
                    unify_method_ty(
                        &mut self.table,
                        &mut self.handler,
                        hir,
                        base_ty,
                        *base_span,
                        method_ty,
                        *method_name,
                        *module,
                    );
                }
            }
        }
    }
}

fn unify_field_ty(
    table: &mut Table,
    handler: &mut ErrorHandler,
    hir: &Hir,
    base_ty: &PartialTy,
    base_span: Range<u32>,
    field_ty: &PartialTy,
    field_name: SpanIdent,
    module: ModuleId,
) {
    let base_ty = normalize_ty(table, base_ty);

    let base_id = match base_ty {
        PartialTy::Named(id) => id,
        PartialTy::Var(_) => {
            handler.report(ErrorKind::UninferredVarType, base_span, module);
            return;
        }
        no_fields_ty => {
            handler.report(ErrorKind::NoFieldsType(no_fields_ty), base_span, module);
            return;
        }
    };

    let ty_info = hir.ty_info(base_id);

    if ty_info.opaque && ty_info.module != module {
        handler.report(
            ErrorKind::OpaqueType(hir.ty_ident(base_id).ident),
            base_span,
            module,
        );
        return;
    }

    let Some(field) = ty_info.get_field(field_name.ident) else {
        handler.report(
            ErrorKind::NoSuchField(hir.ty_ident(base_id).ident, field_name.ident),
            field_name.span,
            module,
        );
        return;
    };

    unify_ty_ty(
        table,
        handler,
        field_name.span,
        module,
        field_ty,
        &PartialTy::from(&field.ty),
    );
}

fn unify_method_ty(
    table: &mut Table,
    handler: &mut ErrorHandler,
    hir: &Hir,
    base_ty: &PartialTy,
    base_span: Range<u32>,
    method_ty: &PartialTy,
    method_name: SpanIdent,
    module: ModuleId,
) {
    let Ok(base_ty) = Ty::try_from(normalize_ty(table, base_ty)) else {
        handler.report(ErrorKind::UninferredVarType, base_span, module);
        return;
    };

    todo!("Method Resolution")
    // let Some(field) = hir.ty_info(base_id).fields.get(&field_name.ident) else {
    //     return Err(
    //         ErrorKind::MissingField(base_ty, field_name.ident).span(field_name.span, module)
    //     );
    // };

    // unify_ty_ty(
    //     table,
    //     field_name.span,
    //     module,
    //     field_ty,
    //     &PartialTy::from(&field.ty),
    // )
}

/// Recursively traverse two types until at least one is a type variable,
/// at which point we unify them in the table,
/// or until we can no longer traverse them or we know they're mismatched,
/// at which point we error.
fn unify_ty_ty(
    table: &mut Table,
    handler: &mut ErrorHandler,
    span: Range<u32>,
    module: ModuleId,
    unnorm_lhs: &PartialTy,
    unnorm_rhs: &PartialTy,
) {
    let lhs = normalize_ty(table, unnorm_lhs);
    let rhs = normalize_ty(table, unnorm_rhs);

    match (lhs, rhs) {
        (PartialTy::Int, PartialTy::Int)
        | (PartialTy::UInt, PartialTy::UInt)
        | (PartialTy::Byte, PartialTy::Byte)
        | (PartialTy::Float, PartialTy::Float)
        | (PartialTy::Bool, PartialTy::Bool) => {}
        (PartialTy::Tuple(lhs_elems), PartialTy::Tuple(rhs_elems)) => {
            if lhs_elems.len() != rhs_elems.len() {
                handler.report(
                    ErrorKind::TypeMismatch {
                        expected: PartialTy::Tuple(lhs_elems),
                        found: PartialTy::Tuple(rhs_elems),
                    },
                    span,
                    module,
                );
                return;
            }
            for (l, r) in iter::zip(lhs_elems, rhs_elems) {
                unify_ty_ty(table, handler, span, module, &l, &r);
            }
        }
        (PartialTy::Array(lhs_inner), PartialTy::Array(rhs_inner)) => {
            unify_ty_ty(table, handler, span, module, &lhs_inner, &rhs_inner);
        }
        (PartialTy::Fn(lhs_params, lhs_ret), PartialTy::Fn(rhs_params, rhs_ret)) => {
            unify_ty_ty(table, handler, span, module, &lhs_ret, &rhs_ret);
            if lhs_params.len() != rhs_params.len() {
                handler.report(
                    ErrorKind::ArgCount {
                        expected: lhs_params.len(),
                        found: rhs_params.len(),
                    },
                    span,
                    module,
                );
                return;
            }
            for (l, r) in iter::zip(lhs_params, rhs_params) {
                unify_ty_ty(table, handler, r.span, module, &l.ty, &r.ty);
                if l.mutable != r.mutable {
                    handler.report(
                        ErrorKind::MutMismatch {
                            should_be_mut: l.mutable,
                        },
                        r.span,
                        module,
                    );
                }
            }
        }
        (PartialTy::Named(a), PartialTy::Named(b)) if a == b => {}
        (PartialTy::IntVar(lhs_var), PartialTy::IntVar(rhs_var))
        | (PartialTy::Var(lhs_var), PartialTy::Var(rhs_var)) => {
            unify_var_var(table, handler, span, module, lhs_var, rhs_var);
        }
        (PartialTy::Var(var), ty) | (ty, PartialTy::Var(var)) => {
            if occurs_check(&ty, var) {
                handler.report(ErrorKind::Infinite, span, module);
                return;
            }
            unify_var_value(table, handler, span, module, var, ty);
        }
        (
            PartialTy::IntVar(int_var),
            int_ty @ (PartialTy::Int | PartialTy::UInt | PartialTy::Byte),
        )
        | (
            int_ty @ (PartialTy::Int | PartialTy::UInt | PartialTy::Byte),
            PartialTy::IntVar(int_var),
        ) => unify_var_value(table, handler, span, module, int_var, int_ty),
        (lhs, rhs) => {
            handler.report(
                ErrorKind::TypeMismatch {
                    expected: lhs,
                    found: rhs,
                },
                span,
                module,
            );
        }
    }
}

fn unify_var_var(
    table: &mut Table,
    handler: &mut ErrorHandler,
    span: Range<u32>,
    module: ModuleId,
    l: TyVar,
    r: TyVar,
) {
    if let Err((l, r)) = table.unify_var_var(l, r) {
        handler.report(
            ErrorKind::TypeMismatch {
                expected: l,
                found: r,
            },
            span,
            module,
        );
    }
}

fn unify_var_value(
    table: &mut Table,
    handler: &mut ErrorHandler,
    span: Range<u32>,
    module: ModuleId,
    var: TyVar,
    ty: PartialTy,
) {
    if let Err((l, r)) = table.unify_var_value(var, Some(ty)) {
        handler.report(
            ErrorKind::TypeMismatch {
                expected: l,
                found: r,
            },
            span,
            module,
        );
    }
}

fn normalize_ty(table: &mut Table, ty: &PartialTy) -> PartialTy {
    match ty {
        PartialTy::Int | PartialTy::UInt | PartialTy::Byte | PartialTy::Float | PartialTy::Bool => {
            ty.clone()
        }
        PartialTy::Tuple(tys) => {
            PartialTy::Tuple(tys.iter().map(|ty| normalize_ty(table, ty)).collect())
        }
        PartialTy::Array(ty) => PartialTy::Array(Box::new(normalize_ty(table, ty))),
        PartialTy::Fn(params, ret) => {
            let params = params
                .iter()
                .map(|param| Param {
                    ty: normalize_ty(table, &param.ty),
                    mutable: param.mutable,
                    span: param.span,
                })
                .collect();
            let ret = Box::new(normalize_ty(table, ret));
            PartialTy::Fn(params, ret)
        }
        PartialTy::Named(id) => PartialTy::Named(*id),
        PartialTy::Var(v) => match table.probe_value(*v) {
            Some(ty) => normalize_ty(table, &ty),
            None => PartialTy::Var(table.find(*v)),
        },
        PartialTy::IntVar(v) => match table.probe_value(*v) {
            Some(ty) => normalize_ty(table, &ty),
            None => PartialTy::IntVar(table.find(*v)),
        },
    }
}

fn occurs_check(ty: &PartialTy, var: TyVar) -> bool {
    match ty {
        PartialTy::Int
        | PartialTy::UInt
        | PartialTy::Byte
        | PartialTy::Float
        | PartialTy::Bool
        | PartialTy::Named(_) => false,
        PartialTy::Tuple(tys) => tys.iter().any(|ty| occurs_check(ty, var)),
        PartialTy::Array(ty) => occurs_check(ty, var),
        PartialTy::Fn(params, ret) => {
            occurs_check(ret, var) || params.iter().any(|param| occurs_check(&param.ty, var))
        }
        PartialTy::Var(this_var) | PartialTy::IntVar(this_var) => *this_var == var,
    }
}
