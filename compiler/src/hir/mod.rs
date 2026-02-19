mod adts;

use adts::{AdtDefs, AdtId};

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Hir {
    pub type_defs: AdtDefs,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Int,
    UInt,
    Byte,
    Float,
    Bool,
    Char,
    Array(Box<Type>),
    Tuple(Vec<Type>),
    Fn(Vec<Type>, Box<Type>),
    Adt(AdtId, Vec<Type>),
}
