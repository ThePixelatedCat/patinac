use crate::AdtId;

/// The kinds of types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Ty {
    Int,
    UInt,
    Byte,
    Float,
    Char,
    Bool,
    Tuple(Vec<Self>),
    Array(Box<Self>),
    Fn(Vec<Param>, Box<Self>),
    Adt(AdtId),
}

impl Ty {
    /// Helper to create a new empty [`TyKind::Tuple`] for representing the Unit type
    pub const fn unit() -> Self {
        Self::Tuple(vec![])
    }
}

/// A parameter of a function type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Param {
    pub ty: Ty,
    pub mutable: bool,
}
