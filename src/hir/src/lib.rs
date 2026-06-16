//! The high-level intermediate representation of Patina. Produced after name resolution, and used for typechecking.

mod exprs;

use std::{mem, range::Range};

use derive_more::{From, IntoIterator};
use foldhash::HashMap;
use package::ModuleId;
use slotmap::{SecondaryMap, SlotMap, new_key_type};

use ident::{Ident, SpanIdent};

pub use exprs::*;

#[derive(Debug, Default)]
pub struct Hir {
    main: Option<ExecItem>,
    execs: Vec<ExecItem>,
    tys: SlotMap<TyId, SpanIdent>,
    ty_info: SecondaryMap<TyId, TyInfo>,
    exprs: SlotMap<ExprId, Expr>,
    expr_spans: SecondaryMap<ExprId, Range<u32>>,
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

    pub fn take_execs(&mut self) -> Vec<ExecItem> {
        mem::take(&mut self.execs)
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
    pub fn add_expr(&mut self, expr: Expr, span: impl Into<Range<u32>>) -> ExprId {
        let id = self.exprs.insert(expr);
        self.expr_spans.insert(id, span.into());
        id
    }

    pub fn expr(&self, id: ExprId) -> &Expr {
        &self.exprs[id]
    }

    pub fn take_expr(&mut self, id: ExprId) -> Expr {
        self.exprs
            .remove(id)
            .expect("id was gotten from this slotmap")
    }

    pub fn expr_span(&self, id: ExprId) -> Range<u32> {
        self.expr_spans[id]
    }
}

// Var-related functions
impl Hir {
    pub fn add_var(
        &mut self,
        ident: Ident,
        mutable: bool,
        span: Range<u32>,
        module: ModuleId,
    ) -> VarId {
        self.vars.insert(VarInfo {
            ident,
            mutable,
            span,
            module,
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

    pub fn try_var_ty(&self, id: VarId) -> Option<&Ty> {
        self.var_tys.get(id)
    }

    pub fn var_tys(&self) -> impl Iterator<Item = (VarId, Option<&Ty>)> {
        self.vars.iter().map(|(id, _)| (id, self.var_tys.get(id)))
    }
}

new_key_type! { pub struct TyId; }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TyInfo {
    pub fields: HashMap<Ident, Field>,
    pub ctor: VarId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// A field of a `record` or of an `enum` variant.
pub struct Field {
    /// Whether the field is public.
    pub public: bool,
    /// The type of the field.
    pub ty: Ty,
    /// The span of the field.
    pub span: Range<u32>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ExecItem {
    pub module: ModuleId,
    pub var: VarId,
    pub kind: ExecKind,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ExecKind {
    Const(ExprId),
    Func { params: Vec<VarId>, body: ExprId },
}

new_key_type! { pub struct VarId; }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VarInfo {
    pub ident: Ident,
    pub mutable: bool,
    pub span: Range<u32>,
    pub module: ModuleId,
}

/// The kinds of types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Ty {
    Int,
    UInt,
    Byte,
    Float,
    Char,
    Bool,
    Tuple(Vec<Self>),
    Array(Box<Self>),
    Func(Vec<Param>, Box<Self>),
    Named(TyId),
}

impl Ty {
    /// Helper to create a new empty [`TyKind::Tuple`] for representing the Unit type
    pub const fn unit() -> Self {
        Self::Tuple(vec![])
    }
}

/// A parameter of a function type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Param {
    pub ty: Ty,
    pub mutable: bool,
    pub span: Range<u32>,
}
