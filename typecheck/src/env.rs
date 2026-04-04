use fnv::FnvHashMap;
use ident::Ident;
use span::Span;

use crate::{ErrorKind, Result, Ty};

#[derive(Clone, Default)]
pub struct Ctx(im::HashMap<Ident, BindingInfo>);

impl IntoIterator for Ctx {
    type Item = (Ident, BindingInfo);
    type IntoIter = im::hashmap::ConsumingIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<(Ident, BindingInfo)> for Ctx {
    fn from_iter<T: IntoIterator<Item = (Ident, BindingInfo)>>(iter: T) -> Self {
        Self(im::HashMap::from_iter(iter))
    }
}

impl Ctx {
    pub fn insert(&mut self, ident: Ident, ty: Ty, mutable: bool) {
        self.0.insert(ident, BindingInfo { ty, mutable });
    }

    pub fn get(&self, ident: Ident, span: Span) -> Result<BindingInfo> {
        self.0
            .get(&ident)
            .cloned()
            .ok_or_else(|| ErrorKind::UnboundIdent.span(span))
    }
}

#[derive(Clone)]
pub struct BindingInfo {
    pub ty: Ty,
    pub mutable: bool,
}

impl BindingInfo {
    pub const fn new(ty: Ty, mutable: bool) -> Self {
        Self { ty, mutable }
    }
}

#[derive(Clone, Default)]
pub struct TyEnv(im::HashMap<Ident, TyInfo>);

impl TyEnv {
    pub fn get_field(&self, base: Ident, field: Ident, span: Span) -> Result<Ty> {
        self.0
            .get(&base)
            .ok_or_else(|| ErrorKind::UnknownType.span(span))?
            .fields
            .get(&field)
            .cloned()
            .ok_or_else(|| ErrorKind::MissingField.span(span))
    }

    pub fn insert(&mut self, ty: Ident, info: TyInfo) {
        self.0.insert(ty, info);
    }
}

#[derive(Clone, Default)]
pub struct TyInfo {
    //generic_params: Vec<GenericParam>,
    pub fields: FnvHashMap<Ident, Ty>,
}
