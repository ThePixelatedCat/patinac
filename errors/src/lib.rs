use std::borrow::Cow;

use span::Span;

pub type Result<T, E> = std::result::Result<T, Error<E>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error<E> {
    pub kind: E,
    pub span: Span,
    pub ctx: Vec<Cow<'static, str>>,
}

impl<E> Error<E> {
    pub fn span(err: E, span: impl Into<Span>) -> Self {
        Self {
            kind: err,
            span: span.into(),
            ctx: vec![],
        }
    }

    #[must_use]
    pub fn context(mut self, ctx: impl Into<Cow<'static, str>>) -> Self {
        self.ctx.push(ctx.into());
        self
    }
}

pub trait ResultExt {
    #[must_use]
    fn context(self, ctx: impl Into<Cow<'static, str>>) -> Self;
}

impl<T, E> ResultExt for Result<T, E> {
    fn context(self, ctx: impl Into<Cow<'static, str>>) -> Self {
        match self {
            ok @ Ok(_) => ok,
            Err(e) => Err(e.context(ctx)),
        }
    }
}
