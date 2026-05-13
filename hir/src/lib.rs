use foldhash::HashMap;

use slotmap::{SecondaryMap, SlotMap, new_key_type};
use smallvec::SmallVec;

use ident::{Ident, SpanIdent};
use span::Span;

use crate::{
    exprs::{Expr, ExprId},
    items::ExecItem,
    types::Ty,
};

pub mod exprs;
pub mod items;
pub mod patterns;
pub mod types;

#[derive(Debug, Default)]
pub struct Hir {
    pub execs: Vec<ExecItem>,
    adts: SlotMap<AdtId, SpanIdent>,
    adt_info: SecondaryMap<AdtId, AdtInfo>,
    exprs: SlotMap<ExprId, Expr>,
    expr_spans: SecondaryMap<ExprId, Span>,
    vars: SlotMap<VarId, SpanIdent>,
    var_info: SecondaryMap<VarId, VarInfo>,
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

new_key_type! {
    pub struct AdtId;
    pub struct VarId;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdtInfo {
    Record {
        generics: SmallVec<[AdtId; 4]>,
        fields: HashMap<Ident, FieldInfo>,
    },
    Enum {
        generics: SmallVec<[AdtId; 4]>,
        variants: HashMap<Ident, HashMap<Ident, FieldInfo>>,
    },
    Param,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldInfo {
    pub ty: Ty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VarInfo {
    pub mutable: bool,
    pub ty: Option<Ty>,
}
