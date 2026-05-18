use std::borrow::Cow;

use span::Span;

pub type Result<T, E> = std::result::Result<T, Error<E>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error<E>(Box<ErrorInner<E>>);

#[derive(Debug, Clone, PartialEq, Eq)]
struct ErrorInner<E> {
    kind: E,
    span: Span,
    ctx: Vec<Cow<'static, str>>,
}

impl<E> Error<E> {
    pub fn new(err: E, span: impl Into<Span>) -> Self {
        Self(Box::new(ErrorInner {
            kind: err,
            span: span.into(),
            ctx: Vec::new(),
        }))
    }

    #[must_use]
    pub fn with_ctx(mut self, ctx: impl Into<Cow<'static, str>>) -> Self {
        self.0.ctx.push(ctx.into());
        self
    }

    pub fn kind(&self) -> &E {
        &self.0.kind
    }

    pub fn span(&self) -> Span {
        self.0.span
    }

    pub fn ctx(&self) -> &[Cow<'static, str>] {
        &self.0.ctx
    }
}

impl<E: ToString> Error<E> {
    pub fn msg(&self) -> String {
        self.0.kind.to_string()
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
            Err(e) => Err(e.with_ctx(ctx)),
        }
    }
}
