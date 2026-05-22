use slotmap::{SecondaryMap, SlotMap, new_key_type};

use ident::SpanIdent;
use span::Span;

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
    pub execs: Vec<ExecItem>,
    main: Option<ExecItem>,
    pub adts: SlotMap<AdtId, SpanIdent>,
    adt_info: SecondaryMap<AdtId, AdtInfo>,
    exprs: SlotMap<ExprId, Expr>,
    expr_spans: SecondaryMap<ExprId, Span>,
    vars: SlotMap<VarId, SpanIdent>,
    pub var_info: SecondaryMap<VarId, VarInfo>,
}

impl Hir {
    pub const fn main(&self) -> Option<&ExecItem> {
        self.main.as_ref()
    }

    pub fn set_main(&mut self, main: ExecItem) {
        self.main = Some(main);
    }
}

// Adt-related functions
impl Hir {
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
    pub fn add_expr(&mut self, expr: Expr, span: impl Into<Span>) -> ExprId {
        let id = self.exprs.insert(expr);
        self.expr_spans.insert(id, span.into());
        id
    }

    pub fn expr_info(&self, id: ExprId) -> &Expr {
        &self.exprs[id]
    }

    pub fn expr_span(&self, id: ExprId) -> Span {
        self.expr_spans[id]
    }
}

// Var-related functions
impl Hir {
    pub fn add_var(&mut self, ident: SpanIdent, info: VarInfo) -> VarId {
        let id = self.reserve_var(ident);
        self.fulfill_var(id, info);
        id
    }

    pub fn reserve_var(&mut self, ident: SpanIdent) -> VarId {
        self.vars.insert(ident)
    }

    pub fn fulfill_var(&mut self, id: VarId, info: VarInfo) {
        self.var_info.insert(id, info);
    }

    pub fn var_ident(&self, id: VarId) -> SpanIdent {
        self.vars[id]
    }

    pub fn var_info(&self, id: VarId) -> &VarInfo {
        &self.var_info[id]
    }

    pub fn try_var_info(&self, id: VarId) -> Option<&VarInfo> {
        self.var_info.get(id)
    }
}

pub struct TyMap(SecondaryMap<ExprId, Ty>, SecondaryMap<VarId, Ty>);

impl TyMap {
    pub const fn new(expr_map: SecondaryMap<ExprId, Ty>, var_map: SecondaryMap<VarId, Ty>) -> Self {
        Self(expr_map, var_map)
    }

    pub fn expr_ty(&self, expr: ExprId) -> &Ty {
        &self.0[expr]
    }

    pub fn var_ty(&self, var: VarId) -> &Ty {
        &self.1[var]
    }
}

new_key_type! { pub struct VarId; }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VarInfo {
    pub mutable: bool,
    pub ty: Option<Ty>,
}
