use std::fmt::Display;

use derive_more::Display;
use itertools::Itertools;

use ident::{Ident, SpanIdent};
use span::Span;

/// The kinds of types
#[derive(Debug, Display, Clone, PartialEq, Eq, Hash)]
pub enum Ty<A> {
    Int,
    UInt,
    Byte,
    Float,
    Char,
    Bool,
    #[display("{{{}}}", _0.iter().join(", "))]
    Tuple(Vec<Ty<A>>),
    #[display("fn({}) -> {_1}", _0.iter().join(", "))]
    Fn(Vec<Param<A>>, Box<Return<A>>),
    #[display("{_0}[{}]", _1.iter().join(", "))]
    Adt(A, Vec<Ty<A>>),
}

impl<A> Ty<A> {
    /// Helper to create a new empty [`TyKind::Tuple`] for representing the Unit type
    pub const fn unit() -> Self {
        Self::Tuple(vec![])
    }
}

impl Ty<Ident> {
    /// Helper to create a new [`TyKind::Adt`] with no generic parameters, and handling creating the [Ident] automatically
    pub fn named(name: &str) -> Self {
        Self::Adt(Ident::new(name), vec![])
    }

    /// Helper to create a new [`TyKind::Adt`] for a `String`
    pub fn string() -> Self {
        Self::named("String")
    }

    /// Helper to create a new [`TyKind::Adt`] for an `Array` storing the given type
    pub fn array(inner: Ty<Ident>) -> Self {
        Self::Adt(Ident::new("Array"), vec![inner])
    }
}

impl Ty<SpanIdent> {
    pub fn named_span(name: &str, span: impl Into<Span>) -> Self {
        Self::Adt(Ident::new(name).span(span), vec![])
    }

    /// Helper to create a new [`TyKind::Adt`] for a `String`
    pub fn string_span(span: impl Into<Span>) -> Self {
        Self::named_span("String", span)
    }

    /// Helper to create a new [`TyKind::Adt`] for an `Array` storing the given type
    pub fn array_span(inner: Ty<SpanIdent>, span: impl Into<Span>) -> Self {
        Self::Adt(Ident::new("Array").span(span), vec![inner])
    }
}

/// A parameter of a function type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Param<AdtIdent> {
    pub mutable: bool,
    pub ty: Ty<AdtIdent>,
}

impl<A: Display> Display for Param<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.mutable {
            "mut ".fmt(f)?;
        }
        self.ty.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Return<AdtIdent> {
    pub mutable: bool,
    pub ty: Ty<AdtIdent>,
}

impl<T: Display> Display for Return<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.mutable {
            "mut ".fmt(f)?;
        }
        self.ty.fmt(f)
    }
}
