use std::sync::{Mutex, OnceLock};

use string_interner::{DefaultStringInterner, symbol::SymbolU32};

type Symbol = SymbolU32;
type Interner = DefaultStringInterner;

static INTERNER: OnceLock<Mutex<Interner>> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ident(Symbol);

impl From<Symbol> for Ident {
    fn from(value: Symbol) -> Self {
        Self(value)
    }
}

impl Ident {
    pub fn new(string: &str) -> Self {
        INTERNER
            .get_or_init(Mutex::default)
            .lock()
            .expect("Poisoned!")
            .get_or_intern(string)
            .into()
    }
}
