use std::range::Range;

use derive_more::From;
use slotmap::{SecondaryMap, SlotMap, new_key_type};

use ident::{Ident, SpanIdent};

use crate::{
    exprs::{Expr, ExprId},
    items::{AdtId, AdtInfo, ExecItem},
    types::Ty,
};

pub mod exprs;
pub mod items;
pub mod types;

#[derive(Debug, Default)]
pub struct Hir {
    main: Option<ExecItem>,
    execs: Vec<ExecItem>,
    adts: SlotMap<AdtId, SpanIdent>,
    adt_info: SecondaryMap<AdtId, AdtInfo>,
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

// Adt-related functions
impl Hir {
    pub fn adts(&self) -> impl Iterator<Item = (AdtId, SpanIdent)> {
        self.adts.iter().map(|(id, ident)| (id, *ident))
    }

    pub fn add_adt(&mut self, ident: SpanIdent, info: AdtInfo) -> AdtId {
        let id = self.reserve_adt(ident);
        self.fulfill_adt(id, info);
        id
    }

    pub fn reserve_adt(&mut self, ident: SpanIdent) -> AdtId {
        self.adts.insert(ident)
    }

    pub fn fulfill_adt(&mut self, id: AdtId, info: AdtInfo) {
        self.adt_info.insert(id, info);
    }

    pub fn adt_ident(&self, id: AdtId) -> SpanIdent {
        self.adts[id]
    }

    pub fn adt_info(&self, id: AdtId) -> &AdtInfo {
        &self.adt_info[id]
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
