use foldhash::HashMap;
use slotmap::new_key_type;

use ident::Ident;

use crate::{VarId, exprs::ExprId, types::Ty};

new_key_type! { pub struct AdtId; }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdtInfo {
    pub fields: HashMap<Ident, Ty>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ExecItem {
    pub ident: VarId,
    pub kind: ExecKind,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ExecKind {
    Const {
        ty: Option<Ty>,
        val: ExprId,
    },
    Fn {
        params: Vec<Param>,
        ret_ty: Ty,
        body: ExprId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub mutable: bool,
    pub id: VarId,
    pub ty: Ty,
}
