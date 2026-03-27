use std::fmt::Display;

use ast::Ident;
use ena::unify::{EqUnifyValue, UnifyKey};
use string_interner::DefaultStringInterner;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ty {
    Int,
    UInt,
    Byte,
    Float,
    Bool,
    Char,
    Array(Box<Ty>),
    Tuple(Vec<Ty>),
    Func(Vec<Ty>, Box<Ty>),
    Adt(Ident, Vec<Ty>),
    Var(TyVar),
    IntVar(TyVar),
}

impl EqUnifyValue for Ty {}

impl From<ast::Ty> for Ty {
    fn from(value: ast::Ty) -> Self {
        match value.kind {
            ast::TyKind::Int => Self::Int,
            ast::TyKind::UInt => Self::UInt,
            ast::TyKind::Byte => Self::Byte,
            ast::TyKind::Float => Self::Float,
            ast::TyKind::Bool => Self::Bool,
            ast::TyKind::Char => Self::Char,
            ast::TyKind::Adt(ident, args) => {
                Self::Adt(ident, args.into_iter().map(ast::Ty::into).collect())
            }
            ast::TyKind::Array(ty) => Self::Array(Box::new(Self::from(*ty))),
            ast::TyKind::Tuple(tys) => Self::Tuple(tys.into_iter().map(ast::Ty::into).collect()),
            ast::TyKind::Fn(param_tys, return_ty) => Self::Func(
                param_tys.into_iter().map(ast::Ty::into).collect(),
                Box::new(Self::from(*return_ty)),
            ),
        }
    }
}

impl Ty {
    #[must_use]
    pub const fn unit() -> Self {
        Self::Tuple(vec![])
    }

    pub fn string(interner: &mut DefaultStringInterner) -> Self {
        Self::Adt(interner.get_or_intern("String").into(), vec![])
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
            Self::Func(param_tys, result_ty) => {
                write!(f, "fn({}): {result_ty}", itertools::join(param_tys, ", "))
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
