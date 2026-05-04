use std::{
    fmt::{self, Display},
    sync::{LazyLock, Mutex, MutexGuard},
};

use span::Span;

type Symbol = string_interner::symbol::SymbolU32;
type Interner = string_interner::DefaultStringInterner;

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
pub struct Ident(Symbol);

impl fmt::Debug for Ident {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let interner = interner();
        f.debug_tuple("Ident")
            .field(&get_str(&interner, *self))
            .finish()
    }
}

impl Display for Ident {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        get_str(&interner(), *self).fmt(f)
    }
}

impl Ident {
    pub fn new(string: &str) -> Self {
        Self(interner().get_or_intern(string))
    }

    pub fn span(self, span: impl Into<Span>) -> SpanIdent {
        SpanIdent {
            ident: self,
            span: span.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpanIdent {
    pub ident: Ident,
    pub span: Span,
}
