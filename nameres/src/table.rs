use std::ops::Index;

use ast::{items::AdtItem, types::Ty};

use ident::Ident;
use span::Span;

#[derive(Clone, Copy)]
pub struct VarId(u32);

pub struct VarInfo {
    pub ident: Ident,
    pub mutable: bool,
    pub ty: Option<Ty<AdtId>>,
    pub span: Span,
}

#[derive(Clone, Copy)]
pub struct AdtId(u32);

pub enum AdtInfo {
    Item(AdtItem<AdtId>),
    Param(Ident),
}

#[derive(Default)]
pub struct NameTable {
    vars: Vec<VarInfo>,
    adts: Vec<AdtInfo>,
}

impl Index<VarId> for NameTable {
    type Output = VarInfo;

    fn index(&self, index: VarId) -> &Self::Output {
        &self.vars[index.0 as usize]
    }
}

impl Index<AdtId> for NameTable {
    type Output = AdtInfo;

    fn index(&self, index: AdtId) -> &Self::Output {
        &self.adts[index.0 as usize]
    }
}

impl NameTable {
    pub(crate) fn insert_var(&mut self, info: VarInfo) -> VarId {
        let id = VarId(self.vars.len().try_into().unwrap());
        self.vars.push(info);
        id
    }

    pub(crate) fn insert_adt(&mut self, info: AdtInfo) -> AdtId {
        let id = AdtId(self.vars.len().try_into().unwrap());
        self.adts.push(info);
        id
    }

    pub(crate) fn reserve_var(&mut self) -> Reservation<'_, VarId> {
        let id = VarId(self.vars.len().try_into().unwrap());
        Reservation { table: self, id }
    }

    pub(crate) fn reserve_adt(&mut self) -> Reservation<'_, AdtId> {
        let id = AdtId(self.vars.len().try_into().unwrap());
        Reservation { table: self, id }
    }
}

pub struct Reservation<'a, I> {
    table: &'a mut NameTable,
    id: I,
}

impl<I> Reservation<'_, I> {
    pub(crate) const fn table(&self) -> &NameTable {
        self.table
    }

    pub(crate) const fn id(&self) -> I
    where
        I: Copy,
    {
        self.id
    }
}

impl Reservation<'_, VarId> {
    pub(crate) fn check_in(self, info: VarInfo) -> VarId {
        self.table.vars.push(info);
        self.id
    }
}

impl Reservation<'_, AdtId> {
    pub(crate) fn check_in(self, info: AdtInfo) -> AdtId {
        self.table.adts.push(info);
        self.id
    }
}
