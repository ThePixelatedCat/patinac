use std::{
    error::Error,
    fmt::Display,
    ops::{Deref, Range},
};

#[macro_export]
macro_rules! span {
    ($t:ident as $s:ident) => {
        pub type $s = $crate::helpers::Spanned<$t>;
        impl $t {
            pub fn spanned(
                self,
                span: impl Into<$crate::helpers::Span>,
            ) -> $crate::helpers::Spanned<Self> {
                $crate::helpers::Spanned {
                    inner: self,
                    span: span.into(),
                }
            }
        }
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spanned<T> {
    pub inner: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn as_deref(&self) -> Spanned<&T::Target>
    where
        T: Deref,
    {
        Spanned {
            inner: &*self.inner,
            span: self.span,
        }
    }

    pub fn span(inner: T, span: impl Into<Span>) -> Self {
        Self {
            inner,
            span: span.into(),
        }
    }
}

impl<T: Display> Display for Spanned<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.span, self.inner)
    }
}

impl<T: Error> Error for Spanned<T> {}

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
