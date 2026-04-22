use std::{
    fmt::{self, Display},
    sync::{LazyLock, Mutex, MutexGuard},
};

type Symbol = string_interner::symbol::SymbolU32;
type Interner = string_interner::DefaultStringInterner;

fn interner() -> MutexGuard<'static, Interner> {
    static INTERNER: LazyLock<Mutex<Interner>> = LazyLock::new(Mutex::default);
    INTERNER.lock().expect("Interner Poisoned!")
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ident(Symbol);

impl fmt::Debug for Ident {
    #[allow(
        clippy::significant_drop_tightening,
        reason = "literally cannot do that, shut up clippy"
    )]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let interner = interner();
        let val = interner
            .resolve(self.0)
            .expect("Idents can only be created through interning a value, so the value will exist in the interner");
        f.debug_tuple("Ident").field(&val).finish()
    }
}

impl Display for Ident {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        interner()
            .resolve(self.0)
            .expect("Idents can only be created through interning a value, so the value will exist in the interner")
            .fmt(f)
    }
}

impl Ident {
    pub fn new(string: &str) -> Self {
        Self(interner().get_or_intern(string))
    }

    pub fn new_static(string: &'static str) -> Self {
        Self(interner().get_or_intern_static(string))
    }
}
