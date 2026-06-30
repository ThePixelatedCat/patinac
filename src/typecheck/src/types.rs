use std::range::Range;

use derive_more::Display;
use ena::unify::{EqUnifyValue, UnifyKey};

use irs::hir::{Ty, TyId};

use crate::Table;

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct TyVar(u32);

impl UnifyKey for TyVar {
    type Value = Option<PartialTy>;

    fn index(&self) -> u32 {
        self.0
    }

    fn from_index(u: u32) -> Self {
        Self(u)
    }

    fn tag() -> &'static str {
        "TypeVar"
    }
}

#[derive(Debug, Display, Clone, PartialEq, Eq, Hash)]
pub enum PartialTy {
    #[display("Int")]
    Int,
    #[display("UInt")]
    UInt,
    #[display("Byte")]
    Byte,
    #[display("Float")]
    Float,
    #[display("Bool")]
    Bool,
    #[display("({})", itertools::join(_0, ", "))]
    Tuple(Vec<Self>),
    #[display("[{_0}]")]
    Array(Box<Self>),
    #[display("fn({}) -> {_1}", itertools::join(_0, ", "))]
    Fn(Vec<Param>, Box<Self>),
    #[display("temp{_0:?}")]
    Named(TyId),
    #[display("{{var}}")]
    Var(TyVar),
    #[display("{{integer}}")]
    IntVar(TyVar),
}

impl PartialTy {
    pub const fn unit() -> Self {
        Self::Tuple(vec![])
    }

    pub fn var(table: &mut Table) -> Self {
        Self::Var(table.new_key(None))
    }

    pub fn int_var(table: &mut Table) -> Self {
        Self::IntVar(table.new_key(None))
    }
}

impl EqUnifyValue for PartialTy {}

impl From<&Ty> for PartialTy {
    fn from(value: &Ty) -> Self {
        match &value {
            Ty::Int => Self::Int,
            Ty::UInt => Self::UInt,
            Ty::Byte => Self::Byte,
            Ty::Float => Self::Float,
            Ty::Bool => Self::Bool,
            Ty::Tuple(tys) => Self::Tuple(tys.iter().map(Self::from).collect()),
            Ty::Array(ty) => Self::Array(Box::new(ty.as_ref().into())),
            Ty::Func(params, ret) => {
                let params = params
                    .iter()
                    .map(|param| Param {
                        ty: (&param.ty).into(),
                        mutable: param.mutable,
                        span: param.span,
                    })
                    .collect();
                let ret = Box::new(Self::from(&**ret));
                Self::Fn(params, ret)
            }
            Ty::Named(id) => Self::Named(*id),
        }
    }
}

#[derive(Debug, Display, Clone, PartialEq, Eq, Hash)]
#[display("{}{ty}", if *mutable { "mut " } else { "" })]
pub struct Param {
    pub ty: PartialTy,
    pub mutable: bool,
    pub span: Range<u32>,
}

pub fn convert(table: &mut Table, ast_ty: Option<&Ty>) -> PartialTy {
    ast_ty.map_or_else(|| PartialTy::var(table), PartialTy::from)
}
