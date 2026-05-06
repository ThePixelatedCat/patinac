use std::ops::Index;

use ast::{items::AdtItem, types::Ty};

use ident::Ident;
use span::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VarId(u32);
impl From<VarId> for u32 {
    fn from(value: VarId) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VarInfo {
    pub ident: Ident,
    pub mutable: bool,
    pub ty: Option<Ty<AdtId>>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AdtId(u32);
impl From<AdtId> for u32 {
    fn from(value: AdtId) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdtInfo {
    Item(AdtItem<AdtId>),
    Param(Ident),
}

#[derive(Debug, Default, PartialEq)]
pub struct NameTable {
    pub adts: AdtTable,
    pub vars: VarTable,
}

impl Index<VarId> for NameTable {
    type Output = VarInfo;

    fn index(&self, index: VarId) -> &Self::Output {
        &self.vars[index]
    }
}

impl Index<AdtId> for NameTable {
    type Output = AdtInfo;

    fn index(&self, index: AdtId) -> &Self::Output {
        &self.adts[index]
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct PartialAdtTable(Vec<Option<AdtInfo>>);

impl Index<AdtId> for PartialAdtTable {
    type Output = Option<AdtInfo>;

    fn index(&self, index: AdtId) -> &Self::Output {
        &self.0[index.0 as usize]
    }
}

impl PartialAdtTable {
    pub(crate) fn insert(&mut self, info: AdtInfo) -> AdtId {
        let id = AdtId(self.0.len().try_into().unwrap());
        self.0.push(Some(info));
        id
    }

    pub(crate) fn reserve(&mut self) -> AdtId {
        let id = AdtId(self.0.len().try_into().unwrap());
        self.0.push(None);
        id
    }

    pub(crate) fn fulfill(&mut self, id: AdtId, info: AdtInfo) {
        self.0[id.0 as usize].get_or_insert(info);
    }

    pub(crate) fn finalise(self) -> AdtTable {
        AdtTable(
            self.0
                .into_iter()
                .map(|i| i.expect("Defined but unresolved adt- this indicates an internal bug"))
                .collect(),
        )
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct AdtTable(Vec<AdtInfo>);

impl Index<AdtId> for AdtTable {
    type Output = AdtInfo;

    fn index(&self, index: AdtId) -> &Self::Output {
        &self.0[index.0 as usize]
    }
}

impl AdtTable {
    pub(crate) fn insert(&mut self, info: AdtInfo) -> AdtId {
        let id = AdtId(self.0.len().try_into().unwrap());
        self.0.push(info);
        id
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct PartialVarTable(Vec<Option<VarInfo>>);

impl Index<VarId> for PartialVarTable {
    type Output = Option<VarInfo>;

    fn index(&self, index: VarId) -> &Self::Output {
        &self.0[index.0 as usize]
    }
}

impl PartialVarTable {
    pub(crate) fn insert(&mut self, info: VarInfo) -> VarId {
        let id = VarId(self.0.len().try_into().unwrap());
        self.0.push(Some(info));
        id
    }

    pub(crate) fn reserve(&mut self) -> VarId {
        let id = VarId(self.0.len().try_into().unwrap());
        self.0.push(None);
        id
    }

    pub(crate) fn fulfill(&mut self, id: VarId, info: VarInfo) {
        self.0[id.0 as usize].get_or_insert(info);
    }

    pub(crate) fn finalise(self) -> VarTable {
        VarTable(
            self.0
                .into_iter()
                .map(|i| {
                    i.expect("Defined but unresolved variable name- this indicates an internal bug")
                })
                .collect(),
        )
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct VarTable(Vec<VarInfo>);

impl Index<VarId> for VarTable {
    type Output = VarInfo;

    fn index(&self, index: VarId) -> &Self::Output {
        &self.0[index.0 as usize]
    }
}

impl VarTable {
    pub(crate) fn insert(&mut self, info: VarInfo) -> VarId {
        let id = VarId(self.0.len().try_into().unwrap());
        self.0.push(info);
        id
    }
}
