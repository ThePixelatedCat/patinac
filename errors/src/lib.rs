use smol_str::SmolStr;
use span::Span;

pub type Result<T, E> = std::result::Result<T, Error<E>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error<E>(Box<ErrorInner<E>>);

#[derive(Debug, Clone, PartialEq, Eq)]
struct ErrorInner<E> {
    kind: E,
    span: Span,
    ctx: Vec<SmolStr>,
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
    pub fn with_ctx(mut self, ctx: impl Into<SmolStr>) -> Self {
        self.0.ctx.push(ctx.into());
        self
    }

    #[must_use]
    pub fn with_static_ctx(mut self, ctx: &'static str) -> Self {
        self.0.ctx.push(SmolStr::new_static(ctx));
        self
    }

    pub fn add_ctx(&mut self, ctx: impl Into<SmolStr>) {
        self.0.ctx.push(ctx.into());
    }

    pub fn add_static_ctx(&mut self, ctx: &'static str) {
        self.0.ctx.push(SmolStr::new_static(ctx));
    }

    pub fn kind(&self) -> &E {
        &self.0.kind
    }

    pub fn span(&self) -> Span {
        self.0.span
    }

    pub fn ctx(&self) -> &[SmolStr] {
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
    fn with_ctx(self, ctx: impl Into<SmolStr>) -> Self;

    #[must_use]
    fn with_static_ctx(self, ctx: &'static str) -> Self;
}

impl<T, E> ResultExt for Result<T, E> {
    fn with_ctx(self, ctx: impl Into<SmolStr>) -> Self {
        match self {
            ok @ Ok(_) => ok,
            Err(e) => Err(e.with_ctx(ctx)),
        }
    }

    fn with_static_ctx(self, ctx: &'static str) -> Self {
        match self {
            ok @ Ok(_) => ok,
            Err(e) => Err(e.with_static_ctx(ctx)),
        }
    }
}
