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
    pub fn add_record(
        &mut self,
        name: String,
        generic_params: Vec<String>,
        fields: Vec<Field>,
    ) -> AdtId {
        let id = AdtId::from(self.0.len());
        self.0.push(AdtInfo {
            name,
            generic_params,
            data: AdtData::Record(fields),
        });
        id
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
    name: String,
    generic_params: Vec<String>,
    data: AdtData,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdtData {
    Record(Vec<Field>),
    Enum(Vec<Variant>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Variant {
    Unit(String),
    Tuple(String, Vec<Type>),
    Struct(String, Vec<Field>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    pub ty: Type,
}
