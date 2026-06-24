//! The high-level intermediate representation of Patina. Produced after name resolution, and used for typechecking.

mod exprs;

use std::range::Range;

use foldhash::HashMap;
use package::ModuleId;
use slotmap::{SecondaryMap, SlotMap, new_key_type};

use ident::{Ident, SpanIdent};

pub use exprs::*;

/// An AST-like structure, with additional metadata in the form of type information and resolved variable identifiers.
#[derive(Debug, Default)]
pub struct Hir {
    main: Option<ExecItem>,
    execs: Vec<ExecItem>,
    tys: SlotMap<TyId, SpanIdent>,
    ty_info: SecondaryMap<TyId, TyInfo>,
    exprs: SlotMap<ExprId, (Expr, Range<u32>)>,
    vars: SlotMap<VarId, VarInfo>,
    var_tys: SecondaryMap<VarId, Ty>,
}

impl Hir {
    /// Returns all of the "executable items". These are the items that contain expressions.
    pub fn execs(&self) -> &[ExecItem] {
        &self.execs
    }

    /// Adds an "executable item". These are the items that contain expressions.
    pub fn add_exec(&mut self, exec: ExecItem) {
        self.execs.push(exec);
    }

    /// Returns the main function, if it exists.
    pub const fn main(&self) -> Option<&ExecItem> {
        self.main.as_ref()
    }

    /// Sets the main function.
    pub fn set_main(&mut self, main: ExecItem) {
        self.main = Some(main);
    }
}

// Type-related functions
impl Hir {
    /// Returns an iterator over all user-defined types.
    pub fn tys(&self) -> impl Iterator<Item = TyId> {
        self.tys.keys()
    }

    /// Reserves a user-defined type, storing only it's name and returning a unique ID for it.
    ///
    /// This allows getting an ID for a type before having the full information for it.
    /// The reservation should be fullfilled with [`fulfill_ty`][Self::fulfill_ty] as soon as possible.
    pub fn reserve_ty(&mut self, ident: SpanIdent) -> TyId {
        self.tys.insert(ident)
    }

    /// Fulfills a previously-reserved user-defined type, providing the rest of the information for the type.
    ///
    /// See [`reserve_ty`][Self::reserve_ty] for more information.
    pub fn fulfill_ty(&mut self, id: TyId, info: TyInfo) {
        self.ty_info.insert(id, info);
    }

    /// Returns the name of the given type.
    pub fn ty_ident(&self, id: TyId) -> SpanIdent {
        self.tys[id]
    }

    /// Returns the information for the given type.
    ///
    /// # Panics
    /// Panics if the type was reserved with [`reserve_ty`][Self::reserve_ty], but not yet fulfilled with [`fulfill_ty`][Self::fulfill_ty].
    pub fn ty_info(&self, id: TyId) -> &TyInfo {
        &self.ty_info[id]
    }
}

// Expr-related functions
impl Hir {
    /// Adds a spanned expression, returning a unique ID for it.
    pub fn add_expr(&mut self, expr: Expr, span: impl Into<Range<u32>>) -> ExprId {
        self.exprs.insert((expr, span.into()))
    }

    /// Returns the actual expression associated with the given ID.
    pub fn expr(&self, id: ExprId) -> &Expr {
        &self.exprs[id].0
    }

    /// Returns the span of the expression associated with the given ID.
    pub fn expr_span(&self, id: ExprId) -> Range<u32> {
        self.exprs[id].1
    }
}

// Var-related functions
impl Hir {
    /// Adds a variable with the provided information, returning a unique ID for it.
    pub fn add_var(&mut self, info: VarInfo) -> VarId {
        self.vars.insert(info)
    }

    /// Returns the information for the given variable.
    pub fn var_info(&self, id: VarId) -> VarInfo {
        self.vars[id]
    }

    /// Adds a type for the given variable.
    pub fn add_var_ty(&mut self, id: VarId, ty: Ty) {
        self.var_tys.insert(id, ty);
    }

    /// Returns the type for the given variable.
    ///
    /// # Panics
    /// Panics if the variable has not yet been typed. Prior to typechecking, [`try_var_ty`][Self::try_var_ty] should be used instead.
    pub fn var_ty(&self, id: VarId) -> &Ty {
        &self.var_tys[id]
    }

    /// Returns the type for the given variable, if it has been typed yet.
    pub fn try_var_ty(&self, id: VarId) -> Option<&Ty> {
        self.var_tys.get(id)
    }
}

new_key_type! {
    /// An ID representing a user-defined type. Information about the type can be acquired through [`Hir::ty_ident`] and [`Hir::ty_info`].
    pub struct TyId;
}
/// Information for a user-defined type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TyInfo {
    /// The fields of the type.
    pub fields: HashMap<Ident, Field>,
    /// The ID of the type's constructor function.
    pub ctor: VarId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// A field of a `record` or of a `union` variant.
pub struct Field {
    /// The type of the field.
    pub ty: Ty,
    /// The span of the field.
    pub span: Range<u32>,
}

/// An "executable item". These are the items that contain expressions, namely constants and functions.
#[derive(Debug, PartialEq, Eq)]
pub struct ExecItem {
    /// The name of the item.
    pub var: VarId,
    /// The kind of the item (constant or function).
    pub kind: ExecKind,
    /// The module containing the item's definition.
    pub module: ModuleId,
}

/// The information of an [`ExecItem`] specific to whether it's a constant or function.
#[derive(Debug, PartialEq, Eq)]
pub enum ExecKind {
    /// A constant item.
    Const(ExprId),
    /// A function item.
    Func {
        /// The value parameters.
        params: Vec<VarId>,
        /// The body.
        body: ExprId,
    },
}

new_key_type! {
    /// An ID representing a variable. Information about the variable can be acquired through [`Hir::var_info`].
    pub struct VarId;
}
/// Information for a variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VarInfo {
    /// The name of the variable, with span.
    pub ident: SpanIdent,
    /// Whether the variable was declared mutable.
    pub mutable: bool,
    /// Whether the variable is global (i.e. represents an [`ExecItem`]).
    pub global: bool,
    /// The module containing the variable's declaration.
    pub module: ModuleId,
}

/// The kinds of types.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Ty {
    /// 64-bit signed integer.
    Int,
    /// 64-bit unsigned integer.
    UInt,
    /// 8-bit unsigned integer.
    Byte,
    /// Double-precision floating point number (binary64).
    Float,
    /// Truth value (`true`/`false`).
    Bool,
    /// A dynamic homogenous array.
    Array(Box<Self>),
    /// A heterogenous tuple (compile-time length).
    Tuple(Vec<Self>),
    /// A first-class function value, implemented as a closure.
    Func(Vec<Param>, Box<Self>),
    /// A user-defined type, such as a `record` or `union`.
    Named(TyId),
}

impl Ty {
    /// Constructs an empty [`Ty::Tuple`] for representing the Unit type.
    pub const fn unit() -> Self {
        Self::Tuple(vec![])
    }
}

/// A parameter of a function type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Param {
    /// The type of the paremeter.
    pub ty: Ty,
    /// Whether the parameter is mutable.
    pub mutable: bool,
    /// The total span of the parameter.
    /// If `mutable` is false, this should be identical to the span of `ty`.
    /// Otherwise, it should include the span of the `mut` keyword.
    pub span: Range<u32>,
}
