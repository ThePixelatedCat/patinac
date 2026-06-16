use std::{
    fmt::{self, Display, Formatter},
    range::Range,
};

use ena::unify::{EqUnifyValue, UnifyKey};

use hir::{Ty, TyId};

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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PartialTy {
    Int,
    UInt,
    Byte,
    Float,
    Bool,
    Char,
    Tuple(Vec<Self>),
    Array(Box<Self>),
    Fn(Vec<Param>, Box<Self>),
    Named(TyId),
    Var(TyVar),
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
            Ty::Char => Self::Char,
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

impl Display for PartialTy {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self {
            Self::Int => "Int".fmt(f),
            Self::UInt => "UInt".fmt(f),
            Self::Byte => "Byte".fmt(f),
            Self::Float => "Float".fmt(f),
            Self::Bool => "Bool".fmt(f),
            Self::Char => "Char".fmt(f),
            Self::Tuple(tys) => write!(f, "#({})", itertools::join(tys, ", ")),
            Self::Array(ty) => write!(f, "Array[{ty}]"),
            Self::Fn(params, result_ty) => {
                write!(f, "fn({}) -> {result_ty}", itertools::join(params, ", "))
            }
            Self::Named(name) => {
                write!(f, "temp{name:?}") //TODO properly print
            }
            Self::Var(_) => "{var}".fmt(f),
            Self::IntVar(_) => "{integer}".fmt(f),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Param {
    pub ty: PartialTy,
    pub mutable: bool,
    pub span: Range<u32>,
}

impl Display for Param {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.mutable {
            "mut ".fmt(f)?;
        }
        self.ty.fmt(f)
    }
}

pub fn convert(table: &mut Table, ast_ty: Option<&Ty>) -> PartialTy {
    ast_ty.map_or_else(|| PartialTy::var(table), PartialTy::from)
}
