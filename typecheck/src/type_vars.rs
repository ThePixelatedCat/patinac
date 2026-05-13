use std::fmt::Display;

use ena::unify::{EqUnifyValue, UnifyKey};

use hir::{AdtId, types::Ty};
use ident::Ident;

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
    Fn(Vec<Param>, Return),
    Adt(AdtId, Vec<Self>),
    Var(TyVar),
    IntVar(TyVar),
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
            Ty::Tuple(tys) => Self::Tuple(tys.iter().map(PartialTy::from).collect()),
            Ty::Fn(params, ret) => Self::Fn(
                params
                    .iter()
                    .map(|param| Param {
                        mutable: param.mutable,
                        ty: (&param.ty).into(),
                    })
                    .collect(),
                Return {
                    mutable: ret.mutable,
                    ty: Box::new(Self::from(&*ret.ty)),
                },
            ),
            Ty::Adt(id, args) => Self::Adt(*id, args.iter().map(PartialTy::from).collect()),
        }
    }
}

impl From<Ty> for PartialTy {
    fn from(value: Ty) -> Self {
        Self::from(&value)
    }
}

impl Display for PartialTy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Self::Var(_) => "{var}".fmt(f),
            Self::IntVar(_) => "{integer}".fmt(f),
            Self::Adt(name, args) => {
                write!(f, "temp{name:?}")?; //TODO properly print error
                if !args.is_empty() {
                    write!(f, "[{}]", itertools::join(args, ", "))?;
                }
                Ok(())
            }
            Self::Int => "Int".fmt(f),
            Self::UInt => "UInt".fmt(f),
            Self::Byte => "Byte".fmt(f),
            Self::Float => "Float".fmt(f),
            Self::Bool => "Bool".fmt(f),
            Self::Char => "Char".fmt(f),
            Self::Tuple(tys) => write!(f, "#({})", itertools::join(tys, ", ")),
            Self::Fn(params, result_ty) => {
                write!(f, "fn({}) -> {result_ty}", itertools::join(params, ", "))
            }
        }
    }
}

impl PartialTy {
    pub const fn unit() -> Self {
        Self::Tuple(vec![])
    }

    pub fn string() -> Self {
        Self::Adt(Ident::new("String"), vec![])
    }

    pub fn array(inner: PartialTy) -> Self {
        Self::Adt(Ident::new("Array"), vec![inner])
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Param {
    pub mutable: bool,
    pub ty: PartialTy,
}

impl Display for Param {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.mutable {
            true => write!(f, "mut {}", self.ty),
            false => self.ty.fmt(f),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Return {
    pub mutable: bool,
    pub ty: Box<PartialTy>,
}

impl Display for Return {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.mutable {
            true => write!(f, "mut {}", self.ty),
            false => self.ty.fmt(f),
        }
    }
}
