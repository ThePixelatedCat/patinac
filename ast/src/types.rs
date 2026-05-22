use std::fmt::Display;

use derive_more::Display;
use itertools::Itertools;

use ident::Ident;
use span::Span;

#[derive(Debug, Display, Clone, PartialEq, Eq, Hash)]
#[display("{kind}")]
pub struct Ty {
    pub kind: TyKind,
    pub span: Span,
}

/// The kinds of types
#[derive(Debug, Display, Clone, PartialEq, Eq, Hash)]
pub enum TyKind {
    Int,
    UInt,
    Byte,
    Float,
    Char,
    Bool,
    #[display("{{{}}}", _0.iter().join(", "))]
    Tuple(Vec<Ty>),
    #[display("fn({}) -> {_1}", _0.iter().join(", "))]
    Fn(Vec<Param>, Return),
    #[display("{_0}[{}]", _1.iter().join(", "))]
    Adt(Ident, Vec<Ty>),
}

impl TyKind {
    pub fn span(self, span: impl Into<Span>) -> Ty {
        Ty {
            kind: self,
            span: span.into(),
        }
    }

    /// Helper to create a new empty [`TyKind::Tuple`] for representing the Unit type
    pub const fn unit() -> Self {
        Self::Tuple(vec![])
    }

    pub fn named(name: &str) -> Self {
        Self::Adt(Ident::new(name), vec![])
    }

    /// Helper to create a new [`TyKind::Adt`] for a `String`
    pub fn string() -> Self {
        Self::named("String")
    }

    /// Helper to create a new [`TyKind::Adt`] for an `Array` storing the given type
    pub fn array(inner: Ty) -> Self {
        Self::Adt(Ident::new("Array"), vec![inner])
    }
}

/// A parameter of a function type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Param {
    pub ty: Ty,
    pub mutable: bool,
    pub span: Span,
}

impl Display for Param {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.mutable {
            "mut ".fmt(f)?;
        }
        self.ty.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Return {
    pub mutable: bool,
    pub ty: Box<Ty>,
}

impl Display for Return {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.mutable {
            "mut ".fmt(f)?;
        }
        self.ty.fmt(f)
    }
}
