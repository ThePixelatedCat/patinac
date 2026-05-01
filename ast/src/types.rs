use std::fmt::Display;

use derive_more::Display;
use itertools::Itertools;

use ident::Ident;
use span::impl_span;

impl_span!(TyKind<AdtIdent> as Ty<AdtIdent>);

impl<T: Eq> Eq for Ty<T> {}

impl<A: Display> Display for Ty<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.kind.fmt(f)
    }
}

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
    #[display("fn({}) -> {result}", params.iter().join(", "))]
    Fn {
        params: Vec<Param<AdtIdent>>,
        result: Box<Ty<AdtIdent>>,
    },
    #[display("{_0}[{}]", _1.iter().map(|ty| &ty.kind).join(", "))]
    Adt(AdtIdent, Vec<Ty<AdtIdent>>),
}

fn fn_display_helper<A: Display>(generics: &[A]) -> String {
    if generics.is_empty() {
        String::new()
    } else {
        format!("[{}]", generics.iter().join(", "))
    }
}

impl TyKind<Ident> {
    pub fn named(name: &str) -> Self {
        Self::Adt(Ident::new(name), vec![])
    }

    pub fn string() -> Self {
        Self::named("String")
    }

    pub fn array(inner: Ty<Ident>) -> Self {
        Self::Adt(Ident::new("Array"), vec![inner])
    }

    pub const fn unit() -> Self {
        Self::Tuple(vec![])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param<AdtIdent> {
    pub mutable: bool,
    pub ty: Ty<AdtIdent>,
}

impl<A: Display> Display for Param<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.mutable {
            "mut".fmt(f)?;
        }
        write!(f, "{}", self.ty.kind)
    }
}
