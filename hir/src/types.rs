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
    Fn(Vec<Param>, Return),
    Adt(AdtId, Vec<Self>),
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
    pub mutable: bool,
    pub ty: Ty,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Return {
    pub mutable: bool,
    pub ty: Box<Ty>,
}
