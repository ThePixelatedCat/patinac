use std::ops::Index;

use super::Type;

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct AdtDefs(Vec<AdtInfo>);

impl Index<AdtId> for AdtDefs {
    type Output = AdtInfo;

    fn index(&self, index: AdtId) -> &Self::Output {
        &self.0[index.0]
    }
}

impl AdtDefs {
    pub fn add_record<'src>(
        &mut self,
        ident: String,
        generics: Vec<String>,
        fields: Vec<Field>,
    ) -> (AdtId, &'src str) {
        todo!();
        // let id = AdtId::from(self.0.len());
        // self.0.push(AdtInfo {
        //     ident,
        //     generics,
        //     data: AdtData::Record(fields),
        // });
        // id
    }

    pub fn add_enum<'src>(
        &mut self,
        ident: String,
        generics: Vec<String>,
        variants: Vec<Variant>
    ) -> (AdtId, &'src str) {
        (AdtId(0), &ident)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdtId(usize);

impl From<usize> for AdtId {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdtInfo {
    ident: String,
    generics: Vec<String>,
    data: AdtData,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdtData {
    Record(Vec<Field>),
    Enum(Vec<Variant>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    ident: String,
    fields: Vec<Field>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub ident: String,
    pub ty: Type,
}
