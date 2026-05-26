use derive_more::{Display, Error};
use smallvec::SmallVec;
use smol_str::SmolStr;
use span::Span;

pub type Result<T, E = HandledError> = std::result::Result<T, E>;

pub const TEST_HANDLER: ErrorHandler = ErrorHandler::new(&|str, span| eprintln!("{span}: {str}"));
pub const DUMMY_HANDLER: ErrorHandler = ErrorHandler::new(&|_, _| {});

#[derive(Debug, PartialEq, Eq)]
pub struct Error<E>(Box<ErrorInner<E>>);

#[derive(Debug, PartialEq, Eq)]
struct ErrorInner<E> {
    kind: E,
    span: Span,
    ctx: SmallVec<[SmolStr; 1]>,
}

impl<E> Error<E> {
    pub fn new(err: E, span: impl Into<Span>) -> Self {
        Self(Box::new(ErrorInner {
            kind: err,
            span: span.into(),
            ctx: SmallVec::new(),
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

#[derive(Clone)]
pub struct ErrorHandler<'a> {
    f: &'a dyn Fn(&str, Span),
    has_err: bool,
}
impl<'a> ErrorHandler<'a> {
    pub const fn new(f: &'a dyn Fn(&str, Span)) -> Self {
        Self { f, has_err: false }
    }

    #[allow(
        clippy::needless_pass_by_value,
        reason = "Semantically useful to enforce that an error can only be reported once"
    )]
    pub fn err<E: ToString>(&mut self, error: Error<E>) -> HandledError {
        self.has_err = true;
        (self.f)(&error.msg(), error.span());
        HandledError
    }

    /// Returns the provided value wrapped in [`Ok`], or a [`HandledError`] if this handler has reported any errors
    ///
    /// # Errors
    /// [`HandledError`] if this handler has reported any errors
    pub fn checked<T>(&self, val: T) -> Result<T> {
        if self.has_err {
            Err(HandledError)
        } else {
            Ok(val)
        }
    }
}

/// Indicates that an error occurred, and detailed information was provided to an [`ErrorHandler`]
#[derive(Error, Debug, Display, Clone, Copy, PartialEq, Eq)]
#[display("Detailed error was printed to stderr")]
#[non_exhaustive]
pub struct HandledError;

pub trait TryCollectEager<T, E> {
    /// Equivalent to [`try_collect`][`Iterator::try_collect`],
    /// but eagerly collecting all elements of the iterator before returning an error if any of the elements were errors.
    /// Only for unit error types.
    ///
    /// # Errors
    /// Returns the error value if any of the elements of the iterator were an error, but only after evaluating every element
    fn try_collect_eager<U: FromIterator<T>>(self) -> Result<U, E>;
}

impl<T, I: Iterator<Item = Result<T>>> TryCollectEager<T, HandledError> for I {
    fn try_collect_eager<U: FromIterator<T>>(self) -> Result<U, HandledError> {
        try_collect_eager_helper(self, HandledError)
    }
}

impl<T, I: Iterator<Item = Result<T>>> TryCollectEager<T, ()> for I {
    fn try_collect_eager<U: FromIterator<T>>(self) -> Result<U, ()> {
        try_collect_eager_helper(self, ())
    }
}

fn try_collect_eager_helper<T, U: FromIterator<T>, E>(
    iter: impl Iterator<Item = Result<T>>,
    err: E,
) -> Result<U, E> {
    let mut has_err = false;
    let out = iter
        .flat_map(|v| v.inspect_err(|_| has_err = true))
        .collect();
    if has_err { Err(err) } else { Ok(out) }
}
