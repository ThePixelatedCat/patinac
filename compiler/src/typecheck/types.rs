use std::fmt::Display;

use ena::unify::UnifyKey;

use crate::parser::ast::Type as AstType;

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
    Named { name: String, args: Vec<Type> },
}

impl From<AstType> for Type {
    fn from(value: AstType) -> Self {
        match value {
            AstType::Int => Self::Int,
            AstType::UInt => Self::UInt,
            AstType::Byte => Self::Byte,
            AstType::Float => Self::Float,
            AstType::Bool => Self::Bool,
            AstType::Char => Self::Char,
            AstType::Named { name, args } => Self::Named {
                name,
                args: args.into_iter().map(|ty| ty.inner.into()).collect(),
            },
            AstType::Array(ty) => Self::Array(Box::new(ty.inner.into())),
            AstType::Tuple(tys) => Self::Tuple(tys.into_iter().map(|ty| ty.inner.into()).collect()),
            AstType::Fn(param_tys, return_ty) => Self::Fn(
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
        Self::Named {
            name: name.into(),
            args: vec![],
        }
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
            Self::Named { name, args } => {
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
