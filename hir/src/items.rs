use smallvec::SmallVec;

use crate::{AdtId, VarId, exprs::ExprId, patterns::Pat, types::Ty};

#[derive(Debug, PartialEq)]
pub struct ExecItem {
    pub ident: VarId,
    pub kind: ExecKind,
}

#[derive(Debug, PartialEq)]
pub enum ExecKind {
    Const {
        ty: Option<Ty>,
        val: ExprId,
    },
    Fn {
        generics: SmallVec<[AdtId; 4]>,
        params: Vec<Param>,
        ret_mut: bool,
        ret_ty: Ty,
        body: ExprId,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub mutable: bool,
    pub pat: Pat,
    pub ty: Ty,
}
