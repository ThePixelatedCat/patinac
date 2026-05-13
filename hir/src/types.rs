use span::Span;

use crate::AdtId;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Ty {
    pub kind: TyKind,
    pub span: Span,
}

/// The kinds of types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TyKind {
    Int,
    UInt,
    Byte,
    Float,
    Char,
    Bool,
    Tuple(Vec<Ty>),
    Fn(Vec<Param>, Return),
    Adt(AdtId, Vec<Ty>),
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

    // pub fn named(name: &str) -> Self {
    //     Self::Adt(Ident::new(name), vec![])
    // }

    // /// Helper to create a new [`TyKind::Adt`] for a `String`
    // pub fn string() -> Self {
    //     Self::named("String")
    // }

    // /// Helper to create a new [`TyKind::Adt`] for an `Array` storing the given type
    // pub fn array(inner: Ty) -> Self {
    //     Self::Adt(Ident::new("Array"), vec![inner])
    // }
}

/// A parameter of a function type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Param {
    pub mutable: bool,
    pub ty: Ty,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Return {
    pub mutable: bool,
    pub ty: Box<Ty>,
}
