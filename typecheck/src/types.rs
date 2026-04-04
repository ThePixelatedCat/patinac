use std::fmt::Display;

use ena::unify::{EqUnifyValue, UnifyKey};

use ast::types::{Ty as AstTy, TyKind as AstTyKind};
use ident::Ident;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConcreteTy {
    Int,
    UInt,
    Byte,
    Float,
    Bool,
    Char,
    Array(Box<Self>),
    Tuple(Vec<Self>),
    Func(Vec<Param<Self>>, Box<Self>),
    Adt(Ident, Vec<Self>),
}

impl Display for ConcreteTy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
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
            Self::Func(params, result_ty) => {
                write!(f, "fn({}): {result_ty}", itertools::join(params, ", "))
            }
        }
    }
}

impl ConcreteTy {
    pub const fn unit() -> Self {
        Self::Tuple(vec![])
    }

    pub fn string() -> Self {
        Self::Adt(Ident::new("String"), vec![])
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Ty {
    Int,
    UInt,
    Byte,
    Float,
    Bool,
    Char,
    Array(Box<Self>),
    Tuple(Vec<Self>),
    Func(Vec<Param<Self>>, Box<Self>),
    Adt(Ident, Vec<Self>),
    Var(TyVar),
    IntVar(TyVar),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Param<T: Display> {
    pub mutable: bool,
    pub ty: T,
}

impl<T: Display> Display for Param<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.mutable, self.ty)
    }
}

impl EqUnifyValue for Ty {}

impl From<&AstTyKind> for Ty {
    fn from(value: &AstTyKind) -> Self {
        match &value {
            AstTyKind::Int => Self::Int,
            AstTyKind::UInt => Self::UInt,
            AstTyKind::Byte => Self::Byte,
            AstTyKind::Float => Self::Float,
            AstTyKind::Bool => Self::Bool,
            AstTyKind::Char => Self::Char,
            AstTyKind::Adt(ident, args) => Self::Adt(*ident, args.iter().map(Ty::from).collect()),
            AstTyKind::Array(ty) => Self::Array(Box::new(Self::from(ty.as_ref()))),
            AstTyKind::Tuple(tys) => Self::Tuple(tys.iter().map(Ty::from).collect()),
            AstTyKind::Fn(params, return_ty) => Self::Func(
                params
                    .iter()
                    .map(|param| Param {
                        mutable: param.mutable,
                        ty: (&param.ty).into(),
                    })
                    .collect(),
                Box::new(Self::from(return_ty.as_ref())),
            ),
        }
    }
}

impl From<&AstTy> for Ty {
    fn from(value: &AstTy) -> Self {
        Self::from(&value.kind)
    }
}

impl Ty {
    pub const fn unit() -> Self {
        Self::Tuple(vec![])
    }

    pub fn string() -> Self {
        Self::Adt(Ident::new("String"), vec![])
    }
}

impl Display for Ty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Self::Var(_) => "{var}".fmt(f),
            Self::IntVar(_) => "{integer}".fmt(f),
            Self::Adt(name, args) => {
                write!(f, "temp{name:?}")?; //TODO properly print error
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
            Self::Func(params, result_ty) => {
                write!(f, "fn({}): {result_ty}", itertools::join(params, ", "))
            }
        }
    }
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct TyVar(u32);

impl UnifyKey for TyVar {
    type Value = Option<Ty>;

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
