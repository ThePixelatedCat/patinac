//! The representation of source code produced by the parser, before any semantic analysis.
//! The primary type is [`Ast`].

use std::{
    fmt::{self, Display, Formatter},
    range::Range,
};

use derive_more::Display;
use itertools::Itertools as _;
use smallvec::SmallVec;

use ident::{Ident, SpanIdent};

/// The top-level representation of a single module, containing all of the module's items.
///
/// For easier manipulation, these items are split into [ type definitions][TyItem] and "[executable items][ExecItem]" (items containing expressions).
#[derive(Default)]
pub struct Ast {
    /// The type definitions of a module, containing both `enum` and `record` definitions.
    pub tys: Vec<TyItem>,
    /// The "executable items" of a module. These are the items that contain expressions.
    pub execs: Vec<ExecItem>,
}

/// A path made of one or more identifiers.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Path {
    head: SmallVec<[Ident; 4]>,
    tail: Ident,
}

impl Display for Path {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut string = String::new();
        for ident in &self.head {
            string.push_str(&ident.str());
            string.push_str("::");
        }
        string.push_str(&self.tail.str());
        string.fmt(f)
    }
}

impl Path {
    /// Create a path made of a single identifier.
    pub const fn single(ident: Ident) -> Self {
        Self {
            head: SmallVec::new_const(),
            tail: ident,
        }
    }

    /// Attempts to create a path, returning None if the provided `Vec` is empty.
    pub fn new(mut path: Vec<Ident>) -> Option<Self> {
        let tail = path.pop()?;
        Some(Self {
            head: path.into(),
            tail,
        })
    }

    /// Creates a path of a constant length.
    ///
    /// # Panics
    /// Will panic at compile-time if the length is 0.
    pub fn new_const<const N: usize>(path: [Ident; N]) -> Self {
        const { assert!(N > 0, "path must be non-empty") }
        Self {
            head: SmallVec::from_slice(&path[0..N - 1]),
            tail: path[N - 1],
        }
    }

    /// Add to the end of the path.
    pub fn push(&mut self, ident: Ident) {
        self.head.push(self.tail);
        self.tail = ident;
    }

    /// Returns the first identifier of the path.
    pub fn start(&self) -> Ident {
        if self.head.is_empty() {
            self.tail
        } else {
            self.head[0]
        }
    }

    /// Returns true if the path is made up of a single identifier.
    pub fn is_single_ident(&self) -> bool {
        self.head.is_empty()
    }

    /// Returns the first identifier of the path, and the rest of the path if it had more than 1 segment.
    pub fn split(mut self) -> (Ident, Option<Self>) {
        if self.head.is_empty() {
            (self.tail, None)
        } else {
            let start = self.head.remove(0);
            (start, Some(self))
        }
    }
}

/// The definition of a type, either a `record` or a `enum`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TyItem {
    /// The name of the type.
    pub ident: SpanIdent,
    /// The declared generic parameters.
    pub generics: SmallVec<[SpanIdent; 4]>,
    /// The kind of type (`record` or `enum`).
    pub kind: TyItemKind,
}

/// The information of a [`TyItem`] specific to whether it's a `record` or an `enum`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TyItemKind {
    /// A `record` type.
    Record(Vec<Field>),
    /// An `enum` type.
    Enum(Vec<Variant>),
}

/// A variant of an `enum`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    /// The name of the variant.
    pub ident: SpanIdent,
    /// The fields contained within the variant.
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// A field of a `record` or of an `enum` variant.
pub struct Field {
    /// The name of the field.
    pub ident: SpanIdent,
    /// The type of the field.
    pub ty: Ty,
}

#[derive(Debug, PartialEq)]
/// An "executable item". These are the items that contain expressions, namely constants and functions.
pub struct ExecItem {
    /// The name of the item.
    pub ident: SpanIdent,
    /// The kind of the item (`const` or `fn`).
    pub kind: ExecKind,
}

#[derive(Debug, PartialEq)]
/// The information of an [`ExecItem`] specific to whether it's a `const` or a `fn`.
pub enum ExecKind {
    /// A constant item.
    Const {
        /// The constant's type.
        ty: Ty,
        /// The initialiser expression.
        val: Expr,
    },
    /// A function item.
    Fn {
        /// The generic parameters.
        generics: SmallVec<[SpanIdent; 4]>,
        /// The value parameters.
        params: Vec<Param>,
        /// Whether the return type is mutable (i.e. a projection).
        ret_mut: bool,
        /// The return type.
        ret_ty: Ty,
        /// The body.
        body: Expr,
    },
}

/// A parameter of a [function item][ExecKind::Fn].
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    /// Whether the parameter is mutable (a second-class in-out reference).
    pub mutable: bool,
    /// The pattern to bind the parameter's variables.
    pub pat: Pat,
    /// The type of the parameter.
    pub ty: Ty,
    /// The span of the parameter, from the `mut` keyword if present to the [`ty`][Param::ty].
    pub span: Range<usize>,
}

/// A statement. Always contained within a [`BlockExpr`].
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// A variable declaration.
    Decl {
        /// The binding information for the variable.
        binding: Binding,
        /// The initial value for the variable.
        val: Expr,
        /// The span of the declaration, starting from the `let` and ending after the [`val`][`Stmt::Decl::val`].
        span: Range<usize>,
    },
    /// An expression used as a statement. Evaluated purely for side-effects.
    Expr(Expr),
}

/// A spanned [expression][ExprKind].
#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    /// The kind of the expression.
    pub kind: ExprKind,
    /// The span of the expression.
    pub span: Range<usize>,
}

impl Expr {
    /// Creates a single-statement block containing this expression.
    ///
    /// The provided span is to account for the curly braces.
    ///
    /// This function primarily exists as a helper for tests.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::range::Range;
    /// # use ast::{ExprKind, Stmt, BlockExpr};
    /// let expr = ExprKind::Tuple(vec![]).span(2..3);
    /// assert_eq!(
    ///     expr.clone().as_block(0..5),
    ///     BlockExpr {
    ///         stmts: vec![Stmt::Expr(expr.clone())],
    ///         span: Range::from(0..5)
    ///     }
    /// )
    /// ```
    pub fn as_block(self, span: impl Into<Range<usize>>) -> BlockExpr {
        BlockExpr {
            stmts: vec![Stmt::Expr(self)],
            span: span.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
/// The kinds of expressions.
pub enum ExprKind {
    /// A variable name, such as `foo`.
    Var(Path),
    /// A basic literal value, such as `1.2` or `"Hello, World"`. The specific kinds of literals are represented by [`LitExpr`].
    Lit(LitExpr),
    /// An array literal, such as `[1, 2, 3]`.
    Array(Vec<Expr>),
    /// A tuple literal, such as `(1, 2.0, "3")`.
    Tuple(Vec<Expr>),
    /// An infix operation, such as `1 + 2`. This includes assignment.
    Infix {
        /// The infix operator used.
        op: InfixOp,
        /// The left-hand side of the operation.
        lhs: Box<Expr>,
        /// The right-hand side of the operation.
        rhs: Box<Expr>,
    },
    /// A prefix operation, such as `!true`.
    Prefix {
        /// The prefix operator used.
        op: PrefixOp,
        /// The base expression the operator is attached to.
        expr: Box<Expr>,
    },
    /// Record field access, such as `foo.bar`.
    Field {
        /// The base expression from which the field is being accessed.
        base: Box<Expr>,
        /// The name of the field.
        field: SpanIdent,
    },
    /// Array indexing, such as `foo.[0]`.
    Index {
        /// The base expression being indexed into.
        arr: Box<Expr>,
        /// The index to access.
        idx: Box<Expr>,
    },
    /// A function call, such as `sin(90.0)`.
    Call {
        /// The function being called.
        func: Box<Expr>,
        /// The list of arguments being applied.
        args: Vec<Arg>,
    },
    /// A lambda expression, such as `fn(x) -> x * 2`.
    Lambda {
        /// The parameters of the function.
        params: Vec<Binding>,
        /// The body of the function.
        body: Box<Expr>,
    },
    /// An if expression, such as `if foo { 1.0 } else { 2.0 }`.
    If {
        /// The condition of the if.
        cond: Box<Expr>,
        /// The "then" block.
        th: BlockExpr,
        /// The "else" block, if there is one.
        el: Option<BlockExpr>,
    },
    /// A match expression, such as `foo.match { Some(x) -> x, None -> panic() }`.
    Match {
        /// The value being matched against.
        scrutinee: Box<Expr>,
        /// The match arms.
        arms: Vec<MatchArm>,
    },
    /// A for-loop expression, such as `for x in [1, 2, 3] { println(x) }`.
    For {
        pat: Pat,
        iter: Box<Expr>,
        body: BlockExpr,
    },
    Loop(BlockExpr),
    Break,
    Continue,
    Return(Box<Expr>),
    Block(BlockExpr),

    Print(Box<Expr>),
}

impl ExprKind {
    pub fn span(self, span: impl Into<Range<usize>>) -> Expr {
        Expr {
            kind: self,
            span: span.into(),
        }
    }

    pub fn ident(string: &str) -> Self {
        Self::Var(Path::single(Ident::new(string)))
    }

    pub const fn int(i: u64) -> Self {
        Self::Lit(LitExpr::Int(i))
    }

    pub const fn float(f: f64) -> Self {
        Self::Lit(LitExpr::Float(f))
    }

    pub const fn char(c: char) -> Self {
        Self::Lit(LitExpr::Char(c))
    }

    pub fn string(s: &str) -> Self {
        Self::Lit(LitExpr::String(String::from(s)))
    }

    pub const fn bool(b: bool) -> Self {
        Self::Lit(LitExpr::Bool(b))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LitExpr {
    Int(u64),
    Float(f64),
    Char(char),
    String(String),
    Bool(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Binding {
    pub mutable: bool,
    pub pat: Pat,
    pub ty: Option<Ty>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Arg {
    pub val: Expr,
    pub mutable: bool,
    pub span: Range<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pat: Pat,
    pub body: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockExpr {
    pub stmts: Vec<Stmt>,
    pub span: Range<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfixOp {
    Assign,
    Add,
    AddF,
    Sub,
    SubF,
    Mul,
    MulF,
    Div,
    DivF,
    Exp,
    And,
    Or,
    Xor,
    Eqq,
    Neq,
    Gt,
    Lt,
    Geq,
    Leq,
}

impl InfixOp {
    pub const fn binding_power(self) -> (u8, u8) {
        match self {
            Self::Assign => (1, 0),
            Self::Or => (3, 4),
            Self::And => (5, 6),
            Self::Eqq | Self::Neq => (7, 8),
            Self::Gt | Self::Lt | Self::Leq | Self::Geq => (9, 10),
            Self::Xor => (13, 14),
            Self::Add | Self::AddF | Self::Sub | Self::SubF => (17, 18),
            Self::Mul | Self::MulF | Self::Div | Self::DivF => (19, 20),
            Self::Exp => (22, 21),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefixOp {
    Not,
    Neg,
}

impl PrefixOp {
    pub const fn binding_power(self) -> u8 {
        match self {
            Self::Neg | Self::Not => 51,
        }
    }
}

#[derive(Debug, Display, Clone, PartialEq, Eq, Hash)]
#[display("{kind}")]
pub struct Ty {
    pub kind: TyKind,
    pub span: Range<usize>,
}

/// The kinds of types.
#[derive(Debug, Display, Clone, PartialEq, Eq, Hash)]
pub enum TyKind {
    Int,
    UInt,
    Byte,
    Float,
    Char,
    Bool,
    #[display("[{_0}]")]
    Array(Box<Ty>),
    #[display("{{{}}}", _0.iter().join(", "))]
    Tuple(Vec<Ty>),
    #[display("fn({}) -> {_1}", _0.iter().join(", "))]
    Fn(Vec<ParamTy>, Return),
    #[display("{_0}[{}]", _1.iter().join(", "))]
    Named(Path, Vec<Ty>),
}

impl TyKind {
    pub fn span(self, span: impl Into<Range<usize>>) -> Ty {
        Ty {
            kind: self,
            span: span.into(),
        }
    }

    /// Helper to create a new empty [`TyKind::Tuple`] for representing the Unit type.
    pub const fn unit() -> Self {
        Self::Tuple(vec![])
    }

    pub fn named(name: &str) -> Self {
        Self::Named(Path::single(Ident::new(name)), vec![])
    }

    /// Helper to create a new [`TyKind::Named`] for a `String`.
    pub fn string() -> Self {
        Self::named("String")
    }
}

/// A parameter of a function type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ParamTy {
    pub ty: Ty,
    pub mutable: bool,
    pub span: Range<usize>,
}

impl Display for ParamTy {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.mutable {
            "mut ".fmt(f)?;
        }
        self.ty.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Return {
    pub mutable: bool,
    pub ty: Box<Ty>,
}

impl Display for Return {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.mutable {
            "mut ".fmt(f)?;
        }
        self.ty.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Pat {
    pub kind: PatKind,
    pub span: Range<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PatKind {
    Literal { negate: bool, lit: LitExpr },
    Wildcard,
    Ident(Ident),
    Constructor(Ident, Vec<Pat>),
    Tuple(Vec<Pat>),
}

impl PatKind {
    pub fn span(self, span: impl Into<Range<usize>>) -> Pat {
        Pat {
            kind: self,
            span: span.into(),
        }
    }

    pub fn ident(name: &str) -> Self {
        Self::Ident(Ident::new(name))
    }
}
