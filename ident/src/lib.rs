use std::{
    fmt::{self, Display, Formatter},
    ops::Deref,
    range::Range,
    sync::{LazyLock, Mutex, MutexGuard},
};

use derive_more::Display;
use string_interner::{DefaultStringInterner as Interner, symbol::SymbolU32};

fn interner() -> MutexGuard<'static, Interner> {
    static INTERNER: LazyLock<Mutex<Interner>> = LazyLock::new(Mutex::default);
    INTERNER.lock().expect("Interner Poisoned!")
}

fn get_str(interner: &Interner, ident: Ident) -> &str {
    interner
            .resolve(ident.0)
            .expect("Idents can only be created through interning a value, so the value will exist in the interner")
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ident(SymbolU32);

impl fmt::Debug for Ident {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let interner = interner();
        f.debug_tuple("Ident")
            .field(&get_str(&interner, *self))
            .finish()
    }
}

impl Display for Ident {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        get_str(&interner(), *self).fmt(f)
    }
}

impl PartialEq<&str> for Ident {
    fn eq(&self, other: &&str) -> bool {
        get_str(&interner(), *self) == *other
    }
}

impl Ident {
    pub fn new(string: &str) -> Self {
        Self(interner().get_or_intern(string))
    }

    pub fn span(self, span: impl Into<Range<usize>>) -> SpanIdent {
        SpanIdent {
            ident: self,
            span: span.into(),
        }
    }

    pub fn str<'guard>(self) -> StrGuard<'guard> {
        StrGuard {
            ident: self,
            guard: interner(),
        }
    }
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash)]
#[display("{ident}")]
pub struct SpanIdent {
    pub ident: Ident,
    pub span: Range<usize>,
}

pub struct StrGuard<'guard> {
    ident: Ident,
    guard: MutexGuard<'guard, Interner>,
}

impl Deref for StrGuard<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        get_str(&self.guard, self.ident)
    }
}
