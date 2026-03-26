use std::{
    error::Error,
    fmt::Display,
    ops::{Deref, Range},
};

impl Spannable for String {}

impl<T, E> SpanErr<T, E> for Result<T, E> {
    fn span_err(self, span: Span) -> Result<T, Spnd<E>> {
        self.map_err(|e| Spnd::span(e, span))
    }
}

pub trait SpanErr<T, E> {
    fn span_err(self, span: Span) -> Result<T, Spnd<E>>;
}

pub trait Spannable
where
    Self: Sized,
{
    fn span(self, span: impl Into<Span>) -> Spnd<Self> {
        Spnd::span(self, span)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spnd<T>(pub T, pub Span);

impl<T> Spnd<T> {
    pub fn as_deref(&self) -> Spnd<&T::Target>
    where
        T: Deref,
    {
        Spnd(&*self.0, self.1)
    }

    pub fn span(inner: T, span: impl Into<Span>) -> Self {
        Self(inner, span.into())
    }
}

impl<T: Display> Display for Spnd<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.1, self.0)
    }
}

impl<T: Error> Error for Spnd<T> {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl From<Range<usize>> for Span {
    fn from(value: Range<usize>) -> Self {
        Self {
            start: value.start,
            end: value.end,
        }
    }
}

impl From<Span> for Range<usize> {
    fn from(value: Span) -> Self {
        value.start..value.end
    }
}

impl Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}