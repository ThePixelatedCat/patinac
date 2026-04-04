use std::{
    fmt::Display,
    sync::{Mutex, MutexGuard, OnceLock},
};

use string_interner::{DefaultStringInterner, symbol::SymbolU32};

type Symbol = SymbolU32;
type Interner = DefaultStringInterner;

static INTERNER: OnceLock<Mutex<Interner>> = OnceLock::new();

fn get_interner() -> MutexGuard<'static, Interner> {
    INTERNER
        .get_or_init(Mutex::default)
        .lock()
        .expect("Poisoned!")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ident(Symbol);

impl From<Symbol> for Ident {
    fn from(value: Symbol) -> Self {
        Self(value)
    }
}

impl Display for Ident {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        get_interner().resolve(self.0).unwrap().fmt(f)
    }
}

impl Ident {
    pub fn new(string: &str) -> Self {
        get_interner().get_or_intern(string).into()
    }

    pub fn new_static(string: &'static str) -> Self {
        get_interner().get_or_intern_static(string).into()
    }
}
