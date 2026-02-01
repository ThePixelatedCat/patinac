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
    Func(Vec<Type>, Box<Type>),
    Var(TypeId),
    IntVar(TypeId),
    Named { name: String, generics: Vec<Type> },
}

impl From<AstType> for Type {
    fn from(value: AstType) -> Self {
        // TODO handle primitives properly
        match value {
            AstType::Named { name, .. } if name == "Int" => Type::Int,
            AstType::Named { name, generics } => Type::Named {
                name,
                generics: generics.into_iter().map(|ty| ty.inner.into()).collect(),
            },
            AstType::Array(ty) => Type::Array(Box::new((*ty).inner.into())),
            AstType::Tuple(tys) => Type::Tuple(tys.into_iter().map(|ty| ty.inner.into()).collect()),
            AstType::Fn { params, result } => Type::Func(
                params.into_iter().map(|ty| ty.inner.into()).collect(),
                Box::new((*result).inner.into()),
            ),
        }
    }
}

impl Type {
    pub fn id(&self) -> Option<TypeId> {
        match self {
            Type::Var(id) => Some(*id),
            _ => None,
        }
    }

    pub fn named(name: &str) -> Self {
        Self::Named {
            name: name.into(),
            generics: vec![],
        }
    }

    pub fn unit() -> Self {
        Self::Tuple(vec![])
    }

    pub fn string() -> Self {
        Self::named("String")
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Type::Var(_) => "{var}".fmt(f),
            Type::IntVar(_) => "{integer}".fmt(f),
            Type::Named {
                name,
                generics: args,
            } => {
                write!(f, "{name}")?;
                if !args.is_empty() {
                    write!(f, "<{}>", itertools::join(args, ", "))?;
                }
                Ok(())
            }
            Type::Int => "Int".fmt(f),
            Type::UInt => "UInt".fmt(f),
            Type::Byte => "Byte".fmt(f),
            Type::Float => "Float".fmt(f),
            Type::Bool => "Bool".fmt(f),
            Type::Char => "Char".fmt(f),
            Type::Array(ty) => write!(f, "[{ty}]"),
            Type::Tuple(tys) => write!(f, "({})", itertools::join(tys, ", ")),
            Type::Func(param_tys, result_ty) => {
                write!(f, "fn({}): {result_ty}", itertools::join(param_tys, ", "))
            }
        }
    }
}

// impl Display for TypeS {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         self.inner.fmt(f)
//     }
// }

// impl From<AstTypeS> for TypeS {
//     fn from(ty: AstTypeS) -> Self {
//         Spanned::span(ty.inner.into(), ty.span)
//     }
// }

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct TypeId(u32);

impl From<u32> for TypeId {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl UnifyKey for TypeId {
    type Value = Type;
    fn index(&self) -> u32 {
        self.0
    }
    fn from_index(u: u32) -> TypeId {
        u.into()
    }
    fn tag() -> &'static str {
        "TypeId"
    }
}
