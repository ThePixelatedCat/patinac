use std::{fmt::Display, ops::Range};

#[macro_export]
macro_rules! impl_span {
    ($self:path as $spanned:ident $(, $doc:expr)?) => {
        impl_span!($self as $spanned<> $(, $doc)?);
    };
    ($self:path as $spanned:ident<$($gen:ident),*> $(, $doc:expr)?) => {
        $(#[doc = $doc])?
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Debug, Clone, PartialEq)]
        pub struct $spanned<$( $gen ),*> {
            pub kind: $self,
            pub span: $crate::Span,
        }

        impl<$( $gen ),*> $self {
            pub fn span(self, span: impl Into<$crate::Span>) -> $spanned<$( $gen ),*> {
                $spanned {
                    kind: self,
                    span: span.into(),
                }
            }
        }
    };
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
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
