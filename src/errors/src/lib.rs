//! Error-handling utilities.

use std::{range::Range, result};

use derive_more::{Display, Error};
use smallvec::SmallVec;
use smol_str::SmolStr;

/// The [`Result`][result::Result] type alias used throughout most of the compiler. Defaults to [`HandledError`] for it's `Err` variant.
pub type Result<T, E = HandledError> = result::Result<T, E>;

/// Indicates that an error occurred, and detailed information was provided to an [`ErrorHandler`].
#[derive(Error, Debug, Display, Clone, Copy, PartialEq, Eq)]
#[display("Detailed error was printed to stderr")]
#[non_exhaustive]
pub struct HandledError;

/// A general-purpose error type, wrapping a generic error kind with location information and context support.
///
/// The internal data is boxed to minimise the size of this type.
#[derive(Debug, PartialEq, Eq)]
pub struct Error<E>(Box<ErrorInner<E>>);

impl<E> Error<E> {
    /// Constructs a new error with the provided kind and span, and no context information.
    pub fn new(err: E, span: impl Into<Range<usize>>) -> Self {
        Self(Box::new(ErrorInner {
            kind: err,
            span: span.into(),
            ctx: SmallVec::new(),
        }))
    }

    /// Appends the given context message to this error. Can be chained.
    ///
    /// Consider using [`Self::with_static_ctx()`] if your context message is a string literal.
    #[must_use]
    pub fn with_ctx(mut self, ctx: impl Into<SmolStr>) -> Self {
        self.0.ctx.push(ctx.into());
        self
    }

    /// Appends the given static context message to this error. Can be chained.
    ///
    /// Will never allocate for the context message, so may be more efficient than [`Self::with_ctx()`] for literal context messages.
    #[must_use]
    pub fn with_static_ctx(mut self, ctx: &'static str) -> Self {
        self.0.ctx.push(SmolStr::new_static(ctx));
        self
    }

    /// Returns the underlying error kind.
    pub fn kind(&self) -> &E {
        &self.0.kind
    }

    /// Returns the span of the error.
    pub fn span(&self) -> Range<usize> {
        self.0.span
    }

    /// Returns any provided context information for the error.
    pub fn ctx(&self) -> &[SmolStr] {
        &self.0.ctx
    }
}

impl<E: ToString> Error<E> {
    /// Calls the [`ToString`] implementation of the underlying error kind.
    pub fn msg(&self) -> String {
        self.0.kind.to_string()
    }
}

#[derive(Debug, PartialEq, Eq)]
struct ErrorInner<E> {
    kind: E,
    span: Range<usize>,
    ctx: SmallVec<[SmolStr; 1]>,
}

/// A utility type to handle configurable, recoverable diagnostic reporting.
///
/// The primary usecase is reporting errors via [`err()`][Self::err()] and returning the [`HandledError`].
/// This type internally tracks if any errors have been reported, which can be used through [`checked()`][`Self::checked()`].
///
/// Cloning this type is cheap, and it could implement `Copy` but doesn't for similar reasons to iterators.
#[derive(Clone)]
pub struct ErrorHandler<'callback> {
    f: &'callback dyn Fn(&str, Range<usize>, DiagnosticKind),
    has_err: bool,
}

impl<'callback> ErrorHandler<'callback> {
    /// Constructs a new `ErrorHandler` with the provided reporting callback.
    pub const fn new(f: &'callback dyn Fn(&str, Range<usize>, DiagnosticKind)) -> Self {
        Self { f, has_err: false }
    }

    /// Reports the provided error and produces a [`HandledError`] for the caller to use.
    #[allow(
        clippy::needless_pass_by_value,
        reason = "Semantically useful to enforce that an error can only be reported once"
    )]
    pub fn err<E: ToString>(&mut self, error: Error<E>) -> HandledError {
        self.has_err = true;
        (self.f)(&error.msg(), error.span(), DiagnosticKind::Error);
        HandledError
    }

    /// Reports a warning.
    pub fn warn(&self, msg: &str, span: Range<usize>) {
        (self.f)(msg, span, DiagnosticKind::Warning);
    }

    /// Returns the provided value wrapped in [`Ok`], or a [`HandledError`] if this handler has reported any errors.
    ///
    /// # Errors
    /// [`HandledError`] if this handler has reported any errors.
    pub fn checked<T>(&self, val: T) -> Result<T> {
        if self.has_err {
            Err(HandledError)
        } else {
            Ok(val)
        }
    }

    /// An error handler used for tests. Provides simple debug output of errors.
    #[allow(
        clippy::use_debug,
        reason = "This handler is for use in tests, where debug output is desirable"
    )]
    pub const TEST: Self =
        ErrorHandler::new(&|str, span, kind| eprintln!("{kind:?}: {str} ({span:?})"));
    /// An error handler that discards the errors. Primarily for tests intended to produce errors, to avoid clogging the terminal.
    pub const DUMMY: Self = ErrorHandler::new(&|_, _, _| {});
}

/// Signals the kind of diagnostic being reported.
#[derive(Debug, Clone, Copy)]
pub enum DiagnosticKind {
    /// A fatal error.
    Error,
    /// A warning. Compilation can continue as normal, but the programmer should be notified of something.
    Warning,
}

/// Iterator extension trait providing a method that behehaves similarly to [`try_collect`][`Iterator::try_collect`],
/// but collects all elements eagerly before returning any error.
pub trait TryCollectEager<T, E> {
    /// Similar to [`try_collect`][`Iterator::try_collect`],
    /// but eagerly collects all elements of the iterator before returning an error if any of the elements were errors.
    /// Only for unit error types.
    ///
    /// # Errors
    /// Returns the error value if any of the elements of the iterator were an error, but only after evaluating every element.
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
