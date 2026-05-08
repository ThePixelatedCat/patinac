use std::fmt::Display;

use derive_more::Display;
use itertools::Itertools;

use ident::Ident;
use span::impl_span;

use crate::items::Return;

impl_span!(TyKind<AdtIdent> as Ty<AdtIdent>, "A type with associated span");

impl<T: Eq> Eq for Ty<T> {}

impl<A: Display> Display for Ty<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.kind.fmt(f)
    }
}

/// The kinds of types as viewed by the parser
#[derive(Debug, Display, Clone, PartialEq, Eq)]
pub enum TyKind<AdtIdent> {
    Int,
    UInt,
    Byte,
    Float,
    Char,
    Bool,
    #[display("{{{}}}", _0.iter().map(|ty| &ty.kind).join(", "))]
    Tuple(Vec<Ty<AdtIdent>>),
    #[display("fn({}) -> {ret}", params.iter().join(", "))]
    Fn {
        params: Vec<Param<AdtIdent>>,
        ret: Box<Return<AdtIdent>>,
    },
    #[display("{_0}[{}]", _1.iter().map(|ty| &ty.kind).join(", "))]
    Adt(AdtIdent, Vec<Ty<AdtIdent>>),
}

impl TyKind<Ident> {
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

    /// Helper to create a new empty [`TyKind::Tuple`] for representing the Unit type
    pub const fn unit() -> Self {
        Self::Tuple(vec![])
    }
}

/// A parameter of a function type
#[derive(Debug, Clone, PartialEq, Eq)]
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
