use std::sync::atomic::{AtomicU32, Ordering};

use slotmap::{SecondaryMap, SlotMap};

use span::Span;

use crate::{
    exprs::{Expr, ExprId},
    items::{AdtItem, ExecItem},
};

pub mod exprs;
pub mod items;
pub mod patterns;

#[derive(Debug, Default)]
pub struct Ast {
    pub adts: Vec<AdtItem>,
    pub execs: Vec<ExecItem>,
    exprs: SlotMap<ExprId, Expr>,
    expr_spans: SecondaryMap<ExprId, Span>,
}

impl PartialEq for Ast {
    fn eq(&self, other: &Self) -> bool {
        self.adts == other.adts
            && self.execs == other.execs
            && self.exprs.iter().eq(other.exprs.iter())
            && self.expr_spans == other.expr_spans
    }
}

impl Ast {
    pub fn add_expr(&mut self, expr: Expr, span: impl Into<Span>) -> ExprId {
        let id = self.exprs.insert(expr);
        self.expr_spans.insert(id, span.into());
        id
    }

    pub fn expr(&self, id: ExprId) -> &Expr {
        &self.exprs[id]
    }

    pub fn span_of(&self, id: ExprId) -> Span {
        self.expr_spans[id]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VarId(u32);
static VAR_ID_CTR: AtomicU32 = AtomicU32::new(0);

impl VarId {
    pub fn new() -> Self {
        Self(VAR_ID_CTR.fetch_add(1, Ordering::AcqRel))
    }
}

// #[derive(Debug, Clone, PartialEq, Eq)]
// pub struct Path<PreIdent> {
//     pub prefix: SmallVec<[PreIdent; 4]>,
//     pub end: Ident,
// }
