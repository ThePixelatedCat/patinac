use std::range::Range;

use derive_more::From;
use slotmap::{SecondaryMap, SlotMap, new_key_type};

use ident::{Ident, SpanIdent};

use crate::{
    exprs::{Expr, ExprId},
    items::{ExecItem, TyId, TyInfo},
    types::Ty,
};

pub mod exprs;
pub mod items;
pub mod types;

#[derive(Debug, Default)]
pub struct Hir {
    main: Option<ExecItem>,
    execs: Vec<ExecItem>,
    tys: SlotMap<TyId, SpanIdent>,
    ty_info: SecondaryMap<TyId, TyInfo>,
    exprs: SlotMap<ExprId, Expr>,
    expr_spans: SecondaryMap<ExprId, Range<usize>>,
    vars: SlotMap<VarId, VarInfo>,
    var_tys: SecondaryMap<VarId, Ty>,
}

impl Hir {
    pub fn execs(&self) -> &[ExecItem] {
        &self.execs
    }

    pub fn add_execs(&mut self, execs: impl IntoIterator<Item = ExecItem>) {
        self.execs.extend(execs);
    }

    pub const fn main(&self) -> Option<&ExecItem> {
        self.main.as_ref()
    }

    pub fn set_main(&mut self, main: ExecItem) {
        self.main = Some(main);
    }
}

// Type-related functions
impl Hir {
    pub fn tys(&self) -> impl Iterator<Item = (TyId, SpanIdent)> {
        self.tys.iter().map(|(id, ident)| (id, *ident))
    }

    pub fn add_ty(&mut self, ident: SpanIdent, info: TyInfo) -> TyId {
        let id = self.reserve_ty(ident);
        self.fulfill_ty(id, info);
        id
    }

    pub fn reserve_ty(&mut self, ident: SpanIdent) -> TyId {
        self.tys.insert(ident)
    }

    pub fn fulfill_ty(&mut self, id: TyId, info: TyInfo) {
        self.ty_info.insert(id, info);
    }

    pub fn ty_ident(&self, id: TyId) -> SpanIdent {
        self.tys[id]
    }

    pub fn ty_info(&self, id: TyId) -> &TyInfo {
        &self.ty_info[id]
    }
}

// Expr-related functions
impl Hir {
    pub fn add_expr(&mut self, expr: Expr, span: impl Into<Range<usize>>) -> ExprId {
        let id = self.exprs.insert(expr);
        self.expr_spans.insert(id, span.into());
        id
    }

    pub fn expr_info(&self, id: ExprId) -> &Expr {
        &self.exprs[id]
    }

    pub fn expr_span(&self, id: ExprId) -> Range<usize> {
        self.expr_spans[id]
    }
}

// Var-related functions
impl Hir {
    pub fn add_var(&mut self, ident: Ident, mutable: bool, span: Range<usize>) -> VarId {
        self.vars.insert(VarInfo {
            ident,
            mutable,
            span,
        })
    }

    pub fn var_info(&self, id: VarId) -> VarInfo {
        self.vars[id]
    }

    pub fn add_var_ty(&mut self, id: VarId, ty: Ty) {
        self.var_tys.insert(id, ty);
    }

    pub fn var_ty(&self, id: VarId) -> &Ty {
        &self.var_tys[id]
    }

    pub fn var_tys(&self) -> impl Iterator<Item = (VarId, Option<&Ty>)> {
        self.vars.iter().map(|(id, _)| (id, self.var_tys.get(id)))
    }
}

#[derive(From)]
pub struct TyMap(SecondaryMap<ExprId, Ty>);

impl TyMap {
    pub fn ty(&self, expr: ExprId) -> &Ty {
        &self.0[expr]
    }
}

new_key_type! { pub struct VarId; }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VarInfo {
    pub ident: Ident,
    pub mutable: bool,
    pub span: Range<usize>,
}
