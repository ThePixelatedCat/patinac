use smallvec::SmallVec;

pub mod exprs;
pub mod items;
pub mod patterns;

pub struct Ast<TyInfo, AdtIdent, VarIdent> {
    pub adts: Vec<items::AdtItem<AdtIdent>>,
    pub execs: Vec<items::ExecItem<TyInfo, AdtIdent, VarIdent>>,
}

impl<T, A, V> Default for Ast<T, A, V> {
    fn default() -> Self {
        Self {
            adts: Vec::default(),
            execs: Vec::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path<PreIdent, EndIdent> {
    pub prefix: SmallVec<[PreIdent; 4]>,
    pub end: EndIdent,
}
