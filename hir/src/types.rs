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
    Fn(Vec<Param>, Return),
    Adt(AdtId, Vec<Self>),
}

impl Ty {
    // /// Helper to create a new empty [`TyKind::Tuple`] for representing the Unit type
    // pub const fn unit() -> Self {
    //     Self::Tuple(vec![])
    // }

    // pub fn named(name: &str) -> Self {
    //     //Self::Adt(Ident::new(name), vec![])
    //     todo!()
    // }

    // /// Helper to create a new [`TyKind::Adt`] for a `String`
    // pub fn string() -> Self {
    //     Self::named("String")
    // }

    // /// Helper to create a new [`TyKind::Adt`] for an `Array` storing the given type
    // pub fn array(inner: Ty) -> Self {
    //     //Self::Adt(Ident::new("Array"), vec![inner])
    //     todo!()
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
