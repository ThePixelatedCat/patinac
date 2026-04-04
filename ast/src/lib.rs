use ident::Ident;

pub mod exprs;
pub mod items;
pub mod patterns;
pub mod types;

use crate::{
    items::{AdtItem, ExecItem},
    patterns::Pat,
    types::Ty,
};

#[derive(Default)]
pub struct Ast<T> {
    pub adts: Vec<AdtItem>,
    pub execs: Vec<ExecItem<T>>,
}
