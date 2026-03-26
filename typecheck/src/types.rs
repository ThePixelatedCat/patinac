use std::fmt::Display;

use ena::unify::UnifyKey;

use crate::{hir::Ty as HirTy, hir::TyKind, resolver::DefId};

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    UInt,
    Byte,
    Float,
    Bool,
    Char,
    Array(Box<Type>),
    Tuple(Vec<Type>),
    Fn(Vec<Type>, Box<Type>),
    Var(TypeId),
    IntVar(TypeId),
    Adt(DefId, Vec<Type>),
}

impl From<HirTy> for Type {
    fn from(value: HirTy) -> Self {
        match value.kind {
            TyKind::Int => Self::Int,
            TyKind::UInt => Self::UInt,
            TyKind::Byte => Self::Byte,
            TyKind::Float => Self::Float,
            TyKind::Bool => Self::Bool,
            TyKind::Char => Self::Char,
            TyKind::Adt(id) => {
                Self::Adt(name, args.into_iter().map(|ty| ty.inner.into()).collect())
            }
            TyKind::Array(ty) => Self::Array(Box::new(ty.inner.into())),
            TyKind::Tuple(tys) => Self::Tuple(tys.into_iter().map(|ty| ty.inner.into()).collect()),
            TyKind::Fn(param_tys, return_ty) => Self::Fn(
                param_tys.into_iter().map(|ty| ty.inner.into()).collect(),
                Box::new(return_ty.inner.into()),
            ),
        }
    }
}

impl Type {
    pub const fn id(&self) -> Option<TypeId> {
        match self {
            Self::Var(id) | Self::IntVar(id) => Some(*id),
            _ => None,
        }
    }

    pub fn named(name: &str) -> Self {
        Self::Adt(name.into(), vec![])
    }

    pub const fn unit() -> Self {
        Self::Tuple(vec![])
    }

    pub fn string() -> Self {
        Self::named("String")
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Self::Var(_) => "{var}".fmt(f),
            Self::IntVar(_) => "{integer}".fmt(f),
            Self::Adt(name, args) => {
                write!(f, "{name}")?;
                if !args.is_empty() {
                    write!(f, "<{}>", itertools::join(args, ", "))?;
                }
                Ok(())
            }
            Self::Int => "Int".fmt(f),
            Self::UInt => "UInt".fmt(f),
            Self::Byte => "Byte".fmt(f),
            Self::Float => "Float".fmt(f),
            Self::Bool => "Bool".fmt(f),
            Self::Char => "Char".fmt(f),
            Self::Array(ty) => write!(f, "[{ty}]"),
            Self::Tuple(tys) => write!(f, "({})", itertools::join(tys, ", ")),
            Self::Fn(param_tys, result_ty) => {
                write!(f, "fn({}): {result_ty}", itertools::join(param_tys, ", "))
            }
        }
    }
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct TypeId(u32);

impl UnifyKey for TypeId {
    type Value = Type;

    fn index(&self) -> u32 {
        self.0
    }

    fn from_index(id: u32) -> Self {
        Self(id)
    }

    fn tag() -> &'static str {
        "TypeId"
    }
}
