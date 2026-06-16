//! Lowers a [`Hir`] into a [`Mir`].
//! This involves the following:
//! - Unifying records and tuples, and rearranging their fields for best packing.
//! - Lowering field access to a numeric field number.

mod exprs;

use errors::ErrorHandler;
use hir::{ExecItem, ExecKind, Hir};
use mir::{Item, ItemKind, Mir};
use slotmap::SecondaryMap;

struct LowerInfo<'err> {
    handler: ErrorHandler<'err>,
    hir: Hir,
    expr_tys: SecondaryMap<hir::ExprId, hir::Ty>,
    mir: Mir,
    var_map: SecondaryMap<hir::VarId, mir::VarId>,
}

/// Resolves and lowers the provided [`Package`] into a single [`Hir`].
///
/// # Errors
/// Returns an error if there are any unbound variables, undefined types, or multiple items with the same name.
pub fn lower(handler: ErrorHandler, hir: Hir, expr_tys: SecondaryMap<hir::ExprId, hir::Ty>) -> Mir {
    LowerInfo {
        handler,
        hir,
        expr_tys,
        mir: Mir::default(),
        var_map: SecondaryMap::new(),
    }
    .lower()
}

impl LowerInfo<'_> {
    fn lower(mut self) -> Mir {
        for item in self.hir.execs() {
            let ty = match item.kind {
                ExecKind::Const(_) => self.lower_ty(self.hir.var_ty(item.var)),
                ExecKind::Func { .. } => {
                    let hir::Ty::Func(params, ret_ty) = self.hir.var_ty(item.var) else {
                        unreachable!("function with non-function type")
                    };
                    let params = params
                        .into_iter()
                        .map(|param| mir::Param {
                            ty: self.lower_ty(&param.ty),
                            mutable: param.mutable,
                        })
                        .collect();
                    let ret_ty = Box::new(self.lower_ty(&ret_ty));
                    mir::Ty::FuncPtr(params, ret_ty)
                }
            };
            let var_info = self.hir.var_info(item.var);
            let new_var = self.mir.add_var(var_info.ident, ty, var_info.mutable);
            self.var_map.insert(item.var, new_var);
        }

        for item in self.hir.take_execs() {
            let item = self.lower_item(item);
            self.mir.add_exec(item);
        }

        self.mir
    }

    fn lower_var(&mut self, var: hir::VarId) -> mir::VarId {
        let var_info = self.hir.var_info(var);
        let new_var = self.mir.add_var(
            var_info.ident,
            self.lower_ty(self.hir.var_ty(var)),
            var_info.mutable,
        );
        self.var_map.insert(var, new_var);
        new_var
    }

    fn lower_item(&mut self, item: ExecItem) -> Item {
        match item.kind {
            ExecKind::Const(val) => Item {
                var: self.var_map[item.var],
                kind: ItemKind::Const(self.lower_expr(item.module, val)),
            },
            ExecKind::Func { params, body } => {
                let params = params.iter().map(|var| self.lower_var(*var)).collect();
                let body = self.lower_expr(item.module, body);
                Item {
                    var: self.var_map[item.var],
                    kind: ItemKind::Func { params, body },
                }
            }
        }
    }

    fn lower_ty(&self, ty: &hir::Ty) -> mir::Ty {
        match ty {
            hir::Ty::Int => mir::Ty::Int,
            hir::Ty::UInt => mir::Ty::UInt,
            hir::Ty::Byte => mir::Ty::Byte,
            hir::Ty::Float => mir::Ty::Float,
            hir::Ty::Char => todo!("Strings"),
            hir::Ty::Bool => mir::Ty::Bool,
            hir::Ty::Array(elem_ty) => mir::Ty::Array(Box::new(self.lower_ty(elem_ty))),
            hir::Ty::Tuple(elem_tys) => {
                mir::Ty::Fields(elem_tys.into_iter().map(|ty| self.lower_ty(ty)).collect())
            }
            hir::Ty::Func(params, ret_ty) => {
                let params = params
                    .into_iter()
                    .map(|param| mir::Param {
                        ty: self.lower_ty(&param.ty),
                        mutable: param.mutable,
                    })
                    .collect();
                let ret_ty = Box::new(self.lower_ty(ret_ty));
                // Assume it's a closure. Special handling for function pointers will be done elsewhere.
                mir::Ty::Closure(params, ret_ty)
            }
            hir::Ty::Named(id) => {
                let fields = self
                    .hir
                    .ty_info(*id)
                    .fields
                    .iter()
                    .map(|(_, field)| self.lower_ty(&field.ty))
                    .collect();
                mir::Ty::Fields(fields)
            }
        }
    }

    // fn lower_pat(
    //     scope: &mut Scope,
    //     hir: &mut Hir,
    //     pat: Pat,
    //     mutable: bool,
    //     ty: Option<hir::Ty>,
    // ) -> VarId {
    //     match pat.kind {
    //         PatKind::Ident(ident) => {
    //             let id = hir.add_var(ident, mutable, pat.span, scope.module());
    //             if let Some(ty) = ty {
    //                 hir.add_var_ty(id, ty);
    //             }
    //             scope.add_var(ident, id);
    //             id
    //         }
    //         _ => todo!("Pattern Matching"),
    //     }
    // }
}

// #[cfg(any(test, feature = "test"))]
// #[allow(clippy::unwrap_used, reason = "test utility")]
// pub fn test_resolve_expr(input: &str) -> Result<(ExprId, Hir)> {
//     let expr = parse::Parser::parse_expr(input).unwrap();
//     let mut hir = Hir::default();
//     let mut handler = ErrorHandler::TEST;
//     let expr = exprs::resolve_expr(
//         &Scope::new(ModuleId::default()),
//         &mut hir,
//         &mut handler,
//         expr,
//     )?;
//     Ok((expr, hir))
// }

// #[cfg(any(test, feature = "test"))]
// #[allow(clippy::unwrap_used, reason = "test utility")]
// pub fn test_resolve_ast(src: &str) -> Result<Hir> {
//     let mut hir = Hir::default();
//     let mut handler = ErrorHandler::TEST;
//     resolve_ast(
//         &mut Scope::new(ModuleId::default()),
//         parse::Parser::new_test(src).parse().unwrap(),
//         &mut hir,
//         &mut handler,
//         true,
//     );
//     handler.checked(hir)
// }
