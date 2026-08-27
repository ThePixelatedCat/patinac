use std::{
    fmt::{self, Display, Formatter},
    range::Range,
};

use errors::{Diagnostic, Report};
use ident::Ident;

#[derive(Debug, PartialEq, Eq)]
pub enum ErrorKind {
    UnknownName(ItemKind, Ident),
    PrivateItem(ItemKind, Ident),
    DuplicateItem(ItemKind, Ident),
    Mutation(Ident),
    NotPlaceExpr,
    OverlappingArgs(Range<u32>, Range<u32>),
    DupFields(Ident),
    InvalidMain,
    SelfOutsideImpl,
}

impl Diagnostic for ErrorKind {
    fn report(self) -> Report {
        match self {
            Self::UnknownName(kind, ident) => Report::error(format!("unresolved {kind}"))
                .with_label(format!("cannot resolve `{ident}`")),
            Self::PrivateItem(kind, ident) => Report::error(format!("private {kind}"))
                .with_label(format!("`{ident}` is not visible")),
            Self::DuplicateItem(kind, ident) => Report::error(format!("duplicate {kind}"))
                .with_label(format!(
                    "there is already a {kind} named `{ident}` in this scope"
                )),
            Self::Mutation(ident) => Report::error("attempted mutation of immutable variable")
                .with_label(format!("variable `{ident}` is not mutable")),
            Self::NotPlaceExpr => Report::error("attempted mutation of immutable value")
                .with_note("only mutable variables, fields, and indices can be mutated"),
            Self::OverlappingArgs(range, range1) => Report::error("overlapping mutable arguments")
                .with_label("this mutable argument is not unique"),
            Self::DupFields(ident) => Report::error("duplicate field"),
            Self::InvalidMain => Report::error("invalid main function")
                .with_note("the main function must take no parameters and have a unit return type"),
            Self::SelfOutsideImpl => Report::error("self parameter on free function")
                .with_note("self parameters can only appear on functions within impl blocks"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemKind {
    Module,
    Type,
    Value,
    Unknown,
}

impl Display for ItemKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Module => "module",
            Self::Type => "type",
            Self::Value => "value",
            Self::Unknown => "item",
        }
        .fmt(f)
    }
}
