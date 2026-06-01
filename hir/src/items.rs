use derive_more::{From, IntoIterator};
use slotmap::new_key_type;

use ident::{Ident, SpanIdent};
use smallvec::SmallVec;

use crate::{VarId, exprs::ExprId, types::Ty};

new_key_type! { pub struct TyId; }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TyInfo {
    pub fields: Fields,
    pub constructor_id: VarId,
}

#[derive(From, Debug, Clone, PartialEq, Eq, IntoIterator)]
#[into_iterator(ref, ref_mut, owned)]
pub struct Fields(Vec<(SpanIdent, Ty)>);
impl Fields {
    pub fn get_ty(&self, ident: Ident) -> Option<&Ty> {
        self.0
            .iter()
            .find(|(id, _)| id.ident == ident)
            .map(|(_, ty)| ty)
    }

    /// # Panics
    /// Panics if there is no field with the given name
    pub fn get_ty_idx(&self, ident: Ident) -> (u32, &Ty) {
        self.0
            .iter()
            .enumerate()
            .find(|(_, (id, _))| id.ident == ident)
            .map(|(idx, (_, ty))| (u32::try_from(idx).unwrap(), ty))
            .unwrap()
    }

    pub fn tys(&self) -> impl Iterator<Item = &Ty> {
        self.0.iter().map(|(_, ty)| ty)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ExecItem {
    pub id: VarId,
    pub kind: ExecKind,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ExecKind {
    Const {
        val: ExprId,
    },
    Fn {
        params: SmallVec<[VarId; 3]>,
        body: ExprId,
    },
}
