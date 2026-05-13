use std::{
    fmt::Display,
    sync::atomic::{AtomicU32, Ordering},
};

use derive_more::Display;
use itertools::Itertools;

use ident::{Ident, SpanIdent};
use span::Span;

/// The kinds of types
#[derive(Debug, Display, Clone, PartialEq, Eq, Hash)]
pub enum Ty {
    Int,
    UInt,
    Byte,
    Float,
    Char,
    Bool,
    #[display("{{{}}}", _0.iter().join(", "))]
    Tuple(Vec<Self>),
    #[display("fn({}) -> {_1}", _0.iter().join(", "))]
    Fn(Vec<Param>, Box<Return>),
    #[display("{_0}[{}]", _2.iter().join(", "))]
    Adt(SpanIdent, AdtId, Vec<Self>),
}

impl Ty {}

// impl Ty {
//     /// Helper to create a new [`TyKind::Adt`] with no generic parameters, and handling creating the [Ident] automatically
//     pub fn named(name: &str) -> Self {
//         Self::Adt(Ident::new(name), vec![])
//     }

//     /// Helper to create a new [`TyKind::Adt`] for a `String`
//     pub fn string() -> Self {
//         Self::named("String")
//     }

//     /// Helper to create a new [`TyKind::Adt`] for an `Array` storing the given type
//     pub fn array(inner: Self) -> Self {
//         Self::Adt(Ident::new("Array"), vec![inner])
//     }
// }

impl Ty {
    /// Helper to create a new empty [`TyKind::Tuple`] for representing the Unit type
    pub const fn unit() -> Self {
        Self::Tuple(vec![])
    }

    pub fn adt(ident: SpanIdent, args: Vec<Self>) -> Self {
        Self::Adt(ident, AdtId::new(), args)
    }

    pub fn named_span(name: &str, span: impl Into<Span>) -> Self {
        Self::Adt(Ident::new(name).span(span), AdtId::new(), vec![])
    }

    /// Helper to create a new [`TyKind::Adt`] for a `String`
    pub fn string_span(span: impl Into<Span>) -> Self {
        Self::named_span("String", span)
    }

    /// Helper to create a new [`TyKind::Adt`] for an `Array` storing the given type
    pub fn array_span(inner: Self, span: impl Into<Span>) -> Self {
        Self::Adt(Ident::new("Array").span(span), AdtId::new(), vec![inner])
    }
}

/// A parameter of a function type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Param {
    pub mutable: bool,
    pub ty: Ty,
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
    pub ty: Ty,
}

impl Display for Return {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.mutable {
            "mut ".fmt(f)?;
        }
        self.ty.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdtId(u32);
static ADT_ID_CTR: AtomicU32 = AtomicU32::new(0);

impl AdtId {
    pub fn new() -> Self {
        Self(ADT_ID_CTR.fetch_add(1, Ordering::Relaxed))
    }
}
